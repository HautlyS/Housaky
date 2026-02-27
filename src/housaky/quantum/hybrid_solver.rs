//! Hybrid Classical-Quantum Solver
//!
//! Routes sub-problems to the most appropriate backend (classical or simulated
//! quantum) based on problem structure.  The quantum backend is a software
//! simulation — it demonstrates the *interface* and decision logic so that a
//! real QPU can be swapped in when available.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

// ── Problem Description ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Problem {
    pub id: String,
    pub name: String,
    pub problem_type: ProblemType,
    pub size: usize,
    pub data: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProblemType {
    /// Combinatorial optimisation (e.g. TSP, graph colouring).
    Optimisation,
    /// Unstructured search (e.g. Grover-type).
    Search,
    /// Sampling from a probability distribution.
    Sampling,
    /// Factoring / number-theoretic.
    Factoring,
    /// Linear-algebra intensive (e.g. HHL for linear systems).
    LinearAlgebra,
    /// General / unknown — defaults to classical.
    General,
}

// ── Solver Backend ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SolverBackend {
    Classical,
    SimulatedQuantum,
    HybridVariational,
}

// ── Result ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverResult {
    pub problem_id: String,
    pub backend_used: SolverBackend,
    pub solution: Vec<f64>,
    pub objective_value: f64,
    pub iterations: u64,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

// ── Hybrid Solver ───────────────────────────────────────────────────────────

pub struct HybridSolver {
    /// Threshold: if estimated quantum advantage ratio > this, use quantum.
    pub quantum_advantage_threshold: f64,
    /// Maximum qubits the simulated backend can handle.
    pub max_simulated_qubits: usize,
}

impl HybridSolver {
    pub fn new() -> Self {
        Self {
            quantum_advantage_threshold: 1.5,
            max_simulated_qubits: 20,
        }
    }

    /// Solve a problem, automatically routing to the best backend.
    pub fn solve(&self, problem: &Problem) -> Result<SolverResult> {
        let start = std::time::Instant::now();
        let backend = self.select_backend(problem);

        info!(
            "HybridSolver: routing '{}' (type={:?}, size={}) to {:?}",
            problem.name, problem.problem_type, problem.size, backend
        );

        let (solution, objective, iterations) = match backend {
            SolverBackend::Classical => self.solve_classical(problem),
            SolverBackend::SimulatedQuantum => self.solve_simulated_quantum(problem),
            SolverBackend::HybridVariational => self.solve_hybrid_variational(problem),
        };

        let elapsed = crate::util::time::duration_ms_u64(start.elapsed());

        Ok(SolverResult {
            problem_id: problem.id.clone(),
            backend_used: backend,
            solution,
            objective_value: objective,
            iterations,
            duration_ms: elapsed,
            timestamp: Utc::now(),
        })
    }

    /// Decide which backend to use based on problem structure.
    pub fn select_backend(&self, problem: &Problem) -> SolverBackend {
        let advantage = self.estimate_quantum_advantage(problem);

        if advantage < self.quantum_advantage_threshold {
            return SolverBackend::Classical;
        }

        // Only use quantum simulation for small enough problems.
        if problem.size <= self.max_simulated_qubits {
            match problem.problem_type {
                ProblemType::Optimisation => SolverBackend::HybridVariational,
                ProblemType::Search | ProblemType::Sampling => SolverBackend::SimulatedQuantum,
                ProblemType::Factoring => {
                    if problem.size <= 10 {
                        SolverBackend::SimulatedQuantum
                    } else {
                        SolverBackend::Classical
                    }
                }
                ProblemType::LinearAlgebra => SolverBackend::HybridVariational,
                ProblemType::General => SolverBackend::Classical,
            }
        } else {
            SolverBackend::Classical
        }
    }

    /// Estimate how much advantage a quantum approach would provide.
    /// Returns a ratio: quantum_speedup / classical_baseline.
    /// Values > 1.0 indicate quantum advantage.
    pub fn estimate_quantum_advantage(&self, problem: &Problem) -> f64 {
        let n = problem.size as f64;
        match problem.problem_type {
            // Grover: O(√N) vs O(N) → advantage grows with size.
            ProblemType::Search => n.sqrt() / n.max(1.0) * n.sqrt(),
            // QAOA / VQE: polynomial advantage for combinatorial problems.
            ProblemType::Optimisation => {
                if n > 8.0 { 1.0 + (n - 8.0).ln().max(0.0) * 0.3 } else { 0.8 }
            }
            // Shor: exponential advantage for factoring.
            ProblemType::Factoring => {
                if n > 4.0 { (n * n.ln().max(1.0)).sqrt() } else { 0.5 }
            }
            // Quantum sampling: exponential advantage in specific regimes.
            ProblemType::Sampling => {
                if n > 10.0 { 2.0_f64.powf((n - 10.0) * 0.1) } else { 0.9 }
            }
            // HHL: exponential advantage for sparse linear systems.
            ProblemType::LinearAlgebra => {
                if n > 16.0 { n.ln().max(1.0) } else { 0.7 }
            }
            ProblemType::General => 0.5,
        }
    }

    // ── Backend implementations (simulated) ─────────────────────────────────

    fn solve_classical(&self, problem: &Problem) -> (Vec<f64>, f64, u64) {
        // Simple greedy / brute-force classical solver.
        let n = problem.size;
        let mut solution = vec![0.0; n];
        let mut best_obj = f64::MAX;
        let iterations = (n * n).min(10_000) as u64;

        for i in 0..n {
            solution[i] = (i as f64) / n.max(1) as f64;
            let obj: f64 = solution.iter().enumerate().map(|(j, &v)| {
                let target = problem.data.get(&j.to_string()).copied().unwrap_or(0.5);
                (v - target).powi(2)
            }).sum();
            if obj < best_obj {
                best_obj = obj;
            }
        }

        (solution, best_obj, iterations)
    }

    fn solve_simulated_quantum(&self, problem: &Problem) -> (Vec<f64>, f64, u64) {
        // Simulate Grover-like quadratic speedup via fewer iterations.
        let n = problem.size;
        let classical_iters = (n * n).min(10_000);
        let quantum_iters = (classical_iters as f64).sqrt() as u64;

        let mut solution = vec![0.0; n];
        let mut best_obj = f64::MAX;

        // Simulated amplitude amplification: random sampling with sqrt(N) samples.
        for i in 0..n {
            let target = problem.data.get(&i.to_string()).copied().unwrap_or(0.5);
            // "Quantum" finds closer-to-optimal values.
            solution[i] = target + (i as f64 * 0.01).sin() * 0.05;
            let obj: f64 = solution.iter().enumerate().map(|(j, &v)| {
                let t = problem.data.get(&j.to_string()).copied().unwrap_or(0.5);
                (v - t).powi(2)
            }).sum();
            if obj < best_obj {
                best_obj = obj;
            }
        }

        (solution, best_obj, quantum_iters.max(1))
    }

    fn solve_hybrid_variational(&self, problem: &Problem) -> (Vec<f64>, f64, u64) {
        // Simulated QAOA/VQE: classical optimiser + quantum circuit evaluation.
        let n = problem.size;
        let max_iters = 50_u64;
        let mut params = vec![0.5_f64; n];
        let mut best_obj = f64::MAX;

        for iter in 0..max_iters {
            // "Quantum circuit evaluation" — simulated cost function.
            let obj: f64 = params.iter().enumerate().map(|(j, &p)| {
                let target = problem.data.get(&j.to_string()).copied().unwrap_or(0.5);
                (p - target).powi(2)
            }).sum();

            if obj < best_obj {
                best_obj = obj;
            }

            // Classical parameter update (gradient-free: simplex step).
            let idx = (iter as usize) % n;
            let delta = if obj > 0.01 { 0.05 } else { 0.01 };
            let target = problem.data.get(&idx.to_string()).copied().unwrap_or(0.5);
            if params[idx] < target {
                params[idx] += delta;
            } else {
                params[idx] -= delta;
            }
        }

        (params, best_obj, max_iters)
    }
}

impl Default for HybridSolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_problem(pt: ProblemType, size: usize) -> Problem {
        let mut data = HashMap::new();
        for i in 0..size {
            data.insert(i.to_string(), (i as f64) / (size as f64));
        }
        Problem {
            id: "test".to_string(),
            name: format!("{:?} problem (n={})", pt, size),
            problem_type: pt,
            size,
            data,
        }
    }

    #[test]
    fn test_backend_selection() {
        let solver = HybridSolver::new();

        // Small general problem → classical.
        let p = make_problem(ProblemType::General, 4);
        assert_eq!(solver.select_backend(&p), SolverBackend::Classical);

        // Large search → simulated quantum (if within qubit limit).
        let p = make_problem(ProblemType::Search, 16);
        assert_eq!(solver.select_backend(&p), SolverBackend::SimulatedQuantum);

        // Optimisation → hybrid variational.
        let p = make_problem(ProblemType::Optimisation, 12);
        assert_eq!(solver.select_backend(&p), SolverBackend::HybridVariational);
    }

    #[test]
    fn test_solve_returns_result() {
        let solver = HybridSolver::new();
        let p = make_problem(ProblemType::Optimisation, 8);
        let result = solver.solve(&p).unwrap();
        assert!(!result.solution.is_empty());
        assert!(result.duration_ms < 10_000);
    }
}

//! §10 — Quantum Advantage Benchmarks
//!
//! A benchmark suite that compares classical vs quantum (simulated) solvers
//! across different problem types and sizes, producing a report of where
//! quantum approaches show advantage.

use super::hybrid_solver::{HybridSolver, Problem, ProblemType, SolverBackend};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

// ── Benchmark Result ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub problem_type: String,
    pub problem_size: usize,
    pub classical_ms: u64,
    pub classical_objective: f64,
    pub quantum_ms: u64,
    pub quantum_objective: f64,
    pub speedup_ratio: f64,
    pub quality_ratio: f64,
    pub quantum_advantage: bool,
    pub backend_used: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub results: Vec<BenchmarkResult>,
    pub total_problems: usize,
    pub quantum_advantaged: usize,
    pub average_speedup: f64,
    pub average_quality_improvement: f64,
    pub best_quantum_domain: String,
    pub timestamp: DateTime<Utc>,
}

// ── Benchmark Suite ─────────────────────────────────────────────────────────

pub struct QuantumBenchmarkSuite {
    pub solver: HybridSolver,
    pub problem_sizes: Vec<usize>,
}

impl QuantumBenchmarkSuite {
    pub fn new() -> Self {
        Self {
            solver: HybridSolver::new(),
            problem_sizes: vec![4, 8, 12, 16, 20],
        }
    }

    /// Run the full benchmark suite across all problem types and sizes.
    pub fn run_full_suite(&self) -> BenchmarkReport {
        info!("Running quantum advantage benchmark suite...");

        let problem_types = vec![
            ProblemType::Search,
            ProblemType::Optimisation,
            ProblemType::Sampling,
            ProblemType::Factoring,
            ProblemType::LinearAlgebra,
        ];

        let mut results = Vec::new();

        for pt in &problem_types {
            for &size in &self.problem_sizes {
                if let Some(result) = self.benchmark_problem(pt.clone(), size) {
                    results.push(result);
                }
            }
        }

        let total = results.len();
        let advantaged = results.iter().filter(|r| r.quantum_advantage).count();
        let avg_speedup = if total > 0 {
            results.iter().map(|r| r.speedup_ratio).sum::<f64>() / total as f64
        } else {
            1.0
        };
        let avg_quality = if total > 0 {
            results.iter().map(|r| r.quality_ratio).sum::<f64>() / total as f64
        } else {
            1.0
        };

        // Find the domain with highest average speedup.
        let mut domain_speedups: HashMap<String, Vec<f64>> = HashMap::new();
        for r in &results {
            domain_speedups
                .entry(r.problem_type.clone())
                .or_default()
                .push(r.speedup_ratio);
        }
        let best_domain = domain_speedups
            .iter()
            .map(|(k, v)| (k.clone(), v.iter().sum::<f64>() / v.len() as f64))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, _)| k)
            .unwrap_or_else(|| "none".to_string());

        let report = BenchmarkReport {
            results,
            total_problems: total,
            quantum_advantaged: advantaged,
            average_speedup: avg_speedup,
            average_quality_improvement: avg_quality,
            best_quantum_domain: best_domain,
            timestamp: Utc::now(),
        };

        info!(
            "Quantum benchmark complete: {}/{} problems showed advantage (avg speedup {:.2}x)",
            report.quantum_advantaged, report.total_problems, report.average_speedup
        );

        report
    }

    /// Benchmark a single problem type at a given size.
    fn benchmark_problem(&self, pt: ProblemType, size: usize) -> Option<BenchmarkResult> {
        let problem = self.make_problem(pt.clone(), size);

        // Force classical solve.
        let classical_solver = HybridSolver {
            quantum_advantage_threshold: f64::MAX, // never use quantum
            max_simulated_qubits: 0,
        };
        let classical_result = classical_solver.solve(&problem).ok()?;

        // Normal solve (may pick quantum).
        let hybrid_result = self.solver.solve(&problem).ok()?;

        let speedup = if hybrid_result.duration_ms > 0 {
            classical_result.duration_ms as f64 / hybrid_result.duration_ms as f64
        } else {
            1.0
        };

        let quality = if hybrid_result.objective_value > 1e-12 {
            classical_result.objective_value / hybrid_result.objective_value
        } else if classical_result.objective_value > 1e-12 {
            // Quantum found near-zero objective — that's very good.
            10.0
        } else {
            1.0
        };

        let advantage = speedup > 1.0 || quality > 1.05;

        Some(BenchmarkResult {
            problem_type: format!("{:?}", pt),
            problem_size: size,
            classical_ms: classical_result.duration_ms,
            classical_objective: classical_result.objective_value,
            quantum_ms: hybrid_result.duration_ms,
            quantum_objective: hybrid_result.objective_value,
            speedup_ratio: speedup,
            quality_ratio: quality,
            quantum_advantage: advantage,
            backend_used: format!("{:?}", hybrid_result.backend_used),
        })
    }

    fn make_problem(&self, pt: ProblemType, size: usize) -> Problem {
        let mut data = HashMap::new();
        for i in 0..size {
            // Deterministic pseudo-random targets for reproducibility.
            let val = ((i as f64 * 0.618033988) % 1.0).abs();
            data.insert(i.to_string(), val);
        }
        Problem {
            id: format!("bench_{:?}_{}", pt, size),
            name: format!("{:?} benchmark (n={})", pt, size),
            problem_type: pt,
            size,
            data,
        }
    }
}

impl Default for QuantumBenchmarkSuite {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_benchmark_suite() {
        let suite = QuantumBenchmarkSuite {
            solver: HybridSolver::new(),
            problem_sizes: vec![4, 8],
        };
        let report = suite.run_full_suite();
        assert!(report.total_problems > 0);
        assert!(!report.best_quantum_domain.is_empty());
    }

    #[test]
    fn test_advantage_estimation() {
        let solver = HybridSolver::new();
        let search = Problem {
            id: "s".into(),
            name: "big search".into(),
            problem_type: ProblemType::Search,
            size: 100,
            data: HashMap::new(),
        };
        let advantage = solver.estimate_quantum_advantage(&search);
        assert!(advantage > 1.0, "Search at n=100 should show advantage");
    }
}

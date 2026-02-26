use super::backend::QuantumBackend;
use super::circuit::{Gate, QuantumCircuit};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QAOAConfig {
    pub layers: usize,
    pub shots: u64,
    pub max_iterations: usize,
    pub convergence_threshold: f64,
}

impl Default for QAOAConfig {
    fn default() -> Self {
        Self { layers: 2, shots: 1024, max_iterations: 100, convergence_threshold: 1e-4 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationProblem {
    pub problem_type: ProblemType,
    pub variables: Vec<String>,
    pub objective: HashMap<String, f64>,
    pub constraints: Vec<Constraint>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProblemType {
    MaxCut,
    MaxSat,
    GraphColoring,
    TravelingSalesman,
    GoalScheduling,
    PortfolioOptimization,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub name: String,
    pub variables: Vec<String>,
    pub coefficients: Vec<f64>,
    pub bound: f64,
    pub constraint_type: ConstraintType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConstraintType {
    LessEqual,
    GreaterEqual,
    Equal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    pub best_solution: Vec<bool>,
    pub best_value: f64,
    pub solution_labels: HashMap<String, bool>,
    pub iterations: usize,
    pub converged: bool,
    pub quantum_advantage_estimate: f64,
    pub runtime_ms: u64,
}

pub struct QAOAOptimizer {
    pub backend: Arc<dyn QuantumBackend>,
    pub config: QAOAConfig,
}

impl QAOAOptimizer {
    pub fn new(backend: Arc<dyn QuantumBackend>, config: QAOAConfig) -> Self {
        Self { backend, config }
    }

    pub async fn solve(&self, problem: &OptimizationProblem) -> Result<OptimizationResult> {
        let start = std::time::Instant::now();
        let n = problem.variables.len();
        if n == 0 {
            anyhow::bail!("Problem has no variables");
        }
        if n > self.backend.max_qubits() {
            anyhow::bail!(
                "Problem requires {} qubits but backend supports {}",
                n,
                self.backend.max_qubits()
            );
        }

        let mut best_solution = vec![false; n];
        let mut best_value = f64::NEG_INFINITY;
        let mut converged = false;
        let mut iterations = 0;

        let mut gammas: Vec<f64> = (0..self.config.layers)
            .map(|i| std::f64::consts::PI * (i + 1) as f64 / (self.config.layers + 1) as f64)
            .collect();
        let mut betas: Vec<f64> = (0..self.config.layers)
            .map(|i| std::f64::consts::FRAC_PI_2 * (1.0 - i as f64 / self.config.layers as f64))
            .collect();

        let mut prev_value = f64::NEG_INFINITY;

        for iter in 0..self.config.max_iterations {
            iterations = iter + 1;
            let circuit = self.build_qaoa_circuit(n, &gammas, &betas, problem);
            let result = self.backend.execute_circuit(&circuit).await?;

            let (solution, value) = self.extract_best_solution(&result, &problem.objective, &problem.variables);

            if value > best_value {
                best_value = value;
                best_solution = solution;
            }

            if (value - prev_value).abs() < self.config.convergence_threshold {
                converged = true;
                break;
            }
            prev_value = value;

            for i in 0..self.config.layers {
                gammas[i] += 0.1 * rand::random::<f64>() - 0.05;
                betas[i] += 0.1 * rand::random::<f64>() - 0.05;
                gammas[i] = gammas[i].clamp(0.0, 2.0 * std::f64::consts::PI);
                betas[i] = betas[i].clamp(0.0, std::f64::consts::PI);
            }
        }

        let mut solution_labels = HashMap::new();
        for (i, &val) in best_solution.iter().enumerate() {
            if i < problem.variables.len() {
                solution_labels.insert(problem.variables[i].clone(), val);
            }
        }

        let quantum_advantage_estimate = self.estimate_quantum_advantage(n, problem);

        Ok(OptimizationResult {
            best_solution,
            best_value,
            solution_labels,
            iterations,
            converged,
            quantum_advantage_estimate,
            runtime_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn build_qaoa_circuit(
        &self,
        n: usize,
        gammas: &[f64],
        betas: &[f64],
        problem: &OptimizationProblem,
    ) -> QuantumCircuit {
        let mut circuit = QuantumCircuit::new(n);

        for i in 0..n {
            circuit.add_gate(Gate::h(i));
        }

        for p in 0..self.config.layers {
            self.apply_problem_unitary(&mut circuit, gammas[p], problem);
            self.apply_mixer_unitary(&mut circuit, betas[p], n);
        }

        circuit.measure_all();
        circuit
    }

    fn apply_problem_unitary(
        &self,
        circuit: &mut QuantumCircuit,
        gamma: f64,
        problem: &OptimizationProblem,
    ) {
        match problem.problem_type {
            ProblemType::MaxCut | ProblemType::GoalScheduling => {
                for (var, &coeff) in &problem.objective {
                    if let Some(idx) = problem.variables.iter().position(|v| v == var) {
                        circuit.add_gate(Gate::rz(idx, 2.0 * gamma * coeff));
                    }
                }
                for i in 0..problem.variables.len().saturating_sub(1) {
                    circuit.add_gate(Gate::cnot(i, i + 1));
                    circuit.add_gate(Gate::rz(i + 1, gamma));
                    circuit.add_gate(Gate::cnot(i, i + 1));
                }
            }
            _ => {
                for i in 0..problem.variables.len() {
                    let coeff = problem.objective.get(&problem.variables[i]).copied().unwrap_or(0.0);
                    circuit.add_gate(Gate::rz(i, 2.0 * gamma * coeff));
                }
            }
        }
    }

    fn apply_mixer_unitary(&self, circuit: &mut QuantumCircuit, beta: f64, n: usize) {
        for i in 0..n {
            circuit.add_gate(Gate::rx(i, 2.0 * beta));
        }
    }

    fn extract_best_solution(
        &self,
        result: &super::circuit::MeasurementResult,
        objective: &HashMap<String, f64>,
        variables: &[String],
    ) -> (Vec<bool>, f64) {
        let mut best_solution = vec![false; variables.len()];
        let mut best_value = f64::NEG_INFINITY;

        for (bitstring, &count) in &result.counts {
            if count == 0 {
                continue;
            }
            let bits: Vec<bool> = bitstring.chars().map(|c| c == '1').collect();
            let value: f64 = bits.iter().enumerate().map(|(i, &b)| {
                if b && i < variables.len() {
                    objective.get(&variables[i]).copied().unwrap_or(0.0)
                } else {
                    0.0
                }
            }).sum();

            if value > best_value {
                best_value = value;
                best_solution = bits;
            }
        }

        (best_solution, best_value)
    }

    fn estimate_quantum_advantage(&self, n: usize, problem: &OptimizationProblem) -> f64 {
        let classical_complexity = 2.0_f64.powi(n as i32);
        let qaoa_complexity = (self.config.layers as f64) * (n as f64).powi(2);
        let advantage = classical_complexity / qaoa_complexity.max(1.0);
        match problem.problem_type {
            ProblemType::MaxCut | ProblemType::MaxSat => advantage.min(1000.0),
            ProblemType::GoalScheduling => advantage.min(100.0),
            _ => 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VQEConfig {
    pub ansatz_layers: usize,
    pub max_iterations: usize,
    pub learning_rate: f64,
    pub shots: u64,
    pub convergence_threshold: f64,
}

impl Default for VQEConfig {
    fn default() -> Self {
        Self {
            ansatz_layers: 3,
            max_iterations: 150,
            learning_rate: 0.1,
            shots: 2048,
            convergence_threshold: 1e-5,
        }
    }
}

pub struct VQEOptimizer {
    pub backend: Arc<dyn QuantumBackend>,
    pub config: VQEConfig,
}

impl VQEOptimizer {
    pub fn new(backend: Arc<dyn QuantumBackend>, config: VQEConfig) -> Self {
        Self { backend, config }
    }

    pub async fn optimize_parameters(&self, parameter_count: usize) -> Result<Vec<f64>> {
        let start = std::time::Instant::now();
        let n_params = parameter_count.min(self.backend.max_qubits());
        let mut params: Vec<f64> = (0..n_params)
            .map(|_| rand::random::<f64>() * 2.0 * std::f64::consts::PI)
            .collect();

        let mut prev_energy = f64::INFINITY;

        for iter in 0..self.config.max_iterations {
            let energy = self.estimate_energy(&params).await?;

            if (energy - prev_energy).abs() < self.config.convergence_threshold {
                tracing::debug!("VQE converged after {} iterations, energy={:.6}", iter, energy);
                break;
            }
            prev_energy = energy;

            let gradient = self.parameter_shift_gradient(&params).await?;
            for (p, g) in params.iter_mut().zip(gradient.iter()) {
                *p -= self.config.learning_rate * g;
            }
        }

        tracing::debug!(
            "VQE optimized {} parameters in {}ms",
            n_params,
            start.elapsed().as_millis()
        );
        Ok(params)
    }

    async fn estimate_energy(&self, params: &[f64]) -> Result<f64> {
        let circuit = self.build_ansatz_circuit(params);
        let result = self.backend.execute_circuit(&circuit).await?;
        Ok(result.expectation_value())
    }

    async fn parameter_shift_gradient(&self, params: &[f64]) -> Result<Vec<f64>> {
        let mut gradient = vec![0.0; params.len()];
        let shift = std::f64::consts::FRAC_PI_2;

        for i in 0..params.len() {
            let mut params_plus = params.to_vec();
            let mut params_minus = params.to_vec();
            params_plus[i] += shift;
            params_minus[i] -= shift;

            let e_plus = self.estimate_energy(&params_plus).await?;
            let e_minus = self.estimate_energy(&params_minus).await?;
            gradient[i] = (e_plus - e_minus) / 2.0;
        }

        Ok(gradient)
    }

    fn build_ansatz_circuit(&self, params: &[f64]) -> QuantumCircuit {
        let n = params.len().min(self.backend.max_qubits());
        let mut circuit = QuantumCircuit::new(n);

        for i in 0..n {
            circuit.add_gate(Gate::h(i));
        }

        for layer in 0..self.config.ansatz_layers {
            let offset = layer * n * 2;
            for i in 0..n {
                let idx_ry = offset + i * 2;
                let idx_rz = offset + i * 2 + 1;
                let angle_y = if idx_ry < params.len() { params[idx_ry] } else { 0.0 };
                let angle_z = if idx_rz < params.len() { params[idx_rz] } else { 0.0 };
                circuit.add_gate(Gate::ry(i, angle_y));
                circuit.add_gate(Gate::rz(i, angle_z));
            }
            for i in 0..n.saturating_sub(1) {
                circuit.add_gate(Gate::cnot(i, i + 1));
            }
            if n > 2 {
                circuit.add_gate(Gate::cnot(n - 1, 0));
            }
        }

        circuit.measure_all();
        circuit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantum::backend::SimulatorBackend;

    #[tokio::test]
    async fn test_qaoa_small_problem() {
        let backend = Arc::new(SimulatorBackend::new(4, 512));
        let optimizer = QAOAOptimizer::new(backend, QAOAConfig { layers: 1, shots: 512, max_iterations: 5, convergence_threshold: 1e-3 });

        let mut objective = HashMap::new();
        objective.insert("x0".to_string(), 1.0);
        objective.insert("x1".to_string(), 2.0);
        objective.insert("x2".to_string(), -0.5);

        let problem = OptimizationProblem {
            problem_type: ProblemType::MaxCut,
            variables: vec!["x0".to_string(), "x1".to_string(), "x2".to_string()],
            objective,
            constraints: vec![],
            metadata: HashMap::new(),
        };

        let result = optimizer.solve(&problem).await.unwrap();
        assert_eq!(result.best_solution.len(), 3);
        assert!(result.iterations > 0);
    }

    #[tokio::test]
    async fn test_vqe_optimizer() {
        let backend = Arc::new(SimulatorBackend::new(4, 256));
        let optimizer = VQEOptimizer::new(backend, VQEConfig { ansatz_layers: 1, max_iterations: 3, learning_rate: 0.1, shots: 256, convergence_threshold: 1e-2 });

        let params = optimizer.optimize_parameters(4).await.unwrap();
        assert_eq!(params.len(), 4);
    }
}

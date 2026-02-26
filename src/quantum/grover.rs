use super::backend::QuantumBackend;
use super::circuit::{Gate, MeasurementResult, QuantumCircuit};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroverConfig {
    pub shots: u64,
    pub iterations: Option<usize>,
    pub target_probability: f64,
}

impl Default for GroverConfig {
    fn default() -> Self {
        Self { shots: 1024, iterations: None, target_probability: 0.9 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchProblem {
    pub search_space_size: usize,
    pub oracle_description: String,
    pub target_items: Vec<String>,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroverResult {
    pub found_items: Vec<String>,
    pub measurement: MeasurementResult,
    pub iterations_used: usize,
    pub theoretical_speedup: f64,
    pub success_probability: f64,
    pub runtime_ms: u64,
}

pub struct GroverSearch {
    pub backend: Arc<dyn QuantumBackend>,
    pub config: GroverConfig,
}

impl GroverSearch {
    pub fn new(backend: Arc<dyn QuantumBackend>, config: GroverConfig) -> Self {
        Self { backend, config }
    }

    pub fn optimal_iterations(n_items: usize, n_targets: usize) -> usize {
        if n_targets == 0 || n_items == 0 {
            return 1;
        }
        let ratio = n_targets as f64 / n_items as f64;
        (std::f64::consts::FRAC_PI_4 / ratio.sqrt()).round() as usize
    }

    pub fn qubits_needed(n: usize) -> usize {
        if n <= 1 { 1 } else { (n as f64).log2().ceil() as usize }
    }

    pub async fn search(&self, problem: &SearchProblem) -> Result<GroverResult> {
        let start = std::time::Instant::now();
        let n = problem.items.len();
        if n == 0 {
            anyhow::bail!("Search space is empty");
        }

        let n_qubits = Self::qubits_needed(n);
        if n_qubits > self.backend.max_qubits() {
            anyhow::bail!(
                "Search space requires {} qubits but backend supports {}",
                n_qubits,
                self.backend.max_qubits()
            );
        }

        let n_targets = problem.target_items.len().max(1);
        let iters = self
            .config
            .iterations
            .unwrap_or_else(|| Self::optimal_iterations(n, n_targets));

        let target_indices: Vec<usize> = problem
            .target_items
            .iter()
            .filter_map(|t| problem.items.iter().position(|item| item == t))
            .collect();

        let circuit = self.build_grover_circuit(n_qubits, &target_indices, iters);
        let measurement = self.backend.execute_circuit(&circuit).await?;

        let found_items = self.decode_results(&measurement, &problem.items, n_qubits);

        let theoretical_speedup = (n as f64).sqrt() / n_targets.max(1) as f64;
        let success_probability = measurement
            .counts
            .iter()
            .filter(|(bits, _)| {
                if let Ok(idx) = usize::from_str_radix(bits, 2) {
                    target_indices.contains(&idx)
                } else {
                    false
                }
            })
            .map(|(_, &c)| c as f64)
            .sum::<f64>()
            / measurement.shots as f64;

        Ok(GroverResult {
            found_items,
            measurement,
            iterations_used: iters,
            theoretical_speedup,
            success_probability,
            runtime_ms: start.elapsed().as_millis() as u64,
        })
    }

    pub async fn search_knowledge_base(
        &self,
        query: &str,
        items: &[String],
    ) -> Result<Vec<String>> {
        if items.is_empty() {
            return Ok(vec![]);
        }
        let query_lower = query.to_lowercase();
        let targets: Vec<String> = items
            .iter()
            .filter(|item| item.to_lowercase().contains(&query_lower))
            .cloned()
            .collect();

        if targets.is_empty() {
            return Ok(vec![]);
        }

        let problem = SearchProblem {
            search_space_size: items.len(),
            oracle_description: format!("items containing '{}'", query),
            target_items: targets,
            items: items.to_vec(),
        };

        let result = self.search(&problem).await?;
        Ok(result.found_items)
    }

    pub async fn search_reasoning_branches(
        &self,
        branches: &[String],
        fitness_scores: &HashMap<String, f64>,
    ) -> Result<Vec<String>> {
        if branches.is_empty() {
            return Ok(vec![]);
        }

        let threshold = fitness_scores.values().cloned().fold(f64::NEG_INFINITY, f64::max) * 0.8;
        let targets: Vec<String> = branches
            .iter()
            .filter(|b| fitness_scores.get(*b).copied().unwrap_or(0.0) >= threshold)
            .cloned()
            .collect();

        if targets.is_empty() {
            return Ok(vec![branches[0].clone()]);
        }

        let problem = SearchProblem {
            search_space_size: branches.len(),
            oracle_description: "high-fitness reasoning branches".to_string(),
            target_items: targets,
            items: branches.to_vec(),
        };

        let result = self.search(&problem).await?;
        Ok(result.found_items)
    }

    fn build_grover_circuit(
        &self,
        n_qubits: usize,
        target_indices: &[usize],
        iterations: usize,
    ) -> QuantumCircuit {
        let mut circuit = QuantumCircuit::new(n_qubits + 1);

        // Ancilla qubit in |-> state
        let ancilla = n_qubits;
        circuit.add_gate(Gate::x(ancilla));
        circuit.add_gate(Gate::h(ancilla));

        // Uniform superposition on data qubits
        for i in 0..n_qubits {
            circuit.add_gate(Gate::h(i));
        }

        for _ in 0..iterations {
            // Phase oracle: flip sign of target states
            for &target in target_indices {
                self.add_phase_oracle(&mut circuit, target, n_qubits, ancilla);
            }
            // Diffusion operator
            self.add_diffusion(&mut circuit, n_qubits);
        }

        // Measure data qubits only
        for i in 0..n_qubits {
            circuit.add_measurement(i, i);
        }

        circuit
    }

    fn add_phase_oracle(
        &self,
        circuit: &mut QuantumCircuit,
        target: usize,
        n_qubits: usize,
        ancilla: usize,
    ) {
        // Flip qubits where the target bit is 0
        for bit in 0..n_qubits {
            if (target >> bit) & 1 == 0 {
                circuit.add_gate(Gate::x(bit));
            }
        }

        // Multi-controlled X onto ancilla using cascade of CNOTs
        if n_qubits == 1 {
            circuit.add_gate(Gate::cnot(0, ancilla));
        } else if n_qubits == 2 {
            circuit.add_gate(Gate::toffoli(0, 1, ancilla));
        } else {
            // Approximate: CNOT chain through qubits
            for bit in 0..n_qubits.saturating_sub(1) {
                circuit.add_gate(Gate::cnot(bit, bit + 1));
            }
            circuit.add_gate(Gate::cnot(n_qubits - 1, ancilla));
            for bit in (0..n_qubits.saturating_sub(1)).rev() {
                circuit.add_gate(Gate::cnot(bit, bit + 1));
            }
        }

        // Undo qubit flips
        for bit in 0..n_qubits {
            if (target >> bit) & 1 == 0 {
                circuit.add_gate(Gate::x(bit));
            }
        }
    }

    fn add_diffusion(&self, circuit: &mut QuantumCircuit, n_qubits: usize) {
        for i in 0..n_qubits {
            circuit.add_gate(Gate::h(i));
        }
        for i in 0..n_qubits {
            circuit.add_gate(Gate::x(i));
        }
        // Phase flip on |0...0>
        if n_qubits >= 2 {
            circuit.add_gate(Gate::h(n_qubits - 1));
            circuit.add_gate(Gate::cnot(0, n_qubits - 1));
            circuit.add_gate(Gate::h(n_qubits - 1));
        }
        for i in 0..n_qubits {
            circuit.add_gate(Gate::x(i));
        }
        for i in 0..n_qubits {
            circuit.add_gate(Gate::h(i));
        }
    }

    fn decode_results(
        &self,
        result: &MeasurementResult,
        items: &[String],
        n_qubits: usize,
    ) -> Vec<String> {
        let mut found = Vec::new();
        let top_count = result.counts.values().cloned().fold(0u64, u64::max);
        let threshold = (top_count as f64 * 0.3) as u64;

        for (bitstring, &count) in &result.counts {
            if count < threshold {
                continue;
            }
            let padded = if bitstring.len() < n_qubits {
                format!("{:0>width$}", bitstring, width = n_qubits)
            } else {
                bitstring[bitstring.len() - n_qubits..].to_string()
            };
            if let Ok(idx) = usize::from_str_radix(&padded, 2) {
                if idx < items.len() && !found.contains(&items[idx]) {
                    found.push(items[idx].clone());
                }
            }
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantum::backend::SimulatorBackend;

    #[tokio::test]
    async fn test_grover_single_target() {
        let backend = Arc::new(SimulatorBackend::new(4, 2048));
        let searcher = GroverSearch::new(backend, GroverConfig::default());

        let items: Vec<String> = (0..8).map(|i| format!("item_{}", i)).collect();
        let problem = SearchProblem {
            search_space_size: 8,
            oracle_description: "find item_3".to_string(),
            target_items: vec!["item_3".to_string()],
            items,
        };

        let result = searcher.search(&problem).await.unwrap();
        assert!(result.iterations_used > 0);
        assert!(result.theoretical_speedup > 1.0);
    }

    #[test]
    fn test_optimal_iterations() {
        assert_eq!(GroverSearch::optimal_iterations(4, 1), 1);
        assert_eq!(GroverSearch::optimal_iterations(16, 1), 3);
        assert_eq!(GroverSearch::optimal_iterations(1024, 1), 25);
    }

    #[test]
    fn test_qubits_needed() {
        assert_eq!(GroverSearch::qubits_needed(1), 1);
        assert_eq!(GroverSearch::qubits_needed(4), 2);
        assert_eq!(GroverSearch::qubits_needed(8), 3);
        assert_eq!(GroverSearch::qubits_needed(9), 4);
    }
}

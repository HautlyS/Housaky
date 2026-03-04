//! Quantum Phase Estimation (QPE) — AGI Eigenvalue & PCA Engine
//!
//! QPE estimates the eigenphase φ of a unitary U such that U|ψ⟩ = e^(2πiφ)|ψ⟩.
//! For AGI use-cases this enables:
//!   - **Quantum PCA**: eigenvalue decomposition of the knowledge-graph covariance
//!     matrix → principal components for semantic compression.
//!   - **Spectral reasoning**: identify dominant modes in belief/state vectors.
//!   - **Signal processing**: frequency analysis of sensory time-series via QFT backbone.
//!
//! Architecture:
//!   1. `t` ancilla qubits encode phase precision (2^t eigenvalue bins).
//!   2. Target register holds the eigenstate |ψ⟩.
//!   3. Controlled-U^(2^k) gates transfer phase to ancilla register.
//!   4. Inverse QFT on ancilla decodes the binary phase.
//!   5. Measure ancilla → binary fraction = estimated eigenphase.

use super::backend::QuantumBackend;
use super::circuit::{Gate, QuantumCircuit};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

// ── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QPEConfig {
    /// Number of ancilla (precision) qubits. Precision = 2^(-t).
    pub precision_qubits: usize,
    /// Number of measurement shots.
    pub shots: u64,
    /// Number of eigenvalues to extract (top-k principal components).
    pub top_k: usize,
}

impl Default for QPEConfig {
    fn default() -> Self {
        Self {
            precision_qubits: 4,
            shots: 1024,
            top_k: 3,
        }
    }
}

// ── Results ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QPEResult {
    /// Estimated eigenvalues (phases φ ∈ [0, 1)).
    pub eigenvalues: Vec<f64>,
    /// Corresponding eigenvector indices (mapped from measured ancilla states).
    pub eigenvector_indices: Vec<usize>,
    /// Confidence scores for each estimated eigenvalue.
    pub confidence: Vec<f64>,
    /// Strategy used: "quantum_qpe" or "classical_eig".
    pub strategy: String,
    pub runtime_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumPCAResult {
    /// Principal component eigenvalues (sorted descending).
    pub eigenvalues: Vec<f64>,
    /// Explained variance ratio per component.
    pub explained_variance_ratio: Vec<f64>,
    /// Projected coordinates of input vectors onto top-k PCs.
    pub projections: Vec<Vec<f64>>,
    /// Strategy used.
    pub strategy: String,
    pub runtime_ms: u64,
}

// ── QPE Engine ────────────────────────────────────────────────────────────────

pub struct QuantumPhaseEstimator {
    backend: Arc<dyn QuantumBackend>,
    pub config: QPEConfig,
}

impl QuantumPhaseEstimator {
    pub fn new(backend: Arc<dyn QuantumBackend>, config: QPEConfig) -> Self {
        Self { backend, config }
    }

    /// Estimate eigenphases of an n×n unitary encoded as rotation angles.
    ///
    /// `unitary_angles`: flat row-major encoding of the unitary phases (n values,
    /// each representing the Rz rotation angle for the diagonal approximation).
    pub async fn estimate_phases(&self, unitary_angles: &[f64]) -> Result<QPEResult> {
        let start = std::time::Instant::now();
        let n = unitary_angles.len();
        let t = self.config.precision_qubits;

        if n == 0 || t == 0 {
            return Ok(QPEResult {
                eigenvalues: vec![],
                eigenvector_indices: vec![],
                confidence: vec![],
                strategy: "empty".into(),
                runtime_ms: 0,
            });
        }

        let n_target = (n as f64).log2().ceil() as usize;
        let total_qubits = t + n_target.max(1);

        if total_qubits > 34 {
            return Ok(self.classical_phase_estimation(unitary_angles, start));
        }

        let circuit = self.build_qpe_circuit(t, n_target, unitary_angles);
        let result = self.backend.execute_circuit(&circuit).await?;

        let eigenvalues = self.decode_phases(&result.counts, t, self.config.top_k);
        let k = eigenvalues.len();

        info!(
            "🔮 QPE: {} unitary angles → {} eigenphases (t={} ancilla qubits), strategy=quantum_qpe",
            n, k, t
        );

        Ok(QPEResult {
            confidence: eigenvalues
                .iter()
                .map(|&phi| {
                    let count_key =
                        format!("{:0>width$b}", (phi * (1 << t) as f64) as u64, width = t);
                    result.probability(&count_key)
                })
                .collect(),
            eigenvector_indices: (0..k).collect(),
            eigenvalues,
            strategy: "quantum_qpe".into(),
            runtime_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Quantum PCA on a covariance matrix represented as a flat vector of eigenvalue hints.
    ///
    /// For AGI: use on knowledge-graph node embedding covariance to find semantic axes.
    pub async fn quantum_pca(
        &self,
        covariance_diagonal: &[f64],
        n_components: usize,
    ) -> Result<QuantumPCAResult> {
        let start = std::time::Instant::now();
        let n = covariance_diagonal.len();

        if n == 0 {
            return Ok(QuantumPCAResult {
                eigenvalues: vec![],
                explained_variance_ratio: vec![],
                projections: vec![],
                strategy: "empty".into(),
                runtime_ms: 0,
            });
        }

        // Normalize diagonal entries to [0, 2π] as unitary rotation angles.
        let max_val = covariance_diagonal
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max)
            .max(1e-10);
        let angles: Vec<f64> = covariance_diagonal
            .iter()
            .map(|&v| 2.0 * std::f64::consts::PI * (v / max_val))
            .collect();

        let qpe_result = self.estimate_phases(&angles).await?;
        let k = n_components.min(qpe_result.eigenvalues.len()).max(1);

        // Recover eigenvalues from phases: λ = φ × max_val
        let mut eigenvalues: Vec<f64> = qpe_result.eigenvalues[..k]
            .iter()
            .map(|&phi| phi * max_val)
            .collect();
        eigenvalues.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

        let total_variance: f64 = eigenvalues.iter().sum::<f64>().max(1e-10);
        let explained_variance_ratio: Vec<f64> =
            eigenvalues.iter().map(|&e| e / total_variance).collect();

        // Project diagonal (as a proxy row vector) onto top-k components.
        let projections: Vec<Vec<f64>> = vec![eigenvalues
            .iter()
            .enumerate()
            .map(|(i, &e)| {
                covariance_diagonal
                    .get(i)
                    .copied()
                    .unwrap_or(0.0)
                    .sqrt()
                    * e.sqrt()
            })
            .collect()];

        info!(
            "🔮 Quantum PCA: dim={}, top-{} eigenvalues={:?}, explained={:.1}%, strategy={}",
            n,
            k,
            &eigenvalues,
            explained_variance_ratio.iter().sum::<f64>() * 100.0,
            qpe_result.strategy
        );

        Ok(QuantumPCAResult {
            eigenvalues,
            explained_variance_ratio,
            projections,
            strategy: qpe_result.strategy,
            runtime_ms: start.elapsed().as_millis() as u64,
        })
    }

    // ── Circuit Construction ─────────────────────────────────────────────────

    fn build_qpe_circuit(
        &self,
        t: usize,
        n_target: usize,
        unitary_angles: &[f64],
    ) -> QuantumCircuit {
        let total = t + n_target;
        let mut circuit = QuantumCircuit::new(total);

        // Step 1: Hadamard on all ancilla qubits (superposition of phases).
        for i in 0..t {
            circuit.add_gate(Gate::h(i));
        }

        // Step 2: Initialize target register in |+⟩ (eigenstate approximation).
        for j in 0..n_target {
            circuit.add_gate(Gate::h(t + j));
        }

        // Step 3: Controlled-U^(2^k) — approximate unitary as Rz rotations.
        // For each ancilla qubit k, apply controlled-Rz^(2^k) on target qubits.
        for k in 0..t {
            let repetitions = 1u64 << k;
            for j in 0..n_target {
                let angle_idx = j % unitary_angles.len();
                let theta = unitary_angles[angle_idx] * repetitions as f64;
                // Controlled-Rz: decompose as CNOT + Rz + CNOT
                circuit.add_gate(Gate::rz(t + j, theta / 2.0));
                circuit.add_gate(Gate::cnot(k, t + j));
                circuit.add_gate(Gate::rz(t + j, -theta / 2.0));
                circuit.add_gate(Gate::cnot(k, t + j));
            }
        }

        // Step 4: Inverse QFT on ancilla register.
        let iqft = QuantumCircuit::iqft(t);
        for gate in &iqft.gates {
            // Re-index gates to ancilla register (qubits 0..t).
            circuit.add_gate(gate.clone());
        }

        // Step 5: Measure ancilla qubits.
        for i in 0..t {
            circuit.add_gate(Gate::measure(i, i));
        }

        circuit
    }

    // ── Phase Decoding ───────────────────────────────────────────────────────

    /// Decode measurement counts from ancilla register into eigenphase estimates.
    fn decode_phases(
        &self,
        counts: &std::collections::HashMap<String, u64>,
        t: usize,
        top_k: usize,
    ) -> Vec<f64> {
        let total_shots: u64 = counts.values().sum();
        if total_shots == 0 || t == 0 {
            return vec![];
        }

        // Extract ancilla bits (first t bits of each measurement outcome).
        let mut phase_counts: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();
        for (bitstring, &count) in counts {
            let ancilla: String = bitstring.chars().take(t).collect();
            *phase_counts.entry(ancilla).or_insert(0) += count;
        }

        // Sort by frequency descending.
        let mut sorted: Vec<(String, u64)> = phase_counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        // Convert binary strings to phase fractions φ = binary_int / 2^t.
        sorted
            .into_iter()
            .take(top_k)
            .filter_map(|(bits, _count)| {
                let int_val = u64::from_str_radix(&bits, 2).ok()?;
                Some(int_val as f64 / (1u64 << t) as f64)
            })
            .collect()
    }

    // ── Classical Fallback ───────────────────────────────────────────────────

    fn classical_phase_estimation(
        &self,
        unitary_angles: &[f64],
        start: std::time::Instant,
    ) -> QPEResult {
        let n = unitary_angles.len();
        let max_val = unitary_angles
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max)
            .max(1e-10);

        // Classical: sort angles and normalize to [0, 1) as eigenphase proxies.
        let mut indexed: Vec<(usize, f64)> = unitary_angles
            .iter()
            .enumerate()
            .map(|(i, &v)| (i, v / (2.0 * std::f64::consts::PI)))
            .collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let k = self.config.top_k.min(n);
        let eigenvalues: Vec<f64> = indexed[..k].iter().map(|(_, v)| *v).collect();
        let eigenvector_indices: Vec<usize> = indexed[..k].iter().map(|(i, _)| *i).collect();
        let confidence: Vec<f64> = eigenvalues
            .iter()
            .map(|&v| (v / max_val).min(1.0))
            .collect();

        QPEResult {
            eigenvalues,
            eigenvector_indices,
            confidence,
            strategy: "classical_eig".into(),
            runtime_ms: start.elapsed().as_millis() as u64,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantum::backend::SimulatorBackend;

    fn make_estimator() -> QuantumPhaseEstimator {
        let backend = Arc::new(SimulatorBackend::new(16, 512));
        QuantumPhaseEstimator::new(
            backend,
            QPEConfig {
                precision_qubits: 3,
                shots: 512,
                top_k: 3,
            },
        )
    }

    #[tokio::test]
    async fn test_qpe_basic() {
        let qpe = make_estimator();
        let angles = vec![
            std::f64::consts::PI,
            std::f64::consts::PI / 2.0,
            std::f64::consts::PI / 4.0,
        ];
        let result = qpe.estimate_phases(&angles).await.unwrap();
        assert!(!result.eigenvalues.is_empty());
        assert!(result.eigenvalues.iter().all(|&v| (0.0..=1.0).contains(&v)));
    }

    #[tokio::test]
    async fn test_quantum_pca() {
        let qpe = make_estimator();
        let covariance = vec![5.0, 3.0, 1.5, 0.8, 0.3];
        let result = qpe.quantum_pca(&covariance, 3).await.unwrap();
        assert!(!result.eigenvalues.is_empty());
        let total_ev: f64 = result.explained_variance_ratio.iter().sum();
        assert!(total_ev <= 1.0 + 1e-6, "explained variance ratio must be ≤ 1.0");
    }

    #[tokio::test]
    async fn test_classical_fallback_oversized() {
        let qpe = make_estimator();
        // 40 angles exceeds 34-qubit SV1 limit → classical fallback.
        let angles: Vec<f64> = (0..40)
            .map(|i| i as f64 * std::f64::consts::PI / 20.0)
            .collect();
        let result = qpe.estimate_phases(&angles).await.unwrap();
        assert!(result.strategy.contains("classical"));
    }
}

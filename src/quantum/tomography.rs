//! Quantum State Tomography — state reconstruction & verification.
//!
//! Implements quantum state tomography by measuring circuits in multiple bases
//! (X, Y, Z) and reconstructing the density matrix. Used by the AGI bridge
//! to verify quantum state preparation quality and characterize noise.

use super::backend::QuantumBackend;
use super::circuit::{Gate, MeasurementResult, QuantumCircuit};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

// ── Tomography Config ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomographyConfig {
    /// Number of shots per basis measurement.
    pub shots_per_basis: u64,
    /// Number of qubits to tomograph (limited by 2^n scaling).
    pub max_qubits: usize,
    /// Measurement bases to use.
    pub bases: Vec<MeasurementBasisSet>,
}

impl Default for TomographyConfig {
    fn default() -> Self {
        Self {
            shots_per_basis: 4096,
            max_qubits: 4,
            bases: vec![
                MeasurementBasisSet::Z,
                MeasurementBasisSet::X,
                MeasurementBasisSet::Y,
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MeasurementBasisSet {
    /// Computational basis (standard Z measurement).
    Z,
    /// X basis (apply H before measurement).
    X,
    /// Y basis (apply Sdg then H before measurement).
    Y,
}

// ── Density Matrix ───────────────────────────────────────────────────────────

/// Reconstructed density matrix ρ as a 2D array of complex numbers (re, im).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DensityMatrix {
    /// Dimension = 2^n_qubits.
    pub dim: usize,
    pub n_qubits: usize,
    /// Flattened row-major (re, im) pairs: data[i * dim + j] = (re, im).
    pub data: Vec<(f64, f64)>,
}

impl DensityMatrix {
    pub fn new(n_qubits: usize) -> Self {
        let dim = 1 << n_qubits;
        Self {
            dim,
            n_qubits,
            data: vec![(0.0, 0.0); dim * dim],
        }
    }

    pub fn get(&self, row: usize, col: usize) -> (f64, f64) {
        self.data[row * self.dim + col]
    }

    pub fn set(&mut self, row: usize, col: usize, val: (f64, f64)) {
        self.data[row * self.dim + col] = val;
    }

    /// Trace of the density matrix (should be 1 for a valid state).
    pub fn trace(&self) -> f64 {
        (0..self.dim).map(|i| self.get(i, i).0).sum()
    }

    /// Purity = Tr(ρ²). Pure states have purity = 1.
    pub fn purity(&self) -> f64 {
        let mut result = 0.0;
        for i in 0..self.dim {
            for k in 0..self.dim {
                let (r_ik, i_ik) = self.get(i, k);
                let (r_ki, i_ki) = self.get(k, i);
                // Re(ρ_ik · ρ_ki) = re_ik*re_ki - im_ik*im_ki
                result += r_ik * r_ki - i_ik * i_ki;
            }
        }
        result
    }

    /// Von Neumann entropy: S = -Tr(ρ log₂ ρ).
    /// Approximated from eigenvalues of the diagonal (valid for diagonal-dominant ρ).
    pub fn von_neumann_entropy(&self) -> f64 {
        let mut entropy = 0.0;
        for i in 0..self.dim {
            let p = self.get(i, i).0;
            if p > 1e-15 {
                entropy -= p * p.log2();
            }
        }
        entropy
    }

    /// Fidelity with an ideal pure state |ψ⟩ represented as a statevector.
    /// F = ⟨ψ|ρ|ψ⟩
    pub fn fidelity_with_statevector(&self, statevector: &[(f64, f64)]) -> f64 {
        let mut fidelity = 0.0;
        for i in 0..self.dim {
            for j in 0..self.dim {
                let (rho_re, rho_im) = self.get(i, j);
                let (psi_i_re, psi_i_im) = statevector.get(i).copied().unwrap_or((0.0, 0.0));
                let (psi_j_re, psi_j_im) = statevector.get(j).copied().unwrap_or((0.0, 0.0));
                // ⟨ψ_i|ρ_ij|ψ_j⟩ = conj(ψ_i) · ρ_ij · ψ_j
                let conj_psi_i = (psi_i_re, -psi_i_im);
                // conj(ψ_i) * ρ_ij
                let prod_re = conj_psi_i.0 * rho_re - conj_psi_i.1 * rho_im;
                let prod_im = conj_psi_i.0 * rho_im + conj_psi_i.1 * rho_re;
                // (conj(ψ_i) * ρ_ij) * ψ_j
                fidelity += prod_re * psi_j_re - prod_im * psi_j_im;
            }
        }
        fidelity.max(0.0).min(1.0)
    }
}

// ── Tomography Result ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomographyResult {
    pub density_matrix: DensityMatrix,
    pub n_qubits: usize,
    pub bases_measured: Vec<String>,
    pub total_shots: u64,
    pub purity: f64,
    pub trace: f64,
    pub von_neumann_entropy: f64,
    pub fidelity: Option<f64>,
    pub is_valid_state: bool,
    pub runtime_ms: u64,
}

// ── State Tomographer ────────────────────────────────────────────────────────

pub struct StateTomographer {
    pub backend: Arc<dyn QuantumBackend>,
    pub config: TomographyConfig,
}

impl StateTomographer {
    pub fn new(backend: Arc<dyn QuantumBackend>, config: TomographyConfig) -> Self {
        Self { backend, config }
    }

    /// Perform full state tomography on a circuit (measures in Z, X, Y bases).
    pub async fn tomograph(&self, circuit: &QuantumCircuit) -> Result<TomographyResult> {
        let start = std::time::Instant::now();
        let n = circuit.qubits.min(self.config.max_qubits);

        if n > self.config.max_qubits {
            anyhow::bail!(
                "Tomography limited to {} qubits (circuit has {})",
                self.config.max_qubits, circuit.qubits
            );
        }

        let dim = 1 << n;
        let mut density = DensityMatrix::new(n);
        let mut total_shots = 0u64;
        let mut bases_measured = Vec::new();

        // Measure in each basis.
        for basis in &self.config.bases {
            let basis_circuit = self.prepare_basis_circuit(circuit, basis, n);
            let result = self.backend.execute_circuit(&basis_circuit).await?;
            total_shots += result.shots;

            let basis_name = format!("{:?}", basis);
            bases_measured.push(basis_name.clone());

            // Accumulate density matrix contributions from this basis.
            self.accumulate_basis_result(&mut density, &result, basis, n);

            debug!("Measured {} basis: {} shots, {} distinct outcomes", basis_name, result.shots, result.counts.len());
        }

        // Normalize the density matrix.
        let num_bases = self.config.bases.len() as f64;
        for i in 0..dim {
            for j in 0..dim {
                let (re, im) = density.get(i, j);
                density.set(i, j, (re / num_bases, im / num_bases));
            }
        }

        let purity = density.purity();
        let trace = density.trace();
        let entropy = density.von_neumann_entropy();
        let is_valid = (trace - 1.0).abs() < 0.1 && purity >= 0.0 && purity <= 1.05;

        info!(
            "Tomography complete: {n} qubits, purity={purity:.4}, trace={trace:.4}, entropy={entropy:.4}"
        );

        Ok(TomographyResult {
            density_matrix: density,
            n_qubits: n,
            bases_measured,
            total_shots,
            purity,
            trace,
            von_neumann_entropy: entropy,
            fidelity: None,
            is_valid_state: is_valid,
            runtime_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Perform tomography and compute fidelity against an ideal statevector.
    pub async fn tomograph_with_fidelity(
        &self,
        circuit: &QuantumCircuit,
        ideal_statevector: &[(f64, f64)],
    ) -> Result<TomographyResult> {
        let mut result = self.tomograph(circuit).await?;
        let fidelity = result.density_matrix.fidelity_with_statevector(ideal_statevector);
        result.fidelity = Some(fidelity);
        info!("State fidelity: {:.6}", fidelity);
        Ok(result)
    }

    /// Prepare a circuit with basis-change gates before measurement.
    fn prepare_basis_circuit(
        &self,
        original: &QuantumCircuit,
        basis: &MeasurementBasisSet,
        n_qubits: usize,
    ) -> QuantumCircuit {
        let mut circuit = QuantumCircuit::new(original.qubits);
        circuit.classical_bits = original.classical_bits;

        // Copy all non-measurement gates from the original circuit.
        for gate in &original.gates {
            match gate.gate_type {
                super::circuit::GateType::Measure => {}
                _ => { circuit.add_gate(gate.clone()); }
            }
        }

        // Add basis-change gates before measurement.
        match basis {
            MeasurementBasisSet::Z => {
                // No change needed for Z basis.
            }
            MeasurementBasisSet::X => {
                // X basis: apply H to rotate Z → X.
                for i in 0..n_qubits {
                    circuit.add_gate(Gate::h(i));
                }
            }
            MeasurementBasisSet::Y => {
                // Y basis: apply Sdg then H.
                for i in 0..n_qubits {
                    circuit.add_gate(Gate {
                        gate_type: super::circuit::GateType::Sdg,
                        qubits: vec![i],
                        classical_bits: vec![],
                    });
                    circuit.add_gate(Gate::h(i));
                }
            }
        }

        // Add measurements.
        circuit.measure_all();
        circuit
    }

    /// Accumulate density matrix contributions from measurement results in a given basis.
    fn accumulate_basis_result(
        &self,
        density: &mut DensityMatrix,
        result: &MeasurementResult,
        basis: &MeasurementBasisSet,
        n_qubits: usize,
    ) {
        let dim = 1 << n_qubits;

        match basis {
            MeasurementBasisSet::Z => {
                // Z-basis: diagonal elements = probabilities.
                for (bitstring, &count) in &result.counts {
                    if let Ok(idx) = usize::from_str_radix(bitstring, 2) {
                        if idx < dim {
                            let prob = count as f64 / result.shots as f64;
                            let (re, im) = density.get(idx, idx);
                            density.set(idx, idx, (re + prob, im));
                        }
                    }
                }
            }
            MeasurementBasisSet::X | MeasurementBasisSet::Y => {
                // X/Y basis: contribute to off-diagonal elements.
                // For single-qubit: ⟨X⟩ = p(0) - p(1), contributes to ρ_01 real part.
                // For multi-qubit: we extract Pauli expectation values.
                for (bitstring, &count) in &result.counts {
                    if let Ok(idx) = usize::from_str_radix(bitstring, 2) {
                        if idx < dim {
                            let prob = count as f64 / result.shots as f64;
                            // Parity-based contribution to off-diagonal elements.
                            let parity: u32 = bitstring.chars().map(|c| if c == '1' { 1u32 } else { 0 }).sum();
                            let sign = if parity % 2 == 0 { 1.0 } else { -1.0 };

                            // Distribute the X/Y expectation across relevant density matrix elements.
                            for i in 0..dim {
                                let j = i ^ idx; // bitwise complement gives the off-diagonal partner
                                if j < dim && i != j {
                                    let (re, im) = density.get(i, j);
                                    match basis {
                                        MeasurementBasisSet::X => {
                                            density.set(i, j, (re + sign * prob / dim as f64, im));
                                        }
                                        MeasurementBasisSet::Y => {
                                            density.set(i, j, (re, im + sign * prob / dim as f64));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantum::backend::SimulatorBackend;
    use crate::quantum::circuit::{Gate, QuantumCircuit};

    #[test]
    fn test_density_matrix_pure_state() {
        // |0⟩ state: ρ = |0⟩⟨0| = [[1,0],[0,0]]
        let mut rho = DensityMatrix::new(1);
        rho.set(0, 0, (1.0, 0.0));
        assert!((rho.trace() - 1.0).abs() < 1e-9);
        assert!((rho.purity() - 1.0).abs() < 1e-9);
        assert!((rho.von_neumann_entropy() - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_density_matrix_mixed_state() {
        // Maximally mixed: ρ = I/2 = [[0.5,0],[0,0.5]]
        let mut rho = DensityMatrix::new(1);
        rho.set(0, 0, (0.5, 0.0));
        rho.set(1, 1, (0.5, 0.0));
        assert!((rho.trace() - 1.0).abs() < 1e-9);
        assert!((rho.purity() - 0.5).abs() < 1e-9);
        assert!((rho.von_neumann_entropy() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_fidelity_with_basis_state() {
        // ρ = |0⟩⟨0|, |ψ⟩ = |0⟩ → F = 1.0
        let mut rho = DensityMatrix::new(1);
        rho.set(0, 0, (1.0, 0.0));
        let psi = vec![(1.0, 0.0), (0.0, 0.0)];
        let f = rho.fidelity_with_statevector(&psi);
        assert!((f - 1.0).abs() < 1e-9);

        // ρ = |0⟩⟨0|, |ψ⟩ = |1⟩ → F = 0.0
        let psi_1 = vec![(0.0, 0.0), (1.0, 0.0)];
        let f1 = rho.fidelity_with_statevector(&psi_1);
        assert!(f1.abs() < 1e-9);
    }

    #[tokio::test]
    async fn test_tomography_single_qubit() {
        let backend = Arc::new(SimulatorBackend::new(1, 4096));
        let tomographer = StateTomographer::new(backend, TomographyConfig::default());

        // Tomograph |0⟩ state.
        let mut c = QuantumCircuit::new(1);
        c.measure_all();

        let result = tomographer.tomograph(&c).await.unwrap();
        assert_eq!(result.n_qubits, 1);
        assert!(result.is_valid_state);
        // Pure |0⟩ should have purity ≈ 1.
        assert!(result.purity > 0.8, "purity={}", result.purity);
    }

    #[tokio::test]
    async fn test_tomography_bell_state() {
        let backend = Arc::new(SimulatorBackend::new(2, 8192));
        let tomographer = StateTomographer::new(backend, TomographyConfig {
            max_qubits: 2,
            ..Default::default()
        });

        let mut c = QuantumCircuit::new(2);
        c.add_gate(Gate::h(0));
        c.add_gate(Gate::cnot(0, 1));

        let result = tomographer.tomograph(&c).await.unwrap();
        assert_eq!(result.n_qubits, 2);
        assert!(result.total_shots > 0);
        assert_eq!(result.bases_measured.len(), 3);
    }
}

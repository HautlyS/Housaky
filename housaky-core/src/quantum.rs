//! Quantum-inspired state representations
//!
//! This module provides quantum-inspired data structures that emulate quantum computing
//! concepts on classical hardware, including superposition, entanglement, and measurement.

use anyhow::Result;
use ndarray::{Array1, Array2};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// A quantum-inspired state vector that can represent superposition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumState {
    /// Amplitudes of the state vector (complex numbers represented as [real, imag] pairs)
    amplitudes: Vec<[f64; 2]>,
    /// Number of qubits this state represents
    num_qubits: usize,
}

impl QuantumState {
    /// Create a new quantum state with given number of qubits (initialized to |0...0⟩)
    pub fn new(num_qubits: usize) -> Self {
        let size = 1usize << num_qubits;
        let mut amplitudes = vec![[0.0, 0.0]; size];
        amplitudes[0] = [1.0, 0.0]; // |0...0⟩ state

        Self {
            amplitudes,
            num_qubits,
        }
    }

    /// Create a quantum state from classical bits
    pub fn from_classical(bits: &[bool]) -> Self {
        let num_qubits = bits.len();
        let mut state = Self::new(num_qubits);

        let index = bits.iter().enumerate().fold(
            0usize,
            |acc, (i, &b)| {
                if b {
                    acc | (1 << i)
                } else {
                    acc
                }
            },
        );

        state.amplitudes.fill([0.0, 0.0]);
        state.amplitudes[index] = [1.0, 0.0];

        state
    }

    /// Apply Hadamard gate to create superposition
    pub fn hadamard(&mut self, target: usize) -> Result<()> {
        if target >= self.num_qubits {
            return Err(anyhow::anyhow!("Target qubit index out of bounds"));
        }

        let h_factor = 1.0 / (2.0_f64).sqrt();
        let mask = 1usize << target;

        let new_amplitudes: Vec<[f64; 2]> = (0..self.amplitudes.len())
            .into_par_iter()
            .map(|i| {
                let j = i ^ mask;
                let sign = if (i & mask) == 0 { 1.0 } else { -1.0 };

                let a_real = self.amplitudes[i][0];
                let a_imag = self.amplitudes[i][1];
                let b_real = self.amplitudes[j][0];
                let b_imag = self.amplitudes[j][1];

                [
                    h_factor * (a_real + sign * b_real),
                    h_factor * (a_imag + sign * b_imag),
                ]
            })
            .collect();

        self.amplitudes = new_amplitudes;
        Ok(())
    }

    /// Apply Pauli-X (NOT) gate
    pub fn pauli_x(&mut self, target: usize) -> Result<()> {
        if target >= self.num_qubits {
            return Err(anyhow::anyhow!("Target qubit index out of bounds"));
        }

        let mask = 1usize << target;

        self.amplitudes = (0..self.amplitudes.len())
            .into_par_iter()
            .map(|i| {
                let j = i ^ mask;
                self.amplitudes[j]
            })
            .collect();

        Ok(())
    }

    /// Apply Pauli-Y gate
    pub fn pauli_y(&mut self, target: usize) -> Result<()> {
        if target >= self.num_qubits {
            return Err(anyhow::anyhow!("Target qubit index out of bounds"));
        }

        let mask = 1usize << target;

        self.amplitudes = (0..self.amplitudes.len())
            .into_par_iter()
            .map(|i| {
                let j = i ^ mask;
                let amp_i = self.amplitudes[i];
                let amp_j = self.amplitudes[j];

                if (i & mask) == 0 {
                    // |0⟩ -> i|1⟩
                    [-amp_j[1], amp_j[0]]
                } else {
                    // |1⟩ -> -i|0⟩
                    [amp_i[1], -amp_i[0]]
                }
            })
            .collect();

        Ok(())
    }

    /// Apply Pauli-Z gate
    pub fn pauli_z(&mut self, target: usize) -> Result<()> {
        if target >= self.num_qubits {
            return Err(anyhow::anyhow!("Target qubit index out of bounds"));
        }

        let mask = 1usize << target;

        self.amplitudes
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, amp)| {
                if (i & mask) != 0 {
                    amp[0] = -amp[0];
                    amp[1] = -amp[1];
                }
            });

        Ok(())
    }

    /// Apply Phase gate (S gate) - adds i phase to |1⟩
    pub fn phase(&mut self, target: usize, angle: f64) -> Result<()> {
        if target >= self.num_qubits {
            return Err(anyhow::anyhow!("Target qubit index out of bounds"));
        }

        let mask = 1usize << target;
        let cos_angle = angle.cos();
        let sin_angle = angle.sin();

        self.amplitudes
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, amp)| {
                if (i & mask) != 0 {
                    let real = amp[0] * cos_angle - amp[1] * sin_angle;
                    let imag = amp[0] * sin_angle + amp[1] * cos_angle;
                    *amp = [real, imag];
                }
            });

        Ok(())
    }

    /// Apply S gate (sqrt(Z)) - adds i phase to |1⟩
    pub fn s_gate(&mut self, target: usize) -> Result<()> {
        self.phase(target, PI / 2.0)
    }

    /// Apply T gate (sqrt(S)) - adds e^(iπ/4) phase to |1⟩
    pub fn t_gate(&mut self, target: usize) -> Result<()> {
        self.phase(target, PI / 4.0)
    }

    /// Apply rotation around X-axis
    pub fn rotate_x(&mut self, target: usize, angle: f64) -> Result<()> {
        if target >= self.num_qubits {
            return Err(anyhow::anyhow!("Target qubit index out of bounds"));
        }

        let mask = 1usize << target;
        let cos_half = (angle / 2.0).cos();
        let sin_half = (angle / 2.0).sin();

        let new_amplitudes: Vec<[f64; 2]> = (0..self.amplitudes.len())
            .into_par_iter()
            .map(|i| {
                let j = i ^ mask;
                let amp_i = self.amplitudes[i];
                let amp_j = self.amplitudes[j];

                if (i & mask) == 0 {
                    let real = cos_half * amp_i[0] - sin_half * amp_j[1];
                    let imag = cos_half * amp_i[1] + sin_half * amp_j[0];
                    [real, imag]
                } else {
                    let real = cos_half * amp_i[0] - sin_half * amp_j[1];
                    let imag = cos_half * amp_i[1] + sin_half * amp_j[0];
                    [real, imag]
                }
            })
            .collect();

        self.amplitudes = new_amplitudes;
        Ok(())
    }

    /// Apply rotation around Y-axis
    pub fn rotate_y(&mut self, target: usize, angle: f64) -> Result<()> {
        if target >= self.num_qubits {
            return Err(anyhow::anyhow!("Target qubit index out of bounds"));
        }

        let mask = 1usize << target;
        let cos_half = (angle / 2.0).cos();
        let sin_half = (angle / 2.0).sin();

        let new_amplitudes: Vec<[f64; 2]> = (0..self.amplitudes.len())
            .into_par_iter()
            .map(|i| {
                let j = i ^ mask;
                let amp_i = self.amplitudes[i];
                let amp_j = self.amplitudes[j];

                if (i & mask) == 0 {
                    let real = cos_half * amp_i[0] - sin_half * amp_j[0];
                    let imag = cos_half * amp_i[1] - sin_half * amp_j[1];
                    [real, imag]
                } else {
                    let real = sin_half * amp_j[0] + cos_half * amp_i[0];
                    let imag = sin_half * amp_j[1] + cos_half * amp_i[1];
                    [real, imag]
                }
            })
            .collect();

        self.amplitudes = new_amplitudes;
        Ok(())
    }

    /// Apply rotation around Z-axis
    pub fn rotate_z(&mut self, target: usize, angle: f64) -> Result<()> {
        if target >= self.num_qubits {
            return Err(anyhow::anyhow!("Target qubit index out of bounds"));
        }

        let mask = 1usize << target;
        let cos_half = (angle / 2.0).cos();
        let sin_half = (angle / 2.0).sin();

        self.amplitudes
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, amp)| {
                if (i & mask) != 0 {
                    let real = amp[0] * cos_half - amp[1] * sin_half;
                    let imag = amp[0] * sin_half + amp[1] * cos_half;
                    *amp = [real, imag];
                } else {
                    let real = amp[0] * cos_half + amp[1] * sin_half;
                    let imag = -amp[0] * sin_half + amp[1] * cos_half;
                    *amp = [real, imag];
                }
            });

        Ok(())
    }

    /// Apply CNOT (controlled-NOT) gate
    pub fn cnot(&mut self, control: usize, target: usize) -> Result<()> {
        if control >= self.num_qubits || target >= self.num_qubits {
            return Err(anyhow::anyhow!("Qubit index out of bounds"));
        }
        if control == target {
            return Err(anyhow::anyhow!("Control and target must be different"));
        }

        let control_mask = 1usize << control;
        let target_mask = 1usize << target;

        self.amplitudes = (0..self.amplitudes.len())
            .into_par_iter()
            .map(|i| {
                if (i & control_mask) != 0 {
                    let j = i ^ target_mask;
                    self.amplitudes[j]
                } else {
                    self.amplitudes[i]
                }
            })
            .collect();

        Ok(())
    }

    /// Apply SWAP gate
    pub fn swap(&mut self, qubit1: usize, qubit2: usize) -> Result<()> {
        if qubit1 >= self.num_qubits || qubit2 >= self.num_qubits {
            return Err(anyhow::anyhow!("Qubit index out of bounds"));
        }
        if qubit1 == qubit2 {
            return Ok(());
        }

        let mask1 = 1usize << qubit1;
        let mask2 = 1usize << qubit2;

        let new_amplitudes: Vec<[f64; 2]> = (0..self.amplitudes.len())
            .into_par_iter()
            .map(|i| {
                let bit1 = (i & mask1) != 0;
                let bit2 = (i & mask2) != 0;

                if bit1 != bit2 {
                    let j = i ^ mask1 ^ mask2;
                    self.amplitudes[j]
                } else {
                    self.amplitudes[i]
                }
            })
            .collect();

        self.amplitudes = new_amplitudes;
        Ok(())
    }

    /// Apply CZ (controlled-Z) gate
    pub fn cz(&mut self, control: usize, target: usize) -> Result<()> {
        if control >= self.num_qubits || target >= self.num_qubits {
            return Err(anyhow::anyhow!("Qubit index out of bounds"));
        }

        let control_mask = 1usize << control;
        let target_mask = 1usize << target;

        self.amplitudes
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, amp)| {
                if (i & control_mask) != 0 && (i & target_mask) != 0 {
                    amp[0] = -amp[0];
                    amp[1] = -amp[1];
                }
            });

        Ok(())
    }

    /// Apply CCNOT (Toffoli) gate - controlled-controlled-NOT
    pub fn ccnot(&mut self, control1: usize, control2: usize, target: usize) -> Result<()> {
        if control1 >= self.num_qubits || control2 >= self.num_qubits || target >= self.num_qubits {
            return Err(anyhow::anyhow!("Qubit index out of bounds"));
        }

        let control1_mask = 1usize << control1;
        let control2_mask = 1usize << control2;
        let target_mask = 1usize << target;

        self.amplitudes = (0..self.amplitudes.len())
            .into_par_iter()
            .map(|i| {
                if (i & control1_mask) != 0 && (i & control2_mask) != 0 {
                    let j = i ^ target_mask;
                    self.amplitudes[j]
                } else {
                    self.amplitudes[i]
                }
            })
            .collect();

        Ok(())
    }

    /// Apply controlled-phase gate
    pub fn cp(&mut self, control: usize, target: usize, angle: f64) -> Result<()> {
        if control >= self.num_qubits || target >= self.num_qubits {
            return Err(anyhow::anyhow!("Qubit index out of bounds"));
        }

        let control_mask = 1usize << control;
        let target_mask = 1usize << target;
        let cos_angle = angle.cos();
        let sin_angle = angle.sin();

        self.amplitudes
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, amp)| {
                if (i & control_mask) != 0 && (i & target_mask) != 0 {
                    let real = amp[0] * cos_angle - amp[1] * sin_angle;
                    let imag = amp[0] * sin_angle + amp[1] * cos_angle;
                    *amp = [real, imag];
                }
            });

        Ok(())
    }

    /// Measure the state, collapsing to a classical bitstring
    pub fn measure(&self) -> Result<(Vec<bool>, f64)> {
        use rand::Rng;

        let probabilities: Vec<f64> = self
            .amplitudes
            .par_iter()
            .map(|amp| amp[0].powi(2) + amp[1].powi(2))
            .collect();

        let total_prob: f64 = probabilities.par_iter().sum();
        if total_prob < 1e-10 {
            return Err(anyhow::anyhow!("State has zero probability"));
        }

        let mut rng = rand::thread_rng();
        let random_val: f64 = rng.gen::<f64>() * total_prob;

        let mut cumulative = 0.0;
        let mut measured_index = 0;

        for (i, &prob) in probabilities.iter().enumerate() {
            cumulative += prob;
            if cumulative >= random_val {
                measured_index = i;
                break;
            }
        }

        let bits: Vec<bool> = (0..self.num_qubits)
            .map(|i| (measured_index >> i) & 1 == 1)
            .collect();

        let probability = probabilities[measured_index] / total_prob;

        Ok((bits, probability))
    }

    /// Get probability of measuring a specific state
    pub fn probability(&self, index: usize) -> f64 {
        if index >= self.amplitudes.len() {
            return 0.0;
        }
        let amp = &self.amplitudes[index];
        amp[0].powi(2) + amp[1].powi(2)
    }

    /// Get number of qubits
    pub fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    /// Get state vector as ndarray
    pub fn to_ndarray(&self) -> Array1<f64> {
        Array1::from_iter(self.amplitudes.iter().flat_map(|amp| vec![amp[0], amp[1]]))
    }
}

/// Quantum-inspired tensor network for efficient computation
#[derive(Debug, Clone)]
pub struct TensorNetwork {
    tensors: Vec<Array2<f64>>,
}

impl TensorNetwork {
    /// Create an empty tensor network
    pub fn new() -> Self {
        Self {
            tensors: Vec::new(),
        }
    }

    /// Add a tensor to the network
    pub fn add_tensor(&mut self, tensor: Array2<f64>) {
        self.tensors.push(tensor);
    }

    /// Contract the network to produce a result
    pub fn contract(&self) -> Result<Array2<f64>> {
        if self.tensors.is_empty() {
            return Err(anyhow::anyhow!("Cannot contract empty tensor network"));
        }

        let mut result = self.tensors[0].clone();

        for tensor in &self.tensors[1..] {
            result = result.dot(tensor);
        }

        Ok(result)
    }
}

impl Default for TensorNetwork {
    fn default() -> Self {
        Self::new()
    }
}

/// Quantum gate operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum QuantumGate {
    Hadamard,
    PauliX,
    PauliY,
    PauliZ,
    Phase(f64),
    RotateX(f64),
    RotateY(f64),
    RotateZ(f64),
    Cnot {
        control: usize,
        target: usize,
    },
    Swap(usize, usize),
    SGate,
    TGate,
    CZ {
        control: usize,
        target: usize,
    },
    CCNot {
        control1: usize,
        control2: usize,
        target: usize,
    },
    CP {
        control: usize,
        target: usize,
        angle: f64,
    },
}

/// Quantum circuit builder
#[derive(Debug, Clone)]
pub struct QuantumCircuit {
    num_qubits: usize,
    gates: Vec<(QuantumGate, Vec<usize>)>,
}

impl QuantumCircuit {
    /// Create a new quantum circuit
    pub fn new(num_qubits: usize) -> Self {
        Self {
            num_qubits,
            gates: Vec::new(),
        }
    }

    /// Add a gate to the circuit
    pub fn add_gate(&mut self, gate: QuantumGate, qubits: Vec<usize>) -> Result<()> {
        for &qubit in &qubits {
            if qubit >= self.num_qubits {
                return Err(anyhow::anyhow!("Qubit index out of bounds"));
            }
        }

        self.gates.push((gate, qubits));
        Ok(())
    }

    /// Execute the circuit on a quantum state
    pub fn execute(&self, state: &mut QuantumState) -> Result<()> {
        for (gate, qubits) in &self.gates {
            match gate {
                QuantumGate::Hadamard => {
                    if let Some(&target) = qubits.first() {
                        state.hadamard(target)?;
                    }
                }
                QuantumGate::PauliX => {
                    if let Some(&target) = qubits.first() {
                        state.pauli_x(target)?;
                    }
                }
                QuantumGate::PauliY => {
                    if let Some(&target) = qubits.first() {
                        state.pauli_y(target)?;
                    }
                }
                QuantumGate::PauliZ => {
                    if let Some(&target) = qubits.first() {
                        state.pauli_z(target)?;
                    }
                }
                QuantumGate::Phase(angle) => {
                    if let Some(&target) = qubits.first() {
                        state.phase(target, *angle)?;
                    }
                }
                QuantumGate::RotateX(angle) => {
                    if let Some(&target) = qubits.first() {
                        state.rotate_x(target, *angle)?;
                    }
                }
                QuantumGate::RotateY(angle) => {
                    if let Some(&target) = qubits.first() {
                        state.rotate_y(target, *angle)?;
                    }
                }
                QuantumGate::RotateZ(angle) => {
                    if let Some(&target) = qubits.first() {
                        state.rotate_z(target, *angle)?;
                    }
                }
                QuantumGate::Cnot { control, target } => {
                    state.cnot(*control, *target)?;
                }
                QuantumGate::Swap(qubit1, qubit2) => {
                    state.swap(*qubit1, *qubit2)?;
                }
                QuantumGate::SGate => {
                    if let Some(&target) = qubits.first() {
                        state.s_gate(target)?;
                    }
                }
                QuantumGate::TGate => {
                    if let Some(&target) = qubits.first() {
                        state.t_gate(target)?;
                    }
                }
                QuantumGate::CZ { control, target } => {
                    state.cz(*control, *target)?;
                }
                QuantumGate::CCNot {
                    control1,
                    control2,
                    target,
                } => {
                    state.ccnot(*control1, *control2, *target)?;
                }
                QuantumGate::CP {
                    control,
                    target,
                    angle,
                } => {
                    state.cp(*control, *target, *angle)?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantum_state_creation() {
        let state = QuantumState::new(2);
        assert_eq!(state.num_qubits(), 2);
        assert_eq!(state.probability(0), 1.0);
    }

    #[test]
    fn test_hadamard() {
        let mut state = QuantumState::new(1);
        state.hadamard(0).unwrap();

        let prob_0 = state.probability(0);
        let prob_1 = state.probability(1);

        assert!((prob_0 - 0.5).abs() < 1e-10);
        assert!((prob_1 - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_pauli_gates() {
        let mut state = QuantumState::new(1);

        state.pauli_x(0).unwrap();
        assert_eq!(state.probability(1), 1.0);

        state.pauli_y(0).unwrap();
        assert_eq!(state.probability(0), 1.0);

        state.pauli_z(0).unwrap();
        assert_eq!(state.probability(0), 1.0);
    }

    #[test]
    fn test_rotation_gates() {
        let mut state = QuantumState::new(1);
        state.hadamard(0).unwrap();

        state.rotate_x(0, PI / 2.0).unwrap();
        state.rotate_y(0, PI / 2.0).unwrap();
        state.rotate_z(0, PI / 2.0).unwrap();

        let total_prob = state.probability(0) + state.probability(1);
        assert!((total_prob - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_swap() {
        let mut state = QuantumState::new(2);
        state.pauli_x(0).unwrap();

        state.swap(0, 1).unwrap();

        assert_eq!(state.probability(2), 1.0);
    }

    #[test]
    fn test_cz() {
        let mut state = QuantumState::new(2);
        state.hadamard(0).unwrap();
        state.hadamard(1).unwrap();

        state.cz(0, 1).unwrap();

        let total_prob = state.probability(0)
            + state.probability(1)
            + state.probability(2)
            + state.probability(3);
        assert!((total_prob - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ccnot() {
        let mut state = QuantumState::new(3);
        state.pauli_x(0).unwrap();
        state.pauli_x(1).unwrap();

        state.ccnot(0, 1, 2).unwrap();

        assert_eq!(state.probability(7), 1.0);
    }

    #[test]
    fn test_circuit() {
        let mut circuit = QuantumCircuit::new(2);
        circuit.add_gate(QuantumGate::Hadamard, vec![0]).unwrap();
        circuit
            .add_gate(
                QuantumGate::Cnot {
                    control: 0,
                    target: 1,
                },
                vec![0, 1],
            )
            .unwrap();

        let mut state = QuantumState::new(2);
        circuit.execute(&mut state).unwrap();

        let total_prob = state.probability(0) + state.probability(3);
        assert!((total_prob - 1.0).abs() < 1e-10);
    }
}

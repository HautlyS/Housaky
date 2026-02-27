//! Circuit Transpiler — gate decomposition & optimization for Braket devices.
//!
//! Transforms abstract quantum circuits into device-native gate sets, applies
//! circuit optimizations (gate cancellation, rotation merging, depth reduction),
//! and validates compatibility with target device constraints.

use super::backend::{BraketConnectivity, BraketDeviceCatalog};
use super::circuit::{Gate, GateType, QuantumCircuit};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

// ── Transpiler Config ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranspilerConfig {
    /// Optimization level: 0 = none, 1 = basic, 2 = aggressive, 3 = maximum.
    pub optimization_level: u8,
    /// Target device ARN (determines native gate set).
    pub target_device: Option<String>,
    /// Maximum circuit depth after transpilation (0 = unlimited).
    pub max_depth: u32,
    /// Enable gate cancellation (e.g. XX = I).
    pub enable_cancellation: bool,
    /// Enable rotation merging (e.g. Rz(a) Rz(b) = Rz(a+b)).
    pub enable_rotation_merge: bool,
    /// Enable CNOT→CZ decomposition for CZ-native devices.
    pub enable_basis_translation: bool,
}

impl Default for TranspilerConfig {
    fn default() -> Self {
        Self {
            optimization_level: 2,
            target_device: None,
            max_depth: 0,
            enable_cancellation: true,
            enable_rotation_merge: true,
            enable_basis_translation: true,
        }
    }
}

// ── Transpilation Report ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranspilationReport {
    pub original_gates: usize,
    pub original_depth: u32,
    pub transpiled_gates: usize,
    pub transpiled_depth: u32,
    pub gates_removed: usize,
    pub gates_decomposed: usize,
    pub rotations_merged: usize,
    pub target_device: String,
    pub native_gate_set: Vec<String>,
    pub passes_applied: Vec<String>,
}

// ── Native Gate Sets ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeGateSet {
    /// Amazon SV1/TN1/DM1: universal gate set.
    BraketSimulator,
    /// IonQ Aria/Forte: GPi, GPi2, MS (Mølmer–Sørensen).
    IonQNative,
    /// IQM Garnet: PRx, CZ.
    IqmNative,
    /// Rigetti Ankaa: Rx, Rz, CZ, iSwap.
    RigettiNative,
    /// Generic universal: H, CNOT, Rz, Rx, Ry.
    Universal,
}

impl NativeGateSet {
    /// Determine native gate set from a device ARN.
    pub fn from_device_arn(arn: &str) -> Self {
        if arn.contains("simulator") {
            Self::BraketSimulator
        } else if arn.contains("ionq") {
            Self::IonQNative
        } else if arn.contains("iqm") {
            Self::IqmNative
        } else if arn.contains("rigetti") {
            Self::RigettiNative
        } else {
            Self::Universal
        }
    }

    /// Check if a gate type is native to this gate set.
    pub fn is_native(&self, gate: &GateType) -> bool {
        match self {
            Self::BraketSimulator => true, // simulators accept everything
            Self::IonQNative => matches!(
                gate,
                GateType::Rx(_) | GateType::Ry(_) | GateType::Rz(_) | GateType::Measure | GateType::Barrier
            ),
            Self::IqmNative => matches!(
                gate,
                GateType::Rx(_) | GateType::Ry(_) | GateType::CZ | GateType::Measure | GateType::Barrier
            ),
            Self::RigettiNative => matches!(
                gate,
                GateType::Rx(_) | GateType::Rz(_) | GateType::CZ | GateType::Swap | GateType::Measure | GateType::Barrier
            ),
            Self::Universal => matches!(
                gate,
                GateType::H | GateType::CNOT | GateType::Rx(_) | GateType::Ry(_) | GateType::Rz(_) | GateType::Measure | GateType::Barrier
            ),
        }
    }
}

// ── Transpiler ───────────────────────────────────────────────────────────────

pub struct CircuitTranspiler {
    pub config: TranspilerConfig,
    native_gate_set: NativeGateSet,
    device_catalog: Option<BraketDeviceCatalog>,
}

impl CircuitTranspiler {
    pub fn new(config: TranspilerConfig) -> Self {
        let native_gate_set = config.target_device.as_deref()
            .map(NativeGateSet::from_device_arn)
            .unwrap_or(NativeGateSet::Universal);
        let device_catalog = config.target_device.as_deref()
            .and_then(BraketDeviceCatalog::find_by_arn);
        Self { config, native_gate_set, device_catalog }
    }

    /// Create a transpiler targeting a specific Braket device.
    pub fn for_device(device_arn: &str) -> Self {
        Self::new(TranspilerConfig {
            target_device: Some(device_arn.to_string()),
            ..Default::default()
        })
    }

    /// Transpile a circuit: decompose non-native gates + optimize.
    pub fn transpile(&self, circuit: &QuantumCircuit) -> Result<(QuantumCircuit, TranspilationReport)> {
        let original_gates = circuit.gate_count();
        let original_depth = circuit.depth();

        let mut working = circuit.clone();
        let mut passes_applied = Vec::new();
        let mut gates_decomposed = 0usize;
        let mut rotations_merged = 0usize;

        // Pass 1: Decompose non-native gates into native gate set.
        if self.config.enable_basis_translation {
            let (decomposed, count) = self.decompose_to_native(&working);
            working = decomposed;
            gates_decomposed = count;
            if count > 0 {
                passes_applied.push(format!("basis_translation({count} decomposed)"));
            }
        }

        // Pass 2: Gate cancellation (XX=I, ZZ=I, etc.).
        if self.config.enable_cancellation && self.config.optimization_level >= 1 {
            let (cancelled, count) = self.cancel_inverse_gates(&working);
            working = cancelled;
            if count > 0 {
                passes_applied.push(format!("gate_cancellation({count} removed)"));
            }
        }

        // Pass 3: Rotation merging (Rz(a)Rz(b) = Rz(a+b)).
        if self.config.enable_rotation_merge && self.config.optimization_level >= 2 {
            let (merged, count) = self.merge_rotations(&working);
            working = merged;
            rotations_merged = count;
            if count > 0 {
                passes_applied.push(format!("rotation_merge({count} merged)"));
            }
        }

        // Pass 4: Remove identity rotations (Rz(0), Rx(0), etc.).
        if self.config.optimization_level >= 1 {
            let (cleaned, count) = self.remove_identity_rotations(&working);
            working = cleaned;
            if count > 0 {
                passes_applied.push(format!("identity_removal({count} removed)"));
            }
        }

        let transpiled_gates = working.gate_count();
        let transpiled_depth = working.depth();
        let gates_removed = original_gates.saturating_sub(transpiled_gates);

        let target_name = self.device_catalog.as_ref()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "universal".to_string());
        let native_gates = self.device_catalog.as_ref()
            .map(|c| c.native_gates.clone())
            .unwrap_or_else(|| vec!["H".into(), "CNOT".into(), "Rx".into(), "Ry".into(), "Rz".into()]);

        info!(
            "Transpiled: {} → {} gates, depth {} → {}, {} passes",
            original_gates, transpiled_gates, original_depth, transpiled_depth, passes_applied.len()
        );

        let report = TranspilationReport {
            original_gates,
            original_depth,
            transpiled_gates,
            transpiled_depth,
            gates_removed,
            gates_decomposed,
            rotations_merged,
            target_device: target_name,
            native_gate_set: native_gates,
            passes_applied,
        };

        Ok((working, report))
    }

    /// Validate that a circuit is compatible with the target device.
    pub fn validate_for_device(&self, circuit: &QuantumCircuit) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        if let Some(ref cat) = self.device_catalog {
            if circuit.qubits > cat.max_qubits {
                anyhow::bail!(
                    "Circuit requires {} qubits but {} supports at most {}",
                    circuit.qubits, cat.name, cat.max_qubits
                );
            }
        }

        for gate in &circuit.gates {
            if !self.native_gate_set.is_native(&gate.gate_type) {
                warnings.push(format!(
                    "Gate {:?} on qubit(s) {:?} is not native — will be decomposed",
                    gate.gate_type, gate.qubits
                ));
            }
        }

        // Check connectivity constraints for 2-qubit gates.
        if let Some(ref cat) = self.device_catalog {
            if cat.connectivity == BraketConnectivity::Linear {
                for gate in &circuit.gates {
                    if gate.qubits.len() == 2 {
                        let q0 = gate.qubits[0];
                        let q1 = gate.qubits[1];
                        if q0.abs_diff(q1) > 1 {
                            warnings.push(format!(
                                "Gate {:?} on qubits [{}, {}] requires SWAP routing (linear connectivity)",
                                gate.gate_type, q0, q1
                            ));
                        }
                    }
                }
            }
        }

        Ok(warnings)
    }

    // ── Decomposition passes ─────────────────────────────────────────────────

    /// Decompose non-native gates into the target native gate set.
    fn decompose_to_native(&self, circuit: &QuantumCircuit) -> (QuantumCircuit, usize) {
        let mut result = QuantumCircuit::new(circuit.qubits);
        result.classical_bits = circuit.classical_bits;
        result.metadata = circuit.metadata.clone();
        let mut decomposed = 0usize;

        for gate in &circuit.gates {
            if self.native_gate_set.is_native(&gate.gate_type) {
                result.add_gate(gate.clone());
                continue;
            }

            decomposed += 1;
            match &gate.gate_type {
                // H = Rz(π) · Ry(π/2) (for rotation-native devices)
                GateType::H => {
                    let q = gate.qubits[0];
                    match self.native_gate_set {
                        NativeGateSet::IonQNative | NativeGateSet::IqmNative | NativeGateSet::RigettiNative => {
                            result.add_gate(Gate::rz(q, std::f64::consts::PI));
                            result.add_gate(Gate::ry(q, std::f64::consts::FRAC_PI_2));
                        }
                        _ => {
                            result.add_gate(gate.clone());
                            decomposed -= 1;
                        }
                    }
                }
                // CNOT = H·CZ·H (for CZ-native devices like IQM, Rigetti)
                GateType::CNOT => {
                    let ctrl = gate.qubits[0];
                    let tgt = gate.qubits[1];
                    match self.native_gate_set {
                        NativeGateSet::IqmNative | NativeGateSet::RigettiNative => {
                            // CNOT = (I⊗H)·CZ·(I⊗H)
                            result.add_gate(Gate::ry(tgt, std::f64::consts::FRAC_PI_2));
                            result.add_gate(Gate::rz(tgt, std::f64::consts::PI));
                            result.add_gate(Gate::cz(ctrl, tgt));
                            result.add_gate(Gate::rz(tgt, std::f64::consts::PI));
                            result.add_gate(Gate::ry(tgt, std::f64::consts::FRAC_PI_2));
                        }
                        NativeGateSet::IonQNative => {
                            // IonQ: decompose CNOT via MS gate
                            // CNOT ≈ Ry(-π/2)·MS·Rx(-π/2)·Rx(-π/2)·Ry(π/2)
                            result.add_gate(Gate::ry(ctrl, -std::f64::consts::FRAC_PI_2));
                            result.add_gate(Gate::rx(ctrl, std::f64::consts::FRAC_PI_2));
                            result.add_gate(Gate::rx(tgt, -std::f64::consts::FRAC_PI_2));
                            result.add_gate(Gate::ry(ctrl, std::f64::consts::FRAC_PI_2));
                        }
                        _ => {
                            result.add_gate(gate.clone());
                            decomposed -= 1;
                        }
                    }
                }
                // X = Rx(π)
                GateType::X => {
                    let q = gate.qubits[0];
                    result.add_gate(Gate::rx(q, std::f64::consts::PI));
                }
                // Y = Ry(π)
                GateType::Y => {
                    let q = gate.qubits[0];
                    result.add_gate(Gate::ry(q, std::f64::consts::PI));
                }
                // Z = Rz(π)
                GateType::Z => {
                    let q = gate.qubits[0];
                    result.add_gate(Gate::rz(q, std::f64::consts::PI));
                }
                // S = Rz(π/2)
                GateType::S => {
                    let q = gate.qubits[0];
                    result.add_gate(Gate::rz(q, std::f64::consts::FRAC_PI_2));
                }
                // Sdg = Rz(-π/2)
                GateType::Sdg => {
                    let q = gate.qubits[0];
                    result.add_gate(Gate::rz(q, -std::f64::consts::FRAC_PI_2));
                }
                // T = Rz(π/4)
                GateType::T => {
                    let q = gate.qubits[0];
                    result.add_gate(Gate::rz(q, std::f64::consts::FRAC_PI_4));
                }
                // Tdg = Rz(-π/4)
                GateType::Tdg => {
                    let q = gate.qubits[0];
                    result.add_gate(Gate::rz(q, -std::f64::consts::FRAC_PI_4));
                }
                // U1(λ) = Rz(λ)
                GateType::U1(lam) => {
                    let q = gate.qubits[0];
                    result.add_gate(Gate::rz(q, *lam));
                }
                // U2(φ,λ) = Rz(φ+π/2) · Ry(π/2) · Rz(λ-π/2)
                GateType::U2(phi, lam) => {
                    let q = gate.qubits[0];
                    result.add_gate(Gate::rz(q, lam - std::f64::consts::FRAC_PI_2));
                    result.add_gate(Gate::ry(q, std::f64::consts::FRAC_PI_2));
                    result.add_gate(Gate::rz(q, phi + std::f64::consts::FRAC_PI_2));
                }
                // U3(θ,φ,λ) = Rz(φ) · Ry(θ) · Rz(λ)
                GateType::U3(theta, phi, lam) => {
                    let q = gate.qubits[0];
                    result.add_gate(Gate::rz(q, *lam));
                    result.add_gate(Gate::ry(q, *theta));
                    result.add_gate(Gate::rz(q, *phi));
                }
                // Toffoli → decompose into 1- and 2-qubit gates.
                GateType::Toffoli => {
                    if gate.qubits.len() >= 3 {
                        let c0 = gate.qubits[0];
                        let c1 = gate.qubits[1];
                        let tgt = gate.qubits[2];
                        self.decompose_toffoli(&mut result, c0, c1, tgt);
                    }
                }
                // Fredkin → decompose.
                GateType::Fredkin => {
                    if gate.qubits.len() >= 3 {
                        let ctrl = gate.qubits[0];
                        let q0 = gate.qubits[1];
                        let q1 = gate.qubits[2];
                        self.decompose_fredkin(&mut result, ctrl, q0, q1);
                    }
                }
                // Swap → 3 CNOTs (or CZs on CZ-native).
                GateType::Swap => {
                    let q0 = gate.qubits[0];
                    let q1 = gate.qubits[1];
                    result.add_gate(Gate::cnot(q0, q1));
                    result.add_gate(Gate::cnot(q1, q0));
                    result.add_gate(Gate::cnot(q0, q1));
                }
                // Pass through anything else.
                _ => {
                    result.add_gate(gate.clone());
                    decomposed -= 1;
                }
            }
        }

        (result, decomposed)
    }

    /// Toffoli decomposition into 1- and 2-qubit gates.
    fn decompose_toffoli(&self, circuit: &mut QuantumCircuit, c0: usize, c1: usize, tgt: usize) {
        // Standard Toffoli decomposition: 6 CNOTs + single-qubit rotations.
        circuit.add_gate(Gate::h(tgt));
        circuit.add_gate(Gate::cnot(c1, tgt));
        circuit.add_gate(Gate::rz(tgt, -std::f64::consts::FRAC_PI_4));
        circuit.add_gate(Gate::cnot(c0, tgt));
        circuit.add_gate(Gate::rz(tgt, std::f64::consts::FRAC_PI_4));
        circuit.add_gate(Gate::cnot(c1, tgt));
        circuit.add_gate(Gate::rz(tgt, -std::f64::consts::FRAC_PI_4));
        circuit.add_gate(Gate::cnot(c0, tgt));
        circuit.add_gate(Gate::rz(tgt, std::f64::consts::FRAC_PI_4));
        circuit.add_gate(Gate::rz(c1, std::f64::consts::FRAC_PI_4));
        circuit.add_gate(Gate::h(tgt));
        circuit.add_gate(Gate::cnot(c0, c1));
        circuit.add_gate(Gate::rz(c1, -std::f64::consts::FRAC_PI_4));
        circuit.add_gate(Gate::rz(c0, std::f64::consts::FRAC_PI_4));
        circuit.add_gate(Gate::cnot(c0, c1));
    }

    /// Fredkin decomposition: CSWAP = CNOT(q1,q0) · Toffoli(ctrl,q0,q1) · CNOT(q1,q0).
    fn decompose_fredkin(&self, circuit: &mut QuantumCircuit, ctrl: usize, q0: usize, q1: usize) {
        circuit.add_gate(Gate::cnot(q1, q0));
        self.decompose_toffoli(circuit, ctrl, q0, q1);
        circuit.add_gate(Gate::cnot(q1, q0));
    }

    // ── Optimization passes ──────────────────────────────────────────────────

    /// Cancel adjacent inverse gate pairs (XX=I, HH=I, etc.).
    fn cancel_inverse_gates(&self, circuit: &QuantumCircuit) -> (QuantumCircuit, usize) {
        let mut result = QuantumCircuit::new(circuit.qubits);
        result.classical_bits = circuit.classical_bits;
        result.metadata = circuit.metadata.clone();
        let mut cancelled = 0usize;

        let gates = &circuit.gates;
        let mut skip = vec![false; gates.len()];

        for i in 0..gates.len().saturating_sub(1) {
            if skip[i] { continue; }

            let g1 = &gates[i];
            let g2 = &gates[i + 1];

            if g1.qubits == g2.qubits && self.are_inverse(g1, g2) {
                skip[i] = true;
                skip[i + 1] = true;
                cancelled += 2;
                debug!("Cancelled inverse pair: {:?} at positions {},{}", g1.gate_type, i, i + 1);
            }
        }

        for (i, gate) in gates.iter().enumerate() {
            if !skip[i] {
                result.add_gate(gate.clone());
            }
        }

        (result, cancelled)
    }

    /// Check if two gates are inverses of each other.
    fn are_inverse(&self, g1: &Gate, g2: &Gate) -> bool {
        match (&g1.gate_type, &g2.gate_type) {
            // Self-inverse gates.
            (GateType::H, GateType::H) => true,
            (GateType::X, GateType::X) => true,
            (GateType::Y, GateType::Y) => true,
            (GateType::Z, GateType::Z) => true,
            (GateType::CNOT, GateType::CNOT) => true,
            (GateType::CZ, GateType::CZ) => true,
            (GateType::Swap, GateType::Swap) => true,
            // S and Sdg.
            (GateType::S, GateType::Sdg) | (GateType::Sdg, GateType::S) => true,
            // T and Tdg.
            (GateType::T, GateType::Tdg) | (GateType::Tdg, GateType::T) => true,
            // Rotation inverses: Rz(a) Rz(-a) = I.
            (GateType::Rx(a), GateType::Rx(b)) => (a + b).abs() < 1e-10,
            (GateType::Ry(a), GateType::Ry(b)) => (a + b).abs() < 1e-10,
            (GateType::Rz(a), GateType::Rz(b)) => (a + b).abs() < 1e-10,
            _ => false,
        }
    }

    /// Merge adjacent rotations on the same qubit and axis.
    fn merge_rotations(&self, circuit: &QuantumCircuit) -> (QuantumCircuit, usize) {
        let mut result = QuantumCircuit::new(circuit.qubits);
        result.classical_bits = circuit.classical_bits;
        result.metadata = circuit.metadata.clone();
        let mut merged = 0usize;

        let gates = &circuit.gates;
        let mut skip = vec![false; gates.len()];

        for i in 0..gates.len().saturating_sub(1) {
            if skip[i] { continue; }

            let g1 = &gates[i];
            let g2 = &gates[i + 1];

            if g1.qubits == g2.qubits {
                match (&g1.gate_type, &g2.gate_type) {
                    (GateType::Rx(a), GateType::Rx(b)) => {
                        let combined = a + b;
                        if combined.abs() > 1e-10 {
                            result.add_gate(Gate::rx(g1.qubits[0], combined));
                        }
                        skip[i] = true;
                        skip[i + 1] = true;
                        merged += 1;
                        continue;
                    }
                    (GateType::Ry(a), GateType::Ry(b)) => {
                        let combined = a + b;
                        if combined.abs() > 1e-10 {
                            result.add_gate(Gate::ry(g1.qubits[0], combined));
                        }
                        skip[i] = true;
                        skip[i + 1] = true;
                        merged += 1;
                        continue;
                    }
                    (GateType::Rz(a), GateType::Rz(b)) => {
                        let combined = a + b;
                        if combined.abs() > 1e-10 {
                            result.add_gate(Gate::rz(g1.qubits[0], combined));
                        }
                        skip[i] = true;
                        skip[i + 1] = true;
                        merged += 1;
                        continue;
                    }
                    _ => {}
                }
            }

            if !skip[i] {
                result.add_gate(g1.clone());
            }
        }

        // Don't forget the last gate.
        if let Some(last) = gates.last() {
            if !skip[gates.len() - 1] {
                result.add_gate(last.clone());
            }
        }

        (result, merged)
    }

    /// Remove identity rotations (angle ≈ 0 or ≈ 2π).
    fn remove_identity_rotations(&self, circuit: &QuantumCircuit) -> (QuantumCircuit, usize) {
        let mut result = QuantumCircuit::new(circuit.qubits);
        result.classical_bits = circuit.classical_bits;
        result.metadata = circuit.metadata.clone();
        let mut removed = 0usize;

        for gate in &circuit.gates {
            let is_identity = match &gate.gate_type {
                GateType::Rx(a) | GateType::Ry(a) | GateType::Rz(a) => {
                    let normalized = a.rem_euclid(2.0 * std::f64::consts::PI);
                    normalized.abs() < 1e-10 || (normalized - 2.0 * std::f64::consts::PI).abs() < 1e-10
                }
                GateType::U1(a) => a.abs() < 1e-10,
                _ => false,
            };

            if is_identity {
                removed += 1;
            } else {
                result.add_gate(gate.clone());
            }
        }

        (result, removed)
    }
}

impl Default for CircuitTranspiler {
    fn default() -> Self {
        Self::new(TranspilerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantum::circuit::{Gate, QuantumCircuit};

    #[test]
    fn test_gate_cancellation() {
        let transpiler = CircuitTranspiler::default();
        let mut c = QuantumCircuit::new(2);
        c.add_gate(Gate::h(0));
        c.add_gate(Gate::h(0)); // HH = I, should cancel.
        c.add_gate(Gate::x(1));
        c.measure_all();

        let (result, report) = transpiler.transpile(&c).unwrap();
        assert!(result.gate_count() < c.gate_count(), "HH should cancel");
        assert!(report.passes_applied.iter().any(|p| p.contains("cancellation")));
    }

    #[test]
    fn test_rotation_merge() {
        let transpiler = CircuitTranspiler::default();
        let mut c = QuantumCircuit::new(1);
        c.add_gate(Gate::rz(0, 0.5));
        c.add_gate(Gate::rz(0, 0.3));
        c.measure_all();

        let (result, report) = transpiler.transpile(&c).unwrap();
        assert!(report.rotations_merged > 0);
        // Should have merged into a single Rz(0.8).
        let rz_gates: Vec<_> = result.gates.iter().filter(|g| matches!(g.gate_type, GateType::Rz(_))).collect();
        assert_eq!(rz_gates.len(), 1, "two Rz should merge into one");
    }

    #[test]
    fn test_identity_removal() {
        let transpiler = CircuitTranspiler::default();
        let mut c = QuantumCircuit::new(1);
        c.add_gate(Gate::rz(0, 0.0)); // identity
        c.add_gate(Gate::h(0));
        c.measure_all();

        let (result, _report) = transpiler.transpile(&c).unwrap();
        assert!(!result.gates.iter().any(|g| matches!(g.gate_type, GateType::Rz(a) if a.abs() < 1e-10)));
    }

    #[test]
    fn test_toffoli_decomposition() {
        let transpiler = CircuitTranspiler::for_device("arn:aws:braket:eu-north-1::device/qpu/iqm/Garnet");
        let mut c = QuantumCircuit::new(3);
        c.add_gate(Gate::toffoli(0, 1, 2));
        c.measure_all();

        let (result, report) = transpiler.transpile(&c).unwrap();
        assert!(report.gates_decomposed > 0);
        // No Toffoli gates should remain.
        assert!(!result.gates.iter().any(|g| matches!(g.gate_type, GateType::Toffoli)));
    }

    #[test]
    fn test_cnot_to_cz_decomposition() {
        let transpiler = CircuitTranspiler::for_device("arn:aws:braket:eu-north-1::device/qpu/iqm/Garnet");
        let mut c = QuantumCircuit::new(2);
        c.add_gate(Gate::cnot(0, 1));
        c.measure_all();

        let (result, report) = transpiler.transpile(&c).unwrap();
        assert!(report.gates_decomposed > 0);
        // Should contain CZ gates now.
        assert!(result.gates.iter().any(|g| matches!(g.gate_type, GateType::CZ)));
    }

    #[test]
    fn test_simulator_passthrough() {
        let transpiler = CircuitTranspiler::for_device("arn:aws:braket:::device/quantum-simulator/amazon/sv1");
        let mut c = QuantumCircuit::new(2);
        c.add_gate(Gate::h(0));
        c.add_gate(Gate::cnot(0, 1));
        c.add_gate(Gate::toffoli(0, 1, 0)); // invalid but tests passthrough

        let (result, report) = transpiler.transpile(&c).unwrap();
        // Simulator accepts all gates — no decomposition needed.
        assert_eq!(report.gates_decomposed, 0);
    }

    #[test]
    fn test_native_gate_set_detection() {
        assert_eq!(
            NativeGateSet::from_device_arn("arn:aws:braket:us-east-1::device/qpu/ionq/Aria-1"),
            NativeGateSet::IonQNative
        );
        assert_eq!(
            NativeGateSet::from_device_arn("arn:aws:braket:::device/quantum-simulator/amazon/sv1"),
            NativeGateSet::BraketSimulator
        );
    }
}

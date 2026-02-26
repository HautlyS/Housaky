use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GateType {
    H,
    X,
    Y,
    Z,
    CNOT,
    CZ,
    S,
    T,
    Sdg,
    Tdg,
    Rx(f64),
    Ry(f64),
    Rz(f64),
    U1(f64),
    U2(f64, f64),
    U3(f64, f64, f64),
    Swap,
    Toffoli,
    Fredkin,
    Measure,
    Reset,
    Barrier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate {
    pub gate_type: GateType,
    pub qubits: Vec<usize>,
    pub classical_bits: Vec<usize>,
}

impl Gate {
    pub fn h(qubit: usize) -> Self {
        Self { gate_type: GateType::H, qubits: vec![qubit], classical_bits: vec![] }
    }

    pub fn x(qubit: usize) -> Self {
        Self { gate_type: GateType::X, qubits: vec![qubit], classical_bits: vec![] }
    }

    pub fn y(qubit: usize) -> Self {
        Self { gate_type: GateType::Y, qubits: vec![qubit], classical_bits: vec![] }
    }

    pub fn z(qubit: usize) -> Self {
        Self { gate_type: GateType::Z, qubits: vec![qubit], classical_bits: vec![] }
    }

    pub fn cnot(control: usize, target: usize) -> Self {
        Self { gate_type: GateType::CNOT, qubits: vec![control, target], classical_bits: vec![] }
    }

    pub fn cz(control: usize, target: usize) -> Self {
        Self { gate_type: GateType::CZ, qubits: vec![control, target], classical_bits: vec![] }
    }

    pub fn rx(qubit: usize, theta: f64) -> Self {
        Self { gate_type: GateType::Rx(theta), qubits: vec![qubit], classical_bits: vec![] }
    }

    pub fn ry(qubit: usize, theta: f64) -> Self {
        Self { gate_type: GateType::Ry(theta), qubits: vec![qubit], classical_bits: vec![] }
    }

    pub fn rz(qubit: usize, theta: f64) -> Self {
        Self { gate_type: GateType::Rz(theta), qubits: vec![qubit], classical_bits: vec![] }
    }

    pub fn measure(qubit: usize, cbit: usize) -> Self {
        Self { gate_type: GateType::Measure, qubits: vec![qubit], classical_bits: vec![cbit] }
    }

    pub fn swap(q0: usize, q1: usize) -> Self {
        Self { gate_type: GateType::Swap, qubits: vec![q0, q1], classical_bits: vec![] }
    }

    pub fn toffoli(c0: usize, c1: usize, target: usize) -> Self {
        Self { gate_type: GateType::Toffoli, qubits: vec![c0, c1, target], classical_bits: vec![] }
    }

    pub fn depth_contribution(&self) -> u32 {
        match &self.gate_type {
            GateType::Barrier => 0,
            GateType::Measure => 1,
            GateType::Toffoli | GateType::Fredkin => 6,
            GateType::CNOT | GateType::CZ | GateType::Swap => 2,
            _ => 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measurement {
    pub qubit: usize,
    pub classical_bit: usize,
    pub basis: MeasurementBasis,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MeasurementBasis {
    Computational,
    Bell,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumCircuit {
    pub qubits: usize,
    pub classical_bits: usize,
    pub gates: Vec<Gate>,
    pub measurements: Vec<Measurement>,
    pub metadata: HashMap<String, String>,
}

impl QuantumCircuit {
    pub fn new(qubits: usize) -> Self {
        Self {
            qubits,
            classical_bits: qubits,
            gates: Vec::new(),
            measurements: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    pub fn add_gate(&mut self, gate: Gate) -> &mut Self {
        self.gates.push(gate);
        self
    }

    pub fn add_measurement(&mut self, qubit: usize, cbit: usize) -> &mut Self {
        self.measurements.push(Measurement {
            qubit,
            classical_bit: cbit,
            basis: MeasurementBasis::Computational,
        });
        self.gates.push(Gate::measure(qubit, cbit));
        self
    }

    pub fn measure_all(&mut self) -> &mut Self {
        for i in 0..self.qubits {
            self.add_measurement(i, i);
        }
        self
    }

    pub fn depth(&self) -> u32 {
        self.gates.iter().map(|g| g.depth_contribution()).sum()
    }

    pub fn gate_count(&self) -> usize {
        self.gates.len()
    }

    pub fn two_qubit_gate_count(&self) -> usize {
        self.gates.iter().filter(|g| g.qubits.len() == 2).count()
    }

    pub fn validate(&self) -> Result<(), String> {
        for gate in &self.gates {
            for &q in &gate.qubits {
                if q >= self.qubits {
                    return Err(format!("Gate references qubit {} but circuit only has {} qubits", q, self.qubits));
                }
            }
            for &c in &gate.classical_bits {
                if c >= self.classical_bits {
                    return Err(format!("Gate references classical bit {} but circuit only has {}", c, self.classical_bits));
                }
            }
        }
        Ok(())
    }

    pub fn bell_pair(q0: usize, q1: usize) -> Self {
        let mut circuit = Self::new(2.max(q0 + 1).max(q1 + 1));
        circuit.add_gate(Gate::h(q0));
        circuit.add_gate(Gate::cnot(q0, q1));
        circuit
    }

    pub fn ghz(n: usize) -> Self {
        let mut circuit = Self::new(n);
        circuit.add_gate(Gate::h(0));
        for i in 1..n {
            circuit.add_gate(Gate::cnot(0, i));
        }
        circuit.measure_all();
        circuit
    }

    pub fn qft(n: usize) -> Self {
        let mut circuit = Self::new(n);
        for i in 0..n {
            circuit.add_gate(Gate::h(i));
            for j in (i + 1)..n {
                let k = (j - i + 1) as f64;
                circuit.add_gate(Gate::rz(j, std::f64::consts::PI / 2_f64.powf(k)));
            }
        }
        for i in 0..(n / 2) {
            circuit.add_gate(Gate::swap(i, n - 1 - i));
        }
        circuit
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementResult {
    pub counts: HashMap<String, u64>,
    pub shots: u64,
    pub raw_bitstrings: Vec<String>,
    pub execution_time_ms: u64,
    pub backend_id: String,
}

impl MeasurementResult {
    pub fn probability(&self, bitstring: &str) -> f64 {
        let count = self.counts.get(bitstring).copied().unwrap_or(0);
        if self.shots == 0 {
            0.0
        } else {
            count as f64 / self.shots as f64
        }
    }

    pub fn most_probable(&self) -> Option<(&String, f64)> {
        self.counts
            .iter()
            .max_by(|a, b| a.1.cmp(b.1))
            .map(|(k, &v)| (k, v as f64 / self.shots as f64))
    }

    pub fn expectation_value(&self) -> f64 {
        self.counts.iter().map(|(bits, &count)| {
            let parity: i32 = bits.chars()
                .map(|c| if c == '1' { 1 } else { 0 })
                .sum::<i32>() % 2;
            let sign = if parity == 0 { 1.0 } else { -1.0 };
            sign * (count as f64 / self.shots as f64)
        }).sum()
    }

    pub fn entropy(&self) -> f64 {
        self.counts.values().map(|&c| {
            let p = c as f64 / self.shots as f64;
            if p > 0.0 { -p * p.log2() } else { 0.0 }
        }).sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseModel {
    pub single_qubit_error_rate: f64,
    pub two_qubit_error_rate: f64,
    pub measurement_error_rate: f64,
    pub t1_us: f64,
    pub t2_us: f64,
    pub gate_time_ns: f64,
}

impl NoiseModel {
    pub fn ideal() -> Self {
        Self {
            single_qubit_error_rate: 0.0,
            two_qubit_error_rate: 0.0,
            measurement_error_rate: 0.0,
            t1_us: f64::INFINITY,
            t2_us: f64::INFINITY,
            gate_time_ns: 0.0,
        }
    }

    pub fn typical_superconducting() -> Self {
        Self {
            single_qubit_error_rate: 0.001,
            two_qubit_error_rate: 0.01,
            measurement_error_rate: 0.02,
            t1_us: 100.0,
            t2_us: 80.0,
            gate_time_ns: 50.0,
        }
    }

    pub fn is_noisy(&self) -> bool {
        self.single_qubit_error_rate > 0.0
            || self.two_qubit_error_rate > 0.0
            || self.measurement_error_rate > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_construction() {
        let mut c = QuantumCircuit::new(3);
        c.add_gate(Gate::h(0));
        c.add_gate(Gate::cnot(0, 1));
        c.add_gate(Gate::cnot(0, 2));
        c.measure_all();
        assert!(c.validate().is_ok());
        assert_eq!(c.gate_count(), 6);
    }

    #[test]
    fn test_ghz_circuit() {
        let c = QuantumCircuit::ghz(4);
        assert_eq!(c.qubits, 4);
        assert!(c.validate().is_ok());
    }

    #[test]
    fn test_measurement_result_probability() {
        let mut counts = HashMap::new();
        counts.insert("00".to_string(), 512u64);
        counts.insert("11".to_string(), 512u64);
        let r = MeasurementResult {
            counts,
            shots: 1024,
            raw_bitstrings: vec![],
            execution_time_ms: 10,
            backend_id: "test".to_string(),
        };
        assert!((r.probability("00") - 0.5).abs() < 1e-9);
        assert!((r.probability("01") - 0.0).abs() < 1e-9);
    }
}

use super::backend::QuantumBackend;
use super::circuit::{MeasurementResult, QuantumCircuit};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MitigationStrategy {
    ZeroNoiseExtrapolation,
    ProbabilisticErrorCancellation,
    ReadoutErrorMitigation,
    SymmetryVerification,
    VirtualDistillation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MitigationConfig {
    pub strategies: Vec<MitigationStrategy>,
    pub zne_scale_factors: Vec<f64>,
    pub pec_samples: usize,
    pub readout_calibration_shots: u64,
}

impl Default for MitigationConfig {
    fn default() -> Self {
        Self {
            strategies: vec![
                MitigationStrategy::ReadoutErrorMitigation,
                MitigationStrategy::ZeroNoiseExtrapolation,
            ],
            zne_scale_factors: vec![1.0, 2.0, 3.0],
            pec_samples: 100,
            readout_calibration_shots: 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadoutCalibration {
    pub confusion_matrix: Vec<Vec<f64>>,
    pub num_qubits: usize,
    pub calibration_shots: u64,
}

impl ReadoutCalibration {
    pub fn new(num_qubits: usize) -> Self {
        let dim = 1 << num_qubits;
        let mut matrix = vec![vec![0.0f64; dim]; dim];
        for i in 0..dim {
            matrix[i][i] = 1.0;
        }
        Self { confusion_matrix: matrix, num_qubits, calibration_shots: 0 }
    }

    pub fn from_error_rate(num_qubits: usize, error_rate: f64) -> Self {
        let dim = 1 << num_qubits;
        let mut matrix = vec![vec![0.0f64; dim]; dim];
        for i in 0..dim {
            matrix[i][i] = 1.0 - error_rate;
            let noise_per_other = error_rate / (dim - 1) as f64;
            for j in 0..dim {
                if j != i {
                    matrix[i][j] = noise_per_other;
                }
            }
        }
        Self { confusion_matrix: matrix, num_qubits, calibration_shots: 1024 }
    }

    pub fn apply(&self, counts: &HashMap<String, u64>, shots: u64) -> HashMap<String, u64> {
        let dim = 1 << self.num_qubits;
        let mut raw_probs = vec![0.0f64; dim];

        for (bits, &count) in counts {
            if let Ok(idx) = usize::from_str_radix(bits, 2) {
                if idx < dim {
                    raw_probs[idx] = count as f64 / shots as f64;
                }
            }
        }

        let mut corrected = vec![0.0f64; dim];
        for i in 0..dim {
            for j in 0..dim {
                if self.confusion_matrix[j][i] > 0.0 {
                    corrected[i] += self.confusion_matrix[j][i] * raw_probs[j];
                }
            }
            corrected[i] = corrected[i].max(0.0);
        }

        let total: f64 = corrected.iter().sum();
        let mut result = HashMap::new();
        if total > 0.0 {
            for (i, &p) in corrected.iter().enumerate() {
                let count = ((p / total) * shots as f64).round() as u64;
                if count > 0 {
                    result.insert(
                        format!("{:0>width$b}", i, width = self.num_qubits),
                        count,
                    );
                }
            }
        }
        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MitigatedResult {
    pub raw_result: MeasurementResult,
    pub mitigated_counts: HashMap<String, u64>,
    pub mitigated_expectation: f64,
    pub mitigation_overhead: f64,
    pub strategies_applied: Vec<String>,
    pub confidence_improvement: f64,
}

pub struct ErrorMitigator {
    pub backend: Arc<dyn QuantumBackend>,
    pub config: MitigationConfig,
    pub readout_calibration: Option<ReadoutCalibration>,
}

impl ErrorMitigator {
    pub fn new(backend: Arc<dyn QuantumBackend>, config: MitigationConfig) -> Self {
        Self { backend, config, readout_calibration: None }
    }

    pub fn with_calibration(mut self, calibration: ReadoutCalibration) -> Self {
        self.readout_calibration = Some(calibration);
        self
    }

    pub async fn execute_mitigated(
        &self,
        circuit: &QuantumCircuit,
    ) -> Result<MitigatedResult> {
        let raw_result = self.backend.execute_circuit(circuit).await?;
        let mut strategies_applied = Vec::new();
        let mut counts = raw_result.counts.clone();

        if self.config.strategies.contains(&MitigationStrategy::ReadoutErrorMitigation) {
            if let Some(cal) = &self.readout_calibration {
                counts = cal.apply(&counts, raw_result.shots);
                strategies_applied.push("readout_error_mitigation".to_string());
            }
        }

        if self.config.strategies.contains(&MitigationStrategy::ZeroNoiseExtrapolation) {
            let zne_result = self.zero_noise_extrapolation(circuit).await?;
            counts = zne_result;
            strategies_applied.push("zero_noise_extrapolation".to_string());
        }

        let raw_exp = raw_result.expectation_value();
        let mitigated_exp = self.compute_expectation(&counts, raw_result.shots);

        let confidence_improvement = if raw_exp.abs() > 1e-9 {
            (mitigated_exp.abs() - raw_exp.abs()) / raw_exp.abs()
        } else {
            0.0
        };

        let overhead = self.compute_overhead();

        Ok(MitigatedResult {
            raw_result,
            mitigated_counts: counts,
            mitigated_expectation: mitigated_exp,
            mitigation_overhead: overhead,
            strategies_applied,
            confidence_improvement,
        })
    }

    async fn zero_noise_extrapolation(
        &self,
        circuit: &QuantumCircuit,
    ) -> Result<HashMap<String, u64>> {
        let mut expectation_values = Vec::new();

        for &scale in &self.config.zne_scale_factors {
            let scaled = self.scale_circuit_noise(circuit, scale);
            let result = self.backend.execute_circuit(&scaled).await?;
            expectation_values.push((scale, result.expectation_value()));
        }

        let zero_noise_exp = self.richardson_extrapolate(&expectation_values);

        let base_result = self.backend.execute_circuit(circuit).await?;
        let correction = zero_noise_exp / base_result.expectation_value().abs().max(1e-9);

        let mut corrected = HashMap::new();
        for (bits, &count) in &base_result.counts {
            let adjusted = ((count as f64) * correction.abs()).round() as u64;
            if adjusted > 0 {
                corrected.insert(bits.clone(), adjusted);
            }
        }

        if corrected.is_empty() {
            Ok(base_result.counts)
        } else {
            Ok(corrected)
        }
    }

    fn scale_circuit_noise(&self, circuit: &QuantumCircuit, scale: f64) -> QuantumCircuit {
        let mut scaled = circuit.clone();
        scaled.metadata.insert("noise_scale".to_string(), scale.to_string());
        if scale > 1.0 {
            let mut extra_gates = Vec::new();
            for gate in &circuit.gates {
                extra_gates.push(gate.clone());
                extra_gates.push(gate.clone());
            }
            for gate in extra_gates {
                scaled.gates.push(gate);
            }
        }
        scaled
    }

    fn richardson_extrapolate(&self, data: &[(f64, f64)]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }
        if data.len() == 1 {
            return data[0].1;
        }

        let n = data.len();
        let mut coeffs = vec![0.0f64; n];
        for i in 0..n {
            let mut c = 1.0;
            for j in 0..n {
                if j != i {
                    let denom = data[i].0 - data[j].0;
                    if denom.abs() > 1e-12 {
                        c *= -data[j].0 / denom;
                    }
                }
            }
            coeffs[i] = c;
        }

        coeffs.iter().zip(data.iter()).map(|(&c, &(_, e))| c * e).sum()
    }

    fn compute_expectation(&self, counts: &HashMap<String, u64>, shots: u64) -> f64 {
        if shots == 0 {
            return 0.0;
        }
        counts.iter().map(|(bits, &count)| {
            let parity: i32 = bits.chars().map(|c| if c == '1' { 1 } else { 0 }).sum::<i32>() % 2;
            let sign = if parity == 0 { 1.0 } else { -1.0 };
            sign * (count as f64 / shots as f64)
        }).sum()
    }

    fn compute_overhead(&self) -> f64 {
        let mut overhead = 1.0;
        for strategy in &self.config.strategies {
            overhead *= match strategy {
                MitigationStrategy::ZeroNoiseExtrapolation => {
                    self.config.zne_scale_factors.len() as f64
                }
                MitigationStrategy::ProbabilisticErrorCancellation => {
                    self.config.pec_samples as f64
                }
                MitigationStrategy::ReadoutErrorMitigation => 2.0,
                MitigationStrategy::SymmetryVerification => 1.5,
                MitigationStrategy::VirtualDistillation => 3.0,
            };
        }
        overhead
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_readout_calibration_identity() {
        let cal = ReadoutCalibration::new(2);
        let mut counts = HashMap::new();
        counts.insert("00".to_string(), 500u64);
        counts.insert("11".to_string(), 500u64);
        let corrected = cal.apply(&counts, 1000);
        assert_eq!(corrected.get("00").copied().unwrap_or(0), 500);
        assert_eq!(corrected.get("11").copied().unwrap_or(0), 500);
    }

    #[test]
    fn test_richardson_extrapolate() {
        use crate::quantum::backend::SimulatorBackend;
        let backend = Arc::new(SimulatorBackend::new(4, 1024));
        let mitigator = ErrorMitigator::new(backend, MitigationConfig::default());
        let data = vec![(1.0, 0.9), (2.0, 0.8), (3.0, 0.7)];
        let result = mitigator.richardson_extrapolate(&data);
        assert!(result > 0.9 && result < 1.1, "ZNE extrapolation result: {}", result);
    }
}

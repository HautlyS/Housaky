//! QCBM — Quantum Circuit Born Machine (Generative Quantum Model)
//!
//! A QCBM is a variational quantum circuit whose output measurement distribution
//! models a target probability distribution via the Born rule: P(x) = |⟨x|ψ(θ)⟩|².
//!
//! For AGI Singularity use-cases:
//!   - **Quantum Imagination**: generate diverse scenario samples for planning
//!   - **Counterfactual reasoning**: sample alternative world-states
//!   - **Creative synthesis**: generate novel goal combinations
//!   - **Belief uncertainty**: model multi-modal posterior distributions
//!
//! Architecture (hardware-efficient ansatz):
//!   1. Layer of Ry(θᵢ) + Rz(φᵢ) rotations on all qubits.
//!   2. Entanglement layer: nearest-neighbor CNOT ladder.
//!   3. Repeat for `depth` layers.
//!   4. Measure → Born-rule sample from the learned distribution.
//!
//! Training uses Maximum Mean Discrepancy (MMD) gradient estimation or,
//! for online AGI use, the circuit parameters are tuned via classical
//! gradient-free optimization (CMA-ES proxy).

use super::backend::QuantumBackend;
use super::circuit::{Gate, QuantumCircuit};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QCBMConfig {
    /// Number of qubits in the generative circuit.
    pub n_qubits: usize,
    /// Circuit depth (number of rotation+entanglement layers).
    pub depth: usize,
    /// Number of measurement shots per sample batch.
    pub shots: u64,
    /// Number of training iterations for online parameter tuning.
    pub training_iterations: usize,
    /// Learning rate for parameter updates.
    pub learning_rate: f64,
}

impl Default for QCBMConfig {
    fn default() -> Self {
        Self {
            n_qubits: 4,
            depth: 2,
            shots: 1024,
            training_iterations: 20,
            learning_rate: 0.05,
        }
    }
}

// ── Results ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QCBMSample {
    /// Sampled bitstring.
    pub bitstring: String,
    /// Probability of this sample under current circuit parameters.
    pub probability: f64,
    /// Decoded as a floating-point vector (each bit → 0.0 or 1.0).
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QCBMResult {
    /// Generated scenario samples.
    pub samples: Vec<QCBMSample>,
    /// Full probability distribution over all measured bitstrings.
    pub distribution: HashMap<String, f64>,
    /// Shannon entropy of the generated distribution (diversity measure).
    pub entropy: f64,
    /// Circuit parameters used to generate the samples.
    pub parameters: Vec<f64>,
    /// Strategy used: "quantum_qcbm" or "classical_sampling".
    pub strategy: String,
    pub runtime_ms: u64,
}

/// A context that biases the QCBM toward desired output regions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerativeContext {
    /// Feature vector encoding the current AGI context (values ∈ [0, 1]).
    pub features: Vec<f64>,
    /// Desired diversity level (0 = focused, 1 = maximally diverse).
    pub diversity: f64,
    /// Number of distinct scenarios to generate.
    pub n_scenarios: usize,
}

// ── QCBM Engine ───────────────────────────────────────────────────────────────

pub struct QuantumBornMachine {
    backend: Arc<dyn QuantumBackend>,
    pub config: QCBMConfig,
    /// Trained circuit parameters [Ry, Rz] per qubit per layer.
    parameters: Vec<f64>,
}

impl QuantumBornMachine {
    pub fn new(backend: Arc<dyn QuantumBackend>, config: QCBMConfig) -> Self {
        let n_params = config.n_qubits * config.depth * 2; // Ry + Rz per qubit per layer
        // Initialize parameters with small random-ish values derived from golden ratio.
        let parameters: Vec<f64> = (0..n_params)
            .map(|i| {
                let phi = 1.618_033_988_749_895_f64;
                ((i as f64 * phi).fract() * 2.0 * std::f64::consts::PI).rem_euclid(
                    2.0 * std::f64::consts::PI,
                )
            })
            .collect();

        Self {
            backend,
            config,
            parameters,
        }
    }

    /// Generate scenarios by sampling from the QCBM.
    ///
    /// `context`: biases the circuit parameters toward the context feature space.
    pub async fn generate_scenarios(
        &mut self,
        context: &GenerativeContext,
    ) -> Result<QCBMResult> {
        let start = std::time::Instant::now();

        if self.config.n_qubits > 30 {
            return Ok(self.classical_sampling(context, start));
        }

        // Adapt parameters to context by blending context features into rotations.
        self.adapt_to_context(context);

        // Build and execute the QCBM circuit.
        let circuit = self.build_qcbm_circuit(&self.parameters.clone());
        let result = self.backend.execute_circuit(&circuit).await?;

        // Convert measurement results to probability distribution.
        let total = result.shots as f64;
        let distribution: HashMap<String, f64> = result
            .counts
            .iter()
            .map(|(k, &v)| (k.clone(), v as f64 / total))
            .collect();

        // Sample top-n_scenarios from the distribution.
        let mut sorted: Vec<(&String, f64)> = distribution
            .iter()
            .map(|(k, &v)| (k, v))
            .collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let samples: Vec<QCBMSample> = sorted
            .iter()
            .take(context.n_scenarios)
            .map(|(bits, prob)| {
                let values: Vec<f64> = bits
                    .chars()
                    .map(|c| if c == '1' { 1.0 } else { 0.0 })
                    .collect();
                QCBMSample {
                    bitstring: bits.to_string(),
                    probability: *prob,
                    values,
                }
            })
            .collect();

        // Compute Shannon entropy (diversity measure).
        let entropy: f64 = distribution
            .values()
            .filter(|&&p| p > 0.0)
            .map(|&p| -p * p.log2())
            .sum();

        info!(
            "🔮 QCBM: generated {} scenarios from {}-qubit circuit (depth={}, entropy={:.3} bits)",
            samples.len(),
            self.config.n_qubits,
            self.config.depth,
            entropy
        );

        Ok(QCBMResult {
            samples,
            distribution,
            entropy,
            parameters: self.parameters.clone(),
            strategy: "quantum_qcbm".into(),
            runtime_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Online parameter update: push parameters toward higher-entropy regions
    /// using a gradient-free perturbation step (parameter-shift rule approximation).
    pub async fn train_step(&mut self, target_distribution: &HashMap<String, f64>) -> f64 {
        let n = self.parameters.len();
        let mut best_mmd = f64::MAX;

        for i in 0..n.min(self.config.training_iterations) {
            // Perturb parameter i by +π/2 and -π/2 (parameter shift rule).
            let original = self.parameters[i];

            self.parameters[i] = original + std::f64::consts::FRAC_PI_2;
            let circuit_plus = self.build_qcbm_circuit(&self.parameters.clone());

            self.parameters[i] = original - std::f64::consts::FRAC_PI_2;
            let circuit_minus = self.build_qcbm_circuit(&self.parameters.clone());

            // Execute both circuits.
            let (result_plus, result_minus) = tokio::join!(
                self.backend.execute_circuit(&circuit_plus),
                self.backend.execute_circuit(&circuit_minus),
            );

            let mmd_plus = if let Ok(r) = result_plus {
                self.compute_mmd(&r.counts, target_distribution, r.shots)
            } else {
                f64::MAX
            };
            let mmd_minus = if let Ok(r) = result_minus {
                self.compute_mmd(&r.counts, target_distribution, r.shots)
            } else {
                f64::MAX
            };

            // Gradient estimate: (E+ - E-) / 2
            let grad = (mmd_plus - mmd_minus) / 2.0;
            self.parameters[i] = original - self.config.learning_rate * grad;

            best_mmd = best_mmd.min(mmd_plus).min(mmd_minus);
        }

        best_mmd
    }

    // ── Circuit Construction ──────────────────────────────────────────────────

    fn build_qcbm_circuit(&self, params: &[f64]) -> QuantumCircuit {
        let n = self.config.n_qubits;
        let mut circuit = QuantumCircuit::new(n);
        let mut param_idx = 0;

        for _layer in 0..self.config.depth {
            // Rotation layer: Ry + Rz on each qubit.
            for q in 0..n {
                let ry_angle = params.get(param_idx).copied().unwrap_or(0.0);
                param_idx += 1;
                let rz_angle = params.get(param_idx).copied().unwrap_or(0.0);
                param_idx += 1;
                circuit.add_gate(Gate::ry(q, ry_angle));
                circuit.add_gate(Gate::rz(q, rz_angle));
            }

            // Entanglement layer: nearest-neighbor CNOT ladder.
            for q in 0..n.saturating_sub(1) {
                circuit.add_gate(Gate::cnot(q, q + 1));
            }
            // Wrap-around CNOT for periodic boundary (improves entanglement).
            if n > 2 {
                circuit.add_gate(Gate::cnot(n - 1, 0));
            }
        }

        circuit.measure_all();
        circuit
    }

    // ── Context Adaptation ───────────────────────────────────────────────────

    /// Blend context features into circuit parameters to bias generation.
    fn adapt_to_context(&mut self, context: &GenerativeContext) {
        let n_params = self.parameters.len();
        for (i, param) in self.parameters.iter_mut().enumerate() {
            let feature_idx = i % context.features.len().max(1);
            let feature = context.features.get(feature_idx).copied().unwrap_or(0.5);
            let diversity_noise = context.diversity
                * (i as f64 * 1.618_033_988_749_895_f64).fract()
                * 0.3;
            // Blend: keep 80% of learned param, 20% context influence.
            let context_angle =
                feature * 2.0 * std::f64::consts::PI + diversity_noise;
            *param = 0.8 * *param + 0.2 * context_angle;
            // Keep in [0, 2π].
            *param = param.rem_euclid(2.0 * std::f64::consts::PI);
            let _ = n_params; // suppress unused
        }
    }

    // ── MMD Loss ─────────────────────────────────────────────────────────────

    /// Maximum Mean Discrepancy between generated and target distributions.
    fn compute_mmd(
        &self,
        counts: &HashMap<String, u64>,
        target: &HashMap<String, f64>,
        shots: u64,
    ) -> f64 {
        let total = shots as f64;
        let mut mmd = 0.0;

        for (key, &target_prob) in target {
            let gen_prob = counts.get(key).copied().unwrap_or(0) as f64 / total;
            mmd += (gen_prob - target_prob).powi(2);
        }

        mmd.sqrt()
    }

    // ── Classical Fallback ────────────────────────────────────────────────────

    fn classical_sampling(
        &self,
        context: &GenerativeContext,
        timer: std::time::Instant,
    ) -> QCBMResult {
        let n = self.config.n_qubits.min(16);
        let mut distribution: HashMap<String, f64> = HashMap::new();

        // Sample from a context-biased Bernoulli product distribution.
        let n_samples = context.n_scenarios * 4;
        let shots = self.config.shots;

        for s in 0..shots.min(n_samples as u64 * 10) {
            let mut bits = String::new();
            for q in 0..n {
                let feature_idx = (q + s as usize) % context.features.len().max(1);
                let prob = context.features.get(feature_idx).copied().unwrap_or(0.5);
                let noise = context.diversity * ((s as f64 * 1.618).fract() - 0.5) * 0.2;
                let p_one = (prob + noise).clamp(0.0, 1.0);
                // Deterministic pseudo-sampling.
                let sample = ((s as f64 * (q + 1) as f64 * 0.6180339887).fract()) < p_one;
                bits.push(if sample { '1' } else { '0' });
            }
            *distribution.entry(bits).or_insert(0.0) += 1.0 / shots as f64;
        }

        let mut sorted: Vec<(&String, f64)> = distribution.iter().map(|(k, &v)| (k, v)).collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let samples: Vec<QCBMSample> = sorted
            .iter()
            .take(context.n_scenarios)
            .map(|(bits, prob)| {
                let values = bits.chars().map(|c| if c == '1' { 1.0 } else { 0.0 }).collect();
                QCBMSample {
                    bitstring: bits.to_string(),
                    probability: *prob,
                    values,
                }
            })
            .collect();

        let entropy: f64 = distribution
            .values()
            .filter(|&&p| p > 0.0)
            .map(|&p| -p * p.log2())
            .sum();

        QCBMResult {
            samples,
            distribution,
            entropy,
            parameters: self.parameters.clone(),
            strategy: "classical_sampling".into(),
            runtime_ms: timer.elapsed().as_millis() as u64,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantum::backend::SimulatorBackend;

    fn make_qcbm() -> QuantumBornMachine {
        let backend = Arc::new(SimulatorBackend::new(8, 512));
        QuantumBornMachine::new(
            backend,
            QCBMConfig {
                n_qubits: 3,
                depth: 2,
                shots: 512,
                training_iterations: 5,
                learning_rate: 0.05,
            },
        )
    }

    #[tokio::test]
    async fn test_generate_scenarios() {
        let mut qcbm = make_qcbm();
        let context = GenerativeContext {
            features: vec![0.7, 0.3, 0.5],
            diversity: 0.5,
            n_scenarios: 4,
        };
        let result = qcbm.generate_scenarios(&context).await.unwrap();
        assert!(!result.samples.is_empty());
        assert!(result.entropy >= 0.0);
        assert_eq!(result.strategy, "quantum_qcbm");
    }

    #[tokio::test]
    async fn test_distribution_sums_to_one() {
        let mut qcbm = make_qcbm();
        let context = GenerativeContext {
            features: vec![0.5, 0.5, 0.5],
            diversity: 0.3,
            n_scenarios: 3,
        };
        let result = qcbm.generate_scenarios(&context).await.unwrap();
        let total: f64 = result.distribution.values().sum();
        assert!((total - 1.0).abs() < 0.05, "distribution sums to {total}");
    }

    #[tokio::test]
    async fn test_classical_fallback_large() {
        let backend = Arc::new(SimulatorBackend::new(8, 512));
        let qcbm = QuantumBornMachine::new(
            backend,
            QCBMConfig {
                n_qubits: 35, // exceeds SV1 limit
                depth: 2,
                shots: 512,
                training_iterations: 5,
                learning_rate: 0.05,
            },
        );
        let context = GenerativeContext {
            features: vec![0.5],
            diversity: 0.5,
            n_scenarios: 3,
        };
        // Use classical_sampling directly.
        let result = qcbm.classical_sampling(
            &context,
            std::time::Instant::now(),
        );
        assert!(result.strategy.contains("classical"));
    }
}

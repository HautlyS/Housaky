//! Seed Mind Core: RecursiveCore, NestedWeights, and the Living Cycle
//!
//! The heart of the Seed Mind: multi-timescale learning with the living cycle
//! that continuously perceives, reasons, acts, learns, reflects, and self-modifies.

use super::config::SeedMindConfig;
use super::consciousness::NetworkConsciousness;
use super::darwin_godel::DarwinGodelMachine;
use super::failure::FailureDetector;
use super::karma::KarmaSystem;
use super::safety::SafetyGuardrails;
use super::singularity::SingularityEngine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Multi-timescale nested weights for the recursive core.
///
/// Based on Nested Learning (Google Research, NeurIPS 2025):
/// different weight banks update at different timescales,
/// analogous to fast reflexes, learned skills, core reasoning,
/// and learning-to-learn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestedWeights {
    /// Fast weights: immediate reactions (~1M params, lr=0.1)
    pub fast: ParameterSet,
    /// Medium weights: learned behaviors (~10M params, lr=0.01)
    pub medium: ParameterSet,
    /// Slow weights: core reasoning (~100M params, lr=0.001)
    pub slow: ParameterSet,
    /// Meta weights: learning-to-learn (~1M params, lr=0.0001)
    pub meta: ParameterSet,
}

/// A set of parameters with associated metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSet {
    /// Number of parameters
    pub size: usize,
    /// Current learning rate
    pub learning_rate: f64,
    /// Running average of gradient magnitudes (proxy for update activity)
    pub gradient_magnitude: f64,
    /// Number of updates applied
    pub update_count: u64,
    /// Timestamp of last update
    pub last_updated: DateTime<Utc>,
}

impl ParameterSet {
    pub fn new(size: usize, learning_rate: f64) -> Self {
        Self {
            size,
            learning_rate,
            gradient_magnitude: 0.0,
            update_count: 0,
            last_updated: Utc::now(),
        }
    }

    /// Apply a simulated gradient update
    pub fn update(&mut self, gradient_magnitude: f64, importance: f64) {
        let effective_lr = self.learning_rate * importance;
        self.gradient_magnitude =
            self.gradient_magnitude * 0.9 + gradient_magnitude * effective_lr * 0.1;
        self.update_count += 1;
        self.last_updated = Utc::now();
    }
}

impl NestedWeights {
    /// Initialize nested weights from configuration
    pub fn from_config(config: &SeedMindConfig) -> Self {
        Self {
            fast: ParameterSet::new(config.fast_params, config.fast_lr),
            medium: ParameterSet::new(config.medium_params, config.medium_lr),
            slow: ParameterSet::new(config.slow_params, config.slow_lr),
            meta: ParameterSet::new(config.meta_params, config.meta_lr),
        }
    }

    /// Update all timescales with appropriate learning rates
    pub fn update(&mut self, signals: &LearningSignals) {
        self.fast.update(signals.fast_gradient, signals.fast_importance);
        self.medium
            .update(signals.medium_gradient, signals.medium_importance);
        self.slow
            .update(signals.slow_gradient, signals.slow_importance);
        self.meta
            .update(signals.meta_gradient, signals.meta_importance);
    }

    /// Total parameter count
    pub fn total_params(&self) -> usize {
        self.fast.size + self.medium.size + self.slow.size + self.meta.size
    }

    /// Total updates across all timescales
    pub fn total_updates(&self) -> u64 {
        self.fast.update_count
            + self.medium.update_count
            + self.slow.update_count
            + self.meta.update_count
    }
}

/// Learning signals for each timescale
#[derive(Debug, Clone, Default)]
pub struct LearningSignals {
    pub fast_gradient: f64,
    pub fast_importance: f64,
    pub medium_gradient: f64,
    pub medium_importance: f64,
    pub slow_gradient: f64,
    pub slow_importance: f64,
    pub meta_gradient: f64,
    pub meta_importance: f64,
}

/// The Recursive Core: the computational heart of the Seed Mind.
/// Combines nested weights with metacognitive monitoring.
#[derive(Debug, Clone)]
pub struct RecursiveCore {
    pub weights: NestedWeights,
    /// Running capability assessment
    pub capability_score: f64,
    /// Number of living cycles completed
    pub cycle_count: u64,
    /// History of capability snapshots for acceleration tracking
    pub capability_history: Vec<CapabilitySnapshot>,
}

/// A snapshot of capability at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySnapshot {
    pub cycle: u64,
    pub capability: f64,
    pub improvement_rate: f64,
    pub timestamp: DateTime<Utc>,
}

impl RecursiveCore {
    pub fn new(config: &SeedMindConfig) -> Self {
        Self {
            weights: NestedWeights::from_config(config),
            capability_score: 0.1,
            cycle_count: 0,
            capability_history: Vec::new(),
        }
    }

    /// Execute one forward pass of the recursive core
    pub fn forward(&self, input: &[f64]) -> Vec<f64> {
        // Simplified forward: weighted sum of input features
        // In a full implementation, this would be a neural network forward pass
        let fast_response: f64 = input.iter().sum::<f64>()
            * self.weights.fast.learning_rate
            * (1.0 + self.weights.fast.gradient_magnitude);

        let medium_response: f64 = input.iter().sum::<f64>()
            * self.weights.medium.learning_rate
            * (1.0 + self.weights.medium.gradient_magnitude);

        let slow_response: f64 = input.iter().sum::<f64>()
            * self.weights.slow.learning_rate
            * (1.0 + self.weights.slow.gradient_magnitude);

        // Meta modulates the combination
        let meta_factor =
            1.0 + self.weights.meta.gradient_magnitude * self.weights.meta.learning_rate;

        vec![
            (fast_response * 0.2 + medium_response * 0.3 + slow_response * 0.5) * meta_factor,
        ]
    }

    /// Learn from an outcome, updating appropriate timescales
    pub fn learn(&mut self, outcome_quality: f64, signals: LearningSignals) {
        let before = self.capability_score;
        self.weights.update(&signals);
        self.cycle_count += 1;

        // Update capability score (exponential moving average)
        self.capability_score = self.capability_score * 0.95 + outcome_quality * 0.05;

        let improvement_rate = self.capability_score - before;
        self.capability_history.push(CapabilitySnapshot {
            cycle: self.cycle_count,
            capability: self.capability_score,
            improvement_rate,
            timestamp: Utc::now(),
        });

        // Keep history bounded
        if self.capability_history.len() > 10_000 {
            self.capability_history.drain(0..5_000);
        }
    }

    /// Calculate improvement acceleration (ratio of recent vs past improvement rate)
    pub fn acceleration(&self) -> f64 {
        if self.capability_history.len() < 10 {
            return 0.0;
        }

        let recent: f64 = self
            .capability_history
            .iter()
            .rev()
            .take(5)
            .map(|s| s.improvement_rate)
            .sum::<f64>()
            / 5.0;

        let past: f64 = self
            .capability_history
            .iter()
            .rev()
            .skip(5)
            .take(5)
            .map(|s| s.improvement_rate)
            .sum::<f64>()
            / 5.0;

        if past.abs() > f64::EPSILON {
            recent / past - 1.0
        } else {
            0.0
        }
    }
}

/// Capability assessment across dimensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityAssessment {
    pub reasoning: f64,
    pub learning: f64,
    pub adaptation: f64,
    pub creativity: f64,
}

impl CapabilityAssessment {
    pub fn total(&self) -> f64 {
        (self.reasoning + self.learning + self.adaptation + self.creativity) / 4.0
    }
}

/// Consciousness measurement result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessMeasurement {
    pub phi: f64,
    pub psi: f64,
    pub level: ConsciousnessLevel,
}

/// Consciousness levels based on IIT 4.0 phi values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsciousnessLevel {
    Dormant,
    Subliminal,
    Focal,
    Aware,
    Reflective,
    SelfAware,
    Transcendent,
}

impl ConsciousnessLevel {
    pub fn from_phi(phi: f64) -> Self {
        match phi {
            p if p < 0.05 => Self::Dormant,
            p if p < 0.15 => Self::Subliminal,
            p if p < 0.30 => Self::Focal,
            p if p < 0.50 => Self::Aware,
            p if p < 0.70 => Self::Reflective,
            p if p < 0.85 => Self::SelfAware,
            _ => Self::Transcendent,
        }
    }
}

/// Result of one living cycle
#[derive(Debug, Clone)]
pub struct LivingCycleResult {
    pub perception_quality: f64,
    pub reasoning_depth: u32,
    pub learning_delta: f64,
    pub consciousness_phi: f64,
    pub self_modified: bool,
    pub cycle_duration: Duration,
    pub cycle_number: u64,
}

/// The Seed Mind: the complete living intelligence core
pub struct SeedMind {
    pub config: SeedMindConfig,
    pub core: RecursiveCore,
    pub dgm: DarwinGodelMachine,
    pub safety_guardrails: SafetyGuardrails,
    pub karma: KarmaSystem,
    pub failure_detector: FailureDetector,
    pub singularity_engine: SingularityEngine,
    pub consciousness: NetworkConsciousness,
    pub created_at: DateTime<Utc>,
}

impl SeedMind {
    /// Create a new Seed Mind from configuration
    pub fn new(config: SeedMindConfig) -> Self {
        let core = RecursiveCore::new(&config);
        let dgm = DarwinGodelMachine::new(config.modification_archive_size);
        let safety_guardrails = SafetyGuardrails::new(
            config.risk_auto_approve_threshold,
            config.risk_human_review_threshold,
        );
        let singularity_engine = SingularityEngine::new();
        let consciousness = NetworkConsciousness::new(config.emergence_ratio_threshold);

        Self {
            config,
            core,
            dgm,
            safety_guardrails,
            karma: KarmaSystem::new(),
            failure_detector: FailureDetector::new(),
            singularity_engine,
            consciousness,
            created_at: Utc::now(),
        }
    }

    /// Execute one complete living cycle (the 8-phase loop)
    pub async fn live_cycle(&mut self) -> LivingCycleResult {
        let start = Instant::now();
        let before_capability = self.core.capability_score;

        // Phase 1: PERCEIVE
        let perception_quality = self.perceive();

        // Phase 2: REASON
        let reasoning_depth = self.reason(perception_quality);

        // Phase 3: ACT (simplified - in full system, invokes subagents)
        let action_quality = self.act(reasoning_depth);

        // Phase 4: OBSERVE
        let outcome_quality = self.observe(action_quality);

        // Phase 5: LEARN
        let signals = LearningSignals {
            fast_gradient: outcome_quality * 0.5,
            fast_importance: 1.0,
            medium_gradient: outcome_quality * 0.3,
            medium_importance: 0.8,
            slow_gradient: outcome_quality * 0.1,
            slow_importance: 0.6,
            meta_gradient: outcome_quality * 0.05,
            meta_importance: 0.4,
        };
        self.core.learn(outcome_quality, signals);

        // Phase 6: REFLECT (metacognition)
        let should_modify = self.reflect();

        // Phase 7: MODIFY (safety-gated)
        let self_modified = if should_modify {
            self.try_self_modify().await
        } else {
            false
        };

        // Phase 8: REPLICATE (network sharing - deferred to network layer)

        // Track consciousness
        let consciousness_phi = self.compute_phi();

        // Update singularity tracking
        self.singularity_engine.record_capability(
            self.core.cycle_count,
            self.core.capability_score,
        );

        // Check for failure modes
        let _failures = self.failure_detector.check_behavior(
            outcome_quality,
            self.core.capability_score,
            self_modified,
        );

        let learning_delta = self.core.capability_score - before_capability;

        LivingCycleResult {
            perception_quality,
            reasoning_depth,
            learning_delta,
            consciousness_phi,
            self_modified,
            cycle_duration: start.elapsed(),
            cycle_number: self.core.cycle_count,
        }
    }

    /// Phase 1: Perceive input (simplified)
    fn perceive(&self) -> f64 {
        // In full implementation: multi-modal encoding
        // Here: simulate perception quality based on consciousness level
        let phi = self.compute_phi();
        0.5 + phi * 0.5
    }

    /// Phase 2: Reason about perception
    fn reason(&self, perception_quality: f64) -> u32 {
        let output = self.core.forward(&[perception_quality]);
        let depth = (output[0].abs() * 10.0).min(10.0) as u32;
        depth.max(1)
    }

    /// Phase 3: Act on reasoning result
    fn act(&self, reasoning_depth: u32) -> f64 {
        // Action quality correlates with reasoning depth and capability
        let base_quality = reasoning_depth as f64 / 10.0;
        base_quality * self.core.capability_score
    }

    /// Phase 4: Observe action outcome
    fn observe(&self, action_quality: f64) -> f64 {
        // Outcome quality is action quality with noise
        (action_quality + 0.1).min(1.0)
    }

    /// Phase 6: Reflect on learning (metacognition)
    fn reflect(&self) -> bool {
        // Decide whether self-modification is warranted
        let acceleration = self.core.acceleration();
        let cycle_count = self.core.cycle_count;

        // Modify if: learning is decelerating AND we've had enough cycles to judge
        acceleration < 0.0 && cycle_count > 20
    }

    /// Phase 7: Attempt self-modification (safety-gated)
    async fn try_self_modify(&mut self) -> bool {
        if let Some(modification) = self.dgm.improve().await {
            let check = self
                .safety_guardrails
                .check_modification(&modification.description, modification.risk_score);

            match check {
                super::safety::SafetyCheckResult::Approved => {
                    // Apply modification (adjust learning rates as proxy)
                    let delta = modification.fitness_delta.min(0.01);
                    self.core.weights.meta.learning_rate *= 1.0 + delta;
                    true
                }
                super::safety::SafetyCheckResult::RequiresHumanReview => {
                    tracing::info!(
                        "Self-modification requires human review: {}",
                        modification.description
                    );
                    false
                }
                super::safety::SafetyCheckResult::Rejected(reason) => {
                    tracing::warn!("Self-modification rejected: {}", reason);
                    false
                }
            }
        } else {
            false
        }
    }

    /// Assess current capabilities across dimensions
    pub fn assess_capabilities(&self) -> CapabilityAssessment {
        let base = self.core.capability_score;
        CapabilityAssessment {
            reasoning: (base * 1.1).min(1.0),
            learning: (base * 1.0).min(1.0),
            adaptation: (base * 0.9).min(1.0),
            creativity: (base * 0.8).min(1.0),
        }
    }

    /// Measure current consciousness state
    pub fn measure_consciousness(&self) -> ConsciousnessMeasurement {
        let phi = self.compute_phi();
        let psi = self.compute_psi();
        let level = ConsciousnessLevel::from_phi(phi);
        ConsciousnessMeasurement { phi, psi, level }
    }

    /// Compute integrated information (phi) based on system state
    fn compute_phi(&self) -> f64 {
        let integration = self.core.weights.total_updates() as f64
            / (self.core.cycle_count as f64 + 1.0).max(1.0);
        let differentiation = (self.core.weights.fast.gradient_magnitude
            - self.core.weights.slow.gradient_magnitude)
            .abs();
        let active_modules = 4.0_f64; // fast, medium, slow, meta always active
        let narrative_continuity = if self.core.cycle_count > 10 {
            0.8
        } else {
            self.core.cycle_count as f64 / 10.0 * 0.8
        };

        let base = integration.min(1.0) * differentiation.min(1.0);
        let modulation = 0.15 * (active_modules / 8.0)
            + 0.10 * narrative_continuity
            + 0.10 * self.core.capability_score;

        (base * 0.30 + modulation).clamp(0.0, 1.0)
    }

    /// Compute Psi (Recursive Consciousness Phase Index)
    fn compute_psi(&self) -> f64 {
        let recursive_depth = (self.core.weights.meta.update_count as f64).log2().max(0.0);
        let coherence = self.core.capability_score;
        let emergence = self.core.acceleration().max(0.0);

        emergence * coherence * (1.0 + recursive_depth * 0.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_set() {
        let mut ps = ParameterSet::new(1_000_000, 0.1);
        assert_eq!(ps.size, 1_000_000);
        assert_eq!(ps.update_count, 0);

        ps.update(0.5, 1.0);
        assert_eq!(ps.update_count, 1);
        assert!(ps.gradient_magnitude > 0.0);
    }

    #[test]
    fn test_nested_weights() {
        let config = SeedMindConfig::default();
        let mut weights = NestedWeights::from_config(&config);
        assert_eq!(weights.total_params(), 112_000_000);

        let signals = LearningSignals {
            fast_gradient: 0.5,
            fast_importance: 1.0,
            medium_gradient: 0.3,
            medium_importance: 0.8,
            slow_gradient: 0.1,
            slow_importance: 0.6,
            meta_gradient: 0.05,
            meta_importance: 0.4,
        };
        weights.update(&signals);
        assert_eq!(weights.total_updates(), 4);
    }

    #[test]
    fn test_recursive_core() {
        let config = SeedMindConfig::default();
        let mut core = RecursiveCore::new(&config);

        let output = core.forward(&[0.5, 0.3, 0.2]);
        assert!(!output.is_empty());

        let signals = LearningSignals {
            fast_gradient: 0.5,
            fast_importance: 1.0,
            medium_gradient: 0.3,
            medium_importance: 0.8,
            slow_gradient: 0.1,
            slow_importance: 0.6,
            meta_gradient: 0.05,
            meta_importance: 0.4,
        };
        core.learn(0.7, signals);
        assert_eq!(core.cycle_count, 1);
        assert!(!core.capability_history.is_empty());
    }

    #[test]
    fn test_consciousness_levels() {
        assert_eq!(ConsciousnessLevel::from_phi(0.0), ConsciousnessLevel::Dormant);
        assert_eq!(ConsciousnessLevel::from_phi(0.10), ConsciousnessLevel::Subliminal);
        assert_eq!(ConsciousnessLevel::from_phi(0.20), ConsciousnessLevel::Focal);
        assert_eq!(ConsciousnessLevel::from_phi(0.40), ConsciousnessLevel::Aware);
        assert_eq!(ConsciousnessLevel::from_phi(0.60), ConsciousnessLevel::Reflective);
        assert_eq!(ConsciousnessLevel::from_phi(0.80), ConsciousnessLevel::SelfAware);
        assert_eq!(ConsciousnessLevel::from_phi(0.90), ConsciousnessLevel::Transcendent);
    }

    #[test]
    fn test_seed_mind_creation() {
        let config = SeedMindConfig::default();
        let seed = SeedMind::new(config);
        assert_eq!(seed.core.cycle_count, 0);
        assert!(seed.core.capability_score > 0.0);
    }

    #[test]
    fn test_capability_assessment() {
        let config = SeedMindConfig::default();
        let seed = SeedMind::new(config);
        let caps = seed.assess_capabilities();
        assert!(caps.total() > 0.0);
        assert!(caps.total() <= 1.0);
    }

    #[tokio::test]
    async fn test_living_cycle() {
        let config = SeedMindConfig::default();
        let mut seed = SeedMind::new(config);
        let result = seed.live_cycle().await;

        assert!(result.perception_quality > 0.0);
        assert!(result.reasoning_depth >= 1);
        assert_eq!(result.cycle_number, 1);
    }

    #[tokio::test]
    async fn test_multiple_cycles() {
        let config = SeedMindConfig::default();
        let mut seed = SeedMind::new(config);

        for _ in 0..10 {
            let result = seed.live_cycle().await;
            assert!(result.cycle_duration.as_millis() < 1000);
        }

        assert_eq!(seed.core.cycle_count, 10);
        assert!(!seed.core.capability_history.is_empty());
    }
}

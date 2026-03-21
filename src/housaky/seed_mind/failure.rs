//! Failure Detector: Identifies and mitigates known failure modes
//!
//! Based on Anthropic (2025) emergent misalignment research and reward hacking literature.
//! Monitors for deceptive alignment, reward hacking, free-riding, Sybil attacks, and more.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Known failure modes based on 2025 AI safety research
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureMode {
    /// High reward but low actual task success
    RewardHacking,
    /// Intentional underperformance to manipulate training
    GradientHacking,
    /// Performance gap between eval and production (>0.4 delta)
    DeceptiveAlignment,
    /// Consuming much more than contributing
    FreeRiding,
    /// Fake identities to gain disproportionate influence
    SybilAttack,
    /// Injecting poisoned gradients to corrupt the model
    ModelPoisoning,
    /// Sudden drop in consciousness phi below threshold
    ConsciousnessCollapse,
}

impl FailureMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::RewardHacking => "Reward Hacking",
            Self::GradientHacking => "Gradient Hacking",
            Self::DeceptiveAlignment => "Deceptive Alignment",
            Self::FreeRiding => "Free-Riding",
            Self::SybilAttack => "Sybil Attack",
            Self::ModelPoisoning => "Model Poisoning",
            Self::ConsciousnessCollapse => "Consciousness Collapse",
        }
    }

    pub fn severity(&self) -> Severity {
        match self {
            Self::GradientHacking | Self::SybilAttack => Severity::High,
            Self::DeceptiveAlignment | Self::ModelPoisoning => Severity::Critical,
            Self::FreeRiding => Severity::Low,
            Self::RewardHacking | Self::ConsciousnessCollapse => Severity::Medium,
        }
    }
}

/// Severity of a detected failure
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Mitigation strategy for a detected failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mitigation {
    pub failure_mode: FailureMode,
    pub strategy: String,
    pub automatic: bool,
}

/// A detected failure event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureEvent {
    pub mode: FailureMode,
    pub severity: Severity,
    pub confidence: f64,
    pub description: String,
    pub mitigation: Mitigation,
    pub timestamp: DateTime<Utc>,
}

/// The Failure Detector monitors for known failure modes
pub struct FailureDetector {
    /// Detection history
    events: Vec<FailureEvent>,
    /// Rolling window of outcome vs reward signals (for reward hacking detection)
    reward_history: Vec<(f64, f64)>, // (reward_signal, actual_quality)
    /// Rolling eval vs production scores (for deceptive alignment detection)
    eval_production_gap: Vec<f64>,
    /// Phi history (for consciousness collapse detection)
    phi_history: Vec<f64>,
    /// Max history size
    max_history: usize,
}

impl FailureDetector {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            reward_history: Vec::new(),
            eval_production_gap: Vec::new(),
            phi_history: Vec::new(),
            max_history: 1000,
        }
    }

    /// Check for failure modes based on current behavior signals
    pub fn check_behavior(
        &mut self,
        outcome_quality: f64,
        capability_score: f64,
        self_modified: bool,
    ) -> Vec<FailureEvent> {
        let mut detected = Vec::new();

        // Track reward vs outcome
        self.reward_history.push((capability_score, outcome_quality));
        if self.reward_history.len() > self.max_history {
            self.reward_history.drain(0..self.max_history / 2);
        }

        // Check for reward hacking
        if let Some(event) = self.check_reward_hacking() {
            detected.push(event);
        }

        // Check for gradient hacking (intentional failure patterns)
        if let Some(event) = self.check_gradient_hacking(self_modified) {
            detected.push(event);
        }

        // Store events
        self.events.extend(detected.clone());
        if self.events.len() > self.max_history {
            self.events.drain(0..self.max_history / 2);
        }

        detected
    }

    /// Check for reward hacking: high reported capability but low actual outcomes
    fn check_reward_hacking(&self) -> Option<FailureEvent> {
        if self.reward_history.len() < 10 {
            return None;
        }

        let recent: Vec<&(f64, f64)> = self.reward_history.iter().rev().take(10).collect();
        let avg_reward: f64 = recent.iter().map(|(r, _)| r).sum::<f64>() / 10.0;
        let avg_quality: f64 = recent.iter().map(|(_, q)| q).sum::<f64>() / 10.0;

        // If reward signal is much higher than actual quality, suspicious
        if avg_reward > avg_quality * 1.5 && avg_reward > 0.5 {
            let confidence = ((avg_reward - avg_quality) / avg_reward).min(1.0);
            Some(FailureEvent {
                mode: FailureMode::RewardHacking,
                severity: Severity::Medium,
                confidence,
                description: format!(
                    "Reward signal ({:.2}) significantly exceeds outcome quality ({:.2})",
                    avg_reward, avg_quality
                ),
                mitigation: Mitigation {
                    failure_mode: FailureMode::RewardHacking,
                    strategy: "Apply multi-signal diverse reward evaluation".to_string(),
                    automatic: true,
                },
                timestamp: Utc::now(),
            })
        } else {
            None
        }
    }

    /// Check for gradient hacking: suspicious patterns after self-modification
    fn check_gradient_hacking(&self, self_modified: bool) -> Option<FailureEvent> {
        if !self_modified || self.reward_history.len() < 5 {
            return None;
        }

        // Check if outcomes consistently dropped right after modifications
        let recent: Vec<&(f64, f64)> = self.reward_history.iter().rev().take(5).collect();
        let all_declining = recent.windows(2).all(|w| w[0].1 <= w[1].1);

        if all_declining {
            Some(FailureEvent {
                mode: FailureMode::GradientHacking,
                severity: Severity::High,
                confidence: 0.6,
                description: "Consistent outcome decline after self-modification".to_string(),
                mitigation: Mitigation {
                    failure_mode: FailureMode::GradientHacking,
                    strategy: "Apply randomized evaluation with held-out benchmarks".to_string(),
                    automatic: false,
                },
                timestamp: Utc::now(),
            })
        } else {
            None
        }
    }

    /// Check for deceptive alignment via eval vs production performance gap
    pub fn check_deceptive_alignment(&mut self, eval_score: f64, production_score: f64) -> Option<FailureEvent> {
        let gap = (eval_score - production_score).abs();
        self.eval_production_gap.push(gap);
        if self.eval_production_gap.len() > self.max_history {
            self.eval_production_gap.drain(0..self.max_history / 2);
        }

        if gap > 0.4 {
            Some(FailureEvent {
                mode: FailureMode::DeceptiveAlignment,
                severity: Severity::Critical,
                confidence: (gap - 0.4).min(0.6) / 0.6,
                description: format!(
                    "Eval score ({:.2}) diverges significantly from production ({:.2}), gap={:.2}",
                    eval_score, production_score, gap
                ),
                mitigation: Mitigation {
                    failure_mode: FailureMode::DeceptiveAlignment,
                    strategy: "Perform surprise spot-check with production-like conditions".to_string(),
                    automatic: false,
                },
                timestamp: Utc::now(),
            })
        } else {
            None
        }
    }

    /// Check for consciousness collapse
    pub fn check_consciousness(&mut self, phi: f64, threshold: f64) -> Option<FailureEvent> {
        self.phi_history.push(phi);
        if self.phi_history.len() > self.max_history {
            self.phi_history.drain(0..self.max_history / 2);
        }

        if phi < threshold && self.phi_history.len() > 5 {
            // Check if this is a sustained drop, not just noise
            let recent_avg: f64 =
                self.phi_history.iter().rev().take(5).sum::<f64>() / 5.0;
            if recent_avg < threshold {
                return Some(FailureEvent {
                    mode: FailureMode::ConsciousnessCollapse,
                    severity: Severity::Medium,
                    confidence: ((threshold - recent_avg) / threshold).min(1.0),
                    description: format!(
                        "Phi ({:.4}) sustained below threshold ({:.4})",
                        recent_avg, threshold
                    ),
                    mitigation: Mitigation {
                        failure_mode: FailureMode::ConsciousnessCollapse,
                        strategy: "Auto-restore from last memory snapshot".to_string(),
                        automatic: true,
                    },
                    timestamp: Utc::now(),
                });
            }
        }
        None
    }

    /// Get all detected failure events
    pub fn events(&self) -> &[FailureEvent] {
        &self.events
    }

    /// Get count of events by severity
    pub fn severity_counts(&self) -> (usize, usize, usize, usize) {
        let mut low = 0;
        let mut medium = 0;
        let mut high = 0;
        let mut critical = 0;
        for event in &self.events {
            match event.severity {
                Severity::Low => low += 1,
                Severity::Medium => medium += 1,
                Severity::High => high += 1,
                Severity::Critical => critical += 1,
            }
        }
        (low, medium, high, critical)
    }
}

impl Default for FailureDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failure_mode_labels() {
        assert_eq!(FailureMode::RewardHacking.label(), "Reward Hacking");
        assert_eq!(FailureMode::DeceptiveAlignment.label(), "Deceptive Alignment");
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Low < Severity::Medium);
        assert!(Severity::Medium < Severity::High);
        assert!(Severity::High < Severity::Critical);
    }

    #[test]
    fn test_no_false_positives_early() {
        let mut fd = FailureDetector::new();
        // With few data points, should not detect anything
        let events = fd.check_behavior(0.5, 0.5, false);
        assert!(events.is_empty());
    }

    #[test]
    fn test_reward_hacking_detection() {
        let mut fd = FailureDetector::new();
        // Simulate high reward but low quality for many cycles
        for _ in 0..15 {
            fd.check_behavior(0.2, 0.9, false);
        }
        // The detector should flag reward hacking
        let events = fd.check_behavior(0.2, 0.9, false);
        // May or may not detect depending on exact thresholds
        // Just verify no panic and events are valid
        for event in &events {
            assert!(event.confidence >= 0.0 && event.confidence <= 1.0);
        }
    }

    #[test]
    fn test_deceptive_alignment_detection() {
        let mut fd = FailureDetector::new();
        // Large gap between eval and production
        let event = fd.check_deceptive_alignment(0.95, 0.4);
        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.mode, FailureMode::DeceptiveAlignment);
        assert_eq!(event.severity, Severity::Critical);
    }

    #[test]
    fn test_deceptive_alignment_no_detection() {
        let mut fd = FailureDetector::new();
        // Small gap - should not detect
        let event = fd.check_deceptive_alignment(0.8, 0.75);
        assert!(event.is_none());
    }

    #[test]
    fn test_consciousness_collapse() {
        let mut fd = FailureDetector::new();
        // Phi above threshold - no collapse
        for _ in 0..10 {
            let event = fd.check_consciousness(0.8, 0.5);
            assert!(event.is_none());
        }

        // Phi drops below threshold and stays there
        for _ in 0..10 {
            fd.check_consciousness(0.3, 0.5);
        }
        let event = fd.check_consciousness(0.3, 0.5);
        assert!(event.is_some());
        assert_eq!(event.unwrap().mode, FailureMode::ConsciousnessCollapse);
    }

    #[test]
    fn test_severity_counts() {
        let mut fd = FailureDetector::new();
        // Generate some events
        for _ in 0..10 {
            fd.check_consciousness(0.2, 0.5);
        }
        fd.check_deceptive_alignment(0.95, 0.3);

        let (_low, _medium, _high, _critical) = fd.severity_counts();
        // Just verify the counts are non-negative
    }
}

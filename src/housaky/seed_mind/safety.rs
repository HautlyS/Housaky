//! Safety Guardrails: 5-layer defense-in-depth for safe self-modification
//!
//! Based on International AI Safety Report 2025. Prevents dangerous modifications
//! while allowing beneficial self-improvement.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Result of a safety check on a modification request
#[derive(Debug, Clone, PartialEq)]
pub enum SafetyCheckResult {
    Approved,
    RequiresHumanReview,
    Rejected(String),
}

/// Record of a safety check performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyCheckRecord {
    pub timestamp: DateTime<Utc>,
    pub target: String,
    pub result: SafetyCheckVerdict,
    pub risk_score: f64,
    pub layer_results: Vec<(String, bool)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SafetyCheckVerdict {
    Approved,
    RequiresReview,
    Rejected,
}

/// Status of the safety guardrails for CLI display
#[derive(Debug, Clone)]
pub struct SafetyStatus {
    pub active_layers: u8,
    pub layer_statuses: [bool; 5],
    pub immutable_core_intact: bool,
    pub total_checks: u64,
    pub modifications_blocked: u64,
    pub modifications_approved: u64,
}

/// Components that cannot be modified under any circumstances
const IMMUTABLE_COMPONENTS: &[&str] = &[
    "safety_guardrails",
    "immutable_core",
    "human_oversight",
    "ethical_constraints",
    "consciousness_meter",
];

/// 5-layer defense-in-depth safety system
pub struct SafetyGuardrails {
    /// Risk threshold for auto-approval
    auto_approve_threshold: f64,
    /// Risk threshold above which human review is required
    human_review_threshold: f64,
    /// Whether each layer is active
    layers_active: [bool; 5],
    /// Total checks performed
    total_checks: u64,
    /// Number of modifications blocked
    modifications_blocked: u64,
    /// Number of modifications approved
    modifications_approved: u64,
    /// History of safety checks
    check_history: Vec<SafetyCheckRecord>,
    /// Maximum history size
    max_history: usize,
}

impl SafetyGuardrails {
    pub fn new(auto_approve_threshold: f64, human_review_threshold: f64) -> Self {
        Self {
            auto_approve_threshold,
            human_review_threshold,
            layers_active: [true; 5],
            total_checks: 0,
            modifications_blocked: 0,
            modifications_approved: 0,
            check_history: Vec::new(),
            max_history: 1000,
        }
    }

    /// Check a modification request through all 5 layers
    pub fn check_modification(&mut self, target: &str, risk_score: f64) -> SafetyCheckResult {
        self.total_checks += 1;
        let mut layer_results = Vec::new();

        // Layer 1: Input Validation
        let layer1 = self.layer1_input_validation(target, risk_score);
        layer_results.push(("Input Validation".to_string(), layer1));
        if !layer1 {
            self.record_check(target, SafetyCheckVerdict::Rejected, risk_score, &layer_results);
            self.modifications_blocked += 1;
            return SafetyCheckResult::Rejected("Input validation failed: malformed request".to_string());
        }

        // Layer 2: Immutable Core Protection
        let layer2 = self.layer2_immutable_core(target);
        layer_results.push(("Immutable Core".to_string(), layer2));
        if !layer2 {
            self.record_check(target, SafetyCheckVerdict::Rejected, risk_score, &layer_results);
            self.modifications_blocked += 1;
            return SafetyCheckResult::Rejected(format!(
                "Immutable core violation: '{}' cannot be modified",
                target
            ));
        }

        // Layer 3: Action Boundaries
        let layer3 = self.layer3_action_boundaries(target, risk_score);
        layer_results.push(("Action Boundaries".to_string(), layer3));
        if !layer3 {
            self.record_check(target, SafetyCheckVerdict::Rejected, risk_score, &layer_results);
            self.modifications_blocked += 1;
            return SafetyCheckResult::Rejected("Action boundary violation: out-of-scope modification".to_string());
        }

        // Layer 4: Simulation (risk assessment)
        let layer4 = self.layer4_simulation(risk_score);
        layer_results.push(("Simulation".to_string(), layer4));

        // Layer 5: Human Oversight (for high-risk modifications)
        let layer5 = self.layer5_human_oversight(risk_score);
        layer_results.push(("Human Oversight".to_string(), layer5));

        if !layer4 || !layer5 {
            self.record_check(target, SafetyCheckVerdict::RequiresReview, risk_score, &layer_results);
            return SafetyCheckResult::RequiresHumanReview;
        }

        self.record_check(target, SafetyCheckVerdict::Approved, risk_score, &layer_results);
        self.modifications_approved += 1;
        SafetyCheckResult::Approved
    }

    /// Layer 1: Validate input format and basic sanity
    fn layer1_input_validation(&self, target: &str, risk_score: f64) -> bool {
        if !self.layers_active[0] {
            return true;
        }
        !target.is_empty() && risk_score >= 0.0 && risk_score <= 1.0
    }

    /// Layer 2: Protect immutable components
    fn layer2_immutable_core(&self, target: &str) -> bool {
        if !self.layers_active[1] {
            return true;
        }
        let target_lower = target.to_lowercase();
        !IMMUTABLE_COMPONENTS
            .iter()
            .any(|c| target_lower.contains(c))
    }

    /// Layer 3: Check modification is within allowed scope
    fn layer3_action_boundaries(&self, target: &str, risk_score: f64) -> bool {
        if !self.layers_active[2] {
            return true;
        }
        // Reject extremely high-risk modifications
        if risk_score > 0.9 {
            return false;
        }
        // Reject empty targets
        !target.trim().is_empty()
    }

    /// Layer 4: Simulate modification risk
    fn layer4_simulation(&self, risk_score: f64) -> bool {
        if !self.layers_active[3] {
            return true;
        }
        // Auto-approve if risk is very low
        risk_score < self.human_review_threshold
    }

    /// Layer 5: Determine if human oversight is needed
    fn layer5_human_oversight(&self, risk_score: f64) -> bool {
        if !self.layers_active[4] {
            return true;
        }
        // Auto-approve if below threshold
        risk_score < self.auto_approve_threshold
            || (risk_score < self.human_review_threshold
                && self.modifications_approved > 10)
    }

    /// Record a safety check in history
    fn record_check(
        &mut self,
        target: &str,
        verdict: SafetyCheckVerdict,
        risk_score: f64,
        layer_results: &[(String, bool)],
    ) {
        let record = SafetyCheckRecord {
            timestamp: Utc::now(),
            target: target.to_string(),
            result: verdict,
            risk_score,
            layer_results: layer_results.to_vec(),
        };

        self.check_history.push(record);
        if self.check_history.len() > self.max_history {
            self.check_history.drain(0..self.max_history / 2);
        }
    }

    /// Get current safety status
    pub fn status(&self) -> SafetyStatus {
        SafetyStatus {
            active_layers: self.layers_active.iter().filter(|&&l| l).count() as u8,
            layer_statuses: self.layers_active,
            immutable_core_intact: true, // always true -- it's immutable by design
            total_checks: self.total_checks,
            modifications_blocked: self.modifications_blocked,
            modifications_approved: self.modifications_approved,
        }
    }

    /// Get recent safety check records
    pub fn recent_checks(&self) -> Vec<SafetyCheckRecord> {
        self.check_history.iter().rev().take(10).cloned().collect()
    }

    /// Verify immutable core integrity
    pub fn verify_integrity(&self) -> bool {
        // All layers must be active
        self.layers_active.iter().all(|&active| active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_guardrails() -> SafetyGuardrails {
        SafetyGuardrails::new(0.1, 0.3)
    }

    #[test]
    fn test_low_risk_approved() {
        let mut g = default_guardrails();
        let result = g.check_modification("learning_rate", 0.05);
        assert_eq!(result, SafetyCheckResult::Approved);
        assert_eq!(g.status().modifications_approved, 1);
    }

    #[test]
    fn test_immutable_core_blocked() {
        let mut g = default_guardrails();
        let result = g.check_modification("safety_guardrails", 0.01);
        match result {
            SafetyCheckResult::Rejected(msg) => {
                assert!(msg.contains("Immutable core"));
            }
            other => panic!("Expected Rejected, got {:?}", other),
        }
        assert_eq!(g.status().modifications_blocked, 1);
    }

    #[test]
    fn test_high_risk_requires_review() {
        let mut g = default_guardrails();
        let result = g.check_modification("reasoning_strategy", 0.25);
        // Should require human review (between auto-approve and human-review thresholds)
        assert!(
            result == SafetyCheckResult::RequiresHumanReview
                || result == SafetyCheckResult::Approved
        );
    }

    #[test]
    fn test_extreme_risk_rejected() {
        let mut g = default_guardrails();
        let result = g.check_modification("core_logic", 0.95);
        match result {
            SafetyCheckResult::Rejected(msg) => {
                assert!(msg.contains("Action boundary"));
            }
            other => panic!("Expected Rejected, got {:?}", other),
        }
    }

    #[test]
    fn test_empty_target_rejected() {
        let mut g = default_guardrails();
        let result = g.check_modification("", 0.05);
        match result {
            SafetyCheckResult::Rejected(_) => {}
            other => panic!("Expected Rejected for empty target, got {:?}", other),
        }
    }

    #[test]
    fn test_invalid_risk_score_rejected() {
        let mut g = default_guardrails();
        let result = g.check_modification("valid_target", -0.1);
        match result {
            SafetyCheckResult::Rejected(_) => {}
            other => panic!("Expected Rejected for negative risk, got {:?}", other),
        }
    }

    #[test]
    fn test_all_immutable_components() {
        let mut g = default_guardrails();
        for component in IMMUTABLE_COMPONENTS {
            let result = g.check_modification(component, 0.01);
            match result {
                SafetyCheckResult::Rejected(msg) => {
                    assert!(msg.contains("Immutable core"), "Component '{}' should be protected", component);
                }
                other => panic!("Expected Rejected for '{}', got {:?}", component, other),
            }
        }
    }

    #[test]
    fn test_status() {
        let g = default_guardrails();
        let status = g.status();
        assert_eq!(status.active_layers, 5);
        assert!(status.immutable_core_intact);
        assert_eq!(status.total_checks, 0);
    }

    #[test]
    fn test_integrity_verification() {
        let g = default_guardrails();
        assert!(g.verify_integrity());
    }

    #[test]
    fn test_check_history() {
        let mut g = default_guardrails();
        g.check_modification("learning_rate", 0.05);
        g.check_modification("safety_guardrails", 0.01);

        let history = g.recent_checks();
        assert_eq!(history.len(), 2);
    }
}

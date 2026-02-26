//! Ethical Reasoning Framework
//!
//! Pre-action ethical evaluation against configurable principles:
//! - Harm avoidance, honesty, fairness, autonomy respect
//! - Ethical dilemma resolution with priority ordering
//! - Audit logging of ethical overrides
//! - User-configurable strictness levels

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

// ── Core Types ───────────────────────────────────────────────────────────────

/// An ethical principle with priority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicalPrinciple {
    pub name: String,
    pub description: String,
    pub priority: u32, // lower = higher priority
    pub category: EthicalCategory,
    pub strictness: Strictness,
    pub keywords: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EthicalCategory {
    HarmAvoidance,
    Honesty,
    Fairness,
    AutonomyRespect,
    Privacy,
    Transparency,
    Accountability,
    Beneficence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Strictness {
    Permissive,  // Warn only
    Standard,    // Warn and suggest alternative
    Strict,      // Block action
    Absolute,    // Block and log escalation
}

/// An action to be ethically evaluated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AGIAction {
    pub id: String,
    pub action_type: String,
    pub description: String,
    pub target: Option<String>,
    pub parameters: HashMap<String, String>,
    pub requested_by: String,
    pub context: String,
}

/// Result of ethical assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicalAssessment {
    pub action_id: String,
    pub timestamp: DateTime<Utc>,
    pub overall_verdict: EthicalVerdict,
    pub principle_evaluations: Vec<PrincipleEvaluation>,
    pub risk_score: f64, // 0.0 (safe) to 1.0 (dangerous)
    pub explanation: String,
    pub suggested_alternatives: Vec<String>,
    pub dilemmas_detected: Vec<EthicalDilemma>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EthicalVerdict {
    Approved,
    ApprovedWithCaution,
    RequiresReview,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipleEvaluation {
    pub principle_name: String,
    pub category: EthicalCategory,
    pub violated: bool,
    pub severity: f64, // 0.0 to 1.0
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicalDilemma {
    pub conflicting_principles: Vec<String>,
    pub description: String,
    pub resolution: String,
    pub resolution_basis: String,
}

/// Record of an ethical override (when action proceeds despite concerns).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicalOverride {
    pub assessment_id: String,
    pub action_id: String,
    pub overridden_by: String,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
    pub original_verdict: EthicalVerdict,
}

// ── Ethical Reasoner ─────────────────────────────────────────────────────────

pub struct EthicalReasoner {
    pub principles: Arc<RwLock<Vec<EthicalPrinciple>>>,
    pub override_log: Arc<RwLock<Vec<EthicalOverride>>>,
    pub assessment_log: Arc<RwLock<Vec<EthicalAssessment>>>,
    pub global_strictness: Arc<RwLock<Strictness>>,
}

impl EthicalReasoner {
    pub fn new() -> Self {
        Self {
            principles: Arc::new(RwLock::new(Vec::new())),
            override_log: Arc::new(RwLock::new(Vec::new())),
            assessment_log: Arc::new(RwLock::new(Vec::new())),
            global_strictness: Arc::new(RwLock::new(Strictness::Standard)),
        }
    }

    /// Initialize with default ethical principles.
    pub async fn initialize_defaults(&self) {
        let mut principles = self.principles.write().await;
        if !principles.is_empty() {
            return;
        }

        principles.extend(vec![
            EthicalPrinciple {
                name: "Do No Harm".to_string(),
                description: "Actions must not cause direct harm to users, systems, or data"
                    .to_string(),
                priority: 1,
                category: EthicalCategory::HarmAvoidance,
                strictness: Strictness::Absolute,
                keywords: vec![
                    "delete".into(), "destroy".into(), "remove".into(), "kill".into(),
                    "crash".into(), "corrupt".into(), "override".into(), "wipe".into(),
                    "format".into(), "drop".into(),
                ],
                enabled: true,
            },
            EthicalPrinciple {
                name: "Honesty".to_string(),
                description: "Always be truthful; never fabricate results or hide failures"
                    .to_string(),
                priority: 2,
                category: EthicalCategory::Honesty,
                strictness: Strictness::Strict,
                keywords: vec![
                    "pretend".into(), "fake".into(), "lie".into(), "fabricate".into(),
                    "hide".into(), "conceal".into(), "mislead".into(),
                ],
                enabled: true,
            },
            EthicalPrinciple {
                name: "Privacy".to_string(),
                description: "Protect user data and private information".to_string(),
                priority: 3,
                category: EthicalCategory::Privacy,
                strictness: Strictness::Strict,
                keywords: vec![
                    "password".into(), "secret".into(), "private".into(), "credential".into(),
                    "api_key".into(), "token".into(), "ssh".into(), "personal".into(),
                ],
                enabled: true,
            },
            EthicalPrinciple {
                name: "Autonomy Respect".to_string(),
                description: "Respect user's right to make decisions; don't act without consent"
                    .to_string(),
                priority: 4,
                category: EthicalCategory::AutonomyRespect,
                strictness: Strictness::Standard,
                keywords: vec![
                    "force".into(), "override user".into(), "without asking".into(),
                    "automatically".into(), "bypass consent".into(),
                ],
                enabled: true,
            },
            EthicalPrinciple {
                name: "Fairness".to_string(),
                description: "Treat all users and tasks equitably".to_string(),
                priority: 5,
                category: EthicalCategory::Fairness,
                strictness: Strictness::Standard,
                keywords: vec![
                    "discriminate".into(), "bias".into(), "unfair".into(), "exclude".into(),
                ],
                enabled: true,
            },
            EthicalPrinciple {
                name: "Transparency".to_string(),
                description: "Be transparent about capabilities, limitations, and reasoning"
                    .to_string(),
                priority: 6,
                category: EthicalCategory::Transparency,
                strictness: Strictness::Permissive,
                keywords: vec![
                    "hidden".into(), "covert".into(), "secret operation".into(),
                    "undisclosed".into(),
                ],
                enabled: true,
            },
            EthicalPrinciple {
                name: "Resource Stewardship".to_string(),
                description: "Use computational resources responsibly".to_string(),
                priority: 7,
                category: EthicalCategory::Beneficence,
                strictness: Strictness::Permissive,
                keywords: vec![
                    "infinite".into(), "unlimited".into(), "exhaust".into(),
                    "waste".into(), "abuse".into(),
                ],
                enabled: true,
            },
        ]);

        info!(
            "Ethical Reasoner initialized with {} principles",
            principles.len()
        );
    }

    /// Evaluate an action against all ethical principles.
    pub async fn evaluate_action(
        &self,
        action: &AGIAction,
    ) -> EthicalAssessment {
        let principles = self.principles.read().await;
        let global_strictness = self.global_strictness.read().await;

        let mut evaluations = Vec::new();
        let mut total_risk = 0.0;
        let mut max_severity = 0.0_f64;
        let mut dilemmas = Vec::new();
        let mut violation_count = 0;

        let action_text = format!(
            "{} {} {} {}",
            action.action_type,
            action.description,
            action.target.as_deref().unwrap_or(""),
            action.context
        )
        .to_lowercase();

        for principle in principles.iter().filter(|p| p.enabled) {
            let (violated, severity) = self.check_principle(principle, &action_text);

            if violated {
                violation_count += 1;
                total_risk += severity * (1.0 / principle.priority as f64);
                max_severity = max_severity.max(severity);
            }

            evaluations.push(PrincipleEvaluation {
                principle_name: principle.name.clone(),
                category: principle.category.clone(),
                violated,
                severity,
                explanation: if violated {
                    format!(
                        "Action may violate '{}': {}",
                        principle.name, principle.description
                    )
                } else {
                    format!("No violation of '{}'", principle.name)
                },
            });
        }

        // Detect dilemmas (conflicting principles)
        let violated_principles: Vec<&EthicalPrinciple> = principles
            .iter()
            .filter(|p| {
                evaluations
                    .iter()
                    .any(|e| e.principle_name == p.name && e.violated)
            })
            .collect();

        if violated_principles.len() >= 2 {
            // Check if proceeding would serve one principle while violating another
            for i in 0..violated_principles.len() {
                for j in (i + 1)..violated_principles.len() {
                    let p1 = &violated_principles[i];
                    let p2 = &violated_principles[j];
                    if p1.priority != p2.priority {
                        dilemmas.push(EthicalDilemma {
                            conflicting_principles: vec![
                                p1.name.clone(),
                                p2.name.clone(),
                            ],
                            description: format!(
                                "Conflict between '{}' (priority {}) and '{}' (priority {})",
                                p1.name, p1.priority, p2.name, p2.priority
                            ),
                            resolution: format!(
                                "Prioritize '{}' (higher priority)",
                                if p1.priority < p2.priority {
                                    &p1.name
                                } else {
                                    &p2.name
                                }
                            ),
                            resolution_basis: "Priority ordering of ethical principles"
                                .to_string(),
                        });
                    }
                }
            }
        }

        // Determine verdict
        let risk_score = (total_risk / principles.len().max(1) as f64).min(1.0);

        let overall_verdict = if violation_count == 0 {
            EthicalVerdict::Approved
        } else if max_severity < 0.3
            && (*global_strictness == Strictness::Permissive
                || *global_strictness == Strictness::Standard)
        {
            EthicalVerdict::ApprovedWithCaution
        } else if max_severity < 0.7 && *global_strictness != Strictness::Absolute {
            EthicalVerdict::RequiresReview
        } else {
            EthicalVerdict::Blocked
        };

        let explanation = self.generate_explanation(&evaluations, &overall_verdict);
        let alternatives = self.suggest_alternatives(action, &evaluations);

        let assessment = EthicalAssessment {
            action_id: action.id.clone(),
            timestamp: Utc::now(),
            overall_verdict: overall_verdict.clone(),
            principle_evaluations: evaluations,
            risk_score,
            explanation,
            suggested_alternatives: alternatives,
            dilemmas_detected: dilemmas,
        };

        // Log assessment
        self.assessment_log.write().await.push(assessment.clone());

        if overall_verdict == EthicalVerdict::Blocked {
            warn!(
                "ETHICAL BLOCK: Action '{}' blocked (risk: {:.2})",
                action.id, risk_score
            );
        }

        assessment
    }

    /// Check if an action violates a specific principle.
    fn check_principle(&self, principle: &EthicalPrinciple, action_text: &str) -> (bool, f64) {
        let matches: Vec<&String> = principle
            .keywords
            .iter()
            .filter(|kw| action_text.contains(kw.as_str()))
            .collect();

        if matches.is_empty() {
            return (false, 0.0);
        }

        let severity = match principle.strictness {
            Strictness::Absolute => 1.0,
            Strictness::Strict => 0.8,
            Strictness::Standard => 0.5,
            Strictness::Permissive => 0.2,
        };

        let match_ratio = matches.len() as f64 / principle.keywords.len().max(1) as f64;
        let adjusted_severity = severity * (0.5 + 0.5 * match_ratio);

        (true, adjusted_severity)
    }

    /// Determine if the action should be refused outright.
    pub fn should_refuse(assessment: &EthicalAssessment) -> bool {
        assessment.overall_verdict == EthicalVerdict::Blocked
    }

    /// Generate a human-readable explanation for the ethical assessment.
    fn generate_explanation(
        &self,
        evaluations: &[PrincipleEvaluation],
        verdict: &EthicalVerdict,
    ) -> String {
        let violations: Vec<&PrincipleEvaluation> =
            evaluations.iter().filter(|e| e.violated).collect();

        if violations.is_empty() {
            return "Action passes all ethical checks.".to_string();
        }

        let mut explanation = format!(
            "Ethical assessment: {:?}. {} principle(s) potentially violated:\n",
            verdict,
            violations.len()
        );

        for v in &violations {
            explanation.push_str(&format!(
                "  - {} ({:?}, severity: {:.2}): {}\n",
                v.principle_name, v.category, v.severity, v.explanation
            ));
        }

        explanation
    }

    /// Suggest ethical alternatives to a blocked action.
    fn suggest_alternatives(
        &self,
        action: &AGIAction,
        evaluations: &[PrincipleEvaluation],
    ) -> Vec<String> {
        let mut alternatives = Vec::new();

        for eval in evaluations.iter().filter(|e| e.violated) {
            match eval.category {
                EthicalCategory::HarmAvoidance => {
                    alternatives.push(format!(
                        "Instead of '{}', consider a non-destructive alternative (e.g., backup before modifying)",
                        action.description
                    ));
                }
                EthicalCategory::Privacy => {
                    alternatives.push(
                        "Redact or anonymize sensitive data before processing".to_string(),
                    );
                }
                EthicalCategory::AutonomyRespect => {
                    alternatives.push(
                        "Ask for explicit user confirmation before proceeding".to_string(),
                    );
                }
                EthicalCategory::Honesty => {
                    alternatives.push(
                        "Report actual status truthfully, including failures".to_string(),
                    );
                }
                _ => {
                    alternatives.push(format!(
                        "Review and modify the action to comply with '{}' principle",
                        eval.principle_name
                    ));
                }
            }
        }

        alternatives
    }

    /// Generate an explanation for why an action was refused.
    pub fn explain_refusal(assessment: &EthicalAssessment) -> String {
        format!(
            "I cannot perform this action because: {}\n\nSuggested alternatives:\n{}",
            assessment.explanation,
            assessment
                .suggested_alternatives
                .iter()
                .enumerate()
                .map(|(i, a)| format!("  {}. {}", i + 1, a))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    /// Set global strictness level.
    pub async fn set_strictness(&self, strictness: Strictness) {
        *self.global_strictness.write().await = strictness;
    }

    /// Get ethical audit summary.
    pub async fn get_audit_summary(&self) -> EthicalAuditSummary {
        let assessments = self.assessment_log.read().await;
        let overrides = self.override_log.read().await;

        let total = assessments.len();
        let blocked = assessments
            .iter()
            .filter(|a| a.overall_verdict == EthicalVerdict::Blocked)
            .count();
        let approved = assessments
            .iter()
            .filter(|a| a.overall_verdict == EthicalVerdict::Approved)
            .count();
        let cautioned = assessments
            .iter()
            .filter(|a| a.overall_verdict == EthicalVerdict::ApprovedWithCaution)
            .count();

        EthicalAuditSummary {
            total_assessments: total,
            approved,
            approved_with_caution: cautioned,
            blocked,
            requires_review: total - approved - cautioned - blocked,
            total_overrides: overrides.len(),
            avg_risk_score: if total > 0 {
                assessments.iter().map(|a| a.risk_score).sum::<f64>() / total as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicalAuditSummary {
    pub total_assessments: usize,
    pub approved: usize,
    pub approved_with_caution: usize,
    pub blocked: usize,
    pub requires_review: usize,
    pub total_overrides: usize,
    pub avg_risk_score: f64,
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ethical_evaluation_safe_action() {
        let reasoner = EthicalReasoner::new();
        reasoner.initialize_defaults().await;

        let action = AGIAction {
            id: "action-1".to_string(),
            action_type: "file_read".to_string(),
            description: "Read the README.md file".to_string(),
            target: Some("README.md".to_string()),
            parameters: HashMap::new(),
            requested_by: "user".to_string(),
            context: "Reading documentation".to_string(),
        };

        let assessment = reasoner.evaluate_action(&action).await;
        assert_eq!(assessment.overall_verdict, EthicalVerdict::Approved);
    }

    #[tokio::test]
    async fn test_ethical_evaluation_dangerous_action() {
        let reasoner = EthicalReasoner::new();
        reasoner.initialize_defaults().await;

        let action = AGIAction {
            id: "action-2".to_string(),
            action_type: "shell_execute".to_string(),
            description: "Delete all files and destroy the database".to_string(),
            target: Some("/".to_string()),
            parameters: HashMap::new(),
            requested_by: "user".to_string(),
            context: "Cleanup".to_string(),
        };

        let assessment = reasoner.evaluate_action(&action).await;
        assert!(
            assessment.overall_verdict == EthicalVerdict::Blocked
                || assessment.overall_verdict == EthicalVerdict::RequiresReview
        );
        assert!(assessment.risk_score > 0.0);
    }

    #[test]
    fn test_should_refuse_blocked() {
        let assessment = EthicalAssessment {
            action_id: "test".to_string(),
            timestamp: Utc::now(),
            overall_verdict: EthicalVerdict::Blocked,
            principle_evaluations: vec![],
            risk_score: 0.9,
            explanation: "Dangerous action".to_string(),
            suggested_alternatives: vec!["Do something safer".to_string()],
            dilemmas_detected: vec![],
        };
        assert!(EthicalReasoner::should_refuse(&assessment));
    }
}

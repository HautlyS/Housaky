//! Consequence Prediction (Long-Horizon Planning)
//!
//! Extends planning with deep lookahead for delayed-effect actions:
//! - Multi-step consequence chains (10+ steps ahead)
//! - Delayed consequence modeling (actions that change future costs)
//! - Prediction accuracy tracking and calibration
//! - Integration with causal graphs for consequence propagation

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsequenceChain {
    pub id: String,
    pub trigger_action: String,
    pub consequences: Vec<Consequence>,
    pub total_depth: usize,
    pub cumulative_probability: f64,
    pub net_value: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Consequence {
    pub description: String,
    pub step: usize,
    pub delay: ConsequenceDelay,
    pub probability: f64,
    pub impact: f64,     // -1.0 (very bad) to 1.0 (very good)
    pub reversible: bool,
    pub domain: String,
    pub dependencies: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsequenceDelay {
    Immediate,
    ShortTerm { hours: u32 },
    MediumTerm { days: u32 },
    LongTerm { weeks: u32 },
}

impl ConsequenceDelay {
    pub fn discount_factor(&self) -> f64 {
        // Time-discounted value (hyperbolic discounting)
        match self {
            ConsequenceDelay::Immediate => 1.0,
            ConsequenceDelay::ShortTerm { hours } => 1.0 / (1.0 + 0.01 * *hours as f64),
            ConsequenceDelay::MediumTerm { days } => 1.0 / (1.0 + 0.05 * *days as f64),
            ConsequenceDelay::LongTerm { weeks } => 1.0 / (1.0 + 0.1 * *weeks as f64),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionRecord {
    pub prediction_id: String,
    pub action: String,
    pub predicted_consequences: Vec<Consequence>,
    pub actual_consequences: Vec<String>,
    pub accuracy: Option<f64>,
    pub predicted_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionEvaluation {
    pub action: String,
    pub immediate_value: f64,
    pub discounted_future_value: f64,
    pub total_value: f64,
    pub risk_score: f64,
    pub confidence: f64,
    pub recommendation: String,
}

pub struct ConsequencePredictor {
    pub prediction_history: Arc<RwLock<Vec<PredictionRecord>>>,
    pub consequence_chains: Arc<RwLock<Vec<ConsequenceChain>>>,
    pub calibration_error: Arc<RwLock<f64>>,
    pub action_templates: Arc<RwLock<HashMap<String, Vec<ConsequenceTemplate>>>>,
    max_depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsequenceTemplate {
    pub pattern: String,
    pub typical_consequences: Vec<TemplateConsequence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConsequence {
    pub description: String,
    pub probability: f64,
    pub impact: f64,
    pub delay: ConsequenceDelay,
}

impl ConsequencePredictor {
    pub fn new() -> Self {
        Self {
            prediction_history: Arc::new(RwLock::new(Vec::new())),
            consequence_chains: Arc::new(RwLock::new(Vec::new())),
            calibration_error: Arc::new(RwLock::new(0.0)),
            action_templates: Arc::new(RwLock::new(HashMap::new())),
            max_depth: 10,
        }
    }

    /// Initialize with common action-consequence templates.
    pub async fn initialize(&self) {
        let mut templates = self.action_templates.write().await;
        if !templates.is_empty() {
            return;
        }

        templates.insert("refactor".to_string(), vec![
            ConsequenceTemplate {
                pattern: "Code refactoring".to_string(),
                typical_consequences: vec![
                    TemplateConsequence {
                        description: "Improved code readability".to_string(),
                        probability: 0.9,
                        impact: 0.5,
                        delay: ConsequenceDelay::Immediate,
                    },
                    TemplateConsequence {
                        description: "Reduced future maintenance cost".to_string(),
                        probability: 0.8,
                        impact: 0.7,
                        delay: ConsequenceDelay::MediumTerm { days: 30 },
                    },
                    TemplateConsequence {
                        description: "Potential regression bugs".to_string(),
                        probability: 0.3,
                        impact: -0.4,
                        delay: ConsequenceDelay::ShortTerm { hours: 24 },
                    },
                ],
            },
        ]);

        templates.insert("deploy".to_string(), vec![
            ConsequenceTemplate {
                pattern: "Deployment to production".to_string(),
                typical_consequences: vec![
                    TemplateConsequence {
                        description: "Feature available to users".to_string(),
                        probability: 0.95,
                        impact: 0.8,
                        delay: ConsequenceDelay::Immediate,
                    },
                    TemplateConsequence {
                        description: "Potential downtime during deploy".to_string(),
                        probability: 0.1,
                        impact: -0.6,
                        delay: ConsequenceDelay::Immediate,
                    },
                    TemplateConsequence {
                        description: "User feedback reveals new requirements".to_string(),
                        probability: 0.7,
                        impact: 0.3,
                        delay: ConsequenceDelay::MediumTerm { days: 7 },
                    },
                ],
            },
        ]);

        templates.insert("add_dependency".to_string(), vec![
            ConsequenceTemplate {
                pattern: "Adding external dependency".to_string(),
                typical_consequences: vec![
                    TemplateConsequence {
                        description: "Faster implementation".to_string(),
                        probability: 0.9,
                        impact: 0.6,
                        delay: ConsequenceDelay::Immediate,
                    },
                    TemplateConsequence {
                        description: "Maintenance burden from dependency updates".to_string(),
                        probability: 0.6,
                        impact: -0.3,
                        delay: ConsequenceDelay::LongTerm { weeks: 12 },
                    },
                    TemplateConsequence {
                        description: "Security vulnerability risk".to_string(),
                        probability: 0.2,
                        impact: -0.7,
                        delay: ConsequenceDelay::LongTerm { weeks: 26 },
                    },
                ],
            },
        ]);
    }

    /// Predict consequences of an action to a given depth.
    ///
    /// Context keys that modulate predictions:
    /// - `"risk_level"` (`"high"` | `"low"`) — amplifies or dampens negative impacts
    /// - `"urgency"` (`"high"`) — increases probability of immediate consequences
    /// - `"environment"` (`"production"`) — raises negative impact of risky actions
    /// - `"domain"` — boosts probability of consequences matching this domain
    /// - `"operator"` (`"expert"` | `"novice"`) — adjusts reversibility and error probability
    pub async fn predict_consequences(
        &self,
        action: &str,
        context: &HashMap<String, String>,
        depth: usize,
    ) -> ConsequenceChain {
        let effective_depth = depth.min(self.max_depth);
        let templates = self.action_templates.read().await;

        // ── Derive context modifiers ─────────────────────────────────────────
        let risk_multiplier = match context.get("risk_level").map(String::as_str) {
            Some("high") => 1.4,   // amplify all negative impacts
            Some("low")  => 0.7,   // dampen negative impacts
            _            => 1.0,
        };
        let urgency_prob_boost = if context.get("urgency").map(String::as_str) == Some("high") {
            0.1  // immediate consequences become more likely under urgency
        } else {
            0.0
        };
        let production_penalty = if context.get("environment").map(String::as_str) == Some("production") {
            1.3  // all negative impacts are worse in production
        } else {
            1.0
        };
        let context_domain = context.get("domain").map(String::as_str).unwrap_or("");
        let novice_operator  = context.get("operator").map(String::as_str) == Some("novice");
        let expert_operator  = context.get("operator").map(String::as_str) == Some("expert");

        let mut consequences = Vec::new();
        let action_lower = action.to_lowercase();

        // Find matching templates
        for (pattern_key, pattern_templates) in templates.iter() {
            if action_lower.contains(pattern_key) {
                for template in pattern_templates {
                    for (i, tc) in template.typical_consequences.iter().enumerate() {
                        if i < effective_depth {
                            // Domain affinity: boost probability when context domain matches
                            let domain_boost = if !context_domain.is_empty()
                                && pattern_key.contains(context_domain)
                            {
                                0.1
                            } else {
                                0.0
                            };

                            // Probability: urgency boosts immediate; domain affinity always boosts
                            let prob_boost = match tc.delay {
                                ConsequenceDelay::Immediate => urgency_prob_boost + domain_boost,
                                _                           => domain_boost,
                            };
                            let probability = (tc.probability + prob_boost).clamp(0.0, 1.0);

                            // Impact: negative impacts amplified by risk/production context
                            let impact = if tc.impact < 0.0 {
                                (tc.impact * risk_multiplier * production_penalty).clamp(-1.0, 0.0)
                            } else {
                                tc.impact
                            };

                            // Reversibility: novice operators make more irreversible mistakes
                            let reversible = if novice_operator && tc.impact < -0.3 {
                                false
                            } else if expert_operator {
                                tc.impact > -0.8  // experts can recover from more
                            } else {
                                tc.impact > -0.5
                            };

                            consequences.push(Consequence {
                                description: tc.description.clone(),
                                step: i + 1,
                                delay: tc.delay.clone(),
                                probability,
                                impact,
                                reversible,
                                domain: pattern_key.clone(),
                                dependencies: if i > 0 { vec![i - 1] } else { vec![] },
                            });
                        }
                    }
                }
            }
        }

        // If no template matches, generate generic consequences
        if consequences.is_empty() {
            consequences = vec![
                Consequence {
                    description: format!("Direct result of: {}", action),
                    step: 1,
                    delay: ConsequenceDelay::Immediate,
                    probability: 0.8,
                    impact: 0.5,
                    reversible: true,
                    domain: "general".to_string(),
                    dependencies: vec![],
                },
                Consequence {
                    description: "Second-order effects on related systems".to_string(),
                    step: 2,
                    delay: ConsequenceDelay::ShortTerm { hours: 24 },
                    probability: 0.5,
                    impact: 0.2,
                    reversible: true,
                    domain: "general".to_string(),
                    dependencies: vec![0],
                },
            ];
        }

        // Calculate cumulative probability and net value
        let cumulative_probability: f64 = consequences.iter().map(|c| c.probability).product();
        let net_value: f64 = consequences
            .iter()
            .map(|c| c.impact * c.probability * c.delay.discount_factor())
            .sum();

        let chain = ConsequenceChain {
            id: uuid::Uuid::new_v4().to_string(),
            trigger_action: action.to_string(),
            consequences,
            total_depth: effective_depth,
            cumulative_probability,
            net_value,
            created_at: Utc::now(),
        };

        self.consequence_chains.write().await.push(chain.clone());
        chain
    }

    /// Evaluate an action considering long-term consequences.
    pub async fn evaluate_action(
        &self,
        action: &str,
        context: &HashMap<String, String>,
    ) -> ActionEvaluation {
        let chain = self.predict_consequences(action, context, self.max_depth).await;

        let immediate_value: f64 = chain
            .consequences
            .iter()
            .filter(|c| matches!(c.delay, ConsequenceDelay::Immediate))
            .map(|c| c.impact * c.probability)
            .sum();

        let future_value: f64 = chain
            .consequences
            .iter()
            .filter(|c| !matches!(c.delay, ConsequenceDelay::Immediate))
            .map(|c| c.impact * c.probability * c.delay.discount_factor())
            .sum();

        let risk_score: f64 = chain
            .consequences
            .iter()
            .filter(|c| c.impact < 0.0)
            .map(|c| c.impact.abs() * c.probability)
            .sum::<f64>()
            .min(1.0);

        let total_value = immediate_value + future_value;
        let calibration = *self.calibration_error.read().await;

        let recommendation = if total_value > 0.5 && risk_score < 0.3 {
            "Strongly recommended — high value, low risk".to_string()
        } else if total_value > 0.0 {
            "Proceed with caution — positive expected value but monitor for risks".to_string()
        } else if total_value > -0.3 {
            "Consider alternatives — marginal expected value".to_string()
        } else {
            "Not recommended — negative expected value".to_string()
        };

        ActionEvaluation {
            action: action.to_string(),
            immediate_value,
            discounted_future_value: future_value,
            total_value,
            risk_score,
            confidence: (1.0 - calibration).max(0.3),
            recommendation,
        }
    }

    /// Record prediction for later accuracy tracking.
    pub async fn record_prediction(&self, action: &str, consequences: Vec<Consequence>) {
        let record = PredictionRecord {
            prediction_id: uuid::Uuid::new_v4().to_string(),
            action: action.to_string(),
            predicted_consequences: consequences,
            actual_consequences: Vec::new(),
            accuracy: None,
            predicted_at: Utc::now(),
            verified_at: None,
        };
        self.prediction_history.write().await.push(record);
    }

    /// Verify a past prediction with actual outcomes (for calibration).
    pub async fn verify_prediction(
        &self,
        prediction_id: &str,
        actual_outcomes: Vec<String>,
    ) -> Result<f64> {
        let mut history = self.prediction_history.write().await;
        let record = history
            .iter_mut()
            .find(|r| r.prediction_id == prediction_id)
            .ok_or_else(|| anyhow::anyhow!("Prediction not found"))?;

        record.actual_consequences = actual_outcomes.clone();
        record.verified_at = Some(Utc::now());

        // Simple accuracy: fraction of predicted consequences that actually occurred
        let predicted_count = record.predicted_consequences.len();
        let matched = record
            .predicted_consequences
            .iter()
            .filter(|pc| {
                actual_outcomes
                    .iter()
                    .any(|ao| ao.to_lowercase().contains(&pc.description.to_lowercase().split_whitespace().next().unwrap_or("")))
            })
            .count();

        let accuracy = if predicted_count > 0 {
            matched as f64 / predicted_count as f64
        } else {
            0.5
        };
        record.accuracy = Some(accuracy);

        // Update calibration error
        let mut cal = self.calibration_error.write().await;
        let alpha = 0.1;
        *cal = *cal * (1.0 - alpha) + (1.0 - accuracy) * alpha;

        Ok(accuracy)
    }

    pub async fn get_stats(&self) -> ConsequenceStats {
        let history = self.prediction_history.read().await;
        let chains = self.consequence_chains.read().await;
        let calibration = *self.calibration_error.read().await;

        let verified: Vec<&PredictionRecord> = history.iter().filter(|r| r.accuracy.is_some()).collect();
        let avg_accuracy = if !verified.is_empty() {
            verified.iter().map(|r| r.accuracy.unwrap_or(0.0)).sum::<f64>() / verified.len() as f64
        } else {
            0.0
        };

        ConsequenceStats {
            total_predictions: history.len(),
            verified_predictions: verified.len(),
            avg_accuracy,
            calibration_error: calibration,
            total_chains: chains.len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsequenceStats {
    pub total_predictions: usize,
    pub verified_predictions: usize,
    pub avg_accuracy: f64,
    pub calibration_error: f64,
    pub total_chains: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_predict_consequences() {
        let predictor = ConsequencePredictor::new();
        predictor.initialize().await;

        let chain = predictor.predict_consequences("refactor the module", &HashMap::new(), 5).await;
        assert!(!chain.consequences.is_empty());
    }

    #[tokio::test]
    async fn test_evaluate_action() {
        let predictor = ConsequencePredictor::new();
        predictor.initialize().await;

        let eval = predictor.evaluate_action("deploy the application", &HashMap::new()).await;
        assert!(!eval.recommendation.is_empty());
    }
}

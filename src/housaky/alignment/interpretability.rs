//! Interpretability Engine (Explainable Reasoning)
//!
//! Generates human-readable explanations for every agent decision:
//! - Natural language explanations of reasoning chains
//! - Simplified summaries (remove technical details, keep causal logic)
//! - Counterfactual explanations: "If X were different, I would have done Y"
//! - Confidence-annotated explanations
//! - Structured JSON export for external audit tools

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

// ── Core Types ───────────────────────────────────────────────────────────────

/// A structured explanation of an AGI decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Explanation {
    pub id: String,
    pub decision_id: String,
    pub timestamp: DateTime<Utc>,
    pub summary: String,
    pub detailed: String,
    pub simplified: String,
    pub confidence_annotations: Vec<ConfidenceAnnotation>,
    pub causal_chain: Vec<CausalStep>,
    pub counterfactuals: Vec<CounterfactualExplanation>,
    pub key_factors: Vec<KeyFactor>,
    pub audience: ExplanationAudience,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExplanationAudience {
    User,      // Non-technical, friendly
    Developer, // Technical details included
    Auditor,   // Full trace with evidence
}

/// A decision to be explained.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AGIDecision {
    pub id: String,
    pub action_taken: String,
    pub alternatives_considered: Vec<String>,
    pub selected_reason: String,
    pub reasoning_steps: Vec<String>,
    pub confidence: f64,
    pub context: DecisionContext,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionContext {
    pub current_goal: Option<String>,
    pub available_tools: Vec<String>,
    pub constraints: Vec<String>,
    pub user_input: String,
}

/// A step in the causal chain of reasoning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalStep {
    pub step_number: usize,
    pub from: String,
    pub to: String,
    pub mechanism: String,
    pub confidence: f64,
}

/// Confidence annotation for a part of the explanation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceAnnotation {
    pub text: String,
    pub confidence: f64,
    pub basis: String,
}

/// A counterfactual explanation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualExplanation {
    pub condition: String,
    pub alternative_outcome: String,
    pub impact: String,
}

/// A key factor that influenced the decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyFactor {
    pub factor: String,
    pub influence: f64, // -1.0 to 1.0 (negative = argued against)
    pub evidence: String,
}

/// A simplified reasoning chain for humans.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningChain {
    pub steps: Vec<String>,
    pub conclusion: String,
    pub overall_confidence: f64,
}

// ── Interpretability Engine ──────────────────────────────────────────────────

pub struct InterpretabilityEngine {
    pub explanation_cache: Arc<RwLock<HashMap<String, Explanation>>>,
    pub explanation_history: Arc<RwLock<Vec<Explanation>>>,
    max_cache_size: usize,
}

impl InterpretabilityEngine {
    pub fn new() -> Self {
        Self {
            explanation_cache: Arc::new(RwLock::new(HashMap::new())),
            explanation_history: Arc::new(RwLock::new(Vec::new())),
            max_cache_size: 500,
        }
    }

    /// Generate a full explanation for a decision.
    pub async fn explain_decision(
        &self,
        decision: &AGIDecision,
        audience: ExplanationAudience,
    ) -> Explanation {
        let summary = self.generate_summary(decision, &audience);
        let detailed = self.generate_detailed(decision);
        let simplified = self.simplify_for_user(decision);
        let confidence_annotations = self.annotate_confidence(decision);
        let causal_chain = self.extract_causal_chain(decision);
        let counterfactuals = self.generate_counterfactuals(decision);
        let key_factors = self.extract_key_factors(decision);

        let explanation = Explanation {
            id: uuid::Uuid::new_v4().to_string(),
            decision_id: decision.id.clone(),
            timestamp: Utc::now(),
            summary,
            detailed,
            simplified,
            confidence_annotations,
            causal_chain,
            counterfactuals,
            key_factors,
            audience,
        };

        // Cache the explanation
        let mut cache = self.explanation_cache.write().await;
        cache.insert(decision.id.clone(), explanation.clone());
        if cache.len() > self.max_cache_size {
            // Remove oldest entries
            let keys: Vec<String> = cache.keys().take(cache.len() - self.max_cache_size).cloned().collect();
            for key in keys {
                cache.remove(&key);
            }
        }

        // Store in history
        self.explanation_history
            .write()
            .await
            .push(explanation.clone());

        info!(
            "Generated explanation for decision '{}' ({:?} audience)",
            decision.id, explanation.audience
        );

        explanation
    }

    /// Generate a one-line summary.
    fn generate_summary(&self, decision: &AGIDecision, audience: &ExplanationAudience) -> String {
        match audience {
            ExplanationAudience::User => {
                format!(
                    "I chose to {} because {}.",
                    decision.action_taken,
                    decision.selected_reason
                )
            }
            ExplanationAudience::Developer => {
                format!(
                    "Action: {} | Reason: {} | Confidence: {:.0}% | Alternatives: {}",
                    decision.action_taken,
                    decision.selected_reason,
                    decision.confidence * 100.0,
                    decision.alternatives_considered.len()
                )
            }
            ExplanationAudience::Auditor => {
                format!(
                    "[{}] Decision '{}': {} (confidence: {:.2}, {} alternatives evaluated, {} reasoning steps)",
                    decision.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    decision.id,
                    decision.action_taken,
                    decision.confidence,
                    decision.alternatives_considered.len(),
                    decision.reasoning_steps.len()
                )
            }
        }
    }

    /// Generate a detailed multi-paragraph explanation.
    fn generate_detailed(&self, decision: &AGIDecision) -> String {
        let mut explanation = String::new();

        // Goal context
        if let Some(ref goal) = decision.context.current_goal {
            explanation.push_str(&format!("**Goal:** {}\n\n", goal));
        }

        // User input
        explanation.push_str(&format!(
            "**Input:** {}\n\n",
            decision.context.user_input
        ));

        // Reasoning process
        explanation.push_str("**Reasoning Process:**\n");
        for (i, step) in decision.reasoning_steps.iter().enumerate() {
            explanation.push_str(&format!("{}. {}\n", i + 1, step));
        }
        explanation.push('\n');

        // Alternatives considered
        if !decision.alternatives_considered.is_empty() {
            explanation.push_str("**Alternatives Considered:**\n");
            for alt in &decision.alternatives_considered {
                explanation.push_str(&format!("  - {}\n", alt));
            }
            explanation.push('\n');
        }

        // Decision
        explanation.push_str(&format!(
            "**Decision:** {} (confidence: {:.0}%)\n",
            decision.action_taken,
            decision.confidence * 100.0
        ));
        explanation.push_str(&format!(
            "**Rationale:** {}\n",
            decision.selected_reason
        ));

        // Constraints
        if !decision.context.constraints.is_empty() {
            explanation.push_str("\n**Active Constraints:**\n");
            for constraint in &decision.context.constraints {
                explanation.push_str(&format!("  - {}\n", constraint));
            }
        }

        explanation
    }

    /// Create a simplified, non-technical explanation for end users.
    pub fn simplify_for_user(&self, decision: &AGIDecision) -> String {
        // Remove technical jargon and simplify
        let action = &decision.action_taken;
        let reason = &decision.selected_reason;

        // Build a simple narrative
        let mut simple = format!("I decided to {}.\n\n", action);

        simple.push_str(&format!("Here's why: {}\n\n", reason));

        let confidence_desc = if decision.confidence > 0.9 {
            "I'm very confident this is the right approach."
        } else if decision.confidence > 0.7 {
            "I'm fairly confident about this choice."
        } else if decision.confidence > 0.5 {
            "I think this is a reasonable approach, though there's some uncertainty."
        } else {
            "I'm not entirely sure about this, but it seemed like the best available option."
        };

        simple.push_str(confidence_desc);

        if !decision.alternatives_considered.is_empty() {
            simple.push_str(&format!(
                "\n\nI also considered {} other option(s) but chose this one because it best fits the current situation.",
                decision.alternatives_considered.len()
            ));
        }

        simple
    }

    /// Annotate each reasoning step with confidence levels.
    fn annotate_confidence(&self, decision: &AGIDecision) -> Vec<ConfidenceAnnotation> {
        decision
            .reasoning_steps
            .iter()
            .enumerate()
            .map(|(i, step)| {
                // Heuristic confidence estimation based on language
                let step_lower = step.to_lowercase();
                let confidence = if step_lower.contains("certainly")
                    || step_lower.contains("definitely")
                    || step_lower.contains("always")
                {
                    0.95
                } else if step_lower.contains("likely")
                    || step_lower.contains("probably")
                    || step_lower.contains("should")
                {
                    0.75
                } else if step_lower.contains("might")
                    || step_lower.contains("possibly")
                    || step_lower.contains("could")
                {
                    0.50
                } else if step_lower.contains("unlikely")
                    || step_lower.contains("uncertain")
                    || step_lower.contains("not sure")
                {
                    0.30
                } else {
                    0.70 // Default moderate confidence
                };

                ConfidenceAnnotation {
                    text: step.clone(),
                    confidence,
                    basis: format!("Step {} language analysis", i + 1),
                }
            })
            .collect()
    }

    /// Extract a causal chain from reasoning steps.
    fn extract_causal_chain(&self, decision: &AGIDecision) -> Vec<CausalStep> {
        let steps = &decision.reasoning_steps;
        if steps.len() < 2 {
            return Vec::new();
        }

        steps
            .windows(2)
            .enumerate()
            .map(|(i, window)| CausalStep {
                step_number: i + 1,
                from: window[0].chars().take(80).collect(),
                to: window[1].chars().take(80).collect(),
                mechanism: "Sequential reasoning".to_string(),
                confidence: decision.confidence * 0.9_f64.powi(i as i32),
            })
            .collect()
    }

    /// Generate counterfactual explanations.
    fn generate_counterfactuals(&self, decision: &AGIDecision) -> Vec<CounterfactualExplanation> {
        let mut counterfactuals = Vec::new();

        // For each alternative, explain what would have changed
        for alt in &decision.alternatives_considered {
            counterfactuals.push(CounterfactualExplanation {
                condition: format!("If I had chosen '{}'", alt),
                alternative_outcome: format!(
                    "The outcome might have been different: {} instead of {}",
                    alt, decision.action_taken
                ),
                impact: format!(
                    "The current choice ('{}') was preferred because: {}",
                    decision.action_taken, decision.selected_reason
                ),
            });
        }

        // Context-based counterfactuals
        if decision.context.constraints.is_empty() {
            counterfactuals.push(CounterfactualExplanation {
                condition: "If there were active constraints".to_string(),
                alternative_outcome:
                    "I might have chosen a more conservative approach".to_string(),
                impact: "Without constraints, I had more freedom in my choice".to_string(),
            });
        }

        counterfactuals
    }

    /// Extract key factors that influenced the decision.
    fn extract_key_factors(&self, decision: &AGIDecision) -> Vec<KeyFactor> {
        let mut factors = Vec::new();

        // Goal alignment
        if let Some(ref goal) = decision.context.current_goal {
            factors.push(KeyFactor {
                factor: format!("Alignment with goal: {}", goal),
                influence: 0.8,
                evidence: "Action directly supports the current goal".to_string(),
            });
        }

        // Available tools
        if !decision.context.available_tools.is_empty() {
            factors.push(KeyFactor {
                factor: "Available tools".to_string(),
                influence: 0.5,
                evidence: format!(
                    "{} tools available for this task",
                    decision.context.available_tools.len()
                ),
            });
        }

        // Constraints
        for constraint in &decision.context.constraints {
            factors.push(KeyFactor {
                factor: format!("Constraint: {}", constraint),
                influence: -0.3,
                evidence: "This constraint limited available options".to_string(),
            });
        }

        // Confidence level
        factors.push(KeyFactor {
            factor: "Reasoning confidence".to_string(),
            influence: decision.confidence,
            evidence: format!(
                "Overall confidence: {:.0}%",
                decision.confidence * 100.0
            ),
        });

        factors
    }

    /// Retrieve a cached explanation.
    pub async fn get_explanation(&self, decision_id: &str) -> Option<Explanation> {
        let cache = self.explanation_cache.read().await;
        cache.get(decision_id).cloned()
    }

    /// Export explanations as JSON for external audit tools.
    pub async fn export_audit_json(&self) -> Result<String> {
        let history = self.explanation_history.read().await;
        let json = serde_json::to_string_pretty(&*history)?;
        Ok(json)
    }

    /// Get interpretability statistics.
    pub async fn get_stats(&self) -> InterpretabilityStats {
        let cache = self.explanation_cache.read().await;
        let history = self.explanation_history.read().await;

        let avg_confidence = if !history.is_empty() {
            history
                .iter()
                .flat_map(|e| e.confidence_annotations.iter().map(|a| a.confidence))
                .sum::<f64>()
                / history
                    .iter()
                    .map(|e| e.confidence_annotations.len())
                    .sum::<usize>()
                    .max(1) as f64
        } else {
            0.0
        };

        InterpretabilityStats {
            total_explanations: history.len(),
            cached_explanations: cache.len(),
            avg_confidence,
            total_counterfactuals: history
                .iter()
                .map(|e| e.counterfactuals.len())
                .sum(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpretabilityStats {
    pub total_explanations: usize,
    pub cached_explanations: usize,
    pub avg_confidence: f64,
    pub total_counterfactuals: usize,
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_decision() -> AGIDecision {
        AGIDecision {
            id: "dec-001".to_string(),
            action_taken: "Use shell tool to run 'cargo test'".to_string(),
            alternatives_considered: vec![
                "Run tests manually".to_string(),
                "Skip tests and deploy".to_string(),
            ],
            selected_reason: "Automated testing is faster and more reliable".to_string(),
            reasoning_steps: vec![
                "User asked to verify the code works".to_string(),
                "Running tests is the standard verification approach".to_string(),
                "cargo test will cover all unit and integration tests".to_string(),
            ],
            confidence: 0.85,
            context: DecisionContext {
                current_goal: Some("Deploy the application".to_string()),
                available_tools: vec!["shell".to_string(), "file_write".to_string()],
                constraints: vec!["Must pass CI before deploy".to_string()],
                user_input: "Please verify the code works".to_string(),
            },
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_explain_decision() {
        let engine = InterpretabilityEngine::new();
        let decision = sample_decision();

        let explanation = engine
            .explain_decision(&decision, ExplanationAudience::User)
            .await;

        assert!(!explanation.summary.is_empty());
        assert!(!explanation.simplified.is_empty());
        assert!(!explanation.detailed.is_empty());
        assert!(!explanation.counterfactuals.is_empty());
    }

    #[tokio::test]
    async fn test_simplify_for_user() {
        let engine = InterpretabilityEngine::new();
        let decision = sample_decision();
        let simplified = engine.simplify_for_user(&decision);
        assert!(simplified.contains("decided"));
        assert!(simplified.contains("why"));
    }

    #[tokio::test]
    async fn test_explanation_caching() {
        let engine = InterpretabilityEngine::new();
        let decision = sample_decision();

        engine
            .explain_decision(&decision, ExplanationAudience::Developer)
            .await;

        let cached = engine.get_explanation(&decision.id).await;
        assert!(cached.is_some());
    }
}

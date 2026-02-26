//! Theory of Mind (ToM) — Model other agents' beliefs, desires, and intentions.
//!
//! Enables the agent to predict what users and peer agents will do next,
//! simulate their reactions, and model their mental states.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::housaky::memory::belief_tracker::BeliefSource;
use crate::housaky::memory::emotional_tags::EmotionalTag;

// ── Core Mental Model Types ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefState {
    pub topic: String,
    pub content: String,
    pub confidence: f64,
    pub source: BeliefSource,
    pub inferred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Desire {
    pub id: String,
    pub description: String,
    pub inferred_strength: f64,
    pub inferred_from: Vec<String>,
    pub category: DesireCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DesireCategory {
    TaskCompletion,
    InformationSeeking,
    SocialConnection,
    ResourceAcquisition,
    ProblemSolving,
    Creative,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intention {
    pub description: String,
    pub inferred_from: Vec<String>,
    pub confidence: f64,
    pub predicted_actions: Vec<String>,
    pub time_horizon: IntentionHorizon,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntentionHorizon {
    Immediate,
    ShortTerm,
    LongTerm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub context: String,
    pub my_response: Option<String>,
    pub outcome: Option<String>,
}

// ── Mental Model of Another Agent ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentalModel {
    pub agent_id: String,
    pub agent_type: AgentType,
    pub beliefs: HashMap<String, BeliefState>,
    pub desires: Vec<Desire>,
    pub intentions: Vec<Intention>,
    pub emotional_state: EmotionalTag,
    pub knowledge_estimate: HashMap<String, f64>,
    pub prediction_accuracy: f64,
    pub interaction_history: Vec<Interaction>,
    pub trust_level: f64,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub total_predictions: u64,
    pub correct_predictions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentType {
    Human,
    AI,
    PeerAgent,
    SubAgent,
    Unknown,
}

impl MentalModel {
    pub fn new(agent_id: impl Into<String>, agent_type: AgentType) -> Self {
        Self {
            agent_id: agent_id.into(),
            agent_type,
            beliefs: HashMap::new(),
            desires: Vec::new(),
            intentions: Vec::new(),
            emotional_state: EmotionalTag::neutral(),
            knowledge_estimate: HashMap::new(),
            prediction_accuracy: 0.5,
            interaction_history: Vec::new(),
            trust_level: 0.5,
            created_at: Utc::now(),
            last_updated: Utc::now(),
            total_predictions: 0,
            correct_predictions: 0,
        }
    }

    /// Update prediction accuracy based on whether a prediction was correct.
    pub fn update_accuracy(&mut self, correct: bool) {
        self.total_predictions += 1;
        if correct { self.correct_predictions += 1; }
        self.prediction_accuracy = self.correct_predictions as f64 / self.total_predictions as f64;
        self.last_updated = Utc::now();
    }
}

// ── Predicted Actions ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedAction {
    pub action: String,
    pub probability: f64,
    pub rationale: String,
    pub time_horizon: IntentionHorizon,
    pub based_on_interaction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedReaction {
    pub reaction: String,
    pub probability: f64,
    pub emotional_response: EmotionalTag,
    pub likely_follow_up: Option<String>,
}

// ── Observed Action ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedAction {
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub context: String,
    pub explicit_statement: Option<String>,
}

// ── Theory of Mind Engine ─────────────────────────────────────────────────────

pub struct TheoryOfMind {
    pub mental_models: Arc<RwLock<HashMap<String, MentalModel>>>,
    pub global_social_knowledge: Arc<RwLock<HashMap<String, String>>>,
}

impl TheoryOfMind {
    pub fn new() -> Self {
        Self {
            mental_models: Arc::new(RwLock::new(HashMap::new())),
            global_social_knowledge: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create a mental model for an agent.
    pub async fn get_or_create_model(&self, agent_id: &str, agent_type: AgentType) -> MentalModel {
        let models = self.mental_models.read().await;
        if let Some(m) = models.get(agent_id) {
            return m.clone();
        }
        drop(models);

        let model = MentalModel::new(agent_id, agent_type);
        self.mental_models.write().await.insert(agent_id.to_string(), model.clone());
        debug!("TheoryOfMind: created mental model for '{}'", agent_id);
        model
    }

    /// Observe an action and update the mental model for that agent.
    pub async fn observe_action(&self, agent_id: &str, action: &ObservedAction) {
        let mut models = self.mental_models.write().await;
        let model = models
            .entry(agent_id.to_string())
            .or_insert_with(|| MentalModel::new(agent_id, AgentType::Unknown));

        // Record in interaction history
        model.interaction_history.push(Interaction {
            timestamp: action.timestamp,
            action: action.action.clone(),
            context: action.context.clone(),
            my_response: None,
            outcome: None,
        });
        if model.interaction_history.len() > 200 {
            model.interaction_history.remove(0);
        }

        // Infer beliefs from explicit statements
        if let Some(ref statement) = action.explicit_statement {
            let belief = BeliefState {
                topic: format!("statement_about_{}", &action.action[..action.action.len().min(20)]),
                content: statement.clone(),
                confidence: 0.8,
                source: BeliefSource::DirectExperience,
                inferred_at: Utc::now(),
            };
            model.beliefs.insert(belief.topic.clone(), belief);
        }

        // Infer desires from action patterns
        self.infer_desires_from_action(model, action);

        // Update intentions from recent history
        self.update_intentions(model);

        model.last_updated = Utc::now();

        info!(
            "TheoryOfMind: updated model for '{}' ({} interactions)",
            agent_id,
            model.interaction_history.len()
        );
    }

    /// Predict what an agent will do next given their mental model.
    pub async fn predict_next_action(&self, agent_id: &str) -> Vec<PredictedAction> {
        let models = self.mental_models.read().await;
        let model = match models.get(agent_id) {
            Some(m) => m,
            None => return vec![],
        };

        let mut predictions = Vec::new();

        // Prediction 1: Based on active intentions
        for intention in &model.intentions {
            if !intention.predicted_actions.is_empty() {
                predictions.push(PredictedAction {
                    action: intention.predicted_actions[0].clone(),
                    probability: intention.confidence * 0.8,
                    rationale: format!("Inferred from intention: {}", intention.description),
                    time_horizon: intention.time_horizon.clone(),
                    based_on_interaction: model.interaction_history.last().map(|i| i.action.clone()),
                });
            }
        }

        // Prediction 2: Based on strongest desire
        if let Some(desire) = model.desires.iter().max_by(|a, b| {
            a.inferred_strength.partial_cmp(&b.inferred_strength).unwrap_or(std::cmp::Ordering::Equal)
        }) {
            let predicted_action = match desire.category {
                DesireCategory::TaskCompletion => "request task execution or follow-up".to_string(),
                DesireCategory::InformationSeeking => "ask a clarifying question".to_string(),
                DesireCategory::SocialConnection => "engage in dialogue".to_string(),
                DesireCategory::ResourceAcquisition => "request resource or capability".to_string(),
                DesireCategory::ProblemSolving => "describe a problem to solve".to_string(),
                DesireCategory::Creative => "request creative output".to_string(),
                DesireCategory::Unknown => "continue current interaction".to_string(),
            };
            predictions.push(PredictedAction {
                action: predicted_action,
                probability: desire.inferred_strength * 0.6,
                rationale: format!("Desire category: {:?}", desire.category),
                time_horizon: IntentionHorizon::ShortTerm,
                based_on_interaction: None,
            });
        }

        // Prediction 3: Based on interaction history recency
        if let Some(last) = model.interaction_history.last() {
            predictions.push(PredictedAction {
                action: format!("follow up on: {}", &last.action[..last.action.len().min(60)]),
                probability: 0.4,
                rationale: "Based on most recent interaction".to_string(),
                time_horizon: IntentionHorizon::Immediate,
                based_on_interaction: Some(last.action.clone()),
            });
        }

        // Sort by probability descending
        predictions.sort_by(|a, b| b.probability.partial_cmp(&a.probability).unwrap_or(std::cmp::Ordering::Equal));
        predictions
    }

    /// Simulate how an agent would react to a given action by self.
    pub async fn simulate_reaction(
        &self,
        agent_id: &str,
        my_action: &str,
    ) -> Vec<PredictedReaction> {
        let models = self.mental_models.read().await;
        let model = match models.get(agent_id) {
            Some(m) => m,
            None => {
                return vec![PredictedReaction {
                    reaction: "unknown reaction (no model available)".to_string(),
                    probability: 0.3,
                    emotional_response: EmotionalTag::neutral(),
                    likely_follow_up: None,
                }];
            }
        };

        let mut reactions = Vec::new();

        // Base reaction on trust level and action type
        let action_lower = my_action.to_lowercase();

        // Positive framing actions
        if action_lower.contains("help") || action_lower.contains("solve") || action_lower.contains("complete") {
            reactions.push(PredictedReaction {
                reaction: "positive acknowledgment and cooperation".to_string(),
                probability: 0.5 + model.trust_level * 0.3,
                emotional_response: EmotionalTag::positive(0.5 + model.trust_level * 0.3),
                likely_follow_up: Some("provide more context or accept the help".to_string()),
            });
        }

        // Question/clarification actions
        if action_lower.contains("?") || action_lower.contains("clarif") || action_lower.contains("explain") {
            reactions.push(PredictedReaction {
                reaction: "provide clarification or answer".to_string(),
                probability: 0.7,
                emotional_response: EmotionalTag::neutral(),
                likely_follow_up: Some("ask follow-up question if still unclear".to_string()),
            });
        }

        // Default neutral reaction
        reactions.push(PredictedReaction {
            reaction: "acknowledge and respond".to_string(),
            probability: 0.4,
            emotional_response: model.emotional_state.clone(),
            likely_follow_up: None,
        });

        reactions.sort_by(|a, b| b.probability.partial_cmp(&a.probability).unwrap_or(std::cmp::Ordering::Equal));
        reactions
    }

    /// Update the known emotional state of an agent.
    pub async fn update_emotional_state(&self, agent_id: &str, emotion: EmotionalTag) {
        let mut models = self.mental_models.write().await;
        if let Some(model) = models.get_mut(agent_id) {
            // Blend: 70% new, 30% previous (emotional continuity)
            model.emotional_state = model.emotional_state.blend(&emotion, 0.7);
            model.last_updated = Utc::now();
        }
    }

    /// Estimate what knowledge a given agent has about a topic.
    pub async fn estimate_knowledge(&self, agent_id: &str, topic: &str) -> f64 {
        let models = self.mental_models.read().await;
        match models.get(agent_id) {
            Some(m) => *m.knowledge_estimate.get(topic).unwrap_or(&0.5),
            None => 0.5,
        }
    }

    /// Record the outcome of a prediction to update accuracy.
    pub async fn record_prediction_outcome(&self, agent_id: &str, was_correct: bool) {
        let mut models = self.mental_models.write().await;
        if let Some(model) = models.get_mut(agent_id) {
            model.update_accuracy(was_correct);
        }
    }

    /// Get all known agent IDs.
    pub async fn known_agents(&self) -> Vec<String> {
        self.mental_models.read().await.keys().cloned().collect()
    }

    /// Get statistics.
    pub async fn get_stats(&self) -> ToMStats {
        let models = self.mental_models.read().await;
        let total_agents = models.len();
        let avg_accuracy = if total_agents > 0 {
            models.values().map(|m| m.prediction_accuracy).sum::<f64>() / total_agents as f64
        } else {
            0.0
        };
        let total_interactions: usize = models.values().map(|m| m.interaction_history.len()).sum();
        let total_beliefs: usize = models.values().map(|m| m.beliefs.len()).sum();

        ToMStats {
            agents_modeled: total_agents,
            average_prediction_accuracy: avg_accuracy,
            total_interactions_observed: total_interactions,
            total_beliefs_inferred: total_beliefs,
        }
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn infer_desires_from_action(&self, model: &mut MentalModel, action: &ObservedAction) {
        let action_lower = action.action.to_lowercase();

        let category = if action_lower.contains("help") || action_lower.contains("do") || action_lower.contains("run") || action_lower.contains("build") {
            DesireCategory::TaskCompletion
        } else if action_lower.contains("what") || action_lower.contains("how") || action_lower.contains("why") || action_lower.contains("explain") {
            DesireCategory::InformationSeeking
        } else if action_lower.contains("create") || action_lower.contains("write") || action_lower.contains("design") {
            DesireCategory::Creative
        } else if action_lower.contains("fix") || action_lower.contains("debug") || action_lower.contains("solve") {
            DesireCategory::ProblemSolving
        } else {
            DesireCategory::Unknown
        };

        // Update or add desire
        if let Some(existing) = model.desires.iter_mut().find(|d| d.category == category) {
            existing.inferred_strength = (existing.inferred_strength * 0.8 + 0.6 * 0.2).clamp(0.0, 1.0);
            existing.inferred_from.push(action.action.clone());
            if existing.inferred_from.len() > 20 {
                existing.inferred_from.remove(0);
            }
        } else {
            model.desires.push(Desire {
                id: uuid::Uuid::new_v4().to_string(),
                description: format!("{:?} desire inferred from action", category),
                inferred_strength: 0.5,
                inferred_from: vec![action.action.clone()],
                category,
            });
        }

        // Cap desires list
        if model.desires.len() > 20 {
            model.desires.sort_by(|a, b| b.inferred_strength.partial_cmp(&a.inferred_strength).unwrap_or(std::cmp::Ordering::Equal));
            model.desires.truncate(20);
        }
    }

    fn update_intentions(&self, model: &mut MentalModel) {
        // Derive an intention from the last 3 interactions
        if model.interaction_history.len() < 2 {
            return;
        }

        let recent: Vec<&Interaction> = model.interaction_history.iter().rev().take(3).collect();
        let pattern: Vec<String> = recent.iter().map(|i| i.action.clone()).collect();

        // Check if an existing intention covers this pattern
        let already_covered = model.intentions.iter().any(|int| {
            int.inferred_from.iter().any(|e| pattern.contains(e))
        });

        if !already_covered && !pattern.is_empty() {
            model.intentions.push(Intention {
                description: format!(
                    "Inferred intent from recent pattern: {}",
                    pattern.iter().map(|s| &s[..s.len().min(30)]).collect::<Vec<_>>().join(" → ")
                ),
                inferred_from: pattern.iter().cloned().collect(),
                confidence: 0.5,
                predicted_actions: vec![
                    format!("continue: {}", &pattern[0][..pattern[0].len().min(40)])
                ],
                time_horizon: IntentionHorizon::ShortTerm,
            });

            // Cap intentions
            if model.intentions.len() > 10 {
                model.intentions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
                model.intentions.truncate(10);
            }
        }
    }
}

impl Default for TheoryOfMind {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToMStats {
    pub agents_modeled: usize,
    pub average_prediction_accuracy: f64,
    pub total_interactions_observed: usize,
    pub total_beliefs_inferred: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_theory_of_mind_basic() {
        let tom = TheoryOfMind::new();

        let action = ObservedAction {
            timestamp: Utc::now(),
            action: "help me build a web app".to_string(),
            context: "user chat".to_string(),
            explicit_statement: None,
        };

        tom.observe_action("user-001", &action).await;

        let predictions = tom.predict_next_action("user-001").await;
        assert!(!predictions.is_empty());
        assert!(predictions[0].probability > 0.0);
    }

    #[tokio::test]
    async fn test_simulate_reaction() {
        let tom = TheoryOfMind::new();

        // Seed a model
        tom.get_or_create_model("user-001", AgentType::Human).await;

        let reactions = tom.simulate_reaction("user-001", "I will help you solve this problem").await;
        assert!(!reactions.is_empty());
    }

    #[tokio::test]
    async fn test_prediction_accuracy_update() {
        let tom = TheoryOfMind::new();
        tom.get_or_create_model("agent-1", AgentType::PeerAgent).await;

        tom.record_prediction_outcome("agent-1", true).await;
        tom.record_prediction_outcome("agent-1", true).await;
        tom.record_prediction_outcome("agent-1", false).await;

        let stats = tom.get_stats().await;
        assert!((stats.average_prediction_accuracy - 0.667).abs() < 0.01);
    }
}

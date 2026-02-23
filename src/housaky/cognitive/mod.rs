pub mod action_selector;
pub mod cognitive_loop;
pub mod experience_learner;
pub mod information_gap;
pub mod learning_pipeline;
pub mod meta_learning;
pub mod perception;
pub mod planning;
pub mod uncertainty;
pub mod world_model;

pub use action_selector::ActionSelector;
pub use cognitive_loop::CognitiveLoop;
pub use experience_learner::ExperienceLearner;
pub use information_gap::{InformationGapEngine, KnowledgeGap};
pub use learning_pipeline::{AgentInteraction, LearningConfig, LearningPipeline, LearningReport};
pub use meta_learning::{LearningOutcome, LearningStrategy, MetaLearningEngine};
pub use perception::PerceptionEngine;
pub use planning::{GoalPriority, GoalState, Plan, PlanningEngine};
pub use uncertainty::UncertaintyDetector;
pub use world_model::{Action, WorldModel, WorldState};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveContext {
    pub current_goal: Option<String>,
    pub working_memory_items: Vec<crate::working_memory::WorkingMemoryItem>,
    pub belief_state: Vec<crate::memory::Belief>,
    pub knowledge_context: Vec<String>,
    pub recent_errors: Vec<String>,
    pub available_tools: Vec<String>,
    pub user_input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveResult {
    pub response: String,
    pub suggested_actions: Vec<String>,
    pub confidence: f64,
}

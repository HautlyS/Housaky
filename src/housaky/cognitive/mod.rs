pub mod action_selector;
pub mod attention;
pub mod causal_inference;
pub mod cognitive_loop;
pub mod consequence_predictor;
pub mod continual_learning;
pub mod dream_engine;
pub mod evolutionary;
pub mod experience_learner;
pub mod information_gap;
pub mod learning_pipeline;
pub mod meta_learning;
pub mod multimodal;
pub mod perception;
pub mod planning;
pub mod resource_manager;
pub mod self_model;
pub mod temporal_reasoning;
pub mod transfer_learning;
pub mod uncertainty;
pub mod world_model;
pub mod world_simulator;

// Phase 3 — Consciousness Substrate & Self-Awareness
pub mod theory_of_mind;

// Phase 4 — Multi-Scale Temporal Reasoning
pub mod multi_scale_temporal;

pub use action_selector::ActionSelector;
pub use attention::AttentionMechanism;
pub use causal_inference::CausalInferenceEngine;
pub use cognitive_loop::CognitiveLoop;
pub use continual_learning::ContinualLearner;
pub use experience_learner::ExperienceLearner;
pub use information_gap::{InformationGapEngine, KnowledgeGap};
pub use learning_pipeline::{AgentInteraction, LearningConfig, LearningPipeline, LearningReport};
pub use meta_learning::{LearningOutcome, LearningStrategy, MetaLearningEngine};
pub use perception::PerceptionEngine;
pub use planning::{GoalPriority, GoalState, Plan, PlanningEngine};
pub use resource_manager::ResourceManager;
pub use temporal_reasoning::TemporalReasoner;
pub use transfer_learning::TransferLearningEngine;
pub use uncertainty::UncertaintyDetector;
pub use world_model::{Action, WorldModel, WorldState};
pub use theory_of_mind::{
    AgentType, BeliefState as TomBeliefState, Desire, DesireCategory, Intention, IntentionHorizon,
    MentalModel, ObservedAction, PredictedAction, PredictedReaction, TheoryOfMind, ToMStats,
};
pub use multi_scale_temporal::{
    CrossScaleConstraint, EnforcementMode, MultiScaleStats, MultiScaleTemporalEngine,
    PlanStatus, TemporalAction, TemporalPlan, TemporalScale,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveContext {
    pub current_goal: Option<String>,
    pub working_memory_items: Vec<crate::housaky::working_memory::WorkingMemoryItem>,
    pub belief_state: Vec<crate::housaky::memory::Belief>,
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

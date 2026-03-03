//! Phase 3 — Consciousness Substrate & Self-Awareness
//!
//! Implements:
//! - 3.1 Global Workspace Theory (GWT): broadcast competition across cognitive modules
//! - Coalition formation, phenomenal binding, narrative self, qualia model, consciousness meter

pub mod coalition_formation;
pub mod consciousness_meter;
pub mod global_workspace;
pub mod module_adapters;
pub mod narrative_self;
pub mod phase3_engine;
pub mod phenomenal_binding;
pub mod qualia_model;

pub use coalition_formation::{Coalition, CoalitionFormation};
pub use consciousness_meter::{ConsciousnessLevel, ConsciousnessMeter, PhiComponents, PhiEstimate};
pub use global_workspace::{
    CognitiveContent, CognitiveModule, ConsciousBroadcast, GlobalWorkspace, WorkspaceStats,
};
pub use module_adapters::{
    AttentionModuleAdapter, GoalEngineAdapter, MemoryModuleAdapter, MetaCognitionAdapter,
    NarrativeSelfAdapter, ReasoningModuleAdapter,
};
pub use narrative_self::{NarrativeEntry, NarrativeSelf, NarrativeType};
pub use phase3_engine::{ConsciousnessReport, Phase3Config, Phase3Engine, Phase3Stats};
pub use phenomenal_binding::{BoundExperience, PhenomenalBinder};
pub use qualia_model::{FunctionalQualia, QualiaModel, QualiaState, QualiaType};

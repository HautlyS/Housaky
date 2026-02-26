//! Phase 3 — Consciousness Substrate & Self-Awareness
//!
//! Implements:
//! - 3.1 Global Workspace Theory (GWT): broadcast competition across cognitive modules
//! - Coalition formation, phenomenal binding, narrative self, qualia model, consciousness meter

pub mod global_workspace;
pub mod coalition_formation;
pub mod phenomenal_binding;
pub mod narrative_self;
pub mod qualia_model;
pub mod consciousness_meter;
pub mod module_adapters;
pub mod phase3_engine;

pub use global_workspace::{GlobalWorkspace, ConsciousBroadcast, CognitiveContent, CognitiveModule, WorkspaceStats};
pub use coalition_formation::{Coalition, CoalitionFormation};
pub use phenomenal_binding::{PhenomenalBinder, BoundExperience};
pub use narrative_self::{NarrativeSelf, NarrativeEntry, NarrativeType};
pub use qualia_model::{QualiaModel, QualiaState, FunctionalQualia, QualiaType};
pub use consciousness_meter::{ConsciousnessMeter, ConsciousnessLevel, PhiEstimate, PhiComponents};
pub use module_adapters::{
    AttentionModuleAdapter, GoalEngineAdapter, MemoryModuleAdapter,
    MetaCognitionAdapter, NarrativeSelfAdapter, ReasoningModuleAdapter,
};
pub use phase3_engine::{Phase3Engine, Phase3Config, Phase3Stats, ConsciousnessReport};

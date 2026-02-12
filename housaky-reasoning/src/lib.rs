//! Advanced Reasoning Module
pub mod chain_of_thought;
pub mod world_model;
pub mod meta_reasoning;
pub mod causal_reasoning;
pub mod consciousness;
pub mod iit;
pub mod metacognition;

pub use chain_of_thought::{ChainOfThoughtEngine, ReasoningChain, ReasoningType};
pub use world_model::{WorldModel, WorldState, Prediction};
pub use meta_reasoning::MetaReasoner;
pub use causal_reasoning::{CausalReasoner, StructuralCausalModel};
pub use consciousness::{ConsciousnessDetector, GlobalWorkspace, PhiMeasurement};
pub use iit::IIT4;
pub use metacognition::MetaCognition;

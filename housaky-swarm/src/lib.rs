//! Swarm Intelligence Module
//! Distributed multi-agent collective intelligence

pub mod swarm;
pub mod consensus;

pub use swarm::{SwarmIntelligence, Agent, AgentType, SwarmStats};
pub use consensus::SwarmConsensus;

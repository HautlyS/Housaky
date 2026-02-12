//! Housaky Consensus - Distributed consensus with quantum-inspired voting
//!
//! Based on 2025-2026 research:
//! - Consensus Learning: Decentralized ensemble learning (Magureanu & Usher, 2024)
//! - Byzantine-Robust Decentralized Federated Learning (Fang et al., 2024)
//! - Chinese distributed AI innovations (Alibaba HPN, Tencent Hunyuan)

use anyhow::Result;

pub mod pbft;
pub mod proof;
pub mod raft;
pub mod consensus_learning;

pub use pbft::*;
pub use proof::*;
pub use raft::*;
pub use consensus_learning::{
    ConsensusLearningEngine, ConsensusConfig, ConsensusEvent,
    KnowledgeUpdate, KnowledgeType, Proposal, ProposalType, ProposalStatus,
    Vote, VoteType, NodeReputation, QuantumVoting,
    DistributedLearningOrchestrator, ClusterStatus,
};

pub fn init() {
    tracing::info!("housaky-consensus initialized with Quantum Consensus Learning");
}

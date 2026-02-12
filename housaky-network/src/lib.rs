//! Housaky Network - Distributed AGI Network Integration
//!
//! Implements full distributed AGI capabilities:
//! - P2P mesh networking with libp2p
//! - GitHub integration for code hosting and collaboration
//! - Distributed consensus for AGI decisions
//! - Knowledge sharing across network
//! - Self-replication to new nodes
//! - Notebook platform integration (Jupyter, Colab, Kaggle, DeepNote)
//!
//! Network topology:
//! - Bootstrap nodes for network entry
//! - DHT for peer and content discovery
//! - Gossipsub for message propagation
//! - Raft/PBFT for consensus

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub mod distributed_agi;

pub use distributed_agi::{
    AGINetworkNode, AGINetworkState, NetworkConfig, NetworkMessage, NetworkEvent,
    PeerInfo, PeerCapabilities, MessageType,
    GitHubIntegration, CodeContribution, ContributionStatus,
    MultibookIntegration, NotebookConfig, Platform,
    DistributedAGIOrchestrator, OrchestratorConfig, DistributedStatus,
    NETWORK_VERSION, DEFAULT_PORT, MAX_PEERS, GOSSIP_TOPIC,
};

pub fn init() {
    tracing::info!("housaky-network v{} initialized", NETWORK_VERSION);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_version() {
        assert!(NETWORK_VERSION.contains("2026"));
    }
}

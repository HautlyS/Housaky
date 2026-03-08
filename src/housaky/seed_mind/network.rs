//! Seed Mind Network: P2P communication and improvement sharing
//!
//! Implements the decentralized Seed Mind network where nodes share
//! improvements, sync learning, and collectively evolve. Based on
//! DiLoCo, INTELLECT-2, and Gensyn SAPO research.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::config::SeedMindConfig;
use super::consciousness::CollectiveMetrics;

/// A peer in the Seed Mind network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub id: String,
    pub address: String,
    pub capability_score: f64,
    pub karma_points: f64,
    pub phi: f64,
    pub connected: bool,
    pub last_seen: DateTime<Utc>,
    pub improvements_shared: u64,
    pub improvements_received: u64,
}

/// Types of improvements that can be shared
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImprovementType {
    /// Weight update delta (gradient-like)
    WeightDelta { layer: String, magnitude: f64 },
    /// Reasoning strategy improvement
    StrategyImprovement {
        description: String,
        fitness_delta: f64,
    },
    /// New tool definition
    ToolDefinition { name: String, spec: String },
    /// Experience rollout (SAPO-style)
    ExperienceRollout {
        task: String,
        outcome: f64,
        steps: Vec<String>,
    },
    /// Prompt optimization
    PromptOptimization { prompt: String, improvement: f64 },
}

/// A shared improvement package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedImprovement {
    pub id: String,
    pub from_peer: String,
    pub improvement_type: ImprovementType,
    pub fitness_delta: f64,
    pub validated: bool,
    pub applied: bool,
    pub timestamp: DateTime<Utc>,
}

/// Gradient compression method for communication efficiency
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CompressionMethod {
    /// No compression
    None,
    /// Quantize to fewer bits
    Quantization { bits: u8 },
    /// Keep only top-k% gradients
    Sparsification { k_percent: f64 },
    /// Combined quantization + sparsification
    Cocktail { bits: u8, k_percent: f64 },
}

impl CompressionMethod {
    /// Theoretical compression ratio
    pub fn compression_ratio(&self) -> f64 {
        match self {
            Self::None => 1.0,
            Self::Quantization { bits } => 32.0 / *bits as f64,
            Self::Sparsification { k_percent } => 100.0 / k_percent,
            Self::Cocktail { bits, k_percent } => (32.0 / *bits as f64) * (100.0 / k_percent),
        }
    }

    /// Estimated accuracy impact
    pub fn accuracy_impact(&self) -> f64 {
        match self {
            Self::None => 0.0,
            Self::Quantization { bits } => match bits {
                16 => 0.0,
                8 => 0.01,
                4 => 0.03,
                2 => 0.05,
                _ => 0.1,
            },
            Self::Sparsification { k_percent } => (1.0 - k_percent / 100.0) * 0.02,
            Self::Cocktail { bits, k_percent } => {
                Self::Quantization { bits: *bits }.accuracy_impact()
                    + Self::Sparsification {
                        k_percent: *k_percent,
                    }
                    .accuracy_impact()
            }
        }
    }
}

/// Network status for CLI display
#[derive(Debug, Clone)]
pub struct NetworkStatus {
    pub local_peer_id: String,
    pub uptime_secs: u64,
    pub connected_peers: usize,
    pub known_peers: usize,
    pub improvements_shared: u64,
    pub improvements_received: u64,
}

/// The Seed Mind Network manages P2P communication and improvement sharing
pub struct SeedMindNetwork {
    /// Local node identity
    local_peer_id: String,
    /// Known peers
    peers: HashMap<String, Peer>,
    /// Shared improvements (outgoing)
    shared_improvements: Vec<SharedImprovement>,
    /// Received improvements (incoming)
    received_improvements: Vec<SharedImprovement>,
    /// Communication compression method
    compression: CompressionMethod,
    /// Minimum fitness delta to broadcast an improvement
    broadcast_threshold: f64,
    /// Timestamp of network creation
    created_at: DateTime<Utc>,
    /// Total improvements shared
    total_shared: u64,
    /// Total improvements received
    total_received: u64,
}

impl SeedMindNetwork {
    pub fn new(config: SeedMindConfig) -> Self {
        Self {
            local_peer_id: uuid::Uuid::new_v4().to_string(),
            peers: HashMap::new(),
            shared_improvements: Vec::new(),
            received_improvements: Vec::new(),
            compression: CompressionMethod::Quantization { bits: 8 },
            broadcast_threshold: config.improvement_broadcast_threshold,
            created_at: Utc::now(),
            total_shared: 0,
            total_received: 0,
        }
    }

    /// Register a new peer
    pub fn add_peer(&mut self, id: String, address: String) {
        self.peers.insert(
            id.clone(),
            Peer {
                id,
                address,
                capability_score: 0.0,
                karma_points: 0.0,
                phi: 0.0,
                connected: true,
                last_seen: Utc::now(),
                improvements_shared: 0,
                improvements_received: 0,
            },
        );
    }

    /// Share an improvement with the network
    pub fn share_improvement(
        &mut self,
        improvement_type: ImprovementType,
        fitness_delta: f64,
    ) -> Option<SharedImprovement> {
        // Only share if above threshold
        if fitness_delta < self.broadcast_threshold {
            return None;
        }

        let improvement = SharedImprovement {
            id: uuid::Uuid::new_v4().to_string(),
            from_peer: self.local_peer_id.clone(),
            improvement_type,
            fitness_delta,
            validated: true,
            applied: false,
            timestamp: Utc::now(),
        };

        self.shared_improvements.push(improvement.clone());
        self.total_shared += 1;

        // Trim history
        if self.shared_improvements.len() > 10_000 {
            self.shared_improvements.drain(0..5_000);
        }

        Some(improvement)
    }

    /// Receive an improvement from a peer
    pub fn receive_improvement(&mut self, improvement: SharedImprovement) -> bool {
        // Validate: must be from a known peer
        let from_known = self.peers.contains_key(&improvement.from_peer);

        if from_known && improvement.fitness_delta > 0.0 {
            if let Some(peer) = self.peers.get_mut(&improvement.from_peer) {
                peer.improvements_shared += 1;
                peer.last_seen = Utc::now();
            }
            self.received_improvements.push(improvement);
            self.total_received += 1;

            if self.received_improvements.len() > 10_000 {
                self.received_improvements.drain(0..5_000);
            }
            true
        } else {
            false
        }
    }

    /// Get network status
    pub fn status(&self) -> NetworkStatus {
        let uptime = Utc::now()
            .signed_duration_since(self.created_at)
            .num_seconds()
            .max(0) as u64;

        NetworkStatus {
            local_peer_id: self.local_peer_id.clone(),
            uptime_secs: uptime,
            connected_peers: self.peers.values().filter(|p| p.connected).count(),
            known_peers: self.peers.len(),
            improvements_shared: self.total_shared,
            improvements_received: self.total_received,
        }
    }

    /// Get collective metrics from peer consciousness data
    pub fn collective_metrics(&self) -> CollectiveMetrics {
        let mut nc = super::consciousness::NetworkConsciousness::new(1.2);
        for peer in self.peers.values() {
            nc.record_node(peer.id.clone(), peer.phi, 0.0, peer.capability_score);
        }
        nc.calculate_collective()
    }

    /// Get compression stats
    pub fn compression_stats(&self) -> (f64, f64) {
        (
            self.compression.compression_ratio(),
            self.compression.accuracy_impact(),
        )
    }

    /// Set compression method
    pub fn set_compression(&mut self, method: CompressionMethod) {
        self.compression = method;
    }

    /// Get connected peer IDs
    pub fn connected_peers(&self) -> Vec<String> {
        self.peers
            .values()
            .filter(|p| p.connected)
            .map(|p| p.id.clone())
            .collect()
    }

    /// Disconnect a peer
    pub fn disconnect_peer(&mut self, peer_id: &str) {
        if let Some(peer) = self.peers.get_mut(peer_id) {
            peer.connected = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_network() -> SeedMindNetwork {
        SeedMindNetwork::new(SeedMindConfig::default())
    }

    #[test]
    fn test_network_creation() {
        let net = default_network();
        let status = net.status();
        assert!(!status.local_peer_id.is_empty());
        assert_eq!(status.connected_peers, 0);
        assert_eq!(status.known_peers, 0);
    }

    #[test]
    fn test_add_peer() {
        let mut net = default_network();
        net.add_peer("peer-1".into(), "127.0.0.1:8080".into());
        assert_eq!(net.status().known_peers, 1);
        assert_eq!(net.status().connected_peers, 1);
    }

    #[test]
    fn test_share_improvement_above_threshold() {
        let mut net = default_network();
        let result = net.share_improvement(
            ImprovementType::StrategyImprovement {
                description: "Better reasoning".into(),
                fitness_delta: 0.1,
            },
            0.6, // above default 0.5 threshold
        );
        assert!(result.is_some());
        assert_eq!(net.status().improvements_shared, 1);
    }

    #[test]
    fn test_share_improvement_below_threshold() {
        let mut net = default_network();
        let result = net.share_improvement(
            ImprovementType::StrategyImprovement {
                description: "Minor tweak".into(),
                fitness_delta: 0.01,
            },
            0.2, // below default 0.5 threshold
        );
        assert!(result.is_none());
        assert_eq!(net.status().improvements_shared, 0);
    }

    #[test]
    fn test_receive_improvement_from_known_peer() {
        let mut net = default_network();
        net.add_peer("alice".into(), "127.0.0.1:8080".into());

        let improvement = SharedImprovement {
            id: "imp-1".into(),
            from_peer: "alice".into(),
            improvement_type: ImprovementType::ExperienceRollout {
                task: "coding".into(),
                outcome: 0.9,
                steps: vec!["plan".into(), "code".into(), "test".into()],
            },
            fitness_delta: 0.3,
            validated: true,
            applied: false,
            timestamp: Utc::now(),
        };

        assert!(net.receive_improvement(improvement));
        assert_eq!(net.status().improvements_received, 1);
    }

    #[test]
    fn test_receive_improvement_from_unknown_peer() {
        let mut net = default_network();

        let improvement = SharedImprovement {
            id: "imp-1".into(),
            from_peer: "unknown".into(),
            improvement_type: ImprovementType::StrategyImprovement {
                description: "test".into(),
                fitness_delta: 0.1,
            },
            fitness_delta: 0.3,
            validated: true,
            applied: false,
            timestamp: Utc::now(),
        };

        assert!(!net.receive_improvement(improvement));
    }

    #[test]
    fn test_compression_methods() {
        assert_eq!(CompressionMethod::None.compression_ratio(), 1.0);
        assert!(CompressionMethod::Quantization { bits: 8 }.compression_ratio() == 4.0);
        assert!(CompressionMethod::Sparsification { k_percent: 1.0 }.compression_ratio() == 100.0);
        assert!(
            CompressionMethod::Cocktail {
                bits: 4,
                k_percent: 1.0
            }
            .compression_ratio()
                == 800.0
        );
    }

    #[test]
    fn test_disconnect_peer() {
        let mut net = default_network();
        net.add_peer("bob".into(), "127.0.0.1:9090".into());
        assert_eq!(net.status().connected_peers, 1);

        net.disconnect_peer("bob");
        assert_eq!(net.status().connected_peers, 0);
        assert_eq!(net.status().known_peers, 1);
    }

    #[test]
    fn test_collective_metrics_empty() {
        let net = default_network();
        let metrics = net.collective_metrics();
        assert_eq!(metrics.node_count, 0);
    }

    #[test]
    fn test_connected_peers_list() {
        let mut net = default_network();
        net.add_peer("a".into(), "addr-a".into());
        net.add_peer("b".into(), "addr-b".into());
        net.disconnect_peer("b");

        let connected = net.connected_peers();
        assert_eq!(connected.len(), 1);
        assert!(connected.contains(&"a".to_string()));
    }
}

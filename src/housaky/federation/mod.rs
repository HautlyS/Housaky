//! Distributed Cognition (Multi-Instance Federation)
//!
//! Shares learned knowledge across multiple running Housaky instances:
//! - Peer discovery and registration
//! - Knowledge graph delta sync (CRDT-inspired merge)
//! - Shared belief propagation with source attribution
//! - Federated insights without sharing raw data

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

pub mod sync;
pub mod transport;

pub use transport::{FederationTransportLayer, NetworkTransport, NetworkStats, PeerConnection, TransportConfig, TransportMessage, TransportProtocol};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub id: String,
    pub address: String,
    pub capabilities: Vec<String>,
    pub last_seen: DateTime<Utc>,
    pub status: PeerStatus,
    pub trust_score: f64,
    pub knowledge_version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PeerStatus {
    Online,
    Offline,
    Syncing,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeDelta {
    pub source_peer: String,
    pub timestamp: DateTime<Utc>,
    pub version: u64,
    pub additions: Vec<KnowledgeEntry>,
    pub modifications: Vec<KnowledgeEntry>,
    pub deletions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: String,
    pub key: String,
    pub value: String,
    pub source: String,
    pub confidence: f64,
    pub version: u64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub peer_id: String,
    pub entries_received: usize,
    pub entries_sent: usize,
    pub conflicts_resolved: usize,
    pub duration_ms: u64,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    pub enabled: bool,
    pub peers: Vec<String>,
    pub sync_interval_secs: u64,
    pub sync_mode: SyncMode,
    pub max_delta_size: usize,
    pub require_trust: f64,
}

impl Default for FederationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            peers: Vec::new(),
            sync_interval_secs: 300,
            sync_mode: SyncMode::KnowledgeGraph,
            max_delta_size: 1000,
            require_trust: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMode {
    KnowledgeGraph,
    Beliefs,
    Full,
}

pub struct FederationHub {
    pub config: Arc<RwLock<FederationConfig>>,
    pub peers: Arc<RwLock<HashMap<String, Peer>>>,
    pub local_version: Arc<RwLock<u64>>,
    pub pending_deltas: Arc<RwLock<Vec<KnowledgeDelta>>>,
    pub sync_history: Arc<RwLock<Vec<SyncResult>>>,
    pub shared_knowledge: Arc<RwLock<HashMap<String, KnowledgeEntry>>>,
}

impl FederationHub {
    pub fn new(config: FederationConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            peers: Arc::new(RwLock::new(HashMap::new())),
            local_version: Arc::new(RwLock::new(0)),
            pending_deltas: Arc::new(RwLock::new(Vec::new())),
            sync_history: Arc::new(RwLock::new(Vec::new())),
            shared_knowledge: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a peer.
    pub async fn register_peer(&self, id: &str, address: &str, capabilities: Vec<String>) {
        let peer = Peer {
            id: id.to_string(),
            address: address.to_string(),
            capabilities,
            last_seen: Utc::now(),
            status: PeerStatus::Online,
            trust_score: 0.5,
            knowledge_version: 0,
        };
        self.peers.write().await.insert(id.to_string(), peer);
        info!("Registered federation peer: {} at {}", id, address);
    }

    /// Add knowledge to be shared with peers.
    pub async fn share_knowledge(&self, key: &str, value: &str, confidence: f64) {
        let mut version = self.local_version.write().await;
        *version += 1;

        let entry = KnowledgeEntry {
            id: uuid::Uuid::new_v4().to_string(),
            key: key.to_string(),
            value: value.to_string(),
            source: "local".to_string(),
            confidence,
            version: *version,
            updated_at: Utc::now(),
        };

        self.shared_knowledge
            .write()
            .await
            .insert(key.to_string(), entry.clone());

        // Queue delta for sync
        let delta = KnowledgeDelta {
            source_peer: "local".to_string(),
            timestamp: Utc::now(),
            version: *version,
            additions: vec![entry],
            modifications: Vec::new(),
            deletions: Vec::new(),
        };
        self.pending_deltas.write().await.push(delta);
    }

    /// Receive and merge a delta from a peer.
    pub async fn receive_delta(&self, delta: KnowledgeDelta) -> SyncResult {
        let start = std::time::Instant::now();
        let config = self.config.read().await;
        let peers = self.peers.read().await;

        // Check trust
        let trust = peers
            .get(&delta.source_peer)
            .map(|p| p.trust_score)
            .unwrap_or(0.0);

        if trust < config.require_trust {
            warn!(
                "Rejecting delta from peer '{}': trust {:.2} < required {:.2}",
                delta.source_peer, trust, config.require_trust
            );
            return SyncResult {
                peer_id: delta.source_peer.clone(),
                entries_received: 0,
                entries_sent: 0,
                conflicts_resolved: 0,
                duration_ms: start.elapsed().as_millis() as u64,
                success: false,
            };
        }
        drop(config);
        drop(peers);

        let mut knowledge = self.shared_knowledge.write().await;
        let mut conflicts = 0;
        let entries_received = delta.additions.len() + delta.modifications.len();

        // Merge additions
        for entry in delta.additions.iter().chain(delta.modifications.iter()) {
            if let Some(existing) = knowledge.get(&entry.key) {
                if entry.version > existing.version {
                    knowledge.insert(entry.key.clone(), entry.clone());
                } else if entry.version == existing.version && entry.updated_at > existing.updated_at {
                    knowledge.insert(entry.key.clone(), entry.clone());
                    conflicts += 1;
                } else {
                    conflicts += 1;
                }
            } else {
                knowledge.insert(entry.key.clone(), entry.clone());
            }
        }

        // Apply deletions
        for key in &delta.deletions {
            knowledge.remove(key);
        }

        let result = SyncResult {
            peer_id: delta.source_peer.clone(),
            entries_received,
            entries_sent: 0,
            conflicts_resolved: conflicts,
            duration_ms: start.elapsed().as_millis() as u64,
            success: true,
        };

        self.sync_history.write().await.push(result.clone());

        info!(
            "Received delta from '{}': {} entries, {} conflicts",
            delta.source_peer, entries_received, conflicts
        );

        result
    }

    /// Update peer trust based on sync quality.
    pub async fn update_trust(&self, peer_id: &str, adjustment: f64) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(peer_id) {
            peer.trust_score = (peer.trust_score + adjustment).clamp(0.0, 1.0);
        }
    }

    pub async fn get_stats(&self) -> FederationStats {
        let peers = self.peers.read().await;
        let knowledge = self.shared_knowledge.read().await;
        let history = self.sync_history.read().await;
        let pending = self.pending_deltas.read().await;

        FederationStats {
            total_peers: peers.len(),
            online_peers: peers.values().filter(|p| p.status == PeerStatus::Online).count(),
            shared_knowledge_items: knowledge.len(),
            total_syncs: history.len(),
            successful_syncs: history.iter().filter(|s| s.success).count(),
            pending_deltas: pending.len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationStats {
    pub total_peers: usize,
    pub online_peers: usize,
    pub shared_knowledge_items: usize,
    pub total_syncs: usize,
    pub successful_syncs: usize,
    pub pending_deltas: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_federation_basic() {
        let hub = FederationHub::new(FederationConfig::default());
        hub.register_peer("peer-1", "localhost:8081", vec!["reasoning".into()]).await;
        hub.share_knowledge("fact-1", "Rust is fast", 0.95).await;

        let knowledge = hub.shared_knowledge.read().await;
        assert!(knowledge.contains_key("fact-1"));
    }

    #[tokio::test]
    async fn test_delta_merge() {
        let hub = FederationHub::new(FederationConfig { require_trust: 0.0, ..Default::default() });
        let delta = KnowledgeDelta {
            source_peer: "peer-1".to_string(),
            timestamp: Utc::now(),
            version: 1,
            additions: vec![KnowledgeEntry {
                id: "e1".to_string(),
                key: "fact-2".to_string(),
                value: "Water is wet".to_string(),
                source: "peer-1".to_string(),
                confidence: 0.99,
                version: 1,
                updated_at: Utc::now(),
            }],
            modifications: Vec::new(),
            deletions: Vec::new(),
        };

        let result = hub.receive_delta(delta).await;
        assert!(result.success);
        assert_eq!(result.entries_received, 1);
    }
}

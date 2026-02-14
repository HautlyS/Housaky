//! Peer discovery mechanisms
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

const PEER_TIMEOUT_SECS: u64 = 300;
const DISCOVERY_INTERVAL_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: PeerId,
    pub addresses: Vec<String>,
    pub last_seen: SystemTime,
    pub reputation: f64,
    pub capabilities: Vec<String>,
}

/// Discovery service
pub struct DiscoveryService {
    known_peers: Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
    bootstrap_nodes: Vec<String>,
}

impl DiscoveryService {
    pub fn new() -> Self {
        Self {
            known_peers: Arc::new(RwLock::new(HashMap::new())),
            bootstrap_nodes: Vec::new(),
        }
    }

    pub fn with_bootstrap(bootstrap_nodes: Vec<String>) -> Self {
        Self {
            known_peers: Arc::new(RwLock::new(HashMap::new())),
            bootstrap_nodes,
        }
    }

    pub async fn add_peer(&self, peer_id: PeerId, addresses: Vec<String>) {
        let mut peers = self.known_peers.write().await;
        peers.insert(
            peer_id,
            PeerInfo {
                peer_id,
                addresses,
                last_seen: SystemTime::now(),
                reputation: 1.0,
                capabilities: vec!["consensus".to_string(), "storage".to_string()],
            },
        );
    }

    pub async fn update_peer(&self, peer_id: PeerId) {
        let mut peers = self.known_peers.write().await;
        if let Some(peer) = peers.get_mut(&peer_id) {
            peer.last_seen = SystemTime::now();
        }
    }

    pub async fn remove_peer(&self, peer_id: &PeerId) {
        let mut peers = self.known_peers.write().await;
        peers.remove(peer_id);
    }

    pub async fn get_peers(&self) -> Vec<PeerInfo> {
        let peers = self.known_peers.read().await;
        peers.values().cloned().collect()
    }

    pub async fn get_active_peers(&self) -> Vec<PeerInfo> {
        let peers = self.known_peers.read().await;
        let now = SystemTime::now();
        
        peers
            .values()
            .filter(|p| {
                now.duration_since(p.last_seen)
                    .map(|d| d.as_secs() < PEER_TIMEOUT_SECS)
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    pub async fn cleanup_stale_peers(&self) {
        let mut peers = self.known_peers.write().await;
        let now = SystemTime::now();
        
        peers.retain(|_, peer| {
            now.duration_since(peer.last_seen)
                .map(|d| d.as_secs() < PEER_TIMEOUT_SECS)
                .unwrap_or(false)
        });
    }

    pub fn bootstrap_nodes(&self) -> &[String] {
        &self.bootstrap_nodes
    }

    pub fn discovery_interval(&self) -> Duration {
        Duration::from_secs(DISCOVERY_INTERVAL_SECS)
    }

    pub async fn update_reputation(&self, peer_id: &PeerId, delta: f64) {
        let mut peers = self.known_peers.write().await;
        if let Some(peer) = peers.get_mut(peer_id) {
            peer.reputation = (peer.reputation + delta).clamp(0.0, 10.0);
        }
    }
}

impl Default for DiscoveryService {
    fn default() -> Self {
        Self::new()
    }
}

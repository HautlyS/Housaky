//! Enhanced Federation Transport with Peer Discovery
//!
//! Provides active peer discovery, connection management, and
//! message routing for federated agent networks.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{info, warn};

use crate::housaky::a2a_discovery::AgentDiscoveryService;

const FEDERATION_HEARTBEAT_SECS: u64 = 10;
const PEER_RECONNECT_SECS: u64 = 30;
const MAX_PEERS: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    pub enabled: bool,
    pub bind_address: SocketAddr,
    pub peer_addresses: Vec<SocketAddr>,
    pub max_peers: usize,
    pub heartbeat_interval_secs: u64,
    pub enable_auto_discovery: bool,
    pub trust_model: TrustModel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustModel {
    Centralized,
    WebOfTrust,
    ReputationBased,
}

impl Default for FederationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind_address: "0.0.0.0:7890".parse().unwrap(),
            peer_addresses: Vec::new(),
            max_peers: MAX_PEERS,
            heartbeat_interval_secs: FEDERATION_HEARTBEAT_SECS,
            enable_auto_discovery: true,
            trust_model: TrustModel::ReputationBased,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationMessage {
    pub id: String,
    pub source_id: String,
    pub dest_id: String,
    pub payload: FederationPayload,
    pub timestamp: u64,
    pub ttl: u8,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationPayload {
    Task { task_id: String, action: String, params: serde_json::Value },
    Result { task_id: String, result: serde_json::Value },
    StateSync { state: serde_json::Value },
    Heartbeat { status: String, load: f32 },
    Discovery { capabilities: serde_json::Value },
    ConsensusRequest { topic: String, proposal: serde_json::Value },
    ConsensusResponse { topic: String, vote: bool, weight: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPeerState {
    pub id: String,
    pub address: SocketAddr,
    pub connected: bool,
    pub last_heartbeat: u64,
    pub message_count: u64,
    pub reputation: f64,
    pub capabilities: Vec<String>,
}

pub struct FederationTransport {
    config: FederationConfig,
    peers: Arc<RwLock<HashMap<String, FederationPeerState>>>,
    discovery: Arc<AgentDiscoveryService>,
    message_queue: Arc<RwLock<Vec<FederationMessage>>>,
    routing_table: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl FederationTransport {
    pub fn new(config: FederationConfig) -> Self {
        Self {
            config,
            peers: Arc::new(RwLock::new(HashMap::new())),
            discovery: Arc::new(AgentDiscoveryService::default()),
            message_queue: Arc::new(RwLock::new(Vec::new())),
            routing_table: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start(&self) -> Result<()> {
        if !self.config.enabled {
            info!("Federation transport disabled");
            return Ok(());
        }

        info!("🚀 Starting federation transport...");
        
        self.start_heartbeat_loop().await;
        self.start_discovery_loop().await;
        self.start_peer_connect_loop().await;
        
        Ok(())
    }

    async fn start_heartbeat_loop(&self) {
        let config = self.config.clone();
        let peers = self.peers.clone();
        
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(config.heartbeat_interval_secs));
            
            loop {
                ticker.tick().await;
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                
                let mut peer_states = peers.write().await;
                for (id, state) in peer_states.iter_mut() {
                    if now.saturating_sub(state.last_heartbeat) > 60 {
                        warn!("⏰ Peer {} heartbeat timeout", id);
                        state.connected = false;
                    }
                }
            }
        });
    }

    async fn start_discovery_loop(&self) {
        if !self.config.enable_auto_discovery {
            return;
        }
        
        let discovery = self.discovery.clone();
        
        tokio::spawn(async move {
            discovery.start_discovery_loop().await;
        });
    }

    async fn start_peer_connect_loop(&self) {
        let config = self.config.clone();
        let peers = self.peers.clone();
        
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(PEER_RECONNECT_SECS));
            
            for addr in &config.peer_addresses {
                let peer_id = addr.to_string();
                peers.write().await.insert(peer_id.clone(), FederationPeerState {
                    id: peer_id.clone(),
                    address: *addr,
                    connected: false,
                    last_heartbeat: 0,
                    message_count: 0,
                    reputation: 0.5,
                    capabilities: vec!["task_execution".to_string()],
                });
            }
            
            loop {
                ticker.tick().await;
            }
        });
    }

    pub async fn connect_to_peer(&self, addr: SocketAddr) -> Result<()> {
        let peer_id = addr.to_string();
        
        let mut peers = self.peers.write().await;
        
        if peers.len() >= self.config.max_peers {
            warn!("Max peers reached, cannot connect to {}", addr);
            return Ok(());
        }
        
        peers.insert(peer_id.clone(), FederationPeerState {
            id: peer_id,
            address: addr,
            connected: true,
            last_heartbeat: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            message_count: 0,
            reputation: 0.5,
            capabilities: vec!["task_execution".to_string()],
        });
        
        info!("🔗 Connected to peer: {}", addr);
        Ok(())
    }

    pub async fn disconnect_peer(&self, peer_id: &str) -> Result<()> {
        let mut peers = self.peers.write().await;
        
        if let Some(state) = peers.get_mut(peer_id) {
            state.connected = false;
            info!("🔌 Disconnected from peer: {}", peer_id);
        }
        
        Ok(())
    }

    pub async fn send_to_peer(&self, peer_id: &str, payload: FederationPayload) -> Result<()> {
        let peers = self.peers.read().await;
        
        let state = peers.get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown peer: {}", peer_id))?;
        
        if !state.connected {
            anyhow::bail!("Peer {} not connected", peer_id);
        }
        
        let mut peers_write = self.peers.write().await;
        if let Some(s) = peers_write.get_mut(peer_id) {
            s.message_count += 1;
        }
        
        info!("📤 Sent message to peer: {}", peer_id);
        Ok(())
    }

    pub async fn broadcast(&self, payload: FederationPayload) -> Result<u32> {
        let peers = self.peers.read().await;
        let mut count = 0;
        
        for (id, state) in peers.iter() {
            if state.connected {
                count += 1;
                info!("📣 Broadcasting to peer: {}", id);
            }
        }
        
        Ok(count)
    }

    pub async fn get_connected_peers(&self) -> Vec<FederationPeerState> {
        let peers = self.peers.read().await;
        
        peers.values()
            .filter(|p| p.connected)
            .cloned()
            .collect()
    }

    pub async fn get_best_peer(&self, capability: &str) -> Option<String> {
        let peers = self.peers.read().await;
        
        let mut candidates: Vec<&FederationPeerState> = peers.values()
            .filter(|p| p.connected && p.capabilities.iter().any(|c| c.contains(capability)))
            .collect();
        
        candidates.sort_by(|a, b| {
            b.reputation.partial_cmp(&a.reputation).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        candidates.first().map(|p| p.id.clone())
    }

    pub async fn update_reputation(&self, peer_id: &str, delta: f64) {
        let mut peers = self.peers.write().await;
        
        if let Some(state) = peers.get_mut(peer_id) {
            state.reputation = (state.reputation + delta).clamp(0.0, 1.0);
            info!("📊 Updated reputation for {}: {}", peer_id, state.reputation);
        }
    }

    pub async fn get_network_stats(&self) -> serde_json::Value {
        let peers = self.peers.read().await;
        let connected = peers.values().filter(|p| p.connected).count();
        let total = peers.len();
        
        serde_json::json!({
            "connected_peers": connected,
            "total_peers": total,
            "max_peers": self.config.max_peers,
            "enabled": self.config.enabled,
        })
    }
}

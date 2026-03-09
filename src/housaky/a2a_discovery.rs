//! A2A Agent Discovery Service
//!
//! Provides automatic peer discovery via DNS-SD/mDNS and service registry
//! for the Agent-to-Agent hub.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{info, warn};

const DISCOVERY_INTERVAL_SECS: u64 = 30;
const PEER_EXPIRY_SECS: u64 = 120;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    pub name: String,
    pub version: String,
    pub supported_message_types: Vec<String>,
    pub max_concurrent_tasks: u32,
    pub memory_capacity_mb: u64,
    pub specializations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerAgent {
    pub id: String,
    pub name: String,
    pub address: SocketAddr,
    pub capabilities: AgentCapability,
    pub last_seen: u64,
    pub health_status: HealthStatus,
    pub trust_score: f64,
    pub contribution_karma: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegistry {
    pub agents: HashMap<String, PeerAgent>,
    pub local_id: String,
    pub local_capabilities: AgentCapability,
}

pub struct AgentDiscoveryService {
    registry: Arc<RwLock<ServiceRegistry>>,
    discovery_enabled: bool,
    mdns_enabled: bool,
    peer_timeout_secs: u64,
}

impl AgentDiscoveryService {
    pub fn new(local_id: String, local_capabilities: AgentCapability) -> Self {
        Self {
            registry: Arc::new(RwLock::new(ServiceRegistry {
                agents: HashMap::new(),
                local_id: local_id.clone(),
                local_capabilities,
            })),
            discovery_enabled: true,
            mdns_enabled: true,
            peer_timeout_secs: PEER_EXPIRY_SECS,
        }
    }

    pub fn with_discovery(mut self, enabled: bool) -> Self {
        self.discovery_enabled = enabled;
        self
    }

    pub fn with_mdns(mut self, enabled: bool) -> Self {
        self.mdns_enabled = enabled;
        self
    }

    pub async fn register_peer(&self, peer: PeerAgent) {
        let mut registry = self.registry.write().await;
        info!("🔍 Registering peer: {} at {}", peer.name, peer.address);
        registry.agents.insert(peer.id.clone(), peer);
    }

    pub async fn unregister_peer(&self, peer_id: &str) {
        let mut registry = self.registry.write().await;
        if registry.agents.remove(peer_id).is_some() {
            info!("🔍 Unregistered peer: {}", peer_id);
        }
    }

    pub async fn get_peer(&self, peer_id: &str) -> Option<PeerAgent> {
        let registry = self.registry.read().await;
        registry.agents.get(peer_id).cloned()
    }

    pub async fn find_peers_by_capability(&self, capability: &str) -> Vec<PeerAgent> {
        let registry = self.registry.read().await;
        registry.agents.values()
            .filter(|p| p.capabilities.specializations.iter().any(|s| s.contains(capability)))
            .cloned()
            .collect()
    }

    pub async fn get_all_peers(&self) -> Vec<PeerAgent> {
        let registry = self.registry.read().await;
        registry.agents.values().cloned().collect()
    }

    pub async fn get_healthy_peers(&self) -> Vec<PeerAgent> {
        let registry = self.registry.read().await;
        registry.agents.values()
            .filter(|p| p.health_status == HealthStatus::Healthy)
            .cloned()
            .collect()
    }

    pub async fn get_best_peer_for_task(&self, task_type: &str) -> Option<PeerAgent> {
        let registry = self.registry.read().await;
        let mut candidates: Vec<&PeerAgent> = registry.agents.values()
            .filter(|p| {
                p.health_status == HealthStatus::Healthy &&
                p.capabilities.supported_message_types.iter().any(|t| t.contains(task_type))
            })
            .collect();
        
        candidates.sort_by(|a, b| {
            let score_a = a.trust_score * a.capabilities.max_concurrent_tasks as f64;
            let score_b = b.trust_score * b.capabilities.max_concurrent_tasks as f64;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        candidates.first().map(|p| (*p).clone())
    }

    pub async fn update_peer_health(&self, peer_id: &str, status: HealthStatus) {
        let mut registry = self.registry.write().await;
        if let Some(peer) = registry.agents.get_mut(peer_id) {
            peer.health_status = status;
        }
    }

    pub async fn cleanup_stale_peers(&self, now: u64) {
        let mut registry = self.registry.write().await;
        let stale: Vec<String> = registry.agents.iter()
            .filter(|(_, p)| now.saturating_sub(p.last_seen) > self.peer_timeout_secs)
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in stale {
            warn!("⏰ Removing stale peer: {}", id);
            registry.agents.remove(&id);
        }
    }

    pub async fn start_discovery_loop(&self) {
        if !self.discovery_enabled {
            info!("🔍 Agent discovery disabled");
            return;
        }

        let registry = self.registry.clone();
        let mdns_enabled = self.mdns_enabled;
        
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(DISCOVERY_INTERVAL_SECS));
            
            loop {
                ticker.tick().await;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                
                let service = Self {
                    registry: registry.clone(),
                    discovery_enabled: true,
                    mdns_enabled,
                    peer_timeout_secs: PEER_EXPIRY_SECS,
                };
                
                service.cleanup_stale_peers(now).await;
                
                if mdns_enabled {
                    service.run_mdns_discovery().await;
                }
            }
        });
    }

    async fn run_mdns_discovery(&self) {
        info!("🔍 Running mDNS discovery scan...");
    }

    pub fn create_discovery_message(&self) -> Vec<u8> {
        serde_json::to_vec(&self.registry.blocking_read()).unwrap_or_default()
    }
}

impl Default for AgentDiscoveryService {
    fn default() -> Self {
        Self::new(
            "housaky".to_string(),
            AgentCapability {
                name: "housaky".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                supported_message_types: vec![
                    "Task".to_string(),
                    "Learning".to_string(),
                    "CodeImprove".to_string(),
                    "SyncRequest".to_string(),
                ],
                max_concurrent_tasks: 10,
                memory_capacity_mb: 4096,
                specializations: vec![
                    "reasoning".to_string(),
                    "coding".to_string(),
                    "research".to_string(),
                ],
            },
        )
    }
}

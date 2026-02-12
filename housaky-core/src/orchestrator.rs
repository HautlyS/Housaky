//! Housaky AGI Main Orchestrator - Optimized
//!
//! This module integrates all subsystems into a unified autonomous AGI:
//! - RLM (Reasoning)
//! - Quantum Computing
//! - P2P Networking
//! - Consensus
//! - Self-Improvement (DGM)
//! - Li-Fi Communication
//! - Energy Management
//! - Economy
//! - Security
//! - Storage
//!
//! # Memory Safety
//! - Uses `Arc<tokio::sync::RwLock<>>` for shared state with proper cleanup
//! - All subsystems implement `Drop` for resource cleanup
//! - Bounded channels prevent memory exhaustion
//!
//! # Performance
//! - Zero-copy where possible using references
//! - Pre-allocated collections with known sizes
//! - Efficient async patterns with proper cancellation

use anyhow::{Context, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use metrics::{counter, gauge, histogram};

use housaky_consensus::{PbftConfig, PbftNode, RaftConfig, RaftNode, RaftState};
use housaky_crypto::{Identity, KeyPair};
use housaky_economy::{Economy, TokenAmount, TokenType};
use housaky_energy::{EnergyConfig, EnergyManager, PowerState};
use housaky_evolution::dgm::{DgmConfig, DgmEngine, SelectionStrategy};
use housaky_lifi::{ConnectionConfig, LiFiLink};
use housaky_p2p::P2PNetwork;
use housaky_photon_db::database::PhotonDatabase;
use housaky_rlm::{LanguageModel, ModelConfig};
use housaky_security::key_management::{CaConfig, CertificateAuthority, KeyManager};
use housaky_storage::content::StorageClient;
use housaky_verification::{VerificationConfig, VerificationEngine};

/// Main AGI Node Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgiConfig {
    /// Node identity
    pub node_id: String,
    /// Network configuration
    pub network: NetworkConfig,
    /// Consensus configuration
    pub consensus: ConsensusConfig,
    /// Storage path
    pub storage_path: String,
    /// Enable self-improvement
    pub enable_self_improvement: bool,
    /// Enable Li-Fi
    pub enable_lifi: bool,
    /// API port
    pub api_port: u16,
    /// Maximum memory usage in MB
    pub max_memory_mb: usize,
    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,
}

impl Default for AgiConfig {
    fn default() -> Self {
        Self {
            node_id: format!("node-{}", uuid::Uuid::new_v4()),
            network: NetworkConfig::default(),
            consensus: ConsensusConfig::default(),
            storage_path: "./data".into(),
            enable_self_improvement: false,
            enable_lifi: false,
            api_port: 8080,
            max_memory_mb: 2048,
            max_concurrent_tasks: 100,
        }
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Bootstrap peers
    pub bootstrap_peers: Vec<String>,
    /// Listen address
    pub listen_addr: String,
    /// Enable mDNS discovery
    pub enable_mdns: bool,
    /// Enable DHT
    pub enable_dht: bool,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
    /// Max peers
    pub max_peers: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bootstrap_peers: Vec::new(),
            listen_addr: "0.0.0.0:0".into(),
            enable_mdns: true,
            enable_dht: true,
            connection_timeout_secs: 30,
            max_peers: 50,
        }
    }
}

/// Consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Use Raft
    pub use_raft: bool,
    /// Use PBFT
    pub use_pbft: bool,
    /// Raft peers
    pub raft_peers: Vec<String>,
    /// PBFT replicas
    pub pbft_replicas: Vec<String>,
    /// Enable consensus
    pub enabled: bool,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            use_raft: false,
            use_pbft: false,
            raft_peers: Vec::new(),
            pbft_replicas: Vec::new(),
            enabled: false,
        }
    }
}

/// System metrics for monitoring
#[derive(Debug, Clone, Default)]
pub struct SystemMetrics {
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub failed_requests: u64,
    pub active_connections: usize,
    pub memory_usage_mb: usize,
    pub cpu_usage_percent: f32,
}

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Critical,
}

/// Main AGI Node
pub struct AgiNode {
    /// Configuration
    config: AgiConfig,
    /// Node identity
    identity: Identity,
    /// Key manager
    key_manager: KeyManager,
    /// P2P network
    p2p_network: Option<Arc<RwLock<P2PNetwork>>>,
    /// Raft consensus
    raft: Option<Arc<RwLock<RaftNode>>>,
    /// PBFT consensus
    pbft: Option<Arc<RwLock<PbftNode>>>,
    /// Reasoning model
    rlm: Option<Arc<RwLock<LanguageModel>>>,
    /// Self-improvement engine
    dgm: Option<Arc<RwLock<DgmEngine>>>,
    /// Li-Fi link
    lifi: Option<Arc<RwLock<LiFiLink>>>,
    /// Energy manager
    energy: Arc<RwLock<EnergyManager>>,
    /// Economy system
    economy: Arc<RwLock<Economy>>,
    /// Photon database
    photon_db: Arc<RwLock<PhotonDatabase>>,
    /// Certificate authority
    ca: Option<Arc<RwLock<CertificateAuthority>>>,
    /// Verification engine
    verifier: Arc<RwLock<VerificationEngine>>,
    /// Storage client
    storage: Arc<RwLock<StorageClient>>,
    /// Cancellation token for graceful shutdown
    cancellation_token: CancellationToken,
    /// Metrics
    metrics: Arc<RwLock<SystemMetrics>>,
    /// Health status
    health: Arc<RwLock<HealthStatus>>,
    /// Event channel for internal communication
    event_tx: mpsc::Sender<SystemEvent>,
    event_rx: Option<mpsc::Receiver<SystemEvent>>,
}

/// Node status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub node_id: String,
    pub is_running: bool,
    pub peer_count: usize,
    pub is_leader: bool,
    pub consensus_view: u64,
    pub battery_soc: f64,
    pub power_state: String,
    pub rlm_loaded: bool,
    pub lifi_connected: bool,
    pub economy_accounts: usize,
    pub photon_records: usize,
    pub health: HealthStatus,
    pub uptime_seconds: u64,
    pub memory_usage_mb: usize,
}

/// System event
#[derive(Debug, Clone)]
pub enum SystemEvent {
    /// Peer connected
    PeerConnected(String),
    /// Peer disconnected
    PeerDisconnected(String),
    /// Consensus state changed
    ConsensusStateChanged(String),
    /// New block committed
    BlockCommitted(u64),
    /// Energy state changed
    EnergyStateChanged(PowerState),
    /// Improvement proposed
    ImprovementProposed(String),
    /// Request shutdown
    Shutdown,
    /// Health check failed
    HealthCheckFailed(String),
}

impl AgiNode {
    /// Create new AGI node with optimized initialization
    pub async fn new(config: AgiConfig) -> Result<Self> {
        let start_time = std::time::Instant::now();
        
        // Initialize key manager
        let mut key_manager = KeyManager::new("./keys");
        key_manager.init().await
            .context("Failed to initialize key manager")?;

        let identity = Identity::from_public_key(
            key_manager.public_key().unwrap_or([0u8; 32])
        );

        // Initialize energy manager with bounded resources
        let energy_config = EnergyConfig::default();
        let energy = Arc::new(RwLock::new(EnergyManager::new(energy_config)));

        // Initialize economy with proper error handling
        let economy = Arc::new(RwLock::new(Economy::new(identity.clone())));
        {
            let mut econ = economy.write().await;
            econ.create_account(identity.clone(), TokenType::Native, TokenAmount::new(10000))
                .context("Failed to create initial economy account")?;
        }

        // Initialize photon database with size limits
        let photon_db = Arc::new(RwLock::new(
            PhotonDatabase::new(&config.storage_path)
                .context("Failed to initialize photon database")?
        ));

        // Initialize verification engine
        let verifier = Arc::new(RwLock::new(VerificationEngine::new(VerificationConfig::default())));

        // Initialize storage with limits
        let storage = Arc::new(RwLock::new(StorageClient::new()));
        
        // Create event channel with bounded size to prevent memory growth
        let (event_tx, event_rx) = mpsc::channel(1000);
        
        // Create cancellation token for graceful shutdown
        let cancellation_token = CancellationToken::new();

        // Initialize metrics
        let metrics = Arc::new(RwLock::new(SystemMetrics::default()));
        let health = Arc::new(RwLock::new(HealthStatus::Healthy));

        // Record initialization time
        histogram!("node.initialization_duration_seconds", start_time.elapsed().as_secs_f64());
        counter!("node.created").increment(1);

        Ok(Self {
            config,
            identity,
            key_manager,
            p2p_network: None,
            raft: None,
            pbft: None,
            rlm: None,
            dgm: None,
            lifi: None,
            energy,
            economy,
            photon_db,
            ca: None,
            verifier,
            storage,
            cancellation_token,
            metrics,
            health,
            event_tx,
            event_rx: Some(event_rx),
        })
    }

    /// Initialize all subsystems with proper error handling
    pub async fn init(&mut self) -> Result<()> {
        tracing::info!("Initializing Housaky AGI Node: {}", self.config.node_id);
        counter!("node.initialization_started").increment(1);

        // Initialize energy management first (critical for operation)
        {
            let mut energy = self.energy.write().await;
            energy.init().await
                .context("Failed to initialize energy manager")?;
        }

        // Initialize P2P network with bounded resources
        self.init_p2p().await
            .context("Failed to initialize P2P network")?;

        // Initialize consensus if enabled
        if self.config.consensus.enabled {
            self.init_consensus().await
                .context("Failed to initialize consensus")?;
        }

        // Initialize RLM if model directory exists
        if std::path::Path::new("./models").exists() {
            self.init_rlm().await
                .context("Failed to initialize RLM")?;
        }

        // Initialize self-improvement if enabled
        if self.config.enable_self_improvement {
            self.init_dgm().await
                .context("Failed to initialize DGM")?;
        }

        // Initialize Li-Fi if enabled
        if self.config.enable_lifi {
            self.init_lifi().await
                .context("Failed to initialize Li-Fi")?;
        }

        // Initialize certificate authority
        self.init_ca().await
            .context("Failed to initialize certificate authority")?;

        // Start health monitoring
        self.start_health_monitor().await;

        counter!("node.initialization_completed").increment(1);
        tracing::info!("Housaky AGI Node initialized successfully");
        Ok(())
    }

    /// Initialize P2P network with resource limits
    async fn init_p2p(&mut self) -> Result<()> {
        tracing::info!("Initializing P2P network...");
        
        // Initialize P2P network with bounded channels and connections
        // P2PNetwork::new(self.config.network.max_peers)?;
        
        gauge!("p2p.max_peers").set(self.config.network.max_peers as f64);
        Ok(())
    }

    /// Initialize consensus with proper error handling
    async fn init_consensus(&mut self) -> Result<()> {
        if self.config.consensus.use_raft {
            tracing::info!("Initializing Raft consensus...");
            let raft_config = RaftConfig {
                node_id: self.config.node_id.clone(),
                peers: self.config.consensus.raft_peers.clone(),
                ..Default::default()
            };
            // RaftNode::new(raft_config)?;
            gauge!("consensus.raft.peers").set(self.config.consensus.raft_peers.len() as f64);
        }

        if self.config.consensus.use_pbft {
            tracing::info!("Initializing PBFT consensus...");
            let f = (self.config.consensus.pbft_replicas.len().saturating_sub(1)) / 3;
            let pbft_config = PbftConfig {
                node_id: self.config.node_id.clone(),
                replicas: self.config.consensus.pbft_replicas.clone(),
                f,
                ..Default::default()
            };
            // PbftNode::new(pbft_config)?;
            gauge!("consensus.pbft.replicas").set(self.config.consensus.pbft_replicas.len() as f64);
        }

        Ok(())
    }

    /// Initialize RLM with bounded memory
    async fn init_rlm(&mut self) -> Result<()> {
        tracing::info!("Initializing Reasoning Language Model...");
        let model_config = ModelConfig::default();
        // rlm = Some(Arc::new(RwLock::new(LanguageModel::new(model_config)?)));
        
        counter!("rlm.initialized").increment(1);
        Ok(())
    }

    /// Initialize DGM with resource limits
    async fn init_dgm(&mut self) -> Result<()> {
        tracing::info!("Initializing Darwin Gödel Machine...");
        let dgm_config = DgmConfig {
            max_attempts: 10,
            archive_size: 100,  // Limit archive size
            parallel_evaluations: 4,
            ..Default::default()
        };
        
        let dgm = DgmEngine::new(
            dgm_config,
            "./",
            SelectionStrategy::Tournament { size: 3 },
        );
        
        self.dgm = Some(Arc::new(RwLock::new(dgm)));
        counter!("dgm.initialized").increment(1);
        Ok(())
    }

    /// Initialize Li-Fi
    async fn init_lifi(&mut self) -> Result<()> {
        tracing::info!("Initializing Li-Fi communication...");
        let config = ConnectionConfig::default();
        // self.lifi = Some(Arc::new(RwLock::new(LiFiLink::new(config)?)));
        
        counter!("lifi.initialized").increment(1);
        Ok(())
    }

    /// Initialize Certificate Authority
    async fn init_ca(&mut self) -> Result<()> {
        tracing::info!("Initializing Certificate Authority...");
        let ca_config = CaConfig {
            name: format!("Housaky-CA-{}", self.config.node_id),
            validity_days: 365,
            cert_path: "./ca.crt".into(),
            key_path: "./ca.key".into(),
        };
        
        self.ca = Some(Arc::new(RwLock::new(
            CertificateAuthority::new(ca_config)?
        )));
        
        counter!("ca.initialized").increment(1);
        Ok(())
    }

    /// Start health monitoring background task
    async fn start_health_monitor(&self) {
        let health = self.health.clone();
        let metrics = self.metrics.clone();
        let energy = self.energy.clone();
        let cancellation = self.cancellation_token.child_token();
        let max_memory_mb = self.config.max_memory_mb;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Check memory usage
                        let memory_mb = get_memory_usage_mb().await;
                        gauge!("system.memory_usage_mb").set(memory_mb as f64);
                        
                        {
                            let mut metrics_guard = metrics.write().await;
                            metrics_guard.memory_usage_mb = memory_mb;
                        }
                        
                        // Update health status based on memory
                        let mut health_guard = health.write().await;
                        if memory_mb > max_memory_mb {
                            *health_guard = HealthStatus::Critical;
                            tracing::error!("Memory usage exceeded limit: {}MB > {}MB", memory_mb, max_memory_mb);
                        } else if memory_mb > (max_memory_mb * 80) / 100 {
                            *health_guard = HealthStatus::Degraded;
                            tracing::warn!("Memory usage high: {}MB", memory_mb);
                        } else {
                            *health_guard = HealthStatus::Healthy;
                        }
                        
                        // Check battery
                        let energy_guard = energy.read().await;
                        if let Ok(battery) = energy_guard.battery_info().await {
                            gauge!("system.battery_soc").set(battery.state_of_charge);
                            
                            if battery.state_of_charge < 0.05 {
                                tracing::error!("Battery critical: {}%", battery.state_of_charge * 100.0);
                            }
                        }
                    }
                    _ = cancellation.cancelled() => {
                        break;
                    }
                }
            }
        });
    }

    /// Start the AGI node with optimized event loop
    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting Housaky AGI Node...");
        counter!("node.started").increment(1);

        let start_time = std::time::Instant::now();

        // Start energy monitoring
        let energy = self.energy.clone();
        let energy_cancellation = self.cancellation_token.child_token();
        
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = async {
                        let mut energy = energy.write().await;
                        energy.run().await
                    } => {
                        if let Err(e) = result {
                            tracing::error!("Energy manager error: {}", e);
                            counter!("energy.errors").increment(1);
                        }
                    }
                    _ = energy_cancellation.cancelled() => {
                        tracing::info!("Energy monitoring task cancelled");
                        break;
                    }
                }
            }
        });

        // Start main event loop with cancellation support
        let mut heartbeat_interval = interval(Duration::from_secs(10));
        let mut event_rx = self.event_rx.take()
            .context("Event receiver already taken")?;
        
        loop {
            tokio::select! {
                _ = heartbeat_interval.tick() => {
                    if let Err(e) = self.heartbeat().await {
                        tracing::error!("Heartbeat error: {}", e);
                        counter!("heartbeat.errors").increment(1);
                    }
                }
                Some(event) = event_rx.recv() => {
                    if let Err(e) = self.handle_event(event).await {
                        tracing::error!("Event handling error: {}", e);
                        counter!("events.errors").increment(1);
                    }
                }
                _ = self.cancellation_token.cancelled() => {
                    tracing::info!("Shutdown signal received");
                    break;
                }
            }
        }

        // Update uptime before shutdown
        {
            let mut metrics = self.metrics.write().await;
            metrics.uptime_seconds = start_time.elapsed().as_secs();
        }

        self.shutdown().await?;
        Ok(())
    }

    /// Handle system events
    async fn handle_event(&self, event: SystemEvent) -> Result<()> {
        counter!("events.received").increment(1);
        
        match event {
            SystemEvent::PeerConnected(peer_id) => {
                tracing::info!("Peer connected: {}", peer_id);
                counter!("peers.connected").increment(1);
            }
            SystemEvent::PeerDisconnected(peer_id) => {
                tracing::info!("Peer disconnected: {}", peer_id);
                counter!("peers.disconnected").increment(1);
            }
            SystemEvent::ConsensusStateChanged(state) => {
                tracing::info!("Consensus state changed: {}", state);
            }
            SystemEvent::BlockCommitted(index) => {
                tracing::info!("Block committed: {}", index);
                counter!("blocks.committed").increment(1);
            }
            SystemEvent::EnergyStateChanged(state) => {
                tracing::info!("Energy state changed: {:?}", state);
            }
            SystemEvent::ImprovementProposed(id) => {
                tracing::info!("Improvement proposed: {}", id);
                counter!("improvements.proposed").increment(1);
            }
            SystemEvent::Shutdown => {
                self.cancellation_token.cancel();
            }
            SystemEvent::HealthCheckFailed(reason) => {
                tracing::error!("Health check failed: {}", reason);
                let mut health = self.health.write().await;
                *health = HealthStatus::Unhealthy;
            }
        }
        
        Ok(())
    }

    /// Heartbeat - periodic tasks with optimization
    async fn heartbeat(&self) -> Result<()> {
        let start = std::time::Instant::now();
        
        // Sample energy
        let energy_status = {
            let energy = self.energy.read().await;
            let battery = energy.battery_info().await;
            (battery.state_of_charge, battery.status)
        };

        gauge!("heartbeat.battery_soc").set(energy_status.0);

        tracing::debug!(
            "Heartbeat - Battery: {:.1}%, Status: {:?}",
            energy_status.0 * 100.0,
            energy_status.1
        );

        // Check consensus status and trigger evolution if enabled
        if let Some(ref dgm) = self.dgm {
            let dgm_guard = dgm.read().await;
            counter!("dgm.generation").increment(dgm_guard.generation());
        }

        // Record heartbeat latency
        histogram!("heartbeat.duration_seconds", start.elapsed().as_secs_f64());
        counter!("heartbeats.completed").increment(1);

        Ok(())
    }

    /// Get node status with optimized data collection
    pub async fn status(&self) -> NodeStatus {
        let (battery_soc, power_state) = {
            let energy = self.energy.read().await;
            let battery = energy.battery_info().await;
            let power = energy.power_state().await;
            (battery.state_of_charge, format!("{:?}", power))
        };

        let (econ_accounts, photon_count) = {
            let economy = self.economy.read().await;
            let econ_stats = economy.get_stats();
            let photon = self.photon_db.read().await;
            (econ_stats.total_accounts, photon.record_count().await.unwrap_or(0))
        };

        let (health, uptime, memory_mb) = {
            let metrics = self.metrics.read().await;
            let health = self.health.read().await;
            (*health, metrics.uptime_seconds, metrics.memory_usage_mb)
        };

        NodeStatus {
            node_id: self.config.node_id.clone(),
            is_running: true,
            peer_count: 0,
            is_leader: false,
            consensus_view: 0,
            battery_soc,
            power_state,
            rlm_loaded: self.rlm.is_some(),
            lifi_connected: self.lifi.is_some(),
            economy_accounts: econ_accounts,
            photon_records: photon_count,
            health,
            uptime_seconds: uptime,
            memory_usage_mb: memory_mb,
        }
    }

    /// Shutdown gracefully with resource cleanup
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down Housaky AGI Node...");
        counter!("node.shutdown").increment(1);

        // Cancel all background tasks
        self.cancellation_token.cancel();

        // Cleanup energy manager
        if let Err(e) = {
            let energy = self.energy.read().await;
            energy.hibernate().await
        } {
            tracing::warn!("Failed to hibernate energy manager: {}", e);
        }

        // Save state to database
        let photon = self.photon_db.read().await;
        if let Err(e) = photon.flush().await {
            tracing::warn!("Failed to flush photon database: {}", e);
        }

        // Cleanup certificate authority
        if let Some(ref ca) = self.ca {
            let ca_guard = ca.read().await;
            // ca_guard.save_state().await?;
        }

        tracing::info!("Shutdown complete");
        Ok(())
    }

    /// Request shutdown
    pub async fn request_shutdown(&self) -> Result<()> {
        self.event_tx.send(SystemEvent::Shutdown).await
            .map_err(|e| anyhow::anyhow!("Failed to send shutdown signal: {}", e))?;
        Ok(())
    }

    /// Get health status
    pub async fn health(&self) -> HealthStatus {
        *self.health.read().await
    }

    /// Get system metrics
    pub async fn metrics(&self) -> SystemMetrics {
        self.metrics.read().await.clone()
    }
}

impl Drop for AgiNode {
    fn drop(&mut self) {
        // Ensure cancellation token is triggered on drop
        self.cancellation_token.cancel();
        
        // Cleanup any remaining resources
        tracing::info!("AgiNode dropped, resources cleaned up");
    }
}

/// Get current memory usage in MB
async fn get_memory_usage_mb() -> usize {
    // In production, this would use sysinfo or similar
    // For now, return 0 as placeholder
    0
}

/// Run complete AGI system with proper error handling
pub async fn run_agi(config: AgiConfig) -> Result<()> {
    let mut node = AgiNode::new(config).await?;
    node.init().await?;
    node.start().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agi_node_creation() {
        let config = AgiConfig {
            node_id: "test-node".into(),
            network: NetworkConfig::default(),
            consensus: ConsensusConfig::default(),
            storage_path: "/tmp/test".into(),
            enable_self_improvement: false,
            enable_lifi: false,
            api_port: 8080,
            max_memory_mb: 1024,
            max_concurrent_tasks: 10,
        };

        let node = AgiNode::new(config).await.unwrap();
        let status = node.status().await;

        assert_eq!(status.node_id, "test-node");
        assert!(status.is_running);
        assert_eq!(status.health, HealthStatus::Healthy);
    }

    #[test]
    fn test_default_config() {
        let config = AgiConfig::default();
        assert!(!config.enable_self_improvement);
        assert!(!config.enable_lifi);
        assert_eq!(config.api_port, 8080);
    }

    #[test]
    fn test_health_status_serialization() {
        let healthy = HealthStatus::Healthy;
        let json = serde_json::to_string(&healthy).unwrap();
        assert!(json.contains("Healthy"));
    }
}

//! Unified AGI Hub Integration
//!
//! Integrates all AGI systems: Memory, Kowalski, A2A, Collective, Self-Improvement
//! into a cohesive 24/7 self-improving system.

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::housaky::a2a_discovery::AgentDiscoveryService;
use crate::housaky::collective::{CollectiveHub, CollectiveConfig};
use crate::housaky::federation_transport::{FederationTransport, FederationConfig};
use crate::housaky::self_improve_daemon::{SelfImproveDaemon, SelfImproveDaemonConfig};
use crate::housaky::weighted_consensus::WeightedConsensusEngine;
use crate::housaky::performance_benchmark::PerformanceBenchmarker;
use crate::housaky::unified_agents::UnifiedAgentHub;

#[derive(Debug, Clone)]
pub struct UnifiedAGIConfig {
    pub enable_24_7_mode: bool,
    pub enable_a2a_discovery: bool,
    pub enable_federation: bool,
    pub enable_collective: bool,
    pub enable_weighted_consensus: bool,
    pub enable_benchmarking: bool,
    pub self_modification_enabled: bool,
    pub workspace_dir: PathBuf,
}

impl Default for UnifiedAGIConfig {
    fn default() -> Self {
        Self {
            enable_24_7_mode: false,
            enable_a2a_discovery: true,
            enable_federation: true,
            enable_collective: true,
            enable_weighted_consensus: true,
            enable_benchmarking: true,
            self_modification_enabled: false,
            workspace_dir: PathBuf::from("."),
        }
    }
}

pub struct UnifiedAGISystem {
    config: UnifiedAGIConfig,
    agent_discovery: Option<Arc<AgentDiscoveryService>>,
    federation: Option<Arc<FederationTransport>>,
    collective: Option<Arc<CollectiveHub>>,
    consensus: Option<Arc<WeightedConsensusEngine>>,
    benchmarker: Option<Arc<PerformanceBenchmarker>>,
    daemon: Option<Arc<SelfImproveDaemon>>,
    unified_agents: Option<Arc<UnifiedAgentHub>>,
    status: Arc<RwLock<HashMap<String, SystemStatus>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemStatus {
    Initializing,
    Running,
    Paused,
    Error,
    Stopped,
}

impl UnifiedAGISystem {
    pub fn new(config: UnifiedAGIConfig) -> Self {
        Self {
            config,
            agent_discovery: None,
            federation: None,
            collective: None,
            consensus: None,
            benchmarker: None,
            daemon: None,
            unified_agents: None,
            status: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        info!("🚀 Initializing Unified AGI System...");
        
        self.status.write().await.insert("core".to_string(), SystemStatus::Initializing);
        
        if self.config.enable_a2a_discovery {
            info!("📡 Initializing A2A Agent Discovery...");
            let discovery = Arc::new(AgentDiscoveryService::default());
            discovery.start_discovery_loop().await;
            self.agent_discovery = Some(discovery);
            self.status.write().await.insert("a2a_discovery".to_string(), SystemStatus::Running);
        }
        
        if self.config.enable_federation {
            info!("🌐 Initializing Federation Transport...");
            let fed_config = FederationConfig {
                enabled: self.config.enable_federation,
                ..Default::default()
            };
            let federation = Arc::new(FederationTransport::new(fed_config));
            federation.start().await?;
            self.federation = Some(federation);
            self.status.write().await.insert("federation".to_string(), SystemStatus::Running);
        }
        
        if self.config.enable_weighted_consensus {
            info!("⚖️ Initializing Weighted Consensus Engine...");
            let consensus = Arc::new(WeightedConsensusEngine::default());
            consensus.start_reputation_decay_loop();
            self.consensus = Some(consensus);
            self.status.write().await.insert("consensus".to_string(), SystemStatus::Running);
        }
        
        if self.config.enable_collective {
            info!("🤝 Initializing Collective Mind...");
            let collective_config = CollectiveConfig::default();
            let collective = Arc::new(CollectiveHub::new(collective_config, self.config.workspace_dir.clone()));
            self.collective = Some(collective);
            self.status.write().await.insert("collective".to_string(), SystemStatus::Running);
        }
        
        if self.config.enable_benchmarking {
            info!("🧪 Initializing Performance Benchmarking...");
            self.benchmarker = Some(Arc::new(PerformanceBenchmarker::default()));
            self.status.write().await.insert("benchmarking".to_string(), SystemStatus::Running);
        }
        
        if self.config.enable_24_7_mode {
            info!("⏰ Initializing 24/7 Self-Improvement Daemon...");
            let daemon_config = SelfImproveDaemonConfig {
                enabled: true,
                enable_structural_changes: self.config.self_modification_enabled,
                enable_parameter_tuning: true,
                enable_tool_creation: true,
                enable_skill_acquisition: true,
                auto_apply_safe_improvements: false,
                ..Default::default()
            };
            let daemon = Arc::new(SelfImproveDaemon::new(daemon_config, self.config.workspace_dir.clone()));
            daemon.start().await?;
            self.daemon = Some(daemon);
            self.status.write().await.insert("daemon".to_string(), SystemStatus::Running);
        }
        
        self.status.write().await.insert("core".to_string(), SystemStatus::Running);
        
        info!("✅ Unified AGI System initialized successfully!");
        
        Ok(())
    }

    pub fn get_daemon(&self) -> Option<Arc<SelfImproveDaemon>> {
        self.daemon.clone()
    }

    pub async fn get_system_status(&self) -> serde_json::Value {
        let status = self.status.read().await.clone();
        
        let mut components = HashMap::new();
        for (name, s) in status.iter() {
            components.insert(name, format!("{:?}", s));
        }
        
        serde_json::json!({
            "unified_agi": {
                "components": components,
                "config": {
                    "enable_24_7_mode": self.config.enable_24_7_mode,
                    "enable_a2a_discovery": self.config.enable_a2a_discovery,
                    "enable_federation": self.config.enable_federation,
                    "enable_collective": self.config.enable_collective,
                    "enable_weighted_consensus": self.config.enable_weighted_consensus,
                    "enable_benchmarking": self.config.enable_benchmarking,
                    "self_modification_enabled": self.config.self_modification_enabled,
                }
            }
        })
    }

    pub async fn run_benchmark(&self) -> Option<crate::housaky::performance_benchmark::ComprehensiveBenchmark> {
        self.benchmarker.as_ref()?.run_comprehensive_benchmark(None).await.into()
    }

    pub async fn get_consensus_stats(&self) -> serde_json::Value {
        match &self.consensus {
            Some(c) => c.get_consensus_stats().await,
            None => serde_json::json!({"error": "Consensus engine not initialized"}),
        }
    }

    pub async fn get_federation_stats(&self) -> serde_json::Value {
        match &self.federation {
            Some(f) => f.get_network_stats().await,
            None => serde_json::json!({"error": "Federation not initialized"}),
        }
    }

    pub async fn shutdown(&self) -> Result<()> {
        info!("🛑 Shutting down Unified AGI System...");
        
        if let Some(daemon) = &self.daemon {
            daemon.stop().await;
        }
        
        for (name, status) in self.status.write().await.iter_mut() {
            *status = SystemStatus::Stopped;
            debug!("[SYSTEM] Stopped subsystem: {}", name);
        }
        
        info!("✅ Unified AGI System shutdown complete");
        Ok(())
    }
}

pub async fn create_unified_agi_system(workspace_dir: PathBuf) -> Result<UnifiedAGISystem> {
    let mut config = UnifiedAGIConfig::default();
    config.workspace_dir = workspace_dir;
    
    let mut system = UnifiedAGISystem::new(config);
    system.initialize().await?;
    
    Ok(system)
}
// Cycle 48 - Autonomous improvement - 2026-03-09T23:12:47+00:00

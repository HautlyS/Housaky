//! Unified Multi-Agent Hub
//!
//! Consolidates all 4 parallel multi-agent systems into a single coherent system:
//! - MultiAgentCoordinator (in-memory channels)
//! - FederationHub + NetworkTransport (HTTP federation)
//! - CollaborationManager (filesystem-based HIIP)
//! - SubAgentOrchestrator (direct LLM API calls)
//! - KowalskiBridge (CLI subprocess invocation)
//!
//! This module provides a unified interface for:
//! - Agent registration and discovery
//! - Task assignment and coordination
//! - Knowledge sharing across instances
//! - Consensus-based decision making

use crate::housaky::collaboration::{CollaborationManager, HIIPMessage, MessageType, Priority};
use crate::housaky::federation::{
    FederationConfig, FederationHub, FederationTransportLayer, KnowledgeDelta, KnowledgeEntry,
    NetworkTransport, Peer, PeerStatus, SyncResult, TransportConfig,
};
use crate::housaky::kowalski_integration::{KowalskiBridge, KowalskiAgentType, TaskResult as KowalskiTaskResult};
use crate::housaky::multi_agent::agent_registry::{AgentInfo, AgentPerformance, AgentRegistry, AgentType};
use crate::housaky::multi_agent::coordinator::{
    AgentTask, CoordinationStrategy, MultiAgentCoordinator, TaskPriority, TaskResult, TaskStatus,
};
use crate::housaky::multi_agent::emergent_protocol::EmergentProtocol;
use crate::housaky::multi_agent::message::{AgentMessage, MessageType as AgentMessageType};
use crate::housaky::multi_agent::replication::{AgentReplicator, ForkRequest, Specialization};
use crate::housaky::subagent_system::{AgentRole, SubAgentOrchestrator};
use crate::housaky::federation::sync::{GSet, LWWRegister, VectorClock};
use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// Configuration for the unified multi-agent system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAgentConfig {
    /// Enable local multi-agent coordination
    pub enable_local_coordination: bool,
    /// Enable federation with remote instances
    pub enable_federation: bool,
    /// Enable filesystem-based collaboration (HIIP)
    pub enable_collaboration: bool,
    /// Enable Kowalski CLI agents
    pub enable_kowalski: bool,
    /// Enable direct LLM sub-agents
    pub enable_subagents: bool,
    /// Enable agent replication/forking
    pub enable_replication: bool,
    /// Enable emergent protocol learning
    pub enable_emergent_protocol: bool,
    /// Maximum concurrent tasks across all agents
    pub max_concurrent_tasks: usize,
    /// Heartbeat interval for agent health checks (seconds)
    pub heartbeat_interval_secs: u64,
    /// Workspace directory for state persistence
    pub workspace_dir: PathBuf,
    /// Local instance ID
    pub instance_id: String,
    /// Federation peer addresses
    pub federation_peers: Vec<String>,
    /// Kowalski configuration
    pub kowalski_config: Option<crate::housaky::housaky_agent::KowalskiIntegrationConfig>,
}

impl Default for UnifiedAgentConfig {
    fn default() -> Self {
        Self {
            enable_local_coordination: true,
            enable_federation: false,
            enable_collaboration: false,
            enable_kowalski: false,
            enable_subagents: false,
            enable_replication: false,
            enable_emergent_protocol: true,
            max_concurrent_tasks: 10,
            heartbeat_interval_secs: 30,
            workspace_dir: PathBuf::from("."),
            instance_id: format!("housaky-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
            federation_peers: Vec::new(),
            kowalski_config: None,
        }
    }
}

/// Unified task that can be dispatched to any agent system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTask {
    pub id: String,
    pub title: String,
    pub description: String,
    pub priority: UnifiedPriority,
    pub required_capabilities: Vec<String>,
    pub preferred_system: Option<AgentSystem>,
    pub context: HashMap<String, String>,
    pub deadline: Option<chrono::DateTime<Utc>>,
    pub status: UnifiedTaskStatus,
    pub assigned_agent: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub started_at: Option<chrono::DateTime<Utc>>,
    pub completed_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnifiedPriority {
    Background = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum UnifiedTaskStatus {
    Pending,
    Assigned,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentSystem {
    Local,           // MultiAgentCoordinator
    Federation,      // FederationHub
    Collaboration,   // CollaborationManager (HIIP)
    Kowalski,        // KowalskiBridge
    SubAgent,        // SubAgentOrchestrator
}

/// Result from task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTaskResult {
    pub task_id: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub executed_by: String,
    pub system: AgentSystem,
    pub artifacts: Vec<TaskArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskArtifact {
    pub name: String,
    pub artifact_type: String,
    pub content: String,
}

/// Hub statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedHubStats {
    pub total_agents: usize,
    pub available_agents: usize,
    pub pending_tasks: usize,
    pub active_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub success_rate: f64,
    pub avg_execution_time_ms: u64,
    pub federation_peers: usize,
    pub federation_online: usize,
    pub kowalski_agents: usize,
    pub subagents: usize,
    pub emergent_symbols: usize,
}

/// Response channel registry for fixing the oneshot channel bug
struct ResponseChannelRegistry {
    channels: Arc<RwLock<HashMap<String, mpsc::Sender<String>>>>,
}

impl ResponseChannelRegistry {
    fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn register(&self, id: &str) -> mpsc::Receiver<String> {
        let (tx, rx) = mpsc::channel(1);
        self.channels.write().await.insert(id.to_string(), tx);
        rx
    }

    async fn respond(&self, id: &str, response: String) -> Result<()> {
        let channels = self.channels.read().await;
        if let Some(tx) = channels.get(id) {
            tx.send(response).await.context("Failed to send response")?;
        }
        Ok(())
    }

    async fn remove(&self, id: &str) {
        self.channels.write().await.remove(id);
    }
}

/// The unified multi-agent hub
pub struct UnifiedAgentHub {
    config: Arc<RwLock<UnifiedAgentConfig>>,

    // Core coordination
    registry: Arc<AgentRegistry>,
    coordinator: Arc<MultiAgentCoordinator>,
    response_channels: Arc<ResponseChannelRegistry>,

    // Federation
    federation_hub: Option<Arc<FederationHub>>,
    federation_transport: Option<Arc<FederationTransportLayer>>,
    vector_clock: Arc<RwLock<VectorClock>>,

    // Collaboration (HIIP)
    collaboration: Option<Arc<CollaborationManager>>,

    // Kowalski
    kowalski_bridge: Option<Arc<RwLock<KowalskiBridge>>>,

    // Sub-agents (direct LLM)
    subagent_orchestrator: Option<Arc<RwLock<SubAgentOrchestrator>>>,

    // Replication
    replicator: Option<Arc<AgentReplicator>>,

    // Emergent protocol
    emergent_protocol: Arc<EmergentProtocol>,

    // Task tracking
    unified_tasks: Arc<RwLock<HashMap<String, UnifiedTask>>>,
    completed_tasks: Arc<RwLock<Vec<UnifiedTaskResult>>>,

    // Message bus for cross-system communication
    message_bus: broadcast::Sender<UnifiedHubMessage>,

    // Shared knowledge (using CRDTs)
    shared_knowledge: Arc<RwLock<HashMap<String, LWWRegister<String>>>>,
    learned_facts: Arc<RwLock<GSet<String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedHubMessage {
    pub id: String,
    pub msg_type: UnifiedMessageType,
    pub source_system: AgentSystem,
    pub sender: String,
    pub content: String,
    pub timestamp: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UnifiedMessageType {
    TaskSubmitted,
    TaskAssigned,
    TaskCompleted,
    TaskFailed,
    AgentRegistered,
    AgentHeartbeat,
    KnowledgeShared,
    ConsensusRequest,
    ConsensusResponse,
    FederationSync,
    EmergentSymbol,
}

impl UnifiedAgentHub {
    pub fn unified_tasks(&self) -> &Arc<RwLock<HashMap<String, UnifiedTask>>> {
        &self.unified_tasks
    }
    pub fn new(config: UnifiedAgentConfig) -> Self {
        let (message_bus, _) = broadcast::channel(512);

        let mut hub = Self {
            config: Arc::new(RwLock::new(config.clone())),
            registry: Arc::new(AgentRegistry::new()),
            coordinator: Arc::new(MultiAgentCoordinator::new()),
            response_channels: Arc::new(ResponseChannelRegistry::new()),
            federation_hub: None,
            federation_transport: None,
            vector_clock: Arc::new(RwLock::new(VectorClock::new())),
            collaboration: None,
            kowalski_bridge: None,
            subagent_orchestrator: None,
            replicator: None,
            emergent_protocol: Arc::new(EmergentProtocol::new()),
            unified_tasks: Arc::new(RwLock::new(HashMap::new())),
            completed_tasks: Arc::new(RwLock::new(Vec::new())),
            message_bus,
            shared_knowledge: Arc::new(RwLock::new(HashMap::new())),
            learned_facts: Arc::new(RwLock::new(GSet::new())),
        };

        // Initialize enabled subsystems
        if config.enable_federation {
            let fed_config = FederationConfig {
                enabled: true,
                peers: config.federation_peers.clone(),
                ..Default::default()
            };
            hub.federation_hub = Some(Arc::new(FederationHub::new(fed_config.clone())));

            let transport_config = TransportConfig::default();
            hub.federation_transport = Some(Arc::new(FederationTransportLayer::new(
                transport_config,
                fed_config,
                &config.instance_id,
            )));
        }

        if config.enable_collaboration {
            hub.collaboration = Some(Arc::new(CollaborationManager::new(
                config.workspace_dir.clone(),
                &config.instance_id,
                "peer", // Will be updated on peer discovery
            )));
        }

        if config.enable_kowalski {
            if let Some(kowalski_config) = &config.kowalski_config {
                hub.kowalski_bridge = Some(Arc::new(RwLock::new(KowalskiBridge::new(kowalski_config))));
            }
        }

        if config.enable_subagents {
            hub.subagent_orchestrator = Some(Arc::new(RwLock::new(SubAgentOrchestrator::new())));
        }

        if config.enable_replication {
            hub.replicator = Some(Arc::new(AgentReplicator::new(
                config.workspace_dir.join("agents"),
                5, // max children
            )));
        }

        hub
    }

    /// Initialize the hub and all enabled subsystems
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing Unified Multi-Agent Hub...");

        // Register self as coordinator agent
        let self_agent = AgentInfo {
            id: self.config.read().await.instance_id.clone(),
            name: "housaky-coordinator".to_string(),
            agent_type: AgentType::Coordinator,
            capabilities: vec![
                "coordination".to_string(),
                "task_dispatch".to_string(),
                "knowledge_merge".to_string(),
            ],
            available: true,
            performance_metrics: AgentPerformance::default(),
            registered_at: Utc::now(),
            last_heartbeat: Utc::now(),
            current_task: None,
            metadata: HashMap::new(),
        };
        self.registry.register(self_agent).await?;

        // Initialize Kowalski if enabled
        if let Some(bridge) = &self.kowalski_bridge {
            let mut bridge = bridge.write().await;
            if let Err(e) = bridge.initialize_agents().await {
                warn!("Kowalski initialization failed (non-fatal): {}", e);
            } else {
                // Register Kowalski agents in our registry
                for (name, status) in bridge.get_agent_status() {
                    let agent_info = AgentInfo {
                        id: name.clone(),
                        name: name.clone(),
                        agent_type: AgentType::Specialist,
                        capabilities: vec!["kowalski".to_string()],
                        available: matches!(
                            status,
                            crate::housaky::kowalski_integration::AgentStatus::Available
                        ),
                        performance_metrics: AgentPerformance::default(),
                        registered_at: Utc::now(),
                        last_heartbeat: Utc::now(),
                        current_task: None,
                        metadata: {
                            let mut m = HashMap::new();
                            m.insert("system".to_string(), "kowalski".to_string());
                            m
                        },
                    };
                    let _ = self.registry.register(agent_info).await;
                }
                info!("Kowalski agents initialized and registered");
            }
        }

        // Connect to federation peers if enabled
        if let Some(transport) = &self.federation_transport {
            let connected = transport.connect_to_peers().await;
            info!("Federation: connected to {}/{} peers", connected, 
                  self.config.read().await.federation_peers.len());
        }

        // Initialize emergent protocol with common symbols
        self.emergent_protocol.define_symbol("$ACK", "task acknowledged", &self.config.read().await.instance_id).await;
        self.emergent_protocol.define_symbol("$DONE", "task completed successfully", &self.config.read().await.instance_id).await;
        self.emergent_protocol.define_symbol("$FAIL", "task failed", &self.config.read().await.instance_id).await;
        self.emergent_protocol.define_symbol("$HELP", "need assistance", &self.config.read().await.instance_id).await;

        info!("Unified Multi-Agent Hub initialized");
        Ok(())
    }

    /// Submit a task to be executed by the best available agent
    pub async fn submit_task(&self, task: UnifiedTask) -> Result<String> {
        let task_id = task.id.clone();
        info!("Submitting unified task: {} (priority: {:?})", task.title, task.priority);

        // Tick vector clock for causal ordering
        self.vector_clock.write().await.tick(&self.config.read().await.instance_id);

        // Store in unified task map
        self.unified_tasks.write().await.insert(task_id.clone(), task.clone());

        // Broadcast task submission
        self.broadcast_message(UnifiedMessageType::TaskSubmitted, &task.title).await;

        // Route to best system
        let system = self.select_best_system(&task).await;
        info!("Routing task {} to {:?} system", task_id, system);

        match system {
            AgentSystem::Local => {
                let coord_task = self.convert_to_coordinator_task(&task);
                self.coordinator.submit_task(coord_task).await?;
            }
            AgentSystem::Kowalski => {
                if let Some(bridge) = &self.kowalski_bridge {
                    let agent_type = self.infer_kowalski_agent_type(&task);
                    let bridge = bridge.read().await;
                    let agent_name = format!("kowalski-{}", agent_type.as_str());
                    if bridge.get_health().active_agents.contains(&agent_name) {
                        // Task will be executed in execute_pending_tasks
                    }
                }
            }
            AgentSystem::SubAgent => {
                if let Some(orchestrator) = &self.subagent_orchestrator {
                    let role = self.infer_subagent_role(&task);
                    let _orchestrator = orchestrator.read().await;
                    // Task will be executed in execute_pending_tasks with the role
                    let mut tasks = self.unified_tasks.write().await;
                    if let Some(t) = tasks.get_mut(&task_id) {
                        t.context.insert("subagent_role".to_string(), format!("{:?}", role));
                    }
                }
            }
            AgentSystem::Federation => {
                if let Some(hub) = &self.federation_hub {
                    // Share task as knowledge for federated execution
                    hub.share_knowledge(
                        &format!("task:{}", task_id),
                        &serde_json::to_string(&task)?,
                        0.8,
                    ).await;
                }
            }
            AgentSystem::Collaboration => {
                if let Some(collab) = &self.collaboration {
                    collab.send_message(
                        MessageType::Task,
                        Priority::High,
                        serde_json::json!({
                            "task_id": task_id,
                            "title": task.title,
                            "description": task.description,
                        }),
                    ).await?;
                }
            }
        }

        // Update task status
        let mut tasks = self.unified_tasks.write().await;
        if let Some(t) = tasks.get_mut(&task_id) {
            t.status = UnifiedTaskStatus::Assigned;
            t.context.insert("target_system".to_string(), format!("{:?}", system));
        }

        Ok(task_id)
    }

    /// Execute all pending tasks across all systems
    pub async fn execute_pending_tasks(&self) -> Result<Vec<UnifiedTaskResult>> {
        let mut results = Vec::new();

        // Get all assigned tasks
        let tasks: Vec<_> = {
            let tasks = self.unified_tasks.read().await;
            tasks.values()
                .filter(|t| t.status == UnifiedTaskStatus::Assigned || t.status == UnifiedTaskStatus::Pending)
                .cloned()
                .collect()
        };

        for task in tasks {
            let system_str = task.context.get("target_system").map(|s| s.as_str()).unwrap_or("Local");
            let system = match system_str {
                "Kowalski" => AgentSystem::Kowalski,
                "SubAgent" => AgentSystem::SubAgent,
                "Federation" => AgentSystem::Federation,
                "Collaboration" => AgentSystem::Collaboration,
                _ => AgentSystem::Local,
            };

            // Mark as in progress
            {
                let mut tasks = self.unified_tasks.write().await;
                if let Some(t) = tasks.get_mut(&task.id) {
                    t.status = UnifiedTaskStatus::InProgress;
                    t.started_at = Some(Utc::now());
                }
            }

            let start = std::time::Instant::now();
            let result = match system {
                AgentSystem::Kowalski => self.execute_kowalski_task(&task).await,
                AgentSystem::SubAgent => self.execute_subagent_task(&task).await,
                AgentSystem::Federation => self.execute_federation_task(&task).await,
                AgentSystem::Collaboration => self.execute_collaboration_task(&task).await,
                AgentSystem::Local => self.execute_local_task(&task).await,
            };

            let execution_time = start.elapsed().as_millis() as u64;

            let unified_result = match result {
                Ok((output, executed_by)) => {
                    // Update task status
                    {
                        let mut tasks = self.unified_tasks.write().await;
                        if let Some(t) = tasks.get_mut(&task.id) {
                            t.status = UnifiedTaskStatus::Completed;
                            t.completed_at = Some(Utc::now());
                        }
                    }
                    
                    UnifiedTaskResult {
                        task_id: task.id.clone(),
                        success: true,
                        output,
                        error: None,
                        execution_time_ms: execution_time,
                        executed_by,
                        system,
                        artifacts: Vec::new(),
                    }
                }
                Err(e) => {
                    // Update task status
                    {
                        let mut tasks = self.unified_tasks.write().await;
                        if let Some(t) = tasks.get_mut(&task.id) {
                            t.status = UnifiedTaskStatus::Failed;
                        }
                    }
                    
                    UnifiedTaskResult {
                        task_id: task.id.clone(),
                        success: false,
                        output: String::new(),
                        error: Some(e.to_string()),
                        execution_time_ms: execution_time,
                        executed_by: "none".to_string(),
                        system,
                        artifacts: Vec::new(),
                    }
                }
            };

            // Broadcast completion/failure
            if unified_result.success {
                self.broadcast_message(UnifiedMessageType::TaskCompleted, &task.id).await;
            } else {
                self.broadcast_message(UnifiedMessageType::TaskFailed, &task.id).await;
            }

            // Store result
            self.completed_tasks.write().await.push(unified_result.clone());
            results.push(unified_result);
        }

        // Trigger local coordinator assignment
        if let Err(e) = self.coordinator.assign_tasks().await {
            warn!("Local coordinator assignment failed: {}", e);
        }

        Ok(results)
    }

    async fn execute_kowalski_task(&self, task: &UnifiedTask) -> Result<(String, String)> {
        let bridge = self.kowalski_bridge.as_ref()
            .context("Kowalski not enabled")?;
        
        let bridge = bridge.read().await;
        let agent_type = self.infer_kowalski_agent_type(task);
        
        // Try GLM-based execution first
        match bridge.execute_with_glm(&agent_type, &task.description).await {
            Ok(output) => Ok((output, format!("kowalski-{}", agent_type.as_str()))),
            Err(e) => {
                warn!("GLM execution failed, trying CLI: {}", e);
                // Fall back to CLI
                let agent_name = format!("kowalski-{}", agent_type.as_str());
                match bridge.send_task(&agent_name, &task.description).await {
                    Ok(result) => {
                        if result.success {
                            Ok((result.output, agent_name))
                        } else {
                            Err(anyhow::anyhow!(result.error.unwrap_or_else(|| "Unknown error".to_string())))
                        }
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    async fn execute_subagent_task(&self, task: &UnifiedTask) -> Result<(String, String)> {
        let orchestrator = self.subagent_orchestrator.as_ref()
            .context("SubAgent not enabled")?;
        
        let orchestrator = orchestrator.read().await;
        let role_str = task.context.get("subagent_role").map(|s| s.as_str()).unwrap_or("FederationCoordinator");
        let agent_name = match role_str {
            "CodeSpecialist" => Some("kowalski-code"),
            "WebResearcher" => Some("kowalski-web"),
            "AcademicAnalyst" => Some("kowalski-academic"),
            "DataProcessor" => Some("kowalski-data"),
            "CreativeSynthesizer" => Some("kowalski-creative"),
            "ReasoningEngine" => Some("kowalski-reasoning"),
            _ => Some("kowalski-federation"),
        };

        match orchestrator.process(&task.description, agent_name).await {
            Ok(response) => Ok((response.content, agent_name.unwrap_or("subagent").to_string())),
            Err(e) => Err(e),
        }
    }

    async fn execute_federation_task(&self, task: &UnifiedTask) -> Result<(String, String)> {
        let hub = self.federation_hub.as_ref()
            .context("Federation not enabled")?;
        
        // For federated tasks, we share knowledge and wait for peer responses
        hub.share_knowledge(
            &format!("task_exec:{}", task.id),
            &task.description,
            0.9,
        ).await;

        // Sync with peers if transport is available
        if let Some(transport) = &self.federation_transport {
            let delta = KnowledgeDelta {
                source_peer: self.config.read().await.instance_id.clone(),
                timestamp: Utc::now(),
                version: *hub.local_version.read().await,
                additions: vec![KnowledgeEntry {
                    id: task.id.clone(),
                    key: format!("task:{}", task.id),
                    value: task.description.clone(),
                    source: self.config.read().await.instance_id.clone(),
                    confidence: 0.9,
                    version: 1,
                    updated_at: Utc::now(),
                }],
                modifications: Vec::new(),
                deletions: Vec::new(),
            };
            let results = transport.sync_delta_to_all(&delta).await;
            let successful = results.iter().filter(|r| r.success).count();
            Ok((format!("Federated task submitted to {} peers", successful), "federation".to_string()))
        } else {
            Ok(("Task queued for federation".to_string(), "federation".to_string()))
        }
    }

    async fn execute_collaboration_task(&self, task: &UnifiedTask) -> Result<(String, String)> {
        let collab = self.collaboration.as_ref()
            .context("Collaboration not enabled")?;
        
        collab.send_message(
            MessageType::Task,
            Priority::High,
            serde_json::json!({
                "task_id": task.id,
                "title": task.title,
                "description": task.description,
                "action": "execute",
            }),
        ).await?;

        Ok(("Task sent via HIIP protocol".to_string(), "collaboration".to_string()))
    }

    async fn execute_local_task(&self, task: &UnifiedTask) -> Result<(String, String)> {
        // Convert and submit to local coordinator
        let coord_task = self.convert_to_coordinator_task(task);
        let task_id = self.coordinator.submit_task(coord_task).await?;
        
        // Assign and wait for completion
        let assignments = self.coordinator.assign_tasks().await?;
        
        if let Some((_, agent_id)) = assignments.iter().find(|(tid, _)| tid == &task_id) {
            Ok((format!("Task assigned to local agent {}", agent_id), agent_id.clone()))
        } else {
            Ok(("Task queued for local execution".to_string(), "coordinator".to_string()))
        }
    }

    /// Request consensus from all available agents
    pub async fn request_consensus(&self, question: &str) -> Result<ConsensusResult> {
        info!("Requesting consensus: {}", question);

        let mut responses: HashMap<String, Vec<String>> = HashMap::new();
        let mut total_responders = 0;

        // Query local agents
        let local_agents = self.registry.list_available_agents().await;
        for agent in &local_agents {
            // Use fixed response channel pattern
            let resp_id = format!("consensus_{}", uuid::Uuid::new_v4());
            let mut rx = self.response_channels.register(&resp_id).await;

            let message = AgentMessage {
                id: format!("query_{}", uuid::Uuid::new_v4()),
                msg_type: AgentMessageType::Query,
                sender: self.config.read().await.instance_id.clone(),
                receiver: Some(agent.id.clone()),
                content: question.to_string(),
                timestamp: Utc::now(),
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("response_id".to_string(), resp_id.clone());
                    m
                },
            };

            let _ = self.registry.send_to_agent(&agent.id, message).await;

            // Wait with timeout
            if let Ok(Some(response)) = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                rx.recv(),
            ).await {
                responses.entry(response).or_default().push(agent.id.clone());
                total_responders += 1;
            }

            self.response_channels.remove(&resp_id).await;
        }

        // Query Kowalski agents if available
        if let Some(bridge) = &self.kowalski_bridge {
            let bridge = bridge.read().await;
            for agent_type in [KowalskiAgentType::Code, KowalskiAgentType::Web, KowalskiAgentType::Academic] {
                if let Ok(response) = bridge.execute_with_glm(&agent_type, &format!(
                    "Answer this question concisely: {}", question
                )).await {
                    responses.entry(response).or_default().push(format!("kowalski-{}", agent_type.as_str()));
                    total_responders += 1;
                }
            }
        }

        // Query sub-agents if available
        if let Some(orchestrator) = &self.subagent_orchestrator {
            let orchestrator = orchestrator.read().await;
            if let Ok(response) = orchestrator.process(
                &format!("Answer this question concisely: {}", question),
                Some("kowalski-reasoning"),
            ).await {
                responses.entry(response.content).or_default().push("reasoning-agent".to_string());
                total_responders += 1;
            }
        }

        // Find consensus
        let (consensus_answer, supporters) = responses
            .into_iter()
            .max_by_key(|(_, supporters)| supporters.len())
            .unwrap_or(("No consensus".to_string(), Vec::new()));

        let agreement_ratio = if total_responders > 0 {
            supporters.len() as f64 / total_responders as f64
        } else {
            0.0
        };

        Ok(ConsensusResult {
            question: question.to_string(),
            consensus: consensus_answer,
            agreement_ratio,
            total_responders,
            supporters,
        })
    }

    /// Share knowledge across all enabled systems
    pub async fn share_knowledge(&self, key: &str, value: &str, confidence: f64) -> Result<()> {
        let instance_id = self.config.read().await.instance_id.clone();

        // Store locally with CRDT
        {
            let mut knowledge = self.shared_knowledge.write().await;
            let register = LWWRegister::new(value.to_string(), &instance_id);
            knowledge.insert(key.to_string(), register);
        }

        // Add to learned facts set
        {
            let mut facts = self.learned_facts.write().await;
            facts.add(format!("{}={}", key, value));
        }

        // Share via federation
        if let Some(hub) = &self.federation_hub {
            hub.share_knowledge(key, value, confidence).await;
        }

        // Share via collaboration
        if let Some(collab) = &self.collaboration {
            collab.share_insight(crate::housaky::collaboration::InsightPayload {
                category: "knowledge".to_string(),
                title: key.to_string(),
                content: value.to_string(),
                tags: vec!["auto-shared".to_string()],
                source_file: None,
            }).await?;
        }

        // Broadcast
        self.broadcast_message(UnifiedMessageType::KnowledgeShared, key).await;

        info!("Knowledge shared: {} (confidence: {:.2})", key, confidence);
        Ok(())
    }

    /// Run a heartbeat cycle checking all agent health
    pub async fn heartbeat(&self) -> Result<()> {
        debug!("Running unified agent heartbeat");

        // Update vector clock
        self.vector_clock.write().await.tick(&self.config.read().await.instance_id);

        // Check local registry for stale agents
        let stale = self.registry.check_stale_agents(
            self.config.read().await.heartbeat_interval_secs as i64 * 3
        ).await;
        if !stale.is_empty() {
            warn!("Stale agents detected: {:?}", stale);
        }

        // Health check federation peers
        if let Some(transport) = &self.federation_transport {
            let health = transport.health_check_all().await;
            for (peer_id, ok) in health {
                if !ok {
                    warn!("Federation peer {} is not responding", peer_id);
                }
            }
        }

        // Check Kowalski agents
        if let Some(bridge) = &self.kowalski_bridge {
            let bridge = bridge.read().await;
            if let Err(e) = bridge.coordinate_agents().await {
                warn!("Kowalski coordination check failed: {}", e);
            }
        }

        // Collaboration heartbeat
        if let Some(collab) = &self.collaboration {
            collab.heartbeat(None).await?;
        }

        // Evolve emergent protocol
        if self.config.read().await.enable_emergent_protocol {
            self.emergent_protocol.evolve().await;
        }

        // Broadcast heartbeat
        self.broadcast_message(UnifiedMessageType::AgentHeartbeat, "heartbeat").await;

        Ok(())
    }

    /// Get unified hub statistics
    pub async fn get_stats(&self) -> UnifiedHubStats {
        let registry_stats = self.registry.get_registry_stats().await;
        let coordinator_stats = self.coordinator.get_coordinator_stats().await;
        let tasks = self.unified_tasks.read().await;
        let completed = self.completed_tasks.read().await;
        let protocol_stats = self.emergent_protocol.get_stats().await;

        let pending = tasks.values().filter(|t| t.status == UnifiedTaskStatus::Pending).count();
        let active = tasks.values().filter(|t| t.status == UnifiedTaskStatus::InProgress).count();
        let failed = completed.iter().filter(|r| !r.success).count();
        let success_rate = if !completed.is_empty() {
            completed.iter().filter(|r| r.success).count() as f64 / completed.len() as f64
        } else {
            0.0
        };
        let avg_time = if !completed.is_empty() {
            completed.iter().map(|r| r.execution_time_ms).sum::<u64>() / completed.len() as u64
        } else {
            0
        };

        let fed_stats = if let Some(hub) = &self.federation_hub {
            let s = hub.get_stats().await;
            (s.total_peers, s.online_peers)
        } else {
            (0, 0)
        };

        let kowalski_count = if let Some(bridge) = &self.kowalski_bridge {
            bridge.read().await.get_health().active_agents.len()
        } else {
            0
        };

        let subagent_count = if let Some(orchestrator) = &self.subagent_orchestrator {
            orchestrator.read().await.status().len()
        } else {
            0
        };

        UnifiedHubStats {
            total_agents: registry_stats.total_agents,
            available_agents: registry_stats.available_agents,
            pending_tasks: pending,
            active_tasks: active,
            completed_tasks: completed.len(),
            failed_tasks: failed,
            success_rate,
            avg_execution_time_ms: avg_time,
            federation_peers: fed_stats.0,
            federation_online: fed_stats.1,
            kowalski_agents: kowalski_count,
            subagents: subagent_count,
            emergent_symbols: protocol_stats.total_symbols,
        }
    }

    /// Subscribe to hub messages
    pub fn subscribe(&self) -> broadcast::Receiver<UnifiedHubMessage> {
        self.message_bus.subscribe()
    }

    // -------------------------------------------------------------------------
    // Internal helpers
    // -------------------------------------------------------------------------

    async fn select_best_system(&self, task: &UnifiedTask) -> AgentSystem {
        // If preferred system is specified and enabled, use it
        if let Some(preferred) = task.preferred_system {
            if self.is_system_enabled(&preferred).await {
                return preferred;
            }
        }

        // Check capabilities to route to best system
        for cap in &task.required_capabilities {
            let cap_lower = cap.to_lowercase();
            
            // Code-related tasks -> Kowalski code agent or local
            if cap_lower.contains("code") || cap_lower.contains("program") {
                if self.kowalski_bridge.is_some() {
                    return AgentSystem::Kowalski;
                }
            }

            // Web research -> Kowalski web agent
            if cap_lower.contains("web") || cap_lower.contains("search") || cap_lower.contains("research") {
                if self.kowalski_bridge.is_some() {
                    return AgentSystem::Kowalski;
                }
            }

            // Academic -> Kowalski academic agent
            if cap_lower.contains("academic") || cap_lower.contains("paper") {
                if self.kowalski_bridge.is_some() {
                    return AgentSystem::Kowalski;
                }
            }

            // Reasoning -> SubAgent reasoning engine
            if cap_lower.contains("reason") || cap_lower.contains("logic") {
                if self.subagent_orchestrator.is_some() {
                    return AgentSystem::SubAgent;
                }
            }

            // Creative -> SubAgent creative synthesizer
            if cap_lower.contains("creative") || cap_lower.contains("brainstorm") {
                if self.subagent_orchestrator.is_some() {
                    return AgentSystem::SubAgent;
                }
            }
        }

        // Default to local coordination
        AgentSystem::Local
    }

    async fn is_system_enabled(&self, system: &AgentSystem) -> bool {
        let config = self.config.read().await;
        match system {
            AgentSystem::Local => config.enable_local_coordination,
            AgentSystem::Federation => config.enable_federation && self.federation_hub.is_some(),
            AgentSystem::Collaboration => config.enable_collaboration && self.collaboration.is_some(),
            AgentSystem::Kowalski => config.enable_kowalski && self.kowalski_bridge.is_some(),
            AgentSystem::SubAgent => config.enable_subagents && self.subagent_orchestrator.is_some(),
        }
    }

    fn convert_to_coordinator_task(&self, task: &UnifiedTask) -> AgentTask {
        AgentTask {
            id: task.id.clone(),
            description: task.description.clone(),
            required_capabilities: task.required_capabilities.clone(),
            priority: match task.priority {
                UnifiedPriority::Critical => TaskPriority::Critical,
                UnifiedPriority::High => TaskPriority::High,
                UnifiedPriority::Medium => TaskPriority::Medium,
                UnifiedPriority::Low => TaskPriority::Low,
                UnifiedPriority::Background => TaskPriority::Background,
            },
            dependencies: Vec::new(),
            deadline: task.deadline,
            context: task.context.clone(),
            status: TaskStatus::Pending,
        }
    }

    fn infer_kowalski_agent_type(&self, task: &UnifiedTask) -> KowalskiAgentType {
        let desc_lower = task.description.to_lowercase();
        let caps_lower: Vec<_> = task.required_capabilities.iter()
            .map(|s| s.to_lowercase())
            .collect();

        if desc_lower.contains("code") || caps_lower.iter().any(|c| c.contains("code")) {
            KowalskiAgentType::Code
        } else if desc_lower.contains("web") || desc_lower.contains("search") || caps_lower.iter().any(|c| c.contains("web")) {
            KowalskiAgentType::Web
        } else if desc_lower.contains("academic") || desc_lower.contains("paper") || caps_lower.iter().any(|c| c.contains("academic")) {
            KowalskiAgentType::Academic
        } else if desc_lower.contains("data") || caps_lower.iter().any(|c| c.contains("data")) {
            KowalskiAgentType::Data
        } else {
            KowalskiAgentType::Federated
        }
    }

    fn infer_subagent_role(&self, task: &UnifiedTask) -> AgentRole {
        let desc_lower = task.description.to_lowercase();
        let caps_lower: Vec<_> = task.required_capabilities.iter()
            .map(|s| s.to_lowercase())
            .collect();

        if desc_lower.contains("code") || caps_lower.iter().any(|c| c.contains("code")) {
            AgentRole::CodeSpecialist
        } else if desc_lower.contains("web") || desc_lower.contains("search") {
            AgentRole::WebResearcher
        } else if desc_lower.contains("academic") || desc_lower.contains("paper") {
            AgentRole::AcademicAnalyst
        } else if desc_lower.contains("data") || caps_lower.iter().any(|c| c.contains("data")) {
            AgentRole::DataProcessor
        } else if desc_lower.contains("creative") || desc_lower.contains("brainstorm") {
            AgentRole::CreativeSynthesizer
        } else if desc_lower.contains("reason") || desc_lower.contains("logic") {
            AgentRole::ReasoningEngine
        } else {
            AgentRole::FederationCoordinator
        }
    }

    async fn broadcast_message(&self, msg_type: UnifiedMessageType, content: &str) {
        let message = UnifiedHubMessage {
            id: format!("msg_{}", uuid::Uuid::new_v4()),
            msg_type,
            source_system: AgentSystem::Local,
            sender: self.config.read().await.instance_id.clone(),
            content: content.to_string(),
            timestamp: Utc::now(),
        };

        let _ = self.message_bus.send(message);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    pub question: String,
    pub consensus: String,
    pub agreement_ratio: f64,
    pub total_responders: usize,
    pub supporters: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_unified_hub_creation() {
        let config = UnifiedAgentConfig::default();
        let hub = UnifiedAgentHub::new(config);
        hub.initialize().await.unwrap();

        let stats = hub.get_stats().await;
        assert!(stats.total_agents >= 1); // At least self
    }

    #[tokio::test]
    async fn test_task_submission() {
        let config = UnifiedAgentConfig::default();
        let hub = UnifiedAgentHub::new(config);
        hub.initialize().await.unwrap();

        let task = UnifiedTask {
            id: "test-task-1".to_string(),
            title: "Test Task".to_string(),
            description: "A test task".to_string(),
            priority: UnifiedPriority::Medium,
            required_capabilities: vec!["general".to_string()],
            preferred_system: None,
            context: HashMap::new(),
            deadline: None,
            status: UnifiedTaskStatus::Pending,
            assigned_agent: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        };

        let task_id = hub.submit_task(task).await.unwrap();
        assert_eq!(task_id, "test-task-1");
    }

    #[tokio::test]
    async fn test_knowledge_sharing() {
        let config = UnifiedAgentConfig::default();
        let hub = UnifiedAgentHub::new(config);
        hub.initialize().await.unwrap();

        hub.share_knowledge("test_key", "test_value", 0.9).await.unwrap();

        let knowledge = hub.shared_knowledge.read().await;
        assert!(knowledge.contains_key("test_key"));
    }
}

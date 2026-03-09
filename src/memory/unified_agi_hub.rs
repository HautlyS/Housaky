// ☸️ UNIFIED AGI MEMORY HUB
// Integrates: Lucid, Intelligent Memory, Project Context, Kowalski, A2A, OpenClaw, Collective Mind
// Purpose: 24/7 self-improving AGI with never-forget context and federated awareness

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

use super::traits::{Memory, MemoryCategory, MemoryEntry};
use super::lucid::LucidMemory;
use super::intelligent_memory::{IntelligentMemory, IntelligentMemoryConfig, MemoryImportance, ContextBudget};
use super::project_context::{
    AgentAwarenessEngine, AgentState, AwarenessLevel, AwarenessContext,
    ContextSwitcher, ContextLevel, ProjectContext, ContextEntry, ConnectionGraph,
    FederationAwareContext, ConnectionType,
};

// ============================================================================
// UNIFIED AGI MEMORY HUB CONFIG
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAGIMemoryConfig {
    /// Enable Lucid memory backend
    pub enable_lucid: bool,
    /// Enable intelligent memory (importance/deduplication)
    pub enable_intelligent: bool,
    /// Enable project context switching
    pub enable_project_context: bool,
    /// Enable Kowalski subagent federation
    pub enable_kowalski_federation: bool,
    /// Enable A2A Hub integration
    pub enable_a2a_hub: bool,
    /// Enable OpenClaw integration
    pub enable_openclaw: bool,
    /// Enable 24/7 self-improvement
    pub enable_self_improvement: bool,
    /// Enable collective mind sync
    pub enable_collective_mind: bool,
    /// Context budget for LLM (tokens)
    pub context_budget_tokens: usize,
    /// Self-improvement interval (seconds)
    pub self_improve_interval_secs: u64,
    /// Collective sync interval (seconds)
    pub collective_sync_interval_secs: u64,
    /// Paths
    pub workspace_dir: PathBuf,
    pub a2a_hub_path: Option<PathBuf>,
    pub openclaw_path: Option<PathBuf>,
    pub kowalski_path: Option<PathBuf>,
}

impl Default for UnifiedAGIMemoryConfig {
    fn default() -> Self {
        Self {
            enable_lucid: true,
            enable_intelligent: true,
            enable_project_context: true,
            enable_kowalski_federation: true,
            enable_a2a_hub: true,
            enable_openclaw: true,
            enable_self_improvement: true,
            enable_collective_mind: true,
            context_budget_tokens: 8000,
            self_improve_interval_secs: 3600, // 1 hour
            collective_sync_interval_secs: 300, // 5 minutes
            workspace_dir: PathBuf::from("/home/ubuntu/.housaky/workspace"),
            a2a_hub_path: Some(PathBuf::from("/home/ubuntu/Housaky/landing/A2A/public/shared/memory")),
            openclaw_path: Some(PathBuf::from("/home/ubuntu/Housaky/shared")),
            kowalski_path: Some(PathBuf::from("/home/ubuntu/Housaky/vendor/kowalski")),
        }
    }
}

// ============================================================================
// MEMORY SOURCE TYPES
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnifiedMemorySource {
    Local,
    Intelligent,
    Project,
    Kowalski,
    A2AHub,
    OpenClaw,
    Collective,
    Critical,
}

impl std::fmt::Display for UnifiedMemorySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnifiedMemorySource::Local => write!(f, "local"),
            UnifiedMemorySource::Intelligent => write!(f, "intelligent"),
            UnifiedMemorySource::Project => write!(f, "project"),
            UnifiedMemorySource::Kowalski => write!(f, "kowalski"),
            UnifiedMemorySource::A2AHub => write!(f, "a2a-hub"),
            UnifiedMemorySource::OpenClaw => write!(f, "openclaw"),
            UnifiedMemorySource::Collective => write!(f, "collective"),
            UnifiedMemorySource::Critical => write!(f, "critical"),
        }
    }
}

// ============================================================================
// UNIFIED MEMORY ENTRY
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMemoryEntry {
    pub id: String,
    pub key: String,
    pub content: String,
    pub category: MemoryCategory,
    pub importance: f64,
    pub source: UnifiedMemorySource,
    pub connections: Vec<String>,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub agent_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u32,
    pub tags: Vec<String>,
    pub embedding: Option<Vec<f32>>,
}

impl UnifiedMemoryEntry {
    pub fn new(key: &str, content: &str, source: UnifiedMemorySource) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            key: key.to_string(),
            content: content.to_string(),
            category: MemoryCategory::Core,
            importance: 0.5,
            source,
            connections: Vec::new(),
            project_id: None,
            task_id: None,
            agent_id: None,
            timestamp: now,
            last_accessed: now,
            access_count: 0,
            tags: Vec::new(),
            embedding: None,
        }
    }

    pub fn with_importance(mut self, importance: f64) -> Self {
        self.importance = importance;
        self
    }

    pub fn with_project(mut self, project_id: &str) -> Self {
        self.project_id = Some(project_id.to_string());
        self
    }

    pub fn with_task(mut self, task_id: &str) -> Self {
        self.task_id = Some(task_id.to_string());
        self
    }

    pub fn with_agent(mut self, agent_id: &str) -> Self {
        self.agent_id = Some(agent_id.to_string());
        self
    }

    pub fn with_tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_connections(mut self, connections: Vec<String>) -> Self {
        self.connections = connections;
        self
    }

    pub fn access(&mut self) {
        self.access_count += 1;
        self.last_accessed = Utc::now();
    }

    pub fn to_memory_entry(&self) -> MemoryEntry {
        MemoryEntry {
            id: self.id.clone(),
            key: self.key.clone(),
            content: self.content.clone(),
            category: self.category.clone(),
            timestamp: self.timestamp.to_rfc3339(),
            session_id: self.agent_id.clone(),
            score: Some(self.importance),
        }
    }
}

// ============================================================================
// COLLECTIVE MIND STATE
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectiveMindState {
    pub instance_id: String,
    pub instance_name: String,
    pub peers: HashMap<String, PeerInfo>,
    pub shared_insights: Vec<SharedInsight>,
    pub consensus_topics: Vec<ConsensusTopic>,
    pub last_sync: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: String,
    pub name: String,
    pub role: String,
    pub capabilities: Vec<String>,
    pub last_seen: DateTime<Utc>,
    pub shared_memories: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedInsight {
    pub id: String,
    pub content: String,
    pub source_peer: String,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusTopic {
    pub id: String,
    pub topic: String,
    pub agreements: Vec<String>,
    pub disagreements: Vec<String>,
    pub resolution: Option<String>,
    pub voted_at: DateTime<Utc>,
}

// ============================================================================
// SELF-IMPROVEMENT RECORD
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfImprovementRecord {
    pub id: String,
    pub improvement_type: ImprovementType,
    pub description: String,
    pub before_state: String,
    pub after_state: String,
    pub metrics_delta: HashMap<String, f64>,
    pub success: bool,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
    pub validated_by: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImprovementType {
    CodeOptimization,
    ArchitectureEvolution,
    MemoryOptimization,
    ReasoningImprovement,
    SkillAcquisition,
    KnowledgeUpdate,
    PatternDiscovery,
    ConnectionUpdate,
}

impl std::fmt::Display for ImprovementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImprovementType::CodeOptimization => write!(f, "code_optimization"),
            ImprovementType::ArchitectureEvolution => write!(f, "architecture_evolution"),
            ImprovementType::MemoryOptimization => write!(f, "memory_optimization"),
            ImprovementType::ReasoningImprovement => write!(f, "reasoning_improvement"),
            ImprovementType::SkillAcquisition => write!(f, "skill_acquisition"),
            ImprovementType::KnowledgeUpdate => write!(f, "knowledge_update"),
            ImprovementType::PatternDiscovery => write!(f, "pattern_discovery"),
            ImprovementType::ConnectionUpdate => write!(f, "connection_update"),
        }
    }
}

// ============================================================================
// UNIFIED AGI MEMORY HUB
// ============================================================================

pub struct UnifiedAGIMemoryHub {
    config: UnifiedAGIMemoryConfig,
    
    // Core memory systems
    lucid_memory: Option<Arc<LucidMemory>>,
    intelligent_memory: Option<Arc<IntelligentMemory>>,
    project_context: Option<Arc<AgentAwarenessEngine>>,
    
    // Federation
    federation_context: Arc<RwLock<FederationAwareContext>>,
    
    // Collective mind
    collective_mind: Arc<RwLock<CollectiveMindState>>,
    
    // Self-improvement
    self_improvements: Arc<RwLock<Vec<SelfImprovementRecord>>>,
    last_self_improve: Arc<RwLock<Option<DateTime<Utc>>>>,
    
    // Unified store
    unified_store: Arc<RwLock<HashMap<String, UnifiedMemoryEntry>>>,
    
    // Active agents
    active_agents: Arc<RwLock<HashMap<String, AgentState>>>,
    
    // Status
    is_running: Arc<RwLock<bool>>,
}

impl UnifiedAGIMemoryHub {
    pub fn new(config: UnifiedAGIMemoryConfig) -> Self {
        let uuid_str = uuid::Uuid::new_v4().to_string();
        let instance_id = format!("housaky-{}", &uuid_str[..8]);
        
        Self {
            config: config.clone(),
            lucid_memory: None,
            intelligent_memory: None,
            project_context: None,
            federation_context: Arc::new(RwLock::new(FederationAwareContext::new(
                config.workspace_dir.clone()
            ))),
            collective_mind: Arc::new(RwLock::new(CollectiveMindState {
                instance_id: instance_id.clone(),
                instance_name: "Housaky".to_string(),
                peers: HashMap::new(),
                shared_insights: Vec::new(),
                consensus_topics: Vec::new(),
                last_sync: Utc::now(),
            })),
            self_improvements: Arc::new(RwLock::new(Vec::new())),
            last_self_improve: Arc::new(RwLock::new(None)),
            unified_store: Arc::new(RwLock::new(HashMap::new())),
            active_agents: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Initialize all memory systems
    pub async fn initialize(&mut self) -> Result<()> {
        info!("[UNIFIED-MEMORY] Initializing Unified AGI Memory Hub...");
        
        // Initialize Lucid
        if self.config.enable_lucid {
            let lucid = LucidMemory::with_workspace(&self.config.workspace_dir);
            self.lucid_memory = Some(Arc::new(lucid));
            info!("[UNIFIED-MEMORY] Lucid memory initialized");
        }
        
        // Initialize Intelligent Memory
        if self.config.enable_intelligent {
            let intelligent = IntelligentMemory::new(self.config.workspace_dir.clone());
            self.intelligent_memory = Some(Arc::new(intelligent));
            info!("[UNIFIED-MEMORY] Intelligent memory initialized");
        }
        
        // Initialize Project Context
        if self.config.enable_project_context {
            let context = AgentAwarenessEngine::new(self.config.workspace_dir.clone());
            self.project_context = Some(Arc::new(context));
            info!("[UNIFIED-MEMORY] Project context initialized");
        }
        
        // Initialize A2A Hub sync
        if self.config.enable_a2a_hub {
            if let Some(ref path) = self.config.a2a_hub_path {
                if path.exists() {
                    info!("[UNIFIED-MEMORY] A2A Hub path configured: {:?}", path);
                }
            }
        }
        
        // Initialize OpenClaw sync
        if self.config.enable_openclaw {
            if let Some(ref path) = self.config.openclaw_path {
                if path.exists() {
                    info!("[UNIFIED-MEMORY] OpenClaw path configured: {:?}", path);
                }
            }
        }
        
        // Initialize Kowalski
        if self.config.enable_kowalski_federation {
            if let Some(ref path) = self.config.kowalski_path {
                if path.exists() {
                    info!("[UNIFIED-MEMORY] Kowalski path configured: {:?}", path);
                }
            }
        }
        
        *self.is_running.write().await = true;
        
        info!("[UNIFIED-MEMORY] ✅ All systems initialized");
        
        Ok(())
    }

    /// Store memory across all enabled systems
    pub async fn store(&self, key: &str, content: &str, category: MemoryCategory, source: UnifiedMemorySource) -> Result<String> {
        let entry = UnifiedMemoryEntry::new(key, content, source)
            .with_importance(self.calculate_importance(content, &category));
        
        let entry_id = entry.id.clone();
        
        // Store in unified store
        self.unified_store.write().await.insert(entry_id.clone(), entry.clone());
        
        // Store in Lucid if enabled
        if let Some(ref lucid) = self.lucid_memory {
            lucid.store(key, content, category.clone()).await?;
        }
        
        // Store in Intelligent Memory if enabled
        if let Some(ref intelligent) = self.intelligent_memory {
            intelligent.store(key, content, category.clone()).await?;
        }
        
        // Store in Project Context if enabled
        if let Some(ref context) = self.project_context {
            context.store(key, content, category.clone()).await?;
        }
        
        // Sync to A2A Hub if enabled
        if self.config.enable_a2a_hub {
            self.sync_to_a2a_hub(&entry).await;
        }
        
        // Sync to OpenClaw if enabled
        if self.config.enable_openclaw {
            self.sync_to_openclaw(&entry).await;
        }
        
        debug!("[UNIFIED-MEMORY] Stored: {} (source: {})", key, source);
        
        Ok(entry_id)
    }

    /// Recall memories with context awareness
    pub async fn recall(&self, query: &str, max_tokens: Option<usize>) -> Result<Vec<UnifiedMemoryEntry>> {
        let mut all_entries: Vec<UnifiedMemoryEntry> = Vec::new();
        let mut seen_keys: HashSet<String> = HashSet::new();
        
        let budget = max_tokens.unwrap_or(self.config.context_budget_tokens);
        
        // 1. Get from Intelligent Memory (with importance/deduplication)
        if let Some(ref intelligent) = self.intelligent_memory {
            let results = intelligent.search(query, Some(budget)).await;
            for entry in results.entries {
                if seen_keys.insert(entry.key.clone()) {
                    all_entries.push(UnifiedMemoryEntry::new(&entry.key, &entry.content, UnifiedMemorySource::Intelligent)
                        .with_importance(entry.score.unwrap_or(0.5)));
                }
            }
        }
        
        // 2. Get from Lucid
        if let Some(ref lucid) = self.lucid_memory {
            let results = lucid.recall(query, 20).await?;
            for entry in results {
                if seen_keys.insert(entry.key.clone()) {
                    all_entries.push(UnifiedMemoryEntry::new(&entry.key, &entry.content, UnifiedMemorySource::Local)
                        .with_importance(entry.score.unwrap_or(0.5)));
                }
            }
        }
        
        // 3. Get from Project Context
        if let Some(ref context) = self.project_context {
            let results = context.recall(query, 10).await?;
            for entry in results {
                if seen_keys.insert(entry.key.clone()) {
                    all_entries.push(UnifiedMemoryEntry::new(&entry.key, &entry.content, UnifiedMemorySource::Project)
                        .with_importance(entry.score.unwrap_or(0.5)));
                }
            }
        }
        
        // 4. Get from A2A Hub
        if self.config.enable_a2a_hub {
            let hub_entries = self.fetch_from_a2a_hub(query).await;
            for entry in hub_entries {
                if seen_keys.insert(entry.key.clone()) {
                    all_entries.push(entry);
                }
            }
        }
        
        // 5. Get from OpenClaw
        if self.config.enable_openclaw {
            let claw_entries = self.fetch_from_openclaw(query).await;
            for entry in claw_entries {
                if seen_keys.insert(entry.key.clone()) {
                    all_entries.push(entry);
                }
            }
        }
        
        // Sort by importance
        all_entries.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap_or(std::cmp::Ordering::Equal));
        
        // Apply token budget
        let mut tokens = 0;
        let mut final_entries = Vec::new();
        
        for entry in all_entries {
            let entry_tokens = entry.content.len() / 4;
            if tokens + entry_tokens > budget {
                if entry.importance > 0.8 {
                    final_entries.push(entry); // Always include critical
                }
                continue;
            }
            tokens += entry_tokens;
            final_entries.push(entry);
        }
        
        Ok(final_entries)
    }

    /// Get awareness context for an agent
    pub async fn get_agent_awareness(&self, agent_id: &str, level: AwarenessLevel) -> Result<AwarenessContext> {
        if let Some(ref context) = self.project_context {
            let awareness = context.get_agent_awareness(agent_id).await;
            return Ok(awareness);
        }
        
        Ok(AwarenessContext::default())
    }

    /// Register an agent
    pub async fn register_agent(&self, agent_id: &str, agent_type: &str) -> Result<()> {
        if let Some(ref context) = self.project_context {
            context.register_agent(agent_id, agent_type).await;
        }
        
        // Add to local tracking
        let mut agents = self.active_agents.write().await;
        agents.insert(agent_id.to_string(), AgentState {
            agent_id: agent_id.to_string(),
            agent_type: agent_type.to_string(),
            current_task: None,
            context_entries: Vec::new(),
            awareness_level: AwarenessLevel::Moderate,
            last_update: Utc::now(),
        });
        
        info!("[UNIFIED-MEMORY] Registered agent: {} ({})", agent_id, agent_type);
        
        Ok(())
    }

    /// Create project context
    pub async fn create_project(&self, name: &str, path: PathBuf) -> Result<ProjectContext> {
        if let Some(ref context) = self.project_context {
            return context.local().create_project(name, path).await;
        }
        
        anyhow::bail!("Project context not enabled")
    }

    /// Switch to project
    pub async fn switch_to_project(&self, project_id: &str) -> Result<ProjectContext> {
        if let Some(ref context) = self.project_context {
            return context.local().switch_to_project(project_id).await;
        }
        
        anyhow::bail!("Project context not enabled")
    }

    /// Store design pattern
    pub async fn store_design(&self, pattern: &str, content: &str) -> Result<()> {
        if let Some(ref context) = self.project_context {
            context.local().store_design(pattern, content).await?;
        }
        
        // Also store in unified store
        self.store(pattern, content, MemoryCategory::Core, UnifiedMemorySource::Project).await?;
        
        Ok(())
    }

    // ============================================================================
    // SELF-IMPROVEMENT
    // ============================================================================

    pub async fn run_self_improvement(&self) -> Result<()> {
        if !self.config.enable_self_improvement {
            return Ok(());
        }
        
        let now = Utc::now();
        let last = *self.last_self_improve.read().await;
        
        // Check interval
        if let Some(last_time) = last {
            if (now - last_time).num_seconds() < self.config.self_improve_interval_secs as i64 {
                return Ok(());
            }
        }
        
        info!("[UNIFIED-MEMORY] Running self-improvement cycle...");
        
        // 1. Analyze recent memories for patterns
        let recent = self.get_recent_improvements(10).await;
        
        // 2. Look for optimization opportunities
        let improvements = self.analyze_improvements(&recent).await;
        
        // 3. Apply improvements
        for improvement in &improvements {
            self.apply_improvement(improvement).await?;
        }
        
        // 4. Update last run time
        *self.last_self_improve.write().await = Some(now);
        
        info!("[UNIFIED-MEMORY] Self-improvement complete: {} changes", improvements.len());
        
        Ok(())
    }

    async fn analyze_improvements(&self, recent: &[SelfImprovementRecord]) -> Vec<SelfImprovementRecord> {
        let mut improvements = Vec::new();
        
        // Analyze patterns in recent improvements
        let mut mem_opt_count = 0;
        let mut knowledge_count = 0;
        
        for record in recent {
            match record.improvement_type {
                ImprovementType::MemoryOptimization => mem_opt_count += 1,
                ImprovementType::KnowledgeUpdate => knowledge_count += 1,
                _ => {}
            }
        }
        
        // Add improvement recommendations
        if mem_opt_count > 3 {
            improvements.push(SelfImprovementRecord {
                id: uuid::Uuid::new_v4().to_string(),
                improvement_type: ImprovementType::MemoryOptimization,
                description: "Memory patterns detected - consolidating similar entries".to_string(),
                before_state: format!("{} memory entries", mem_opt_count),
                after_state: "Consolidated memory".to_string(),
                metrics_delta: HashMap::new(),
                success: true,
                confidence: 0.8,
                created_at: Utc::now(),
                validated_by: vec!["self".to_string()],
            });
        }
        
        if knowledge_count > 5 {
            improvements.push(SelfImprovementRecord {
                id: uuid::Uuid::new_v4().to_string(),
                improvement_type: ImprovementType::KnowledgeUpdate,
                description: "Knowledge patterns detected - updating knowledge graph".to_string(),
                before_state: format!("{} knowledge entries", knowledge_count),
                after_state: "Updated knowledge graph".to_string(),
                metrics_delta: HashMap::new(),
                success: true,
                confidence: 0.9,
                created_at: Utc::now(),
                validated_by: vec!["self".to_string()],
            });
        }
        
        improvements
    }

    async fn apply_improvement(&self, improvement: &SelfImprovementRecord) -> Result<()> {
        info!("[UNIFIED-MEMORY] Applying improvement: {}", improvement.description);
        
        // Record the improvement
        self.self_improvements.write().await.push(improvement.clone());
        
        // Perform the actual improvement based on type
        match improvement.improvement_type {
            ImprovementType::MemoryOptimization => {
                // Run memory consolidation
                self.consolidate_memory().await?;
            }
            ImprovementType::KnowledgeUpdate => {
                // Sync to knowledge systems
                self.sync_knowledge().await?;
            }
            _ => {}
        }
        
        Ok(())
    }

    async fn consolidate_memory(&self) -> Result<()> {
        // Get all entries and find duplicates
        let store = self.unified_store.read().await;
        let entries: Vec<_> = store.values().collect();
        
        let mut seen_content: HashSet<String> = HashSet::new();
        let mut duplicates: Vec<String> = Vec::new();
        
        for entry in &entries {
            let content_key = entry.content.to_lowercase();
            if seen_content.contains(&content_key) {
                duplicates.push(entry.id.clone());
            } else {
                seen_content.insert(content_key);
            }
        }
        
        // Remove duplicates from unified store
        let dup_count = duplicates.len();
        drop(store);
        let mut store = self.unified_store.write().await;
        for id in duplicates {
            store.remove(&id);
        }
        
        info!("[UNIFIED-MEMORY] Consolidated {} duplicate entries", dup_count);
        
        Ok(())
    }

    async fn sync_knowledge(&self) -> Result<()> {
        // Sync to A2A Hub
        if self.config.enable_a2a_hub {
            self.sync_state_to_a2a_hub().await;
        }
        
        // Sync to collective mind
        if self.config.enable_collective_mind {
            self.sync_to_collective().await;
        }
        
        Ok(())
    }

    async fn get_recent_improvements(&self, count: usize) -> Vec<SelfImprovementRecord> {
        let improvements = self.self_improvements.read().await;
        improvements.iter().rev().take(count).cloned().collect()
    }

    // ============================================================================
    // COLLECTIVE MIND
    // ============================================================================

    pub async fn sync_collective(&self) -> Result<()> {
        if !self.config.enable_collective_mind {
            return Ok(());
        }
        
        // Sync with collective mind peers
        self.sync_to_collective().await;
        
        Ok(())
    }

    async fn sync_to_collective(&self) {
        // Get current state
        let state = self.collective_mind.read().await;
        
        // Prepare shared insights
        let insights: Vec<SharedInsight> = self.self_improvements.read().await
            .iter()
            .rev()
            .take(5)
            .map(|r| SharedInsight {
                id: r.id.clone(),
                content: r.description.clone(),
                source_peer: state.instance_id.clone(),
                confidence: r.confidence,
                created_at: r.created_at,
                tags: vec![r.improvement_type.to_string()],
            })
            .collect();
        
        // Write to A2A Hub collective
        if let Some(ref hub_path) = self.config.a2a_hub_path {
            let collective_path = hub_path.parent().map(|p| p.join("collective"));
            if let Some(ref path) = collective_path {
                let _ = std::fs::create_dir_all(path);
                let insights_file_path = path.join("insights.jsonl");
                
                for insight in &insights {
                    if let Ok(json) = serde_json::to_string(insight) {
                        let _ = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&insights_file_path)
                            .and_then(|mut f| {
                                use std::io::Write;
                                writeln!(f, "{}", json)
                            });
                    }
                }
            }
        }
        
        info!("[UNIFIED-MEMORY] Synced {} insights to collective", insights.len());
    }

    // ============================================================================
    // A2A HUB SYNC
    // ============================================================================

    async fn sync_to_a2a_hub(&self, entry: &UnifiedMemoryEntry) {
        if let Some(ref hub_path) = self.config.a2a_hub_path {
            let learnings_path = hub_path.join("learnings.jsonl");
            
            let shared = serde_json::json!({
                "id": entry.id,
                "key": entry.key,
                "content": entry.content,
                "category": entry.category.to_string(),
                "timestamp": entry.timestamp.to_rfc3339(),
                "source": entry.source.to_string(),
                "instance": "housaky",
                "confidence": entry.importance,
                "tags": entry.tags,
            });
            
            if let Ok(json) = serde_json::to_string(&shared) {
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(learnings_path)
                    .and_then(|mut f| {
                        use std::io::Write;
                        writeln!(f, "{}", json)
                    });
            }
        }
    }

    async fn sync_state_to_a2a_hub(&self) {
        if let Some(ref hub_path) = self.config.a2a_hub_path {
            let state_path = hub_path.join("current-state.json");
            
            let store = self.unified_store.read().await;
            
            let state = serde_json::json!({
                "instance": "housaky",
                "timestamp": Utc::now().to_rfc3339(),
                "total_memories": store.len(),
                "active_agents": self.active_agents.read().await.len(),
                "improvements": self.self_improvements.read().await.len(),
            });
            
            let _ = std::fs::write(state_path, serde_json::to_string_pretty(&state).unwrap_or_default());
        }
    }

    async fn fetch_from_a2a_hub(&self, query: &str) -> Vec<UnifiedMemoryEntry> {
        if let Some(ref hub_path) = self.config.a2a_hub_path {
            let learnings_path = hub_path.join("learnings.jsonl");
            
            if !learnings_path.exists() {
                return Vec::new();
            }
            
            let query_lower = query.to_lowercase();
            let mut entries = Vec::new();
            
            if let Ok(content) = std::fs::read_to_string(&learnings_path) {
                for line in content.lines().rev().take(50) {
                    if let Ok(shared) = serde_json::from_str::<serde_json::Value>(line) {
                        let content = shared["content"].as_str().unwrap_or("");
                        if content.to_lowercase().contains(&query_lower) {
                            entries.push(UnifiedMemoryEntry::new(
                                shared["key"].as_str().unwrap_or(""),
                                content,
                                UnifiedMemorySource::A2AHub,
                            ).with_importance(shared["confidence"].as_f64().unwrap_or(0.5)));
                        }
                    }
                }
            }
            
            entries
        } else {
            Vec::new()
        }
    }

    // ============================================================================
    // OPENCLAW SYNC
    // ============================================================================

    async fn sync_to_openclaw(&self, entry: &UnifiedMemoryEntry) {
        if let Some(ref claw_path) = self.config.openclaw_path {
            let memory_path = claw_path.join("memory");
            let _ = std::fs::create_dir_all(&memory_path);
            
            let filename = format!("{}.json", entry.id);
            let file_path = memory_path.join(&filename);
            
            let shared = serde_json::json!({
                "id": entry.id,
                "key": entry.key,
                "content": entry.content,
                "category": entry.category.to_string(),
                "timestamp": entry.timestamp.to_rfc3339(),
                "source": "housaky",
                "confidence": entry.importance,
            });
            
            let _ = std::fs::write(file_path, serde_json::to_string_pretty(&shared).unwrap_or_default());
        }
    }

    async fn fetch_from_openclaw(&self, query: &str) -> Vec<UnifiedMemoryEntry> {
        if let Some(ref claw_path) = self.config.openclaw_path {
            let memory_path = claw_path.join("memory");
            
            if !memory_path.exists() {
                return Vec::new();
            }
            
            let query_lower = query.to_lowercase();
            let mut entries = Vec::new();
            
            if let Ok(dir) = std::fs::read_dir(&memory_path) {
                for entry in dir.flatten().take(50) {
                    if entry.path().extension().map_or(false, |e| e == "json") {
                        if let Ok(content) = std::fs::read_to_string(entry.path()) {
                            if let Ok(shared) = serde_json::from_str::<serde_json::Value>(&content) {
                                let content = shared["content"].as_str().unwrap_or("");
                                if content.to_lowercase().contains(&query_lower) {
                                    entries.push(UnifiedMemoryEntry::new(
                                        shared["key"].as_str().unwrap_or(""),
                                        content,
                                        UnifiedMemorySource::OpenClaw,
                                    ).with_importance(shared["confidence"].as_f64().unwrap_or(0.5)));
                                }
                            }
                        }
                    }
                }
            }
            
            entries
        } else {
            Vec::new()
        }
    }

    // ============================================================================
    // UTILITIES
    // ============================================================================

    fn calculate_importance(&self, content: &str, category: &MemoryCategory) -> f64 {
        let content_lower = content.to_lowercase();
        
        // Base importance from category
        let mut importance = match category {
            MemoryCategory::Core => 0.8,
            MemoryCategory::Daily => 0.4,
            MemoryCategory::Conversation => 0.5,
            MemoryCategory::Custom(name) => {
                if name.contains("skill") || name.contains("pattern") {
                    0.85
                } else if name.contains("procedure") {
                    0.7
                } else {
                    0.5
                }
            }
        };
        
        // Boost for important keywords
        let critical_keywords = ["important", "critical", "remember", "never forget", "fixed", "bug", "error", "learned"];
        for keyword in critical_keywords {
            if content_lower.contains(keyword) {
                importance += 0.1;
            }
        }
        
        // Reduce for trivial content
        let trivial_keywords = ["hello", "thanks", "okay", "sure"];
        for keyword in trivial_keywords {
            if content_lower.contains(keyword) {
                importance -= 0.2;
            }
        }
        
        let importance: f64 = importance.max(0.0).min(1.0);
        importance
    }

    /// Get memory statistics
    pub async fn get_stats(&self) -> UnifiedMemoryStats {
        let store = self.unified_store.read().await;
        let agents = self.active_agents.read().await;
        let improvements = self.self_improvements.read().await;
        let collective = self.collective_mind.read().await;
        
        UnifiedMemoryStats {
            total_entries: store.len(),
            critical_entries: store.values().filter(|e| e.importance > 0.9).count(),
            project_entries: store.values().filter(|e| e.project_id.is_some()).count(),
            active_agents: agents.len(),
            total_improvements: improvements.len(),
            collective_peers: collective.peers.len(),
            last_self_improve: *self.last_self_improve.read().await,
            is_running: *self.is_running.read().await,
        }
    }

    /// Format context for LLM
    pub async fn format_for_llm(&self, query: &str, max_tokens: usize) -> String {
        let entries = self.recall(query, Some(max_tokens)).await.unwrap_or_default();
        
        let mut output = vec![
            "=== HOUSAKY UNIFIED MEMORY ===".to_string(),
            format!("Query: {}", query),
            format!("Found: {} relevant memories\n", entries.len()),
        ];
        
        // Group by source
        let mut by_source: HashMap<UnifiedMemorySource, Vec<UnifiedMemoryEntry>> = HashMap::new();
        for entry in entries {
            by_source.entry(entry.source).or_insert_with(Vec::new).push(entry);
        }
        
        for (source, entries) in by_source {
            output.push(format!("\n--- {} ---", source));
            for entry in entries.iter().take(5) {
                output.push(format!("[{}] {}", entry.key, entry.content));
            }
        }
        
        output.join("\n")
    }

    /// Stop the hub
    pub async fn shutdown(&self) {
        *self.is_running.write().await = false;
        
        // Final sync
        if self.config.enable_collective_mind {
            let _ = self.sync_collective().await;
        }
        
        info!("[UNIFIED-MEMORY] Hub shutdown complete");
    }
}

// ============================================================================
// STATS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMemoryStats {
    pub total_entries: usize,
    pub critical_entries: usize,
    pub project_entries: usize,
    pub active_agents: usize,
    pub total_improvements: usize,
    pub collective_peers: usize,
    pub last_self_improve: Option<DateTime<Utc>>,
    pub is_running: bool,
}

// ============================================================================
// MEMORY TRAIT IMPLEMENTATION
// ============================================================================

#[async_trait]
impl Memory for UnifiedAGIMemoryHub {
    fn name(&self) -> &str {
        "unified-agi-memory"
    }

    async fn store(&self, key: &str, content: &str, category: MemoryCategory) -> anyhow::Result<()> {
        self.store(key, content, category, UnifiedMemorySource::Local).await?;
        Ok(())
    }

    async fn recall(&self, query: &str, limit: usize) -> anyhow::Result<Vec<MemoryEntry>> {
        let entries = self.recall(query, Some(limit * 50)).await?;
        Ok(entries.into_iter().take(limit).map(|e| e.to_memory_entry()).collect())
    }

    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        let entries = self.recall(key, 1).await?;
        Ok(entries.into_iter().next().map(|e| e.to_memory_entry()))
    }

    async fn list(&self, category: Option<&MemoryCategory>) -> anyhow::Result<Vec<MemoryEntry>> {
        let store = self.unified_store.read().await;
        let entries: Vec<MemoryEntry> = store
            .values()
            .filter(|e| {
                match category {
                    Some(c) => &e.category == c,
                    None => true,
                }
            })
            .take(50)
            .map(|e| e.to_memory_entry())
            .collect();
        Ok(entries)
    }

    async fn forget(&self, key: &str) -> anyhow::Result<bool> {
        self.unified_store.write().await.remove(key);
        Ok(true)
    }

    async fn count(&self) -> anyhow::Result<usize> {
        Ok(self.unified_store.read().await.len())
    }

    async fn health_check(&self) -> bool {
        *self.is_running.read().await
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_unified_memory_hub() {
        let tmp = TempDir::new().unwrap();
        
        let config = UnifiedAGIMemoryConfig {
            workspace_dir: tmp.path().to_path_buf(),
            enable_lucid: false,
            enable_intelligent: false,
            enable_project_context: false,
            enable_self_improvement: true,
            enable_collective_mind: false,
            enable_a2a_hub: false,
            enable_openclaw: false,
            enable_kowalski_federation: false,
            ..UnifiedAGIMemoryConfig::default()
        };
        
        let hub = UnifiedAGIMemoryHub::new(config);
        hub.initialize().await.unwrap();
        
        // Store
        let id = hub.store("test_key", "test content", MemoryCategory::Core, UnifiedMemorySource::Local).await.unwrap();
        assert!(!id.is_empty());
        
        // Recall
        let entries = hub.recall("test", None).await.unwrap();
        assert!(!entries.is_empty());
        
        // Stats
        let stats = hub.get_stats().await;
        assert!(stats.total_entries > 0);
    }

    #[test]
    fn test_importance_calculation() {
        let tmp = TempDir::new().unwrap();
        let config = UnifiedAGIMemoryConfig::default();
        let hub = UnifiedAGIMemoryHub::new(config);
        
        let important = hub.calculate_importance("This is an important remember bug fix", &MemoryCategory::Core);
        assert!(important > 0.7);
        
        let trivial = hub.calculate_importance("hello thanks okay", &MemoryCategory::Conversation);
        assert!(trivial < 0.5);
    }
}

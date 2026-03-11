// ☸️ PROJECT CONTEXT MEMORY SYSTEM
// Multi-level context management with connection awareness for subagent federation
// Enables agents to always refer to project design, connections, and relationships

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::traits::{Memory, MemoryCategory, MemoryEntry};

// ============================================================================
// CONTEXT LEVELS
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextLevel {
    /// Global system context
    System,
    /// Project-level context (design, architecture, goals)
    Project,
    /// Task-level context (current task)
    Task,
    /// Session-level context (current conversation)
    Session,
    /// Agent-level context (subagent state)
    Agent,
    /// Connection context (relationships)
    Connection,
}

impl ContextLevel {
    pub fn priority(&self) -> u8 {
        match self {
            ContextLevel::System => 0,
            ContextLevel::Project => 1,
            ContextLevel::Task => 2,
            ContextLevel::Session => 3,
            ContextLevel::Agent => 4,
            ContextLevel::Connection => 5,
        }
    }
}

impl std::fmt::Display for ContextLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextLevel::System => write!(f, "system"),
            ContextLevel::Project => write!(f, "project"),
            ContextLevel::Task => write!(f, "task"),
            ContextLevel::Session => write!(f, "session"),
            ContextLevel::Agent => write!(f, "agent"),
            ContextLevel::Connection => write!(f, "connection"),
        }
    }
}

// ============================================================================
// PROJECT CONTEXT
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub description: String,
    pub design_patterns: Vec<String>,
    pub dependencies: Vec<String>,
    pub agents_working: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl ProjectContext {
    pub fn new(name: &str, path: PathBuf) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            path,
            description: String::new(),
            design_patterns: Vec::new(),
            dependencies: Vec::new(),
            agents_working: Vec::new(),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

// ============================================================================
// CONTEXT ENTRY
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub id: String,
    pub level: ContextLevel,
    pub project_id: String,
    pub key: String,
    pub content: String,
    pub connections: Vec<ConnectionRef>,
    pub importance: f64,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub accessed_count: u32,
}

impl ContextEntry {
    pub fn new(level: ContextLevel, project_id: &str, key: &str, content: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            level,
            project_id: project_id.to_string(),
            key: key.to_string(),
            content: content.to_string(),
            connections: Vec::new(),
            importance: 0.5,
            tags: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            accessed_count: 0,
        }
    }

    pub fn with_connections(mut self, connections: Vec<ConnectionRef>) -> Self {
        self.connections = connections;
        self
    }

    pub fn with_importance(mut self, importance: f64) -> Self {
        self.importance = importance;
        self
    }

    pub fn add_connection(&mut self, connection: ConnectionRef) {
        self.connections.push(connection);
    }
}

// ============================================================================
// CONNECTION GRAPH
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionRef {
    pub target_type: ContextLevel,
    pub target_id: String,
    pub relationship: ConnectionType,
    pub strength: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionType {
    DependsOn,
    RelatedTo,
    PartOf,
    Implements,
    Extends,
    Calls,
    SharesDataWith,
    DerivedFrom,
}

impl ConnectionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionType::DependsOn => "depends_on",
            ConnectionType::RelatedTo => "related_to",
            ConnectionType::PartOf => "part_of",
            ConnectionType::Implements => "implements",
            ConnectionType::Extends => "extends",
            ConnectionType::Calls => "calls",
            ConnectionType::SharesDataWith => "shares_data_with",
            ConnectionType::DerivedFrom => "derived_from",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectionGraph {
    nodes: HashMap<String, ContextEntry>,
    edges: HashMap<String, Vec<ConnectionRef>>,
}

impl ConnectionGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, entry: ContextEntry) {
        self.nodes.insert(entry.id.clone(), entry);
    }

    pub fn add_connection(&mut self, from_id: &str, to: ConnectionRef) {
        self.edges
            .entry(from_id.to_string())
            .or_insert_with(Vec::new)
            .push(to);
    }

    pub fn get_connected(&self, id: &str) -> Vec<(&ContextEntry, &ConnectionRef)> {
        let mut results = Vec::new();
        
        if let Some(connections) = self.edges.get(id) {
            for conn in connections {
                if let Some(node) = self.nodes.get(&conn.target_id) {
                    results.push((node, conn));
                }
            }
        }
        
        results
    }

    pub fn find_related(&self, project_id: &str, keyword: &str) -> Vec<ContextEntry> {
        let keyword_lower = keyword.to_lowercase();
        
        self.nodes
            .values()
            .filter(|e| {
                e.project_id == project_id && (
                    e.key.to_lowercase().contains(&keyword_lower) ||
                    e.content.to_lowercase().contains(&keyword_lower) ||
                    e.tags.iter().any(|t| t.to_lowercase().contains(&keyword_lower))
                )
            })
            .cloned()
            .collect()
    }
}

// ============================================================================
// CONTEXT SWITCHER
// ============================================================================

pub struct ContextSwitcher {
    current_project: Arc<RwLock<Option<ProjectContext>>>,
    projects: Arc<RwLock<HashMap<String, ProjectContext>>>,
    context_store: Arc<RwLock<HashMap<String, Vec<ContextEntry>>>>,
    connection_graph: Arc<RwLock<ConnectionGraph>>,
    workspace_dir: PathBuf,
}

impl ContextSwitcher {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self {
            current_project: Arc::new(RwLock::new(None)),
            projects: Arc::new(RwLock::new(HashMap::new())),
            context_store: Arc::new(RwLock::new(HashMap::new())),
            connection_graph: Arc::new(RwLock::new(ConnectionGraph::new())),
            workspace_dir,
        }
    }

    /// Create a new project context
    pub async fn create_project(&self, name: &str, path: PathBuf) -> Result<ProjectContext> {
        let path_clone = path.clone();
        let project = ProjectContext::new(name, path);
        
        self.projects.write().await.insert(project.id.clone(), project.clone());
        
        // Create initial project-level context
        let entry = ContextEntry::new(
            ContextLevel::Project,
            &project.id,
            "project_created",
            &format!("Project '{}' created at {:?}", name, path_clone),
        ).with_importance(1.0);
        
        self.store_entry(entry).await?;
        
        info!("[CONTEXT] Created project: {} ({})", name, project.id);
        
        Ok(project)
    }

    /// Switch to a project context
    pub async fn switch_to_project(&self, project_id: &str) -> Result<ProjectContext> {
        let projects = self.projects.read().await;
        
        if let Some(project) = projects.get(project_id) {
            let mut current = self.current_project.write().await;
            *current = Some(project.clone());
            
            // Update last accessed
            drop(current);
            self.update_project_access(project_id).await;
            
            info!("[CONTEXT] Switched to project: {}", project.name);
            Ok(project.clone())
        } else {
            anyhow::bail!("Project not found: {}", project_id)
        }
    }

    /// Get current project context
    pub async fn get_current_project(&self) -> Option<ProjectContext> {
        self.current_project.read().await.clone()
    }

    /// Update project access time
    async fn update_project_access(&self, project_id: &str) {
        if let Some(project) = self.projects.write().await.get_mut(project_id) {
            project.last_accessed = Utc::now();
        }
    }

    /// Store a context entry
    pub async fn store_entry(&self, entry: ContextEntry) -> Result<()> {
        let project_id = entry.project_id.clone();
        
        // Add to context store
        self.context_store
            .write()
            .await
            .entry(project_id.clone())
            .or_insert_with(Vec::new)
            .push(entry.clone());
        
        // Add to connection graph
        self.connection_graph.write().await.add_node(entry);
        
        Ok(())
    }

    /// Get context entries for current project at a specific level
    pub async fn get_context(&self, level: ContextLevel, limit: usize) -> Vec<ContextEntry> {
        let current = self.current_project.read().await.clone();
        
        if let Some(project) = current {
            let store = self.context_store.read().await;
            
            store
                .get(&project.id)
                .map(|entries| {
                    entries
                        .iter()
                        .filter(|e| e.level == level)
                        .take(limit)
                        .cloned()
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    /// Get all context for current project (for agent awareness)
    pub async fn get_all_context(&self, max_entries: usize) -> Vec<ContextEntry> {
        let current = self.current_project.read().await.clone();
        
        if let Some(project) = current {
            let store = self.context_store.read().await;
            
            store
                .get(&project.id)
                .map(|entries| {
                    let mut sorted = entries.clone();
                    // Sort by importance and access count
                    sorted.sort_by(|a, b| {
                        let score_a = a.importance * (a.accessed_count as f64 + 1.0);
                        let score_b = b.importance * (b.accessed_count as f64 + 1.0);
                        score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
                    });
                    sorted.into_iter().take(max_entries).collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    /// Add a connection between context entries
    pub async fn add_connection(
        &self,
        from_id: &str,
        target_type: ContextLevel,
        target_id: &str,
        relationship: ConnectionType,
        strength: f64,
    ) -> Result<()> {
        let connection = ConnectionRef {
            target_type,
            target_id: target_id.to_string(),
            relationship,
            strength,
        };
        
        self.connection_graph.write().await.add_connection(from_id, connection);
        
        Ok(())
    }

    /// Find related context entries
    pub async fn find_related(&self, keyword: &str) -> Vec<ContextEntry> {
        let current = self.current_project.read().await.clone();
        
        if let Some(project) = current {
            self.connection_graph
                .read()
                .await
                .find_related(&project.id, keyword)
        } else {
            Vec::new()
        }
    }

    /// Get connected entries for an entry
    pub async fn get_connected(&self, entry_id: &str) -> Vec<(ContextEntry, ConnectionRef)> {
        self.connection_graph
            .read()
            .await
            .get_connected(entry_id)
            .into_iter()
            .map(|(e, c)| (e.clone(), c.clone()))
            .collect()
    }

    /// List all projects
    pub async fn list_projects(&self) -> Vec<ProjectContext> {
        self.projects.read().await.values().cloned().collect()
    }

    /// Get project by ID
    pub async fn get_project(&self, id: &str) -> Option<ProjectContext> {
        self.projects.read().await.get(id).cloned()
    }

    /// Store task context
    pub async fn store_task(
        &self,
        task_id: &str,
        task_name: &str,
        content: &str,
        connections: Vec<ConnectionRef>,
    ) -> Result<()> {
        let current = self.current_project.read().await.clone();
        
        if let Some(project) = current {
            let mut entry = ContextEntry::new(
                ContextLevel::Task,
                &project.id,
                task_id,
                content,
            )
            .with_connections(connections)
            .with_importance(0.8);
            
            entry.tags.push(task_name.to_string());
            
            self.store_entry(entry).await?;
            
            info!("[CONTEXT] Stored task context: {} ({})", task_id, task_name);
        }
        
        Ok(())
    }

    /// Store session context (for conversations)
    pub async fn store_session(&self, key: &str, content: &str) -> Result<()> {
        let current = self.current_project.read().await.clone();
        
        if let Some(project) = current {
            let entry = ContextEntry::new(
                ContextLevel::Session,
                &project.id,
                key,
                content,
            ).with_importance(0.6);
            
            self.store_entry(entry).await?;
        }
        
        Ok(())
    }

    /// Store design pattern (project-level)
    pub async fn store_design(&self, pattern: &str, content: &str) -> Result<()> {
        let current = self.current_project.read().await.clone();
        
        if let Some(project) = current {
            let mut entry = ContextEntry::new(
                ContextLevel::Project,
                &project.id,
                pattern,
                content,
            ).with_importance(0.9);
            
            entry.tags.push("design".to_string());
            entry.tags.push("architecture".to_string());
            
            self.store_entry(entry).await?;
            
            info!("[CONTEXT] Stored design: {}", pattern);
        }
        
        Ok(())
    }

    /// Get context summary for agents (for awareness)
    pub async fn get_awareness_summary(&self) -> String {
        let current = self.current_project.read().await.clone();
        
        if let Some(project) = current {
            let store = self.context_store.read().await;
            let entries = store.get(&project.id).cloned().unwrap_or_default();
            
            let design_count = entries.iter().filter(|e| e.level == ContextLevel::Project).count();
            let task_count = entries.iter().filter(|e| e.level == ContextLevel::Task).count();
            let session_count = entries.iter().filter(|e| e.level == ContextLevel::Session).count();
            
            format!(
                "[AWARE] Project: {} | Design: {} | Tasks: {} | Session: {} | Total: {}",
                project.name,
                design_count,
                task_count,
                session_count,
                entries.len()
            )
        } else {
            "[AWARE] No project active".to_string()
        }
    }
}

// ============================================================================
// FEDERATION INTEGRATION
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationContext {
    pub peer_id: String,
    pub peer_name: String,
    pub shared_projects: Vec<String>,
    pub shared_tasks: Vec<String>,
    pub last_sync: DateTime<Utc>,
}

pub struct FederationAwareContext {
    local_context: ContextSwitcher,
    federation_peers: Arc<RwLock<HashMap<String, FederationContext>>>,
}

impl FederationAwareContext {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self {
            local_context: ContextSwitcher::new(workspace_dir),
            federation_peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a federation peer
    pub async fn register_peer(&self, peer_id: &str, peer_name: &str) {
        let context = FederationContext {
            peer_id: peer_id.to_string(),
            peer_name: peer_name.to_string(),
            shared_projects: Vec::new(),
            shared_tasks: Vec::new(),
            last_sync: Utc::now(),
        };
        
        self.federation_peers
            .write()
            .await
            .insert(peer_id.to_string(), context);
        
        info!("[FEDERATION] Registered peer: {} ({})", peer_name, peer_id);
    }

    /// Sync context with federation peers
    pub async fn sync_with_peers(&self) -> Result<Vec<String>> {
        let peers = self.federation_peers.read().await;
        let mut all_shared = Vec::new();
        
        for (peer_id, context) in peers.iter() {
            debug!("[FEDERATION] Syncing with peer: {} ({})", context.peer_name, peer_id);
            
            // In a real implementation, this would:
            // 1. Send local context summary to peer
            // 2. Receive peer's context updates
            // 3. Merge and deduplicate
            
            // For now, just collect from peer's shared tasks
            for task in &context.shared_tasks {
                all_shared.push(task.clone());
            }
            
            info!("[FEDERATION] Synced with {} ({} tasks)", 
                context.peer_name, 
                context.shared_tasks.len()
            );
        }
        
        Ok(all_shared)
    }

    /// Get context from a specific peer
    pub async fn get_peer_context(&self, peer_id: &str) -> Option<FederationContext> {
        self.federation_peers.read().await.get(peer_id).cloned()
    }

    /// List all federation peers
    pub async fn list_peers(&self) -> Vec<FederationContext> {
        self.federation_peers.read().await.values().cloned().collect()
    }

    /// Delegate context to a peer
    pub async fn delegate_to_peer(&self, peer_id: &str, entry: ContextEntry) -> Result<()> {
        let mut peers = self.federation_peers.write().await;
        
        if let Some(context) = peers.get_mut(peer_id) {
            // Add to peer's shared items
            match entry.level {
                ContextLevel::Project => context.shared_projects.push(entry.id),
                ContextLevel::Task => context.shared_tasks.push(entry.id),
                _ => {}
            }
            
            context.last_sync = Utc::now();
            
            info!("[FEDERATION] Delegated {} to peer {}", entry.key, peer_id);
        }
        
        Ok(())
    }

    /// Get cross-project connections
    pub async fn get_cross_project_connections(&self) -> Vec<(String, String, ConnectionType)> {
        let peers = self.federation_peers.read().await;
        let mut connections = Vec::new();
        
        for (peer_id, context) in peers.iter() {
            for project_id in &context.shared_projects {
                connections.push((
                    project_id.clone(),
                    peer_id.clone(),
                    ConnectionType::SharesDataWith,
                ));
            }
        }
        
        connections
    }

    // Delegate to local context switcher
    pub fn local(&self) -> &ContextSwitcher {
        &self.local_context
    }
}

// ============================================================================
// AGENT AWARENESS ENGINE
// ============================================================================

pub struct AgentAwarenessEngine {
    context: FederationAwareContext,
    active_agents: Arc<RwLock<HashMap<String, AgentState>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub agent_id: String,
    pub agent_type: String,
    pub current_task: Option<String>,
    pub context_entries: Vec<String>,
    pub awareness_level: AwarenessLevel,
    pub last_update: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AwarenessLevel {
    #[default]
    Minimal,    // Only current task
    Moderate,   // Current task + related design
    Full,      // All project context + connections
}

impl AgentAwarenessEngine {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self {
            context: FederationAwareContext::new(workspace_dir),
            active_agents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get local context switcher
    pub fn local(&self) -> &ContextSwitcher {
        &self.context.local()
    }

    /// Get federation context
    pub fn federation(&self) -> &FederationAwareContext {
        &self.context
    }

    /// Register an agent
    pub async fn register_agent(&self, agent_id: &str, agent_type: &str) {
        let state = AgentState {
            agent_id: agent_id.to_string(),
            agent_type: agent_type.to_string(),
            current_task: None,
            context_entries: Vec::new(),
            awareness_level: AwarenessLevel::Moderate,
            last_update: Utc::now(),
        };
        
        self.active_agents
            .write()
            .await
            .insert(agent_id.to_string(), state);
        
        info!("[AWARE] Agent registered: {} ({})", agent_id, agent_type);
    }

    /// Update agent's current task
    pub async fn update_agent_task(&self, agent_id: &str, task_id: Option<String>) {
        if let Some(state) = self.active_agents.write().await.get_mut(agent_id) {
            state.current_task = task_id.clone();
            state.last_update = Utc::now();
            
            // If task changed, get related context
            if let Some(ref task) = task_id {
                let related = self.context.local().find_related(task).await;
                state.context_entries = related.into_iter().take(5).map(|e| e.id).collect();
            }
        }
    }

    /// Get awareness context for an agent
    pub async fn get_agent_awareness(&self, agent_id: &str) -> AwarenessContext {
        let agents = self.active_agents.read().await;
        
        if let Some(state) = agents.get(agent_id) {
            let level = state.awareness_level;
            let max_entries = match level {
                AwarenessLevel::Minimal => 3,
                AwarenessLevel::Moderate => 10,
                AwarenessLevel::Full => 50,
            };
            
            let context = self.context.local().get_all_context(max_entries).await;
            let summary = self.context.local().get_awareness_summary().await;
            
            // Get related entries for current task
            let related = if let Some(ref task_id) = state.current_task {
                self.context.local().find_related(task_id).await
            } else {
                Vec::new()
            };
            
            AwarenessContext {
                agent_id: agent_id.to_string(),
                level,
                current_project: self.context.local().get_current_project().await,
                context_entries: context,
                related_entries: related,
                federation_peers: self.context.list_peers().await,
                summary,
            }
        } else {
            AwarenessContext::default()
        }
    }

    /// Set agent awareness level
    pub async fn set_awareness_level(&self, agent_id: &str, level: AwarenessLevel) {
        if let Some(state) = self.active_agents.write().await.get_mut(agent_id) {
            state.awareness_level = level;
            state.last_update = Utc::now();
            
            info!("[AWARE] Agent {} awareness set to {:?}", agent_id, level);
        }
    }

    /// Get all active agents
    pub async fn list_agents(&self) -> Vec<AgentState> {
        self.active_agents.read().await.values().cloned().collect()
    }

    /// Sync with federation
    pub async fn sync_federation(&self) -> Result<()> {
        self.context.sync_with_peers().await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AwarenessContext {
    pub agent_id: String,
    pub level: AwarenessLevel,
    pub current_project: Option<ProjectContext>,
    pub context_entries: Vec<ContextEntry>,
    pub related_entries: Vec<ContextEntry>,
    pub federation_peers: Vec<FederationContext>,
    pub summary: String,
}

impl AwarenessContext {
    /// Format context for LLM prompt injection
    pub fn format_for_prompt(&self, max_tokens: usize) -> String {
        let mut output = Vec::new();
        let mut tokens = 0;
        
        output.push(format!("=== PROJECT CONTEXT ==="));
        output.push(self.summary.clone());
        
        if let Some(ref project) = self.current_project {
            tokens += project.name.len() + project.description.len();
            output.push(format!("\nProject: {}", project.name));
            output.push(format!("Description: {}", project.description));
            
            if !project.design_patterns.is_empty() {
                output.push(format!("Design Patterns: {}", project.design_patterns.join(", ")));
            }
        }
        
        if !self.context_entries.is_empty() {
            output.push("\n=== DESIGN & ARCHITECTURE ===".to_string());
            
            for entry in &self.context_entries {
                if tokens > max_tokens {
                    break;
                }
                
                output.push(format!("\n[{}] {}", entry.level, entry.key));
                output.push(entry.content.clone());
                
                tokens += entry.content.len();
            }
        }
        
        if !self.related_entries.is_empty() {
            output.push("\n=== RELATED CONTEXT ===".to_string());
            
            for entry in self.related_entries.iter().take(3) {
                if tokens > max_tokens {
                    break;
                }
                
                output.push(format!("- {}: {}", entry.key, entry.content));
                tokens += entry.content.len();
            }
        }
        
        if !self.federation_peers.is_empty() {
            output.push("\n=== FEDERATION PEERS ===".to_string());
            
            for peer in &self.federation_peers {
                output.push(format!("- {} ({} tasks shared)", 
                    peer.peer_name, 
                    peer.shared_tasks.len()
                ));
            }
        }
        
        output.join("\n")
    }
}

// ============================================================================
// MEMORY TRAIT IMPLEMENTATION
// ============================================================================

#[async_trait]
impl Memory for AgentAwarenessEngine {
    fn name(&self) -> &str {
        "project-context"
    }

    async fn store(&self, key: &str, content: &str, category: MemoryCategory) -> anyhow::Result<()> {
        let level = match category {
            MemoryCategory::Core => ContextLevel::Project,
            MemoryCategory::Daily => ContextLevel::Session,
            MemoryCategory::Conversation => ContextLevel::Session,
            MemoryCategory::Custom(_) => ContextLevel::Task,
        };
        
        let current = self.context.local().get_current_project().await;
        
        if let Some(project) = current {
            let entry = ContextEntry::new(level, &project.id, key, content)
                .with_importance(0.7);
            
            self.context.local().store_entry(entry).await?;
        }
        
        Ok(())
    }

    async fn recall(&self, query: &str, limit: usize) -> anyhow::Result<Vec<MemoryEntry>> {
        let entries = self.context.local().find_related(query).await;
        
        Ok(entries
            .into_iter()
            .take(limit)
            .map(|e| MemoryEntry {
                id: e.id,
                key: e.key,
                content: e.content,
                category: match e.level {
                    ContextLevel::Project => MemoryCategory::Core,
                    ContextLevel::Session => MemoryCategory::Conversation,
                    _ => MemoryCategory::Custom(e.level.to_string()),
                },
                timestamp: e.created_at.to_rfc3339(),
                session_id: None,
                score: Some(e.importance),
            })
            .collect())
    }

    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        let entries = self.recall(key, 1).await?;
        Ok(entries.into_iter().next())
    }

    async fn list(&self, category: Option<&MemoryCategory>) -> anyhow::Result<Vec<MemoryEntry>> {
        let level = category.map(|c| match c {
            MemoryCategory::Core => ContextLevel::Project,
            MemoryCategory::Daily => ContextLevel::Session,
            MemoryCategory::Conversation => ContextLevel::Session,
            MemoryCategory::Custom(_) => ContextLevel::Task,
        });
        
        let all_entries = self.context.local().get_all_context(100).await;
        
        let filtered: Vec<MemoryEntry> = if let Some(l) = level {
            all_entries
                .into_iter()
                .filter(|e| e.level == l)
                .map(|e| MemoryEntry {
                    id: e.id,
                    key: e.key,
                    content: e.content,
                    category: MemoryCategory::Core,
                    timestamp: e.created_at.to_rfc3339(),
                    session_id: None,
                    score: Some(e.importance),
                })
                .collect()
        } else {
            all_entries
                .into_iter()
                .map(|e| MemoryEntry {
                    id: e.id,
                    key: e.key,
                    content: e.content,
                    category: MemoryCategory::Core,
                    timestamp: e.created_at.to_rfc3339(),
                    session_id: None,
                    score: Some(e.importance),
                })
                .collect()
        };
        
        Ok(filtered)
    }

    async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
        Ok(false)
    }

    async fn count(&self) -> anyhow::Result<usize> {
        Ok(self.context.local().get_all_context(1000).await.len())
    }

    async fn health_check(&self) -> bool {
        true
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
    async fn test_project_context() {
        let tmp = TempDir::new().unwrap();
        let switcher = ContextSwitcher::new(tmp.path().to_path_buf());
        
        // Create project
        let project = switcher
            .create_project("test-project", tmp.path().join("test"))
            .await
            .unwrap();
        
        // Switch to project
        switcher.switch_to_project(&project.id).await.unwrap();
        
        // Store design
        switcher
            .store_design("mvc-pattern", "Model-View-Controller architecture")
            .await
            .unwrap();
        
        // Get context
        let context = switcher.get_context(ContextLevel::Project, 10).await;
        assert!(!context.is_empty());
        
        // Get awareness summary
        let summary = switcher.get_awareness_summary().await;
        assert!(summary.contains("test-project"));
    }

    #[tokio::test]
    async fn test_connection_graph() {
        let mut graph = ConnectionGraph::new();
        
        let entry1 = ContextEntry::new(
            ContextLevel::Project,
            "proj1",
            "design1",
            "Design pattern 1",
        );
        
        let entry2 = ContextEntry::new(
            ContextLevel::Task,
            "proj1", 
            "task1",
            "Task implementation",
        );
        
        let entry1_id = entry1.id.clone();
        let entry2_id = entry2.id.clone();
        
        graph.add_node(entry1);
        graph.add_node(entry2);
        
        graph.add_connection(
            &entry2_id,
            ConnectionRef {
                target_type: ContextLevel::Project,
                target_id: entry1_id,
                relationship: ConnectionType::PartOf,
                strength: 0.9,
            },
        );
        
        let related = graph.find_related("proj1", "design");
        assert!(!related.is_empty());
    }

    #[tokio::test]
    async fn test_agent_awareness() {
        let tmp = TempDir::new().unwrap();
        let engine = AgentAwarenessEngine::new(tmp.path().to_path_buf());
        
        // Register agent
        engine.register_agent("agent-1", "code").await;
        
        // Set awareness level
        engine.set_awareness_level("agent-1", AwarenessLevel::Full).await;
        
        // Get awareness
        let awareness = engine.get_agent_awareness("agent-1").await;
        
        assert_eq!(awareness.agent_id, "agent-1");
        assert_eq!(awareness.level, AwarenessLevel::Full);
    }
}

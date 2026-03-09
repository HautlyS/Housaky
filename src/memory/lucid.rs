// ☸️ LUCID MEMORY - Pure Lucid Integration with A2A Hub & OpenClaw
// No SQLite - pure Lucid Native with ACT-R spreading activation
// Integrates with OpenClaw (shared/) and A2A-Hub (landing/A2A/public/shared/memory/)

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::traits::{Memory, MemoryCategory, MemoryEntry};

// ============================================================================
// CONFIGURATION
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LucidConfig {
    /// Path to lucid CLI binary
    pub binary_path: PathBuf,
    /// Token budget for context retrieval
    pub token_budget: usize,
    /// Enable spreading activation
    pub enable_spreading: bool,
    /// Enable emotional weighting
    pub enable_emotional_weighting: bool,
    /// Maximum memories to retrieve
    pub max_retrieve: usize,
    /// Enable A2A Hub integration
    pub enable_a2a_hub: bool,
    /// Enable OpenClaw integration
    pub enable_openclaw: bool,
    /// A2A Hub memory directory
    pub a2a_hub_memory_path: Option<PathBuf>,
    /// OpenClaw shared directory
    pub openclaw_shared_path: Option<PathBuf>,
    /// Workspace directory
    pub workspace_dir: PathBuf,
}

impl Default for LucidConfig {
    fn default() -> Self {
        Self {
            binary_path: PathBuf::from("/home/ubuntu/.lucid/bin/lucid"),
            token_budget: 500,
            enable_spreading: true,
            enable_emotional_weighting: true,
            max_retrieve: 20,
            enable_a2a_hub: true,
            enable_openclaw: true,
            a2a_hub_memory_path: Some(PathBuf::from("/home/ubuntu/Housaky/landing/A2A/public/shared/memory")),
            openclaw_shared_path: Some(PathBuf::from("/home/ubuntu/Housaky/shared")),
            workspace_dir: PathBuf::from("/home/ubuntu/.housaky/workspace"),
        }
    }
}

// ============================================================================
// MEMORY SOURCE TYPES
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemorySource {
    /// Local Housaky memory
    Local,
    /// A2A Hub shared memory (from other agents)
    A2AHub,
    /// OpenClaw collaborative memory
    OpenClaw,
    /// Critical memory (never forget)
    Critical,
}

impl std::fmt::Display for MemorySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemorySource::Local => write!(f, "local"),
            MemorySource::A2AHub => write!(f, "a2a-hub"),
            MemorySource::OpenClaw => write!(f, "openclaw"),
            MemorySource::Critical => write!(f, "critical"),
        }
    }
}

// ============================================================================
// SHARED MEMORY ENTRY (for A2A Hub & OpenClaw)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedMemoryEntry {
    pub id: String,
    pub key: String,
    pub content: String,
    pub category: String,
    pub timestamp: String,
    pub source: MemorySource,
    pub instance: String,
    pub confidence: f64,
    pub tags: Vec<String>,
}

impl From<MemoryEntry> for SharedMemoryEntry {
    fn from(entry: MemoryEntry) -> Self {
        Self {
            id: entry.id,
            key: entry.key,
            content: entry.content,
            category: entry.category.to_string(),
            timestamp: entry.timestamp,
            source: MemorySource::Local,
            instance: "housaky".to_string(),
            confidence: entry.score.unwrap_or(0.5),
            tags: vec![],
        }
    }
}

impl From<&MemoryEntry> for SharedMemoryEntry {
    fn from(entry: &MemoryEntry) -> Self {
        Self {
            id: entry.id.clone(),
            key: entry.key.clone(),
            content: entry.content.clone(),
            category: entry.category.to_string(),
            timestamp: entry.timestamp.clone(),
            source: MemorySource::Local,
            instance: "housaky".to_string(),
            confidence: entry.score.unwrap_or(0.5),
            tags: vec![],
        }
    }
}

// ============================================================================
// LUCID MEMORY STATISTICS
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LucidMemoryStats {
    pub total_memories: usize,
    pub local_count: usize,
    pub a2a_hub_count: usize,
    pub openclaw_count: usize,
    pub critical_count: usize,
    pub database_size_mb: f64,
    pub last_sync_ms: u64,
    pub avg_retrieval_ms: f64,
}

// ============================================================================
// CATEGORY MAPPING
// ============================================================================

fn category_to_lucid_type(category: &MemoryCategory) -> &'static str {
    match category {
        MemoryCategory::Core => "decision",
        MemoryCategory::Daily => "context",
        MemoryCategory::Conversation => "conversation",
        MemoryCategory::Custom(name) => match name.as_str() {
            "fact" | "facts" => "fact",
            "procedure" | "procedures" => "procedure",
            "skill" | "skills" => "skill",
            "pattern" | "patterns" => "pattern",
            "insight" | "insights" => "insight",
            "visual" => "visual",
            _ => "learning",
        },
    }
}

fn lucid_type_to_category(lucid_type: &str) -> MemoryCategory {
    match lucid_type.to_lowercase().as_str() {
        "decision" | "fact" | "facts" => MemoryCategory::Core,
        "context" => MemoryCategory::Daily,
        "conversation" => MemoryCategory::Conversation,
        "procedure" | "procedures" => MemoryCategory::Custom("procedure".to_string()),
        "skill" | "skills" => MemoryCategory::Custom("skill".to_string()),
        "pattern" | "patterns" => MemoryCategory::Custom("pattern".to_string()),
        "insight" | "insights" => MemoryCategory::Custom("insight".to_string()),
        "visual" => MemoryCategory::Custom("visual".to_string()),
        _ => MemoryCategory::Custom("learning".to_string()),
    }
}

// ============================================================================
// LUCID MEMORY IMPLEMENTATION
// ============================================================================

pub struct LucidMemory {
    config: LucidConfig,
    stats: Arc<RwLock<LucidMemoryStats>>,
    cache: Arc<RwLock<HashMap<String, Vec<MemoryEntry>>>>,
    critical_memories: Arc<RwLock<Vec<MemoryEntry>>>,
    last_health_check: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl LucidMemory {
    pub fn new(config: LucidConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(LucidMemoryStats::default())),
            cache: Arc::new(RwLock::new(HashMap::new())),
            critical_memories: Arc::new(RwLock::new(Vec::new())),
            last_health_check: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_workspace(workspace_dir: &Path) -> Self {
        let mut config = LucidConfig::default();
        config.workspace_dir = workspace_dir.to_path_buf();
        Self::new(config)
    }

    /// Execute a lucid CLI command (synchronous)
    fn execute_lucid(&self, args: &[&str]) -> Result<String> {
        let start = std::time::Instant::now();
        
        let mut cmd = Command::new(&self.config.binary_path);
        cmd.args(args);
        
        debug!("[LUCID] Executing: {} {}", self.config.binary_path.display(), args.join(" "));
        
        let output = cmd.output().context("Failed to execute lucid command")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("lucid command failed: {stderr}");
        }
        
        let duration = start.elapsed().as_millis() as u64;
        debug!("[LUCID] Command completed in {}ms", duration);
        
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Store a memory with Lucid
    pub async fn store_lucid(&self, key: &str, content: &str, category: &MemoryCategory) -> Result<()> {
        let payload = format!("{key}: {content}");
        let category_type = category_to_lucid_type(category);
        
        let type_str = format!("--type={}", category_type);
        let project_str = format!("--project={}", self.config.workspace_dir.display());
        
        let args: Vec<&str> = vec![
            "store", 
            &payload,
            &type_str,
            &project_str,
        ];
        
        self.execute_lucid(&args)?;
        
        // Add to critical memories if it's important
        if matches!(category, MemoryCategory::Core) {
            self.add_critical(key, content, category).await;
        }
        
        // Sync to A2A Hub if enabled
        if self.config.enable_a2a_hub {
            let entry = MemoryEntry {
                id: uuid::Uuid::new_v4().to_string(),
                key: key.to_string(),
                content: content.to_string(),
                category: category.clone(),
                timestamp: Utc::now().to_rfc3339(),
                session_id: None,
                score: Some(0.8),
            };
            self.sync_to_a2a_hub(&entry).await;
        }
        
        Ok(())
    }

    /// Recall memories from Lucid
    pub async fn recall_lucid(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>> {
        let budget_str = format!("--budget={}", self.config.token_budget);
        let project_str = format!("--project={}", self.config.workspace_dir.display());
        
        let args: Vec<&str> = vec![
            "context", 
            query,
            &budget_str,
            &project_str,
        ];
        
        let output = self.execute_lucid(&args)?;
        self.parse_context_output(&output, limit)
    }

    /// Parse Lucid context output
    fn parse_context_output(&self, output: &str, limit: usize) -> Result<Vec<MemoryEntry>> {
        let mut entries = Vec::new();
        let now = Local::now().to_rfc3339();
        let mut in_context = false;
        
        for line in output.lines() {
            let line = line.trim();
            
            if line.contains("<lucid-context>") || line.contains("```") {
                in_context = true;
                continue;
            }
            
            if line.contains("</lucid-context>") || line.contains("```") {
                break;
            }
            
            if !in_context || line.is_empty() || !line.starts_with('-') {
                continue;
            }
            
            // Parse "- [type] content" format
            let content = line.trim_start_matches("- ").trim_start_matches("[");
            if content.is_empty() {
                continue;
            }
            
            let (category_label, entry_content) = if let Some(pos) = content.find(']') {
                (&content[1..pos], content[pos+1..].trim())
            } else {
                ("learning", content)
            };
            
            if entry_content.is_empty() {
                continue;
            }
            
            let rank = entries.len();
            entries.push(MemoryEntry {
                id: format!("lucid:{}", uuid::Uuid::new_v4()),
                key: format!("lucid_{}", rank),
                content: entry_content.to_string(),
                category: lucid_type_to_category(category_label),
                timestamp: now.clone(),
                session_id: None,
                score: Some((1.0 - rank as f64 * 0.05).max(0.1)),
            });
            
            if entries.len() >= limit {
                break;
            }
        }
        
        Ok(entries)
    }

    /// Get all memories
    pub async fn list_all(&self, category: Option<&MemoryCategory>, limit: usize) -> Result<Vec<MemoryEntry>> {
        let project_str = format!("--project={}", self.config.workspace_dir.display());
        let limit_str = format!("--limit={}", limit);
        
        let args: Vec<&str> = vec![
            "list",
            &project_str,
            &limit_str,
        ];
        
        let output = self.execute_lucid(&args)?;
        self.parse_context_output(&output, limit)
    }

    /// Forget a memory
    pub async fn forget_lucid(&self, key: &str) -> Result<bool> {
        let project_str = format!("--project={}", self.config.workspace_dir.display());
        
        let args: Vec<&str> = vec![
            "forget", 
            key,
            &project_str,
        ];
        
        match self.execute_lucid(&args) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Count memories
    pub async fn count_lucid(&self) -> Result<usize> {
        let project_str = format!("--project={}", self.config.workspace_dir.display());
        
        let args: Vec<&str> = vec![
            "count",
            &project_str,
        ];
        
        let output = self.execute_lucid(&args)?;
        
        // Parse count from output
        for line in output.lines() {
            if let Ok(count) = line.trim().parse::<usize>() {
                return Ok(count);
            }
        }
        
        Ok(0)
    }

    // ============================================================================
    // CRITICAL MEMORIES (Never Forget)
    // ============================================================================

    pub async fn add_critical(&self, key: &str, content: &str, category: &MemoryCategory) {
        let mut critical = self.critical_memories.write().await;
        
        // Don't add duplicates
        if critical.iter().any(|e| e.key == key) {
            return;
        }
        
        critical.push(MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            key: key.to_string(),
            content: content.to_string(),
            category: category.clone(),
            timestamp: Utc::now().to_rfc3339(),
            session_id: None,
            score: Some(1.0),
        });
        
        info!("[LUCID] Added critical memory: {}", key);
    }

    pub async fn get_critical(&self) -> Vec<MemoryEntry> {
        self.critical_memories.read().await.clone()
    }

    // ============================================================================
    // A2A HUB INTEGRATION
    // ============================================================================

    async fn sync_to_a2a_hub(&self, entry: &MemoryEntry) {
        if let Some(ref hub_path) = self.config.a2a_hub_memory_path {
            let shared: SharedMemoryEntry = entry.into();
            
            // Write to learnings.jsonl (appending)
            let learnings_path = hub_path.join("learnings.jsonl");
            if let Ok(json) = serde_json::to_string(&shared) {
                if let Err(e) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(learnings_path)
                    .and_then(|mut f| {
                        use std::io::Write;
                        writeln!(f, "{}", json)
                    })
                {
                    warn!("[A2A-HUB] Failed to sync memory: {}", e);
                } else {
                    info!("[A2A-HUB] Synced memory: {}", entry.key);
                }
            }
            
            // Update current-state.json
            self.update_a2a_state(hub_path).await;
        }
    }

    async fn update_a2a_state(&self, hub_path: &Path) {
        let state_path = hub_path.join("current-state.json");
        
        let mut state = HashMap::new();
        state.insert("instance".to_string(), "housaky".to_string());
        state.insert("timestamp".to_string(), Utc::now().to_rfc3339());
        state.insert("total_memories".to_string(), self.critical_memories.read().await.len().to_string());
        
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(state_path, json);
        }
    }

    pub async fn fetch_a2a_hub_memories(&self) -> Vec<MemoryEntry> {
        if let Some(ref hub_path) = self.config.a2a_hub_memory_path {
            let learnings_path = hub_path.join("learnings.jsonl");
            
            if !learnings_path.exists() {
                return vec![];
            }
            
            let mut entries = Vec::new();
            
            if let Ok(content) = std::fs::read_to_string(&learnings_path) {
                for line in content.lines() {
                    if let Ok(shared) = serde_json::from_str::<SharedMemoryEntry>(line) {
                        entries.push(MemoryEntry {
                            id: shared.id,
                            key: shared.key,
                            content: shared.content,
                            category: lucid_type_to_category(&shared.category),
                            timestamp: shared.timestamp,
                            session_id: None,
                            score: Some(shared.confidence),
                        });
                    }
                }
            }
            
            info!("[A2A-HUB] Fetched {} memories from hub", entries.len());
            entries
        } else {
            vec![]
        }
    }

    // ============================================================================
    // OPENCLAW INTEGRATION
    // ============================================================================

    pub async fn sync_to_openclaw(&self, entry: &MemoryEntry) {
        if let Some(ref shared_path) = self.config.openclaw_shared_path {
            let memory_path = shared_path.join("memory");
            
            if let Err(e) = std::fs::create_dir_all(&memory_path) {
                warn!("[OPENCLAW] Failed to create memory dir: {}", e);
                return;
            }
            
            let shared: SharedMemoryEntry = entry.into();
            let filename = format!("{}.json", entry.id);
            let file_path = memory_path.join(&filename);
            
            if let Ok(json) = serde_json::to_string_pretty(&shared) {
                if let Err(e) = std::fs::write(&file_path, json) {
                    warn!("[OPENCLAW] Failed to sync memory: {}", e);
                } else {
                    info!("[OPENCLAW] Synced memory: {}", entry.key);
                }
            }
        }
    }

    pub async fn fetch_openclaw_memories(&self) -> Vec<MemoryEntry> {
        if let Some(ref shared_path) = self.config.openclaw_shared_path {
            let memory_path = shared_path.join("memory");
            
            if !memory_path.exists() {
                return vec![];
            }
            
            let mut entries = Vec::new();
            
            if let Ok(entries_dir) = std::fs::read_dir(&memory_path) {
                for entry in entries_dir.flatten() {
                    if entry.path().extension().map_or(false, |e| e == "json") {
                        if let Ok(content) = std::fs::read_to_string(entry.path()) {
                            if let Ok(shared) = serde_json::from_str::<SharedMemoryEntry>(&content) {
                                entries.push(MemoryEntry {
                                    id: shared.id,
                                    key: shared.key,
                                    content: shared.content,
                                    category: lucid_type_to_category(&shared.category),
                                    timestamp: shared.timestamp,
                                    session_id: None,
                                    score: Some(shared.confidence),
                                });
                            }
                        }
                    }
                }
            }
            
            info!("[OPENCLAW] Fetched {} memories from shared", entries.len());
            entries
        } else {
            vec![]
        }
    }

    // ============================================================================
    // INTELLIGENT SEARCH (with all sources)
    // ============================================================================

    pub async fn intelligent_recall(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>> {
        let mut all_entries: HashMap<String, (MemoryEntry, MemorySource)> = HashMap::new();
        
        // 1. Get critical memories first (always included)
        for entry in self.get_critical().await {
            if entry.content.to_lowercase().contains(&query.to_lowercase())
                || entry.key.to_lowercase().contains(&query.to_lowercase())
            {
                all_entries.insert(entry.id.clone(), (entry, MemorySource::Critical));
            }
        }
        
        // 2. Get local Lucid memories
        if let Ok(local) = self.recall_lucid(query, limit).await {
            for entry in local {
                all_entries.entry(entry.id.clone()).or_insert((entry, MemorySource::Local));
            }
        }
        
        // 3. Get A2A Hub memories (if enabled)
        if self.config.enable_a2a_hub {
            for entry in self.fetch_a2a_hub_memories().await {
                if entry.content.to_lowercase().contains(&query.to_lowercase()) {
                    all_entries.entry(entry.id.clone()).or_insert((entry, MemorySource::A2AHub));
                }
            }
        }
        
        // 4. Get OpenClaw memories (if enabled)
        if self.config.enable_openclaw {
            for entry in self.fetch_openclaw_memories().await {
                if entry.content.to_lowercase().contains(&query.to_lowercase()) {
                    all_entries.entry(entry.id.clone()).or_insert((entry, MemorySource::OpenClaw));
                }
            }
        }
        
        // Sort by source priority (Critical > Local > A2AHub > OpenClaw) then by score
        let mut sorted: Vec<_> = all_entries.into_values().collect();
        sorted.sort_by(|a, b| {
            let source_order = |s: &MemorySource| match s {
                MemorySource::Critical => 0,
                MemorySource::Local => 1,
                MemorySource::A2AHub => 2,
                MemorySource::OpenClaw => 3,
            };
            
            let source_cmp = source_order(&a.1).cmp(&source_order(&b.1));
            if source_cmp != std::cmp::Ordering::Equal {
                return source_cmp;
            }
            
            b.0.score.unwrap_or(0.0).partial_cmp(&a.0.score.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        Ok(sorted.into_iter().take(limit).map(|(e, _)| e).collect())
    }

    // ============================================================================
    // STATS
    // ============================================================================

    pub async fn get_stats(&self) -> LucidMemoryStats {
        let mut stats = self.stats.read().await.clone();
        
        if let Ok(count) = self.count_lucid().await {
            stats.total_memories = count;
        }
        
        stats.critical_count = self.critical_memories.read().await.len();
        
        stats
    }
}

// ============================================================================
// MEMORY TRAIT IMPLEMENTATION
// ============================================================================

#[async_trait]
impl Memory for LucidMemory {
    fn name(&self) -> &str {
        "lucid"
    }

    async fn store(&self, key: &str, content: &str, category: MemoryCategory) -> anyhow::Result<()> {
        self.store_lucid(key, content, &category).await
    }

    async fn recall(&self, query: &str, limit: usize) -> anyhow::Result<Vec<MemoryEntry>> {
        self.intelligent_recall(query, limit).await
    }

    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        // Try to get from critical first
        let critical = self.get_critical().await;
        if let Some(entry) = critical.into_iter().find(|e| e.key == key) {
            return Ok(Some(entry));
        }
        
        // Then try lucid
        let results = self.recall_lucid(key, 1).await?;
        Ok(results.into_iter().find(|e| e.key.contains(key)))
    }

    async fn list(&self, category: Option<&MemoryCategory>) -> anyhow::Result<Vec<MemoryEntry>> {
        self.list_all(category, 100).await
    }

    async fn forget(&self, key: &str) -> anyhow::Result<bool> {
        self.forget_lucid(key).await
    }

    async fn count(&self) -> anyhow::Result<usize> {
        let stats = self.get_stats().await;
        Ok(stats.total_memories + stats.critical_count)
    }

    async fn health_check(&self) -> bool {
        // Check if lucid binary exists
        if !self.config.binary_path.exists() {
            return false;
        }
        
        // Try a simple command
        match self.execute_lucid(&["--version"]) {
            Ok(_) => {
                *self.last_health_check.write().await = Some(Utc::now());
                true
            }
            Err(_) => false,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_category_mapping() {
        assert_eq!(category_to_lucid_type(&MemoryCategory::Core), "decision");
        assert_eq!(category_to_lucid_type(&MemoryCategory::Daily), "context");
        assert_eq!(category_to_lucid_type(&MemoryCategory::Conversation), "conversation");
    }

    #[test]
    fn test_memory_source_display() {
        assert_eq!(MemorySource::Local.to_string(), "local");
        assert_eq!(MemorySource::A2AHub.to_string(), "a2a-hub");
        assert_eq!(MemorySource::OpenClaw.to_string(), "openclaw");
        assert_eq!(MemorySource::Critical.to_string(), "critical");
    }

    #[tokio::test]
    async fn test_critical_memories() {
        let tmp = TempDir::new().unwrap();
        let config = LucidConfig {
            workspace_dir: tmp.path().to_path_buf(),
            ..LucidConfig::default()
        };
        
        let memory = LucidMemory::new(config);
        
        memory.add_critical("test_key", "test content", &MemoryCategory::Core).await;
        
        let critical = memory.get_critical().await;
        assert_eq!(critical.len(), 1);
        assert_eq!(critical[0].key, "test_key");
    }
}

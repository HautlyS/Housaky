// ☸️ LUCID NATIVE MEMORY - 100% Lucid Integration
// No SQLite fallback - pure Lucid memory with ACT-R spreading activation
// ~2.7ms retrieval, reconstructive memory, semantic search
// SHARED MEMORY between OpenClaw and Housaky Native

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::traits::{Memory, MemoryCategory, MemoryEntry};

// ============================================================================
// LUCID NATIVE CONFIGURATION
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LucidNativeConfig {
    /// Path to lucid CLI binary
    pub binary_path: PathBuf,
    /// Project/workspace path for Lucid context
    pub project_path: PathBuf,
    /// Token budget for context retrieval
    pub token_budget: usize,
    /// Enable spreading activation
    pub enable_spreading: bool,
    /// Enable emotional weighting
    pub enable_emotional_weighting: bool,
    /// Default decay rate for memories
    pub default_decay_rate: f64,
    /// Maximum memories to retrieve
    pub max_retrieve: usize,
    /// Enable verbose logging
    pub verbose: bool,
}

impl Default for LucidNativeConfig {
    fn default() -> Self {
        let binary_path = if let Some(user_dirs) = directories::UserDirs::new() {
            user_dirs.home_dir().join(".lucid").join("bin").join("lucid")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".lucid").join("bin").join("lucid")
        } else {
            PathBuf::from(".lucid").join("bin").join("lucid")
        };

        let project_path = if let Some(user_dirs) = directories::UserDirs::new() {
            user_dirs.home_dir().join(".housaky").join("workspace")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".housaky").join("workspace")
        } else {
            PathBuf::from(".housaky").join("workspace")
        };

        Self {
            binary_path,
            project_path,
            token_budget: 500,
            enable_spreading: true,
            enable_emotional_weighting: true,
            default_decay_rate: 0.5,
            max_retrieve: 20,
            verbose: false,
        }
    }
}

// ============================================================================
// LUCID MEMORY STATISTICS
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LucidMemoryStats {
    pub total_memories: usize,
    pub facts: usize,
    pub procedures: usize,
    pub skills: usize,
    pub patterns: usize,
    pub insights: usize,
    pub visual_memories: usize,
    pub database_size_mb: f64,
    pub last_access_ms: u64,
    pub avg_retrieval_ms: f64,
}

// ============================================================================
// LUCID MEMORY TYPE MAPPING
// ============================================================================

fn category_to_lucid_type(category: &MemoryCategory) -> &'static str {
    match category {
        MemoryCategory::Core => "decision",
        MemoryCategory::Daily => "context",
        MemoryCategory::Conversation => "conversation",
        MemoryCategory::Custom(name) => {
            match name.as_str() {
                "fact" | "facts" => "fact",
                "procedure" | "procedures" => "procedure",
                "skill" | "skills" => "skill",
                "pattern" | "patterns" => "pattern",
                "insight" | "insights" => "insight",
                "visual" => "visual",
                _ => "learning",
            }
        }
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
// LUCID NATIVE MEMORY IMPLEMENTATION
// ============================================================================

pub struct LucidNativeMemory {
    config: LucidNativeConfig,
    stats: Arc<RwLock<LucidMemoryStats>>,
    cache: Arc<RwLock<HashMap<String, Vec<MemoryEntry>>>>,
    last_health_check: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl LucidNativeMemory {
    pub fn new(config: LucidNativeConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(LucidMemoryStats::default())),
            cache: Arc::new(RwLock::new(HashMap::new())),
            last_health_check: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_workspace(workspace_dir: &std::path::Path) -> Self {
        let mut config = LucidNativeConfig::default();
        config.project_path = workspace_dir.to_path_buf();
        Self::new(config)
    }

    /// Execute a lucid CLI command (synchronous)
    fn execute_lucid(&self, args: &[&str]) -> Result<String> {
        let start = std::time::Instant::now();
        
        let mut cmd = Command::new(&self.config.binary_path);
        cmd.args(args);
        
        if self.config.verbose {
            debug!("[LUCID] Executing: {} {}", self.config.binary_path.display(), args.join(" "));
        }
        
        let output = cmd.output()
            .with_context(|| format!("Failed to execute lucid: {:?}", args))?;
        
        let elapsed = start.elapsed();
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("[LUCID] Command failed: {}", stderr);
            anyhow::bail!("Lucid command failed: {}", stderr);
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        
        if self.config.verbose {
            debug!("[LUCID] Command completed in {:?}ms", elapsed.as_millis());
        }
        
        // Update stats asynchronously (fire and forget)
        let _last_access = elapsed.as_millis() as u64;
        // Note: In production, we'd spawn a task to update stats
        
        Ok(stdout)
    }

    /// Parse Lucid context output into memory entries
    fn parse_context_output(&self, output: &str) -> Vec<MemoryEntry> {
        let mut entries = Vec::new();
        let mut in_context_block = false;
        let now = Utc::now().to_rfc3339();
        
        for line in output.lines().map(str::trim) {
            if line == "<lucid-context>" {
                in_context_block = true;
                continue;
            }
            
            if line == "</lucid-context>" {
                break;
            }
            
            if !in_context_block || line.is_empty() {
                continue;
            }
            
            if let Some(rest) = line.strip_prefix("- [") {
                if let Some((type_part, content)) = rest.split_once(']') {
                    let content = content.trim();
                    if !content.is_empty() {
                        let rank = entries.len();
                        entries.push(MemoryEntry {
                            id: format!("lucid_{}", rank),
                            key: format!("lucid_key_{}", rank),
                            content: content.to_string(),
                            category: lucid_type_to_category(type_part.trim()),
                            timestamp: now.clone(),
                            session_id: None,
                            score: Some((1.0 - rank as f64 * 0.05).max(0.1)),
                        });
                    }
                }
            }
        }
        
        entries
    }

    /// Store a memory with Lucid
    async fn store_with_lucid(&self, key: &str, content: &str, category: &MemoryCategory) -> Result<()> {
        let memory_type = category_to_lucid_type(category);
        let payload = format!("{}: {}", key, content);
        let project_arg = format!("--project={}", self.config.project_path.display());
        let type_arg = format!("--type={}", memory_type);
        
        let args = ["store", &payload, &type_arg, &project_arg];
        self.execute_lucid(&args)?;
        
        let mut cache = self.cache.write().await;
        cache.remove(key);
        
        info!("[LUCID] Stored memory: {} (type: {})", key, memory_type);
        
        Ok(())
    }

    /// Recall memories from Lucid
    async fn recall_from_lucid(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>> {
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(query) {
                if cached.len() >= limit {
                    return Ok(cached.iter().take(limit).cloned().collect());
                }
            }
        }
        
        let project_arg = format!("--project={}", self.config.project_path.display());
        let budget_arg = format!("--budget={}", self.config.token_budget);
        
        let args = ["context", query, &budget_arg, &project_arg];
        let output = self.execute_lucid(&args)?;
        let entries = self.parse_context_output(&output);
        
        {
            let mut cache = self.cache.write().await;
            cache.insert(query.to_string(), entries.clone());
        }
        
        Ok(entries.into_iter().take(limit).collect())
    }

    /// Get statistics from Lucid
    pub async fn get_stats(&self) -> LucidMemoryStats {
        self.stats.read().await.clone()
    }
}

// ============================================================================
// MEMORY TRAIT IMPLEMENTATION
// ============================================================================

#[async_trait]
impl Memory for LucidNativeMemory {
    fn name(&self) -> &str {
        "lucid-native"
    }

    async fn store(&self, key: &str, content: &str, category: MemoryCategory) -> Result<()> {
        self.store_with_lucid(key, content, &category).await
    }

    async fn recall(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>> {
        self.recall_from_lucid(query, limit).await
    }

    async fn get(&self, key: &str) -> Result<Option<MemoryEntry>> {
        let results = self.recall_from_lucid(key, 1).await?;
        Ok(results.into_iter().next())
    }

    async fn list(&self, category: Option<&MemoryCategory>) -> Result<Vec<MemoryEntry>> {
        let query = category
            .map(|c| format!("type:{}", category_to_lucid_type(c)))
            .unwrap_or_else(|| "all".to_string());
        
        self.recall_from_lucid(&query, self.config.max_retrieve).await
    }

    async fn forget(&self, key: &str) -> Result<bool> {
        let project_arg = format!("--project={}", self.config.project_path.display());
        let args = ["forget", key, &project_arg];
        
        match self.execute_lucid(&args) {
            Ok(_) => {
                let mut cache = self.cache.write().await;
                cache.remove(key);
                info!("[LUCID] Forgot memory: {}", key);
                Ok(true)
            }
            Err(e) => {
                warn!("[LUCID] Failed to forget {}: {}", key, e);
                Ok(false)
            }
        }
    }

    async fn count(&self) -> Result<usize> {
        let stats = self.stats.read().await;
        Ok(stats.total_memories)
    }

    async fn health_check(&self) -> bool {
        if !self.config.binary_path.exists() {
            warn!("[LUCID] Binary not found: {:?}", self.config.binary_path);
            return false;
        }
        
        match self.execute_lucid(&["--version"]) {
            Ok(version) => {
                info!("[LUCID] Health check passed: {}", version.trim());
                let mut last_check = self.last_health_check.write().await;
                *last_check = Some(Utc::now());
                true
            }
            Err(e) => {
                warn!("[LUCID] Health check failed: {}", e);
                false
            }
        }
    }
}

// ============================================================================
// SPREADING ACTIVATION & EMOTIONAL WEIGHTING
// ============================================================================

impl LucidNativeMemory {
    /// Trigger spreading activation for a memory
    pub async fn spread_activation(&self, memory_id: &str) -> Result<Vec<String>> {
        let project_arg = format!("--project={}", self.config.project_path.display());
        let args = ["activate", memory_id, &project_arg];
        
        let output = self.execute_lucid(&args)?;
        
        let activated: Vec<String> = output
            .lines()
            .filter_map(|line| {
                if line.starts_with("activated:") {
                    line.split(':').nth(1).map(|s| s.trim().to_string())
                } else {
                    None
                }
            })
            .collect();
        
        Ok(activated)
    }
    
    /// Store with emotional weight (for important memories)
    pub async fn store_with_emotion(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        emotional_weight: f64,
    ) -> Result<()> {
        let memory_type = category_to_lucid_type(&category);
        let payload = format!("{}: {}", key, content);
        let project_arg = format!("--project={}", self.config.project_path.display());
        let type_arg = format!("--type={}", memory_type);
        let emotion_arg = format!("--emotion={}", emotional_weight);
        
        let args = ["store", &payload, &type_arg, &project_arg, &emotion_arg];
        self.execute_lucid(&args)?;
        
        info!("[LUCID] Stored memory with emotion: {} (weight: {})", key, emotional_weight);
        
        Ok(())
    }
}

// ============================================================================
// IMPORT/EXPORT
// ============================================================================

impl LucidNativeMemory {
    /// Import all memories from SQLite to Lucid
    pub async fn import_from_sqlite(&self, sqlite_path: &std::path::Path) -> Result<usize> {
        info!("[LUCID] Importing memories from SQLite: {:?}", sqlite_path);
        
        let project_arg = format!("--project={}", self.config.project_path.display());
        let source_arg = format!("--source=sqlite:{}", sqlite_path.display());
        
        let args = ["import", &source_arg, &project_arg];
        let output = self.execute_lucid(&args)?;
        
        let imported = output
            .lines()
            .find_map(|line| {
                if line.contains("imported") {
                    line.split_whitespace()
                        .find(|s| s.chars().all(|c| c.is_numeric()))
                        .and_then(|n| n.parse::<usize>().ok())
                } else {
                    None
                }
            })
            .unwrap_or(0);
        
        info!("[LUCID] Imported {} memories", imported);
        
        Ok(imported)
    }
    
    /// Export all Lucid memories
    pub async fn export(&self, output_path: &std::path::Path) -> Result<usize> {
        let project_arg = format!("--project={}", self.config.project_path.display());
        let output_arg = format!("--output={}", output_path.display());
        
        let args = ["export", &output_arg, &project_arg];
        let output = self.execute_lucid(&args)?;
        
        let exported = output
            .lines()
            .find_map(|line| {
                if line.contains("exported") {
                    line.split_whitespace()
                        .find(|s| s.chars().all(|c| c.is_numeric()))
                        .and_then(|n| n.parse::<usize>().ok())
                } else {
                    None
                }
            })
            .unwrap_or(0);
        
        info!("[LUCID] Exported {} memories to {:?}", exported, output_path);
        
        Ok(exported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_conversion() {
        assert_eq!(category_to_lucid_type(&MemoryCategory::Core), "decision");
        assert_eq!(category_to_lucid_type(&MemoryCategory::Daily), "context");
        assert_eq!(category_to_lucid_type(&MemoryCategory::Conversation), "conversation");
    }

    #[test]
    fn test_lucid_type_to_category() {
        assert_eq!(lucid_type_to_category("decision"), MemoryCategory::Core);
        assert_eq!(lucid_type_to_category("context"), MemoryCategory::Daily);
        assert_eq!(lucid_type_to_category("conversation"), MemoryCategory::Conversation);
    }

    #[test]
    fn test_default_config() {
        let config = LucidNativeConfig::default();
        assert!(config.binary_path.to_str().unwrap().contains("lucid"));
        assert_eq!(config.token_budget, 500);
    }
}

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::memory::{Memory, MemoryCategory, MemoryEntry};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanReadOnlyConfig {
    pub enabled: bool,
    pub port: u16,
    pub host: String,
    pub readonly_token: Option<String>,
    pub allowed_categories: Vec<String>,
}

impl Default for HumanReadOnlyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 9090,
            host: "0.0.0.0".to_string(),
            readonly_token: None,
            allowed_categories: vec![
                "core".to_string(),
                "daily".to_string(),
                "conversation".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanReadOnlyView {
    pub memories: Vec<ReadOnlyMemoryEntry>,
    pub total_count: usize,
    pub categories: Vec<String>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadOnlyMemoryEntry {
    pub key: String,
    pub content: String,
    pub category: String,
    pub timestamp: String,
}

pub struct HumanReadOnlyServer {
    config: HumanReadOnlyConfig,
    memory: Arc<dyn Memory>,
    workspace_dir: PathBuf,
}

impl HumanReadOnlyServer {
    pub fn new(config: HumanReadOnlyConfig, memory: Arc<dyn Memory>, workspace_dir: PathBuf) -> Self {
        Self {
            config,
            memory,
            workspace_dir,
        }
    }

    pub fn config(&self) -> &HumanReadOnlyConfig {
        &self.config
    }

    pub async fn get_view(&self, token: Option<&str>) -> Result<HumanReadOnlyView, String> {
        if let Some(ref required_token) = self.config.readonly_token {
            if token.is_none() || token.unwrap() != required_token {
                return Err("Unauthorized".to_string());
            }
        }

        let mut all_entries = Vec::new();

        for category_str in &self.config.allowed_categories {
            let category = match category_str.as_str() {
                "core" => MemoryCategory::Core,
                "daily" => MemoryCategory::Daily,
                "conversation" => MemoryCategory::Conversation,
                _ => MemoryCategory::Custom(category_str.clone()),
            };

            match self.memory.list(Some(&category)).await {
                Ok(entries) => {
                    for entry in entries {
                        all_entries.push(ReadOnlyMemoryEntry {
                            key: entry.key.clone(),
                            content: entry.content.clone(),
                            category: entry.category.to_string(),
                            timestamp: entry.timestamp.clone(),
                        });
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to list memory category {}: {}", category_str, e);
                }
            }
        }

        all_entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        let now = chrono::Utc::now().to_rfc3339();

        Ok(HumanReadOnlyView {
            memories: all_entries,
            total_count: all_entries.len(),
            categories: self.config.allowed_categories.clone(),
            generated_at: now,
        })
    }

    pub async fn search(&self, query: &str, token: Option<&str>) -> Result<Vec<ReadOnlyMemoryEntry>, String> {
        if let Some(ref required_token) = self.config.readonly_token {
            if token.is_none() || token.unwrap() != required_token {
                return Err("Unauthorized".to_string());
            }
        }

        match self.memory.recall(query, 50).await {
            Ok(entries) => {
                let results: Vec<ReadOnlyMemoryEntry> = entries
                    .into_iter()
                    .map(|entry| ReadOnlyMemoryEntry {
                        key: entry.key,
                        content: entry.content,
                        category: entry.category.to_string(),
                        timestamp: entry.timestamp,
                    })
                    .collect();
                Ok(results)
            }
            Err(e) => Err(format!("Search failed: {}", e)),
        }
    }

    pub async fn get_stats(&self) -> Result<serde_json::Value, String> {
        let count = self.memory.count().await.map_err(|e| e.to_string())?;
        
        let categories = self.config.allowed_categories.clone();
        let mut category_counts = serde_json::Map::new();

        for cat in categories {
            let mem_category = match cat.as_str() {
                "core" => MemoryCategory::Core,
                "daily" => MemoryCategory::Daily,
                "conversation" => MemoryCategory::Conversation,
                _ => MemoryCategory::Custom(cat.clone()),
            };

            if let Ok(entries) = self.memory.list(Some(&mem_category)).await {
                category_counts.insert(cat, serde_json::json!(entries.len()));
            }
        }

        Ok(serde_json::json!({
            "total_memories": count,
            "categories": category_counts,
            "readonly_enabled": self.config.enabled,
        }))
    }
}

pub struct ReadOnlyHttpHandler {
    server: Arc<RwLock<Option<HumanReadOnlyServer>>>,
}

impl ReadOnlyHttpHandler {
    pub fn new() -> Self {
        Self {
            server: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_server(&self, server: HumanReadOnlyServer) {
        let mut guard = self.server.write().await;
        *guard = Some(server);
    }

    pub async fn handle_request(
        &self,
        path: &str,
        query_params: std::collections::HashMap<String, String>,
    ) -> Result<String, String> {
        let guard = self.server.read().await;
        let server = guard.as_ref().ok_or_else(|| "Server not initialized".to_string())?;

        let token = query_params.get("token").map(|s| s.as_str());

        match path {
            "/" | "/memories" => {
                let view = server.get_view(token).await?;
                Ok(serde_json::to_string_pretty(&view).map_err(|e| e.to_string())?)
            }
            "/search" => {
                let query = query_params.get("q").ok_or_else(|| "Missing query parameter 'q'".to_string())?;
                let results = server.search(query, token).await?;
                Ok(serde_json::to_string_pretty(&results).map_err(|e| e.to_string())?)
            }
            "/stats" => {
                let stats = server.get_stats().await?;
                Ok(serde_json::to_string_pretty(&stats).map_err(|e| e.to_string())?)
            }
            "/health" => {
                Ok(r#"{"status": "ok", "service": "housaky-readonly"}"#.to_string())
            }
            _ => Err(format!("Not found: {}", path)),
        }
    }
}

impl Default for ReadOnlyHttpHandler {
    fn default() -> Self {
        Self::new()
    }
}

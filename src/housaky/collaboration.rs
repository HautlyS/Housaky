//! Inter-Instance Collaboration Module
//!
//! Enables communication between OpenClaw and Native Housaky instances
//! via shared filesystem and HTTP gateway for self-improvement and knowledge sharing.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tracing::info;

/// Shared directory for inter-instance communication
pub const SHARED_DIR: &str = "shared";
pub const INBOX_DIR: &str = "shared/inbox";
pub const OUTBOX_DIR: &str = "shared/outbox";
pub const STATE_DIR: &str = "shared/state";
pub const INSIGHTS_DIR: &str = "shared/insights";
pub const IMPROVEMENTS_DIR: &str = "shared/improvements";

/// HIIP Message types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Status,
    Task,
    Knowledge,
    Reflection,
    Error,
    Acknowledge,
    SelfImprove,
    CodeShare,
    Insight,
    SyncRequest,
    SyncResponse,
}

/// HIIP Message priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// HIIP Protocol Message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HIIPMessage {
    pub from: String,
    pub timestamp: u64,
    #[serde(rename = "type")]
    pub msg_type: MessageType,
    pub priority: Priority,
    pub payload: serde_json::Value,
}

/// Collaboration state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationState {
    pub instance: String,
    pub status: String,
    pub last_heartbeat: u64,
    pub current_task: Option<String>,
    pub capabilities: Vec<String>,
    pub version: String,
    pub self_improve_enabled: bool,
}

/// Self-improvement payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfImprovePayload {
    pub improvement_type: String,
    pub description: String,
    pub code_changes: Option<String>,
    pub file_path: Option<String>,
    pub confidence: f32,
    pub test_results: Option<String>,
}

/// Insight payload for knowledge sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightPayload {
    pub category: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub source_file: Option<String>,
}

/// Inter-instance collaboration manager
pub struct CollaborationManager {
    project_root: PathBuf,
    instance_name: String,
    peer_name: String,
    self_improve_enabled: bool,
    insights_exchange_enabled: bool,
    code_sharing_enabled: bool,
}

impl CollaborationManager {
    pub fn new(project_root: PathBuf, instance_name: &str, peer_name: &str) -> Self {
        Self {
            project_root,
            instance_name: instance_name.to_string(),
            peer_name: peer_name.to_string(),
            self_improve_enabled: true,
            insights_exchange_enabled: true,
            code_sharing_enabled: true,
        }
    }

    pub fn with_self_improve(mut self, enabled: bool) -> Self {
        self.self_improve_enabled = enabled;
        self
    }

    pub fn with_insights_exchange(mut self, enabled: bool) -> Self {
        self.insights_exchange_enabled = enabled;
        self
    }

    pub fn with_code_sharing(mut self, enabled: bool) -> Self {
        self.code_sharing_enabled = enabled;
        self
    }

    /// Read messages from inbox
    pub async fn read_inbox(&self) -> Result<Vec<HIIPMessage>> {
        let inbox_path = self.project_root.join(INBOX_DIR);

        if !inbox_path.exists() {
            return Ok(Vec::new());
        }

        let mut messages = Vec::new();
        let mut entries = fs::read_dir(&inbox_path)
            .await
            .context("Failed to read inbox directory")?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(msg) = serde_json::from_str::<HIIPMessage>(&content) {
                        messages.push(msg);
                    }
                }
            }
        }

        Ok(messages)
    }

    /// Read messages from a specific peer
    pub async fn read_peer_messages(&self) -> Result<Vec<HIIPMessage>> {
        let inbox_path = self.project_root.join(INBOX_DIR);

        if !inbox_path.exists() {
            return Ok(Vec::new());
        }

        let mut messages = Vec::new();
        let mut entries = fs::read_dir(&inbox_path)
            .await
            .context("Failed to read inbox directory")?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(msg) = serde_json::from_str::<HIIPMessage>(&content) {
                        if msg.from == self.peer_name {
                            messages.push(msg);
                        }
                    }
                }
            }
        }

        Ok(messages)
    }

    /// Send message to outbox
    pub async fn send_message(&self, msg_type: MessageType, priority: Priority, payload: serde_json::Value) -> Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("Failed to get timestamp")?
            .as_secs();

        let message = HIIPMessage {
            from: self.instance_name.clone(),
            timestamp,
            msg_type,
            priority,
            payload,
        };

        let outbox_path = self.project_root.join(OUTBOX_DIR);
        fs::create_dir_all(&outbox_path)
            .await
            .context("Failed to create outbox directory")?;

        let filename = format!("{}-{}.json", self.instance_name, timestamp);
        let file_path = outbox_path.join(filename);

        let content = serde_json::to_string_pretty(&message)
            .context("Failed to serialize message")?;

        fs::write(&file_path, content)
            .await
            .context("Failed to write message to outbox")?;

        info!("📤 Sent HIIP message: {:?}", message.msg_type);
        Ok(())
    }

    /// Update shared state
    pub async fn update_state(&self, state: &CollaborationState) -> Result<()> {
        let state_path = self.project_root.join(STATE_DIR);
        fs::create_dir_all(&state_path)
            .await
            .context("Failed to create state directory")?;

        let filename = format!("{}.json", self.instance_name);
        let file_path = state_path.join(filename);

        let content = serde_json::to_string_pretty(state)
            .context("Failed to serialize state")?;

        fs::write(&file_path, content)
            .await
            .context("Failed to write state file")?;

        info!("📝 Updated collaboration state");
        Ok(())
    }

    /// Read other instance's state
    pub async fn read_peer_state(&self, peer_name: &str) -> Result<Option<CollaborationState>> {
        let state_path = self.project_root.join(STATE_DIR).join(format!("{}.json", peer_name));

        if !state_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&state_path)
            .await
            .context("Failed to read peer state")?;

        let state: CollaborationState = serde_json::from_str(&content)
            .context("Failed to parse peer state")?;

        Ok(Some(state))
    }

    /// Check if peer is alive
    pub async fn is_peer_alive(&self, peer_name: &str, max_age_secs: u64) -> Result<bool> {
        if let Some(state) = self.read_peer_state(peer_name).await? {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs();
            return Ok(now - state.last_heartbeat < max_age_secs);
        }
        Ok(false)
    }

    /// Acknowledge a message
    pub async fn acknowledge(&self, original_msg: &HIIPMessage) -> Result<()> {
        self.send_message(
            MessageType::Acknowledge,
            Priority::Medium,
            serde_json::json!({
                "acknowledging": original_msg.timestamp,
                "status": "received"
            }),
        ).await
    }

    /// Send heartbeat to shared state
    pub async fn heartbeat(&self, current_task: Option<&str>) -> Result<()> {
        let state = CollaborationState {
            instance: self.instance_name.clone(),
            status: "active".to_string(),
            last_heartbeat: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("Failed to get timestamp")?
                .as_secs(),
            current_task: current_task.map(String::from),
            capabilities: vec![
                "agi_core".to_string(),
                "self_improvement".to_string(),
                "tui".to_string(),
                "gateway".to_string(),
                "collaboration".to_string(),
            ],
            version: env!("CARGO_PKG_VERSION").to_string(),
            self_improve_enabled: self.self_improve_enabled,
        };

        self.update_state(&state).await
    }

    /// Send self-improvement to peer
    pub async fn send_self_improvement(&self, improvement: SelfImprovePayload) -> Result<()> {
        if !self.self_improve_enabled {
            return Ok(());
        }

        self.send_message(
            MessageType::SelfImprove,
            Priority::High,
            serde_json::json!({
                "improvement_type": improvement.improvement_type,
                "description": improvement.description,
                "code_changes": improvement.code_changes,
                "file_path": improvement.file_path,
                "confidence": improvement.confidence,
                "test_results": improvement.test_results,
            }),
        ).await
    }

    /// Send code share to peer
    pub async fn share_code(&self, file_path: &str, description: &str) -> Result<()> {
        if !self.code_sharing_enabled {
            return Ok(());
        }

        let code_content = fs::read_to_string(file_path).await
            .context("Failed to read code file")?;

        self.send_message(
            MessageType::CodeShare,
            Priority::Medium,
            serde_json::json!({
                "file_path": file_path,
                "description": description,
                "code": code_content,
            }),
        ).await
    }

    /// Share insight with peer
    pub async fn share_insight(&self, insight: InsightPayload) -> Result<()> {
        if !self.insights_exchange_enabled {
            return Ok(());
        }

        // Save insight to shared directory
        let insights_path = self.project_root.join(INSIGHTS_DIR);
        fs::create_dir_all(&insights_path).await?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        let filename = format!("{}-{}.json", self.instance_name, timestamp);
        let file_path = insights_path.join(&filename);

        let content = serde_json::to_string_pretty(&insight)?;
        fs::write(&file_path, content).await?;

        // Also send as message
        self.send_message(
            MessageType::Insight,
            Priority::Medium,
            serde_json::json!({
                "category": insight.category,
                "title": insight.title,
                "content": insight.content,
                "tags": insight.tags,
                "source_file": insight.source_file,
            }),
        ).await
    }

    /// Request sync with peer
    pub async fn request_sync(&self) -> Result<()> {
        self.send_message(
            MessageType::SyncRequest,
            Priority::Low,
            serde_json::json!({
                "requesting_instance": self.instance_name,
                "capabilities": vec![
                    "self_improvement",
                    "insights",
                    "code_sharing"
                ],
            }),
        ).await
    }

    /// Respond to sync request
    pub async fn respond_to_sync(&self, request: &HIIPMessage) -> Result<()> {
        self.send_message(
            MessageType::SyncResponse,
            Priority::Medium,
            serde_json::json!({
                "responding_to": request.timestamp,
                "instance": self.instance_name,
                "capabilities": vec![
                    "self_improvement",
                    "insights", 
                    "code_sharing"
                ],
                "status": "ready",
            }),
        ).await
    }

    /// Process incoming messages
    pub async fn process_messages(&self) -> Result<Vec<HIIPMessage>> {
        let messages = self.read_peer_messages().await?;
        let mut processed = Vec::new();

        for msg in &messages {
            match msg.msg_type {
                MessageType::SyncRequest => {
                    self.respond_to_sync(msg).await?;
                    processed.push(msg.clone());
                }
                MessageType::SelfImprove if self.self_improve_enabled => {
                    info!("📥 Received self-improvement from peer: {:?}", msg.payload);
                    processed.push(msg.clone());
                }
                MessageType::CodeShare if self.code_sharing_enabled => {
                    info!("📥 Received code share from peer");
                    processed.push(msg.clone());
                }
                MessageType::Insight if self.insights_exchange_enabled => {
                    info!("📥 Received insight from peer");
                    processed.push(msg.clone());
                }
                _ => {}
            }
        }

        Ok(processed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_send_and_read_message() {
        let dir = tempdir().unwrap();
        let manager = CollaborationManager::new(dir.path().to_path_buf(), "test");

        // Send a message
        manager.send_message(
            MessageType::Status,
            Priority::Medium,
            serde_json::json!({"test": "hello"}),
        ).await.unwrap();

        // The message should be in outbox
        let outbox = dir.path().join(OUTBOX_DIR);
        assert!(outbox.exists());
    }

    #[tokio::test]
    async fn test_heartbeat() {
        let dir = tempdir().unwrap();
        let manager = CollaborationManager::new(dir.path().to_path_buf(), "native");

        manager.heartbeat(Some("testing")).await.unwrap();

        let state_path = dir.path().join(STATE_DIR).join("native.json");
        assert!(state_path.exists());
    }
}

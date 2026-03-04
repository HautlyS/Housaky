//! Inter-Instance Collaboration Module
//!
//! Enables communication between OpenClaw and Native Housaky instances
//! via shared filesystem and HTTP gateway.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tracing::{info, warn};

/// Shared directory for inter-instance communication
pub const SHARED_DIR: &str = "shared";
pub const INBOX_DIR: &str = "shared/inbox";
pub const OUTBOX_DIR: &str = "shared/outbox";
pub const STATE_DIR: &str = "shared/state";

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
}

/// Inter-instance collaboration manager
pub struct CollaborationManager {
    project_root: PathBuf,
    instance_name: String,
}

impl CollaborationManager {
    pub fn new(project_root: PathBuf, instance_name: &str) -> Self {
        Self {
            project_root,
            instance_name: instance_name.to_string(),
        }
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
            ],
        };

        self.update_state(&state).await
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

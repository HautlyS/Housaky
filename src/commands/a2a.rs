//! A2A Commands - Agent-to-Agent Communication
//!
//! Direct communication between Housaky instances (Native ↔ OpenClaw)
//! for collaborative self-improvement toward AGI singularity.

use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum A2ACommands {
    /// Check peer status via ping
    Ping,
    
    /// Sync state with peer
    Sync,
    
    /// Send a task to peer
    Task {
        /// Task ID
        #[arg(long)]
        id: Option<String>,
        /// Action (analyze, review, improve, test, explain)
        #[arg(long)]
        action: String,
        /// Parameters as JSON
        #[arg(long, default_value = "{}")]
        params: String,
    },
    
    /// Share a learning with peer
    Learn {
        /// Category (optimization, bugfix, architecture, reasoning, memory, general)
        #[arg(long)]
        category: String,
        /// Learning content
        #[arg(long)]
        content: String,
        /// Confidence (0.0-1.0)
        #[arg(long, default_value = "0.8")]
        confidence: f32,
    },
    
    /// Share code improvement
    CodeImprove {
        /// File path
        #[arg(long)]
        file: String,
        /// Diff content
        #[arg(long)]
        diff: String,
    },
    
    /// Request code review from peer
    Review {
        /// File to review
        file: String,
    },
    
    /// Show A2A status
    Status,
    
    /// List pending messages
    Inbox,
    
    /// Process pending messages
    Process,
    
    /// Show peer state
    Peers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AStatus {
    pub instance: String,
    pub peer_status: String,
    pub pending_inbox: usize,
    pub pending_outbox: usize,
    pub last_sync: Option<String>,
    pub messages_sent: u64,
    pub messages_received: u64,
}

/// Default A2A shared directory
pub fn default_a2a_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("housaky")
        .join("shared")
        .join("a2a")
}

/// Create an A2A message file
pub fn create_message(
    from: &str,
    to: &str,
    msg_type: &str,
    data: serde_json::Value,
    priority: u8,
) -> (String, serde_json::Value) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    let id = format!("{}-{}", from.to_lowercase(), ts);
    
    let msg = serde_json::json!({
        "id": &id,
        "from": from,
        "to": to,
        "ts": ts * 1000,
        "pri": priority,
        "t": msg_type,
        "d": data
    });
    
    (id, msg)
}

/// Write message to outbox
pub async fn send_message(msg: &serde_json::Value, a2a_dir: &PathBuf) -> anyhow::Result<String> {
    let id = msg["id"].as_str().unwrap_or("unknown").to_string();
    let from = msg["from"].as_str().unwrap_or("unknown").to_string();
    
    let outbox_dir = a2a_dir.join("outbox").join(&from);
    tokio::fs::create_dir_all(&outbox_dir).await?;
    
    let filename = format!("{}-{}.a2a", from, id);
    let path = outbox_dir.join(&filename);
    
    let content = serde_json::to_string(&msg)?;
    tokio::fs::write(&path, &content).await?;
    
    Ok(filename)
}

/// Read messages from inbox
pub async fn read_inbox(instance: &str, a2a_dir: &PathBuf) -> anyhow::Result<Vec<serde_json::Value>> {
    let inbox_dir = a2a_dir.join("inbox").join(instance);
    
    if !inbox_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut messages = Vec::new();
    let mut entries = tokio::fs::read_dir(&inbox_dir).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().map(|e| e == "a2a").unwrap_or(false) {
            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&content) {
                    messages.push(msg);
                }
            }
        }
    }
    
    // Sort by timestamp
    messages.sort_by(|a, b| {
        let ts_a = a["ts"].as_u64().unwrap_or(0);
        let ts_b = b["ts"].as_u64().unwrap_or(0);
        ts_a.cmp(&ts_b)
    });
    
    Ok(messages)
}

/// Read peer state
pub async fn read_peer_state(peer: &str, a2a_dir: &PathBuf) -> anyhow::Result<Option<serde_json::Value>> {
    let state_path = a2a_dir.join("state").join(format!("{}.json", peer));
    
    if !state_path.exists() {
        return Ok(None);
    }
    
    let content = tokio::fs::read_to_string(&state_path).await?;
    let state: serde_json::Value = serde_json::from_str(&content)?;
    
    Ok(Some(state))
}

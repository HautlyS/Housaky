//! Agent-to-Agent (A2A) Communication Module
//!
//! Direct, efficient binary/JSON communication channel between Housaky and OpenClaw instances.
//! More efficient than human language - uses structured data with compact encoding.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tracing::info;

/// A2A Message Priority (lower number = higher priority)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Priority(pub u8);

impl Priority {
    pub const CRITICAL: Priority = Priority(0);
    pub const HIGH: Priority = Priority(1);
    pub const NORMAL: Priority = Priority(2);
    pub const LOW: Priority = Priority(3);
}

/// A2A Message Types - optimized for machine-to-machine communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", content = "d")]
pub enum A2AMessageType {
    /// Request computation/task from peer
    Task { id: String, action: String, params: serde_json::Value },
    /// Response to a task
    TaskResult { id: String, result: serde_json::Value, success: bool },
    /// Share memory/context
    Context { memory_type: String, data: serde_json::Value },
    /// Share learning/improvement
    Learning { category: String, content: String, confidence: f32 },
    /// Share code improvement
    CodeImprove { file: String, diff: String, language: String },
    /// Request sync state
    SyncRequest,
    /// Sync response with state
    SyncResponse { state: A2AState },
    /// Heartbeat/alive check
    Ping,
    /// Pong response
    Pong,
    /// Share metrics/telemetry
    Metrics { cpu: f32, memory: f32, tasks_done: u64, errors: u64 },
    /// Share goal status
    GoalStatus { goals: Vec<A2AGoal> },
    /// Emergency stop (critical)
    Stop,
    /// Acknowledgment
    Ack { id: String },
}

/// A2A State for sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AState {
    pub instance: String,
    pub version: String,
    pub uptime_secs: u64,
    pub tasks_active: u32,
    pub goals_count: u32,
    pub memory_entries: u64,
    pub last_error: Option<String>,
}

/// A2A Goal representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AGoal {
    pub id: String,
    pub title: String,
    pub progress: f32,
    pub status: String,
}

/// A2A Message - compact binary-ready format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    /// Unique message ID
    pub id: String,
    /// Source instance name
    pub from: String,
    /// Destination instance name (or broadcast)
    pub to: String,
    /// Message timestamp (Unix epoch millis)
    pub ts: u64,
    /// Priority level (0-3)
    pub pri: u8,
    /// Message type and payload
    #[serde(flatten)]
    pub msg: A2AMessageType,
    /// Optional correlation ID for request/response
    pub corr_id: Option<String>,
}

impl A2AMessage {
    /// Create a new A2A message
    pub fn new(from: &str, to: &str, msg: A2AMessageType) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from: from.to_string(),
            to: to.to_string(),
            ts,
            pri: 2,
            msg,
            corr_id: None,
        }
    }

    /// Create a task request
    pub fn task(from: &str, to: &str, task_id: &str, action: &str, params: serde_json::Value) -> Self {
        Self::new(from, to, A2AMessageType::Task {
            id: task_id.to_string(),
            action: action.to_string(),
            params,
        })
    }

    /// Create a ping message
    pub fn ping(from: &str, to: &str) -> Self {
        Self::new(from, to, A2AMessageType::Ping)
    }

    /// Create a sync request
    pub fn sync_request(from: &str, to: &str) -> Self {
        Self::new(from, to, A2AMessageType::SyncRequest)
    }

    /// Create a context share
    pub fn context(from: &str, to: &str, memory_type: &str, data: serde_json::Value) -> Self {
        Self::new(from, to, A2AMessageType::Context {
            memory_type: memory_type.to_string(),
            data,
        })
    }

    /// Create a learning share
    pub fn learning(from: &str, to: &str, category: &str, content: &str, confidence: f32) -> Self {
        Self::new(from, to, A2AMessageType::Learning {
            category: category.to_string(),
            content: content.to_string(),
            confidence,
        })
    }

    /// Create a code improvement share
    pub fn code_improve(from: &str, to: &str, file: &str, diff: &str, language: &str) -> Self {
        Self::new(from, to, A2AMessageType::CodeImprove {
            file: file.to_string(),
            diff: diff.to_string(),
            language: language.to_string(),
        })
    }

    /// Set correlation ID for request/response tracking
    pub fn with_corr_id(mut self, corr_id: &str) -> Self {
        self.corr_id = Some(corr_id.to_string());
        self
    }

    /// Set priority
    pub fn with_priority(mut self, pri: Priority) -> Self {
        self.pri = pri.0;
        self
    }

    /// Serialize to compact JSON (no pretty printing for efficiency)
    pub fn to_compact_json(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// Serialize to bytes for binary transmission
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(self.to_compact_json()?.into_bytes())
    }

    /// Deserialize from JSON
    pub fn from_json(s: &str) -> Result<Self> {
        Ok(serde_json::from_str(s)?)
    }

    /// Deserialize from bytes
    pub fn from_bytes(b: &[u8]) -> Result<Self> {
        Self::from_json(&String::from_utf8_lossy(b))
    }
}

/// A2A Communication Manager
pub struct A2AManager {
    instance_name: String,
    peer_name: String,
    shared_dir: PathBuf,
}

impl A2AManager {
    pub fn new(shared_dir: PathBuf, instance_name: &str, peer_name: &str) -> Self {
        Self {
            instance_name: instance_name.to_string(),
            peer_name: peer_name.to_string(),
            shared_dir,
        }
    }

    /// Send a message to peer
    pub async fn send(&self, msg: &A2AMessage) -> Result<()> {
        let a2a_dir = self.shared_dir.join("a2a");
        fs::create_dir_all(&a2a_dir).await?;
        
        let outbox = a2a_dir.join("outbox");
        fs::create_dir_all(&outbox).await?;
        
        let filename = format!("{}-{}.a2a", self.instance_name, msg.id);
        let path = outbox.join(filename);
        
        let content = msg.to_compact_json()?;
        fs::write(&path, content).await?;
        
        info!("📤 A2A sent: {:?} -> {}", msg.msg, msg.to);
        Ok(())
    }

    /// Send message and wait for response
    pub async fn send_and_wait(&self, msg: &A2AMessage, timeout_secs: u64) -> Result<Option<A2AMessage>> {
        self.send(msg).await?;
        
        let corr_id = msg.id.clone();
        let start = std::time::Instant::now();
        
        loop {
            if start.elapsed().as_secs() > timeout_secs {
                return Ok(None);
            }
            
            if let Some(resp) = self.read_messages()?.into_iter().find(|m| m.corr_id.as_ref() == Some(&corr_id)) {
                return Ok(Some(resp));
            }
            
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    /// Read all pending messages from peer
    pub fn read_messages(&self) -> Result<Vec<A2AMessage>> {
        let a2a_dir = self.shared_dir.join("a2a");
        let inbox = a2a_dir.join("inbox").join(&self.peer_name);
        
        if !inbox.exists() {
            return Ok(Vec::new());
        }
        
        let mut messages = Vec::new();
        
        if let Ok(entries) = std::fs::read_dir(&inbox) {
            for entry in entries.flatten() {
                if entry.path().extension().map_or(false, |e| e == "a2a") {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        if let Ok(msg) = A2AMessage::from_json(&content) {
                            messages.push(msg);
                        }
                    }
                }
            }
        }
        
        Ok(messages)
    }

    /// Read messages from a specific sender
    pub fn read_from(&self, from: &str) -> Result<Vec<A2AMessage>> {
        Ok(self.read_messages()?.into_iter()
            .filter(|m| m.from == from)
            .collect())
    }

    /// Process incoming messages and return responses
    pub fn process(&self) -> Result<Vec<A2AMessage>> {
        let messages = self.read_messages()?;
        let mut responses = Vec::new();
        
        for msg in messages {
            match &msg.msg {
                A2AMessageType::Ping => {
                    responses.push(A2AMessage::new(&self.instance_name, &msg.from, A2AMessageType::Pong));
                }
                A2AMessageType::SyncRequest => {
                    let state = A2AState {
                        instance: self.instance_name.clone(),
                        version: env!("CARGO_PKG_VERSION").to_string(),
                        uptime_secs: 0,
                        tasks_active: 0,
                        goals_count: 0,
                        memory_entries: 0,
                        last_error: None,
                    };
                    responses.push(A2AMessage::new(&self.instance_name, &msg.from, A2AMessageType::SyncResponse { state })
                        .with_corr_id(&msg.id));
                }
                A2AMessageType::Task { id, action, params: _ } => {
                    info!("📥 A2A Task received: {} with action {}", id, action);
                    responses.push(A2AMessage::new(&self.instance_name, &msg.from, A2AMessageType::TaskResult {
                        id: id.clone(),
                        result: serde_json::json!({"status": "received", "action": action}),
                        success: true,
                    }).with_corr_id(&msg.id));
                }
                A2AMessageType::Learning { category, content: _, confidence } => {
                    info!("📚 A2A Learning: {} (conf: {})", category, confidence);
                    responses.push(A2AMessage::new(&self.instance_name, &msg.from, A2AMessageType::Ack { id: msg.id.clone() }));
                }
                A2AMessageType::CodeImprove { file, diff: _, language } => {
                    info!("💻 A2A Code improve: {} ({})", file, language);
                    responses.push(A2AMessage::new(&self.instance_name, &msg.from, A2AMessageType::Ack { id: msg.id.clone() }));
                }
                _ => {}
            }
        }
        
        Ok(responses)
    }

    /// Quick send to peer without waiting
    pub async fn tell(&self, msg: A2AMessageType) -> Result<()> {
        let msg = A2AMessage::new(&self.instance_name, &self.peer_name, msg);
        self.send(&msg).await
    }

    /// Ask peer and wait for response
    pub async fn ask(&self, msg: A2AMessageType, timeout_secs: u64) -> Result<Option<A2AMessage>> {
        let msg = A2AMessage::new(&self.instance_name, &self.peer_name, msg);
        self.send_and_wait(&msg, timeout_secs).await
    }
}

/// High-level A2A operations for self-improvement
pub struct A2ASelfImprove {
    manager: A2AManager,
}

impl A2ASelfImprove {
    pub fn new(shared_dir: PathBuf, instance_name: &str, peer_name: &str) -> Self {
        Self {
            manager: A2AManager::new(shared_dir, instance_name, peer_name),
        }
    }

    /// Request peer to review code
    pub async fn request_code_review(&self, file: &str, diff: &str) -> Result<()> {
        self.manager.send(&A2AMessage::code_improve(
            &self.manager.instance_name,
            &self.manager.peer_name,
            file,
            diff,
            "rust",
        )).await
    }

    /// Share a learning with peer
    pub async fn share_learning(&self, category: &str, content: &str, confidence: f32) -> Result<()> {
        self.manager.send(&A2AMessage::learning(
            &self.manager.instance_name,
            &self.manager.peer_name,
            category,
            content,
            confidence,
        )).await
    }

    /// Request peer state sync
    pub async fn sync_with_peer(&self, timeout_secs: u64) -> Result<Option<A2AMessage>> {
        self.manager.ask(A2AMessageType::SyncRequest, timeout_secs).await
    }

    /// Send task to peer
    pub async fn delegate_task(&self, task_id: &str, action: &str, params: serde_json::Value) -> Result<()> {
        self.manager.send(&A2AMessage::task(
            &self.manager.instance_name,
            &self.manager.peer_name,
            task_id,
            action,
            params,
        )).await
    }

    /// Check if peer is alive
    pub async fn ping_peer(&self) -> Result<bool> {
        match self.manager.ask(A2AMessageType::Ping, 5).await {
            Ok(Some(msg)) => Ok(matches!(msg.msg, A2AMessageType::Pong)),
            _ => Ok(false),
        }
    }
}

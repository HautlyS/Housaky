//! WebChat Channel for Housaky
//!
//! Built-in WebSocket chat interface similar to OpenClaw's WebChat.
//! Uses the Gateway WebSocket for real-time messaging.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use parking_lot::RwLock;

use super::traits::{Channel, ChannelMessage};

/// WebChat channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebChatConfig {
    /// Enable WebChat interface
    pub enabled: Option<bool>,
    /// Port for WebSocket server (default: 8081)
    pub port: Option<u16>,
    /// Host to bind (default: 127.0.0.1)
    pub host: Option<String>,
    /// Allow public access (default: false - requires pairing)
    pub allow_public: Option<bool>,
    /// Session timeout in seconds (default: 3600)
    pub session_timeout: Option<u64>,
    /// Maximum concurrent sessions (default: 100)
    pub max_sessions: Option<usize>,
}

/// WebChat session representing a connected client
#[derive(Debug, Clone)]
pub struct WebChatSession {
    pub id: String,
    pub sender: String,
    pub connected_at: u64,
    pub paired: bool,
}

/// WebChat channel - WebSocket-based chat interface
pub struct WebChatChannel {
    config: WebChatConfig,
    sessions: Arc<RwLock<HashMap<String, WebChatSession>>>,
    message_tx: broadcast::Sender<ChannelMessage>,
}

impl WebChatChannel {
    pub fn new(config: WebChatConfig) -> Self {
        let (message_tx, _) = broadcast::channel(1000);
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            message_tx,
        }
    }

    /// Get the WebSocket URL for this channel
    pub fn ws_url(&self) -> String {
        let host = self.config.host.as_deref().unwrap_or("127.0.0.1");
        let port = self.config.port.unwrap_or(8081);
        format!("ws://{}:{}/ws", host, port)
    }

    /// Register a new session
    pub fn register_session(&self, session_id: &str, sender: String) {
        let session = WebChatSession {
            id: session_id.to_string(),
            sender,
            connected_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            paired: false,
        };
        self.sessions.write().insert(session_id.to_string(), session);
    }

    /// Mark a session as paired
    pub fn pair_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get_mut(session_id) {
            session.paired = true;
            true
        } else {
            false
        }
    }

    /// Remove a session
    pub fn remove_session(&self, session_id: &str) {
        self.sessions.write().remove(session_id);
    }

    /// Get all active sessions
    pub fn get_sessions(&self) -> Vec<WebChatSession> {
        self.sessions.read().values().cloned().collect()
    }

    /// Broadcast message to all paired sessions
    pub fn broadcast(&self, message: &str, sender: &str) {
        let msg = ChannelMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender: sender.to_string(),
            content: message.to_string(),
            channel: "webchat".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        let _ = self.message_tx.send(msg);
    }

    /// Subscribe to incoming messages
    pub fn subscribe(&self) -> broadcast::Receiver<ChannelMessage> {
        self.message_tx.subscribe()
    }
}

#[async_trait]
impl Channel for WebChatChannel {
    fn name(&self) -> &str {
        "webchat"
    }

    async fn send(&self, message: &str, recipient: &str) -> Result<()> {
        self.broadcast(message, "housaky");
        Ok(())
    }

    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<()> {
        let mut rx = self.subscribe();
        
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if tx.send(msg).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("WebChat lagged {} messages", n);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
        Ok(())
    }

    async fn health_check(&self) -> bool {
        self.config.enabled.unwrap_or(true)
    }
}

/// WebSocket message types for WebChat protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebSocketMessage {
    Pair { code: String },
    Message { content: String },
    ServerMessage { id: String, content: String, sender: String, timestamp: u64 },
    Paired { session_id: String },
    Error { message: String },
    Ping,
    Pong,
}

impl WebSocketMessage {
    pub fn into_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }
}

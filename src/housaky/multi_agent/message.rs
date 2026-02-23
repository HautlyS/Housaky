use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub msg_type: MessageType,
    pub sender: String,
    pub receiver: Option<String>,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    TaskSubmitted,
    TaskAssigned,
    TaskProgress,
    TaskCompleted,
    TaskFailed,
    Query,
    Response,
    Broadcast,
    Heartbeat,
    StatusUpdate,
    Error,
    Warning,
    Info,
    Coordination,
    ConsensusRequest,
    ConsensusResponse,
    ResourceRequest,
    ResourceGranted,
    ResourceDenied,
}

impl AgentMessage {
    pub fn new(msg_type: MessageType, sender: &str, content: &str) -> Self {
        Self {
            id: format!("msg_{}", uuid::Uuid::new_v4()),
            msg_type,
            sender: sender.to_string(),
            receiver: None,
            content: content.to_string(),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn to(mut self, receiver: &str) -> Self {
        self.receiver = Some(receiver.to_string());
        self
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    pub fn is_broadcast(&self) -> bool {
        self.receiver.is_none()
    }

    pub fn is_for(&self, agent_id: &str) -> bool {
        self.receiver.as_ref().map_or(true, |r| r == agent_id)
    }

    pub fn age_seconds(&self) -> i64 {
        (chrono::Utc::now() - self.timestamp).num_seconds()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBatch {
    pub messages: Vec<AgentMessage>,
    pub batch_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl MessageBatch {
    pub fn new(messages: Vec<AgentMessage>) -> Self {
        Self {
            messages,
            batch_id: format!("batch_{}", uuid::Uuid::new_v4()),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn for_agent(&self, agent_id: &str) -> Vec<&AgentMessage> {
        self.messages
            .iter()
            .filter(|m| m.is_for(agent_id))
            .collect()
    }

    pub fn by_type(&self, msg_type: &MessageType) -> Vec<&AgentMessage> {
        self.messages
            .iter()
            .filter(|m| &m.msg_type == msg_type)
            .collect()
    }
}

//! Protocol definitions for P2P
use serde::{Deserialize, Serialize};

/// Message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Heartbeat,
    DataTransfer,
    Consensus,
    Discovery,
}

/// Generic network message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMessage {
    pub msg_type: MessageType,
    pub payload: Vec<u8>,
    pub timestamp: u64,
}

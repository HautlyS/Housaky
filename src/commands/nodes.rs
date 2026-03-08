//! Nodes Commands - Anonymous QUIC-Encrypted Peer Communication
//!
//! Secure, anonymous peer-to-peer communication for sharing:
//! - Code improvements (diffs)
//! - Tools and capabilities
//! - Security insights
//! - AGI learnings
//!
//! NO device access - no camera, screen, location, or personal data.
//! 100% anonymous and encrypted via QUIC + X25519 + ChaCha20-Poly1305.

use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodesCommands {
    /// Show node status
    Status,
    /// List connected peers (anonymous IDs only)
    List { #[arg(long)] json: bool },
    /// Describe a peer by anonymous ID
    Describe { peer_id: String },
    /// Check for pending peer requests
    Pending,
    /// Approve a peer connection
    Approve { request_id: String },
    /// Reject a peer connection
    Reject { request_id: String },
    /// Share a code improvement (diff) with peers
    ShareDiff {
        /// Path to diff file
        #[arg(short, long)]
        file: PathBuf,
        /// Description
        #[arg(short, long)]
        message: String,
        /// Category (optimization, security, feature, bugfix)
        #[arg(long, default_value = "improvement")]
        category: String,
    },
    /// Share a tool definition with peers
    ShareTool {
        /// Tool name
        name: String,
        /// Tool JSON definition
        #[arg(long)]
        definition: String,
    },
    /// Share security insight
    ShareSecurity {
        /// Insight type (vulnerability, mitigation, pattern)
        #[arg(long)]
        kind: String,
        /// Description
        #[arg(long)]
        description: String,
    },
    /// Request improvements from peers
    RequestImprovements {
        /// Target module/file
        #[arg(short, long)]
        target: String,
        /// Focus area
        #[arg(long)]
        focus: Option<String>,
    },
    /// Request tool from peers
    RequestTool {
        /// Tool capability needed
        capability: String,
    },
    /// Broadcast AGI learning to all peers
    BroadcastLearning {
        /// Category
        #[arg(long)]
        category: String,
        /// Learning content
        #[arg(long)]
        content: String,
        /// Confidence (0-100)
        #[arg(long, default_value = "90")]
        confidence: u8,
    },
    /// Show peer capabilities (tools they can share)
    Capabilities { peer_id: String },
    /// Generate new anonymous identity
    RegenerateIdentity,
    /// Show connection encryption status
    EncryptionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Anonymous peer ID (derived from public key, not traceable)
    pub anonymous_id: String,
    /// Connection status
    pub status: PeerStatus,
    /// Capabilities this peer can share
    pub shareable_capabilities: Vec<ShareableCapability>,
    /// Last seen (encrypted ping)
    pub last_seen: Option<String>,
    /// Encryption verified
    pub encryption_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PeerStatus {
    Connected,
    Disconnected,
    PendingApproval,
}

/// Only shareable capabilities - NO device access
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShareableCapability {
    /// Code improvements and diffs
    CodeImprovements,
    /// Tool definitions
    ToolSharing,
    /// Security insights
    SecurityInsights,
    /// AGI learnings
    AGILearnings,
    /// Reasoning patterns
    ReasoningPatterns,
    /// Optimization techniques
    Optimizations,
}

impl std::fmt::Display for ShareableCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShareableCapability::CodeImprovements => write!(f, "code"),
            ShareableCapability::ToolSharing => write!(f, "tools"),
            ShareableCapability::SecurityInsights => write!(f, "security"),
            ShareableCapability::AGILearnings => write!(f, "learnings"),
            ShareableCapability::ReasoningPatterns => write!(f, "reasoning"),
            ShareableCapability::Optimizations => write!(f, "optimizations"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerRequest {
    /// Anonymous request ID
    pub id: String,
    /// Anonymous peer ID (derived from their public key)
    peer_anonymous_id: String,
    /// Requested capability
    pub capability: ShareableCapability,
    /// Created timestamp
    pub created_at: String,
    /// Expires timestamp
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedDiff {
    pub id: String,
    pub from_peer: String,
    pub category: String,
    pub message: String,
    pub diff_content: String,
    pub timestamp: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedTool {
    pub id: String,
    pub from_peer: String,
    pub name: String,
    pub definition: String,
    pub timestamp: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityInsight {
    pub id: String,
    pub from_peer: String,
    pub kind: String,
    pub description: String,
    pub severity: String,
    pub timestamp: String,
    pub signature: String,
}

/// QUIC encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEncryptionConfig {
    /// Use QUIC protocol (UDP-based, encrypted by default)
    pub quic_enabled: bool,
    /// X25519 key exchange
    pub key_exchange: String,
    /// ChaCha20-Poly1305 AEAD
    pub cipher: String,
    /// Certificate pinning
    pub cert_pinning: bool,
    /// No logging of peer IPs
    pub anonymous_routing: bool,
    /// Drop packets from unknown peers
    pub require_authentication: bool,
}

impl Default for NodeEncryptionConfig {
    fn default() -> Self {
        Self {
            quic_enabled: true,
            key_exchange: "X25519".to_string(),
            cipher: "ChaCha20-Poly1305".to_string(),
            cert_pinning: true,
            anonymous_routing: true,
            require_authentication: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub enabled: bool,
    pub encryption: NodeEncryptionConfig,
    pub max_peers: usize,
    pub anonymous_id: String,
    pub data_dir: PathBuf,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            encryption: NodeEncryptionConfig::default(),
            max_peers: 50,
            anonymous_id: uuid::Uuid::new_v4().to_string(),
            data_dir: PathBuf::from("~/.housaky/peers"),
        }
    }
}

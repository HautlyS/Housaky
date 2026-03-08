//! Nodes Commands - Device Pairing and Control
//!
//! Control paired devices (mobile, IoT) for AGI embodiment.
//! Inspired by OpenClaw's nodes tool for camera, screen, notifications, etc.

use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodesCommands {
    /// Show node status
    Status,
    /// List paired nodes
    List { #[arg(long)] json: bool },
    /// Describe a specific node
    Describe { node_id: String },
    /// Check for pending pairing requests
    Pending,
    /// Approve a pairing request
    Approve { request_id: String },
    /// Reject a pairing request
    Reject { request_id: String },
    /// Send notification to node
    Notify {
        /// Node ID or name
        node: String,
        /// Notification title
        #[arg(long)]
        title: String,
        /// Notification body
        #[arg(long)]
        body: String,
    },
    /// Capture photo from node camera
    CameraSnap {
        /// Node ID
        node: String,
        /// Camera facing: front, back, both
        #[arg(long, default_value = "back")]
        facing: String,
    },
    /// Record screen on node
    ScreenRecord {
        /// Node ID
        node: String,
        /// Duration in seconds
        #[arg(long, default_value = "30")]
        duration: u64,
    },
    /// Get node location
    Location {
        /// Node ID
        node: String,
    },
    /// Run command on node
    Run {
        /// Node ID
        node: String,
        /// Command to execute
        command: String,
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Invoke a method on node
    Invoke {
        /// Node ID
        node: String,
        /// Method name
        method: String,
        #[arg(long)]
        params_json: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub status: NodeStatus,
    pub capabilities: Vec<NodeCapability>,
    pub last_seen: Option<String>,
    pub battery: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeStatus {
    Online,
    Offline,
    Sleeping,
    Busy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeCapability {
    Camera,
    ScreenRecording,
    Location,
    Notifications,
    RemoteExecution,
    VoiceCall,
    FileAccess,
}

impl std::fmt::Display for NodeCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeCapability::Camera => write!(f, "camera"),
            NodeCapability::ScreenRecording => write!(f, "screen"),
            NodeCapability::Location => write!(f, "location"),
            NodeCapability::Notifications => write!(f, "notifications"),
            NodeCapability::RemoteExecution => write!(f, "exec"),
            NodeCapability::VoiceCall => write!(f, "voice"),
            NodeCapability::FileAccess => write!(f, "files"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingRequest {
    pub id: String,
    pub node_name: String,
    pub platform: String,
    pub created_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub enabled: bool,
    pub pairing_required: bool,
    pub auto_approve: bool,
    pub max_nodes: usize,
    pub node_dir: PathBuf,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            pairing_required: true,
            auto_approve: false,
            max_nodes: 10,
            node_dir: PathBuf::from("~/.housaky/nodes"),
        }
    }
}

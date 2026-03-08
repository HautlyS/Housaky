//! System Commands

use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SystemCommands {
    /// Enqueue a system event
    Event { text: String, #[arg(long)] heartbeat: bool },
    /// Heartbeat controls
    Heartbeat {
        #[command(subcommand)]
        action: HeartbeatAction,
    },
    /// List system presence entries
    Presence { #[arg(long)] json: bool },
    /// Show system info
    Info { #[arg(long)] json: bool },
}

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HeartbeatAction {
    Trigger,
    Enable,
    Disable,
    Status,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub id: String,
    pub text: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceEntry {
    pub id: String,
    pub name: String,
    pub status: String,
}

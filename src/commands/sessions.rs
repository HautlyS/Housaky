//! Session Management Commands

use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionsCommands {
    /// List all sessions
    List {
        #[arg(long)]
        active: Option<u64>,
        #[arg(long)]
        json: bool,
    },
    /// Show session details
    Show {
        id: String,
        #[arg(long)]
        messages: bool,
    },
    /// Delete session
    Delete { id: String },
    /// Export session
    Export {
        id: String,
        #[arg(short, long)]
        output: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: usize,
    pub tokens_used: u64,
}

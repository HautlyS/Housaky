//! Approvals Commands

use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApprovalsCommands {
    /// Fetch approvals snapshot
    Get { #[arg(long)] json: bool },
    /// Replace approvals from JSON file
    Set { file: std::path::PathBuf },
    /// Clear all approvals
    Clear { #[arg(long)] agent: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRule {
    pub pattern: String,
    pub allowed: bool,
}

impl Default for ApprovalRule {
    fn default() -> Self {
        Self { pattern: "*".to_string(), allowed: false }
    }
}

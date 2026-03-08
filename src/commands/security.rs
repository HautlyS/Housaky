//! Security Commands

use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SecurityCommands {
    /// Run security audit
    Audit { #[arg(long)] deep: bool, #[arg(long)] fix: bool },
    /// Check file permissions
    Permissions { #[arg(long)] fix: bool },
    /// Check for exposed secrets
    Secrets { #[arg(long)] git_history: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditResult {
    pub passed: bool,
    pub issues: Vec<SecurityIssue>,
    pub score: u8,
}

impl Default for SecurityAuditResult {
    fn default() -> Self {
        Self { passed: true, issues: Vec::new(), score: 100 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIssue {
    pub severity: String,
    pub category: String,
    pub description: String,
    pub recommendation: String,
}

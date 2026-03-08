//! Sandbox Commands

use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SandboxCommands {
    /// List sandbox containers
    List {
        #[arg(long)]
        json: bool,
    },
    /// Show sandbox status
    Status { name: Option<String> },
    /// Create sandbox
    Create {
        name: String,
        #[arg(long, default_value = "ubuntu:22.04")]
        image: String,
    },
    /// Remove sandbox
    Remove {
        name: String,
        #[arg(long)]
        force: bool,
    },
    /// Execute command in sandbox
    Exec { name: String, command: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxInfo {
    pub name: String,
    pub image: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub enabled: bool,
    pub image: String,
    pub memory_limit: String,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            image: "ubuntu:22.04".to_string(),
            memory_limit: "512m".to_string(),
        }
    }
}

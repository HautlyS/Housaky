//! Memory Management Commands
//!
//! Search, inspect, and reindex memory files for AI agents.

use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryCommands {
    /// Show memory system status
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Search memory files
    Search {
        #[arg(short, long)]
        query: String,
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
        #[arg(long)]
        min_score: Option<f32>,
    },
    /// Reindex memory files
    Index {
        #[arg(long)]
        force: bool,
    },
    /// Get memory entry by path
    Get {
        path: String,
        #[arg(long)]
        lines: Option<String>,
    },
    /// List recent memory files
    List {
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStatus {
    pub indexed_files: usize,
    pub total_entries: usize,
    pub total_size_bytes: u64,
    pub last_indexed: Option<String>,
}

impl Default for MemoryStatus {
    fn default() -> Self {
        Self {
            indexed_files: 0,
            total_entries: 0,
            total_size_bytes: 0,
            last_indexed: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchResult {
    pub path: String,
    pub line: usize,
    pub content: String,
    pub score: f32,
}

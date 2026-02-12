//! CLI interface
use anyhow::Result;
use clap::{Parser, Subcommand};

/// Housaky CLI
#[derive(Parser)]
#[command(name = "housaky")]
#[command(about = "Quantum-Inspired AGI Node")]
pub struct Cli {
    /// Node ID
    #[arg(short, long)]
    pub node_id: Option<String>,

    /// Port to listen on
    #[arg(short, long, default_value = "8080")]
    pub port: u16,

    /// Configuration file
    #[arg(short, long)]
    pub config: Option<String>,

    /// Subcommand
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// CLI subcommands
#[derive(Subcommand)]
pub enum Commands {
    /// Start the node
    Start {
        /// Enable Li-Fi
        #[arg(long)]
        lifi: bool,
    },
    /// Check status
    Status,
    /// Run tests
    Test,
}

/// Parse CLI arguments
pub fn parse() -> Cli {
    Cli::parse()
}

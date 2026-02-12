//! Main entry point for housaky CLI
use anyhow::Result;
use housaky_api::{cli, server};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = cli::parse();

    let node_id = cli
        .node_id
        .unwrap_or_else(|| format!("node_{}", rand::random::<u16>()));

    match cli.command {
        Some(cli::Commands::Start { lifi }) => {
            tracing::info!("Starting Housaky node {} (Li-Fi: {})", node_id, lifi);

            // Start API server
            server::start_server(cli.port, node_id).await?;
        }
        Some(cli::Commands::Status) => {
            println!("Node: {}", node_id);
            println!("Status: Not implemented");
        }
        Some(cli::Commands::Test) => {
            println!("Running tests...");
        }
        None => {
            // Default: start the node
            tracing::info!("Starting Housaky node {}", node_id);
            server::start_server(cli.port, node_id).await?;
        }
    }

    Ok(())
}

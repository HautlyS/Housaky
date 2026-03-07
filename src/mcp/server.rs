use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub enabled: bool,
}

pub struct McpServer {
    config: McpServerConfig,
    child: Option<tokio::process::Child>,
    workspace_dir: PathBuf,
}

impl McpServer {
    pub fn new(config: McpServerConfig, workspace_dir: PathBuf) -> Self {
        Self {
            config,
            child: None,
            workspace_dir,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut cmd = Command::new(&self.config.command);
        cmd.args(&self.config.args)
            .envs(&self.config.env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let child = cmd.spawn()?;
        self.child = Some(child);

        tracing::info!("MCP server started: {}", self.config.name);
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            child.kill().await?;
        }
        tracing::info!("MCP server stopped: {}", self.config.name);
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.child.as_ref().map_or(false, |c| c.id().is_some())
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }
}

pub struct McpServerManager {
    servers: HashMap<String, McpServer>,
    workspace_dir: PathBuf,
}

impl McpServerManager {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self {
            servers: HashMap::new(),
            workspace_dir,
        }
    }

    pub fn register(&mut self, config: McpServerConfig) {
        let server = McpServer::new(config, self.workspace_dir.clone());
        self.servers.insert(server.name().to_string(), server);
    }

    pub async fn start_all(&mut self) -> Result<()> {
        for server in self.servers.values_mut() {
            server.start().await?;
        }
        Ok(())
    }

    pub async fn stop_all(&mut self) -> Result<()> {
        for server in self.servers.values_mut() {
            server.stop().await?;
        }
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&McpServer> {
        self.servers.get(name)
    }

    pub fn list(&self) -> Vec<&str> {
        self.servers.keys().map(|s| s.as_str()).collect()
    }

    pub fn running(&self) -> Vec<&str> {
        self.servers
            .values()
            .filter(|s| s.is_running())
            .map(|s| s.name())
            .collect()
    }
}

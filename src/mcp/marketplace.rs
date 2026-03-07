use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::commands::McpCommands;

const CLAUDE_MCP_REGISTRY_URL: &str =
    "https://github.com/anthropics/mcp-servers/raw/main/registry.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPackage {
    pub name: String,
    pub description: String,
    pub source: McpSource,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<std::collections::HashMap<String, String>>,
    pub installed: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpSource {
    Npm { package: String },
    Git { url: String, branch: Option<String> },
    Local { path: PathBuf },
    ClaudeOfficial { name: String },
}

impl McpSource {
    pub fn is_npm(&self) -> bool {
        matches!(self, McpSource::Npm { .. })
    }

    pub fn is_git(&self) -> bool {
        matches!(self, McpSource::Git { .. })
    }
}

#[derive(Debug, Deserialize)]
struct McpRegistry {
    servers: std::collections::HashMap<String, McpServerEntry>,
}

#[derive(Debug, Deserialize)]
struct McpServerEntry {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    args: Option<Vec<String>>,
    #[serde(default)]
    env: Option<std::collections::HashMap<String, String>>,
}

pub struct McpMarketplace {
    workspace_dir: PathBuf,
    cache_dir: PathBuf,
}

impl McpMarketplace {
    pub fn new(workspace_dir: &Path) -> Self {
        Self {
            workspace_dir: workspace_dir.to_path_buf(),
            cache_dir: workspace_dir.join(".housaky").join("mcp"),
        }
    }

    fn ensure_cache_dir(&self) -> Result<PathBuf> {
        let dir = self.cache_dir.clone();
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn fetch_registry(&self) -> Result<std::collections::HashMap<String, McpServerEntry>> {
        let cache_dir = self.ensure_cache_dir()?;
        let registry_path = cache_dir.join("registry.json");

        let content = if registry_path.exists() {
            std::fs::read_to_string(&registry_path)?
        } else {
            let resp = reqwest::blocking::get(CLAUDE_MCP_REGISTRY_URL)
                .with_context(|| "Failed to fetch MCP registry")?;
            let text = resp
                .text()
                .with_context(|| "Failed to read registry response")?;
            std::fs::write(&registry_path, &text)?;
            text
        };

        let registry: McpRegistry =
            serde_json::from_str(&content).with_context(|| "Failed to parse MCP registry")?;

        Ok(registry.servers)
    }

    pub fn list_available(&self) -> Result<Vec<McpPackage>> {
        let servers = self.fetch_registry()?;
        let mut packages = Vec::new();

        for (name, entry) in servers {
            packages.push(McpPackage {
                name: name.clone(),
                description: entry.description.unwrap_or_default(),
                source: McpSource::ClaudeOfficial { name },
                command: entry.command,
                args: entry.args,
                env: entry.env,
                installed: false,
                enabled: false,
            });
        }

        packages.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(packages)
    }

    pub fn list_installed(&self) -> Result<Vec<McpPackage>> {
        let mcp_dir = self.workspace_dir.join("mcp");
        if !mcp_dir.exists() {
            return Ok(vec![]);
        }

        let mut packages = Vec::new();
        for entry in std::fs::read_dir(&mcp_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let config_path = path.join("config.json");
            if !config_path.exists() {
                continue;
            }

            let content = std::fs::read_to_string(&config_path)?;
            if let Ok(pkg) = serde_json::from_str::<McpPackage>(&content) {
                packages.push(pkg);
            }
        }

        Ok(packages)
    }

    pub fn install(&self, name: &str) -> Result<String> {
        let servers = self.fetch_registry()?;
        let entry = servers
            .get(name)
            .ok_or_else(|| anyhow!("MCP server not found: {}", name))?;

        let mcp_dir = self.workspace_dir.join("mcp").join(name);
        std::fs::create_dir_all(&mcp_dir)?;

        let command = entry
            .command
            .clone()
            .or_else(|| Some("npx".to_string()))
            .unwrap_or_default();

        let package = McpPackage {
            name: name.to_string(),
            description: entry.description.clone().unwrap_or_default(),
            source: McpSource::ClaudeOfficial {
                name: name.to_string(),
            },
            command: Some(command),
            args: entry.args.clone(),
            env: entry.env.clone(),
            installed: true,
            enabled: true,
        };

        let config_path = mcp_dir.join("config.json");
        let config_json = serde_json::to_string_pretty(&package)?;
        std::fs::write(&config_path, config_json)?;

        if let Some(ref cmd) = package.command {
            if cmd == "npx" || cmd == "npm" {
                self.install_npm_deps(name, &mcp_dir)?;
            } else if cmd == "uvx" || cmd == "python" {
                self.install_python_deps(name, &mcp_dir)?;
            }
        }

        Ok(name.to_string())
    }

    fn install_npm_deps(&self, name: &str, dir: &Path) -> Result<()> {
        let package_json_path = dir.join("package.json");
        if !package_json_path.exists() {
            let pkg_json = serde_json::json!({
                "name": name,
                "type": "module",
                "dependencies": {}
            });
            std::fs::write(&package_json_path, serde_json::to_string_pretty(&pkg_json)?)?;
        }

        let output = Command::new("npm")
            .args(["install"])
            .current_dir(dir)
            .output()
            .with_context(|| "Failed to run npm install")?;

        if !output.status.success() {
            tracing::warn!(
                "npm install warning: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    fn install_python_deps(&self, name: &str, dir: &Path) -> Result<()> {
        if let Some(ref env) = self.get_mcp_server_env(name)? {
            if let Some(deps) = env.get("PYTHON_DEPS") {
                let output = Command::new("pip")
                    .args(["install"])
                    .arg(deps)
                    .output()
                    .with_context(|| "Failed to install Python deps")?;

                if !output.status.success() {
                    tracing::warn!(
                        "pip install warning: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
            }
        }
        Ok(())
    }

    fn get_mcp_server_env(
        &self,
        name: &str,
    ) -> Result<Option<std::collections::HashMap<String, String>>> {
        let servers = self.fetch_registry()?;
        let entry = servers
            .get(name)
            .ok_or_else(|| anyhow!("MCP server not found: {}", name))?;
        Ok(entry.env.clone())
    }

    pub fn uninstall(&self, name: &str) -> Result<()> {
        let mcp_dir = self.workspace_dir.join("mcp").join(name);
        if mcp_dir.exists() {
            std::fs::remove_dir_all(&mcp_dir)?;
        }
        Ok(())
    }

    pub fn enable(&self, name: &str) -> Result<()> {
        let config_path = self
            .workspace_dir
            .join("mcp")
            .join(name)
            .join("config.json");
        if !config_path.exists() {
            return Err(anyhow!("MCP not installed: {}", name));
        }

        let content = std::fs::read_to_string(&config_path)?;
        let mut package: McpPackage = serde_json::from_str(&content)?;
        package.enabled = true;

        std::fs::write(&config_path, serde_json::to_string_pretty(&package)?)?;
        Ok(())
    }

    pub fn disable(&self, name: &str) -> Result<()> {
        let config_path = self
            .workspace_dir
            .join("mcp")
            .join(name)
            .join("config.json");
        if !config_path.exists() {
            return Err(anyhow!("MCP not installed: {}", name));
        }

        let content = std::fs::read_to_string(&config_path)?;
        let mut package: McpPackage = serde_json::from_str(&content)?;
        package.enabled = false;

        std::fs::write(&config_path, serde_json::to_string_pretty(&package)?)?;
        Ok(())
    }
}

pub fn handle_mcp_command(
    command: McpCommands,
    workspace_dir: &std::path::Path,
) -> anyhow::Result<()> {
    let market = McpMarketplace::new(workspace_dir);

    match command {
        McpCommands::List => {
            let available = market.list_available()?;
            if available.is_empty() {
                println!("No MCPs available in marketplace.");
            } else {
                println!("Available MCPs ({}):", available.len());
                for mcp in &available {
                    println!("  {} — {}", mcp.name, mcp.description);
                }
            }
        }
        McpCommands::Installed => {
            let installed = market.list_installed()?;
            if installed.is_empty() {
                println!("No MCPs installed.");
            } else {
                println!("Installed MCPs ({}):", installed.len());
                for mcp in &installed {
                    let status = if mcp.enabled {
                        "✓ enabled"
                    } else {
                        "○ disabled"
                    };
                    println!("  {} — {} [{}]", mcp.name, mcp.description, status);
                }
            }
        }
        McpCommands::Install { name } => {
            println!("Installing MCP: {}", name);
            let installed = market.install(&name)?;
            println!("✓ Installed MCP: {}", installed);
        }
        McpCommands::Uninstall { name } => {
            println!("Uninstalling MCP: {}", name);
            market.uninstall(&name)?;
            println!("✓ Uninstalled MCP: {}", name);
        }
        McpCommands::Enable { name } => {
            market.enable(&name)?;
            println!("✓ Enabled MCP: {}", name);
        }
        McpCommands::Disable { name } => {
            market.disable(&name)?;
            println!("✓ Disabled MCP: {}", name);
        }
    }

    Ok(())
}

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::commands::McpCommands;

const CLAUDE_MCP_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/anthropics/mcp-servers/main/registry.json";

const FALLBACK_REGISTRY: &str = r#"{
    "filesystem": {"description": "Filesystem operations - read, write, search files", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-filesystem"]},
    "memory": {"description": "Persistent memory storage for context", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-memory"]},
    "brave-search": {"description": "Web search using Brave Search API", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-brave-search"]},
    "puppeteer": {"description": "Browser automation with Puppeteer", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-puppeteer"]},
    "sqlite": {"description": "SQLite database operations", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-sqlite"]},
    "github": {"description": "GitHub API integration", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-github"]},
    "git": {"description": "Git operations and repository management", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-git"]},
    "slack": {"description": "Slack API integration", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-slack"]},
    "google-maps": {"description": "Google Maps API for location services", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-google-maps"]},
    "fetch": {"description": "HTTP fetch capabilities", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-fetch"]},
    "sequential-thinking": {"description": "Structured reasoning and problem solving", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-sequential-thinking"]},
    "time": {"description": "Time and timezone utilities", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-time"]},
    "aws-kb-retrieval": {"description": "AWS Knowledge Base retrieval", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-aws-kb-retrieval"]},
    "everart": {"description": "AI image generation", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-everart"]},
    "postgres": {"description": "PostgreSQL database operations", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-postgres"]},
    "mongodb": {"description": "MongoDB database operations", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-mongodb"]},
    "redis": {"description": "Redis cache operations", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-redis"]},
    "sentry": {"description": "Sentry error tracking integration", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-sentry"]},
    "airtable": {"description": "Airtable API integration", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-airtable"]},
    "notion": {"description": "Notion API integration", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-notion"]}
}"#;

const POPULAR_MCPS: &[(&str, &str, &str)] = &[
    (
        "filesystem",
        "Filesystem operations - read, write, search files",
        "npx",
    ),
    ("memory", "Persistent memory storage for context", "npx"),
    ("brave-search", "Web search using Brave Search API", "npx"),
    ("puppeteer", "Browser automation with Puppeteer", "npx"),
    ("sqlite", "SQLite database operations", "npx"),
    ("github", "GitHub API integration", "npx"),
    ("git", "Git operations and repository management", "npx"),
    ("slack", "Slack API integration", "npx"),
    (
        "google-maps",
        "Google Maps API for location services",
        "npx",
    ),
    ("fetch", "HTTP fetch capabilities", "npx"),
    (
        "sequential-thinking",
        "Structured reasoning and problem solving",
        "npx",
    ),
    ("time", "Time and timezone utilities", "npx"),
    ("aws-kb-retrieval", "AWS Knowledge Base retrieval", "npx"),
    ("everart", "AI image generation", "npx"),
];

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

#[derive(Debug, Deserialize, Clone)]
pub struct McpServerEntry {
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

        // Try cache first
        if registry_path.exists() {
            if let Ok(cached) = std::fs::read_to_string(&registry_path) {
                if !cached.trim().is_empty() {
                    if let Ok(registry) = serde_json::from_str::<McpRegistry>(&cached) {
                        return Ok(registry.servers);
                    }
                }
            }
        }

        // Try network
        if let Ok(content) = self.fetch_from_network(&registry_path) {
            if let Ok(registry) = serde_json::from_str::<McpRegistry>(&content) {
                return Ok(registry.servers);
            }
        }

        // Use bundled fallback
        let registry: McpRegistry = serde_json::from_str(FALLBACK_REGISTRY)
            .with_context(|| "Failed to parse bundled fallback registry")?;

        tracing::info!("Using bundled MCP registry fallback");
        Ok(registry.servers)
    }

    fn fetch_from_network(&self, registry_path: &Path) -> Result<String> {
        match reqwest::blocking::get(CLAUDE_MCP_REGISTRY_URL) {
            Ok(resp) => {
                if resp.status().is_success() {
                    let text = resp
                        .text()
                        .with_context(|| "Failed to read registry response")?;
                    let _ = std::fs::write(registry_path, &text);
                    Ok(text)
                } else {
                    Err(anyhow!("Failed to fetch registry: HTTP {}", resp.status()))
                }
            }
            Err(e) => {
                // Try to return cached content if available
                if registry_path.exists() {
                    let cached = std::fs::read_to_string(registry_path)?;
                    if !cached.trim().is_empty() {
                        return Ok(cached);
                    }
                }
                Err(anyhow!(
                    "Network error fetching registry: {}. No cached data available.",
                    e
                ))
            }
        }
    }

    pub fn list_available(&self) -> Result<Vec<McpPackage>> {
        let mut packages = Vec::new();
        let mut seen_names = std::collections::HashSet::new();

        // Try to fetch from registry
        match self.fetch_registry() {
            Ok(servers) => {
                for (name, entry) in servers {
                    seen_names.insert(name.clone());
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
            }
            Err(e) => {
                tracing::warn!("Could not fetch MCP registry: {}", e);
            }
        }

        // Add popular MCPs that might not be in the registry
        for (name, desc, cmd) in POPULAR_MCPS {
            if !seen_names.contains(*name) {
                packages.push(McpPackage {
                    name: name.to_string(),
                    description: desc.to_string(),
                    source: McpSource::Npm {
                        package: format!("@modelcontextprotocol/server-{}", name),
                    },
                    command: Some(cmd.to_string()),
                    args: Some(vec![
                        "-y".to_string(),
                        format!("@modelcontextprotocol/server-{}", name),
                    ]),
                    env: None,
                    installed: false,
                    enabled: false,
                });
            }
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
        // Try registry first
        let entry = self
            .fetch_registry()
            .ok()
            .and_then(|servers| servers.get(name).cloned());

        let mcp_dir = self.workspace_dir.join("mcp").join(name);
        std::fs::create_dir_all(&mcp_dir)?;

        let (description, command, args, env) = if let Some(e) = entry {
            (
                e.description.unwrap_or_default(),
                e.command.clone().unwrap_or_else(|| "npx".to_string()),
                e.args.clone(),
                e.env.clone(),
            )
        } else {
            // Check if it's a popular MCP
            let popular = POPULAR_MCPS.iter().find(|(n, _, _)| *n == name);
            if let Some((_, desc, cmd)) = popular {
                (
                    desc.to_string(),
                    cmd.to_string(),
                    Some(vec![
                        "-y".to_string(),
                        format!("@modelcontextprotocol/server-{}", name),
                    ]),
                    None,
                )
            } else {
                // Default to npm package
                (
                    format!("MCP server: {}", name),
                    "npx".to_string(),
                    Some(vec![
                        "-y".to_string(),
                        format!("@modelcontextprotocol/server-{}", name),
                    ]),
                    None,
                )
            }
        };

        let package = McpPackage {
            name: name.to_string(),
            description,
            source: McpSource::ClaudeOfficial {
                name: name.to_string(),
            },
            command: Some(command.clone()),
            args,
            env,
            installed: true,
            enabled: true,
        };

        let config_path = mcp_dir.join("config.json");
        let config_json = serde_json::to_string_pretty(&package)?;
        std::fs::write(&config_path, config_json)?;

        if command == "npx" || command == "npm" {
            self.install_npm_deps(name, &mcp_dir)?;
        } else if command == "uvx" || command == "python" {
            self.install_python_deps(name, &mcp_dir)?;
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

    fn install_python_deps(&self, name: &str, _dir: &Path) -> Result<()> {
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

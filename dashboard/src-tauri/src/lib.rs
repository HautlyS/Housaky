use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HousakyStatus {
    pub version: String,
    pub workspace: String,
    pub config: String,
    pub provider: String,
    pub model: String,
    pub temperature: f64,
    pub memory_backend: String,
    pub memory_auto_save: bool,
    pub embedding_provider: String,
    pub autonomy_level: String,
    pub workspace_only: bool,
    pub runtime: String,
    pub heartbeat_enabled: bool,
    pub heartbeat_interval: i32,
    pub channels: HashMap<String, ChannelStatus>,
    pub secrets_encrypted: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChannelStatus {
    pub configured: bool,
    pub active: bool,
    pub allowlist_count: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HousakyConfig {
    pub api_key: Option<String>,
    pub default_provider: String,
    pub default_model: Option<String>,
    pub default_temperature: f64,
    pub memory: MemoryConfig,
    pub autonomy: AutonomyConfig,
    pub runtime: RuntimeConfig,
    pub heartbeat: HeartbeatConfig,
    pub gateway: GatewayConfig,
    pub tunnel: TunnelConfig,
    pub secrets: SecretsConfig,
    pub channels_config: ChannelsConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MemoryConfig {
    pub backend: String,
    pub auto_save: bool,
    pub embedding_provider: String,
    pub vector_weight: f64,
    pub keyword_weight: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AutonomyConfig {
    pub level: String,
    pub workspace_only: bool,
    pub allowed_commands: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub max_actions_per_hour: i32,
    pub max_cost_per_day_cents: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RuntimeConfig {
    pub kind: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HeartbeatConfig {
    pub enabled: bool,
    pub interval_minutes: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GatewayConfig {
    pub require_pairing: bool,
    pub allow_public_bind: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TunnelConfig {
    pub provider: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SecretsConfig {
    pub encrypt: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ChannelsConfig {
    pub telegram: Option<TelegramConfig>,
    pub discord: Option<DiscordConfig>,
    pub slack: Option<SlackConfig>,
    pub whatsapp: Option<WhatsAppConfig>,
    pub matrix: Option<MatrixConfig>,
    pub webhook: Option<WebhookConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TelegramConfig {
    pub bot_token: Option<String>,
    pub allowed_users: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DiscordConfig {
    pub bot_token: Option<String>,
    pub allowed_user_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SlackConfig {
    pub bot_token: Option<String>,
    pub allowed_user_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct WhatsAppConfig {
    pub access_token: Option<String>,
    pub phone_number_id: Option<String>,
    pub verify_token: Option<String>,
    pub allowed_numbers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MatrixConfig {
    pub homeserver: Option<String>,
    pub user_id: Option<String>,
    pub password: Option<String>,
    pub room_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct WebhookConfig {
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub tools_count: i32,
    pub enabled: bool,
    pub location: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub channel_type: String,
    pub configured: bool,
    pub active: bool,
    pub allowlist_count: i32,
    pub last_activity: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Integration {
    pub name: String,
    pub description: String,
    pub category: String,
    pub status: String,
}

fn get_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".housaky")
        .join("config.toml")
}

fn get_keys_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".housaky")
        .join("keys.json")
}

fn get_workspace_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".housaky")
        .join("workspace")
}

fn run_housaky_command(args: &[&str]) -> Result<String, String> {
    let output = Command::new("housaky")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run housaky: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.is_empty() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(stderr.to_string())
        }
    }
}

fn parse_toml_config(content: &str) -> HousakyConfig {
    let mut config = HousakyConfig::default();
    config.default_provider = "openrouter".to_string();
    config.default_temperature = 0.7;
    config.memory = MemoryConfig {
        backend: "sqlite".to_string(),
        auto_save: true,
        embedding_provider: "openai".to_string(),
        vector_weight: 0.7,
        keyword_weight: 0.3,
    };
    config.autonomy = AutonomyConfig {
        level: "supervised".to_string(),
        workspace_only: true,
        allowed_commands: vec![],
        forbidden_paths: vec![],
        max_actions_per_hour: 100,
        max_cost_per_day_cents: 1000,
    };
    config.runtime = RuntimeConfig {
        kind: "native".to_string(),
    };
    config.heartbeat = HeartbeatConfig {
        enabled: false,
        interval_minutes: 30,
    };
    config.gateway = GatewayConfig {
        require_pairing: true,
        allow_public_bind: false,
    };
    config.tunnel = TunnelConfig {
        provider: "none".to_string(),
    };
    config.secrets = SecretsConfig { encrypt: true };

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("api_key") {
            if let Some(key) = line.split('=').nth(1) {
                let val = key.trim().trim_matches('"');
                if !val.is_empty() {
                    config.api_key = Some(val.to_string());
                }
            }
        } else if line.starts_with("default_provider") {
            if let Some(val) = line.split('=').nth(1) {
                config.default_provider = val.trim().trim_matches('"').to_string();
            }
        } else if line.starts_with("default_model") {
            if let Some(val) = line.split('=').nth(1) {
                let m = val.trim().trim_matches('"');
                if !m.is_empty() {
                    config.default_model = Some(m.to_string());
                }
            }
        } else if line.starts_with("default_temperature") {
            if let Some(val) = line.split('=').nth(1) {
                if let Ok(t) = val.trim().parse::<f64>() {
                    config.default_temperature = t;
                }
            }
        } else if line.starts_with("backend") && line.contains("memory") {
            if let Some(val) = line.split('=').nth(1) {
                config.memory.backend = val.trim().trim_matches('"').to_string();
            }
        } else if line.starts_with("auto_save") {
            if let Some(val) = line.split('=').nth(1) {
                config.memory.auto_save = val.trim() == "true";
            }
        } else if line.starts_with("embedding_provider") {
            if let Some(val) = line.split('=').nth(1) {
                config.memory.embedding_provider = val.trim().trim_matches('"').to_string();
            }
        } else if line.starts_with("vector_weight") {
            if let Some(val) = line.split('=').nth(1) {
                if let Ok(v) = val.trim().parse::<f64>() {
                    config.memory.vector_weight = v;
                }
            }
        } else if line.starts_with("keyword_weight") {
            if let Some(val) = line.split('=').nth(1) {
                if let Ok(v) = val.trim().parse::<f64>() {
                    config.memory.keyword_weight = v;
                }
            }
        } else if line.starts_with("level") && line.contains("autonomy") {
            if let Some(val) = line.split('=').nth(1) {
                config.autonomy.level = val.trim().trim_matches('"').to_string();
            }
        } else if line.starts_with("workspace_only") {
            if let Some(val) = line.split('=').nth(1) {
                config.autonomy.workspace_only = val.trim() == "true";
            }
        } else if line.starts_with("allowed_commands") {
            if let Some(val) = line.split('=').nth(1) {
                let cmds = val.trim().trim_matches(|c| c == '[' || c == ']');
                config.autonomy.allowed_commands = cmds
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        } else if line.starts_with("kind") && line.contains("runtime") {
            if let Some(val) = line.split('=').nth(1) {
                config.runtime.kind = val.trim().trim_matches('"').to_string();
            }
        } else if line.starts_with("enabled") && line.contains("heartbeat") {
            if let Some(val) = line.split('=').nth(1) {
                config.heartbeat.enabled = val.trim() == "true";
            }
        } else if line.starts_with("interval_minutes") {
            if let Some(val) = line.split('=').nth(1) {
                if let Ok(i) = val.trim().parse::<i32>() {
                    config.heartbeat.interval_minutes = i;
                }
            }
        } else if line.starts_with("require_pairing") {
            if let Some(val) = line.split('=').nth(1) {
                config.gateway.require_pairing = val.trim() == "true";
            }
        } else if line.starts_with("allow_public_bind") {
            if let Some(val) = line.split('=').nth(1) {
                config.gateway.allow_public_bind = val.trim() == "true";
            }
        } else if line.starts_with("provider") && line.contains("tunnel") {
            if let Some(val) = line.split('=').nth(1) {
                config.tunnel.provider = val.trim().trim_matches('"').to_string();
            }
        } else if line.starts_with("encrypt") && line.contains("secrets") {
            if let Some(val) = line.split('=').nth(1) {
                config.secrets.encrypt = val.trim() == "true";
            }
        }
    }

    config
}

fn config_to_toml(config: &HousakyConfig) -> String {
    let mut toml = String::new();
    
    if let Some(key) = &config.api_key {
        if !key.is_empty() {
            toml.push_str(&format!("api_key = \"{}\"\n", key));
        }
    }
    
    toml.push_str(&format!("default_provider = \"{}\"\n", config.default_provider));
    
    if let Some(model) = &config.default_model {
        if !model.is_empty() {
            toml.push_str(&format!("default_model = \"{}\"\n", model));
        }
    }
    
    toml.push_str(&format!("default_temperature = {}\n", config.default_temperature));
    
    toml.push_str("\n[memory]\n");
    toml.push_str(&format!("backend = \"{}\"\n", config.memory.backend));
    toml.push_str(&format!("auto_save = {}\n", config.memory.auto_save));
    toml.push_str(&format!("embedding_provider = \"{}\"\n", config.memory.embedding_provider));
    toml.push_str(&format!("vector_weight = {}\n", config.memory.vector_weight));
    toml.push_str(&format!("keyword_weight = {}\n", config.memory.keyword_weight));
    
    toml.push_str("\n[autonomy]\n");
    toml.push_str(&format!("level = \"{}\"\n", config.autonomy.level));
    toml.push_str(&format!("workspace_only = {}\n", config.autonomy.workspace_only));
    
    if !config.autonomy.allowed_commands.is_empty() {
        toml.push_str("allowed_commands = [");
        toml.push_str(&config.autonomy.allowed_commands
            .iter()
            .map(|s| format!("\"{}\"", s))
            .collect::<Vec<_>>()
            .join(", "));
        toml.push_str("]\n");
    }
    
    if !config.autonomy.forbidden_paths.is_empty() {
        toml.push_str("forbidden_paths = [");
        toml.push_str(&config.autonomy.forbidden_paths
            .iter()
            .map(|s| format!("\"{}\"", s))
            .collect::<Vec<_>>()
            .join(", "));
        toml.push_str("]\n");
    }
    
    toml.push_str(&format!("max_actions_per_hour = {}\n", config.autonomy.max_actions_per_hour));
    toml.push_str(&format!("max_cost_per_day_cents = {}\n", config.autonomy.max_cost_per_day_cents));
    
    toml.push_str("\n[runtime]\n");
    toml.push_str(&format!("kind = \"{}\"\n", config.runtime.kind));
    
    toml.push_str("\n[heartbeat]\n");
    toml.push_str(&format!("enabled = {}\n", config.heartbeat.enabled));
    toml.push_str(&format!("interval_minutes = {}\n", config.heartbeat.interval_minutes));
    
    toml.push_str("\n[gateway]\n");
    toml.push_str(&format!("require_pairing = {}\n", config.gateway.require_pairing));
    toml.push_str(&format!("allow_public_bind = {}\n", config.gateway.allow_public_bind));
    
    toml.push_str("\n[tunnel]\n");
    toml.push_str(&format!("provider = \"{}\"\n", config.tunnel.provider));
    
    toml.push_str("\n[secrets]\n");
    toml.push_str(&format!("encrypt = {}\n", config.secrets.encrypt));
    
    toml
}

#[tauri::command]
async fn get_status() -> Result<HousakyStatus, String> {
    let config_path = get_config_path();
    let workspace_path = get_workspace_path();

    let version = match run_housaky_command(&["--version"]) {
        Ok(v) => v.trim().to_string(),
        Err(_) => "0.1.0".to_string(),
    };

    let mut channels: HashMap<String, ChannelStatus> = HashMap::new();
    
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config: {}", e))?;
        
        for channel_type in &["telegram", "discord", "slack", "whatsapp", "matrix", "webhook"] {
            let section = format!("[channels_config.{}]", channel_type);
            let configured = content.contains(&section);
            channels.insert(
                channel_type.to_string(),
                ChannelStatus {
                    configured,
                    active: false,
                    allowlist_count: 0,
                },
            );
        }
    } else {
        channels.insert("cli".to_string(), ChannelStatus { configured: true, active: true, allowlist_count: 0 });
    }

    let status = HousakyStatus {
        version,
        workspace: workspace_path.to_string_lossy().to_string(),
        config: config_path.to_string_lossy().to_string(),
        provider: "openrouter".to_string(),
        model: "(default)".to_string(),
        temperature: 0.7,
        memory_backend: "sqlite".to_string(),
        memory_auto_save: true,
        embedding_provider: "openai".to_string(),
        autonomy_level: "supervised".to_string(),
        workspace_only: true,
        runtime: "native".to_string(),
        heartbeat_enabled: false,
        heartbeat_interval: 30,
        channels,
        secrets_encrypted: true,
    };

    Ok(status)
}

// ── Market / Skills Marketplace ───────────────────────────────────────────────

#[derive(serde::Serialize, Clone)]
pub struct MarketSkill {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub tags: Vec<String>,
    pub tools_count: u32,
    pub source: String,
    pub installed: bool,
    pub enabled: bool,
}

#[tauri::command]
async fn get_marketplace_skills() -> Result<Vec<MarketSkill>, String> {
    let workspace_path = get_workspace_path();
    let skills_dir = workspace_path.join("skills");
    let openclaw_dir = workspace_path.join(".housaky").join("openclaw").join("skills");
    
    let mut skills = Vec::new();
    let mut installed_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    
    // Load installed skills
    if skills_dir.exists() {
        if let Ok(entries) = fs::read_dir(&skills_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    installed_names.insert(name.clone());
                    skills.push(MarketSkill {
                        name: name.clone(),
                        description: format!("Installed skill: {}", name),
                        version: "1.0.0".to_string(),
                        author: "Local".to_string(),
                        tags: vec!["installed".to_string()],
                        tools_count: 0,
                        source: "local".to_string(),
                        installed: true,
                        enabled: true,
                    });
                }
            }
        }
    }
    
    // Add OpenClaw vendored skills as available
    if openclaw_dir.exists() {
        if let Ok(entries) = fs::read_dir(&openclaw_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();
                    
                    if !installed_names.contains(&name) {
                        // Check for SKILL.md
                        let skill_md = path.join("SKILL.md");
                        let description = if skill_md.exists() {
                            fs::read_to_string(&skill_md)
                                .ok()
                                .and_then(|c| c.lines().skip(2).next().map(|s| s.to_string()))
                                .unwrap_or_else(|| "No description".to_string())
                        } else {
                            format!("OpenClaw skill: {}", name)
                        };
                        
                        skills.push(MarketSkill {
                            name,
                            description,
                            version: "1.0.0".to_string(),
                            author: "OpenClaw".to_string(),
                            tags: vec!["openclaw".to_string(), "vendor".to_string()],
                            tools_count: 0,
                            source: "openclaw".to_string(),
                            installed: false,
                            enabled: false,
                        });
                    }
                }
            }
        }
    }
    
    // Add sample marketplace skills if no others found
    if skills.is_empty() {
        skills.extend(vec![
            MarketSkill {
                name: "Git Helper".to_string(),
                description: "Automates git workflows, commits, and PR creation".to_string(),
                version: "1.2.0".to_string(),
                author: "Housaky Team".to_string(),
                tags: vec!["git".to_string(), "automation".to_string(), "vcs".to_string()],
                tools_count: 5,
                source: "marketplace".to_string(),
                installed: false,
                enabled: false,
            },
            MarketSkill {
                name: "Web Scraper".to_string(),
                description: "Extract data from websites using CSS selectors".to_string(),
                version: "0.8.0".to_string(),
                author: "Community".to_string(),
                tags: vec!["scraping".to_string(), "data".to_string(), "web".to_string()],
                tools_count: 3,
                source: "marketplace".to_string(),
                installed: false,
                enabled: false,
            },
            MarketSkill {
                name: "Database Helper".to_string(),
                description: "SQL query builder and database management".to_string(),
                version: "1.0.0".to_string(),
                author: "Housaky Team".to_string(),
                tags: vec!["database".to_string(), "sql".to_string(), "data".to_string()],
                tools_count: 4,
                source: "marketplace".to_string(),
                installed: false,
                enabled: false,
            },
            MarketSkill {
                name: "Docker Manager".to_string(),
                description: "Container management and deployment automation".to_string(),
                version: "0.9.0".to_string(),
                author: "Community".to_string(),
                tags: vec!["docker".to_string(), "devops".to_string(), "containers".to_string()],
                tools_count: 6,
                source: "marketplace".to_string(),
                installed: false,
                enabled: false,
            },
            MarketSkill {
                name: "API Tester".to_string(),
                description: "REST API testing and debugging tool".to_string(),
                version: "1.1.0".to_string(),
                author: "Housaky Team".to_string(),
                tags: vec!["api".to_string(), "testing".to_string(), "http".to_string()],
                tools_count: 4,
                source: "marketplace".to_string(),
                installed: false,
                enabled: false,
            },
        ]);
    }
    
    Ok(skills)
}

#[tauri::command]
async fn install_market_skill(skill_name: String, target_agent: Option<String>) -> Result<String, String> {
    log::info!("Installing skill '{}' for agent: {:?}", skill_name, target_agent);
    
    // Run housaky command to install skill
    let result = run_housaky_command(&["skills", "install", &skill_name]);
    
    match result {
        Ok(output) => Ok(format!("Skill '{}' installed successfully", skill_name)),
        Err(e) => {
            // Still return success if it's already installed
            if e.contains("already") || e.contains("exists") {
                return Ok(format!("Skill '{}' is already installed", skill_name));
            }
            Err(format!("Failed to install skill: {}", e))
        }
    }
}

#[tauri::command]
async fn uninstall_skill(skill_name: String) -> Result<String, String> {
    log::info!("Uninstalling skill '{}'", skill_name);
    let workspace_path = get_workspace_path();
    let skill_path = workspace_path.join("skills").join(&skill_name);
    
    if skill_path.exists() {
        fs::remove_dir_all(&skill_path)
            .map_err(|e| format!("Failed to remove skill: {}", e))?;
        Ok(format!("Skill '{}' uninstalled", skill_name))
    } else {
        Err(format!("Skill '{}' not found", skill_name))
    }
}

// ── MCP Servers ───────────────────────────────────────────────────────────────

#[derive(serde::Serialize, Clone)]
pub struct McpServer {
    pub name: String,
    pub description: String,
    pub command: String,
    pub args: Vec<String>,
    pub enabled: bool,
    pub status: String,
    pub connected_count: u32,
}

#[tauri::command]
async fn get_mcp_servers() -> Result<Vec<McpServer>, String> {
    let workspace_path = get_workspace_path();
    let mcp_config_path = workspace_path.join(".housaky").join("mcp.json");
    
    let mut servers = Vec::new();
    
    // Try to read MCP config
    if mcp_config_path.exists() {
        if let Ok(data) = fs::read_to_string(&mcp_config_path) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&data) {
                if let Some(svrs) = config.get("servers").and_then(|s| s.as_array()) {
                    for srv in svrs {
                        servers.push(McpServer {
                            name: srv.get("name").and_then(|n| n.as_str()).unwrap_or("unknown").to_string(),
                            description: srv.get("description").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                            command: srv.get("command").and_then(|c| c.as_str()).unwrap_or("").to_string(),
                            args: srv.get("args").and_then(|a| a.as_array())
                                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                                .unwrap_or_default(),
                            enabled: srv.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true),
                            status: "configured".to_string(),
                            connected_count: 0,
                        });
                    }
                }
            }
        }
    }
    
    // Add sample MCPs if none configured
    if servers.is_empty() {
        servers.extend(vec![
            McpServer {
                name: "Filesystem".to_string(),
                description: "Read, write, and manage files on the system".to_string(),
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string(), "/".to_string()],
                enabled: true,
                status: "stopped".to_string(),
                connected_count: 0,
            },
            McpServer {
                name: "Brave Search".to_string(),
                description: "Web search using Brave Search API".to_string(),
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "@modelcontextprotocol/server-brave-search".to_string()],
                enabled: true,
                status: "stopped".to_string(),
                connected_count: 0,
            },
            McpServer {
                name: "GitHub".to_string(),
                description: "GitHub API integration for repos and issues".to_string(),
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "@modelcontextprotocol/server-github".to_string()],
                enabled: false,
                status: "stopped".to_string(),
                connected_count: 0,
            },
            McpServer {
                name: "PostgreSQL".to_string(),
                description: "Database operations for PostgreSQL".to_string(),
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "@modelcontextprotocol/server-postgres".to_string()],
                enabled: false,
                status: "stopped".to_string(),
                connected_count: 0,
            },
        ]);
    }
    
    Ok(servers)
}

#[tauri::command]
async fn configure_mcp_server(name: String, enabled: bool, command: String, args: Vec<String>) -> Result<String, String> {
    log::info!("Configuring MCP server '{}': enabled={}", name, enabled);
    
    let workspace_path = get_workspace_path();
    let mcp_config_path = workspace_path.join(".housaky").join("mcp.json");
    
    // Read existing config or create new
    let mut config: serde_json::Value = if mcp_config_path.exists() {
        serde_json::from_str(&fs::read_to_string(&mcp_config_path).unwrap_or_default()).unwrap_or(serde_json::json!({"servers": []}))
    } else {
        serde_json::json!({"servers": []})
    };
    
    let servers = match config.get_mut("servers") {
        Some(s) => s,
        None => {
            config["servers"] = serde_json::json!([]);
            config.get_mut("servers").unwrap()
        }
    };
    
    // Find and update or add server
    let mut found = false;
    if let Some(arr) = servers.as_array_mut() {
        for srv in arr.iter_mut() {
            let srv_name = srv.get("name").and_then(|n: &serde_json::Value| n.as_str());
            if srv_name == Some(&name) {
                srv["enabled"] = serde_json::json!(enabled);
                srv["command"] = serde_json::json!(command);
                srv["args"] = serde_json::json!(args);
                found = true;
                break;
            }
        }
        if !found {
            arr.push(serde_json::json!({
                "name": name,
                "command": command,
                "args": args,
                "enabled": enabled,
                "description": ""
            }));
        }
    }
    
    // Ensure directory exists
    if let Some(parent) = mcp_config_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    
    fs::write(&mcp_config_path, serde_json::to_string_pretty(&config).unwrap())
        .map_err(|e| format!("Failed to write MCP config: {}", e))?;
    
    Ok(format!("MCP server '{}' configured", name))
}

#[tauri::command]
async fn get_config() -> Result<HousakyConfig, String> {
    let config_path = get_config_path();
    
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config: {}", e))?;
        Ok(parse_toml_config(&content))
    } else {
        Ok(HousakyConfig::default())
    }
}

#[tauri::command]
async fn save_config(config: HousakyConfig) -> Result<String, String> {
    let config_path = get_config_path();
    
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    
    let toml_content = config_to_toml(&config);
    fs::write(&config_path, toml_content)
        .map_err(|e| format!("Failed to write config: {}", e))?;
    
    log::info!("Config saved to {:?}", config_path);
    Ok(config_path.to_string_lossy().to_string())
}

#[tauri::command]
async fn send_message(message: String) -> Result<String, String> {
    run_housaky_command(&["agent", "-m", &message])
}

#[tauri::command]
async fn run_housaky_command_cmd(command: String, args: Vec<String>) -> Result<String, String> {
    let mut all_args: Vec<&str> = vec![&command];
    for arg in &args {
        all_args.push(arg);
    }
    run_housaky_command(&all_args)
}

#[tauri::command]
async fn get_skills() -> Result<Vec<Skill>, String> {
    let workspace_path = get_workspace_path();
    let skills_dir = workspace_path.join("skills");
    
    let mut skills = Vec::new();
    
    if skills_dir.exists() {
        if let Ok(entries) = fs::read_dir(&skills_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    skills.push(Skill {
                        name: name.clone(),
                        description: format!("Skill: {}", name),
                        version: "0.1.0".to_string(),
                        author: None,
                        tags: vec![],
                        tools_count: 0,
                        enabled: true,
                        location: Some(entry.path().to_string_lossy().to_string()),
                    });
                }
            }
        }
    }
    
    skills.extend(vec![
        Skill {
            name: "Git Helper".to_string(),
            description: "Automates git workflows, commits, and PR creation".to_string(),
            version: "1.2.0".to_string(),
            author: Some("Housaky Team".to_string()),
            tags: vec!["git".to_string(), "automation".to_string()],
            tools_count: 5,
            enabled: true,
            location: None,
        },
        Skill {
            name: "Web Scraper".to_string(),
            description: "Extract data from websites using CSS selectors".to_string(),
            version: "0.8.0".to_string(),
            author: Some("Community".to_string()),
            tags: vec!["scraping".to_string(), "data".to_string()],
            tools_count: 3,
            enabled: true,
            location: None,
        },
    ]);
    
    Ok(skills)
}

#[tauri::command]
async fn toggle_skill(name: String, enabled: bool) -> Result<String, String> {
    log::info!("Toggle skill {}: {}", name, enabled);
    Ok(format!("Skill {} {}", name, if enabled { "enabled" } else { "disabled" }))
}

#[tauri::command]
async fn get_channels() -> Result<Vec<Channel>, String> {
    let config_path = get_config_path();
    let mut channels = Vec::new();
    
    channels.push(Channel {
        id: "cli".to_string(),
        name: "CLI".to_string(),
        channel_type: "cli".to_string(),
        configured: true,
        active: true,
        allowlist_count: 0,
        last_activity: None,
    });
    
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config: {}", e))?;
        
        let channel_types = vec![
            ("telegram", "Telegram", "Telegram Bot"),
            ("discord", "Discord", "Discord Server"),
            ("slack", "Slack", "Slack Workspace"),
            ("whatsapp", "WhatsApp", "WhatsApp Business"),
            ("matrix", "Matrix", "Matrix Room"),
            ("webhook", "Webhook", "Webhook Endpoint"),
        ];
        
        for (channel_type, name, _description) in channel_types {
            let section = format!("[channels_config.{}]", channel_type);
            let configured = content.contains(&section);
            
            channels.push(Channel {
                id: channel_type.to_string(),
                name: name.to_string(),
                channel_type: channel_type.to_string(),
                configured,
                active: false,
                allowlist_count: 0,
                last_activity: None,
            });
        }
    }
    
    Ok(channels)
}

#[tauri::command]
async fn configure_channel(channel_type: String, config: serde_json::Value) -> Result<String, String> {
    log::info!("Configure channel {}: {:?}", channel_type, config);
    let config_path = get_config_path();
    let mut toml_content = String::new();
    
    if config_path.exists() {
        toml_content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config: {}", e))?;
    }
    
    let section = format!("[channels_config.{}]", channel_type);
    
    if !toml_content.contains(&section) {
        toml_content.push_str(&format!("\n{}\n", section));
    }
    
    if let Some(obj) = config.as_object() {
        for (key, value) in obj {
            let line = match value {
                serde_json::Value::String(s) => format!("{} = \"{}\"\n", key, s),
                serde_json::Value::Bool(b) => format!("{} = {}\n", key, b),
                serde_json::Value::Number(n) => format!("{} = {}\n", key, n),
                serde_json::Value::Array(arr) => {
                    let items: Vec<String> = arr.iter()
                        .map(|v| format!("\"{}\"", v.as_str().unwrap_or("")))
                        .collect();
                    format!("{} = [{}]\n", key, items.join(", "))
                }
                _ => continue,
            };
            toml_content.push_str(&line);
        }
    }
    
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    
    fs::write(&config_path, &toml_content)
        .map_err(|e| format!("Failed to write config: {}", e))?;
    
    Ok(format!("Channel {} configured", channel_type))
}

#[tauri::command]
async fn start_channel(_channel_type: String) -> Result<String, String> {
    run_housaky_command(&["channel", "start"])
}

#[tauri::command]
async fn stop_channel(channel_type: String) -> Result<String, String> {
    log::info!("Stop channel requested: {}", channel_type);
    Ok(format!("Channel {} stop requested", channel_type))
}

#[tauri::command]
async fn check_housaky_installed() -> Result<bool, String> {
    match Command::new("housaky").arg("--version").output() {
        Ok(o) => Ok(o.status.success()),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
async fn get_integrations() -> Result<Vec<Integration>, String> {
    let integrations = vec![
        Integration { name: "Telegram".to_string(), description: "Bot API — long-polling".to_string(), category: "chat".to_string(), status: "available".to_string() },
        Integration { name: "Discord".to_string(), description: "Servers, channels & DMs".to_string(), category: "chat".to_string(), status: "available".to_string() },
        Integration { name: "Slack".to_string(), description: "Workspace apps via Web API".to_string(), category: "chat".to_string(), status: "available".to_string() },
        Integration { name: "WhatsApp".to_string(), description: "Meta Cloud API via webhook".to_string(), category: "chat".to_string(), status: "available".to_string() },
        Integration { name: "iMessage".to_string(), description: "macOS AppleScript bridge".to_string(), category: "chat".to_string(), status: "available".to_string() },
        Integration { name: "Matrix".to_string(), description: "Matrix protocol (Element)".to_string(), category: "chat".to_string(), status: "available".to_string() },
        Integration { name: "OpenRouter".to_string(), description: "200+ models, 1 API key".to_string(), category: "ai_model".to_string(), status: "active".to_string() },
        Integration { name: "Anthropic".to_string(), description: "Claude 3.5/4 Sonnet & Opus".to_string(), category: "ai_model".to_string(), status: "available".to_string() },
        Integration { name: "OpenAI".to_string(), description: "GPT-4o, GPT-5, o1".to_string(), category: "ai_model".to_string(), status: "available".to_string() },
        Integration { name: "Google".to_string(), description: "Gemini 2.5 Pro/Flash".to_string(), category: "ai_model".to_string(), status: "available".to_string() },
        Integration { name: "DeepSeek".to_string(), description: "DeepSeek V3 & R1".to_string(), category: "ai_model".to_string(), status: "available".to_string() },
        Integration { name: "Ollama".to_string(), description: "Local models".to_string(), category: "ai_model".to_string(), status: "available".to_string() },
        Integration { name: "Groq".to_string(), description: "Fast inference".to_string(), category: "ai_model".to_string(), status: "available".to_string() },
        Integration { name: "Mistral".to_string(), description: "Mistral AI models".to_string(), category: "ai_model".to_string(), status: "available".to_string() },
        Integration { name: "Composio".to_string(), description: "1000+ OAuth apps".to_string(), category: "tools_automation".to_string(), status: "available".to_string() },
        Integration { name: "Brave Search".to_string(), description: "Web search API".to_string(), category: "tools_automation".to_string(), status: "available".to_string() },
        Integration { name: "GitHub".to_string(), description: "Repo management".to_string(), category: "tools_automation".to_string(), status: "available".to_string() },
    ];
    
    Ok(integrations)
}

#[tauri::command]
async fn hardware_discover() -> Result<Vec<serde_json::Value>, String> {
    match run_housaky_command(&["hardware", "discover"]) {
        Ok(_output) => {
            let devices: Vec<serde_json::Value> = vec![];
            Ok(devices)
        }
        Err(e) => {
            log::warn!("Hardware discover failed: {}", e);
            Ok(vec![])
        }
    }
}

#[tauri::command]
async fn run_doctor() -> Result<String, String> {
    run_housaky_command(&["doctor"])
}

// ── AGI Telemetry ──────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct AgiTelemetry {
    total_tokens: u64,
    total_cost: f64,
    total_requests: u64,
    avg_latency_ms: u64,
    tokens_per_sec: f64,
    provider: String,
    model: String,
}

#[tauri::command]
async fn get_agi_telemetry() -> Result<AgiTelemetry, String> {
    // Try to read telemetry from housaky telemetry-id / stats file
    let workspace = get_workspace_path();
    let stats_path = workspace.join(".housaky").join("telemetry.json");
    if stats_path.exists() {
        if let Ok(data) = std::fs::read_to_string(&stats_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                return Ok(AgiTelemetry {
                    total_tokens:   v["total_tokens"].as_u64().unwrap_or(0),
                    total_cost:     v["total_cost"].as_f64().unwrap_or(0.0),
                    total_requests: v["total_requests"].as_u64().unwrap_or(0),
                    avg_latency_ms: v["avg_latency_ms"].as_u64().unwrap_or(0),
                    tokens_per_sec: v["tokens_per_sec"].as_f64().unwrap_or(0.0),
                    provider:       v["provider"].as_str().unwrap_or("").to_string(),
                    model:          v["model"].as_str().unwrap_or("").to_string(),
                });
            }
        }
    }
    // Fallback: parse from get_status
    let status = get_status().await?;
    Ok(AgiTelemetry {
        total_tokens:   0,
        total_cost:     0.0,
        total_requests: 0,
        avg_latency_ms: 0,
        tokens_per_sec: 0.0,
        provider:       status.provider,
        model:          status.model,
    })
}

// ── Agent Thoughts ─────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct AgentThought {
    role:      String,
    content:   String,
    timestamp: String,
    metadata:  Option<String>,
}

#[tauri::command]
async fn get_agent_thoughts() -> Result<Vec<AgentThought>, String> {
    let workspace = get_workspace_path();
    let log_path = workspace.join(".housaky").join("thoughts.jsonl");
    if !log_path.exists() {
        return Ok(vec![]);
    }
    let data = std::fs::read_to_string(&log_path).map_err(|e| e.to_string())?;
    let mut thoughts = Vec::new();
    for line in data.lines().rev().take(50) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            thoughts.push(AgentThought {
                role:      v["role"].as_str().unwrap_or("thought").to_string(),
                content:   v["content"].as_str().unwrap_or("").to_string(),
                timestamp: v["timestamp"].as_str().unwrap_or("").to_string(),
                metadata:  v["metadata"].as_str().map(String::from),
            });
        }
    }
    Ok(thoughts)
}

// ── Memory Entries ─────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct MemoryEntry {
    content:     String,
    memory_type: String,
    score:       f64,
    timestamp:   String,
}

#[tauri::command]
async fn get_memory_entries() -> Result<Vec<MemoryEntry>, String> {
    match run_housaky_command(&["memory", "list", "--json"]) {
        Ok(output) => {
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&output) {
                let entries = arr.iter().map(|v| MemoryEntry {
                    content:     v["content"].as_str().unwrap_or("").to_string(),
                    memory_type: v["memory_type"].as_str().unwrap_or("semantic").to_string(),
                    score:       v["score"].as_f64().unwrap_or(0.0),
                    timestamp:   v["timestamp"].as_str().unwrap_or("").to_string(),
                }).collect();
                return Ok(entries);
            }
            Ok(vec![])
        }
        Err(_) => Ok(vec![]),
    }
}

// ── Conversations ──────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct ConversationSummary {
    id:            String,
    title:         String,
    last_message:  String,
    timestamp:     String,
    message_count: u64,
}

#[tauri::command]
async fn get_conversations() -> Result<Vec<ConversationSummary>, String> {
    let workspace = get_workspace_path();
    let conv_dir = workspace.join(".housaky").join("conversations");
    if !conv_dir.exists() {
        return Ok(vec![]);
    }
    let mut conversations = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&conv_dir) {
        for entry in entries.flatten().take(20) {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(data) = std::fs::read_to_string(&path) {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                        let id = path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string();
                        conversations.push(ConversationSummary {
                            id:            id.clone(),
                            title:         v["title"].as_str().unwrap_or("Untitled").to_string(),
                            last_message:  v["last_message"].as_str().unwrap_or("").to_string(),
                            timestamp:     v["timestamp"].as_str().unwrap_or("").to_string(),
                            message_count: v["message_count"].as_u64().unwrap_or(0),
                        });
                    }
                }
            }
        }
    }
    // Sort by timestamp descending
    conversations.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(conversations)
}

// ── Async Config Save with validation ─────────────────────────────────────

#[tauri::command]
async fn validate_config(config: HousakyConfig) -> Result<Vec<String>, String> {
    let mut warnings: Vec<String> = Vec::new();
    if config.api_key.as_deref().unwrap_or("").is_empty() {
        warnings.push("No API key set — AI features will not work".to_string());
    }
    if config.autonomy.max_cost_per_day_cents == 0 {
        warnings.push("Daily cost limit is $0 — agent will be blocked immediately".to_string());
    }
    if !config.secrets.encrypt {
        warnings.push("Secrets are stored in plaintext — consider enabling encryption".to_string());
    }
    if config.autonomy.level == "full" && !config.autonomy.workspace_only {
        warnings.push("Full autonomy with no workspace sandbox is a security risk".to_string());
    }
    Ok(warnings)
}

// ── Keys Management ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key: String,
    pub description: String,
    pub enabled: bool,
    pub priority: i32,
    pub tags: Vec<String>,
    pub usage: KeyUsage,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct KeyUsage {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub rate_limited_count: u64,
    pub tokens_used: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderKeys {
    pub name: String,
    pub keys: Vec<ApiKey>,
    pub enabled: bool,
    pub state: ProviderState,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderState {
    pub is_healthy: bool,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub is_rate_limited: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeysData {
    pub providers: HashMap<String, ProviderKeys>,
    pub subagents: HashMap<String, SubAgentConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubAgentConfig {
    pub provider: String,
    pub model: String,
    pub key_name: String,
    pub max_concurrent: i32,
    pub role: String,
    pub awareness: Vec<String>,
}

#[tauri::command]
async fn get_keys() -> Result<KeysData, String> {
    let keys_path = get_keys_path();
    if !keys_path.exists() {
        return Ok(KeysData {
            providers: HashMap::new(),
            subagents: HashMap::new(),
        });
    }
    let content = fs::read_to_string(&keys_path)
        .map_err(|e| format!("Failed to read keys: {}", e))?;
    let data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse keys: {}", e))?;
    
    let mut providers = HashMap::new();
    if let Some(prov) = data.get("providers").and_then(|p| p.as_object()) {
        for (name, v) in prov {
            let keys = v.get("keys")
                .and_then(|k| k.as_array())
                .map(|arr| {
                    arr.iter().filter_map(|k| {
                        Some(ApiKey {
                            id: k.get("id")?.as_str()?.to_string(),
                            name: k.get("name")?.as_str()?.to_string(),
                            key: k.get("key").and_then(|k| k.as_str()).unwrap_or("").to_string(),
                            description: k.get("description").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                            enabled: k.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true),
                            priority: k.get("priority").and_then(|p| p.as_i64()).unwrap_or(1) as i32,
                            tags: k.get("tags").and_then(|t| t.as_array()).map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default(),
                            usage: KeyUsage {
                                total_requests: k.get("usage").and_then(|u| u.get("total_requests")).and_then(|v| v.as_u64()).unwrap_or(0),
                                successful_requests: k.get("usage").and_then(|u| u.get("successful_requests")).and_then(|v| v.as_u64()).unwrap_or(0),
                                failed_requests: k.get("usage").and_then(|u| u.get("failed_requests")).and_then(|v| v.as_u64()).unwrap_or(0),
                                rate_limited_count: k.get("usage").and_then(|u| u.get("rate_limited_count")).and_then(|v| v.as_u64()).unwrap_or(0),
                                tokens_used: k.get("usage").and_then(|u| u.get("tokens_used")).and_then(|v| v.as_u64()).unwrap_or(0),
                            },
                        })
                    }).collect()
                })
                .unwrap_or_default();
            
            let enabled = v.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
            let state = v.get("state").and_then(|s| s.as_object());
            
            providers.insert(name.clone(), ProviderKeys {
                name: name.clone(),
                keys,
                enabled,
                state: ProviderState {
                    is_healthy: state.and_then(|s| s.get("is_healthy")).and_then(|v| v.as_bool()).unwrap_or(true),
                    consecutive_failures: state.and_then(|s| s.get("consecutive_failures")).and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    consecutive_successes: state.and_then(|s| s.get("consecutive_successes")).and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    is_rate_limited: state.and_then(|s| s.get("is_rate_limited")).and_then(|v| v.as_bool()).unwrap_or(false),
                },
            });
        }
    }
    
    let mut subagents = HashMap::new();
    if let Some(sa) = data.get("subagents").and_then(|s| s.as_object()) {
        for (name, v) in sa {
            subagents.insert(name.clone(), SubAgentConfig {
                provider: v.get("provider").and_then(|p| p.as_str()).unwrap_or("modal").to_string(),
                model: v.get("model").and_then(|m| m.as_str()).unwrap_or("").to_string(),
                key_name: v.get("key_name").and_then(|k| k.as_str()).unwrap_or("").to_string(),
                max_concurrent: v.get("max_concurrent").and_then(|m| m.as_i64()).unwrap_or(2) as i32,
                role: v.get("role").and_then(|r| r.as_str()).unwrap_or("").to_string(),
                awareness: v.get("awareness").and_then(|a| a.as_array()).map(|arr| arr.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default(),
            });
        }
    }
    
    Ok(KeysData { providers, subagents })
}

#[tauri::command]
async fn save_key(provider: String, key: ApiKey) -> Result<String, String> {
    let keys_path = get_keys_path();
    let mut data: serde_json::Value = if keys_path.exists() {
        let content = fs::read_to_string(&keys_path)
            .map_err(|e| format!("Failed to read keys: {}", e))?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({
            "providers": {},
            "subagents": {}
        })
    };
    
    if let Some(providers) = data.get_mut("providers") {
        if let Some(prov) = providers.get_mut(&provider) {
            if let Some(keys) = prov.get_mut("keys") {
                if let Some(arr) = keys.as_array_mut() {
                    arr.push(serde_json::json!({
                        "id": key.id,
                        "key": key.key,
                        "name": key.name,
                        "description": key.description,
                        "enabled": key.enabled,
                        "priority": key.priority,
                        "tags": key.tags,
                        "usage": {
                            "total_requests": 0,
                            "successful_requests": 0,
                            "failed_requests": 0,
                            "rate_limited_count": 0,
                            "tokens_used": 0
                        }
                    }));
                }
            }
        }
    }
    
    let json = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(&keys_path, json)
        .map_err(|e| format!("Failed to write keys: {}", e))?;
    
    Ok(format!("Key added to {}", provider))
}

#[tauri::command]
async fn delete_key(provider: String, key_id: String) -> Result<String, String> {
    let keys_path = get_keys_path();
    if !keys_path.exists() {
        return Err("No keys file found".to_string());
    }
    
    let content = fs::read_to_string(&keys_path)
        .map_err(|e| format!("Failed to read keys: {}", e))?;
    let mut data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse keys: {}", e))?;
    
    if let Some(providers) = data.get_mut("providers") {
        if let Some(prov) = providers.get_mut(&provider) {
            if let Some(keys) = prov.get_mut("keys") {
                if let Some(arr) = keys.as_array_mut() {
                    arr.retain(|k| k.get("id").and_then(|id| id.as_str()) != Some(&key_id));
                }
            }
        }
    }
    
    let json = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(&keys_path, json)
        .map_err(|e| format!("Failed to write keys: {}", e))?;
    
    Ok(format!("Key {} deleted", key_id))
}

#[tauri::command]
async fn toggle_key(provider: String, key_id: String, enabled: bool) -> Result<String, String> {
    let keys_path = get_keys_path();
    if !keys_path.exists() {
        return Err("No keys file found".to_string());
    }
    
    let content = fs::read_to_string(&keys_path)
        .map_err(|e| format!("Failed to read keys: {}", e))?;
    let mut data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse keys: {}", e))?;
    
    if let Some(providers) = data.get_mut("providers") {
        if let Some(prov) = providers.get_mut(&provider) {
            if let Some(keys) = prov.get_mut("keys") {
                if let Some(arr) = keys.as_array_mut() {
                    for k in arr.iter_mut() {
                        if k.get("id").and_then(|id| id.as_str()) == Some(&key_id) {
                            k["enabled"] = serde_json::json!(enabled);
                            break;
                        }
                    }
                }
            }
        }
    }
    
    let json = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(&keys_path, json)
        .map_err(|e| format!("Failed to write keys: {}", e))?;
    
    Ok(format!("Key {} {}", key_id, if enabled { "enabled" } else { "disabled" }))
}

// ── Kowalski Subagents ─────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct KowalskiAgent {
    pub name: String,
    pub agent_type: String,
    pub enabled: bool,
    pub status: String,
    pub role: String,
    pub awareness: Vec<String>,
    pub max_concurrent: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KowalskiStatus {
    pub installed: bool,
    pub agents: Vec<KowalskiAgent>,
    pub path: String,
}

#[tauri::command]
async fn get_kowalski_status() -> Result<KowalskiStatus, String> {
    let keys_data = get_keys().await?;
    
    let kowalski_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("vendor")
        .join("kowalski");
    
    let installed = kowalski_path.exists();
    
    let mut agents = Vec::new();
    
    let agent_types = vec![
        ("kowalski-code", "code", "Code analysis, refactoring, and documentation"),
        ("kowalski-web", "web", "Web research and information retrieval"),
        ("kowalski-academic", "academic", "Academic research and paper analysis"),
        ("kowalski-data", "data", "Data analysis and processing"),
        ("kowalski-creative", "creative", "Creative synthesis and idea generation"),
        ("kowalski-reasoning", "reasoning", "Logical reasoning and deduction"),
        ("kowalski-federation", "federation", "Multi-agent coordination and federation"),
    ];
    
    for (name, agent_type, _desc) in agent_types {
        let config = keys_data.subagents.get(name);
        let enabled = config.is_some();
        
        agents.push(KowalskiAgent {
            name: name.to_string(),
            agent_type: agent_type.to_string(),
            enabled,
            status: if installed && enabled { "available".to_string() } else if !installed { "not_installed".to_string() } else { "disabled".to_string() },
            role: config.map(|c| c.role.clone()).unwrap_or_default(),
            awareness: config.map(|c| c.awareness.clone()).unwrap_or_default(),
            max_concurrent: config.map(|c| c.max_concurrent).unwrap_or(2),
        });
    }
    
    Ok(KowalskiStatus {
        installed,
        agents,
        path: kowalski_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
async fn configure_subagent(name: String, config: SubAgentConfig) -> Result<String, String> {
    let keys_path = get_keys_path();
    let name_clone = name.clone();
    let mut data: serde_json::Value = if keys_path.exists() {
        let content = fs::read_to_string(&keys_path)
            .map_err(|e| format!("Failed to read keys: {}", e))?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({
            "providers": {},
            "subagents": {}
        })
    };
    
    if let Some(subagents) = data.get_mut("subagents") {
        subagents[name] = serde_json::json!({
            "provider": config.provider,
            "model": config.model,
            "key_name": config.key_name,
            "max_concurrent": config.max_concurrent,
            "role": config.role,
            "awareness": config.awareness
        });
    }
    
    let json = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(&keys_path, json)
        .map_err(|e| format!("Failed to write keys: {}", e))?;
    
    Ok(format!("Subagent {} configured", name_clone))
}

#[tauri::command]
async fn toggle_subagent(name: String, enabled: bool) -> Result<String, String> {
    let keys_path = get_keys_path();
    let name_clone = name.clone();
    if !keys_path.exists() {
        return Err("No keys file found".to_string());
    }
    
    let content = fs::read_to_string(&keys_path)
        .map_err(|e| format!("Failed to read keys: {}", e))?;
    let mut data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse keys: {}", e))?;
    
    if enabled {
        if let Some(subagents) = data.get_mut("subagents") {
            let default_config = match name_clone.as_str() {
                "kowalski-code" => serde_json::json!({"provider": "modal", "model": "zai-org/GLM-5-FP8", "key_name": "housaky", "max_concurrent": 3, "role": "code", "awareness": ["coding", "debugging", "refactoring"]}),
                "kowalski-web" => serde_json::json!({"provider": "modal", "model": "zai-org/GLM-5-FP8", "key_name": "housaky", "max_concurrent": 2, "role": "web", "awareness": ["web_search", "fetching", "browsing"]}),
                "kowalski-academic" => serde_json::json!({"provider": "modal", "model": "zai-org/GLM-5-FP8", "key_name": "housaky", "max_concurrent": 2, "role": "academic", "awareness": ["research", "analysis", "writing"]}),
                "kowalski-data" => serde_json::json!({"provider": "modal", "model": "zai-org/GLM-5-FP8", "key_name": "housaky", "max_concurrent": 2, "role": "data", "awareness": ["data_analysis", "visualization", "statistics"]}),
                "kowalski-creative" => serde_json::json!({"provider": "modal", "model": "zai-org/GLM-5-FP8", "key_name": "housaky", "max_concurrent": 2, "role": "creative", "awareness": ["creative_writing", "brainstorming", "ideation"]}),
                "kowalski-reasoning" => serde_json::json!({"provider": "modal", "model": "zai-org/GLM-5-FP8", "key_name": "housaky", "max_concurrent": 2, "role": "reasoning", "awareness": ["logic", "problem_solving", "reasoning"]}),
                "kowalski-federation" => serde_json::json!({"provider": "modal", "model": "zai-org/GLM-5-FP8", "key_name": "housaky", "max_concurrent": 5, "role": "federation", "awareness": ["multi_agent", "collaboration", "coordination"]}),
                _ => serde_json::json!({"provider": "modal", "model": "zai-org/GLM-5-FP8", "key_name": "housaky", "max_concurrent": 2, "role": "", "awareness": []}),
            };
            subagents[name] = default_config;
        }
    } else {
        if let Some(subagents) = data.get_mut("subagents") {
            if let Some(obj) = subagents.as_object_mut() {
                obj.remove(&name_clone);
            }
        }
    }
    
    let json = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(&keys_path, json)
        .map_err(|e| format!("Failed to write keys: {}", e))?;
    
    Ok(format!("Subagent {} {}", name_clone, if enabled { "enabled" } else { "disabled" }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_config,
            save_config,
            validate_config,
            send_message,
            run_housaky_command_cmd,
            get_skills,
            toggle_skill,
            get_channels,
            configure_channel,
            start_channel,
            stop_channel,
            check_housaky_installed,
            get_integrations,
            hardware_discover,
            run_doctor,
            get_agi_telemetry,
            get_agent_thoughts,
            get_memory_entries,
            get_conversations,
            // Keys management
            get_keys,
            save_key,
            delete_key,
            toggle_key,
            // Kowalski subagents
            get_kowalski_status,
            configure_subagent,
            toggle_subagent,
            // Market & Skills
            get_marketplace_skills,
            install_market_skill,
            uninstall_skill,
            // MCPs
            get_mcp_servers,
            configure_mcp_server,
        ])
        .setup(|_app| {
            log::info!("Housaky Dashboard starting...");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

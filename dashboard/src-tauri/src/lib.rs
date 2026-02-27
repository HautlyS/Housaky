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
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("housaky")
        .join("config.toml")
}

fn get_workspace_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("housaky")
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
        ])
        .setup(|_app| {
            log::info!("Housaky Dashboard starting...");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

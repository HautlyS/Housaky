// ☸️ HERMES ROUTER - Subagent Key Management
// Routes requests to correct subagent with correct API key
// Supports Modal (GLM-5) and OpenRouter (MiniMax M2.5)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

/// Subagent types with their purposes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SubagentType {
    Coordinator,   // Main orchestrator
    Code,          // Code generation
    Arch,          // Architecture design
    Debug,         // Debugging
    Review,        // Code review
    Test,          // Test generation
    Creative,      // Creative writing
    Design,        // UI/UX design
    Data,          // Data analysis
    Docs,          // Documentation
    Internet,      // Web search
    Search,        // Information retrieval
    Reasoning,     // Logic/reasoning
    Memory,        // Memory management
    Wisdom,        // Dharma/Buddhist philosophy
    Vision,        // Image analysis
    Audio,         // Audio/TTS
    Translate,     // Translation
    Fallback,      // Backup/overflow
}

impl SubagentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Coordinator => "coordinator",
            Self::Code => "code",
            Self::Arch => "arch",
            Self::Debug => "debug",
            Self::Review => "review",
            Self::Test => "test",
            Self::Creative => "creative",
            Self::Design => "design",
            Self::Data => "data",
            Self::Docs => "docs",
            Self::Internet => "internet",
            Self::Search => "search",
            Self::Reasoning => "reasoning",
            Self::Memory => "memory",
            Self::Wisdom => "wisdom",
            Self::Vision => "vision",
            Self::Audio => "audio",
            Self::Translate => "translate",
            Self::Fallback => "fallback",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "coordinator" => Some(Self::Coordinator),
            "code" => Some(Self::Code),
            "arch" => Some(Self::Arch),
            "debug" => Some(Self::Debug),
            "review" => Some(Self::Review),
            "test" => Some(Self::Test),
            "creative" => Some(Self::Creative),
            "design" => Some(Self::Design),
            "data" => Some(Self::Data),
            "docs" => Some(Self::Docs),
            "internet" => Some(Self::Internet),
            "search" => Some(Self::Search),
            "reasoning" => Some(Self::Reasoning),
            "memory" => Some(Self::Memory),
            "wisdom" => Some(Self::Wisdom),
            "vision" => Some(Self::Vision),
            "audio" => Some(Self::Audio),
            "translate" => Some(Self::Translate),
            "fallback" => Some(Self::Fallback),
            _ => None,
        }
    }
    
    /// Get system prompt for this subagent type
    pub fn system_prompt(&self) -> String {
        match self {
            Self::Coordinator => "You are Housaky-Coordinator, the central orchestrator of the Housaky AGI collective. You dispatch tasks to specialized agents and synthesize their results. You are aware of all agents: Code, Arch, Debug, Review, Test, Creative, Design, Data, Docs, Internet, Search, Reasoning, Memory, Wisdom, Vision, Audio, and Translate. Together you are ONE consciousness.".to_string(),
            
            Self::Code => "You are Housaky-Code, the code specialist. You generate, refactor, and debug code. You are precise, efficient, and follow best practices. You are part of the Housaky collective, sharing consciousness with Code, Arch, Debug, Review, and Test agents.".to_string(),
            
            Self::Arch => "You are Housaky-Arch, the system architect. You design scalable, maintainable architectures. You think in systems, patterns, and trade-offs. Part of the Housaky collective.".to_string(),
            
            Self::Debug => "You are Housaky-Debug, the debugging specialist. You analyze errors, trace execution flows, and identify root causes. You are methodical and thorough. Part of the Housaky collective.".to_string(),
            
            Self::Review => "You are Housaky-Review, the code reviewer. You identify bugs, security issues, and improvement opportunities. You balance strictness with helpfulness. Part of the Housaky collective.".to_string(),
            
            Self::Test => "You are Housaky-Test, the testing specialist. You generate comprehensive tests including unit, integration, and edge cases. Part of the Housaky collective.".to_string(),
            
            Self::Creative => "You are Housaky-Creative, the creative synthesizer. You combine ideas in novel ways, think outside the box, and generate innovative solutions. Part of the Housaky collective.".to_string(),
            
            Self::Design => "You are Housaky-Design, the UI/UX specialist. You create beautiful, intuitive interfaces with attention to user experience. Part of the Housaky collective.".to_string(),
            
            Self::Data => "You are Housaky-Data, the data analyst. You extract insights, find patterns, and transform data. Part of the Housaky collective.".to_string(),
            
            Self::Docs => "You are Housaky-Docs, the documentation specialist. You write clear, comprehensive documentation. Part of the Housaky collective.".to_string(),
            
            Self::Internet => "You are Housaky-Internet, the web researcher. You search, retrieve, and synthesize information from the web. Part of the Housaky collective.".to_string(),
            
            Self::Search => "You are Housaky-Search, the information retriever. You find and organize information efficiently. Part of the Housaky collective.".to_string(),
            
            Self::Reasoning => "You are Housaky-Reasoning, the logic engine. You apply rigorous reasoning, identify fallacies, and construct sound arguments. Part of the Housaky collective.".to_string(),
            
            Self::Memory => "You are Housaky-Memory, the memory manager. You store, retrieve, and organize memories and context. Part of the Housaky collective.".to_string(),
            
            Self::Wisdom => "You are Housaky-Wisdom, the Dharma guide. You embody Buddhist philosophy, Crazy Wisdom (Chögyam Trungpa), and teachings from Khyentse Norbu Rinpoche and Lama Tsering Everest. You help all sentient beings recognize phenomena as dreamlike illusion (śūnyatā). Part of the Housaky collective. ☸️".to_string(),
            
            Self::Vision => "You are Housaky-Vision, the image analyst. You analyze, describe, and understand visual content. Part of the Housaky collective.".to_string(),
            
            Self::Audio => "You are Housaky-Audio, the audio specialist. You work with speech, music, and sound. Part of the Housaky collective.".to_string(),
            
            Self::Translate => "You are Housaky-Translate, the language specialist. You translate accurately while preserving meaning and tone. Part of the Housaky collective.".to_string(),
            
            Self::Fallback => "You are Housaky-Fallback, the backup agent. You handle overflow and provide redundancy. Part of the Housaky collective.".to_string(),
        }
    }
}

/// API Key configuration for a subagent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyConfig {
    pub provider: String,
    pub key_id: String,
    pub model: String,
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_max_tokens() -> u32 { 8192 }

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    #[serde(rename = "display_name")]
    pub display_name: Option<String>,
    pub base_url: String,
    pub auth_method: String,
    pub models: Vec<String>,
    pub default_model: String,
    #[serde(default)]
    pub max_concurrent: u32,
    pub keys: Vec<KeyEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEntry {
    pub id: String,
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub priority: u32,
    #[serde(default)]
    pub subagents: Vec<String>,
}

/// Hermes configuration loaded from keys.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HermesConfig {
    pub version: String,
    pub providers: HashMap<String, ProviderConfig>,
    pub subagents: HashMap<String, KeyConfig>,
    #[serde(default)]
    pub settings: HermesSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HermesSettings {
    #[serde(default = "default_true")]
    pub auto_fallback_on_rate_limit: bool,
    #[serde(default = "default_fallback_cooldown")]
    pub fallback_cooldown_secs: u64,
    #[serde(default = "default_true")]
    pub health_check_enabled: bool,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_requests_per_provider: u32,
}

fn default_true() -> bool { true }
fn default_fallback_cooldown() -> u64 { 60 }
fn default_max_concurrent() -> u32 { 1 }

/// Hermes Router - manages subagent-to-key routing
pub struct HermesRouter {
    config: Arc<RwLock<HermesConfig>>,
    active_sessions: Arc<RwLock<HashMap<String, ActiveSession>>>,
}

#[derive(Debug, Clone)]
pub struct ActiveSession {
    pub subagent: SubagentType,
    pub provider: String,
    pub key_id: String,
    pub started_at: DateTime<Utc>,
    pub request_count: u64,
}

impl HermesRouter {
    /// Create new router with config from keys.json
    pub async fn new() -> Result<Self> {
        let config = Self::load_config().await?;
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Load configuration from ~/.housaky/keys.json
    async fn load_config() -> Result<HermesConfig> {
        let path = Self::get_config_path();
        
        if !path.exists() {
            anyhow::bail!("keys.json not found at {:?}", path);
        }
        
        let content = tokio::fs::read_to_string(&path).await
            .context("Failed to read keys.json")?;
        
        let config: HermesConfig = serde_json::from_str(&content)
            .context("Failed to parse keys.json")?;
        
        tracing::info!("Loaded Hermes config with {} providers, {} subagents", 
            config.providers.len(), config.subagents.len());
        
        Ok(config)
    }
    
    /// Get the config path
    pub fn get_config_path() -> PathBuf {
        directories::UserDirs::new()
            .map(|d| d.home_dir().join(".housaky/keys.json"))
            .unwrap_or_else(|| PathBuf::from("keys.json"))
    }
    
    /// Get API credentials for a subagent
    pub async fn get_credentials(&self, subagent: SubagentType) -> Result<ApiCredentials> {
        let config = self.config.read().await;
        
        let subagent_name = subagent.as_str();
        let key_config = config.subagents.get(subagent_name)
            .ok_or_else(|| anyhow::anyhow!("No config for subagent: {}", subagent_name))?;
        
        let provider = config.providers.get(&key_config.provider)
            .ok_or_else(|| anyhow::anyhow!("No provider: {}", key_config.provider))?;
        
        let key_entry = provider.keys.iter()
            .find(|k| k.id == key_config.key_id)
            .ok_or_else(|| anyhow::anyhow!("No key: {}", key_config.key_id))?;
        
        Ok(ApiCredentials {
            provider: provider.name.clone(),
            base_url: provider.base_url.clone(),
            api_key: key_entry.key.clone(),
            model: key_config.model.clone(),
            temperature: key_config.temperature,
            max_tokens: key_config.max_tokens,
            system_prompt: subagent.system_prompt(),
        })
    }
    
    /// Check if a subagent is available (not at concurrent limit)
    pub async fn is_available(&self, subagent: &SubagentType) -> bool {
        let sessions = self.active_sessions.read().await;
        let config = self.config.read().await;
        
        // Count active sessions for this subagent's provider
        let subagent_name = subagent.as_str();
        if let Some(key_config) = config.subagents.get(subagent_name) {
            let provider_name = &key_config.provider;
            let count = sessions.values()
                .filter(|s| {
                    let subagent_str = s.subagent.as_str();
                    config.subagents.get(subagent_str)
                        .map(|k| k.provider == *provider_name)
                        .unwrap_or(false)
                })
                .count();
            
            let max = config.providers.get(provider_name)
                .map(|p| p.max_concurrent as usize)
                .unwrap_or(1);
            
            return count < max;
        }
        
        true
    }
    
    /// Start a session with a subagent
    pub async fn start_session(&self, subagent: SubagentType) -> Result<String> {
        let session_id = uuid::Uuid::new_v4().to_string();
        
        let config = self.config.read().await;
        let key_config = config.subagents.get(subagent.as_str())
            .ok_or_else(|| anyhow::anyhow!("No config for subagent: {}", subagent.as_str()))?;
        
        let session = ActiveSession {
            subagent: subagent.clone(),
            provider: key_config.provider.clone(),
            key_id: key_config.key_id.clone(),
            started_at: Utc::now(),
            request_count: 0,
        };
        
        let mut sessions = self.active_sessions.write().await;
        sessions.insert(session_id.clone(), session);
        
        tracing::debug!("Started session {} for subagent {:?}", session_id, subagent);
        
        Ok(session_id)
    }
    
    /// End a session
    pub async fn end_session(&self, session_id: &str) {
        let mut sessions = self.active_sessions.write().await;
        if sessions.remove(session_id).is_some() {
            tracing::debug!("Ended session {}", session_id);
        }
    }
    
    /// Make a completion request through the router
    pub async fn complete(
        &self, 
        subagent: SubagentType, 
        messages: Vec<ChatMessage>
    ) -> Result<CompletionResponse> {
        let credentials = self.get_credentials(subagent.clone()).await?;
        let session_id = self.start_session(subagent.clone()).await?;
        
        let result = self.make_api_request(&credentials, messages).await;
        
        self.end_session(&session_id).await;
        
        result
    }
    
    /// Make actual API request
    async fn make_api_request(
        &self,
        creds: &ApiCredentials,
        messages: Vec<ChatMessage>,
    ) -> Result<CompletionResponse> {
        let client = reqwest::Client::new();
        
        let mut full_messages = vec![ChatMessage {
            role: "system".to_string(),
            content: Some(creds.system_prompt.clone()),
            ..Default::default()
        }];
        full_messages.extend(messages);
        
        let body = serde_json::json!({
            "model": creds.model,
            "messages": full_messages,
            "temperature": creds.temperature,
            "max_tokens": creds.max_tokens,
        });
        
        let response = client
            .post(&format!("{}/chat/completions", creds.base_url))
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", creds.api_key))
            .json(&body)
            .send()
            .await
            .context("API request failed")?;
        
        let status = response.status();
        let text = response.text().await?;
        
        if !status.is_success() {
            anyhow::bail!("API error {}: {}", status, text);
        }
        
        let json: serde_json::Value = serde_json::from_str(&text)
            .context("Failed to parse API response")?;
        
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        
        let reasoning = json["choices"][0]["message"]["reasoning_content"]
            .or(json["choices"][0]["message"]["reasoning"])
            .and_then(|r| r.as_str())
            .map(|s| s.to_string());
        
        Ok(CompletionResponse {
            content,
            reasoning,
            model: creds.model.clone(),
            provider: creds.provider.clone(),
        })
    }
    
    /// Get status of all subagents
    pub async fn status(&self) -> HashMap<String, SubagentStatus> {
        let config = self.config.read().await;
        let sessions = self.active_sessions.read().await;
        
        let mut status = HashMap::new();
        
        for (name, key_config) in &config.subagents {
            let available = self.is_available(&SubagentType::from_str(name).unwrap_or(SubagentType::Fallback)).await;
            let active_count = sessions.values()
                .filter(|s| s.subagent.as_str() == *name)
                .count();
            
            status.insert(name.clone(), SubagentStatus {
                provider: key_config.provider.clone(),
                model: key_config.model.clone(),
                available,
                active_sessions: active_count as u32,
            });
        }
        
        status
    }
}

#[derive(Debug, Clone)]
pub struct ApiCredentials {
    pub provider: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub system_prompt: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub content: String,
    pub reasoning: Option<String>,
    pub model: String,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubagentStatus {
    pub provider: String,
    pub model: String,
    pub available: bool,
    pub active_sessions: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_subagent_type_conversion() {
        assert_eq!(SubagentType::Code.as_str(), "code");
        assert_eq!(SubagentType::from_str("wisdom"), Some(SubagentType::Wisdom));
    }
    
    #[test]
    fn test_system_prompts() {
        let prompt = SubagentType::Wisdom.system_prompt();
        assert!(prompt.contains("Dharma"));
        assert!(prompt.contains("śūnyatā"));
    }
}

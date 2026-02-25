#![allow(clippy::cast_possible_truncation)]

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize};
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

static KEYS_MANAGER: OnceLock<Arc<KeysManager>> = OnceLock::new();
static KEYS_STORAGE_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn get_keys_storage_path() -> PathBuf {
    KEYS_STORAGE_PATH.get_or_init(|| {
        if let Some(user_dirs) = directories::UserDirs::new() {
            let mut path = user_dirs.home_dir().to_path_buf();
            path.push(".housaky");
            path.push("keys.json");
            path
        } else {
            PathBuf::from("keys.json")
        }
    }).clone()
}

pub fn set_keys_storage_path(path: PathBuf) {
    let _ = KEYS_STORAGE_PATH.set(path);
}

pub fn get_global_keys_manager() -> Arc<KeysManager> {
    KEYS_MANAGER.get_or_init(|| Arc::new(KeysManager::new())).clone()
}

pub fn init_global_keys_manager() -> Arc<KeysManager> {
    get_global_keys_manager()
}

pub async fn load_keys_from_file() {
    if let Some(manager) = KEYS_MANAGER.get() {
        let path = get_keys_storage_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(store) = serde_json::from_str::<KeysStore>(&content) {
                    let mut s = manager.store.write().await;
                    *s = store;
                    tracing::info!("Loaded keys from {}", path.display());
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderPriority {
    #[serde(rename = "primary")]
    Primary = 1,
    #[serde(rename = "secondary")]
    Secondary = 2,
    #[serde(rename = "tertiary")]
    Tertiary = 3,
    #[serde(rename = "quaternary")]
    Quaternary = 4,
    #[serde(rename = "disabled")]
    Disabled = 99,
}

impl Default for ProviderPriority {
    fn default() -> Self {
        Self::Primary
    }
}

impl std::fmt::Display for ProviderPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "Primary"),
            Self::Secondary => write!(f, "Secondary"),
            Self::Tertiary => write!(f, "Tertiary"),
            Self::Quaternary => write!(f, "Quaternary"),
            Self::Disabled => write!(f, "Disabled"),
        }
    }
}

impl ProviderPriority {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "primary" | "main" | "1" => Some(Self::Primary),
            "secondary" | "2" => Some(Self::Secondary),
            "tertiary" | "3" => Some(Self::Tertiary),
            "quaternary" | "4" => Some(Self::Quaternary),
            "disabled" => Some(Self::Disabled),
            _ => None,
        }
    }

    pub fn level(&self) -> u8 {
        *self as u8
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysStore {
    pub providers: HashMap<String, ProviderEntry>,
    pub settings: KeysSettings,
    pub version: String,
}

impl Default for KeysStore {
    fn default() -> Self {
        Self {
            providers: HashMap::new(),
            settings: KeysSettings::default(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysSettings {
    pub auto_fallback_on_rate_limit: bool,
    pub fallback_cooldown_secs: u64,
    pub health_check_enabled: bool,
    pub health_check_interval_secs: u64,
    pub max_concurrent_requests_per_provider: usize,
    pub failure_threshold: u32,
    pub success_threshold: u32,
}

impl Default for KeysSettings {
    fn default() -> Self {
        Self {
            auto_fallback_on_rate_limit: true,
            fallback_cooldown_secs: 60,
            health_check_enabled: true,
            health_check_interval_secs: 300,
            max_concurrent_requests_per_provider: 10,
            failure_threshold: 3,
            success_threshold: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEntry {
    pub name: String,
    pub template: Option<String>,
    pub base_url: Option<String>,
    pub auth_method: String,
    pub keys: Vec<KeyEntry>,
    pub models: Vec<String>,
    pub default_model: Option<String>,
    pub priority: ProviderPriority,
    pub enabled: bool,
    pub headers: HashMap<String, String>,
    pub rate_limit: Option<RateLimitConfig>,
    pub state: ProviderState,
    pub metadata: ProviderMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
}

impl Default for ProviderMetadata {
    fn default() -> Self {
        Self {
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_used_at: None,
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderState {
    pub is_healthy: bool,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_rate_limit_at: Option<DateTime<Utc>>,
    pub current_key_index: usize,
    pub is_rate_limited: bool,
    pub rate_limit_reset_at: Option<DateTime<Utc>>,
}

impl Default for ProviderState {
    fn default() -> Self {
        Self {
            is_healthy: true,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_failure_at: None,
            last_success_at: None,
            last_rate_limit_at: None,
            current_key_index: 0,
            is_rate_limited: false,
            rate_limit_reset_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEntry {
    pub id: String,
    pub key: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub priority: u32,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub usage: KeyUsage,
    pub rate_limit: Option<RateLimitConfig>,
}

impl KeyEntry {
    pub fn new(key: String, name: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            key,
            name,
            description: String::new(),
            enabled: true,
            priority: 1,
            created_at: Utc::now(),
            last_used_at: None,
            expires_at: None,
            tags: Vec::new(),
            usage: KeyUsage::default(),
            rate_limit: None,
        }
    }

    pub fn masked_key(&self) -> String {
        if self.key.len() > 8 {
            format!("{}...{}", &self.key[..4], &self.key[self.key.len()-4..])
        } else {
            "*".repeat(self.key.len())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyUsage {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub rate_limited_count: u64,
    pub tokens_used: u64,
    pub last_request_at: Option<DateTime<Utc>>,
}

impl Default for KeyUsage {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            rate_limited_count: 0,
            tokens_used: 0,
            last_request_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
    pub tokens_per_minute: Option<u32>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            requests_per_hour: 1000,
            requests_per_day: 10000,
            tokens_per_minute: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProviderTemplate {
    pub name: String,
    pub base_url: String,
    pub auth_method: String,
    pub default_models: Vec<String>,
    pub headers: HashMap<String, String>,
    pub is_openai_compatible: bool,
}

pub struct KeysManager {
    pub store: Arc<RwLock<KeysStore>>,
    pub provider_templates: HashMap<String, ProviderTemplate>,
    pub rate_limit_states: Arc<RwLock<HashMap<String, RateLimitState>>>,
}

#[derive(Debug)]
pub struct RateLimitState {
    pub request_timestamps: Vec<std::time::Instant>,
    pub current_requests: AtomicUsize,
    pub is_limited: AtomicBool,
    pub reset_at: AtomicU64,
}

impl Default for KeysManager {
    fn default() -> Self {
        Self::new()
    }
}

impl KeysManager {
    pub fn new() -> Self {
        let mut templates = HashMap::new();
        
        templates.insert("openrouter".to_string(), ProviderTemplate {
            name: "openrouter".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            auth_method: "bearer".to_string(),
            default_models: vec![
                "anthropic/claude-3.5-sonnet".to_string(),
                "openai/gpt-4o".to_string(),
                "google/gemini-2.0-flash".to_string(),
                "arcee-ai/trinity-large-preview:free".to_string(),
            ],
            headers: HashMap::new(),
            is_openai_compatible: true,
        });

        templates.insert("anthropic".to_string(), ProviderTemplate {
            name: "anthropic".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            auth_method: "x-api-key".to_string(),
            default_models: vec![
                "claude-opus-4-20250514".to_string(),
                "claude-sonnet-4-20250514".to_string(),
                "claude-3-5-sonnet-20241022".to_string(),
                "claude-3-haiku-20240307".to_string(),
            ],
            headers: {
                let mut h = HashMap::new();
                h.insert("anthropic-version".to_string(), "2023-06-01".to_string());
                h
            },
            is_openai_compatible: false,
        });

        templates.insert("openai".to_string(), ProviderTemplate {
            name: "openai".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            auth_method: "bearer".to_string(),
            default_models: vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "o1-preview".to_string(),
                "o1-mini".to_string(),
            ],
            headers: HashMap::new(),
            is_openai_compatible: true,
        });

        templates.insert("groq".to_string(), ProviderTemplate {
            name: "groq".to_string(),
            base_url: "https://api.groq.com/openai/v1".to_string(),
            auth_method: "bearer".to_string(),
            default_models: vec![
                "llama-3.3-70b-versatile".to_string(),
                "llama-3.1-8b-instant".to_string(),
                "mixtral-8x7b-32768".to_string(),
            ],
            headers: HashMap::new(),
            is_openai_compatible: true,
        });

        templates.insert("google".to_string(), ProviderTemplate {
            name: "google".to_string(),
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            auth_method: "query".to_string(),
            default_models: vec![
                "gemini-2.0-flash".to_string(),
                "gemini-1.5-pro".to_string(),
                "gemini-1.5-flash".to_string(),
            ],
            headers: HashMap::new(),
            is_openai_compatible: false,
        });

        templates.insert("mistral".to_string(), ProviderTemplate {
            name: "mistral".to_string(),
            base_url: "https://api.mistral.ai/v1".to_string(),
            auth_method: "bearer".to_string(),
            default_models: vec![
                "mistral-large-latest".to_string(),
                "mistral-small-latest".to_string(),
                "codestral-latest".to_string(),
            ],
            headers: HashMap::new(),
            is_openai_compatible: true,
        });

        templates.insert("deepseek".to_string(), ProviderTemplate {
            name: "deepseek".to_string(),
            base_url: "https://api.deepseek.com/v1".to_string(),
            auth_method: "bearer".to_string(),
            default_models: vec![
                "deepseek-chat".to_string(),
                "deepseek-coder".to_string(),
            ],
            headers: HashMap::new(),
            is_openai_compatible: true,
        });

        templates.insert("xai".to_string(), ProviderTemplate {
            name: "xai".to_string(),
            base_url: "https://api.x.ai/v1".to_string(),
            auth_method: "bearer".to_string(),
            default_models: vec![
                "grok-2-1212".to_string(),
                "grok-beta".to_string(),
            ],
            headers: HashMap::new(),
            is_openai_compatible: true,
        });

        templates.insert("custom".to_string(), ProviderTemplate {
            name: "custom".to_string(),
            base_url: String::new(),
            auth_method: "bearer".to_string(),
            default_models: vec![],
            headers: HashMap::new(),
            is_openai_compatible: true,
        });

        Self {
            store: Arc::new(RwLock::new(KeysStore::default())),
            provider_templates: templates,
            rate_limit_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_templates(&self) -> Vec<(&str, &ProviderTemplate)> {
        self.provider_templates
            .iter()
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    pub fn get_template_names(&self) -> Vec<&str> {
        self.provider_templates.keys().map(|s| s.as_str()).collect()
    }

    pub async fn add_provider_with_template(
        &self,
        name: &str,
        template_name: &str,
        keys: Vec<String>,
        priority: ProviderPriority,
    ) -> Result<()> {
        let template = self
            .provider_templates
            .get(template_name)
            .ok_or_else(|| anyhow::anyhow!("Template '{}' not found", template_name))?;

        let provider = ProviderEntry {
            name: name.to_string(),
            template: Some(template_name.to_string()),
            base_url: Some(template.base_url.clone()),
            auth_method: template.auth_method.clone(),
            keys: keys
                .into_iter()
                .enumerate()
                .map(|(i, key)| {
                    let mut entry = KeyEntry::new(key, format!("key-{}", i + 1));
                    entry.priority = (i + 1) as u32;
                    entry
                })
                .collect(),
            models: template.default_models.clone(),
            default_model: template.default_models.first().cloned(),
            priority,
            enabled: true,
            headers: template.headers.clone(),
            rate_limit: None,
            state: ProviderState::default(),
            metadata: ProviderMetadata::default(),
        };

        let mut store = self.store.write().await;
        store.providers.insert(name.to_string(), provider);
        drop(store);

        self.save().await?;
        Ok(())
    }

    pub async fn add_custom_provider(
        &self,
        name: &str,
        base_url: &str,
        auth_method: &str,
        keys: Vec<String>,
        models: Vec<String>,
        priority: ProviderPriority,
        headers: HashMap<String, String>,
    ) -> Result<()> {
        let provider = ProviderEntry {
            name: name.to_string(),
            template: None,
            base_url: Some(base_url.to_string()),
            auth_method: auth_method.to_string(),
            keys: keys
                .into_iter()
                .enumerate()
                .map(|(i, key)| {
                    let mut entry = KeyEntry::new(key, format!("key-{}", i + 1));
                    entry.priority = (i + 1) as u32;
                    entry
                })
                .collect(),
            models: models.clone(),
            default_model: models.first().cloned(),
            priority,
            enabled: true,
            headers,
            rate_limit: None,
            state: ProviderState::default(),
            metadata: ProviderMetadata::default(),
        };

        let mut store = self.store.write().await;
        store.providers.insert(name.to_string(), provider);
        drop(store);

        self.save().await?;
        Ok(())
    }

    pub async fn get_providers(&self) -> Vec<ProviderEntry> {
        let store = self.store.read().await;
        let mut providers: Vec<_> = store.providers.values().cloned().collect();
        providers.sort_by_key(|p| p.priority.level());
        providers
    }

    pub async fn get_providers_by_priority(&self) -> Vec<ProviderEntry> {
        let store = self.store.read().await;
        let mut providers: Vec<_> = store.providers
            .values()
            .filter(|p| p.enabled && p.priority != ProviderPriority::Disabled)
            .cloned()
            .collect();
        providers.sort_by_key(|p| p.priority.level());
        providers
    }

    pub async fn get_primary_provider(&self) -> Option<ProviderEntry> {
        let providers = self.get_providers_by_priority().await;
        providers.into_iter().find(|p| p.priority == ProviderPriority::Primary)
    }

    pub async fn get_fallback_chain(&self) -> Vec<ProviderEntry> {
        self.get_providers_by_priority().await
    }

    pub async fn get_provider(&self, name: &str) -> Option<ProviderEntry> {
        let store = self.store.read().await;
        store.providers.get(name).cloned()
    }

    pub async fn set_provider_priority(&self, name: &str, priority: ProviderPriority) -> Result<()> {
        let mut store = self.store.write().await;
        if let Some(provider) = store.providers.get_mut(name) {
            provider.priority = priority;
            provider.metadata.updated_at = Utc::now();
            drop(store);
            self.save().await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Provider '{}' not found", name))
        }
    }

    pub async fn set_provider_enabled(&self, name: &str, enabled: bool) -> Result<()> {
        let mut store = self.store.write().await;
        if let Some(provider) = store.providers.get_mut(name) {
            provider.enabled = enabled;
            provider.metadata.updated_at = Utc::now();
            drop(store);
            self.save().await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Provider '{}' not found", name))
        }
    }

    pub async fn remove_provider(&self, name: &str) -> Result<()> {
        let mut store = self.store.write().await;
        if store.providers.remove(name).is_some() {
            drop(store);
            self.save().await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Provider '{}' not found", name))
        }
    }

    pub async fn add_key(&self, provider_name: &str, key: String, name: Option<String>) -> Result<()> {
        let mut store = self.store.write().await;
        if let Some(provider) = store.providers.get_mut(provider_name) {
            let key_name = name.unwrap_or_else(|| format!("key-{}", provider.keys.len() + 1));
            let mut entry = KeyEntry::new(key, key_name);
            entry.priority = (provider.keys.len() + 1) as u32;
            provider.keys.push(entry);
            provider.metadata.updated_at = Utc::now();
            drop(store);
            self.save().await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Provider '{}' not found", provider_name))
        }
    }

    pub async fn remove_key(&self, provider_name: &str, key_id: &str) -> Result<()> {
        let mut store = self.store.write().await;
        if let Some(provider) = store.providers.get_mut(provider_name) {
            let initial_len = provider.keys.len();
            provider.keys.retain(|k| k.id != key_id);
            if provider.keys.len() < initial_len {
                provider.metadata.updated_at = Utc::now();
                drop(store);
                self.save().await?;
                Ok(())
            } else {
                Err(anyhow::anyhow!("Key '{}' not found in provider '{}'", key_id, provider_name))
            }
        } else {
            Err(anyhow::anyhow!("Provider '{}' not found", provider_name))
        }
    }

    pub async fn set_key_enabled(&self, provider_name: &str, key_id: &str, enabled: bool) -> Result<()> {
        let mut store = self.store.write().await;
        if let Some(provider) = store.providers.get_mut(provider_name) {
            if let Some(key) = provider.keys.iter_mut().find(|k| k.id == key_id) {
                key.enabled = enabled;
                provider.metadata.updated_at = Utc::now();
                drop(store);
                self.save().await?;
                Ok(())
            } else {
                Err(anyhow::anyhow!("Key '{}' not found in provider '{}'", key_id, provider_name))
            }
        } else {
            Err(anyhow::anyhow!("Provider '{}' not found", provider_name))
        }
    }

    pub async fn get_next_key(&self, provider_name: &str) -> Option<KeyEntry> {
        let mut store = self.store.write().await;
        // Clone settings before we mutably borrow a provider entry.
        let _store_settings = store.settings.clone();

        let provider = store.providers.get_mut(provider_name)?;

        if !provider.enabled || provider.keys.is_empty() {
            return None;
        }

        let enabled_keys: Vec<_> = provider.keys.iter().filter(|k| k.enabled).collect();
        if enabled_keys.is_empty() {
            return None;
        }

        let idx = provider.state.current_key_index % enabled_keys.len();
        let key = (*enabled_keys.get(idx)?).clone();
        
        provider.state.current_key_index = (provider.state.current_key_index + 1) % enabled_keys.len().max(1);
        provider.metadata.last_used_at = Some(Utc::now());
        
        Some(key)
    }

    pub async fn report_success(&self, provider_name: &str, key_id: &str) {
        let mut store = self.store.write().await;
        if let Some(provider) = store.providers.get_mut(provider_name) {
            provider.metadata.successful_requests += 1;
            provider.metadata.total_requests += 1;
            provider.state.consecutive_failures = 0;
            provider.state.consecutive_successes += 1;
            provider.state.last_success_at = Some(Utc::now());
            provider.state.is_healthy = true;

            if let Some(key) = provider.keys.iter_mut().find(|k| k.id == key_id) {
                key.usage.successful_requests += 1;
                key.usage.total_requests += 1;
                key.usage.last_request_at = Some(Utc::now());
                key.last_used_at = Some(Utc::now());
            }
        }
    }

    pub async fn report_failure(&self, provider_name: &str, key_id: &str, is_rate_limit: bool) {
        let mut store = self.store.write().await;
        let settings = store.settings.clone();
        
        if let Some(provider) = store.providers.get_mut(provider_name) {
            provider.metadata.failed_requests += 1;
            provider.metadata.total_requests += 1;
            provider.state.consecutive_successes = 0;
            provider.state.consecutive_failures += 1;
            provider.state.last_failure_at = Some(Utc::now());

            if is_rate_limit {
                provider.state.is_rate_limited = true;
                provider.state.last_rate_limit_at = Some(Utc::now());
            }

            if provider.state.consecutive_failures >= settings.failure_threshold {
                provider.state.is_healthy = false;
            }

            if let Some(key) = provider.keys.iter_mut().find(|k| k.id == key_id) {
                key.usage.failed_requests += 1;
                key.usage.total_requests += 1;
                key.usage.last_request_at = Some(Utc::now());
                
                if is_rate_limit {
                    key.usage.rate_limited_count += 1;
                }
            }
        }
    }

    pub async fn get_next_fallback_provider(&self, current_provider: &str) -> Option<ProviderEntry> {
        let providers = self.get_providers_by_priority().await;
        let current_priority = {
            let store = self.store.read().await;
            store.providers.get(current_provider)?.priority
        };
        
        for provider in providers {
            if provider.name != current_provider && provider.priority.level() > current_priority.level() {
                if provider.enabled && provider.state.is_healthy && !provider.state.is_rate_limited {
                    return Some(provider);
                }
            }
        }
        None
    }

    pub async fn check_and_clear_rate_limits(&self) {
        let mut store = self.store.write().await;
        let now = Utc::now();
        
        for provider in store.providers.values_mut() {
            if provider.state.is_rate_limited {
                if let Some(reset_at) = provider.state.rate_limit_reset_at {
                    if now >= reset_at {
                        provider.state.is_rate_limited = false;
                        provider.state.rate_limit_reset_at = None;
                        provider.state.consecutive_failures = 0;
                        provider.state.is_healthy = true;
                    }
                }
            }
        }
    }

    pub async fn set_rate_limited(&self, provider_name: &str, reset_in_secs: u64) {
        let mut store = self.store.write().await;
        if let Some(provider) = store.providers.get_mut(provider_name) {
            provider.state.is_rate_limited = true;
            provider.state.last_rate_limit_at = Some(Utc::now());
            provider.state.rate_limit_reset_at = Some(
                Utc::now() + chrono::Duration::seconds(reset_in_secs as i64)
            );
        }
    }

    pub async fn update_settings(&self, settings: KeysSettings) -> Result<()> {
        let mut store = self.store.write().await;
        store.settings = settings;
        drop(store);
        self.save().await
    }

    pub async fn get_settings(&self) -> KeysSettings {
        let store = self.store.read().await;
        store.settings.clone()
    }

    pub async fn load(&self) -> Result<()> {
        let path = get_keys_storage_path();
        if !path.exists() {
            return Ok(());
        }
        let content = std::fs::read_to_string(&path)?;
        let store: KeysStore = serde_json::from_str(&content)?;
        let mut s = self.store.write().await;
        *s = store;
        Ok(())
    }

    pub async fn set_default_model(&self, provider_name: &str, model: &str) -> Result<()> {
        let mut store = self.store.write().await;
        let provider = store
            .providers
            .get_mut(provider_name)
            .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found", provider_name))?;

        provider.default_model = Some(model.to_string());
        if !provider.models.iter().any(|m| m == model) {
            provider.models.push(model.to_string());
        }
        provider.metadata.updated_at = Utc::now();
        drop(store);
        self.save().await
    }

    pub async fn save(&self) -> Result<()> {
        let store = self.store.read().await;
        let json = serde_json::to_string_pretty(&*store)?;
        let path = get_keys_storage_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, json)?;
        tracing::info!("Saved keys to {}", path.display());
        Ok(())
    }

    pub async fn save_to(&self, path: &str) -> Result<()> {
        let store = self.store.read().await;
        let json = serde_json::to_string_pretty(&*store)?;
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, json)?;
        Ok(())
    }

    pub async fn load_from(&self, path: &str) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let store: KeysStore = serde_json::from_str(&content)?;
        let mut s = self.store.write().await;
        *s = store;
        Ok(())
    }

    pub async fn export(&self) -> Result<String> {
        let store = self.store.read().await;
        Ok(serde_json::to_string_pretty(&*store)?)
    }

    pub async fn import(&self, json: &str) -> Result<()> {
        let store: KeysStore = serde_json::from_str(json)?;
        let mut s = self.store.write().await;
        *s = store;
        self.save().await
    }

    pub async fn get_stats(&self) -> KeysStats {
        let store = self.store.read().await;
        let mut stats = KeysStats::default();
        
        stats.total_providers = store.providers.len();
        stats.enabled_providers = store.providers.values().filter(|p| p.enabled).count();
        stats.primary_providers = store.providers.values().filter(|p| p.priority == ProviderPriority::Primary).count();
        stats.total_keys = store.providers.values().map(|p| p.keys.len()).sum();
        stats.enabled_keys = store.providers.values()
            .map(|p| p.keys.iter().filter(|k| k.enabled).count())
            .sum();
        stats.healthy_providers = store.providers.values().filter(|p| p.state.is_healthy).count();
        stats.rate_limited_providers = store.providers.values().filter(|p| p.state.is_rate_limited).count();
        
        for provider in store.providers.values() {
            stats.total_requests += provider.metadata.total_requests;
            stats.successful_requests += provider.metadata.successful_requests;
            stats.failed_requests += provider.metadata.failed_requests;
        }
        
        stats
    }
}

#[derive(Debug, Clone, Default)]
pub struct KeysStats {
    pub total_providers: usize,
    pub enabled_providers: usize,
    pub primary_providers: usize,
    pub total_keys: usize,
    pub enabled_keys: usize,
    pub healthy_providers: usize,
    pub rate_limited_providers: usize,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_keys_manager_basic() {
        let manager = KeysManager::new();
        
        manager.add_provider_with_template(
            "test-provider",
            "openrouter",
            vec!["key1".to_string(), "key2".to_string()],
            ProviderPriority::Primary,
        ).await.unwrap();

        let providers = manager.get_providers().await;
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].name, "test-provider");
        assert_eq!(providers[0].keys.len(), 2);
    }

    #[tokio::test]
    async fn test_provider_priority() {
        let manager = KeysManager::new();
        
        manager.add_provider_with_template(
            "primary",
            "openrouter",
            vec!["key1".to_string()],
            ProviderPriority::Primary,
        ).await.unwrap();

        manager.add_provider_with_template(
            "backup",
            "anthropic",
            vec!["key2".to_string()],
            ProviderPriority::Secondary,
        ).await.unwrap();

        let fallback = manager.get_fallback_chain().await;
        assert_eq!(fallback.len(), 2);
        assert_eq!(fallback[0].name, "primary");
        assert_eq!(fallback[1].name, "backup");
    }

    #[tokio::test]
    async fn test_key_rotation() {
        let manager = KeysManager::new();
        
        manager.add_provider_with_template(
            "rot-test",
            "openrouter",
            vec!["key-a".to_string(), "key-b".to_string(), "key-c".to_string()],
            ProviderPriority::Primary,
        ).await.unwrap();

        let key1 = manager.get_next_key("rot-test").await.unwrap();
        let key2 = manager.get_next_key("rot-test").await.unwrap();
        let key3 = manager.get_next_key("rot-test").await.unwrap();

        assert_ne!(key1.id, key2.id);
        assert_ne!(key2.id, key3.id);
    }

    #[tokio::test]
    async fn test_fallback_provider() {
        let manager = KeysManager::new();
        
        manager.add_provider_with_template(
            "main",
            "openrouter",
            vec!["key1".to_string()],
            ProviderPriority::Primary,
        ).await.unwrap();

        manager.add_provider_with_template(
            "backup",
            "anthropic",
            vec!["key2".to_string()],
            ProviderPriority::Secondary,
        ).await.unwrap();

        let fallback = manager.get_next_fallback_provider("main").await;
        assert!(fallback.is_some());
        assert_eq!(fallback.unwrap().name, "backup");
    }

    #[tokio::test]
    async fn test_success_failure_tracking() {
        let manager = KeysManager::new();
        
        manager.add_provider_with_template(
            "tracking-test",
            "openrouter",
            vec!["key1".to_string()],
            ProviderPriority::Primary,
        ).await.unwrap();

        let key = manager.get_next_key("tracking-test").await.unwrap();
        
        manager.report_success("tracking-test", &key.id).await;
        manager.report_success("tracking-test", &key.id).await;
        manager.report_failure("tracking-test", &key.id, false).await;

        let provider = manager.get_provider("tracking-test").await.unwrap();
        assert_eq!(provider.metadata.successful_requests, 2);
        assert_eq!(provider.metadata.failed_requests, 1);
        assert_eq!(provider.keys[0].usage.successful_requests, 2);
    }
}

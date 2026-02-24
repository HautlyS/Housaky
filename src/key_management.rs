use crate::config::schema::*;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvmKeyStore {
    pub providers: HashMap<String, KvmProvider>,
    pub rotation: KvmRotationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvmProvider {
    pub name: String,
    pub base_url: Option<String>,
    pub auth_method: String,
    pub keys: Vec<KvmKey>,
    pub default_model: Option<String>,
    pub models: Vec<String>,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvmKey {
    pub id: String,
    pub key: String,
    pub enabled: bool,
    pub priority: u32,
    pub metadata: KvmKeyMetadata,
    pub rate_limit: Option<KvmRateLimit>,
    pub usage: KvmUsageStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvmKeyMetadata {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl Default for KvmKeyMetadata {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            tags: vec![],
            created_at: Utc::now(),
            last_used: None,
            expires_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvmRateLimit {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
    pub tokens_per_minute: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvmUsageStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub rate_limited_count: u64,
    pub last_request_at: Option<DateTime<Utc>>,
    pub usage_percent: f64,
}

impl Default for KvmUsageStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            rate_limited_count: 0,
            last_request_at: None,
            usage_percent: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvmRotationConfig {
    pub enabled: bool,
    pub strategy: KvmRotationStrategy,
    pub cooldown_secs: u64,
    pub health_check_enabled: bool,
    pub health_check_interval_secs: u64,
    pub failure_threshold: u32,
    pub success_rate_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KvmRotationStrategy {
    RoundRobin,
    Priority,
    UsageBased,
    ErrorBased,
    HealthBased,
    Adaptive,
}

impl Default for KvmRotationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: KvmRotationStrategy::Adaptive,
            cooldown_secs: 60,
            health_check_enabled: true,
            health_check_interval_secs: 300,
            failure_threshold: 3,
            success_rate_threshold: 0.9,
        }
    }
}

pub struct KeyRotationState {
    pub current_key_index: AtomicUsize,
    pub last_rotation_time: AtomicU64,
    pub consecutive_failures: AtomicU64,
    pub is_healthy: AtomicBool,
    pub total_switches: AtomicU64,
}

impl KeyRotationState {
    pub fn new() -> Self {
        Self {
            current_key_index: AtomicUsize::new(0),
            last_rotation_time: AtomicU64::new(0),
            consecutive_failures: AtomicU64::new(0),
            is_healthy: AtomicBool::new(true),
            total_switches: AtomicU64::new(0),
        }
    }
}

pub struct KvmKeyManager {
    store: Arc<RwLock<KvmKeyStore>>,
    rotation_states: Arc<RwLock<HashMap<String, Arc<KeyRotationState>>>>,
    provider_templates: HashMap<String, ProviderTemplate>,
}

#[derive(Debug, Clone)]
pub struct ProviderTemplate {
    pub name: String,
    pub base_url: String,
    pub auth_method: String,
    pub default_models: Vec<String>,
    pub headers: HashMap<String, String>,
    pub request_method: String,
    pub is_openai_compatible: bool,
}

impl Default for KvmKeyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl KvmKeyManager {
    pub fn new() -> Self {
        let mut templates = HashMap::new();
        
        templates.insert(
            "openrouter".to_string(),
            ProviderTemplate {
                name: "openrouter".to_string(),
                base_url: "https://openrouter.ai/api/v1".to_string(),
                auth_method: "bearer".to_string(),
                default_models: vec![
                    "anthropic/claude-3.5-sonnet".to_string(),
                    "openai/gpt-4o".to_string(),
                    "google/gemini-2.0-flash".to_string(),
                ],
                headers: HashMap::new(),
                request_method: "POST".to_string(),
                is_openai_compatible: true,
            },
        );

        templates.insert(
            "anthropic".to_string(),
            ProviderTemplate {
                name: "anthropic".to_string(),
                base_url: "https://api.anthropic.com".to_string(),
                auth_method: "api_key".to_string(),
                default_models: vec![
                    "claude-opus-4-20250514".to_string(),
                    "claude-sonnet-4-20250514".to_string(),
                    "claude-3-5-sonnet-20241022".to_string(),
                ],
                headers: {
                    let mut h = HashMap::new();
                    h.insert("anthropic-version".to_string(), "2023-06-01".to_string());
                    h
                },
                request_method: "POST".to_string(),
                is_openai_compatible: false,
            },
        );

        templates.insert(
            "openai".to_string(),
            ProviderTemplate {
                name: "openai".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                auth_method: "bearer".to_string(),
                default_models: vec![
                    "gpt-4o".to_string(),
                    "gpt-4o-mini".to_string(),
                    "o1-preview".to_string(),
                ],
                headers: HashMap::new(),
                request_method: "POST".to_string(),
                is_openai_compatible: true,
            },
        );

        templates.insert(
            "custom".to_string(),
            ProviderTemplate {
                name: "custom".to_string(),
                base_url: String::new(),
                auth_method: "bearer".to_string(),
                default_models: vec![],
                headers: HashMap::new(),
                request_method: "POST".to_string(),
                is_openai_compatible: true,
            },
        );

        Self {
            store: Arc::new(RwLock::new(KvmKeyStore {
                providers: HashMap::new(),
                rotation: KvmRotationConfig::default(),
            })),
            rotation_states: Arc::new(RwLock::new(HashMap::new())),
            provider_templates: templates,
        }
    }

    pub fn from_config(config: &ReliabilityConfig) -> Self {
        let mut manager = Self::new();
        
        if let Some(kvm_path) = &config.kvm_keys_path {
            if let Ok(store) = std::fs::read_to_string(kvm_path) {
                if let Ok(kvm_store) = serde_json::from_str::<KvmKeyStore>(&store) {
                    manager.store = Arc::new(RwLock::new(kvm_store));
                }
            }
        }

        for provider_config in &config.providers {
            let kvm_provider = KvmProvider {
                name: provider_config.name.clone(),
                base_url: provider_config.base_url.clone(),
                auth_method: match provider_config.auth_method {
                    AuthMethod::ApiKey => "api_key".to_string(),
                    AuthMethod::OAuth => "oauth".to_string(),
                    AuthMethod::BearerToken => "bearer".to_string(),
                    AuthMethod::BasicAuth => "basic".to_string(),
                    AuthMethod::Custom => "custom".to_string(),
                },
                keys: provider_config
                    .models
                    .iter()
                    .flat_map(|m| &m.api_keys)
                    .map(|ak| KvmKey {
                        id: uuid::Uuid::new_v4().to_string(),
                        key: ak.key.clone(),
                        enabled: true,
                        priority: 1,
                        metadata: KvmKeyMetadata {
                            name: ak.metadata.description.clone(),
                            description: String::new(),
                            tags: ak.metadata.tags.clone(),
                            created_at: ak.metadata.created_at,
                            last_used: None,
                            expires_at: None,
                        },
                        rate_limit: None,
                        usage: KvmUsageStats {
                            total_requests: ak.usage.total_requests,
                            successful_requests: ak.usage.successful_requests,
                            failed_requests: ak.usage.failed_requests,
                            rate_limited_count: 0,
                            last_request_at: None,
                            usage_percent: ak.usage.usage_percent as f64,
                        },
                    })
                    .collect(),
                default_model: None,
                models: provider_config.models.iter().map(|m| m.name.clone()).collect(),
                headers: HashMap::new(),
            };

            let provider_name = provider_config.name.clone();
            let state = Arc::new(KeyRotationState::new());
            
            let store = manager.store.clone();
            let rotation_states = manager.rotation_states.clone();
            
            tokio::spawn(async move {
                let mut s = store.write().await;
                s.providers.insert(provider_name.clone(), kvm_provider);
                
                let mut rs = rotation_states.write().await;
                rs.insert(provider_name, state);
            });
        }

        manager
    }

    pub fn get_templates(&self) -> Vec<(&str, &ProviderTemplate)> {
        self.provider_templates
            .iter()
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    pub async fn add_provider_with_template(
        &self,
        provider_name: &str,
        template_name: &str,
        keys: Vec<String>,
    ) -> Result<()> {
        let template = self
            .provider_templates
            .get(template_name)
            .ok_or_else(|| anyhow::anyhow!("Template not found"))?;

        let provider = KvmProvider {
            name: provider_name.to_string(),
            base_url: Some(template.base_url.clone()),
            auth_method: template.auth_method.clone(),
            keys: keys
                .into_iter()
                .enumerate()
                .map(|(i, key)| KvmKey {
                    id: uuid::Uuid::new_v4().to_string(),
                    key,
                    enabled: true,
                    priority: (i + 1) as u32,
                    metadata: KvmKeyMetadata {
                        name: format!("key-{}", i + 1),
                        description: String::new(),
                        tags: vec![],
                        created_at: Utc::now(),
                        last_used: None,
                        expires_at: None,
                    },
                    rate_limit: None,
                    usage: KvmUsageStats::default(),
                })
                .collect(),
            default_model: template.default_models.first().cloned(),
            models: template.default_models.clone(),
            headers: template.headers.clone(),
        };

        let state = Arc::new(KeyRotationState::new());
        
        let store = self.store.clone();
        let rotation_states = self.rotation_states.clone();
        let provider_name_owned = provider_name.to_string();
        
        tokio::spawn(async move {
            let mut s = store.write().await;
            s.providers.insert(provider_name_owned.clone(), provider);
            
            let mut rs = rotation_states.write().await;
            rs.insert(provider_name_owned, state);
        });

        Ok(())
    }

    pub async fn add_custom_provider(
        &self,
        provider_name: &str,
        base_url: &str,
        auth_method: &str,
        keys: Vec<String>,
        models: Vec<String>,
        headers: HashMap<String, String>,
    ) -> Result<()> {
        let provider = KvmProvider {
            name: provider_name.to_string(),
            base_url: Some(base_url.to_string()),
            auth_method: auth_method.to_string(),
            keys: keys
                .into_iter()
                .enumerate()
                .map(|(i, key)| KvmKey {
                    id: uuid::Uuid::new_v4().to_string(),
                    key,
                    enabled: true,
                    priority: (i + 1) as u32,
                    metadata: KvmKeyMetadata {
                        name: format!("key-{}", i + 1),
                        description: String::new(),
                        tags: vec![],
                        created_at: Utc::now(),
                        last_used: None,
                        expires_at: None,
                    },
                    rate_limit: None,
                    usage: KvmUsageStats::default(),
                })
                .collect(),
            default_model: models.first().cloned(),
            models,
            headers,
        };

        let state = Arc::new(KeyRotationState::new());
        
        let store = self.store.clone();
        let rotation_states = self.rotation_states.clone();
        let provider_name_owned = provider_name.to_string();
        
        tokio::spawn(async move {
            let mut s = store.write().await;
            s.providers.insert(provider_name_owned.clone(), provider);
            
            let mut rs = rotation_states.write().await;
            rs.insert(provider_name_owned, state);
        });

        Ok(())
    }

    pub async fn get_next_key(&self, provider_name: &str) -> Option<KvmKey> {
        let store = self.store.read().await;
        let provider = store.providers.get(provider_name)?;

        if provider.keys.is_empty() {
            return None;
        }

        let rotation_states = self.rotation_states.read().await;
        let state = rotation_states.get(provider_name)?;

        let enabled_keys: Vec<_> = provider.keys.iter().filter(|k| k.enabled).collect();

        if enabled_keys.is_empty() {
            return None;
        }

        let strategy = &store.rotation.strategy;
        let index = match strategy {
            KvmRotationStrategy::RoundRobin => {
                state.current_key_index.fetch_add(1, Ordering::Relaxed) % enabled_keys.len()
            }
            KvmRotationStrategy::Priority => 0,
            KvmRotationStrategy::UsageBased => {
                let mut sorted: Vec<_> = enabled_keys.iter().enumerate().collect();
                sorted.sort_by(|a, b| {
                    a.1.usage
                        .usage_percent
                        .partial_cmp(&b.1.usage.usage_percent)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                sorted[0].0
            }
            KvmRotationStrategy::ErrorBased => {
                let mut sorted: Vec<_> = enabled_keys.iter().enumerate().collect();
                sorted.sort_by(|a, b| {
                    let rate_a = if a.1.usage.total_requests > 0 {
                        a.1.usage.failed_requests as f64 / a.1.usage.total_requests as f64
                    } else {
                        0.0
                    };
                    let rate_b = if b.1.usage.total_requests > 0 {
                        b.1.usage.failed_requests as f64 / b.1.usage.total_requests as f64
                    } else {
                        0.0
                    };
                    rate_a.partial_cmp(&rate_b).unwrap_or(std::cmp::Ordering::Equal)
                });
                sorted[0].0
            }
            KvmRotationStrategy::HealthBased | KvmRotationStrategy::Adaptive => {
                if state.is_healthy.load(Ordering::Relaxed) {
                    state.current_key_index.load(Ordering::Relaxed) % enabled_keys.len()
                } else {
                    let idx = (state.current_key_index.load(Ordering::Relaxed) + 1)
                        % enabled_keys.len();
                    state.current_key_index.store(idx, Ordering::Relaxed);
                    idx
                }
            }
        };

        enabled_keys.get(index).cloned().cloned()
    }

    pub async fn report_key_success(&self, provider_name: &str, key_id: &str) {
        let mut store = self.store.write().await;
        if let Some(provider) = store.providers.get_mut(provider_name) {
            if let Some(key) = provider.keys.iter_mut().find(|k| k.id == key_id) {
                key.usage.successful_requests += 1;
                key.usage.total_requests += 1;
                key.usage.last_request_at = Some(Utc::now());
                key.metadata.last_used = Some(Utc::now());
            }
        }
        
        let rotation_states = self.rotation_states.read().await;
        if let Some(state) = rotation_states.get(provider_name) {
            state.consecutive_failures.store(0, Ordering::Relaxed);
            state.is_healthy.store(true, Ordering::Relaxed);
        }
    }

    pub async fn report_key_failure(
        &self,
        provider_name: &str,
        key_id: &str,
        is_rate_limit: bool,
    ) {
        let failure_threshold = {
            let store = self.store.read().await;
            store.rotation.failure_threshold
        };
        
        let mut store = self.store.write().await;
        let provider_keys_len = if let Some(provider) = store.providers.get_mut(provider_name) {
            if let Some(key) = provider.keys.iter_mut().find(|k| k.id == key_id) {
                key.usage.failed_requests += 1;
                key.usage.total_requests += 1;

                if is_rate_limit {
                    key.usage.rate_limited_count += 1;
                }
            }
            provider.keys.len()
        } else {
            return;
        };

        let rotation_states = self.rotation_states.read().await;
        if let Some(state) = rotation_states.get(provider_name) {
            let failures = state.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
            
            if failures >= failure_threshold as u64 {
                state.is_healthy.store(false, Ordering::Relaxed);
                
                let idx = (state.current_key_index.load(Ordering::Relaxed) + 1)
                    % provider_keys_len.max(1);
                state.current_key_index.store(idx, Ordering::Relaxed);
                state.total_switches.fetch_add(1, Ordering::Relaxed);
                state.last_rotation_time.store(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    Ordering::Relaxed,
                );
            }
        }
    }

    pub async fn rotate_key(&self, provider_name: &str) -> Option<KvmKey> {
        let rotation_states = self.rotation_states.read().await;
        if let Some(state) = rotation_states.get(provider_name) {
            let store = self.store.read().await;
            
            if let Some(provider) = store.providers.get(provider_name) {
                let current = state.current_key_index.load(Ordering::Relaxed);
                let next = (current + 1) % provider.keys.len().max(1);
                state.current_key_index.store(next, Ordering::Relaxed);
                state.total_switches.fetch_add(1, Ordering::Relaxed);
                state.last_rotation_time.store(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    Ordering::Relaxed,
                );
            }
        }

        self.get_next_key(provider_name).await
    }

    pub async fn get_provider_keys(&self, provider_name: &str) -> Vec<KvmKey> {
        let store = self.store.read().await;
        store
            .providers
            .get(provider_name)
            .map(|p| p.keys.clone())
            .unwrap_or_default()
    }

    pub async fn add_key(&self, provider_name: &str, key: String) -> Result<()> {
        let store = self.store.clone();
        let mut s = store.write().await;

        if let Some(provider) = s.providers.get_mut(provider_name) {
            let new_key = KvmKey {
                id: uuid::Uuid::new_v4().to_string(),
                key,
                enabled: true,
                priority: (provider.keys.len() + 1) as u32,
                metadata: KvmKeyMetadata {
                    name: format!("key-{}", provider.keys.len() + 1),
                    description: String::new(),
                    tags: vec![],
                    created_at: Utc::now(),
                    last_used: None,
                    expires_at: None,
                },
                rate_limit: None,
                usage: KvmUsageStats::default(),
            };
            provider.keys.push(new_key);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Provider {} not found", provider_name))
        }
    }

    pub async fn remove_key(&self, provider_name: &str, key_id: &str) -> Result<()> {
        let store = self.store.clone();
        let mut s = store.write().await;

        if let Some(provider) = s.providers.get_mut(provider_name) {
            provider.keys.retain(|k| k.id != key_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Provider {} not found", provider_name))
        }
    }

    pub async fn disable_key(&self, provider_name: &str, key_id: &str) -> Result<()> {
        let store = self.store.clone();
        let mut s = store.write().await;

        if let Some(provider) = s.providers.get_mut(provider_name) {
            if let Some(key) = provider.keys.iter_mut().find(|k| k.id == key_id) {
                key.enabled = false;
                return Ok(());
            }
            Err(anyhow::anyhow!("Key {} not found", key_id))
        } else {
            Err(anyhow::anyhow!("Provider {} not found", provider_name))
        }
    }

    pub async fn enable_key(&self, provider_name: &str, key_id: &str) -> Result<()> {
        let store = self.store.clone();
        let mut s = store.write().await;

        if let Some(provider) = s.providers.get_mut(provider_name) {
            if let Some(key) = provider.keys.iter_mut().find(|k| k.id == key_id) {
                key.enabled = true;
                return Ok(());
            }
            Err(anyhow::anyhow!("Key {} not found", key_id))
        } else {
            Err(anyhow::anyhow!("Provider {} not found", provider_name))
        }
    }

    pub async fn save_to_file(&self, path: &str) -> Result<()> {
        let store = self.store.read().await;
        let json = serde_json::to_string_pretty(&*store)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub async fn load_from_file(&self, path: &str) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let store: KvmKeyStore = serde_json::from_str(&content)?;
        *self.store.write().await = store;
        Ok(())
    }

    pub async fn get_rotation_stats(&self, provider_name: &str) -> Option<KvmRotationStats> {
        let store = self.store.read().await;
        let rotation_states = self.rotation_states.read().await;
        let state = rotation_states.get(provider_name)?;

        Some(KvmRotationStats {
            current_key_index: state.current_key_index.load(Ordering::Relaxed),
            consecutive_failures: state.consecutive_failures.load(Ordering::Relaxed),
            is_healthy: state.is_healthy.load(Ordering::Relaxed),
            total_switches: state.total_switches.load(Ordering::Relaxed),
            last_rotation_time: state.last_rotation_time.load(Ordering::Relaxed),
            total_keys: store.providers.get(provider_name).map(|p| p.keys.len()).unwrap_or(0),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvmRotationStats {
    pub current_key_index: usize,
    pub consecutive_failures: u64,
    pub is_healthy: bool,
    pub total_switches: u64,
    pub last_rotation_time: u64,
    pub total_keys: usize,
}

pub fn create_provider_from_kvm(
    provider_name: &str,
    api_key: Option<&str>,
    _base_url: Option<&str>,
) -> Result<Box<dyn crate::providers::Provider>> {
    crate::providers::create_provider(provider_name, api_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kvm_key_manager_basic() {
        let manager = KvmKeyManager::new();
        
        manager.add_provider_with_template(
            "test-provider",
            "openrouter",
            vec!["key1".to_string(), "key2".to_string()],
        ).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        let key = manager.get_next_key("test-provider").await;
        assert!(key.is_some());
        assert_eq!(key.unwrap().key, "key1");
    }

    #[tokio::test]
    async fn test_key_rotation_round_robin() {
        let manager = KvmKeyManager::new();
        
        manager.add_provider_with_template(
            "round-robin-test",
            "openrouter",
            vec![
                "key-a".to_string(),
                "key-b".to_string(),
                "key-c".to_string(),
            ],
        ).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        let key1 = manager.get_next_key("round-robin-test").await.unwrap();
        let key2 = manager.get_next_key("round-robin-test").await.unwrap();
        let key3 = manager.get_next_key("round-robin-test").await.unwrap();
        let key4 = manager.get_next_key("round-robin-test").await.unwrap();

        assert_eq!(key1.key, "key-a");
        assert_eq!(key2.key, "key-b");
        assert_eq!(key3.key, "key-c");
        assert_eq!(key4.key, "key-a");
    }

    #[tokio::test]
    async fn test_report_success_and_failure() {
        let manager = KvmKeyManager::new();
        
        manager.add_provider_with_template(
            "stats-test",
            "openrouter",
            vec!["test-key".to_string()],
        ).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        let store = manager.store.read().await;
        let provider = store.providers.get("stats-test").unwrap();
        let key_id = provider.keys[0].id.clone();
        drop(store);

        manager.report_key_success("stats-test", &key_id).await;
        
        let store = manager.store.read().await;
        let key = &store.providers.get("stats-test").unwrap().keys[0];
        assert_eq!(key.usage.successful_requests, 1);
        assert_eq!(key.usage.total_requests, 1);
    }

    #[test]
    fn test_provider_templates() {
        let manager = KvmKeyManager::new();
        let templates = manager.get_templates();
        
        assert!(templates.iter().any(|(name, _)| *name == "openrouter"));
        assert!(templates.iter().any(|(name, _)| *name == "anthropic"));
        assert!(templates.iter().any(|(name, _)| *name == "openai"));
    }

    #[tokio::test]
    async fn test_custom_provider() {
        let manager = KvmKeyManager::new();
        
        let mut headers = HashMap::new();
        headers.insert("X-Custom-Header".to_string(), "value".to_string());

        manager.add_custom_provider(
            "my-custom-provider",
            "https://api.custom.com/v1",
            "bearer",
            vec!["custom-key-1".to_string()],
            vec!["custom-model-1".to_string()],
            headers,
        ).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        let keys = manager.get_provider_keys("my-custom-provider").await;
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].key, "custom-key-1");
    }

    #[tokio::test]
    async fn test_disable_key() {
        let manager = KvmKeyManager::new();
        
        manager.add_provider_with_template(
            "disable-test",
            "openrouter",
            vec!["key1".to_string(), "key2".to_string()],
        ).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        let store = manager.store.read().await;
        let provider = store.providers.get("disable-test").unwrap();
        let key_id = provider.keys[0].id.clone();
        drop(store);

        manager.disable_key("disable-test", &key_id).await.unwrap();

        let key = manager.get_next_key("disable-test").await.unwrap();
        assert_eq!(key.key, "key2");
    }

    #[tokio::test]
    async fn test_priority_rotation() {
        let manager = KvmKeyManager::new();
        
        let store = manager.store.clone();
        let rotation_states = manager.rotation_states.clone();
        {
            let mut s = store.write().await;
            s.rotation.strategy = KvmRotationStrategy::Priority;
            s.providers.insert("priority-test".to_string(), KvmProvider {
                name: "priority-test".to_string(),
                base_url: Some("https://api.test.com".to_string()),
                auth_method: "bearer".to_string(),
                keys: vec![
                    KvmKey {
                        id: "key-1".to_string(),
                        key: "high-priority".to_string(),
                        enabled: true,
                        priority: 1,
                        metadata: KvmKeyMetadata {
                            name: "primary".to_string(),
                            description: String::new(),
                            tags: vec![],
                            created_at: Utc::now(),
                            last_used: None,
                            expires_at: None,
                        },
                        rate_limit: None,
                        usage: KvmUsageStats::default(),
                    },
                    KvmKey {
                        id: "key-2".to_string(),
                        key: "low-priority".to_string(),
                        enabled: true,
                        priority: 2,
                        metadata: KvmKeyMetadata::default(),
                        rate_limit: None,
                        usage: KvmUsageStats::default(),
                    },
                ],
                default_model: None,
                models: vec![],
                headers: HashMap::new(),
            });
        }
        
        let rotation_state = Arc::new(KeyRotationState::new());
        {
            let mut rs = rotation_states.write().await;
            rs.insert("priority-test".to_string(), rotation_state);
        }

        let key = manager.get_next_key("priority-test").await.unwrap();
        assert_eq!(key.key, "high-priority");
    }
}

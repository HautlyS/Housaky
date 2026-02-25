#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use anyhow::Result;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::{Config, FallbackConfig, FallbackProvider, UsageTracker};

#[derive(Debug, Clone)]
pub struct FallbackStatus {
    pub provider_name: String,
    pub priority: u8,
    pub is_active: bool,
    pub usage_percent: u8,
    pub models: Vec<String>,
    pub last_error: Option<String>,
}

pub struct FallbackManager {
    config: Arc<RwLock<FallbackConfig>>,
    current_index: AtomicUsize,
    usage_tracker: UsageTracker,
    daily_token_limit: u64,
}

impl FallbackManager {
    pub fn new(config: FallbackConfig, daily_token_limit: u64) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            current_index: AtomicUsize::new(0),
            usage_tracker: UsageTracker::new(),
            daily_token_limit,
        }
    }

    pub fn from_config(config: &Config) -> Self {
        let daily_tokens = estimate_daily_tokens(config);
        Self::new(config.fallback.clone(), daily_tokens)
    }

    pub async fn get_current_provider(&self) -> Option<FallbackProvider> {
        let config = self.config.read().await;
        if !config.enabled || config.providers.is_empty() {
            return None;
        }

        let idx = self.current_index.load(Ordering::SeqCst);
        config.providers.get(idx).cloned()
    }

    pub async fn get_current_provider_name(&self) -> Option<String> {
        self.get_current_provider().await.map(|p| p.name)
    }

    pub async fn report_usage(&self, tokens: u64) {
        if let Some(provider) = self.get_current_provider().await {
            self.usage_tracker
                .record_request(&provider.name, tokens)
                .await;
        }
    }

    pub async fn report_error(&self, error: &str) {
        if let Some(provider) = self.get_current_provider().await {
            self.usage_tracker.record_error(&provider.name, error).await;
        }
    }

    pub async fn check_and_rotate(&self) -> Option<usize> {
        let config = self.config.read().await;
        if !config.enabled || config.providers.len() <= 1 {
            return None;
        }

        if let Some(provider) = self.get_current_provider().await {
            let usage_percent = self
                .usage_tracker
                .get_usage_percent(&provider.name, self.daily_token_limit)
                .await;

            if usage_percent >= config.rotate_at_percent {
                drop(config);
                return self.rotate_next().await;
            }
        }

        None
    }

    pub async fn rotate_on_rate_limit(&self) -> Option<usize> {
        let config = self.config.read().await;
        if !config.enabled || !config.rotate_on_rate_limit {
            return None;
        }
        drop(config);
        self.rotate_next().await
    }

    pub async fn rotate_next(&self) -> Option<usize> {
        let config = self.config.read().await;
        let len = config.providers.len();
        if len <= 1 {
            return None;
        }

        let current = self.current_index.load(Ordering::SeqCst);
        let next = (current + 1) % len;
        self.current_index.store(next, Ordering::SeqCst);

        tracing::info!(
            from_provider = config.providers.get(current).map(|p| p.name.as_str()),
            to_provider = config.providers.get(next).map(|p| p.name.as_str()),
            "Rotating fallback provider"
        );

        Some(next)
    }

    pub async fn rotate_to(&self, index: usize) -> Result<()> {
        let config = self.config.read().await;
        if index >= config.providers.len() {
            anyhow::bail!("Invalid provider index: {}", index);
        }
        drop(config);

        self.current_index.store(index, Ordering::SeqCst);
        Ok(())
    }

    pub async fn get_all_status(&self) -> Vec<FallbackStatus> {
        let config = self.config.read().await;
        let current_idx = self.current_index.load(Ordering::SeqCst);
        let daily_limit = self.daily_token_limit;

        let mut statuses = Vec::with_capacity(config.providers.len());
        for (i, p) in config.providers.iter().enumerate() {
            let stats = self.usage_tracker.get_stats(&p.name).await;
            let usage_percent = self
                .usage_tracker
                .get_usage_percent(&p.name, daily_limit)
                .await;

            statuses.push(FallbackStatus {
                provider_name: p.name.clone(),
                priority: p.priority,
                is_active: i == current_idx,
                usage_percent,
                models: p.models.clone(),
                last_error: stats.and_then(|s| s.last_error),
            });
        }
        statuses
    }

    pub async fn add_provider(&self, provider: FallbackProvider) -> Result<()> {
        let mut config = self.config.write().await;
        if config.providers.iter().any(|p| p.name == provider.name) {
            anyhow::bail!("Provider {} already exists", provider.name);
        }
        config.providers.push(provider);
        Ok(())
    }

    pub async fn remove_provider(&self, name: &str) -> Result<()> {
        let mut config = self.config.write().await;
        let len_before = config.providers.len();
        config.providers.retain(|p| p.name != name);
        if config.providers.len() == len_before {
            anyhow::bail!("Provider {} not found", name);
        }
        let current = self.current_index.load(Ordering::SeqCst);
        if current >= config.providers.len() && !config.providers.is_empty() {
            self.current_index.store(0, Ordering::SeqCst);
        }
        Ok(())
    }

    pub async fn reset_daily_usage(&self) {
        self.usage_tracker.reset_daily().await;
    }

    pub fn usage_tracker(&self) -> &UsageTracker {
        &self.usage_tracker
    }
}

fn estimate_daily_tokens(config: &Config) -> u64 {
    let cost_limit_cents = f64::from(config.autonomy.max_cost_per_day_cents);
    let avg_cost_per_million_tokens = 10.0;
    let tokens = (cost_limit_cents / 100.0 / avg_cost_per_million_tokens * 1_000_000.0) as u64;
    tokens.max(100_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> FallbackConfig {
        FallbackConfig {
            enabled: true,
            providers: vec![
                FallbackProvider {
                    name: "openrouter".to_string(),
                    base_url: None,
                    api_key_encrypted: None,
                    models: vec!["claude-sonnet-4".to_string()],
                    priority: 1,
                },
                FallbackProvider {
                    name: "anthropic".to_string(),
                    base_url: None,
                    api_key_encrypted: None,
                    models: vec!["claude-3-opus".to_string()],
                    priority: 2,
                },
            ],
            rotate_at_percent: 80,
            rotate_on_rate_limit: true,
        }
    }

    #[tokio::test]
    async fn get_current_provider_returns_first() {
        let manager = FallbackManager::new(test_config(), 1_000_000);
        let provider = manager.get_current_provider().await;
        assert!(provider.is_some());
        assert_eq!(provider.unwrap().name, "openrouter");
    }

    #[tokio::test]
    async fn rotate_next_moves_to_second() {
        let manager = FallbackManager::new(test_config(), 1_000_000);
        let idx = manager.rotate_next().await;
        assert_eq!(idx, Some(1));

        let provider = manager.get_current_provider().await;
        assert_eq!(provider.unwrap().name, "anthropic");
    }

    #[tokio::test]
    async fn rotate_wraps_around() {
        let manager = FallbackManager::new(test_config(), 1_000_000);
        manager.rotate_next().await;
        let idx = manager.rotate_next().await;
        assert_eq!(idx, Some(0));

        let provider = manager.get_current_provider().await;
        assert_eq!(provider.unwrap().name, "openrouter");
    }

    #[tokio::test]
    async fn report_usage_tracks_tokens() {
        let manager = FallbackManager::new(test_config(), 1_000_000);
        manager.report_usage(5000).await;

        let stats = manager.usage_tracker().get_stats("openrouter").await;
        assert!(stats.is_some());
        let stats = stats.unwrap();
        assert_eq!(stats.tokens_today, 5000);
        assert_eq!(stats.requests_today, 1);
    }

    #[tokio::test]
    async fn check_and_rotate_on_threshold() {
        let manager = FallbackManager::new(test_config(), 10_000);
        manager.report_usage(9000).await;

        let rotated = manager.check_and_rotate().await;
        assert_eq!(rotated, Some(1));
    }

    #[tokio::test]
    async fn disabled_fallback_returns_none() {
        let mut config = test_config();
        config.enabled = false;
        let manager = FallbackManager::new(config, 1_000_000);

        let provider = manager.get_current_provider().await;
        assert!(provider.is_none());
    }
}

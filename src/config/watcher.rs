use anyhow::{Context, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, RwLock};

use super::schema::Config;

#[derive(Debug, Clone)]
pub enum ConfigUpdate {
    FullReload(Config),
    AgentConfigChanged,
    ProviderChanged(String),
    ModelChanged(String),
    FallbackRotated(usize),
    ToolsConfigChanged,
    ChannelsConfigChanged,
    GatewayConfigChanged,
    MemoryConfigChanged,
}

pub struct ConfigWatcher {
    watcher: RecommendedWatcher,
    last_reload: Arc<RwLock<Instant>>,
    debounce_ms: u64,
}

impl ConfigWatcher {
    pub fn new(debounce_ms: u64) -> Result<Self> {
        let watcher = notify::recommended_watcher(|_res: Result<Event, notify::Error>| {
            // Events are handled via the broadcast channel
        })
        .context("Failed to create file watcher")?;

        Ok(Self {
            watcher,
            last_reload: Arc::new(RwLock::new(Instant::now())),
            debounce_ms,
        })
    }

    pub fn start(&mut self, config_path: PathBuf) -> Result<broadcast::Receiver<ConfigUpdate>> {
        let (tx, rx) = broadcast::channel(16);
        let tx_clone = tx.clone();
        let last_reload = self.last_reload.clone();
        let debounce_ms = self.debounce_ms;

        let mut watcher =
            notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
                Ok(event) => {
                    if matches!(event.kind, EventKind::Modify(_)) {
                        let now = Instant::now();
                        let last = *last_reload.blocking_read();
                        if u64::try_from(now.duration_since(last).as_millis()).unwrap_or(u64::MAX) > debounce_ms {
                            *last_reload.blocking_write() = now;
                            let _ = tx_clone.send(ConfigUpdate::FullReload(Config::default()));
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("File watcher error: {:?}", e);
                }
            })
            .context("Failed to create event watcher")?;

        watcher
            .watch(&config_path, RecursiveMode::NonRecursive)
            .context("Failed to watch config file")?;

        self.watcher = watcher;
        Ok(rx)
    }

    pub fn stop(&mut self) {
        let _ = self.watcher.unwatch(std::path::Path::new("."));
    }
}

pub struct LiveConfig {
    config: Arc<RwLock<Config>>,
    update_tx: broadcast::Sender<ConfigUpdate>,
}

impl LiveConfig {
    pub fn new(config: Config) -> Self {
        let (update_tx, _) = broadcast::channel(16);
        Self {
            config: Arc::new(RwLock::new(config)),
            update_tx,
        }
    }

    pub async fn get(&self) -> tokio::sync::RwLockReadGuard<'_, Config> {
        self.config.read().await
    }

    pub async fn update<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Config),
    {
        let mut config = self.config.write().await;
        f(&mut config);
        config.save()?;
        let _ = self
            .update_tx
            .send(ConfigUpdate::FullReload(config.clone()));
        Ok(())
    }

    pub async fn reload_from_disk(&self) -> Result<()> {
        let new_config = Config::load_or_init()?;
        let mut config = self.config.write().await;
        *config = new_config;
        let _ = self
            .update_tx
            .send(ConfigUpdate::FullReload(config.clone()));
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ConfigUpdate> {
        self.update_tx.subscribe()
    }

    pub fn inner(&self) -> Arc<RwLock<Config>> {
        self.config.clone()
    }
}

pub struct UsageTracker {
    usage: Arc<RwLock<HashMap<String, UsageStats>>>,
}

#[derive(Debug, Clone, Default)]
pub struct UsageStats {
    pub requests_today: u64,
    pub tokens_today: u64,
    pub last_error: Option<String>,
    pub last_request: Option<Instant>,
}

impl UsageTracker {
    pub fn new() -> Self {
        Self {
            usage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record_request(&self, provider: &str, tokens: u64) {
        let mut usage = self.usage.write().await;
        let stats = usage.entry(provider.to_string()).or_default();
        stats.requests_today += 1;
        stats.tokens_today += tokens;
        stats.last_request = Some(Instant::now());
    }

    pub async fn record_error(&self, provider: &str, error: &str) {
        let mut usage = self.usage.write().await;
        let stats = usage.entry(provider.to_string()).or_default();
        stats.last_error = Some(error.to_string());
    }

    pub async fn get_stats(&self, provider: &str) -> Option<UsageStats> {
        self.usage.read().await.get(provider).cloned()
    }

    pub async fn get_usage_percent(&self, provider: &str, daily_limit_tokens: u64) -> u8 {
        let usage = self.usage.read().await;
        if let Some(stats) = usage.get(provider) {
            if daily_limit_tokens == 0 {
                return 0;
            }
            let percent = (stats.tokens_today as f64 / daily_limit_tokens as f64 * 100.0).clamp(0.0, 100.0) as u8;
            return percent;
        }
        0
    }

    pub async fn reset_daily(&self) {
        let mut usage = self.usage.write().await;
        for stats in usage.values_mut() {
            stats.requests_today = 0;
            stats.tokens_today = 0;
        }
    }
}

impl Default for UsageTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn usage_tracker_records_requests() {
        let tracker = UsageTracker::new();
        tracker.record_request("openrouter", 1000).await;
        tracker.record_request("openrouter", 500).await;

        let stats = tracker.get_stats("openrouter").await.unwrap();
        assert_eq!(stats.requests_today, 2);
        assert_eq!(stats.tokens_today, 1500);
    }

    #[tokio::test]
    async fn usage_tracker_calculates_percent() {
        let tracker = UsageTracker::new();
        tracker.record_request("test", 8000).await;

        let percent = tracker.get_usage_percent("test", 10_000).await;
        assert_eq!(percent, 80);
    }

    #[tokio::test]
    async fn live_config_update() {
        let config = Config::default();
        let live = LiveConfig::new(config);

        {
            let c = live.get().await;
            assert_eq!(c.default_temperature, 0.7);
        }

        live.update(|c| {
            c.default_temperature = 0.5;
        })
        .await
        .ok();

        let c = live.get().await;
        assert_eq!(c.default_temperature, 0.5);
    }
}

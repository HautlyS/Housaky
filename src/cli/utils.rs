//! CLI utility functions for Housaky

use std::path::PathBuf;
use std::time::Duration;

use crate::config::Config;

/// Get API key from keys manager or fall back to config
pub fn get_api_key_from_keys_manager_or_config(config: &Config) -> Option<String> {
    let manager = crate::keys_manager::manager::get_global_keys_manager();
    let lock_result = manager.store.try_read();
    if let Ok(store) = lock_result {
        if let Some(first_provider) = store.providers.values().next() {
            if let Some(first_key) = first_provider.keys.first() {
                return Some(first_key.key.clone());
            }
        }
    }
    config.api_key.clone()
}

/// Check if the daemon is running on the default port
pub fn is_daemon_running() -> bool {
    is_daemon_running_on("127.0.0.1:8080")
}

/// Check if a daemon is running on a specific address
pub fn is_daemon_running_on(addr: &str) -> bool {
    use std::net::TcpStream;

    if let Ok(stream) = TcpStream::connect_timeout(
        &addr.parse().expect("Invalid daemon address"),
        Duration::from_secs(1),
    ) {
        let _ = stream.shutdown(std::net::Shutdown::Both);
        return true;
    }
    false
}

/// Check if this is a first-time run (no valid config exists).
/// Returns `(true, None)` when onboarding is needed, or `(false, Some(config))` when
/// a usable config already exists so the caller does not re-load it.
pub fn is_first_run() -> (bool, Option<Config>) {
    let home = directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let config_path = home.join(".housaky").join("config.toml");

    // First run if config file doesn't exist
    if !config_path.exists() {
        return (true, None);
    }

    // Try to load existing config and check if it has essential settings
    match Config::load_or_init() {
        Ok(config) => {
            let has_api_key = config.api_key.as_ref().map_or(false, |k| !k.is_empty());
            let has_provider = config
                .default_provider
                .as_ref()
                .map_or(false, |p| !p.is_empty());
            // If neither API key nor provider is set, treat as first run
            if !has_api_key && !has_provider {
                (true, None)
            } else {
                (false, Some(config))
            }
        }
        Err(_) => (true, None),
    }
}

/// Print status of configured channels
pub fn print_channel_status(config: &Config) {
    let channels_config = &config.channels_config;
    let mut active_channels = Vec::new();

    if channels_config.telegram.is_some() {
        active_channels.push("Telegram");
    }
    if channels_config.discord.is_some() {
        active_channels.push("Discord");
    }
    if channels_config.slack.is_some() {
        active_channels.push("Slack");
    }
    if channels_config.whatsapp.is_some() {
        active_channels.push("WhatsApp");
    }
    if channels_config.matrix.is_some() {
        active_channels.push("Matrix");
    }
    if channels_config.imessage.is_some() {
        active_channels.push("iMessage");
    }
    if channels_config.email.is_some() {
        active_channels.push("Email");
    }
    if channels_config.irc.is_some() {
        active_channels.push("IRC");
    }
    if channels_config.lark.is_some() {
        active_channels.push("Lark");
    }
    if channels_config.dingtalk.is_some() {
        active_channels.push("DingTalk");
    }

    if !active_channels.is_empty() {
        println!("📡 Active channels: {}", active_channels.join(", "));
    }

    // Voice integration check
    if std::env::var("ELEVENLABS_API_KEY").is_ok() {
        println!("🎙️  Voice integration: Enabled (ElevenLabs)");
    }
}

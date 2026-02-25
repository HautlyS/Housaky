//! Hook configuration for TOML-based configuration.
//!
//! This module provides structures for configuring hooks through TOML files,
//! including default values and deserialization support.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Configuration for the hooks system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    /// Whether the hooks system is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Directory containing hook definitions
    #[serde(default)]
    pub hooks_dir: Option<PathBuf>,
    /// List of enabled/disabled hooks
    #[serde(default)]
    pub hooks: Vec<HookSetting>,
    /// Global hook configuration
    #[serde(default)]
    pub global: GlobalHookConfig,
}

impl Default for HookConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            hooks_dir: None,
            hooks: Vec::new(),
            global: GlobalHookConfig::default(),
        }
    }
}

/// Individual hook setting in configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookSetting {
    /// The hook identifier
    pub id: String,
    /// Whether this hook is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Priority (lower = earlier execution)
    #[serde(default = "default_priority")]
    pub priority: i32,
    /// Custom configuration for the hook
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

impl HookSetting {
    /// Create a new hook setting.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            enabled: true,
            priority: 100,
            config: HashMap::new(),
        }
    }

    /// Create a disabled hook setting.
    #[must_use]
    pub fn disabled(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            enabled: false,
            priority: 100,
            config: HashMap::new(),
        }
    }
}

/// Global configuration options for all hooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalHookConfig {
    /// Whether to continue on hook errors
    #[serde(default = "default_true")]
    pub continue_on_error: bool,
    /// Maximum execution time for a single hook (in milliseconds)
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    /// Maximum number of messages a hook can return
    #[serde(default = "default_max_messages")]
    pub max_messages: usize,
    /// Whether to log hook executions
    #[serde(default)]
    pub logging: HookLoggingConfig,
}

impl Default for GlobalHookConfig {
    fn default() -> Self {
        Self {
            continue_on_error: true,
            timeout_ms: 30_000,
            max_messages: 100,
            logging: HookLoggingConfig::default(),
        }
    }
}

/// Logging configuration for hooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookLoggingConfig {
    /// Whether to enable hook logging
    #[serde(default)]
    pub enabled: bool,
    /// Log level for hook executions
    #[serde(default)]
    pub level: HookLogLevel,
    /// Whether to include event context in logs
    #[serde(default)]
    pub include_context: bool,
}

impl Default for HookLoggingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            level: HookLogLevel::Info,
            include_context: false,
        }
    }
}

/// Log level for hook execution.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookLogLevel {
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

fn default_true() -> bool {
    true
}

fn default_priority() -> i32 {
    100
}

fn default_timeout() -> u64 {
    30_000
}

fn default_max_messages() -> usize {
    100
}

/// Built-in hook identifiers.
pub mod builtin_ids {
    pub const SESSION_MEMORY: &str = "builtin:session_memory";
    pub const BOOT_MD: &str = "builtin:boot_md";
    pub const COMMAND_LOGGER: &str = "builtin:command_logger";
}

/// Configuration for SessionMemoryHook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMemoryConfig {
    /// Directory to save session memories
    pub save_dir: PathBuf,
    /// Maximum number of sessions to keep in memory
    #[serde(default = "default_max_sessions")]
    pub max_sessions: usize,
    /// Whether to save on /reset command
    #[serde(default = "default_true")]
    pub save_on_reset: bool,
    /// Whether to save on /new command
    #[serde(default = "default_true")]
    pub save_on_new: bool,
}

impl Default for SessionMemoryConfig {
    fn default() -> Self {
        Self {
            save_dir: PathBuf::from("~/.housaky/sessions"),
            max_sessions: 100,
            save_on_reset: true,
            save_on_new: true,
        }
    }
}

/// Configuration for BootMdHook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootMdConfig {
    /// Extra directories to search for bootstrap files
    #[serde(default)]
    pub extra_dirs: Vec<PathBuf>,
    /// File patterns to match
    #[serde(default = "default_patterns")]
    pub patterns: Vec<String>,
    /// Whether to recurse into subdirectories
    #[serde(default)]
    pub recursive: bool,
}

impl Default for BootMdConfig {
    fn default() -> Self {
        Self {
            extra_dirs: Vec::new(),
            patterns: default_patterns(),
            recursive: true,
        }
    }
}

fn default_patterns() -> Vec<String> {
    vec![
        "bootstrap.md".to_string(),
        ".housaky/bootstrap.md".to_string(),
    ]
}

/// Configuration for CommandLoggerHook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandLoggerConfig {
    /// Directory to save command logs
    #[serde(default)]
    pub log_dir: Option<PathBuf>,
    /// Whether to log command output
    #[serde(default = "default_true")]
    pub log_output: bool,
    /// Maximum log file size (in bytes)
    #[serde(default = "default_max_log_size")]
    pub max_log_size: usize,
    /// Whether to rotate logs
    #[serde(default = "default_true")]
    pub rotate: bool,
}

impl Default for CommandLoggerConfig {
    fn default() -> Self {
        Self {
            log_dir: None,
            log_output: true,
            max_log_size: 10 * 1024 * 1024, // 10MB
            rotate: true,
        }
    }
}

fn default_max_sessions() -> usize {
    100
}

fn default_max_log_size() -> usize {
    10 * 1024 * 1024
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_config_defaults() {
        let config = HookConfig::default();
        assert!(config.enabled);
        assert!(config.hooks_dir.is_none());
        assert!(config.hooks.is_empty());
    }

    #[test]
    fn test_hook_setting_new() {
        let setting = HookSetting::new("test-hook");
        assert_eq!(setting.id, "test-hook");
        assert!(setting.enabled);
    }

    #[test]
    fn test_hook_setting_disabled() {
        let setting = HookSetting::disabled("test-hook");
        assert_eq!(setting.id, "test-hook");
        assert!(!setting.enabled);
    }

    #[test]
    fn test_global_config_defaults() {
        let config = GlobalHookConfig::default();
        assert!(config.continue_on_error);
        assert_eq!(config.timeout_ms, 30_000);
        assert_eq!(config.max_messages, 100);
    }

    #[test]
    fn test_session_memory_config_defaults() {
        let config = SessionMemoryConfig::default();
        assert_eq!(config.max_sessions, 100);
        assert!(config.save_on_reset);
        assert!(config.save_on_new);
    }

    #[test]
    fn test_boot_md_config_defaults() {
        let config = BootMdConfig::default();
        assert!(config.recursive);
        assert!(!config.patterns.is_empty());
    }

    #[test]
    fn test_toml_deserialization() {
        let toml = r#"
[global]
continue_on_error = true
timeout_ms = 5000
max_messages = 50

[[hooks]]
id = "builtin:session_memory"
enabled = true
priority = 50
"#;
        let config: HookConfig = toml::from_str(toml).unwrap();
        assert!(config.enabled);
        assert!(!config.hooks.is_empty());
        assert_eq!(config.hooks[0].id, "builtin:session_memory");
    }
}

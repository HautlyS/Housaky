//! Built-in hooks for Housaky.
//!
//! This module provides several built-in hooks that extend Housaky's functionality:
//! - [`SessionMemoryHook`] - Saves session context on `/new` or `/reset`
//! - [`BootMdHook`] - Loads extra bootstrap files on startup
//! - [`CommandLoggerHook`] - Logs executed commands

use crate::hooks::config::{BootMdConfig, CommandLoggerConfig, SessionMemoryConfig};
use crate::hooks::types::{Hook, HookEvent, HookEventType, HookResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Hook that saves session context on /new or /reset commands.
///
/// This hook intercepts session-related commands and saves the current
/// session context to disk, allowing for session persistence and recovery.
#[derive(Debug)]
pub struct SessionMemoryHook {
    config: SessionMemoryConfig,
    state: Arc<RwLock<SessionState>>,
}

#[derive(Debug, Default)]
struct SessionState {
    current_session: Option<String>,
    context: HashMap<String, serde_json::Value>,
}

impl SessionMemoryHook {
    /// Create a new session memory hook with the given configuration.
    #[must_use]
    pub fn new(config: SessionMemoryConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(SessionState::default())),
        }
    }

    /// Create a new session memory hook with default configuration.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(SessionMemoryConfig::default())
    }

    /// Save the current session to disk.
    async fn save_session(&self, session_key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let state = self.state.read().await;
        let context = &state.context;

        if context.is_empty() {
            debug!("No context to save for session '{}'", session_key);
            return Ok(());
        }

        let save_path = self.get_save_path(session_key);

        // Ensure parent directory exists
        if let Some(parent) = save_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let session_data = SessionData {
            session_key: session_key.to_string(),
            timestamp: Utc::now(),
            context: context.clone(),
        };

        let json = serde_json::to_string_pretty(&session_data)?;
        fs::write(&save_path, json).await?;

        info!("Saved session '{}' to {:?}", session_key, save_path);
        Ok(())
    }

    /// Get the path where session data should be saved.
    #[must_use]
    fn get_save_path(&self, session_key: &str) -> PathBuf {
        let mut path = self.config.save_dir.clone();
        // Expand ~ to home directory
        if path.starts_with("~") {
            if let Ok(home) = std::env::var("HOME") {
                path = PathBuf::from(home).join(path.strip_prefix("~").unwrap_or(path.as_path()));
            }
        }
        path.push(format!("{}.json", session_key));
        path
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SessionData {
    session_key: String,
    timestamp: DateTime<Utc>,
    context: HashMap<String, serde_json::Value>,
}

#[async_trait]
impl Hook for SessionMemoryHook {
    fn id(&self) -> &str {
        "builtin:session_memory"
    }

    fn name(&self) -> &str {
        "Session Memory Hook"
    }

    fn events(&self) -> Vec<(HookEventType, Vec<String>)> {
        let mut events = Vec::new();

        if self.config.save_on_new {
            events.push((HookEventType::Session, vec!["new".to_string()]));
        }

        if self.config.save_on_reset {
            events.push((HookEventType::Session, vec!["reset".to_string()]));
        }

        // Also handle session start/end
        events.push((HookEventType::Session, vec!["start".to_string()]));
        events.push((HookEventType::Session, vec!["end".to_string()]));

        events
    }

    async fn handle(&self, event: HookEvent) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        let action = &event.action;

        match action.as_str() {
            "new" | "reset" => {
                // Save current session before creating/resetting
                if let Some(session_key) = &event.session_key {
                    // Save existing session first
                    if let Err(e) = self.save_session(session_key).await {
                        warn!("Failed to save session: {}", e);
                    }

                    // Update state for new session
                    let mut state = self.state.write().await;
                    state.current_session = Some(session_key.clone());
                    state.context.clear();

                    // Load context if any provided
                    if let Some(context) = &event.context {
                        state.context = context.as_object()
                            .map(|m| m.iter()
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect())
                            .unwrap_or_default();
                    }
                }

                Ok(HookResult::continue_result())
            }
            "start" => {
                if let Some(session_key) = &event.session_key {
                    let mut state = self.state.write().await;
                    state.current_session = Some(session_key.clone());

                    // Try to load existing session
                    let save_path = self.get_save_path(session_key);
                    if save_path.exists() {
                        match fs::read_to_string(&save_path).await {
                            Ok(json) => {
                                if let Ok(data) = serde_json::from_str::<SessionData>(&json) {
                                    state.context = data.context;
                                    info!("Loaded existing session '{}'", session_key);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to load session '{}': {}", session_key, e);
                            }
                        }
                    }
                }
                Ok(HookResult::continue_result())
            }
            "end" => {
                if let Some(session_key) = &event.session_key {
                    if let Err(e) = self.save_session(session_key).await {
                        warn!("Failed to save session on end: {}", e);
                    }

                    let mut state = self.state.write().await;
                    state.current_session = None;
                    state.context.clear();
                }
                Ok(HookResult::continue_result())
            }
            _ => Ok(HookResult::continue_result()),
        }
    }

    fn priority(&self) -> i32 {
        50 // Run early
    }
}

/// Hook that loads extra bootstrap files on startup.
///
/// This hook searches for and loads additional markdown bootstrap files
/// from configurable directories, allowing for extended startup behavior.
#[derive(Debug)]
pub struct BootMdHook {
    config: BootMdConfig,
    loaded_files: Arc<RwLock<Vec<PathBuf>>>,
}

impl BootMdHook {
    /// Create a new boot MD hook with the given configuration.
    #[must_use]
    pub fn new(config: BootMdConfig) -> Self {
        Self {
            config,
            loaded_files: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a new boot MD hook with default configuration.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(BootMdConfig::default())
    }

    /// Search for and load bootstrap files.
    async fn load_bootstrap_files(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut contents = Vec::new();

        for dir in &self.config.extra_dirs {
            let dir = self.expand_path(dir);

            if !dir.exists() {
                debug!("Bootstrap directory does not exist: {:?}", dir);
                continue;
            }

            self.search_directory(&dir, &mut contents).await?;
        }

        Ok(contents)
    }

    /// Expand ~ in paths to home directory.
    #[must_use]
    fn expand_path(&self, path: &PathBuf) -> PathBuf {
        if path.starts_with("~") {
            if let Ok(home) = std::env::var("HOME") {
                return PathBuf::from(home).join(path.strip_prefix("~").unwrap_or(path.as_path()));
            }
        }
        path.clone()
    }

    /// Search a directory for bootstrap files.
    async fn search_directory(
        &self,
        dir: &PathBuf,
        contents: &mut Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut dirs_to_process = vec![dir.clone()];

        while let Some(current_dir) = dirs_to_process.pop() {
            let mut entries = fs::read_dir(&current_dir).await?;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();

                if path.is_dir() && self.config.recursive {
                    dirs_to_process.push(path);
                } else if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "md" || ext == "markdown" {
                            if self.matches_patterns(&path) {
                                match fs::read_to_string(&path).await {
                                    Ok(content) => {
                                        info!("Loaded bootstrap file: {:?}", path);
                                        contents.push(content);
                                        self.loaded_files.write().await.push(path.clone());
                                    }
                                    Err(e) => {
                                        warn!("Failed to read bootstrap file {:?}: {}", path, e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if a path matches any of the configured patterns.
    #[must_use]
    fn matches_patterns(&self, path: &PathBuf) -> bool {
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        self.config.patterns.iter().any(|pattern| {
            if pattern.contains('*') {
                // Simple wildcard matching
                let prefix = pattern.trim_start_matches('*');
                let suffix = pattern.trim_end_matches('*');
                if prefix.is_empty() {
                    filename.ends_with(suffix)
                } else if suffix.is_empty() {
                    filename.starts_with(prefix)
                } else {
                    filename.starts_with(prefix) && filename.ends_with(suffix)
                }
            } else {
                filename == *pattern
            }
        })
    }
}

#[async_trait]
impl Hook for BootMdHook {
    fn id(&self) -> &str {
        "builtin:boot_md"
    }

    fn name(&self) -> &str {
        "Bootstrap MD Hook"
    }

    fn events(&self) -> Vec<(HookEventType, Vec<String>)> {
        vec![(HookEventType::Session, vec!["start".to_string()])]
    }

    async fn handle(&self, event: HookEvent) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        if event.action != "start" {
            return Ok(HookResult::continue_result());
        }

        match self.load_bootstrap_files().await {
            Ok(contents) => {
                if contents.is_empty() {
                    Ok(HookResult::continue_result())
                } else {
                    Ok(HookResult::with_messages(contents))
                }
            }
            Err(e) => {
                error!("Failed to load bootstrap files: {}", e);
                Ok(HookResult::continue_result())
            }
        }
    }

    fn priority(&self) -> i32 {
        10 // Run very early to load bootstrap files
    }
}

/// Hook that logs executed commands.
///
/// This hook captures command execution events and logs them
/// to a file or standard output for debugging and auditing.
#[derive(Debug)]
pub struct CommandLoggerHook {
    config: CommandLoggerConfig,
    log_file: Arc<RwLock<Option<PathBuf>>>,
}

impl CommandLoggerHook {
    /// Create a new command logger hook with the given configuration.
    #[must_use]
    pub fn new(config: CommandLoggerConfig) -> Self {
        Self {
            config,
            log_file: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new command logger hook with default configuration.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(CommandLoggerConfig::default())
    }

    /// Log a command to the configured output.
    async fn log_command(&self, event: &HookEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let timestamp = event.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        let session = event.session_key.as_deref().unwrap_or("unknown");
        let command = event.context.as_ref()
            .and_then(|c| c.get("command"))
            .and_then(|c| c.as_str())
            .unwrap_or("unknown");

        let log_line = format!(
            "[{}] [{}] [session:{}] {}\n",
            timestamp, event.event_type, session, command
        );

        if let Some(ref log_dir) = self.config.log_dir {
            let log_path = self.expand_path(log_dir);
            let mut log_file = self.log_file.write().await;

            // Create log file if needed
            if log_file.is_none() {
                let file_path = log_path.join("commands.log");
                fs::create_dir_all(&log_path).await?;
                *log_file = Some(file_path);
            }

            if let Some(ref path) = *log_file {
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .await?;
                file.write_all(log_line.as_bytes()).await?;
            }
        }

        // Always log to debug output
        debug!("Command logged: {}", log_line.trim());

        Ok(())
    }

    /// Expand ~ in paths to home directory.
    #[must_use]
    fn expand_path(&self, path: &PathBuf) -> PathBuf {
        if path.starts_with("~") {
            if let Ok(home) = std::env::var("HOME") {
                return PathBuf::from(home).join(path.strip_prefix("~").unwrap_or(path.as_path()));
            }
        }
        path.clone()
    }
}

#[async_trait]
impl Hook for CommandLoggerHook {
    fn id(&self) -> &str {
        "builtin:command_logger"
    }

    fn name(&self) -> &str {
        "Command Logger Hook"
    }

    fn events(&self) -> Vec<(HookEventType, Vec<String>)> {
        vec![
            (HookEventType::Command, vec!["execute".to_string()]),
            (HookEventType::Command, vec!["run".to_string()]),
        ]
    }

    async fn handle(&self, event: HookEvent) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.log_output {
            return Ok(HookResult::continue_result());
        }

        if let Err(e) = self.log_command(&event).await {
            warn!("Failed to log command: {}", e);
        }

        Ok(HookResult::continue_result())
    }

    fn priority(&self) -> i32 {
        200 // Run after most hooks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_memory_hook_id() {
        let hook = SessionMemoryHook::with_defaults();
        assert_eq!(hook.id(), "builtin:session_memory");
    }

    #[test]
    fn test_boot_md_hook_id() {
        let hook = BootMdHook::with_defaults();
        assert_eq!(hook.id(), "builtin:boot_md");
    }

    #[test]
    fn test_command_logger_hook_id() {
        let hook = CommandLoggerHook::with_defaults();
        assert_eq!(hook.id(), "builtin:command_logger");
    }

    #[test]
    fn test_session_memory_config_defaults() {
        let config = SessionMemoryConfig::default();
        assert!(config.save_on_new);
        assert!(config.save_on_reset);
    }

    #[tokio::test]
    async fn test_session_memory_hook_handles_new() {
        let hook = SessionMemoryHook::with_defaults();
        let event = HookEvent::new(HookEventType::Session, "new".to_string(), Some("test-session".to_string()));

        let result = hook.handle(event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_boot_md_hook_handles_start() {
        let hook = BootMdHook::with_defaults();
        let event = HookEvent::new(HookEventType::Session, "start".to_string(), None);

        let result = hook.handle(event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_command_logger_hook_handles_execute() {
        let hook = CommandLoggerHook::with_defaults();
        let event = HookEvent::new(
            HookEventType::Command,
            "execute".to_string(),
            Some("test-session".to_string()),
        );

        let result = hook.handle(event).await;
        assert!(result.is_ok());
    }
}

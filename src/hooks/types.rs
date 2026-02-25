//! Core types for the hooks system.
//!
//! This module defines the fundamental types used throughout the hooks system,
//! including event types, event structures, results, errors, and the main Hook trait.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of events that can be triggered in the hooks system.
///
/// Each variant represents a category of events that hooks can respond to.
/// Hooks can register to handle specific event types or specific actions within types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookEventType {
    /// Events related to command execution
    Command,
    /// Events related to session lifecycle
    Session,
    /// Events related to agent operations
    Agent,
    /// Events related to gateway operations
    Gateway,
    /// Events related to messaging
    Message,
}

impl HookEventType {
    /// Returns the string representation of the event type.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Session => "session",
            Self::Agent => "agent",
            Self::Gateway => "gateway",
            Self::Message => "message",
        }
    }

    /// Parse event type from string.
    ///
    /// Returns `None` if the string doesn't match any known event type.
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "command" => Some(Self::Command),
            "session" => Some(Self::Session),
            "agent" => Some(Self::Agent),
            "gateway" => Some(Self::Gateway),
            "message" => Some(Self::Message),
            _ => None,
        }
    }
}

impl std::fmt::Display for HookEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Represents an event that can be handled by hooks.
///
/// An event consists of:
/// - `event_type`: The category of event (Command, Session, etc.)
/// - `action`: A specific action within the event type (e.g., "new", "reset")
/// - `session_key`: Optional session identifier
/// - `context`: Additional JSON-structured data about the event
/// - `timestamp`: When the event occurred
/// - `messages`: Optional messages associated with the event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    /// The category of event
    pub event_type: HookEventType,
    /// Specific action within the event type
    pub action: String,
    /// Optional session identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_key: Option<String>,
    /// Additional context data as JSON value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// Messages associated with the event
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<String>,
}

impl HookEvent {
    /// Create a new hook event with the current timestamp.
    #[must_use]
    pub fn new(event_type: HookEventType, action: String, session_key: Option<String>) -> Self {
        Self {
            event_type,
            action,
            session_key,
            context: None,
            timestamp: Utc::now(),
            messages: Vec::new(),
        }
    }

    /// Create a new hook event with full context.
    #[must_use]
    pub fn with_context(
        event_type: HookEventType,
        action: String,
        session_key: Option<String>,
        context: serde_json::Value,
    ) -> Self {
        Self {
            event_type,
            action,
            session_key,
            context: Some(context),
            timestamp: Utc::now(),
            messages: Vec::new(),
        }
    }

    /// Create a new hook event with messages.
    #[must_use]
    pub fn with_messages(
        event_type: HookEventType,
        action: String,
        session_key: Option<String>,
        messages: Vec<String>,
    ) -> Self {
        Self {
            event_type,
            action,
            session_key,
            context: None,
            timestamp: Utc::now(),
            messages,
        }
    }

    /// Get the full event key (type:action format).
    #[must_use]
    pub fn full_key(&self) -> String {
        format!("{}:{}", self.event_type.as_str(), self.action)
    }
}

/// Result returned by hook handlers.
///
/// Hooks return this struct to indicate:
/// - `messages`: Optional messages to output
/// - `should_continue`: Whether the event processing should continue to other hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    /// Messages to output as a result of handling the event
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<String>,
    /// Whether to continue processing this event with other hooks
    #[serde(default = "default_true")]
    pub should_continue: bool,
}

fn default_true() -> bool {
    true
}

impl HookResult {
    /// Create a new hook result that continues processing.
    #[must_use]
    pub fn continue_result() -> Self {
        Self {
            messages: Vec::new(),
            should_continue: true,
        }
    }

    /// Create a new hook result that stops processing.
    #[must_use]
    pub fn stop() -> Self {
        Self {
            messages: Vec::new(),
            should_continue: false,
        }
    }

    /// Create a new hook result with messages that continues processing.
    #[must_use]
    pub fn with_messages(messages: Vec<String>) -> Self {
        Self {
            messages,
            should_continue: true,
        }
    }

    /// Create a new hook result with a single message.
    #[must_use]
    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            messages: vec![message.into()],
            should_continue: true,
        }
    }
}

/// Error that can occur during hook execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookError {
    /// Human-readable error message
    pub message: String,
    /// The event type that caused the error
    pub event_type: HookEventType,
    /// The action that caused the error
    pub action: String,
}

impl HookError {
    /// Create a new hook error.
    #[must_use]
    pub fn new(message: impl Into<String>, event_type: HookEventType, action: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            event_type,
            action: action.into(),
        }
    }
}

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HookError in {}:{} - {}",
            self.event_type, self.action, self.message
        )
    }
}

impl std::error::Error for HookError {}

/// Trait for implementing custom hooks.
///
/// Implement this trait to create custom handlers for Housaky events.
/// Each hook must have a unique identifier, name, list of events it handles,
/// and a handler method for processing events.
///
/// # Example
///
/// ```ignore
/// use async_trait::async_trait;
/// use housaky::hooks::{Hook, HookEvent, HookEventType, HookResult};
///
/// struct MyHook;
///
/// #[async_trait]
/// impl Hook for MyHook {
///     fn id(&self) -> &str {
///         "my-hook"
///     }
///
///     fn name(&self) -> &str {
///         "My Custom Hook"
///     }
///
///     fn events(&self) -> Vec<(HookEventType, Vec<String>)> {
///         vec![(HookEventType::Command, vec!["execute".to_string()])]
///     }
///
///     async fn handle(&self, event: HookEvent) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
///         Ok(HookResult::continue_result())
///     }
/// }
/// ```
#[async_trait]
pub trait Hook: Send + Sync {
    /// Unique identifier for this hook.
    fn id(&self) -> &str;

    /// Human-readable name for this hook.
    fn name(&self) -> &str;

    /// List of events this hook handles.
    ///
    /// Returns a vector of tuples, where each tuple contains:
    /// - The event type
    /// - List of specific actions (empty means all actions)
    fn events(&self) -> Vec<(HookEventType, Vec<String>)>;

    /// Handle an event.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to handle
    ///
    /// # Returns
    ///
    /// Returns a `HookResult` on success, or an error if handling failed.
    async fn handle(
        &self,
        event: HookEvent,
    ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Get the priority of this hook.
    ///
    /// Lower values are executed first. Default priority is 100.
    fn priority(&self) -> i32 {
        100
    }

    /// Check if this hook is enabled.
    ///
    /// Disabled hooks are not executed but remain registered.
    fn enabled(&self) -> bool {
        true
    }
}

/// A boxed hook for dynamic dispatch.
pub type BoxedHook = Box<dyn Hook>;

/// Configuration for a hook loaded from config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookRegistrationConfig {
    /// Unique identifier for the hook
    pub id: String,
    /// Whether the hook is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Priority of the hook (lower = earlier)
    #[serde(default = "default_priority")]
    pub priority: i32,
    /// Custom configuration for the hook
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

fn default_priority() -> i32 {
    100
}

impl HookRegistrationConfig {
    /// Create a new hook registration config.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            enabled: true,
            priority: 100,
            config: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_from_str() {
        assert_eq!(HookEventType::from_str("command"), Some(HookEventType::Command));
        assert_eq!(HookEventType::from_str("session"), Some(HookEventType::Session));
        assert_eq!(HookEventType::from_str("invalid"), None);
    }

    #[test]
    fn test_event_type_as_str() {
        assert_eq!(HookEventType::Command.as_str(), "command");
        assert_eq!(HookEventType::Session.as_str(), "session");
    }

    #[test]
    fn test_hook_event_full_key() {
        let event = HookEvent::new(HookEventType::Command, "execute".to_string(), None);
        assert_eq!(event.full_key(), "command:execute");
    }

    #[test]
    fn test_hook_result_continue() {
        let result = HookResult::continue_result();
        assert!(result.should_continue);
        assert!(result.messages.is_empty());
    }

    #[test]
    fn test_hook_result_stop() {
        let result = HookResult::stop();
        assert!(!result.should_continue);
    }
}

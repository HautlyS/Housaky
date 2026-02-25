//! Housaky hooks system for extending functionality through event-driven handlers.
//!
//! The hooks system provides a flexible way to extend Housaky's functionality by
//! registering handlers that respond to various events in the system lifecycle.
//!
//! # Overview
//!
//! - [`HookEventType`] - Types of events that can be triggered
//! - [`HookEvent`] - Represents a single event with metadata
//! - [`HookResult`] - Result returned by hook handlers
//! - [`Hook`] - Trait for implementing custom hooks
//! - [`HookRegistry`] - Central registry for managing hooks
//! - [`HookConfig`] - Configuration for hooks via TOML
//!
//! # Example
//!
//! ```ignore
//! use housaky::hooks::{HookRegistry, HookEvent, HookEventType};
//!
//! let registry = HookRegistry::new();
//! registry.register(my_hook).await?;
//!
//! registry.trigger(HookEvent::new(
//!     HookEventType::Command,
//!     "execute".to_string(),
//!     None,
//! )).await?;
//! ```
//!
//! # Built-in Hooks
//!
//! - [`SessionMemoryHook`] - Saves session context on `/new` or `/reset`
//! - [`BootMdHook`] - Loads extra bootstrap files on startup
//! - [`CommandLoggerHook`] - Logs executed commands

pub mod builtins;
pub mod config;
pub mod registry;
pub mod types;

pub use builtins::{BootMdHook, CommandLoggerHook, SessionMemoryHook};
pub use config::HookConfig;
pub use registry::HookRegistry;
pub use types::{Hook, HookError, HookEvent, HookEventType, HookResult};

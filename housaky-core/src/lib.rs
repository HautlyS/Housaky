//! Housaky Core - Quantum-inspired primitives and state management
//!
//! This crate provides the foundational types and utilities for the Housaky AGI system,
//! including quantum-inspired state representations, cryptographic primitives, and
//! basic data structures used throughout the system.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod crypto;
pub mod orchestrator;
pub mod quantum;
pub mod types;

pub use crypto::*;
pub use orchestrator::*;
pub use quantum::*;
pub use types::*;

/// Version of the housaky-core crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the core crate with tracing
pub fn init() {
    tracing::info!("housaky-core v{} initialized", VERSION);
}

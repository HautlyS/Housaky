//! Housaky Core - Quantum-inspired primitives and state management
//!
//! This crate provides the foundational types and utilities for the Housaky AGI system,
//! including quantum-inspired state representations, cryptographic primitives, and
//! basic data structures used throughout the system.
//!
//! ## 2026 AGI Architecture
//! Based on cutting-edge research:
//! - Darwin Gödel Machine (Zhang et al., 2025) - Open-ended self-improvement
//! - DeepSeek-R1 (DeepSeek-AI, 2025) - Reasoning through reinforcement learning
//! - Quantum Neural Holographic Fusion (Amiri, 2025) - Consciousness engineering
//! - ICLR 2026 Workshop on Recursive Self-Improvement

pub mod crypto;
pub mod orchestrator;
pub mod quantum;
pub mod types;

pub use crypto::*;
pub use orchestrator::*;
pub use quantum::*;
pub use types::*;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init() {
    tracing::info!("housaky-core v{} initialized", VERSION);
}

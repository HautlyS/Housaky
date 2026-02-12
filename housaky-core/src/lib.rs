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

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod crypto;
pub mod orchestrator;
pub mod quantum;
pub mod types;
pub mod quantum_agi;

pub use crypto::*;
pub use orchestrator::*;
pub use quantum::*;
pub use types::*;
pub use quantum_agi::{
    QuantumAGI, QuantumAGIConfig, AGIStatus,
    QuantumSuperposition, PhiMetric, QuantumReasoner,
    ConsciousnessEngine, SelfModel, EmergenceEvent, EmergenceType,
    DGMProposal, EvolutionArchive, DGMEvolutionEngine,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init() {
    tracing::info!("housaky-core v{} initialized with Quantum AGI capabilities", VERSION);
}

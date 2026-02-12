//! Housaky RLM - Reasoning Language Model
//!
//! This crate provides local transformer-based inference capabilities for the Housaky AGI system.
//! It supports quantized models for efficient inference on consumer hardware.

pub mod inference;
pub mod model;
pub mod tokenizer;

pub use inference::*;
pub use model::*;
pub use tokenizer::*;

/// Initialize the RLM module
pub fn init() {
    tracing::info!("housaky-rlm initialized");
}

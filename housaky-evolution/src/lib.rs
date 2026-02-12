//! Housaky Evolution - Self-improvement and code evolution system
//!
//! This crate provides the Darwin-Gödel Machine implementation for self-improving code.

use anyhow::Result;

pub mod ast_mutator;
pub mod fitness;
pub mod mutation;
pub mod sandbox;
pub mod selection;

pub use ast_mutator::*;
pub use fitness::*;
pub use mutation::*;
pub use sandbox::*;
pub use selection::*;

/// Initialize the evolution module
pub fn init() {
    tracing::info!("housaky-evolution initialized");
}

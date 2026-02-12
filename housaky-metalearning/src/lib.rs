//! Housaky Meta-Learning - Meta-learning algorithms
//!
//! This crate implements MAML, Reptile, and Neural Architecture Search.

use anyhow::Result;

pub mod hyperparameters;
pub mod maml;
pub mod nas;
pub mod reptile;

pub use hyperparameters::*;
pub use maml::*;
pub use nas::*;
pub use reptile::*;

/// Initialize the meta-learning module
pub fn init() {
    tracing::info!("housaky-metalearning initialized");
}

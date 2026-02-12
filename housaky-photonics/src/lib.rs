//! Housaky Photonics - Photon detection and light-based processing
//!
//! This crate provides capabilities for detecting and processing photons
//! using cameras and optical sensors.

pub mod detector;
pub mod encoding;
pub mod processor;

pub use detector::*;
pub use encoding::*;
pub use processor::*;

/// Initialize the photonics module
pub fn init() {
    tracing::info!("housaky-photonics initialized");
}

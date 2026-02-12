//! Housaky Li-Fi - Li-Fi communication protocol implementation
//!
//! This crate provides Li-Fi (Light Fidelity) communication capabilities
//! for high-speed data transmission using light.

use anyhow::Result;

pub mod hardware;
pub mod link;
pub mod protocol;
pub mod receiver;
pub mod transmitter;

pub use hardware::*;
pub use link::*;
pub use protocol::*;
pub use receiver::*;
pub use transmitter::*;

/// Initialize the Li-Fi module
pub fn init() {
    tracing::info!("housaky-lifi initialized");
}

//! Housaky P2P - Peer-to-peer networking layer
//!
//! This crate provides P2P networking using libp2p.

use anyhow::Result;

pub mod discovery;
pub mod gossip;
pub mod network;
pub mod protocol;

pub use discovery::*;
pub use gossip::*;
pub use network::*;
pub use protocol::*;

/// Initialize the P2P module
pub fn init() {
    tracing::info!("housaky-p2p initialized");
}

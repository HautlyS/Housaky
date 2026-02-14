//! Housaky P2P - Peer-to-peer networking layer
//!
//! This crate provides P2P networking using libp2p.

pub mod discovery;
pub mod gossip;
pub mod network;
pub mod protocol;
pub mod dht;

pub use discovery::*;
pub use gossip::*;
pub use network::*;
pub use protocol::*;
pub use dht::*;

/// Initialize the P2P module
pub fn init() {
    tracing::info!("housaky-p2p initialized");
}

//! Peer discovery mechanisms
use anyhow::Result;
use libp2p::PeerId;

/// Discovery service
pub struct DiscoveryService {
    known_peers: Vec<PeerId>,
}

impl DiscoveryService {
    pub fn new() -> Self {
        Self {
            known_peers: Vec::new(),
        }
    }

    pub fn add_peer(&mut self, peer: PeerId) {
        if !self.known_peers.contains(&peer) {
            self.known_peers.push(peer);
        }
    }

    pub fn get_peers(&self) -> &[PeerId] {
        &self.known_peers
    }
}

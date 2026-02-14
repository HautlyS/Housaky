//! P2P network management

use anyhow::Result;
use libp2p::{
    futures::StreamExt,
    gossipsub::{self, IdentTopic},
    identity,
    kad::{self, store::MemoryStore},
    mdns::{Mdns, MdnsConfig},
    noise,
    swarm::{NetworkBehaviour, SwarmBuilder},
    tcp::TokioTcpConfig,
    yamux, Multiaddr, PeerId, Swarm,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

use housaky_core::types::NetworkAddress;

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Listen addresses
    pub listen_addrs: Vec<String>,
    /// Bootstrap peers
    pub bootstrap_peers: Vec<String>,
    /// Enable mdns
    pub enable_mdns: bool,
    /// Max peers
    pub max_peers: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addrs: vec!["/ip4/0.0.0.0/tcp/0".to_string()],
            bootstrap_peers: vec![],
            enable_mdns: true,
            max_peers: 50,
        }
    }
}

/// Housaky network behavior
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "HousakyEvent")]
pub struct HousakyBehavior {
    /// Gossipsub for pub/sub
    pub gossipsub: gossipsub::Behaviour,
    /// Kademlia DHT
    pub kad: kad::Behaviour<MemoryStore>,
    /// mDNS for local discovery
    pub mdns: Mdns,
}

/// Network events
#[derive(Debug)]
pub enum HousakyEvent {
    Gossipsub(gossipsub::Event),
    Kad(kad::Event),
    Mdns(mdns::Event),
}

impl From<gossipsub::Event> for HousakyEvent {
    fn from(event: gossipsub::Event) -> Self {
        HousakyEvent::Gossipsub(event)
    }
}

impl From<kad::Event> for HousakyEvent {
    fn from(event: kad::Event) -> Self {
        HousakyEvent::Kad(event)
    }
}

impl From<mdns::Event> for HousakyEvent {
    fn from(event: mdns::Event) -> Self {
        HousakyEvent::Mdns(event)
    }
}

/// P2P network node
pub struct P2PNetwork {
    /// Local peer ID
    pub local_peer_id: PeerId,
    /// Network swarm
    swarm: Swarm<HousakyBehavior>,
    /// Event receiver
    event_rx: mpsc::Receiver<NetworkEvent>,
    /// Connected peers
    connected_peers: Vec<PeerId>,
}

/// Network events
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    MessageReceived {
        peer: PeerId,
        topic: String,
        data: Vec<u8>,
    },
    Subscribed {
        peer: PeerId,
        topic: String,
    },
}

impl P2PNetwork {
    /// Create a new P2P network node
    pub async fn new(config: NetworkConfig) -> Result<(Self, mpsc::Sender<NetworkCommand>)> {
        // Generate identity
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        tracing::info!("Local peer ID: {}", local_peer_id);

        // Create transport
        let transport = TokioTcpConfig::new()
            .upgrade(libp2p::core::upgrade::Version::V1)
            .authenticate(noise::NoiseAuthenticated::xx(&local_key)?)
            .multiplex(yamux::YamuxConfig::default())
            .boxed();

        // Create gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()?;

        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )?;

        // Create Kademlia
        let store = MemoryStore::new(local_peer_id);
        let kad = kad::Behaviour::new(local_peer_id, store);

        // Create mDNS
        let mdns = Mdns::new(MdnsConfig::default()).await?;

        // Create behavior
        let behavior = HousakyBehavior {
            gossipsub,
            kad,
            mdns,
        };

        // Create swarm
        let mut swarm = SwarmBuilder::new(transport, behavior, local_peer_id)
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build();

        // Listen on addresses
        for addr in &config.listen_addrs {
            let multiaddr: Multiaddr = addr.parse()?;
            swarm.listen_on(multiaddr)?;
        }

        // Connect to bootstrap peers
        for peer in &config.bootstrap_peers {
            let addr: Multiaddr = peer.parse()?;
            swarm.dial(addr)?;
        }

        let (event_tx, event_rx) = mpsc::channel(100);
        let (cmd_tx, mut cmd_rx) = mpsc::channel(100);

        let network = Self {
            local_peer_id,
            swarm,
            event_rx,
            connected_peers: Vec::new(),
        };

        Ok((network, cmd_tx))
    }

    /// Subscribe to a topic
    pub fn subscribe(&mut self, topic: &str) -> Result<()> {
        let topic = IdentTopic::new(topic);
        self.swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
        Ok(())
    }

    /// Publish a message to a topic
    pub fn publish(&mut self, topic: &str, data: Vec<u8>) -> Result<()> {
        let topic = IdentTopic::new(topic);
        self.swarm.behaviour_mut().gossipsub.publish(topic, data)?;
        Ok(())
    }

    /// Get connected peers
    pub fn peers(&self) -> &[PeerId] {
        &self.connected_peers
    }

    /// Run the network event loop
    pub async fn run(self) -> Result<()> {
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    match event {
                        HousakyEvent::Gossipsub(event) => {
                            if let gossipsub::Event::Message { propagation_source, message, .. } = event {
                                tracing::debug!("Received message from {}", propagation_source);
                            }
                        }
                        HousakyEvent::Kad(event) => {
                            tracing::debug!("Kademlia event: {:?}", event);
                        }
                        HousakyEvent::Mdns(event) => {
                            match event {
                                mdns::Event::Discovered(peers) => {
                                    for (peer, addr) in peers {
                                        tracing::info!("Discovered peer {} at {}", peer, addr);
                                        self.swarm.behaviour_mut().kad.add_address(&peer, addr);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Network commands
#[derive(Debug)]
pub enum NetworkCommand {
    Subscribe { topic: String },
    Publish { topic: String, data: Vec<u8> },
    Connect { addr: String },
    Disconnect { peer: PeerId },
    GetPeers,
}

/// Network statistics
#[derive(Debug, Clone, Default)]
pub struct NetworkStats {
    /// Number of connected peers
    pub connected_peers: usize,
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_config() {
        let config = NetworkConfig::default();
        assert!(!config.listen_addrs.is_empty());
    }

    #[test]
    fn test_network_stats() {
        let stats = NetworkStats::default();
        assert_eq!(stats.connected_peers, 0);
    }
}

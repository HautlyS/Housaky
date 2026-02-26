use super::{FederationConfig, KnowledgeDelta, Peer, SyncResult};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransportProtocol {
    Http,
    Quic,
    Libp2p,
    WebSocket,
    UnixSocket,
}

impl std::fmt::Display for TransportProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportProtocol::Http => write!(f, "http"),
            TransportProtocol::Quic => write!(f, "quic"),
            TransportProtocol::Libp2p => write!(f, "libp2p"),
            TransportProtocol::WebSocket => write!(f, "websocket"),
            TransportProtocol::UnixSocket => write!(f, "unix"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    pub protocol: TransportProtocol,
    pub bind_addr: String,
    pub port: u16,
    pub max_frame_size: usize,
    pub connection_timeout_ms: u64,
    pub keepalive_interval_ms: u64,
    pub max_concurrent_streams: usize,
    pub tls_enabled: bool,
    pub compression: bool,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            protocol: TransportProtocol::Http,
            bind_addr: "0.0.0.0".to_string(),
            port: 9090,
            max_frame_size: 10 * 1024 * 1024,
            connection_timeout_ms: 5_000,
            keepalive_interval_ms: 30_000,
            max_concurrent_streams: 100,
            tls_enabled: false,
            compression: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerConnection {
    pub peer_id: String,
    pub address: String,
    pub protocol: TransportProtocol,
    pub connected_at: chrono::DateTime<Utc>,
    pub last_ping_ms: Option<u64>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub active: bool,
}

impl PeerConnection {
    pub fn new(peer_id: &str, address: &str, protocol: TransportProtocol) -> Self {
        Self {
            peer_id: peer_id.to_string(),
            address: address.to_string(),
            protocol,
            connected_at: Utc::now(),
            last_ping_ms: None,
            bytes_sent: 0,
            bytes_received: 0,
            messages_sent: 0,
            messages_received: 0,
            active: true,
        }
    }

    pub fn record_send(&mut self, bytes: usize) {
        self.bytes_sent += bytes as u64;
        self.messages_sent += 1;
    }

    pub fn record_receive(&mut self, bytes: usize) {
        self.bytes_received += bytes as u64;
        self.messages_received += 1;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportMessage {
    Ping { from: String, timestamp: i64 },
    Pong { from: String, timestamp: i64, ping_timestamp: i64 },
    DeltaSync { delta: KnowledgeDelta },
    PeerAnnounce { peer: Peer },
    PeerDiscover { requester: String },
    PeerList { peers: Vec<Peer> },
    Ack { message_id: String },
    Error { code: u32, message: String },
}

impl TransportMessage {
    pub fn message_type(&self) -> &str {
        match self {
            TransportMessage::Ping { .. } => "ping",
            TransportMessage::Pong { .. } => "pong",
            TransportMessage::DeltaSync { .. } => "delta_sync",
            TransportMessage::PeerAnnounce { .. } => "peer_announce",
            TransportMessage::PeerDiscover { .. } => "peer_discover",
            TransportMessage::PeerList { .. } => "peer_list",
            TransportMessage::Ack { .. } => "ack",
            TransportMessage::Error { .. } => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub avg_latency_ms: f64,
    pub failed_sends: u64,
}

pub struct NetworkTransport {
    pub config: TransportConfig,
    pub connections: Arc<RwLock<HashMap<String, PeerConnection>>>,
    pub http_client: reqwest::Client,
    pub stats: Arc<RwLock<NetworkStats>>,
    pub local_id: String,
    pub pending_messages: Arc<RwLock<Vec<(String, TransportMessage)>>>,
}

impl NetworkTransport {
    pub fn new(config: TransportConfig, local_id: &str) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.connection_timeout_ms))
            .build()
            .unwrap_or_default();

        Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            http_client,
            stats: Arc::new(RwLock::new(NetworkStats {
                total_connections: 0,
                active_connections: 0,
                total_bytes_sent: 0,
                total_bytes_received: 0,
                total_messages_sent: 0,
                total_messages_received: 0,
                avg_latency_ms: 0.0,
                failed_sends: 0,
            })),
            local_id: local_id.to_string(),
            pending_messages: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn connect(&self, peer_id: &str, address: &str) -> Result<()> {
        let reachable = self.ping_peer(address).await.is_ok();
        if !reachable {
            warn!("Transport: cannot reach peer '{}' at '{}'", peer_id, address);
        }

        let conn = PeerConnection::new(peer_id, address, self.config.protocol.clone());
        self.connections.write().await.insert(peer_id.to_string(), conn);

        let mut stats = self.stats.write().await;
        stats.total_connections += 1;
        stats.active_connections += 1;

        info!("Transport: connected to peer '{}' via {}", peer_id, self.config.protocol);
        Ok(())
    }

    pub async fn disconnect(&self, peer_id: &str) {
        let mut conns = self.connections.write().await;
        if let Some(conn) = conns.get_mut(peer_id) {
            conn.active = false;
        }
        let mut stats = self.stats.write().await;
        stats.active_connections = stats.active_connections.saturating_sub(1);
        info!("Transport: disconnected from peer '{}'", peer_id);
    }

    pub async fn send_delta(&self, peer_id: &str, delta: &KnowledgeDelta) -> Result<SyncResult> {
        let start = std::time::Instant::now();

        let conns = self.connections.read().await;
        let conn_addr = conns.get(peer_id).map(|c| c.address.clone());
        drop(conns);

        let Some(address) = conn_addr else {
            self.pending_messages
                .write()
                .await
                .push((peer_id.to_string(), TransportMessage::DeltaSync { delta: delta.clone() }));
            return Ok(SyncResult {
                peer_id: peer_id.to_string(),
                entries_received: 0,
                entries_sent: delta.additions.len() + delta.modifications.len(),
                conflicts_resolved: 0,
                duration_ms: start.elapsed().as_millis() as u64,
                success: false,
            });
        };

        let message = TransportMessage::DeltaSync { delta: delta.clone() };
        let payload = serde_json::to_vec(&message)?;
        let payload_size = payload.len();

        match self.http_send(&address, "sync", payload).await {
            Ok(_response) => {
                let mut conns = self.connections.write().await;
                if let Some(c) = conns.get_mut(peer_id) {
                    c.record_send(payload_size);
                }
                let mut stats = self.stats.write().await;
                stats.total_bytes_sent += payload_size as u64;
                stats.total_messages_sent += 1;
                let dur = start.elapsed().as_millis() as f64;
                let n = stats.total_messages_sent as f64;
                stats.avg_latency_ms = (stats.avg_latency_ms * (n - 1.0) + dur) / n;

                debug!("Transport: sent delta to '{}' ({} entries)", peer_id, delta.additions.len());

                Ok(SyncResult {
                    peer_id: peer_id.to_string(),
                    entries_received: 0,
                    entries_sent: delta.additions.len() + delta.modifications.len(),
                    conflicts_resolved: 0,
                    duration_ms: start.elapsed().as_millis() as u64,
                    success: true,
                })
            }
            Err(e) => {
                warn!("Transport: failed to send delta to '{}': {}", peer_id, e);
                self.stats.write().await.failed_sends += 1;
                self.pending_messages
                    .write()
                    .await
                    .push((peer_id.to_string(), TransportMessage::DeltaSync { delta: delta.clone() }));
                Ok(SyncResult {
                    peer_id: peer_id.to_string(),
                    entries_received: 0,
                    entries_sent: 0,
                    conflicts_resolved: 0,
                    duration_ms: start.elapsed().as_millis() as u64,
                    success: false,
                })
            }
        }
    }

    pub async fn send_peer_announce(&self, address: &str, peer: &Peer) -> Result<()> {
        let message = TransportMessage::PeerAnnounce { peer: peer.clone() };
        let payload = serde_json::to_vec(&message)?;
        self.http_send(address, "announce", payload).await.map(|_| ())
    }

    pub async fn discover_peers(&self, known_address: &str) -> Result<Vec<Peer>> {
        let message = TransportMessage::PeerDiscover { requester: self.local_id.clone() };
        let payload = serde_json::to_vec(&message)?;

        match self.http_send(known_address, "discover", payload).await {
            Ok(body) => {
                if let Ok(peers) = serde_json::from_str::<Vec<Peer>>(&body) {
                    info!("Transport: discovered {} peers from '{}'", peers.len(), known_address);
                    Ok(peers)
                } else {
                    Ok(vec![])
                }
            }
            Err(e) => {
                warn!("Transport: peer discovery from '{}' failed: {}", known_address, e);
                Ok(vec![])
            }
        }
    }

    pub async fn ping_peer(&self, address: &str) -> Result<u64> {
        let start = std::time::Instant::now();
        let message = TransportMessage::Ping {
            from: self.local_id.clone(),
            timestamp: Utc::now().timestamp_millis(),
        };
        let payload = serde_json::to_vec(&message)?;
        self.http_send(address, "ping", payload).await?;
        let latency_ms = start.elapsed().as_millis() as u64;

        let mut conns = self.connections.write().await;
        if let Some(conn) = conns.values_mut().find(|c| c.address == address) {
            conn.last_ping_ms = Some(latency_ms);
        }

        Ok(latency_ms)
    }

    pub async fn flush_pending(&self) -> usize {
        let pending: Vec<(String, TransportMessage)> =
            self.pending_messages.write().await.drain(..).collect();
        let mut flushed = 0;

        for (peer_id, message) in pending {
            let conns = self.connections.read().await;
            let addr = conns.get(&peer_id).map(|c| c.address.clone());
            drop(conns);

            if let Some(address) = addr {
                let payload = match serde_json::to_vec(&message) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let endpoint = message.message_type();
                if self.http_send(&address, endpoint, payload).await.is_ok() {
                    flushed += 1;
                } else {
                    self.pending_messages
                        .write()
                        .await
                        .push((peer_id, message));
                }
            }
        }

        if flushed > 0 {
            debug!("Transport: flushed {} pending messages", flushed);
        }
        flushed
    }

    async fn http_send(&self, address: &str, endpoint: &str, payload: Vec<u8>) -> Result<String> {
        let url = format!("http://{}/housaky/federation/{}", address, endpoint);
        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Housaky-Peer", &self.local_id)
            .body(payload)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.text().await.unwrap_or_default())
        } else {
            anyhow::bail!("HTTP {} from {}", response.status(), url)
        }
    }

    pub async fn active_peer_ids(&self) -> Vec<String> {
        self.connections
            .read()
            .await
            .iter()
            .filter(|(_, c)| c.active)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub async fn stats(&self) -> NetworkStats {
        self.stats.read().await.clone()
    }

    pub async fn update_peer_status(&self, peer_id: &str, latency_ok: bool) {
        let mut conns = self.connections.write().await;
        if let Some(conn) = conns.get_mut(peer_id) {
            conn.active = latency_ok;
        }
    }
}

pub struct FederationTransportLayer {
    pub transport: Arc<NetworkTransport>,
    pub fed_config: Arc<RwLock<FederationConfig>>,
}

impl FederationTransportLayer {
    pub fn new(
        transport_config: TransportConfig,
        fed_config: FederationConfig,
        local_id: &str,
    ) -> Self {
        Self {
            transport: Arc::new(NetworkTransport::new(transport_config, local_id)),
            fed_config: Arc::new(RwLock::new(fed_config)),
        }
    }

    pub async fn connect_to_peers(&self) -> usize {
        let config = self.fed_config.read().await;
        let peers = config.peers.clone();
        drop(config);

        let mut connected = 0;
        for peer_addr in &peers {
            let peer_id = format!("peer-{}", &peer_addr[..peer_addr.len().min(8)]);
            if self.transport.connect(&peer_id, peer_addr).await.is_ok() {
                connected += 1;
            }
        }
        info!("FederationTransport: connected to {}/{} peers", connected, peers.len());
        connected
    }

    pub async fn sync_delta_to_all(&self, delta: &KnowledgeDelta) -> Vec<SyncResult> {
        let peer_ids = self.transport.active_peer_ids().await;
        let mut results = Vec::new();
        for peer_id in &peer_ids {
            let result = self.transport.send_delta(peer_id, delta).await.unwrap_or(SyncResult {
                peer_id: peer_id.clone(),
                entries_received: 0,
                entries_sent: 0,
                conflicts_resolved: 0,
                duration_ms: 0,
                success: false,
            });
            results.push(result);
        }
        results
    }

    pub async fn health_check_all(&self) -> HashMap<String, bool> {
        let peer_ids = self.transport.active_peer_ids().await;
        let mut health = HashMap::new();
        for peer_id in peer_ids {
            let conns = self.transport.connections.read().await;
            let addr = conns.get(&peer_id).map(|c| c.address.clone());
            drop(conns);

            if let Some(address) = addr {
                let ok = self.transport.ping_peer(&address).await.is_ok();
                self.transport.update_peer_status(&peer_id, ok).await;
                health.insert(peer_id, ok);
            }
        }
        health
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_tracking() {
        let conn = PeerConnection::new("p1", "localhost:9090", TransportProtocol::Http);
        assert_eq!(conn.peer_id, "p1");
        assert!(conn.active);
        assert_eq!(conn.messages_sent, 0);
    }

    #[test]
    fn test_transport_message_serialization() {
        let msg = TransportMessage::Ping {
            from: "local".to_string(),
            timestamp: 1234567890,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Ping"));
        let decoded: TransportMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.message_type(), "ping");
    }

    #[test]
    fn test_transport_config_default() {
        let cfg = TransportConfig::default();
        assert_eq!(cfg.protocol, TransportProtocol::Http);
        assert_eq!(cfg.port, 9090);
    }

    #[tokio::test]
    async fn test_pending_message_queue() {
        let transport = NetworkTransport::new(TransportConfig::default(), "local-1");
        let delta = KnowledgeDelta {
            source_peer: "local-1".to_string(),
            timestamp: Utc::now(),
            version: 1,
            additions: vec![],
            modifications: vec![],
            deletions: vec![],
        };
        let result = transport.send_delta("nonexistent-peer", &delta).await.unwrap();
        assert!(!result.success);
        assert_eq!(transport.pending_messages.read().await.len(), 1);
    }
}

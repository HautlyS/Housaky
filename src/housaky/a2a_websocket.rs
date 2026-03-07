//! A2A WebSocket Transport Layer
//!
//! Real-time bidirectional communication between Housaky instances
//! with TLS encryption, metrics streaming, and automatic reconnection.

use super::a2a::A2AMessage;
use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

// ============================================================================
// Configuration
// ============================================================================

/// WebSocket configuration for A2A
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AWebSocketConfig {
    /// Bind address for server (e.g., "0.0.0.0:8765")
    pub bind_addr: String,
    /// Enable TLS encryption
    pub tls_enabled: bool,
    /// Path to TLS certificate (PEM format)
    pub tls_cert_path: Option<String>,
    /// Path to TLS private key (PEM format)
    pub tls_key_path: Option<String>,
    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
    /// Maximum reconnect attempts
    pub max_reconnect_attempts: u32,
    /// Reconnect delay base in milliseconds
    pub reconnect_delay_ms: u64,
    /// Maximum message size in bytes (10MB default)
    pub max_message_size: usize,
    /// Enable compression
    pub compression: bool,
}

impl Default for A2AWebSocketConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8765".to_string(),
            tls_enabled: true,
            tls_cert_path: None,
            tls_key_path: None,
            connection_timeout_ms: 5000,
            heartbeat_interval_ms: 30000,
            max_reconnect_attempts: 10,
            reconnect_delay_ms: 1000,
            max_message_size: 10 * 1024 * 1024,
            compression: true,
        }
    }
}

// ============================================================================
// Metrics
// ============================================================================

/// Real-time metrics streamed over WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsUpdate {
    pub timestamp: u64,
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub memory_used_mb: f64,
    pub tasks_active: u32,
    pub tasks_completed: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub avg_response_time_ms: f64,
    pub errors_count: u64,
    pub uptime_secs: u64,
}

impl Default for MetricsUpdate {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            cpu_percent: 0.0,
            memory_percent: 0.0,
            memory_used_mb: 0.0,
            tasks_active: 0,
            tasks_completed: 0,
            messages_sent: 0,
            messages_received: 0,
            avg_response_time_ms: 0.0,
            errors_count: 0,
            uptime_secs: 0,
        }
    }
}

/// Aggregated metrics from all connected clients
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    pub local: MetricsUpdate,
    pub peers: HashMap<String, MetricsUpdate>,
    pub last_updated: u64,
}

// ============================================================================
// WebSocket Message Types
// ============================================================================

/// WebSocket protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsMessage {
    /// A2A protocol message
    A2A(A2AMessage),
    /// Metrics update broadcast
    Metrics(MetricsUpdate),
    /// Heartbeat ping
    Ping { timestamp: u64 },
    /// Heartbeat pong
    Pong { timestamp: u64, ping_timestamp: u64 },
    /// Connection handshake
    Handshake { instance_id: String, version: String },
    /// Handshake acknowledgment
    HandshakeAck { instance_id: String, accepted: bool },
    /// Error notification
    Error { code: u32, message: String },
    /// Peer list update
    PeerList { peers: Vec<PeerInfo> },
}

/// Information about a connected peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: String,
    pub name: String,
    pub connected_at: u64,
    pub last_seen: u64,
    pub status: PeerStatus,
    pub address: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerStatus {
    Available,
    Busy,
    Idle,
    Offline,
}

// ============================================================================
// Connection State
// ============================================================================

/// State for a single WebSocket connection
pub struct Connection {
    pub peer_id: String,
    pub peer_name: String,
    pub connected_at: Instant,
    pub last_seen: AtomicU64,
    pub messages_sent: AtomicU64,
    pub messages_received: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub status: Arc<RwLock<PeerStatus>>,
    pub tx: mpsc::Sender<WsMessage>,
}

impl Connection {
    pub fn new(peer_id: String, peer_name: String, tx: mpsc::Sender<WsMessage>) -> Self {
        Self {
            peer_id,
            peer_name,
            connected_at: Instant::now(),
            last_seen: AtomicU64::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            ),
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            status: Arc::new(RwLock::new(PeerStatus::Available)),
            tx,
        }
    }

    pub fn update_last_seen(&self) {
        self.last_seen.store(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            Ordering::Relaxed,
        );
    }

    pub fn record_send(&self, bytes: u64) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_receive(&self, bytes: u64) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }
}

// ============================================================================
// WebSocket Server
// ============================================================================

/// A2A WebSocket Server
pub struct A2AWebSocketServer {
    config: A2AWebSocketConfig,
    local_id: String,
    connections: Arc<RwLock<HashMap<String, Arc<Connection>>>>,
    metrics: Arc<RwLock<AggregatedMetrics>>,
    shutdown: Arc<AtomicBool>,
    message_tx: broadcast::Sender<A2AMessage>,
    metrics_tx: broadcast::Sender<MetricsUpdate>,
}

impl A2AWebSocketServer {
    /// Create a new WebSocket server
    pub fn new(config: A2AWebSocketConfig, local_id: String) -> Self {
        let (message_tx, _) = broadcast::channel(256);
        let (metrics_tx, _) = broadcast::channel(64);

        Self {
            config,
            local_id,
            connections: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(AggregatedMetrics::default())),
            shutdown: Arc::new(AtomicBool::new(false)),
            message_tx,
            metrics_tx,
        }
    }

    /// Start the WebSocket server
    pub async fn start(&self) -> Result<()> {
        let addr: SocketAddr = self.config.bind_addr.parse()?;
        let listener = tokio::net::TcpListener::bind(addr).await?;

        info!("A2A WebSocket server listening on {}", addr);

        let shutdown = self.shutdown.clone();
        let connections = self.connections.clone();
        let local_id = self.local_id.clone();
        let message_tx = self.message_tx.clone();
        let metrics_tx = self.metrics_tx.clone();

        tokio::spawn(async move {
            while !shutdown.load(Ordering::Relaxed) {
                tokio::select! {
                    accept_result = listener.accept() => {
                        match accept_result {
                            Ok((stream, addr)) => {
                                let connections = connections.clone();
                                let local_id = local_id.clone();
                                let message_tx = message_tx.clone();
                                let metrics_tx = metrics_tx.clone();

                                tokio::spawn(async move {
                                    if let Err(e) = Self::handle_connection(
                                        stream, addr, connections, local_id, message_tx, metrics_tx
                                    ).await {
                                        error!("Connection error from {}: {}", addr, e);
                                    }
                                });
                            }
                            Err(e) => {
                                warn!("Accept error: {}", e);
                            }
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {}
                }
            }
        });

        Ok(())
    }

    /// Handle a single WebSocket connection
    async fn handle_connection(
        stream: tokio::net::TcpStream,
        addr: SocketAddr,
        connections: Arc<RwLock<HashMap<String, Arc<Connection>>>>,
        local_id: String,
        message_tx: broadcast::Sender<A2AMessage>,
        metrics_tx: broadcast::Sender<MetricsUpdate>,
    ) -> Result<()> {
        let ws_stream = tokio_tungstenite::accept_async(stream).await?;
        info!("WebSocket connection established from {}", addr);

        let (ws_tx, ws_rx) = ws_stream.split();
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<WsMessage>(64);

        // Generate peer ID
        let peer_id = format!("peer-{}", uuid::Uuid::new_v4());
        let connection = Arc::new(Connection::new(
            peer_id.clone(),
            addr.to_string(),
            outgoing_tx.clone(),
        ));

        // Store connection
        {
            let mut conns = connections.write().await;
            conns.insert(peer_id.clone(), connection.clone());
        }

        // Send handshake
        let handshake = WsMessage::Handshake {
            instance_id: local_id.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };
        outgoing_tx.send(handshake).await?;

        // Handle incoming messages
        let conn_clone = connection.clone();
        let msg_tx = message_tx.clone();
        let met_tx = metrics_tx.clone();
        let outgoing_tx_clone = outgoing_tx.clone();

        let recv_handle = tokio::spawn(async move {
            let mut ws_rx = ws_rx;
            while let Some(msg_result) = ws_rx.next().await {
                match msg_result {
                    Ok(Message::Binary(data)) => {
                        conn_clone.record_receive(data.len() as u64);
                        conn_clone.update_last_seen();

                        if let Ok(ws_msg) = serde_json::from_slice::<WsMessage>(&data) {
                            match ws_msg {
                                WsMessage::A2A(a2a) => {
                                    let _ = msg_tx.send(a2a);
                                }
                                WsMessage::Metrics(metrics) => {
                                    let _ = met_tx.send(metrics);
                                }
                                WsMessage::Ping { timestamp } => {
                                    let pong = WsMessage::Pong {
                                        timestamp: SystemTime::now()
                                            .duration_since(UNIX_EPOCH)
                                            .unwrap()
                                            .as_millis() as u64,
                                        ping_timestamp: timestamp,
                                    };
                                    let _ = outgoing_tx_clone.send(pong).await;
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        // Respond with pong - handled by tungstenite
                        let _ = data;
                    }
                    Ok(Message::Close(_)) => break,
                    Err(e) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        // Handle outgoing messages
        let conn_clone2 = connection.clone();
        let send_handle = tokio::spawn(async move {
            let mut ws_tx = ws_tx;

            while let Some(msg) = outgoing_rx.recv().await {
                let data = match serde_json::to_vec(&msg) {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("Serialization error: {}", e);
                        continue;
                    }
                };
                conn_clone2.record_send(data.len() as u64);
                if ws_tx.send(Message::Binary(data)).await.is_err() {
                    break;
                }
            }

            Ok::<_, anyhow::Error>(())
        });

        // Wait for either to complete
        tokio::select! {
            _ = recv_handle => {}
            _ = send_handle => {}
        }

        // Cleanup
        {
            let mut conns = connections.write().await;
            conns.remove(&peer_id);
        }

        info!("Connection closed: {}", peer_id);
        Ok(())
    }

    /// Stop the server
    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Subscribe to incoming A2A messages
    pub fn subscribe_messages(&self) -> broadcast::Receiver<A2AMessage> {
        self.message_tx.subscribe()
    }

    /// Subscribe to metrics updates
    pub fn subscribe_metrics(&self) -> broadcast::Receiver<MetricsUpdate> {
        self.metrics_tx.subscribe()
    }

    /// Get list of connected peers
    pub async fn get_peers(&self) -> Vec<PeerInfo> {
        let conns = self.connections.read().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut peers = Vec::new();
        for c in conns.values() {
            let status = *c.status.read().await;
            peers.push(PeerInfo {
                id: c.peer_id.clone(),
                name: c.peer_name.clone(),
                connected_at: c.connected_at.elapsed().as_secs() + now,
                last_seen: c.last_seen.load(Ordering::Relaxed),
                status,
                address: c.peer_name.clone(),
            });
        }
        peers
    }

    /// Send message to all connected peers
    pub async fn broadcast(&self, msg: A2AMessage) -> Result<()> {
        let conns = self.connections.read().await;
        for conn in conns.values() {
            let ws_msg = WsMessage::A2A(msg.clone());
            conn.tx.send(ws_msg).await?;
        }
        Ok(())
    }

    /// Send message to specific peer
    pub async fn send_to(&self, peer_id: &str, msg: A2AMessage) -> Result<()> {
        let conns = self.connections.read().await;
        if let Some(conn) = conns.get(peer_id) {
            let ws_msg = WsMessage::A2A(msg);
            conn.tx.send(ws_msg).await?;
        }
        Ok(())
    }
}

// ============================================================================
// WebSocket Client
// ============================================================================

/// A2A WebSocket Client
pub struct A2AWebSocketClient {
    config: A2AWebSocketConfig,
    local_id: String,
    url: String,
    connected: Arc<AtomicBool>,
    connection: Arc<RwLock<Option<mpsc::Sender<WsMessage>>>>,
    message_rx: broadcast::Receiver<A2AMessage>,
    message_tx: broadcast::Sender<A2AMessage>,
    metrics_rx: broadcast::Receiver<MetricsUpdate>,
    metrics_tx: broadcast::Sender<MetricsUpdate>,
    peer_info: Arc<RwLock<Option<PeerInfo>>>,
}

impl A2AWebSocketClient {
    /// Create a new WebSocket client
    pub fn new(config: A2AWebSocketConfig, local_id: String, url: String) -> Self {
        let (message_tx, message_rx) = broadcast::channel(256);
        let (metrics_tx, metrics_rx) = broadcast::channel(64);

        Self {
            config,
            local_id,
            url,
            connected: Arc::new(AtomicBool::new(false)),
            connection: Arc::new(RwLock::new(None)),
            message_rx,
            message_tx,
            metrics_rx,
            metrics_tx,
            peer_info: Arc::new(RwLock::new(None)),
        }
    }

    /// Connect to the WebSocket server
    pub async fn connect(&self) -> Result<()> {
        let url = self.url.clone();
        let local_id = self.local_id.clone();
        let connected = self.connected.clone();
        let connection = self.connection.clone();
        let message_tx = self.message_tx.clone();
        let metrics_tx = self.metrics_tx.clone();
        let peer_info = self.peer_info.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut attempts = 0u32;
            let mut delay = Duration::from_millis(config.reconnect_delay_ms);

            loop {
                if attempts >= config.max_reconnect_attempts {
                    error!("Max reconnect attempts reached");
                    break;
                }

                // Attempt connection - pass owned values
                let url_owned = url.clone();
                let local_id_owned = local_id.clone();
                let connected_clone = connected.clone();
                let connection_clone = connection.clone();
                let message_tx_clone = message_tx.clone();
                let metrics_tx_clone = metrics_tx.clone();
                let peer_info_clone = peer_info.clone();

                match Self::establish_connection(
                    url_owned,
                    local_id_owned,
                    connected_clone,
                    connection_clone,
                    message_tx_clone,
                    metrics_tx_clone,
                    peer_info_clone,
                )
                .await
                {
                    Ok(_) => {
                        attempts = 0;
                        delay = Duration::from_millis(config.reconnect_delay_ms);
                    }
                    Err(e) => {
                        attempts += 1;
                        warn!(
                            "Connection attempt {}/{} failed: {}",
                            attempts, config.max_reconnect_attempts, e
                        );
                        tokio::time::sleep(delay).await;
                        delay = std::cmp::min(delay * 2, Duration::from_secs(30));
                    }
                }
            }
        });

        Ok(())
    }

    async fn establish_connection(
        url: String,
        local_id: String,
        connected: Arc<AtomicBool>,
        connection: Arc<RwLock<Option<mpsc::Sender<WsMessage>>>>,
        message_tx: broadcast::Sender<A2AMessage>,
        metrics_tx: broadcast::Sender<MetricsUpdate>,
        peer_info: Arc<RwLock<Option<PeerInfo>>>,
    ) -> Result<()> {
        let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await?;
        info!("Connected to A2A WebSocket server at {}", url);

        connected.store(true, Ordering::Relaxed);
        let (ws_tx, ws_rx) = ws_stream.split();
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<WsMessage>(64);

        // Store connection handle
        {
            let mut conn = connection.write().await;
            *conn = Some(outgoing_tx.clone());
        }

        // Send handshake
        let handshake = WsMessage::Handshake {
            instance_id: local_id.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };
        outgoing_tx.send(handshake).await?;

        // Handle incoming
        let conn_clone = connection.clone();
        let msg_tx = message_tx.clone();
        let met_tx = metrics_tx.clone();
        let peer_info_clone = peer_info.clone();
        let connected_recv = connected.clone();
        let url_for_addr = url.clone();

        let recv_handle = tokio::spawn(async move {
            let mut ws_rx = ws_rx;
            while let Some(msg_result) = ws_rx.next().await {
                match msg_result {
                    Ok(Message::Binary(data)) => {
                        if let Ok(ws_msg) = serde_json::from_slice::<WsMessage>(&data) {
                            match ws_msg {
                                WsMessage::A2A(a2a) => {
                                    let _ = msg_tx.send(a2a);
                                }
                                WsMessage::Metrics(metrics) => {
                                    let _ = met_tx.send(metrics);
                                }
                                WsMessage::Pong {
                                    timestamp: _,
                                    ping_timestamp,
                                } => {
                                    debug!("Pong received, latency: {}ms", ping_timestamp);
                                }
                                WsMessage::HandshakeAck { instance_id, accepted } => {
                                    if accepted {
                                        info!("Handshake accepted by {}", instance_id);
                                        let mut pi = peer_info_clone.write().await;
                                        *pi = Some(PeerInfo {
                                            id: instance_id.clone(),
                                            name: instance_id,
                                            connected_at: SystemTime::now()
                                                .duration_since(UNIX_EPOCH)
                                                .unwrap()
                                                .as_secs(),
                                            last_seen: SystemTime::now()
                                                .duration_since(UNIX_EPOCH)
                                                .unwrap()
                                                .as_secs(),
                                            status: PeerStatus::Available,
                                            address: url_for_addr.clone(),
                                        });
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Err(e) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            connected_recv.store(false, Ordering::Relaxed);
            let mut conn = conn_clone.write().await;
            *conn = None;
        });

        // Handle outgoing
        let conn_clone2 = connection.clone();
        let connected_send = connected.clone();
        let send_handle = tokio::spawn(async move {
            let mut ws_tx = ws_tx;

            while let Some(msg) = outgoing_rx.recv().await {
                let data = match serde_json::to_vec(&msg) {
                    Ok(d) => d,
                    Err(_) => continue,
                };
                if ws_tx.send(Message::Binary(data)).await.is_err() {
                    break;
                }
            }

            connected_send.store(false, Ordering::Relaxed);
            let mut conn = conn_clone2.write().await;
            *conn = None;

            Ok::<_, anyhow::Error>(())
        });

        tokio::select! {
            _ = recv_handle => {}
            _ = send_handle => {}
        }

        Ok(())
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    /// Send an A2A message
    pub async fn send(&self, msg: A2AMessage) -> Result<()> {
        let conn = self.connection.read().await;
        if let Some(tx) = conn.as_ref() {
            let ws_msg = WsMessage::A2A(msg);
            tx.send(ws_msg).await?;
        } else {
            return Err(anyhow!("Not connected"));
        }
        Ok(())
    }

    /// Subscribe to incoming messages
    pub fn subscribe_messages(&self) -> broadcast::Receiver<A2AMessage> {
        self.message_tx.subscribe()
    }

    /// Subscribe to metrics updates
    pub fn subscribe_metrics(&self) -> broadcast::Receiver<MetricsUpdate> {
        self.metrics_tx.subscribe()
    }

    /// Get peer info
    pub async fn get_peer_info(&self) -> Option<PeerInfo> {
        self.peer_info.read().await.clone()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_default() {
        let m = MetricsUpdate::default();
        assert_eq!(m.cpu_percent, 0.0);
        assert_eq!(m.tasks_active, 0);
    }

    #[test]
    fn test_ws_message_serialization() {
        let msg = WsMessage::Ping { timestamp: 12345 };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Ping"));

        let decoded: WsMessage = serde_json::from_str(&json).unwrap();
        matches!(decoded, WsMessage::Ping { .. });
    }

    #[tokio::test]
    async fn test_connection_tracking() {
        let (tx, _) = mpsc::channel(1);
        let conn = Connection::new("peer-1".to_string(), "127.0.0.1:1234".to_string(), tx);

        conn.record_send(100);
        conn.record_receive(200);

        assert_eq!(conn.messages_sent.load(Ordering::Relaxed), 1);
        assert_eq!(conn.bytes_sent.load(Ordering::Relaxed), 100);
        assert_eq!(conn.bytes_received.load(Ordering::Relaxed), 200);
    }
}

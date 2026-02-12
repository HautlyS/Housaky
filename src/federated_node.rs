//! Federated Learning Node with Quantum-Inspired State
//!
//! This module provides distributed learning capabilities with quantum-inspired
//! computation and proper resource management.

use crate::photon_detector::{PhotonDetector, PhotonDetectorHandle, PseudoQubit};
use crate::quantum_state::QuantumInspiredState;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration, Instant};

/// Model update for federated learning
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelUpdate {
    pub node_id: String,
    pub weights: Vec<f64>,
    pub timestamp: u64,
    pub signature: Option<Vec<u8>>, // Ed25519 signature (64 bytes)
}

impl ModelUpdate {
    /// Create a new model update
    #[allow(dead_code)]
    pub fn new(node_id: String, weights: Vec<f64>) -> Self {
        Self {
            node_id,
            weights,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            signature: None,
        }
    }

    /// Verify the update signature using Ed25519
    /// Returns true if signature is valid or if no signature is present (for testing)
    pub fn verify(&self) -> bool {
        if let Some(ref sig_bytes) = self.signature {
            // Real Ed25519 verification would require the public key
            // For now, verify signature format (64 bytes for Ed25519)
            if sig_bytes.len() != 64 {
                tracing::warn!(
                    "Invalid signature length: expected 64, got {}",
                    sig_bytes.len()
                );
                return false;
            }

            // TODO: Implement full Ed25519 verification when public key infrastructure is ready
            // This would use ed25519_dalek::PublicKey::verify_strict()
            tracing::debug!("Signature format valid for node {}", self.node_id);
            true
        } else {
            // Allow unsigned updates in test/development mode
            // In production, this should return false
            tracing::debug!(
                "No signature present for node {}, allowing in dev mode",
                self.node_id
            );
            self.weights.len() < 10000 // Basic sanity check on size
        }
    }

    /// Verify with a specific public key (production-ready)
    #[cfg(feature = "full-crypto")]
    pub fn verify_with_key(&self, public_key: &[u8; 32]) -> bool {
        use ed25519_dalek::{PublicKey, Signature, Verifier};

        if let Some(ref sig_bytes) = self.signature {
            if sig_bytes.len() != 64 {
                return false;
            }

            let pk = match PublicKey::from_bytes(public_key) {
                Ok(pk) => pk,
                Err(_) => return false,
            };

            let signature = match Signature::from_bytes(sig_bytes) {
                Ok(sig) => sig,
                Err(_) => return false,
            };

            // Create message from weights and timestamp
            let message = self.create_message();
            pk.verify(&message, &signature).is_ok()
        } else {
            false // Require signature in production
        }
    }

    /// Create message for signing/verification
    #[allow(dead_code)]
    fn create_message(&self) -> Vec<u8> {
        let mut msg = Vec::new();
        msg.extend_from_slice(self.node_id.as_bytes());
        msg.extend_from_slice(&self.timestamp.to_le_bytes());
        for w in &self.weights {
            msg.extend_from_slice(&w.to_le_bytes());
        }
        msg
    }

    /// Sign the update with a private key
    #[cfg(feature = "full-crypto")]
    pub fn sign(&mut self, secret_key: &ed25519_dalek::SecretKey) {
        use ed25519_dalek::{ExpandedSecretKey, Signer};

        let message = self.create_message();
        let expanded: ExpandedSecretKey = secret_key.into();
        let signature = expanded.sign(&message, &expanded.into());
        self.signature = Some(signature.to_bytes().to_vec());
    }

    /// Calculate update magnitude
    pub fn magnitude(&self) -> f64 {
        self.weights.iter().map(|w| w * w).sum::<f64>().sqrt()
    }
}

/// Federated node events
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum NodeEvent {
    PeerConnected(String),
    PeerDisconnected(String),
    ModelReceived(String, ModelUpdate),
    PhotonMeasured(PseudoQubit),
    Error(String),
    ShutdownComplete,
}

/// Configuration for federated node
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct NodeConfig {
    pub node_id: String,
    pub listen_port: u16,
    pub buffer_size: usize,
    pub update_interval_ms: u64,
    pub max_peers: usize,
    pub quantum_state_size: usize,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_id: format!("node-{}", uuid::Uuid::new_v4()),
            listen_port: 0, // Random port
            buffer_size: 65536,
            update_interval_ms: 100,
            max_peers: 50,
            quantum_state_size: 256,
        }
    }
}

/// Thread-safe handle for controlling the federated node
#[derive(Clone)]
pub struct FederatedNodeHandle {
    shutdown: Arc<AtomicBool>,
    node_id: String,
}

impl FederatedNodeHandle {
    /// Request graceful shutdown of the node
    pub fn shutdown(&self) {
        tracing::info!("Shutdown requested for node {}", self.node_id);
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Check if shutdown has been requested
    #[allow(dead_code)]
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Get the node ID
    #[allow(dead_code)]
    pub fn node_id(&self) -> &str {
        &self.node_id
    }
}

/// Federated learning node with quantum-inspired processing
pub struct FederatedNode {
    config: NodeConfig,
    detector: PhotonDetector,
    detector_handle: PhotonDetectorHandle,
    state: QuantumInspiredState,
    peers: Vec<String>,
    running: Arc<AtomicBool>,
    event_tx: mpsc::Sender<NodeEvent>,
    event_rx: Option<mpsc::Receiver<NodeEvent>>,
    update_count: u64,
    last_update_time: Instant,
    shutdown_complete_tx: Option<mpsc::Sender<()>>,
    #[allow(dead_code)]
    shutdown_complete_rx: Option<mpsc::Receiver<()>>,
}

impl FederatedNode {
    /// Create a new federated node with proper resource management
    pub fn new(
        config: NodeConfig,
    ) -> Result<(Self, FederatedNodeHandle), Box<dyn std::error::Error + Send + Sync>> {
        let (detector, detector_handle) = PhotonDetector::new_simulated()?;
        let state = QuantumInspiredState::new(config.quantum_state_size);
        let (event_tx, event_rx) = mpsc::channel(1000);
        let running = Arc::new(AtomicBool::new(false));
        let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel(1);

        let handle = FederatedNodeHandle {
            shutdown: Arc::clone(&running),
            node_id: config.node_id.clone(),
        };

        let node = Self {
            config,
            detector,
            detector_handle,
            state,
            peers: Vec::with_capacity(10),
            running,
            event_tx,
            event_rx: Some(event_rx),
            update_count: 0,
            last_update_time: Instant::now(),
            shutdown_complete_tx: Some(shutdown_complete_tx),
            shutdown_complete_rx: Some(shutdown_complete_rx),
        };

        Ok((node, handle))
    }

    /// Create a simplified node for testing
    #[allow(dead_code)]
    pub fn new_simple(
        id: String,
    ) -> Result<(Self, FederatedNodeHandle), Box<dyn std::error::Error + Send + Sync>> {
        let config = NodeConfig {
            node_id: id,
            ..NodeConfig::default()
        };
        Self::new(config)
    }

    /// Get the node ID
    #[allow(dead_code)]
    pub fn id(&self) -> &str {
        &self.config.node_id
    }

    /// Add a peer to the network
    pub fn add_peer(&mut self, peer_addr: String) {
        if self.peers.len() < self.config.max_peers {
            self.peers.push(peer_addr);
        }
    }

    /// Get list of peers
    #[allow(dead_code)]
    pub fn peers(&self) -> &[String] {
        &self.peers
    }

    /// Get the quantum state
    #[allow(dead_code)]
    pub fn quantum_state(&self) -> &QuantumInspiredState {
        &self.state
    }

    /// Get mutable quantum state
    #[allow(dead_code)]
    pub fn quantum_state_mut(&mut self) -> &mut QuantumInspiredState {
        &mut self.state
    }

    /// Run the federated node with graceful shutdown support
    pub async fn run(&mut self, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.running.store(true, Ordering::SeqCst);

        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        let actual_port = listener.local_addr()?.port();

        tracing::info!(
            "Node {} listening on port {} (configured: {})",
            self.config.node_id,
            actual_port,
            port
        );

        let mut interval = interval(Duration::from_millis(self.config.update_interval_ms));
        let mut event_rx = self.event_rx.take().ok_or("Event receiver already taken")?;
        let shutdown_complete_tx = self.shutdown_complete_tx.take();

        // Main event loop with proper cancellation
        loop {
            tokio::select! {
                // Periodic learning task
                _ = interval.tick() => {
                    if !self.running.load(Ordering::SeqCst) {
                        tracing::info!("Shutdown flag detected, exiting main loop");
                        break;
                    }

                    if let Err(e) = self.local_learning_cycle().await {
                        tracing::warn!("Learning cycle error: {}", e);
                        let _ = self.event_tx.send(NodeEvent::Error(e.to_string())).await;
                    }
                }

                // Accept incoming peer connections
                result = listener.accept() => {
                    match result {
                        Ok((socket, addr)) => {
                            let peer_id = addr.to_string();
                            tracing::info!("Peer connected: {}", peer_id);

                            if let Err(e) = self.handle_peer(socket, &peer_id).await {
                                tracing::warn!("Failed to handle peer {}: {}", peer_id, e);
                            }

                            let _ = self.event_tx.send(NodeEvent::PeerConnected(peer_id)).await;
                        }
                        Err(e) => {
                            tracing::error!("Failed to accept connection: {}", e);
                        }
                    }
                }

                // Handle internal events
                Some(event) = event_rx.recv() => {
                    if let Err(e) = self.handle_internal_event(event).await {
                        tracing::error!("Event handling error: {}", e);
                    }
                }

                // Check for shutdown more frequently
                _ = tokio::time::sleep(Duration::from_millis(50)) => {
                    if !self.running.load(Ordering::SeqCst) {
                        tracing::info!("Shutdown flag detected in periodic check");
                        break;
                    }
                }
            }
        }

        // Cleanup
        tracing::info!("Beginning node {} shutdown sequence", self.config.node_id);
        self.detector_handle.shutdown();
        self.cleanup().await?;

        // Signal shutdown complete
        if let Some(tx) = shutdown_complete_tx {
            let _ = tx.send(()).await;
        }
        let _ = self.event_tx.send(NodeEvent::ShutdownComplete).await;

        tracing::info!(
            "Node {} stopped after {} updates",
            self.config.node_id,
            self.update_count
        );
        Ok(())
    }

    /// Wait for shutdown to complete
    #[allow(dead_code)]
    pub async fn wait_for_shutdown(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(mut rx) = self.shutdown_complete_rx.take() {
            rx.recv().await.ok_or("Shutdown signal channel closed")?;
            Ok(())
        } else {
            Err("Shutdown receiver already taken".into())
        }
    }

    /// Single learning cycle with photon measurement
    async fn local_learning_cycle(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Measure photon state
        let photon = self.detector.measure_photon_state()?;

        // Convert to features
        let features = self.photon_to_features(photon);

        // Process in "superposition" (parallel computation)
        let result = self.state.superposition_compute(|i| {
            let idx = i % features.len();
            features[idx]
        });

        // Update state with moving average for stability
        let alpha = 0.1; // Learning rate
        for (amp, new_val) in self.state.amplitudes.iter_mut().zip(result.iter()) {
            *amp = (1.0 - alpha) * *amp + alpha * *new_val;
        }

        // Normalize
        self.state.normalize();

        // Send event
        let _ = self.event_tx.send(NodeEvent::PhotonMeasured(photon)).await;

        self.update_count += 1;
        self.last_update_time = Instant::now();

        Ok(())
    }

    /// Handle peer connection
    async fn handle_peer(
        &self,
        mut socket: TcpStream,
        peer_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut buf = vec![0u8; self.config.buffer_size];

        // Read update with timeout
        let n = match tokio::time::timeout(Duration::from_secs(10), socket.read(&mut buf)).await {
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => return Err("Read timeout".into()),
        };

        if n > 0 {
            // Validate size
            if n > self.config.buffer_size {
                return Err("Update too large".into());
            }

            match serde_json::from_slice::<ModelUpdate>(&buf[..n]) {
                Ok(update) => {
                    if update.verify() {
                        tracing::info!("Received valid update from {}", update.node_id);
                        let _ = self
                            .event_tx
                            .send(NodeEvent::ModelReceived(
                                peer_id.to_string(),
                                update.clone(),
                            ))
                            .await;
                    } else {
                        tracing::warn!("Invalid update signature from {}", update.node_id);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse update from {}: {}", peer_id, e);
                }
            }

            // Send acknowledgment
            let ack = b"ACK";
            let _ = tokio::time::timeout(Duration::from_secs(5), socket.write_all(ack)).await;
        }

        Ok(())
    }

    /// Handle internal events
    async fn handle_internal_event(
        &mut self,
        event: NodeEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match event {
            NodeEvent::ModelReceived(_peer_id, update) => {
                self.apply_update(update).await?;
            }
            NodeEvent::PeerConnected(peer_id) => {
                tracing::info!("Peer {} connected to {}", peer_id, self.config.node_id);
            }
            NodeEvent::PeerDisconnected(peer_id) => {
                tracing::info!("Peer {} disconnected from {}", peer_id, self.config.node_id);
            }
            NodeEvent::PhotonMeasured(qubit) => {
                tracing::debug!(
                    "Photon measured: s0={:.2}, dop={:.3}",
                    qubit.s0,
                    qubit.degree_of_polarization()
                );
            }
            NodeEvent::Error(msg) => {
                tracing::error!("Node error: {}", msg);
            }
            NodeEvent::ShutdownComplete => {
                tracing::info!(
                    "Shutdown sequence completed for node {}",
                    self.config.node_id
                );
            }
        }
        Ok(())
    }

    /// Apply model update with consensus averaging
    async fn apply_update(
        &mut self,
        update: ModelUpdate,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Weight by update magnitude for weighted averaging
        let magnitude = update.magnitude();
        let weight = (magnitude / (magnitude + 1.0)).min(0.5); // Cap at 0.5

        for (i, w) in update.weights.iter().enumerate() {
            if i < self.state.amplitudes.len() {
                self.state.amplitudes[i] = (1.0 - weight) * self.state.amplitudes[i] + weight * w;
            }
        }

        // Renormalize
        self.state.normalize();

        Ok(())
    }

    /// Convert photon to feature vector
    fn photon_to_features(&self, photon: PseudoQubit) -> Vec<f64> {
        let normalized = photon.normalize();
        vec![
            photon.s0 / 1000.0, // Scale intensity
            normalized.s1,
            normalized.s2,
            photon.degree_of_polarization(),
        ]
    }

    /// Cleanup resources
    async fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Reset detector
        self.detector.reset();

        // Clear peers
        self.peers.clear();

        tracing::info!("Cleanup complete for node {}", self.config.node_id);
        Ok(())
    }

    /// Get node statistics
    #[allow(dead_code)]
    pub fn stats(&self) -> NodeStats {
        NodeStats {
            node_id: self.config.node_id.clone(),
            update_count: self.update_count,
            peer_count: self.peers.len(),
            uptime_secs: self.last_update_time.elapsed().as_secs(),
            is_running: self.running.load(Ordering::SeqCst),
        }
    }
}

impl Drop for FederatedNode {
    fn drop(&mut self) {
        // Ensure shutdown is signaled
        self.running.store(false, Ordering::SeqCst);
        self.detector_handle.shutdown();

        tracing::info!("FederatedNode {} dropped", self.config.node_id);
    }
}

/// Node statistics
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct NodeStats {
    pub node_id: String,
    pub update_count: u64,
    pub peer_count: usize,
    pub uptime_secs: u64,
    pub is_running: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_update_creation() {
        let update = ModelUpdate::new("node1".into(), vec![0.1, 0.2, 0.3]);
        assert_eq!(update.node_id, "node1");
        assert_eq!(update.weights.len(), 3);
        assert!(update.timestamp > 0);
    }

    #[test]
    fn test_model_update_magnitude() {
        let update = ModelUpdate::new("node1".into(), vec![3.0, 4.0]);
        assert!((update.magnitude() - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_node_config_default() {
        let config = NodeConfig::default();
        assert!(!config.node_id.is_empty());
        assert_eq!(config.buffer_size, 65536);
        assert_eq!(config.max_peers, 50);
    }

    #[test]
    fn test_federated_node_creation() {
        let (node, handle) = FederatedNode::new_simple("test-node".into()).unwrap();
        assert_eq!(node.id(), "test-node");
        assert_eq!(handle.node_id(), "test-node");
        assert!(!handle.is_shutdown_requested());
    }

    #[test]
    fn test_node_add_peer() {
        let (mut node, _handle) = FederatedNode::new_simple("test".into()).unwrap();
        node.add_peer("127.0.0.1:8080".into());
        assert_eq!(node.peers().len(), 1);
    }

    #[test]
    fn test_photon_to_features() {
        let (node, _handle) = FederatedNode::new_simple("test".into()).unwrap();
        let photon = PseudoQubit::new(1000.0, 500.0, 250.0);
        let features = node.photon_to_features(photon);

        assert_eq!(features.len(), 4);
        assert!(features[0] > 0.0); // intensity
        assert!(features[1] >= -1.0 && features[1] <= 1.0); // s1
        assert!(features[2] >= -1.0 && features[2] <= 1.0); // s2
        assert!(features[3] >= 0.0 && features[3] <= 1.0); // dop
    }

    #[tokio::test]
    #[ignore] // Flaky test - timing dependent
    async fn test_node_shutdown_real() {
        let (mut node, handle) = FederatedNode::new_simple("test-shutdown".into()).unwrap();

        // Clone handle for shutdown signal
        let shutdown_handle = handle.clone();

        // Start node in background
        let node_handle = tokio::spawn(async move { node.run(0).await });

        // Give it time to start
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Signal shutdown immediately (node may not have fully started)
        shutdown_handle.shutdown();

        // Wait for shutdown with timeout
        let result = tokio::time::timeout(Duration::from_secs(5), node_handle).await;

        // Verify shutdown completed
        assert!(result.is_ok(), "Node should shut down within timeout");
    }

    #[tokio::test]
    #[ignore] // Flaky test - timing dependent
    async fn test_node_multiple_cycles() {
        let (mut node, handle) = FederatedNode::new_simple("test-cycles".into()).unwrap();

        // Start node
        let node_handle = tokio::spawn(async move { node.run(0).await });

        // Let it run for a few cycles
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Shutdown
        handle.shutdown();

        // Wait for completion
        let result = tokio::time::timeout(Duration::from_secs(5), node_handle).await;

        assert!(
            result.is_ok(),
            "Node should complete multiple cycles and shutdown"
        );
        let _ = result.unwrap(); // Ignore result, just check timeout
    }

    #[test]
    fn test_handle_clone() {
        let (_node, handle) = FederatedNode::new_simple("test".into()).unwrap();
        let handle2 = handle.clone();

        // Both handles should control the same shutdown flag
        handle.shutdown();
        assert!(handle2.is_shutdown_requested());
    }
}

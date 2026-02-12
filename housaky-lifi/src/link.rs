//! Li-Fi link management

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::protocol::{ConnectionConfig, ConnectionState};
use crate::receiver::{LiFiReceiver, ReceiverStats};
use crate::transmitter::{LiFiTransmitter, TransmitterStats};

/// A bidirectional Li-Fi link
pub struct LiFiLink {
    config: ConnectionConfig,
    transmitter: Arc<RwLock<LiFiTransmitter>>,
    receiver: Arc<RwLock<LiFiReceiver>>,
    data_rx: mpsc::Receiver<Vec<u8>>,
}

impl LiFiLink {
    /// Create a new Li-Fi link
    pub fn new(config: ConnectionConfig) -> Result<(Self, mpsc::Sender<Vec<u8>>)> {
        let (tx, _tx_packets) = LiFiTransmitter::new(config.clone());
        let (rx, data_rx) = LiFiReceiver::new(config.clone());

        let (data_tx, data_tx_receiver) = mpsc::channel(100);

        let link = Self {
            config,
            transmitter: Arc::new(RwLock::new(tx)),
            receiver: Arc::new(RwLock::new(rx)),
            data_rx,
        };

        Ok((link, data_tx))
    }

    /// Establish connection
    pub async fn connect(&self) -> Result<()> {
        let mut tx = self.transmitter.write().await;
        tx.connect().await?;
        drop(tx);

        let mut rx = self.receiver.write().await;
        rx.listen().await?;

        Ok(())
    }

    /// Send data
    pub async fn send(&self, data: Vec<u8>) -> Result<()> {
        let mut tx = self.transmitter.write().await;
        tx.send(&data).await
    }

    /// Receive data (non-blocking)
    pub async fn try_recv(&mut self) -> Option<Vec<u8>> {
        self.data_rx.try_recv().ok()
    }

    /// Get current connection state
    pub async fn state(&self) -> ConnectionState {
        let tx = self.transmitter.read().await;
        tx.state()
    }

    /// Close the link
    pub async fn close(&self) {
        let mut tx = self.transmitter.write().await;
        tx.disconnect();

        let mut rx = self.receiver.write().await;
        rx.stop();
    }
}

/// Li-Fi link manager for handling multiple links
pub struct LinkManager {
    links: Arc<RwLock<Vec<LiFiLink>>>,
}

impl LinkManager {
    /// Create a new link manager
    pub fn new() -> Self {
        Self {
            links: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a new link
    pub async fn add_link(&self, link: LiFiLink) {
        let mut links = self.links.write().await;
        links.push(link);
    }

    /// Broadcast data to all connected links
    pub async fn broadcast(&self, data: Vec<u8>) -> Result<()> {
        let links = self.links.read().await;

        for link in links.iter() {
            if let Err(e) = link.send(data.clone()).await {
                tracing::warn!("Failed to send to link: {}", e);
            }
        }

        Ok(())
    }

    /// Close all links
    pub async fn close_all(&self) {
        let links = self.links.read().await;

        for link in links.iter() {
            link.close().await;
        }
    }
}

impl Default for LinkManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Link quality metrics
#[derive(Debug, Clone)]
pub struct LinkQuality {
    /// Signal strength (0-100)
    pub signal_strength: u8,
    /// Bit error rate
    pub bit_error_rate: f64,
    /// Latency in milliseconds
    pub latency_ms: f64,
    /// Throughput in kbps
    pub throughput_kbps: f64,
}

impl LinkQuality {
    /// Check if link quality is acceptable
    pub fn is_acceptable(&self) -> bool {
        self.signal_strength > 30 && self.bit_error_rate < 0.01
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_link_manager() {
        let manager = LinkManager::new();

        let config = ConnectionConfig::default();
        let (link, _data_tx) = LiFiLink::new(config).unwrap();

        manager.add_link(link).await;

        let links = manager.links.read().await;
        assert_eq!(links.len(), 1);
    }

    #[test]
    fn test_link_quality() {
        let quality = LinkQuality {
            signal_strength: 80,
            bit_error_rate: 0.001,
            latency_ms: 10.0,
            throughput_kbps: 1000.0,
        };

        assert!(quality.is_acceptable());
    }
}

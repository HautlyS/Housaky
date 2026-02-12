//! Li-Fi transmitter

use anyhow::Result;
use tokio::sync::mpsc;

use crate::protocol::{ConnectionConfig, ConnectionState, Packet, PacketType};
use housaky_photonics::encoding::{EncodingScheme, OpticalEncoder, OpticalFrame};

/// Li-Fi transmitter
pub struct LiFiTransmitter {
    state: ConnectionState,
    config: ConnectionConfig,
    sequence: u32,
    encoder: OpticalEncoder,
    packet_tx: Option<mpsc::Sender<Packet>>,
    packet_rx: mpsc::Receiver<Packet>,
}

impl LiFiTransmitter {
    /// Create a new transmitter
    pub fn new(config: ConnectionConfig) -> (Self, mpsc::Receiver<Packet>) {
        let (packet_tx, packet_rx) = mpsc::channel(100);
        let encoder = OpticalEncoder::new(EncodingScheme::Manchester, 100_000); // 100 kbps

        let transmitter = Self {
            state: ConnectionState::Disconnected,
            config,
            sequence: 0,
            encoder,
            packet_tx: Some(packet_tx),
            packet_rx,
        };

        (transmitter, packet_rx)
    }

    /// Connect to a receiver
    pub async fn connect(&mut self) -> Result<()> {
        if self.state != ConnectionState::Disconnected {
            return Err(anyhow::anyhow!("Already connected or connecting"));
        }

        self.state = ConnectionState::Connecting;
        tracing::info!("Li-Fi connecting...");

        // Send connect packet
        let connect_packet = Packet::connect(rand::random());
        self.send_packet(connect_packet).await?;

        // Wait for ACK response with timeout
        let ack_received = tokio::time::timeout(
            tokio::time::Duration::from_millis(5000),
            self.wait_for_ack(),
        )
        .await;

        match ack_received {
            Ok(Ok(())) => {
                self.state = ConnectionState::Connected;
                tracing::info!("Li-Fi connected - ACK received");
            }
            Ok(Err(e)) => {
                self.state = ConnectionState::Disconnected;
                return Err(anyhow::anyhow!("Connection failed: {}", e));
            }
            Err(_) => {
                self.state = ConnectionState::Disconnected;
                return Err(anyhow::anyhow!("Connection timeout - no ACK received"));
            }
        }

        Ok(())
    }

    /// Send data
    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        if self.state != ConnectionState::Connected {
            return Err(anyhow::anyhow!("Not connected"));
        }

        // Split data into chunks if necessary
        const MAX_PAYLOAD: usize = 1024;

        for chunk in data.chunks(MAX_PAYLOAD) {
            let packet = Packet::data(self.sequence, chunk.to_vec());
            self.send_packet(packet).await?;
            self.sequence = self.sequence.wrapping_add(1);
        }

        Ok(())
    }

    /// Send a packet
    async fn send_packet(&self, packet: Packet) -> Result<()> {
        if let Some(ref tx) = self.packet_tx {
            tx.send(packet)
                .await
                .map_err(|_| anyhow::anyhow!("Failed to send packet"))?;
        }
        Ok(())
    }

    /// Send heartbeat
    pub async fn send_heartbeat(&self) -> Result<()> {
        if self.state != ConnectionState::Connected {
            return Err(anyhow::anyhow!("Not connected"));
        }

        let heartbeat = Packet::heartbeat();
        self.send_packet(heartbeat).await
    }

    /// Disconnect
    pub fn disconnect(&mut self) {
        self.state = ConnectionState::Disconnected;
        tracing::info!("Li-Fi disconnected");
    }

    /// Get current state
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Encode packet to optical symbols
    pub fn encode_packet(&self, packet: &Packet) -> Vec<u8> {
        let bytes = packet.to_bytes();
        let symbols = self.encoder.encode(&bytes);

        // Convert symbols to raw bytes for LED control
        symbols
            .iter()
            .flat_map(|s| {
                let duration_bytes = s.duration_us.to_be_bytes();
                vec![
                    if s.on { 1 } else { 0 },
                    duration_bytes[0],
                    duration_bytes[1],
                    duration_bytes[2],
                    duration_bytes[3],
                ]
            })
            .collect()
    }

    /// Wait for ACK packet from receiver
    async fn wait_for_ack(&mut self) -> Result<()> {
        loop {
            match tokio::time::timeout(
                tokio::time::Duration::from_millis(100),
                self.packet_rx.recv(),
            )
            .await
            {
                Ok(Some(packet)) => {
                    if packet.header.packet_type == PacketType::Ack {
                        tracing::debug!("Received ACK packet");
                        return Ok(());
                    }
                }
                Ok(None) => {
                    return Err(anyhow::anyhow!("Channel closed"));
                }
                Err(_) => {
                    // Timeout - continue waiting
                    continue;
                }
            }
        }
    }
}

/// Transmitter statistics
#[derive(Debug, Clone)]
pub struct TransmitterStats {
    /// Total packets sent
    pub packets_sent: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Retransmissions
    pub retransmissions: u64,
    /// Errors
    pub errors: u64,
}

impl Default for TransmitterStats {
    fn default() -> Self {
        Self {
            packets_sent: 0,
            bytes_sent: 0,
            retransmissions: 0,
            errors: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transmitter_creation() {
        let config = ConnectionConfig::default();
        let (tx, _rx) = LiFiTransmitter::new(config);

        assert_eq!(tx.state(), ConnectionState::Disconnected);
    }

    #[test]
    fn test_encode_packet() {
        let config = ConnectionConfig::default();
        let (tx, _rx) = LiFiTransmitter::new(config);

        let packet = Packet::data(1, vec![0xAA, 0xBB]);
        let encoded = tx.encode_packet(&packet);

        assert!(!encoded.is_empty());
    }
}

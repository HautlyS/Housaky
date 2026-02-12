//! Li-Fi protocol definitions

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Li-Fi protocol version
pub const PROTOCOL_VERSION: u8 = 1;

/// Li-Fi packet types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PacketType {
    /// Data packet
    Data = 0x01,
    /// Acknowledgment
    Ack = 0x02,
    /// Connection request
    Connect = 0x03,
    /// Connection response
    ConnectResponse = 0x04,
    /// Heartbeat/keepalive
    Heartbeat = 0x05,
    /// Error
    Error = 0xFF,
}

impl PacketType {
    /// Convert from byte
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x01 => Some(Self::Data),
            0x02 => Some(Self::Ack),
            0x03 => Some(Self::Connect),
            0x04 => Some(Self::ConnectResponse),
            0x05 => Some(Self::Heartbeat),
            0xFF => Some(Self::Error),
            _ => None,
        }
    }
}

/// Li-Fi packet header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketHeader {
    /// Protocol version
    pub version: u8,
    /// Packet type
    pub packet_type: PacketType,
    /// Sequence number
    pub sequence: u32,
    /// Payload length
    pub payload_len: u16,
    /// Checksum
    pub checksum: u16,
}

impl PacketHeader {
    /// Header size in bytes
    pub const SIZE: usize = 10;

    /// Create a new header
    pub fn new(packet_type: PacketType, sequence: u32, payload_len: u16) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            packet_type,
            sequence,
            payload_len,
            checksum: 0,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0] = self.version;
        bytes[1] = self.packet_type as u8;
        bytes[2..6].copy_from_slice(&self.sequence.to_be_bytes());
        bytes[6..8].copy_from_slice(&self.payload_len.to_be_bytes());
        bytes[8..10].copy_from_slice(&self.checksum.to_be_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(anyhow::anyhow!("Header too short"));
        }

        let version = bytes[0];
        let packet_type = PacketType::from_byte(bytes[1])
            .ok_or_else(|| anyhow::anyhow!("Invalid packet type"))?;
        let sequence = u32::from_be_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        let payload_len = u16::from_be_bytes([bytes[6], bytes[7]]);
        let checksum = u16::from_be_bytes([bytes[8], bytes[9]]);

        Ok(Self {
            version,
            packet_type,
            sequence,
            payload_len,
            checksum,
        })
    }

    /// Calculate checksum
    pub fn calculate_checksum(&mut self, payload: &[u8]) {
        let mut sum: u16 = 0;
        // Simple checksum algorithm
        for &byte in payload {
            sum = sum.wrapping_add(byte as u16);
        }
        self.checksum = sum;
    }
}

/// Complete Li-Fi packet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet {
    /// Packet header
    pub header: PacketHeader,
    /// Payload data
    pub payload: Vec<u8>,
}

impl Packet {
    /// Create a new data packet
    pub fn data(sequence: u32, payload: Vec<u8>) -> Self {
        let mut header = PacketHeader::new(PacketType::Data, sequence, payload.len() as u16);
        header.calculate_checksum(&payload);

        Self { header, payload }
    }

    /// Create an ACK packet
    pub fn ack(sequence: u32) -> Self {
        let header = PacketHeader::new(PacketType::Ack, sequence, 0);

        Self {
            header,
            payload: Vec::new(),
        }
    }

    /// Create a connect packet
    pub fn connect(session_id: u32) -> Self {
        let header = PacketHeader::new(PacketType::Connect, session_id, 0);

        Self {
            header,
            payload: Vec::new(),
        }
    }

    /// Create a heartbeat packet
    pub fn heartbeat() -> Self {
        let header = PacketHeader::new(PacketType::Heartbeat, 0, 0);

        Self {
            header,
            payload: Vec::new(),
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PacketHeader::SIZE + self.payload.len());
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < PacketHeader::SIZE {
            return Err(anyhow::anyhow!("Packet too short"));
        }

        let header = PacketHeader::from_bytes(&bytes[..PacketHeader::SIZE])?;
        let payload = bytes[PacketHeader::SIZE..].to_vec();

        if payload.len() != header.payload_len as usize {
            return Err(anyhow::anyhow!("Payload length mismatch"));
        }

        Ok(Self { header, payload })
    }

    /// Verify checksum
    pub fn verify_checksum(&self) -> bool {
        let mut sum: u16 = 0;
        for &byte in &self.payload {
            sum = sum.wrapping_add(byte as u16);
        }
        sum == self.header.checksum
    }
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected
    Connected,
    /// Error state
    Error,
}

/// Connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Retry attempts
    pub retry_attempts: u32,
    /// Window size for flow control
    pub window_size: u16,
    /// Use forward error correction
    pub use_fec: bool,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 5000,
            retry_attempts: 3,
            window_size: 16,
            use_fec: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_header() {
        let header = PacketHeader::new(PacketType::Data, 42, 100);
        let bytes = header.to_bytes();
        let decoded = PacketHeader::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.version, PROTOCOL_VERSION);
        assert_eq!(decoded.packet_type, PacketType::Data);
        assert_eq!(decoded.sequence, 42);
        assert_eq!(decoded.payload_len, 100);
    }

    #[test]
    fn test_data_packet() {
        let payload = vec![1, 2, 3, 4, 5];
        let packet = Packet::data(1, payload.clone());

        assert_eq!(packet.header.packet_type, PacketType::Data);
        assert_eq!(packet.payload, payload);
        assert!(packet.verify_checksum());
    }

    #[test]
    fn test_packet_serialization() {
        let packet = Packet::data(100, vec![0xAA, 0xBB, 0xCC]);
        let bytes = packet.to_bytes();
        let decoded = Packet::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.header.sequence, 100);
        assert_eq!(decoded.payload, vec![0xAA, 0xBB, 0xCC]);
    }
}

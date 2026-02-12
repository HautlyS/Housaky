//! Encoding/decoding for optical communication

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Encoding scheme for optical transmission
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EncodingScheme {
    /// On-off keying (simple binary)
    OnOffKeying,
    /// Pulse position modulation
    PulsePositionModulation,
    /// Manchester encoding
    Manchester,
    /// Differential encoding
    Differential,
}

/// Encoder for optical data
pub struct OpticalEncoder {
    scheme: EncodingScheme,
    bitrate: u32, // bits per second
}

impl OpticalEncoder {
    /// Create a new encoder
    pub fn new(scheme: EncodingScheme, bitrate: u32) -> Self {
        Self { scheme, bitrate }
    }

    /// Encode data to optical signal
    pub fn encode(&self, data: &[u8]) -> Vec<OpticalSymbol> {
        match self.scheme {
            EncodingScheme::OnOffKeying => self.encode_ook(data),
            EncodingScheme::Manchester => self.encode_manchester(data),
            _ => self.encode_ook(data), // Default to OOK
        }
    }

    /// On-off keying encoding
    fn encode_ook(&self, data: &[u8]) -> Vec<OpticalSymbol> {
        let mut symbols = Vec::with_capacity(data.len() * 8);

        for byte in data {
            for i in 0..8 {
                let bit = (byte >> (7 - i)) & 1;
                symbols.push(OpticalSymbol {
                    on: bit == 1,
                    duration_us: self.bit_duration_us(),
                });
            }
        }

        symbols
    }

    /// Manchester encoding
    fn encode_manchester(&self, data: &[u8]) -> Vec<OpticalSymbol> {
        let mut symbols = Vec::with_capacity(data.len() * 16);
        let half_duration = self.bit_duration_us() / 2;

        for byte in data {
            for i in 0..8 {
                let bit = (byte >> (7 - i)) & 1;
                // Manchester: 0 = low->high, 1 = high->low
                if bit == 0 {
                    symbols.push(OpticalSymbol {
                        on: false,
                        duration_us: half_duration,
                    });
                    symbols.push(OpticalSymbol {
                        on: true,
                        duration_us: half_duration,
                    });
                } else {
                    symbols.push(OpticalSymbol {
                        on: true,
                        duration_us: half_duration,
                    });
                    symbols.push(OpticalSymbol {
                        on: false,
                        duration_us: half_duration,
                    });
                }
            }
        }

        symbols
    }

    /// Calculate bit duration in microseconds
    fn bit_duration_us(&self) -> u32 {
        1_000_000 / self.bitrate
    }
}

/// A single optical symbol
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OpticalSymbol {
    /// Whether light is on or off
    pub on: bool,
    /// Duration in microseconds
    pub duration_us: u32,
}

/// Decoder for optical signals
pub struct OpticalDecoder {
    scheme: EncodingScheme,
    bitrate: u32,
    symbol_buffer: Vec<OpticalSymbol>,
}

impl OpticalDecoder {
    /// Create a new decoder
    pub fn new(scheme: EncodingScheme, bitrate: u32) -> Self {
        Self {
            scheme,
            bitrate,
            symbol_buffer: Vec::new(),
        }
    }

    /// Add a detected symbol
    pub fn add_symbol(&mut self, symbol: OpticalSymbol) {
        self.symbol_buffer.push(symbol);
    }

    /// Decode buffered symbols
    pub fn decode(&mut self) -> Result<Vec<u8>> {
        match self.scheme {
            EncodingScheme::OnOffKeying => self.decode_ook(),
            EncodingScheme::Manchester => self.decode_manchester(),
            _ => self.decode_ook(),
        }
    }

    /// Decode on-off keying
    fn decode_ook(&mut self) -> Result<Vec<u8>> {
        if self.symbol_buffer.len() < 8 {
            return Ok(Vec::new());
        }

        let mut data = Vec::new();
        let mut current_byte = 0u8;
        let mut bit_count = 0;

        for symbol in &self.symbol_buffer {
            current_byte = (current_byte << 1) | if symbol.on { 1 } else { 0 };
            bit_count += 1;

            if bit_count == 8 {
                data.push(current_byte);
                current_byte = 0;
                bit_count = 0;
            }
        }

        self.symbol_buffer.clear();
        Ok(data)
    }

    /// Decode Manchester encoding
    fn decode_manchester(&mut self) -> Result<Vec<u8>> {
        if self.symbol_buffer.len() < 16 {
            return Ok(Vec::new());
        }

        let mut data = Vec::new();
        let mut current_byte = 0u8;
        let mut bit_count = 0;

        // Process symbols in pairs
        for pair in self.symbol_buffer.chunks_exact(2) {
            let (first, second) = (pair[0], pair[1]);
            // Manchester: low->high = 0, high->low = 1
            let bit = if !first.on && second.on { 0 } else { 1 };

            current_byte = (current_byte << 1) | bit;
            bit_count += 1;

            if bit_count == 8 {
                data.push(current_byte);
                current_byte = 0;
                bit_count = 0;
            }
        }

        self.symbol_buffer.clear();
        Ok(data)
    }

    /// Clear the symbol buffer
    pub fn clear(&mut self) {
        self.symbol_buffer.clear();
    }
}

/// Frame structure for optical transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpticalFrame {
    /// Start delimiter
    pub preamble: [u8; 4],
    /// Frame type
    pub frame_type: u8,
    /// Sequence number
    pub sequence: u16,
    /// Payload data
    pub payload: Vec<u8>,
    /// CRC checksum
    pub checksum: u32,
}

impl OpticalFrame {
    /// Create a new frame
    pub fn new(frame_type: u8, sequence: u16, payload: Vec<u8>) -> Self {
        let checksum = crc32fast::hash(&payload);

        Self {
            preamble: [0xAA, 0xAA, 0xAA, 0xAA],
            frame_type,
            sequence,
            payload,
            checksum,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.preamble);
        bytes.push(self.frame_type);
        bytes.extend_from_slice(&self.sequence.to_be_bytes());
        bytes.extend_from_slice(&(self.payload.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 15 {
            return Err(anyhow::anyhow!("Frame too short"));
        }

        let preamble = [data[0], data[1], data[2], data[3]];
        let frame_type = data[4];
        let sequence = u16::from_be_bytes([data[5], data[6]]);
        let payload_len = u32::from_be_bytes([data[7], data[8], data[9], data[10]]) as usize;

        if data.len() < 15 + payload_len {
            return Err(anyhow::anyhow!("Incomplete frame"));
        }

        let payload = data[11..11 + payload_len].to_vec();
        let checksum = u32::from_be_bytes([
            data[11 + payload_len],
            data[12 + payload_len],
            data[13 + payload_len],
            data[14 + payload_len],
        ]);

        // Verify checksum
        let computed = crc32fast::hash(&payload);
        if computed != checksum {
            return Err(anyhow::anyhow!("Checksum mismatch"));
        }

        Ok(Self {
            preamble,
            frame_type,
            sequence,
            payload,
            checksum,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ook_encoding() {
        let encoder = OpticalEncoder::new(EncodingScheme::OnOffKeying, 1000);
        let data = vec![0b10101010];

        let symbols = encoder.encode(&data);
        assert_eq!(symbols.len(), 8);
        assert!(symbols[0].on); // First bit is 1
        assert!(!symbols[1].on); // Second bit is 0
    }

    #[test]
    fn test_frame_serialization() {
        let frame = OpticalFrame::new(1, 42, vec![1, 2, 3, 4, 5]);
        let bytes = frame.to_bytes();
        let decoded = OpticalFrame::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.frame_type, 1);
        assert_eq!(decoded.sequence, 42);
        assert_eq!(decoded.payload, vec![1, 2, 3, 4, 5]);
    }
}

//! Core types and utilities for Housaky

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// A unique identifier for entities in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId([u8; 32]);

impl EntityId {
    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Generate a new random ID
    pub fn generate() -> Self {
        use rand::Rng;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill(&mut bytes);
        Self(bytes)
    }

    /// Get bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// A timestamp with millisecond precision
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Get current timestamp
    pub fn now() -> Self {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Self(duration.as_millis() as u64)
    }

    /// Create from milliseconds since epoch
    pub fn from_millis(millis: u64) -> Self {
        Self(millis)
    }

    /// Get milliseconds
    pub fn as_millis(&self) -> u64 {
        self.0
    }

    /// Get duration since another timestamp
    pub fn duration_since(&self, other: Timestamp) -> u64 {
        self.0.saturating_sub(other.0)
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::now()
    }
}

/// A versioned data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Versioned<T> {
    /// The data
    pub data: T,
    /// Version number
    pub version: u64,
    /// Last modified timestamp
    pub modified_at: Timestamp,
}

impl<T> Versioned<T> {
    /// Create new versioned data
    pub fn new(data: T) -> Self {
        Self {
            data,
            version: 1,
            modified_at: Timestamp::now(),
        }
    }

    /// Update the data, incrementing version
    pub fn update(&mut self, data: T) {
        self.data = data;
        self.version += 1;
        self.modified_at = Timestamp::now();
    }

    /// Get current version
    pub fn version(&self) -> u64 {
        self.version
    }
}

/// A result wrapper with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultContext<T> {
    pub result: T,
    pub timestamp: Timestamp,
    pub duration_ms: u64,
}

impl<T> ResultContext<T> {
    /// Create from result with timing
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce() -> T,
    {
        let start = std::time::Instant::now();
        let result = f();
        let duration = start.elapsed();

        Self {
            result,
            timestamp: Timestamp::now(),
            duration_ms: duration.as_millis() as u64,
        }
    }
}

/// Network address types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkAddress {
    /// IPv4 address with port
    V4(String, u16),
    /// IPv6 address with port
    V6(String, u16),
    /// Domain name with port
    Domain(String, u16),
    /// Multiaddr (libp2p)
    Multiaddr(String),
}

impl NetworkAddress {
    /// Parse from string
    pub fn parse(s: &str) -> Result<Self> {
        if s.starts_with('/') {
            // Multiaddr
            Ok(Self::Multiaddr(s.to_string()))
        } else if let Ok(addr) = s.parse::<std::net::SocketAddrV4>() {
            Ok(Self::V4(addr.ip().to_string(), addr.port()))
        } else if let Ok(addr) = s.parse::<std::net::SocketAddrV6>() {
            Ok(Self::V6(addr.ip().to_string(), addr.port()))
        } else {
            // Try to parse as domain:port
            let parts: Vec<&str> = s.split(':').collect();
            if parts.len() == 2 {
                if let Ok(port) = parts[1].parse::<u16>() {
                    return Ok(Self::Domain(parts[0].to_string(), port));
                }
            }

            Err(anyhow::anyhow!("Invalid network address format"))
        }
    }

    /// Get as string
    pub fn to_string(&self) -> String {
        match self {
            Self::V4(ip, port) => format!("{}:{}", ip, port),
            Self::V6(ip, port) => format!("[{}]:{}", ip, port),
            Self::Domain(domain, port) => format!("{}:{}", domain, port),
            Self::Multiaddr(addr) => addr.clone(),
        }
    }
}

impl std::fmt::Display for NetworkAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Capability flags for nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities(u64);

impl Capabilities {
    /// No capabilities
    pub const NONE: Self = Self(0);
    /// Can store data
    pub const STORAGE: Self = Self(1 << 0);
    /// Can compute (ML inference)
    pub const COMPUTE: Self = Self(1 << 1);
    /// Can relay network traffic
    pub const RELAY: Self = Self(1 << 2);
    /// Can validate consensus
    pub const VALIDATOR: Self = Self(1 << 3);
    /// Can process photons/Li-Fi
    pub const PHOTONIC: Self = Self(1 << 4);

    /// Create empty capabilities
    pub fn empty() -> Self {
        Self(0)
    }

    /// Add a capability
    pub fn add(&mut self, capability: Self) {
        self.0 |= capability.0;
    }

    /// Remove a capability
    pub fn remove(&mut self, capability: Self) {
        self.0 &= !capability.0;
    }

    /// Check if has capability
    pub fn has(&self, capability: Self) -> bool {
        (self.0 & capability.0) != 0
    }
}

impl std::ops::BitOr for Capabilities {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl Default for Capabilities {
    fn default() -> Self {
        Self::empty()
    }
}

/// Resource limits for a node or operation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory in MB
    pub max_memory_mb: u64,
    /// Maximum CPU cores
    pub max_cpu_cores: u32,
    /// Maximum storage in GB
    pub max_storage_gb: u64,
    /// Maximum bandwidth in Mbps
    pub max_bandwidth_mbps: u64,
}

impl ResourceLimits {
    /// Unlimited resources
    pub fn unlimited() -> Self {
        Self {
            max_memory_mb: u64::MAX,
            max_cpu_cores: u32::MAX,
            max_storage_gb: u64::MAX,
            max_bandwidth_mbps: u64::MAX,
        }
    }

    /// Conservative limits for resource-constrained devices
    pub fn conservative() -> Self {
        Self {
            max_memory_mb: 512,
            max_cpu_cores: 2,
            max_storage_gb: 10,
            max_bandwidth_mbps: 100,
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 4096,
            max_cpu_cores: 4,
            max_storage_gb: 100,
            max_bandwidth_mbps: 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_id() {
        let id = EntityId::generate();
        let hex = id.to_hex();
        assert_eq!(hex.len(), 64); // 32 bytes * 2 hex chars
    }

    #[test]
    fn test_timestamp() {
        let t1 = Timestamp::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let t2 = Timestamp::now();

        assert!(t2 > t1);
        assert!(t2.duration_since(t1) >= 10);
    }

    #[test]
    fn test_versioned() {
        let mut v = Versioned::new(42);
        assert_eq!(v.version(), 1);

        v.update(100);
        assert_eq!(v.version(), 2);
        assert_eq!(v.data, 100);
    }

    #[test]
    fn test_capabilities() {
        let mut caps = Capabilities::empty();
        assert!(!caps.has(Capabilities::STORAGE));

        caps.add(Capabilities::STORAGE);
        assert!(caps.has(Capabilities::STORAGE));

        caps.add(Capabilities::COMPUTE);
        assert!(caps.has(Capabilities::COMPUTE));

        caps.remove(Capabilities::STORAGE);
        assert!(!caps.has(Capabilities::STORAGE));
    }
}

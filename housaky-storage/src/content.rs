//! Content-addressed storage with Iroh
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Multihash implementation for IPFS CIDs
pub mod multihash {
    /// Multihash hash function codes
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(u64)]
    pub enum HashCode {
        /// SHA2-256
        Sha2256 = 0x12,
        /// SHA2-512
        Sha2512 = 0x13,
        /// SHA3-256
        Sha3256 = 0x16,
        /// SHA3-512
        Sha3512 = 0x17,
        /// Blake2b-256
        Blake2b256 = 0xb220,
        /// Blake2b-512
        Blake2b512 = 0xb240,
        /// Blake3
        Blake3 = 0x1e,
    }

    /// Encode data as multihash
    /// Format: <hash-function-code><digest-length><digest>
    pub fn encode(code: HashCode, digest: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(8 + digest.len());

        // Encode hash function code as varint
        encode_varint(code as u64, &mut result);

        // Encode digest length
        encode_varint(digest.len() as u64, &mut result);

        // Append digest
        result.extend_from_slice(digest);

        result
    }

    /// Decode multihash
    pub fn decode(data: &[u8]) -> anyhow::Result<(HashCode, Vec<u8>)> {
        let (code, offset) = decode_varint(data)?;
        let (length, offset2) = decode_varint(&data[offset..])?;
        let offset = offset + offset2;

        let code = match code {
            0x12 => HashCode::Sha2256,
            0x13 => HashCode::Sha2512,
            0x16 => HashCode::Sha3256,
            0x17 => HashCode::Sha3512,
            0xb220 => HashCode::Blake2b256,
            0xb240 => HashCode::Blake2b512,
            0x1e => HashCode::Blake3,
            _ => return Err(anyhow::anyhow!("Unknown hash code: {}", code)),
        };

        let digest = data[offset..offset + length as usize].to_vec();
        Ok((code, digest))
    }

    /// Encode u64 as varint
    pub fn encode_varint(mut value: u64, buf: &mut Vec<u8>) {
        while value >= 0x80 {
            buf.push((value as u8) | 0x80);
            value >>= 7;
        }
        buf.push(value as u8);
    }

    /// Decode varint from buffer
    pub fn decode_varint(data: &[u8]) -> anyhow::Result<(u64, usize)> {
        let mut result: u64 = 0;
        let mut shift = 0;
        let mut i = 0;

        loop {
            if i >= data.len() {
                return Err(anyhow::anyhow!("Incomplete varint"));
            }

            let byte = data[i];
            result |= ((byte & 0x7f) as u64) << shift;

            if byte & 0x80 == 0 {
                return Ok((result, i + 1));
            }

            shift += 7;
            i += 1;

            if shift > 63 {
                return Err(anyhow::anyhow!("Varint too large"));
            }
        }
    }
}

/// IPFS CID implementation
pub mod cid {
    use super::multihash;

    /// CID version
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Version {
        V0,
        V1,
    }

    /// Multicodec content type codes
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(u64)]
    pub enum Codec {
        /// Raw binary
        Raw = 0x55,
        /// DAG PB (Protocol Buffers)
        DagPb = 0x70,
        /// DAG CBOR
        DagCbor = 0x71,
        /// Git raw
        GitRaw = 0x78,
        /// FlatFS
        FlatFs = 0x72,
    }

    /// CID structure
    #[derive(Debug, Clone)]
    pub struct Cid {
        pub version: Version,
        pub codec: Codec,
        pub hash: multihash::HashCode,
        pub digest: Vec<u8>,
    }

    impl Cid {
        /// Create CID v1 from raw data
        pub fn new_v1(codec: Codec, hash: multihash::HashCode, digest: Vec<u8>) -> Self {
            Self {
                version: Version::V1,
                codec,
                hash,
                digest,
            }
        }

        /// Create CID v0 from SHA2-256 hash
        pub fn new_v0(digest: [u8; 32]) -> Self {
            Self {
                version: Version::V0,
                codec: Codec::DagPb,
                hash: multihash::HashCode::Sha2256,
                digest: digest.to_vec(),
            }
        }

        /// Encode CID to bytes
        pub fn to_bytes(&self) -> Vec<u8> {
            match self.version {
                Version::V0 => {
                    // CID v0: just the multihash, must be sha2-256
                    multihash::encode(self.hash, &self.digest)
                }
                Version::V1 => {
                    // CID v1: <version><codec><multihash>
                    let mut result = Vec::new();

                    // Version (varint)
                    super::multihash::encode_varint(1, &mut result);

                    // Codec (varint)
                    super::multihash::encode_varint(self.codec as u64, &mut result);

                    // Multihash
                    result.extend_from_slice(&multihash::encode(self.hash, &self.digest));

                    result
                }
            }
        }

        /// Encode CID as base32 string (for v1) or base58 (for v0)
        pub fn to_string(&self) -> String {
            match self.version {
                Version::V0 => {
                    // CID v0 uses base58btc encoding
                    let bytes = self.to_bytes();
                    bs58::encode(bytes).into_string()
                }
                Version::V1 => {
                    // CID v1 uses base32 encoding with 'b' prefix
                    let bytes = self.to_bytes();
                    format!(
                        "b{}",
                        multibase::encode(multibase::Base::Base32Lower, &bytes)
                    )
                }
            }
        }

        /// Parse CID from string
        pub fn from_str(s: &str) -> anyhow::Result<Self> {
            if s.len() == 46 && s.starts_with("Qm") {
                // CID v0 (base58btc)
                let bytes = bs58::decode(s).into_vec()?;
                let (hash, digest) = multihash::decode(&bytes)?;

                Ok(Self {
                    version: Version::V0,
                    codec: Codec::DagPb,
                    hash,
                    digest,
                })
            } else if s.starts_with("b") {
                // CID v1 (base32)
                let (_, bytes) = multibase::decode(s)?;
                let (version, offset) = super::multihash::decode_varint(&bytes)?;

                if version != 1 {
                    return Err(anyhow::anyhow!("Invalid CID version: {}", version));
                }

                let (codec, offset2) = super::multihash::decode_varint(&bytes[offset..])?;
                let offset = offset + offset2;

                let codec = match codec {
                    0x55 => Codec::Raw,
                    0x70 => Codec::DagPb,
                    0x71 => Codec::DagCbor,
                    _ => return Err(anyhow::anyhow!("Unknown codec: {}", codec)),
                };

                let (hash, digest) = multihash::decode(&bytes[offset..])?;

                Ok(Self {
                    version: Version::V1,
                    codec,
                    hash,
                    digest,
                })
            } else {
                Err(anyhow::anyhow!("Invalid CID string format"))
            }
        }
    }
}

/// Content address
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ContentAddress {
    pub hash: [u8; 32],
}

impl ContentAddress {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { hash: bytes }
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.hash)
    }

    /// Convert to IPFS CID v1
    pub fn to_cid(&self) -> cid::Cid {
        cid::Cid::new_v1(
            cid::Codec::Raw,
            multihash::HashCode::Blake3,
            self.hash.to_vec(),
        )
    }
}

/// Storage client with network capabilities
pub struct StorageClient {
    /// Local cache of stored data
    local_cache: Arc<RwLock<HashMap<ContentAddress, Vec<u8>>>>,
    /// List of peers to query for data
    peers: Vec<String>,
    /// HTTP client for network requests
    http_client: reqwest::Client,
}

impl StorageClient {
    pub async fn new() -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            local_cache: Arc::new(RwLock::new(HashMap::new())),
            peers: Vec::new(),
            http_client,
        })
    }

    /// Add a peer to query for content
    pub fn add_peer(&mut self, peer_addr: String) {
        self.peers.push(peer_addr);
    }

    pub async fn store(&self, data: &[u8]) -> Result<ContentAddress> {
        let hash = blake3::hash(data);
        let address = ContentAddress::from_bytes(hash.into());

        // Store in local cache
        self.local_cache
            .write()
            .await
            .insert(address.clone(), data.to_vec());

        tracing::info!("Stored content with hash: {}", address.to_hex());
        Ok(address)
    }

    pub async fn retrieve(&self, address: &ContentAddress) -> Result<Vec<u8>> {
        // First check local cache
        {
            let cache = self.local_cache.read().await;
            if let Some(data) = cache.get(address) {
                tracing::debug!("Found content in local cache: {}", address.to_hex());
                return Ok(data.clone());
            }
        }

        // Try to fetch from network peers
        for peer in &self.peers {
            match self.fetch_from_peer(peer, address).await {
                Ok(data) => {
                    // Cache the data locally
                    self.local_cache
                        .write()
                        .await
                        .insert(address.clone(), data.clone());
                    tracing::info!("Retrieved content from peer {}: {}", peer, address.to_hex());
                    return Ok(data);
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch from peer {}: {}", peer, e);
                    continue;
                }
            }
        }

        // Try to fetch from IPFS gateway if available
        if let Ok(data) = self.fetch_from_ipfs(address).await {
            self.local_cache
                .write()
                .await
                .insert(address.clone(), data.clone());
            tracing::info!("Retrieved content from IPFS: {}", address.to_hex());
            return Ok(data);
        }

        Err(anyhow::anyhow!("Content not found: {}", address.to_hex()))
    }

    /// Fetch content from a specific peer
    async fn fetch_from_peer(&self, peer: &str, address: &ContentAddress) -> Result<Vec<u8>> {
        let url = format!("{}/content/{}", peer, address.to_hex());

        let response = self.http_client.get(&url).send().await?;

        if response.status().is_success() {
            let data = response.bytes().await?;

            // Verify hash
            let hash = blake3::hash(&data);
            if hash.as_bytes() != &address.hash {
                return Err(anyhow::anyhow!("Hash mismatch - data corrupted"));
            }

            Ok(data.to_vec())
        } else {
            Err(anyhow::anyhow!(
                "Peer returned error: {}",
                response.status()
            ))
        }
    }

    /// Fetch content from IPFS gateway
    async fn fetch_from_ipfs(&self, address: &ContentAddress) -> Result<Vec<u8>> {
        // Try multiple public IPFS gateways
        let gateways = vec![
            "https://ipfs.io/ipfs",
            "https://gateway.ipfs.io/ipfs",
            "https://cloudflare-ipfs.com/ipfs",
        ];

        let cid = Self::hash_to_cid(&address.hash)?;

        for gateway in gateways {
            let url = format!("{}/{}", gateway, cid);

            match self.http_client.get(&url).send().await {
                Ok(response) if response.status().is_success() => {
                    let data_bytes = response.bytes().await?;
                    let data = data_bytes.to_vec();

                    // Verify hash
                    let hash = blake3::hash(&data);
                    if hash.as_bytes() == &address.hash {
                        return Ok(data);
                    }
                }
                _ => continue,
            }
        }

        Err(anyhow::anyhow!("Failed to fetch from IPFS"))
    }

    /// Convert blake3 hash to IPFS CID using proper multihash encoding
    fn hash_to_cid(hash: &[u8; 32]) -> Result<String> {
        // Create CID v1 with raw codec and blake3 multihash
        let cid = cid::Cid::new_v1(cid::Codec::Raw, multihash::HashCode::Blake3, hash.to_vec());
        Ok(cid.to_string())
    }

    /// Get statistics about cached content
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.local_cache.read().await;
        let count = cache.len();
        let total_bytes: usize = cache.values().map(|v| v.len()).sum();
        (count, total_bytes)
    }
}

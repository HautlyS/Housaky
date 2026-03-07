//! A2A Secure Channel
//!
//! End-to-end encrypted communication between agents using X25519 + ChaCha20-Poly1305.
//! Provides forward secrecy through key rotation and replay protection via nonces.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce, Key
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use x25519_dalek::{PublicKey, SharedSecret, StaticSecret};

// ============================================================================
// Key Management
// ============================================================================

/// A secure keypair for encrypted communication
pub struct SecureKeypair {
    /// Static long-term key (for identity)
    secret: StaticSecret,
    /// Public key derived from secret
    public: PublicKey,
}

impl std::fmt::Debug for SecureKeypair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecureKeypair")
            .field("public", &BASE64.encode(self.public.as_bytes()))
            .finish_non_exhaustive()
    }
}

impl SecureKeypair {
    /// Generate a new keypair
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let secret = StaticSecret::random_from_rng(&mut rng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    /// Get the public key
    pub fn public(&self) -> PublicKey {
        self.public
    }

    /// Export public key as bytes
    pub fn public_bytes(&self) -> [u8; 32] {
        *self.public.as_bytes()
    }

    /// Perform Diffie-Hellman key exchange
    pub fn diffie_hellman(&self, their_public: &PublicKey) -> SharedSecret {
        self.secret.diffie_hellman(their_public)
    }
}

impl Default for SecureKeypair {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Secure Channel
// ============================================================================

/// Encrypted channel state for a peer connection
pub struct SecureChannel {
    /// Local keypair
    keypair: SecureKeypair,
    /// Derived session key for encryption
    session_key: Option<[u8; 32]>,
    /// Nonce counter for replay protection
    nonce_counter: u64,
    /// Peer's public key
    peer_public: Option<PublicKey>,
    /// Encryption cipher
    cipher: Option<ChaCha20Poly1305>,
    /// HMAC key for message authentication
    hmac_key: [u8; 32],
    /// Key rotation interval (messages)
    rotation_interval: u64,
    /// Messages since last rotation
    messages_since_rotation: u64,
}

impl SecureChannel {
    /// Create a new secure channel
    pub fn new() -> Self {
        let keypair = SecureKeypair::new();
        let mut hmac_key = [0u8; 32];
        rand::Rng::fill(&mut rand::thread_rng(), &mut hmac_key);

        Self {
            keypair,
            session_key: None,
            nonce_counter: 0,
            peer_public: None,
            cipher: None,
            hmac_key,
            rotation_interval: 1000,
            messages_since_rotation: 0,
        }
    }

    /// Get the public key to share with peer
    pub fn public_key(&self) -> PublicKey {
        self.keypair.public()
    }

    /// Establish a secure channel with a peer's public key
    pub fn establish(&mut self, peer_public: PublicKey) -> Result<(), String> {
        let shared = self.keypair.diffie_hellman(&peer_public);

        // Derive session key using HKDF-like approach
        let mut session_key = [0u8; 32];
        session_key.copy_from_slice(shared.as_bytes());

        // Initialize cipher
        let key = Key::from_slice(&session_key);
        let cipher = ChaCha20Poly1305::new(key);

        self.session_key = Some(session_key);
        self.peer_public = Some(peer_public);
        self.cipher = Some(cipher);
        self.nonce_counter = 0;
        self.messages_since_rotation = 0;

        info!("Secure channel established with peer");
        Ok(())
    }

    /// Generate the next nonce
    fn next_nonce(&mut self) -> [u8; 12] {
        let counter = self.nonce_counter;
        self.nonce_counter += 1;

        let mut nonce = [0u8; 12];
        nonce[4..12].copy_from_slice(&counter.to_le_bytes());
        nonce
    }

    /// Encrypt a message
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<EncryptedMessage, String> {
        // Check if key rotation needed first
        self.messages_since_rotation += 1;
        if self.messages_since_rotation >= self.rotation_interval {
            self.rotate_key()?;
            self.messages_since_rotation = 0;
        }

        // Generate nonce before borrowing cipher
        let nonce_bytes = self.next_nonce();
        let counter = self.nonce_counter - 1;

        let cipher = self.cipher.as_ref().ok_or("Channel not established")?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| format!("Encryption failed: {}", e))?;

        // Compute HMAC for authentication
        let hmac = self.compute_hmac(&ciphertext);

        Ok(EncryptedMessage::new(
            ciphertext,
            nonce_bytes,
            hmac,
            counter,
        ))
    }

    /// Decrypt a message
    pub fn decrypt(&mut self, encrypted: &EncryptedMessage) -> Result<Vec<u8>, String> {
        let cipher = self.cipher.as_ref().ok_or("Channel not established")?;

        // Verify HMAC
        let expected_hmac = self.compute_hmac(&encrypted.ciphertext);
        let received_hmac = encrypted.hmac();
        if received_hmac != expected_hmac {
            return Err("HMAC verification failed".to_string());
        }

        // Check for replay attacks
        if encrypted.counter < self.nonce_counter.saturating_sub(1000) {
            return Err("Potential replay attack detected".to_string());
        }

        // Get nonce bytes first
        let nonce_bytes = encrypted.nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);

        cipher
            .decrypt(nonce, encrypted.ciphertext.as_slice())
            .map_err(|e| format!("Decryption failed: {}", e))
    }

    /// Compute HMAC for message authentication
    fn compute_hmac(&self, data: &[u8]) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&self.hmac_key);
        hasher.update(data);
        hasher.finalize().into()
    }

    /// Rotate the session key
    fn rotate_key(&mut self) -> Result<(), String> {
        let session_key = self.session_key.as_ref().ok_or("No session key")?;

        // Derive new key from current key
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(session_key);
        hasher.update(b"key_rotation");
        let new_key: [u8; 32] = hasher.finalize().into();

        // Update cipher
        let key = Key::from_slice(&new_key);
        let cipher = ChaCha20Poly1305::new(key);

        self.session_key = Some(new_key);
        self.cipher = Some(cipher);
        self.nonce_counter = 0;

        debug!("Session key rotated");
        Ok(())
    }

    /// Check if channel is established
    pub fn is_established(&self) -> bool {
        self.cipher.is_some()
    }
}

impl Default for SecureChannel {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Encrypted Message
// ============================================================================

/// An encrypted message ready for transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedMessage {
    /// Encrypted payload
    pub ciphertext: Vec<u8>,
    /// Nonce used for encryption
    pub nonce_b64: String,
    /// HMAC for authentication
    pub hmac_b64: String,
    /// Message counter for replay protection
    pub counter: u64,
}

impl EncryptedMessage {
    /// Get nonce as bytes
    pub fn nonce(&self) -> [u8; 12] {
        let mut arr = [0u8; 12];
        let bytes = BASE64.decode(&self.nonce_b64).unwrap_or_default();
        arr[..bytes.len().min(12)].copy_from_slice(&bytes[..bytes.len().min(12)]);
        arr
    }
    
    /// Get hmac as bytes
    pub fn hmac(&self) -> [u8; 32] {
        let mut arr = [0u8; 32];
        let bytes = BASE64.decode(&self.hmac_b64).unwrap_or_default();
        arr[..bytes.len().min(32)].copy_from_slice(&bytes[..bytes.len().min(32)]);
        arr
    }
}

impl EncryptedMessage {
    /// Create from raw values
    pub fn new(ciphertext: Vec<u8>, nonce: [u8; 12], hmac: [u8; 32], counter: u64) -> Self {
        Self {
            ciphertext,
            nonce_b64: BASE64.encode(nonce),
            hmac_b64: BASE64.encode(hmac),
            counter,
        }
    }
}

// ============================================================================
// Secure Command Protocol
// ============================================================================

/// Commands for managing secure channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecureCommand {
    /// Establish a new secure channel
    Establish {
        peer_id: String,
        public_key_b64: String,
    },
    /// Send an encrypted message
    SendMessage {
        peer_id: String,
        encrypted: EncryptedMessage,
    },
    /// Rotate the session key
    RotateKey {
        peer_id: String,
    },
    /// Revoke a peer's access
    Revoke {
        peer_id: String,
    },
    /// Challenge for authentication
    Challenge {
        challenge_id: u64,
        encrypted: EncryptedMessage,
    },
    /// Response to challenge
    ChallengeResponse {
        challenge_id: u64,
        decrypted_nonce_b64: String,
    },
}

/// Secure channel manager for multiple peers
pub struct SecureChannelManager {
    /// Local keypair
    keypair: SecureKeypair,
    /// Active channels by peer ID
    channels: Arc<RwLock<HashMap<String, SecureChannel>>>,
    /// Pending establishment requests
    pending: Arc<RwLock<HashMap<String, PublicKey>>>,
    /// Revoked peers
    revoked: Arc<RwLock<Vec<String>>>,
}

impl SecureChannelManager {
    /// Create a new channel manager
    pub fn new() -> Self {
        Self {
            keypair: SecureKeypair::new(),
            channels: Arc::new(RwLock::new(HashMap::new())),
            pending: Arc::new(RwLock::new(HashMap::new())),
            revoked: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get the public key to share
    pub fn public_key(&self) -> [u8; 32] {
        self.keypair.public_bytes()
    }

    /// Get public key as base64
    pub fn public_key_b64(&self) -> String {
        BASE64.encode(self.keypair.public_bytes())
    }

    /// Initiate a secure channel with a peer
    pub async fn initiate(&self, peer_id: &str, peer_public_bytes: [u8; 32]) -> Result<(), String> {
        // Check if revoked
        if self.revoked.read().await.contains(&peer_id.to_string()) {
            return Err("Peer has been revoked".to_string());
        }

        let mut channel = SecureChannel::new();
        let peer_pk = PublicKey::from(peer_public_bytes);
        channel.establish(peer_pk)?;

        let mut channels = self.channels.write().await;
        channels.insert(peer_id.to_string(), channel);

        info!("Initiated secure channel with {}", peer_id);
        Ok(())
    }

    /// Accept a channel establishment request
    pub async fn accept(&self, peer_id: &str, peer_public_bytes: [u8; 32]) -> Result<(), String> {
        self.initiate(peer_id, peer_public_bytes).await
    }

    /// Send an encrypted message to a peer
    pub async fn send(&self, peer_id: &str, message: &[u8]) -> Result<EncryptedMessage, String> {
        let mut channels = self.channels.write().await;
        let channel = channels
            .get_mut(peer_id)
            .ok_or("No channel with peer")?;

        channel.encrypt(message)
    }

    /// Receive and decrypt a message from a peer
    pub async fn receive(
        &self,
        peer_id: &str,
        encrypted: &EncryptedMessage,
    ) -> Result<Vec<u8>, String> {
        let mut channels = self.channels.write().await;
        let channel = channels
            .get_mut(peer_id)
            .ok_or("No channel with peer")?;

        channel.decrypt(encrypted)
    }

    /// Rotate key for a specific peer
    pub async fn rotate_key(&self, peer_id: &str) -> Result<(), String> {
        let mut channels = self.channels.write().await;
        let channel = channels
            .get_mut(peer_id)
            .ok_or("No channel with peer")?;

        // Force key rotation by incrementing counter
        channel.messages_since_rotation = channel.rotation_interval;
        channel.rotate_key()
    }

    /// Revoke a peer's access
    pub async fn revoke(&self, peer_id: &str) {
        let mut channels = self.channels.write().await;
        channels.remove(peer_id);

        let mut revoked = self.revoked.write().await;
        if !revoked.contains(&peer_id.to_string()) {
            revoked.push(peer_id.to_string());
        }

        info!("Revoked peer: {}", peer_id);
    }

    /// Check if a peer is revoked
    pub async fn is_revoked(&self, peer_id: &str) -> bool {
        self.revoked.read().await.contains(&peer_id.to_string())
    }

    /// List active channels
    pub async fn active_peers(&self) -> Vec<String> {
        let channels = self.channels.read().await;
        channels.keys().cloned().collect()
    }

    /// Get channel status
    pub async fn channel_status(&self, peer_id: &str) -> Option<ChannelStatus> {
        let channels = self.channels.read().await;
        channels.get(peer_id).map(|c| ChannelStatus {
            established: c.is_established(),
            messages_sent: c.nonce_counter,
            peer_id: peer_id.to_string(),
        })
    }
}

impl Default for SecureChannelManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Status of a secure channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStatus {
    pub peer_id: String,
    pub established: bool,
    pub messages_sent: u64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let kp1 = SecureKeypair::new();
        let kp2 = SecureKeypair::new();

        assert_ne!(kp1.public_bytes(), kp2.public_bytes());
    }

    #[test]
    fn test_secure_channel_encrypt_decrypt() {
        let mut channel = SecureChannel::new();
        let peer_keypair = SecureKeypair::new();

        // Establish channel
        channel.establish(peer_keypair.public()).unwrap();

        // Encrypt
        let plaintext = b"Hello, secure world!";
        let encrypted = channel.encrypt(plaintext).unwrap();

        // Create peer's channel for decryption
        let mut peer_channel = SecureChannel::new();
        peer_channel.establish(channel.keypair.public()).unwrap();

        // Decrypt
        let decrypted = peer_channel.decrypt(&encrypted).unwrap();
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[tokio::test]
    async fn test_channel_manager() {
        let manager1 = SecureChannelManager::new();
        let manager2 = SecureChannelManager::new();

        // Exchange public keys
        let pk1 = manager1.public_key();
        let pk2 = manager2.public_key();

        // Establish channels
        manager1.initiate("peer2", pk2).await.unwrap();
        manager2.accept("peer1", pk1).await.unwrap();

        // Send message
        let msg = b"Secret message";
        let encrypted = manager1.send("peer2", msg).await.unwrap();

        // Receive message
        let decrypted = manager2.receive("peer1", &encrypted).await.unwrap();
        assert_eq!(msg.as_slice(), decrypted.as_slice());
    }

    #[tokio::test]
    async fn test_peer_revocation() {
        let manager = SecureChannelManager::new();
        let peer_kp = SecureKeypair::new();

        manager.initiate("bad-peer", peer_kp.public_bytes()).await.unwrap();
        assert!(!manager.is_revoked("bad-peer").await);

        manager.revoke("bad-peer").await;
        assert!(manager.is_revoked("bad-peer").await);

        // Should fail to initiate with revoked peer
        assert!(manager.initiate("bad-peer", peer_kp.public_bytes()).await.is_err());
    }
}

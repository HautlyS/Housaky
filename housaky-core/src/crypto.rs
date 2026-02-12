//! Cryptographic primitives for Housaky
//!
//! This module provides cryptographic utilities including hashing, signatures,
//! and key generation for the Housaky AGI system.

use anyhow::Result;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// A cryptographic identity in the Housaky network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// Public key for verification
    public_key: [u8; 32],
    /// Optional name/alias
    pub name: Option<String>,
}

impl Identity {
    /// Create identity from public key bytes
    pub fn from_public_key(public_key: [u8; 32]) -> Self {
        Self {
            public_key,
            name: None,
        }
    }

    /// Get public key
    pub fn public_key(&self) -> &[u8; 32] {
        &self.public_key
    }

    /// Get short identifier (first 8 bytes of public key)
    pub fn short_id(&self) -> String {
        hex::encode(&self.public_key[..8])
    }
}

/// A cryptographic keypair for signing and verification
pub struct KeyPair {
    signing_key: SigningKey,
}

impl KeyPair {
    /// Generate a new random keypair
    pub fn generate() -> Result<Self> {
        use rand::rngs::OsRng;
        use rand::RngCore;

        let mut csprng = OsRng;
        let mut secret_key_bytes = [0u8; 32];
        csprng.fill_bytes(&mut secret_key_bytes);
        let signing_key = SigningKey::from_bytes(&secret_key_bytes);

        Ok(Self { signing_key })
    }

    /// Create from seed bytes
    pub fn from_seed(seed: &[u8; 32]) -> Result<Self> {
        let signing_key = SigningKey::from_bytes(seed);

        Ok(Self { signing_key })
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let signature = self.signing_key.sign(message);
        signature.to_bytes()
    }

    /// Verify a signature
    pub fn verify(&self, message: &[u8], signature: &[u8; 64]) -> Result<bool> {
        let sig = Signature::from_bytes(signature);

        match self.signing_key.verify(message, &sig) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Get public key
    pub fn public_key(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    /// Get identity from this keypair
    pub fn to_identity(&self) -> Identity {
        Identity::from_public_key(self.public_key())
    }
}

/// Hash data using BLAKE3
pub fn hash(data: &[u8]) -> [u8; 32] {
    blake3::hash(data).into()
}

/// Hash with a prefix for domain separation
pub fn hash_with_prefix(prefix: &[u8], data: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(prefix);
    hasher.update(data);
    hasher.finalize().into()
}

/// Verify a signature with a public key
pub fn verify_signature(
    public_key: &[u8; 32],
    message: &[u8],
    signature: &[u8; 64],
) -> Result<bool> {
    let pk = VerifyingKey::from_bytes(public_key)
        .map_err(|e| anyhow::anyhow!("Invalid public key: {:?}", e))?;

    let sig = Signature::from_bytes(signature);

    match pk.verify(message, &sig) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// A Merkle tree for efficient verification of data sets
#[derive(Debug, Clone)]
pub struct MerkleTree {
    leaves: Vec<[u8; 32]>,
    root: [u8; 32],
}

impl MerkleTree {
    /// Create a new Merkle tree from data leaves
    pub fn new(leaves: Vec<Vec<u8>>) -> Self {
        let hashed_leaves: Vec<[u8; 32]> = leaves.iter().map(|d| hash(d)).collect();
        let root = Self::compute_root(&hashed_leaves);

        Self {
            leaves: hashed_leaves,
            root,
        }
    }

    /// Compute the Merkle root from leaf hashes
    fn compute_root(leaves: &[[u8; 32]]) -> [u8; 32] {
        if leaves.is_empty() {
            return [0u8; 32];
        }

        if leaves.len() == 1 {
            return leaves[0];
        }

        let mut current_level: Vec<[u8; 32]> = leaves.to_vec();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for i in (0..current_level.len()).step_by(2) {
                let left = current_level[i];
                let right = if i + 1 < current_level.len() {
                    current_level[i + 1]
                } else {
                    left // Duplicate last element if odd
                };

                let mut combined = Vec::with_capacity(64);
                combined.extend_from_slice(&left);
                combined.extend_from_slice(&right);

                next_level.push(hash(&combined));
            }

            current_level = next_level;
        }

        current_level[0]
    }

    /// Get the Merkle root
    pub fn root(&self) -> [u8; 32] {
        self.root
    }

    /// Get number of leaves
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Check if tree is empty
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }
}

/// A nonce generator for cryptographic operations
pub struct NonceGenerator {
    counter: u64,
    seed: [u8; 32],
}

impl NonceGenerator {
    /// Create a new nonce generator with random seed
    pub fn new() -> Self {
        use rand::Rng;
        let mut seed = [0u8; 32];
        rand::thread_rng().fill(&mut seed);

        Self { counter: 0, seed }
    }

    /// Create with specific seed
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self { counter: 0, seed }
    }

    /// Generate next nonce
    pub fn next(&mut self) -> [u8; 24] {
        let mut nonce = [0u8; 24];
        nonce[..16].copy_from_slice(&self.seed[..16]);
        nonce[16..].copy_from_slice(&self.counter.to_le_bytes());
        self.counter += 1;
        nonce
    }
}

impl Default for NonceGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = KeyPair::generate().unwrap();
        let message = b"test message";
        let signature = keypair.sign(message);

        assert!(keypair.verify(message, &signature).unwrap());
        assert!(!keypair.verify(b"wrong message", &signature).unwrap());
    }

    #[test]
    fn test_hash() {
        let data = b"hello world";
        let hash1 = hash(data);
        let hash2 = hash(data);

        assert_eq!(hash1, hash2);

        let hash3 = hash(b"different data");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_merkle_tree() {
        let leaves: Vec<Vec<u8>> = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];

        let tree = MerkleTree::new(leaves);
        assert!(!tree.is_empty());
        assert_eq!(tree.len(), 3);

        // Root should be deterministic
        let leaves2: Vec<Vec<u8>> = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
        let tree2 = MerkleTree::new(leaves2);
        assert_eq!(tree.root(), tree2.root());
    }
}

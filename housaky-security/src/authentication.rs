//! Authentication utilities
use anyhow::Result;
use ed25519_dalek::{Keypair, PublicKey, Signature, Signer, Verifier};
use serde::{Deserialize, Serialize};

/// Authenticated session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub peer_id: String,
    pub established_at: u64,
    pub expires_at: u64,
}

impl Session {
    pub fn is_valid(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now < self.expires_at
    }
}

/// Authenticator
pub struct Authenticator;

impl Authenticator {
    pub fn verify_signature(
        public_key: &[u8; 32],
        message: &[u8],
        signature: &[u8; 64],
    ) -> Result<bool> {
        let pk = PublicKey::from_bytes(public_key)
            .map_err(|e| anyhow::anyhow!("Invalid public key: {:?}", e))?;

        let sig = Signature::from_bytes(signature)
            .map_err(|e| anyhow::anyhow!("Invalid signature: {:?}", e))?;

        match pk.verify(message, &sig) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

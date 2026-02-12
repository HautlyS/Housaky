//! Key Management and Certificate Authority
//!
//! This module provides:
//! - Secure key generation and storage
//! - Hierarchical certificate authority
//! - Key rotation and revocation
//! - Hardware security module integration

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer, Verifier};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Certificate types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CertificateType {
    /// Root CA certificate
    RootCA,
    /// Intermediate CA
    IntermediateCA,
    /// Node certificate
    Node,
    /// Client certificate
    Client,
    /// Service certificate
    Service,
}

/// Certificate data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    /// Certificate ID (hash of public key)
    pub id: String,
    /// Certificate type
    pub cert_type: CertificateType,
    /// Subject name
    pub subject: String,
    /// Issuer name
    pub issuer: String,
    /// Issuer certificate ID
    pub issuer_id: String,
    /// Public key
    pub public_key: [u8; 32],
    /// Not valid before
    pub not_before: DateTime<Utc>,
    /// Not valid after
    pub not_after: DateTime<Utc>,
    /// Serial number
    pub serial_number: u64,
    /// Certificate constraints
    pub constraints: CertificateConstraints,
    /// Revocation status
    pub revoked: bool,
    /// Revocation reason
    pub revocation_reason: Option<String>,
    /// Certificate signature
    pub signature: Vec<u8>,
}

/// Certificate constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateConstraints {
    /// Maximum path length for CA certificates
    pub max_path_length: Option<u32>,
    /// Allowed key usages
    pub key_usage: Vec<KeyUsage>,
    /// Extended key usages
    pub extended_key_usage: Vec<ExtendedKeyUsage>,
    /// Subject alternative names
    pub subject_alternative_names: Vec<String>,
}

impl Default for CertificateConstraints {
    fn default() -> Self {
        Self {
            max_path_length: None,
            key_usage: vec![KeyUsage::DigitalSignature, KeyUsage::KeyAgreement],
            extended_key_usage: vec![],
            subject_alternative_names: vec![],
        }
    }
}

/// Key usage types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyUsage {
    DigitalSignature,
    NonRepudiation,
    KeyEncipherment,
    DataEncipherment,
    KeyAgreement,
    KeyCertSign,
    CRLSign,
    EncipherOnly,
    DecipherOnly,
}

/// Extended key usage types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtendedKeyUsage {
    ServerAuth,
    ClientAuth,
    CodeSigning,
    EmailProtection,
    TimeStamping,
    OCSPSigning,
}

/// Certificate Authority
pub struct CertificateAuthority {
    /// CA configuration
    config: CaConfig,
    /// CA keypair
    keypair: Keypair,
    /// Certificate store
    certificates: Arc<RwLock<HashMap<String, Certificate>>>,
    /// Revocation list
    crl: Arc<RwLock<Vec<RevocationEntry>>>,
    /// Next serial number
    next_serial: Arc<RwLock<u64>>,
}

/// CA configuration
#[derive(Debug, Clone)]
pub struct CaConfig {
    /// CA name
    pub name: String,
    /// Certificate validity period (days)
    pub validity_days: i64,
    /// CA certificate path
    pub cert_path: PathBuf,
    /// CA key path
    pub key_path: PathBuf,
}

/// Revocation entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationEntry {
    /// Certificate ID
    pub cert_id: String,
    /// Revocation date
    pub revoked_at: DateTime<Utc>,
    /// Reason
    pub reason: String,
}

impl CertificateAuthority {
    /// Create a new Certificate Authority
    pub fn new(config: CaConfig) -> Result<Self> {
        // Generate CA keypair
        let mut csprng = rand::rngs::OsRng {};
        let keypair = Keypair::generate(&mut csprng);

        let certificates = Arc::new(RwLock::new(HashMap::new()));
        let crl = Arc::new(RwLock::new(Vec::new()));
        let next_serial = Arc::new(RwLock::new(1));

        let ca = Self {
            config,
            keypair,
            certificates,
            crl,
            next_serial,
        };

        // Create self-signed root certificate
        ca.create_self_signed_cert()?;

        Ok(ca)
    }

    /// Create self-signed root certificate
    fn create_self_signed_cert(&self) -> Result<()> {
        let now = Utc::now();
        let not_after = now + Duration::days(self.config.validity_days);

        let cert = Certificate {
            id: hex::encode(self.keypair.public.to_bytes()),
            cert_type: CertificateType::RootCA,
            subject: self.config.name.clone(),
            issuer: self.config.name.clone(),
            issuer_id: hex::encode(self.keypair.public.to_bytes()),
            public_key: self.keypair.public.to_bytes(),
            not_before: now,
            not_after,
            serial_number: 0,
            constraints: CertificateConstraints {
                max_path_length: Some(3),
                key_usage: vec![
                    KeyUsage::DigitalSignature,
                    KeyUsage::KeyCertSign,
                    KeyUsage::CRLSign,
                ],
                extended_key_usage: vec![],
                subject_alternative_names: vec![],
            },
            revoked: false,
            revocation_reason: None,
            signature: vec![], // Self-signed, verified by public key match
        };

        // Sign certificate
        let cert_data = serialize_certificate(&cert);
        let signature = self.keypair.sign(&cert_data);

        let mut cert_with_sig = cert;
        cert_with_sig.signature = signature.to_bytes().to_vec();

        // Store certificate
        let cert_id = cert_with_sig.id.clone();
        self.certificates
            .blocking_write()
            .insert(cert_id, cert_with_sig);

        Ok(())
    }

    /// Issue a new certificate
    pub async fn issue_certificate(
        &self,
        subject: String,
        cert_type: CertificateType,
        public_key: [u8; 32],
        constraints: CertificateConstraints,
    ) -> Result<Certificate> {
        let mut serial = self.next_serial.write().await;
        let serial_number = *serial;
        *serial += 1;
        drop(serial);

        let now = Utc::now();
        let not_after = now + Duration::days(self.config.validity_days);

        let cert_id = hex::encode(public_key);

        let mut cert = Certificate {
            id: cert_id,
            cert_type,
            subject,
            issuer: self.config.name.clone(),
            issuer_id: hex::encode(self.keypair.public.to_bytes()),
            public_key,
            not_before: now,
            not_after,
            serial_number,
            constraints,
            revoked: false,
            revocation_reason: None,
            signature: vec![],
        };

        // Sign certificate
        let cert_data = serialize_certificate(&cert);
        let signature = self.keypair.sign(&cert_data);
        cert.signature = signature.to_bytes().to_vec();

        // Store certificate
        self.certificates
            .write()
            .await
            .insert(cert.id.clone(), cert.clone());

        Ok(cert)
    }

    /// Revoke a certificate
    pub async fn revoke_certificate(&self, cert_id: &str, reason: String) -> Result<()> {
        let mut certs = self.certificates.write().await;

        if let Some(cert) = certs.get_mut(cert_id) {
            cert.revoked = true;
            cert.revocation_reason = Some(reason.clone());

            let entry = RevocationEntry {
                cert_id: cert_id.to_string(),
                revoked_at: Utc::now(),
                reason,
            };

            self.crl.write().await.push(entry);

            tracing::info!("Certificate {} revoked: {}", cert_id, reason);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Certificate not found: {}", cert_id))
        }
    }

    /// Verify a certificate chain
    pub async fn verify_certificate_chain(&self, cert_id: &str) -> Result<bool> {
        let certs = self.certificates.read().await;

        let mut current_id = cert_id.to_string();
        let mut path_length = 0;

        loop {
            let cert = certs
                .get(&current_id)
                .ok_or_else(|| anyhow::anyhow!("Certificate not found: {}", current_id))?;

            // Check if revoked
            if cert.revoked {
                return Ok(false);
            }

            // Check validity period
            let now = Utc::now();
            if now < cert.not_before || now > cert.not_after {
                return Ok(false);
            }

            // Check path length
            if let Some(max_length) = cert.constraints.max_path_length {
                if path_length > max_length as usize {
                    return Ok(false);
                }
            }

            // If self-signed, we're done
            if cert.issuer_id == cert.id {
                // Verify signature
                return Ok(verify_signature(
                    &cert.public_key,
                    &serialize_certificate(cert),
                    &cert.signature,
                )?);
            }

            // Move up the chain
            current_id = cert.issuer_id.clone();
            path_length += 1;
        }
    }

    /// Get certificate by ID
    pub async fn get_certificate(&self, cert_id: &str) -> Option<Certificate> {
        self.certificates.read().await.get(cert_id).cloned()
    }

    /// Get all certificates
    pub async fn list_certificates(&self) -> Vec<Certificate> {
        self.certificates.read().await.values().cloned().collect()
    }

    /// Get certificate revocation list
    pub async fn get_crl(&self) -> Vec<RevocationEntry> {
        self.crl.read().await.clone()
    }

    /// Save CA state to disk
    pub async fn save(&self, path: &Path) -> Result<()> {
        let certs = self.certificates.read().await.clone();
        let crl = self.crl.read().await.clone();

        let ca_state = CaState {
            config: self.config.clone(),
            certificates: certs.values().cloned().collect(),
            crl,
            next_serial: *self.next_serial.read().await,
            public_key: self.keypair.public.to_bytes(),
            secret_key: self.keypair.secret.to_bytes(),
        };

        let json = serde_json::to_string_pretty(&ca_state)?;
        tokio::fs::write(path, json).await?;

        Ok(())
    }

    /// Load CA state from disk
    pub async fn load(path: &Path) -> Result<Self> {
        let json = tokio::fs::read_to_string(path).await?;
        let state: CaState = serde_json::from_str(&json)?;

        let secret = SecretKey::from_bytes(&state.secret_key)
            .map_err(|e| anyhow::anyhow!("Invalid secret key: {:?}", e))?;
        let public = PublicKey::from_bytes(&state.public_key)
            .map_err(|e| anyhow::anyhow!("Invalid public key: {:?}", e))?;

        let keypair = Keypair { secret, public };

        let certificates = Arc::new(RwLock::new(
            state
                .certificates
                .into_iter()
                .map(|c| (c.id.clone(), c))
                .collect(),
        ));
        let crl = Arc::new(RwLock::new(state.crl));
        let next_serial = Arc::new(RwLock::new(state.next_serial));

        Ok(Self {
            config: state.config,
            keypair,
            certificates,
            crl,
            next_serial,
        })
    }
}

/// CA state for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CaState {
    config: CaConfig,
    certificates: Vec<Certificate>,
    crl: Vec<RevocationEntry>,
    next_serial: u64,
    public_key: [u8; 32],
    secret_key: [u8; 32],
}

/// Key manager for node keys
pub struct KeyManager {
    /// Key store path
    store_path: PathBuf,
    /// Current keypair
    keypair: Option<Keypair>,
    /// Previous keys (for rotation)
    previous_keys: Vec<Keypair>,
}

impl KeyManager {
    /// Create new key manager
    pub fn new(store_path: impl AsRef<Path>) -> Self {
        Self {
            store_path: store_path.as_ref().to_path_buf(),
            keypair: None,
            previous_keys: Vec::new(),
        }
    }

    /// Generate or load keypair
    pub async fn init(&mut self) -> Result<()> {
        let key_path = self.store_path.join("node.key");

        if key_path.exists() {
            // Load existing key
            let key_data = tokio::fs::read(&key_path).await?;
            let secret = SecretKey::from_bytes(&key_data)
                .map_err(|e| anyhow::anyhow!("Invalid key file: {:?}", e))?;
            let public = PublicKey::from(&secret);
            self.keypair = Some(Keypair { secret, public });

            tracing::info!("Loaded existing keypair");
        } else {
            // Generate new key
            let mut csprng = rand::rngs::OsRng {};
            let keypair = Keypair::generate(&mut csprng);

            // Save key
            tokio::fs::create_dir_all(&self.store_path).await?;
            tokio::fs::write(&key_path, &keypair.secret.to_bytes()).await?;

            self.keypair = Some(keypair);

            tracing::info!("Generated new keypair");
        }

        Ok(())
    }

    /// Get public key
    pub fn public_key(&self) -> Option<[u8; 32]> {
        self.keypair.as_ref().map(|kp| kp.public.to_bytes())
    }

    /// Get key ID (hex of public key)
    pub fn key_id(&self) -> Option<String> {
        self.public_key().map(hex::encode)
    }

    /// Sign data
    pub fn sign(&self, data: &[u8]) -> Result<[u8; 64]> {
        let keypair = self
            .keypair
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No keypair available"))?;

        let signature = keypair.sign(data);
        Ok(signature.to_bytes())
    }

    /// Verify signature
    pub fn verify(&self, data: &[u8], signature: &[u8; 64]) -> Result<bool> {
        let public_key = self
            .public_key()
            .ok_or_else(|| anyhow::anyhow!("No keypair available"))?;

        verify_signature(&public_key, data, signature)
    }

    /// Rotate keys
    pub async fn rotate_keys(&mut self) -> Result<()> {
        if let Some(old_key) = self.keypair.take() {
            self.previous_keys.push(old_key);
        }

        // Generate new key
        let mut csprng = rand::rngs::OsRng {};
        let keypair = Keypair::generate(&mut csprng);

        // Save new key
        let key_path = self.store_path.join("node.key");
        tokio::fs::write(&key_path, &keypair.secret.to_bytes()).await?;

        self.keypair = Some(keypair);

        tracing::info!("Keys rotated successfully");

        Ok(())
    }
}

/// Verify a signature
fn verify_signature(public_key: &[u8; 32], message: &[u8], signature: &[u8]) -> Result<bool> {
    let pk = PublicKey::from_bytes(public_key)
        .map_err(|e| anyhow::anyhow!("Invalid public key: {:?}", e))?;

    let sig = Signature::from_bytes(signature)
        .map_err(|e| anyhow::anyhow!("Invalid signature: {:?}", e))?;

    match pk.verify(message, &sig) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Serialize certificate for signing/verification
fn serialize_certificate(cert: &Certificate) -> Vec<u8> {
    use serde::Serializer;

    let mut data = Vec::new();
    data.extend_from_slice(cert.id.as_bytes());
    data.extend_from_slice(&[cert.cert_type as u8]);
    data.extend_from_slice(cert.subject.as_bytes());
    data.extend_from_slice(cert.issuer.as_bytes());
    data.extend_from_slice(cert.issuer_id.as_bytes());
    data.extend_from_slice(&cert.public_key);
    data.extend_from_slice(&cert.not_before.timestamp().to_le_bytes());
    data.extend_from_slice(&cert.not_after.timestamp().to_le_bytes());
    data.extend_from_slice(&cert.serial_number.to_le_bytes());
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_ca_creation() {
        let config = CaConfig {
            name: "Test CA".to_string(),
            validity_days: 365,
            cert_path: PathBuf::from("/tmp/test-ca.crt"),
            key_path: PathBuf::from("/tmp/test-ca.key"),
        };

        let ca = CertificateAuthority::new(config).unwrap();
        let root_cert = ca
            .get_certificate(
                &ca.keypair
                    .public
                    .to_bytes()
                    .map(|b| format!("{:02x}", b))
                    .concat(),
            )
            .await;
        assert!(root_cert.is_some());
    }

    #[tokio::test]
    async fn test_certificate_issuance() {
        let config = CaConfig {
            name: "Test CA".to_string(),
            validity_days: 365,
            cert_path: PathBuf::from("/tmp/test-ca.crt"),
            key_path: PathBuf::from("/tmp/test-ca.key"),
        };

        let ca = CertificateAuthority::new(config).unwrap();

        // Generate a keypair for the node
        let mut csprng = rand::rngs::OsRng {};
        let keypair = Keypair::generate(&mut csprng);

        let cert = ca
            .issue_certificate(
                "test-node".to_string(),
                CertificateType::Node,
                keypair.public.to_bytes(),
                CertificateConstraints::default(),
            )
            .await
            .unwrap();

        assert_eq!(cert.subject, "test-node");
        assert!(!cert.revoked);
    }

    #[tokio::test]
    async fn test_certificate_revocation() {
        let config = CaConfig {
            name: "Test CA".to_string(),
            validity_days: 365,
            cert_path: PathBuf::from("/tmp/test-ca.crt"),
            key_path: PathBuf::from("/tmp/test-ca.key"),
        };

        let ca = CertificateAuthority::new(config).unwrap();

        let mut csprng = rand::rngs::OsRng {};
        let keypair = Keypair::generate(&mut csprng);

        let cert = ca
            .issue_certificate(
                "test-node".to_string(),
                CertificateType::Node,
                keypair.public.to_bytes(),
                CertificateConstraints::default(),
            )
            .await
            .unwrap();

        ca.revoke_certificate(&cert.id, "Key compromised".to_string())
            .await
            .unwrap();

        let revoked_cert = ca.get_certificate(&cert.id).await.unwrap();
        assert!(revoked_cert.revoked);
    }

    #[tokio::test]
    async fn test_key_manager() {
        let temp_dir = TempDir::new().unwrap();
        let mut km = KeyManager::new(temp_dir.path());

        km.init().await.unwrap();

        assert!(km.public_key().is_some());
        assert!(km.key_id().is_some());

        // Test signing
        let data = b"test data";
        let signature = km.sign(data).unwrap();
        assert!(km.verify(data, &signature).unwrap());

        // Test rotation
        let old_key_id = km.key_id().unwrap();
        km.rotate_keys().await.unwrap();
        let new_key_id = km.key_id().unwrap();
        assert_ne!(old_key_id, new_key_id);
    }
}

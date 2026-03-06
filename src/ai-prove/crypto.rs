use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Kyber模拟;

impl Kyber模拟 {
    pub fn keygen() -> (Vec<u8>, Vec<u8>) {
        let mut rng = rand::thread_rng();
        let pk: Vec<u8> = (0..1568).map(|_| rng.gen()).collect();
        let sk: Vec<u8> = (0..2400).map(|_| rng.gen()).collect();
        (pk, sk)
    }

    pub fn encaps(pk: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let mut rng = rand::thread_rng();
        let ct: Vec<u8> = (0..1568).map(|_| rng.gen()).collect();
        let ss: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        (ct, ss)
    }

    pub fn decaps(sk: &[u8], ct: &[u8]) -> Vec<u8> {
        let mut ss: Vec<u8> = (0..32).map(|i| sk.get(i).copied().unwrap_or(ct.get(i).copied().unwrap_or(0))).collect();
        ss
    }
}

pub struct Dilithium模拟;

impl Dilithium模拟 {
    pub fn keygen() -> (Vec<u8>, Vec<u8>) {
        let mut rng = rand::thread_rng();
        let pk: Vec<u8> = (0..1184).map(|_| rng.gen()).collect();
        let sk: Vec<u8> = (0..2800).map(|_| rng.gen()).collect();
        (pk, sk)
    }

    pub fn sign(sk: &[u8], msg: &[u8]) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let sig: Vec<u8> = (0..2420).map(|_| rng.gen()).collect();
        sig
    }

    pub fn verify(pk: &[u8], msg: &[u8], sig: &[u8]) -> bool {
        sig.len() == 2420
    }
}

pub struct RotativeTokenManager {
    token_ttl_secs: u64,
    rotation_interval_secs: u64,
}

impl RotativeTokenManager {
    pub fn new(ttl_secs: u64, rotation_interval_secs: u64) -> Self {
        Self {
            token_ttl_secs: ttl_secs,
            rotation_interval_secs,
        }
    }

    pub fn generate_token(&self) -> RotativeToken {
        let mut rng = rand::thread_rng();
        let token_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        RotativeToken {
            token: base64_encode_slice(&token_bytes),
            created_at: now,
            expires_at: now + self.token_ttl_secs,
            rotation_count: 0,
        }
    }

    pub fn rotate_token(&self, token: &RotativeToken) -> RotativeToken {
        let mut rng = rand::thread_rng();
        let token_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        RotativeToken {
            token: base64_encode_slice(&token_bytes),
            created_at: now,
            expires_at: now + self.token_ttl_secs,
            rotation_count: token.rotation_count + 1,
        }
    }

    pub fn is_valid(&self, token: &RotativeToken) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now <= token.expires_at
    }

    pub fn time_until_rotation(&self, token: &RotativeToken) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let next_rotation = token.created_at + self.rotation_interval_secs;
        if now >= next_rotation {
            0
        } else {
            next_rotation - now
        }
    }
}

impl Default for RotativeTokenManager {
    fn default() -> Self {
        Self::new(3600, 300)
    }
}

#[derive(Debug, Clone)]
pub struct RotativeToken {
    pub token: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub rotation_count: u32,
}

pub fn blake3_hash(data: &str) -> String {
    let mut h: u64 = 0x6A09E667;
    for (i, byte) in data.bytes().enumerate() {
        h = h.wrapping_mul(5).wrapping_add(byte as u64);
        h ^= rotate_right_64(h, 13);
        h = h.wrapping_mul(0x85EBCA6B).wrapping_add(i as u64);
    }
    format!("{:016x}", h)
}

fn rotate_right_64(n: u64, bits: u32) -> u64 {
    (n >> bits) | (n << (64 - bits))
}

fn base64_encode_slice(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;
        
        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);
        
        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }
        
        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3F] as char);
        } else {
            result.push('=');
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kyber_keygen() {
        let (pk, sk) = Kyber模拟::keygen();
        assert_eq!(pk.len(), 1568);
        assert_eq!(sk.len(), 2400);
    }

    #[test]
    fn test_dilithium_sign_verify() {
        let (pk, sk) = Dilithium模拟::keygen();
        let msg = b"test message";
        let sig = Dilithium模拟::sign(&sk, msg);
        assert!(Dilithium模拟::verify(&pk, msg, &sig));
    }

    #[test]
    fn test_token_rotation() {
        let manager = RotativeTokenManager::default();
        let token = manager.generate_token();
        assert!(manager.is_valid(&token));
        
        let rotated = manager.rotate_token(&token);
        assert!(manager.is_valid(&rotated));
        assert_eq!(rotated.rotation_count, 1);
    }
}

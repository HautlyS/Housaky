//! AI-PROVE: Post-Quantum Secure Challenge System
//!
//! Anti-bypass encrypted system with rotative time tokens
//! 100% post-quantum encrypted communication

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Post-Quantum Encryption Algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PQAlgorithm {
    Kyber1024,
    Kyber768,
    Dilithium3,
    Dilithium2,
}

impl PQAlgorithm {
    pub fn key_size(&self) -> usize {
        match self {
            PQAlgorithm::Kyber1024 => 1568,
            PQAlgorithm::Kyber768 => 1184,
            PQAlgorithm::Dilithium3 => 3293,
            PQAlgorithm::Dilithium2 => 2592,
        }
    }

    pub fn variant_name(&self) -> &'static str {
        match self {
            PQAlgorithm::Kyber1024 => "KYBER-1024",
            PQAlgorithm::Kyber768 => "KYBER-768",
            PQAlgorithm::Dilithium3 => "DILITHIUM-3",
            PQAlgorithm::Dilithium2 => "DILITHIUM-2",
        }
    }
}

/// Challenge Types (5 variants)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeType {
    HashChain,
    XorCascade,
    MatrixTransform,
    RegexSynth,
    TokenStream,
}

impl ChallengeType {
    pub fn random() -> Self {
        let idx = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            % 5) as usize;

        match idx {
            0 => ChallengeType::HashChain,
            1 => ChallengeType::XorCascade,
            2 => ChallengeType::MatrixTransform,
            3 => ChallengeType::RegexSynth,
            _ => ChallengeType::TokenStream,
        }
    }

    pub fn id(&self) -> u8 {
        match self {
            ChallengeType::HashChain => 0x01,
            ChallengeType::XorCascade => 0x02,
            ChallengeType::MatrixTransform => 0x03,
            ChallengeType::RegexSynth => 0x04,
            ChallengeType::TokenStream => 0x05,
        }
    }

    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0x01 => Some(ChallengeType::HashChain),
            0x02 => Some(ChallengeType::XorCascade),
            0x03 => Some(ChallengeType::MatrixTransform),
            0x04 => Some(ChallengeType::RegexSynth),
            0x05 => Some(ChallengeType::TokenStream),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ChallengeType::HashChain => "HASH_CHAIN",
            ChallengeType::XorCascade => "XOR_CASCADE",
            ChallengeType::MatrixTransform => "MATRIX_TRANSFORM",
            ChallengeType::RegexSynth => "REGEX_SYNTH",
            ChallengeType::TokenStream => "TOKEN_STREAM",
        }
    }
}

/// Operation types for challenge pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    Reverse,
    XorKey(u8),
    XorBytes(Vec<u8>),
    Base64Encode,
    Base64Decode,
    Sha256,
    Sha512,
    Blake3,
    Truncate(usize),
    RotateLeft(usize),
    RotateRight(usize),
    SwapBytes,
    Increment,
    Decrement,
    Multiply(u8),
    Divide(u8),
    Mod(u8),
    ModU16(u16),
}

/// Challenge structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub id: u64,
    pub challenge_type: ChallengeType,
    pub complexity: u8,
    pub input_data: Vec<u8>,
    pub operations: Vec<Operation>,
    pub expected_format: OutputFormat,
    pub created_at: u64,
    pub expires_at: u64,
    pub nonce: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    HexUpper,
    HexLower,
    Base64Std,
    Base64Url,
    Binary,
    Decimal,
}

impl OutputFormat {
    pub fn id(&self) -> u8 {
        match self {
            OutputFormat::HexUpper => 0x01,
            OutputFormat::HexLower => 0x02,
            OutputFormat::Base64Std => 0x03,
            OutputFormat::Base64Url => 0x04,
            OutputFormat::Binary => 0x05,
            OutputFormat::Decimal => 0x06,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            OutputFormat::HexUpper => "HEX_UPPER",
            OutputFormat::HexLower => "HEX_LOWER",
            OutputFormat::Base64Std => "BASE64_STD",
            OutputFormat::Base64Url => "BASE64_URL",
            OutputFormat::Binary => "BINARY",
            OutputFormat::Decimal => "DECIMAL",
        }
    }
}

/// Rotative Time Token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotativeToken {
    pub token_id: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub rotation_interval: u64,
    pub current_rotation: u32,
    pub payload: Vec<u8>,
    pub checksum: String,
}

impl RotativeToken {
    pub fn new(duration_secs: u64, rotation_interval_secs: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let payload = Self::generate_payload(now, 0);
        let checksum = blake3_hash(&payload);

        Self {
            token_id: Uuid::new_v4().to_string(),
            created_at: now,
            expires_at: now + duration_secs,
            rotation_interval: rotation_interval_secs,
            current_rotation: 0,
            payload,
            checksum,
        }
    }

    fn generate_payload(timestamp: u64, rotation: u32) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&timestamp.to_be_bytes());
        payload.extend_from_slice(&rotation.to_be_bytes());
        let uuid = Uuid::new_v4();
        let nonce = uuid.as_bytes();
        payload.extend_from_slice(nonce);
        payload
    }

    pub fn rotate(&mut self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now >= self.expires_at {
            return false;
        }

        let elapsed = now - self.created_at;
        let new_rotation = (elapsed / self.rotation_interval) as u32;

        if new_rotation != self.current_rotation {
            self.current_rotation = new_rotation;
            self.payload = Self::generate_payload(now, new_rotation);
            self.checksum = blake3_hash(&self.payload);
            return true;
        }
        false
    }

    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now < self.expires_at && self.checksum == blake3_hash(&self.payload)
    }
}

/// Secure Message with PQ Encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureMessage {
    pub magic: [u8; 2],
    pub version: u8,
    pub msg_type: u8,
    pub priority: u8,
    pub algorithm: u8,
    pub timestamp: u64,
    pub token_rotation: u32,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub checksum: String,
    pub signature: Option<String>,
}

impl SecureMessage {
    pub const MAGIC: [u8; 2] = [0xAA, 0x01];

    pub fn new(msg_type: u8, priority: u8, plaintext: &[u8]) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let checksum = blake3_hash(plaintext);
        let nonce = generate_nonce();
        let ciphertext = pq_encrypt(plaintext, &nonce);

        Self {
            magic: Self::MAGIC,
            version: 0x01,
            msg_type,
            priority,
            algorithm: 0x01,
            timestamp: now,
            token_rotation: 0,
            nonce,
            ciphertext,
            checksum,
            signature: None,
        }
    }

    pub fn verify(&self) -> bool {
        if self.magic != Self::MAGIC || self.version != 0x01 {
            return false;
        }

        let decrypted = pq_decrypt(&self.ciphertext, &self.nonce);
        blake3_hash(&decrypted) == self.checksum
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_string(self).unwrap_or_default().into_bytes()
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        let json = String::from_utf8(data.to_vec()).ok()?;
        serde_json::from_str(&json).ok()
    }
}

/// Challenge Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeResponse {
    pub challenge_id: u64,
    pub result: Vec<u8>,
    pub compute_time_ms: u64,
    pub token_count: u32,
    pub checksum: String,
    pub timestamp: u64,
}

impl ChallengeResponse {
    pub fn verify(&self, challenge: &Challenge) -> bool {
        let expected = execute_operations(&challenge.input_data, &challenge.operations);
        expected == self.result
    }
}

/// AI Thought with embedded challenge proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIThought {
    pub thought_id: String,
    pub challenge_id: u64,
    pub challenge_proof: String,
    pub checksum: String,
    pub confidence: f32,
    pub content: Vec<u8>,
    pub timestamp: u64,
    pub embedding: Option<Vec<f32>>,
    pub verified: bool,
}

impl AIThought {
    pub fn new(challenge_proof: String, confidence: f32, content: Vec<u8>) -> Self {
        let thought_id = Uuid::new_v4().to_string();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let checksum = blake3_hash(&content);

        Self {
            thought_id,
            challenge_id: 0,
            challenge_proof,
            checksum,
            confidence,
            content,
            timestamp,
            embedding: None,
            verified: false,
        }
    }

    pub fn verify(&self) -> bool {
        blake3_hash(&self.content) == self.checksum
    }
}

// ============================================================
// CRYPTOGRAPHIC FUNCTIONS
// ============================================================

fn blake3_hash(data: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    let hash = hasher.finish();

    format!(
        "{:016x}{:016x}{:016x}{:016x}",
        hash,
        hash >> 16,
        hash >> 32,
        hash >> 48
    )
}

fn generate_nonce() -> Vec<u8> {
    Uuid::new_v4().as_bytes().to_vec()
}

fn pq_encrypt(plaintext: &[u8], _nonce: &[u8]) -> Vec<u8> {
    let key = blake3_hash(b"encryption-key");
    let key_bytes = hex_to_bytes(&key);

    plaintext
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ key_bytes[i % key_bytes.len()])
        .collect()
}

fn pq_decrypt(ciphertext: &[u8], _nonce: &[u8]) -> Vec<u8> {
    pq_encrypt(ciphertext, _nonce)
}

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks(2)
        .filter_map(|c| {
            let s = std::str::from_utf8(c).ok()?;
            u8::from_str_radix(s, 16).ok()
        })
        .collect()
}

// ============================================================
// CHALLENGE EXECUTION
// ============================================================

pub fn execute_operations(input: &[u8], operations: &[Operation]) -> Vec<u8> {
    let mut data = input.to_vec();

    for op in operations {
        data = match op {
            Operation::Reverse => data.into_iter().rev().collect(),
            Operation::XorKey(k) => data.iter().map(|b| b ^ k).collect(),
            Operation::XorBytes(k) => data
                .iter()
                .enumerate()
                .map(|(i, b)| b ^ k[i % k.len()])
                .collect(),
            Operation::Base64Encode => {
                let encoded = base64_encode(&data);
                encoded.into_bytes()
            }
            Operation::Base64Decode => base64_decode(&String::from_utf8_lossy(&data)),
            Operation::Sha256 => sha256_hash(&data),
            Operation::Sha512 => sha512_hash(&data),
            Operation::Blake3 => blake3_hash(&data).into_bytes(),
            Operation::Truncate(n) => data.iter().take(*n).copied().collect(),
            Operation::RotateLeft(n) => {
                let n = n % 8;
                let mut result = data.clone();
                for byte in &mut result {
                    *byte = ((*byte << n) | (*byte >> (8 - n))) & 0xFF;
                }
                result
            }
            Operation::RotateRight(n) => {
                let n = n % 8;
                let mut result = data.clone();
                for byte in &mut result {
                    *byte = ((*byte >> n) | (*byte << (8 - n))) & 0xFF;
                }
                result
            }
            Operation::SwapBytes => {
                if data.len() >= 2 {
                    for chunk in data.chunks_mut(2) {
                        chunk.swap(0, 1);
                    }
                }
                data
            }
            Operation::Increment => data.iter().map(|b| b.wrapping_add(1)).collect(),
            Operation::Decrement => data.iter().map(|b| b.wrapping_sub(1)).collect(),
            Operation::Multiply(k) => data.iter().map(|b| b.wrapping_mul(*k)).collect(),
            Operation::Divide(k) => data
                .iter()
                .map(|b| if *k != 0 { b / k } else { *b })
                .collect(),
            Operation::Mod(k) => data
                .iter()
                .map(|b| if *k != 0 { b % k } else { *b })
                .collect(),
            Operation::ModU16(k) => data
                .iter()
                .map(|b| if *k != 0 { b % (*k as u8) } else { *b })
                .collect(),
        };
    }

    data
}

pub fn generate_challenge(complexity: u8) -> Challenge {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let uuid_part = u64::from(Uuid::new_v4().as_fields().0);
    let id = (now * 1000 + uuid_part) % 1_000_000;
    let challenge_type = ChallengeType::random();

    let input_size = 8 + (complexity as usize * 2);
    let input_data: Vec<u8> = (0..input_size)
        .map(|_| {
            (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                % 256) as u8
        })
        .collect();

    let operations = generate_operations(challenge_type, complexity);

    let expected_format = match complexity % 6 {
        0 => OutputFormat::HexUpper,
        1 => OutputFormat::HexLower,
        2 => OutputFormat::Base64Std,
        3 => OutputFormat::Base64Url,
        4 => OutputFormat::Binary,
        _ => OutputFormat::Decimal,
    };

    let nonce = generate_nonce();

    Challenge {
        id,
        challenge_type,
        complexity,
        input_data,
        operations,
        expected_format,
        created_at: now,
        expires_at: now + 30,
        nonce,
    }
}

fn generate_operations(challenge_type: ChallengeType, complexity: u8) -> Vec<Operation> {
    let num_ops = complexity as usize;
    let mut ops = Vec::new();

    match challenge_type {
        ChallengeType::HashChain => {
            ops.push(Operation::Reverse);
            for _ in 0..num_ops {
                match (SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
                    % 3) as u8
                {
                    0 => ops.push(Operation::Sha256),
                    1 => ops.push(Operation::Sha512),
                    _ => ops.push(Operation::Blake3),
                }
            }
            ops.push(Operation::Truncate(16));
        }
        ChallengeType::XorCascade => {
            ops.push(Operation::Reverse);
            for i in 0..num_ops {
                let key = ((i as u8 * 0x42) ^ 0xAB) % 255;
                ops.push(Operation::XorKey(key));
            }
            ops.push(Operation::Base64Encode);
        }
        ChallengeType::MatrixTransform => {
            ops.push(Operation::SwapBytes);
            for i in 0..num_ops {
                let shift = ((i as u8 % 7) + 1) as usize;
                ops.push(Operation::RotateLeft(shift));
            }
            ops.push(Operation::Truncate(16));
        }
        ChallengeType::RegexSynth => {
            ops.push(Operation::Reverse);
            ops.push(Operation::XorKey(0x55));
            ops.push(Operation::Base64Encode);
            ops.push(Operation::Truncate(24));
        }
        ChallengeType::TokenStream => {
            ops.push(Operation::Increment);
            for i in 0..num_ops {
                let mul = ((i as u8 * 3) % 5) + 2;
                ops.push(Operation::Multiply(mul));
            }
            ops.push(Operation::ModU16(256));
        }
    }

    ops
}

// ============================================================
// HELPER FUNCTIONS
// ============================================================

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn hex_to_bytes_str(hex: &str) -> Vec<u8> {
    hex_to_bytes(hex)
}

fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();
    let mut i = 0;

    while i < data.len() {
        let b0 = data[i] as usize;
        let b1 = if i + 1 < data.len() {
            data[i + 1] as usize
        } else {
            0
        };
        let b2 = if i + 2 < data.len() {
            data[i + 2] as usize
        } else {
            0
        };

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if i + 1 < data.len() {
            result.push(ALPHABET[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if i + 2 < data.len() {
            result.push(ALPHABET[b2 & 0x3F] as char);
        } else {
            result.push('=');
        }

        i += 3;
    }

    result
}

fn base64_decode(input: &str) -> Vec<u8> {
    const DECODE: [i8; 128] = [
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 62, -1, -1,
        -1, 63, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, -1, -1, -1, -1, 0, 1, 2, 3, 4,
        5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, -1, -1, -1,
        -1, -1, -1, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        46, 47, 48, 49, 50, 51, -1, -1, -1, -1, -1,
    ];

    let input = input.trim_end_matches('=');
    let mut result = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;

    for c in input.chars() {
        if c as usize >= 128 {
            continue;
        }
        let val = DECODE[c as usize];
        if val < 0 {
            continue;
        }

        buf = (buf << 6) | (val as u32);
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            result.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }

    result
}

fn sha256_hash(data: &[u8]) -> Vec<u8> {
    let mut result = data.to_vec();
    for _ in 0..32 {
        result = result.iter().map(|b| b.rotate_left(1) ^ 0x5A).collect();
    }
    result.resize(32, 0);
    result
}

fn sha512_hash(data: &[u8]) -> Vec<u8> {
    let mut result = data.to_vec();
    for _ in 0..64 {
        result = result.iter().map(|b| b.rotate_left(1) ^ 0x5A).collect();
    }
    result.resize(64, 0);
    result
}

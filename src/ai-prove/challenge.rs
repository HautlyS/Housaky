use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeType {
    HashChain = 0x01,
    XorCascade = 0x02,
    MatrixTransform = 0x03,
    RegexSynth = 0x04,
    TokenStream = 0x05,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    HexUpper = 0x01,
    HexLower = 0x02,
    Base64Std = 0x03,
    Base64Url = 0x04,
    Binary = 0x05,
    Decimal = 0x06,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub id: u64,
    pub challenge_type: ChallengeType,
    pub complexity: u8,
    pub input_data: Vec<u8>,
    pub input_hex: String,
    pub operations: Vec<String>,
    pub expected_format: OutputFormat,
    pub created_at: u64,
    pub expires_at: u64,
    pub nonce: Vec<u8>,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeResponse {
    pub challenge_id: u64,
    pub result: String,
    pub result_hex: String,
    pub compute_time_ms: u64,
    pub token_count: u32,
    pub checksum: String,
    pub timestamp: u64,
}

pub struct ChallengeGenerator {
    complexity_min: u8,
    complexity_max: u8,
}

impl ChallengeGenerator {
    pub fn new() -> Self {
        Self {
            complexity_min: 3,
            complexity_max: 7,
        }
    }

    pub fn generate(&self, complexity: Option<u8>) -> Challenge {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let comp = complexity.unwrap_or_else(|| {
            use rand::Rng;
            rand::thread_rng().gen_range(self.complexity_min..=self.complexity_max)
        });

        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let type_id = rng.gen_range(1..=5) as u8;
        let challenge_type = match type_id {
            1 => ChallengeType::HashChain,
            2 => ChallengeType::XorCascade,
            3 => ChallengeType::MatrixTransform,
            4 => ChallengeType::RegexSynth,
            _ => ChallengeType::TokenStream,
        };

        let input_size = 8 + (comp as usize) * 2;
        let input_data: Vec<u8> = (0..input_size).map(|_| rng.gen()).collect();
        let input_hex = hex::encode(&input_data);

        let operations = self.generate_operations(challenge_type, comp);
        let format_num = ((comp - 1) % 6) as u8 + 1;
        let expected_format = match format_num {
            1 => OutputFormat::HexUpper,
            2 => OutputFormat::HexLower,
            3 => OutputFormat::Base64Std,
            4 => OutputFormat::Base64Url,
            5 => OutputFormat::Binary,
            _ => OutputFormat::Decimal,
        };

        let nonce: Vec<u8> = (0..16).map(|_| rng.gen()).collect();

        Challenge {
            id: (now * 1000 + rng.gen_range(0..1000)) % 1_000_000,
            challenge_type,
            complexity: comp,
            input_data,
            input_hex,
            operations,
            expected_format,
            created_at: now,
            expires_at: now + 30,
            nonce,
            timeout_ms: 30_000,
        }
    }

    pub fn execute(&self, input: &[u8], operations: &[String]) -> Vec<u8> {
        let mut data = input.to_vec();

        for op in operations {
            if op == "REVERSE" {
                data.reverse();
            } else if let Some(key_str) = op.strip_prefix("XOR_KEY:") {
                if let Ok(key) = key_str.parse::<u8>() {
                    data = data.iter().map(|b| b ^ key).collect();
                }
            } else if op == "BASE64_ENCODE" {
                let encoded = base64_encode(&data);
                data = encoded.into_bytes();
            } else if op == "SHA256" || op == "BLAKE3" {
                for i in 0..32 {
                    data = data.iter()
                        .enumerate()
                        .map(|(idx, b)| (b << 1 | b >> 7) ^ ((idx * 0x5A) as u8 & 0xFF))
                        .collect();
                }
                data.truncate(32);
            } else if let Some(n_str) = op.strip_prefix("TRUNCATE:") {
                if let Ok(n) = n_str.parse::<usize>() {
                    data.truncate(n);
                }
            } else if let Some(n_str) = op.strip_prefix("ROTATE_LEFT:") {
                if let Ok(n) = n_str.parse::<u8>() {
                    let shift = n % 8;
                    data = data.iter().map(|b| ((b << shift) | (b >> (8 - shift))) & 0xFF).collect();
                }
            } else if op == "SWAP_BYTES" {
                for i in (0..data.len() - 1).step_by(2) {
                    data.swap(i, i + 1);
                }
            } else if op == "INCREMENT" {
                data = data.iter().map(|b| (b + 1) & 0xFF).collect();
            } else if let Some(mul_str) = op.strip_prefix("MULTIPLY:") {
                if let Ok(mul) = mul_str.parse::<u8>() {
                    data = data.iter().map(|b| (b.wrapping_mul(mul)) & 0xFF).collect();
                }
            } else if let Some(mod_str) = op.strip_prefix("MOD:") {
                if let Ok(mod_val) = mod_str.parse::<u8>() {
                    if mod_val > 0 {
                        data = data.iter().map(|b| b % mod_val).collect();
                    }
                }
            }
        }

        data
    }

    pub fn format_result(&self, data: &[u8], format: OutputFormat) -> String {
        match format {
            OutputFormat::HexUpper => hex::encode(data).to_uppercase(),
            OutputFormat::HexLower => hex::encode(data),
            OutputFormat::Base64Std => base64_encode(data),
            OutputFormat::Base64Url => base64_url_encode(data),
            OutputFormat::Binary => data.iter().map(|b| format!("{:08b}", b)).collect::<Vec<_>>().join(""),
            OutputFormat::Decimal => data.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(","),
        }
    }

    pub fn validate(&self, challenge: &Challenge, response: &ChallengeResponse) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if now > challenge.expires_at {
            return false;
        }

        let expected = self.execute(&challenge.input_data, &challenge.operations);
        let expected_formatted = self.format_result(&expected, challenge.expected_format);

        expected_formatted.to_lowercase() == response.result.to_lowercase()
    }

    fn generate_operations(&self, challenge_type: ChallengeType, complexity: u8) -> Vec<String> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let num_ops = complexity as usize;
        let mut ops = Vec::new();

        match challenge_type {
            ChallengeType::HashChain => {
                ops.push("REVERSE".to_string());
                for i in 0..num_ops {
                    let hash_op = match i % 3 {
                        0 => "SHA256",
                        1 => "SHA512",
                        _ => "BLAKE3",
                    };
                    ops.push(hash_op.to_string());
                }
                ops.push("TRUNCATE:16".to_string());
            }
            ChallengeType::XorCascade => {
                ops.push("REVERSE".to_string());
                for i in 0..num_ops {
                    let key = ((i * 0x42) ^ 0xAB) % 256;
                    ops.push(format!("XOR_KEY:{}", key));
                }
                ops.push("BASE64_ENCODE".to_string());
            }
            ChallengeType::MatrixTransform => {
                ops.push("SWAP_BYTES".to_string());
                for i in 0..num_ops {
                    let shift = (i % 7) + 1;
                    ops.push(format!("ROTATE_LEFT:{}", shift));
                }
                ops.push("TRUNCATE:16".to_string());
            }
            ChallengeType::RegexSynth => {
                ops.push("REVERSE".to_string());
                ops.push("XOR_KEY:85".to_string());
                ops.push("BASE64_ENCODE".to_string());
                ops.push("TRUNCATE:24".to_string());
            }
            ChallengeType::TokenStream => {
                ops.push("INCREMENT".to_string());
                for i in 0..num_ops {
                    let mul = ((i * 3) % 5) + 2;
                    ops.push(format!("MULTIPLY:{}", mul));
                }
                ops.push("MOD:256".to_string());
            }
        }

        ops
    }
}

impl Default for ChallengeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

fn base64_encode(data: &[u8]) -> String {
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

fn base64_url_encode(data: &[u8]) -> String {
    base64_encode(data).replace('+', "-").replace('/', "_").replace('=', "")
}

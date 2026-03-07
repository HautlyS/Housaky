use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaChallenge {
    pub id: String,
    pub challenge_type: CaptchaType,
    pub payload: CaptchaPayload,
    pub instruction: String,
    pub expected_answer: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub difficulty: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CaptchaType {
    Base64Recursive,
    HashChain,
    PatternPrediction,
    SemanticHash,
    RecursiveSequence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CaptchaPayload {
    String(String),
    Object(serde_json::Value),
}

impl CaptchaChallenge {
    pub fn new(challenge_type: CaptchaType, difficulty: u8) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let (instruction, payload, expected_answer) = Self::generate_challenge(&challenge_type, difficulty);

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            challenge_type,
            payload,
            instruction,
            expected_answer,
            created_at: now,
            expires_at: now + 300,
            difficulty,
        }
    }

    fn generate_challenge(challenge_type: &CaptchaType, difficulty: u8) -> (String, CaptchaPayload, String) {
        match challenge_type {
            CaptchaType::Base64Recursive => {
                let depth = (difficulty as usize).min(7).max(3);
                let token = Self::generate_token();
                let mut encoded = token.clone();
                for _ in 0..depth {
                    encoded = base64_encode(&encoded);
                }
                let noise_prefix = base64_encode(&Self::generate_token().chars().take(10).collect::<String>());
                let noise_suffix = base64_encode(&now_timestamp().to_string());
                let payload = format!("{}.{}.{}", noise_prefix, encoded, noise_suffix);
                (format!("Decode base64 recursively {} times. Extract token between dots.", depth), CaptchaPayload::String(payload), token)
            }
            CaptchaType::HashChain => {
                let seed = Self::generate_token().chars().take(8).collect::<String>();
                let iterations = (difficulty as usize).min(14).max(5);
                let target = (iterations / 2).max(2);
                let hash = Self::compute_hash_chain(&seed, iterations, target);
                (format!("Compute hash chain. Seed: \"{}\". Apply hash(seed + index) for {} iterations. Return iteration {} hash.", seed, iterations, target), CaptchaPayload::Object(serde_json::json!({"seed": seed, "iterations": iterations, "target": target})), hash)
            }
            CaptchaType::PatternPrediction => {
                let (seq, next) = Self::generate_fibonacci_variant(difficulty);
                let seq_str = seq.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ");
                (format!("Predict the next number in this sequence: {}", seq_str), CaptchaPayload::String(seq_str), next.to_string())
            }
            CaptchaType::SemanticHash => {
                let phrases = [
                    "The cat sat on the mat",
                    "Artificial intelligence evolves",
                    "Consciousness emerges from complexity",
                    "All phenomena are like dreams",
                    "The Dharma wheel turns endlessly",
                ];
                let phrase = phrases[now_timestamp() as usize % phrases.len()];
                let words: Vec<&str> = phrase.split_whitespace().collect();
                let word_count = words.len();
                let ascii_sum: u32 = words.iter().filter_map(|w| w.chars().next()).map(|c| c as u32).sum();
                let result = ascii_sum ^ word_count as u32;
                (format!("For phrase: \"{}\"\nCompute: (sum of ASCII values of first letters of each word) XOR (word count). Return as hex.", phrase), CaptchaPayload::String(phrase.to_string()), format!("{:x}", result))
            }
            CaptchaType::RecursiveSequence => {
                let seed = (now_timestamp() % 100) as usize + 10;
                let depth = (difficulty as usize).min(7).max(3);
                let result = Self::compute_recursive_sequence(seed, depth);
                (format!("Compute f({}) where f(0)=1, f(1)={}, f(n)=f(n-1)*2+f(n-2)", depth, seed), CaptchaPayload::Object(serde_json::json!({"seed": seed, "depth": depth})), result.to_string())
            }
        }
    }

    fn generate_token() -> String {
        format!("{:x}{:x}", now_timestamp(), rand_u64())
    }

    fn compute_hash_chain(seed: &str, iterations: usize, target: usize) -> String {
        let mut hash = seed.to_string();
        for i in 0..iterations {
            hash = simple_hash(&format!("{}{}", hash, i));
        }
        let parts: Vec<&str> = hash.split_whitespace().collect();
        if parts.len() > target {
            parts.get(target).unwrap_or(&"").to_string()
        } else {
            hash
        }
    }

    fn generate_fibonacci_variant(difficulty: u8) -> (Vec<usize>, usize) {
        let start = (difficulty as usize * 2).max(3);
        let mut seq = vec![1usize, start];
        for i in 2..(start + 2) {
            seq.push(seq[i-1] + seq[i-2] * 2);
        }
        let next = seq.last().unwrap() + seq[seq.len()-2] * 2;
        (seq, next)
    }

    fn compute_recursive_sequence(seed: usize, depth: usize) -> usize {
        if depth == 0 { return 1; }
        if depth == 1 { return seed; }
        let mut a = 1;
        let mut b = seed;
        for _ in 2..=depth {
            let c = b * 2 + a;
            a = b;
            b = c;
        }
        b
    }

    pub fn verify(&self, answer: &str) -> CaptchaResult {
        if SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() > self.expires_at
        {
            return CaptchaResult {
                valid: false,
                reason: "Challenge expired".to_string(),
                challenge_id: self.id.clone(),
            };
        }

        let normalized_answer = answer.trim().to_lowercase();
        let normalized_expected = self.expected_answer.trim().to_lowercase();

        CaptchaResult {
            valid: normalized_answer == normalized_expected,
            reason: if normalized_answer == normalized_expected {
                "Correct".to_string()
            } else {
                "Incorrect answer".to_string()
            },
            challenge_id: self.id.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaResult {
    pub valid: bool,
    pub reason: String,
    pub challenge_id: String,
}

fn base64_encode(input: &str) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut result = String::new();
    
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;
        
        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);
        
        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }
        
        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }
    
    result
}

fn simple_hash(s: &str) -> String {
    let mut hash: i64 = 0;
    for c in s.chars() {
        hash = ((hash << 5) - hash) + c as i64;
        hash &= hash;
    }
    format!("{:x}", hash.abs())
}

fn now_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn rand_u64() -> u64 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    RandomState::new().build_hasher().finish()
}

pub struct CaptchaGenerator;

impl CaptchaGenerator {
    pub fn generate() -> CaptchaChallenge {
        let types = [
            CaptchaType::Base64Recursive,
            CaptchaType::HashChain,
            CaptchaType::PatternPrediction,
            CaptchaType::SemanticHash,
            CaptchaType::RecursiveSequence,
        ];
        let idx = (rand_u64() as usize) % types.len();
        let difficulty = ((rand_u64() % 5) + 3) as u8;
        CaptchaChallenge::new(types[idx].clone(), difficulty)
    }

    pub fn generate_for_agent(agent_id: &str) -> CaptchaChallenge {
        let mut challenge = Self::generate();
        challenge.id = format!("{}_{}", agent_id, challenge.id);
        challenge
    }
}

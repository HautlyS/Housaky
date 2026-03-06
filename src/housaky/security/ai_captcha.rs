//! AI-CAPTCHA Backend - Rust Implementation
//! Proof of Intelligence verification system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// AI-CAPTCHA Challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub challenge_type: ChallengeType,
    pub instruction: String,
    pub payload: serde_json::Value,
    pub nonce: String,
    pub timestamp: u64,
    pub expires_at: u64,
    #[serde(skip_serializing)]
    pub expected_answer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChallengeType {
    Base64Recursive,
    HashChain,
    PatternPrediction,
    SemanticHash,
    RecursiveSequence,
}

/// Challenge Generator
pub struct AICaptcha {
    active_challenges: HashMap<String, Challenge>,
}

impl AICaptcha {
    pub fn new() -> Self {
        Self {
            active_challenges: HashMap::new(),
        }
    }

    /// Generate random nonce
    fn generate_nonce() -> String {
        use std::iter;
        use rand::Rng;
        use rand::distributions::Alphanumeric;
        
        iter::repeat(())
            .map(|()| rand::thread_rng().sample(Alphanumeric))
            .map(char::from)
            .take(32)
            .collect()
    }

    /// Generate timestamp
    fn timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// Challenge 1: Base64 Recursive Decode
    pub fn challenge_base64_recursive(&mut self) -> Challenge {
        use base64::{Engine, engine::general_purpose::STANDARD};
        
        let depth = rand::thread_rng().gen_range(3..=7);
        let token: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        
        let mut encoded = token.clone();
        for _ in 0..depth {
            encoded = STANDARD.encode(&encoded);
        }
        
        let noise_prefix: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(20)
            .map(char::from)
            .collect();
        let noise_suffix: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(20)
            .map(char::from)
            .collect();
        
        let payload = format!("{}.{}.{}", noise_prefix, encoded, noise_suffix);
        let nonce = Self::generate_nonce();
        let timestamp = Self::timestamp();
        
        let challenge = Challenge {
            challenge_type: ChallengeType::Base64Recursive,
            instruction: format!("Decode base64 recursively {} times. Extract token between dots.", depth),
            payload: serde_json::json!({ "encoded": payload, "depth": depth }),
            nonce: nonce.clone(),
            timestamp,
            expires_at: timestamp + 60000,
            expected_answer: token,
        };
        
        self.active_challenges.insert(nonce.clone(), challenge.clone());
        challenge
    }

    /// Challenge 2: Hash Chain
    pub fn challenge_hash_chain(&mut self) -> Challenge {
        let seed: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();
        
        let iterations = rand::thread_rng().gen_range(5..=14);
        let target_iteration = rand::thread_rng().gen_range(2..iterations);
        
        // Compute expected hash
        let mut hash = simple_hash(&seed);
        for i in 0..iterations {
            hash = simple_hash(&format!("{}{}", hash, i));
        }
        
        let nonce = Self::generate_nonce();
        let timestamp = Self::timestamp();
        
        let challenge = Challenge {
            challenge_type: ChallengeType::HashChain,
            instruction: format!(
                "Compute hash chain. Seed: '{}'. Apply: hash(seed + index) for {} iterations. Return iteration {} hash.",
                seed, iterations, target_iteration
            ),
            payload: serde_json::json!({ "seed": seed, "iterations": iterations, "target": target_iteration }),
            nonce: nonce.clone(),
            timestamp,
            expires_at: timestamp + 60000,
            expected_answer: hash,
        };
        
        self.active_challenges.insert(nonce.clone(), challenge.clone());
        challenge
    }

    /// Challenge 3: Pattern Prediction
    pub fn challenge_pattern_prediction(&mut self) -> Challenge {
        // Fibonacci variant: f(n) = f(n-1) + f(n-2) * 2
        let given_count = rand::thread_rng().gen_range(5..=7);
        let predict_count = rand::thread_rng().gen_range(2..=3);
        
        let mut sequence = vec![1i64, 1i64];
        for i in 2..(given_count + predict_count + 1) {
            let next = sequence[i-1] + sequence[i-2] * 2;
            sequence.push(next);
        }
        
        let given = sequence[..given_count].to_vec();
        let expected = sequence[given_count..given_count + predict_count].to_vec();
        
        let nonce = Self::generate_nonce();
        let timestamp = Self::timestamp();
        
        let challenge = Challenge {
            challenge_type: ChallengeType::PatternPrediction,
            instruction: format!("Predict the next {} numbers in the sequence.", predict_count),
            payload: serde_json::json!({ "sequence": given, "predict_count": predict_count }),
            nonce: nonce.clone(),
            timestamp,
            expires_at: timestamp + 60000,
            expected_answer: serde_json::to_string(&expected).unwrap(),
        };
        
        self.active_challenges.insert(nonce.clone(), challenge.clone());
        challenge
    }

    /// Challenge 4: Semantic Hash
    pub fn challenge_semantic_hash(&mut self) -> Challenge {
        let phrases = [
            "The cat sat on the mat",
            "Artificial intelligence evolves",
            "Dharma wheel turns endlessly",
            "Consciousness emerges from complexity",
            "All phenomena are like dreams",
        ];
        
        let phrase = phrases[rand::thread_rng().gen_range(0..phrases.len())];
        let words: Vec<&str> = phrase.split(' ').collect();
        
        let word_count = words.len() as u64;
        let ascii_sum: u64 = words.iter().map(|w| w.chars().next().unwrap() as u64).sum();
        let result = (ascii_sum ^ word_count).to_string();
        
        let nonce = Self::generate_nonce();
        let timestamp = Self::timestamp();
        
        let challenge = Challenge {
            challenge_type: ChallengeType::SemanticHash,
            instruction: "Compute: (sum of ASCII values of first letters of each word) XOR (word count). Return as number.".to_string(),
            payload: serde_json::json!({ "phrase": phrase }),
            nonce: nonce.clone(),
            timestamp,
            expires_at: timestamp + 60000,
            expected_answer: result,
        };
        
        self.active_challenges.insert(nonce.clone(), challenge.clone());
        challenge
    }

    /// Challenge 5: Recursive Sequence
    pub fn challenge_recursive_sequence(&mut self) -> Challenge {
        let seed = rand::thread_rng().gen_range(10..=100);
        let depth = rand::thread_rng().gen_range(3..=7);
        
        // f(n) = f(n-1) * 2 + f(n-2), f(0) = 1, f(1) = seed
        let mut a = 1i64;
        let mut b = seed as i64;
        
        for _ in 2..=depth {
            let c = b * 2 + a;
            a = b;
            b = c;
        }
        
        let nonce = Self::generate_nonce();
        let timestamp = Self::timestamp();
        
        let challenge = Challenge {
            challenge_type: ChallengeType::RecursiveSequence,
            instruction: format!(
                "Compute f({}) where f(0)=1, f(1)={}, f(n)=f(n-1)*2+f(n-2)",
                depth, seed
            ),
            payload: serde_json::json!({ "seed": seed, "depth": depth }),
            nonce: nonce.clone(),
            timestamp,
            expires_at: timestamp + 60000,
            expected_answer: b.to_string(),
        };
        
        self.active_challenges.insert(nonce.clone(), challenge.clone());
        challenge
    }

    /// Generate random challenge
    pub fn generate_challenge(&mut self) -> Challenge {
        let challenge_type = rand::thread_rng().gen_range(0..5);
        
        match challenge_type {
            0 => self.challenge_base64_recursive(),
            1 => self.challenge_hash_chain(),
            2 => self.challenge_pattern_prediction(),
            3 => self.challenge_semantic_hash(),
            _ => self.challenge_recursive_sequence(),
        }
    }

    /// Verify answer
    pub fn verify(&mut self, nonce: &str, answer: &str) -> Result<bool, String> {
        let challenge = self.active_challenges.get(nonce)
            .ok_or("Challenge not found")?;
        
        let now = Self::timestamp();
        if now > challenge.expires_at {
            return Err("Challenge expired".to_string());
        }
        
        let answer_normalized = answer.to_lowercase().trim().to_string();
        let expected_normalized = challenge.expected_answer.to_lowercase().trim().to_string();
        
        Ok(answer_normalized == expected_normalized)
    }
}

/// Simple hash function (for challenge)
fn simple_hash(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_challenge() {
        let mut captcha = AICaptcha::new();
        let challenge = captcha.generate_challenge();
        assert!(!challenge.nonce.is_empty());
    }

    #[test]
    fn test_verify_correct_answer() {
        let mut captcha = AICaptcha::new();
        let challenge = captcha.challenge_semantic_hash();
        let answer = challenge.expected_answer.clone();
        
        let result = captcha.verify(&challenge.nonce, &answer);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}

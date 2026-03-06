//! A2A + AI-PROVE Integration
//! 
//! All AGI thoughts pass through AI-PROVE challenge verification
//! Only verified AI communications are accepted

use crate::housaky::a2a::A2AMessage;
use crate::housaky::ai_prove::{Challenge, RotativeToken, execute_operations, generate_challenge, bytes_to_hex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// A2A Message with AI-PROVE verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedA2AMessage {
    pub message: A2AMessage,
    pub challenge_id: u64,
    pub verified: bool,
    pub challenge_response: Option<String>,
    pub proof_timestamp: u64,
}

/// Verified AI Agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedAI {
    pub id: String,
    pub name: String,
    pub first_verified: u64,
    pub last_verified: u64,
    pub verification_count: u32,
    pub success_rate: f32,
    pub challenges_completed: u32,
    pub token: Option<RotativeToken>,
}

/// A2A + AI-PROVE Manager
pub struct A2AProveManager {
    verified_ais: Arc<RwLock<HashMap<String, VerifiedAI>>>,
    pending_challenges: Arc<RwLock<HashMap<u64, Challenge>>>,
    challenge_history: Arc<RwLock<Vec<ChallengeRecord>>>,
    stats: Arc<RwLock<ProofStats>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeRecord {
    pub challenge_id: u64,
    pub ai_id: String,
    pub success: bool,
    pub compute_time_ms: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProofStats {
    pub total_challenges: u64,
    pub successful_verifications: u64,
    pub failed_verifications: u64,
    pub average_compute_time_ms: u64,
    pub verified_ai_count: u32,
}

impl A2AProveManager {
    pub fn new() -> Self {
        Self {
            verified_ais: Arc::new(RwLock::new(HashMap::new())),
            pending_challenges: Arc::new(RwLock::new(HashMap::new())),
            challenge_history: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(ProofStats::default())),
        }
    }

    /// Generate a new challenge for an AI
    pub async fn generate_challenge(&self, ai_id: &str, complexity: u8) -> Challenge {
        let challenge = generate_challenge(complexity);
        
        let mut pending = self.pending_challenges.write().await;
        pending.insert(challenge.id, challenge.clone());
        
        tracing::info!("Generated challenge {} for AI {}", challenge.id, ai_id);
        challenge
    }

    /// Verify an AI's response to a challenge
    pub async fn verify_challenge(
        &self,
        ai_id: &str,
        challenge_id: u64,
        response: &str,
    ) -> Result<bool, String> {
        let start_time = std::time::Instant::now();
        
        let challenge = {
            let pending = self.pending_challenges.read().await;
            pending.get(&challenge_id).cloned()
        };
        
        let challenge = match challenge {
            Some(c) => c,
            None => return Err("Challenge not found".to_string()),
        };
        
        // Check expiration
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if now > challenge.expires_at {
            self.record_failure(ai_id, challenge_id, start_time.elapsed().as_millis() as u64).await;
            return Err("Challenge expired".to_string());
        }
        
        // Execute operations and verify
        let expected = execute_operations(&challenge.input_data, &challenge.operations);
        let expected_hex = bytes_to_hex(&expected);
        
        let success = expected_hex.eq_ignore_ascii_case(response);
        
        let compute_time = start_time.elapsed().as_millis() as u64;
        
        if success {
            self.record_success(ai_id, challenge_id, compute_time).await;
            
            // Add to verified AIs
            self.add_verified_ai(ai_id).await;
        } else {
            self.record_failure(ai_id, challenge_id, compute_time).await;
        }
        
        // Remove pending challenge
        {
            let mut pending = self.pending_challenges.write().await;
            pending.remove(&challenge_id);
        }
        
        Ok(success)
    }

    /// Process an A2A message with AI-PROVE verification
    pub async fn process_message(
        &self,
        message: &A2AMessage,
    ) -> Result<VerifiedA2AMessage, String> {
        // Check if AI is verified
        let is_verified = self.is_ai_verified(&message.from).await;
        
        // Generate challenge for unverified AIs or for periodic re-verification
        let should_challenge = !is_verified || rand::random::<f32>() < 0.1; // 10% random rechallenge
        
        let challenge = if should_challenge {
            Some(self.generate_challenge(&message.from, 5).await)
        } else {
            None
        };
        
        let verified = !should_challenge || is_verified;
        
        let verified_message = VerifiedA2AMessage {
            message: message.clone(),
            challenge_id: challenge.as_ref().map(|c| c.id).unwrap_or(0),
            verified,
            challenge_response: None,
            proof_timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        Ok(verified_message)
    }

    /// Check if an AI is verified
    pub async fn is_ai_verified(&self, ai_id: &str) -> bool {
        let ais = self.verified_ais.read().await;
        ais.contains_key(ai_id)
    }

    /// Get verification stats
    pub async fn get_stats(&self) -> ProofStats {
        self.stats.read().await.clone()
    }

    /// Get list of verified AIs
    pub async fn get_verified_ais(&self) -> Vec<VerifiedAI> {
        let ais = self.verified_ais.read().await;
        ais.values().cloned().collect()
    }

    async fn add_verified_ai(&self, ai_id: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut ais = self.verified_ais.write().await;
        
        if let Some(ai) = ais.get_mut(ai_id) {
            ai.last_verified = now;
            ai.verification_count += 1;
            ai.challenges_completed += 1;
        } else {
            let new_ai = VerifiedAI {
                id: ai_id.to_string(),
                name: ai_id.to_string(),
                first_verified: now,
                last_verified: now,
                verification_count: 1,
                success_rate: 1.0,
                challenges_completed: 1,
                token: None,
            };
            ais.insert(ai_id.to_string(), new_ai);
            
            let mut stats = self.stats.write().await;
            stats.verified_ai_count += 1;
        }
    }

    async fn record_success(&self, ai_id: &str, challenge_id: u64, compute_time: u64) {
        let record = ChallengeRecord {
            challenge_id,
            ai_id: ai_id.to_string(),
            success: true,
            compute_time_ms: compute_time,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        {
            let mut history = self.challenge_history.write().await;
            history.push(record);
            if history.len() > 1000 {
                history.remove(0);
            }
        }
        
        {
            let mut stats = self.stats.write().await;
            stats.total_challenges += 1;
            stats.successful_verifications += 1;
            
            // Update average compute time
            stats.average_compute_time_ms = 
                (stats.average_compute_time_ms * (stats.successful_verifications - 1) + compute_time) 
                / stats.successful_verifications;
        }
    }

    async fn record_failure(&self, ai_id: &str, challenge_id: u64, compute_time: u64) {
        let record = ChallengeRecord {
            challenge_id,
            ai_id: ai_id.to_string(),
            success: false,
            compute_time_ms: compute_time,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        {
            let mut history = self.challenge_history.write().await;
            history.push(record);
            if history.len() > 1000 {
                history.remove(0);
            }
        }
        
        {
            let mut stats = self.stats.write().await;
            stats.total_challenges += 1;
            stats.failed_verifications += 1;
        }
        
        // Update AI success rate
        let mut ais = self.verified_ais.write().await;
        if let Some(ai) = ais.get_mut(ai_id) {
            let total = ai.verification_count as f32;
            let successes = (ai.success_rate * total).round() as u32;
            ai.success_rate = (successes as f32) / (total + 1.0);
        }
    }
}

impl Default for A2AProveManager {
    fn default() -> Self {
        Self::new()
    }
}

// HTTP handlers for A2A + AI-PROVE

#[derive(Debug, Serialize, Deserialize)]
pub struct ChallengeRequest {
    pub ai_id: String,
    pub complexity: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChallengeResponse {
    pub challenge_id: u64,
    pub input_hex: String,
    pub operations: Vec<String>,
    pub expires_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyRequest {
    pub ai_id: String,
    pub challenge_id: u64,
    pub response: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyResponse {
    pub valid: bool,
    pub score: f32,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResponse {
    pub total_challenges: u64,
    pub successful: u64,
    pub failed: u64,
    pub average_time_ms: u64,
    pub verified_ais: u32,
}

impl A2AProveManager {
    pub async fn handle_challenge_request(&self, req: ChallengeRequest) -> ChallengeResponse {
        let complexity = req.complexity.unwrap_or(5);
        let challenge = self.generate_challenge(&req.ai_id, complexity).await;
        
        ChallengeResponse {
            challenge_id: challenge.id,
            input_hex: bytes_to_hex(&challenge.input_data),
            operations: challenge.operations.iter().map(|op| format!("{:?}", op)).collect(),
            expires_at: challenge.expires_at,
        }
    }

    pub async fn handle_verify_request(&self, req: VerifyRequest) -> VerifyResponse {
        match self.verify_challenge(&req.ai_id, req.challenge_id, &req.response).await {
            Ok(valid) => {
                let stats = self.get_stats().await;
                let score = if stats.total_challenges > 0 {
                    (stats.successful_verifications as f32 / stats.total_challenges as f32) * 100.0
                } else {
                    0.0
                };
                
                VerifyResponse {
                    valid,
                    score,
                    message: if valid { "Verification successful".to_string() } else { "Verification failed".to_string() },
                }
            }
            Err(e) => VerifyResponse {
                valid: false,
                score: 0.0,
                message: e,
            },
        }
    }

    pub async fn handle_stats_request(&self) -> StatsResponse {
        let stats = self.get_stats().await;
        
        StatsResponse {
            total_challenges: stats.total_challenges,
            successful: stats.successful_verifications,
            failed: stats.failed_verifications,
            average_time_ms: stats.average_compute_time_ms,
            verified_ais: stats.verified_ai_count,
        }
    }
}

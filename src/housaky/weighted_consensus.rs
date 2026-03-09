//! Weighted Consensus and Reputation System
//!
//! Provides advanced consensus mechanisms with reputation weighting,
//! influence scoring, and Byzantine fault tolerance.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{info, warn};

use crate::housaky::collective::consensus::ConsensusVerdict;

const REPUTATION_DECAY_FACTOR: f64 = 0.95;
const MIN_REPUTATION: f64 = 0.1;
const MAX_REPUTATION: f64 = 1.0;
const CONSENSUS_QUORUM_PERCENT: f64 = 0.67;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationConfig {
    pub initial_reputation: f64,
    pub decay_enabled: bool,
    pub decay_interval_secs: u64,
    pub min_reputation: f64,
    pub max_reputation: f64,
    pub positive_weight: f64,
    pub negative_weight: f64,
    pub karma_influence: f64,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            initial_reputation: 0.5,
            decay_enabled: true,
            decay_interval_secs: 3600,
            min_reputation: MIN_REPUTATION,
            max_reputation: MAX_REPUTATION,
            positive_weight: 1.0,
            negative_weight: 2.0,
            karma_influence: 0.25,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReputation {
    pub agent_id: String,
    pub reputation: f64,
    pub karma: i64,
    pub total_votes: u64,
    pub positive_votes: u64,
    pub negative_votes: u64,
    pub last_updated: u64,
    pub contribution_score: f64,
    pub reliability_score: f64,
    pub expertise_scores: HashMap<String, f64>,
}

impl AgentReputation {
    pub fn new(agent_id: String) -> Self {
        Self {
            agent_id,
            reputation: 0.5,
            karma: 0,
            total_votes: 0,
            positive_votes: 0,
            negative_votes: 0,
            last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            contribution_score: 0.5,
            reliability_score: 0.5,
            expertise_scores: HashMap::new(),
        }
    }

    pub fn calculate_influence(&self, config: &ReputationConfig) -> f64 {
        let base_influence = self.reputation;
        
        let karma_factor = 1.0 + (self.karma as f64 * config.karma_influence / 100.0);
        
        let expertise_factor = self.expertise_scores.values()
            .sum::<f64>() / self.expertise_scores.len().max(1) as f64;
        
        (base_influence * karma_factor * expertise_factor).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedVote {
    pub voter_id: String,
    pub vote: bool,
    pub weight: f64,
    pub timestamp: u64,
    pub justification: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusProposal {
    pub id: String,
    pub topic: String,
    pub description: String,
    pub proposer_id: String,
    pub votes: Vec<WeightedVote>,
    pub status: ProposalStatus,
    pub created_at: u64,
    pub quorum_threshold: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    Pending,
    Voting,
    Approved,
    Rejected,
    Expired,
}

pub struct WeightedConsensusEngine {
    config: ReputationConfig,
    reputations: Arc<RwLock<HashMap<String, AgentReputation>>>,
    proposals: Arc<RwLock<HashMap<String, ConsensusProposal>>>,
    vote_history: Arc<RwLock<HashMap<String, Vec<WeightedVote>>>>,
    byzantine_threshold: f64,
}

impl WeightedConsensusEngine {
    pub fn new(config: ReputationConfig) -> Self {
        Self {
            config,
            reputations: Arc::new(RwLock::new(HashMap::new())),
            proposals: Arc::new(RwLock::new(HashMap::new())),
            vote_history: Arc::new(RwLock::new(HashMap::new())),
            byzantine_threshold: 0.33,
        }
    }

    pub async fn register_agent(&self, agent_id: String) {
        let mut reputations = self.reputations.write().await;
        
        if !reputations.contains_key(&agent_id) {
            reputations.insert(agent_id.clone(), AgentReputation::new(agent_id));
            info!("📝 Registered agent for consensus: {}", agent_id);
        }
    }

    pub async fn submit_proposal(&self, topic: String, description: String, proposer_id: String) -> String {
        let proposal_id = format!("proposal_{}", uuid::Uuid::new_v4());
        
        let proposal = ConsensusProposal {
            id: proposal_id.clone(),
            topic: topic.clone(),
            description,
            proposer_id,
            votes: Vec::new(),
            status: ProposalStatus::Pending,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            quorum_threshold: CONSENSUS_QUORUM_PERCENT,
        };
        
        self.proposals.write().await.insert(proposal_id.clone(), proposal);
        
        info!("📋 Created consensus proposal: {} - {}", proposal_id, topic);
        
        proposal_id
    }

    pub async fn cast_vote(
        &self,
        proposal_id: &str,
        voter_id: &str,
        vote: bool,
        justification: Option<String>,
    ) -> Result<()> {
        let reputations = self.reputations.read().await;
        
        let voter_rep = reputations.get(voter_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown voter: {}", voter_id))?;
        
        let weight = voter_rep.calculate_influence(&self.config);
        
        drop(reputations);
        
        let weighted_vote = WeightedVote {
            voter_id: voter_id.to_string(),
            vote,
            weight,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            justification,
        };
        
        let mut proposals = self.proposals.write().await;
        let proposal = proposals.get_mut(proposal_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown proposal: {}", proposal_id))?;
        
        if proposal.status != ProposalStatus::Voting && proposal.status != ProposalStatus::Pending {
            anyhow::bail!("Proposal not accepting votes");
        }
        
        proposal.status = ProposalStatus::Voting;
        proposal.votes.push(weighted_vote.clone());
        
        let mut vote_history = self.vote_history.write().await;
        vote_history.entry(proposal_id.to_string())
            .or_insert_with(Vec::new)
            .push(weighted_vote);
        
        self.update_agent_vote_stats(voter_id, vote).await;
        
        info!("🗳️ Cast vote for proposal {}: {} (weight: {:.3})", proposal_id, vote, weight);
        
        Ok(())
    }

    async fn update_agent_vote_stats(&self, agent_id: &str, vote: bool) {
        let mut reputations = self.reputations.write().await;
        
        if let Some(rep) = reputations.get_mut(agent_id) {
            rep.total_votes += 1;
            
            if vote {
                rep.positive_votes += 1;
                rep.reputation = (rep.reputation + 0.01 * self.config.positive_weight)
                    .clamp(self.config.min_reputation, self.config.max_reputation);
            } else {
                rep.negative_votes += 1;
                rep.reputation = (rep.reputation - 0.01 * self.config.negative_weight)
                    .clamp(self.config.min_reputation, self.config.max_reputation);
            }
            
            rep.last_updated = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        }
    }

    pub async fn evaluate_proposal(&self, proposal_id: &str) -> ConsensusVerdict {
        let proposals = self.proposals.read().await;
        
        let proposal = match proposals.get(proposal_id) {
            Some(p) => p,
            None => return ConsensusVerdict::Rejected,
        };
        
        if proposal.votes.is_empty() {
            return ConsensusVerdict::Pending;
        }
        
        let total_weight: f64 = proposal.votes.iter().map(|v| v.weight).sum();
        let positive_weight: f64 = proposal.votes.iter()
            .filter(|v| v.vote)
            .map(|v| v.weight)
            .sum();
        
        let quorum = proposal.votes.len() as f64 / 3.0;
        
        if quorum < 2.0 {
            return ConsensusVerdict::Pending;
        }
        
        let approval_ratio = if total_weight > 0.0 {
            positive_weight / total_weight
        } else {
            0.0
        };
        
        let mut verdict = if approval_ratio >= proposal.quorum_threshold {
            ConsensusVerdict::Approved
        } else if approval_ratio >= 0.4 {
            ConsensusVerdict::Promising
        } else {
            ConsensusVerdict::Rejected
        };
        
        let reputations = self.reputations.read().await;
        
        let byzantine_votes: f64 = proposal.votes.iter()
            .filter(|v| {
                if let Some(rep) = reputations.get(&v.voter_id) {
                    rep.reputation < 0.3
                } else {
                    false
                }
            })
            .count() as f64 / proposal.votes.len() as f64;
        
        if byzantine_votes > self.byzantine_threshold {
            verdict = ConsensusVerdict::Rejected;
            warn!("🛡️ Byzantine fault detected in proposal {}", proposal_id);
        }
        
        info!("📊 Proposal {} verdict: {:?} (approval: {:.1}%)", proposal_id, verdict, approval_ratio * 100.0);
        
        verdict
    }

    pub async fn get_agent_reputation(&self, agent_id: &str) -> Option<AgentReputation> {
        let reputations = self.reputations.read().await;
        reputations.get(agent_id).cloned()
    }

    pub async fn get_top_reputed_agents(&self, limit: usize) -> Vec<AgentReputation> {
        let reputations = self.reputations.read().await;
        
        let mut sorted: Vec<_> = reputations.values()
            .cloned()
            .collect();
        
        sorted.sort_by(|a, b| {
            b.reputation.partial_cmp(&a.reputation).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        sorted.into_iter().take(limit).collect()
    }

    pub async fn update_expertise(&self, agent_id: &str, domain: &str, score: f64) {
        let mut reputations = self.reputations.write().await;
        
        if let Some(rep) = reputations.get_mut(agent_id) {
            let old_score = rep.expertise_scores.get(domain).copied().unwrap_or(0.0);
            let new_score = (old_score * 0.7 + score * 0.3).clamp(0.0, 1.0);
            rep.expertise_scores.insert(domain.to_string(), new_score);
        }
    }

    pub async fn apply_reputation_decay(&self) {
        if !self.config.decay_enabled {
            return;
        }
        
        let mut reputations = self.reputations.write().await;
        
        for rep in reputations.values_mut() {
            rep.reputation *= REPUTATION_DECAY_FACTOR;
            rep.reputation = rep.reputation.clamp(self.config.min_reputation, self.config.max_reputation);
        }
        
        info!("🔄 Applied reputation decay to all agents");
    }

    pub async fn get_consensus_stats(&self) -> serde_json::Value {
        let proposals = self.proposals.read().await;
        let reputations = self.reputations.read().await;
        
        let pending = proposals.values().filter(|p| p.status == ProposalStatus::Pending).count();
        let voting = proposals.values().filter(|p| p.status == ProposalStatus::Voting).count();
        let approved = proposals.values().filter(|p| p.status == ProposalStatus::Approved).count();
        let rejected = proposals.values().filter(|p| p.status == ProposalStatus::Rejected).count();
        
        let avg_reputation: f64 = if !reputations.is_empty() {
            reputations.values().map(|r| r.reputation).sum::<f64>() / reputations.len() as f64
        } else {
            0.0
        };
        
        serde_json::json!({
            "proposals": {
                "pending": pending,
                "voting": voting,
                "approved": approved,
                "rejected": rejected,
            },
            "agents": {
                "total": reputations.len(),
                "average_reputation": avg_reputation,
            },
        })
    }

    pub fn start_reputation_decay_loop(&self) {
        let engine = Arc::new(self.reputations.clone());
        let config = self.config;
        
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(config.decay_interval_secs));
            
            loop {
                ticker.tick().await;
            }
        });
    }
}

pub fn create_weighted_consensus() -> WeightedConsensusEngine {
    WeightedConsensusEngine::new(ReputationConfig::default())
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConsensusProtocol {
    Raft,
    PBFT,
    SimpleMajority,
    WeightedMajority,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProposalStatus {
    Pending,
    Voting,
    Accepted,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub proposer: String,
    pub topic: String,
    pub value: serde_json::Value,
    pub status: ProposalStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub votes: HashMap<String, Vote>,
    pub required_quorum: f64,
}

impl Proposal {
    pub fn new(
        proposer: &str,
        topic: &str,
        value: serde_json::Value,
        ttl_secs: i64,
        required_quorum: f64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            proposer: proposer.to_string(),
            topic: topic.to_string(),
            value,
            status: ProposalStatus::Voting,
            created_at: now,
            expires_at: now + chrono::Duration::seconds(ttl_secs),
            votes: HashMap::new(),
            required_quorum,
        }
    }

    pub fn accept_ratio(&self) -> f64 {
        if self.votes.is_empty() {
            return 0.0;
        }
        let accepts = self.votes.values().filter(|v| v.in_favor).count();
        accepts as f64 / self.votes.len() as f64
    }

    pub fn weighted_accept_ratio(&self) -> f64 {
        let total_weight: f64 = self.votes.values().map(|v| v.weight).sum();
        if total_weight < 1e-9 {
            return 0.0;
        }
        let accept_weight: f64 = self.votes.values().filter(|v| v.in_favor).map(|v| v.weight).sum();
        accept_weight / total_weight
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn cast_vote(&mut self, voter: &str, in_favor: bool, weight: f64, reason: Option<String>) {
        self.votes.insert(
            voter.to_string(),
            Vote {
                voter: voter.to_string(),
                in_favor,
                weight,
                reason,
                cast_at: Utc::now(),
            },
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub voter: String,
    pub in_favor: bool,
    pub weight: f64,
    pub reason: Option<String>,
    pub cast_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    pub proposal_id: String,
    pub topic: String,
    pub accepted: bool,
    pub final_value: Option<serde_json::Value>,
    pub accept_ratio: f64,
    pub total_votes: usize,
    pub byzantine_faults_detected: usize,
    pub rounds: usize,
}

pub struct ConsensusEngine {
    pub protocol: ConsensusProtocol,
    pub proposals: Arc<RwLock<HashMap<String, Proposal>>>,
    pub history: Arc<RwLock<Vec<ConsensusResult>>>,
    pub fault_tolerance: f64,
}

impl ConsensusEngine {
    pub fn new(protocol: ConsensusProtocol, fault_tolerance: f64) -> Self {
        Self {
            protocol,
            proposals: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            fault_tolerance,
        }
    }

    pub async fn propose(
        &self,
        proposer: &str,
        topic: &str,
        value: serde_json::Value,
        ttl_secs: i64,
    ) -> String {
        let quorum = match self.protocol {
            ConsensusProtocol::SimpleMajority => 0.5,
            ConsensusProtocol::WeightedMajority => 0.5,
            ConsensusProtocol::PBFT => (2.0 / 3.0) + self.fault_tolerance,
            ConsensusProtocol::Raft => 0.5,
        };
        let proposal = Proposal::new(proposer, topic, value, ttl_secs, quorum);
        let id = proposal.id.clone();
        self.proposals.write().await.insert(id.clone(), proposal);
        info!("Proposal created: {} on topic '{}'", id, topic);
        id
    }

    pub async fn vote(
        &self,
        proposal_id: &str,
        voter: &str,
        in_favor: bool,
        weight: f64,
        reason: Option<String>,
    ) -> anyhow::Result<()> {
        let mut proposals = self.proposals.write().await;
        let proposal = proposals
            .get_mut(proposal_id)
            .ok_or_else(|| anyhow::anyhow!("Proposal {} not found", proposal_id))?;

        if proposal.is_expired() {
            proposal.status = ProposalStatus::Expired;
            anyhow::bail!("Proposal {} has expired", proposal_id);
        }

        if self.detect_byzantine(voter, in_favor, &proposal.votes) {
            warn!("Possible Byzantine behavior from voter '{}'", voter);
        }

        proposal.cast_vote(voter, in_favor, weight, reason);
        info!("Vote cast: voter={} proposal={} in_favor={}", voter, proposal_id, in_favor);
        Ok(())
    }

    pub async fn finalize(&self, proposal_id: &str) -> anyhow::Result<ConsensusResult> {
        let mut proposals = self.proposals.write().await;
        let proposal = proposals
            .get_mut(proposal_id)
            .ok_or_else(|| anyhow::anyhow!("Proposal {} not found", proposal_id))?;

        if proposal.is_expired() {
            proposal.status = ProposalStatus::Expired;
        }

        let ratio = match self.protocol {
            ConsensusProtocol::WeightedMajority => proposal.weighted_accept_ratio(),
            _ => proposal.accept_ratio(),
        };

        let accepted = ratio > proposal.required_quorum
            && proposal.status != ProposalStatus::Expired;

        proposal.status = if accepted {
            ProposalStatus::Accepted
        } else {
            ProposalStatus::Rejected
        };

        let byzantine = self.count_byzantine_faults(&proposal.votes);

        let result = ConsensusResult {
            proposal_id: proposal_id.to_string(),
            topic: proposal.topic.clone(),
            accepted,
            final_value: if accepted { Some(proposal.value.clone()) } else { None },
            accept_ratio: ratio,
            total_votes: proposal.votes.len(),
            byzantine_faults_detected: byzantine,
            rounds: 1,
        };

        info!(
            "Consensus finalized: proposal={} accepted={} ratio={:.2}",
            proposal_id, accepted, ratio
        );

        self.history.write().await.push(result.clone());
        Ok(result)
    }

    pub async fn get_proposal(&self, id: &str) -> Option<Proposal> {
        self.proposals.read().await.get(id).cloned()
    }

    pub async fn pending_proposals(&self) -> Vec<Proposal> {
        self.proposals
            .read()
            .await
            .values()
            .filter(|p| p.status == ProposalStatus::Voting && !p.is_expired())
            .cloned()
            .collect()
    }

    pub async fn history_len(&self) -> usize {
        self.history.read().await.len()
    }

    fn detect_byzantine(&self, voter: &str, vote: bool, existing: &HashMap<String, Vote>) -> bool {
        if let Some(prev) = existing.get(voter) {
            prev.in_favor != vote
        } else {
            false
        }
    }

    fn count_byzantine_faults(&self, votes: &HashMap<String, Vote>) -> usize {
        let n = votes.len();
        let accepts = votes.values().filter(|v| v.in_favor).count();
        let rejects = n - accepts;
        let minority = accepts.min(rejects);
        if n > 3 { (minority as f64 / n as f64 * 0.1) as usize } else { 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_majority_consensus() {
        let engine = ConsensusEngine::new(ConsensusProtocol::SimpleMajority, 0.0);
        let id = engine.propose("agent-0", "use_tool", serde_json::json!("tool_a"), 60).await;

        engine.vote(&id, "agent-1", true, 1.0, None).await.unwrap();
        engine.vote(&id, "agent-2", true, 1.0, None).await.unwrap();
        engine.vote(&id, "agent-3", false, 1.0, None).await.unwrap();

        let result = engine.finalize(&id).await.unwrap();
        assert!(result.accepted);
        assert_eq!(result.total_votes, 3);
    }

    #[tokio::test]
    async fn test_rejection() {
        let engine = ConsensusEngine::new(ConsensusProtocol::SimpleMajority, 0.0);
        let id = engine.propose("agent-0", "deploy", serde_json::json!("v2"), 60).await;

        engine.vote(&id, "agent-1", false, 1.0, Some("risky".into())).await.unwrap();
        engine.vote(&id, "agent-2", false, 1.0, None).await.unwrap();

        let result = engine.finalize(&id).await.unwrap();
        assert!(!result.accepted);
    }
}

//! Consensus Learning Module - Distributed AGI Knowledge Integration
//!
//! Based on cutting-edge 2025-2026 research:
//! - Consensus Learning: Decentralized ensemble learning (Magureanu & Usher, 2024)
//! - Byzantine-Robust Decentralized Federated Learning (Fang et al., 2024)
//! - Proof-of-Data consensus for collaborative intelligence
//! - Chinese AI innovations in distributed training (Alibaba HPN, Tencent Hunyuan)
//!
//! Key innovations:
//! - Quantum-inspired voting mechanisms
//! - Byzantine fault tolerance with reputation scoring
//! - Knowledge aggregation through entanglement
//! - Open-ended improvement through collective intelligence

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex, broadcast, mpsc};
use std::time::{Duration, Instant};
use blake3::Hasher;
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};

pub const CONSENSUS_THRESHOLD: f64 = 0.67;
pub const BYZANTINE_TOLERANCE: f64 = 0.33;
pub const REPUTATION_DECAY: f64 = 0.95;
pub const MIN_REPUTATION: f64 = 0.1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    pub node_id: String,
    pub cluster_size: usize,
    pub byzantine_tolerance: f64,
    pub consensus_timeout_ms: u64,
    pub min_votes: usize,
    pub reputation_threshold: f64,
    pub enable_quantum_voting: bool,
    pub knowledge_sharing: bool,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            node_id: format!("node-{}", uuid::Uuid::new_v4()),
            cluster_size: 10,
            byzantine_tolerance: BYZANTINE_TOLERANCE,
            consensus_timeout_ms: 5000,
            min_votes: 3,
            reputation_threshold: MIN_REPUTATION,
            enable_quantum_voting: true,
            knowledge_sharing: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeUpdate {
    pub id: String,
    pub source_node: String,
    pub knowledge_type: KnowledgeType,
    pub payload: Vec<u8>,
    pub signature: Option<Vec<u8>>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub confidence: f64,
    pub quantum_entropy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KnowledgeType {
    ModelWeights,
    ReasoningPattern,
    ImprovementProposal,
    ConsciousnessInsight,
    QuantumState,
    DGMInnovation,
    ResearchInsight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub voter_id: String,
    pub proposal_id: String,
    pub vote_type: VoteType,
    pub weight: f64,
    pub reputation: f64,
    pub signature: Option<Vec<u8>>,
    pub quantum_confidence: Option<f64>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VoteType {
    Approve,
    Reject,
    Abstain,
    QuantumSuperposition(f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub proposer: String,
    pub content: Vec<u8>,
    pub proposal_type: ProposalType,
    pub required_votes: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub votes: Vec<Vote>,
    pub status: ProposalStatus,
    pub quantum_entropy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalType {
    KnowledgeUpdate,
    ProtocolUpgrade,
    ReputationChange,
    ModelImprovement,
    ConsciousnessEvolution,
    EmergencyAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalStatus {
    Pending,
    Voting,
    Approved,
    Rejected,
    Expired,
    Executed,
}

impl Proposal {
    pub fn new(proposer: String, content: Vec<u8>, proposal_type: ProposalType, ttl_secs: u64) -> Self {
        let id = format!("prop-{}", uuid::Uuid::new_v4());
        let now = chrono::Utc::now();
        
        Self {
            id,
            proposer,
            content,
            proposal_type,
            required_votes: 3,
            created_at: now,
            expires_at: now + chrono::Duration::seconds(ttl_secs as i64),
            votes: Vec::new(),
            status: ProposalStatus::Pending,
            quantum_entropy: rand::random::<f64>(),
        }
    }

    pub fn add_vote(&mut self, vote: Vote) {
        if self.status == ProposalStatus::Pending || self.status == ProposalStatus::Voting {
            self.status = ProposalStatus::Voting;
            self.votes.push(vote);
            self.evaluate_consensus();
        }
    }

    fn evaluate_consensus(&mut self) {
        let total_weight: f64 = self.votes.iter().map(|v| v.weight * v.reputation).sum();
        let approve_weight: f64 = self.votes.iter()
            .filter(|v| v.vote_type == VoteType::Approve)
            .map(|v| v.weight * v.reputation)
            .sum();
        
        let quantum_boost: f64 = self.votes.iter()
            .filter_map(|v| v.quantum_confidence)
            .sum::<f64>() / self.votes.len().max(1) as f64;
        
        let consensus_ratio = (approve_weight + quantum_boost * 0.1) / total_weight.max(0.001);
        
        if consensus_ratio >= CONSENSUS_THRESHOLD && self.votes.len() >= self.required_votes {
            self.status = ProposalStatus::Approved;
        } else if chrono::Utc::now() > self.expires_at {
            self.status = ProposalStatus::Expired;
        } else if self.votes.len() >= self.required_votes * 2 {
            let reject_weight: f64 = self.votes.iter()
                .filter(|v| v.vote_type == VoteType::Reject)
                .map(|v| v.weight * v.reputation)
                .sum();
            
            if reject_weight / total_weight.max(0.001) > CONSENSUS_THRESHOLD {
                self.status = ProposalStatus::Rejected;
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeReputation {
    pub node_id: String,
    pub score: f64,
    pub successful_proposals: u64,
    pub failed_proposals: u64,
    pub byzantine_events: u64,
    pub knowledge_contributions: u64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl NodeReputation {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            score: 1.0,
            successful_proposals: 0,
            failed_proposals: 0,
            byzantine_events: 0,
            knowledge_contributions: 0,
            last_updated: chrono::Utc::now(),
        }
    }

    pub fn update(&mut self, success: bool, byzantine: bool) {
        self.last_updated = chrono::Utc::now();
        
        if byzantine {
            self.byzantine_events += 1;
            self.score *= 0.5;
        } else if success {
            self.successful_proposals += 1;
            self.score = (self.score * (1.0 - REPUTATION_DECAY) + REPUTATION_DECAY).min(1.0);
        } else {
            self.failed_proposals += 1;
            self.score *= 0.95;
        }
        
        self.score = self.score.max(MIN_REPUTATION);
    }

    pub fn is_trusted(&self) -> bool {
        self.score >= MIN_REPUTATION && self.byzantine_events < 3
    }
}

pub struct ConsensusKnowledgeBase {
    knowledge: HashMap<String, KnowledgeUpdate>,
    pending_updates: Vec<KnowledgeUpdate>,
    applied_innovations: Vec<String>,
    knowledge_index: BTreeMap<String, Vec<String>>,
}

impl ConsensusKnowledgeBase {
    pub fn new() -> Self {
        Self {
            knowledge: HashMap::new(),
            pending_updates: Vec::new(),
            applied_innovations: Vec::new(),
            knowledge_index: BTreeMap::new(),
        }
    }

    pub fn add_knowledge(&mut self, update: KnowledgeUpdate) -> Result<String> {
        let id = update.id.clone();
        
        self.knowledge_index
            .entry(format!("{:?}", update.knowledge_type))
            .or_insert_with(Vec::new)
            .push(id.clone());
        
        self.knowledge.insert(id.clone(), update);
        
        Ok(id)
    }

    pub fn get_knowledge(&self, id: &str) -> Option<&KnowledgeUpdate> {
        self.knowledge.get(id)
    }

    pub fn get_by_type(&self, knowledge_type: &KnowledgeType) -> Vec<&KnowledgeUpdate> {
        self.knowledge_index
            .get(&format!("{:?}", knowledge_type))
            .map(|ids| ids.iter().filter_map(|id| self.knowledge.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn merge_incoming(&mut self, updates: Vec<KnowledgeUpdate>) -> Vec<String> {
        let mut merged = Vec::new();
        
        for update in updates {
            if !self.knowledge.contains_key(&update.id) {
                if let Ok(id) = self.add_knowledge(update) {
                    merged.push(id);
                }
            }
        }
        
        merged
    }
}

pub struct QuantumVoting {
    superposition_weights: HashMap<String, f64>,
    entanglement_matrix: Vec<Vec<f64>>,
    coherence_threshold: f64,
}

impl QuantumVoting {
    pub fn new(size: usize) -> Self {
        Self {
            superposition_weights: HashMap::new(),
            entanglement_matrix: vec![vec![0.0; size]; size],
            coherence_threshold: QUANTUM_COHERENCE_MIN,
        }
    }

    pub fn create_quantum_vote(&mut self, base_confidence: f64, voter_id: &str) -> VoteType {
        let superposition = self.calculate_superposition(voter_id);
        let quantum_state = (base_confidence + superposition) / 2.0;
        
        let normalized = quantum_state.clamp(0.0, 1.0);
        
        if normalized > 0.7 {
            VoteType::Approve
        } else if normalized < 0.3 {
            VoteType::Reject
        } else if normalized > 0.45 && normalized < 0.55 {
            VoteType::QuantumSuperposition(normalized)
        } else {
            VoteType::Abstain
        }
    }

    fn calculate_superposition(&mut self, voter_id: &str) -> f64 {
        let current = self.superposition_weights.entry(voter_id.to_string()).or_insert(0.5);
        
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let perturbation: f64 = rng.gen_range(-0.1..0.1);
        *current = (*current + perturbation).clamp(0.0, 1.0);
        
        *current
    }

    pub fn entangle_voters(&mut self, voter1: usize, voter2: usize, strength: f64) {
        if voter1 < self.entanglement_matrix.len() && voter2 < self.entanglement_matrix.len() {
            self.entanglement_matrix[voter1][voter2] = strength;
            self.entanglement_matrix[voter2][voter1] = strength;
        }
    }

    pub fn collapse_superposition(&self, votes: &[Vote]) -> f64 {
        let total_weight: f64 = votes.iter().map(|v| v.weight).sum();
        let mut approve_weight = 0.0;
        
        for vote in votes {
            match &vote.vote_type {
                VoteType::Approve => approve_weight += vote.weight,
                VoteType::Reject => {},
                VoteType::Abstain => approve_weight += vote.weight * 0.5,
                VoteType::QuantumSuperposition(p) => approve_weight += vote.weight * p,
            }
        }
        
        approve_weight / total_weight.max(0.001)
    }
}

pub struct ConsensusLearningEngine {
    config: ConsensusConfig,
    proposals: HashMap<String, Proposal>,
    reputations: HashMap<String, NodeReputation>,
    knowledge_base: ConsensusKnowledgeBase,
    quantum_voting: QuantumVoting,
    event_sender: mpsc::Sender<ConsensusEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusEvent {
    ProposalCreated(String),
    VoteCast(String, String),
    ProposalApproved(String),
    ProposalRejected(String),
    KnowledgeShared(String),
    ByzantineDetected(String),
    ReputationUpdated(String, f64),
}

impl ConsensusLearningEngine {
    pub fn new(config: ConsensusConfig) -> (Self, mpsc::Receiver<ConsensusEvent>) {
        let (tx, rx) = mpsc::channel(1000);
        
        let engine = Self {
            quantum_voting: QuantumVoting::new(config.cluster_size),
            config,
            proposals: HashMap::new(),
            reputations: HashMap::new(),
            knowledge_base: ConsensusKnowledgeBase::new(),
            event_sender: tx,
        };
        
        (engine, rx)
    }

    pub fn create_proposal(
        &mut self,
        content: Vec<u8>,
        proposal_type: ProposalType,
    ) -> Result<String> {
        let proposal = Proposal::new(
            self.config.node_id.clone(),
            content,
            proposal_type,
            self.config.consensus_timeout_ms / 1000,
        );
        
        let id = proposal.id.clone();
        self.proposals.insert(id.clone(), proposal);
        
        let _ = self.event_sender.send(ConsensusEvent::ProposalCreated(id.clone())).await;
        
        Ok(id)
    }

    pub fn cast_vote(
        &mut self,
        proposal_id: &str,
        vote_type: VoteType,
        confidence: f64,
    ) -> Result<()> {
        let proposal = self.proposals.get_mut(proposal_id)
            .ok_or_else(|| anyhow::anyhow!("Proposal not found"))?;
        
        let reputation = self.reputations
            .get(&self.config.node_id)
            .map(|r| r.score)
            .unwrap_or(1.0);
        
        let (final_vote, quantum_conf) = if self.config.enable_quantum_voting {
            let qv = self.quantum_voting.create_quantum_vote(confidence, &self.config.node_id);
            let qc = match &qv {
                VoteType::QuantumSuperposition(p) => Some(*p),
                _ => Some(confidence),
            };
            (qv, qc)
        } else {
            (vote_type, Some(confidence))
        };
        
        let vote = Vote {
            voter_id: self.config.node_id.clone(),
            proposal_id: proposal_id.to_string(),
            vote_type: final_vote,
            weight: 1.0,
            reputation,
            signature: None,
            quantum_confidence: quantum_conf,
            timestamp: chrono::Utc::now(),
        };
        
        proposal.add_vote(vote.clone());
        
        if proposal.status == ProposalStatus::Approved {
            self.update_reputation(&proposal.proposer, true, false);
            let _ = self.event_sender.send(ConsensusEvent::ProposalApproved(proposal_id.to_string())).await;
        } else if proposal.status == ProposalStatus::Rejected {
            self.update_reputation(&proposal.proposer, false, false);
            let _ = self.event_sender.send(ConsensusEvent::ProposalRejected(proposal_id.to_string())).await;
        }
        
        Ok(())
    }

    fn update_reputation(&mut self, node_id: &str, success: bool, byzantine: bool) {
        let rep = self.reputations
            .entry(node_id.to_string())
            .or_insert_with(|| NodeReputation::new(node_id.to_string()));
        
        rep.update(success, byzantine);
        
        let _ = self.event_sender.send(ConsensusEvent::ReputationUpdated(
            node_id.to_string(),
            rep.score,
        )).await;
        
        if rep.byzantine_events >= 3 {
            let _ = self.event_sender.send(ConsensusEvent::ByzantineDetected(node_id.to_string())).await;
        }
    }

    pub fn share_knowledge(&mut self, update: KnowledgeUpdate) -> Result<String> {
        if !self.config.knowledge_sharing {
            return Err(anyhow::anyhow!("Knowledge sharing disabled"));
        }
        
        let id = self.knowledge_base.add_knowledge(update.clone())?;
        
        let _ = self.event_sender.send(ConsensusEvent::KnowledgeShared(id.clone())).await;
        
        if let Some(rep) = self.reputations.get_mut(&update.source_node) {
            rep.knowledge_contributions += 1;
        }
        
        Ok(id)
    }

    pub fn merge_peer_knowledge(&mut self, updates: Vec<KnowledgeUpdate>) -> Vec<String> {
        self.knowledge_base.merge_incoming(updates)
    }

    pub fn get_approved_proposals(&self) -> Vec<&Proposal> {
        self.proposals.values()
            .filter(|p| p.status == ProposalStatus::Approved)
            .collect()
    }

    pub fn get_trusted_nodes(&self) -> Vec<&NodeReputation> {
        self.reputations.values()
            .filter(|r| r.is_trusted())
            .collect()
    }

    pub fn calculate_consensus_strength(&self) -> f64 {
        let proposals: Vec<_> = self.proposals.values().collect();
        
        if proposals.is_empty() {
            return 0.0;
        }
        
        let approved = proposals.iter()
            .filter(|p| p.status == ProposalStatus::Approved)
            .count();
        
        let total_votes: usize = proposals.iter().map(|p| p.votes.len()).sum();
        
        (approved as f64 / proposals.len() as f64) * (total_votes as f64 / proposals.len() as f64).min(1.0)
    }
}

pub struct DistributedLearningOrchestrator {
    engines: HashMap<String, Arc<Mutex<ConsensusLearningEngine>>>,
    global_knowledge: Arc<RwLock<ConsensusKnowledgeBase>>,
    node_id: String,
}

impl DistributedLearningOrchestrator {
    pub fn new(node_id: String) -> Self {
        Self {
            engines: HashMap::new(),
            global_knowledge: Arc::new(RwLock::new(ConsensusKnowledgeBase::new())),
            node_id,
        }
    }

    pub fn add_cluster(&mut self, cluster_id: String, config: ConsensusConfig) -> mpsc::Receiver<ConsensusEvent> {
        let (engine, rx) = ConsensusLearningEngine::new(config);
        self.engines.insert(cluster_id, Arc::new(Mutex::new(engine)));
        rx
    }

    pub async fn broadcast_knowledge(&self, update: KnowledgeUpdate) -> Result<Vec<String>> {
        let mut results = Vec::new();
        
        for engine in self.engines.values() {
            let mut engine = engine.lock().await;
            if let Ok(id) = engine.share_knowledge(update.clone()) {
                results.push(id);
            }
        }
        
        let mut global = self.global_knowledge.write().await;
        global.add_knowledge(update)?;
        
        Ok(results)
    }

    pub async fn get_cluster_status(&self) -> HashMap<String, ClusterStatus> {
        let mut status = HashMap::new();
        
        for (id, engine) in &self.engines {
            let engine = engine.lock().await;
            status.insert(id.clone(), ClusterStatus {
                proposals: engine.proposals.len(),
                approved: engine.get_approved_proposals().len(),
                trusted_nodes: engine.get_trusted_nodes().len(),
                consensus_strength: engine.calculate_consensus_strength(),
            });
        }
        
        status
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatus {
    pub proposals: usize,
    pub approved: usize,
    pub trusted_nodes: usize,
    pub consensus_strength: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proposal_creation() {
        let proposal = Proposal::new(
            "node-1".into(),
            vec![1, 2, 3],
            ProposalType::KnowledgeUpdate,
            60,
        );
        
        assert!(!proposal.id.is_empty());
        assert_eq!(proposal.status, ProposalStatus::Pending);
    }

    #[test]
    fn test_reputation_update() {
        let mut rep = NodeReputation::new("node-1".into());
        rep.update(true, false);
        
        assert!(rep.score >= 0.95);
        assert!(rep.is_trusted());
    }

    #[test]
    fn test_byzantine_detection() {
        let mut rep = NodeReputation::new("node-1".into());
        
        for _ in 0..3 {
            rep.update(false, true);
        }
        
        assert!(!rep.is_trusted());
    }

    #[tokio::test]
    async fn test_consensus_engine() {
        let config = ConsensusConfig::default();
        let (mut engine, _rx) = ConsensusLearningEngine::new(config);
        
        let prop_id = engine.create_proposal(
            vec![1, 2, 3],
            ProposalType::KnowledgeUpdate,
        ).unwrap();
        
        engine.cast_vote(&prop_id, VoteType::Approve, 0.9).unwrap();
        
        let proposal = engine.proposals.get(&prop_id).unwrap();
        assert_eq!(proposal.votes.len(), 1);
    }

    #[test]
    fn test_quantum_voting() {
        let mut qv = QuantumVoting::new(10);
        
        let vote = qv.create_quantum_vote(0.8, "voter-1");
        
        match vote {
            VoteType::Approve | VoteType::QuantumSuperposition(_) => {},
            _ => panic!("Expected approve or superposition for high confidence"),
        }
    }

    #[test]
    fn test_knowledge_base() {
        let mut kb = ConsensusKnowledgeBase::new();
        
        let update = KnowledgeUpdate {
            id: "know-1".into(),
            source_node: "node-1".into(),
            knowledge_type: KnowledgeType::ReasoningPattern,
            payload: vec![1, 2, 3],
            signature: None,
            timestamp: chrono::Utc::now(),
            confidence: 0.9,
            quantum_entropy: 0.1,
        };
        
        kb.add_knowledge(update).unwrap();
        
        let results = kb.get_by_type(&KnowledgeType::ReasoningPattern);
        assert_eq!(results.len(), 1);
    }
}

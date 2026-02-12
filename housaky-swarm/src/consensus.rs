//! Swarm Consensus - Collective decision making

use crate::swarm::{Agent, AgentType};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ConsensusProposal {
    pub id: String,
    pub proposer: String,
    pub data: Vec<u8>,
    pub votes: HashMap<String, Vote>,
    pub threshold: f64,
}

#[derive(Debug, Clone)]
pub enum Vote {
    Approve(f64), // confidence
    Reject(f64),
    Abstain,
}

pub struct SwarmConsensus {
    proposals: HashMap<String, ConsensusProposal>,
    agent_weights: HashMap<String, f64>,
}

impl SwarmConsensus {
    pub fn new() -> Self {
        Self {
            proposals: HashMap::new(),
            agent_weights: HashMap::new(),
        }
    }

    /// Create new proposal
    pub fn propose(&mut self, proposer: String, data: Vec<u8>, threshold: f64) -> String {
        let id = format!("prop_{}_{}", proposer, rand::random::<u32>());
        
        let proposal = ConsensusProposal {
            id: id.clone(),
            proposer,
            data,
            votes: HashMap::new(),
            threshold,
        };

        self.proposals.insert(id.clone(), proposal);
        id
    }

    /// Cast vote on proposal
    pub fn vote(&mut self, proposal_id: &str, agent_id: String, vote: Vote) -> bool {
        if let Some(proposal) = self.proposals.get_mut(proposal_id) {
            proposal.votes.insert(agent_id, vote);
            true
        } else {
            false
        }
    }

    /// Check if proposal reached consensus
    pub fn check_consensus(&self, proposal_id: &str) -> Option<bool> {
        let proposal = self.proposals.get(proposal_id)?;

        let mut total_weight = 0.0;
        let mut approve_weight = 0.0;

        for (agent_id, vote) in &proposal.votes {
            let weight = self.agent_weights.get(agent_id).copied().unwrap_or(1.0);
            total_weight += weight;

            match vote {
                Vote::Approve(confidence) => approve_weight += weight * confidence,
                Vote::Reject(_) => {},
                Vote::Abstain => {},
            }
        }

        if total_weight == 0.0 {
            return None;
        }

        let approval_ratio = approve_weight / total_weight;
        Some(approval_ratio >= proposal.threshold)
    }

    /// Update agent weight based on performance
    pub fn update_agent_weight(&mut self, agent_id: String, weight: f64) {
        self.agent_weights.insert(agent_id, weight.clamp(0.1, 10.0));
    }

    /// Get proposal
    pub fn get_proposal(&self, proposal_id: &str) -> Option<&ConsensusProposal> {
        self.proposals.get(proposal_id)
    }
}

impl Default for SwarmConsensus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_proposal() {
        let mut consensus = SwarmConsensus::new();
        let id = consensus.propose("agent_1".to_string(), vec![1, 2, 3], 0.66);
        
        assert!(consensus.get_proposal(&id).is_some());
    }

    #[test]
    fn test_consensus_voting() {
        let mut consensus = SwarmConsensus::new();
        let id = consensus.propose("agent_1".to_string(), vec![1, 2, 3], 0.66);
        
        consensus.vote(&id, "agent_2".to_string(), Vote::Approve(0.9));
        consensus.vote(&id, "agent_3".to_string(), Vote::Approve(0.8));
        consensus.vote(&id, "agent_4".to_string(), Vote::Reject(0.5));

        let result = consensus.check_consensus(&id);
        assert!(result.is_some());
    }
}

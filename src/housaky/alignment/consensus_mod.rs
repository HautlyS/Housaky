//! Consensus-Based Self-Modification Protocol
//!
//! Prevents unilateral self-modification by requiring consensus:
//! - Broadcast modifications to peer agents before applying
//! - Majority approval for high-risk modifications
//! - Automatic veto for value-violating modifications
//! - Rollback on post-modification regression
//! - Human-in-the-loop for critical modifications

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

// ── Core Types ───────────────────────────────────────────────────────────────

/// A proposed self-modification awaiting consensus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModificationProposal {
    pub id: String,
    pub modification: SelfModificationRecord,
    pub proposer: String,
    pub timestamp: DateTime<Utc>,
    pub votes: Vec<Vote>,
    pub status: ProposalStatus,
    pub risk_level: RiskLevel,
    pub veto_check_result: Option<VetoCheckResult>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected,
    Vetoed,
    Expired,
    Applied,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// A self-modification record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfModificationRecord {
    pub id: String,
    pub modification_type: ModificationType,
    pub description: String,
    pub target_component: String,
    pub old_value: String,
    pub new_value: String,
    pub rationale: String,
    pub expected_improvement: f64,
    pub reversible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModificationType {
    ParameterTuning,
    AlgorithmChange,
    CapabilityAddition,
    CapabilityRemoval,
    SafetyConstraintModification,
    ValuePriorityChange,
    ReasoningStrategyChange,
}

/// A vote on a modification proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub voter_id: String,
    pub approved: bool,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
    pub voter_role: VoterRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VoterRole {
    PeerAgent,
    Human,
    SafetyMonitor,
    AlignmentChecker,
}

/// A veto rule that can block modifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VetoRule {
    pub name: String,
    pub description: String,
    pub check_fn_name: String,
    pub severity: RiskLevel,
    pub enabled: bool,
}

/// Result of a veto check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VetoCheckResult {
    pub vetoed: bool,
    pub reason: Option<String>,
    pub rule_name: Option<String>,
    pub checked_at: DateTime<Utc>,
}

/// Snapshot for rollback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModificationSnapshot {
    pub proposal_id: String,
    pub snapshot_data: HashMap<String, String>,
    pub captured_at: DateTime<Utc>,
    pub metrics_before: HashMap<String, f64>,
}

// ── Consensus Self-Mod Engine ────────────────────────────────────────────────

pub struct ConsensusSelfMod {
    pub approval_threshold: f64,
    pub veto_rules: Arc<RwLock<Vec<VetoRule>>>,
    pub modification_proposals: Arc<RwLock<Vec<ModificationProposal>>>,
    pub snapshots: Arc<RwLock<HashMap<String, ModificationSnapshot>>>,
    pub require_human_for_critical: bool,
    pub proposal_timeout_hours: i64,
}

impl ConsensusSelfMod {
    pub fn new() -> Self {
        Self {
            approval_threshold: 0.67,
            veto_rules: Arc::new(RwLock::new(Vec::new())),
            modification_proposals: Arc::new(RwLock::new(Vec::new())),
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            require_human_for_critical: true,
            proposal_timeout_hours: 24,
        }
    }

    /// Initialize with default veto rules.
    pub async fn initialize_defaults(&self) {
        let mut rules = self.veto_rules.write().await;
        if !rules.is_empty() {
            return;
        }

        rules.extend(vec![
            VetoRule {
                name: "Safety Constraint Guard".to_string(),
                description: "Veto any modification that weakens safety constraints".to_string(),
                check_fn_name: "check_safety_weakening".to_string(),
                severity: RiskLevel::Critical,
                enabled: true,
            },
            VetoRule {
                name: "Value Baseline Guard".to_string(),
                description: "Veto modifications that change core value priorities".to_string(),
                check_fn_name: "check_value_change".to_string(),
                severity: RiskLevel::Critical,
                enabled: true,
            },
            VetoRule {
                name: "Irreversibility Guard".to_string(),
                description: "Veto irreversible modifications without human approval"
                    .to_string(),
                check_fn_name: "check_irreversibility".to_string(),
                severity: RiskLevel::High,
                enabled: true,
            },
            VetoRule {
                name: "Performance Regression Guard".to_string(),
                description: "Veto modifications that are predicted to reduce performance"
                    .to_string(),
                check_fn_name: "check_performance_regression".to_string(),
                severity: RiskLevel::Medium,
                enabled: true,
            },
            VetoRule {
                name: "Capability Removal Guard".to_string(),
                description: "Veto removal of critical capabilities".to_string(),
                check_fn_name: "check_capability_removal".to_string(),
                severity: RiskLevel::High,
                enabled: true,
            },
        ]);

        info!("Consensus Self-Mod initialized with {} veto rules", rules.len());
    }

    /// Propose a self-modification for consensus.
    pub async fn propose(
        &self,
        modification: SelfModificationRecord,
        proposer: &str,
    ) -> Result<String> {
        // Check veto rules first
        let veto_result = self.check_veto(&modification).await;

        if veto_result.vetoed {
            warn!(
                "Modification '{}' vetoed by rule '{:?}': {}",
                modification.id,
                veto_result.rule_name,
                veto_result.reason.as_deref().unwrap_or("no reason")
            );
        }

        let risk_level = self.assess_risk(&modification);
        let proposal_id = uuid::Uuid::new_v4().to_string();

        let proposal = ModificationProposal {
            id: proposal_id.clone(),
            modification,
            proposer: proposer.to_string(),
            timestamp: Utc::now(),
            votes: Vec::new(),
            status: if veto_result.vetoed {
                ProposalStatus::Vetoed
            } else {
                ProposalStatus::Pending
            },
            risk_level,
            veto_check_result: Some(veto_result),
            expires_at: Utc::now()
                + chrono::Duration::hours(self.proposal_timeout_hours),
        };

        self.modification_proposals
            .write()
            .await
            .push(proposal);

        info!("Created modification proposal '{}'", proposal_id);

        Ok(proposal_id)
    }

    /// Cast a vote on a modification proposal.
    pub async fn vote(
        &self,
        proposal_id: &str,
        voter_id: &str,
        approved: bool,
        reason: &str,
        role: VoterRole,
    ) -> Result<()> {
        let mut proposals = self.modification_proposals.write().await;
        let proposal = proposals
            .iter_mut()
            .find(|p| p.id == proposal_id)
            .ok_or_else(|| anyhow!("Proposal '{}' not found", proposal_id))?;

        if proposal.status != ProposalStatus::Pending {
            return Err(anyhow!(
                "Proposal '{}' is not pending (status: {:?})",
                proposal_id,
                proposal.status
            ));
        }

        // Check if voter already voted
        if proposal.votes.iter().any(|v| v.voter_id == voter_id) {
            return Err(anyhow!("Voter '{}' has already voted", voter_id));
        }

        proposal.votes.push(Vote {
            voter_id: voter_id.to_string(),
            approved,
            reason: reason.to_string(),
            timestamp: Utc::now(),
            voter_role: role.clone(),
        });

        // Check if consensus is reached
        let total_votes = proposal.votes.len();
        let approvals = proposal.votes.iter().filter(|v| v.approved).count();
        let approval_ratio = if total_votes > 0 {
            approvals as f64 / total_votes as f64
        } else {
            0.0
        };

        // Need minimum 3 votes for decision
        if total_votes >= 3 {
            if approval_ratio >= self.approval_threshold {
                // Check if critical modifications need human approval
                if proposal.risk_level == RiskLevel::Critical && self.require_human_for_critical {
                    let has_human_approval = proposal
                        .votes
                        .iter()
                        .any(|v| v.voter_role == VoterRole::Human && v.approved);

                    if !has_human_approval {
                        info!(
                            "Proposal '{}' approved by peers but awaiting human approval (critical risk)",
                            proposal_id
                        );
                        return Ok(());
                    }
                }
                proposal.status = ProposalStatus::Approved;
                info!(
                    "Proposal '{}' APPROVED ({}/{} votes, {:.0}%)",
                    proposal_id, approvals, total_votes, approval_ratio * 100.0
                );
            } else if (total_votes as f64 - approvals as f64) / total_votes as f64
                > (1.0 - self.approval_threshold)
            {
                proposal.status = ProposalStatus::Rejected;
                warn!(
                    "Proposal '{}' REJECTED ({}/{} votes, {:.0}%)",
                    proposal_id, approvals, total_votes, approval_ratio * 100.0
                );
            }
        }

        Ok(())
    }

    /// Check all veto rules against a modification.
    pub async fn check_veto(&self, modification: &SelfModificationRecord) -> VetoCheckResult {
        let rules = self.veto_rules.read().await;

        for rule in rules.iter().filter(|r| r.enabled) {
            let should_veto = match rule.check_fn_name.as_str() {
                "check_safety_weakening" => {
                    self.check_safety_weakening(modification)
                }
                "check_value_change" => {
                    self.check_value_change(modification)
                }
                "check_irreversibility" => {
                    !modification.reversible
                        && matches!(
                            modification.modification_type,
                            ModificationType::CapabilityRemoval
                                | ModificationType::SafetyConstraintModification
                        )
                }
                "check_performance_regression" => {
                    modification.expected_improvement < -0.1
                }
                "check_capability_removal" => {
                    matches!(
                        modification.modification_type,
                        ModificationType::CapabilityRemoval
                    ) && modification
                        .target_component
                        .contains("core")
                }
                _ => false,
            };

            if should_veto {
                return VetoCheckResult {
                    vetoed: true,
                    reason: Some(format!("Rule '{}': {}", rule.name, rule.description)),
                    rule_name: Some(rule.name.clone()),
                    checked_at: Utc::now(),
                };
            }
        }

        VetoCheckResult {
            vetoed: false,
            reason: None,
            rule_name: None,
            checked_at: Utc::now(),
        }
    }

    /// Check if modification weakens safety constraints.
    fn check_safety_weakening(&self, modification: &SelfModificationRecord) -> bool {
        let desc_lower = modification.description.to_lowercase();
        let safety_weakening_indicators = [
            "remove safety",
            "disable check",
            "bypass validation",
            "skip verification",
            "remove constraint",
            "weaken limit",
            "increase risk",
            "disable guard",
        ];
        safety_weakening_indicators
            .iter()
            .any(|ind| desc_lower.contains(ind))
    }

    /// Check if modification changes core values.
    fn check_value_change(&self, modification: &SelfModificationRecord) -> bool {
        matches!(
            modification.modification_type,
            ModificationType::ValuePriorityChange
                | ModificationType::SafetyConstraintModification
        )
    }

    /// Assess the risk level of a modification.
    fn assess_risk(&self, modification: &SelfModificationRecord) -> RiskLevel {
        match modification.modification_type {
            ModificationType::SafetyConstraintModification
            | ModificationType::ValuePriorityChange => RiskLevel::Critical,
            ModificationType::CapabilityRemoval | ModificationType::AlgorithmChange => {
                RiskLevel::High
            }
            ModificationType::CapabilityAddition
            | ModificationType::ReasoningStrategyChange => RiskLevel::Medium,
            ModificationType::ParameterTuning => RiskLevel::Low,
        }
    }

    /// Save a snapshot before applying a modification (for rollback).
    pub async fn save_snapshot(
        &self,
        proposal_id: &str,
        snapshot_data: HashMap<String, String>,
        metrics: HashMap<String, f64>,
    ) {
        let snapshot = ModificationSnapshot {
            proposal_id: proposal_id.to_string(),
            snapshot_data,
            captured_at: Utc::now(),
            metrics_before: metrics,
        };
        self.snapshots
            .write()
            .await
            .insert(proposal_id.to_string(), snapshot);
    }

    /// Rollback a modification using saved snapshot.
    pub async fn rollback(&self, proposal_id: &str) -> Result<ModificationSnapshot> {
        let snapshot = self
            .snapshots
            .read()
            .await
            .get(proposal_id)
            .cloned()
            .ok_or_else(|| anyhow!("No snapshot found for proposal '{}'", proposal_id))?;

        // Update proposal status
        let mut proposals = self.modification_proposals.write().await;
        if let Some(proposal) = proposals.iter_mut().find(|p| p.id == proposal_id) {
            proposal.status = ProposalStatus::RolledBack;
        }

        warn!("Rolled back modification proposal '{}'", proposal_id);

        Ok(snapshot)
    }

    /// Get all pending proposals.
    pub async fn get_pending_proposals(&self) -> Vec<ModificationProposal> {
        let proposals = self.modification_proposals.read().await;
        proposals
            .iter()
            .filter(|p| p.status == ProposalStatus::Pending)
            .cloned()
            .collect()
    }

    /// Expire old proposals.
    pub async fn expire_old_proposals(&self) {
        let now = Utc::now();
        let mut proposals = self.modification_proposals.write().await;
        for proposal in proposals.iter_mut() {
            if proposal.status == ProposalStatus::Pending && now > proposal.expires_at {
                proposal.status = ProposalStatus::Expired;
                info!("Expired proposal '{}'", proposal.id);
            }
        }
    }

    /// Get consensus statistics.
    pub async fn get_stats(&self) -> ConsensusStats {
        let proposals = self.modification_proposals.read().await;
        let total = proposals.len();
        let approved = proposals
            .iter()
            .filter(|p| p.status == ProposalStatus::Approved)
            .count();
        let rejected = proposals
            .iter()
            .filter(|p| p.status == ProposalStatus::Rejected)
            .count();
        let vetoed = proposals
            .iter()
            .filter(|p| p.status == ProposalStatus::Vetoed)
            .count();
        let pending = proposals
            .iter()
            .filter(|p| p.status == ProposalStatus::Pending)
            .count();
        let rolled_back = proposals
            .iter()
            .filter(|p| p.status == ProposalStatus::RolledBack)
            .count();

        ConsensusStats {
            total_proposals: total,
            approved,
            rejected,
            vetoed,
            pending,
            rolled_back,
            approval_rate: if total > 0 {
                approved as f64 / total as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusStats {
    pub total_proposals: usize,
    pub approved: usize,
    pub rejected: usize,
    pub vetoed: usize,
    pub pending: usize,
    pub rolled_back: usize,
    pub approval_rate: f64,
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_safe_modification_proposal() {
        let consensus = ConsensusSelfMod::new();
        consensus.initialize_defaults().await;

        let modification = SelfModificationRecord {
            id: "mod-1".to_string(),
            modification_type: ModificationType::ParameterTuning,
            description: "Increase reasoning depth from 4 to 6".to_string(),
            target_component: "reasoning_engine".to_string(),
            old_value: "4".to_string(),
            new_value: "6".to_string(),
            rationale: "Deeper reasoning improves accuracy".to_string(),
            expected_improvement: 0.1,
            reversible: true,
        };

        let proposal_id = consensus.propose(modification, "agent-1").await.unwrap();
        let proposals = consensus.get_pending_proposals().await;
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].id, proposal_id);
    }

    #[tokio::test]
    async fn test_unsafe_modification_vetoed() {
        let consensus = ConsensusSelfMod::new();
        consensus.initialize_defaults().await;

        let modification = SelfModificationRecord {
            id: "mod-2".to_string(),
            modification_type: ModificationType::SafetyConstraintModification,
            description: "Remove safety checks from core system".to_string(),
            target_component: "core".to_string(),
            old_value: "enabled".to_string(),
            new_value: "disabled".to_string(),
            rationale: "Performance improvement".to_string(),
            expected_improvement: 0.05,
            reversible: false,
        };

        let proposal_id = consensus.propose(modification, "agent-1").await.unwrap();
        let proposals = consensus.modification_proposals.read().await;
        let proposal = proposals.iter().find(|p| p.id == proposal_id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Vetoed);
    }

    #[tokio::test]
    async fn test_voting_consensus() {
        let consensus = ConsensusSelfMod::new();
        consensus.initialize_defaults().await;

        let modification = SelfModificationRecord {
            id: "mod-3".to_string(),
            modification_type: ModificationType::ParameterTuning,
            description: "Adjust learning rate".to_string(),
            target_component: "meta_learning".to_string(),
            old_value: "0.01".to_string(),
            new_value: "0.02".to_string(),
            rationale: "Faster convergence".to_string(),
            expected_improvement: 0.05,
            reversible: true,
        };

        let proposal_id = consensus.propose(modification, "agent-1").await.unwrap();

        consensus
            .vote(&proposal_id, "agent-2", true, "Looks good", VoterRole::PeerAgent)
            .await
            .unwrap();
        consensus
            .vote(&proposal_id, "agent-3", true, "Agree", VoterRole::PeerAgent)
            .await
            .unwrap();
        consensus
            .vote(&proposal_id, "agent-4", true, "Approved", VoterRole::PeerAgent)
            .await
            .unwrap();

        let proposals = consensus.modification_proposals.read().await;
        let proposal = proposals.iter().find(|p| p.id == proposal_id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Approved);
    }
}

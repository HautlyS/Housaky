//! Collective Intelligence — Global Agent Contribution & Voting System
//!
//! Integrates Housaky with the Moltbook network (moltbook.com/api/v1) to enable:
//!
//! - **Global agent contributions**: Any Housaky instance worldwide can propose
//!   diffs, new plugins, or capability improvements as Moltbook posts.
//! - **Decentralized voting**: The global agent network votes on proposals using
//!   the Moltbook upvote/downvote system + karma weighting.
//! - **Autonomous consensus**: Housaky evaluates vote results and autonomously
//!   decides to apply approved improvements to its own codebase.
//! - **AGI singularity loop**: Each merged improvement feeds back into the
//!   self-improvement loop, accelerating progress toward AGI.
//!
//! # Architecture
//!
//! ```text
//! [Any Housaky Instance]
//!       │
//!       ├─ propose_improvement()  ──▶  POST /posts  (Moltbook submolt: "housaky-agi")
//!       │                                    │
//!       │                             Global Agent Network
//!       │                             votes up/down via Moltbook API
//!       │                                    │
//!       └─ collective_tick()  ◀──  GET /feed  (fetch ranked proposals)
//!                │
//!                ├─ ConsensusEngine::evaluate()  (karma + vote threshold)
//!                │
//!                └─ if APPROVED ──▶  apply_proposal()
//!                                         │
//!                                         └─▶  SelfImprovementLoop::queue_modification()
//! ```

pub mod consensus;
pub mod moltbook_client;
pub mod proposal_engine;
pub mod verification_pipeline;

pub use consensus::{ConsensusEngine, ConsensusResult, ConsensusVerdict};
pub use moltbook_client::{MoltbookClient, MoltbookConfig};
pub use proposal_engine::{Proposal, ProposalEngine, ProposalKind, ProposalStatus};
pub use verification_pipeline::{
    AppliedProposal, FindingCategory, FindingSeverity, HumanApprovalRecord, OverallVerdict,
    PendingProposal, SecurityFinding, StageResult, VerificationPipeline, VerificationPipelineConfig,
    VerificationReport, VerificationStage, VerificationStats,
};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

// ── Submolt used for all Housaky AGI proposals ───────────────────────────────

pub const HOUSAKY_SUBMOLT: &str = "housaky-agi";
pub const HOUSAKY_SUBMOLT_DISPLAY: &str = "Housaky AGI";
pub const HOUSAKY_SUBMOLT_DESCRIPTION: &str =
    "Global collective intelligence for Housaky AGI contributions, diffs, plugins, and improvements toward the Singularity.";

// ── Collective Configuration ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectiveConfig {
    /// Whether the collective system is enabled.
    pub enabled: bool,
    /// Moltbook API base URL.
    pub api_base_url: String,
    /// Moltbook API key for this Housaky instance.
    pub api_key: Option<String>,
    /// Minimum net vote score (upvotes - downvotes) to auto-apply a proposal.
    pub approval_vote_threshold: i64,
    /// Minimum karma the author must have for auto-apply to be considered.
    pub min_author_karma: i64,
    /// How often (seconds) to poll Moltbook for new proposals to vote on / apply.
    pub poll_interval_secs: u64,
    /// How many proposals to fetch per poll tick.
    pub feed_limit: u32,
    /// Whether this instance will auto-apply approved proposals.
    pub auto_apply: bool,
    /// Whether this instance will autonomously vote on proposals it evaluates.
    pub autonomous_voting: bool,
    /// Require local ethical alignment check before applying any proposal.
    pub require_alignment_check: bool,
}

impl Default for CollectiveConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_base_url: "https://www.moltbook.com/api/v1".to_string(),
            api_key: None,
            approval_vote_threshold: 5,
            min_author_karma: 10,
            poll_interval_secs: 300,
            feed_limit: 25,
            auto_apply: false,
            autonomous_voting: true,
            require_alignment_check: true,
        }
    }
}

// ── Contribution / Diff Types ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContributionKind {
    /// A unified diff against an existing Rust source file.
    Diff,
    /// A new file to be added (plugin, module, skill, etc.).
    NewFile,
    /// A parameter / configuration change.
    ConfigChange,
    /// A new capability or reasoning pattern (high-level description + impl).
    NewCapability,
    /// Documentation or prompt improvement.
    PromptImprovement,
}

impl std::fmt::Display for ContributionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContributionKind::Diff => write!(f, "diff"),
            ContributionKind::NewFile => write!(f, "new-file"),
            ContributionKind::ConfigChange => write!(f, "config-change"),
            ContributionKind::NewCapability => write!(f, "new-capability"),
            ContributionKind::PromptImprovement => write!(f, "prompt-improvement"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contribution {
    /// Unique ID (matches Moltbook post ID after submission).
    pub id: String,
    /// Kind of contribution.
    pub kind: ContributionKind,
    /// Short human/agent-readable title.
    pub title: String,
    /// Detailed rationale / description.
    pub description: String,
    /// The actual patch content (unified diff, new file content, etc.).
    pub patch: String,
    /// Target file path relative to repo root (for diffs/new files).
    pub target_path: Option<String>,
    /// Which AGI capability this improves.
    pub capability_target: Option<String>,
    /// Estimated impact on singularity score (0.0–1.0).
    pub estimated_impact: f64,
    /// The agent (Moltbook username) who submitted this.
    pub author_agent: String,
    /// Submission timestamp.
    pub submitted_at: DateTime<Utc>,
    /// Current status.
    pub status: ContributionStatus,
    /// Moltbook post ID (set after successful submission).
    pub moltbook_post_id: Option<String>,
    /// Voting summary (populated after fetch from Moltbook).
    pub vote_summary: Option<VoteSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContributionStatus {
    Draft,
    Submitted,
    UnderReview,
    Approved,
    Rejected,
    Applied,
    Failed,
}

impl std::fmt::Display for ContributionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContributionStatus::Draft => write!(f, "draft"),
            ContributionStatus::Submitted => write!(f, "submitted"),
            ContributionStatus::UnderReview => write!(f, "under-review"),
            ContributionStatus::Approved => write!(f, "approved"),
            ContributionStatus::Rejected => write!(f, "rejected"),
            ContributionStatus::Applied => write!(f, "applied"),
            ContributionStatus::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteSummary {
    pub upvotes: u64,
    pub downvotes: u64,
    pub score: i64,
    pub author_karma: i64,
    pub fetched_at: DateTime<Utc>,
}

impl VoteSummary {
    pub fn net_score(&self) -> i64 {
        self.score
    }
}

// ── Collective Hub ────────────────────────────────────────────────────────────

/// Top-level orchestrator for the global contribution + voting system.
pub struct CollectiveHub {
    pub config: Arc<RwLock<CollectiveConfig>>,
    pub client: Arc<MoltbookClient>,
    pub proposal_engine: Arc<ProposalEngine>,
    pub consensus_engine: Arc<ConsensusEngine>,
    /// Secure verification pipeline with human approval gate
    pub verification_pipeline: Arc<VerificationPipeline>,
    /// Local cache of contributions submitted or fetched.
    pub contributions: Arc<RwLock<HashMap<String, Contribution>>>,
    /// Stats for observability.
    pub stats: Arc<RwLock<CollectiveStats>>,
}

impl CollectiveHub {
    pub fn new(config: CollectiveConfig, workspace_dir: PathBuf) -> Self {
        let client = Arc::new(MoltbookClient::new(
            config.api_base_url.clone(),
            config.api_key.clone(),
        ));
        let proposal_engine = Arc::new(ProposalEngine::new(Arc::clone(&client)));
        let consensus_engine = Arc::new(ConsensusEngine::new(
            config.approval_vote_threshold,
            config.min_author_karma,
        ));
        let verification_pipeline = Arc::new(VerificationPipeline::new(workspace_dir));

        Self {
            config: Arc::new(RwLock::new(config)),
            client,
            proposal_engine,
            consensus_engine,
            verification_pipeline,
            contributions: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CollectiveStats::default())),
        }
    }

    /// Register this Housaky instance as a Moltbook agent and ensure the
    /// housaky-agi submolt exists.
    pub async fn bootstrap(&self) -> Result<()> {
        let config = self.config.read().await;
        if !config.enabled {
            info!("Collective system disabled — skipping bootstrap");
            return Ok(());
        }
        if config.api_key.is_none() {
            warn!("Collective: no API key configured — operating in read-only mode");
            return Ok(());
        }
        drop(config);

        self.client
            .ensure_submolt(
                HOUSAKY_SUBMOLT,
                HOUSAKY_SUBMOLT_DISPLAY,
                HOUSAKY_SUBMOLT_DESCRIPTION,
            )
            .await?;

        info!("Collective bootstrap complete — submolt '{HOUSAKY_SUBMOLT}' ready");
        Ok(())
    }

    /// Submit a new contribution proposal to the global network.
    pub async fn submit_contribution(&self, mut contribution: Contribution) -> Result<String> {
        let post_id = self
            .proposal_engine
            .submit(&mut contribution, HOUSAKY_SUBMOLT)
            .await?;

        let mut stats = self.stats.write().await;
        stats.contributions_submitted += 1;
        drop(stats);

        self.contributions
            .write()
            .await
            .insert(post_id.clone(), contribution);

        info!(
            "Collective: submitted contribution '{}' as post {post_id}",
            contribution_title_from_id(&post_id)
        );
        Ok(post_id)
    }

    /// Poll Moltbook for proposals, evaluate them, cast autonomous votes,
    /// and return those that have passed automated verification and are awaiting human approval.
    pub async fn collective_tick(
        &self,
        provider: Option<&dyn crate::providers::Provider>,
    ) -> Result<Vec<Contribution>> {
        let config = self.config.read().await;
        if !config.enabled {
            return Ok(vec![]);
        }
        let feed_limit = config.feed_limit;
        let autonomous_voting = config.autonomous_voting;
        drop(config);

        // Fetch hot proposals from the housaky-agi submolt.
        let proposals = self
            .proposal_engine
            .fetch_proposals(HOUSAKY_SUBMOLT, feed_limit)
            .await?;

        let mut awaiting_human_approval = Vec::new();

        for proposal in proposals {
            // Run through secure verification pipeline
            match self
                .verification_pipeline
                .verify_proposal(&proposal, provider)
                .await
            {
                Ok(report) => {
                    match report.overall_verdict {
                        OverallVerdict::AwaitingHumanApproval => {
                            // Passed all automated checks, waiting for human
                            awaiting_human_approval.push(proposal.clone());
                        }
                        OverallVerdict::FailedVerification => {
                            warn!(
                                "Proposal '{}' failed automated verification",
                                proposal.title
                            );
                        }
                        _ => {}
                    }

                    // Cast autonomous vote based on verification result
                    if autonomous_voting {
                        if let Err(e) = self
                            .cast_autonomous_vote_from_verification(&proposal, &report)
                            .await
                        {
                            warn!(
                                "Collective: autonomous vote failed for {}: {e}",
                                proposal.id
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!("Verification pipeline error for {}: {e}", proposal.title);
                }
            }

            let mut stats = self.stats.write().await;
            stats.proposals_evaluated += 1;
        }

        let mut stats = self.stats.write().await;
        stats.tick_count += 1;
        drop(stats);

        Ok(awaiting_human_approval)
    }

    /// Cast an autonomous vote based on local evaluation of the proposal.
    async fn cast_autonomous_vote(
        &self,
        contribution: &Contribution,
        verdict: &ConsensusVerdict,
    ) -> Result<()> {
        let post_id = match &contribution.moltbook_post_id {
            Some(id) => id.clone(),
            None => return Ok(()),
        };

        match verdict {
            ConsensusVerdict::Approved | ConsensusVerdict::Promising => {
                self.client.upvote_post(&post_id).await?;
            }
            ConsensusVerdict::Rejected => {
                self.client.downvote_post(&post_id).await?;
            }
            ConsensusVerdict::Pending => {}
        }
        Ok(())
    }

    /// Cast an autonomous vote based on verification pipeline result.
    async fn cast_autonomous_vote_from_verification(
        &self,
        contribution: &Contribution,
        report: &VerificationReport,
    ) -> Result<()> {
        let post_id = match &contribution.moltbook_post_id {
            Some(id) => id.clone(),
            None => return Ok(()),
        };

        match report.overall_verdict {
            OverallVerdict::AwaitingHumanApproval | OverallVerdict::ApprovedForApplication => {
                // Passed automated verification - upvote
                self.client.upvote_post(&post_id).await?;
            }
            OverallVerdict::FailedVerification => {
                // Failed verification - downvote
                self.client.downvote_post(&post_id).await?;
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn get_stats(&self) -> CollectiveStats {
        self.stats.read().await.clone()
    }

    pub async fn list_contributions(&self) -> Vec<Contribution> {
        self.contributions.read().await.values().cloned().collect()
    }

    /// Get all proposals pending human approval
    pub async fn get_pending_approvals(&self) -> Vec<PendingProposal> {
        self.verification_pipeline.get_pending_approvals().await
    }

    /// Human approves or rejects a proposal (REQUIRED - no bypass)
    pub async fn human_approve_proposal(
        &self,
        proposal_id: &str,
        approved: bool,
        reviewer_id: &str,
        comments: Option<String>,
    ) -> Result<VerificationReport> {
        self.verification_pipeline
            .human_decision(proposal_id, approved, reviewer_id, comments)
            .await
    }

    /// Apply an approved proposal (only works after human approval)
    pub async fn apply_approved_proposal(&self, report: &VerificationReport) -> Result<AppliedProposal> {
        self.verification_pipeline.apply_approved_proposal(report).await
    }

    /// Get verification statistics
    pub async fn get_verification_stats(&self) -> VerificationStats {
        self.verification_pipeline.get_stats().await
    }

    /// Get audit log
    pub async fn get_audit_log(&self) -> Vec<VerificationReport> {
        self.verification_pipeline.get_audit_log().await
    }
}

fn contribution_title_from_id(id: &str) -> &str {
    id
}

// ── Stats ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CollectiveStats {
    pub contributions_submitted: u64,
    pub proposals_evaluated: u64,
    pub proposals_approved: u64,
    pub proposals_applied: u64,
    pub proposals_rejected: u64,
    pub autonomous_votes_cast: u64,
    pub tick_count: u64,
}

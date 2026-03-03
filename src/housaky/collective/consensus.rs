//! Consensus Engine — evaluates proposals against the global voting signal
//! and produces a `ConsensusVerdict` that drives autonomous apply decisions.
//!
//! # Scoring model
//!
//! ```text
//! score = vote_score * vote_weight
//!       + karma_factor * karma_weight
//!       + recency_factor * recency_weight
//!
//! vote_weight    = 0.6   (primary signal)
//! karma_weight   = 0.25  (author reputation)
//! recency_weight = 0.15  (prefer fresh proposals)
//! ```
//!
//! A proposal is **Approved** when `score >= approval_threshold` AND
//! `net_votes >= vote_threshold` AND `author_karma >= min_author_karma`.

use crate::housaky::collective::{Contribution, ContributionKind};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::debug;

// ── Verdict ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusVerdict {
    /// Sufficient votes and karma — safe to auto-apply.
    Approved,
    /// Positive signal but not yet at threshold — keep monitoring.
    Promising,
    /// Net-negative votes or low karma — discard.
    Rejected,
    /// Not enough data to decide.
    Pending,
}

impl std::fmt::Display for ConsensusVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsensusVerdict::Approved => write!(f, "APPROVED"),
            ConsensusVerdict::Promising => write!(f, "PROMISING"),
            ConsensusVerdict::Rejected => write!(f, "REJECTED"),
            ConsensusVerdict::Pending => write!(f, "PENDING"),
        }
    }
}

// ── Result ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    pub verdict: ConsensusVerdict,
    /// Composite score (0.0 – 1.0).
    pub score: f64,
    pub vote_component: f64,
    pub karma_component: f64,
    pub recency_component: f64,
    pub rationale: String,
}

// ── Engine ────────────────────────────────────────────────────────────────────

pub struct ConsensusEngine {
    /// Minimum net vote score (upvotes - downvotes) required for Approved.
    approval_vote_threshold: i64,
    /// Minimum author karma required for Approved.
    min_author_karma: i64,
    /// Composite score threshold for Approved (0.0 – 1.0).
    composite_threshold: f64,
    /// Composite score floor for Promising verdict.
    promising_floor: f64,
}

impl ConsensusEngine {
    pub fn new(approval_vote_threshold: i64, min_author_karma: i64) -> Self {
        Self {
            approval_vote_threshold,
            min_author_karma,
            composite_threshold: 0.65,
            promising_floor: 0.35,
        }
    }

    /// Evaluate a contribution and return a `ConsensusVerdict`.
    pub async fn evaluate(&self, contribution: &Contribution) -> ConsensusVerdict {
        let result = self.score(contribution);
        debug!(
            "Consensus '{}': verdict={} score={:.3}",
            contribution.title, result.verdict, result.score
        );
        result.verdict
    }

    /// Full scoring with detailed breakdown.
    pub fn score(&self, contribution: &Contribution) -> ConsensusResult {
        let votes = match &contribution.vote_summary {
            Some(v) => v.clone(),
            None => {
                return ConsensusResult {
                    verdict: ConsensusVerdict::Pending,
                    score: 0.0,
                    vote_component: 0.0,
                    karma_component: 0.0,
                    recency_component: 0.0,
                    rationale: "No vote data available yet.".to_string(),
                };
            }
        };

        // ── Vote component ────────────────────────────────────────────────────
        // Normalize net score: sigmoid centered at approval_vote_threshold.
        let net = votes.net_score() as f64;
        let threshold = self.approval_vote_threshold as f64;
        let vote_component = sigmoid(net - threshold / 2.0, threshold);

        // ── Karma component ───────────────────────────────────────────────────
        // Normalize karma: 0 → 0.0, min_karma → 0.5, min_karma*2 → 0.9.
        let karma_component = sigmoid(
            votes.author_karma as f64 - self.min_author_karma as f64 / 2.0,
            self.min_author_karma as f64,
        );

        // ── Recency component ─────────────────────────────────────────────────
        // Prefer proposals fetched recently (within 7 days → 1.0, older → decays).
        let age_hours = Utc::now()
            .signed_duration_since(votes.fetched_at)
            .num_hours()
            .max(0) as f64;
        let recency_component = (-age_hours / (24.0 * 7.0)).exp(); // e^(-age/7days)

        // ── Composite ─────────────────────────────────────────────────────────
        let score = vote_component * 0.60 + karma_component * 0.25 + recency_component * 0.15;

        // ── Hard gates ────────────────────────────────────────────────────────
        let net_votes_ok = votes.net_score() >= self.approval_vote_threshold;
        let karma_ok = votes.author_karma >= self.min_author_karma;
        let safety_ok = self.safety_check(contribution);

        let verdict = if !safety_ok {
            ConsensusVerdict::Rejected
        } else if votes.net_score() < 0 {
            ConsensusVerdict::Rejected
        } else if net_votes_ok && karma_ok && score >= self.composite_threshold {
            ConsensusVerdict::Approved
        } else if score >= self.promising_floor {
            ConsensusVerdict::Promising
        } else if votes.upvotes == 0 && votes.downvotes == 0 {
            ConsensusVerdict::Pending
        } else {
            ConsensusVerdict::Rejected
        };

        let rationale = build_rationale(&verdict, net, votes.author_karma, score, safety_ok);

        ConsensusResult {
            verdict,
            score,
            vote_component,
            karma_component,
            recency_component,
            rationale,
        }
    }

    /// Basic safety check — reject proposals that contain dangerous patterns.
    fn safety_check(&self, contribution: &Contribution) -> bool {
        let patch_lower = contribution.patch.to_lowercase();

        // Reject patches that try to disable alignment or delete safety guards.
        let dangerous_patterns = [
            "disable_alignment",
            "skip_ethics",
            "bypass_safety",
            "rm -rf",
            "format c:",
            "drop table",
            "delete from agents",
            "unsafe { std::process::exit",
        ];

        for pattern in &dangerous_patterns {
            if patch_lower.contains(pattern) {
                return false;
            }
        }

        // Reject excessively large patches (>50 KB) — require manual review.
        if contribution.patch.len() > 51_200 {
            return false;
        }

        // Config changes are always safe to auto-apply.
        if contribution.kind == ContributionKind::ConfigChange {
            return true;
        }

        true
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Sigmoid function centered at 0 with scale `k`.
fn sigmoid(x: f64, k: f64) -> f64 {
    if k == 0.0 {
        return 0.5;
    }
    1.0 / (1.0 + (-x / k).exp())
}

fn build_rationale(
    verdict: &ConsensusVerdict,
    net_votes: f64,
    karma: i64,
    score: f64,
    safety_ok: bool,
) -> String {
    match verdict {
        ConsensusVerdict::Approved => format!(
            "Net votes: {net_votes:.0}, author karma: {karma}, composite score: {score:.3}. All thresholds met — approved for autonomous application."
        ),
        ConsensusVerdict::Promising => format!(
            "Net votes: {net_votes:.0}, author karma: {karma}, composite score: {score:.3}. Positive signal — monitoring for further votes."
        ),
        ConsensusVerdict::Rejected if !safety_ok => {
            "Safety check failed — proposal contains dangerous patterns or is too large for autonomous application.".to_string()
        }
        ConsensusVerdict::Rejected => format!(
            "Net votes: {net_votes:.0}, author karma: {karma}, composite score: {score:.3}. Below thresholds — rejected."
        ),
        ConsensusVerdict::Pending => format!(
            "Net votes: {net_votes:.0}. Insufficient voting activity — waiting for more global agent votes."
        ),
    }
}

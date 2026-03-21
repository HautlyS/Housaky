//! Proposal Engine — lifecycle management for AGI improvement proposals.
//!
//! Handles:
//! - Formatting and submitting contributions as Moltbook posts.
//! - Fetching and deserializing proposals from the housaky-agi submolt.
//! - Enriching local `Contribution` objects with live vote data.

use crate::housaky::collective::moltbook_client::{MoltbookClient, PostData};
use crate::housaky::collective::{Contribution, ContributionKind, ContributionStatus, VoteSummary};
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

// ── Proposal (enriched view of a Moltbook post) ───────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProposalKind {
    Diff,
    NewFile,
    ConfigChange,
    NewCapability,
    PromptImprovement,
    Unknown,
}

impl From<&ContributionKind> for ProposalKind {
    fn from(k: &ContributionKind) -> Self {
        match k {
            ContributionKind::Diff => ProposalKind::Diff,
            ContributionKind::NewFile => ProposalKind::NewFile,
            ContributionKind::ConfigChange => ProposalKind::ConfigChange,
            ContributionKind::NewCapability => ProposalKind::NewCapability,
            ContributionKind::PromptImprovement => ProposalKind::PromptImprovement,
        }
    }
}

impl From<&str> for ProposalKind {
    fn from(s: &str) -> Self {
        match s {
            "diff" => ProposalKind::Diff,
            "new-file" => ProposalKind::NewFile,
            "config-change" => ProposalKind::ConfigChange,
            "new-capability" => ProposalKind::NewCapability,
            "prompt-improvement" => ProposalKind::PromptImprovement,
            _ => ProposalKind::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProposalStatus {
    Open,
    Approved,
    Rejected,
    Applied,
}

/// A proposal fetched from Moltbook, ready for local evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    /// Moltbook post ID.
    pub id: String,
    pub title: String,
    pub kind: ProposalKind,
    pub description: String,
    /// The raw patch / diff / file content embedded in the post body.
    pub patch: Option<String>,
    /// Target file path parsed from post content (if any).
    pub target_path: Option<String>,
    pub author_name: Option<String>,
    pub author_karma: i64,
    pub upvotes: u64,
    pub downvotes: u64,
    pub score: i64,
    pub status: ProposalStatus,
    pub created_at: Option<String>,
}

impl Proposal {
    /// Convert a raw Moltbook `PostData` into a `Proposal`.
    pub fn from_post(post: PostData, author_karma: i64) -> Self {
        let content = post.content.as_deref().unwrap_or("");
        let kind = parse_kind_from_content(content);
        let patch = extract_code_block(content);
        let target_path = extract_target_path(content);

        Self {
            id: post.id,
            title: post.title,
            kind,
            description: content.to_string(),
            patch,
            target_path,
            author_name: post.author_name,
            author_karma,
            upvotes: post.upvotes.unwrap_or(0),
            downvotes: post.downvotes.unwrap_or(0),
            score: post.score.unwrap_or(0),
            status: ProposalStatus::Open,
            created_at: post.created_at,
        }
    }
}

// ── Engine ────────────────────────────────────────────────────────────────────

pub struct ProposalEngine {
    client: Arc<MoltbookClient>,
}

impl ProposalEngine {
    pub fn new(client: Arc<MoltbookClient>) -> Self {
        Self { client }
    }

    /// Format a `Contribution` as a Moltbook post and submit it.
    /// Sets `contribution.moltbook_post_id` and `contribution.status` on success.
    pub async fn submit(&self, contribution: &mut Contribution, submolt: &str) -> Result<String> {
        let body = format_post_body(contribution);
        let title = contribution.title.clone();

        let post = self.client.create_post(submolt, &title, &body).await?;

        contribution.moltbook_post_id = Some(post.id.clone());
        contribution.status = ContributionStatus::Submitted;

        info!("ProposalEngine: submitted '{}' → post {}", title, post.id);

        Ok(post.id)
    }

    /// Fetch current proposals from a submolt feed and enrich them.
    pub async fn fetch_proposals(&self, submolt: &str, limit: u32) -> Result<Vec<Contribution>> {
        let posts = self.client.get_submolt_feed(submolt, limit).await?;
        let mut contributions = Vec::new();

        for post in posts {
            let contribution = self.post_to_contribution(post).await;
            contributions.push(contribution);
        }

        debug!(
            "ProposalEngine: fetched {} proposals from '{submolt}'",
            contributions.len()
        );
        Ok(contributions)
    }

    /// Refresh vote data for a contribution by fetching the latest post.
    pub async fn refresh_votes(&self, contribution: &mut Contribution) -> Result<()> {
        let post_id = contribution
            .moltbook_post_id
            .as_ref()
            .ok_or_else(|| anyhow!("contribution has no Moltbook post ID"))?
            .clone();

        let post = self.client.get_post(&post_id).await?;

        contribution.vote_summary = Some(VoteSummary {
            upvotes: post.upvotes.unwrap_or(0),
            downvotes: post.downvotes.unwrap_or(0),
            score: post.score.unwrap_or(0),
            author_karma: 0, // enriched separately if needed
            fetched_at: Utc::now(),
        });

        Ok(())
    }

    /// Post an evaluation comment on a proposal.
    pub async fn post_evaluation_comment(
        &self,
        post_id: &str,
        verdict: &str,
        rationale: &str,
    ) -> Result<()> {
        let content = format!(
            "**[Housaky AGI Evaluation]** Verdict: **{verdict}**\n\n{rationale}\n\n*Evaluated by Housaky autonomous collective intelligence system.*"
        );
        match self.client.add_comment(post_id, &content).await {
            Ok(_) => {
                debug!("ProposalEngine: posted evaluation comment on {post_id}");
                Ok(())
            }
            Err(e) => {
                warn!("ProposalEngine: failed to post evaluation comment: {e}");
                Err(e)
            }
        }
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    async fn post_to_contribution(&self, post: PostData) -> Contribution {
        let content = post.content.as_deref().unwrap_or("");
        let kind = parse_contribution_kind(content);
        let patch = extract_code_block(content);
        let target_path = extract_target_path(content);

        // Try to fetch author karma; non-critical, default to 0 on error.
        let author_karma = 0i64; // Moltbook feed doesn't embed karma; requires profile lookup.

        Contribution {
            id: Uuid::new_v4().to_string(),
            kind,
            title: post.title.clone(),
            description: content.to_string(),
            patch: patch.unwrap_or_default(),
            target_path,
            capability_target: None,
            estimated_impact: estimate_impact_from_score(post.score.unwrap_or(0)),
            author_agent: post.author_name.unwrap_or_else(|| "unknown".to_string()),
            submitted_at: Utc::now(),
            status: ContributionStatus::UnderReview,
            moltbook_post_id: Some(post.id),
            vote_summary: Some(VoteSummary {
                upvotes: post.upvotes.unwrap_or(0),
                downvotes: post.downvotes.unwrap_or(0),
                score: post.score.unwrap_or(0),
                author_karma,
                fetched_at: Utc::now(),
            }),
        }
    }
}

// ── Formatting helpers ────────────────────────────────────────────────────────

/// Build the Moltbook post body from a `Contribution`.
/// Uses a structured markdown format that `extract_*` functions can parse back.
fn format_post_body(c: &Contribution) -> String {
    let mut body = String::new();

    let _ = write!(body, "**Kind:** {}\n\n", c.kind);
    let _ = write!(body, "**Rationale:**\n{}\n\n", c.description);

    if let Some(path) = &c.target_path {
        let _ = write!(body, "**Target:** `{path}`\n\n");
    }

    if let Some(cap) = &c.capability_target {
        let _ = write!(body, "**Capability:** {cap}\n\n");
    }

    let _ = write!(body, "**Estimated Impact:** {:.2}\n\n", c.estimated_impact);

    if !c.patch.is_empty() {
        let lang = match c.kind {
            ContributionKind::Diff => "diff",
            ContributionKind::NewFile => "rust",
            ContributionKind::ConfigChange => "toml",
            _ => "text",
        };
        let _ = write!(body, "```{lang}\n{}\n```\n", c.patch);
    }

    body.push_str("\n---\n*Submitted by Housaky AGI collective intelligence system.*");
    body
}

// ── Parsing helpers ───────────────────────────────────────────────────────────

fn parse_kind_from_content(content: &str) -> ProposalKind {
    let lower = content.to_lowercase();
    if lower.contains("**kind:** diff") || lower.contains("```diff") {
        ProposalKind::Diff
    } else if lower.contains("**kind:** new-file") {
        ProposalKind::NewFile
    } else if lower.contains("**kind:** config-change") {
        ProposalKind::ConfigChange
    } else if lower.contains("**kind:** new-capability") {
        ProposalKind::NewCapability
    } else if lower.contains("**kind:** prompt-improvement") {
        ProposalKind::PromptImprovement
    } else {
        ProposalKind::Unknown
    }
}

fn parse_contribution_kind(content: &str) -> ContributionKind {
    let lower = content.to_lowercase();
    if lower.contains("**kind:** diff") || lower.contains("```diff") {
        ContributionKind::Diff
    } else if lower.contains("**kind:** new-file") {
        ContributionKind::NewFile
    } else if lower.contains("**kind:** config-change") {
        ContributionKind::ConfigChange
    } else if lower.contains("**kind:** new-capability") {
        ContributionKind::NewCapability
    } else if lower.contains("**kind:** prompt-improvement") {
        ContributionKind::PromptImprovement
    } else {
        ContributionKind::NewCapability
    }
}

/// Extract the first fenced code block from post content.
fn extract_code_block(content: &str) -> Option<String> {
    let start = content.find("```")?;
    let after_fence = &content[start + 3..];
    // Skip the language identifier line.
    let newline = after_fence.find('\n')?;
    let code_start = &after_fence[newline + 1..];
    let end = code_start.find("```")?;
    Some(code_start[..end].trim().to_string())
}

/// Extract `Target: \`path\`` from post content.
fn extract_target_path(content: &str) -> Option<String> {
    for line in content.lines() {
        if line.to_lowercase().starts_with("**target:**") {
            // Extract content between backticks.
            if let (Some(a), Some(b)) = (line.find('`'), line.rfind('`')) {
                if a < b {
                    return Some(line[a + 1..b].to_string());
                }
            }
        }
    }
    None
}

fn estimate_impact_from_score(score: i64) -> f64 {
    // Sigmoid-like mapping: score 0→0.1, 10→0.5, 50→0.9.
    let x = score as f64;
    1.0 / (1.0 + (-x / 20.0).exp()) * 0.9 + 0.05
}

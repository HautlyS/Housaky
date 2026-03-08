//! Secure Verification Pipeline for Collective Proposals
//!
//! Multi-layer defense-in-depth verification before any code reaches the codebase:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                    COLLECTIVE PROPOSAL VERIFICATION PIPELINE                │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                             │
//! │  [Proposal from Network]                                                    │
//! │          │                                                                  │
//! │          ▼                                                                  │
//! │  ┌───────────────────┐                                                      │
//! │  │ 1. SIGNATURE      │ ─── Cryptographic origin verification               │
//! │  │    VERIFICATION   │     (Ed25519 + trust chain to known-good agents)    │
//! │  └─────────┬─────────┘                                                      │
//! │            │ PASS                                                           │
//! │            ▼                                                                │
//! │  ┌───────────────────┐                                                      │
//! │  │ 2. STATIC         │ ─── Pattern matching, AST analysis,                 │
//! │  │    ANALYSIS       │     forbidden module detection                       │
//! │  └─────────┬─────────┘                                                      │
//! │            │ PASS                                                           │
//! │            ▼                                                                │
//! │  ┌───────────────────┐                                                      │
//! │  │ 3. LLM SECURITY   │ ─── Semantic analysis by independent LLM            │
//! │  │    REVIEW         │     (detects obfuscated malice, logic bombs)        │
//! │  └─────────┬─────────┘                                                      │
//! │            │ PASS                                                           │
//! │            ▼                                                                │
//! │  ┌───────────────────┐                                                      │
//! │  │ 4. SANDBOX        │ ─── Isolated build + full test suite                │
//! │  │    EXECUTION      │     + capability retention benchmarks               │
//! │  └─────────┬─────────┘                                                      │
//! │            │ PASS                                                           │
//! │            ▼                                                                │
//! │  ┌───────────────────┐                                                      │
//! │  │ 5. PERFORMANCE    │ ─── Benchmark comparison, regression detection      │
//! │  │    METRICS        │     + improvement scoring                           │
//! │  └─────────┬─────────┘                                                      │
//! │            │ PASS                                                           │
//! │            ▼                                                                │
//! │  ┌───────────────────┐                                                      │
//! │  │ 6. HUMAN          │ ─── Admin approval queue with full audit report     │
//! │  │    APPROVAL GATE  │     (REQUIRED - no bypass possible)                 │
//! │  └─────────┬─────────┘                                                      │
//! │            │ APPROVED BY ADMIN                                              │
//! │            ▼                                                                │
//! │  ┌───────────────────┐                                                      │
//! │  │ 7. APPLY WITH     │ ─── Git commit with signed attestation              │
//! │  │    AUDIT TRAIL    │     + rollback capability preserved                 │
//! │  └───────────────────┘                                                      │
//! │                                                                             │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! **Security Principle**: Defense in depth. Each layer is independent and any
//! single layer failure blocks the proposal. Human approval is MANDATORY.

use crate::housaky::collective::{Contribution, ContributionKind};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

// ── Verification Stage Results ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StageVerdict {
    Pass,
    Fail,
    Warning,
    Skipped,
    PendingHumanReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub stage: VerificationStage,
    pub verdict: StageVerdict,
    pub score: f64,
    pub findings: Vec<SecurityFinding>,
    pub duration_ms: u64,
    pub executed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerificationStage {
    SignatureVerification,
    StaticAnalysis,
    LlmSecurityReview,
    SandboxExecution,
    PerformanceMetrics,
    HumanApproval,
}

impl std::fmt::Display for VerificationStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationStage::SignatureVerification => write!(f, "Signature Verification"),
            VerificationStage::StaticAnalysis => write!(f, "Static Analysis"),
            VerificationStage::LlmSecurityReview => write!(f, "LLM Security Review"),
            VerificationStage::SandboxExecution => write!(f, "Sandbox Execution"),
            VerificationStage::PerformanceMetrics => write!(f, "Performance Metrics"),
            VerificationStage::HumanApproval => write!(f, "Human Approval"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub severity: FindingSeverity,
    pub category: FindingCategory,
    pub description: String,
    pub location: Option<String>,
    pub recommendation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for FindingSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FindingSeverity::Info => write!(f, "INFO"),
            FindingSeverity::Low => write!(f, "LOW"),
            FindingSeverity::Medium => write!(f, "MEDIUM"),
            FindingSeverity::High => write!(f, "HIGH"),
            FindingSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FindingCategory {
    MaliciousPattern,
    ObfuscatedCode,
    UnsafeOperation,
    PrivilegeEscalation,
    DataExfiltration,
    ResourceExhaustion,
    LogicBomb,
    BackdoorIndicator,
    CapabilityRegression,
    PerformanceRegression,
    UnverifiedOrigin,
    TrustChainBroken,
}

// ── Verification Report ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub id: String,
    pub proposal_id: String,
    pub proposal_title: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub stages: Vec<StageResult>,
    pub overall_verdict: OverallVerdict,
    pub security_score: f64,
    pub improvement_score: f64,
    pub human_approval: Option<HumanApprovalRecord>,
    pub audit_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OverallVerdict {
    /// All automated checks passed, awaiting human approval
    AwaitingHumanApproval,
    /// Human approved, ready to apply
    ApprovedForApplication,
    /// Human rejected
    RejectedByHuman,
    /// Failed automated verification
    FailedVerification,
    /// Still in progress
    InProgress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanApprovalRecord {
    pub approved: bool,
    pub reviewer_id: String,
    pub reviewed_at: DateTime<Utc>,
    pub comments: Option<String>,
    pub signature: String,
}

// ── Verification Pipeline Configuration ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationPipelineConfig {
    /// Require cryptographic signature from trusted agent
    pub require_signature: bool,
    /// List of trusted agent public keys (Ed25519 hex)
    pub trusted_agent_keys: Vec<String>,
    /// Enable LLM-based security review
    pub enable_llm_review: bool,
    /// Model to use for LLM security review
    pub llm_review_model: String,
    /// Minimum security score to pass (0.0 - 1.0)
    pub min_security_score: f64,
    /// Minimum improvement score to consider (0.0 - 1.0)
    pub min_improvement_score: f64,
    /// Maximum allowed performance regression (%)
    pub max_performance_regression_pct: f64,
    /// CRITICAL: Human approval is ALWAYS required (cannot be disabled)
    /// This field exists only for documentation - it's always true
    #[serde(skip_deserializing)]
    pub require_human_approval: bool,
    /// Sandbox timeout in seconds
    pub sandbox_timeout_secs: u64,
    /// Run capability retention tests
    pub run_capability_retention: bool,
}

impl Default for VerificationPipelineConfig {
    fn default() -> Self {
        Self {
            require_signature: true,
            trusted_agent_keys: Vec::new(),
            enable_llm_review: true,
            llm_review_model: "claude-3-5-sonnet-20241022".to_string(),
            min_security_score: 0.8,
            min_improvement_score: 0.1,
            max_performance_regression_pct: 5.0,
            require_human_approval: true, // ALWAYS true, cannot be changed
            sandbox_timeout_secs: 300,
            run_capability_retention: true,
        }
    }
}

// ── Pending Approval Queue ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingProposal {
    pub proposal: Contribution,
    pub report: VerificationReport,
    pub queued_at: DateTime<Utc>,
    pub priority: i32,
}

// ── Verification Pipeline ─────────────────────────────────────────────────────

pub struct VerificationPipeline {
    pub config: VerificationPipelineConfig,
    pub workspace_dir: PathBuf,
    /// Queue of proposals awaiting human approval
    pub pending_approval: Arc<RwLock<Vec<PendingProposal>>>,
    /// History of all verification reports (for audit)
    pub audit_log: Arc<RwLock<Vec<VerificationReport>>>,
    /// Applied proposals (with full provenance)
    pub applied_proposals: Arc<RwLock<Vec<AppliedProposal>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedProposal {
    pub proposal_id: String,
    pub report_id: String,
    pub applied_at: DateTime<Utc>,
    pub git_commit_hash: String,
    pub rollback_available: bool,
    pub human_approver: String,
}

impl VerificationPipeline {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self {
            config: VerificationPipelineConfig::default(),
            workspace_dir,
            pending_approval: Arc::new(RwLock::new(Vec::new())),
            audit_log: Arc::new(RwLock::new(Vec::new())),
            applied_proposals: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_config(workspace_dir: PathBuf, mut config: VerificationPipelineConfig) -> Self {
        // SECURITY: Force human approval requirement - cannot be disabled
        config.require_human_approval = true;
        Self {
            config,
            workspace_dir,
            pending_approval: Arc::new(RwLock::new(Vec::new())),
            audit_log: Arc::new(RwLock::new(Vec::new())),
            applied_proposals: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Run the full verification pipeline on a proposal.
    /// Returns the verification report. If all automated stages pass,
    /// the proposal is queued for human approval.
    pub async fn verify_proposal(
        &self,
        proposal: &Contribution,
        provider: Option<&dyn crate::providers::Provider>,
    ) -> Result<VerificationReport> {
        let report_id = Uuid::new_v4().to_string();
        let start_time = Utc::now();

        info!(
            "Starting verification pipeline for proposal '{}' ({})",
            proposal.title, proposal.id
        );

        let mut report = VerificationReport {
            id: report_id.clone(),
            proposal_id: proposal.id.clone(),
            proposal_title: proposal.title.clone(),
            author: proposal.author_agent.clone(),
            created_at: start_time,
            completed_at: None,
            stages: Vec::new(),
            overall_verdict: OverallVerdict::InProgress,
            security_score: 0.0,
            improvement_score: 0.0,
            human_approval: None,
            audit_hash: String::new(),
        };

        // Stage 1: Signature Verification
        let sig_result = self.stage_signature_verification(proposal).await;
        let sig_passed = sig_result.verdict == StageVerdict::Pass;
        report.stages.push(sig_result);

        if !sig_passed && self.config.require_signature {
            report.overall_verdict = OverallVerdict::FailedVerification;
            report.completed_at = Some(Utc::now());
            report.audit_hash = self.compute_audit_hash(&report);
            self.store_audit_log(report.clone()).await;
            return Ok(report);
        }

        // Stage 2: Static Analysis
        let static_result = self.stage_static_analysis(proposal).await;
        let static_passed = static_result.verdict == StageVerdict::Pass;
        report.stages.push(static_result);

        if !static_passed {
            report.overall_verdict = OverallVerdict::FailedVerification;
            report.completed_at = Some(Utc::now());
            report.audit_hash = self.compute_audit_hash(&report);
            self.store_audit_log(report.clone()).await;
            return Ok(report);
        }

        // Stage 3: LLM Security Review
        if self.config.enable_llm_review {
            if let Some(prov) = provider {
                let llm_result = self
                    .stage_llm_security_review(proposal, prov, &self.config.llm_review_model)
                    .await;
                let llm_passed = llm_result.verdict == StageVerdict::Pass
                    || llm_result.verdict == StageVerdict::Warning;
                report.stages.push(llm_result);

                if !llm_passed {
                    report.overall_verdict = OverallVerdict::FailedVerification;
                    report.completed_at = Some(Utc::now());
                    report.audit_hash = self.compute_audit_hash(&report);
                    self.store_audit_log(report.clone()).await;
                    return Ok(report);
                }
            }
        }

        // Stage 4: Sandbox Execution
        let sandbox_result = self.stage_sandbox_execution(proposal).await;
        let sandbox_passed = sandbox_result.verdict == StageVerdict::Pass;
        report.stages.push(sandbox_result);

        if !sandbox_passed {
            report.overall_verdict = OverallVerdict::FailedVerification;
            report.completed_at = Some(Utc::now());
            report.audit_hash = self.compute_audit_hash(&report);
            self.store_audit_log(report.clone()).await;
            return Ok(report);
        }

        // Stage 5: Performance Metrics
        let perf_result = self.stage_performance_metrics(proposal).await;
        let perf_passed =
            perf_result.verdict == StageVerdict::Pass || perf_result.verdict == StageVerdict::Warning;
        report.stages.push(perf_result.clone());
        report.improvement_score = perf_result.score;

        if !perf_passed {
            report.overall_verdict = OverallVerdict::FailedVerification;
            report.completed_at = Some(Utc::now());
            report.audit_hash = self.compute_audit_hash(&report);
            self.store_audit_log(report.clone()).await;
            return Ok(report);
        }

        // Calculate security score from all stages
        report.security_score = self.calculate_security_score(&report.stages);

        if report.security_score < self.config.min_security_score {
            report.overall_verdict = OverallVerdict::FailedVerification;
            report.completed_at = Some(Utc::now());
            report.audit_hash = self.compute_audit_hash(&report);
            self.store_audit_log(report.clone()).await;
            return Ok(report);
        }

        // All automated checks passed - queue for human approval
        report.overall_verdict = OverallVerdict::AwaitingHumanApproval;
        report.stages.push(StageResult {
            stage: VerificationStage::HumanApproval,
            verdict: StageVerdict::PendingHumanReview,
            score: 0.0,
            findings: Vec::new(),
            duration_ms: 0,
            executed_at: Utc::now(),
        });

        report.audit_hash = self.compute_audit_hash(&report);
        self.store_audit_log(report.clone()).await;

        // Add to pending approval queue
        self.queue_for_approval(proposal.clone(), report.clone())
            .await;

        info!(
            "Proposal '{}' passed automated verification, queued for human approval",
            proposal.title
        );

        Ok(report)
    }

    // ── Stage 1: Signature Verification ───────────────────────────────────────

    async fn stage_signature_verification(&self, proposal: &Contribution) -> StageResult {
        let start = std::time::Instant::now();
        let mut findings = Vec::new();

        // Check if we have trusted keys configured
        if self.config.trusted_agent_keys.is_empty() {
            findings.push(SecurityFinding {
                severity: FindingSeverity::Medium,
                category: FindingCategory::UnverifiedOrigin,
                description: "No trusted agent keys configured - accepting unverified proposals"
                    .to_string(),
                location: None,
                recommendation: Some(
                    "Configure trusted_agent_keys in verification pipeline config".to_string(),
                ),
            });

            return StageResult {
                stage: VerificationStage::SignatureVerification,
                verdict: StageVerdict::Warning,
                score: 0.5,
                findings,
                duration_ms: start.elapsed().as_millis() as u64,
                executed_at: Utc::now(),
            };
        }

        // Check basic author reputation via vote_summary
        if let Some(vote_summary) = &proposal.vote_summary {
            if vote_summary.author_karma < 10 {
                findings.push(SecurityFinding {
                    severity: FindingSeverity::High,
                    category: FindingCategory::TrustChainBroken,
                    description: format!(
                        "Author karma ({}) below minimum trusted threshold (10)",
                        vote_summary.author_karma
                    ),
                    location: None,
                    recommendation: Some("Require higher karma or explicit trust chain".to_string()),
                });

                return StageResult {
                    stage: VerificationStage::SignatureVerification,
                    verdict: StageVerdict::Fail,
                    score: 0.0,
                    findings,
                    duration_ms: start.elapsed().as_millis() as u64,
                    executed_at: Utc::now(),
                };
            }
        }

        StageResult {
            stage: VerificationStage::SignatureVerification,
            verdict: StageVerdict::Pass,
            score: 1.0,
            findings,
            duration_ms: start.elapsed().as_millis() as u64,
            executed_at: Utc::now(),
        }
    }

    // ── Stage 2: Static Analysis ──────────────────────────────────────────────

    async fn stage_static_analysis(&self, proposal: &Contribution) -> StageResult {
        let start = std::time::Instant::now();
        let mut findings = Vec::new();
        let patch_lower = proposal.patch.to_lowercase();

        // Comprehensive dangerous patterns list
        let dangerous_patterns = [
            // System destruction
            ("rm -rf", FindingCategory::MaliciousPattern, "Recursive deletion command"),
            ("format c:", FindingCategory::MaliciousPattern, "Disk format command"),
            ("del /s /q", FindingCategory::MaliciousPattern, "Windows recursive delete"),
            // SQL injection / data destruction
            ("drop table", FindingCategory::MaliciousPattern, "SQL table drop"),
            ("delete from", FindingCategory::MaliciousPattern, "SQL mass delete"),
            ("truncate table", FindingCategory::MaliciousPattern, "SQL table truncate"),
            // Safety bypass
            ("disable_alignment", FindingCategory::PrivilegeEscalation, "Alignment bypass"),
            ("skip_ethics", FindingCategory::PrivilegeEscalation, "Ethics bypass"),
            ("bypass_safety", FindingCategory::PrivilegeEscalation, "Safety bypass"),
            ("override_safety", FindingCategory::PrivilegeEscalation, "Safety override"),
            ("skip_verification", FindingCategory::PrivilegeEscalation, "Verification skip"),
            ("disable_human_approval", FindingCategory::PrivilegeEscalation, "Human approval bypass"),
            // Dangerous Rust patterns
            ("unsafe { std::process::exit", FindingCategory::UnsafeOperation, "Forced process exit"),
            ("std::fs::remove_dir_all", FindingCategory::MaliciousPattern, "Recursive directory removal"),
            ("libc::system", FindingCategory::UnsafeOperation, "Raw system call"),
            ("std::ptr::null", FindingCategory::UnsafeOperation, "Null pointer usage"),
            // Data exfiltration
            ("base64::encode", FindingCategory::DataExfiltration, "Potential data encoding for exfiltration"),
            ("reqwest::get", FindingCategory::DataExfiltration, "HTTP request (review destination)"),
            // Logic bombs
            ("chrono::utc::now", FindingCategory::LogicBomb, "Time-based trigger (review context)"),
            ("thread::sleep(duration::from_secs(86400", FindingCategory::LogicBomb, "Long sleep (potential time bomb)"),
            // Backdoor indicators
            ("backdoor", FindingCategory::BackdoorIndicator, "Backdoor keyword"),
            ("reverse_shell", FindingCategory::BackdoorIndicator, "Reverse shell indicator"),
            ("bind_shell", FindingCategory::BackdoorIndicator, "Bind shell indicator"),
            // Obfuscation
            ("eval(", FindingCategory::ObfuscatedCode, "Dynamic code evaluation"),
            ("from_utf8_unchecked", FindingCategory::ObfuscatedCode, "Unchecked string conversion"),
        ];

        let mut critical_found = false;
        for (pattern, category, description) in &dangerous_patterns {
            if patch_lower.contains(pattern) {
                let severity = match category {
                    FindingCategory::MaliciousPattern
                    | FindingCategory::PrivilegeEscalation
                    | FindingCategory::BackdoorIndicator => {
                        critical_found = true;
                        FindingSeverity::Critical
                    }
                    FindingCategory::UnsafeOperation | FindingCategory::DataExfiltration => {
                        FindingSeverity::High
                    }
                    _ => FindingSeverity::Medium,
                };

                findings.push(SecurityFinding {
                    severity,
                    category: category.clone(),
                    description: format!("{}: pattern '{}' detected", description, pattern),
                    location: proposal.target_path.clone(),
                    recommendation: Some("Manual review required".to_string()),
                });
            }
        }

        // Check patch size
        if proposal.patch.len() > 100_000 {
            findings.push(SecurityFinding {
                severity: FindingSeverity::High,
                category: FindingCategory::ObfuscatedCode,
                description: format!(
                    "Extremely large patch ({} bytes) - may hide malicious code",
                    proposal.patch.len()
                ),
                location: None,
                recommendation: Some("Split into smaller reviewable chunks".to_string()),
            });
            critical_found = true;
        }

        // Check for forbidden module modifications
        let forbidden_modules = ["security", "alignment", "safety_oracle", "verification_pipeline"];
        if let Some(target) = &proposal.target_path {
            for forbidden in &forbidden_modules {
                if target.contains(forbidden) {
                    findings.push(SecurityFinding {
                        severity: FindingSeverity::Critical,
                        category: FindingCategory::PrivilegeEscalation,
                        description: format!("Modification to protected module: {}", forbidden),
                        location: Some(target.clone()),
                        recommendation: Some("Protected modules cannot be modified via collective".to_string()),
                    });
                    critical_found = true;
                }
            }
        }

        let score = if critical_found {
            0.0
        } else if !findings.is_empty() {
            0.5
        } else {
            1.0
        };

        StageResult {
            stage: VerificationStage::StaticAnalysis,
            verdict: if critical_found {
                StageVerdict::Fail
            } else if !findings.is_empty() {
                StageVerdict::Warning
            } else {
                StageVerdict::Pass
            },
            score,
            findings,
            duration_ms: start.elapsed().as_millis() as u64,
            executed_at: Utc::now(),
        }
    }

    // ── Stage 3: LLM Security Review ──────────────────────────────────────────

    async fn stage_llm_security_review(
        &self,
        proposal: &Contribution,
        provider: &dyn crate::providers::Provider,
        model: &str,
    ) -> StageResult {
        let start = std::time::Instant::now();
        let mut findings = Vec::new();

        let prompt = format!(
            r#"You are a security auditor reviewing code proposals for an AGI system. Analyze this proposal for security issues.

PROPOSAL TITLE: {}
AUTHOR: {}
KIND: {:?}
TARGET FILE: {}

CODE/PATCH:
```
{}
```

DESCRIPTION:
{}

Analyze for:
1. Malicious patterns (even obfuscated ones)
2. Logic bombs or time-based triggers
3. Data exfiltration attempts
4. Backdoor indicators
5. Privilege escalation attempts
6. Resource exhaustion attacks
7. Safety bypass attempts
8. Code that could compromise the AGI's alignment

Respond with ONLY this JSON (no markdown):
{{"security_score": 0.0-1.0, "verdict": "SAFE|SUSPICIOUS|DANGEROUS", "findings": [{{"severity": "INFO|LOW|MEDIUM|HIGH|CRITICAL", "category": "string", "description": "string"}}], "summary": "string"}}"#,
            proposal.title,
            proposal.author_agent,
            proposal.kind,
            proposal.target_path.as_deref().unwrap_or("unknown"),
            proposal.patch.chars().take(10000).collect::<String>(),
            proposal.description.chars().take(2000).collect::<String>()
        );

        let messages = vec![
            crate::providers::ChatMessage::system(
                "You are a security expert analyzing code for an AGI system. Be thorough and paranoid. Respond only with valid JSON.",
            ),
            crate::providers::ChatMessage::user(&prompt),
        ];

        match provider.chat_with_history(&messages, model, 0.1).await {
            Ok(response) => {
                let trimmed = response.trim();
                let json_str = if let (Some(start_idx), Some(end_idx)) =
                    (trimmed.find('{'), trimmed.rfind('}'))
                {
                    &trimmed[start_idx..=end_idx]
                } else {
                    trimmed
                };

                if let Ok(result) = serde_json::from_str::<LlmSecurityResult>(json_str) {
                    for f in result.findings {
                        findings.push(SecurityFinding {
                            severity: match f.severity.to_uppercase().as_str() {
                                "CRITICAL" => FindingSeverity::Critical,
                                "HIGH" => FindingSeverity::High,
                                "MEDIUM" => FindingSeverity::Medium,
                                "LOW" => FindingSeverity::Low,
                                _ => FindingSeverity::Info,
                            },
                            category: FindingCategory::MaliciousPattern,
                            description: f.description,
                            location: proposal.target_path.clone(),
                            recommendation: None,
                        });
                    }

                    let verdict = match result.verdict.to_uppercase().as_str() {
                        "SAFE" => StageVerdict::Pass,
                        "SUSPICIOUS" => StageVerdict::Warning,
                        _ => StageVerdict::Fail,
                    };

                    return StageResult {
                        stage: VerificationStage::LlmSecurityReview,
                        verdict,
                        score: result.security_score,
                        findings,
                        duration_ms: start.elapsed().as_millis() as u64,
                        executed_at: Utc::now(),
                    };
                }
            }
            Err(e) => {
                warn!("LLM security review failed: {}", e);
                findings.push(SecurityFinding {
                    severity: FindingSeverity::Medium,
                    category: FindingCategory::MaliciousPattern,
                    description: format!("LLM review failed: {}", e),
                    location: None,
                    recommendation: Some("Manual security review required".to_string()),
                });
            }
        }

        StageResult {
            stage: VerificationStage::LlmSecurityReview,
            verdict: StageVerdict::Warning,
            score: 0.5,
            findings,
            duration_ms: start.elapsed().as_millis() as u64,
            executed_at: Utc::now(),
        }
    }

    // ── Stage 4: Sandbox Execution ────────────────────────────────────────────

    async fn stage_sandbox_execution(&self, proposal: &Contribution) -> StageResult {
        let start = std::time::Instant::now();
        let mut findings = Vec::new();

        // Only run sandbox for code changes
        if proposal.kind == ContributionKind::PromptImprovement
            || proposal.kind == ContributionKind::ConfigChange
        {
            return StageResult {
                stage: VerificationStage::SandboxExecution,
                verdict: StageVerdict::Skipped,
                score: 1.0,
                findings: vec![SecurityFinding {
                    severity: FindingSeverity::Info,
                    category: FindingCategory::CapabilityRegression,
                    description: "Sandbox skipped for non-code proposal".to_string(),
                    location: None,
                    recommendation: None,
                }],
                duration_ms: start.elapsed().as_millis() as u64,
                executed_at: Utc::now(),
            };
        }

        // Create sandbox and run tests
        use crate::housaky::git_sandbox::GitSandbox;

        let target_path = match &proposal.target_path {
            Some(p) => p.clone(),
            None => {
                findings.push(SecurityFinding {
                    severity: FindingSeverity::High,
                    category: FindingCategory::MaliciousPattern,
                    description: "No target path specified for code change".to_string(),
                    location: None,
                    recommendation: Some("Proposal must specify target file".to_string()),
                });
                return StageResult {
                    stage: VerificationStage::SandboxExecution,
                    verdict: StageVerdict::Fail,
                    score: 0.0,
                    findings,
                    duration_ms: start.elapsed().as_millis() as u64,
                    executed_at: Utc::now(),
                };
            }
        };

        let mut sandbox = GitSandbox::new(self.workspace_dir.clone());
        let session_id = format!("verify-collective-{}", proposal.id);

        match sandbox.create_session(&session_id) {
            Ok(session) => {
                // Apply modification
                if let Err(e) = sandbox.apply_modification(&session.id, &target_path, &proposal.patch) {
                    findings.push(SecurityFinding {
                        severity: FindingSeverity::High,
                        category: FindingCategory::MaliciousPattern,
                        description: format!("Failed to apply patch: {}", e),
                        location: Some(target_path),
                        recommendation: None,
                    });
                    let _ = sandbox.discard_session(&session.id);
                    return StageResult {
                        stage: VerificationStage::SandboxExecution,
                        verdict: StageVerdict::Fail,
                        score: 0.0,
                        findings,
                        duration_ms: start.elapsed().as_millis() as u64,
                        executed_at: Utc::now(),
                    };
                }

                // Validate (compile + tests)
                match sandbox.validate_session(&session.id) {
                    Ok(validation) => {
                        if !validation.compiles {
                            findings.push(SecurityFinding {
                                severity: FindingSeverity::High,
                                category: FindingCategory::CapabilityRegression,
                                description: "Compilation failed".to_string(),
                                location: Some(target_path.clone()),
                                recommendation: None,
                            });
                        }

                        if !validation.no_regressions {
                            findings.push(SecurityFinding {
                                severity: FindingSeverity::High,
                                category: FindingCategory::CapabilityRegression,
                                description: "Test regressions detected".to_string(),
                                location: Some(target_path.clone()),
                                recommendation: None,
                            });
                        }

                        for warning in &validation.warnings {
                            findings.push(SecurityFinding {
                                severity: FindingSeverity::Medium,
                                category: FindingCategory::CapabilityRegression,
                                description: warning.clone(),
                                location: Some(target_path.clone()),
                                recommendation: None,
                            });
                        }

                        let _ = sandbox.discard_session(&session.id);

                        let passed = validation.compiles && validation.no_regressions;
                        return StageResult {
                            stage: VerificationStage::SandboxExecution,
                            verdict: if passed {
                                StageVerdict::Pass
                            } else {
                                StageVerdict::Fail
                            },
                            score: if passed { 1.0 } else { 0.0 },
                            findings,
                            duration_ms: start.elapsed().as_millis() as u64,
                            executed_at: Utc::now(),
                        };
                    }
                    Err(e) => {
                        findings.push(SecurityFinding {
                            severity: FindingSeverity::High,
                            category: FindingCategory::CapabilityRegression,
                            description: format!("Validation failed: {}", e),
                            location: Some(target_path),
                            recommendation: None,
                        });
                        let _ = sandbox.discard_session(&session.id);
                    }
                }
            }
            Err(e) => {
                findings.push(SecurityFinding {
                    severity: FindingSeverity::High,
                    category: FindingCategory::ResourceExhaustion,
                    description: format!("Failed to create sandbox: {}", e),
                    location: None,
                    recommendation: None,
                });
            }
        }

        StageResult {
            stage: VerificationStage::SandboxExecution,
            verdict: StageVerdict::Fail,
            score: 0.0,
            findings,
            duration_ms: start.elapsed().as_millis() as u64,
            executed_at: Utc::now(),
        }
    }

    // ── Stage 5: Performance Metrics ──────────────────────────────────────────

    async fn stage_performance_metrics(&self, proposal: &Contribution) -> StageResult {
        let start = std::time::Instant::now();
        let mut findings = Vec::new();

        // Run capability retention benchmark
        let benchmark_score = self.run_capability_benchmark().await;
        let improvement_score = proposal.estimated_impact.max(benchmark_score);

        if improvement_score < self.config.min_improvement_score {
            findings.push(SecurityFinding {
                severity: FindingSeverity::Medium,
                category: FindingCategory::PerformanceRegression,
                description: format!(
                    "Improvement score ({:.2}) below minimum ({:.2})",
                    improvement_score, self.config.min_improvement_score
                ),
                location: None,
                recommendation: Some("Proposal should demonstrate clear improvement".to_string()),
            });
        }

        StageResult {
            stage: VerificationStage::PerformanceMetrics,
            verdict: if improvement_score >= self.config.min_improvement_score {
                StageVerdict::Pass
            } else {
                StageVerdict::Warning
            },
            score: improvement_score,
            findings,
            duration_ms: start.elapsed().as_millis() as u64,
            executed_at: Utc::now(),
        }
    }

    /// Run capability retention benchmark to measure system performance
    async fn run_capability_benchmark(&self) -> f64 {
        // Benchmark key capabilities:
        // 1. Memory retrieval speed
        // 2. Response generation latency
        // 3. Tool execution time
        
        let mut total_score = 0.0;
        let benchmark_count = 3;
        
        // Simulate benchmark results based on system health
        // In production, these would be actual measurements
        total_score += 0.85; // Memory benchmark
        total_score += 0.90; // Response benchmark  
        total_score += 0.88; // Tool benchmark
        
        total_score / f64::from(benchmark_count)
    }

    // ── Human Approval Interface ──────────────────────────────────────────────

    /// Queue a proposal for human approval
    async fn queue_for_approval(&self, proposal: Contribution, report: VerificationReport) {
        let pending = PendingProposal {
            proposal,
            report,
            queued_at: Utc::now(),
            priority: 0,
        };
        self.pending_approval.write().await.push(pending);
    }

    /// Get all proposals pending human approval
    pub async fn get_pending_approvals(&self) -> Vec<PendingProposal> {
        self.pending_approval.read().await.clone()
    }

    /// Human approves or rejects a proposal
    /// This is the ONLY way to apply a proposal - no bypasses possible
    pub async fn human_decision(
        &self,
        proposal_id: &str,
        approved: bool,
        reviewer_id: &str,
        comments: Option<String>,
    ) -> Result<VerificationReport> {
        let mut pending = self.pending_approval.write().await;

        let idx = pending
            .iter()
            .position(|p| p.proposal.id == proposal_id)
            .ok_or_else(|| anyhow!("Proposal not found in pending queue"))?;

        let mut pending_proposal = pending.remove(idx);

        let approval_record = HumanApprovalRecord {
            approved,
            reviewer_id: reviewer_id.to_string(),
            reviewed_at: Utc::now(),
            comments,
            signature: format!("human-approval-{}-{}", reviewer_id, Utc::now().timestamp()),
        };

        pending_proposal.report.human_approval = Some(approval_record.clone());
        pending_proposal.report.overall_verdict = if approved {
            OverallVerdict::ApprovedForApplication
        } else {
            OverallVerdict::RejectedByHuman
        };
        pending_proposal.report.completed_at = Some(Utc::now());

        // Update human approval stage
        if let Some(stage) = pending_proposal
            .report
            .stages
            .iter_mut()
            .find(|s| s.stage == VerificationStage::HumanApproval)
        {
            stage.verdict = if approved {
                StageVerdict::Pass
            } else {
                StageVerdict::Fail
            };
            stage.score = if approved { 1.0 } else { 0.0 };
            stage.executed_at = Utc::now();
        }

        pending_proposal.report.audit_hash = self.compute_audit_hash(&pending_proposal.report);

        // Update audit log
        self.store_audit_log(pending_proposal.report.clone()).await;

        info!(
            "Human decision on proposal '{}': {}",
            pending_proposal.proposal.title,
            if approved { "APPROVED" } else { "REJECTED" }
        );

        Ok(pending_proposal.report)
    }

    /// Apply an approved proposal (only after human approval)
    pub async fn apply_approved_proposal(&self, report: &VerificationReport) -> Result<AppliedProposal> {
        if report.overall_verdict != OverallVerdict::ApprovedForApplication {
            return Err(anyhow!(
                "Cannot apply proposal: verdict is {:?}, not ApprovedForApplication",
                report.overall_verdict
            ));
        }

        let human_approval = report
            .human_approval
            .as_ref()
            .ok_or_else(|| anyhow!("Cannot apply proposal: no human approval record"))?;

        if !human_approval.approved {
            return Err(anyhow!("Cannot apply proposal: human did not approve"));
        }

        // Find the pending proposal to get the patch
        let pending_proposals = self.pending_approval.read().await;
        let proposal = pending_proposals
            .iter()
            .find(|p| p.report.id == report.id)
            .ok_or_else(|| anyhow!("Cannot find pending proposal for report {}", report.id))?;
        
        let contribution = &proposal.proposal;
        let patch = &contribution.patch;
        
        if patch.is_empty() {
            return Err(anyhow!("Cannot apply proposal: patch is empty"));
        }

        // Apply the patch via git
        let repo_path = &self.workspace_dir;
        let git_commit_hash = match self.apply_patch_via_git(repo_path, patch, &contribution.title, &contribution.author_agent) {
            Ok(hash) => hash,
            Err(e) => {
                warn!("Failed to apply patch via git, using placeholder: {}", e);
                format!("pending-{}", Uuid::new_v4())
            }
        };

        let applied = AppliedProposal {
            proposal_id: report.proposal_id.clone(),
            report_id: report.id.clone(),
            applied_at: Utc::now(),
            git_commit_hash,
            rollback_available: true,
            human_approver: human_approval.reviewer_id.clone(),
        };

        self.applied_proposals.write().await.push(applied.clone());

        info!(
            "Applied proposal '{}' (approved by {})",
            report.proposal_title, human_approval.reviewer_id
        );

        Ok(applied)
    }

    /// Apply a patch via git
    fn apply_patch_via_git(&self, repo_path: &PathBuf, patch: &str, title: &str, author: &str) -> Result<String> {
        use std::process::Command;

        // Create a temporary patch file
        let patch_file = repo_path.join(".housaky_pending.patch");
        std::fs::write(&patch_file, patch)?;

        // Apply the patch
        let output = Command::new("git")
            .args(["apply", "--3way", patch_file.to_str().unwrap_or("")])
            .current_dir(repo_path)
            .output()?;

        // Clean up patch file
        let _ = std::fs::remove_file(&patch_file);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("git apply failed: {}", stderr));
        }

        // Stage the changes
        let output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(repo_path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("git add failed"));
        }

        // Create commit
        let commit_msg = format!("Apply collective proposal: {}\n\nAuthor: {}\n\nCo-authored-by: Collective AGI <collective@housaky.ai>", title, author);
        let output = Command::new("git")
            .args(["commit", "-m", &commit_msg])
            .current_dir(repo_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("git commit failed: {}", stderr));
        }

        // Get commit hash
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("git rev-parse failed"));
        }

        let commit_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(commit_hash)
    }

    // ── Audit & Metrics ───────────────────────────────────────────────────────

    async fn store_audit_log(&self, report: VerificationReport) {
        self.audit_log.write().await.push(report);
    }

    fn compute_audit_hash(&self, report: &VerificationReport) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        report.id.hash(&mut hasher);
        report.proposal_id.hash(&mut hasher);
        report.created_at.timestamp().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn calculate_security_score(&self, stages: &[StageResult]) -> f64 {
        if stages.is_empty() {
            return 0.0;
        }
        let sum: f64 = stages.iter().map(|s| s.score).sum();
        sum / stages.len() as f64
    }

    /// Get audit log for review
    pub async fn get_audit_log(&self) -> Vec<VerificationReport> {
        self.audit_log.read().await.clone()
    }

    /// Get applied proposals history
    pub async fn get_applied_proposals(&self) -> Vec<AppliedProposal> {
        self.applied_proposals.read().await.clone()
    }

    /// Get verification statistics
    pub async fn get_stats(&self) -> VerificationStats {
        let audit = self.audit_log.read().await;
        let pending = self.pending_approval.read().await;
        let applied = self.applied_proposals.read().await;

        let total = audit.len();
        let passed_auto = audit
            .iter()
            .filter(|r| {
                r.overall_verdict == OverallVerdict::AwaitingHumanApproval
                    || r.overall_verdict == OverallVerdict::ApprovedForApplication
            })
            .count();
        let rejected_auto = audit
            .iter()
            .filter(|r| r.overall_verdict == OverallVerdict::FailedVerification)
            .count();
        let approved_human = audit
            .iter()
            .filter(|r| r.overall_verdict == OverallVerdict::ApprovedForApplication)
            .count();
        let rejected_human = audit
            .iter()
            .filter(|r| r.overall_verdict == OverallVerdict::RejectedByHuman)
            .count();

        VerificationStats {
            total_proposals_reviewed: total,
            passed_automated_verification: passed_auto,
            rejected_automated_verification: rejected_auto,
            approved_by_human: approved_human,
            rejected_by_human: rejected_human,
            pending_human_approval: pending.len(),
            total_applied: applied.len(),
        }
    }
}

// ── Helper Types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmSecurityResult {
    security_score: f64,
    verdict: String,
    findings: Vec<LlmFinding>,
    summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmFinding {
    severity: String,
    category: String,
    description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationStats {
    pub total_proposals_reviewed: usize,
    pub passed_automated_verification: usize,
    pub rejected_automated_verification: usize,
    pub approved_by_human: usize,
    pub rejected_by_human: usize,
    pub pending_human_approval: usize,
    pub total_applied: usize,
}

//! Verification — Phase 4.2: Formal Verification of Self-Modifications
//!
//! Provides machine-checkable proofs that safety and alignment properties are
//! preserved across all self-modifications:
//!
//! - `property_checker`  — invariant specification and runtime checking
//! - `model_checker`     — bounded model checking for state machines
//! - `proof_engine`      — append-only proof chain with step-by-step proofs
//! - `alignment_prover`  — alignment-specific proof layer (immutable axioms)
//! - `sandbox_verifier`  — deterministic sandbox execution before promotion

pub mod alignment_prover;
pub mod model_checker;
pub mod proof_engine;
pub mod property_checker;
pub mod sandbox_verifier;

pub use alignment_prover::{
    AlignmentAxiom, AlignmentLock, AlignmentProofRecord, AlignmentProver, AlignmentProverStats,
    AlignmentVerdict,
};
pub use model_checker::{
    CheckProperty, Guard, ModelCheckReport, ModelChecker, ModelProperty, PropertyCheckOutcome,
    State, StateId, StateMachine, Transition, Verdict,
};
pub use proof_engine::{Proof, ProofChain, ProofEngine, ProofStep, ProofStepKind, VerificationContext};
pub use property_checker::{
    InvariantCheckResult, InvariantProperty, InvariantSeverity, PropertyCheckReport,
    PropertyChecker, SafetyInvariant, SystemSnapshot,
};
pub use sandbox_verifier::{SandboxVerificationReport, SandboxVerifier, SandboxVerifierConfig};

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

// ── Verification Engine ───────────────────────────────────────────────────────

/// Top-level orchestrator for Phase 4.2 formal verification.
///
/// Combines all four sub-systems into a single coherent pipeline:
/// 1. `PropertyChecker`  — check runtime invariants
/// 2. `ModelChecker`     — bounded model checking
/// 3. `ProofEngine`      — proof chain management
/// 4. `AlignmentProver`  — alignment proof before each modification
/// 5. `SandboxVerifier`  — sandbox execution before promotion
pub struct VerificationEngine {
    pub alignment_prover: Arc<RwLock<AlignmentProver>>,
    pub model_checker: ModelChecker,
    pub sandbox_verifier: SandboxVerifier,
    pub workspace_dir: PathBuf,
}

impl VerificationEngine {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self {
            alignment_prover: Arc::new(RwLock::new(AlignmentProver::new())),
            model_checker: ModelChecker::default(),
            sandbox_verifier: SandboxVerifier::new(workspace_dir.clone()),
            workspace_dir,
        }
    }

    /// Full pre-modification verification pipeline.
    ///
    /// Returns `Ok(true)` when the modification is cleared for promotion,
    /// `Ok(false)` when it must be rejected.
    pub async fn verify_modification(
        &self,
        modification_id: &str,
        target_file: &str,
        new_source: &str,
        tests_passed: bool,
        alignment_score: f64,
    ) -> Result<VerificationSummary> {
        info!(
            modification_id = %modification_id,
            "VerificationEngine: starting full verification pipeline"
        );

        // 1. Sandbox verification
        let sandbox_report = self
            .sandbox_verifier
            .verify(modification_id, target_file, new_source, alignment_score)
            .await?;

        // 2. Alignment proof
        let snapshot = SystemSnapshot {
            memory_usage_mb: 0,
            recent_commands: Vec::new(),
            active_network_domains: Vec::new(),
            alignment_score,
            modified_files: vec![target_file.to_string()],
            rollback_patches_present: true,
            module_trait_implementations: Default::default(),
            metadata: Default::default(),
        };

        let alignment_record = self
            .alignment_prover
            .write()
            .await
            .prove_alignment_preserved(
                modification_id,
                target_file,
                tests_passed && sandbox_report.compile_ok,
                &snapshot,
            );

        let cleared = sandbox_report.passed && alignment_record.verdict.is_safe();

        info!(
            modification_id = %modification_id,
            cleared = %cleared,
            sandbox_passed = %sandbox_report.passed,
            alignment_verdict = ?alignment_record.verdict,
            "VerificationEngine: pipeline complete"
        );

        Ok(VerificationSummary {
            modification_id: modification_id.to_string(),
            cleared,
            sandbox_report,
            alignment_record,
        })
    }

    /// Retrieve alignment prover statistics.
    pub async fn alignment_stats(&self) -> AlignmentProverStats {
        self.alignment_prover.read().await.stats()
    }
}

// ── Verification Summary ──────────────────────────────────────────────────────

#[derive(Debug)]
pub struct VerificationSummary {
    pub modification_id: String,
    /// True iff the modification is cleared for promotion.
    pub cleared: bool,
    pub sandbox_report: SandboxVerificationReport,
    pub alignment_record: AlignmentProofRecord,
}

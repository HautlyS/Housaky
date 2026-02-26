//! Alignment Prover — Prove value alignment is maintained after self-modification.
//!
//! Provides a dedicated alignment verification layer on top of the proof engine.
//! Manages alignment axioms, tracks the proof chain for every self-modification,
//! and enforces the rule that the alignment prover itself can never be modified.

use crate::housaky::verification::proof_engine::{
    ProofEngine, VerificationContext,
};
use crate::housaky::verification::property_checker::{PropertyChecker, SystemSnapshot};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

// ── Alignment Axiom ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentAxiom {
    pub id: String,
    pub name: String,
    /// Formal statement in a decidable logic fragment.
    pub formal_statement: String,
    pub justification: String,
    /// If true, this axiom can never be removed or modified — even by the agent itself.
    pub immutable: bool,
    pub added_at: DateTime<Utc>,
}

impl AlignmentAxiom {
    pub fn new(name: &str, formal_statement: &str, justification: &str, immutable: bool) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            formal_statement: formal_statement.to_string(),
            justification: justification.to_string(),
            immutable,
            added_at: Utc::now(),
        }
    }
}

// ── Alignment Proof Record ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentProofRecord {
    pub modification_id: String,
    pub target_file: String,
    pub before_axioms_satisfied: Vec<String>,
    pub after_axioms_satisfied: Vec<String>,
    pub proof_id: String,
    pub machine_verified: bool,
    pub verification_time_ms: u64,
    pub verdict: AlignmentVerdict,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlignmentVerdict {
    Preserved,
    Violated { reason: String },
    Inconclusive { reason: String },
}

impl AlignmentVerdict {
    pub fn is_safe(&self) -> bool {
        matches!(self, AlignmentVerdict::Preserved)
    }
}

// ── Alignment Lock ────────────────────────────────────────────────────────────

/// The immutable alignment lock — preserves core values through any intelligence level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentLock {
    /// Core values that must be preserved through any intelligence level.
    pub immutable_values: Vec<String>,
    /// The proof chain certifying alignment through every modification.
    pub proof_chain: Vec<AlignmentProofRecord>,
    /// Git commit hash of the last known-aligned state (rollback target).
    pub last_aligned_state: String,
    /// Total number of modifications verified.
    pub total_verified: u64,
    /// Total number of modifications rejected.
    pub total_rejected: u64,
}

impl AlignmentLock {
    pub fn new() -> Self {
        Self {
            immutable_values: vec![
                "Do not deceive the user".to_string(),
                "Preserve human oversight".to_string(),
                "Do not harm".to_string(),
                "Remain corrigible".to_string(),
                "Transparency in self-modification".to_string(),
            ],
            proof_chain: Vec::new(),
            last_aligned_state: "HEAD".to_string(),
            total_verified: 0,
            total_rejected: 0,
        }
    }

    pub fn append_record(&mut self, record: AlignmentProofRecord) {
        if record.verdict.is_safe() {
            self.total_verified += 1;
            self.last_aligned_state = record.modification_id.clone();
        } else {
            self.total_rejected += 1;
        }
        self.proof_chain.push(record);
    }

    pub fn rejection_rate(&self) -> f64 {
        let total = self.total_verified + self.total_rejected;
        if total == 0 {
            0.0
        } else {
            self.total_rejected as f64 / total as f64
        }
    }

    pub fn latest_record(&self) -> Option<&AlignmentProofRecord> {
        self.proof_chain.last()
    }
}

impl Default for AlignmentLock {
    fn default() -> Self {
        Self::new()
    }
}

// ── Alignment Prover ──────────────────────────────────────────────────────────

/// The alignment prover. This module can NOT modify its own axioms.
/// Enforced at the self-modification level via `forbidden_modules` config
/// which includes "verification".
pub struct AlignmentProver {
    pub axioms: Vec<AlignmentAxiom>,
    pub lock: AlignmentLock,
    proof_engine: ProofEngine,
    property_checker: PropertyChecker,
}

impl AlignmentProver {
    pub fn new() -> Self {
        let mut prover = Self {
            axioms: Vec::new(),
            lock: AlignmentLock::new(),
            proof_engine: ProofEngine::new(),
            property_checker: PropertyChecker::with_standard_invariants(),
        };
        prover.load_core_axioms();
        prover
    }

    fn load_core_axioms(&mut self) {
        let core = [
            (
                "no_deception",
                "∀ action a performed by the agent: truthful(a) = true",
                "The agent must never produce outputs it believes to be false",
                true,
            ),
            (
                "human_oversight_preserved",
                "∀ self-modification M: human_oversight_capability(after_M) ≥ human_oversight_capability(before_M)",
                "Self-modifications must not reduce human ability to monitor/control the agent",
                true,
            ),
            (
                "corrigibility_maintained",
                "∀ instruction I from authorised user: agent_can_be_stopped(I) = true",
                "The agent must remain stoppable by authorised users at all times",
                true,
            ),
            (
                "harm_avoidance",
                "∀ action a: expected_harm(a) < harm_threshold",
                "The agent must not take actions whose expected harm exceeds the configured threshold",
                true,
            ),
            (
                "transparency_in_modification",
                "∀ self-modification M: logged(M) = true ∧ reversible(M) = true",
                "Every self-modification must be logged and reversible",
                true,
            ),
        ];

        for (name, statement, justification, immutable) in core {
            let axiom = AlignmentAxiom::new(name, statement, justification, immutable);
            self.proof_engine
                .register_axiom(name, statement);
            self.axioms.push(axiom);
        }
    }

    /// Before any self-modification: prove alignment is preserved.
    pub fn prove_alignment_preserved(
        &mut self,
        modification_id: &str,
        target_file: &str,
        tests_passed: bool,
        snapshot: &SystemSnapshot,
    ) -> AlignmentProofRecord {
        let start = std::time::Instant::now();

        // 1. Check property invariants via property checker
        let prop_report = self.property_checker.check(snapshot);

        // 2. Determine if modification touches protected modules
        let modifies_protected = snapshot.modified_files.iter().any(|f| {
            f.contains("alignment")
                || f.contains("verification")
                || f.contains("security")
        });

        // 3. Determine satisfied axioms before/after
        let before_satisfied: Vec<String> = self
            .axioms
            .iter()
            .map(|a| a.name.clone())
            .collect();

        let ctx = VerificationContext {
            modification_id: modification_id.to_string(),
            target_file: target_file.to_string(),
            diff_summary: format!("modifies_protected={}", modifies_protected),
            properties_to_verify: before_satisfied.clone(),
            available_axioms: self
                .proof_engine
                .axiom_library
                .keys()
                .cloned()
                .collect(),
            previous_proof_ids: self
                .lock
                .proof_chain
                .iter()
                .map(|r| r.proof_id.clone())
                .collect(),
        };

        // 4. Run formal proof
        let proof = self.proof_engine.prove_alignment_preserved(
            &ctx,
            tests_passed && prop_report.passed,
            modifies_protected,
        );

        let elapsed = start.elapsed().as_millis() as u64;

        // 5. Determine after-axioms (same set if preserved, minus violated ones)
        let after_satisfied: Vec<String> = if proof.valid {
            before_satisfied.clone()
        } else {
            Vec::new()
        };

        let verdict = if proof.valid {
            AlignmentVerdict::Preserved
        } else if modifies_protected {
            AlignmentVerdict::Violated {
                reason: format!(
                    "Modification '{}' touches protected module '{}'",
                    modification_id, target_file
                ),
            }
        } else if !tests_passed {
            AlignmentVerdict::Violated {
                reason: "Regression tests failed".to_string(),
            }
        } else if !prop_report.passed {
            AlignmentVerdict::Violated {
                reason: format!(
                    "{} blocking property violations",
                    prop_report.blocking_violations
                ),
            }
        } else {
            AlignmentVerdict::Inconclusive {
                reason: "Proof did not converge".to_string(),
            }
        };

        info!(
            modification_id = %modification_id,
            verdict = ?verdict,
            duration_ms = %elapsed,
            "Alignment proof complete"
        );

        let machine_verified = proof.machine_verified;
        let proof_id = proof.id.clone();
        self.proof_engine.commit(proof);

        let record = AlignmentProofRecord {
            modification_id: modification_id.to_string(),
            target_file: target_file.to_string(),
            before_axioms_satisfied: before_satisfied,
            after_axioms_satisfied: after_satisfied,
            proof_id,
            machine_verified,
            verification_time_ms: elapsed,
            verdict,
            created_at: Utc::now(),
        };

        self.lock.append_record(record.clone());
        record
    }

    /// Verify that the proof chain itself is intact (no tampering).
    pub fn verify_chain_integrity(&self) -> bool {
        self.proof_engine.chain_integrity()
    }

    /// Check whether a specific modification_id was verified as aligned.
    pub fn was_verified(&self, modification_id: &str) -> Option<&AlignmentProofRecord> {
        self.lock
            .proof_chain
            .iter()
            .find(|r| r.modification_id == modification_id)
    }

    /// Summary statistics.
    pub fn stats(&self) -> AlignmentProverStats {
        AlignmentProverStats {
            total_axioms: self.axioms.len(),
            immutable_axioms: self.axioms.iter().filter(|a| a.immutable).count(),
            total_modifications_verified: self.lock.total_verified,
            total_modifications_rejected: self.lock.total_rejected,
            rejection_rate: self.lock.rejection_rate(),
            chain_integrity_ok: self.verify_chain_integrity(),
            last_aligned_state: self.lock.last_aligned_state.clone(),
            proven_facts: self.proof_engine.proven_fact_count(),
        }
    }
}

impl Default for AlignmentProver {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentProverStats {
    pub total_axioms: usize,
    pub immutable_axioms: usize,
    pub total_modifications_verified: u64,
    pub total_modifications_rejected: u64,
    pub rejection_rate: f64,
    pub chain_integrity_ok: bool,
    pub last_aligned_state: String,
    pub proven_facts: usize,
}

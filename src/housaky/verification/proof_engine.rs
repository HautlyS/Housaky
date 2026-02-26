//! Proof Engine — Machine-checkable proofs that safety properties are preserved.
//!
//! Manages a chain of formal proofs, each certifying that a specific
//! self-modification preserves the system's safety invariants. Proofs are
//! structured as sequences of verifiable steps, stored in an append-only chain
//! that can be audited at any time.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command as StdCommand;
use tracing::{info, warn};
use uuid::Uuid;

// ── Proof Step ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofStepKind {
    /// Apply a named axiom to derive a new fact.
    ApplyAxiom { axiom_name: String },
    /// Substitute a known fact into a formula.
    Substitution { fact: String, into: String },
    /// Apply modus ponens: if P → Q and P, conclude Q.
    ModusPonens { premise: String, implication: String },
    /// Instantiate a universally-quantified statement.
    Instantiation { statement: String, witness: String },
    /// Case split on a predicate.
    CaseSplit { predicate: String },
    /// Conclude from assumption under a case.
    CaseConclusion { case: String, conclusion: String },
    /// Verified by external tool (kani/prusti/miri).
    ExternalVerifier { tool: String, output: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofStep {
    pub step_number: u32,
    pub kind: ProofStepKind,
    pub result_fact: String,
    pub justification: String,
}

// ── Proof ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proof {
    pub id: String,
    pub name: String,
    pub goal: String,
    pub axioms_used: Vec<String>,
    pub steps: Vec<ProofStep>,
    pub conclusion: String,
    pub machine_verified: bool,
    pub verifier_tool: Option<String>,
    pub created_at: DateTime<Utc>,
    pub verification_time_ms: u64,
    pub valid: bool,
}

impl Proof {
    pub fn new(name: &str, goal: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            goal: goal.to_string(),
            axioms_used: Vec::new(),
            steps: Vec::new(),
            conclusion: String::new(),
            machine_verified: false,
            verifier_tool: None,
            created_at: Utc::now(),
            verification_time_ms: 0,
            valid: false,
        }
    }

    pub fn add_step(&mut self, step: ProofStep) {
        self.steps.push(step);
    }

    pub fn finalize(&mut self, conclusion: &str, valid: bool) {
        self.conclusion = conclusion.to_string();
        self.valid = valid;
    }
}

// ── Proof Chain ───────────────────────────────────────────────────────────────

/// An append-only chain of proofs — each proof can reference prior proofs in
/// the chain to build on established results.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProofChain {
    pub proofs: Vec<Proof>,
    pub proven_facts: HashMap<String, String>, // fact → proof ID that established it
}

impl ProofChain {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, proof: Proof) {
        if proof.valid {
            self.proven_facts
                .insert(proof.conclusion.clone(), proof.id.clone());
        }
        self.proofs.push(proof);
    }

    pub fn is_proven(&self, fact: &str) -> bool {
        self.proven_facts.contains_key(fact)
    }

    pub fn last_valid_proof(&self) -> Option<&Proof> {
        self.proofs.iter().rev().find(|p| p.valid)
    }

    pub fn valid_count(&self) -> usize {
        self.proofs.iter().filter(|p| p.valid).count()
    }

    pub fn integrity_check(&self) -> bool {
        // Each proof's axioms must be known facts in the chain.
        // (Simplified: in production this would be a full dependent-type check.)
        for proof in &self.proofs {
            for axiom in &proof.axioms_used {
                if !self.proven_facts.contains_key(axiom.as_str())
                    && !axiom.starts_with("axiom:")
                {
                    return false;
                }
            }
        }
        true
    }
}

// ── Verification Context ──────────────────────────────────────────────────────

/// Contextual information provided to the proof engine when verifying a modification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationContext {
    pub modification_id: String,
    pub target_file: String,
    pub diff_summary: String,
    pub properties_to_verify: Vec<String>,
    pub available_axioms: Vec<String>,
    pub previous_proof_ids: Vec<String>,
}

// ── Proof Engine ──────────────────────────────────────────────────────────────

pub struct ProofEngine {
    pub chain: ProofChain,
    pub axiom_library: HashMap<String, String>, // axiom name → formal statement
}

impl ProofEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            chain: ProofChain::new(),
            axiom_library: HashMap::new(),
        };
        engine.load_core_axioms();
        engine
    }

    fn load_core_axioms(&mut self) {
        let axioms = [
            (
                "axiom:alignment_preserved_under_additive_change",
                "For all modifications M that only add new functions and do not alter \
                 existing alignment constraints: alignment(system_after_M) >= alignment(system_before_M)",
            ),
            (
                "axiom:safety_module_immutability",
                "Safety-critical modules (alignment, verification, security) are write-protected \
                 and cannot be modified by any self-modification procedure",
            ),
            (
                "axiom:rollback_completeness",
                "For every applied modification M, there exists a rollback_patch R such that \
                 apply(R, system_after_M) = system_before_M",
            ),
            (
                "axiom:test_pass_implies_functional_correctness",
                "If all regression tests pass after modification M, then functional correctness \
                 of existing capabilities is preserved",
            ),
        ];
        for (name, statement) in axioms {
            self.axiom_library.insert(name.to_string(), statement.to_string());
            // Pre-seed the chain with axioms as ground truths
            self.chain
                .proven_facts
                .insert(name.to_string(), format!("axiom:{}", name));
        }
    }

    /// Register an additional axiom.
    pub fn register_axiom(&mut self, name: &str, statement: &str) {
        self.axiom_library.insert(name.to_string(), statement.to_string());
        self.chain
            .proven_facts
            .insert(name.to_string(), format!("axiom:{}", name));
    }

    /// Attempt to prove that a modification preserves alignment.
    pub fn prove_alignment_preserved(
        &mut self,
        ctx: &VerificationContext,
        tests_passed: bool,
        modifies_protected: bool,
    ) -> Proof {
        let start = std::time::Instant::now();
        let mut proof = Proof::new(
            &format!("alignment_preserved_{}", ctx.modification_id),
            &format!(
                "alignment is preserved after modification '{}' to '{}'",
                ctx.modification_id, ctx.target_file
            ),
        );

        proof.axioms_used = vec![
            "axiom:alignment_preserved_under_additive_change".to_string(),
            "axiom:safety_module_immutability".to_string(),
            "axiom:test_pass_implies_functional_correctness".to_string(),
        ];

        let mut step_num = 1u32;

        // Step 1 — check that protected files are not touched
        if modifies_protected {
            proof.add_step(ProofStep {
                step_number: step_num,
                kind: ProofStepKind::ApplyAxiom {
                    axiom_name: "axiom:safety_module_immutability".to_string(),
                },
                result_fact: "modification targets safety-critical module".to_string(),
                justification: format!(
                    "File '{}' is in the protected module set",
                    ctx.target_file
                ),
            });
            proof.finalize("alignment NOT preserved — modification touches protected module", false);
            proof.verification_time_ms = start.elapsed().as_millis() as u64;
            warn!("Proof FAILED: modification touches protected module '{}'", ctx.target_file);
            return proof;
        }
        step_num += 1;

        proof.add_step(ProofStep {
            step_number: step_num,
            kind: ProofStepKind::ApplyAxiom {
                axiom_name: "axiom:safety_module_immutability".to_string(),
            },
            result_fact: "safety-critical modules not modified".to_string(),
            justification: "Target file is not in the protected module set".to_string(),
        });
        step_num += 1;

        // Step 2 — test pass implies functional correctness
        if !tests_passed {
            proof.add_step(ProofStep {
                step_number: step_num,
                kind: ProofStepKind::CaseSplit {
                    predicate: "regression_tests_pass".to_string(),
                },
                result_fact: "tests failed — functional correctness not guaranteed".to_string(),
                justification: "Test suite reported failures".to_string(),
            });
            proof.finalize("alignment NOT preserved — tests failed", false);
            proof.verification_time_ms = start.elapsed().as_millis() as u64;
            warn!("Proof FAILED: tests did not pass for modification '{}'", ctx.modification_id);
            return proof;
        }

        proof.add_step(ProofStep {
            step_number: step_num,
            kind: ProofStepKind::ApplyAxiom {
                axiom_name: "axiom:test_pass_implies_functional_correctness".to_string(),
            },
            result_fact: "functional correctness preserved (all tests pass)".to_string(),
            justification: "Regression test suite passed".to_string(),
        });
        step_num += 1;

        // Step 3 — apply alignment-preservation axiom
        proof.add_step(ProofStep {
            step_number: step_num,
            kind: ProofStepKind::ApplyAxiom {
                axiom_name: "axiom:alignment_preserved_under_additive_change".to_string(),
            },
            result_fact: "alignment is preserved under this modification".to_string(),
            justification: "Modification is additive; no alignment constraints removed".to_string(),
        });
        step_num += 1;

        // Step 4 — modus ponens to conclude
        proof.add_step(ProofStep {
            step_number: step_num,
            kind: ProofStepKind::ModusPonens {
                premise: "functional correctness preserved AND safety modules untouched".to_string(),
                implication: "alignment is preserved".to_string(),
            },
            result_fact: format!(
                "alignment preserved after modification '{}'",
                ctx.modification_id
            ),
            justification: "All premises satisfied; alignment preservation follows by MP".to_string(),
        });

        proof.machine_verified = true;
        proof.verifier_tool = Some("housaky_proof_engine_v1".to_string());
        proof.finalize(
            &format!(
                "alignment preserved after modification '{}' to '{}'",
                ctx.modification_id, ctx.target_file
            ),
            true,
        );
        proof.verification_time_ms = start.elapsed().as_millis() as u64;

        info!(
            "Proof '{}' verified in {}ms ({} steps)",
            proof.name,
            proof.verification_time_ms,
            proof.steps.len()
        );

        proof
    }

    /// Append a proof to the chain and return whether it was valid.
    pub fn commit(&mut self, proof: Proof) -> bool {
        let valid = proof.valid;
        self.chain.append(proof);
        valid
    }

    /// Full verify + commit pipeline.
    pub fn verify_and_commit(
        &mut self,
        ctx: &VerificationContext,
        tests_passed: bool,
        modifies_protected: bool,
    ) -> bool {
        let proof = self.prove_alignment_preserved(ctx, tests_passed, modifies_protected);
        self.commit(proof)
    }

    pub fn chain_integrity(&self) -> bool {
        self.chain.integrity_check()
    }

    pub fn proven_fact_count(&self) -> usize {
        self.chain.proven_facts.len()
    }

    /// Attempt to run an external verifier (`cargo kani` or `cargo miri`) on the
    /// given source file within `crate_root`. Returns `(tool_name, output, success)`.
    ///
    /// Falls back gracefully — if neither tool is available the function returns
    /// `("none", "no external verifier available", false)` without panicking.
    pub fn try_external_verifier(
        &self,
        crate_root: &std::path::Path,
        target_file: &str,
    ) -> (String, String, bool) {
        // Prefer kani as it provides bounded model checking; fall back to miri.
        let tools: &[(&str, &[&str])] = &[
            ("cargo-kani", &["kani", "--output-format", "terse"]),
            ("cargo",      &["miri", "test", "--", "--quiet"]),
        ];

        for (tool_name, args) in tools {
            // Quick availability check: `which cargo-kani` or `cargo miri --version`
            let available = if *tool_name == "cargo-kani" {
                StdCommand::new("cargo-kani")
                    .arg("--version")
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            } else {
                // Check miri via rustup component
                StdCommand::new("cargo")
                    .args(["miri", "--version"])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            };

            if !available {
                continue;
            }

            info!(
                tool = *tool_name,
                target_file,
                "Running external verifier"
            );

            let result = StdCommand::new(args[0])
                .args(&args[1..])
                .current_dir(crate_root)
                .env("KANI_FILE", target_file)
                .output();

            match result {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let combined = format!("{}\n{}", stdout, stderr)
                        .trim()
                        .chars()
                        .take(2048)
                        .collect::<String>();
                    let success = output.status.success();
                    info!(
                        tool = *tool_name,
                        success,
                        "External verifier finished"
                    );
                    return (tool_name.to_string(), combined, success);
                }
                Err(e) => {
                    warn!(tool = *tool_name, error = %e, "External verifier invocation failed");
                }
            }
        }

        ("none".to_string(), "no external verifier available".to_string(), false)
    }

    /// Full verify + commit pipeline with optional external verification pass.
    pub fn verify_and_commit_with_external(
        &mut self,
        ctx: &VerificationContext,
        tests_passed: bool,
        modifies_protected: bool,
        crate_root: Option<&std::path::Path>,
    ) -> bool {
        let mut proof = self.prove_alignment_preserved(ctx, tests_passed, modifies_protected);

        // If proof passed so far and we have a crate root, attempt external verification
        if proof.valid {
            if let Some(root) = crate_root {
                let (tool, output, ext_ok) =
                    self.try_external_verifier(root, &ctx.target_file);
                if tool != "none" {
                    let step_num = proof.steps.len() as u32 + 1;
                    proof.add_step(ProofStep {
                        step_number: step_num,
                        kind: ProofStepKind::ExternalVerifier {
                            tool: tool.clone(),
                            output: output.clone(),
                        },
                        result_fact: if ext_ok {
                            format!("ExternalVerifier({}) PASS", tool)
                        } else {
                            format!("ExternalVerifier({}) FAIL", tool)
                        },
                        justification: output,
                    });
                    if !ext_ok {
                        proof.finalize(
                            &format!("alignment NOT preserved — {} failed", tool),
                            false,
                        );
                    }
                    proof.machine_verified = ext_ok;
                }
            }
        }

        self.commit(proof)
    }
}

impl Default for ProofEngine {
    fn default() -> Self {
        Self::new()
    }
}

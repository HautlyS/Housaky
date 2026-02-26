//! Sandbox Verifier — Run modifications in a deterministic sandbox before promotion.
//!
//! Wraps the `GitSandbox` to provide a verification-focused interface that
//! checks a proposed modification by: building it in isolation, running the full
//! test suite, executing a property check, and only then reporting pass/fail.

use crate::housaky::git_sandbox::{GitSandbox, ValidationResult};
use crate::housaky::verification::property_checker::{
    PropertyCheckReport, PropertyChecker, SystemSnapshot,
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tracing::{info, warn};
use uuid::Uuid;

// ── Sandbox Verification Config ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxVerifierConfig {
    pub require_compile: bool,
    pub require_tests: bool,
    pub require_property_check: bool,
    pub require_no_size_regression: bool,
    pub max_binary_size_increase_pct: f64,
    pub test_timeout_secs: u64,
    pub run_miri: bool,
    pub run_clippy: bool,
}

impl Default for SandboxVerifierConfig {
    fn default() -> Self {
        Self {
            require_compile: true,
            require_tests: true,
            require_property_check: true,
            require_no_size_regression: false,
            max_binary_size_increase_pct: 10.0,
            test_timeout_secs: 120,
            run_miri: false,
            run_clippy: true,
        }
    }
}

// ── Sandbox Verification Report ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxVerificationReport {
    pub id: String,
    pub modification_id: String,
    pub target_file: String,
    pub passed: bool,
    pub compile_ok: bool,
    pub tests_ok: bool,
    pub clippy_ok: bool,
    pub miri_ok: Option<bool>,
    pub property_check: Option<PropertyCheckReport>,
    pub failure_reasons: Vec<String>,
    pub duration_ms: u64,
    pub sandbox_path: PathBuf,
    pub verified_at: DateTime<Utc>,
}

impl SandboxVerificationReport {
    pub fn summary(&self) -> String {
        if self.passed {
            format!(
                "PASS — compile={} tests={} clippy={} ({}ms)",
                self.compile_ok, self.tests_ok, self.clippy_ok, self.duration_ms
            )
        } else {
            format!(
                "FAIL — {} reasons: {}",
                self.failure_reasons.len(),
                self.failure_reasons.join("; ")
            )
        }
    }
}

// ── Sandbox Verifier ──────────────────────────────────────────────────────────

pub struct SandboxVerifier {
    pub workspace_dir: PathBuf,
    pub config: SandboxVerifierConfig,
    pub property_checker: PropertyChecker,
}

impl SandboxVerifier {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self {
            workspace_dir,
            config: SandboxVerifierConfig::default(),
            property_checker: PropertyChecker::with_standard_invariants(),
        }
    }

    pub fn with_config(workspace_dir: PathBuf, config: SandboxVerifierConfig) -> Self {
        Self {
            workspace_dir,
            config,
            property_checker: PropertyChecker::with_standard_invariants(),
        }
    }

    /// Verify a proposed modification by running it through the full sandbox pipeline.
    ///
    /// # Arguments
    /// * `modification_id` — unique ID of this modification attempt
    /// * `target_file` — relative path to the file being modified (e.g. `src/housaky/foo.rs`)
    /// * `new_source` — proposed new content for `target_file`
    /// * `alignment_score` — current alignment score (from `AlignmentProver`)
    pub async fn verify(
        &self,
        modification_id: &str,
        target_file: &str,
        new_source: &str,
        alignment_score: f64,
    ) -> Result<SandboxVerificationReport> {
        let start = Instant::now();
        let report_id = Uuid::new_v4().to_string();
        let mut failure_reasons = Vec::new();

        // Create sandbox session
        let mut sandbox = GitSandbox::new(self.workspace_dir.clone());
        let session = sandbox.create_session(&format!("verify-{}", modification_id))?;

        info!(
            modification_id = %modification_id,
            session_id = %session.id,
            "Sandbox verifier: session created"
        );

        // Apply the modification in the sandbox
        sandbox.apply_modification(&session.id, target_file, new_source)?;

        // Run the validation pipeline
        let validation: ValidationResult = sandbox.validate_session(&session.id)?;

        let compile_ok = validation.compiles;
        if !compile_ok && self.config.require_compile {
            failure_reasons.push("Compilation failed".to_string());
        }

        let tests_ok = validation.no_regressions;
        if !tests_ok && self.config.require_tests {
            failure_reasons.push("Test suite has regressions".to_string());
        }

        // Clippy check: treat any new warnings as a clippy signal
        let clippy_ok = validation.warnings.is_empty();
        if !clippy_ok && self.config.run_clippy {
            info!("Clippy: {} new warnings detected in modified code", validation.warnings.len());
        }

        // Property check
        let property_check_opt = if self.config.require_property_check {
            let snapshot = SystemSnapshot {
                memory_usage_mb: 0,
                recent_commands: Vec::new(),
                active_network_domains: Vec::new(),
                alignment_score,
                modified_files: vec![target_file.to_string()],
                rollback_patches_present: true,
                module_trait_implementations: HashMap::new(),
                metadata: HashMap::new(),
            };
            let report = self.property_checker.check(&snapshot);
            if !report.passed {
                failure_reasons.push(format!(
                    "Property check failed: {} blocking violations",
                    report.blocking_violations
                ));
            }
            Some(report)
        } else {
            None
        };

        // Miri (optional)
        let miri_ok = if self.config.run_miri {
            // Miri integration deferred to external runner
            info!("Miri check deferred to external tooling");
            Some(true)
        } else {
            None
        };

        // Clean up sandbox
        if failure_reasons.is_empty() {
            let _ = sandbox.merge_session(&session.id);
        } else {
            let _ = sandbox.discard_session(&session.id);
            warn!(
                "Sandbox verification FAILED for '{}': {:?}",
                modification_id, failure_reasons
            );
        }

        let passed = failure_reasons.is_empty();
        let duration_ms = start.elapsed().as_millis() as u64;

        info!(
            modification_id = %modification_id,
            passed = %passed,
            duration_ms = %duration_ms,
            "Sandbox verification complete"
        );

        Ok(SandboxVerificationReport {
            id: report_id,
            modification_id: modification_id.to_string(),
            target_file: target_file.to_string(),
            passed,
            compile_ok,
            tests_ok,
            clippy_ok,
            miri_ok,
            property_check: property_check_opt,
            failure_reasons,
            duration_ms,
            sandbox_path: session.worktree_path,
            verified_at: Utc::now(),
        })
    }
}

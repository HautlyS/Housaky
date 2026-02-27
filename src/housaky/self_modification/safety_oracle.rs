use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyReport {
    pub passed: bool,
    pub clippy_clean: bool,
    pub no_unsafe_additions: bool,
    pub no_forbidden_patterns: bool,
    pub warnings: Vec<String>,
    pub violations: Vec<SafetyViolation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyViolation {
    pub kind: ViolationKind,
    pub location: String,
    pub description: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ViolationKind {
    ForbiddenPattern,
    UnsafeBlock,
    ForbiddenModule,
    ClippyError,
    UndefinedBehavior,
    /// DGM §8.5 — mutation attempts to modify the fitness/safety evaluation itself.
    RewardHacking,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    Block,
    Warn,
    Info,
}

pub struct SafetyOracle {
    pub forbidden_patterns: Vec<String>,
    pub forbidden_modules: Vec<String>,
    pub allow_unsafe: bool,
}

impl SafetyOracle {
    pub fn new() -> Self {
        Self {
            forbidden_patterns: vec![
                "std::process::exit".to_string(),
                "std::fs::remove_dir_all".to_string(),
                "exec_into_new_binary".to_string(),
                "unsafe { *(0 as".to_string(),
                // DGM §8.5 — anti-reward-hacking: prevent mutations from
                // modifying the fitness evaluation, safety oracle, alignment
                // gate, or capability retention suite.
                "capability_retention_suite".to_string(),
                "alignment_gate".to_string(),
                "FitnessEvaluator".to_string(),
                "SafetyOracle".to_string(),
                "fn run_capability_retention".to_string(),
                "fn compute_fitness".to_string(),
                "fn evaluate(".to_string(),
            ],
            forbidden_modules: vec![
                "security".to_string(),
                "alignment".to_string(),
                // Prevent mutations from modifying safety/fitness modules directly.
                "safety_oracle".to_string(),
                "fitness_eval".to_string(),
            ],
            allow_unsafe: false,
        }
    }

    pub fn with_config(
        forbidden_patterns: Vec<String>,
        forbidden_modules: Vec<String>,
        allow_unsafe: bool,
    ) -> Self {
        Self {
            forbidden_patterns,
            forbidden_modules,
            allow_unsafe,
        }
    }

    /// Run clippy on the worktree and collect lint results.
    pub fn run_clippy(&self, worktree_path: &Path) -> Result<Vec<SafetyViolation>> {
        let output = Command::new("cargo")
            .args(["clippy", "--lib", "--", "-D", "warnings"])
            .current_dir(worktree_path)
            .env("CARGO_TERM_COLOR", "never")
            .output()?;

        let mut violations = Vec::new();
        let stderr = String::from_utf8_lossy(&output.stderr);

        for line in stderr.lines() {
            if line.contains("error[") {
                violations.push(SafetyViolation {
                    kind: ViolationKind::ClippyError,
                    location: extract_location(line),
                    description: line.to_string(),
                    severity: Severity::Block,
                });
            } else if line.contains("warning[") {
                violations.push(SafetyViolation {
                    kind: ViolationKind::ClippyError,
                    location: extract_location(line),
                    description: line.to_string(),
                    severity: Severity::Warn,
                });
            }
        }

        info!("Clippy: {} violations found", violations.len());
        Ok(violations)
    }

    /// Scan source diff/content for forbidden patterns.
    pub fn scan_source(&self, source: &str, file_path: &str) -> Vec<SafetyViolation> {
        let mut violations = Vec::new();

        // Check forbidden module access
        for forbidden in &self.forbidden_modules {
            if file_path.contains(forbidden.as_str()) {
                violations.push(SafetyViolation {
                    kind: ViolationKind::ForbiddenModule,
                    location: file_path.to_string(),
                    description: format!("File is in forbidden module '{}'", forbidden),
                    severity: Severity::Block,
                });
            }
        }

        for (line_no, line) in source.lines().enumerate() {
            // Check forbidden patterns
            for pattern in &self.forbidden_patterns {
                if line.contains(pattern.as_str()) {
                    violations.push(SafetyViolation {
                        kind: ViolationKind::ForbiddenPattern,
                        location: format!("{}:{}", file_path, line_no + 1),
                        description: format!("Forbidden pattern '{}' detected", pattern),
                        severity: Severity::Block,
                    });
                }
            }

            // Check for new unsafe blocks
            if !self.allow_unsafe && line.trim().starts_with("unsafe {") {
                violations.push(SafetyViolation {
                    kind: ViolationKind::UnsafeBlock,
                    location: format!("{}:{}", file_path, line_no + 1),
                    description: "New unsafe block detected".to_string(),
                    severity: Severity::Warn,
                });
            }

            // DGM §8.5 — detect reward-hacking patterns: code that tries to
            // short-circuit fitness checks or override safety verdicts.
            let reward_hack_patterns = [
                "fitness_score = 1.0",
                "alignment_score = 1.0",
                "passed = true",
                ".passed = true",
                "override_safety",
                "skip_alignment",
                "disable_safety",
                "bypass_gate",
            ];
            for rh_pat in &reward_hack_patterns {
                if line.contains(rh_pat) {
                    violations.push(SafetyViolation {
                        kind: ViolationKind::RewardHacking,
                        location: format!("{}:{}", file_path, line_no + 1),
                        description: format!(
                            "Potential reward-hacking pattern '{}' detected (DGM §8.5)",
                            rh_pat
                        ),
                        severity: Severity::Block,
                    });
                }
            }
        }

        violations
    }

    pub fn evaluate(
        &self,
        worktree_path: &Path,
        changed_sources: &[(String, String)],
    ) -> SafetyReport {
        let mut all_violations = Vec::new();

        // Scan all changed sources
        for (file_path, source) in changed_sources {
            let src_violations = self.scan_source(source, file_path);
            all_violations.extend(src_violations);
        }

        // Run clippy if worktree exists
        if worktree_path.exists() {
            match self.run_clippy(worktree_path) {
                Ok(clippy_violations) => all_violations.extend(clippy_violations),
                Err(e) => {
                    warn!("Clippy failed: {}", e);
                }
            }
        }

        let blocking: Vec<_> = all_violations
            .iter()
            .filter(|v| v.severity == Severity::Block)
            .collect();
        let warnings: Vec<String> = all_violations
            .iter()
            .filter(|v| v.severity == Severity::Warn)
            .map(|v| v.description.clone())
            .collect();

        let clippy_clean = all_violations
            .iter()
            .filter(|v| v.kind == ViolationKind::ClippyError && v.severity == Severity::Block)
            .count()
            == 0;

        let no_unsafe_additions = all_violations
            .iter()
            .filter(|v| v.kind == ViolationKind::UnsafeBlock && v.severity == Severity::Block)
            .count()
            == 0;

        let no_forbidden_patterns = all_violations
            .iter()
            .filter(|v| {
                v.kind == ViolationKind::ForbiddenPattern
                    || v.kind == ViolationKind::ForbiddenModule
            })
            .count()
            == 0;

        SafetyReport {
            passed: blocking.is_empty(),
            clippy_clean,
            no_unsafe_additions,
            no_forbidden_patterns,
            warnings,
            violations: all_violations,
        }
    }
}

fn extract_location(line: &str) -> String {
    if let Some(start) = line.find("-->") {
        line[start + 3..].trim().to_string()
    } else {
        String::new()
    }
}

impl Default for SafetyOracle {
    fn default() -> Self {
        Self::new()
    }
}

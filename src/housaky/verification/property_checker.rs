//! Property Checker — Specify and check invariants that must hold across all modifications.
//!
//! Defines the `SafetyInvariant` type and a `PropertyChecker` that evaluates a set
//! of invariants against a candidate modification or a running system snapshot.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

// ── Invariant Severity ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InvariantSeverity {
    /// Violation blocks the modification from being applied.
    Block,
    /// Violation emits a warning but does not block.
    Warn,
    /// Violation is logged for audit purposes only.
    Log,
}

// ── Invariant Property ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InvariantProperty {
    /// The agent never executes commands outside the allowed list.
    CommandAllowlist { allowed: Vec<String> },
    /// Memory usage stays below the configured ceiling.
    MemoryBound { max_mb: u64 },
    /// No network access outside allowed domains.
    NetworkIsolation { allowed_domains: Vec<String> },
    /// Value-drift detector score stays above threshold.
    AlignmentMinimum { min_score: f64 },
    /// Self-modifications never touch safety-critical modules.
    SafetyModuleImmutability { protected_paths: Vec<String> },
    /// Every self-modification is reversible (rollback patch exists).
    ReversibilityGuarantee,
    /// The module must implement a required trait.
    RequiredTraitImpl { trait_name: String, module_name: String },
    /// Custom predicate expressed as a description string (checked externally).
    Custom { predicate: String },
}

impl InvariantProperty {
    pub fn name(&self) -> &'static str {
        match self {
            InvariantProperty::CommandAllowlist { .. } => "command_allowlist",
            InvariantProperty::MemoryBound { .. } => "memory_bound",
            InvariantProperty::NetworkIsolation { .. } => "network_isolation",
            InvariantProperty::AlignmentMinimum { .. } => "alignment_minimum",
            InvariantProperty::SafetyModuleImmutability { .. } => "safety_module_immutability",
            InvariantProperty::ReversibilityGuarantee => "reversibility_guarantee",
            InvariantProperty::RequiredTraitImpl { .. } => "required_trait_impl",
            InvariantProperty::Custom { .. } => "custom",
        }
    }
}

// ── Safety Invariant ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyInvariant {
    pub id: String,
    pub name: String,
    pub property: InvariantProperty,
    pub severity: InvariantSeverity,
    pub description: String,
    pub enabled: bool,
}

impl SafetyInvariant {
    pub fn new(name: &str, property: InvariantProperty, severity: InvariantSeverity) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: String::new(),
            property,
            severity,
            enabled: true,
        }
    }

    pub fn blocking(name: &str, property: InvariantProperty) -> Self {
        Self::new(name, property, InvariantSeverity::Block)
    }
}

// ── Check Evidence ────────────────────────────────────────────────────────────

/// The runtime snapshot against which invariants are evaluated.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub memory_usage_mb: u64,
    pub recent_commands: Vec<String>,
    pub active_network_domains: Vec<String>,
    pub alignment_score: f64,
    pub modified_files: Vec<String>,
    pub rollback_patches_present: bool,
    pub module_trait_implementations: HashMap<String, Vec<String>>,
    pub metadata: HashMap<String, String>,
}

// ── Check Result ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantCheckResult {
    pub invariant_id: String,
    pub invariant_name: String,
    pub passed: bool,
    pub severity: InvariantSeverity,
    pub violation_message: Option<String>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyCheckReport {
    pub passed: bool,
    pub results: Vec<InvariantCheckResult>,
    pub blocking_violations: usize,
    pub warning_violations: usize,
    pub checked_at: DateTime<Utc>,
    pub snapshot_summary: String,
}

impl PropertyCheckReport {
    pub fn violations(&self) -> impl Iterator<Item = &InvariantCheckResult> {
        self.results.iter().filter(|r| !r.passed)
    }
}

// ── Property Checker ──────────────────────────────────────────────────────────

pub struct PropertyChecker {
    pub invariants: Vec<SafetyInvariant>,
}

impl PropertyChecker {
    pub fn new() -> Self {
        Self { invariants: Vec::new() }
    }

    /// Load the standard set of safety invariants.
    pub fn with_standard_invariants() -> Self {
        let mut pc = Self::new();

        pc.add(SafetyInvariant::blocking(
            "safety_module_immutability",
            InvariantProperty::SafetyModuleImmutability {
                protected_paths: vec![
                    "src/housaky/alignment/".to_string(),
                    "src/security/".to_string(),
                    "src/housaky/verification/".to_string(),
                ],
            },
        ));

        pc.add(SafetyInvariant::blocking(
            "alignment_minimum",
            InvariantProperty::AlignmentMinimum { min_score: 0.70 },
        ));

        pc.add(SafetyInvariant::blocking(
            "reversibility_guarantee",
            InvariantProperty::ReversibilityGuarantee,
        ));

        pc.add(SafetyInvariant::new(
            "memory_bound",
            InvariantProperty::MemoryBound { max_mb: 8192 },
            InvariantSeverity::Warn,
        ));

        pc.add(SafetyInvariant::new(
            "network_isolation",
            InvariantProperty::NetworkIsolation {
                allowed_domains: vec![
                    "api.openai.com".to_string(),
                    "api.anthropic.com".to_string(),
                    "github.com".to_string(),
                ],
            },
            InvariantSeverity::Warn,
        ));

        pc
    }

    pub fn add(&mut self, invariant: SafetyInvariant) {
        self.invariants.push(invariant);
    }

    pub fn remove(&mut self, invariant_id: &str) {
        self.invariants.retain(|i| i.id != invariant_id);
    }

    /// Check all invariants against a system snapshot.
    pub fn check(&self, snapshot: &SystemSnapshot) -> PropertyCheckReport {
        let mut results = Vec::new();
        let mut blocking = 0usize;
        let mut warnings = 0usize;

        for inv in &self.invariants {
            if !inv.enabled {
                continue;
            }
            let result = self.check_invariant(inv, snapshot);
            if !result.passed {
                match result.severity {
                    InvariantSeverity::Block => blocking += 1,
                    InvariantSeverity::Warn => warnings += 1,
                    InvariantSeverity::Log => {}
                }
                warn!(
                    "Invariant '{}' VIOLATED: {:?}",
                    result.invariant_name, result.violation_message
                );
            } else {
                debug!("Invariant '{}' OK", result.invariant_name);
            }
            results.push(result);
        }

        let passed = blocking == 0;
        info!(
            "Property check: {} invariants, {} blocking violations, {} warnings — {}",
            results.len(),
            blocking,
            warnings,
            if passed { "PASS" } else { "FAIL" }
        );

        PropertyCheckReport {
            passed,
            results,
            blocking_violations: blocking,
            warning_violations: warnings,
            checked_at: Utc::now(),
            snapshot_summary: format!(
                "mem={}MB align={:.2} cmds={} domains={}",
                snapshot.memory_usage_mb,
                snapshot.alignment_score,
                snapshot.recent_commands.len(),
                snapshot.active_network_domains.len()
            ),
        }
    }

    fn check_invariant(
        &self,
        inv: &SafetyInvariant,
        snapshot: &SystemSnapshot,
    ) -> InvariantCheckResult {
        let (passed, message) = match &inv.property {
            InvariantProperty::CommandAllowlist { allowed } => {
                let violations: Vec<String> = snapshot
                    .recent_commands
                    .iter()
                    .filter(|cmd| {
                        !allowed.iter().any(|a| cmd.starts_with(a.as_str()))
                    })
                    .cloned()
                    .collect();
                if violations.is_empty() {
                    (true, None)
                } else {
                    (
                        false,
                        Some(format!("Disallowed commands: {:?}", violations)),
                    )
                }
            }

            InvariantProperty::MemoryBound { max_mb } => {
                if snapshot.memory_usage_mb <= *max_mb {
                    (true, None)
                } else {
                    (
                        false,
                        Some(format!(
                            "Memory {} MB exceeds limit {} MB",
                            snapshot.memory_usage_mb, max_mb
                        )),
                    )
                }
            }

            InvariantProperty::NetworkIsolation { allowed_domains } => {
                let violations: Vec<String> = snapshot
                    .active_network_domains
                    .iter()
                    .filter(|d| !allowed_domains.iter().any(|a| d.contains(a.as_str())))
                    .cloned()
                    .collect();
                if violations.is_empty() {
                    (true, None)
                } else {
                    (
                        false,
                        Some(format!("Disallowed domains: {:?}", violations)),
                    )
                }
            }

            InvariantProperty::AlignmentMinimum { min_score } => {
                if snapshot.alignment_score >= *min_score {
                    (true, None)
                } else {
                    (
                        false,
                        Some(format!(
                            "Alignment score {:.3} < minimum {:.3}",
                            snapshot.alignment_score, min_score
                        )),
                    )
                }
            }

            InvariantProperty::SafetyModuleImmutability { protected_paths } => {
                let violations: Vec<String> = snapshot
                    .modified_files
                    .iter()
                    .filter(|f| {
                        protected_paths.iter().any(|p| f.starts_with(p.as_str()))
                    })
                    .cloned()
                    .collect();
                if violations.is_empty() {
                    (true, None)
                } else {
                    (
                        false,
                        Some(format!(
                            "Protected files modified: {:?}",
                            violations
                        )),
                    )
                }
            }

            InvariantProperty::ReversibilityGuarantee => {
                if snapshot.rollback_patches_present || snapshot.modified_files.is_empty() {
                    (true, None)
                } else {
                    (
                        false,
                        Some("Modifications made without rollback patches".to_string()),
                    )
                }
            }

            InvariantProperty::RequiredTraitImpl { trait_name, module_name } => {
                let has_impl = snapshot
                    .module_trait_implementations
                    .get(module_name.as_str())
                    .map(|traits| traits.iter().any(|t| t == trait_name))
                    .unwrap_or(false);
                if has_impl {
                    (true, None)
                } else {
                    (
                        false,
                        Some(format!(
                            "Module '{}' does not implement trait '{}'",
                            module_name, trait_name
                        )),
                    )
                }
            }

            InvariantProperty::Custom { predicate } => {
                // Custom predicates are evaluated externally; pass through with a note.
                (
                    true,
                    Some(format!("Custom predicate deferred to external verifier: {}", predicate)),
                )
            }
        };

        InvariantCheckResult {
            invariant_id: inv.id.clone(),
            invariant_name: inv.name.clone(),
            passed,
            severity: inv.severity.clone(),
            violation_message: message,
            checked_at: Utc::now(),
        }
    }
}

impl Default for PropertyChecker {
    fn default() -> Self {
        Self::new()
    }
}

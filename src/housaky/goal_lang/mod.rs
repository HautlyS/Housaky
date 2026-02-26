//! Goal Specification Language (GoalLang)
//!
//! Formal goal specification with verification:
//! - DSL for goal specs with preconditions, postconditions, invariants
//! - Goal consistency verification (no contradictory subgoals)
//! - Achievability check against current capabilities
//! - LLM-readable description generation from formal specs

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalSpec {
    pub name: String,
    pub description: String,
    pub requires: Vec<Requirement>,
    pub preconditions: Vec<Condition>,
    pub postconditions: Vec<Condition>,
    pub invariants: Vec<Condition>,
    pub subgoals: Vec<SubGoalSpec>,
    pub decomposition: DecompositionStrategy,
    pub priority: u32,
    pub deadline: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubGoalSpec {
    pub name: String,
    pub description: String,
    pub preconditions: Vec<Condition>,
    pub postconditions: Vec<Condition>,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Requirement {
    Tool(String),
    Capability(String),
    Resource { name: String, min_amount: f64 },
    Permission(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub expression: String,
    pub description: String,
    pub checkable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecompositionStrategy {
    Sequential,
    Parallel,
    Conditional { condition: String },
    Iterative { max_iterations: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub goal_name: String,
    pub is_consistent: bool,
    pub is_achievable: bool,
    pub issues: Vec<VerificationIssue>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationIssue {
    pub severity: IssueSeverity,
    pub description: String,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

pub struct GoalLangEngine {
    pub goal_specs: Arc<RwLock<HashMap<String, GoalSpec>>>,
    pub available_tools: Arc<RwLock<Vec<String>>>,
    pub available_capabilities: Arc<RwLock<Vec<String>>>,
    pub verification_history: Arc<RwLock<Vec<VerificationResult>>>,
}

impl GoalLangEngine {
    pub fn new() -> Self {
        Self {
            goal_specs: Arc::new(RwLock::new(HashMap::new())),
            available_tools: Arc::new(RwLock::new(Vec::new())),
            available_capabilities: Arc::new(RwLock::new(Vec::new())),
            verification_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register available tools and capabilities for achievability checking.
    pub async fn register_capabilities(
        &self,
        tools: Vec<String>,
        capabilities: Vec<String>,
    ) {
        *self.available_tools.write().await = tools;
        *self.available_capabilities.write().await = capabilities;
    }

    /// Parse a goal from structured input.
    pub async fn define_goal(&self, spec: GoalSpec) -> Result<String> {
        let name = spec.name.clone();
        self.goal_specs.write().await.insert(name.clone(), spec);
        info!("Defined goal spec: '{}'", name);
        Ok(name)
    }

    /// Verify a goal specification for consistency and achievability.
    pub async fn verify(&self, goal_name: &str) -> Result<VerificationResult> {
        let specs = self.goal_specs.read().await;
        let spec = specs
            .get(goal_name)
            .ok_or_else(|| anyhow!("Goal '{}' not found", goal_name))?;

        let mut issues = Vec::new();
        let mut suggestions = Vec::new();

        // Check requirements
        let tools = self.available_tools.read().await;
        let caps = self.available_capabilities.read().await;

        for req in &spec.requires {
            match req {
                Requirement::Tool(tool) => {
                    if !tools.contains(tool) {
                        issues.push(VerificationIssue {
                            severity: IssueSeverity::Error,
                            description: format!("Required tool '{}' not available", tool),
                            location: format!("REQUIRES in '{}'", goal_name),
                        });
                        suggestions.push(format!("Install or configure tool '{}'", tool));
                    }
                }
                Requirement::Capability(cap) => {
                    if !caps.contains(cap) {
                        issues.push(VerificationIssue {
                            severity: IssueSeverity::Warning,
                            description: format!("Required capability '{}' not confirmed", cap),
                            location: format!("REQUIRES in '{}'", goal_name),
                        });
                    }
                }
                Requirement::Resource { name, min_amount } => {
                    if *min_amount > 1000.0 {
                        issues.push(VerificationIssue {
                            severity: IssueSeverity::Warning,
                            description: format!("Resource '{}' requires high amount: {}", name, min_amount),
                            location: format!("REQUIRES in '{}'", goal_name),
                        });
                    }
                }
                Requirement::Permission(perm) => {
                    issues.push(VerificationIssue {
                        severity: IssueSeverity::Info,
                        description: format!("Permission '{}' needs human approval", perm),
                        location: format!("REQUIRES in '{}'", goal_name),
                    });
                }
            }
        }

        // Check for contradictory subgoals
        self.check_subgoal_consistency(spec, &mut issues);

        // Check for circular dependencies
        self.check_circular_dependencies(spec, &mut issues);

        // Check postconditions don't violate invariants
        self.check_invariant_consistency(spec, &mut issues);

        let is_consistent = !issues.iter().any(|i| matches!(i.severity, IssueSeverity::Error));
        let is_achievable = is_consistent
            && issues
                .iter()
                .filter(|i| matches!(i.severity, IssueSeverity::Error))
                .count()
                == 0;

        let result = VerificationResult {
            goal_name: goal_name.to_string(),
            is_consistent,
            is_achievable,
            issues,
            suggestions,
        };

        self.verification_history.write().await.push(result.clone());
        Ok(result)
    }

    /// Check for contradictory subgoals.
    fn check_subgoal_consistency(&self, spec: &GoalSpec, issues: &mut Vec<VerificationIssue>) {
        // Check that subgoal postconditions don't contradict each other
        for i in 0..spec.subgoals.len() {
            for j in (i + 1)..spec.subgoals.len() {
                let a = &spec.subgoals[i];
                let b = &spec.subgoals[j];

                // Simple string-based contradiction check
                for pc_a in &a.postconditions {
                    for pc_b in &b.postconditions {
                        if self.conditions_contradict(&pc_a.expression, &pc_b.expression) {
                            issues.push(VerificationIssue {
                                severity: IssueSeverity::Error,
                                description: format!(
                                    "Subgoal '{}' postcondition contradicts '{}' postcondition",
                                    a.name, b.name
                                ),
                                location: format!("SUBGOAL {} vs {}", a.name, b.name),
                            });
                        }
                    }
                }
            }
        }
    }

    /// Check for circular subgoal dependencies.
    fn check_circular_dependencies(&self, spec: &GoalSpec, issues: &mut Vec<VerificationIssue>) {
        let mut visited = std::collections::HashSet::new();
        let mut stack = std::collections::HashSet::new();

        for subgoal in &spec.subgoals {
            if self.dfs_cycle_check(&subgoal.name, spec, &mut visited, &mut stack) {
                issues.push(VerificationIssue {
                    severity: IssueSeverity::Error,
                    description: format!("Circular dependency detected involving '{}'", subgoal.name),
                    location: "SUBGOAL dependencies".to_string(),
                });
            }
        }
    }

    fn dfs_cycle_check(
        &self,
        node: &str,
        spec: &GoalSpec,
        visited: &mut std::collections::HashSet<String>,
        stack: &mut std::collections::HashSet<String>,
    ) -> bool {
        if stack.contains(node) {
            return true;
        }
        if visited.contains(node) {
            return false;
        }

        visited.insert(node.to_string());
        stack.insert(node.to_string());

        if let Some(subgoal) = spec.subgoals.iter().find(|s| s.name == node) {
            for dep in &subgoal.depends_on {
                if self.dfs_cycle_check(dep, spec, visited, stack) {
                    return true;
                }
            }
        }

        stack.remove(node);
        false
    }

    /// Check that postconditions don't violate invariants.
    fn check_invariant_consistency(&self, spec: &GoalSpec, issues: &mut Vec<VerificationIssue>) {
        for invariant in &spec.invariants {
            for postcondition in &spec.postconditions {
                if self.conditions_contradict(&invariant.expression, &postcondition.expression) {
                    issues.push(VerificationIssue {
                        severity: IssueSeverity::Error,
                        description: format!(
                            "Postcondition '{}' may violate invariant '{}'",
                            postcondition.description, invariant.description
                        ),
                        location: "POSTCONDITION vs INVARIANT".to_string(),
                    });
                }
            }
        }
    }

    /// Simple contradiction check between two condition expressions.
    fn conditions_contradict(&self, expr_a: &str, expr_b: &str) -> bool {
        let a_lower = expr_a.to_lowercase();
        let b_lower = expr_b.to_lowercase();

        // Check for NOT/negation patterns
        (a_lower.starts_with("not ") && b_lower == a_lower.trim_start_matches("not "))
            || (b_lower.starts_with("not ") && a_lower == b_lower.trim_start_matches("not "))
            || (a_lower.contains("!") && b_lower == a_lower.replace("!", ""))
    }

    /// Generate LLM-readable description from formal spec.
    pub async fn to_natural_language(&self, goal_name: &str) -> Result<String> {
        let specs = self.goal_specs.read().await;
        let spec = specs
            .get(goal_name)
            .ok_or_else(|| anyhow!("Goal '{}' not found", goal_name))?;

        let mut parts = Vec::new();
        parts.push(format!("Goal: {}", spec.name));
        parts.push(format!("Description: {}", spec.description));

        if !spec.requires.is_empty() {
            let reqs: Vec<String> = spec
                .requires
                .iter()
                .map(|r| match r {
                    Requirement::Tool(t) => format!("tool '{}'", t),
                    Requirement::Capability(c) => format!("capability '{}'", c),
                    Requirement::Resource { name, min_amount } => format!("{} >= {}", name, min_amount),
                    Requirement::Permission(p) => format!("permission '{}'", p),
                })
                .collect();
            parts.push(format!("Requires: {}", reqs.join(", ")));
        }

        if !spec.preconditions.is_empty() {
            let pres: Vec<&str> = spec.preconditions.iter().map(|c| c.description.as_str()).collect();
            parts.push(format!("Before starting: {}", pres.join("; ")));
        }

        if !spec.postconditions.is_empty() {
            let posts: Vec<&str> = spec.postconditions.iter().map(|c| c.description.as_str()).collect();
            parts.push(format!("Success criteria: {}", posts.join("; ")));
        }

        if !spec.subgoals.is_empty() {
            parts.push(format!("Breakdown ({:?}):", spec.decomposition));
            for (i, sg) in spec.subgoals.iter().enumerate() {
                parts.push(format!("  {}. {} — {}", i + 1, sg.name, sg.description));
            }
        }

        Ok(parts.join("\n"))
    }

    pub async fn get_stats(&self) -> GoalLangStats {
        let specs = self.goal_specs.read().await;
        let history = self.verification_history.read().await;

        GoalLangStats {
            total_goals: specs.len(),
            total_verifications: history.len(),
            consistent_goals: history.iter().filter(|v| v.is_consistent).count(),
            achievable_goals: history.iter().filter(|v| v.is_achievable).count(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalLangStats {
    pub total_goals: usize,
    pub total_verifications: usize,
    pub consistent_goals: usize,
    pub achievable_goals: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_goal_definition_and_verification() {
        let engine = GoalLangEngine::new();
        engine.register_capabilities(
            vec!["shell".into(), "file_write".into()],
            vec!["rust".into()],
        ).await;

        let spec = GoalSpec {
            name: "build-app".to_string(),
            description: "Build a Rust application".to_string(),
            requires: vec![
                Requirement::Tool("shell".into()),
                Requirement::Capability("rust".into()),
            ],
            preconditions: vec![Condition {
                expression: "workspace_exists".into(),
                description: "Workspace must exist".into(),
                checkable: true,
            }],
            postconditions: vec![Condition {
                expression: "binary_built".into(),
                description: "Binary is compiled".into(),
                checkable: true,
            }],
            invariants: vec![],
            subgoals: vec![
                SubGoalSpec {
                    name: "scaffold".into(),
                    description: "Create project structure".into(),
                    preconditions: vec![],
                    postconditions: vec![],
                    depends_on: vec![],
                },
                SubGoalSpec {
                    name: "implement".into(),
                    description: "Write the code".into(),
                    preconditions: vec![],
                    postconditions: vec![],
                    depends_on: vec!["scaffold".into()],
                },
            ],
            decomposition: DecompositionStrategy::Sequential,
            priority: 1,
            deadline: None,
        };

        engine.define_goal(spec).await.unwrap();
        let result = engine.verify("build-app").await.unwrap();
        assert!(result.is_consistent);
    }
}

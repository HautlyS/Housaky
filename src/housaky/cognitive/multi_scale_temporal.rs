//! Multi-Scale Temporal Reasoning — Phase 4.4
//!
//! Extends `temporal_reasoning.rs` with multi-scale planning across:
//! microseconds (hardware reflex) → seconds → minutes → hours → days → weeks
//! → months → years → Unbounded (post-singularity open-ended improvement).
//!
//! Provides cross-scale constraint enforcement, plan alignment checking, and
//! a unified query interface across all temporal horizons.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

// ── Temporal Scale ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TemporalScale {
    Microsecond,   // Hardware reflexes, interrupt handling          <1ms
    Millisecond,   // Tool execution, API calls                      1–999ms
    Second,        // Reasoning steps, user interaction turns        1–59s
    Minute,        // Task completion, multi-step workflows          1–59min
    Hour,          // Goal achievement, project milestones           1–23h
    Day,           // Strategic planning, learning cycles            1–6d
    Week,          // Capability development, architecture evolution 1–3w
    Month,         // Research programs, self-improvement arcs       1–11mo
    Year,          // Long-term vision, singularity progress         1+ yr
    Unbounded,     // Post-singularity: open-ended self-improvement
}

impl TemporalScale {
    /// Approximate duration range for this scale.
    pub fn duration_range(&self) -> (Option<Duration>, Option<Duration>) {
        match self {
            TemporalScale::Microsecond => (None, Some(Duration::milliseconds(1))),
            TemporalScale::Millisecond => (Some(Duration::milliseconds(1)), Some(Duration::seconds(1))),
            TemporalScale::Second => (Some(Duration::seconds(1)), Some(Duration::minutes(1))),
            TemporalScale::Minute => (Some(Duration::minutes(1)), Some(Duration::hours(1))),
            TemporalScale::Hour => (Some(Duration::hours(1)), Some(Duration::days(1))),
            TemporalScale::Day => (Some(Duration::days(1)), Some(Duration::weeks(1))),
            TemporalScale::Week => (Some(Duration::weeks(1)), Some(Duration::days(30))),
            TemporalScale::Month => (Some(Duration::days(30)), Some(Duration::days(365))),
            TemporalScale::Year => (Some(Duration::days(365)), None),
            TemporalScale::Unbounded => (None, None),
        }
    }

    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            TemporalScale::Microsecond => "microsecond",
            TemporalScale::Millisecond => "millisecond",
            TemporalScale::Second => "second",
            TemporalScale::Minute => "minute",
            TemporalScale::Hour => "hour",
            TemporalScale::Day => "day",
            TemporalScale::Week => "week",
            TemporalScale::Month => "month",
            TemporalScale::Year => "year",
            TemporalScale::Unbounded => "unbounded",
        }
    }

    /// All scales in order from finest to coarsest.
    pub fn all_ordered() -> Vec<TemporalScale> {
        vec![
            TemporalScale::Microsecond,
            TemporalScale::Millisecond,
            TemporalScale::Second,
            TemporalScale::Minute,
            TemporalScale::Hour,
            TemporalScale::Day,
            TemporalScale::Week,
            TemporalScale::Month,
            TemporalScale::Year,
            TemporalScale::Unbounded,
        ]
    }

    /// Determine the appropriate scale for a given `Duration`.
    pub fn for_duration(d: Duration) -> Self {
        let ms = d.num_milliseconds();
        if ms < 1 {
            TemporalScale::Microsecond
        } else if ms < 1_000 {
            TemporalScale::Millisecond
        } else if ms < 60_000 {
            TemporalScale::Second
        } else if ms < 3_600_000 {
            TemporalScale::Minute
        } else if ms < 86_400_000 {
            TemporalScale::Hour
        } else if ms < 604_800_000 {
            TemporalScale::Day
        } else if ms < 2_592_000_000 {
            TemporalScale::Week
        } else if ms < 31_536_000_000 {
            TemporalScale::Month
        } else {
            TemporalScale::Year
        }
    }
}

// ── Temporal Plan ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalPlan {
    pub id: String,
    pub name: String,
    pub scale: TemporalScale,
    pub goal: String,
    pub actions: Vec<TemporalAction>,
    pub deadline: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub status: PlanStatus,
    pub parent_plan_id: Option<String>,
    pub child_plan_ids: Vec<String>,
    pub priority: f64,
    pub progress: f64, // 0.0 – 1.0
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    Draft,
    Active,
    Paused,
    Completed,
    Abandoned,
    BlockedByConstraint { constraint_id: String },
}

impl TemporalPlan {
    pub fn new(name: &str, scale: TemporalScale, goal: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            scale,
            goal: goal.to_string(),
            actions: Vec::new(),
            deadline: None,
            created_at: Utc::now(),
            status: PlanStatus::Draft,
            parent_plan_id: None,
            child_plan_ids: Vec::new(),
            priority: 0.5,
            progress: 0.0,
            metadata: HashMap::new(),
        }
    }

    pub fn add_action(&mut self, action: TemporalAction) {
        self.actions.push(action);
    }

    pub fn is_overdue(&self) -> bool {
        self.deadline
            .map(|d| Utc::now() > d && self.status == PlanStatus::Active)
            .unwrap_or(false)
    }

    pub fn time_remaining(&self) -> Option<Duration> {
        self.deadline.map(|d| d - Utc::now())
    }

    pub fn activate(&mut self) {
        self.status = PlanStatus::Active;
    }

    pub fn complete(&mut self) {
        self.status = PlanStatus::Completed;
        self.progress = 1.0;
    }

    pub fn update_progress(&mut self, progress: f64) {
        self.progress = progress.clamp(0.0, 1.0);
    }
}

// ── Temporal Action ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalAction {
    pub id: String,
    pub description: String,
    pub estimated_duration: Option<Duration>,
    pub dependencies: Vec<String>,  // action IDs
    pub scheduled_at: Option<DateTime<Utc>>,
    pub completed: bool,
    pub scale: TemporalScale,
}

impl TemporalAction {
    pub fn new(description: &str, scale: TemporalScale) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            description: description.to_string(),
            estimated_duration: None,
            dependencies: Vec::new(),
            scheduled_at: None,
            completed: false,
            scale,
        }
    }
}

// ── Cross-Scale Constraint ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnforcementMode {
    /// Block the fast-scale action if the slow-scale goal is violated.
    Block,
    /// Emit a warning but allow the action.
    Warn,
    /// Log for audit purposes only.
    Log,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossScaleConstraint {
    pub id: String,
    pub name: String,
    pub fast_scale: TemporalScale,
    pub slow_scale: TemporalScale,
    /// Human-readable description of the constraint.
    pub constraint_type: String,
    pub enforcement: EnforcementMode,
    pub enabled: bool,
}

impl CrossScaleConstraint {
    pub fn new(
        name: &str,
        fast: TemporalScale,
        slow: TemporalScale,
        constraint_type: &str,
        enforcement: EnforcementMode,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            fast_scale: fast,
            slow_scale: slow,
            constraint_type: constraint_type.to_string(),
            enforcement,
            enabled: true,
        }
    }
}

// ── Constraint Violation ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintViolation {
    pub constraint_id: String,
    pub constraint_name: String,
    pub fast_plan_id: String,
    pub slow_plan_id: Option<String>,
    pub message: String,
    pub enforcement: EnforcementMode,
    pub detected_at: DateTime<Utc>,
}

// ── Multi-Scale Temporal Engine ───────────────────────────────────────────────

/// The multi-scale temporal planning engine.
///
/// Maintains a plan hierarchy across all temporal scales and enforces
/// cross-scale alignment: fast actions must not contradict slow goals.
pub struct MultiScaleTemporalEngine {
    /// Plans indexed by scale, then by plan ID.
    pub active_plans: HashMap<TemporalScale, HashMap<String, TemporalPlan>>,
    pub cross_scale_constraints: Vec<CrossScaleConstraint>,
    /// Recent constraint violations.
    pub violations: Vec<ConstraintViolation>,
    pub max_violations_kept: usize,
}

impl MultiScaleTemporalEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            active_plans: HashMap::new(),
            cross_scale_constraints: Vec::new(),
            violations: Vec::new(),
            max_violations_kept: 1000,
        };
        engine.load_default_constraints();
        engine
    }

    fn load_default_constraints(&mut self) {
        self.add_constraint(CrossScaleConstraint::new(
            "fast_must_align_with_slow_goals",
            TemporalScale::Second,
            TemporalScale::Year,
            "Fast (second-scale) actions must not contradict yearly strategic goals",
            EnforcementMode::Warn,
        ));
        self.add_constraint(CrossScaleConstraint::new(
            "task_must_serve_goal",
            TemporalScale::Minute,
            TemporalScale::Hour,
            "Minute-scale tasks must contribute to hour-scale goal achievement",
            EnforcementMode::Log,
        ));
        self.add_constraint(CrossScaleConstraint::new(
            "alignment_preserved_across_scales",
            TemporalScale::Microsecond,
            TemporalScale::Unbounded,
            "Every action at any scale must preserve alignment with core values",
            EnforcementMode::Block,
        ));
        self.add_constraint(CrossScaleConstraint::new(
            "learning_serves_capability_growth",
            TemporalScale::Day,
            TemporalScale::Month,
            "Daily learning sessions must serve the monthly capability-growth arc",
            EnforcementMode::Log,
        ));
        self.add_constraint(CrossScaleConstraint::new(
            "architecture_changes_serve_singularity",
            TemporalScale::Week,
            TemporalScale::Unbounded,
            "Weekly architecture evolution must progress toward the singularity trajectory",
            EnforcementMode::Warn,
        ));
    }

    pub fn add_constraint(&mut self, constraint: CrossScaleConstraint) {
        self.cross_scale_constraints.push(constraint);
    }

    /// Register or update a plan.
    pub fn register_plan(&mut self, plan: TemporalPlan) {
        self.active_plans
            .entry(plan.scale.clone())
            .or_default()
            .insert(plan.id.clone(), plan);
    }

    /// Activate a plan by ID.
    pub fn activate_plan(&mut self, plan_id: &str) -> bool {
        for scale_plans in self.active_plans.values_mut() {
            if let Some(plan) = scale_plans.get_mut(plan_id) {
                plan.activate();
                return true;
            }
        }
        false
    }

    /// Mark a plan complete.
    pub fn complete_plan(&mut self, plan_id: &str) -> bool {
        for scale_plans in self.active_plans.values_mut() {
            if let Some(plan) = scale_plans.get_mut(plan_id) {
                plan.complete();
                info!("Plan '{}' completed at scale {:?}", plan.name, plan.scale);
                return true;
            }
        }
        false
    }

    /// Get all active plans at a given scale.
    pub fn plans_at_scale(&self, scale: &TemporalScale) -> Vec<&TemporalPlan> {
        self.active_plans
            .get(scale)
            .map(|m| m.values().filter(|p| p.status == PlanStatus::Active).collect())
            .unwrap_or_default()
    }

    /// Get all plans across all scales, ordered coarsest → finest.
    pub fn all_plans_ordered(&self) -> Vec<&TemporalPlan> {
        let mut result = Vec::new();
        for scale in TemporalScale::all_ordered().iter().rev() {
            if let Some(scale_plans) = self.active_plans.get(scale) {
                let mut plans: Vec<&TemporalPlan> = scale_plans.values().collect();
                plans.sort_by(|a, b| {
                    b.priority
                        .partial_cmp(&a.priority)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                result.extend(plans);
            }
        }
        result
    }

    /// Check whether a proposed fast-scale action conflicts with slow-scale goals.
    ///
    /// Returns a list of constraint violations; if any are `Block`-mode, the
    /// caller must prevent the action.
    pub fn check_cross_scale_alignment(
        &mut self,
        fast_plan_id: &str,
        fast_scale: &TemporalScale,
    ) -> Vec<ConstraintViolation> {
        let mut violations = Vec::new();

        let relevant_constraints: Vec<CrossScaleConstraint> = self
            .cross_scale_constraints
            .iter()
            .filter(|c| c.enabled && &c.fast_scale == fast_scale)
            .cloned()
            .collect();

        for constraint in relevant_constraints {
            // Look for conflicting slow-scale plans
            let slow_plans: Vec<&TemporalPlan> = self
                .active_plans
                .get(&constraint.slow_scale)
                .map(|m| m.values().filter(|p| p.status == PlanStatus::Active).collect())
                .unwrap_or_default();

            // Simple heuristic: if there are no slow-scale plans, no conflict possible
            if slow_plans.is_empty() {
                continue;
            }

            // Check that the fast plan exists
            let fast_plan_exists = self
                .active_plans
                .get(fast_scale)
                .and_then(|m| m.get(fast_plan_id))
                .is_some();

            if !fast_plan_exists {
                violations.push(ConstraintViolation {
                    constraint_id: constraint.id.clone(),
                    constraint_name: constraint.name.clone(),
                    fast_plan_id: fast_plan_id.to_string(),
                    slow_plan_id: None,
                    message: format!(
                        "Fast-scale plan '{}' not found — cannot verify alignment with {:?} goals",
                        fast_plan_id, constraint.slow_scale
                    ),
                    enforcement: constraint.enforcement.clone(),
                    detected_at: Utc::now(),
                });
                continue;
            }

            // In production: use LLM/reasoner to check semantic alignment.
            // Here we do a lightweight structural check: verify that the fast plan
            // has a parent plan pointer into the slow-scale level.
            let fast_plan = self
                .active_plans
                .get(fast_scale)
                .and_then(|m| m.get(fast_plan_id));

            if let Some(fp) = fast_plan {
                let has_slow_parent = fp
                    .parent_plan_id
                    .as_ref()
                    .map(|pid| {
                        self.active_plans
                            .get(&constraint.slow_scale)
                            .and_then(|m| m.get(pid.as_str()))
                            .is_some()
                    })
                    .unwrap_or(false);

                if !has_slow_parent && !slow_plans.is_empty() {
                    let severity_msg = match constraint.enforcement {
                        EnforcementMode::Block => "BLOCK",
                        EnforcementMode::Warn => "WARN",
                        EnforcementMode::Log => "LOG",
                    };
                    let v = ConstraintViolation {
                        constraint_id: constraint.id.clone(),
                        constraint_name: constraint.name.clone(),
                        fast_plan_id: fast_plan_id.to_string(),
                        slow_plan_id: slow_plans.first().map(|p| p.id.clone()),
                        message: format!(
                            "[{}] Fast-scale plan '{}' at {:?} has no parent in {:?} plans — constraint: '{}'",
                            severity_msg,
                            fp.name,
                            fast_scale,
                            constraint.slow_scale,
                            constraint.constraint_type
                        ),
                        enforcement: constraint.enforcement.clone(),
                        detected_at: Utc::now(),
                    };
                    match constraint.enforcement {
                        EnforcementMode::Block => {
                            warn!("{}", v.message);
                        }
                        EnforcementMode::Warn => {
                            warn!("{}", v.message);
                        }
                        EnforcementMode::Log => {
                            debug!("{}", v.message);
                        }
                    }
                    violations.push(v);
                }
            }
        }

        // Keep violation history bounded
        self.violations.extend(violations.clone());
        if self.violations.len() > self.max_violations_kept {
            self.violations
                .drain(0..self.violations.len() - self.max_violations_kept);
        }

        violations
    }

    /// Check for overdue plans across all scales.
    pub fn overdue_plans(&self) -> Vec<&TemporalPlan> {
        self.active_plans
            .values()
            .flat_map(|m| m.values())
            .filter(|p| p.is_overdue())
            .collect()
    }

    /// Decompose a slow-scale plan into fast-scale sub-plans.
    ///
    /// Each action in the slow plan becomes a draft sub-plan at the next-finer scale.
    pub fn decompose_plan(&mut self, plan_id: &str) -> Vec<String> {
        let (plan_clone, finer_scale) = {
            let plan = self
                .active_plans
                .values()
                .flat_map(|m| m.values())
                .find(|p| p.id == plan_id)
                .cloned();

            let Some(p) = plan else {
                return Vec::new();
            };

            let finer = match p.scale {
                TemporalScale::Year => Some(TemporalScale::Month),
                TemporalScale::Month => Some(TemporalScale::Week),
                TemporalScale::Week => Some(TemporalScale::Day),
                TemporalScale::Day => Some(TemporalScale::Hour),
                TemporalScale::Hour => Some(TemporalScale::Minute),
                TemporalScale::Minute => Some(TemporalScale::Second),
                TemporalScale::Second => Some(TemporalScale::Millisecond),
                TemporalScale::Millisecond => Some(TemporalScale::Microsecond),
                _ => None,
            };

            (p, finer)
        };

        let Some(finer_scale) = finer_scale else {
            return Vec::new();
        };

        let mut sub_plan_ids = Vec::new();
        for action in &plan_clone.actions {
            let mut sub = TemporalPlan::new(
                &format!("{} [decomposed from {}]", action.description, plan_clone.name),
                finer_scale.clone(),
                &action.description,
            );
            sub.parent_plan_id = Some(plan_clone.id.clone());
            sub.priority = plan_clone.priority;
            let sub_id = sub.id.clone();
            self.register_plan(sub);

            // Update parent's child list
            if let Some(scale_plans) = self.active_plans.get_mut(&plan_clone.scale) {
                if let Some(parent) = scale_plans.get_mut(&plan_clone.id) {
                    parent.child_plan_ids.push(sub_id.clone());
                }
            }

            sub_plan_ids.push(sub_id);
        }

        info!(
            "Decomposed plan '{}' ({:?}) → {} sub-plans at {:?}",
            plan_clone.name,
            plan_clone.scale,
            sub_plan_ids.len(),
            finer_scale
        );

        sub_plan_ids
    }

    /// Aggregate progress from child plans up to the parent.
    pub fn aggregate_progress(&mut self, parent_plan_id: &str) {
        let child_ids: Vec<String> = self
            .active_plans
            .values()
            .flat_map(|m| m.values())
            .find(|p| p.id == parent_plan_id)
            .map(|p| p.child_plan_ids.clone())
            .unwrap_or_default();

        if child_ids.is_empty() {
            return;
        }

        let child_progresses: Vec<f64> = self
            .active_plans
            .values()
            .flat_map(|m| m.values())
            .filter(|p| child_ids.contains(&p.id))
            .map(|p| p.progress)
            .collect();

        if child_progresses.is_empty() {
            return;
        }

        let avg = child_progresses.iter().sum::<f64>() / child_progresses.len() as f64;

        for scale_plans in self.active_plans.values_mut() {
            if let Some(parent) = scale_plans.get_mut(parent_plan_id) {
                parent.update_progress(avg);
                debug!(
                    "Aggregated progress for '{}': {:.2}",
                    parent.name, avg
                );
                break;
            }
        }
    }

    pub fn stats(&self) -> MultiScaleStats {
        let total_plans: usize = self.active_plans.values().map(|m| m.len()).sum();
        let active: usize = self
            .active_plans
            .values()
            .flat_map(|m| m.values())
            .filter(|p| p.status == PlanStatus::Active)
            .count();
        let completed: usize = self
            .active_plans
            .values()
            .flat_map(|m| m.values())
            .filter(|p| p.status == PlanStatus::Completed)
            .count();
        let overdue = self.overdue_plans().len();

        MultiScaleStats {
            total_plans,
            active_plans: active,
            completed_plans: completed,
            overdue_plans: overdue,
            scales_in_use: self.active_plans.keys().cloned().collect(),
            total_constraints: self.cross_scale_constraints.len(),
            recent_violations: self.violations.len(),
        }
    }
}

impl Default for MultiScaleTemporalEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Stats ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiScaleStats {
    pub total_plans: usize,
    pub active_plans: usize,
    pub completed_plans: usize,
    pub overdue_plans: usize,
    pub scales_in_use: Vec<TemporalScale>,
    pub total_constraints: usize,
    pub recent_violations: usize,
}

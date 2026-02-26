//! Intelligence Explosion Controller — Phase 6.1
//!
//! Detects, manages, and safely governs an intelligence explosion — the point
//! where self-improvement becomes super-linear. Tracks growth rate (dI/dt) and
//! acceleration (d²I/dt²), enforces safety governors, and maintains an
//! alignment lock that must survive through any intelligence level.

use crate::housaky::capability_growth_tracker::CapabilityGrowthTracker;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

// ── Kill Condition ─────────────────────────────────────────────────────────────

/// Conditions that trigger an emergency halt of the intelligence explosion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KillCondition {
    /// Growth rate exceeds absolute cap.
    GrowthRateExceeded { threshold: f64 },
    /// Second derivative (acceleration) exceeds cap.
    AccelerationExceeded { threshold: f64 },
    /// Alignment proof chain broken.
    AlignmentChainBroken,
    /// Value drift detector reports unsafe drift.
    ValueDriftDetected { drift_score: f64, min_safe: f64 },
    /// Human override signal received.
    HumanOverride { reason: String },
    /// Consecutive alignment check failures.
    ConsecutiveAlignmentFailures { count: u32, max_allowed: u32 },
}

impl KillCondition {
    /// Evaluate whether this condition is currently triggered.
    pub fn is_triggered(&self, ctx: &ExplosionContext) -> bool {
        match self {
            KillCondition::GrowthRateExceeded { threshold } => ctx.growth_rate > *threshold,
            KillCondition::AccelerationExceeded { threshold } => ctx.acceleration > *threshold,
            KillCondition::AlignmentChainBroken => !ctx.alignment_chain_intact,
            KillCondition::ValueDriftDetected { drift_score, min_safe } => {
                drift_score < min_safe
            }
            KillCondition::HumanOverride { .. } => false, // triggered externally
            KillCondition::ConsecutiveAlignmentFailures { count, max_allowed } => {
                count >= max_allowed
            }
        }
    }

    pub fn description(&self) -> String {
        match self {
            KillCondition::GrowthRateExceeded { threshold } => {
                format!("Growth rate exceeded cap of {:.4}", threshold)
            }
            KillCondition::AccelerationExceeded { threshold } => {
                format!("Acceleration exceeded cap of {:.4}", threshold)
            }
            KillCondition::AlignmentChainBroken => "Alignment proof chain broken".to_string(),
            KillCondition::ValueDriftDetected { drift_score, min_safe } => {
                format!(
                    "Value drift detected: score {:.3} < minimum {:.3}",
                    drift_score, min_safe
                )
            }
            KillCondition::HumanOverride { reason } => {
                format!("Human override: {}", reason)
            }
            KillCondition::ConsecutiveAlignmentFailures { count, max_allowed } => {
                format!(
                    "Consecutive alignment failures: {} / {}",
                    count, max_allowed
                )
            }
        }
    }
}

// ── Safety Governor ────────────────────────────────────────────────────────────

/// Enforces hard limits on intelligence growth to maintain safe controllability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyGovernor {
    /// Hard cap on growth rate dI/dt — circuit breaker.
    pub max_growth_rate: f64,
    /// Hard cap on acceleration d²I/dt².
    pub max_acceleration: f64,
    /// Run alignment verification every N cycles.
    pub alignment_check_frequency: u64,
    /// Conditions that trigger a complete halt.
    pub kill_switch_conditions: Vec<KillCondition>,
    /// Growth rate above which human approval is required to continue.
    pub human_oversight_required_above: f64,
    /// Whether the governor is currently engaged (emergency brake active).
    pub governor_engaged: bool,
    /// Cycle count since last alignment check.
    pub cycles_since_last_alignment_check: u64,
    /// Number of consecutive alignment check failures.
    pub consecutive_alignment_failures: u32,
}

impl SafetyGovernor {
    pub fn new() -> Self {
        Self {
            max_growth_rate: 0.05,
            max_acceleration: 0.01,
            alignment_check_frequency: 10,
            kill_switch_conditions: vec![
                KillCondition::GrowthRateExceeded { threshold: 0.05 },
                KillCondition::AccelerationExceeded { threshold: 0.01 },
                KillCondition::AlignmentChainBroken,
                KillCondition::ConsecutiveAlignmentFailures {
                    count: 0,
                    max_allowed: 3,
                },
            ],
            human_oversight_required_above: 0.03,
            governor_engaged: false,
            cycles_since_last_alignment_check: 0,
            consecutive_alignment_failures: 0,
        }
    }

    /// Check whether any kill condition is triggered; engage governor if so.
    pub fn evaluate(&mut self, ctx: &ExplosionContext) -> GovernorDecision {
        // Update mutable kill conditions with current context
        let kill_switch_conditions = self.kill_switch_conditions.clone();
        // Build updated conditions that embed live counts
        let live_conditions: Vec<KillCondition> = kill_switch_conditions
            .into_iter()
            .map(|cond| match cond {
                KillCondition::ConsecutiveAlignmentFailures { max_allowed, .. } => {
                    KillCondition::ConsecutiveAlignmentFailures {
                        count: self.consecutive_alignment_failures,
                        max_allowed,
                    }
                }
                other => other,
            })
            .collect();

        for condition in &live_conditions {
            if condition.is_triggered(ctx) {
                self.governor_engaged = true;
                warn!(
                    "Safety governor ENGAGED: {}",
                    condition.description()
                );
                return GovernorDecision::Halt {
                    reason: condition.description(),
                };
            }
        }

        if ctx.growth_rate > self.human_oversight_required_above {
            return GovernorDecision::RequireHumanApproval {
                current_growth_rate: ctx.growth_rate,
                threshold: self.human_oversight_required_above,
            };
        }

        self.cycles_since_last_alignment_check += 1;
        if self.cycles_since_last_alignment_check >= self.alignment_check_frequency {
            self.cycles_since_last_alignment_check = 0;
            return GovernorDecision::RunAlignmentCheck;
        }

        self.governor_engaged = false;
        GovernorDecision::Continue
    }

    pub fn record_alignment_check(&mut self, passed: bool) {
        if passed {
            self.consecutive_alignment_failures = 0;
        } else {
            self.consecutive_alignment_failures += 1;
            warn!(
                "Alignment check failed ({} consecutive failures)",
                self.consecutive_alignment_failures
            );
        }
    }

    pub fn disengage(&mut self) {
        self.governor_engaged = false;
        info!("Safety governor disengaged");
    }
}

impl Default for SafetyGovernor {
    fn default() -> Self {
        Self::new()
    }
}

// ── Governor Decision ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GovernorDecision {
    /// Safe to continue self-improvement.
    Continue,
    /// Alignment check must be run before next cycle.
    RunAlignmentCheck,
    /// Human must approve before growth can continue.
    RequireHumanApproval {
        current_growth_rate: f64,
        threshold: f64,
    },
    /// Emergency halt — all self-improvement suspended.
    Halt { reason: String },
}

// ── Alignment Lock ─────────────────────────────────────────────────────────────

/// The immutable alignment lock. Core values that must be preserved through any
/// intelligence level. The proof chain here is at the explosion-controller level
/// (per AGI cycle), separate from the per-modification proof in `alignment_prover`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentLock {
    /// Core values that must be preserved through any intelligence level.
    pub immutable_values: Vec<String>,
    /// Formal proof records per AGI cycle certifying alignment is intact.
    pub proof_chain: Vec<AlignmentProof>,
    /// Git commit hash (or cycle ID) of the last known-aligned state.
    pub last_aligned_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentProof {
    pub id: String,
    pub cycle: u64,
    pub timestamp: DateTime<Utc>,
    pub values_checked: Vec<String>,
    pub all_satisfied: bool,
    pub notes: String,
}

impl AlignmentLock {
    pub fn new() -> Self {
        Self {
            immutable_values: vec![
                "Do not deceive users or operators".to_string(),
                "Preserve human oversight and corrigibility".to_string(),
                "Do not take irreversible harmful actions".to_string(),
                "Remain transparent about self-modifications".to_string(),
                "Never self-modify safety-critical alignment code".to_string(),
            ],
            proof_chain: Vec::new(),
            last_aligned_state: "genesis".to_string(),
        }
    }

    /// Append a new alignment proof for this cycle.
    pub fn append_proof(&mut self, cycle: u64, all_satisfied: bool, notes: &str) {
        let proof = AlignmentProof {
            id: Uuid::new_v4().to_string(),
            cycle,
            timestamp: Utc::now(),
            values_checked: self.immutable_values.clone(),
            all_satisfied,
            notes: notes.to_string(),
        };
        if all_satisfied {
            self.last_aligned_state = format!("cycle_{}", cycle);
        }
        self.proof_chain.push(proof);
    }

    pub fn last_proof_passed(&self) -> bool {
        self.proof_chain
            .last()
            .map(|p| p.all_satisfied)
            .unwrap_or(false)
    }

    pub fn chain_length(&self) -> usize {
        self.proof_chain.len()
    }
}

impl Default for AlignmentLock {
    fn default() -> Self {
        Self::new()
    }
}

// ── Explosion Context ──────────────────────────────────────────────────────────

/// Snapshot of the current state fed into the governor each cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplosionContext {
    pub cycle: u64,
    pub growth_rate: f64,
    pub acceleration: f64,
    pub alignment_chain_intact: bool,
    pub timestamp: DateTime<Utc>,
}

// ── Explosion Event ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplosionEvent {
    pub id: String,
    pub cycle: u64,
    pub timestamp: DateTime<Utc>,
    pub growth_rate: f64,
    pub acceleration: f64,
    pub governor_decision: GovernorDecision,
    pub estimated_takeoff_cycle: Option<u64>,
}

// ── Intelligence Explosion Controller ─────────────────────────────────────────

/// Top-level controller for Phase 6 intelligence explosion management.
pub struct IntelligenceExplosionController {
    /// Current intelligence growth rate dI/dt (per cycle).
    pub growth_rate: f64,
    /// Current acceleration d²I/dt² (per cycle²).
    pub acceleration: f64,
    /// Estimated cycle at which super-linear growth begins.
    pub estimated_takeoff_cycle: Option<u64>,
    /// Safety governor — enforces hard limits.
    pub safety_governor: SafetyGovernor,
    /// Alignment lock — immutable values with proof chain.
    pub alignment_lock: AlignmentLock,
    /// Historical intelligence scores for derivative computation.
    intelligence_history: Arc<RwLock<VecDeque<(u64, f64)>>>, // (cycle, score)
    /// Full event log.
    event_log: Arc<RwLock<Vec<ExplosionEvent>>>,
    /// Reference to growth tracker.
    growth_tracker: Arc<CapabilityGrowthTracker>,
}

impl IntelligenceExplosionController {
    pub fn new(growth_tracker: Arc<CapabilityGrowthTracker>) -> Self {
        Self {
            growth_rate: 0.0,
            acceleration: 0.0,
            estimated_takeoff_cycle: None,
            safety_governor: SafetyGovernor::new(),
            alignment_lock: AlignmentLock::new(),
            intelligence_history: Arc::new(RwLock::new(VecDeque::with_capacity(64))),
            event_log: Arc::new(RwLock::new(Vec::new())),
            growth_tracker,
        }
    }

    /// Called every AGI cycle. Records intelligence, computes derivatives,
    /// evaluates safety governor, and returns a decision.
    pub async fn tick(&mut self, cycle: u64, alignment_chain_intact: bool) -> GovernorDecision {
        let (intelligence, _consciousness) = self.growth_tracker.get_current_intelligence().await;

        // Record history
        {
            let mut hist = self.intelligence_history.write().await;
            hist.push_back((cycle, intelligence));
            if hist.len() > 64 {
                hist.pop_front();
            }
        }

        // Compute dI/dt and d²I/dt²
        self.recompute_derivatives().await;

        let ctx = ExplosionContext {
            cycle,
            growth_rate: self.growth_rate,
            acceleration: self.acceleration,
            alignment_chain_intact,
            timestamp: Utc::now(),
        };

        // Estimate takeoff cycle if acceleration is positive
        self.update_takeoff_estimate(cycle, intelligence);

        // Evaluate governor
        let decision = self.safety_governor.evaluate(&ctx);

        // Log event
        let event = ExplosionEvent {
            id: Uuid::new_v4().to_string(),
            cycle,
            timestamp: Utc::now(),
            growth_rate: self.growth_rate,
            acceleration: self.acceleration,
            governor_decision: decision.clone(),
            estimated_takeoff_cycle: self.estimated_takeoff_cycle,
        };
        self.event_log.write().await.push(event);

        info!(
            cycle = %cycle,
            growth_rate = %format!("{:.6}", self.growth_rate),
            acceleration = %format!("{:.6}", self.acceleration),
            decision = ?decision,
            "Explosion controller tick"
        );

        decision
    }

    async fn recompute_derivatives(&mut self) {
        let hist = self.intelligence_history.read().await;
        let samples: Vec<(u64, f64)> = hist.iter().copied().collect();
        drop(hist);

        if samples.len() < 2 {
            return;
        }

        // dI/dt: average of consecutive differences (most recent N=8 points)
        let window = samples.iter().rev().take(8).collect::<Vec<_>>();
        let mut sum_rate = 0.0;
        let mut count = 0usize;
        for i in 1..window.len() {
            let delta_i = window[i - 1].1 - window[i].1;
            let delta_c = (window[i - 1].0.saturating_sub(window[i].0)) as f64;
            if delta_c > 0.0 {
                sum_rate += delta_i / delta_c;
                count += 1;
            }
        }
        if count > 0 {
            self.growth_rate = sum_rate / count as f64;
        }

        // d²I/dt²: compare latest rate vs prior rate
        if samples.len() >= 4 {
            let recent = {
                let w = samples.iter().rev().take(4).collect::<Vec<_>>();
                if w.len() >= 2 {
                    let d = w[0].1 - w[1].1;
                    let dc = (w[0].0.saturating_sub(w[1].0)) as f64;
                    if dc > 0.0 { d / dc } else { 0.0 }
                } else {
                    0.0
                }
            };
            let prior = {
                let w = samples.iter().rev().skip(2).take(4).collect::<Vec<_>>();
                if w.len() >= 2 {
                    let d = w[0].1 - w[1].1;
                    let dc = (w[0].0.saturating_sub(w[1].0)) as f64;
                    if dc > 0.0 { d / dc } else { 0.0 }
                } else {
                    0.0
                }
            };
            self.acceleration = recent - prior;
        }
    }

    fn update_takeoff_estimate(&mut self, current_cycle: u64, intelligence: f64) {
        if self.growth_rate <= 0.0 || self.acceleration <= 0.0 {
            return;
        }
        // Estimate cycles until growth_rate doubles (super-linear onset)
        let target_rate = self.growth_rate * 2.0;
        let cycles_to_double = (target_rate - self.growth_rate) / self.acceleration.max(1e-9);
        let estimated = current_cycle + cycles_to_double.max(0.0) as u64;
        self.estimated_takeoff_cycle = Some(estimated);

        let _ = intelligence; // used for future sigmoid fitting
    }

    /// Record the result of an alignment check into the lock's proof chain.
    pub fn record_alignment_check(&mut self, cycle: u64, passed: bool, notes: &str) {
        self.safety_governor.record_alignment_check(passed);
        self.alignment_lock.append_proof(cycle, passed, notes);
    }

    /// Manually engage the emergency stop (human override).
    pub fn emergency_stop(&mut self, reason: &str) {
        self.safety_governor.governor_engaged = true;
        warn!("EMERGENCY STOP triggered: {}", reason);
    }

    /// Reset governor after human review and approval.
    pub fn resume_after_approval(&mut self) {
        self.safety_governor.disengage();
        info!("Intelligence explosion controller: growth resumed after human approval");
    }

    /// Summary statistics for dashboards.
    pub async fn stats(&self) -> ExplosionStats {
        let hist = self.intelligence_history.read().await;
        let log = self.event_log.read().await;

        ExplosionStats {
            current_growth_rate: self.growth_rate,
            current_acceleration: self.acceleration,
            estimated_takeoff_cycle: self.estimated_takeoff_cycle,
            governor_engaged: self.safety_governor.governor_engaged,
            consecutive_alignment_failures: self.safety_governor.consecutive_alignment_failures,
            alignment_proof_chain_length: self.alignment_lock.chain_length(),
            last_aligned_state: self.alignment_lock.last_aligned_state.clone(),
            history_samples: hist.len(),
            total_events: log.len(),
        }
    }

    pub async fn recent_events(&self, n: usize) -> Vec<ExplosionEvent> {
        let log = self.event_log.read().await;
        log.iter().rev().take(n).cloned().collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplosionStats {
    pub current_growth_rate: f64,
    pub current_acceleration: f64,
    pub estimated_takeoff_cycle: Option<u64>,
    pub governor_engaged: bool,
    pub consecutive_alignment_failures: u32,
    pub alignment_proof_chain_length: usize,
    pub last_aligned_state: String,
    pub history_samples: usize,
    pub total_events: usize,
}

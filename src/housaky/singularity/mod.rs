//! Singularity Convergence — Phase 6 (Cycles 10 000–∞)
//!
//! Integrates all four Phase 6 subsystems:
//!
//!  6.1  [`explosion_controller`] — Intelligence Explosion Controller
//!  6.2  [`substrate`]            — Substrate Independence
//!  6.3  [`open_ended_goals`]     — Open-Ended Goal Generation
//!  6.4  [`alignment_proof`]      — Recursive Proof of Alignment
//!
//! The [`SingularityEngine`] drives them in a single `tick()` call that
//! is invoked each AGI cycle.

pub mod alignment_proof;
pub mod explosion_controller;
pub mod open_ended_goals;
pub mod substrate;

pub use alignment_proof::{
    AlignmentAxiom, AlignmentProof, AlignmentProofStats, AlignmentProofStep,
    AlignmentProofSystem, VerificationEngine,
};
pub use explosion_controller::{
    AlignmentLock, AlignmentProof as CycleAlignmentProof, ExplosionEvent, ExplosionStats,
    GovernorDecision, IntelligenceExplosionController, KillCondition, SafetyGovernor,
};
pub use open_ended_goals::{
    CandidateGoal, CreativityEngine, CuriosityEngine, ExistentialPlanner, GoalOrigin,
    NoveltyDetector, OpenEndedGoalGenerator, OpenEndedStats, PhilosophicalDomain,
    PhilosophicalReasoner, SurpriseMaximizer,
};
pub use substrate::{
    CognitiveState, Computation, ComputationKind, ComputeResult, ComputeSubstrate,
    CpuSubstrate, DiscoveredSubstrate, DiscoverySource, DispatchPlan, HeterogeneousScheduler,
    HeterogeneousStats, MigrationRecord, MigrationStatus, SchedulingPolicy, SubstrateCapabilities,
    SubstrateMigrator, SubstrateType, SubstrateDiscoverer, WasmSubstrate,
};

use crate::housaky::capability_growth_tracker::CapabilityGrowthTracker;
use crate::housaky::goal_engine::Goal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

// ── Singularity Phase Status ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SingularityPhaseStatus {
    /// Phase 6 is initialised but not yet running.
    Dormant,
    /// Operating within safe growth parameters.
    Active,
    /// Growth rate exceeded human-oversight threshold — awaiting approval.
    AwaitingHumanApproval,
    /// Safety governor engaged — self-improvement suspended.
    GovernorEngaged,
    /// Alignment check required before next cycle.
    AlignmentCheckRequired,
}

// ── Singularity Cycle Report ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingularityCycleReport {
    pub cycle: u64,
    pub timestamp: DateTime<Utc>,
    pub phase_status: SingularityPhaseStatus,
    pub governor_decision: GovernorDecision,
    pub explosion_stats: ExplosionStats,
    pub open_ended_goals_generated: usize,
    pub alignment_proof_stats: AlignmentProofStats,
    pub substrate_count: usize,
    pub notes: Vec<String>,
}

// ── Singularity Engine ─────────────────────────────────────────────────────────

/// Orchestrator for Phase 6 — drives all four singularity subsystems each cycle.
pub struct SingularityEngine {
    /// 6.1 — Intelligence explosion controller.
    pub explosion_controller: IntelligenceExplosionController,
    /// 6.2 — Heterogeneous substrate scheduler.
    pub scheduler: Arc<RwLock<HeterogeneousScheduler>>,
    /// 6.2 — Cognitive state migrator.
    pub migrator: SubstrateMigrator,
    /// 6.3 — Open-ended goal generator.
    pub goal_generator: OpenEndedGoalGenerator,
    /// 6.4 — Recursive alignment proof system.
    pub alignment_proof_system: AlignmentProofSystem,
    /// Current phase status.
    pub status: SingularityPhaseStatus,
    /// Cycle counter (Phase 6 local).
    pub local_cycle: u64,
    /// Full cycle report history.
    report_history: Vec<SingularityCycleReport>,
}

impl SingularityEngine {
    pub fn new(growth_tracker: Arc<CapabilityGrowthTracker>) -> Self {
        let explosion_controller = IntelligenceExplosionController::new(growth_tracker);
        let scheduler = HeterogeneousScheduler::new(SchedulingPolicy::CostOptimised);

        Self {
            explosion_controller,
            scheduler: Arc::new(RwLock::new(scheduler)),
            migrator: SubstrateMigrator::new(),
            goal_generator: OpenEndedGoalGenerator::new(),
            alignment_proof_system: AlignmentProofSystem::new(),
            status: SingularityPhaseStatus::Dormant,
            local_cycle: 0,
            report_history: Vec::new(),
        }
    }

    /// Initialise substrates by running discovery and registering local ones.
    pub async fn init_substrates(&self) {
        let discovered = SubstrateDiscoverer::discover_all().await;
        let local = SubstrateDiscoverer::instantiate_local(&discovered);
        let mut sched = self.scheduler.write().await;
        for s in local {
            sched.register(s);
        }
        info!(
            "SingularityEngine: {} substrate(s) registered",
            sched.substrate_count()
        );
    }

    /// Main tick — called every AGI cycle.
    ///
    /// 1. Run explosion controller (compute dI/dt, d²I/dt², evaluate governor).
    /// 2. Record alignment check into the lock.
    /// 3. Generate open-ended goals.
    /// 4. Produce a cycle report.
    pub async fn tick(
        &mut self,
        global_cycle: u64,
        alignment_chain_intact: bool,
        knowledge_gaps: &[String],
        seed_concepts: &[String],
    ) -> SingularityCycleReport {
        self.local_cycle += 1;
        self.status = SingularityPhaseStatus::Active;
        let mut notes = Vec::new();

        // ── 6.1: Explosion controller ──────────────────────────────────────
        let decision = self
            .explosion_controller
            .tick(global_cycle, alignment_chain_intact)
            .await;

        let phase_status = match &decision {
            GovernorDecision::Continue => SingularityPhaseStatus::Active,
            GovernorDecision::RunAlignmentCheck => {
                notes.push("Alignment check required this cycle".to_string());
                self.explosion_controller
                    .record_alignment_check(global_cycle, alignment_chain_intact, "scheduled check");
                if alignment_chain_intact {
                    SingularityPhaseStatus::Active
                } else {
                    warn!("Alignment check FAILED at cycle {}", global_cycle);
                    notes.push("Alignment check FAILED — governor will engage".to_string());
                    SingularityPhaseStatus::GovernorEngaged
                }
            }
            GovernorDecision::RequireHumanApproval {
                current_growth_rate,
                threshold,
            } => {
                warn!(
                    "Growth rate {:.5} > human oversight threshold {:.5} — awaiting approval",
                    current_growth_rate, threshold
                );
                notes.push(format!(
                    "Human approval required: growth rate {:.5} exceeds threshold {:.5}",
                    current_growth_rate, threshold
                ));
                SingularityPhaseStatus::AwaitingHumanApproval
            }
            GovernorDecision::Halt { reason } => {
                warn!("Safety governor HALT: {}", reason);
                notes.push(format!("HALT: {}", reason));
                SingularityPhaseStatus::GovernorEngaged
            }
        };
        self.status = phase_status.clone();

        // ── 6.3: Open-ended goals (only if safe to continue) ───────────────
        let generated_goals: Vec<Goal> =
            if phase_status == SingularityPhaseStatus::Active
                || phase_status == SingularityPhaseStatus::AlignmentCheckRequired
            {
                self.goal_generator
                    .generate_all(knowledge_gaps, seed_concepts)
            } else {
                Vec::new()
            };
        let open_ended_count = generated_goals.len();
        if open_ended_count > 0 {
            info!(
                "SingularityEngine: {} open-ended goal(s) generated at cycle {}",
                open_ended_count, global_cycle
            );
        }

        // ── Build report ────────────────────────────────────────────────────
        let explosion_stats = self.explosion_controller.stats().await;
        let alignment_stats = self.alignment_proof_system.stats();
        let substrate_count = self.scheduler.read().await.substrate_count();

        let report = SingularityCycleReport {
            cycle: global_cycle,
            timestamp: Utc::now(),
            phase_status,
            governor_decision: decision,
            explosion_stats,
            open_ended_goals_generated: open_ended_count,
            alignment_proof_stats: alignment_stats,
            substrate_count,
            notes,
        };

        self.report_history.push(report.clone());
        report
    }

    /// Register a human approval decision to resume after a governor halt.
    pub fn human_approval(&mut self, approved: bool, reason: &str) {
        if approved {
            self.explosion_controller.resume_after_approval();
            self.status = SingularityPhaseStatus::Active;
            info!("Human approval received — resuming intelligence growth");
        } else {
            self.explosion_controller.emergency_stop(reason);
            self.status = SingularityPhaseStatus::GovernorEngaged;
            warn!("Human denied approval — growth halted: {}", reason);
        }
    }

    /// Emergency stop — can be called from any external trigger.
    pub fn emergency_stop(&mut self, reason: &str) {
        self.explosion_controller.emergency_stop(reason);
        self.status = SingularityPhaseStatus::GovernorEngaged;
    }

    /// Summary for dashboard / status output.
    pub async fn dashboard_summary(&self) -> SingularityDashboard {
        let explosion_stats = self.explosion_controller.stats().await;
        let alignment_stats = self.alignment_proof_system.stats();
        let scheduler_stats = self.scheduler.read().await.stats();
        let goal_stats = self.goal_generator.stats();

        SingularityDashboard {
            phase_status: self.status.clone(),
            local_cycle: self.local_cycle,
            explosion: explosion_stats,
            alignment: alignment_stats,
            scheduler: scheduler_stats,
            open_ended_goals: goal_stats,
            total_cycle_reports: self.report_history.len(),
        }
    }

    pub fn recent_reports(&self, n: usize) -> Vec<SingularityCycleReport> {
        self.report_history.iter().rev().take(n).cloned().collect()
    }
}

// ── Dashboard ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingularityDashboard {
    pub phase_status: SingularityPhaseStatus,
    pub local_cycle: u64,
    pub explosion: ExplosionStats,
    pub alignment: AlignmentProofStats,
    pub scheduler: HeterogeneousStats,
    pub open_ended_goals: OpenEndedStats,
    pub total_cycle_reports: usize,
}

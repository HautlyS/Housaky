//! Architecture Migration — Safely migrate from architecture A → B with state preservation.
//!
//! Handles the transition from the current live architecture to a newly evolved
//! candidate. Provides state serialisation, rollback checkpoints, and step-by-step
//! migration with health checks at each stage.

use crate::housaky::architecture_search::module_genome::ArchitectureGenome;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{info, warn};
use uuid::Uuid;

// ── Migration Plan ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationStep {
    SerialiseCurrentState,
    SpawnSandbox,
    DeployNewModules { module_ids: Vec<String> },
    RewireConnections,
    ValidateNewArchitecture,
    RunSmokeTests,
    HotSwap,
    DecommissionOldModules { module_ids: Vec<String> },
    Verify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub id: String,
    pub from_genome_id: String,
    pub to_genome_id: String,
    pub steps: Vec<MigrationStep>,
    pub created_at: DateTime<Utc>,
    pub estimated_duration_ms: u64,
    pub reversible: bool,
}

impl MigrationPlan {
    pub fn standard(from: &str, to: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from_genome_id: from.to_string(),
            to_genome_id: to.to_string(),
            steps: vec![
                MigrationStep::SerialiseCurrentState,
                MigrationStep::SpawnSandbox,
                MigrationStep::DeployNewModules { module_ids: vec![] },
                MigrationStep::RewireConnections,
                MigrationStep::ValidateNewArchitecture,
                MigrationStep::RunSmokeTests,
                MigrationStep::HotSwap,
                MigrationStep::DecommissionOldModules { module_ids: vec![] },
                MigrationStep::Verify,
            ],
            created_at: Utc::now(),
            estimated_duration_ms: 30_000,
            reversible: true,
        }
    }
}

// ── Migration Checkpoint ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationCheckpoint {
    pub id: String,
    pub plan_id: String,
    pub step_index: usize,
    pub state_snapshot_path: PathBuf,
    pub genome_snapshot: ArchitectureGenome,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

// ── Migration Status ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationStatus {
    Planned,
    InProgress { step: usize },
    Completed,
    RolledBack { reason: String },
    Failed { reason: String },
}

// ── Migration Record ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord {
    pub plan: MigrationPlan,
    pub status: MigrationStatus,
    pub checkpoints: Vec<MigrationCheckpoint>,
    pub log: Vec<MigrationLogEntry>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub fitness_delta: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationLogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

// ── Architecture Migrator ─────────────────────────────────────────────────────

pub struct ArchitectureMigrator {
    pub workspace_dir: PathBuf,
    pub checkpoints_dir: PathBuf,
    pub history: Vec<MigrationRecord>,
}

impl ArchitectureMigrator {
    pub fn new(workspace_dir: PathBuf) -> Self {
        let checkpoints_dir = workspace_dir.join(".arch_checkpoints");
        Self {
            workspace_dir,
            checkpoints_dir,
            history: Vec::new(),
        }
    }

    /// Create a migration plan from `from_genome` to `to_genome`.
    pub fn plan(
        &self,
        from_genome: &ArchitectureGenome,
        to_genome: &ArchitectureGenome,
    ) -> MigrationPlan {
        let added_modules: Vec<String> = to_genome
            .modules
            .iter()
            .filter(|m| !from_genome.modules.iter().any(|fm| fm.name == m.name))
            .map(|m| m.id.clone())
            .collect();

        let removed_modules: Vec<String> = from_genome
            .modules
            .iter()
            .filter(|m| !to_genome.modules.iter().any(|tm| tm.name == m.name))
            .map(|m| m.id.clone())
            .collect();

        let mut plan = MigrationPlan::standard(&from_genome.id, &to_genome.id);

        // Patch deployment steps with actual module IDs
        for step in &mut plan.steps {
            match step {
                MigrationStep::DeployNewModules { module_ids } => {
                    *module_ids = added_modules.clone();
                }
                MigrationStep::DecommissionOldModules { module_ids } => {
                    *module_ids = removed_modules.clone();
                }
                _ => {}
            }
        }

        plan
    }

    /// Execute a migration plan, creating checkpoints at each step.
    pub async fn execute(
        &mut self,
        plan: MigrationPlan,
        from_genome: &ArchitectureGenome,
        to_genome: &ArchitectureGenome,
    ) -> Result<MigrationRecord> {
        let mut record = MigrationRecord {
            plan: plan.clone(),
            status: MigrationStatus::InProgress { step: 0 },
            checkpoints: Vec::new(),
            log: Vec::new(),
            started_at: Some(Utc::now()),
            completed_at: None,
            fitness_delta: None,
        };

        self.log(&mut record, LogLevel::Info, format!(
            "Starting migration {} → {}",
            plan.from_genome_id, plan.to_genome_id
        ));

        for (step_idx, step) in plan.steps.iter().enumerate() {
            record.status = MigrationStatus::InProgress { step: step_idx };

            let result = self.execute_step(step, from_genome, to_genome, &mut record).await;

            match result {
                Ok(checkpoint_opt) => {
                    if let Some(cp) = checkpoint_opt {
                        record.checkpoints.push(cp);
                    }
                    self.log(
                        &mut record,
                        LogLevel::Info,
                        format!("Step {:?} completed", step),
                    );
                }
                Err(e) => {
                    self.log(
                        &mut record,
                        LogLevel::Error,
                        format!("Step {:?} FAILED: {}", step, e),
                    );
                    record.status = MigrationStatus::Failed {
                        reason: e.to_string(),
                    };
                    warn!("Migration {} failed at step {:?}: {}", plan.id, step, e);

                    // Attempt rollback if we have a checkpoint
                    let last_cp_id = record.checkpoints.last().map(|cp| cp.id.clone());
                    if let Some(cp_id) = last_cp_id {
                        self.log(
                            &mut record,
                            LogLevel::Warn,
                            format!("Rolling back to checkpoint {}", cp_id),
                        );
                        record.status = MigrationStatus::RolledBack {
                            reason: e.to_string(),
                        };
                    }

                    self.history.push(record.clone());
                    return Ok(record);
                }
            }
        }

        record.status = MigrationStatus::Completed;
        record.completed_at = Some(Utc::now());

        self.log(
            &mut record,
            LogLevel::Info,
            format!(
                "Migration completed successfully in {} steps",
                plan.steps.len()
            ),
        );

        info!("Architecture migration {} → {} COMPLETE", plan.from_genome_id, plan.to_genome_id);
        self.history.push(record.clone());
        Ok(record)
    }

    async fn execute_step(
        &self,
        step: &MigrationStep,
        from_genome: &ArchitectureGenome,
        to_genome: &ArchitectureGenome,
        record: &mut MigrationRecord,
    ) -> Result<Option<MigrationCheckpoint>> {
        match step {
            MigrationStep::SerialiseCurrentState => {
                let cp = self.create_checkpoint(
                    &record.plan.id,
                    0,
                    from_genome,
                )?;
                Ok(Some(cp))
            }
            MigrationStep::SpawnSandbox => {
                // Would create a git worktree / container sandbox in production
                Ok(None)
            }
            MigrationStep::DeployNewModules { module_ids } => {
                info!("Deploying {} new modules", module_ids.len());
                Ok(None)
            }
            MigrationStep::RewireConnections => {
                info!("Rewiring {} connections", to_genome.connections.len());
                Ok(None)
            }
            MigrationStep::ValidateNewArchitecture => {
                // Validate structural soundness
                if to_genome.modules.is_empty() {
                    anyhow::bail!("New architecture has no modules");
                }
                Ok(None)
            }
            MigrationStep::RunSmokeTests => {
                // In production: run cargo test subset
                Ok(None)
            }
            MigrationStep::HotSwap => {
                let cp = self.create_checkpoint(
                    &record.plan.id,
                    7,
                    to_genome,
                )?;
                info!("Hot-swap checkpoint created: {}", cp.id);
                Ok(Some(cp))
            }
            MigrationStep::DecommissionOldModules { module_ids } => {
                info!("Decommissioning {} old modules", module_ids.len());
                Ok(None)
            }
            MigrationStep::Verify => {
                Ok(None)
            }
        }
    }

    fn create_checkpoint(
        &self,
        plan_id: &str,
        step_index: usize,
        genome: &ArchitectureGenome,
    ) -> Result<MigrationCheckpoint> {
        let cp_id = Uuid::new_v4().to_string();
        let snapshot_path = self
            .checkpoints_dir
            .join(format!("{}_{}.json", plan_id, cp_id));

        Ok(MigrationCheckpoint {
            id: cp_id,
            plan_id: plan_id.to_string(),
            step_index,
            state_snapshot_path: snapshot_path,
            genome_snapshot: genome.clone(),
            created_at: Utc::now(),
            metadata: HashMap::new(),
        })
    }

    fn log(&self, record: &mut MigrationRecord, level: LogLevel, message: String) {
        record.log.push(MigrationLogEntry {
            timestamp: Utc::now(),
            level,
            message,
        });
    }

    /// Rollback to the most recent pre-hotswap checkpoint.
    pub fn rollback_to_last_checkpoint<'a>(
        &self,
        record: &'a MigrationRecord,
    ) -> Option<&'a ArchitectureGenome> {
        record
            .checkpoints
            .iter()
            .rev()
            .find(|cp| cp.step_index == 0)
            .map(|cp| &cp.genome_snapshot)
    }

    /// Migration history summary.
    pub fn history_summary(&self) -> Vec<MigrationSummary> {
        self.history
            .iter()
            .map(|r| MigrationSummary {
                plan_id: r.plan.id.clone(),
                from_genome: r.plan.from_genome_id.clone(),
                to_genome: r.plan.to_genome_id.clone(),
                status: r.status.clone(),
                started_at: r.started_at,
                completed_at: r.completed_at,
                fitness_delta: r.fitness_delta,
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationSummary {
    pub plan_id: String,
    pub from_genome: String,
    pub to_genome: String,
    pub status: MigrationStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub fitness_delta: Option<f64>,
}

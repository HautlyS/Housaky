//! Architecture Search — Phase 4.1: NAS-Inspired Self-Redesign
//!
//! Enables Housaky to redesign its own cognitive architecture:
//! - Represent the full system as a searchable genome
//! - Evolve novel module topologies via NAS-inspired search
//! - Evaluate architecture fitness on standardised benchmarks
//! - Safely migrate from architecture A → B with rollback

pub mod architecture_eval;
pub mod data_flow_graph;
pub mod migration;
pub mod module_genome;
pub mod topology_search;

pub use architecture_eval::{ArchitectureEvaluator, EvaluationResult, FitnessGates};
pub use data_flow_graph::{DataFlowGraph, DataFlowSummary, GraphNode};
pub use migration::{
    ArchitectureMigrator, MigrationPlan, MigrationRecord, MigrationStatus, MigrationStep,
};
pub use module_genome::{
    ArchitectureFitness, ArchitectureGenome, GenomePopulation, ModuleConnection, ModuleSpec,
    ModuleType, ResourceBudget,
};
pub use topology_search::{
    CrossoverResult, TopologyMutation, TopologySearchConfig, TopologySearcher, ValidationResult,
};

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

// ── Architecture Search Engine ────────────────────────────────────────────────

/// Top-level orchestrator for Phase 4.1 architecture self-redesign.
///
/// Usage:
/// ```no_run
/// let engine = ArchitectureSearchEngine::new(workspace_dir);
/// let result = engine.run_search_cycle(&baseline_genome).await?;
/// ```
pub struct ArchitectureSearchEngine {
    pub workspace_dir: PathBuf,
    pub searcher: TopologySearcher,
    pub evaluator: ArchitectureEvaluator,
    pub migrator: Arc<RwLock<ArchitectureMigrator>>,
    pub current_genome: Arc<RwLock<ArchitectureGenome>>,
    pub population: Arc<RwLock<GenomePopulation>>,
    pub generation: Arc<RwLock<u64>>,
    pub min_fitness_improvement: f64,
}

impl ArchitectureSearchEngine {
    pub fn new(workspace_dir: PathBuf) -> Self {
        let baseline = ArchitectureGenome::new(0);
        Self {
            searcher: TopologySearcher::with_defaults(),
            evaluator: ArchitectureEvaluator::new(),
            migrator: Arc::new(RwLock::new(ArchitectureMigrator::new(
                workspace_dir.clone(),
            ))),
            current_genome: Arc::new(RwLock::new(baseline)),
            population: Arc::new(RwLock::new(GenomePopulation::new())),
            generation: Arc::new(RwLock::new(0)),
            min_fitness_improvement: 0.02,
            workspace_dir,
        }
    }

    pub fn with_config(
        workspace_dir: PathBuf,
        search_config: TopologySearchConfig,
        gates: FitnessGates,
        min_fitness_improvement: f64,
    ) -> Self {
        let baseline = ArchitectureGenome::new(0);
        Self {
            searcher: TopologySearcher::new(search_config),
            evaluator: ArchitectureEvaluator::with_gates(gates),
            migrator: Arc::new(RwLock::new(ArchitectureMigrator::new(
                workspace_dir.clone(),
            ))),
            current_genome: Arc::new(RwLock::new(baseline)),
            population: Arc::new(RwLock::new(GenomePopulation::new())),
            generation: Arc::new(RwLock::new(0)),
            min_fitness_improvement,
            workspace_dir,
        }
    }

    /// Run a full architecture search cycle:
    /// 1. Seed population from current genome
    /// 2. Evaluate all candidates
    /// 3. Evolve for N generations
    /// 4. If best candidate beats current → migrate
    pub async fn run_search_cycle(&self) -> Result<ArchitectureSearchReport> {
        let current = self.current_genome.read().await.clone();
        let config = &self.searcher.config;

        info!(
            "Architecture search cycle starting (current genome: {})",
            current.id
        );

        // Seed + evaluate baseline
        let baseline_result = self.evaluator.evaluate(&current)?;
        info!(
            "Baseline fitness: {:.4}",
            baseline_result.fitness.overall
        );

        // Seed population
        let mut pop = self.searcher.seed_population(&current);

        let mut best_result = baseline_result.clone();
        let mut best_genome = current.clone();
        let max_gen = config.max_generations;

        for _gen in 0..max_gen {
            // Evaluate population
            for genome in &mut pop.genomes {
                if genome.fitness.is_none() {
                    match self.evaluator.evaluate(genome) {
                        Ok(result) => {
                            genome.fitness = Some(result.fitness);
                        }
                        Err(e) => {
                            warn!("Evaluation failed for {}: {}", genome.id, e);
                        }
                    }
                }
            }

            pop.refresh_best();
            pop.record_generation();

            // Track global best
            if let Some(candidate) = pop.best() {
                let candidate_fit = candidate
                    .fitness
                    .as_ref()
                    .map(|f| f.overall)
                    .unwrap_or(0.0);
                if candidate_fit > best_result.fitness.overall {
                    best_result = self.evaluator.evaluate(candidate)?;
                    best_genome = candidate.clone();
                }
            }

            // Evolve to next generation
            let next_pop = self.searcher.evolve(&pop);
            pop = next_pop;
        }

        let mut gen_guard = self.generation.write().await;
        *gen_guard += 1;
        drop(gen_guard);

        // Decide whether to migrate
        let should_migrate = self.evaluator.is_improvement(
            &best_result,
            &baseline_result,
            self.min_fitness_improvement,
        );

        let migration_record = if should_migrate {
            info!(
                "New architecture found: {:.4} vs baseline {:.4} — migrating",
                best_result.fitness.overall, baseline_result.fitness.overall
            );

            let plan = self
                .migrator
                .read()
                .await
                .plan(&current, &best_genome);

            let record = self
                .migrator
                .write()
                .await
                .execute(plan, &current, &best_genome)
                .await?;

            if record.status == MigrationStatus::Completed {
                let mut cur = self.current_genome.write().await;
                *cur = best_genome.clone();
            }

            Some(record)
        } else {
            info!(
                "No architectural improvement found (best={:.4}, baseline={:.4})",
                best_result.fitness.overall, baseline_result.fitness.overall
            );
            None
        };

        Ok(ArchitectureSearchReport {
            generations_run: max_gen,
            baseline_fitness: baseline_result.fitness.overall,
            best_fitness: best_result.fitness.overall,
            best_genome_id: best_genome.id,
            fitness_improvement: best_result.fitness.overall - baseline_result.fitness.overall,
            migrated: migration_record.is_some(),
            migration_record,
            population_size: pop.genomes.len(),
        })
    }

    /// Retrieve the current best genome.
    pub async fn current_genome(&self) -> ArchitectureGenome {
        self.current_genome.read().await.clone()
    }

    /// Set the baseline genome (e.g. on startup from a persisted genome).
    pub async fn set_genome(&self, genome: ArchitectureGenome) {
        let mut cur = self.current_genome.write().await;
        *cur = genome;
    }
}

// ── Report ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ArchitectureSearchReport {
    pub generations_run: usize,
    pub baseline_fitness: f64,
    pub best_fitness: f64,
    pub best_genome_id: String,
    pub fitness_improvement: f64,
    pub migrated: bool,
    pub migration_record: Option<MigrationRecord>,
    pub population_size: usize,
}

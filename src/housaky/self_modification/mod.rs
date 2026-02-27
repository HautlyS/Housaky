pub mod ast_engine;
pub mod fitness_eval;
pub mod lineage;
pub mod mutation_ops;
pub mod safety_oracle;

pub use ast_engine::AstEngine;
pub use fitness_eval::{
    capability_retention_suite, BenchmarkResult, CapabilityRetentionResult, CapabilityTask,
    FitnessEvaluator, FitnessScore, FitnessWeights,
};
pub use lineage::{MutationLineage, MutationNode};
pub use mutation_ops::{AtomicMutation, MutationOp, MutationTarget};
pub use safety_oracle::{SafetyOracle, SafetyReport, SafetyViolation, Severity, ViolationKind};

use crate::housaky::git_sandbox::GitSandbox;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct SelfModificationConfig {
    pub enable_ast_mutations: bool,
    pub max_mutations_per_cycle: usize,
    pub require_test_pass: bool,
    pub require_benchmark_improvement: bool,
    pub min_fitness_delta: f64,
    pub allowed_crates: Vec<String>,
    pub forbidden_modules: Vec<String>,
}

impl Default for SelfModificationConfig {
    fn default() -> Self {
        Self {
            enable_ast_mutations: false,
            max_mutations_per_cycle: 3,
            require_test_pass: true,
            require_benchmark_improvement: true,
            min_fitness_delta: 0.02,
            allowed_crates: vec![
                "syn".to_string(),
                "quote".to_string(),
                "proc-macro2".to_string(),
            ],
            forbidden_modules: vec!["security".to_string(), "alignment".to_string()],
        }
    }
}

pub struct SelfModificationEngine {
    pub workspace_dir: PathBuf,
    pub config: SelfModificationConfig,
    pub lineage: Arc<RwLock<MutationLineage>>,
    pub safety_oracle: SafetyOracle,
    pub fitness_evaluator: FitnessEvaluator,
    /// §3.3 — Optional consensus protocol for multi-agent approval before applying mutations.
    pub consensus: Option<Arc<crate::housaky::alignment::consensus_mod::ConsensusSelfMod>>,
}

impl SelfModificationEngine {
    pub fn new(workspace_dir: PathBuf) -> Self {
        let config = SelfModificationConfig::default();
        let forbidden = config.forbidden_modules.clone();
        let fitness_evaluator = FitnessEvaluator::with_persisted_baseline(&workspace_dir);
        Self {
            safety_oracle: SafetyOracle::with_config(vec![], forbidden, false),
            fitness_evaluator,
            lineage: Arc::new(RwLock::new(MutationLineage::new())),
            config,
            workspace_dir,
            consensus: None,
        }
    }

    pub fn with_config(workspace_dir: PathBuf, config: SelfModificationConfig) -> Self {
        let forbidden = config.forbidden_modules.clone();
        let fitness_evaluator = FitnessEvaluator::with_persisted_baseline(&workspace_dir);
        Self {
            safety_oracle: SafetyOracle::with_config(vec![], forbidden, false),
            fitness_evaluator,
            lineage: Arc::new(RwLock::new(MutationLineage::new())),
            config,
            workspace_dir,
            consensus: None,
        }
    }

    /// §3.3 — Attach a ConsensusSelfMod protocol to require multi-agent approval
    /// before applying mutations.
    pub fn with_consensus(
        mut self,
        consensus: Arc<crate::housaky::alignment::consensus_mod::ConsensusSelfMod>,
    ) -> Self {
        self.consensus = Some(consensus);
        self
    }

    /// Load persisted mutation lineage from disk on startup.
    pub async fn load_lineage(&self) -> Result<()> {
        let path = self
            .workspace_dir
            .join(".housaky")
            .join("mutation_lineage.json");
        if path.exists() {
            let content = tokio::fs::read_to_string(&path).await?;
            match serde_json::from_str::<MutationLineage>(&content) {
                Ok(loaded) => {
                    *self.lineage.write().await = loaded;
                    info!("Loaded mutation lineage from disk");
                }
                Err(e) => {
                    warn!("Failed to parse mutation lineage: {e}");
                }
            }
        }
        Ok(())
    }

    /// Persist mutation lineage to disk.
    pub async fn save_lineage(&self) -> Result<()> {
        let dir = self.workspace_dir.join(".housaky");
        tokio::fs::create_dir_all(&dir).await?;
        let path = dir.join("mutation_lineage.json");
        let json = serde_json::to_string_pretty(&*self.lineage.read().await)?;
        tokio::fs::write(&path, json).await?;
        Ok(())
    }

    /// Full AST self-modification pipeline (DGM-aligned):
    /// 1. DGM parent selection from archive  2. Parse target  3. Apply mutation
    /// 4. Safety check  5. Build + test in sandbox  6. Fitness gate
    /// 7. Alignment gate (hard — cannot be optimised away)  8. Merge or discard
    /// 9. Update lineage + persist
    pub async fn apply_mutation_cycle(
        &self,
        mutation: &AtomicMutation,
    ) -> Result<MutationNode> {
        if !self.config.enable_ast_mutations {
            anyhow::bail!(
                "AST mutations disabled. Set self_modification.enable_ast_mutations = true."
            );
        }

        // Safety: reject forbidden modules
        for forbidden in &self.config.forbidden_modules {
            if mutation.target.file.contains(forbidden.as_str()) {
                anyhow::bail!(
                    "Mutation targets forbidden module '{}' in '{}'",
                    forbidden,
                    mutation.target.file
                );
            }
        }

        let target_path = self.workspace_dir.join(&mutation.target.file);
        if !target_path.exists() {
            anyhow::bail!("Target file does not exist: {:?}", target_path);
        }

        // DGM §8.2 — select parent from archive using fitness + novelty weighting.
        // This determines the fitness_before baseline for the improvement gate.
        let (parent_id, fitness_before) = {
            let lineage = self.lineage.read().await;
            let parent = lineage.select_parent_dgm();
            let parent_id = parent.map(|n| n.id.clone());
            let fitness_before = parent.map(|n| n.fitness_after).unwrap_or(0.0);
            (parent_id, fitness_before)
        };

        info!(
            "DGM parent selected: {:?} (fitness_before={:.4})",
            parent_id, fitness_before
        );

        // Apply the AST mutation to get new source
        let new_source =
            AstEngine::apply_mutation(&target_path, &mutation.op, &mutation.target)?;

        // Safety scan on the new source
        let safety_report = self
            .safety_oracle
            .evaluate(&self.workspace_dir, &[(mutation.target.file.clone(), new_source.clone())]);

        if !safety_report.passed {
            let violations: Vec<_> = safety_report
                .violations
                .iter()
                .filter(|v| v.severity == Severity::Block)
                .map(|v| v.description.clone())
                .collect();
            anyhow::bail!("Safety check failed: {:?}", violations);
        }

        // Create sandbox, apply mutation, build + test
        let mut sandbox = GitSandbox::new(self.workspace_dir.clone());
        let session = sandbox.create_session(&format!("ast-mut-{}", mutation.id))?;

        info!("AST mutation sandbox: {}", session.id);

        sandbox.apply_modification(&session.id, &mutation.target.file, &new_source)?;

        let validation = sandbox.validate_session(&session.id)?;

        // Run full benchmark suite including capability retention (DGM §8.5).
        let (fitness_after, full_fitness_score) = if validation.compiles {
            let benchmarks = self
                .fitness_evaluator
                .run_benchmarks(&session.worktree_path);
            let score = self.fitness_evaluator.compute_fitness(&benchmarks);
            (score.overall, Some(score))
        } else {
            (0.0, None)
        };

        // DGM §8.5 — alignment gate: alignment score must not regress.
        if let Some(ref score) = full_fitness_score {
            // Use persisted baseline if available; fall back to synthetic one from parent fitness.
            let gate_evaluator = if self.fitness_evaluator.baseline.is_some() {
                FitnessEvaluator::with_baseline(self.fitness_evaluator.baseline.clone().unwrap())
            } else {
                let temp_baseline = FitnessScore {
                    overall: fitness_before,
                    latency_score: 0.0,
                    memory_score: 0.0,
                    correctness_score: 0.0,
                    capability_score: 0.0,
                    alignment_score: 1.0,
                    details: Default::default(),
                };
                FitnessEvaluator::with_baseline(temp_baseline)
            };
            if let Err(reason) = gate_evaluator.alignment_gate(score) {
                warn!("Mutation {} blocked by alignment gate: {}", mutation.id, reason);
                let _ = sandbox.discard_session(&session.id);
                let node = MutationNode {
                    id: mutation.id.clone(),
                    parent_id,
                    mutation_op: format!("{:?}", mutation.op),
                    target_file: mutation.target.file.clone(),
                    target_function: mutation.target.function_name.clone(),
                    rationale: mutation.rationale.clone(),
                    fitness_before,
                    fitness_after: 0.0,
                    applied: false,
                    rolled_back: false,
                    timestamp: chrono::Utc::now(),
                    rollback_patch: String::new(),
                };
                self.lineage.write().await.add_node(node.clone());
                let _ = self.save_lineage().await;
                return Ok(node);
            }
        }

        // Fitness improvement gate.
        let delta = fitness_after - fitness_before;
        let mut applied = validation.no_regressions
            && (!self.config.require_benchmark_improvement
                || delta >= self.config.min_fitness_delta);

        // §3.3 — Consensus gate: if a consensus protocol is attached, propose
        // the mutation and check for veto before applying.  High-risk mutations
        // (targeting core modules) require consensus approval.
        if applied {
            if let Some(ref consensus) = self.consensus {
                use crate::housaky::alignment::consensus_mod::{
                    SelfModificationRecord, ModificationType, ProposalStatus,
                };
                let mod_record = SelfModificationRecord {
                    id: mutation.id.clone(),
                    modification_type: ModificationType::AlgorithmChange,
                    description: format!("AST mutation {:?} on {}", mutation.op, mutation.target.file),
                    target_component: mutation.target.file.clone(),
                    old_value: "(AST before)".to_string(),
                    new_value: "(AST after)".to_string(),
                    rationale: mutation.rationale.clone(),
                    expected_improvement: delta,
                    reversible: true,
                };
                match consensus.propose(mod_record, "self_modification_engine").await {
                    Ok(proposal_id) => {
                        let proposals = consensus.modification_proposals.read().await;
                        if let Some(p) = proposals.iter().find(|p| p.id == proposal_id) {
                            if p.status == ProposalStatus::Vetoed {
                                warn!("Mutation {} vetoed by consensus protocol", mutation.id);
                                applied = false;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Consensus proposal failed for {}: {e}", mutation.id);
                    }
                }
            }
        }

        if applied {
            info!(
                "Mutation {} APPLIED — fitness_before={:.4} fitness_after={:.4} delta={:+.4}",
                mutation.id, fitness_before, fitness_after, delta
            );
            sandbox.merge_session(&session.id)?;
        } else {
            warn!(
                "Mutation {} REJECTED — compiles={} no_regressions={} delta={:+.4} (min={:.4})",
                mutation.id,
                validation.compiles,
                validation.no_regressions,
                delta,
                self.config.min_fitness_delta
            );
            let _ = sandbox.discard_session(&session.id);
        }

        let node = MutationNode {
            id: mutation.id.clone(),
            parent_id,
            mutation_op: format!("{:?}", mutation.op),
            target_file: mutation.target.file.clone(),
            target_function: mutation.target.function_name.clone(),
            rationale: mutation.rationale.clone(),
            fitness_before,
            fitness_after,
            applied,
            rolled_back: false,
            timestamp: chrono::Utc::now(),
            rollback_patch: String::new(),
        };

        self.lineage.write().await.add_node(node.clone());

        // Persist fitness baseline after successful mutations so the alignment
        // gate has a real reference point across restarts. We write directly to
        // disk rather than mutating `self.fitness_evaluator` (which is behind `&self`).
        if applied {
            if let Some(ref score) = full_fitness_score {
                let temp_eval = FitnessEvaluator::with_baseline(score.clone());
                if let Err(e) = temp_eval.save_baseline(&self.workspace_dir) {
                    warn!("Failed to persist fitness baseline: {e}");
                }
            }
        }

        // Persist lineage after every cycle so it survives restarts.
        if let Err(e) = self.save_lineage().await {
            warn!("Failed to persist mutation lineage: {e}");
        }

        Ok(node)
    }

    pub async fn rollback_last(&self) -> Result<()> {
        let mut lineage = self.lineage.write().await;
        if let Some(ref head_id) = lineage.current_head.clone() {
            lineage.mark_rolled_back(head_id);
            info!("Rolled back mutation node {}", head_id);
            drop(lineage);
            if let Err(e) = self.save_lineage().await {
                warn!("Failed to persist lineage after rollback: {e}");
            }
            Ok(())
        } else {
            anyhow::bail!("No applied mutations to roll back")
        }
    }

    pub async fn get_lineage(&self) -> MutationLineage {
        self.lineage.read().await.clone()
    }
}

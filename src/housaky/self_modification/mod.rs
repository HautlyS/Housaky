pub mod ast_engine;
pub mod fitness_eval;
pub mod lineage;
pub mod mutation_ops;
pub mod safety_oracle;

pub use ast_engine::AstEngine;
pub use fitness_eval::{BenchmarkResult, FitnessEvaluator, FitnessScore, FitnessWeights};
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
}

impl SelfModificationEngine {
    pub fn new(workspace_dir: PathBuf) -> Self {
        let config = SelfModificationConfig::default();
        let forbidden = config.forbidden_modules.clone();
        Self {
            safety_oracle: SafetyOracle::with_config(vec![], forbidden, false),
            fitness_evaluator: FitnessEvaluator::new(),
            lineage: Arc::new(RwLock::new(MutationLineage::new())),
            config,
            workspace_dir,
        }
    }

    pub fn with_config(workspace_dir: PathBuf, config: SelfModificationConfig) -> Self {
        let forbidden = config.forbidden_modules.clone();
        Self {
            safety_oracle: SafetyOracle::with_config(vec![], forbidden, false),
            fitness_evaluator: FitnessEvaluator::new(),
            lineage: Arc::new(RwLock::new(MutationLineage::new())),
            config,
            workspace_dir,
        }
    }

    /// Full AST self-modification pipeline:
    /// 1. Parse target file  2. Apply mutation  3. Safety check  4. Build + test in sandbox
    /// 5. Fitness gate  6. Merge or discard
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

        let fitness_after = if validation.compiles {
            let benchmarks = self
                .fitness_evaluator
                .run_benchmarks(&session.worktree_path);
            let score = self.fitness_evaluator.compute_fitness(&benchmarks);
            score.overall
        } else {
            0.0
        };

        let applied = validation.no_regressions
            && (!self.config.require_benchmark_improvement
                || self
                    .fitness_evaluator
                    .improvement_delta(&FitnessScore {
                        overall: fitness_after,
                        latency_score: 0.0,
                        memory_score: 0.0,
                        correctness_score: 0.0,
                        capability_score: 0.0,
                        details: Default::default(),
                    })
                    >= self.config.min_fitness_delta);

        if applied {
            info!(
                "Mutation {} APPLIED — fitness_after={:.4}",
                mutation.id, fitness_after
            );
            sandbox.merge_session(&session.id)?;
        } else {
            warn!("Mutation {} REJECTED — discarding sandbox", mutation.id);
            let _ = sandbox.discard_session(&session.id);
        }

        let node = MutationNode {
            id: mutation.id.clone(),
            parent_id: {
                let lineage = self.lineage.read().await;
                lineage.current_head.clone()
            },
            mutation_op: format!("{:?}", mutation.op),
            target_file: mutation.target.file.clone(),
            target_function: mutation.target.function_name.clone(),
            rationale: mutation.rationale.clone(),
            fitness_before: 0.0,
            fitness_after,
            applied,
            rolled_back: false,
            timestamp: chrono::Utc::now(),
            rollback_patch: String::new(),
        };

        self.lineage.write().await.add_node(node.clone());

        Ok(node)
    }

    pub async fn rollback_last(&self) -> Result<()> {
        let mut lineage = self.lineage.write().await;
        if let Some(ref head_id) = lineage.current_head.clone() {
            lineage.mark_rolled_back(head_id);
            info!("Rolled back mutation node {}", head_id);
            Ok(())
        } else {
            anyhow::bail!("No applied mutations to roll back")
        }
    }

    pub async fn get_lineage(&self) -> MutationLineage {
        self.lineage.read().await.clone()
    }
}

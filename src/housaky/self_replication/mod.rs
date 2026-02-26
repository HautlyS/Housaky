pub mod compiler;
pub mod genome;
pub mod hot_swap;
pub mod validator;

pub use compiler::SandboxCompiler;
pub use genome::{
    BuildResult, GenerationLineage, MutationKind, ReplicationCycle, SourceMutation, TestResult,
};
pub use hot_swap::HotSwapper;
pub use validator::BinaryValidator;

use crate::housaky::git_sandbox::{GitSandbox, SandboxConfig};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

pub struct SelfReplicationEngine {
    pub workspace_dir: PathBuf,
    pub compiler: SandboxCompiler,
    pub validator: BinaryValidator,
    pub hot_swapper: HotSwapper,
    pub lineage: Arc<RwLock<GenerationLineage>>,
    pub config: ReplicationConfig,
}

#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    pub enable_replication: bool,
    pub max_mutations_per_cycle: usize,
    pub require_tests: bool,
    pub require_benchmark_improvement: bool,
    pub min_fitness_delta: f64,
    pub max_build_time_secs: u64,
    pub binary_size_regression_pct: f64,
    pub forbidden_modules: Vec<String>,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            enable_replication: false,
            max_mutations_per_cycle: 3,
            require_tests: true,
            require_benchmark_improvement: false,
            min_fitness_delta: 0.02,
            max_build_time_secs: 300,
            binary_size_regression_pct: 5.0,
            forbidden_modules: vec!["security".to_string(), "alignment".to_string()],
        }
    }
}

impl SelfReplicationEngine {
    pub fn new(workspace_dir: PathBuf) -> Self {
        let config = ReplicationConfig::default();
        Self {
            compiler: SandboxCompiler::new(workspace_dir.clone()),
            validator: BinaryValidator::new(),
            hot_swapper: HotSwapper::new(workspace_dir.clone()),
            lineage: Arc::new(RwLock::new(GenerationLineage::new())),
            config,
            workspace_dir,
        }
    }

    pub fn with_config(workspace_dir: PathBuf, config: ReplicationConfig) -> Self {
        Self {
            compiler: SandboxCompiler::with_config(
                workspace_dir.clone(),
                config.max_build_time_secs,
                config.binary_size_regression_pct,
            ),
            validator: BinaryValidator::with_config(
                config.require_tests,
                config.min_fitness_delta,
            ),
            hot_swapper: HotSwapper::new(workspace_dir.clone()),
            lineage: Arc::new(RwLock::new(GenerationLineage::new())),
            config,
            workspace_dir,
        }
    }

    /// Execute a full replication cycle: sandbox → apply mutations → build → test → gate → promote.
    pub async fn run_cycle(
        &self,
        mutations: Vec<SourceMutation>,
        generation: u64,
    ) -> Result<ReplicationCycle> {
        if !self.config.enable_replication {
            anyhow::bail!("Self-replication is disabled. Set enable_replication = true to enable.");
        }

        self.validate_mutations(&mutations)?;

        let sandbox_cfg = SandboxConfig {
            max_build_time_secs: self.config.max_build_time_secs,
            ..SandboxConfig::default()
        };
        let mut sandbox = GitSandbox::with_config(self.workspace_dir.clone(), sandbox_cfg);
        let session = sandbox.create_session(&format!("gen-{}", generation))?;

        info!(
            "Replication cycle {}: sandbox '{}' created at {:?}",
            generation, session.id, session.worktree_path
        );

        self.compiler
            .apply_mutations_to_worktree(&session.worktree_path, &mutations)?;

        let baseline_size = self.compiler.get_current_binary_size();
        let parent_binary_hash = self.compiler.get_current_binary_hash();

        let build_result = self.compiler.build_release(&session.worktree_path);

        let test_results = if build_result.success && self.config.require_tests {
            self.validator
                .run_tests(&session.worktree_path)
                .unwrap_or_else(|e| {
                    warn!("Test run failed: {}", e);
                    vec![TestResult {
                        name: "test_run_error".to_string(),
                        passed: false,
                        duration_ms: 0,
                        output: e.to_string(),
                    }]
                })
        } else {
            vec![]
        };

        let (gates_pass, gate_failures) = self.validator.passes_gates(
            &build_result,
            &test_results,
            baseline_size,
            self.config.binary_size_regression_pct,
        );

        if !gates_pass {
            warn!(
                "Cycle {} failed gates: {:?}. Discarding.",
                generation, gate_failures
            );
            let _ = sandbox.discard_session(&session.id);
        }

        let fitness_score = self.compute_fitness(&build_result, &test_results, gates_pass);

        let cycle = ReplicationCycle {
            generation,
            parent_binary_hash,
            mutations,
            build_result,
            test_results,
            fitness_score,
            promoted: gates_pass,
            created_at: chrono::Utc::now(),
        };

        if gates_pass {
            info!(
                "Cycle {} PROMOTED — fitness={:.4}",
                generation, fitness_score
            );
        }

        self.lineage.write().await.record_cycle(cycle.clone());

        Ok(cycle)
    }

    fn validate_mutations(&self, mutations: &[SourceMutation]) -> Result<()> {
        for m in mutations {
            for forbidden in &self.config.forbidden_modules {
                if m.file.contains(forbidden.as_str()) {
                    anyhow::bail!(
                        "Mutation targets forbidden module '{}' in file '{}'",
                        forbidden,
                        m.file
                    );
                }
            }
        }
        if mutations.len() > self.config.max_mutations_per_cycle {
            anyhow::bail!(
                "Too many mutations: {} > max {}",
                mutations.len(),
                self.config.max_mutations_per_cycle
            );
        }
        Ok(())
    }

    fn compute_fitness(
        &self,
        build: &BuildResult,
        tests: &[TestResult],
        gates_pass: bool,
    ) -> f64 {
        if !gates_pass || !build.success {
            return 0.0;
        }
        let test_pass_rate = if tests.is_empty() {
            0.5
        } else {
            tests.iter().filter(|t| t.passed).count() as f64 / tests.len() as f64
        };
        let warning_penalty = (build.warnings.len() as f64 * 0.01).min(0.1);
        (0.6 * test_pass_rate + 0.4 * 1.0 - warning_penalty).clamp(0.0, 1.0)
    }

    pub async fn get_lineage(&self) -> GenerationLineage {
        self.lineage.read().await.clone()
    }
}

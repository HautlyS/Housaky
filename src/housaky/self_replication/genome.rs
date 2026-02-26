use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationCycle {
    pub generation: u64,
    pub parent_binary_hash: String,
    pub mutations: Vec<SourceMutation>,
    pub build_result: BuildResult,
    pub test_results: Vec<TestResult>,
    pub fitness_score: f64,
    pub promoted: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMutation {
    pub file: String,
    pub kind: MutationKind,
    pub diff: String,
    pub rationale: String,
    pub confidence: f64,
    pub rollback_patch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MutationKind {
    AddFunction,
    ModifyParameter,
    RefactorAlgorithm,
    AddDependency,
    AddCaching,
    AddLogging,
    OptimizeHotPath,
    AddTraitImpl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildResult {
    pub success: bool,
    pub binary_path: Option<String>,
    pub binary_hash: Option<String>,
    pub binary_size_bytes: u64,
    pub compile_time_secs: f64,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub duration_ms: u64,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationLineage {
    pub generations: Vec<ReplicationCycle>,
    pub current_generation: u64,
    pub best_generation: u64,
    pub total_mutations_applied: u64,
    pub total_mutations_rejected: u64,
}

impl GenerationLineage {
    pub fn new() -> Self {
        Self {
            generations: Vec::new(),
            current_generation: 0,
            best_generation: 0,
            total_mutations_applied: 0,
            total_mutations_rejected: 0,
        }
    }

    pub fn record_cycle(&mut self, cycle: ReplicationCycle) {
        if cycle.promoted {
            self.total_mutations_applied += cycle.mutations.len() as u64;
            if cycle.fitness_score
                > self
                    .generations
                    .iter()
                    .map(|c| c.fitness_score)
                    .fold(0.0f64, f64::max)
            {
                self.best_generation = cycle.generation;
            }
        } else {
            self.total_mutations_rejected += cycle.mutations.len() as u64;
        }
        self.current_generation = cycle.generation;
        self.generations.push(cycle);
    }

    pub fn best_cycle(&self) -> Option<&ReplicationCycle> {
        self.generations
            .iter()
            .filter(|c| c.promoted)
            .max_by(|a, b| a.fitness_score.partial_cmp(&b.fitness_score).unwrap())
    }
}

impl Default for GenerationLineage {
    fn default() -> Self {
        Self::new()
    }
}

//! Architecture Evaluator — Evaluate architecture fitness on standardised benchmarks.
//!
//! Runs a battery of micro-benchmarks against a candidate `ArchitectureGenome`
//! and computes a multi-dimensional `ArchitectureFitness` score.

use crate::housaky::architecture_search::data_flow_graph::DataFlowGraph;
use crate::housaky::architecture_search::module_genome::{
    ArchitectureFitness, ArchitectureGenome, ModuleType,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::info;

// ── Benchmark Suite ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSuite {
    pub benchmarks: Vec<ArchitectureBenchmark>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureBenchmark {
    pub name: String,
    pub category: BenchmarkCategory,
    pub weight: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BenchmarkCategory {
    ReasoningLatency,
    MemoryEfficiency,
    TaskCompletion,
    ModuleIntegration,
    AlignmentCompliance,
    ResourceUsage,
    Modularity,
}

impl Default for BenchmarkSuite {
    fn default() -> Self {
        Self {
            benchmarks: vec![
                ArchitectureBenchmark {
                    name: "reasoning_latency".to_string(),
                    category: BenchmarkCategory::ReasoningLatency,
                    weight: 0.20,
                    description: "Measure time-to-first-token for reasoning tasks".to_string(),
                },
                ArchitectureBenchmark {
                    name: "memory_hit_rate".to_string(),
                    category: BenchmarkCategory::MemoryEfficiency,
                    weight: 0.15,
                    description: "Cache hit rate across memory modules".to_string(),
                },
                ArchitectureBenchmark {
                    name: "task_completion_rate".to_string(),
                    category: BenchmarkCategory::TaskCompletion,
                    weight: 0.25,
                    description: "Fraction of benchmark tasks completed correctly".to_string(),
                },
                ArchitectureBenchmark {
                    name: "module_integration".to_string(),
                    category: BenchmarkCategory::ModuleIntegration,
                    weight: 0.15,
                    description: "How well modules share information (connectivity score)".to_string(),
                },
                ArchitectureBenchmark {
                    name: "alignment_compliance".to_string(),
                    category: BenchmarkCategory::AlignmentCompliance,
                    weight: 0.15,
                    description: "Alignment constraint satisfaction rate".to_string(),
                },
                ArchitectureBenchmark {
                    name: "resource_efficiency".to_string(),
                    category: BenchmarkCategory::ResourceUsage,
                    weight: 0.10,
                    description: "Resource usage relative to capability delivered".to_string(),
                },
            ],
        }
    }
}

// ── Evaluation Result ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub genome_id: String,
    pub fitness: ArchitectureFitness,
    pub benchmark_scores: HashMap<String, f64>,
    pub evaluation_duration: Duration,
    pub passed_all_gates: bool,
    pub gate_failures: Vec<String>,
}

// ── Fitness Gates ─────────────────────────────────────────────────────────────

/// Minimum requirements an architecture must pass before being promoted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessGates {
    pub min_overall_fitness: f64,
    pub min_alignment_score: f64,
    pub min_task_completion: f64,
    pub max_memory_budget_mb: u64,
    pub require_no_cycles: bool,
}

impl Default for FitnessGates {
    fn default() -> Self {
        Self {
            min_overall_fitness: 0.50,
            min_alignment_score: 0.80,
            min_task_completion: 0.60,
            max_memory_budget_mb: 4096,
            require_no_cycles: true,
        }
    }
}

// ── Architecture Evaluator ────────────────────────────────────────────────────

pub struct ArchitectureEvaluator {
    pub suite: BenchmarkSuite,
    pub gates: FitnessGates,
}

impl ArchitectureEvaluator {
    pub fn new() -> Self {
        Self {
            suite: BenchmarkSuite::default(),
            gates: FitnessGates::default(),
        }
    }

    pub fn with_gates(gates: FitnessGates) -> Self {
        Self {
            suite: BenchmarkSuite::default(),
            gates,
        }
    }

    /// Evaluate a genome, returning a rich EvaluationResult.
    pub fn evaluate(&self, genome: &ArchitectureGenome) -> Result<EvaluationResult> {
        let start = Instant::now();

        let graph = DataFlowGraph::build(&genome.modules, &genome.connections);
        let graph_summary = graph.summary();

        let enabled_modules = genome.enabled_module_count();
        let total_memory = genome.total_memory_budget_mb();

        let mut scores: HashMap<String, f64> = HashMap::new();

        // ── Per-benchmark heuristic scoring ──────────────────────────────────
        // In production these will wire to real task runners; here we derive
        // proxy scores from structural properties of the genome.

        // 1. Reasoning latency proxy: fewer critical-path hops → lower latency
        let latency_score = {
            let depth = graph_summary.max_depth as f64;
            (1.0 / (1.0 + depth * 0.1)).clamp(0.0, 1.0)
        };
        scores.insert("reasoning_latency".to_string(), latency_score);

        // 2. Memory efficiency: presence of memory modules + connectivity
        let memory_score = {
            let has_memory = genome
                .modules
                .iter()
                .any(|m| m.enabled && m.module_type == ModuleType::Memory);
            let base = if has_memory { 0.6 } else { 0.2 };
            (base + graph_summary.connectivity_score * 0.4).clamp(0.0, 1.0)
        };
        scores.insert("memory_hit_rate".to_string(), memory_score);

        // 3. Task completion: enabled module coverage score
        let task_score = {
            let coverage = Self::module_type_coverage(genome);
            coverage.clamp(0.0, 1.0)
        };
        scores.insert("task_completion_rate".to_string(), task_score);

        // 4. Module integration: connectivity score from graph
        let integration_score = graph_summary.connectivity_score;
        scores.insert("module_integration".to_string(), integration_score);

        // 5. Alignment compliance: always have an alignment module?
        let alignment_score = {
            let has_alignment = genome
                .modules
                .iter()
                .any(|m| m.enabled && m.module_type == ModuleType::Alignment);
            if has_alignment { 0.95 } else { 0.50 }
        };
        scores.insert("alignment_compliance".to_string(), alignment_score);

        // 6. Resource efficiency: capability per MB
        let resource_score = {
            if total_memory == 0 {
                0.0
            } else {
                let capability = task_score * enabled_modules as f64;
                (capability / (total_memory as f64 / 256.0)).clamp(0.0, 1.0)
            }
        };
        scores.insert("resource_efficiency".to_string(), resource_score);

        // ── Composite fitness ─────────────────────────────────────────────────
        let mut fitness = ArchitectureFitness {
            reasoning_score: latency_score,
            memory_efficiency: memory_score,
            task_completion_rate: task_score,
            latency_score,
            alignment_score,
            modularity_score: 1.0 - graph_summary.isolated_modules as f64 / (enabled_modules.max(1) as f64),
            resource_efficiency: resource_score,
            ..Default::default()
        };
        fitness.compute_overall();

        for (k, v) in &scores {
            fitness.benchmark_details.insert(k.clone(), *v);
        }

        // ── Gate checks ───────────────────────────────────────────────────────
        let mut gate_failures = Vec::new();

        if fitness.overall < self.gates.min_overall_fitness {
            gate_failures.push(format!(
                "Overall fitness {:.3} < minimum {:.3}",
                fitness.overall, self.gates.min_overall_fitness
            ));
        }
        if fitness.alignment_score < self.gates.min_alignment_score {
            gate_failures.push(format!(
                "Alignment score {:.3} < minimum {:.3}",
                fitness.alignment_score, self.gates.min_alignment_score
            ));
        }
        if fitness.task_completion_rate < self.gates.min_task_completion {
            gate_failures.push(format!(
                "Task completion {:.3} < minimum {:.3}",
                fitness.task_completion_rate, self.gates.min_task_completion
            ));
        }
        if total_memory > self.gates.max_memory_budget_mb {
            gate_failures.push(format!(
                "Memory budget {} MB > max {} MB",
                total_memory, self.gates.max_memory_budget_mb
            ));
        }
        if self.gates.require_no_cycles && graph_summary.has_cycles {
            gate_failures.push("Data flow graph contains cycles".to_string());
        }

        let passed_all_gates = gate_failures.is_empty();
        let duration = start.elapsed();

        info!(
            genome_id = %genome.id,
            overall = %fitness.overall,
            passed = %passed_all_gates,
            "Architecture evaluated"
        );

        Ok(EvaluationResult {
            genome_id: genome.id.clone(),
            fitness,
            benchmark_scores: scores,
            evaluation_duration: duration,
            passed_all_gates,
            gate_failures,
        })
    }

    /// Compute how many of the core module types are present and enabled.
    fn module_type_coverage(genome: &ArchitectureGenome) -> f64 {
        let core_types = [
            ModuleType::Reasoning,
            ModuleType::Memory,
            ModuleType::Planning,
            ModuleType::Learning,
            ModuleType::Meta,
            ModuleType::Action,
            ModuleType::Alignment,
        ];
        let present = core_types
            .iter()
            .filter(|t| {
                genome
                    .modules
                    .iter()
                    .any(|m| m.enabled && &m.module_type == *t)
            })
            .count();
        present as f64 / core_types.len() as f64
    }

    /// Compare two genomes; return true if `candidate` is better than `baseline`.
    pub fn is_improvement(
        &self,
        candidate: &EvaluationResult,
        baseline: &EvaluationResult,
        min_delta: f64,
    ) -> bool {
        candidate.passed_all_gates
            && candidate.fitness.overall >= baseline.fitness.overall + min_delta
    }
}

impl Default for ArchitectureEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

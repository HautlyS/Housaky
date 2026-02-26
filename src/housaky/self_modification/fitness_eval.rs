use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::time::Instant;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessScore {
    pub overall: f64,
    pub latency_score: f64,
    pub memory_score: f64,
    pub correctness_score: f64,
    pub capability_score: f64,
    pub details: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub duration_ms: f64,
    pub success: bool,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessWeights {
    pub latency: f64,
    pub memory: f64,
    pub correctness: f64,
    pub capability: f64,
}

impl Default for FitnessWeights {
    fn default() -> Self {
        Self {
            latency: 0.25,
            memory: 0.15,
            correctness: 0.40,
            capability: 0.20,
        }
    }
}

pub struct FitnessEvaluator {
    pub weights: FitnessWeights,
    pub baseline: Option<FitnessScore>,
}

impl FitnessEvaluator {
    pub fn new() -> Self {
        Self {
            weights: FitnessWeights::default(),
            baseline: None,
        }
    }

    pub fn with_baseline(baseline: FitnessScore) -> Self {
        Self {
            weights: FitnessWeights::default(),
            baseline: Some(baseline),
        }
    }

    pub fn run_benchmarks(&self, worktree_path: &Path) -> Vec<BenchmarkResult> {
        let mut results = Vec::new();

        let start = Instant::now();
        let output = Command::new("cargo")
            .args(["test", "--lib", "--", "--test-threads=4", "-q"])
            .current_dir(worktree_path)
            .env("CARGO_TERM_COLOR", "never")
            .output();

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

        let (success, out) = match output {
            Ok(o) => (
                o.status.success(),
                format!(
                    "{}{}",
                    String::from_utf8_lossy(&o.stdout),
                    String::from_utf8_lossy(&o.stderr)
                ),
            ),
            Err(e) => (false, e.to_string()),
        };

        results.push(BenchmarkResult {
            name: "test_suite".to_string(),
            duration_ms,
            success,
            output: out,
        });

        // Check compilation time as a proxy for code quality
        let start = Instant::now();
        let check = Command::new("cargo")
            .args(["check", "--lib", "-q"])
            .current_dir(worktree_path)
            .env("CARGO_TERM_COLOR", "never")
            .output();
        let check_ms = start.elapsed().as_secs_f64() * 1000.0;

        let check_ok = check.map(|o| o.status.success()).unwrap_or(false);
        results.push(BenchmarkResult {
            name: "check_time".to_string(),
            duration_ms: check_ms,
            success: check_ok,
            output: String::new(),
        });

        info!("Benchmarks completed: {} results", results.len());
        results
    }

    pub fn compute_fitness(&self, benchmarks: &[BenchmarkResult]) -> FitnessScore {
        let test_bench = benchmarks.iter().find(|b| b.name == "test_suite");
        let check_bench = benchmarks.iter().find(|b| b.name == "check_time");

        let correctness_score = test_bench.map(|b| if b.success { 1.0 } else { 0.0 }).unwrap_or(0.5);

        // Latency score: faster check = better (normalize to ~5s baseline)
        let latency_score = check_bench
            .map(|b| {
                if b.success {
                    (5000.0 / b.duration_ms.max(100.0)).min(1.0)
                } else {
                    0.0
                }
            })
            .unwrap_or(0.5);

        // Memory score: proxy from binary size (not measured here, default 0.7)
        let memory_score = 0.7;

        // Capability score: proxy from test count
        let capability_score = if correctness_score > 0.8 { 0.8 } else { 0.4 };

        let overall = self.weights.latency * latency_score
            + self.weights.memory * memory_score
            + self.weights.correctness * correctness_score
            + self.weights.capability * capability_score;

        let mut details = HashMap::new();
        details.insert("test_pass".to_string(), correctness_score);
        details.insert("check_ms".to_string(), check_bench.map(|b| b.duration_ms).unwrap_or(0.0));

        FitnessScore {
            overall,
            latency_score,
            memory_score,
            correctness_score,
            capability_score,
            details,
        }
    }

    pub fn improvement_delta(&self, new_score: &FitnessScore) -> f64 {
        match &self.baseline {
            Some(base) => new_score.overall - base.overall,
            None => 0.0,
        }
    }

    pub fn meets_threshold(&self, new_score: &FitnessScore, min_delta: f64) -> bool {
        self.improvement_delta(new_score) >= min_delta
    }
}

impl Default for FitnessEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

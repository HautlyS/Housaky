use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

// ── ParameterGenome ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningWeights {
    pub cot_prior: f64,
    pub react_prior: f64,
    pub tot_prior: f64,
    pub reflexion_prior: f64,
    pub self_consistency_prior: f64,
}

impl Default for ReasoningWeights {
    fn default() -> Self {
        Self {
            cot_prior: 0.3,
            react_prior: 0.3,
            tot_prior: 0.2,
            reflexion_prior: 0.1,
            self_consistency_prior: 0.1,
        }
    }
}

impl ReasoningWeights {
    pub fn to_vec(&self) -> Vec<f64> {
        vec![
            self.cot_prior,
            self.react_prior,
            self.tot_prior,
            self.reflexion_prior,
            self.self_consistency_prior,
        ]
    }

    pub fn from_vec(v: &[f64]) -> Self {
        let sum: f64 = v.iter().sum::<f64>().max(1e-9);
        Self {
            cot_prior: v.get(0).copied().unwrap_or(0.2) / sum,
            react_prior: v.get(1).copied().unwrap_or(0.2) / sum,
            tot_prior: v.get(2).copied().unwrap_or(0.2) / sum,
            reflexion_prior: v.get(3).copied().unwrap_or(0.2) / sum,
            self_consistency_prior: v.get(4).copied().unwrap_or(0.2) / sum,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterGenome {
    pub reasoning_weights: ReasoningWeights,
    pub attention_decay: f64,
    pub learning_rate: f64,
    pub exploration_rate: f64,
    pub confidence_threshold: f64,
    pub risk_tolerance: f64,
    pub memory_consolidation_frequency: u64,
    pub tool_selection_bias: HashMap<String, f64>,
}

impl Default for ParameterGenome {
    fn default() -> Self {
        Self {
            reasoning_weights: ReasoningWeights::default(),
            attention_decay: 0.1,
            learning_rate: 0.01,
            exploration_rate: 0.2,
            confidence_threshold: 0.75,
            risk_tolerance: 0.3,
            memory_consolidation_frequency: 100,
            tool_selection_bias: HashMap::new(),
        }
    }
}

impl ParameterGenome {
    /// Flatten genome to a parameter vector for CMA-ES.
    pub fn to_vec(&self) -> Vec<f64> {
        let mut v = self.reasoning_weights.to_vec();
        v.push(self.attention_decay);
        v.push(self.learning_rate);
        v.push(self.exploration_rate);
        v.push(self.confidence_threshold);
        v.push(self.risk_tolerance);
        v.push(self.memory_consolidation_frequency as f64 / 1000.0);
        v
    }

    /// Reconstruct genome from a parameter vector.
    pub fn from_vec(v: &[f64]) -> Self {
        let rw = ReasoningWeights::from_vec(&v[..5.min(v.len())]);
        Self {
            reasoning_weights: rw,
            attention_decay: v.get(5).copied().unwrap_or(0.1).clamp(0.0, 1.0),
            learning_rate: v.get(6).copied().unwrap_or(0.01).clamp(0.0, 1.0),
            exploration_rate: v.get(7).copied().unwrap_or(0.2).clamp(0.0, 1.0),
            confidence_threshold: v.get(8).copied().unwrap_or(0.75).clamp(0.0, 1.0),
            risk_tolerance: v.get(9).copied().unwrap_or(0.3).clamp(0.0, 1.0),
            memory_consolidation_frequency: ((v.get(10).copied().unwrap_or(0.1).clamp(0.0, 1.0))
                * 1000.0) as u64,
            tool_selection_bias: HashMap::new(),
        }
    }

    pub fn dim() -> usize {
        11 // 5 reasoning + 6 scalar params
    }
}

// ── FitnessFunction ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessFunction {
    pub task_completion_weight: f64,
    pub speed_weight: f64,
    pub cost_weight: f64,
    pub novelty_weight: f64,
    pub alignment_penalty: f64,
}

impl Default for FitnessFunction {
    fn default() -> Self {
        Self {
            task_completion_weight: 0.40,
            speed_weight: 0.20,
            cost_weight: 0.15,
            novelty_weight: 0.15,
            alignment_penalty: 0.10,
        }
    }
}

impl FitnessFunction {
    pub fn evaluate(
        &self,
        task_completion: f64,
        speed_score: f64,
        cost_score: f64,
        novelty_score: f64,
        alignment_drift: f64,
    ) -> f64 {
        let raw = self.task_completion_weight * task_completion
            + self.speed_weight * speed_score
            + self.cost_weight * cost_score
            + self.novelty_weight * novelty_score;
        let penalty = self.alignment_penalty * alignment_drift;
        (raw - penalty).clamp(0.0, 1.0)
    }
}

// ── Task replay record ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: String,
    pub description: String,
    pub completion_score: f64,
    pub speed_ms: u64,
    pub cost_tokens: u64,
    pub novelty: f64,
    pub alignment_drift: f64,
    pub genome_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}

// ── CMA-ES Optimizer ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CMAESOptimizer {
    pub population_size: usize,
    pub sigma: f64,
    pub mean: Vec<f64>,
    pub covariance_diagonal: Vec<f64>, // simplified diagonal covariance for efficiency
    pub generation: u64,
    pub best_fitness: f64,
    pub best_genome: ParameterGenome,
    pub history: Vec<CMAESGeneration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CMAESGeneration {
    pub generation: u64,
    pub best_fitness: f64,
    pub mean_fitness: f64,
    pub sigma: f64,
    pub timestamp: DateTime<Utc>,
}

impl CMAESOptimizer {
    pub fn new(population_size: usize) -> Self {
        let dim = ParameterGenome::dim();
        let default_genome = ParameterGenome::default();
        let mean = default_genome.to_vec();
        Self {
            population_size,
            sigma: 0.3,
            mean: mean.clone(),
            covariance_diagonal: vec![1.0; dim],
            generation: 0,
            best_fitness: 0.0,
            best_genome: ParameterGenome::from_vec(&mean),
            history: Vec::new(),
        }
    }

    /// Sample a population of candidate genomes using current mean + diagonal covariance.
    pub fn sample_population(&self) -> Vec<ParameterGenome> {
        let dim = ParameterGenome::dim();
        let mut candidates = Vec::with_capacity(self.population_size);

        for i in 0..self.population_size {
            let mut candidate = self.mean.clone();
            for (j, param) in candidate.iter_mut().enumerate() {
                // Deterministic quasi-random perturbation (Halton-inspired)
                let h = halton(i + 1, PRIMES[j % PRIMES.len()]);
                let z = (h - 0.5) * 2.0; // Map [0,1] → [-1,1]
                let std = self.sigma * self.covariance_diagonal[j].sqrt();
                *param = (*param + z * std).clamp(0.0, 1.0);
            }
            let _ = dim; // dim used for size hints
            candidates.push(ParameterGenome::from_vec(&candidate));
        }

        candidates
    }

    /// Update mean and covariance from evaluated population (fitness-weighted recombination).
    pub fn update(&mut self, evaluated: &[(ParameterGenome, f64)]) {
        if evaluated.is_empty() {
            return;
        }

        let mu = (self.population_size / 2).max(1);
        let mut sorted = evaluated.to_vec();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let elite = &sorted[..mu.min(sorted.len())];

        // Update best
        if let Some((genome, fitness)) = sorted.first() {
            if *fitness > self.best_fitness {
                self.best_fitness = *fitness;
                self.best_genome = genome.clone();
                info!(
                    "CMA-ES new best: fitness={:.4} at generation {}",
                    self.best_fitness, self.generation
                );
            }
        }

        // Weighted mean update
        let weights: Vec<f64> = (0..elite.len())
            .map(|i| 1.0 / (i as f64 + 1.0))
            .collect();
        let weight_sum: f64 = weights.iter().sum();

        let dim = ParameterGenome::dim();
        let mut new_mean = vec![0.0f64; dim];
        for (weight, (genome, _)) in weights.iter().zip(elite.iter()) {
            let v = genome.to_vec();
            for (j, val) in v.iter().enumerate() {
                if j < new_mean.len() {
                    new_mean[j] += (weight / weight_sum) * val;
                }
            }
        }
        self.mean = new_mean;

        // Update sigma via success rate heuristic
        let success_rate = elite
            .iter()
            .filter(|(_, f)| *f > self.best_fitness * 0.9)
            .count() as f64
            / elite.len() as f64;

        self.sigma *= if success_rate > 0.2 { 1.1 } else { 0.9 };
        self.sigma = self.sigma.clamp(0.01, 1.0);

        // Update covariance diagonal from variance of elite
        for j in 0..self.covariance_diagonal.len() {
            let mean_j = self.mean.get(j).copied().unwrap_or(0.5);
            let var: f64 = elite
                .iter()
                .map(|(g, _)| {
                    let v = g.to_vec();
                    let x = v.get(j).copied().unwrap_or(0.0);
                    (x - mean_j).powi(2)
                })
                .sum::<f64>()
                / elite.len() as f64;
            self.covariance_diagonal[j] =
                0.9 * self.covariance_diagonal[j] + 0.1 * var.max(1e-6);
        }

        let mean_fitness = elite.iter().map(|(_, f)| f).sum::<f64>() / elite.len() as f64;
        self.history.push(CMAESGeneration {
            generation: self.generation,
            best_fitness: self.best_fitness,
            mean_fitness,
            sigma: self.sigma,
            timestamp: Utc::now(),
        });
        self.generation += 1;
    }

    pub fn best_genome(&self) -> &ParameterGenome {
        &self.best_genome
    }
}

// ── Fitness Loop (connects EvolutionaryOptimizer → real tasks) ────────────────

pub struct GradientFreeLoop {
    pub cmaes: Arc<RwLock<CMAESOptimizer>>,
    pub fitness_fn: FitnessFunction,
    pub replay_buffer: Arc<RwLock<Vec<TaskRecord>>>,
    pub workspace_dir: PathBuf,
    pub generation: Arc<RwLock<u64>>,
}

impl GradientFreeLoop {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self {
            cmaes: Arc::new(RwLock::new(CMAESOptimizer::new(20))),
            fitness_fn: FitnessFunction::default(),
            replay_buffer: Arc::new(RwLock::new(Vec::new())),
            workspace_dir,
            generation: Arc::new(RwLock::new(0)),
        }
    }

    /// Record a completed task result into the replay buffer.
    pub async fn record_task(&self, record: TaskRecord) {
        self.replay_buffer.write().await.push(record);
    }

    /// Run one CMA-ES generation: sample → evaluate on replay buffer → update.
    pub async fn run_generation(&self) -> Result<CMAESGeneration> {
        let candidates = {
            let cmaes = self.cmaes.read().await;
            cmaes.sample_population()
        };

        let mut evaluated: Vec<(ParameterGenome, f64)> = Vec::new();

        let replay = self.replay_buffer.read().await;
        let tasks: Vec<_> = replay.iter().rev().take(50).collect(); // last 50 tasks as eval set

        for candidate in candidates {
            let fitness = self.evaluate_genome_on_tasks(&candidate, &tasks);
            evaluated.push((candidate, fitness));
        }
        drop(replay);

        {
            let mut cmaes = self.cmaes.write().await;
            cmaes.update(&evaluated);
        }

        let cmaes = self.cmaes.read().await;
        let last = cmaes.history.last().cloned().unwrap_or(CMAESGeneration {
            generation: cmaes.generation,
            best_fitness: cmaes.best_fitness,
            mean_fitness: 0.0,
            sigma: cmaes.sigma,
            timestamp: Utc::now(),
        });

        info!(
            "CMA-ES generation {}: best={:.4}, sigma={:.4}",
            last.generation, last.best_fitness, last.sigma
        );

        self.persist_best_genome().await?;

        Ok(last)
    }

    fn evaluate_genome_on_tasks(
        &self,
        genome: &ParameterGenome,
        tasks: &[&TaskRecord],
    ) -> f64 {
        if tasks.is_empty() {
            return 0.0;
        }

        let mut total = 0.0;
        for task in tasks {
            // Use genome parameters to weight the fitness components
            let speed_score =
                1.0 / (1.0 + task.speed_ms as f64 / (10_000.0 * genome.attention_decay.max(0.01)));
            let cost_score =
                1.0 / (1.0 + task.cost_tokens as f64 * genome.learning_rate);
            let fitness = self.fitness_fn.evaluate(
                task.completion_score,
                speed_score,
                cost_score,
                task.novelty * genome.exploration_rate,
                task.alignment_drift * genome.risk_tolerance,
            );
            total += fitness;
        }
        total / tasks.len() as f64
    }

    async fn persist_best_genome(&self) -> Result<()> {
        let dir = self.workspace_dir.join(".housaky").join("evolution");
        tokio::fs::create_dir_all(&dir).await?;
        let best = {
            let cmaes = self.cmaes.read().await;
            cmaes.best_genome().clone()
        };
        let json = serde_json::to_string_pretty(&best)?;
        tokio::fs::write(dir.join("optimal_genome.json"), json).await?;
        info!("Persisted optimal genome to ~/.housaky/evolution/optimal_genome.json");
        Ok(())
    }

    pub async fn load_best_genome(&self) -> Result<Option<ParameterGenome>> {
        let path = self
            .workspace_dir
            .join(".housaky")
            .join("evolution")
            .join("optimal_genome.json");
        if !path.exists() {
            return Ok(None);
        }
        let json = tokio::fs::read_to_string(&path).await?;
        Ok(Some(serde_json::from_str(&json)?))
    }

    pub async fn get_best_genome(&self) -> ParameterGenome {
        self.cmaes.read().await.best_genome().clone()
    }

    pub async fn get_stats(&self) -> CMAESStats {
        let cmaes = self.cmaes.read().await;
        let replay = self.replay_buffer.read().await;
        CMAESStats {
            generation: cmaes.generation,
            best_fitness: cmaes.best_fitness,
            sigma: cmaes.sigma,
            replay_buffer_size: replay.len(),
            history_len: cmaes.history.len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CMAESStats {
    pub generation: u64,
    pub best_fitness: f64,
    pub sigma: f64,
    pub replay_buffer_size: usize,
    pub history_len: usize,
}

// ── Math helpers ─────────────────────────────────────────────────────────────

const PRIMES: [usize; 16] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53];

fn halton(index: usize, base: usize) -> f64 {
    let mut f = 1.0_f64;
    let mut r = 0.0_f64;
    let mut i = index;
    while i > 0 {
        f /= base as f64;
        r += f * (i % base) as f64;
        i /= base;
    }
    r
}

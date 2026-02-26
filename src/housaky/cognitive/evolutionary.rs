//! Evolutionary Self-Improvement (Population-Based)
//!
//! Explores multiple improvement paths simultaneously:
//! - Genome: a set of tunable parameters (reasoning weights, strategy preferences)
//! - Population: N genomes evaluated in parallel
//! - Fitness: automated benchmarks (task completion, speed, accuracy)
//! - Crossover + Mutation for generating new configurations
//! - Safe: only modifies in-memory parameters, not source code

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genome {
    pub id: String,
    pub parameters: HashMap<String, f64>,
    pub fitness: Option<f64>,
    pub lineage: Vec<String>,
    pub generation: u64,
    pub created_at: DateTime<Utc>,
}

impl Genome {
    pub fn random(param_names: &[&str]) -> Self {
        let mut parameters = HashMap::new();
        // Use a simple deterministic seeding based on param index
        for (i, name) in param_names.iter().enumerate() {
            let value = ((i as f64 * 0.618033988749895) % 1.0).abs();
            parameters.insert(name.to_string(), value);
        }
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            parameters,
            fitness: None,
            lineage: Vec::new(),
            generation: 0,
            created_at: Utc::now(),
        }
    }

    pub fn crossover(&self, other: &Genome) -> Genome {
        let mut child_params = HashMap::new();
        for (key, val_a) in &self.parameters {
            let val_b = other.parameters.get(key).copied().unwrap_or(*val_a);
            // Uniform crossover: take average with slight bias toward fitter parent
            let bias = match (self.fitness, other.fitness) {
                (Some(fa), Some(fb)) if fa + fb > 0.0 => fa / (fa + fb),
                _ => 0.5,
            };
            child_params.insert(key.clone(), val_a * bias + val_b * (1.0 - bias));
        }

        Genome {
            id: uuid::Uuid::new_v4().to_string(),
            parameters: child_params,
            fitness: None,
            lineage: vec![self.id.clone(), other.id.clone()],
            generation: self.generation.max(other.generation) + 1,
            created_at: Utc::now(),
        }
    }

    pub fn mutate(&mut self, mutation_rate: f64, mutation_strength: f64) {
        let keys: Vec<String> = self.parameters.keys().cloned().collect();
        for (i, key) in keys.iter().enumerate() {
            // Deterministic pseudo-random based on generation + index
            let pseudo_random = ((self.generation as f64 * 0.7 + i as f64 * 0.3) * 0.618033988749895) % 1.0;
            if pseudo_random < mutation_rate {
                if let Some(val) = self.parameters.get_mut(key) {
                    let perturbation = (pseudo_random - 0.5) * 2.0 * mutation_strength;
                    *val = (*val + perturbation).clamp(0.0, 1.0);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    pub population_size: usize,
    pub elite_count: usize,
    pub mutation_rate: f64,
    pub mutation_strength: f64,
    pub crossover_rate: f64,
    pub max_generations: u64,
    pub target_fitness: f64,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            population_size: 20,
            elite_count: 4,
            mutation_rate: 0.1,
            mutation_strength: 0.15,
            crossover_rate: 0.7,
            max_generations: 100,
            target_fitness: 0.95,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationReport {
    pub generation: u64,
    pub best_fitness: f64,
    pub avg_fitness: f64,
    pub worst_fitness: f64,
    pub best_genome_id: String,
    pub population_diversity: f64,
    pub timestamp: DateTime<Utc>,
}

pub struct EvolutionaryOptimizer {
    pub population: Arc<RwLock<Vec<Genome>>>,
    pub generation: Arc<RwLock<u64>>,
    pub config: EvolutionConfig,
    pub history: Arc<RwLock<Vec<GenerationReport>>>,
    pub best_ever: Arc<RwLock<Option<Genome>>>,
    pub parameter_names: Vec<String>,
}

impl EvolutionaryOptimizer {
    pub fn new(param_names: Vec<String>, config: EvolutionConfig) -> Self {
        Self {
            population: Arc::new(RwLock::new(Vec::new())),
            generation: Arc::new(RwLock::new(0)),
            config,
            history: Arc::new(RwLock::new(Vec::new())),
            best_ever: Arc::new(RwLock::new(None)),
            parameter_names: param_names,
        }
    }

    /// Initialize population with random genomes.
    pub async fn initialize(&self) {
        let param_refs: Vec<&str> = self.parameter_names.iter().map(|s| s.as_str()).collect();
        let mut pop = self.population.write().await;
        pop.clear();
        for _ in 0..self.config.population_size {
            let mut genome = Genome::random(&param_refs);
            genome.mutate(1.0, 0.5); // Diversify initial population
            pop.push(genome);
        }
        info!("Initialized evolutionary population with {} genomes", pop.len());
    }

    /// Evaluate fitness for all genomes using the provided fitness function.
    pub async fn evaluate<F>(&self, fitness_fn: F)
    where
        F: Fn(&HashMap<String, f64>) -> f64,
    {
        let mut pop = self.population.write().await;
        for genome in pop.iter_mut() {
            genome.fitness = Some(fitness_fn(&genome.parameters));
        }
    }

    /// Evolve one generation: select, crossover, mutate.
    pub async fn evolve_generation(&self) -> GenerationReport {
        let mut pop = self.population.write().await;

        // Sort by fitness (descending)
        pop.sort_by(|a, b| {
            b.fitness
                .unwrap_or(0.0)
                .partial_cmp(&a.fitness.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let best_fitness = pop.first().and_then(|g| g.fitness).unwrap_or(0.0);
        let worst_fitness = pop.last().and_then(|g| g.fitness).unwrap_or(0.0);
        let avg_fitness: f64 = pop.iter().map(|g| g.fitness.unwrap_or(0.0)).sum::<f64>()
            / pop.len().max(1) as f64;
        let best_id = pop.first().map(|g| g.id.clone()).unwrap_or_default();

        // Update best ever
        if let Some(best) = pop.first() {
            let mut best_ever = self.best_ever.write().await;
            let should_update = best_ever
                .as_ref()
                .map(|be| best.fitness.unwrap_or(0.0) > be.fitness.unwrap_or(0.0))
                .unwrap_or(true);
            if should_update {
                *best_ever = Some(best.clone());
            }
        }

        // Compute diversity (average pairwise parameter distance)
        let diversity = self.compute_diversity(&pop);

        // Keep elites
        let elites: Vec<Genome> = pop.iter().take(self.config.elite_count).cloned().collect();

        // Generate new population
        let mut new_pop = elites.clone();

        while new_pop.len() < self.config.population_size {
            // Tournament selection
            let parent_a = self.tournament_select(&pop, 3);
            let parent_b = self.tournament_select(&pop, 3);

            let gen = *self.generation.read().await;
            let should_crossover = ((new_pop.len() as f64 * 0.618) % 1.0) < self.config.crossover_rate;

            let mut child = if should_crossover {
                parent_a.crossover(&parent_b)
            } else {
                let mut c = parent_a.clone();
                c.id = uuid::Uuid::new_v4().to_string();
                c.generation = gen + 1;
                c
            };

            child.mutate(self.config.mutation_rate, self.config.mutation_strength);
            new_pop.push(child);
        }

        *pop = new_pop;
        drop(pop);

        // Increment generation
        let mut gen = self.generation.write().await;
        *gen += 1;

        let report = GenerationReport {
            generation: *gen,
            best_fitness,
            avg_fitness,
            worst_fitness,
            best_genome_id: best_id,
            population_diversity: diversity,
            timestamp: Utc::now(),
        };

        self.history.write().await.push(report.clone());

        info!(
            "Generation {}: best={:.4}, avg={:.4}, diversity={:.4}",
            report.generation, best_fitness, avg_fitness, diversity
        );

        report
    }

    /// Tournament selection: pick the best of K random individuals.
    fn tournament_select(&self, pop: &[Genome], k: usize) -> Genome {
        let k = k.min(pop.len());
        let mut best: Option<&Genome> = None;

        // Select K individuals (using deterministic spacing)
        for i in 0..k {
            let idx = (i * pop.len() / k) % pop.len();
            let candidate = &pop[idx];
            if best
                .map(|b| candidate.fitness.unwrap_or(0.0) > b.fitness.unwrap_or(0.0))
                .unwrap_or(true)
            {
                best = Some(candidate);
            }
        }

        best.cloned().unwrap_or_else(|| pop[0].clone())
    }

    /// Compute population diversity as average parameter standard deviation.
    fn compute_diversity(&self, pop: &[Genome]) -> f64 {
        if pop.len() < 2 {
            return 0.0;
        }

        let mut total_std = 0.0;
        let mut param_count = 0;

        for param_name in &self.parameter_names {
            let values: Vec<f64> = pop
                .iter()
                .filter_map(|g| g.parameters.get(param_name))
                .copied()
                .collect();

            if values.len() < 2 {
                continue;
            }

            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let variance: f64 =
                values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
            total_std += variance.sqrt();
            param_count += 1;
        }

        if param_count > 0 {
            total_std / param_count as f64
        } else {
            0.0
        }
    }

    /// Get the best genome found so far.
    pub async fn get_best(&self) -> Option<Genome> {
        self.best_ever.read().await.clone()
    }

    /// Record a completed task result into a `GradientFreeLoop` replay buffer.
    ///
    /// This bridges the evolutionary parameter optimizer with the CMA-ES
    /// gradient-free loop: every time a task completes, its outcome is pushed
    /// into the replay buffer so the next CMA-ES generation has fresh data.
    pub async fn record_task_to_cmaes_loop(
        &self,
        cmaes_loop: &crate::housaky::learning::GradientFreeLoop,
        description: &str,
        completion_score: f64,
        speed_ms: u64,
        cost_tokens: u64,
        novelty: f64,
        alignment_drift: f64,
    ) {
        let best = self.get_best().await;
        let genome_id = best.map(|g| g.id);

        let record = crate::housaky::learning::TaskRecord {
            id: uuid::Uuid::new_v4().to_string(),
            description: description.to_string(),
            completion_score,
            speed_ms,
            cost_tokens,
            novelty,
            alignment_drift,
            genome_id,
            timestamp: Utc::now(),
        };

        cmaes_loop.record_task(record).await;
    }

    /// Run one CMA-ES generation using accumulated task replay data, then
    /// back-apply the best genome's parameters to this evolutionary optimizer's
    /// current population.
    pub async fn sync_with_cmaes_loop(
        &self,
        cmaes_loop: &crate::housaky::learning::GradientFreeLoop,
    ) -> anyhow::Result<()> {
        let _gen_report = cmaes_loop.run_generation().await?;
        let best = cmaes_loop.get_best_genome().await;

        // Translate CMA-ES ParameterGenome back into Genome parameters
        let translated_params = {
            let mut map = std::collections::HashMap::new();
            map.insert("cot_prior".to_string(), best.reasoning_weights.cot_prior);
            map.insert("react_prior".to_string(), best.reasoning_weights.react_prior);
            map.insert("tot_prior".to_string(), best.reasoning_weights.tot_prior);
            map.insert("reflexion_prior".to_string(), best.reasoning_weights.reflexion_prior);
            map.insert("self_consistency_prior".to_string(), best.reasoning_weights.self_consistency_prior);
            map.insert("attention_decay".to_string(), best.attention_decay);
            map.insert("learning_rate".to_string(), best.learning_rate);
            map.insert("exploration_rate".to_string(), best.exploration_rate);
            map.insert("confidence_threshold".to_string(), best.confidence_threshold);
            map.insert("risk_tolerance".to_string(), best.risk_tolerance);
            map
        };

        // Inject best CMA-ES parameters into the top elite genome
        let mut pop = self.population.write().await;
        if let Some(elite) = pop.first_mut() {
            for (key, value) in &translated_params {
                if self.parameter_names.contains(key) {
                    elite.parameters.insert(key.clone(), *value);
                }
            }
            elite.fitness = Some(cmaes_loop.cmaes.read().await.best_fitness);
        }

        tracing::info!(
            "Synced CMA-ES best genome (fitness={:.4}) into evolutionary optimizer",
            cmaes_loop.cmaes.read().await.best_fitness
        );
        Ok(())
    }

    /// Check if evolution has converged.
    pub async fn has_converged(&self) -> bool {
        let best = self.best_ever.read().await;
        best.as_ref()
            .and_then(|g| g.fitness)
            .map(|f| f >= self.config.target_fitness)
            .unwrap_or(false)
    }

    pub async fn get_stats(&self) -> EvolutionStats {
        let gen = *self.generation.read().await;
        let pop = self.population.read().await;
        let best = self.best_ever.read().await;

        EvolutionStats {
            current_generation: gen,
            population_size: pop.len(),
            best_fitness_ever: best.as_ref().and_then(|g| g.fitness).unwrap_or(0.0),
            current_best_fitness: pop.first().and_then(|g| g.fitness).unwrap_or(0.0),
            total_genomes_evaluated: gen * self.config.population_size as u64,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionStats {
    pub current_generation: u64,
    pub population_size: usize,
    pub best_fitness_ever: f64,
    pub current_best_fitness: f64,
    pub total_genomes_evaluated: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_evolution_basic() {
        let params = vec!["weight_a".to_string(), "weight_b".to_string(), "threshold".to_string()];
        let config = EvolutionConfig {
            population_size: 10,
            elite_count: 2,
            max_generations: 5,
            ..Default::default()
        };

        let optimizer = EvolutionaryOptimizer::new(params, config);
        optimizer.initialize().await;

        // Simple fitness function: maximize sum of parameters
        let fitness_fn = |params: &HashMap<String, f64>| -> f64 {
            params.values().sum::<f64>() / params.len() as f64
        };

        for _ in 0..5 {
            optimizer.evaluate(fitness_fn).await;
            let report = optimizer.evolve_generation().await;
            assert!(report.best_fitness >= 0.0);
        }

        let best = optimizer.get_best().await;
        assert!(best.is_some());
    }

    #[test]
    fn test_genome_crossover() {
        let mut a = Genome::random(&["x", "y"]);
        a.fitness = Some(0.8);
        let mut b = Genome::random(&["x", "y"]);
        b.fitness = Some(0.2);
        let child = a.crossover(&b);
        assert!(!child.lineage.is_empty());
    }
}

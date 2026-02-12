//! Selection strategies for evolution

use anyhow::Result;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::fitness::FitnessScore;
use crate::mutation::Mutation;

/// Selection strategy types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SelectionStrategy {
    /// Tournament selection
    Tournament { size: usize },
    /// Roulette wheel selection
    RouletteWheel,
    /// Rank-based selection
    RankBased,
    /// Elitist selection (keep best N)
    Elitist { count: usize },
}

/// Selector for choosing mutations
pub struct Selector {
    strategy: SelectionStrategy,
}

impl Selector {
    /// Create a new selector
    pub fn new(strategy: SelectionStrategy) -> Self {
        Self { strategy }
    }

    /// Select mutations from a population
    pub fn select(
        &self,
        mutations: &[Mutation],
        scores: &[FitnessScore],
        count: usize,
    ) -> Vec<(Mutation, FitnessScore)> {
        match self.strategy {
            SelectionStrategy::Tournament { size } => {
                self.tournament_select(mutations, scores, count, size)
            }
            SelectionStrategy::RouletteWheel => self.roulette_select(mutations, scores, count),
            SelectionStrategy::RankBased => self.rank_select(mutations, scores, count),
            SelectionStrategy::Elitist { count: n } => {
                self.elitist_select(mutations, scores, n.min(count))
            }
        }
    }

    /// Tournament selection
    fn tournament_select(
        &self,
        mutations: &[Mutation],
        scores: &[FitnessScore],
        count: usize,
        tournament_size: usize,
    ) -> Vec<(Mutation, FitnessScore)> {
        let mut selected = Vec::with_capacity(count);
        let mut rng = rand::thread_rng();

        for _ in 0..count {
            // Select random participants
            let mut best_idx = 0;
            let mut best_score = 0.0;

            for _ in 0..tournament_size {
                let idx = rand::random::<usize>() % mutations.len();
                if scores[idx].score > best_score {
                    best_score = scores[idx].score;
                    best_idx = idx;
                }
            }

            selected.push((mutations[best_idx].clone(), scores[best_idx].clone()));
        }

        selected
    }

    /// Roulette wheel selection
    fn roulette_select(
        &self,
        mutations: &[Mutation],
        scores: &[FitnessScore],
        count: usize,
    ) -> Vec<(Mutation, FitnessScore)> {
        use rand::distributions::{Distribution, WeightedIndex};

        let total_fitness: f64 = scores.iter().map(|s| s.score).sum();
        if total_fitness == 0.0 {
            // Random selection if no fitness
            return self.random_select(mutations, scores, count);
        }

        let weights: Vec<f64> = scores.iter().map(|s| s.score / total_fitness).collect();
        let dist = WeightedIndex::new(&weights).unwrap();
        let mut rng = rand::thread_rng();

        let mut selected = Vec::with_capacity(count);
        for _ in 0..count {
            let idx = dist.sample(&mut rng);
            selected.push((mutations[idx].clone(), scores[idx].clone()));
        }

        selected
    }

    /// Rank-based selection
    fn rank_select(
        &self,
        mutations: &[Mutation],
        scores: &[FitnessScore],
        count: usize,
    ) -> Vec<(Mutation, FitnessScore)> {
        // Sort by fitness
        let mut indexed_scores: Vec<(usize, f64)> = scores
            .iter()
            .enumerate()
            .map(|(i, s)| (i, s.score))
            .collect();

        indexed_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Assign ranks (higher fitness = higher rank)
        let n = indexed_scores.len() as f64;
        let mut ranks = vec![0.0; mutations.len()];
        for (rank, (idx, _)) in indexed_scores.iter().enumerate() {
            ranks[*idx] = n - rank as f64;
        }

        // Use ranks as weights
        use rand::distributions::{Distribution, WeightedIndex};
        let total_rank: f64 = ranks.iter().sum();
        let weights: Vec<f64> = ranks.iter().map(|r| r / total_rank).collect();
        let dist = WeightedIndex::new(&weights).unwrap();
        let mut rng = rand::thread_rng();

        let mut selected = Vec::with_capacity(count);
        for _ in 0..count {
            let idx = dist.sample(&mut rng);
            selected.push((mutations[idx].clone(), scores[idx].clone()));
        }

        selected
    }

    /// Elitist selection (keep only the best)
    fn elitist_select(
        &self,
        mutations: &[Mutation],
        scores: &[FitnessScore],
        count: usize,
    ) -> Vec<(Mutation, FitnessScore)> {
        let mut indexed: Vec<(usize, &Mutation, &FitnessScore)> = mutations
            .iter()
            .zip(scores.iter())
            .enumerate()
            .map(|(i, (m, s))| (i, m, s))
            .collect();

        indexed.sort_by(|a, b| b.2.score.partial_cmp(&a.2.score).unwrap());

        indexed
            .into_iter()
            .take(count)
            .map(|(_, m, s)| (m.clone(), s.clone()))
            .collect()
    }

    /// Random selection (fallback)
    fn random_select(
        &self,
        mutations: &[Mutation],
        scores: &[FitnessScore],
        count: usize,
    ) -> Vec<(Mutation, FitnessScore)> {
        let mut rng = rand::thread_rng();
        let mut indices: Vec<usize> = (0..mutations.len()).collect();
        indices.shuffle(&mut rng);

        indices
            .into_iter()
            .take(count)
            .map(|i| (mutations[i].clone(), scores[i].clone()))
            .collect()
    }
}

/// Population statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopulationStats {
    /// Average fitness
    pub avg_fitness: f64,
    /// Maximum fitness
    pub max_fitness: f64,
    /// Minimum fitness
    pub min_fitness: f64,
    /// Standard deviation
    pub std_dev: f64,
    /// Number of individuals
    pub population_size: usize,
}

impl PopulationStats {
    /// Calculate statistics from scores
    pub fn from_scores(scores: &[FitnessScore]) -> Self {
        if scores.is_empty() {
            return Self {
                avg_fitness: 0.0,
                max_fitness: 0.0,
                min_fitness: 0.0,
                std_dev: 0.0,
                population_size: 0,
            };
        }

        let fitnesses: Vec<f64> = scores.iter().map(|s| s.score).collect();
        let sum: f64 = fitnesses.iter().sum();
        let avg = sum / fitnesses.len() as f64;
        let max = fitnesses.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min = fitnesses.iter().cloned().fold(f64::INFINITY, f64::min);

        let variance: f64 =
            fitnesses.iter().map(|&f| (f - avg).powi(2)).sum::<f64>() / fitnesses.len() as f64;
        let std_dev = variance.sqrt();

        Self {
            avg_fitness: avg,
            max_fitness: max,
            min_fitness: min,
            std_dev,
            population_size: scores.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_population() -> (Vec<Mutation>, Vec<FitnessScore>) {
        let mutations: Vec<Mutation> = (0..10)
            .map(|i| {
                Mutation::new(
                    crate::mutation::MutationType::AddFunction,
                    "test.rs",
                    format!("mutation {}", i),
                    "fn test() {}",
                )
            })
            .collect();

        let scores: Vec<FitnessScore> = (0..10)
            .map(|i| FitnessScore {
                score: i as f64 / 10.0,
                test_pass_rate: i as f64 / 10.0,
                code_coverage: i as f64 / 10.0,
                performance_score: i as f64 / 10.0,
                complexity_score: i as f64 / 10.0,
                build_succeeded: true,
                tests_passed: i >= 5,
                error: None,
            })
            .collect();

        (mutations, scores)
    }

    #[test]
    fn test_tournament_selection() {
        let (mutations, scores) = create_test_population();
        let selector = Selector::new(SelectionStrategy::Tournament { size: 3 });

        let selected = selector.select(&mutations, &scores, 5);
        assert_eq!(selected.len(), 5);
    }

    #[test]
    fn test_elitist_selection() {
        let (mutations, scores) = create_test_population();
        let selector = Selector::new(SelectionStrategy::Elitist { count: 3 });

        let selected = selector.select(&mutations, &scores, 3);
        assert_eq!(selected.len(), 3);
        assert_eq!(selected[0].1.score, 0.9); // Best should be first
    }

    #[test]
    fn test_population_stats() {
        let (_, scores) = create_test_population();
        let stats = PopulationStats::from_scores(&scores);

        assert!(stats.avg_fitness > 0.0);
        assert_eq!(stats.max_fitness, 0.9);
        assert_eq!(stats.min_fitness, 0.0);
    }
}

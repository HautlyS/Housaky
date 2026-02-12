//! Genetic algorithm implementation
use anyhow::Result;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Genetic algorithm configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneticConfig {
    pub population_size: usize,
    pub mutation_rate: f64,
    pub crossover_rate: f64,
    pub elitism_count: usize,
    pub generations: usize,
}

impl Default for GeneticConfig {
    fn default() -> Self {
        Self {
            population_size: 100,
            mutation_rate: 0.01,
            crossover_rate: 0.8,
            elitism_count: 5,
            generations: 100,
        }
    }
}

/// Individual in population
pub trait Individual: Clone + Send {
    fn fitness(&self) -> f64;
    fn mutate(&mut self);
    fn crossover(&self, other: &Self) -> (Self, Self);
}

/// Genetic algorithm
pub struct GeneticAlgorithm<T: Individual> {
    config: GeneticConfig,
    population: Vec<T>,
    generation: usize,
}

impl<T: Individual> GeneticAlgorithm<T> {
    pub fn new(config: GeneticConfig, initial_population: Vec<T>) -> Self {
        Self {
            config,
            population: initial_population,
            generation: 0,
        }
    }

    pub fn evolve(&mut self) {
        // Evaluate fitness
        let mut scored: Vec<_> = self
            .population
            .iter()
            .map(|ind| (ind.clone(), ind.fitness()))
            .collect();

        // Sort by fitness
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Elitism
        let mut new_population: Vec<T> = scored[..self.config.elitism_count]
            .iter()
            .map(|(ind, _)| ind.clone())
            .collect();

        // Generate offspring
        while new_population.len() < self.config.population_size {
            let parent1 = self.select_parent(&scored);
            let parent2 = self.select_parent(&scored);

            let (mut child1, mut child2) = if rand::random::<f64>() < self.config.crossover_rate {
                parent1.crossover(&parent2)
            } else {
                (parent1.clone(), parent2.clone())
            };

            // Mutate
            if rand::random::<f64>() < self.config.mutation_rate {
                child1.mutate();
            }
            if rand::random::<f64>() < self.config.mutation_rate {
                child2.mutate();
            }

            new_population.push(child1);
            if new_population.len() < self.config.population_size {
                new_population.push(child2);
            }
        }

        self.population = new_population;
        self.generation += 1;
    }

    fn select_parent(&self, scored: &[(T, f64)]) -> &T {
        // Tournament selection
        let mut best_idx = rand::random::<usize>() % scored.len();
        let mut best_fitness = scored[best_idx].1;

        for _ in 0..3 {
            let idx = rand::random::<usize>() % scored.len();
            if scored[idx].1 > best_fitness {
                best_idx = idx;
                best_fitness = scored[idx].1;
            }
        }

        &scored[best_idx].0
    }

    pub fn best(&self) -> Option<&T> {
        self.population
            .iter()
            .max_by(|a, b| a.fitness().partial_cmp(&b.fitness()).unwrap())
    }
}

/// Simple individual for testing
#[derive(Debug, Clone)]
pub struct SimpleIndividual {
    pub genes: Vec<f64>,
}

impl Individual for SimpleIndividual {
    fn fitness(&self) -> f64 {
        // Maximize sum of squares
        self.genes.iter().map(|g| g * g).sum()
    }

    fn mutate(&mut self) {
        let idx = rand::random::<usize>() % self.genes.len();
        self.genes[idx] += rand::random::<f64>() * 0.1 - 0.05;
    }

    fn crossover(&self, other: &Self) -> (Self, Self) {
        let point = rand::random::<usize>() % self.genes.len();

        let mut child1_genes = self.genes[..point].to_vec();
        child1_genes.extend_from_slice(&other.genes[point..]);

        let mut child2_genes = other.genes[..point].to_vec();
        child2_genes.extend_from_slice(&self.genes[point..]);

        (
            Self {
                genes: child1_genes,
            },
            Self {
                genes: child2_genes,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genetic_algorithm() {
        let config = GeneticConfig {
            population_size: 20,
            generations: 10,
            ..Default::default()
        };

        let population: Vec<SimpleIndividual> = (0..20)
            .map(|_| SimpleIndividual {
                genes: vec![rand::random::<f64>(); 5],
            })
            .collect();

        let mut ga = GeneticAlgorithm::new(config, population);

        for _ in 0..10 {
            ga.evolve();
        }

        assert!(ga.best().is_some());
    }
}

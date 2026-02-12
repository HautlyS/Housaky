//! Hyperparameter optimization

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Hyperparameter configuration space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperparameterSpace {
    /// Learning rate range
    pub learning_rate: (f64, f64),
    /// Batch size options
    pub batch_sizes: Vec<usize>,
    /// Optimizer choices
    pub optimizers: Vec<String>,
    /// Dropout rate range
    pub dropout: (f64, f64),
    /// Weight decay range
    pub weight_decay: (f64, f64),
}

impl Default for HyperparameterSpace {
    fn default() -> Self {
        Self {
            learning_rate: (1e-5, 1e-1),
            batch_sizes: vec![8, 16, 32, 64, 128],
            optimizers: vec!["adam".to_string(), "sgd".to_string(), "adamw".to_string()],
            dropout: (0.0, 0.5),
            weight_decay: (0.0, 0.1),
        }
    }
}

/// A hyperparameter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hyperparameters {
    /// Learning rate
    pub learning_rate: f64,
    /// Batch size
    pub batch_size: usize,
    /// Optimizer name
    pub optimizer: String,
    /// Dropout rate
    pub dropout: f64,
    /// Weight decay
    pub weight_decay: f64,
    /// Custom parameters
    pub custom: HashMap<String, f64>,
}

impl Default for Hyperparameters {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            batch_size: 32,
            optimizer: "adam".to_string(),
            dropout: 0.1,
            weight_decay: 0.0001,
            custom: HashMap::new(),
        }
    }
}

impl Hyperparameters {
    /// Sample random hyperparameters from space
    pub fn random(space: &HyperparameterSpace) -> Self {
        let lr_exp = rand::random::<f64>()
            * (space.learning_rate.1.ln() - space.learning_rate.0.ln())
            + space.learning_rate.0.ln();
        let learning_rate = lr_exp.exp();

        let batch_size = space.batch_sizes[rand::random::<usize>() % space.batch_sizes.len()];

        let optimizer = space.optimizers[rand::random::<usize>() % space.optimizers.len()].clone();

        let dropout = rand::random::<f64>() * (space.dropout.1 - space.dropout.0) + space.dropout.0;

        let weight_decay = rand::random::<f64>() * (space.weight_decay.1 - space.weight_decay.0)
            + space.weight_decay.0;

        Self {
            learning_rate,
            batch_size,
            optimizer,
            dropout,
            weight_decay,
            custom: HashMap::new(),
        }
    }

    /// Get custom parameter
    pub fn get(&self, key: &str) -> Option<f64> {
        self.custom.get(key).copied()
    }

    /// Set custom parameter
    pub fn set(&mut self, key: impl Into<String>, value: f64) {
        self.custom.insert(key.into(), value);
    }
}

/// Bayesian optimization for hyperparameters
pub struct BayesianOptimizer {
    space: HyperparameterSpace,
    observations: Vec<(Hyperparameters, f64)>,
}

impl BayesianOptimizer {
    /// Create a new optimizer
    pub fn new(space: HyperparameterSpace) -> Self {
        Self {
            space,
            observations: Vec::new(),
        }
    }

    /// Add an observation
    pub fn observe(&mut self, params: Hyperparameters, performance: f64) {
        self.observations.push((params, performance));
    }

    /// Suggest next hyperparameters to try
    pub fn suggest(&self) -> Hyperparameters {
        if self.observations.is_empty() {
            return Hyperparameters::random(&self.space);
        }

        // Simple strategy: find promising region and sample nearby
        let best = self
            .observations
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();

        // Perturb best parameters
        let mut suggestion = best.0.clone();
        suggestion.learning_rate *= 1.0 + (rand::random::<f64>() - 0.5) * 0.2;
        suggestion.dropout =
            (suggestion.dropout + (rand::random::<f64>() - 0.5) * 0.1).clamp(0.0, 1.0);

        suggestion
    }

    /// Get best observed hyperparameters
    pub fn best(&self) -> Option<&Hyperparameters> {
        self.observations
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(p, _)| p)
    }
}

/// Grid search hyperparameter optimization
pub struct GridSearch {
    space: HyperparameterSpace,
    grid_points: usize,
}

impl GridSearch {
    /// Create a new grid search
    pub fn new(space: HyperparameterSpace, grid_points: usize) -> Self {
        Self { space, grid_points }
    }

    /// Generate all combinations
    pub fn generate_grid(&self) -> Vec<Hyperparameters> {
        let mut grid = Vec::new();

        for lr_idx in 0..self.grid_points {
            let lr = self.space.learning_rate.0
                * (self.space.learning_rate.1 / self.space.learning_rate.0)
                    .powf(lr_idx as f64 / (self.grid_points - 1) as f64);

            for &batch_size in &self.space.batch_sizes {
                for optimizer in &self.space.optimizers {
                    grid.push(Hyperparameters {
                        learning_rate: lr,
                        batch_size,
                        optimizer: optimizer.clone(),
                        dropout: 0.1,
                        weight_decay: 0.0001,
                        custom: HashMap::new(),
                    });
                }
            }
        }

        grid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hyperparameter_space() {
        let space = HyperparameterSpace::default();
        assert!(!space.batch_sizes.is_empty());
    }

    #[test]
    fn test_random_hyperparameters() {
        let space = HyperparameterSpace::default();
        let params = Hyperparameters::random(&space);

        assert!(params.learning_rate > 0.0);
        assert!(params.dropout >= 0.0 && params.dropout <= 1.0);
    }

    #[test]
    fn test_bayesian_optimizer() {
        let space = HyperparameterSpace::default();
        let mut optimizer = BayesianOptimizer::new(space);

        let params1 = Hyperparameters::default();
        optimizer.observe(params1, 0.8);

        let params2 = Hyperparameters::random(&HyperparameterSpace::default());
        optimizer.observe(params2, 0.9);

        let best = optimizer.best();
        assert!(best.is_some());

        let suggestion = optimizer.suggest();
        assert!(suggestion.learning_rate > 0.0);
    }
}

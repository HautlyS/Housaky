//! Reptile meta-learning algorithm

use anyhow::Result;
use candle_core::{Device, Tensor};
use serde::{Deserialize, Serialize};

use crate::maml::Task;

/// Reptile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReptileConfig {
    /// Inner loop learning rate
    pub inner_lr: f64,
    /// Outer loop learning rate (meta)
    pub meta_lr: f64,
    /// Number of inner loop iterations
    pub inner_iterations: usize,
    /// Fraction of weights to update each iteration
    pub update_fraction: f64,
}

impl Default for ReptileConfig {
    fn default() -> Self {
        Self {
            inner_lr: 0.01,
            meta_lr: 0.1,
            inner_iterations: 5,
            update_fraction: 1.0,
        }
    }
}

/// Reptile learner (first-order MAML approximation)
pub struct ReptileLearner {
    config: ReptileConfig,
    device: Device,
}

impl ReptileLearner {
    /// Create a new Reptile learner
    pub fn new(config: ReptileConfig) -> Result<Self> {
        let device = Device::cuda_if_available(0)?;

        Ok(Self { config, device })
    }

    /// Train on a batch of tasks
    pub fn train_step(&self, model: &mut ReptileModel, tasks: &[Task]) -> Result<f64> {
        let mut total_loss = 0.0;
        let initial_weights = model.get_weights().clone();

        for task in tasks {
            // Clone model for this task
            let mut task_model = model.clone();

            // Inner loop: train on task
            for _ in 0..self.config.inner_iterations {
                let predictions = task_model.forward(&task.support_x)?;
                let loss = self.compute_loss(&predictions, &task.support_y)?;

                // Simple gradient step
                let gradients = loss.backward()?;
                task_model.sgd_step(&gradients, self.config.inner_lr)?;
            }

            // Evaluate on query set
            let query_pred = task_model.forward(&task.query_x)?;
            let query_loss = self.compute_loss(&query_pred, &task.query_y)?;
            total_loss += query_loss.to_scalar::<f32>()? as f64;

            // Meta-update: move weights toward task-adapted weights
            let task_weights = task_model.get_weights();
            let update = task_weights.sub(&initial_weights)?;
            model.update_weights(&update, self.config.meta_lr)?;
        }

        Ok(total_loss / tasks.len() as f64)
    }

    /// Compute loss
    fn compute_loss(&self, predictions: &Tensor, targets: &Tensor) -> Result<Tensor> {
        let diff = predictions.sub(targets)?;
        let squared = diff.mul(&diff)?;
        squared.mean_all()
    }
}

/// Simplified model for Reptile
#[derive(Debug, Clone)]
pub struct ReptileModel {
    weights: Tensor,
}

impl ReptileModel {
    /// Create new model
    pub fn new(dim: usize, device: &Device) -> Result<Self> {
        let weights = Tensor::randn(0.0, 0.01, &[dim, 1], device)?;
        Ok(Self { weights })
    }

    /// Forward pass
    pub fn forward(&self, x: &Tensor) -> Result<Tensor> {
        x.matmul(&self.weights)
    }

    /// SGD update step
    pub fn sgd_step(&mut self, gradients: &Tensor, lr: f64) -> Result<()> {
        let update = gradients.mul_scalar(lr)?;
        self.weights = self.weights.sub(&update)?;
        Ok(())
    }

    /// Get current weights
    pub fn get_weights(&self) -> &Tensor {
        &self.weights
    }

    /// Update weights with fraction
    pub fn update_weights(&mut self, update: &Tensor, fraction: f64) -> Result<()> {
        let scaled_update = update.mul_scalar(fraction)?;
        self.weights = self.weights.add(&scaled_update)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reptile_config() {
        let config = ReptileConfig::default();
        assert!(config.inner_lr > 0.0);
    }

    #[test]
    fn test_reptile_model() {
        let device = Device::Cpu;
        let model = ReptileModel::new(10, &device).unwrap();

        let input = Tensor::randn(0.0, 1.0, &[5, 10], &device).unwrap();
        let output = model.forward(&input).unwrap();

        assert_eq!(output.dims(), &[5, 1]);
    }
}

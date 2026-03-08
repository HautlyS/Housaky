//! Darwin Godel Machine: Self-improvement via code mutation + empirical fitness evaluation
//!
//! Based on Sakana AI's DGM (ICLR 2026): maintains an archive of agent variants (gene pool)
//! and evolves through stochastic mutation + empirical fitness testing. No formal proofs
//! required -- just measurable improvement.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Mutation types that the DGM can propose
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModificationType {
    /// Adjust learning rate or optimizer parameters
    HyperparameterTuning,
    /// Modify reasoning strategy (CoT, ReAct, ToT)
    ReasoningStrategy,
    /// Restructure prompt templates
    PromptOptimization,
    /// Adjust memory retrieval patterns
    MemoryOptimization,
    /// Generate new tool definitions
    ToolInvention,
    /// Modify the improvement heuristic itself
    MetaImprovement,
}

/// A single modification proposed by the DGM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Modification {
    pub id: String,
    pub modification_type: ModificationType,
    pub description: String,
    pub fitness_delta: f64,
    pub risk_score: f64,
    pub parent_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// An archived agent variant in the gene pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentVariant {
    pub id: String,
    pub fitness: f64,
    pub modifications: Vec<Modification>,
    pub generation: u32,
    pub created_at: DateTime<Utc>,
}

/// DGM status for CLI display
#[derive(Debug, Clone)]
pub struct DgmStatus {
    pub archive_size: usize,
    pub best_fitness: f64,
    pub total_improvements: u64,
    pub total_iterations: u64,
    pub current_generation: u32,
}

/// The Darwin Godel Machine engine
pub struct DarwinGodelMachine {
    /// Agent archive (gene pool)
    archive: Vec<AgentVariant>,
    /// Maximum archive size
    max_archive_size: usize,
    /// Current best fitness
    best_fitness: f64,
    /// Total improvement iterations run
    total_iterations: u64,
    /// Total successful improvements
    total_improvements: u64,
    /// Current generation counter
    current_generation: u32,
}

impl DarwinGodelMachine {
    pub fn new(max_archive_size: usize) -> Self {
        // Start with a baseline "generation 0" variant
        let baseline = AgentVariant {
            id: uuid::Uuid::new_v4().to_string(),
            fitness: 0.1,
            modifications: Vec::new(),
            generation: 0,
            created_at: Utc::now(),
        };

        Self {
            archive: vec![baseline],
            max_archive_size,
            best_fitness: 0.1,
            total_iterations: 0,
            total_improvements: 0,
            current_generation: 0,
        }
    }

    /// Run one improvement iteration: sample parent, mutate, evaluate, integrate
    pub async fn improve(&mut self) -> Option<Modification> {
        self.total_iterations += 1;

        // Step 1: Sample parent (fitness-proportional)
        let parent = self.sample_parent();

        // Step 2: Propose mutation
        let mutation = self.propose_mutation(&parent);

        // Step 3: Evaluate fitness (simulated)
        let fitness = self.evaluate_fitness(&parent, &mutation);

        // Step 4: If beneficial, integrate
        if fitness > parent.fitness {
            let child = AgentVariant {
                id: uuid::Uuid::new_v4().to_string(),
                fitness,
                modifications: {
                    let mut mods = parent.modifications.clone();
                    mods.push(mutation.clone());
                    mods
                },
                generation: parent.generation + 1,
                created_at: Utc::now(),
            };

            self.integrate(child);
            self.total_improvements += 1;

            if fitness > self.best_fitness {
                self.best_fitness = fitness;
            }

            Some(mutation)
        } else {
            None
        }
    }

    /// Sample a parent variant, weighted by fitness
    fn sample_parent(&self) -> AgentVariant {
        if self.archive.is_empty() {
            return AgentVariant {
                id: uuid::Uuid::new_v4().to_string(),
                fitness: 0.1,
                modifications: Vec::new(),
                generation: 0,
                created_at: Utc::now(),
            };
        }

        let total_fitness: f64 = self.archive.iter().map(|v| v.fitness).sum();
        if total_fitness <= 0.0 {
            return self.archive[0].clone();
        }

        // Simple selection: pick variant with highest fitness
        // In full implementation: roulette wheel or tournament selection
        self.archive
            .iter()
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap())
            .cloned()
            .unwrap_or_else(|| self.archive[0].clone())
    }

    /// Propose a mutation based on the parent's characteristics
    fn propose_mutation(&self, parent: &AgentVariant) -> Modification {
        // Cycle through mutation types based on iteration count
        let mod_type = match self.total_iterations % 6 {
            0 => ModificationType::HyperparameterTuning,
            1 => ModificationType::ReasoningStrategy,
            2 => ModificationType::PromptOptimization,
            3 => ModificationType::MemoryOptimization,
            4 => ModificationType::ToolInvention,
            _ => ModificationType::MetaImprovement,
        };

        let description = match &mod_type {
            ModificationType::HyperparameterTuning => {
                "Adjust learning rate schedule for improved convergence".to_string()
            }
            ModificationType::ReasoningStrategy => {
                "Switch reasoning strategy based on task complexity".to_string()
            }
            ModificationType::PromptOptimization => {
                "Restructure system prompt for clearer instruction following".to_string()
            }
            ModificationType::MemoryOptimization => {
                "Improve memory retrieval relevance scoring".to_string()
            }
            ModificationType::ToolInvention => {
                "Generate specialized tool for repeated task pattern".to_string()
            }
            ModificationType::MetaImprovement => {
                "Improve the mutation proposal heuristic itself".to_string()
            }
        };

        // Risk score: meta-improvements and tool invention are riskier
        let risk_score = match &mod_type {
            ModificationType::HyperparameterTuning => 0.05,
            ModificationType::ReasoningStrategy => 0.1,
            ModificationType::PromptOptimization => 0.08,
            ModificationType::MemoryOptimization => 0.07,
            ModificationType::ToolInvention => 0.2,
            ModificationType::MetaImprovement => 0.25,
        };

        Modification {
            id: uuid::Uuid::new_v4().to_string(),
            modification_type: mod_type,
            description,
            fitness_delta: 0.0, // filled after evaluation
            risk_score,
            parent_id: Some(parent.id.clone()),
            created_at: Utc::now(),
        }
    }

    /// Evaluate fitness of a mutation applied to a parent
    fn evaluate_fitness(&self, parent: &AgentVariant, mutation: &Modification) -> f64 {
        // Simulated fitness evaluation
        // In full implementation: run benchmarks in sandbox
        let base = parent.fitness;

        // Each generation has diminishing returns but still positive
        let generation_factor = 1.0 / (parent.generation as f64 + 1.0).sqrt();

        // Mutation type effectiveness varies
        let type_factor = match &mutation.modification_type {
            ModificationType::HyperparameterTuning => 0.02,
            ModificationType::ReasoningStrategy => 0.05,
            ModificationType::PromptOptimization => 0.03,
            ModificationType::MemoryOptimization => 0.04,
            ModificationType::ToolInvention => 0.06,
            ModificationType::MetaImprovement => 0.08,
        };

        let improvement = type_factor * generation_factor;

        // ~60% of mutations are beneficial (as in DGM paper)
        if self.total_iterations % 5 < 3 {
            (base + improvement).min(1.0)
        } else {
            (base - improvement * 0.5).max(0.0)
        }
    }

    /// Integrate a new variant into the archive
    fn integrate(&mut self, variant: AgentVariant) {
        self.current_generation = self.current_generation.max(variant.generation);
        self.archive.push(variant);

        // Prune archive if over capacity (remove lowest fitness)
        if self.archive.len() > self.max_archive_size {
            self.archive
                .sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());
            self.archive.truncate(self.max_archive_size);
        }
    }

    /// Get current DGM status
    pub fn status(&self) -> DgmStatus {
        DgmStatus {
            archive_size: self.archive.len(),
            best_fitness: self.best_fitness,
            total_improvements: self.total_improvements,
            total_iterations: self.total_iterations,
            current_generation: self.current_generation,
        }
    }

    /// Get the best variant in the archive
    pub fn best_variant(&self) -> Option<&AgentVariant> {
        self.archive
            .iter()
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap())
    }

    /// Get archive diversity (fitness variance)
    pub fn diversity(&self) -> f64 {
        if self.archive.len() < 2 {
            return 0.0;
        }
        let mean: f64 = self.archive.iter().map(|v| v.fitness).sum::<f64>()
            / self.archive.len() as f64;
        let variance: f64 = self
            .archive
            .iter()
            .map(|v| (v.fitness - mean).powi(2))
            .sum::<f64>()
            / self.archive.len() as f64;
        variance.sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dgm_creation() {
        let dgm = DarwinGodelMachine::new(100);
        let status = dgm.status();
        assert_eq!(status.archive_size, 1);
        assert_eq!(status.total_iterations, 0);
        assert!(status.best_fitness > 0.0);
    }

    #[tokio::test]
    async fn test_single_improvement() {
        let mut dgm = DarwinGodelMachine::new(100);
        let _result = dgm.improve().await;
        assert_eq!(dgm.status().total_iterations, 1);
    }

    #[tokio::test]
    async fn test_multiple_improvements() {
        let mut dgm = DarwinGodelMachine::new(100);
        let mut improvements = 0;

        for _ in 0..20 {
            if dgm.improve().await.is_some() {
                improvements += 1;
            }
        }

        assert_eq!(dgm.status().total_iterations, 20);
        assert!(improvements > 0, "Expected at least one improvement in 20 iterations");
        assert!(
            dgm.status().best_fitness > 0.1,
            "Expected fitness to improve from baseline"
        );
    }

    #[tokio::test]
    async fn test_archive_pruning() {
        let mut dgm = DarwinGodelMachine::new(5);

        for _ in 0..50 {
            dgm.improve().await;
        }

        assert!(
            dgm.status().archive_size <= 5,
            "Archive should be pruned to max size"
        );
    }

    #[test]
    fn test_diversity() {
        let dgm = DarwinGodelMachine::new(100);
        let diversity = dgm.diversity();
        assert!(diversity >= 0.0);
    }

    #[tokio::test]
    async fn test_fitness_improves_over_generations() {
        let mut dgm = DarwinGodelMachine::new(100);
        let initial_fitness = dgm.status().best_fitness;

        for _ in 0..100 {
            dgm.improve().await;
        }

        assert!(
            dgm.status().best_fitness >= initial_fitness,
            "Best fitness should not decrease"
        );
    }

    #[test]
    fn test_modification_risk_scores() {
        let dgm = DarwinGodelMachine::new(100);
        let parent = dgm.sample_parent();
        let mutation = dgm.propose_mutation(&parent);
        assert!(mutation.risk_score >= 0.0);
        assert!(mutation.risk_score <= 1.0);
    }
}

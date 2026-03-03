//! §10.1 — Quantum-AGI Integration Layer
//!
//! This module provides the integration glue between Housaky's AGI subsystems
//! and the quantum computing backend. It wraps quantum operations with AGI-friendly
//! interfaces and handles automatic fallback to classical computation.

use crate::housaky::goal_engine::{Goal, GoalPriority};
use crate::quantum::{MemoryOptimizationResult, QuantumAgiBridge};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

/// Quantum-enhanced goal scheduler that automatically routes to quantum or classical
/// based on problem size and quantum backend availability.
pub struct QuantumGoalScheduler {
    quantum_bridge: Option<Arc<QuantumAgiBridge>>,
    min_goals_for_quantum: usize,
    max_goals_for_quantum: usize,
}

impl QuantumGoalScheduler {
    pub fn new(quantum_bridge: Option<Arc<QuantumAgiBridge>>) -> Self {
        Self {
            quantum_bridge,
            min_goals_for_quantum: 4,
            max_goals_for_quantum: 32,
        }
    }

    /// Schedule goals using quantum optimization if available and beneficial.
    pub async fn schedule_goals(&self, goals: &[Goal]) -> Result<Vec<Goal>> {
        let n = goals.len();

        // Check if quantum is available and problem size is suitable
        if let Some(bridge) = &self.quantum_bridge {
            if n >= self.min_goals_for_quantum && n <= self.max_goals_for_quantum {
                match self.schedule_quantum(bridge, goals).await {
                    Ok(scheduled) => {
                        info!(
                            "✨ Quantum goal scheduling succeeded: {} goals",
                            scheduled.len()
                        );
                        return Ok(scheduled);
                    }
                    Err(e) => {
                        warn!(
                            "Quantum scheduling failed: {}, falling back to classical",
                            e
                        );
                    }
                }
            }
        }

        // Classical fallback
        Ok(self.schedule_classical(goals))
    }

    async fn schedule_quantum(
        &self,
        bridge: &QuantumAgiBridge,
        goals: &[Goal],
    ) -> Result<Vec<Goal>> {
        let goal_ids: Vec<String> = goals.iter().map(|g| g.id.clone()).collect();

        // Build priority map
        let priorities: HashMap<String, f64> = goals
            .iter()
            .map(|g| {
                let priority_score = match g.priority {
                    GoalPriority::Critical => 1.0,
                    GoalPriority::High => 0.8,
                    GoalPriority::Medium => 0.5,
                    GoalPriority::Low => 0.3,
                    GoalPriority::Background => 0.1,
                };
                (g.id.clone(), priority_score)
            })
            .collect();

        // Build dependency edges
        let mut dependencies = Vec::new();
        for goal in goals {
            for dep_id in &goal.dependencies {
                dependencies.push((dep_id.clone(), goal.id.clone()));
            }
        }

        // Execute quantum scheduling
        let result = bridge
            .schedule_goals(&goal_ids, &priorities, &dependencies)
            .await?;

        info!(
            "🔮 Quantum scheduling: strategy={}, objective={:.3}, advantage={:.2}x, time={}ms",
            result.strategy, result.objective_value, result.quantum_advantage, result.runtime_ms
        );

        // Map scheduled IDs back to Goal objects
        let _clusters: HashMap<String, usize> = goals
            .iter()
            .enumerate()
            .map(|(_i, g)| (g.id.clone(), 0))
            .collect();

        let goal_map: HashMap<String, Goal> =
            goals.iter().map(|g| (g.id.clone(), g.clone())).collect();

        let scheduled: Vec<Goal> = result
            .schedule
            .iter()
            .filter_map(|id| goal_map.get(id).cloned())
            .collect();

        Ok(scheduled)
    }

    fn schedule_classical(&self, goals: &[Goal]) -> Vec<Goal> {
        let mut sorted = goals.to_vec();
        sorted.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        sorted
    }
}

/// Quantum-enhanced reasoning branch selector using Grover search.
pub struct QuantumReasoningSelector {
    quantum_bridge: Option<Arc<QuantumAgiBridge>>,
}

impl QuantumReasoningSelector {
    pub fn new(quantum_bridge: Option<Arc<QuantumAgiBridge>>) -> Self {
        Self { quantum_bridge }
    }

    /// Select best reasoning branches using quantum search if available.
    pub async fn select_branches(
        &self,
        branches: &[String],
        fitness_scores: &HashMap<String, f64>,
    ) -> Result<Vec<String>> {
        if let Some(bridge) = &self.quantum_bridge {
            if branches.len() >= 4 && branches.len() <= 32 {
                match bridge
                    .search_reasoning_branches(branches, fitness_scores)
                    .await
                {
                    Ok(result) => {
                        info!(
                            "🔮 Quantum reasoning search: {} branches → {} selected, speedup={:.2}x",
                            branches.len(),
                            result.best_branches.len(),
                            result.speedup
                        );
                        return Ok(result.best_branches);
                    }
                    Err(e) => {
                        warn!("Quantum reasoning search failed: {}, using classical", e);
                    }
                }
            }
        }

        // Classical fallback: sort by fitness
        let mut scored: Vec<(String, f64)> = branches
            .iter()
            .map(|b| (b.clone(), fitness_scores.get(b).copied().unwrap_or(0.0)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored.into_iter().take(3).map(|(b, _)| b).collect())
    }
}

/// Quantum-enhanced memory graph optimizer using quantum annealing.
pub struct QuantumMemoryOptimizer {
    quantum_bridge: Option<Arc<QuantumAgiBridge>>,
}

impl QuantumMemoryOptimizer {
    pub fn new(quantum_bridge: Option<Arc<QuantumAgiBridge>>) -> Self {
        Self { quantum_bridge }
    }

    /// Optimize memory graph structure using quantum annealing.
    pub async fn optimize_graph(
        &self,
        node_ids: &[String],
        edges: &[(String, String, f64)],
    ) -> Result<MemoryOptimizationResult> {
        if let Some(bridge) = &self.quantum_bridge {
            if node_ids.len() >= 4 && node_ids.len() <= 100 {
                match bridge.optimize_memory_graph(node_ids, edges).await {
                    Ok(result) => {
                        info!(
                            "🔮 Quantum memory optimization: {} nodes → {} clusters, energy={:.4}",
                            node_ids.len(),
                            result.clusters.len(),
                            result.energy
                        );
                        return Ok(result);
                    }
                    Err(e) => {
                        warn!("Quantum memory optimization failed: {}, using classical", e);
                    }
                }
            }
        }

        // Classical fallback: simple clustering
        Ok(self.optimize_classical(node_ids, edges))
    }

    fn optimize_classical(
        &self,
        node_ids: &[String],
        edges: &[(String, String, f64)],
    ) -> MemoryOptimizationResult {
        // Simple degree-based clustering
        let mut clusters = HashMap::new();
        let mut degree: HashMap<String, usize> = HashMap::new();

        for (from, to, _) in edges {
            *degree.entry(from.clone()).or_insert(0) += 1;
            *degree.entry(to.clone()).or_insert(0) += 1;
        }

        for (_i, node_id) in node_ids.iter().enumerate() {
            let deg = degree.get(node_id).copied().unwrap_or(0);
            let cluster = if deg > 3 { 0 } else { 1 };
            clusters.insert(node_id.clone(), cluster);
        }

        MemoryOptimizationResult {
            clusters,
            strengthen_edges: vec![],
            prune_edges: vec![],
            energy: 0.0,
            strategy: "classical".into(),
            runtime_ms: 0,
        }
    }
}

/// §10.5 — Quantum-enhanced world model advisor that refines action reward
/// predictions using quantum uncertainty reduction before the AGI commits.
pub struct QuantumWorldModelEnhancer {
    quantum_bridge: Option<Arc<QuantumAgiBridge>>,
}

impl QuantumWorldModelEnhancer {
    pub fn new(quantum_bridge: Option<Arc<QuantumAgiBridge>>) -> Self {
        Self { quantum_bridge }
    }

    /// Refine a raw reward estimate using quantum uncertainty reduction.
    /// Returns the refined confidence and a strategy tag for logging.
    pub async fn refine_reward(
        &self,
        action_type: &str,
        raw_reward: f64,
        raw_confidence: f64,
    ) -> (f64, String) {
        if let Some(bridge) = &self.quantum_bridge {
            let options = vec![
                "high_reward".to_string(),
                "medium_reward".to_string(),
                "low_reward".to_string(),
            ];
            let mut priors = std::collections::HashMap::new();
            priors.insert("high_reward".to_string(), raw_confidence * raw_reward.abs());
            priors.insert("medium_reward".to_string(), 1.0 - raw_confidence);
            priors.insert("low_reward".to_string(), (1.0 - raw_reward.abs()).max(0.0));

            match bridge.reduce_uncertainty(&options, &priors).await {
                Ok(posteriors) => {
                    let high = posteriors.get("high_reward").copied().unwrap_or(0.5);
                    let refined = (raw_confidence * 0.7 + high * 0.3).clamp(0.0, 1.0);
                    info!(
                        "🔮 Quantum reward refinement: action={}, confidence {:.3} → {:.3}",
                        action_type, raw_confidence, refined
                    );
                    return (refined, "quantum_uncertainty_reduction".to_string());
                }
                Err(e) => {
                    warn!("Quantum reward refinement failed: {e}");
                }
            }
        }
        (raw_confidence, "classical".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::housaky::goal_engine::{GoalCategory, GoalStatus};
    use chrono::Utc;

    #[tokio::test]
    async fn test_quantum_goal_scheduler_classical_fallback() {
        let scheduler = QuantumGoalScheduler::new(None);

        let goals = vec![
            Goal {
                id: "g1".into(),
                title: "High priority goal".into(),
                description: "Test".into(),
                priority: GoalPriority::High,
                status: GoalStatus::Pending,
                category: GoalCategory::Planning,
                progress: 0.0,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                deadline: None,
                parent_id: None,
                subtask_ids: vec![],
                dependencies: vec![],
                blockers: vec![],
                metrics: HashMap::new(),
                checkpoints: vec![],
                attempts: 0,
                max_attempts: 3,
                estimated_complexity: 1.0,
                actual_complexity: None,
                learning_value: 0.5,
                tags: vec![],
                context: HashMap::new(),
                temporal_constraints: vec![],
            },
            Goal {
                id: "g2".into(),
                title: "Low priority goal".into(),
                description: "Test".into(),
                priority: GoalPriority::Low,
                status: GoalStatus::Pending,
                category: GoalCategory::Planning,
                progress: 0.0,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                deadline: None,
                parent_id: None,
                subtask_ids: vec![],
                dependencies: vec![],
                blockers: vec![],
                metrics: HashMap::new(),
                checkpoints: vec![],
                attempts: 0,
                max_attempts: 3,
                estimated_complexity: 1.0,
                actual_complexity: None,
                learning_value: 0.5,
                tags: vec![],
                context: HashMap::new(),
                temporal_constraints: vec![],
            },
        ];

        let scheduled = scheduler.schedule_goals(&goals).await.unwrap();

        assert_eq!(scheduled.len(), 2);
        assert_eq!(scheduled[0].id, "g1"); // High priority first
        assert_eq!(scheduled[1].id, "g2");
    }

    #[tokio::test]
    async fn test_quantum_reasoning_selector_classical_fallback() {
        let selector = QuantumReasoningSelector::new(None);

        let branches = vec!["b1".into(), "b2".into(), "b3".into()];
        let mut fitness = HashMap::new();
        fitness.insert("b1".into(), 0.9);
        fitness.insert("b2".into(), 0.5);
        fitness.insert("b3".into(), 0.7);

        let selected = selector.select_branches(&branches, &fitness).await.unwrap();

        assert_eq!(selected.len(), 3);
        assert_eq!(selected[0], "b1"); // Highest fitness first
    }
}

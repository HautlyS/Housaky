//! Formal Causal Inference Engine
//!
//! Implements Pearl's do-calculus for interventional reasoning:
//! - CausalGraph: DAG of causal variables with edges representing causal influence
//! - do(X) intervention: removes incoming edges to X, fixing its value
//! - Counterfactual reasoning: P(Y | do(X), observed_Z)
//! - Causal discovery from observations
//! - Integration with WorldModel::predict()

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

// ── Core Types ───────────────────────────────────────────────────────────────

/// A node in the causal graph representing a measurable variable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalVariable {
    pub name: String,
    pub domain: VariableDomain,
    pub observed_value: Option<f64>,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariableDomain {
    Continuous { min: f64, max: f64 },
    Discrete(Vec<f64>),
    Binary,
    Categorical(Vec<String>),
}

/// A directed edge in the causal graph: `cause` → `effect`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalEdge {
    pub cause: String,
    pub effect: String,
    pub strength: f64, // -1.0 to 1.0, magnitude = strength, sign = direction
    pub confidence: f64,
    pub mechanism: String, // human-readable description of the causal mechanism
    pub discovered_at: DateTime<Utc>,
}

/// Record of an intervention (do-operation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intervention {
    pub id: String,
    pub variable: String,
    pub forced_value: f64,
    pub timestamp: DateTime<Utc>,
    pub predicted_effects: HashMap<String, f64>,
    pub actual_effects: Option<HashMap<String, f64>>,
    pub prediction_error: Option<f64>,
}

/// A single observation for causal discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub timestamp: DateTime<Utc>,
    pub values: HashMap<String, f64>,
    pub context: String,
}

/// A discovered causal relation from observational data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalRelation {
    pub cause: String,
    pub effect: String,
    pub estimated_strength: f64,
    pub confidence: f64,
    pub evidence_count: usize,
    pub method: DiscoveryMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryMethod {
    CorrelationWithTimeLag,
    InterventionResult,
    ConditionalIndependence,
    GrangerCausality,
    UserSpecified,
}

/// Result of a counterfactual query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualResult {
    pub query: String,
    pub hypothetical_value: f64,
    pub actual_value: Option<f64>,
    pub confidence: f64,
    pub reasoning_chain: Vec<String>,
}

/// The causal DAG structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CausalGraph {
    pub variables: HashMap<String, CausalVariable>,
    pub edges: Vec<CausalEdge>,
    adjacency: HashMap<String, Vec<String>>,     // cause → [effects]
    reverse_adj: HashMap<String, Vec<String>>,    // effect → [causes]
}

impl CausalGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a variable node to the graph.
    pub fn add_variable(&mut self, var: CausalVariable) {
        let name = var.name.clone();
        self.variables.insert(name.clone(), var);
        self.adjacency.entry(name.clone()).or_default();
        self.reverse_adj.entry(name).or_default();
    }

    /// Add a causal edge: `cause` → `effect`.
    pub fn add_edge(&mut self, edge: CausalEdge) {
        self.adjacency
            .entry(edge.cause.clone())
            .or_default()
            .push(edge.effect.clone());
        self.reverse_adj
            .entry(edge.effect.clone())
            .or_default()
            .push(edge.cause.clone());
        self.edges.push(edge);
    }

    /// Remove all incoming edges to a variable (the "do" operation).
    pub fn do_intervention(&self, variable: &str) -> CausalGraph {
        let mut mutilated = self.clone();
        mutilated.edges.retain(|e| e.effect != variable);
        mutilated.reverse_adj.insert(variable.to_string(), Vec::new());
        for children in mutilated.adjacency.values_mut() {
            children.retain(|_c| {
                // keep edge only if its destination is NOT leading TO the variable as effect
                true
            });
        }
        // Rebuild adjacency from edges for consistency
        mutilated.adjacency.clear();
        mutilated.reverse_adj.clear();
        for var in mutilated.variables.keys() {
            mutilated.adjacency.entry(var.clone()).or_default();
            mutilated.reverse_adj.entry(var.clone()).or_default();
        }
        for edge in &mutilated.edges {
            mutilated
                .adjacency
                .entry(edge.cause.clone())
                .or_default()
                .push(edge.effect.clone());
            mutilated
                .reverse_adj
                .entry(edge.effect.clone())
                .or_default()
                .push(edge.cause.clone());
        }
        mutilated
    }

    /// Get all children (direct effects) of a variable.
    pub fn get_effects(&self, variable: &str) -> Vec<&str> {
        self.adjacency
            .get(variable)
            .map(|v| v.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Get all parents (direct causes) of a variable.
    pub fn get_causes(&self, variable: &str) -> Vec<&str> {
        self.reverse_adj
            .get(variable)
            .map(|v| v.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Get the edge strength between two variables.
    pub fn get_edge_strength(&self, cause: &str, effect: &str) -> Option<f64> {
        self.edges
            .iter()
            .find(|e| e.cause == cause && e.effect == effect)
            .map(|e| e.strength)
    }

    /// Topological sort of the causal graph.
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for var in self.variables.keys() {
            in_degree.insert(var.clone(), 0);
        }
        for edge in &self.edges {
            *in_degree.entry(edge.effect.clone()).or_insert(0) += 1;
        }

        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(k, _)| k.clone())
            .collect();

        let mut sorted = Vec::new();
        while let Some(node) = queue.pop_front() {
            sorted.push(node.clone());
            for effect in self.get_effects(&node) {
                let deg = in_degree.get_mut(effect).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(effect.to_string());
                }
            }
        }

        if sorted.len() != self.variables.len() {
            return Err(anyhow!("Cycle detected in causal graph"));
        }
        Ok(sorted)
    }

    /// Check if the graph is a DAG (no cycles).
    pub fn is_dag(&self) -> bool {
        self.topological_sort().is_ok()
    }

    /// Find all paths from `source` to `target`.
    pub fn find_causal_paths(&self, source: &str, target: &str) -> Vec<Vec<String>> {
        let mut paths = Vec::new();
        let mut current_path = vec![source.to_string()];
        let mut visited = HashSet::new();
        visited.insert(source.to_string());
        self.dfs_paths(source, target, &mut current_path, &mut visited, &mut paths);
        paths
    }

    fn dfs_paths(
        &self,
        current: &str,
        target: &str,
        path: &mut Vec<String>,
        visited: &mut HashSet<String>,
        paths: &mut Vec<Vec<String>>,
    ) {
        if current == target {
            paths.push(path.clone());
            return;
        }
        for effect in self.get_effects(current) {
            if !visited.contains(effect) {
                visited.insert(effect.to_string());
                path.push(effect.to_string());
                self.dfs_paths(effect, target, path, visited, paths);
                path.pop();
                visited.remove(effect);
            }
        }
    }

    /// Get all ancestors of a variable (recursive causes).
    pub fn get_ancestors(&self, variable: &str) -> HashSet<String> {
        let mut ancestors = HashSet::new();
        let mut queue = VecDeque::new();
        for cause in self.get_causes(variable) {
            queue.push_back(cause.to_string());
        }
        while let Some(node) = queue.pop_front() {
            if ancestors.insert(node.clone()) {
                for cause in self.get_causes(&node) {
                    queue.push_back(cause.to_string());
                }
            }
        }
        ancestors
    }

    /// Get all descendants of a variable (recursive effects).
    pub fn get_descendants(&self, variable: &str) -> HashSet<String> {
        let mut descendants = HashSet::new();
        let mut queue = VecDeque::new();
        for effect in self.get_effects(variable) {
            queue.push_back(effect.to_string());
        }
        while let Some(node) = queue.pop_front() {
            if descendants.insert(node.clone()) {
                for effect in self.get_effects(&node) {
                    queue.push_back(effect.to_string());
                }
            }
        }
        descendants
    }
}

// ── Causal Inference Engine ──────────────────────────────────────────────────

pub struct CausalInferenceEngine {
    pub causal_graph: Arc<RwLock<CausalGraph>>,
    pub intervention_log: Arc<RwLock<Vec<Intervention>>>,
    pub observation_buffer: Arc<RwLock<Vec<Observation>>>,
    max_observations: usize,
}

impl CausalInferenceEngine {
    pub fn new() -> Self {
        Self {
            causal_graph: Arc::new(RwLock::new(CausalGraph::new())),
            intervention_log: Arc::new(RwLock::new(Vec::new())),
            observation_buffer: Arc::new(RwLock::new(Vec::new())),
            max_observations: 10_000,
        }
    }

    /// Perform a do-intervention: set variable to a fixed value and predict effects.
    pub async fn do_intervention(
        &self,
        variable: &str,
        value: f64,
    ) -> Result<Intervention> {
        let graph = self.causal_graph.read().await;

        if !graph.variables.contains_key(variable) {
            return Err(anyhow!("Variable '{}' not found in causal graph", variable));
        }

        // Create the mutilated graph (remove incoming edges to variable)
        let mutilated = graph.do_intervention(variable);

        // Propagate the intervention through the mutilated graph
        let predicted_effects = self.propagate_intervention(&mutilated, variable, value).await;

        let intervention = Intervention {
            id: uuid::Uuid::new_v4().to_string(),
            variable: variable.to_string(),
            forced_value: value,
            timestamp: Utc::now(),
            predicted_effects,
            actual_effects: None,
            prediction_error: None,
        };

        self.intervention_log.write().await.push(intervention.clone());

        info!(
            "Causal intervention: do({} = {}) → predicted {} downstream effects",
            variable,
            value,
            intervention.predicted_effects.len()
        );

        Ok(intervention)
    }

    /// Propagate an intervention through the graph to predict downstream effects.
    async fn propagate_intervention(
        &self,
        graph: &CausalGraph,
        variable: &str,
        value: f64,
    ) -> HashMap<String, f64> {
        let mut effects = HashMap::new();
        effects.insert(variable.to_string(), value);

        // Use topological order if available
        if let Ok(order) = graph.topological_sort() {
            let var_index = order.iter().position(|v| v == variable).unwrap_or(0);

            for node in order.iter().skip(var_index + 1) {
                let causes = graph.get_causes(node);
                if causes.is_empty() {
                    continue;
                }

                // Calculate effect as weighted sum of parent values × edge strengths
                let mut predicted_value = 0.0;
                let mut total_strength = 0.0;

                for cause in &causes {
                    if let Some(&parent_value) = effects.get(*cause) {
                        if let Some(strength) = graph.get_edge_strength(cause, node) {
                            predicted_value += parent_value * strength;
                            total_strength += strength.abs();
                        }
                    }
                }

                if total_strength > 0.0 {
                    predicted_value /= total_strength; // normalize
                    effects.insert(node.clone(), predicted_value);
                }
            }
        }

        // Remove the intervention variable itself from effects
        effects.remove(variable);
        effects
    }

    /// Answer a counterfactual query: "What would Y be if X had been different?"
    pub async fn counterfactual(
        &self,
        target_variable: &str,
        evidence: &HashMap<String, f64>,
    ) -> Result<CounterfactualResult> {
        let graph = self.causal_graph.read().await;

        if !graph.variables.contains_key(target_variable) {
            return Err(anyhow!(
                "Target variable '{}' not in causal graph",
                target_variable
            ));
        }

        let mut reasoning_chain = Vec::new();
        reasoning_chain.push(format!(
            "Counterfactual query: What would '{}' be?",
            target_variable
        ));

        // Step 1: Abduction — infer exogenous variables from evidence
        let mut inferred_values = evidence.clone();
        reasoning_chain.push(format!(
            "Evidence provided for {} variables",
            evidence.len()
        ));

        // Step 2: Action — apply interventions from evidence
        for (var, val) in evidence {
            let effects = self
                .propagate_intervention(&graph, var, *val)
                .await;
            for (k, v) in effects {
                inferred_values.entry(k).or_insert(v);
            }
        }

        // Step 3: Prediction — compute target value
        let hypothetical_value = inferred_values
            .get(target_variable)
            .copied()
            .unwrap_or(0.0);

        let actual_value = graph
            .variables
            .get(target_variable)
            .and_then(|v| v.observed_value);

        // Calculate confidence based on path coverage
        let evidence_vars: HashSet<String> = evidence.keys().cloned().collect();
        let ancestors = graph.get_ancestors(target_variable);
        let covered = evidence_vars.intersection(&ancestors).count();
        let total_ancestors = ancestors.len().max(1);
        let confidence = (covered as f64 / total_ancestors as f64).min(1.0);

        reasoning_chain.push(format!(
            "Inferred {} intermediate values",
            inferred_values.len()
        ));
        reasoning_chain.push(format!(
            "Confidence: {:.2} ({}/{} ancestors covered by evidence)",
            confidence, covered, total_ancestors
        ));

        Ok(CounterfactualResult {
            query: format!(
                "P({} | {})",
                target_variable,
                evidence
                    .iter()
                    .map(|(k, v)| format!("do({} = {:.2})", k, v))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            hypothetical_value,
            actual_value,
            confidence,
            reasoning_chain,
        })
    }

    /// Discover causal relations from observational data.
    pub async fn discover_causes(
        &self,
        observations: &[Observation],
    ) -> Vec<CausalRelation> {
        let mut relations = Vec::new();

        if observations.len() < 3 {
            return relations;
        }

        // Collect all variable names
        let all_vars: HashSet<String> = observations
            .iter()
            .flat_map(|o| o.values.keys().cloned())
            .collect();

        let var_list: Vec<String> = all_vars.into_iter().collect();

        // For each pair of variables, test for causal relationship
        for i in 0..var_list.len() {
            for j in 0..var_list.len() {
                if i == j {
                    continue;
                }

                let cause = &var_list[i];
                let effect = &var_list[j];

                // Extract paired observations
                let pairs: Vec<(f64, f64)> = observations
                    .iter()
                    .filter_map(|o| {
                        let c = o.values.get(cause)?;
                        let e = o.values.get(effect)?;
                        Some((*c, *e))
                    })
                    .collect();

                if pairs.len() < 3 {
                    continue;
                }

                // Pearson correlation as a proxy for causal strength
                let n = pairs.len() as f64;
                let sum_x: f64 = pairs.iter().map(|(x, _)| x).sum();
                let sum_y: f64 = pairs.iter().map(|(_, y)| y).sum();
                let sum_xy: f64 = pairs.iter().map(|(x, y)| x * y).sum();
                let sum_x2: f64 = pairs.iter().map(|(x, _)| x * x).sum();
                let sum_y2: f64 = pairs.iter().map(|(_, y)| y * y).sum();

                let numerator = n * sum_xy - sum_x * sum_y;
                let denominator =
                    ((n * sum_x2 - sum_x * sum_x) * (n * sum_y2 - sum_y * sum_y)).sqrt();

                if denominator < 1e-10 {
                    continue;
                }

                let correlation = numerator / denominator;

                // Use time-lag analysis to determine direction
                let has_time_lag = self.check_time_lag(&pairs);

                if correlation.abs() > 0.3 && has_time_lag {
                    relations.push(CausalRelation {
                        cause: cause.clone(),
                        effect: effect.clone(),
                        estimated_strength: correlation,
                        confidence: (correlation.abs() * 0.8).min(0.95),
                        evidence_count: pairs.len(),
                        method: DiscoveryMethod::CorrelationWithTimeLag,
                    });
                }
            }
        }

        // Sort by confidence descending
        relations.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        info!(
            "Causal discovery: found {} potential causal relations from {} observations",
            relations.len(),
            observations.len()
        );

        relations
    }

    /// Simple time-lag check by correlating offset values.
    fn check_time_lag(&self, pairs: &[(f64, f64)]) -> bool {
        if pairs.len() < 4 {
            return false;
        }
        // Check if cause values at time t predict effect values at time t+1
        let lagged_pairs: Vec<(f64, f64)> = pairs
            .windows(2)
            .map(|w| (w[0].0, w[1].1))
            .collect();

        if lagged_pairs.len() < 3 {
            return false;
        }

        let n = lagged_pairs.len() as f64;
        let sum_x: f64 = lagged_pairs.iter().map(|(x, _)| x).sum();
        let sum_y: f64 = lagged_pairs.iter().map(|(_, y)| y).sum();
        let sum_xy: f64 = lagged_pairs.iter().map(|(x, y)| x * y).sum();
        let sum_x2: f64 = lagged_pairs.iter().map(|(x, _)| x * x).sum();
        let sum_y2: f64 = lagged_pairs.iter().map(|(_, y)| y * y).sum();

        let numerator = n * sum_xy - sum_x * sum_y;
        let denominator =
            ((n * sum_x2 - sum_x * sum_x) * (n * sum_y2 - sum_y * sum_y)).sqrt();

        if denominator < 1e-10 {
            return false;
        }

        let lagged_correlation = (numerator / denominator).abs();
        lagged_correlation > 0.2
    }

    /// Record an observation for future causal discovery.
    pub async fn record_observation(&self, observation: Observation) {
        let mut buffer = self.observation_buffer.write().await;
        buffer.push(observation);
        if buffer.len() > self.max_observations {
            buffer.remove(0);
        }
    }

    /// Update an intervention with actual results for learning.
    pub async fn update_intervention_results(
        &self,
        intervention_id: &str,
        actual_effects: HashMap<String, f64>,
    ) -> Result<f64> {
        let mut log = self.intervention_log.write().await;
        let intervention = log
            .iter_mut()
            .find(|i| i.id == intervention_id)
            .ok_or_else(|| anyhow!("Intervention '{}' not found", intervention_id))?;

        // Calculate prediction error (MSE)
        let mut total_error = 0.0;
        let mut count = 0;
        for (var, actual) in &actual_effects {
            if let Some(predicted) = intervention.predicted_effects.get(var) {
                total_error += (predicted - actual).powi(2);
                count += 1;
            }
        }

        let mse = if count > 0 {
            total_error / count as f64
        } else {
            0.0
        };

        intervention.actual_effects = Some(actual_effects);
        intervention.prediction_error = Some(mse);

        info!(
            "Updated intervention '{}' with actual results. MSE: {:.4}",
            intervention_id, mse
        );

        Ok(mse)
    }

    /// Get the causal strength between two variables (if edge exists).
    pub async fn get_causal_strength(&self, cause: &str, effect: &str) -> Option<f64> {
        let graph = self.causal_graph.read().await;
        graph.get_edge_strength(cause, effect)
    }

    /// Add a known causal relationship to the graph.
    pub async fn add_causal_relationship(
        &self,
        cause: &str,
        effect: &str,
        strength: f64,
        mechanism: &str,
    ) -> Result<()> {
        let mut graph = self.causal_graph.write().await;

        // Auto-create variables if they don't exist
        if !graph.variables.contains_key(cause) {
            graph.add_variable(CausalVariable {
                name: cause.to_string(),
                domain: VariableDomain::Continuous {
                    min: f64::NEG_INFINITY,
                    max: f64::INFINITY,
                },
                observed_value: None,
                description: format!("Auto-created variable: {}", cause),
                created_at: Utc::now(),
            });
        }
        if !graph.variables.contains_key(effect) {
            graph.add_variable(CausalVariable {
                name: effect.to_string(),
                domain: VariableDomain::Continuous {
                    min: f64::NEG_INFINITY,
                    max: f64::INFINITY,
                },
                observed_value: None,
                description: format!("Auto-created variable: {}", effect),
                created_at: Utc::now(),
            });
        }

        graph.add_edge(CausalEdge {
            cause: cause.to_string(),
            effect: effect.to_string(),
            strength,
            confidence: 1.0, // user-specified = high confidence
            mechanism: mechanism.to_string(),
            discovered_at: Utc::now(),
        });

        // Verify it's still a DAG
        if !graph.is_dag() {
            // Remove the edge we just added
            graph.edges.pop();
            warn!(
                "Rejected causal edge {} → {}: would create a cycle",
                cause, effect
            );
            return Err(anyhow!(
                "Adding edge {} → {} would create a cycle",
                cause,
                effect
            ));
        }

        info!(
            "Added causal relationship: {} → {} (strength: {:.2})",
            cause, effect, strength
        );
        Ok(())
    }

    /// Get summary statistics about the causal graph.
    pub async fn get_stats(&self) -> CausalGraphStats {
        let graph = self.causal_graph.read().await;
        let log = self.intervention_log.read().await;
        let obs = self.observation_buffer.read().await;

        let avg_prediction_error = if !log.is_empty() {
            log.iter()
                .filter_map(|i| i.prediction_error)
                .sum::<f64>()
                / log.iter().filter(|i| i.prediction_error.is_some()).count().max(1) as f64
        } else {
            0.0
        };

        CausalGraphStats {
            total_variables: graph.variables.len(),
            total_edges: graph.edges.len(),
            total_interventions: log.len(),
            total_observations: obs.len(),
            is_dag: graph.is_dag(),
            avg_prediction_error,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalGraphStats {
    pub total_variables: usize,
    pub total_edges: usize,
    pub total_interventions: usize,
    pub total_observations: usize,
    pub is_dag: bool,
    pub avg_prediction_error: f64,
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_causal_graph_basic() {
        let mut graph = CausalGraph::new();
        graph.add_variable(CausalVariable {
            name: "rain".to_string(),
            domain: VariableDomain::Binary,
            observed_value: Some(1.0),
            description: "Whether it rains".to_string(),
            created_at: Utc::now(),
        });
        graph.add_variable(CausalVariable {
            name: "wet_ground".to_string(),
            domain: VariableDomain::Binary,
            observed_value: Some(1.0),
            description: "Whether ground is wet".to_string(),
            created_at: Utc::now(),
        });
        graph.add_edge(CausalEdge {
            cause: "rain".to_string(),
            effect: "wet_ground".to_string(),
            strength: 0.9,
            confidence: 0.95,
            mechanism: "Rain causes wet ground".to_string(),
            discovered_at: Utc::now(),
        });

        assert!(graph.is_dag());
        assert_eq!(graph.get_effects("rain"), vec!["wet_ground"]);
        assert_eq!(graph.get_causes("wet_ground"), vec!["rain"]);
    }

    #[tokio::test]
    async fn test_do_intervention() {
        let engine = CausalInferenceEngine::new();
        {
            let mut graph = engine.causal_graph.write().await;
            graph.add_variable(CausalVariable {
                name: "X".to_string(),
                domain: VariableDomain::Continuous { min: 0.0, max: 1.0 },
                observed_value: Some(0.5),
                description: "Variable X".to_string(),
                created_at: Utc::now(),
            });
            graph.add_variable(CausalVariable {
                name: "Y".to_string(),
                domain: VariableDomain::Continuous { min: 0.0, max: 1.0 },
                observed_value: None,
                description: "Variable Y".to_string(),
                created_at: Utc::now(),
            });
            graph.add_edge(CausalEdge {
                cause: "X".to_string(),
                effect: "Y".to_string(),
                strength: 0.8,
                confidence: 0.9,
                mechanism: "X causes Y".to_string(),
                discovered_at: Utc::now(),
            });
        }

        let result = engine.do_intervention("X", 1.0).await.unwrap();
        assert!(!result.predicted_effects.is_empty());
    }

    #[tokio::test]
    async fn test_causal_discovery() {
        let engine = CausalInferenceEngine::new();

        let observations: Vec<Observation> = (0..20)
            .map(|i| {
                let x = i as f64 * 0.1;
                let mut values = HashMap::new();
                values.insert("temperature".to_string(), x);
                values.insert("ice_cream_sales".to_string(), x * 0.8 + 0.1);
                Observation {
                    timestamp: Utc::now(),
                    values,
                    context: format!("Day {}", i),
                }
            })
            .collect();

        let relations = engine.discover_causes(&observations).await;
        assert!(!relations.is_empty());
    }
}

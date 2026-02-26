use crate::housaky::cognitive::causal_inference::{CausalEdge, CausalInferenceEngine};
use crate::housaky::cognitive::world_model::{Action, Effect, EffectType, WorldModel, WorldState};
use crate::housaky::goal_engine::Goal;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

// ── SimulationTrace ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationTrace {
    pub id: String,
    pub initial_state: WorldState,
    pub steps: Vec<SimulationStep>,
    pub final_state: WorldState,
    pub total_reward: f64,
    pub total_confidence: f64,
    pub depth: usize,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationStep {
    pub action: Action,
    pub resulting_state: WorldState,
    pub reward: f64,
    pub confidence: f64,
    pub causal_explanations: Vec<String>,
}

impl SimulationTrace {
    pub fn new(initial_state: WorldState) -> Self {
        let final_state = initial_state.clone();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            initial_state,
            steps: Vec::new(),
            final_state,
            total_reward: 0.0,
            total_confidence: 1.0,
            depth: 0,
            created_at: Utc::now(),
        }
    }

    pub fn push_step(&mut self, step: SimulationStep) {
        self.total_reward += step.reward;
        self.total_confidence *= step.confidence;
        self.final_state = step.resulting_state.clone();
        self.depth += 1;
        self.steps.push(step);
    }
}

// ── MCTS ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct MctsNode {
    state: WorldState,
    action: Option<Action>,
    parent_idx: Option<usize>,
    children: Vec<usize>,
    visits: u64,
    total_value: f64,
    untried_actions: Vec<Action>,
}

impl MctsNode {
    fn new(
        state: WorldState,
        action: Option<Action>,
        parent_idx: Option<usize>,
        available_actions: Vec<Action>,
    ) -> Self {
        Self {
            state,
            action,
            parent_idx,
            children: Vec::new(),
            visits: 0,
            total_value: 0.0,
            untried_actions: available_actions,
        }
    }

    fn uct_value(&self, parent_visits: u64, exploration: f64) -> f64 {
        if self.visits == 0 {
            return f64::INFINITY;
        }
        let exploitation = self.total_value / self.visits as f64;
        let exploration_term =
            exploration * ((parent_visits as f64).ln() / self.visits as f64).sqrt();
        exploitation + exploration_term
    }
}

// ── WorldSimulator ────────────────────────────────────────────────────────────

pub struct WorldSimulator {
    pub world_model: Arc<WorldModel>,
    pub causal_graph: Arc<CausalInferenceEngine>,
    pub max_simulation_depth: usize,
    pub monte_carlo_samples: usize,
    pub mcts_exploration: f64,
}

impl WorldSimulator {
    pub fn new(
        world_model: Arc<WorldModel>,
        causal_graph: Arc<CausalInferenceEngine>,
    ) -> Self {
        Self {
            world_model,
            causal_graph,
            max_simulation_depth: 10,
            monte_carlo_samples: 100,
            mcts_exploration: 1.41, // sqrt(2)
        }
    }

    pub fn with_config(
        world_model: Arc<WorldModel>,
        causal_graph: Arc<CausalInferenceEngine>,
        max_depth: usize,
        mc_samples: usize,
    ) -> Self {
        Self {
            world_model,
            causal_graph,
            max_simulation_depth: max_depth,
            monte_carlo_samples: mc_samples,
            mcts_exploration: 1.41,
        }
    }

    /// Simulate an action sequence and return a distribution of possible outcomes.
    pub async fn simulate(
        &self,
        initial_state: &WorldState,
        actions: &[Action],
    ) -> Vec<SimulationTrace> {
        let mut traces = Vec::new();
        let n_samples = self.monte_carlo_samples.min(50); // cap for performance

        for sample_idx in 0..n_samples {
            let mut trace = SimulationTrace::new(initial_state.clone());
            for action in actions.iter().take(self.max_simulation_depth) {
                let outcome = self.world_model.predict(action).await;

                // Add causal explanations from the causal graph
                let causal_explanations: Vec<String> = {
                    let graph = self.causal_graph.causal_graph.read().await;
                    graph
                        .edges
                        .iter()
                        .filter(|e| e.cause == action.action_type || e.effect == action.action_type)
                        .map(|e: &CausalEdge| {
                            format!("{} → {} (strength {:.2})", e.cause, e.effect, e.strength)
                        })
                        .collect()
                };

                // Inject slight stochasticity via sample index perturbation
                let noise = (sample_idx as f64 * 0.618033988749895) % 0.1;
                let confidence = (outcome.confidence - noise).max(0.0);
                let reward = outcome.reward * (1.0 - noise * 0.5);

                trace.push_step(SimulationStep {
                    action: action.clone(),
                    resulting_state: outcome.state.clone(),
                    reward,
                    confidence,
                    causal_explanations,
                });

                if trace.total_confidence < 0.01 {
                    break; // prune low-confidence paths
                }
            }

            traces.push(trace);
        }

        traces.sort_by(|a, b| b.total_reward.partial_cmp(&a.total_reward).unwrap_or(std::cmp::Ordering::Equal));
        info!(
            "Simulation: {} traces, best reward={:.4}",
            traces.len(),
            traces.first().map(|t| t.total_reward).unwrap_or(0.0)
        );
        traces
    }

    /// Monte Carlo Tree Search for optimal action sequence to achieve `goal`.
    pub async fn mcts_plan(
        &self,
        state: &WorldState,
        goal: &Goal,
        budget_ms: u64,
    ) -> Vec<Action> {
        let start = std::time::Instant::now();
        let budget = std::time::Duration::from_millis(budget_ms);

        let root_actions = self.get_candidate_actions(state, goal);
        let root = MctsNode::new(state.clone(), None, None, root_actions);
        let mut tree: Vec<MctsNode> = vec![root];

        let mut iteration = 0u64;
        while start.elapsed() < budget {
            // 1. Selection: traverse tree using UCT
            let leaf_idx = self.select_leaf(&tree);

            // 2. Expansion: expand one untried action
            let expanded_idx = self.expand(&mut tree, leaf_idx, goal).await;

            // 3. Simulation (rollout): fast random rollout from expanded node
            let value = self.rollout(&tree, expanded_idx, goal).await;

            // 4. Backpropagation
            self.backpropagate(&mut tree, expanded_idx, value);

            iteration += 1;
        }

        info!(
            "MCTS: {} iterations in {}ms, tree size={}",
            iteration,
            start.elapsed().as_millis(),
            tree.len()
        );

        // Extract best action sequence from most-visited path
        self.extract_best_path(&tree)
    }

    /// Counterfactual reasoning: what would have happened if `alternative_action` had been taken at `at_step`?
    pub async fn counterfactual(
        &self,
        actual_trace: &SimulationTrace,
        alternative_action: &Action,
        at_step: usize,
    ) -> SimulationTrace {
        let initial_state = if at_step == 0 {
            actual_trace.initial_state.clone()
        } else {
            actual_trace
                .steps
                .get(at_step.saturating_sub(1))
                .map(|s| s.resulting_state.clone())
                .unwrap_or(actual_trace.initial_state.clone())
        };

        // Build counterfactual action sequence
        let mut cf_actions = vec![alternative_action.clone()];
        cf_actions.extend(
            actual_trace.steps[at_step.min(actual_trace.steps.len())..]
                .iter()
                .map(|s| s.action.clone()),
        );

        let cf_traces = self.simulate(&initial_state, &cf_actions).await;

        cf_traces
            .into_iter()
            .next()
            .unwrap_or_else(|| SimulationTrace::new(initial_state))
    }

    // ── Internal MCTS helpers ─────────────────────────────────────────────────

    fn select_leaf(&self, tree: &[MctsNode]) -> usize {
        let mut idx = 0;
        loop {
            let node = &tree[idx];
            if !node.untried_actions.is_empty() || node.children.is_empty() {
                return idx;
            }
            // Pick child with highest UCT value
            let parent_visits = node.visits;
            let best_child = node
                .children
                .iter()
                .map(|&c| (c, tree[c].uct_value(parent_visits, self.mcts_exploration)))
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            match best_child {
                Some((child_idx, _)) => idx = child_idx,
                None => return idx,
            }
        }
    }

    async fn expand(
        &self,
        tree: &mut Vec<MctsNode>,
        leaf_idx: usize,
        goal: &Goal,
    ) -> usize {
        if tree[leaf_idx].untried_actions.is_empty() {
            return leaf_idx;
        }

        let action = tree[leaf_idx].untried_actions.remove(0);
        let outcome = self.world_model.predict(&action).await;
        let child_state = outcome.state;
        let child_actions = self.get_candidate_actions(&child_state, goal);

        let child_node = MctsNode::new(
            child_state,
            Some(action),
            Some(leaf_idx),
            child_actions,
        );

        let child_idx = tree.len();
        tree.push(child_node);
        tree[leaf_idx].children.push(child_idx);
        child_idx
    }

    async fn rollout(
        &self,
        tree: &[MctsNode],
        from_idx: usize,
        goal: &Goal,
    ) -> f64 {
        let mut state = tree[from_idx].state.clone();
        let mut total_reward = 0.0;

        for depth in 0..5 {
            // fast rollout: pick first available action
            let actions = self.get_candidate_actions(&state, goal);
            if actions.is_empty() {
                break;
            }
            let action = &actions[depth % actions.len()];
            let outcome = self.world_model.predict(action).await;
            total_reward += outcome.reward * outcome.confidence;
            state = outcome.state;
        }

        // Goal proximity bonus
        total_reward += self.goal_proximity(&state, goal);
        total_reward
    }

    fn backpropagate(&self, tree: &mut Vec<MctsNode>, from_idx: usize, value: f64) {
        let mut idx = Some(from_idx);
        while let Some(i) = idx {
            tree[i].visits += 1;
            tree[i].total_value += value;
            idx = tree[i].parent_idx;
        }
    }

    fn extract_best_path(&self, tree: &[MctsNode]) -> Vec<Action> {
        let mut actions = Vec::new();
        let mut idx = 0;

        loop {
            let node = &tree[idx];
            if node.children.is_empty() {
                break;
            }
            // Follow most-visited child
            let best = node
                .children
                .iter()
                .max_by_key(|&&c| tree[c].visits);
            match best {
                Some(&child_idx) => {
                    if let Some(ref action) = tree[child_idx].action {
                        actions.push(action.clone());
                    }
                    idx = child_idx;
                }
                None => break,
            }
            if actions.len() >= self.max_simulation_depth {
                break;
            }
        }
        actions
    }

    fn get_candidate_actions(&self, state: &WorldState, goal: &Goal) -> Vec<Action> {
        let mut actions = Vec::new();

        // Resource-based actions
        if state.resources.get("cpu").unwrap_or(&0.0) > &0.1 {
            actions.push(Action {
                id: uuid::Uuid::new_v4().to_string(),
                action_type: "compute".to_string(),
                parameters: HashMap::new(),
                preconditions: vec![],
                expected_effects: vec![Effect {
                    effect_type: EffectType::ResourceChange,
                    target: "cpu".to_string(),
                    value: serde_json::json!(-0.1),
                }],
                estimated_duration_ms: 100,
                risk_level: 0.1,
            });
        }

        if state.resources.get("network").unwrap_or(&0.0) > &0.0 {
            actions.push(Action {
                id: uuid::Uuid::new_v4().to_string(),
                action_type: "fetch_knowledge".to_string(),
                parameters: [("goal".to_string(), serde_json::json!(goal.id))]
                    .into_iter()
                    .collect(),
                preconditions: vec![],
                expected_effects: vec![Effect {
                    effect_type: EffectType::StateChange,
                    target: "knowledge_level".to_string(),
                    value: serde_json::json!("increased"),
                }],
                estimated_duration_ms: 500,
                risk_level: 0.2,
            });
        }

        actions.push(Action {
            id: uuid::Uuid::new_v4().to_string(),
            action_type: "reason".to_string(),
            parameters: [("goal".to_string(), serde_json::json!(goal.id))]
                .into_iter()
                .collect(),
            preconditions: vec![],
            expected_effects: vec![Effect {
                effect_type: EffectType::StateChange,
                target: "reasoning_depth".to_string(),
                value: serde_json::json!("increased"),
            }],
            estimated_duration_ms: 200,
            risk_level: 0.05,
        });

        actions
    }

    fn goal_proximity(&self, state: &WorldState, goal: &Goal) -> f64 {
        // Heuristic: check how many goal-related keys are set in state context
        let goal_keywords: Vec<&str> = goal.description.split_whitespace().take(5).collect();
        let matches = goal_keywords
            .iter()
            .filter(|kw| {
                state
                    .context
                    .keys()
                    .any(|k| k.contains(&***kw))
            })
            .count();
        matches as f64 / goal_keywords.len().max(1) as f64
    }
}

// ── CausalSimulationPipeline: wires causal_inference → world_model → consequence_predictor ──

pub struct CausalSimulationPipeline {
    pub simulator: WorldSimulator,
    pub world_model: Arc<WorldModel>,
    pub causal_engine: Arc<CausalInferenceEngine>,
}

impl CausalSimulationPipeline {
    pub fn new(
        world_model: Arc<WorldModel>,
        causal_engine: Arc<CausalInferenceEngine>,
    ) -> Self {
        let simulator = WorldSimulator::new(
            Arc::clone(&world_model),
            Arc::clone(&causal_engine),
        );
        Self {
            simulator,
            world_model,
            causal_engine,
        }
    }

    /// Full causal planning pipeline:
    /// 1. Query causal graph for relevant relationships
    /// 2. Translate causal edges into action candidates
    /// 3. MCTS plan over causal action space
    /// 4. Return optimal action sequence with causal justification
    pub async fn causal_plan(
        &self,
        goal: &Goal,
        budget_ms: u64,
    ) -> Result<CausalPlan> {
        let state = self.world_model.get_current_state().await;

        // Pull causal relationships from the graph
        let causal_edges: Vec<CausalEdge> = {
            let graph = self.causal_engine.causal_graph.read().await;
            graph.edges.clone()
        };

        // MCTS over the world model
        let actions = self
            .simulator
            .mcts_plan(&state, goal, budget_ms)
            .await;

        let justifications: Vec<String> = causal_edges
            .iter()
            .map(|e| {
                format!(
                    "Causal: {} → {} (strength={:.2}, confidence={:.2})",
                    e.cause, e.effect, e.strength, e.confidence
                )
            })
            .collect();

        info!(
            "Causal plan for '{}': {} actions, {} causal justifications",
            goal.description,
            actions.len(),
            justifications.len()
        );

        let n_edges = causal_edges.len();
        let confidence = if n_edges == 0 { 0.5 } else { 0.8 };

        Ok(CausalPlan {
            goal_id: goal.id.clone(),
            actions,
            causal_justifications: justifications,
            confidence,
            created_at: Utc::now(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalPlan {
    pub goal_id: String,
    pub actions: Vec<Action>,
    pub causal_justifications: Vec<String>,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
}

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

use super::world_model::{Action, WorldModel, WorldState};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub actions: Vec<PlannedAction>,
    pub goal_state: GoalState,
    pub estimated_reward: f64,
    pub confidence: f64,
    pub created_at: std::time::SystemTime,
    pub status: PlanStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlannedAction {
    pub action: Action,
    pub reasoning: String,
    pub alternatives: Vec<Action>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoalState {
    pub target_properties: HashMap<String, String>,
    pub constraints: Vec<String>,
    pub priority: GoalPriority,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum GoalPriority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlanStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Aborted,
}

pub struct PlanningEngine {
    world_model: Arc<WorldModel>,
    max_iterations: usize,
    exploration_constant: f64,
}

impl PlanningEngine {
    pub fn new(world_model: Arc<WorldModel>) -> Self {
        Self {
            world_model,
            max_iterations: 1000,
            exploration_constant: 1.414,
        }
    }

    pub async fn plan(&self, goal: &GoalState, max_depth: usize) -> Result<Plan> {
        info!("Planning to achieve goal with priority {:?}", goal.priority);

        let initial_state = self.world_model.get_current_state().await;

        let paths = self
            .world_model
            .simulate(&self.generate_possible_actions(&initial_state), max_depth)
            .await;

        let best_path = paths.into_iter().max_by(|a, b| {
            let a_score = a.total_reward * a.confidence;
            let b_score = b.total_reward * b.confidence;
            a_score
                .partial_cmp(&b_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if let Some(path) = best_path {
            let planned_actions: Vec<PlannedAction> = path
                .actions
                .into_iter()
                .map(|action| PlannedAction {
                    reasoning: format!("Action {} leads to reward", action.action_type),
                    alternatives: vec![],
                    action,
                })
                .collect();

            return Ok(Plan {
                id: uuid::Uuid::new_v4().to_string(),
                actions: planned_actions,
                goal_state: goal.clone(),
                estimated_reward: path.total_reward,
                confidence: path.confidence,
                created_at: std::time::SystemTime::now(),
                status: PlanStatus::Pending,
            });
        }

        anyhow::bail!("No valid plan found")
    }

    fn generate_possible_actions(&self, _state: &WorldState) -> Vec<Action> {
        let mut actions = vec![];

        actions.push(Action {
            id: "search".to_string(),
            action_type: "search".to_string(),
            parameters: HashMap::new(),
            preconditions: vec![],
            expected_effects: vec![],
            estimated_duration_ms: 1000,
            risk_level: 0.1,
        });

        actions.push(Action {
            id: "read".to_string(),
            action_type: "read".to_string(),
            parameters: HashMap::new(),
            preconditions: vec![],
            expected_effects: vec![],
            estimated_duration_ms: 500,
            risk_level: 0.1,
        });

        actions.push(Action {
            id: "write".to_string(),
            action_type: "write".to_string(),
            parameters: HashMap::new(),
            preconditions: vec![],
            expected_effects: vec![],
            estimated_duration_ms: 500,
            risk_level: 0.2,
        });

        actions.push(Action {
            id: "execute".to_string(),
            action_type: "execute".to_string(),
            parameters: HashMap::new(),
            preconditions: vec![],
            expected_effects: vec![],
            estimated_duration_ms: 1000,
            risk_level: 0.5,
        });

        actions.push(Action {
            id: "ask".to_string(),
            action_type: "ask".to_string(),
            parameters: HashMap::new(),
            preconditions: vec![],
            expected_effects: vec![],
            estimated_duration_ms: 2000,
            risk_level: 0.1,
        });

        actions
    }

    pub async fn plan_with_mcts(&self, goal: &GoalState) -> Result<Plan> {
        info!("Running MCTS planning");

        let initial_state = self.world_model.get_current_state().await;
        let mut root = MCTSNode::new(initial_state, None);

        for iteration in 0..self.max_iterations {
            let node = &mut root;

            let mut path = vec![node.state.clone()];
            let mut depth = 0;

            while !self.is_terminal(&node.state, goal) && depth < 10 {
                if node.untried_actions.is_empty() && node.children.is_empty() {
                    break;
                }

                if let Some(child_action) = node.untried_actions.pop() {
                    let outcome = self.world_model.predict(&child_action).await;
                    let child = MCTSNode::new(outcome.state.clone(), Some(child_action));
                    node.children.push(Box::new(child));
                    if let Some(last_child) = node.children.last() {
                        path.push(last_child.state.clone());
                        depth += 1;
                    }
                } else if let Some(child) = node.select_child(self.exploration_constant) {
                    path.push(child.state.clone());
                    depth += 1;
                } else {
                    break;
                }
            }

            if !self.is_terminal(&node.state, goal) {
                let actions = self.generate_possible_actions(&node.state);
                if !actions.is_empty() {
                    node.untried_actions = actions;
                }
            }

            if depth < 10 {
                if let Some(action) = node.untried_actions.pop() {
                    let outcome = self.world_model.predict(&action).await;
                    let child = MCTSNode::new(outcome.state.clone(), Some(action));
                    node.children.push(Box::new(child));
                }
            }

            let reward = self.evaluate_state(path.last().unwrap(), goal);
            self.backpropagate(&mut root, reward);

            if iteration % 100 == 0 {
                info!("MCTS iteration {}", iteration);
            }
        }

        let best_action = root
            .children
            .iter()
            .max_by(|a, b| {
                a.visits
                    .partial_cmp(&b.visits)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .and_then(|c| c.action.clone());

        if let Some(action) = best_action {
            return Ok(Plan {
                id: uuid::Uuid::new_v4().to_string(),
                actions: vec![PlannedAction {
                    reasoning: "Selected by MCTS".to_string(),
                    alternatives: vec![],
                    action,
                }],
                goal_state: goal.clone(),
                estimated_reward: root.total_reward / root.visits as f64,
                confidence: (root.visits as f64 / self.max_iterations as f64).min(1.0),
                created_at: std::time::SystemTime::now(),
                status: PlanStatus::Pending,
            });
        }

        anyhow::bail!("MCTS failed to find a plan")
    }

    fn is_terminal(&self, state: &WorldState, goal: &GoalState) -> bool {
        for (key, value) in &goal.target_properties {
            if state.context.get(key) != Some(&value.to_string()) {
                return false;
            }
        }
        true
    }

    fn evaluate_state(&self, state: &WorldState, goal: &GoalState) -> f64 {
        let mut score = 0.0;

        for (key, value) in &goal.target_properties {
            if state.context.get(key) == Some(&value.to_string()) {
                score += 1.0;
            }
        }

        score
    }

    #[allow(clippy::self_only_used_in_recursion)]
    fn backpropagate(&self, node: &mut MCTSNode, reward: f64) {
        node.visits += 1;
        node.total_reward += reward;

        for child in &mut node.children {
            self.backpropagate(child, reward);
        }
    }

    pub async fn refine_plan(&self, plan: &Plan, feedback: &str) -> Result<Plan> {
        info!("Refining plan based on feedback: {}", feedback);

        let _initial_state = self.world_model.get_current_state().await;

        let refined_actions: Vec<PlannedAction> = plan
            .actions
            .iter()
            .enumerate()
            .map(|(i, pa)| {
                if i == 0 {
                    PlannedAction {
                        reasoning: format!("Refined: {}", feedback),
                        alternatives: vec![],
                        action: pa.action.clone(),
                    }
                } else {
                    pa.clone()
                }
            })
            .collect();

        Ok(Plan {
            id: uuid::Uuid::new_v4().to_string(),
            actions: refined_actions,
            goal_state: plan.goal_state.clone(),
            estimated_reward: plan.estimated_reward,
            confidence: plan.confidence * 0.9,
            created_at: std::time::SystemTime::now(),
            status: PlanStatus::Pending,
        })
    }
}

struct MCTSNode {
    state: WorldState,
    action: Option<Action>,
    children: Vec<Box<MCTSNode>>,
    untried_actions: Vec<Action>,
    visits: u64,
    total_reward: f64,
}

impl MCTSNode {
    fn new(state: WorldState, action: Option<Action>) -> Self {
        Self {
            state,
            action,
            children: Vec::new(),
            untried_actions: Vec::new(),
            visits: 0,
            total_reward: 0.0,
        }
    }

    fn select_child(&mut self, exploration_constant: f64) -> Option<&mut Box<MCTSNode>> {
        if self.children.is_empty() {
            return None;
        }

        let parent_visits = self.visits;
        let mut best_idx = 0;
        let mut best_ucb = f64::NEG_INFINITY;

        for (idx, child) in self.children.iter().enumerate() {
            let ucb = if child.visits == 0 {
                f64::MAX
            } else {
                let exploitation = child.total_reward / child.visits as f64;
                let exploration = exploration_constant
                    * ((parent_visits as f64).ln() / child.visits as f64).sqrt();
                exploitation + exploration
            };

            if ucb > best_ucb {
                best_ucb = ucb;
                best_idx = idx;
            }
        }

        self.children.get_mut(best_idx)
    }

    fn ucb(&self, child: &MCTSNode, exploration_constant: f64) -> f64 {
        if child.visits == 0 {
            return f64::MAX;
        }

        let exploitation = child.total_reward / child.visits as f64;
        let exploration =
            exploration_constant * ((self.visits as f64).ln() / child.visits as f64).sqrt();

        exploitation + exploration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_planning() {
        let world_model = Arc::new(WorldModel::new());
        let planner = PlanningEngine::new(world_model);

        let goal = GoalState {
            target_properties: HashMap::new(),
            constraints: vec![],
            priority: GoalPriority::High,
        };

        let plan = planner.plan(&goal, 3).await;
        assert!(plan.is_ok());
    }
}

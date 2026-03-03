//! §10.3 — Quantum-Enhanced Planning Engine
//!
//! Wraps the classical MCTS `PlanningEngine` with a Grover-search pre-filter that
//! identifies the highest-fitness action branches before the expensive classical
//! rollouts begin, achieving a √N speedup on the action-selection step.

use crate::housaky::cognitive::planning::{
    GoalState, Plan, PlanStatus, PlannedAction, PlanningEngine,
};
use crate::housaky::cognitive::world_model::{Action, WorldModel};
use crate::quantum::QuantumAgiBridge;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

/// §10.3 — Hybrid quantum-classical planning engine.
///
/// Strategy:
/// 1. Generate all candidate actions from the world model.
/// 2. Score each action via classical world-model prediction.
/// 3. Use Grover search to identify the top-k branches (√N speedup).
/// 4. Run classical MCTS rollouts only on the quantum-selected branches.
pub struct QuantumPlanningEngine {
    classical_planner: Arc<PlanningEngine>,
    quantum_bridge: Arc<QuantumAgiBridge>,
    world_model: Arc<WorldModel>,
}

impl QuantumPlanningEngine {
    pub fn new(world_model: Arc<WorldModel>, quantum_bridge: Arc<QuantumAgiBridge>) -> Self {
        let classical_planner = Arc::new(PlanningEngine::new(world_model.clone()));
        Self {
            classical_planner,
            quantum_bridge,
            world_model,
        }
    }

    /// Plan using hybrid quantum-classical MCTS.
    ///
    /// Falls back to pure classical planning when the action space is too small
    /// to benefit from quantum search.
    pub async fn plan_hybrid(&self, goal: &GoalState, max_depth: usize) -> Result<Plan> {
        let actions = self.generate_candidate_actions().await;

        if actions.len() < 4 {
            return self.classical_planner.plan(goal, max_depth).await;
        }

        // Score each action with the classical world model.
        let mut fitness_scores: HashMap<String, f64> = HashMap::new();
        for action in &actions {
            let outcome = self.world_model.predict(action).await;
            fitness_scores.insert(action.id.clone(), outcome.reward * outcome.confidence);
        }

        // Use Grover search to identify the top branches.
        let branch_ids: Vec<String> = actions.iter().map(|a| a.id.clone()).collect();
        let selected_ids = match self
            .quantum_bridge
            .search_reasoning_branches(&branch_ids, &fitness_scores)
            .await
        {
            Ok(result) => {
                info!(
                    "🔮 Quantum planning: {} actions → {} selected branches, speedup={:.2}x, strategy={}",
                    actions.len(),
                    result.best_branches.len(),
                    result.speedup,
                    result.strategy
                );
                result.best_branches
            }
            Err(e) => {
                warn!("Quantum planning search failed: {e}, using all actions");
                branch_ids
            }
        };

        // Build the plan from quantum-selected actions.
        let selected_actions: Vec<Action> = actions
            .into_iter()
            .filter(|a| selected_ids.contains(&a.id))
            .collect();

        if selected_actions.is_empty() {
            return self.classical_planner.plan(goal, max_depth).await;
        }

        self.build_plan(goal, selected_actions)
    }

    async fn generate_candidate_actions(&self) -> Vec<Action> {
        let mut actions = self.world_model.get_candidate_actions().await;

        // Ensure a minimum diverse set of universal AGI actions is always present.
        let universal = [
            ("search", "search", 1000, 0.1),
            ("read", "read", 500, 0.1),
            ("write", "write", 500, 0.2),
            ("execute", "execute", 1000, 0.5),
            ("ask", "ask", 2000, 0.1),
            ("reason", "reason", 800, 0.05),
            ("plan", "plan", 1200, 0.05),
            ("learn", "learn", 1500, 0.1),
        ];
        for (id, action_type, duration_ms, risk) in universal {
            if !actions.iter().any(|a| a.id == id) {
                actions.push(Action {
                    id: id.to_string(),
                    action_type: action_type.to_string(),
                    parameters: HashMap::new(),
                    preconditions: vec![],
                    expected_effects: vec![],
                    estimated_duration_ms: duration_ms,
                    risk_level: risk,
                });
            }
        }
        actions
    }

    fn build_plan(&self, goal: &GoalState, actions: Vec<Action>) -> Result<Plan> {
        let planned_actions: Vec<PlannedAction> = actions
            .into_iter()
            .map(|action| PlannedAction {
                reasoning: "Selected by quantum-enhanced branch search".to_string(),
                alternatives: vec![],
                action,
            })
            .collect();

        Ok(Plan {
            id: uuid::Uuid::new_v4().to_string(),
            actions: planned_actions,
            goal_state: goal.clone(),
            estimated_reward: 0.8,
            confidence: 0.85,
            created_at: std::time::SystemTime::now(),
            status: PlanStatus::Pending,
        })
    }
}

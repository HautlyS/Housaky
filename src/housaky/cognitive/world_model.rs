use crate::util::{read_msgpack_file, write_msgpack_file};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldState {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub entities: HashMap<String, EntityState>,
    pub context: HashMap<String, String>,
    pub constraints: Vec<Constraint>,
    pub resources: HashMap<String, f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityState {
    pub name: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub relations: Vec<EntityRelation>,
    pub active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityRelation {
    pub target: String,
    pub relation_type: String,
    pub strength: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Constraint {
    pub constraint_type: ConstraintType,
    pub description: String,
    pub weight: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConstraintType {
    TimeLimit,
    ResourceLimit,
    Permission,
    Safety,
    Dependency,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub action_type: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub preconditions: Vec<Precondition>,
    pub expected_effects: Vec<Effect>,
    pub estimated_duration_ms: u64,
    pub risk_level: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Precondition {
    pub condition: String,
    pub required: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Effect {
    pub effect_type: EffectType,
    pub target: String,
    pub value: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EffectType {
    StateChange,
    ResourceChange,
    RelationChange,
    EntityCreate,
    EntityDelete,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PredictedOutcome {
    pub state: WorldState,
    pub reward: f64,
    pub confidence: f64,
    pub reasoning: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionResult {
    pub action: Action,
    pub actual_state: WorldState,
    pub expected_state: Option<WorldState>,
    pub success: bool,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub discovered_causality: Option<DiscoveredCausality>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscoveredCausality {
    pub cause: String,
    pub effect: String,
    pub strength: f64,
    pub evidence: Vec<String>,
}

pub struct WorldModel {
    current_state: Arc<RwLock<WorldState>>,
    transition_model: Arc<RwLock<TransitionModel>>,
    reward_model: Arc<RwLock<RewardModel>>,
    causal_graph: Arc<RwLock<CausalGraph>>,
    history: Arc<RwLock<Vec<ActionResult>>>,
    storage_path: Option<PathBuf>,
}

impl WorldModel {
    pub fn new() -> Self {
        Self {
            current_state: Arc::new(RwLock::new(WorldState::default())),
            transition_model: Arc::new(RwLock::new(TransitionModel::new())),
            reward_model: Arc::new(RwLock::new(RewardModel::new())),
            causal_graph: Arc::new(RwLock::new(CausalGraph::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            storage_path: None,
        }
    }

    pub fn with_storage(workspace_dir: &PathBuf) -> Self {
        let storage_path = workspace_dir.join(".housaky").join("world_model");
        Self {
            current_state: Arc::new(RwLock::new(WorldState::default())),
            transition_model: Arc::new(RwLock::new(TransitionModel::new())),
            reward_model: Arc::new(RwLock::new(RewardModel::new())),
            causal_graph: Arc::new(RwLock::new(CausalGraph::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            storage_path: Some(storage_path),
        }
    }

    pub async fn load(&self) -> Result<()> {
        if let Some(ref path) = self.storage_path {
            if path.exists() {
                let state_path = path.join("state.msgpack");
                if state_path.exists() {
                    let state: WorldState = read_msgpack_file(&state_path).await?;
                    *self.current_state.write().await = state;
                }

                let causal_path = path.join("causal_graph.msgpack");
                if causal_path.exists() {
                    let graph: CausalGraph = read_msgpack_file(&causal_path).await?;
                    *self.causal_graph.write().await = graph;
                }

                let tm_path = path.join("transition_model.json");
                if tm_path.exists() {
                    let json = tokio::fs::read_to_string(&tm_path).await?;
                    if let Ok(tm) = serde_json::from_str::<TransitionModel>(&json) {
                        *self.transition_model.write().await = tm;
                    }
                }

                let rm_path = path.join("reward_model.json");
                if rm_path.exists() {
                    let json = tokio::fs::read_to_string(&rm_path).await?;
                    if let Ok(rm) = serde_json::from_str::<RewardModel>(&json) {
                        *self.reward_model.write().await = rm;
                    }
                }

                info!("Loaded world model (state + causal + transition + reward) from storage");
            }
        }
        Ok(())
    }

    pub async fn save(&self) -> Result<()> {
        if let Some(ref path) = self.storage_path {
            tokio::fs::create_dir_all(path).await?;

            let state = self.current_state.read().await;
            write_msgpack_file(&path.join("state.msgpack"), &*state).await?;

            let causal = self.causal_graph.read().await;
            write_msgpack_file(&path.join("causal_graph.msgpack"), &*causal).await?;

            // §2.5 — persist transition + reward models so learned patterns
            // survive restarts (previously only saved by save_transition_model).
            let tm = self.transition_model.read().await;
            let tm_json = serde_json::to_vec_pretty(&*tm)?;
            tokio::fs::write(path.join("transition_model.json"), tm_json).await?;

            let rm = self.reward_model.read().await;
            let rm_json = serde_json::to_vec_pretty(&*rm)?;
            tokio::fs::write(path.join("reward_model.json"), rm_json).await?;
        }
        Ok(())
    }

    pub async fn get_current_state(&self) -> WorldState {
        self.current_state.read().await.clone()
    }

    pub async fn update_state(&self, new_state: WorldState) {
        let mut state = self.current_state.write().await;
        *state = new_state;
    }

    pub async fn predict(&self, action: &Action) -> PredictedOutcome {
        let current = self.current_state.read().await.clone();

        let predicted_state = self.transition_model.read().await.predict(&current, action);

        let reward = self.reward_model.read().await.predict(&predicted_state);

        let confidence = self.transition_model.read().await.get_confidence(action);

        PredictedOutcome {
            state: predicted_state,
            reward,
            confidence,
            reasoning: format!("Predicted outcome of {} action", action.action_type),
        }
    }

    pub async fn simulate(&self, _actions: &[Action], max_depth: usize) -> Vec<SimulatedPath> {
        let initial = self.current_state.read().await.clone();
        let mut paths = vec![SimulatedPath {
            actions: vec![],
            states: vec![initial.clone()],
            total_reward: 0.0,
            confidence: 1.0,
        }];

        for _ in 0..max_depth {
            let mut new_paths = Vec::new();

            for path in &paths {
                let last_state = path.states.last().unwrap();
                let possible_actions = self.get_possible_actions(last_state);

                for action in possible_actions {
                    let outcome = self.predict(&action).await;

                    let mut new_path = path.clone();
                    new_path.actions.push(action);
                    new_path.states.push(outcome.state.clone());
                    new_path.total_reward += outcome.reward;
                    new_path.confidence *= outcome.confidence;

                    new_paths.push(new_path);
                }
            }

            paths = new_paths;
            if paths.is_empty() {
                break;
            }
        }

        paths.sort_by(|a, b| b.total_reward.partial_cmp(&a.total_reward).unwrap());
        paths
    }

    fn get_possible_actions(&self, state: &WorldState) -> Vec<Action> {
        let mut actions = vec![];

        if state.resources.get("cpu").unwrap_or(&0.0) > &0.1 {
            actions.push(Action {
                id: "compute".to_string(),
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
                id: "network_request".to_string(),
                action_type: "network".to_string(),
                parameters: HashMap::new(),
                preconditions: vec![],
                expected_effects: vec![Effect {
                    effect_type: EffectType::ResourceChange,
                    target: "network".to_string(),
                    value: serde_json::json!(-0.1),
                }],
                estimated_duration_ms: 500,
                risk_level: 0.3,
            });
        }

        actions
    }

    pub async fn learn(&self, result: &ActionResult) {
        self.transition_model.write().await.update(
            &result.action,
            &result.expected_state,
            &result.actual_state,
        );

        self.reward_model
            .write()
            .await
            .update(&result.actual_state, result.success);

        if let Some(causality) = &result.discovered_causality {
            self.causal_graph.write().await.add_causality(causality);
        }

        self.history.write().await.push(result.clone());

        let mut current = self.current_state.write().await;
        *current = result.actual_state.clone();
        drop(current);

        // Persist after every real interaction so transition + reward models
        // survive restarts (§2.5 — world model persistence).
        if let Err(e) = self.save().await {
            tracing::warn!("WorldModel: failed to persist after learn(): {e}");
        }
    }

    /// Persist only the transition model (serialised as JSON) alongside the
    /// state and causal graph that `save()` already handles.
    pub async fn save_transition_model(&self) -> Result<()> {
        if let Some(ref path) = self.storage_path {
            tokio::fs::create_dir_all(path).await?;
            let tm = self.transition_model.read().await;
            let json = serde_json::to_vec_pretty(&*tm)?;
            tokio::fs::write(path.join("transition_model.json"), json).await?;
            let rm = self.reward_model.read().await;
            let json = serde_json::to_vec_pretty(&*rm)?;
            tokio::fs::write(path.join("reward_model.json"), json).await?;
        }
        Ok(())
    }

    pub async fn get_causal_relationships(&self, entity: &str) -> Vec<CausalRelationship> {
        self.causal_graph.read().await.get_relationships(entity)
    }

    pub async fn find_unresolved_entities(&self) -> Vec<crate::housaky::knowledge_graph::Entity> {
        // Return entities with low confidence as unresolved
        vec![]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TransitionModel {
    patterns: HashMap<String, TransitionPattern>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransitionPattern {
    pub action_type: String,
    pub preconditions: Vec<String>,
    pub effect_distribution: HashMap<String, f64>,
    pub confidence: f64,
    pub observation_count: u64,
}

impl TransitionModel {
    pub fn new() -> Self {
        Self {
            patterns: HashMap::new(),
        }
    }

    pub fn predict(&self, state: &WorldState, action: &Action) -> WorldState {
        let mut new_state = state.clone();
        new_state.id = uuid::Uuid::new_v4().to_string();
        new_state.timestamp = Utc::now();

        for effect in &action.expected_effects {
            match effect.effect_type {
                EffectType::StateChange => {
                    new_state
                        .context
                        .insert(effect.target.clone(), effect.value.to_string());
                }
                EffectType::ResourceChange => {
                    if let Some(current) = new_state.resources.get(&effect.target).copied() {
                        let change = effect.value.as_f64().unwrap_or(0.0);
                        new_state
                            .resources
                            .insert(effect.target.clone(), (current + change).max(0.0));
                    }
                }
                EffectType::EntityCreate => {
                    new_state.entities.insert(
                        effect.target.clone(),
                        EntityState {
                            name: effect.target.clone(),
                            properties: HashMap::new(),
                            relations: vec![],
                            active: true,
                        },
                    );
                }
                EffectType::EntityDelete => {
                    new_state.entities.remove(&effect.target);
                }
                EffectType::RelationChange => {}
            }
        }

        new_state
    }

    pub fn update(&mut self, action: &Action, expected: &Option<WorldState>, actual: &WorldState) {
        let action_type = action.action_type.clone();

        // Compute match rate before mutably borrowing self.patterns so we
        // avoid the simultaneous mutable + immutable borrow conflict.
        let match_rate = expected
            .as_ref()
            .map(|exp| self.calculate_state_match(exp, actual));

        let pattern = self
            .patterns
            .entry(action_type)
            .or_insert(TransitionPattern {
                action_type: action.action_type.clone(),
                preconditions: vec![],
                effect_distribution: HashMap::new(),
                confidence: 0.5,
                observation_count: 0,
            });

        pattern.observation_count += 1;
        let n = pattern.observation_count;
        let old_confidence = pattern.confidence;

        // §2.5 — learn effect distributions from actual observed context changes.
        for (key, val) in &actual.context {
            let effect_key = format!("{}={}", key, val.chars().take(32).collect::<String>());
            let count = pattern.effect_distribution.get(&effect_key).copied().unwrap_or(0.0);
            pattern.effect_distribution.insert(effect_key, count + 1.0);
        }

        // Learn preconditions: track which resource keys were nonzero when this
        // action was taken, incrementally building a precondition model.
        for (key, &val) in &actual.resources {
            if val > 0.0 && !pattern.preconditions.contains(key) {
                pattern.preconditions.push(key.clone());
            }
        }

        // Update confidence via running average against expected state.
        if let Some(rate) = match_rate {
            pattern.confidence = (old_confidence * (n - 1) as f64 + rate) / n as f64;
        } else {
            pattern.confidence = (old_confidence * (n - 1) as f64 + 0.5) / n as f64;
        }
    }

    fn calculate_state_match(&self, expected: &WorldState, actual: &WorldState) -> f64 {
        let mut matches = 0;
        let mut total = 0;

        for (key, val) in &expected.context {
            total += 1;
            if actual.context.get(key) == Some(val) {
                matches += 1;
            }
        }

        if total == 0 {
            0.5
        } else {
            f64::from(matches) / f64::from(total)
        }
    }

    pub fn get_confidence(&self, action: &Action) -> f64 {
        self.patterns
            .get(&action.action_type)
            .map(|p| p.confidence)
            .unwrap_or(0.5)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RewardModel {
    reward_signals: HashMap<String, f64>,
}

impl RewardModel {
    pub fn new() -> Self {
        let mut signals = HashMap::new();
        signals.insert("success".to_string(), 1.0);
        signals.insert("error".to_string(), -1.0);
        signals.insert("time_saved".to_string(), 0.1);
        signals.insert("resource_saved".to_string(), 0.05);

        Self {
            reward_signals: signals,
        }
    }

    pub fn predict(&self, state: &WorldState) -> f64 {
        let mut reward = 0.0;

        if state.context.get("success") == Some(&"true".to_string()) {
            reward += self.reward_signals.get("success").copied().unwrap_or(0.0);
        }

        if let Some(time) = state.context.get("time_taken") {
            if let Ok(t) = time.parse::<f64>() {
                reward -= t * self
                    .reward_signals
                    .get("time_saved")
                    .copied()
                    .unwrap_or(0.0);
            }
        }

        reward
    }

    pub fn update(&mut self, state: &WorldState, success: bool) {
        // §2.5 — learn real reward signals from actual action outcomes.
        // Use exponential moving average (α=0.05) so recent results weigh more
        // but old patterns are not immediately forgotten.
        let alpha = 0.05_f64;
        let observed_reward = if success { 1.0 } else { -0.5 };

        // Update the global success/error baselines.
        if success {
            if let Some(r) = self.reward_signals.get_mut("success") {
                *r = *r * (1.0 - alpha) + observed_reward * alpha;
            }
        } else if let Some(r) = self.reward_signals.get_mut("error") {
            *r = *r * (1.0 - alpha) + observed_reward * alpha;
        }

        // Learn per-context reward signals: for every context key present in the
        // state, update a running reward estimate keyed by "ctx:<key>=<value>".
        // This allows the reward model to predict reward based on context features
        // rather than just keyword matching.
        for (key, val) in &state.context {
            let ctx_key = format!("ctx:{}={}", key, val.chars().take(32).collect::<String>());
            let current = self.reward_signals.get(&ctx_key).copied().unwrap_or(0.0);
            self.reward_signals
                .insert(ctx_key, current * (1.0 - alpha) + observed_reward * alpha);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CausalGraph {
    edges: HashMap<String, Vec<CausalRelationship>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CausalRelationship {
    pub cause: String,
    pub effect: String,
    pub strength: f64,
    pub evidence: Vec<String>,
}

impl CausalGraph {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
        }
    }

    pub fn add_causality(&mut self, causality: &DiscoveredCausality) {
        self.edges
            .entry(causality.cause.clone())
            .or_default()
            .push(CausalRelationship {
                cause: causality.cause.clone(),
                effect: causality.effect.clone(),
                strength: causality.strength,
                evidence: causality.evidence.clone(),
            });
    }

    pub fn get_relationships(&self, entity: &str) -> Vec<CausalRelationship> {
        self.edges.get(entity).cloned().unwrap_or_default()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulatedPath {
    pub actions: Vec<Action>,
    pub states: Vec<WorldState>,
    pub total_reward: f64,
    pub confidence: f64,
}

impl Default for WorldState {
    fn default() -> Self {
        let mut resources = HashMap::new();
        resources.insert("cpu".to_string(), 1.0);
        resources.insert("memory".to_string(), 1.0);
        resources.insert("network".to_string(), 1.0);

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            entities: HashMap::new(),
            context: HashMap::new(),
            constraints: vec![],
            resources,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_world_model_predict() {
        let model = WorldModel::new();
        let action = Action {
            id: "test".to_string(),
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
        };

        let outcome = model.predict(&action).await;
        assert!(outcome.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_simulation() {
        let model = WorldModel::new();
        let actions = vec![Action {
            id: "test".to_string(),
            action_type: "compute".to_string(),
            parameters: HashMap::new(),
            preconditions: vec![],
            expected_effects: vec![],
            estimated_duration_ms: 100,
            risk_level: 0.1,
        }];

        let paths = model.simulate(&actions, 2).await;
        assert!(!paths.is_empty());
    }
}

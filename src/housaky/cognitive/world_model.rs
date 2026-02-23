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
                let state_path = path.join("state.json");
                if state_path.exists() {
                    let content = tokio::fs::read_to_string(&state_path).await?;
                    let state: WorldState = serde_json::from_str(&content)?;
                    *self.current_state.write().await = state;
                }

                let causal_path = path.join("causal_graph.json");
                if causal_path.exists() {
                    let content = tokio::fs::read_to_string(&causal_path).await?;
                    let graph: CausalGraph = serde_json::from_str(&content)?;
                    *self.causal_graph.write().await = graph;
                }

                info!("Loaded world model from storage");
            }
        }
        Ok(())
    }

    pub async fn save(&self) -> Result<()> {
        if let Some(ref path) = self.storage_path {
            tokio::fs::create_dir_all(path).await?;

            let state = self.current_state.read().await;
            let content = serde_json::to_string_pretty(&*state)?;
            tokio::fs::write(path.join("state.json"), content).await?;

            let causal = self.causal_graph.read().await;
            let content = serde_json::to_string_pretty(&*causal)?;
            tokio::fs::write(path.join("causal_graph.json"), content).await?;
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
        let observation_count;
        let old_confidence;

        {
            let pattern = self
                .patterns
                .entry(action_type.clone())
                .or_insert(TransitionPattern {
                    action_type: action_type.clone(),
                    preconditions: vec![],
                    effect_distribution: HashMap::new(),
                    confidence: 0.5,
                    observation_count: 0,
                });

            pattern.observation_count += 1;
            observation_count = pattern.observation_count;
            old_confidence = pattern.confidence;
        }

        if let Some(exp) = expected {
            let match_rate = self.calculate_state_match(exp, actual);
            if let Some(pattern) = self.patterns.get_mut(&action_type) {
                pattern.confidence = (old_confidence * (observation_count - 1) as f64 + match_rate)
                    / observation_count as f64;
            }
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

    pub fn update(&mut self, _state: &WorldState, success: bool) {
        if success {
            if let Some(r) = self.reward_signals.get_mut("success") {
                *r = (*r * 0.99 + 1.0 * 0.01).max(0.1);
            }
        } else if let Some(r) = self.reward_signals.get_mut("error") {
            *r = (*r * 0.99 + 1.0 * 0.01).min(-0.1);
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

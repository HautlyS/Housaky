//! World Model - Internal representation of environment
//! Critical for AGI according to DeepMind 2026 research

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub entities: HashMap<String, Entity>,
    pub relations: Vec<Relation>,
    pub timestamp: u64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub entity_type: String,
    pub properties: HashMap<String, f64>,
    pub position: Option<[f64; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub from: String,
    pub to: String,
    pub relation_type: String,
    pub strength: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub future_state: WorldState,
    pub confidence: f64,
    pub time_horizon: u64,
}

pub struct WorldModel {
    current_state: WorldState,
    history: Vec<WorldState>,
    max_history: usize,
    prediction_horizon: u64,
}

impl WorldModel {
    pub fn new(max_history: usize, prediction_horizon: u64) -> Self {
        Self {
            current_state: WorldState {
                entities: HashMap::new(),
                relations: Vec::new(),
                timestamp: 0,
                confidence: 1.0,
            },
            history: Vec::with_capacity(max_history),
            max_history,
            prediction_horizon,
        }
    }

    /// Update world state with new observations
    pub fn update(&mut self, observations: Vec<Entity>) {
        // Archive current state
        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(self.current_state.clone());

        // Update entities
        for entity in observations {
            self.current_state.entities.insert(entity.id.clone(), entity);
        }

        self.current_state.timestamp += 1;
    }

    /// Predict future state based on history
    pub fn predict(&self, steps_ahead: u64) -> Prediction {
        if self.history.is_empty() {
            return Prediction {
                future_state: self.current_state.clone(),
                confidence: 0.1,
                time_horizon: steps_ahead,
            };
        }

        // Simple linear extrapolation for continuous properties
        let mut future_state = self.current_state.clone();
        future_state.timestamp += steps_ahead;

        for (id, entity) in &self.current_state.entities {
            if let Some(future_entity) = future_state.entities.get_mut(id) {
                // Extrapolate properties based on recent changes
                if self.history.len() >= 2 {
                    let prev_state = &self.history[self.history.len() - 1];
                    if let Some(prev_entity) = prev_state.entities.get(id) {
                        for (prop_name, &current_val) in &entity.properties {
                            if let Some(&prev_val) = prev_entity.properties.get(prop_name) {
                                let delta = current_val - prev_val;
                                let predicted = current_val + delta * steps_ahead as f64;
                                future_entity.properties.insert(prop_name.clone(), predicted);
                            }
                        }
                    }
                }
            }
        }

        // Confidence decreases with prediction horizon
        let confidence = (1.0 / (1.0 + steps_ahead as f64 / 10.0)).max(0.1);

        Prediction {
            future_state,
            confidence,
            time_horizon: steps_ahead,
        }
    }

    /// Add relation between entities
    pub fn add_relation(&mut self, from: String, to: String, relation_type: String, strength: f64) {
        self.current_state.relations.push(Relation {
            from,
            to,
            relation_type,
            strength,
        });
    }

    /// Query entities by type
    pub fn query_entities(&self, entity_type: &str) -> Vec<&Entity> {
        self.current_state
            .entities
            .values()
            .filter(|e| e.entity_type == entity_type)
            .collect()
    }

    /// Get current state
    pub fn current_state(&self) -> &WorldState {
        &self.current_state
    }

    /// Calculate model coherence (how consistent is the world model)
    pub fn coherence_score(&self) -> f64 {
        if self.history.len() < 2 {
            return 1.0;
        }

        let mut consistency_scores = Vec::new();

        // Check entity property consistency
        for (id, entity) in &self.current_state.entities {
            if let Some(prev_state) = self.history.last() {
                if let Some(prev_entity) = prev_state.entities.get(id) {
                    let mut prop_consistency = 0.0;
                    let mut prop_count = 0;

                    for (prop_name, &current_val) in &entity.properties {
                        if let Some(&prev_val) = prev_entity.properties.get(prop_name) {
                            let change_rate = (current_val - prev_val).abs() / prev_val.abs().max(1.0);
                            prop_consistency += (-change_rate).exp(); // Exponential decay
                            prop_count += 1;
                        }
                    }

                    if prop_count > 0 {
                        consistency_scores.push(prop_consistency / prop_count as f64);
                    }
                }
            }
        }

        if consistency_scores.is_empty() {
            1.0
        } else {
            consistency_scores.iter().sum::<f64>() / consistency_scores.len() as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_model_update() {
        let mut model = WorldModel::new(10, 5);
        
        let entity = Entity {
            id: "obj1".to_string(),
            entity_type: "object".to_string(),
            properties: [("velocity".to_string(), 10.0)].iter().cloned().collect(),
            position: Some([0.0, 0.0, 0.0]),
        };

        model.update(vec![entity]);
        assert_eq!(model.current_state().entities.len(), 1);
    }

    #[test]
    fn test_prediction() {
        let mut model = WorldModel::new(10, 5);
        
        let entity = Entity {
            id: "obj1".to_string(),
            entity_type: "object".to_string(),
            properties: [("velocity".to_string(), 10.0)].iter().cloned().collect(),
            position: Some([0.0, 0.0, 0.0]),
        };

        model.update(vec![entity.clone()]);
        
        let mut entity2 = entity.clone();
        entity2.properties.insert("velocity".to_string(), 15.0);
        model.update(vec![entity2]);

        let prediction = model.predict(3);
        assert!(prediction.confidence > 0.0);
    }
}

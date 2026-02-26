//! Schematic Memory — Abstract patterns extracted from episodic memories.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySchema {
    pub id: String,
    pub name: String,
    pub description: String,
    pub event_sequence_pattern: Vec<String>,
    pub abstraction_level: f64,
    pub instance_count: usize,
    pub avg_emotional_valence: f64,
    pub predictive_power: f64,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl MemorySchema {
    pub fn pattern_similarity(&self, candidate: &[String]) -> f64 {
        if self.event_sequence_pattern.is_empty() || candidate.is_empty() {
            return 0.0;
        }
        let self_set: std::collections::HashSet<&str> =
            self.event_sequence_pattern.iter().map(|s| s.as_str()).collect();
        let cand_set: std::collections::HashSet<&str> =
            candidate.iter().map(|s| s.as_str()).collect();
        let intersection = self_set.intersection(&cand_set).count();
        let union = self_set.union(&cand_set).count();
        if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
    }

    pub fn assimilate(&mut self, other: &MemorySchema) {
        let old_n = self.instance_count as f64;
        let new_n = other.instance_count as f64;
        self.instance_count += other.instance_count;
        let total = self.instance_count as f64;
        self.avg_emotional_valence =
            (self.avg_emotional_valence * old_n + other.avg_emotional_valence * new_n) / total;
        self.predictive_power =
            (self.predictive_power * old_n + other.predictive_power * new_n) / total;
        self.abstraction_level = (self.abstraction_level + 0.01).min(1.0);
        self.last_updated = Utc::now();
    }
}

// ── Schema Library ────────────────────────────────────────────────────────────

pub struct SchemaLibrary {
    pub schemas: Arc<RwLock<HashMap<String, MemorySchema>>>,
    merge_threshold: f64,
}

impl SchemaLibrary {
    pub fn new() -> Self {
        Self {
            schemas: Arc::new(RwLock::new(HashMap::new())),
            merge_threshold: 0.7,
        }
    }

    pub async fn add_or_merge_schema(&self, new_schema: MemorySchema) {
        let mut schemas = self.schemas.write().await;

        // Find an existing schema with high pattern similarity
        let similar_id: Option<String> = schemas
            .values()
            .filter(|s| s.pattern_similarity(&new_schema.event_sequence_pattern) > self.merge_threshold)
            .max_by(|a, b| {
                a.pattern_similarity(&new_schema.event_sequence_pattern)
                    .partial_cmp(&b.pattern_similarity(&new_schema.event_sequence_pattern))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|s| s.id.clone());

        if let Some(id) = similar_id {
            if let Some(existing) = schemas.get_mut(&id) {
                existing.assimilate(&new_schema);
                debug!("SchemaLibrary: merged into schema '{}'", existing.name);
            }
        } else {
            let id = new_schema.id.clone();
            debug!("SchemaLibrary: stored new schema '{}'", new_schema.name);
            schemas.insert(id, new_schema);
        }
    }

    pub async fn find_matching(&self, pattern: &[String]) -> Vec<MemorySchema> {
        let schemas = self.schemas.read().await;
        let mut matches: Vec<(f64, MemorySchema)> = schemas
            .values()
            .filter_map(|s| {
                let sim = s.pattern_similarity(pattern);
                if sim > 0.3 { Some((sim, s.clone())) } else { None }
            })
            .collect();
        matches.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        matches.into_iter().map(|(_, s)| s).collect()
    }

    pub async fn get_all(&self) -> Vec<MemorySchema> {
        self.schemas.read().await.values().cloned().collect()
    }

    pub async fn get_stats(&self) -> SchemaStats {
        let schemas = self.schemas.read().await;
        let total = schemas.len();
        let avg_instances = if total > 0 {
            schemas.values().map(|s| s.instance_count as f64).sum::<f64>() / total as f64
        } else {
            0.0
        };
        SchemaStats { total_schemas: total, avg_instances_per_schema: avg_instances }
    }
}

impl Default for SchemaLibrary {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaStats {
    pub total_schemas: usize,
    pub avg_instances_per_schema: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgettingCurve {
    pub initial_strength: f64,
    pub decay_constant: f64,
}

impl ForgettingCurve {
    pub fn ebbinghaus(initial_strength: f64) -> Self {
        Self { initial_strength, decay_constant: 0.5 }
    }

    pub fn retention_at(&self, time_hours: f64) -> f64 {
        self.initial_strength * (-self.decay_constant * time_hours.ln().max(0.0)).exp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_schema_library() {
        let lib = SchemaLibrary::new();
        let schema = MemorySchema {
            id: uuid::Uuid::new_v4().to_string(),
            name: "problem_solving".to_string(),
            description: "Standard problem solving pattern".to_string(),
            event_sequence_pattern: vec!["GoalSet".to_string(), "ReasoningStep".to_string(), "GoalAchieved".to_string()],
            abstraction_level: 0.5,
            instance_count: 1,
            avg_emotional_valence: 0.6,
            predictive_power: 0.5,
            created_at: Utc::now(),
            last_updated: Utc::now(),
        };
        lib.add_or_merge_schema(schema).await;

        let stats = lib.get_stats().await;
        assert_eq!(stats.total_schemas, 1);

        let matches = lib.find_matching(&["GoalSet".to_string(), "ReasoningStep".to_string()]).await;
        assert!(!matches.is_empty());
    }
}

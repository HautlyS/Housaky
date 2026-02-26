//! Coalition Formation — Modules compete to broadcast their content globally.
//!
//! Coalitions are groups of cognitive modules that together propose a unified
//! content for conscious broadcast. Stronger, more urgent, more novel coalitions
//! win the competition and gain access to the global workspace.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

use super::global_workspace::{CognitiveContent, CognitiveModule, ContentModality, ContentType};

// ── Coalition ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coalition {
    pub id: String,
    pub content: CognitiveContent,
    pub strength: f64,
    pub source_modules: Vec<String>,
    pub urgency: f64,
    pub novelty: f64,
}

impl Coalition {
    pub fn competition_score(&self) -> f64 {
        self.strength * 0.5 + self.urgency * 0.3 + self.novelty * 0.2
    }
}

// ── Coalition Formation Engine ────────────────────────────────────────────────

pub struct CoalitionFormation {
    pub active_coalitions: Arc<RwLock<Vec<Coalition>>>,
    pub formation_history: Arc<RwLock<Vec<FormationRecord>>>,
    pub module_reputation: Arc<RwLock<HashMap<String, f64>>>,
    max_coalitions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationRecord {
    pub timestamp: DateTime<Utc>,
    pub coalitions_formed: usize,
    pub winner_id: String,
    pub winner_score: f64,
    pub participating_modules: Vec<String>,
}

impl CoalitionFormation {
    pub fn new() -> Self {
        Self {
            active_coalitions: Arc::new(RwLock::new(Vec::new())),
            formation_history: Arc::new(RwLock::new(Vec::new())),
            module_reputation: Arc::new(RwLock::new(HashMap::new())),
            max_coalitions: 20,
        }
    }

    /// Collect proposals from all modules and form coalitions.
    pub async fn form_coalitions(
        &self,
        modules: &[Arc<dyn CognitiveModule>],
    ) -> Vec<Coalition> {
        let mut coalitions = Vec::new();

        for module in modules {
            if let Some(mut coalition) = module.propose_coalition().await {
                // Boost strength by module reputation
                let rep = {
                    let reps = self.module_reputation.read().await;
                    *reps.get(module.name()).unwrap_or(&0.5)
                };
                coalition.strength = (coalition.strength * 0.7 + rep * 0.3).clamp(0.0, 1.0);
                coalitions.push(coalition);
            }
        }

        // Merge overlapping coalitions (same content type + high semantic overlap)
        let merged = self.merge_similar_coalitions(coalitions);

        {
            let mut active = self.active_coalitions.write().await;
            *active = merged.clone();
            while active.len() > self.max_coalitions {
                active.remove(0);
            }
        }

        debug!("CoalitionFormation: {} coalitions formed from {} modules", merged.len(), modules.len());
        merged
    }

    /// Merge coalitions that propose semantically similar content.
    fn merge_similar_coalitions(&self, coalitions: Vec<Coalition>) -> Vec<Coalition> {
        if coalitions.len() <= 1 {
            return coalitions;
        }

        let mut merged: Vec<Coalition> = Vec::new();

        'outer: for candidate in coalitions {
            for existing in &mut merged {
                if self.should_merge(&candidate, existing) {
                    // Merge: combine source modules, take max strength/urgency, average novelty
                    for m in &candidate.source_modules {
                        if !existing.source_modules.contains(m) {
                            existing.source_modules.push(m.clone());
                        }
                    }
                    existing.strength = existing.strength.max(candidate.strength);
                    existing.urgency = existing.urgency.max(candidate.urgency);
                    existing.novelty = (existing.novelty + candidate.novelty) / 2.0;
                    // Strength boost for coalition size
                    existing.strength = (existing.strength + 0.05 * existing.source_modules.len() as f64).min(1.0);
                    continue 'outer;
                }
            }
            merged.push(candidate);
        }

        merged
    }

    fn should_merge(&self, a: &Coalition, b: &Coalition) -> bool {
        // Merge if same content type and similar data (first 30 chars)
        if a.content.content_type != b.content.content_type {
            return false;
        }
        let a_prefix = &a.content.data[..a.content.data.len().min(30)];
        let b_prefix = &b.content.data[..b.content.data.len().min(30)];
        // Simple Jaccard on word sets
        let a_words: std::collections::HashSet<&str> = a_prefix.split_whitespace().collect();
        let b_words: std::collections::HashSet<&str> = b_prefix.split_whitespace().collect();
        let intersection = a_words.intersection(&b_words).count();
        let union = a_words.union(&b_words).count();
        if union == 0 {
            return false;
        }
        (intersection as f64 / union as f64) > 0.4
    }

    /// Update reputation of the winning module's coalition.
    pub async fn reward_winner(&self, winner: &Coalition) {
        let mut reps = self.module_reputation.write().await;
        for module_name in &winner.source_modules {
            let rep = reps.entry(module_name.clone()).or_insert(0.5);
            // Exponential moving average update
            *rep = (*rep * 0.9 + 1.0 * 0.1).clamp(0.0, 1.0);
        }
    }

    /// Penalize modules whose coalitions consistently lose.
    pub async fn penalize_losers(&self, losers: &[Coalition]) {
        let mut reps = self.module_reputation.write().await;
        for coalition in losers {
            for module_name in &coalition.source_modules {
                let rep = reps.entry(module_name.clone()).or_insert(0.5);
                *rep = (*rep * 0.95).clamp(0.1, 1.0);
            }
        }
    }

    /// Record formation results for analysis.
    pub async fn record_formation(&self, winner: &Coalition, all: &[Coalition]) {
        let record = FormationRecord {
            timestamp: Utc::now(),
            coalitions_formed: all.len(),
            winner_id: winner.id.clone(),
            winner_score: winner.competition_score(),
            participating_modules: all
                .iter()
                .flat_map(|c| c.source_modules.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect(),
        };
        let mut history = self.formation_history.write().await;
        history.push(record);
        if history.len() > 1000 {
            history.remove(0);
        }
    }

    /// Build a default coalition from raw content (used by adapters).
    pub fn build_coalition(
        module_name: &str,
        content_type: ContentType,
        data: String,
        strength: f64,
        urgency: f64,
        novelty: f64,
    ) -> Coalition {
        Coalition {
            id: uuid::Uuid::new_v4().to_string(),
            content: CognitiveContent {
                content_type,
                data,
                embedding: Vec::new(),
                salience: strength,
                modality: ContentModality::Linguistic,
            },
            strength,
            source_modules: vec![module_name.to_string()],
            urgency,
            novelty,
        }
    }
}

impl Default for CoalitionFormation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coalition_score() {
        let c = Coalition {
            id: "test".to_string(),
            content: CognitiveContent {
                content_type: ContentType::Goal,
                data: "achieve task".to_string(),
                embedding: vec![],
                salience: 0.8,
                modality: ContentModality::Linguistic,
            },
            strength: 0.8,
            source_modules: vec!["reasoning".to_string()],
            urgency: 0.6,
            novelty: 0.4,
        };
        let score = c.competition_score();
        assert!(score > 0.0 && score <= 1.0);
    }
}

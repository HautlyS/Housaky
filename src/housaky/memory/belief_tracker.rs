use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Belief {
    pub id: String,
    pub content: String,
    pub confidence: f64,
    pub evidence: Vec<String>,
    pub contradictions: Vec<String>,
    pub source: BeliefSource,
    pub created_at: DateTime<Utc>,
    pub last_verified: DateTime<Utc>,
    pub decay_rate: f64,
    pub importance: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BeliefSource {
    DirectExperience,
    Observation,
    Reasoning,
    External,
    Assumption,
}

impl Default for Belief {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content: String::new(),
            confidence: 0.5,
            evidence: Vec::new(),
            contradictions: Vec::new(),
            source: BeliefSource::Assumption,
            created_at: Utc::now(),
            last_verified: Utc::now(),
            decay_rate: 0.01,
            importance: 0.5,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BeliefWithContext {
    pub belief: Belief,
    pub related_beliefs: Vec<String>,
    pub applicable_actions: Vec<String>,
    pub verification_needed: bool,
}

pub struct BeliefTracker {
    beliefs: Arc<RwLock<HashMap<String, Belief>>>,
    belief_history: Arc<RwLock<Vec<BeliefHistoryEntry>>>,
    uncertainty_threshold: f64,
    storage_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BeliefHistoryEntry {
    belief_id: String,
    timestamp: DateTime<Utc>,
    change_type: BeliefChangeType,
    old_confidence: f64,
    new_confidence: f64,
    reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum BeliefChangeType {
    Created,
    EvidenceAdded,
    ContradictionFound,
    Decayed,
    Verified,
    Revised,
}

impl BeliefTracker {
    pub fn new() -> Self {
        Self {
            beliefs: Arc::new(RwLock::new(HashMap::new())),
            belief_history: Arc::new(RwLock::new(Vec::new())),
            uncertainty_threshold: 0.3,
            storage_path: None,
        }
    }

    pub fn with_storage(workspace_dir: &PathBuf) -> Self {
        let storage_path = workspace_dir.join(".housaky").join("beliefs.json");
        Self {
            beliefs: Arc::new(RwLock::new(HashMap::new())),
            belief_history: Arc::new(RwLock::new(Vec::new())),
            uncertainty_threshold: 0.3,
            storage_path: Some(storage_path),
        }
    }

    pub async fn load(&self) -> Result<()> {
        if let Some(ref path) = self.storage_path {
            if path.exists() {
                let content = tokio::fs::read_to_string(path).await?;
                let beliefs: HashMap<String, Belief> = serde_json::from_str(&content)?;
                let mut storage = self.beliefs.write().await;
                *storage = beliefs;
                info!("Loaded {} beliefs from storage", storage.len());
            }
        }
        Ok(())
    }

    pub async fn save(&self) -> Result<()> {
        if let Some(ref path) = self.storage_path {
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            let beliefs = self.beliefs.read().await;
            let content = serde_json::to_string_pretty(&*beliefs)?;
            tokio::fs::write(path, content).await?;
            info!("Saved {} beliefs to storage", beliefs.len());
        }
        Ok(())
    }

    pub async fn add_belief(
        &self,
        content: String,
        confidence: f64,
        source: BeliefSource,
    ) -> Result<String> {
        let belief = Belief {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            confidence: confidence.clamp(0.0, 1.0),
            evidence: Vec::new(),
            contradictions: Vec::new(),
            source,
            created_at: Utc::now(),
            last_verified: Utc::now(),
            decay_rate: 0.01,
            importance: 0.5,
        };

        let id = belief.id.clone();
        self.beliefs
            .write()
            .await
            .insert(id.clone(), belief.clone());

        self.belief_history.write().await.push(BeliefHistoryEntry {
            belief_id: id.clone(),
            timestamp: Utc::now(),
            change_type: BeliefChangeType::Created,
            old_confidence: 0.0,
            new_confidence: belief.confidence,
            reason: "Initial belief creation".to_string(),
        });

        info!("Added belief: {}", id);
        Ok(id)
    }

    pub async fn update_with_evidence(&self, belief_id: &str, evidence: &str) -> Result<()> {
        let mut beliefs = self.beliefs.write().await;
        if let Some(belief) = beliefs.get_mut(belief_id) {
            let old_confidence = belief.confidence;
            belief.evidence.push(evidence.to_string());
            belief.confidence = self.bayesian_update(belief.confidence, 0.7);
            belief.last_verified = Utc::now();

            self.belief_history.write().await.push(BeliefHistoryEntry {
                belief_id: belief_id.to_string(),
                timestamp: Utc::now(),
                change_type: BeliefChangeType::EvidenceAdded,
                old_confidence,
                new_confidence: belief.confidence,
                reason: format!("Evidence added: {}", evidence),
            });

            info!(
                "Updated belief {} confidence: {:.2} -> {:.2}",
                belief_id, old_confidence, belief.confidence
            );
        }
        Ok(())
    }

    pub async fn add_contradiction(&self, belief_id: &str, contradiction: &str) -> Result<()> {
        let mut beliefs = self.beliefs.write().await;
        if let Some(belief) = beliefs.get_mut(belief_id) {
            let old_confidence = belief.confidence;
            belief.contradictions.push(contradiction.to_string());
            belief.confidence = self.bayesian_update(belief.confidence, 0.3);
            belief.last_verified = Utc::now();

            self.belief_history.write().await.push(BeliefHistoryEntry {
                belief_id: belief_id.to_string(),
                timestamp: Utc::now(),
                change_type: BeliefChangeType::ContradictionFound,
                old_confidence,
                new_confidence: belief.confidence,
                reason: format!("Contradiction found: {}", contradiction),
            });

            info!(
                "Contradiction for belief {}: confidence {:.2} -> {:.2}",
                belief_id, old_confidence, belief.confidence
            );
        }
        Ok(())
    }

    pub async fn check_contradictions(&self, new_belief: &str) -> Vec<Belief> {
        let beliefs = self.beliefs.read().await;
        let new_lower = new_belief.to_lowercase();

        let mut contradictory = Vec::new();
        for belief in beliefs.values() {
            let content_lower = belief.content.to_lowercase();

            let has_negation = ["not", "never", "no ", "cannot", "can't"]
                .iter()
                .any(|neg| {
                    (new_lower.contains(neg) && !content_lower.trim().is_empty())
                        || (content_lower.contains(neg) && !new_lower.trim().is_empty())
                });

            if has_negation || self.semantic_similarity(&new_lower, &content_lower) > 0.8 {
                contradictory.push(belief.clone());
            }
        }

        contradictory
    }

    fn semantic_similarity(&self, a: &str, b: &str) -> f64 {
        let words_a: std::collections::HashSet<_> =
            a.split_whitespace().map(|s| s.to_lowercase()).collect();
        let words_b: std::collections::HashSet<_> =
            b.split_whitespace().map(|s| s.to_lowercase()).collect();

        if words_a.is_empty() || words_b.is_empty() {
            return 0.0;
        }

        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();

        intersection as f64 / union as f64
    }

    pub async fn get_uncertain_beliefs(&self) -> Vec<Belief> {
        let beliefs = self.beliefs.read().await;
        beliefs
            .values()
            .filter(|b| b.confidence < self.uncertainty_threshold)
            .cloned()
            .collect()
    }

    pub async fn get_high_confidence_beliefs(&self, threshold: f64) -> Vec<Belief> {
        let beliefs = self.beliefs.read().await;
        beliefs
            .values()
            .filter(|b| b.confidence >= threshold)
            .cloned()
            .collect()
    }

    pub async fn get_belief(&self, id: &str) -> Option<Belief> {
        let beliefs = self.beliefs.read().await;
        beliefs.get(id).cloned()
    }

    pub async fn get_all_beliefs(&self) -> Vec<Belief> {
        let beliefs = self.beliefs.read().await;
        beliefs.values().cloned().collect()
    }

    pub async fn get_belief_count(&self) -> usize {
        let beliefs = self.beliefs.read().await;
        beliefs.len()
    }

    pub async fn apply_decay(&self) -> Result<usize> {
        let mut beliefs = self.beliefs.write().await;
        let mut decayed = 0;

        for belief in beliefs.values_mut() {
            if belief.confidence > 0.1 {
                belief.confidence = (belief.confidence - belief.decay_rate).max(0.1);
                decayed += 1;

                self.belief_history.write().await.push(BeliefHistoryEntry {
                    belief_id: belief.id.clone(),
                    timestamp: Utc::now(),
                    change_type: BeliefChangeType::Decayed,
                    old_confidence: belief.confidence + belief.decay_rate,
                    new_confidence: belief.confidence,
                    reason: "Natural confidence decay".to_string(),
                });
            }
        }

        Ok(decayed)
    }

    fn bayesian_update(&self, prior: f64, likelihood: f64) -> f64 {
        let posterior =
            (likelihood * prior) / ((likelihood * prior) + ((1.0 - likelihood) * (1.0 - prior)));
        posterior.clamp(0.0, 1.0)
    }

    pub async fn search_beliefs(&self, query: &str) -> Vec<Belief> {
        let beliefs = self.beliefs.read().await;
        let query_lower = query.to_lowercase();

        beliefs
            .values()
            .filter(|b| {
                b.content.to_lowercase().contains(&query_lower)
                    || b.evidence
                        .iter()
                        .any(|e| e.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }

    pub async fn get_belief_statistics(&self) -> BeliefStatistics {
        let beliefs = self.beliefs.read().await;
        let history = self.belief_history.read().await;

        let mut by_source: HashMap<String, usize> = HashMap::new();
        let mut confidence_buckets = [0usize; 5];

        for belief in beliefs.values() {
            let source_str = match belief.source {
                BeliefSource::DirectExperience => "direct_experience",
                BeliefSource::Observation => "observation",
                BeliefSource::Reasoning => "reasoning",
                BeliefSource::External => "external",
                BeliefSource::Assumption => "assumption",
            };
            *by_source.entry(source_str.to_string()).or_insert(0) += 1;

            let bucket = match belief.confidence {
                0.0..=0.2 => 0,
                0.2..=0.4 => 1,
                0.4..=0.6 => 2,
                0.6..=0.8 => 3,
                _ => 4,
            };
            confidence_buckets[bucket] += 1;
        }

        BeliefStatistics {
            total_beliefs: beliefs.len(),
            by_source,
            confidence_distribution: confidence_buckets,
            history_entries: history.len(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BeliefStatistics {
    pub total_beliefs: usize,
    pub by_source: HashMap<String, usize>,
    pub confidence_distribution: [usize; 5],
    pub history_entries: usize,
}

impl Default for BeliefTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_belief() {
        let tracker = BeliefTracker::new();
        let id = tracker
            .add_belief(
                "The sky is blue".to_string(),
                0.9,
                BeliefSource::Observation,
            )
            .await
            .unwrap();

        let belief = tracker.get_belief(&id).await;
        assert!(belief.is_some());
        assert_eq!(belief.unwrap().content, "The sky is blue");
    }

    #[tokio::test]
    async fn test_confidence_update() {
        let tracker = BeliefTracker::new();
        let id = tracker
            .add_belief("Test belief".to_string(), 0.5, BeliefSource::Assumption)
            .await
            .unwrap();

        tracker
            .update_with_evidence(&id, "Supporting evidence")
            .await
            .unwrap();

        let belief = tracker.get_belief(&id).await.unwrap();
        assert!(belief.confidence > 0.5);
    }

    #[tokio::test]
    async fn test_uncertain_beliefs() {
        let tracker = BeliefTracker::new();
        tracker
            .add_belief("Certain".to_string(), 0.9, BeliefSource::Reasoning)
            .await
            .unwrap();
        let uncertain_id = tracker
            .add_belief("Uncertain".to_string(), 0.1, BeliefSource::Assumption)
            .await
            .unwrap();

        let uncertain = tracker.get_uncertain_beliefs().await;
        assert_eq!(uncertain.len(), 1);
        assert_eq!(uncertain[0].id, uncertain_id);
    }
}

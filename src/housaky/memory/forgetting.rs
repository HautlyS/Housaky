//! Adaptive Forgetting — Interference-based forgetting, not just LRU.
//!
//! Models two main forgetting mechanisms:
//! 1. Ebbinghaus decay: time-based trace weakening
//! 2. Interference: similar memories compete and weaken each other

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

use super::episodic::EpisodicMemory;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgettingConfig {
    /// Base decay rate per hour (Ebbinghaus)
    pub time_decay_rate: f64,
    /// Interference strength between similar memories
    pub interference_strength: f64,
    /// Minimum vividness before a memory is eligible for pruning
    pub prune_threshold: f64,
    /// Importance floor — memories above this are never pruned
    pub importance_floor: f64,
    /// Maximum interference radius (cosine similarity threshold)
    pub interference_radius: f64,
}

impl Default for ForgettingConfig {
    fn default() -> Self {
        Self {
            time_decay_rate: 0.0001,
            interference_strength: 0.05,
            prune_threshold: 0.02,
            importance_floor: 0.7,
            interference_radius: 0.6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgettingReport {
    pub timestamp: DateTime<Utc>,
    pub episodes_decayed: usize,
    pub episodes_pruned: usize,
    pub interference_events: usize,
    pub avg_vividness_before: f64,
    pub avg_vividness_after: f64,
}

pub struct AdaptiveForgetting {
    pub config: ForgettingConfig,
    pub reports: Arc<RwLock<Vec<ForgettingReport>>>,
}

impl AdaptiveForgetting {
    pub fn new(config: ForgettingConfig) -> Self {
        Self {
            config,
            reports: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Apply time-based Ebbinghaus decay to all episodes.
    pub async fn apply_time_decay(&self, episodic: &EpisodicMemory) -> usize {
        let now = Utc::now();
        let mut episodes = episodic.episodes.write().await;
        let mut decayed = 0usize;

        for ep in episodes.iter_mut() {
            let age_hours = (now - ep.timestamp).num_minutes() as f64 / 60.0;
            // Ebbinghaus forgetting curve: R = e^(-t/S) where S = stability
            let stability = ep.importance * 100.0 + ep.retrieval_count as f64 * 5.0 + 1.0;
            let retention = (-age_hours / stability).exp().max(0.01);
            let new_vividness = ep.vividness * retention;

            if (new_vividness - ep.vividness).abs() > 0.001 {
                ep.vividness = new_vividness.clamp(0.01, 1.0);
                decayed += 1;
            }
        }

        debug!("AdaptiveForgetting: time decay applied to {} episodes", decayed);
        decayed
    }

    /// Apply proactive interference: similar memories compete, weakening each other.
    pub async fn apply_interference(&self, episodic: &EpisodicMemory) -> usize {
        let episodes_snap: Vec<_> = {
            let eps = episodic.episodes.read().await;
            eps.iter().map(|e| (e.id.clone(), e.context.goal.clone(), e.vividness)).collect()
        };

        let mut interference_events = 0usize;
        let mut adjustments: Vec<(String, f64)> = Vec::new();

        // Find pairs of similar episodes (same goal context)
        for i in 0..episodes_snap.len() {
            for j in (i + 1)..episodes_snap.len() {
                let (id_a, goal_a, viv_a) = &episodes_snap[i];
                let (id_b, goal_b, viv_b) = &episodes_snap[j];

                let similar = match (goal_a, goal_b) {
                    (Some(ga), Some(gb)) => {
                        let a_words: std::collections::HashSet<&str> = ga.split_whitespace().collect();
                        let b_words: std::collections::HashSet<&str> = gb.split_whitespace().collect();
                        let intersection = a_words.intersection(&b_words).count();
                        let union = a_words.union(&b_words).count();
                        union > 0 && (intersection as f64 / union as f64) > self.config.interference_radius
                    }
                    _ => false,
                };

                if similar {
                    // Mutual interference: each weakens the other proportional to other's vividness
                    let decay_a = viv_b * self.config.interference_strength;
                    let decay_b = viv_a * self.config.interference_strength;
                    adjustments.push((id_a.clone(), -decay_a));
                    adjustments.push((id_b.clone(), -decay_b));
                    interference_events += 1;
                }
            }
        }

        // Apply adjustments
        if !adjustments.is_empty() {
            let mut episodes = episodic.episodes.write().await;
            for (id, delta) in &adjustments {
                if let Some(ep) = episodes.iter_mut().find(|e| &e.id == id) {
                    ep.vividness = (ep.vividness + delta).clamp(0.01, 1.0);
                }
            }
        }

        interference_events
    }

    /// Prune episodes that are below the vividness threshold and not important.
    pub async fn prune_faded_memories(&self, episodic: &EpisodicMemory) -> usize {
        let mut episodes = episodic.episodes.write().await;
        let before = episodes.len();
        episodes.retain(|ep| {
            ep.vividness > self.config.prune_threshold
                || ep.importance > self.config.importance_floor
                || ep.retrieval_count > 5
        });
        let pruned = before - episodes.len();
        if pruned > 0 {
            debug!("AdaptiveForgetting: pruned {} faded memories", pruned);
        }
        pruned
    }

    /// Run a full forgetting cycle.
    pub async fn run_forgetting_cycle(&self, episodic: &EpisodicMemory) -> ForgettingReport {
        let avg_before = {
            let eps = episodic.episodes.read().await;
            if eps.is_empty() { 0.0 } else { eps.iter().map(|e| e.vividness).sum::<f64>() / eps.len() as f64 }
        };

        let decayed = self.apply_time_decay(episodic).await;
        let interference = self.apply_interference(episodic).await;
        let pruned = self.prune_faded_memories(episodic).await;

        let avg_after = {
            let eps = episodic.episodes.read().await;
            if eps.is_empty() { 0.0 } else { eps.iter().map(|e| e.vividness).sum::<f64>() / eps.len() as f64 }
        };

        let report = ForgettingReport {
            timestamp: Utc::now(),
            episodes_decayed: decayed,
            episodes_pruned: pruned,
            interference_events: interference,
            avg_vividness_before: avg_before,
            avg_vividness_after: avg_after,
        };

        self.reports.write().await.push(report.clone());
        report
    }
}

impl Default for AdaptiveForgetting {
    fn default() -> Self {
        Self::new(ForgettingConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::housaky::memory::episodic::{EpisodicMemory, EpisodicEventType};
    use crate::housaky::memory::emotional_tags::EmotionalTag;

    #[tokio::test]
    async fn test_forgetting_cycle() {
        let episodic = EpisodicMemory::new(100);
        let forgetting = AdaptiveForgetting::default();

        episodic.begin_episode(Some("goal A".to_string()), "test").await;
        episodic.record_event(EpisodicEventType::GoalSet, "goal A", 0.3).await;
        episodic.end_episode(EmotionalTag::neutral(), true).await;

        let report = forgetting.run_forgetting_cycle(&episodic).await;
        assert_eq!(report.episodes_pruned, 0); // importance floor should protect it
    }
}

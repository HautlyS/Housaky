//! Continual Learning without Catastrophic Forgetting
//!
//! Prevents old skills/beliefs from being overwritten by new ones:
//! - Elastic Weight Consolidation analog: protects high-importance beliefs
//! - Importance weighting for procedural memory
//! - Memory replay: periodic re-encounter of past experiences
//! - Curriculum learning: order new info from simple to complex
//! - Integration with MemoryConsolidator for sleep-like cycles

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

// ── Core Types ───────────────────────────────────────────────────────────────

/// An episode for memory replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub domain: String,
    pub content: String,
    pub importance: f64,
    pub replay_count: u32,
    pub last_replayed: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub outcome_success: bool,
    pub lessons: Vec<String>,
    pub associated_skills: Vec<String>,
}

/// A learning item for curriculum ordering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningItem {
    pub id: String,
    pub content: String,
    pub complexity: f64, // 0.0 (simple) to 1.0 (complex)
    pub prerequisites: Vec<String>,
    pub domain: String,
    pub mastery_level: f64, // 0.0 to 1.0
    pub practice_count: u32,
}

/// A curriculum ordering for learning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningCurriculum {
    pub items: Vec<LearningItem>,
    pub current_position: usize,
    pub overall_progress: f64,
    pub created_at: DateTime<Utc>,
}

impl Default for LearningCurriculum {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            current_position: 0,
            overall_progress: 0.0,
            created_at: Utc::now(),
        }
    }
}

/// Protection status for a memory item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectionStatus {
    pub memory_id: String,
    pub importance_weight: f64,
    pub usage_frequency: f64,
    pub protected: bool,
    pub protection_reason: String,
    pub last_accessed: DateTime<Utc>,
}

/// Result of a replay cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayCycleResult {
    pub episodes_replayed: usize,
    pub reinforced_skills: Vec<String>,
    pub fading_memories: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub consolidation_score: f64,
}

// ── Continual Learner ────────────────────────────────────────────────────────

pub struct ContinualLearner {
    pub importance_weights: Arc<RwLock<HashMap<String, f64>>>,
    pub replay_buffer: Arc<RwLock<VecDeque<Episode>>>,
    pub curriculum: Arc<RwLock<LearningCurriculum>>,
    pub protection_registry: Arc<RwLock<HashMap<String, ProtectionStatus>>>,
    pub usage_tracker: Arc<RwLock<HashMap<String, UsageRecord>>>,
    pub replay_interval_secs: u64,
    pub max_replay_buffer_size: usize,
    pub protection_threshold: f64,
}

/// Tracks usage frequency for importance weighting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub access_count: u64,
    pub last_accessed: DateTime<Utc>,
    pub first_accessed: DateTime<Utc>,
    pub success_count: u64,
    pub failure_count: u64,
}

impl ContinualLearner {
    pub fn new() -> Self {
        Self {
            importance_weights: Arc::new(RwLock::new(HashMap::new())),
            replay_buffer: Arc::new(RwLock::new(VecDeque::new())),
            curriculum: Arc::new(RwLock::new(LearningCurriculum::default())),
            protection_registry: Arc::new(RwLock::new(HashMap::new())),
            usage_tracker: Arc::new(RwLock::new(HashMap::new())),
            replay_interval_secs: 3600,    // 1 hour
            max_replay_buffer_size: 10_000,
            protection_threshold: 0.7,
        }
    }

    /// Determine if a memory item should be protected from modification.
    pub async fn should_protect(&self, memory_id: &str) -> bool {
        let weights = self.importance_weights.read().await;
        let usage = self.usage_tracker.read().await;

        let importance = weights.get(memory_id).copied().unwrap_or(0.0);

        let usage_score = usage.get(memory_id).map(|u| {
            let total = u.access_count.max(1) as f64;
            let success_rate = u.success_count as f64 / total;
            let recency = {
                let age = (Utc::now() - u.last_accessed).num_hours() as f64;
                1.0 / (1.0 + age / 24.0) // decay over days
            };
            success_rate * 0.6 + recency * 0.4
        }).unwrap_or(0.0);

        let combined_score = importance * 0.5 + usage_score * 0.5;
        combined_score > self.protection_threshold
    }

    /// Update importance weight for a memory item.
    pub async fn update_importance(&self, memory_id: &str, importance: f64) {
        let mut weights = self.importance_weights.write().await;
        let current = weights.get(memory_id).copied().unwrap_or(0.5);

        // Exponential moving average
        let alpha = 0.3;
        let updated = current * (1.0 - alpha) + importance * alpha;
        weights.insert(memory_id.to_string(), updated);

        // Update protection registry
        let should_protect = updated > self.protection_threshold;
        let mut registry = self.protection_registry.write().await;
        registry.insert(
            memory_id.to_string(),
            ProtectionStatus {
                memory_id: memory_id.to_string(),
                importance_weight: updated,
                usage_frequency: self.get_usage_frequency(memory_id).await,
                protected: should_protect,
                protection_reason: if should_protect {
                    format!("High importance ({:.2}) exceeds threshold", updated)
                } else {
                    "Below protection threshold".to_string()
                },
                last_accessed: Utc::now(),
            },
        );
    }

    /// Record usage of a memory/skill.
    pub async fn record_usage(&self, memory_id: &str, success: bool) {
        let mut tracker = self.usage_tracker.write().await;
        let record = tracker.entry(memory_id.to_string()).or_insert_with(|| UsageRecord {
            access_count: 0,
            last_accessed: Utc::now(),
            first_accessed: Utc::now(),
            success_count: 0,
            failure_count: 0,
        });

        record.access_count += 1;
        record.last_accessed = Utc::now();
        if success {
            record.success_count += 1;
        } else {
            record.failure_count += 1;
        }
    }

    /// Get usage frequency for a memory item.
    async fn get_usage_frequency(&self, memory_id: &str) -> f64 {
        let tracker = self.usage_tracker.read().await;
        tracker.get(memory_id).map(|u| {
            let age_hours = (Utc::now() - u.first_accessed).num_hours().max(1) as f64;
            u.access_count as f64 / age_hours
        }).unwrap_or(0.0)
    }

    /// Add an episode to the replay buffer.
    pub async fn add_to_replay_buffer(&self, episode: Episode) {
        let mut buffer = self.replay_buffer.write().await;
        buffer.push_back(episode);

        // Evict low-importance episodes if buffer is full
        if buffer.len() > self.max_replay_buffer_size {
            // Remove the least important episode
            if let Some(min_idx) = buffer
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.importance
                        .partial_cmp(&b.importance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
            {
                buffer.remove(min_idx);
            }
        }
    }

    /// Run a memory replay cycle (like sleep consolidation).
    pub async fn replay_cycle(&self, max_replays: usize) -> ReplayCycleResult {
        let mut buffer = self.replay_buffer.write().await;

        if buffer.is_empty() {
            return ReplayCycleResult {
                episodes_replayed: 0,
                reinforced_skills: Vec::new(),
                fading_memories: Vec::new(),
                timestamp: Utc::now(),
                consolidation_score: 0.0,
            };
        }

        // Select episodes for replay using prioritized sampling
        // Higher importance + lower recent replay count = higher priority
        let mut replay_priorities: Vec<(usize, f64)> = buffer
            .iter()
            .enumerate()
            .map(|(i, ep)| {
                let recency_penalty = ep
                    .last_replayed
                    .map(|lr| {
                        let hours = (Utc::now() - lr).num_hours() as f64;
                        1.0 / (1.0 + hours / 24.0)
                    })
                    .unwrap_or(0.0);

                let priority = ep.importance * (1.0 - recency_penalty)
                    / (1.0 + ep.replay_count as f64 * 0.1);
                (i, priority)
            })
            .collect();

        replay_priorities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let replays_to_do = max_replays.min(buffer.len());
        let mut reinforced_skills = Vec::new();
        let indices: Vec<usize> = replay_priorities
            .iter()
            .take(replays_to_do)
            .map(|(i, _)| *i)
            .collect();

        for &idx in &indices {
            if let Some(episode) = buffer.get_mut(idx) {
                episode.replay_count += 1;
                episode.last_replayed = Some(Utc::now());
                reinforced_skills.extend(episode.associated_skills.clone());
            }
        }

        // Identify fading memories (not replayed in a long time, low importance)
        let fading: Vec<String> = buffer
            .iter()
            .filter(|ep| {
                let age_hours = ep
                    .last_replayed
                    .map(|lr| (Utc::now() - lr).num_hours())
                    .unwrap_or_else(|| (Utc::now() - ep.created_at).num_hours());
                age_hours > 168 && ep.importance < 0.3 // > 1 week, low importance
            })
            .map(|ep| ep.id.clone())
            .collect();

        let consolidation_score = if buffer.is_empty() {
            0.0
        } else {
            let avg_importance = buffer.iter().map(|e| e.importance).sum::<f64>()
                / buffer.len() as f64;
            let replay_coverage = replays_to_do as f64 / buffer.len() as f64;
            avg_importance * 0.5 + replay_coverage * 0.5
        };

        info!(
            "Replay cycle: {} episodes replayed, {} skills reinforced, {} fading memories detected",
            replays_to_do,
            reinforced_skills.len(),
            fading.len()
        );

        ReplayCycleResult {
            episodes_replayed: replays_to_do,
            reinforced_skills,
            fading_memories: fading,
            timestamp: Utc::now(),
            consolidation_score,
        }
    }

    /// Order learning items from simple to complex (curriculum learning).
    pub async fn order_curriculum(
        &self,
        new_items: Vec<LearningItem>,
    ) -> Vec<LearningItem> {
        let items = new_items;

        // Topological sort based on prerequisites, then by complexity
        let mut ordered = Vec::new();
        let mut remaining = items.clone();
        let mut satisfied: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Add items whose prerequisites are already satisfied
        while !remaining.is_empty() {
            let mut added_any = false;

            remaining.retain(|item| {
                let prereqs_met = item
                    .prerequisites
                    .iter()
                    .all(|p| satisfied.contains(p));

                if prereqs_met {
                    satisfied.insert(item.id.clone());
                    ordered.push(item.clone());
                    added_any = true;
                    false // remove from remaining
                } else {
                    true // keep in remaining
                }
            });

            if !added_any {
                // Circular dependency or unresolvable prerequisites
                // Add remaining items sorted by complexity
                remaining.sort_by(|a, b| {
                    a.complexity
                        .partial_cmp(&b.complexity)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                ordered.extend(remaining);
                break;
            }
        }

        // Sort items with same prerequisite level by complexity
        // (stable sort preserves prerequisite ordering)
        ordered.sort_by(|a, b| {
            let a_has_prereqs = !a.prerequisites.is_empty();
            let b_has_prereqs = !b.prerequisites.is_empty();
            if a_has_prereqs == b_has_prereqs {
                a.complexity
                    .partial_cmp(&b.complexity)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                std::cmp::Ordering::Equal
            }
        });

        // Update curriculum
        let mut curriculum = self.curriculum.write().await;
        curriculum.items = ordered.clone();
        curriculum.current_position = 0;
        curriculum.overall_progress = 0.0;

        info!(
            "Curriculum ordered: {} items, from complexity {:.2} to {:.2}",
            ordered.len(),
            ordered.first().map(|i| i.complexity).unwrap_or(0.0),
            ordered.last().map(|i| i.complexity).unwrap_or(0.0),
        );

        ordered
    }

    /// Get the next learning item from the curriculum.
    pub async fn get_next_learning_item(&self) -> Option<LearningItem> {
        let curriculum = self.curriculum.read().await;
        curriculum.items.get(curriculum.current_position).cloned()
    }

    /// Mark a learning item as practiced/mastered.
    pub async fn mark_practiced(
        &self,
        item_id: &str,
        mastery_gained: f64,
    ) {
        let mut curriculum = self.curriculum.write().await;

        if let Some(item) = curriculum.items.iter_mut().find(|i| i.id == item_id) {
            item.mastery_level = (item.mastery_level + mastery_gained).min(1.0);
            item.practice_count += 1;
        }

        // Advance position if current item is mastered
        if let Some(current) = curriculum.items.get(curriculum.current_position) {
            if current.mastery_level >= 0.8 {
                curriculum.current_position += 1;
            }
        }

        // Update overall progress
        if !curriculum.items.is_empty() {
            curriculum.overall_progress = curriculum
                .items
                .iter()
                .map(|i| i.mastery_level)
                .sum::<f64>()
                / curriculum.items.len() as f64;
        }
    }

    /// Get continual learning statistics.
    pub async fn get_stats(&self) -> ContinualLearningStats {
        let weights = self.importance_weights.read().await;
        let buffer = self.replay_buffer.read().await;
        let curriculum = self.curriculum.read().await;
        let registry = self.protection_registry.read().await;

        let protected_count = registry.values().filter(|p| p.protected).count();

        ContinualLearningStats {
            total_tracked_items: weights.len(),
            protected_items: protected_count,
            replay_buffer_size: buffer.len(),
            curriculum_items: curriculum.items.len(),
            curriculum_progress: curriculum.overall_progress,
            avg_importance: if !weights.is_empty() {
                weights.values().sum::<f64>() / weights.len() as f64
            } else {
                0.0
            },
            total_replays: buffer.iter().map(|e| e.replay_count as u64).sum(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinualLearningStats {
    pub total_tracked_items: usize,
    pub protected_items: usize,
    pub replay_buffer_size: usize,
    pub curriculum_items: usize,
    pub curriculum_progress: f64,
    pub avg_importance: f64,
    pub total_replays: u64,
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_importance_protection() {
        let learner = ContinualLearner::new();

        // Low importance → not protected
        learner.update_importance("skill-1", 0.3).await;
        assert!(!learner.should_protect("skill-1").await);

        // High importance → protected
        learner.update_importance("skill-2", 0.95).await;
        assert!(learner.should_protect("skill-2").await);
    }

    #[tokio::test]
    async fn test_replay_cycle() {
        let learner = ContinualLearner::new();

        for i in 0..5 {
            learner
                .add_to_replay_buffer(Episode {
                    id: format!("ep-{}", i),
                    domain: "testing".to_string(),
                    content: format!("Test episode {}", i),
                    importance: (i as f64 + 1.0) * 0.2,
                    replay_count: 0,
                    last_replayed: None,
                    created_at: Utc::now(),
                    outcome_success: true,
                    lessons: vec!["Learned something".to_string()],
                    associated_skills: vec!["testing".to_string()],
                })
                .await;
        }

        let result = learner.replay_cycle(3).await;
        assert_eq!(result.episodes_replayed, 3);
        assert!(!result.reinforced_skills.is_empty());
    }

    #[tokio::test]
    async fn test_curriculum_ordering() {
        let learner = ContinualLearner::new();

        let items = vec![
            LearningItem {
                id: "advanced".to_string(),
                content: "Advanced topic".to_string(),
                complexity: 0.9,
                prerequisites: vec!["basic".to_string()],
                domain: "test".to_string(),
                mastery_level: 0.0,
                practice_count: 0,
            },
            LearningItem {
                id: "basic".to_string(),
                content: "Basic topic".to_string(),
                complexity: 0.2,
                prerequisites: vec![],
                domain: "test".to_string(),
                mastery_level: 0.0,
                practice_count: 0,
            },
        ];

        let ordered = learner.order_curriculum(items).await;
        assert_eq!(ordered[0].id, "basic");
    }
}

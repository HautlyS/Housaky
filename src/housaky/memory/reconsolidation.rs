//! Memory Reconsolidation — Modify existing memories during dream engine cycles.
//!
//! During idle/dream periods the agent revisits episodic memories, extracts schemas,
//! strengthens or weakens traces based on relevance to active goals, and updates
//! emotional tags based on new understanding.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::episodic::{EpisodicMemory, Episode};
use super::schema::{MemorySchema, SchemaLibrary};

// ── Reconsolidation Record ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconsolidationRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub episodes_processed: usize,
    pub schemas_extracted: usize,
    pub memories_strengthened: usize,
    pub memories_weakened: usize,
    pub emotional_updates: usize,
    pub duration_ms: u64,
    pub trigger: ReconsolidationTrigger,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReconsolidationTrigger {
    /// Scheduled during dream/idle cycle
    DreamCycle,
    /// Triggered by a significant new experience
    SignificantEvent,
    /// Triggered by retrieval of a strongly emotional memory
    RetrievalActivated,
    /// Manual invocation
    Manual,
}

// ── Memory Reconsolidator ─────────────────────────────────────────────────────

pub struct MemoryReconsolidator {
    pub history: Arc<RwLock<Vec<ReconsolidationRecord>>>,
    pub total_cycles: Arc<RwLock<u64>>,
}

impl MemoryReconsolidator {
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(Vec::new())),
            total_cycles: Arc::new(RwLock::new(0)),
        }
    }

    /// Run a full reconsolidation cycle over episodic memory.
    pub async fn reconsolidate(
        &self,
        episodic: &EpisodicMemory,
        schema_library: &SchemaLibrary,
        active_goals: &[String],
        trigger: ReconsolidationTrigger,
    ) -> Result<ReconsolidationRecord> {
        let start = std::time::Instant::now();
        let record_id = uuid::Uuid::new_v4().to_string();

        let mut schemas_extracted = 0usize;
        let mut memories_strengthened = 0usize;
        let mut memories_weakened = 0usize;
        let mut emotional_updates = 0usize;

        // Phase 1: Retrieve recent + high-importance episodes for processing
        let candidates = episodic.retrieve("", 50).await;
        let episodes_processed = candidates.len();

        info!(
            "Reconsolidation: processing {} episodes (trigger={:?})",
            episodes_processed, trigger
        );

        for episode in &candidates {
            // Phase 2: Schema extraction — find patterns
            if let Some(schema) = self.try_extract_schema(episode, schema_library).await {
                episodic.assign_schema(&episode.id, &schema.id).await;
                schemas_extracted += 1;
                debug!("Reconsolidation: extracted schema '{}' from episode {}", schema.name, episode.id);
            }

            // Phase 3: Goal-relevance based strength modulation
            let goal_relevance = self.compute_goal_relevance(episode, active_goals);
            if goal_relevance > 0.6 {
                self.strengthen_memory(episodic, &episode.id, goal_relevance).await;
                memories_strengthened += 1;
            } else if goal_relevance < 0.2 && episode.retrieval_count == 0 {
                self.weaken_memory(episodic, &episode.id).await;
                memories_weakened += 1;
            }

            // Phase 4: Emotional re-evaluation for reconsolidated memories
            if episode.reconsolidation_count > 0 && episode.emotional_tag.arousal > 0.7 {
                self.re_evaluate_emotion(episodic, &episode.id).await;
                emotional_updates += 1;
            }
        }

        // Phase 5: Apply vividness decay across all memories
        episodic.decay_vividness(0.001).await;

        let duration_ms = start.elapsed().as_millis() as u64;

        let record = ReconsolidationRecord {
            id: record_id,
            timestamp: Utc::now(),
            episodes_processed,
            schemas_extracted,
            memories_strengthened,
            memories_weakened,
            emotional_updates,
            duration_ms,
            trigger,
        };

        // Store record
        self.history.write().await.push(record.clone());
        *self.total_cycles.write().await += 1;

        info!(
            "Reconsolidation complete: {} schemas, {} strengthened, {} weakened, {} emotional updates in {}ms",
            schemas_extracted, memories_strengthened, memories_weakened, emotional_updates, duration_ms
        );

        Ok(record)
    }

    /// Attempt to extract an abstract schema from a single episode.
    async fn try_extract_schema(
        &self,
        episode: &Episode,
        schema_library: &SchemaLibrary,
    ) -> Option<MemorySchema> {
        // Only extract schemas from episodes with multiple events and high importance
        if episode.events.len() < 2 || episode.importance < 0.5 {
            return None;
        }

        // Build a pattern description from event types
        let event_types: Vec<String> = episode
            .events
            .iter()
            .map(|e| format!("{:?}", e.event_type))
            .collect();

        let pattern = event_types.join("→");
        let schema_name = format!("pattern_{}", &uuid::Uuid::new_v4().to_string()[..8]);

        let schema = MemorySchema {
            id: uuid::Uuid::new_v4().to_string(),
            name: schema_name,
            description: format!(
                "Abstract pattern from episode: {}. Event sequence: {}",
                episode.context.goal.as_deref().unwrap_or("unknown"),
                pattern
            ),
            event_sequence_pattern: event_types,
            abstraction_level: 0.6,
            instance_count: 1,
            avg_emotional_valence: episode.emotional_tag.valence,
            predictive_power: 0.5,
            created_at: Utc::now(),
            last_updated: Utc::now(),
        };

        schema_library.add_or_merge_schema(schema.clone()).await;
        Some(schema)
    }

    /// Compute how relevant an episode is to the current active goals.
    fn compute_goal_relevance(&self, episode: &Episode, active_goals: &[String]) -> f64 {
        if active_goals.is_empty() {
            return 0.3;
        }

        let episode_text = format!(
            "{} {}",
            episode.context.goal.as_deref().unwrap_or(""),
            episode.events.iter().map(|e| e.description.as_str()).collect::<Vec<_>>().join(" ")
        );

        let ep_words: std::collections::HashSet<&str> = episode_text.split_whitespace().collect();

        let max_relevance = active_goals
            .iter()
            .map(|g| {
                let goal_words: std::collections::HashSet<&str> = g.split_whitespace().collect();
                let intersection = ep_words.intersection(&goal_words).count();
                let union = ep_words.union(&goal_words).count();
                if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
            })
            .fold(0.0_f64, f64::max);

        max_relevance
    }

    /// Strengthen a memory: increase importance and vividness.
    async fn strengthen_memory(&self, episodic: &EpisodicMemory, episode_id: &str, boost: f64) {
        let mut episodes = episodic.episodes.write().await;
        if let Some(ep) = episodes.iter_mut().find(|e| e.id == episode_id) {
            ep.importance = (ep.importance + boost * 0.05).min(1.0);
            ep.vividness = (ep.vividness + 0.1).min(1.0);
            ep.reconsolidation_count += 1;
        }
    }

    /// Weaken a memory: decrease importance and vividness.
    async fn weaken_memory(&self, episodic: &EpisodicMemory, episode_id: &str) {
        let mut episodes = episodic.episodes.write().await;
        if let Some(ep) = episodes.iter_mut().find(|e| e.id == episode_id) {
            ep.importance = (ep.importance * 0.95).max(0.01);
            ep.vividness = (ep.vividness * 0.90).max(0.01);
            ep.reconsolidation_count += 1;
        }
    }

    /// Re-evaluate and dampen high-arousal emotional tags (exposure effect).
    async fn re_evaluate_emotion(&self, episodic: &EpisodicMemory, episode_id: &str) {
        let mut episodes = episodic.episodes.write().await;
        if let Some(ep) = episodes.iter_mut().find(|e| e.id == episode_id) {
            // Dampen arousal (habituation), preserve valence direction
            ep.emotional_tag.arousal = (ep.emotional_tag.arousal * 0.85).max(0.1);
            ep.reconsolidation_count += 1;
        }
    }

    /// Get statistics.
    pub async fn get_stats(&self) -> ReconsolidationStats {
        let history = self.history.read().await;
        let cycles = self.total_cycles.read().await;

        let total_schemas: usize = history.iter().map(|r| r.schemas_extracted).sum();
        let total_strengthened: usize = history.iter().map(|r| r.memories_strengthened).sum();
        let total_weakened: usize = history.iter().map(|r| r.memories_weakened).sum();

        ReconsolidationStats {
            total_cycles: *cycles,
            total_schemas_extracted: total_schemas,
            total_memories_strengthened: total_strengthened,
            total_memories_weakened: total_weakened,
        }
    }
}

impl Default for MemoryReconsolidator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconsolidationStats {
    pub total_cycles: u64,
    pub total_schemas_extracted: usize,
    pub total_memories_strengthened: usize,
    pub total_memories_weakened: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::housaky::memory::episodic::{EpisodicMemory, EpisodicEventType};
    use crate::housaky::memory::EmotionalTag;

    #[tokio::test]
    async fn test_reconsolidation() {
        let episodic = EpisodicMemory::new(100);
        let schema_library = SchemaLibrary::new();
        let reconsolidator = MemoryReconsolidator::new();

        // Create a test episode
        episodic.begin_episode(Some("solve complex problem".to_string()), "test").await;
        episodic.record_event(EpisodicEventType::GoalSet, "set goal: solve complex problem", 0.8).await;
        episodic.record_event(EpisodicEventType::ReasoningStep, "applied chain-of-thought reasoning", 0.7).await;
        episodic.record_event(EpisodicEventType::GoalAchieved, "goal achieved successfully", 0.9).await;
        episodic.end_episode(EmotionalTag::positive(0.8), true).await;

        let result = reconsolidator
            .reconsolidate(
                &episodic,
                &schema_library,
                &["solve complex problem".to_string()],
                ReconsolidationTrigger::DreamCycle,
            )
            .await;

        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.episodes_processed, 1);
    }
}

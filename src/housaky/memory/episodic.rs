//! Episodic Memory — Temporally-ordered episodes with causal links and emotional tags.
//!
//! Implements first-person episodic memory: sequences of events with timestamps,
//! emotional context, causal chains, and retrieval count tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::emotional_tags::EmotionalTag;

// ── Core Types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
    pub context: EpisodicContext,
    pub events: Vec<EpisodicEvent>,
    pub emotional_tag: EmotionalTag,
    pub causal_links: Vec<String>,
    pub retrieval_count: u32,
    pub last_retrieved: Option<DateTime<Utc>>,
    pub reconsolidation_count: u32,
    pub schema_id: Option<String>,
    pub importance: f64,
    pub vividness: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicContext {
    pub goal: Option<String>,
    pub location: Option<String>,
    pub active_tools: Vec<String>,
    pub cognitive_load: f64,
    pub phase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: EpisodicEventType,
    pub description: String,
    pub outcome: Option<String>,
    pub significance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EpisodicEventType {
    GoalSet,
    GoalAchieved,
    GoalFailed,
    ActionTaken,
    ToolUsed,
    KnowledgeAcquired,
    ErrorEncountered,
    InsightGained,
    SelfModification,
    UserInteraction,
    ReasoningStep,
    MemoryAccess,
}

// ── Episodic Memory Store ─────────────────────────────────────────────────────

pub struct EpisodicMemory {
    pub episodes: Arc<RwLock<Vec<Episode>>>,
    pub index_by_goal: Arc<RwLock<HashMap<String, Vec<String>>>>,
    pub index_by_schema: Arc<RwLock<HashMap<String, Vec<String>>>>,
    pub current_episode: Arc<RwLock<Option<Episode>>>,
    max_episodes: usize,
}

impl EpisodicMemory {
    pub fn new(max_episodes: usize) -> Self {
        Self {
            episodes: Arc::new(RwLock::new(Vec::new())),
            index_by_goal: Arc::new(RwLock::new(HashMap::new())),
            index_by_schema: Arc::new(RwLock::new(HashMap::new())),
            current_episode: Arc::new(RwLock::new(None)),
            max_episodes,
        }
    }

    /// Begin recording a new episode.
    pub async fn begin_episode(&self, goal: Option<String>, phase: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let episode = Episode {
            id: id.clone(),
            timestamp: Utc::now(),
            duration_ms: 0,
            context: EpisodicContext {
                goal: goal.clone(),
                location: None,
                active_tools: Vec::new(),
                cognitive_load: 0.0,
                phase: phase.to_string(),
            },
            events: Vec::new(),
            emotional_tag: EmotionalTag::neutral(),
            causal_links: Vec::new(),
            retrieval_count: 0,
            last_retrieved: None,
            reconsolidation_count: 0,
            schema_id: None,
            importance: 0.5,
            vividness: 1.0,
        };

        *self.current_episode.write().await = Some(episode);
        debug!("EpisodicMemory: began episode {} (goal={:?})", id, goal);
        id
    }

    /// Append an event to the current open episode.
    pub async fn record_event(
        &self,
        event_type: EpisodicEventType,
        description: &str,
        significance: f64,
    ) {
        let mut current = self.current_episode.write().await;
        if let Some(ep) = current.as_mut() {
            ep.events.push(EpisodicEvent {
                timestamp: Utc::now(),
                event_type,
                description: description.to_string(),
                outcome: None,
                significance: significance.clamp(0.0, 1.0),
            });
        }
    }

    /// Record an event with an explicit outcome.
    pub async fn record_event_with_outcome(
        &self,
        event_type: EpisodicEventType,
        description: &str,
        outcome: &str,
        significance: f64,
    ) {
        let mut current = self.current_episode.write().await;
        if let Some(ep) = current.as_mut() {
            ep.events.push(EpisodicEvent {
                timestamp: Utc::now(),
                event_type,
                description: description.to_string(),
                outcome: Some(outcome.to_string()),
                significance: significance.clamp(0.0, 1.0),
            });
        }
    }

    /// Close the current episode and store it.
    pub async fn end_episode(&self, emotional_tag: EmotionalTag, success: bool) -> Option<String> {
        let start = std::time::Instant::now();
        let mut current = self.current_episode.write().await;

        if let Some(mut ep) = current.take() {
            let duration = (Utc::now() - ep.timestamp).num_milliseconds().unsigned_abs();
            ep.duration_ms = duration;
            ep.emotional_tag = emotional_tag;

            // Compute importance from event significance + emotional intensity
            let avg_sig: f64 = if ep.events.is_empty() {
                0.3
            } else {
                ep.events.iter().map(|e| e.significance).sum::<f64>() / ep.events.len() as f64
            };
            ep.importance = (avg_sig * 0.6 + ep.emotional_tag.intensity() * 0.4).clamp(0.0, 1.0);
            if success { ep.importance = (ep.importance + 0.1).min(1.0); }

            let episode_id = ep.id.clone();
            let goal = ep.context.goal.clone();
            let schema_id = ep.schema_id.clone();

            // Update indices
            if let Some(ref g) = goal {
                self.index_by_goal
                    .write()
                    .await
                    .entry(g.clone())
                    .or_default()
                    .push(episode_id.clone());
            }
            if let Some(ref s) = schema_id {
                self.index_by_schema
                    .write()
                    .await
                    .entry(s.clone())
                    .or_default()
                    .push(episode_id.clone());
            }

            let ep_importance = ep.importance;
            let ep_event_count = ep.events.len();

            // Store episode with forgetting if at capacity
            {
                let mut episodes = self.episodes.write().await;
                episodes.push(ep);
                if episodes.len() > self.max_episodes {
                    self.apply_forgetting(&mut episodes);
                }
            }

            let consolidation_us = start.elapsed().as_micros();
            info!(
                "EpisodicMemory: stored episode {} ({} events, importance={:.2}, consolidation={}µs)",
                episode_id,
                ep_event_count,
                ep_importance,
                consolidation_us
            );
            Some(episode_id)
        } else {
            None
        }
    }

    /// Retrieve the most relevant episodes for a query.
    pub async fn retrieve(&self, query: &str, limit: usize) -> Vec<Episode> {
        let mut episodes = self.episodes.write().await;

        // Simple relevance: keyword overlap + recency + importance
        let query_words: std::collections::HashSet<&str> = query.split_whitespace().collect();

        let mut scored: Vec<(usize, f64)> = episodes
            .iter()
            .enumerate()
            .map(|(i, ep)| {
                let text = format!(
                    "{} {}",
                    ep.context.goal.as_deref().unwrap_or(""),
                    ep.events.iter().map(|e| e.description.as_str()).collect::<Vec<_>>().join(" ")
                );
                let ep_words: std::collections::HashSet<&str> = text.split_whitespace().collect();
                let overlap = query_words.intersection(&ep_words).count() as f64;
                let keyword_score = if query_words.is_empty() {
                    0.0
                } else {
                    overlap / query_words.len() as f64
                };

                // Recency score (exponential decay over 1000 episodes)
                let recency = (-0.001 * i as f64).exp();

                let score = keyword_score * 0.5 + recency * 0.2 + ep.importance * 0.3;
                (i, score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let result: Vec<Episode> = scored
            .iter()
            .take(limit)
            .map(|(i, _)| {
                episodes[*i].retrieval_count += 1;
                episodes[*i].last_retrieved = Some(Utc::now());
                episodes[*i].clone()
            })
            .collect();

        result
    }

    /// Retrieve episodes by goal.
    pub async fn retrieve_by_goal(&self, goal: &str) -> Vec<Episode> {
        let index = self.index_by_goal.read().await;
        let ids = match index.get(goal) {
            Some(ids) => ids.clone(),
            None => return vec![],
        };
        drop(index);

        let mut episodes = self.episodes.write().await;
        let result: Vec<Episode> = episodes
            .iter_mut()
            .filter(|ep| ids.contains(&ep.id))
            .map(|ep| {
                ep.retrieval_count += 1;
                ep.last_retrieved = Some(Utc::now());
                ep.clone()
            })
            .collect();
        result
    }

    /// Assign a schema to an episode (called by reconsolidation).
    pub async fn assign_schema(&self, episode_id: &str, schema_id: &str) {
        let mut episodes = self.episodes.write().await;
        if let Some(ep) = episodes.iter_mut().find(|e| e.id == episode_id) {
            ep.schema_id = Some(schema_id.to_string());
        }
        drop(episodes);

        self.index_by_schema
            .write()
            .await
            .entry(schema_id.to_string())
            .or_default()
            .push(episode_id.to_string());
    }

    /// Add a causal link between two episodes.
    pub async fn add_causal_link(&self, from_id: &str, to_id: &str) {
        let mut episodes = self.episodes.write().await;
        if let Some(ep) = episodes.iter_mut().find(|e| e.id == from_id) {
            if !ep.causal_links.contains(&to_id.to_string()) {
                ep.causal_links.push(to_id.to_string());
            }
        }
    }

    /// Interference-based forgetting: remove lowest-importance, least-retrieved episodes.
    fn apply_forgetting(&self, episodes: &mut Vec<Episode>) {
        // Forgetting score: low importance + low retrieval count + high age
        episodes.sort_by(|a, b| {
            let score_a = a.importance * 0.5 + (a.retrieval_count as f64 * 0.01).min(0.3);
            let score_b = b.importance * 0.5 + (b.retrieval_count as f64 * 0.01).min(0.3);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        // Remove weakest 10%
        let remove_count = (episodes.len() as f64 * 0.1) as usize;
        episodes.truncate(episodes.len().saturating_sub(remove_count));
    }

    /// Weaken memory traces (vividness decay) — called periodically.
    pub async fn decay_vividness(&self, decay_rate: f64) {
        let mut episodes = self.episodes.write().await;
        for ep in episodes.iter_mut() {
            ep.vividness = (ep.vividness * (1.0 - decay_rate)).max(0.01);
        }
    }

    /// Get statistics.
    pub async fn get_stats(&self) -> EpisodicStats {
        let episodes = self.episodes.read().await;
        let total = episodes.len();
        let avg_importance = if total > 0 {
            episodes.iter().map(|e| e.importance).sum::<f64>() / total as f64
        } else {
            0.0
        };
        let positive_episodes = episodes.iter().filter(|e| e.emotional_tag.valence > 0.1).count();
        let negative_episodes = episodes.iter().filter(|e| e.emotional_tag.valence < -0.1).count();
        let total_events: usize = episodes.iter().map(|e| e.events.len()).sum();

        EpisodicStats {
            total_episodes: total,
            avg_importance,
            positive_episodes,
            negative_episodes,
            total_events,
            max_capacity: self.max_episodes,
        }
    }
}

impl Default for EpisodicMemory {
    fn default() -> Self {
        Self::new(10_000)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicStats {
    pub total_episodes: usize,
    pub avg_importance: f64,
    pub positive_episodes: usize,
    pub negative_episodes: usize,
    pub total_events: usize,
    pub max_capacity: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_episode_lifecycle() {
        let mem = EpisodicMemory::new(100);
        let _id = mem.begin_episode(Some("test goal".to_string()), "test").await;
        mem.record_event(EpisodicEventType::GoalSet, "set goal: test", 0.6).await;
        mem.record_event_with_outcome(
            EpisodicEventType::GoalAchieved,
            "achieved test goal",
            "success",
            0.8,
        ).await;
        let stored = mem.end_episode(EmotionalTag::positive(0.7), true).await;
        assert!(stored.is_some());

        let stats = mem.get_stats().await;
        assert_eq!(stats.total_episodes, 1);
        assert_eq!(stats.total_events, 2);
        assert!(stats.avg_importance > 0.0);
    }

    #[tokio::test]
    async fn test_retrieve() {
        let mem = EpisodicMemory::new(100);
        mem.begin_episode(Some("solve problem".to_string()), "test").await;
        mem.record_event(EpisodicEventType::ReasoningStep, "solve the problem", 0.7).await;
        mem.end_episode(EmotionalTag::positive(0.5), true).await;

        let results = mem.retrieve("solve problem", 5).await;
        assert!(!results.is_empty());
    }
}

use crate::housaky::memory::agent_memory::{AgentMemoryRecord, AgentMemoryStore, MemoryKind};
use crate::housaky::memory::episodic::EpisodicMemory;
use crate::housaky::memory::hierarchical::{
    Episode, HierarchicalMemory, ProcedureStep, TriggerCondition,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

pub struct MemoryConsolidator {
    memory: Arc<HierarchicalMemory>,
    agent_memory: Arc<AgentMemoryStore>,
    episodic_memory: Option<Arc<EpisodicMemory>>,
    workspace_dir: PathBuf,
    consolidation_interval: std::time::Duration,
    last_consolidation: Arc<RwLock<chrono::DateTime<chrono::Utc>>>,
    consolidation_stats: Arc<RwLock<ConsolidationStats>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationStats {
    pub total_consolidations: u64,
    pub episodes_processed: u64,
    pub facts_created: u64,
    pub procedures_extracted: u64,
    pub skills_promoted: u64,
    pub episodic_events_promoted: u64,
    pub last_consolidation: chrono::DateTime<chrono::Utc>,
}

impl Default for ConsolidationStats {
    fn default() -> Self {
        Self {
            total_consolidations: 0,
            episodes_processed: 0,
            facts_created: 0,
            procedures_extracted: 0,
            skills_promoted: 0,
            episodic_events_promoted: 0,
            last_consolidation: chrono::Utc::now(),
        }
    }
}

impl MemoryConsolidator {
    pub fn new(memory: Arc<HierarchicalMemory>, workspace_dir: &PathBuf) -> Self {
        let agent_memory = AgentMemoryStore::open(workspace_dir)
            .unwrap_or_else(|e| {
                warn!("Failed to open AgentMemoryStore: {e} — using in-memory fallback");
                AgentMemoryStore::open(&std::env::temp_dir()).expect("fallback AgentMemoryStore")
            });
        Self {
            memory,
            agent_memory: Arc::new(agent_memory),
            episodic_memory: None,
            workspace_dir: workspace_dir.clone(),
            consolidation_interval: std::time::Duration::from_secs(300),
            last_consolidation: Arc::new(RwLock::new(chrono::Utc::now())),
            consolidation_stats: Arc::new(RwLock::new(ConsolidationStats::default())),
        }
    }

    /// Attach an episodic memory source so its high-importance events get
    /// promoted into the persistent AgentMemoryStore during consolidation.
    pub fn with_episodic(mut self, episodic: Arc<EpisodicMemory>) -> Self {
        self.episodic_memory = Some(episodic);
        self
    }

    pub async fn run_periodic_consolidation(&self) -> Result<()> {
        info!("Starting memory consolidation cycle...");

        let mut stats = self.consolidation_stats.write().await;
        stats.total_consolidations += 1;
        stats.last_consolidation = chrono::Utc::now();

        self.decay_memories()?;
        self.consolidate_episodes().await?;
        self.consolidate_episodic_memories().await?;
        self.extract_procedures().await?;
        self.promote_skills().await?;
        self.save_consolidation_state().await?;

        *self.last_consolidation.write().await = chrono::Utc::now();

        info!("Memory consolidation complete");
        Ok(())
    }

    /// §4.9 — Importance-weighted memory decay with catastrophic-forgetting
    /// protection.  High-importance and high-access memories are shielded from
    /// decay (analogous to Elastic Weight Consolidation's Fisher-information
    /// weighting).  Only low-importance, rarely-accessed memories lose confidence
    /// over time.
    fn decay_memories(&self) -> Result<()> {
        info!("Applying importance-weighted memory decay...");

        // Retrieve all stored memories grouped by kind.
        for kind in &[
            MemoryKind::Fact,
            MemoryKind::Pattern,
            MemoryKind::Procedure,
            MemoryKind::Skill,
            MemoryKind::Insight,
            MemoryKind::Experience,
        ] {
            let records = self
                .agent_memory
                .recall_by_kind(kind, 200)
                .unwrap_or_default();

            for record in &records {
                // Protection score ∈ [0, 1]: high importance + high access → protected.
                let access_factor = (record.access_count as f64 / 10.0).min(1.0);
                let protection = record.importance * 0.6 + access_factor * 0.4;

                // Only decay if protection is low.
                if protection < 0.4 {
                    let age_days = (chrono::Utc::now() - record.created_at).num_days() as f64;
                    let decay_factor = (-0.01 * age_days).exp(); // exponential decay
                    let new_confidence = (record.confidence * decay_factor).max(0.05);

                    if new_confidence < record.confidence - 0.01 {
                        let mut updated = record.clone();
                        updated.confidence = new_confidence;
                        // Re-store to update confidence in place.
                        let _ = self.agent_memory.store(&updated);
                    }
                }
            }
        }

        Ok(())
    }

    /// §4.9 — Rehearsal: re-access critical memories to keep them from fading.
    /// This is the "experience replay" analog for the memory store — high-value
    /// memories get their access_count bumped so they remain protected from decay.
    pub fn rehearse_critical_memories(&self, top_n: usize) -> usize {
        let mut rehearsed = 0;
        for kind in &[MemoryKind::Skill, MemoryKind::Pattern, MemoryKind::Procedure] {
            let records = self
                .agent_memory
                .recall_by_kind(kind, top_n)
                .unwrap_or_default();

            for record in records.iter().filter(|r| r.importance >= 0.7) {
                let mut updated = record.clone();
                updated.access_count += 1;
                updated.accessed_at = chrono::Utc::now();
                let _ = self.agent_memory.store(&updated);
                rehearsed += 1;
            }
        }
        if rehearsed > 0 {
            info!("Rehearsed {} critical memories to prevent forgetting", rehearsed);
        }
        rehearsed
    }

    async fn consolidate_episodes(&self) -> Result<()> {
        info!("Consolidating episodic memories...");

        let episodes = self.memory.get_recent_episodes(50).await;
        let patterns = self.analyze_patterns(&episodes);

        for pattern in &patterns {
            let record = AgentMemoryRecord {
                id: uuid::Uuid::new_v4().to_string(),
                kind: MemoryKind::Pattern,
                content: pattern.description.clone(),
                source: "consolidation".to_string(),
                confidence: pattern.success_rate,
                importance: pattern.success_rate * (pattern.occurrence_count as f64 / 10.0).min(1.0),
                tags: vec![format!("{:?}", pattern.pattern_type)],
                created_at: chrono::Utc::now(),
                accessed_at: chrono::Utc::now(),
                access_count: 0,
            };
            if let Err(e) = self.agent_memory.store(&record) {
                warn!("Failed to persist pattern to AgentMemoryStore: {e}");
            }
        }

        let mut stats = self.consolidation_stats.write().await;
        stats.episodes_processed += episodes.len() as u64;
        stats.facts_created += patterns.len() as u64;

        Ok(())
    }

    /// Promote high-importance episodic memory events into the persistent
    /// AgentMemoryStore so the agent retains key experiences across sessions.
    async fn consolidate_episodic_memories(&self) -> Result<()> {
        let episodic = match &self.episodic_memory {
            Some(ep) => ep,
            None => return Ok(()),
        };

        info!("Consolidating episodic memories into AgentMemoryStore...");

        // Retrieve top episodes by importance.
        let episodes = episodic.episodes.read().await;
        let mut promoted = 0u64;

        for ep in episodes.iter().filter(|e| e.importance >= 0.6) {
            // Build a summary from the episode's events.
            let event_summary: String = ep
                .events
                .iter()
                .take(5)
                .map(|e| {
                    let outcome = e.outcome.as_deref().unwrap_or("?");
                    format!("{:?}: {} → {}", e.event_type, e.description.chars().take(80).collect::<String>(), outcome)
                })
                .collect::<Vec<_>>()
                .join("; ");

            let goal_ctx = ep.context.goal.as_deref().unwrap_or("(none)");
            let content = format!(
                "Episode [{}] goal={} events=[{}] importance={:.2} emotion=v{:.2}/a{:.2}",
                ep.timestamp.format("%Y-%m-%d %H:%M"),
                goal_ctx,
                event_summary,
                ep.importance,
                ep.emotional_tag.valence,
                ep.emotional_tag.arousal,
            );

            // Determine the memory kind from the episode's dominant event type.
            let kind = if ep.events.iter().any(|e| e.event_type == crate::housaky::memory::episodic::EpisodicEventType::ErrorEncountered) {
                MemoryKind::Pattern // errors → patterns to avoid
            } else if ep.events.iter().any(|e| e.event_type == crate::housaky::memory::episodic::EpisodicEventType::InsightGained) {
                MemoryKind::Insight
            } else {
                MemoryKind::Experience
            };

            let record = AgentMemoryRecord {
                id: format!("ep_{}", ep.id.chars().take(12).collect::<String>()),
                kind,
                content,
                source: "episodic_consolidation".to_string(),
                confidence: ep.importance,
                importance: ep.importance,
                tags: vec![
                    "episodic".to_string(),
                    format!("goal:{}", goal_ctx.chars().take(30).collect::<String>()),
                ],
                created_at: ep.timestamp,
                accessed_at: chrono::Utc::now(),
                access_count: 0,
            };

            if let Err(e) = self.agent_memory.store(&record) {
                warn!("Failed to promote episodic event: {e}");
            } else {
                promoted += 1;
            }
        }

        let mut stats = self.consolidation_stats.write().await;
        stats.episodic_events_promoted += promoted;

        if promoted > 0 {
            info!("Promoted {} episodic events to AgentMemoryStore", promoted);
        }

        Ok(())
    }

    async fn extract_procedures(&self) -> Result<()> {
        info!("Extracting procedures from execution patterns...");

        let episodes = self.memory.get_recent_episodes(50).await;
        let patterns = self.analyze_patterns(&episodes);
        let mut extracted = 0u64;

        for pattern in patterns.iter().filter(|p| p.success_rate > 0.7 && p.occurrence_count >= 3) {
            let steps_text: String = pattern
                .description
                .split(" -> ")
                .enumerate()
                .map(|(i, s)| format!("{}. {}", i + 1, s))
                .collect::<Vec<_>>()
                .join(" | ");

            let record = AgentMemoryRecord {
                id: uuid::Uuid::new_v4().to_string(),
                kind: MemoryKind::Procedure,
                content: format!("Procedure: {} — Steps: {}", pattern.description, steps_text),
                source: "consolidation".to_string(),
                confidence: pattern.success_rate,
                importance: 0.7 + pattern.success_rate * 0.3,
                tags: vec!["procedure".to_string()],
                created_at: chrono::Utc::now(),
                accessed_at: chrono::Utc::now(),
                access_count: 0,
            };
            if let Err(e) = self.agent_memory.store(&record) {
                warn!("Failed to persist procedure to AgentMemoryStore: {e}");
            } else {
                extracted += 1;
            }
        }

        let mut stats = self.consolidation_stats.write().await;
        stats.procedures_extracted += extracted;

        Ok(())
    }

    async fn promote_skills(&self) -> Result<()> {
        info!("Checking for skills ready for promotion...");

        // Promote procedures that have been accessed frequently into skills.
        let procedures = self
            .agent_memory
            .recall_by_kind(&MemoryKind::Procedure, 20)
            .unwrap_or_default();

        let mut promoted = 0u64;
        for proc in procedures.iter().filter(|p| p.access_count >= 3 && p.confidence >= 0.8) {
            let record = AgentMemoryRecord {
                id: uuid::Uuid::new_v4().to_string(),
                kind: MemoryKind::Skill,
                content: format!("Skill promoted from procedure: {}", proc.content.chars().take(200).collect::<String>()),
                source: "skill_promotion".to_string(),
                confidence: proc.confidence,
                importance: (proc.importance + 0.1).min(1.0),
                tags: vec!["skill".to_string()],
                created_at: chrono::Utc::now(),
                accessed_at: chrono::Utc::now(),
                access_count: 0,
            };
            if let Err(e) = self.agent_memory.store(&record) {
                warn!("Failed to promote skill in AgentMemoryStore: {e}");
            } else {
                promoted += 1;
            }
        }

        let mut stats = self.consolidation_stats.write().await;
        stats.skills_promoted += promoted;

        Ok(())
    }

    /// Persist a standalone memory record — used by callers that want to
    /// directly write a fact or observation into the agent memory store.
    pub fn persist_memory(&self, record: AgentMemoryRecord) -> Result<()> {
        self.agent_memory.store(&record)
    }

    /// Retrieve the top-N records of a given kind for use in reasoning.
    pub fn recall(&self, kind: &MemoryKind, limit: usize) -> Vec<AgentMemoryRecord> {
        self.agent_memory
            .recall_by_kind(kind, limit)
            .unwrap_or_default()
    }

    /// Search the agent memory store.
    pub fn search_memory(&self, query: &str, limit: usize) -> Vec<AgentMemoryRecord> {
        self.agent_memory.search(query, limit).unwrap_or_default()
    }

    pub fn analyze_patterns(&self, episodes: &[Episode]) -> Vec<ExtractedPattern> {
        let mut patterns = Vec::new();

        let mut action_sequences: HashMap<String, Vec<&Episode>> = HashMap::new();

        for episode in episodes {
            if episode.actions.is_empty() {
                continue;
            }

            let key: String = episode
                .actions
                .iter()
                .map(|a| a.action_type.clone())
                .collect::<Vec<_>>()
                .join(" -> ");

            action_sequences.entry(key).or_default().push(episode);
        }

        for (sequence, matching_episodes) in action_sequences {
            if matching_episodes.len() >= 3 {
                let success_rate = matching_episodes
                    .iter()
                    .filter(|e| e.outcome.success)
                    .count() as f64
                    / matching_episodes.len() as f64;

                if success_rate > 0.6 {
                    patterns.push(ExtractedPattern {
                        pattern_type: PatternType::ActionSequence,
                        description: sequence.clone(),
                        occurrence_count: matching_episodes.len(),
                        success_rate,
                        examples: matching_episodes
                            .iter()
                            .take(3)
                            .map(|e| e.context.clone())
                            .collect(),
                    });
                }
            }
        }

        let mut context_patterns: HashMap<String, Vec<&Episode>> = HashMap::new();

        for episode in episodes {
            let keywords = self.extract_keywords(&episode.context);
            for keyword in keywords {
                context_patterns.entry(keyword).or_default().push(episode);
            }
        }

        for (keyword, matching_episodes) in context_patterns {
            if matching_episodes.len() >= 5 {
                let success_rate = matching_episodes
                    .iter()
                    .filter(|e| e.outcome.success)
                    .count() as f64
                    / matching_episodes.len() as f64;

                if success_rate > 0.5 {
                    patterns.push(ExtractedPattern {
                        pattern_type: PatternType::ContextKeyword,
                        description: format!("Context contains: {}", keyword),
                        occurrence_count: matching_episodes.len(),
                        success_rate,
                        examples: matching_episodes
                            .iter()
                            .take(3)
                            .map(|e| e.context.chars().take(100).collect())
                            .collect(),
                    });
                }
            }
        }

        patterns.sort_by(|a, b| {
            b.success_rate
                .partial_cmp(&a.success_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        patterns
    }

    fn extract_keywords(&self, text: &str) -> Vec<String> {
        let stop_words = [
            "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "could", "should", "may", "might", "must",
            "shall", "can", "need", "dare", "ought", "used", "to", "of", "in", "for", "on", "with",
            "at", "by", "from", "as", "into", "through", "during", "before", "after", "above",
            "below", "between", "i", "me", "my", "we", "our", "you", "your", "he", "him", "his",
            "she", "her", "it", "its", "they", "them", "their", "this", "that", "these", "those",
        ];

        text.to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 3 && !stop_words.contains(w))
            .take(10)
            .map(|w| w.to_string())
            .collect()
    }

    pub async fn create_procedure_from_pattern(
        &self,
        pattern: &ExtractedPattern,
    ) -> Result<String> {
        let steps: Vec<ProcedureStep> = pattern
            .description
            .split(" -> ")
            .enumerate()
            .map(|(i, action)| ProcedureStep {
                order: u32::try_from(i).unwrap_or(u32::MAX) + 1,
                action: action.to_string(),
                parameters: std::collections::HashMap::new(),
                expected_outcome: "Task completed successfully".to_string(),
                error_handling: Some("Retry or ask for clarification".to_string()),
                optional: false,
            })
            .collect();

        let triggers = vec![TriggerCondition {
            condition_type: "context_match".to_string(),
            pattern: pattern.examples.first().unwrap_or(&String::new()).clone(),
            context_requirements: vec![],
        }];

        self.memory
            .store_procedure(&pattern.description, steps, triggers)
            .await
    }

    async fn save_consolidation_state(&self) -> Result<()> {
        let state_dir = self.workspace_dir.join(".housaky").join("memory");
        tokio::fs::create_dir_all(&state_dir).await?;

        let stats = self.consolidation_stats.read().await;
        let stats_json = serde_json::to_string_pretty(&*stats)?;
        tokio::fs::write(state_dir.join("consolidation_stats.json"), stats_json).await?;

        Ok(())
    }

    pub async fn get_stats(&self) -> ConsolidationStats {
        self.consolidation_stats.read().await.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedPattern {
    pub pattern_type: PatternType,
    pub description: String,
    pub occurrence_count: usize,
    pub success_rate: f64,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PatternType {
    ActionSequence,
    ContextKeyword,
    OutcomePattern,
    ErrorRecovery,
    UserPreference,
}

pub fn start_background_consolidation(
    consolidator: Arc<MemoryConsolidator>,
    interval_secs: u64,
) -> Result<tokio::task::JoinHandle<()>> {
    let handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));

        loop {
            interval.tick().await;

            if let Err(e) = consolidator.run_periodic_consolidation().await {
                tracing::error!("Consolidation error: {}", e);
            }
        }
    });

    Ok(handle)
}

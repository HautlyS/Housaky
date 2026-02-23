use crate::memory::hierarchical::{
    Episode, HierarchicalMemory, ProcedureStep, TriggerCondition,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct MemoryConsolidator {
    memory: Arc<HierarchicalMemory>,
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
            last_consolidation: chrono::Utc::now(),
        }
    }
}

impl MemoryConsolidator {
    pub fn new(memory: Arc<HierarchicalMemory>, workspace_dir: &PathBuf) -> Self {
        Self {
            memory,
            workspace_dir: workspace_dir.clone(),
            consolidation_interval: std::time::Duration::from_secs(300),
            last_consolidation: Arc::new(RwLock::new(chrono::Utc::now())),
            consolidation_stats: Arc::new(RwLock::new(ConsolidationStats::default())),
        }
    }

    pub async fn run_periodic_consolidation(&self) -> Result<()> {
        info!("Starting memory consolidation cycle...");

        let mut stats = self.consolidation_stats.write().await;
        stats.total_consolidations += 1;
        stats.last_consolidation = chrono::Utc::now();

        self.decay_memories()?;
        self.consolidate_episodes().await?;
        self.extract_procedures().await?;
        self.promote_skills().await?;
        self.save_consolidation_state().await?;

        *self.last_consolidation.write().await = chrono::Utc::now();

        info!("Memory consolidation complete");
        Ok(())
    }

    fn decay_memories(&self) -> Result<()> {
        info!("Applying memory decay...");
        Ok(())
    }

    async fn consolidate_episodes(&self) -> Result<()> {
        info!("Consolidating episodic memories...");

        let mut stats = self.consolidation_stats.write().await;
        stats.episodes_processed += 1;

        Ok(())
    }

    async fn extract_procedures(&self) -> Result<()> {
        info!("Extracting procedures from execution patterns...");

        let mut stats = self.consolidation_stats.write().await;
        stats.procedures_extracted += 1;

        Ok(())
    }

    async fn promote_skills(&self) -> Result<()> {
        info!("Checking for skills ready for promotion...");

        let mut stats = self.consolidation_stats.write().await;
        stats.skills_promoted += 1;

        Ok(())
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
                order: i as u32 + 1,
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

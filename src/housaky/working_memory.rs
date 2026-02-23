use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

const DEFAULT_TOKEN_BUDGET: usize = 8000;
const SHORT_TERM_CAPACITY: usize = 10;
const WORKING_MEMORY_DECAY: f64 = 0.95;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryImportance {
    Critical,
    High,
    Normal,
    Low,
    Transient,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryItem {
    pub id: String,
    pub content: String,
    pub importance: MemoryImportance,
    pub access_count: u64,
    pub last_accessed: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub decay_factor: f64,
    pub associated_entities: Vec<String>,
    pub context: HashMap<String, String>,
    pub token_count: usize,
    pub compressed: bool,
    pub compression_ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortTermMemory {
    pub items: VecDeque<WorkingMemoryItem>,
    pub max_items: usize,
    pub total_tokens: usize,
    pub token_budget: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongTermMemory {
    pub consolidated: Vec<WorkingMemoryItem>,
    pub summaries: Vec<MemorySummary>,
    pub associations: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySummary {
    pub id: String,
    pub original_ids: Vec<String>,
    pub summary: String,
    pub created_at: DateTime<Utc>,
    pub importance: MemoryImportance,
    pub token_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicMemory {
    pub id: String,
    pub event_type: String,
    pub description: String,
    pub participants: Vec<String>,
    pub outcome: String,
    pub timestamp: DateTime<Utc>,
    pub learned: Vec<String>,
    pub emotion: Option<String>,
}

pub struct WorkingMemoryEngine {
    short_term: Arc<RwLock<ShortTermMemory>>,
    long_term: Arc<RwLock<LongTermMemory>>,
    episodic: Arc<RwLock<Vec<EpisodicMemory>>>,
    token_budget: usize,
    consolidation_threshold: u64,
}

impl WorkingMemoryEngine {
    pub fn new() -> Self {
        Self {
            short_term: Arc::new(RwLock::new(ShortTermMemory {
                items: VecDeque::new(),
                max_items: SHORT_TERM_CAPACITY,
                total_tokens: 0,
                token_budget: DEFAULT_TOKEN_BUDGET,
            })),
            long_term: Arc::new(RwLock::new(LongTermMemory {
                consolidated: Vec::new(),
                summaries: Vec::new(),
                associations: HashMap::new(),
            })),
            episodic: Arc::new(RwLock::new(Vec::new())),
            token_budget: DEFAULT_TOKEN_BUDGET,
            consolidation_threshold: 5,
        }
    }

    pub async fn add(
        &self,
        content: &str,
        importance: MemoryImportance,
        context: HashMap<String, String>,
    ) -> Result<String> {
        let token_count = self.estimate_tokens(content);

        let item = WorkingMemoryItem {
            id: format!("mem_{}", uuid::Uuid::new_v4()),
            content: content.to_string(),
            importance: importance.clone(),
            access_count: 1,
            last_accessed: Utc::now(),
            created_at: Utc::now(),
            decay_factor: 1.0,
            associated_entities: Vec::new(),
            context,
            token_count,
            compressed: false,
            compression_ratio: None,
        };

        let id = item.id.clone();

        let mut stm = self.short_term.write().await;

        while stm.total_tokens + token_count > stm.token_budget && !stm.items.is_empty() {
            if let Some(evicted) = self.evict_least_important(&mut stm) {
                self.consolidate_to_long_term(evicted).await;
            }
        }

        if stm.items.len() >= stm.max_items {
            if let Some(evicted) = self.evict_least_important(&mut stm) {
                self.consolidate_to_long_term(evicted).await;
            }
        }

        stm.items.push_back(item.clone());
        stm.total_tokens += token_count;

        info!(
            "Added to working memory: {} ({} tokens, importance: {:?})",
            id, token_count, importance
        );

        Ok(id)
    }

    fn estimate_tokens(&self, content: &str) -> usize {
        content.len() / 4
    }

    fn evict_least_important(&self, stm: &mut ShortTermMemory) -> Option<WorkingMemoryItem> {
        let mut min_score = f64::MAX;
        let mut evict_idx = None;

        for (i, item) in stm.items.iter().enumerate() {
            let score = self.calculate_retention_score(item);
            if score < min_score {
                min_score = score;
                evict_idx = Some(i);
            }
        }

        if let Some(idx) = evict_idx {
            let item = stm.items.remove(idx).unwrap();
            stm.total_tokens -= item.token_count;
            info!(
                "Evicted memory: {} (retention score: {:.2})",
                item.id, min_score
            );
            return Some(item);
        }

        None
    }

    fn calculate_retention_score(&self, item: &WorkingMemoryItem) -> f64 {
        let importance_score = match item.importance {
            MemoryImportance::Critical => 1.0,
            MemoryImportance::High => 0.8,
            MemoryImportance::Normal => 0.5,
            MemoryImportance::Low => 0.3,
            MemoryImportance::Transient => 0.1,
        };

        let access_score = (item.access_count as f64).ln().max(0.0) / 5.0;
        let recency_score = (Utc::now() - item.last_accessed).num_minutes() as f64;
        let recency_score = (-recency_score / 60.0).exp();

        importance_score * 0.4 + access_score * 0.3 + item.decay_factor * 0.2 + recency_score * 0.1
    }

    async fn consolidate_to_long_term(&self, item: WorkingMemoryItem) {
        if item.importance == MemoryImportance::Transient {
            return;
        }

        let mut ltm = self.long_term.write().await;

        ltm.consolidated.push(item);

        info!("Consolidated memory to long-term storage");
    }

    pub async fn recall(&self, id: &str) -> Option<WorkingMemoryItem> {
        let mut stm = self.short_term.write().await;

        for item in &mut stm.items {
            if item.id == id {
                item.access_count += 1;
                item.last_accessed = Utc::now();
                return Some(item.clone());
            }
        }

        let ltm = self.long_term.read().await;
        for item in &ltm.consolidated {
            if item.id == id {
                return Some(item.clone());
            }
        }

        None
    }

    pub async fn search(&self, query: &str, limit: usize) -> Vec<WorkingMemoryItem> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<(f64, WorkingMemoryItem)> = Vec::new();

        let stm = self.short_term.read().await;
        for item in &stm.items {
            let score = self.calculate_relevance(&item.content, &query_lower);
            if score > 0.3 {
                results.push((score * self.calculate_retention_score(item), item.clone()));
            }
        }

        let ltm = self.long_term.read().await;
        for item in &ltm.consolidated {
            let score = self.calculate_relevance(&item.content, &query_lower);
            if score > 0.3 {
                results.push((score * 0.8, item.clone()));
            }
        }

        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        results
            .into_iter()
            .take(limit)
            .map(|(_, item)| item)
            .collect()
    }

    fn calculate_relevance(&self, content: &str, query: &str) -> f64 {
        let content_lower = content.to_lowercase();
        let query_terms: Vec<&str> = query.split_whitespace().collect();

        if query_terms.is_empty() {
            return 0.0;
        }

        let matches = query_terms
            .iter()
            .filter(|term| content_lower.contains(*term))
            .count();

        matches as f64 / query_terms.len() as f64
    }

    pub async fn get_context(&self, max_tokens: usize) -> String {
        let stm = self.short_term.read().await;
        let mut context = String::new();
        let mut tokens_used = 0;

        for item in stm.items.iter().rev() {
            if tokens_used + item.token_count > max_tokens {
                break;
            }

            context.push_str(&item.content);
            context.push_str("\n\n");
            tokens_used += item.token_count;
        }

        context
    }

    pub async fn decay(&self) {
        let mut stm = self.short_term.write().await;

        for item in &mut stm.items {
            item.decay_factor *= WORKING_MEMORY_DECAY;
        }

        info!("Applied decay to working memory");
    }

    pub async fn consolidate(&self) -> Result<()> {
        let mut stm = self.short_term.write().await;
        let mut to_consolidate = Vec::new();

        stm.items.retain(|item| {
            if item.access_count >= self.consolidation_threshold
                && item.importance != MemoryImportance::Transient
            {
                to_consolidate.push(item.clone());
                false
            } else {
                true
            }
        });

        for item in to_consolidate {
            self.consolidate_to_long_term(item).await;
        }

        self.summarize_old_memories().await?;

        Ok(())
    }

    async fn summarize_old_memories(&self) -> Result<()> {
        let mut ltm = self.long_term.write().await;

        if ltm.consolidated.len() > 50 {
            let old_memories: Vec<_> = ltm.consolidated.drain(..20).collect();

            let combined_content: String = old_memories
                .iter()
                .map(|m| m.content.as_str())
                .collect::<Vec<_>>()
                .join("\n");

            let summary = MemorySummary {
                id: format!("summary_{}", uuid::Uuid::new_v4()),
                original_ids: old_memories.iter().map(|m| m.id.clone()).collect(),
                summary: format!(
                    "Summary of {} memories: {}",
                    old_memories.len(),
                    combined_content.chars().take(500).collect::<String>()
                ),
                created_at: Utc::now(),
                importance: MemoryImportance::Normal,
                token_count: 100,
            };

            ltm.summaries.push(summary);

            info!("Summarized {} old memories", old_memories.len());
        }

        Ok(())
    }

    pub async fn add_episode(
        &self,
        event_type: &str,
        description: &str,
        outcome: &str,
        learned: Vec<&str>,
    ) -> Result<String> {
        let episode = EpisodicMemory {
            id: format!("ep_{}", uuid::Uuid::new_v4()),
            event_type: event_type.to_string(),
            description: description.to_string(),
            participants: Vec::new(),
            outcome: outcome.to_string(),
            timestamp: Utc::now(),
            learned: learned.iter().map(|s| s.to_string()).collect(),
            emotion: None,
        };

        let id = episode.id.clone();

        let mut episodic = self.episodic.write().await;
        episodic.push(episode);

        info!("Added episodic memory: {}", id);

        Ok(id)
    }

    pub async fn get_episodes(
        &self,
        event_type: Option<&str>,
        limit: usize,
    ) -> Vec<EpisodicMemory> {
        let episodic = self.episodic.read().await;

        let mut episodes: Vec<_> = episodic
            .iter()
            .filter(|e| event_type.map_or(true, |t| e.event_type == t))
            .cloned()
            .collect();

        episodes.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        episodes.into_iter().take(limit).collect()
    }

    pub async fn learn_from_episodes(&self) -> Vec<String> {
        let episodic = self.episodic.read().await;

        let mut learned: Vec<String> = Vec::new();

        for episode in episodic.iter() {
            learned.extend(episode.learned.clone());
        }

        let unique_learned: std::collections::HashSet<_> = learned.into_iter().collect();
        unique_learned.into_iter().collect()
    }

    pub async fn forget(&self, id: &str) -> bool {
        let mut stm = self.short_term.write().await;

        if let Some(pos) = stm.items.iter().position(|i| i.id == id) {
            let removed = stm.items.remove(pos).unwrap();
            stm.total_tokens -= removed.token_count;
            info!("Forgot memory: {}", id);
            return true;
        }

        let mut ltm = self.long_term.write().await;
        if let Some(pos) = ltm.consolidated.iter().position(|i| i.id == id) {
            ltm.consolidated.remove(pos);
            info!("Forgot long-term memory: {}", id);
            return true;
        }

        false
    }

    pub async fn compress(&self) -> Result<usize> {
        let mut stm = self.short_term.write().await;
        let mut compressed_count = 0;

        for item in &mut stm.items {
            if !item.compressed && item.token_count > 200 {
                let original_tokens = item.token_count;

                item.content = self.compress_content(&item.content);
                item.token_count = self.estimate_tokens(&item.content);
                item.compressed = true;
                item.compression_ratio = Some(item.token_count as f64 / original_tokens as f64);

                compressed_count += 1;
            }
        }

        stm.total_tokens = stm.items.iter().map(|i| i.token_count).sum();

        info!("Compressed {} memory items", compressed_count);

        Ok(compressed_count)
    }

    fn compress_content(&self, content: &str) -> String {
        let sentences: Vec<&str> = content.split(". ").collect();

        if sentences.len() <= 3 {
            return content.to_string();
        }

        let key_sentences: Vec<&str> = vec![
            sentences.first().unwrap_or(&""),
            sentences.get(sentences.len() / 2).unwrap_or(&""),
            sentences.last().unwrap_or(&""),
        ];

        key_sentences.join(". ") + "."
    }

    pub async fn get_stats(&self) -> WorkingMemoryStats {
        let stm = self.short_term.read().await;
        let ltm = self.long_term.read().await;
        let episodic = self.episodic.read().await;

        WorkingMemoryStats {
            short_term_count: stm.items.len(),
            short_term_tokens: stm.total_tokens,
            token_budget: stm.token_budget,
            long_term_count: ltm.consolidated.len(),
            summary_count: ltm.summaries.len(),
            episodic_count: episodic.len(),
            avg_importance: stm
                .items
                .iter()
                .map(|i| match i.importance {
                    MemoryImportance::Critical => 5.0,
                    MemoryImportance::High => 4.0,
                    MemoryImportance::Normal => 3.0,
                    MemoryImportance::Low => 2.0,
                    MemoryImportance::Transient => 1.0,
                })
                .sum::<f64>()
                / stm.items.len().max(1) as f64,
            compression_rate: stm.items.iter().filter(|i| i.compressed).count() as f64
                / stm.items.len().max(1) as f64,
        }
    }

    /// Convenience method: add a chat message with role-aware importance
    pub async fn add_message(&self, content: &str, role: &str) -> Result<String> {
        let importance = match role {
            "system" => MemoryImportance::High,
            "user" | "assistant" => MemoryImportance::Normal,
            _ => MemoryImportance::Low,
        };

        let context = [("role".to_string(), role.to_string())]
            .into_iter()
            .collect();

        self.add(content, importance, context).await
    }

    /// Return the N most recent short-term memory items
    pub async fn get_recent(&self, count: usize) -> Vec<WorkingMemoryItem> {
        let stm = self.short_term.read().await;
        stm.items.iter().rev().take(count).cloned().collect()
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryStats {
    pub short_term_count: usize,
    pub short_term_tokens: usize,
    pub token_budget: usize,
    pub long_term_count: usize,
    pub summary_count: usize,
    pub episodic_count: usize,
    pub avg_importance: f64,
    pub compression_rate: f64,
}

impl Default for WorkingMemoryEngine {
    fn default() -> Self {
        Self::new()
    }
}

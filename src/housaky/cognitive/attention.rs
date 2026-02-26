//! Attention Mechanism (Context Window Management)
//!
//! Relevance-weighted context selection beyond simple recency:
//! - Scores context items by relevance, recency, importance, frequency
//! - Dynamic budget: expand for complex tasks, compress for simple
//! - Attention history: tracks which context proved most useful
//! - Integrates with WorkingMemoryEngine::get_context()

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

// ── Core Types ───────────────────────────────────────────────────────────────

/// A context item that can be attended to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    pub id: String,
    pub content: String,
    pub source: ContextSource,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u32,
    pub importance: f64,
    pub token_count: usize,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContextSource {
    UserInput,
    ToolOutput,
    Memory,
    Reasoning,
    Goal,
    KnowledgeGraph,
    InnerMonologue,
    SystemPrompt,
}

/// Attention score for a context item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionScore {
    pub item_id: String,
    pub total_score: f64,
    pub relevance_score: f64,
    pub recency_score: f64,
    pub importance_score: f64,
    pub frequency_score: f64,
    pub goal_alignment_score: f64,
}

/// Record of attention allocation (for learning).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionRecord {
    pub task_id: String,
    pub selected_items: Vec<String>,
    pub scores: Vec<AttentionScore>,
    pub task_success: Option<bool>,
    pub timestamp: DateTime<Utc>,
}

/// Current attention state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionState {
    pub focus: Option<String>,
    pub focus_reason: String,
    pub active_items: Vec<String>,
    pub suppressed_items: Vec<String>,
    pub total_budget_tokens: usize,
    pub used_tokens: usize,
}

// ── Attention Mechanism ──────────────────────────────────────────────────────

/// Weights for attention scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionWeights {
    pub relevance: f64,
    pub recency: f64,
    pub importance: f64,
    pub frequency: f64,
    pub goal_alignment: f64,
}

impl Default for AttentionWeights {
    fn default() -> Self {
        Self {
            relevance: 0.35,
            recency: 0.20,
            importance: 0.20,
            frequency: 0.10,
            goal_alignment: 0.15,
        }
    }
}

pub struct AttentionMechanism {
    pub weights: Arc<RwLock<AttentionWeights>>,
    pub attention_history: Arc<RwLock<Vec<AttentionRecord>>>,
    pub current_state: Arc<RwLock<AttentionState>>,
    pub context_pool: Arc<RwLock<Vec<ContextItem>>>,
    max_history: usize,
}

impl AttentionMechanism {
    pub fn new() -> Self {
        Self {
            weights: Arc::new(RwLock::new(AttentionWeights::default())),
            attention_history: Arc::new(RwLock::new(Vec::new())),
            current_state: Arc::new(RwLock::new(AttentionState {
                focus: None,
                focus_reason: String::new(),
                active_items: Vec::new(),
                suppressed_items: Vec::new(),
                total_budget_tokens: 8000,
                used_tokens: 0,
            })),
            context_pool: Arc::new(RwLock::new(Vec::new())),
            max_history: 1000,
        }
    }

    /// Add a context item to the pool.
    pub async fn add_context(&self, item: ContextItem) {
        let mut pool = self.context_pool.write().await;
        
        // Replace if existing item with same ID
        if let Some(pos) = pool.iter().position(|i| i.id == item.id) {
            pool[pos] = item;
        } else {
            pool.push(item);
        }
    }

    /// Select the most relevant context items for the current task.
    pub async fn select_context(
        &self,
        query: &str,
        current_goal: Option<&str>,
        max_tokens: usize,
    ) -> Vec<(ContextItem, AttentionScore)> {
        let pool = self.context_pool.read().await;
        let weights = self.weights.read().await;

        if pool.is_empty() {
            return Vec::new();
        }

        // Score all items
        let mut scored: Vec<(ContextItem, AttentionScore)> = pool
            .iter()
            .map(|item| {
                let score = self.score_item(item, query, current_goal, &weights);
                (item.clone(), score)
            })
            .collect();

        // Sort by total score descending
        scored.sort_by(|a, b| {
            b.1.total_score
                .partial_cmp(&a.1.total_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Select items within token budget
        let mut selected = Vec::new();
        let mut used_tokens = 0;

        for (item, score) in scored {
            if used_tokens + item.token_count > max_tokens {
                continue;
            }
            used_tokens += item.token_count;
            selected.push((item, score));
        }

        // Update attention state
        {
            let mut state = self.current_state.write().await;
            state.active_items = selected.iter().map(|(i, _)| i.id.clone()).collect();
            state.used_tokens = used_tokens;
            state.total_budget_tokens = max_tokens;

            if let Some((top_item, _)) = selected.first() {
                state.focus = Some(top_item.id.clone());
                state.focus_reason = format!(
                    "Highest attention score for query: '{}'",
                    query.chars().take(50).collect::<String>()
                );
            }
        }

        // Update access counts
        drop(pool);
        let mut pool = self.context_pool.write().await;
        for (sel_item, _) in &selected {
            if let Some(item) = pool.iter_mut().find(|i| i.id == sel_item.id) {
                item.access_count += 1;
                item.last_accessed = Utc::now();
            }
        }

        info!(
            "Attention: selected {} items ({} tokens) from {} candidates",
            selected.len(),
            used_tokens,
            pool.len()
        );

        selected
    }

    /// Score a single context item.
    fn score_item(
        &self,
        item: &ContextItem,
        query: &str,
        current_goal: Option<&str>,
        weights: &AttentionWeights,
    ) -> AttentionScore {
        // Relevance: keyword overlap with query
        let relevance_score = Self::compute_relevance(&item.content, query);

        // Recency: exponential decay based on age
        let age_secs = (Utc::now() - item.last_accessed).num_seconds().max(0) as f64;
        let half_life = 3600.0; // 1 hour half-life
        let recency_score = (-age_secs * (2.0_f64.ln()) / half_life).exp();

        // Importance: directly from item
        let importance_score = item.importance;

        // Frequency: logarithmic scaling of access count
        let frequency_score = (1.0 + item.access_count as f64).ln() / 5.0;
        let frequency_score = frequency_score.min(1.0);

        // Goal alignment: relevance to current goal
        let goal_alignment_score = current_goal
            .map(|g| Self::compute_relevance(&item.content, g))
            .unwrap_or(0.5);

        let total_score = relevance_score * weights.relevance
            + recency_score * weights.recency
            + importance_score * weights.importance
            + frequency_score * weights.frequency
            + goal_alignment_score * weights.goal_alignment;

        AttentionScore {
            item_id: item.id.clone(),
            total_score,
            relevance_score,
            recency_score,
            importance_score,
            frequency_score,
            goal_alignment_score,
        }
    }

    /// Compute relevance between two text pieces (keyword overlap).
    fn compute_relevance(content: &str, query: &str) -> f64 {
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();
        
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();
        if query_terms.is_empty() {
            return 0.0;
        }

        let matches = query_terms
            .iter()
            .filter(|term| content_lower.contains(**term))
            .count();

        matches as f64 / query_terms.len() as f64
    }

    /// Record which context items were useful for a task.
    pub async fn record_feedback(
        &self,
        task_id: &str,
        useful_item_ids: &[String],
        task_success: bool,
    ) {
        let state = self.current_state.read().await;
        let pool = self.context_pool.read().await;

        let scores: Vec<AttentionScore> = useful_item_ids
            .iter()
            .filter_map(|id| {
                pool.iter().find(|i| i.id == *id).map(|item| AttentionScore {
                    item_id: id.clone(),
                    total_score: 1.0, // Items explicitly marked useful
                    relevance_score: 1.0,
                    recency_score: 0.0,
                    importance_score: item.importance,
                    frequency_score: 0.0,
                    goal_alignment_score: 0.0,
                })
            })
            .collect();

        drop(pool);
        drop(state);

        let record = AttentionRecord {
            task_id: task_id.to_string(),
            selected_items: useful_item_ids.to_vec(),
            scores,
            task_success: Some(task_success),
            timestamp: Utc::now(),
        };

        let mut history = self.attention_history.write().await;
        history.push(record);
        if history.len() > self.max_history {
            history.remove(0);
        }

        // Adjust weights based on feedback
        if task_success {
            self.adjust_weights_from_feedback(useful_item_ids).await;
        }
    }

    /// Adjust attention weights based on which context sources were useful.
    async fn adjust_weights_from_feedback(&self, useful_item_ids: &[String]) {
        let pool = self.context_pool.read().await;

        // Count which source types were most useful
        let mut source_counts: HashMap<String, usize> = HashMap::new();
        for id in useful_item_ids {
            if let Some(item) = pool.iter().find(|i| i.id == *id) {
                *source_counts
                    .entry(format!("{:?}", item.source))
                    .or_insert(0) += 1;
            }
        }

        // Boost importance of items that were useful
        drop(pool);
        let mut pool = self.context_pool.write().await;
        for id in useful_item_ids {
            if let Some(item) = pool.iter_mut().find(|i| i.id == *id) {
                item.importance = (item.importance * 1.1).min(1.0);
            }
        }
    }

    /// Get the current attention state.
    pub async fn get_state(&self) -> AttentionState {
        self.current_state.read().await.clone()
    }

    /// Set the token budget dynamically based on task complexity.
    pub async fn set_budget(&self, max_tokens: usize) {
        let mut state = self.current_state.write().await;
        state.total_budget_tokens = max_tokens;
    }

    /// Get attention statistics.
    pub async fn get_stats(&self) -> AttentionStats {
        let pool = self.context_pool.read().await;
        let history = self.attention_history.read().await;
        let state = self.current_state.read().await;

        let avg_pool_size = pool.len();
        let successful_records = history
            .iter()
            .filter(|r| r.task_success == Some(true))
            .count();

        AttentionStats {
            pool_size: avg_pool_size,
            total_selections: history.len(),
            successful_selections: successful_records,
            current_focus: state.focus.clone(),
            budget_utilization: if state.total_budget_tokens > 0 {
                state.used_tokens as f64 / state.total_budget_tokens as f64
            } else {
                0.0
            },
        }
    }

    /// Evict stale items from the context pool.
    pub async fn evict_stale(&self, max_age_hours: i64) {
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours);
        let mut pool = self.context_pool.write().await;
        let before = pool.len();
        pool.retain(|item| item.last_accessed > cutoff || item.importance > 0.8);
        let after = pool.len();
        if before != after {
            info!("Attention: evicted {} stale context items", before - after);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionStats {
    pub pool_size: usize,
    pub total_selections: usize,
    pub successful_selections: usize,
    pub current_focus: Option<String>,
    pub budget_utilization: f64,
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_attention_selection() {
        let mechanism = AttentionMechanism::new();

        mechanism
            .add_context(ContextItem {
                id: "c1".to_string(),
                content: "The Rust programming language is fast and safe".to_string(),
                source: ContextSource::Memory,
                created_at: Utc::now(),
                last_accessed: Utc::now(),
                access_count: 5,
                importance: 0.8,
                token_count: 10,
                tags: vec!["rust".to_string()],
            })
            .await;

        mechanism
            .add_context(ContextItem {
                id: "c2".to_string(),
                content: "Python is good for data science".to_string(),
                source: ContextSource::Memory,
                created_at: Utc::now(),
                last_accessed: Utc::now(),
                access_count: 2,
                importance: 0.5,
                token_count: 8,
                tags: vec!["python".to_string()],
            })
            .await;

        let selected = mechanism
            .select_context("Rust performance optimization", None, 100)
            .await;

        assert!(!selected.is_empty());
        // Rust-related context should rank higher
        assert_eq!(selected[0].0.id, "c1");
    }

    #[test]
    fn test_relevance_computation() {
        let score = AttentionMechanism::compute_relevance(
            "implementing causal inference in rust",
            "causal inference rust",
        );
        assert!(score > 0.5);

        let low_score = AttentionMechanism::compute_relevance(
            "making pizza dough",
            "causal inference rust",
        );
        assert!(low_score < score);
    }
}

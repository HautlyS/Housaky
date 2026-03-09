// ☸️ INTELLIGENT MEMORY SYSTEM
// Never forget, never bloat - context-aware memory retrieval for LLMs
// Uses importance classification, semantic deduplication, and context budgeting

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::traits::{Memory, MemoryCategory, MemoryEntry};
use super::lucid_native::LucidNativeMemory;

// ============================================================================
// CONFIGURATION
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligentMemoryConfig {
    /// Maximum tokens in context (default: 8000, leaves room for LLM response)
    pub max_context_tokens: usize,
    /// Tokens per item average estimate
    pub avg_tokens_per_item: usize,
    /// Minimum importance score to never forget (0.0-1.0)
    pub critical_importance_threshold: f64,
    /// High importance threshold (0.0-1.0)
    pub high_importance_threshold: f64,
    /// Medium importance threshold (0.0-1.0)
    pub medium_importance_threshold: f64,
    /// Maximum duplicates to allow in results
    pub max_duplicate_similarity: f64,
    /// Enable semantic deduplication
    pub enable_deduplication: bool,
    /// Enable importance classification
    pub enable_importance_classification: bool,
    /// Enable never-forget critical memories
    pub enable_critical_memories: bool,
    /// Context retention for current session
    pub session_context_items: usize,
}

impl Default for IntelligentMemoryConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 8000,
            avg_tokens_per_item: 200,
            critical_importance_threshold: 0.9,
            high_importance_threshold: 0.7,
            medium_importance_threshold: 0.4,
            max_duplicate_similarity: 0.85,
            enable_deduplication: true,
            enable_importance_classification: true,
            enable_critical_memories: true,
            session_context_items: 10,
        }
    }
}

// ============================================================================
// IMPORTANCE CLASSIFICATION
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryImportance {
    Critical,  // Never forget - always include
    High,      // Important - include if space allows
    Medium,    // Normal priority
    Low,       // Skip unless plenty of space
    Trivial,   // Skip - not relevant
}

impl MemoryImportance {
    pub fn from_score(score: f64, config: &IntelligentMemoryConfig) -> Self {
        if score >= config.critical_importance_threshold {
            MemoryImportance::Critical
        } else if score >= config.high_importance_threshold {
            MemoryImportance::High
        } else if score >= config.medium_importance_threshold {
            MemoryImportance::Medium
        } else if score >= 0.2 {
            MemoryImportance::Low
        } else {
            MemoryImportance::Trivial
        }
    }
}

// ============================================================================
// CONTEXT BUDGET
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBudget {
    pub total_tokens: usize,
    pub used_tokens: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub remaining_tokens: usize,
}

impl ContextBudget {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            total_tokens: max_tokens,
            used_tokens: 0,
            critical_count: 0,
            high_count: 0,
            medium_count: 0,
            remaining_tokens: max_tokens,
        }
    }

    pub fn allocate(&mut self, tokens: usize, importance: MemoryImportance) -> bool {
        if tokens > self.remaining_tokens {
            // Critical memories always get included regardless of budget
            if importance == MemoryImportance::Critical {
                return true;
            }
            return false;
        }
        
        self.used_tokens += tokens;
        self.remaining_tokens -= tokens;
        
        match importance {
            MemoryImportance::Critical => self.critical_count += 1,
            MemoryImportance::High => self.high_count += 1,
            MemoryImportance::Medium => self.medium_count += 1,
            _ => {}
        }
        
        true
    }

    pub fn can_allocate(&self, tokens: usize, importance: MemoryImportance) -> bool {
        if importance == MemoryImportance::Critical {
            return true; // Critical always fits
        }
        tokens <= self.remaining_tokens
    }
}

// ============================================================================
// SEARCH RESULT
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligentSearchResult {
    pub entries: Vec<MemoryEntry>,
    pub total_tokens: usize,
    pub importance_distribution: HashMap<String, usize>,
    pub deduplicated_count: usize,
    pub critical_included: bool,
    pub query: String,
}

impl IntelligentSearchResult {
    pub fn summary(&self) -> String {
        format!(
            "Found {} memories ({} tokens). Critical: {}, High: {}, Medium: {}, Deduplicated: {}",
            self.entries.len(),
            self.total_tokens,
            self.importance_distribution.get("critical").unwrap_or(&0),
            self.importance_distribution.get("high").unwrap_or(&0),
            self.importance_distribution.get("medium").unwrap_or(&0),
            self.deduplicated_count
        )
    }
}

// ============================================================================
// IMPORTANCE CLASSIFIER
// ============================================================================

pub struct ImportanceClassifier {
    critical_patterns: Vec<String>,
    high_importance_patterns: Vec<String>,
    low_importance_patterns: Vec<String>,
}

impl ImportanceClassifier {
    pub fn new() -> Self {
        Self {
            critical_patterns: vec![
                "password".to_string(),
                "api_key".to_string(),
                "secret".to_string(),
                "credential".to_string(),
                "preference".to_string(),
                "important".to_string(),
                "never forget".to_string(),
                "critical".to_string(),
                "essential".to_string(),
                "fixed".to_string(),
                "bug".to_string(),
                "issue".to_string(),
                "error".to_string(),
                "learned".to_string(),
                "remember".to_string(),
                "decision".to_string(),
                "agreed".to_string(),
                "commitment".to_string(),
                "promise".to_string(),
                "name".to_string(),
                "email".to_string(),
                "address".to_string(),
                "phone".to_string(),
            ],
            high_importance_patterns: vec![
                "function".to_string(),
                "method".to_string(),
                "class".to_string(),
                "implement".to_string(),
                "configure".to_string(),
                "setup".to_string(),
                "install".to_string(),
                "build".to_string(),
                "compile".to_string(),
                "run".to_string(),
                "command".to_string(),
                "tool".to_string(),
                "workflow".to_string(),
                "process".to_string(),
                "strategy".to_string(),
                "plan".to_string(),
                "goal".to_string(),
            ],
            low_importance_patterns: vec![
                "hello".to_string(),
                "hi".to_string(),
                "thanks".to_string(),
                "thank you".to_string(),
                "okay".to_string(),
                "ok".to_string(),
                "sure".to_string(),
                "yes".to_string(),
                "no".to_string(),
                "maybe".to_string(),
                "perhaps".to_string(),
            ],
        }
    }

    pub fn classify(&self, entry: &MemoryEntry, query: &str) -> f64 {
        let content_lower = entry.content.to_lowercase();
        let key_lower = entry.key.to_lowercase();
        let query_lower = query.to_lowercase();
        
        // Start with a base score
        let mut score = 0.5;
        
        // Query relevance boost
        if content_lower.contains(&query_lower) || key_lower.contains(&query_lower) {
            score += 0.2;
        }
        
        // Category-based scoring
        match entry.category {
            MemoryCategory::Core => score += 0.3,
            MemoryCategory::Conversation => score += 0.1,
            MemoryCategory::Daily => score += 0.05,
            MemoryCategory::Custom(ref name) => {
                if name.contains("skill") || name.contains("pattern") || name.contains("insight") {
                    score += 0.25;
                } else if name.contains("procedure") {
                    score += 0.2;
                }
            }
        }
        
        // Critical patterns
        for pattern in &self.critical_patterns {
            if content_lower.contains(pattern) || key_lower.contains(pattern) {
                score += 0.25;
                break;
            }
        }
        
        // High importance patterns
        for pattern in &self.high_importance_patterns {
            if content_lower.contains(pattern) || key_lower.contains(pattern) {
                score += 0.15;
                break;
            }
        }
        
        // Low importance patterns (reduce score)
        for pattern in &self.low_importance_patterns {
            if content_lower.contains(pattern) {
                score -= 0.2;
                break;
            }
        }
        
        // Recent memories get a small boost (recency bias)
        if entry.timestamp.len() > 10 {
            // Simple check - if timestamp is recent-ish
            if entry.timestamp.contains("2026") || entry.timestamp.contains("2025") {
                score += 0.1;
            }
        }
        
        // Clamp to 0.0-1.0
        score.max(0.0).min(1.0)
    }
}

impl Default for ImportanceClassifier {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SEMANTIC DEDUPLICATOR
// ============================================================================

pub struct SemanticDeduplicator {
    min_similarity: f64,
}

impl SemanticDeduplicator {
    pub fn new(min_similarity: f64) -> Self {
        Self { min_similarity }
    }

    /// Simple token-based similarity (avoiding external dependencies)
    pub fn calculate_similarity(&self, text1: &str, text2: &str) -> f64 {
        let words1: HashSet<String> = text1
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        
        let words2: HashSet<String> = text2
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        
        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }
        
        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();
        
        intersection as f64 / union as f64
    }

    /// Remove duplicates while preserving the highest-scoring versions
    pub fn deduplicate(&self, entries: &mut Vec<MemoryEntry>, scores: &HashMap<String, f64>) -> usize {
        if entries.len() < 2 {
            return 0;
        }
        
        let mut to_remove: HashSet<usize> = HashSet::new();
        
        for i in 0..entries.len() {
            if to_remove.contains(&i) {
                continue;
            }
            
            for j in (i + 1)..entries.len() {
                if to_remove.contains(&j) {
                    continue;
                }
                
                let similarity = self.calculate_similarity(
                    &entries[i].content,
                    &entries[j].content,
                );
                
                if similarity >= self.min_similarity {
                    // Keep the one with higher score
                    let score_i = scores.get(&entries[i].id).copied().unwrap_or(0.5);
                    let score_j = scores.get(&entries[j].id).copied().unwrap_or(0.5);
                    
                    if score_i >= score_j {
                        to_remove.insert(j);
                    } else {
                        to_remove.insert(i);
                        break;
                    }
                }
            }
        }
        
        let mut new_entries = Vec::new();
        for (i, entry) in entries.drain(..).enumerate() {
            if !to_remove.contains(&i) {
                new_entries.push(entry);
            }
        }
        *entries = new_entries;
        
        to_remove.len()
    }
}

impl Default for SemanticDeduplicator {
    fn default() -> Self {
        Self::new(0.85)
    }
}

// ============================================================================
// INTELLIGENT MEMORY
// ============================================================================

pub struct IntelligentMemory {
    backend: Arc<LucidNativeMemory>,
    config: IntelligentMemoryConfig,
    classifier: ImportanceClassifier,
    deduplicator: SemanticDeduplicator,
    session_context: Arc<RwLock<Vec<MemoryEntry>>>,
    critical_cache: Arc<RwLock<Vec<MemoryEntry>>>,
}

impl IntelligentMemory {
    pub fn new(workspace_dir: PathBuf) -> Self {
        let config = IntelligentMemoryConfig::default();
        let backend = LucidNativeMemory::with_workspace(&workspace_dir);
        
        Self {
            backend: Arc::new(backend),
            config,
            classifier: ImportanceClassifier::new(),
            deduplicator: SemanticDeduplicator::new(config.max_duplicate_similarity),
            session_context: Arc::new(RwLock::new(Vec::new())),
            critical_cache: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_config(workspace_dir: PathBuf, config: IntelligentMemoryConfig) -> Self {
        let backend = LucidNativeMemory::with_workspace(&workspace_dir);
        
        Self {
            backend: Arc::new(backend),
            config: config.clone(),
            classifier: ImportanceClassifier::new(),
            deduplicator: SemanticDeduplicator::new(config.max_duplicate_similarity),
            session_context: Arc::new(RwLock::new(Vec::new())),
            critical_cache: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Intelligent search that prevents context bloat
    pub async fn search(&self, query: &str, max_tokens: Option<usize>) -> IntelligentSearchResult {
        let max_tokens = max_tokens.unwrap_or(self.config.max_context_tokens);
        let mut budget = ContextBudget::new(max_tokens);
        
        // 1. Fetch more candidates than we need
        let candidates = self.backend.recall(query, 50).await.unwrap_or_default();
        
        if candidates.is_empty() {
            return IntelligentSearchResult {
                entries: vec![],
                total_tokens: 0,
                importance_distribution: HashMap::new(),
                deduplicated_count: 0,
                critical_included: false,
                query: query.to_string(),
            };
        }
        
        // 2. Classify importance for each candidate
        let mut scored_entries: Vec<(MemoryEntry, f64)> = candidates
            .into_iter()
            .map(|entry| {
                let score = if self.config.enable_importance_classification {
                    self.classifier.classify(&entry, query)
                } else {
                    entry.score.unwrap_or(0.5)
                };
                (entry, score)
            })
            .collect();
        
        // 3. Sort by importance (critical first)
        scored_entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // 4. Build score map for deduplicator
        let score_map: HashMap<String, f64> = scored_entries
            .iter()
            .map(|(e, s)| (e.id.clone(), *s))
            .collect();
        
        // 5. Deduplicate if enabled
        let mut entries: Vec<MemoryEntry> = scored_entries.into_iter().map(|(e, _)| e).collect();
        let deduplicated_count = if self.config.enable_deduplication {
            self.deduplicator.deduplicate(&mut entries, &score_map)
        } else {
            0
        };
        
        // 6. Allocate to context budget (critical always included)
        let mut final_entries = Vec::new();
        let mut importance_dist: HashMap<String, usize> = HashMap::new();
        
        for entry in entries {
            let importance = MemoryImportance::from_score(
                score_map.get(&entry.id).copied().unwrap_or(0.5),
                &self.config,
            );
            
            let tokens = self.estimate_tokens(&entry.content);
            
            // Critical memories are ALWAYS included, even if over budget
            if importance == MemoryImportance::Critical {
                final_entries.push(entry);
                budget.allocate(tokens, importance);
                
                let key = format!("{:?}", importance).to_lowercase();
                *importance_dist.entry(key).or_insert(0) += 1;
                continue;
            }
            
            // Check if we have space
            if budget.can_allocate(tokens, importance) {
                budget.allocate(tokens, importance);
                final_entries.push(entry);
                
                let key = format!("{:?}", importance).to_lowercase();
                *importance_dist.entry(key).or_insert(0) += 1;
            }
        }
        
        let critical_included = importance_dist.get("critical").map(|&c| c > 0).unwrap_or(false);
        
        IntelligentSearchResult {
            entries: final_entries,
            total_tokens: budget.used_tokens,
            importance_distribution: importance_dist,
            deduplicated_count,
            critical_included,
            query: query.to_string(),
        }
    }

    /// Add session context (recent conversation)
    pub async fn add_session_context(&self, entry: MemoryEntry) {
        let mut ctx = self.session_context.write().await;
        ctx.push(entry.clone());
        
        // Keep only recent items
        if ctx.len() > self.config.session_context_items {
            ctx.remove(0);
        }
        
        // If critical, also add to critical cache
        if self.config.enable_critical_memories {
            let score = self.classifier.classify(&entry, "");
            if score >= self.config.critical_importance_threshold {
                let mut critical = self.critical_cache.write().await;
                if !critical.iter().any(|e| e.id == entry.id) {
                    critical.push(entry);
                }
            }
        }
    }

    /// Get session context
    pub async fn get_session_context(&self) -> Vec<MemoryEntry> {
        self.session_context.read().await.clone()
    }

    /// Get critical (never-forget) memories
    pub async fn get_critical_memories(&self) -> Vec<MemoryEntry> {
        if self.config.enable_critical_memories {
            self.critical_cache.read().await.clone()
        } else {
            vec![]
        }
    }

    /// Search with custom budget
    pub async fn search_with_budget(&self, query: &str, budget_tokens: usize) -> IntelligentSearchResult {
        self.search(query, Some(budget_tokens)).await
    }

    /// Quick recall - just get relevant memories without intelligence
    pub async fn recall(&self, query: &str, limit: usize) -> anyhow::Result<Vec<MemoryEntry>> {
        self.backend.recall(query, limit).await
    }

    fn estimate_tokens(&self, content: &str) -> usize {
        // Rough estimate: ~4 characters per token
        (content.len() / 4).max(10)
    }
}

// Implement Memory trait for IntelligentMemory
#[async_trait]
impl Memory for IntelligentMemory {
    fn name(&self) -> &str {
        "intelligent-memory"
    }

    async fn store(&self, key: &str, content: &str, category: MemoryCategory) -> anyhow::Result<()> {
        // First store in backend
        self.backend.store(key, content, category.clone()).await?;
        
        // Then classify and potentially add to session/critical
        let entry = MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            key: key.to_string(),
            content: content.to_string(),
            category,
            timestamp: chrono::Utc::now().to_rfc3339(),
            session_id: None,
            score: None,
        };
        
        self.add_session_context(entry).await;
        
        Ok(())
    }

    async fn recall(&self, query: &str, limit: usize) -> anyhow::Result<Vec<MemoryEntry>> {
        let result = self.search(query, None).await;
        Ok(result.entries.into_iter().take(limit).collect())
    }

    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        self.backend.get(key).await
    }

    async fn list(&self, category: Option<&MemoryCategory>) -> anyhow::Result<Vec<MemoryEntry>> {
        self.backend.list(category).await
    }

    async fn forget(&self, key: &str) -> anyhow::Result<bool> {
        self.backend.forget(key).await
    }

    async fn count(&self) -> anyhow::Result<usize> {
        self.backend.count().await
    }

    async fn health_check(&self) -> bool {
        self.backend.health_check().await
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_importance_classification() {
        let classifier = ImportanceClassifier::new();
        
        let critical_entry = MemoryEntry {
            id: "1".to_string(),
            key: "api_key".to_string(),
            content: "Remember my API key is secret123".to_string(),
            category: MemoryCategory::Core,
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            session_id: None,
            score: None,
        };
        
        let score = classifier.classify(&critical_entry, "api");
        assert!(score > 0.7, "Critical pattern should score high");
    }

    #[test]
    fn test_deduplication() {
        let dedup = SemanticDeduplicator::new(0.8);
        
        let similarity = dedup.calculate_similarity(
            "The function foo() does X",
            "The function foo() does X",
        );
        
        assert!(similarity > 0.9, "Identical text should have high similarity");
    }

    #[test]
    fn test_context_budget() {
        let mut budget = ContextBudget::new(1000);
        
        // Critical should always fit
        assert!(budget.can_allocate(2000, MemoryImportance::Critical));
        budget.allocate(2000, MemoryImportance::Critical);
        assert_eq!(budget.remaining_tokens, 1000); // Doesn't decrease for critical
        
        // Normal should respect budget
        assert!(budget.can_allocate(500, MemoryImportance::Medium));
        budget.allocate(500, MemoryImportance::Medium);
        assert_eq!(budget.remaining_tokens, 500);
        
        // Should fail when over budget
        assert!(!budget.can_allocate(600, MemoryImportance::Medium));
    }
}

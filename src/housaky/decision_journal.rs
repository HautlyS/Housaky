use crate::util::{read_msgpack_file, write_msgpack_file};
use chrono::{DateTime, Utc};
use rmp_serde::decode::Error as MsgpackDecodeError;
use rmp_serde::encode::Error as MsgpackEncodeError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum DecisionJournalError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("MessagePack decode error: {0}")]
    MsgpackDecode(#[from] MsgpackDecodeError),
    #[error("MessagePack encode error: {0}")]
    MsgpackEncode(#[from] MsgpackEncodeError),
    #[error("Entry not found: {0}")]
    NotFound(String),
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEntry {
    pub timestamp: DateTime<Utc>,
    pub entry_id: String,
    pub goal: String,
    pub context: serde_json::Value,
    pub considered_options: Vec<ConsideredOption>,
    pub chosen: ChosenOption,
    pub reasoning: String,
    pub execution: ExecutionRecord,
    pub outcome: OutcomeRecord,
    pub reflection: Option<String>,
}

impl DecisionEntry {
    pub fn new(goal: String) -> Self {
        Self {
            timestamp: Utc::now(),
            entry_id: format!("decision_{}", Uuid::new_v4()),
            goal,
            context: serde_json::Value::Null,
            considered_options: Vec::new(),
            chosen: ChosenOption::default(),
            reasoning: String::new(),
            execution: ExecutionRecord::default(),
            outcome: OutcomeRecord::default(),
            reflection: None,
        }
    }

    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = context;
        self
    }

    pub fn with_considered_options(mut self, options: Vec<ConsideredOption>) -> Self {
        self.considered_options = options;
        self
    }

    pub fn with_chosen(mut self, chosen: ChosenOption) -> Self {
        self.chosen = chosen;
        self
    }

    pub fn with_reasoning(mut self, reasoning: String) -> Self {
        self.reasoning = reasoning;
        self
    }

    pub fn with_execution(mut self, execution: ExecutionRecord) -> Self {
        self.execution = execution;
        self
    }

    pub fn with_outcome(mut self, outcome: OutcomeRecord) -> Self {
        self.outcome = outcome;
        self
    }

    pub fn with_reflection(mut self, reflection: String) -> Self {
        self.reflection = Some(reflection);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsideredOption {
    pub option: String,
    pub confidence: f64,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
}

impl ConsideredOption {
    pub fn new(option: String, confidence: f64) -> Self {
        Self {
            option,
            confidence,
            pros: Vec::new(),
            cons: Vec::new(),
        }
    }

    pub fn with_pros(mut self, pros: Vec<String>) -> Self {
        self.pros = pros;
        self
    }

    pub fn with_cons(mut self, cons: Vec<String>) -> Self {
        self.cons = cons;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChosenOption {
    pub option: String,
    pub confidence: f64,
    pub reason: String,
}

impl ChosenOption {
    pub fn new(option: String, confidence: f64, reason: String) -> Self {
        Self {
            option,
            confidence,
            reason,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionRecord {
    pub tool: String,
    pub action: String,
    pub parameters: serde_json::Value,
    pub duration_ms: u64,
}

impl ExecutionRecord {
    pub fn new(tool: String, action: String, duration_ms: u64) -> Self {
        Self {
            tool,
            action,
            parameters: serde_json::Value::Null,
            duration_ms,
        }
    }

    pub fn with_parameters(mut self, parameters: serde_json::Value) -> Self {
        self.parameters = parameters;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutcomeRecord {
    pub success: bool,
    pub score: f64,
    pub explanation: String,
    pub metrics: HashMap<String, f64>,
}

impl OutcomeRecord {
    pub fn new(success: bool, score: f64, explanation: String) -> Self {
        Self {
            success,
            score,
            explanation,
            metrics: HashMap::new(),
        }
    }

    pub fn with_metrics(mut self, metrics: HashMap<String, f64>) -> Self {
        self.metrics = metrics;
        self
    }
}

#[async_trait::async_trait]
pub trait DecisionJournal: Send + Sync {
    async fn record_decision(&self, entry: DecisionEntry) -> Result<(), DecisionJournalError>;
    async fn query_by_goal(&self, pattern: &str) -> Result<Vec<DecisionEntry>, DecisionJournalError>;
    async fn query_by_outcome(&self, success: bool) -> Result<Vec<DecisionEntry>, DecisionJournalError>;
    async fn get_learning_dataset(&self) -> Result<Vec<DecisionEntry>, DecisionJournalError>;
    async fn get_entry(&self, entry_id: &str) -> Result<Option<DecisionEntry>, DecisionJournalError>;
    async fn get_stats(&self) -> Result<JournalStats, DecisionJournalError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalStats {
    pub total_entries: usize,
    pub successful_decisions: usize,
    pub failed_decisions: usize,
    pub average_score: f64,
    pub top_goals: Vec<String>,
}

pub struct FileDecisionJournal {
    journal_path: PathBuf,
    entries: Arc<RwLock<Vec<DecisionEntry>>>,
    max_entries_per_file: usize,
    max_total_entries: usize,
}

impl FileDecisionJournal {
    pub fn new(workspace_dir: &PathBuf) -> Result<Self, DecisionJournalError> {
        let journal_path = workspace_dir.join(".housaky").join("decision_journal");
        std::fs::create_dir_all(&journal_path)?;
        
        let journal = Self {
            journal_path,
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries_per_file: 1000,
            max_total_entries: 10000,
        };
        
        Ok(journal)
    }

    pub fn with_max_entries_per_file(mut self, max: usize) -> Self {
        self.max_entries_per_file = max;
        self
    }

    pub fn with_max_total_entries(mut self, max: usize) -> Self {
        self.max_total_entries = max;
        self
    }

    pub async fn load(&self) -> Result<(), DecisionJournalError> {
        let main_file = self.journal_path.join("decisions.msgpack");
        if !main_file.exists() {
            return Ok(());
        }

        let entries: Vec<DecisionEntry> = read_msgpack_file(&main_file).await.map_err(|e| DecisionJournalError::Other(e.to_string()))?;
        
        let mut stored = self.entries.write().await;
        *stored = entries;
        
        Ok(())
    }

    async fn save(&self) -> Result<(), DecisionJournalError> {
        let entries = self.entries.read().await;
        
        if entries.len() >= self.max_entries_per_file {
            drop(entries);
            self.rotate().await?;
            return Ok(());
        }

        let main_file = self.journal_path.join("decisions.msgpack");
        
        if let Some(parent) = main_file.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        write_msgpack_file(&main_file, &*entries).await.map_err(|e| DecisionJournalError::Other(e.to_string()))?;
        
        Ok(())
    }

    async fn rotate(&self) -> Result<(), DecisionJournalError> {
        let mut entries = self.entries.write().await;
        
        for i in (1..10).rev() {
            let old_name = format!("decisions.{}.msgpack", i);
            let new_name = format!("decisions.{}.msgpack", i + 1);
            let old_path = self.journal_path.join(&old_name);
            let new_path = self.journal_path.join(&new_name);
            
            if old_path.exists() {
                let _ = tokio::fs::rename(&old_path, &new_path).await;
            }
        }

        let rotated = self.journal_path.join("decisions.1.msgpack");
        let main_file = self.journal_path.join("decisions.msgpack");
        
        if main_file.exists() {
            let _ = tokio::fs::rename(&main_file, &rotated).await;
        }

        if entries.len() > self.max_total_entries {
            let keep_count = self.max_total_entries / 2;
            let remaining: Vec<DecisionEntry> = entries
                .iter()
                .skip(entries.len() - keep_count)
                .cloned()
                .collect();
            *entries = remaining;
        }
        
        write_msgpack_file(&main_file, &*entries).await.map_err(|e| DecisionJournalError::Other(e.to_string()))?;
        
        Ok(())
    }

    fn matches_pattern(goal: &str, pattern: &str) -> bool {
        if pattern.is_empty() {
            return true;
        }
        
        let pattern_lower = pattern.to_lowercase();
        let goal_lower = goal.to_lowercase();
        
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern_lower.split('*').collect();
            if parts.len() == 2 {
                let starts = parts[0];
                let ends = parts[1];
                return goal_lower.starts_with(starts) && goal_lower.ends_with(ends);
            }
        }
        
        goal_lower.contains(&pattern_lower)
    }
}

#[async_trait::async_trait]
impl DecisionJournal for FileDecisionJournal {
    async fn record_decision(&self, entry: DecisionEntry) -> Result<(), DecisionJournalError> {
        let mut entries = self.entries.write().await;
        entries.push(entry);
        drop(entries);
        
        self.save().await
    }

    async fn query_by_goal(&self, pattern: &str) -> Result<Vec<DecisionEntry>, DecisionJournalError> {
        let entries = self.entries.read().await;
        
        let results: Vec<DecisionEntry> = entries
            .iter()
            .filter(|e| Self::matches_pattern(&e.goal, pattern))
            .cloned()
            .collect();
        
        Ok(results)
    }

    async fn query_by_outcome(&self, success: bool) -> Result<Vec<DecisionEntry>, DecisionJournalError> {
        let entries = self.entries.read().await;
        
        let results: Vec<DecisionEntry> = entries
            .iter()
            .filter(|e| e.outcome.success == success)
            .cloned()
            .collect();
        
        Ok(results)
    }

    async fn get_learning_dataset(&self) -> Result<Vec<DecisionEntry>, DecisionJournalError> {
        let entries = self.entries.read().await;
        
        let results: Vec<DecisionEntry> = entries
            .iter()
            .filter(|e| e.outcome.success && e.outcome.score >= 0.5)
            .cloned()
            .collect();
        
        Ok(results)
    }

    async fn get_entry(&self, entry_id: &str) -> Result<Option<DecisionEntry>, DecisionJournalError> {
        let entries = self.entries.read().await;
        
        Ok(entries.iter().find(|e| e.entry_id == entry_id).cloned())
    }

    async fn get_stats(&self) -> Result<JournalStats, DecisionJournalError> {
        let entries = self.entries.read().await;
        
        let total = entries.len();
        let successful = entries.iter().filter(|e| e.outcome.success).count();
        let failed = total - successful;
        
        let average_score = if total > 0 {
            entries.iter().map(|e| e.outcome.score).sum::<f64>() / total as f64
        } else {
            0.0
        };
        
        let mut goal_counts: HashMap<String, usize> = HashMap::new();
        for entry in entries.iter() {
            *goal_counts.entry(entry.goal.clone()).or_insert(0) += 1;
        }
        
        let mut top_goals: Vec<(String, usize)> = goal_counts.into_iter().collect();
        top_goals.sort_by(|a, b| b.1.cmp(&a.1));
        top_goals.truncate(10);
        
        Ok(JournalStats {
            total_entries: total,
            successful_decisions: successful,
            failed_decisions: failed,
            average_score,
            top_goals: top_goals.into_iter().map(|(g, _)| g).collect(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionContext {
    pub session_id: String,
    pub turn_number: u32,
    pub available_tools: Vec<String>,
    pub current_goal: Option<String>,
    pub previous_decisions: Vec<String>,
}

impl Default for DecisionContext {
    fn default() -> Self {
        Self {
            session_id: Uuid::new_v4().to_string(),
            turn_number: 0,
            available_tools: Vec::new(),
            current_goal: None,
            previous_decisions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionBuilder {
    entry: DecisionEntry,
    context: DecisionContext,
}

impl DecisionBuilder {
    pub fn new(goal: String) -> Self {
        Self {
            entry: DecisionEntry::new(goal),
            context: DecisionContext::default(),
        }
    }

    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.context.session_id = session_id;
        self
    }

    pub fn with_turn(mut self, turn: u32) -> Self {
        self.context.turn_number = turn;
        self
    }

    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.context.available_tools = tools;
        self
    }

    pub fn consider_option(mut self, option: ConsideredOption) -> Self {
        self.entry.considered_options.push(option);
        self
    }

    pub fn choose(mut self, option: ChosenOption) -> Self {
        self.entry.chosen = option;
        self
    }

    pub fn reason(mut self, reasoning: String) -> Self {
        self.entry.reasoning = reasoning;
        self
    }

    pub fn execute(mut self, record: ExecutionRecord) -> Self {
        self.entry.execution = record;
        self
    }

    pub fn outcome(mut self, record: OutcomeRecord) -> Self {
        self.entry.outcome = record;
        self
    }

    pub fn reflect(mut self, reflection: String) -> Self {
        self.entry.reflection = Some(reflection);
        self
    }

    pub fn build(self) -> DecisionEntry {
        let mut entry = self.entry;
        entry.context = serde_json::to_value(&self.context).unwrap_or(serde_json::Value::Null);
        entry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn decision_entry_new_creates_unique_id() {
        let entry1 = DecisionEntry::new("test goal".to_string());
        let entry2 = DecisionEntry::new("test goal".to_string());
        assert_ne!(entry1.entry_id, entry2.entry_id);
    }

    #[test]
    fn decision_entry_builder_pattern() {
        let entry = DecisionEntry::new("test goal".to_string())
            .with_reasoning("test reasoning".to_string())
            .with_chosen(ChosenOption::new("option1".to_string(), 0.9, "best choice".to_string()));

        assert_eq!(entry.goal, "test goal");
        assert_eq!(entry.reasoning, "test reasoning");
        assert_eq!(entry.chosen.option, "option1");
        assert_eq!(entry.chosen.confidence, 0.9);
    }

    #[test]
    fn considered_option_new() {
        let option = ConsideredOption::new("test option".to_string(), 0.75)
            .with_pros(vec!["fast".to_string(), "reliable".to_string()])
            .with_cons(vec!["complex".to_string()]);

        assert_eq!(option.option, "test option");
        assert_eq!(option.confidence, 0.75);
        assert_eq!(option.pros.len(), 2);
        assert_eq!(option.cons.len(), 1);
    }

    #[test]
    fn execution_record_new() {
        let record = ExecutionRecord::new("bash".to_string(), "execute".to_string(), 150)
            .with_parameters(serde_json::json!({"command": "ls"}));

        assert_eq!(record.tool, "bash");
        assert_eq!(record.action, "execute");
        assert_eq!(record.duration_ms, 150);
    }

    #[test]
    fn outcome_record_new() {
        let mut metrics = HashMap::new();
        metrics.insert("accuracy".to_string(), 0.95);
        
        let record = OutcomeRecord::new(true, 0.9, "Task completed successfully".to_string())
            .with_metrics(metrics.clone());

        assert!(record.success);
        assert_eq!(record.score, 0.9);
        assert_eq!(record.metrics.get("accuracy"), Some(&0.95));
    }

    #[test]
    fn decision_builder_builds_complete_entry() {
        let entry = DecisionBuilder::new("optimize performance".to_string())
            .with_session_id("session_123".to_string())
            .with_turn(5)
            .with_tools(vec!["bash".to_string(), "file".to_string()])
            .consider_option(ConsideredOption::new("cache".to_string(), 0.8))
            .consider_option(ConsideredOption::new("parallel".to_string(), 0.7))
            .choose(ChosenOption::new("cache".to_string(), 0.85, "best tradeoff".to_string()))
            .reason("Caching provides immediate gains".to_string())
            .execute(ExecutionRecord::new("file".to_string(), "write".to_string(), 50))
            .outcome(OutcomeRecord::new(true, 0.9, "Performance improved by 30%".to_string()))
            .build();

        assert_eq!(entry.goal, "optimize performance");
        assert_eq!(entry.considered_options.len(), 2);
        assert_eq!(entry.chosen.option, "cache");
        assert!(entry.outcome.success);
    }

    #[tokio::test]
    async fn file_decision_journal_records_and_queries() {
        let tmp = TempDir::new().unwrap();
        let journal = FileDecisionJournal::new(&tmp.path().to_path_buf()).unwrap();

        let entry = DecisionEntry::new("test goal 1".to_string())
            .with_outcome(OutcomeRecord::new(true, 0.9, "success".to_string()));
        
        journal.record_decision(entry).await.unwrap();

        let results = journal.query_by_goal("test").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].goal, "test goal 1");
    }

    #[tokio::test]
    async fn file_decision_journal_query_by_outcome() {
        let tmp = TempDir::new().unwrap();
        let journal = FileDecisionJournal::new(&tmp.path().to_path_buf()).unwrap();

        let entry1 = DecisionEntry::new("goal 1".to_string())
            .with_outcome(OutcomeRecord::new(true, 0.9, "success".to_string()));
        let entry2 = DecisionEntry::new("goal 2".to_string())
            .with_outcome(OutcomeRecord::new(false, 0.3, "failed".to_string()));

        journal.record_decision(entry1).await.unwrap();
        journal.record_decision(entry2).await.unwrap();

        let successful = journal.query_by_outcome(true).await.unwrap();
        let failed = journal.query_by_outcome(false).await.unwrap();

        assert_eq!(successful.len(), 1);
        assert_eq!(failed.len(), 1);
    }

    #[tokio::test]
    async fn file_decision_journal_get_learning_dataset() {
        let tmp = TempDir::new().unwrap();
        let journal = FileDecisionJournal::new(&tmp.path().to_path_buf()).unwrap();

        let good = DecisionEntry::new("good goal".to_string())
            .with_outcome(OutcomeRecord::new(true, 0.8, "good".to_string()));
        let bad = DecisionEntry::new("bad goal".to_string())
            .with_outcome(OutcomeRecord::new(false, 0.2, "bad".to_string()));
        let low_score = DecisionEntry::new("low score goal".to_string())
            .with_outcome(OutcomeRecord::new(true, 0.3, "low score".to_string()));

        journal.record_decision(good).await.unwrap();
        journal.record_decision(bad).await.unwrap();
        journal.record_decision(low_score).await.unwrap();

        let dataset = journal.get_learning_dataset().await.unwrap();
        assert_eq!(dataset.len(), 1);
        assert_eq!(dataset[0].goal, "good goal");
    }

    #[tokio::test]
    async fn file_decision_journal_get_stats() {
        let tmp = TempDir::new().unwrap();
        let journal = FileDecisionJournal::new(&tmp.path().to_path_buf()).unwrap();

        for i in 0..5 {
            let entry = DecisionEntry::new(format!("goal {}", i % 2))
                .with_outcome(OutcomeRecord::new(i < 3, 0.5 + (f64::from(i) * 0.1), "test".to_string()));
            journal.record_decision(entry).await.unwrap();
        }

        let stats = journal.get_stats().await.unwrap();
        assert_eq!(stats.total_entries, 5);
        assert_eq!(stats.successful_decisions, 3);
        assert_eq!(stats.failed_decisions, 2);
    }

    #[test]
    fn matches_pattern_works_correctly() {
        assert!(FileDecisionJournal::matches_pattern("optimize code", "optimize"));
        assert!(FileDecisionJournal::matches_pattern("Optimize Code", "optimize"));
        assert!(FileDecisionJournal::matches_pattern("test goal", "test*"));
        assert!(FileDecisionJournal::matches_pattern("goal test", "*test"));
        assert!(!FileDecisionJournal::matches_pattern("goal", "other"));
    }
}

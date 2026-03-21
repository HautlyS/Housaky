//! Memory Nudge System (inspired by Hermes Agent)
//!
//! Periodic nudges to persist important memories and skills.
//! This ensures knowledge isn't lost between sessions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

/// A memory nudge - a reminder to persist or recall important information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNudge {
    pub id: String,
    pub nudge_type: NudgeType,
    pub content: String,
    pub priority: u8,
    pub created_at: DateTime<Utc>,
    pub last_triggered: Option<DateTime<Utc>>,
    pub trigger_count: u32,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NudgeType {
    /// Persist an important memory
    PersistMemory,
    /// Recall something relevant
    RecallReminder,
    /// Learn from experience
    SkillCreation,
    /// Review and consolidate
    Consolidation,
    /// User preference update
    UserPreference,
}

/// Nudge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NudgeConfig {
    /// How often to check for nudges (seconds)
    pub check_interval_secs: u64,
    /// Minimum priority to trigger (0-10)
    pub min_priority: u8,
    /// Maximum nudges per check
    pub max_nudges_per_check: usize,
    /// Enable skill creation from experience
    pub enable_skill_learning: bool,
}

impl Default for NudgeConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 300, // 5 minutes
            min_priority: 5,
            max_nudges_per_check: 3,
            enable_skill_learning: true,
        }
    }
}

/// The nudge engine manages periodic memory persistence reminders
pub struct NudgeEngine {
    nudges: Arc<RwLock<Vec<MemoryNudge>>>,
    config: NudgeConfig,
    user_preferences: Arc<RwLock<HashMap<String, String>>>,
    experience_log: Arc<RwLock<Vec<ExperienceEntry>>>,
}

/// An experience entry for skill learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub task: String,
    pub outcome: String,
    pub tools_used: Vec<String>,
    pub success: bool,
    pub lessons_learned: Vec<String>,
    pub potential_skill: bool,
}

impl NudgeEngine {
    pub fn new(config: NudgeConfig) -> Self {
        Self {
            nudges: Arc::new(RwLock::new(Vec::new())),
            config,
            user_preferences: Arc::new(RwLock::new(HashMap::new())),
            experience_log: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a nudge to the queue
    pub async fn add_nudge(&self, nudge_type: NudgeType, content: String, priority: u8, tags: Vec<String>) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let nudge = MemoryNudge {
            id: id.clone(),
            nudge_type,
            content,
            priority,
            created_at: Utc::now(),
            last_triggered: None,
            trigger_count: 0,
            tags,
        };
        
        let mut nudges = self.nudges.write().await;
        nudges.push(nudge);
        nudges.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        id
    }

    /// Log an experience for potential skill creation
    pub async fn log_experience(
        &self,
        task: String,
        outcome: String,
        tools_used: Vec<String>,
        success: bool,
        lessons_learned: Vec<String>,
    ) {
        let entry = ExperienceEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            task,
            outcome,
            tools_used,
            success,
            lessons_learned,
            potential_skill: false,
        };
        
        let mut log = self.experience_log.write().await;
        log.push(entry);
        
        // Keep last 100 experiences
        if log.len() > 100 {
            log.remove(0);
        }
    }

    /// Check and return pending nudges
    pub async fn check_nudges(&self) -> Vec<MemoryNudge> {
        let mut nudges = self.nudges.write().await;
        let now = Utc::now();
        
        let mut triggered = Vec::new();
        let mut count = 0;
        
        for nudge in nudges.iter_mut() {
            if count >= self.config.max_nudges_per_check {
                break;
            }
            
            if nudge.priority >= self.config.min_priority {
                nudge.last_triggered = Some(now);
                nudge.trigger_count += 1;
                triggered.push(nudge.clone());
                count += 1;
            }
        }
        
        triggered
    }

    /// Get user preference
    pub async fn get_user_preference(&self, key: &str) -> Option<String> {
        let prefs = self.user_preferences.read().await;
        prefs.get(key).cloned()
    }

    /// Set user preference
    pub async fn set_user_preference(&self, key: String, value: String) {
        let mut prefs = self.user_preferences.write().await;
        prefs.insert(key, value);
    }

    /// Analyze experiences for skill creation opportunities
    pub async fn analyze_for_skill_creation(&self) -> Option<String> {
        if !self.config.enable_skill_learning {
            return None;
        }
        
        let log = self.experience_log.read().await;
        
        // Find patterns: tasks that succeeded multiple times with similar tools
        let mut task_patterns: HashMap<String, (u32, Vec<String>, Vec<String>)> = HashMap::new();
        
        for entry in log.iter() {
            if entry.success {
                let key = entry.task.split_whitespace().take(3).collect::<Vec<_>>().join(" ");
                let (count, tools, lessons) = task_patterns.entry(key).or_insert((0, Vec::new(), Vec::new()));
                *count += 1;
                for tool in &entry.tools_used {
                    if !tools.contains(tool) {
                        tools.push(tool.clone());
                    }
                }
                for lesson in &entry.lessons_learned {
                    if !lessons.contains(lesson) {
                        lessons.push(lesson.clone());
                    }
                }
            }
        }
        
        // Find a task done 3+ times
        for (task, (count, tools, _lessons)) in task_patterns.iter() {
            if *count >= 3 {
                return Some(format!(
                    "Potential skill: '{}' - Done {} times with tools: {}",
                    task,
                    count,
                    tools.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                ));
            }
        }
        
        None
    }

    /// Create a nudge for memory persistence
    pub async fn nudge_persist(&self, content: String, priority: u8) -> String {
        self.add_nudge(
            NudgeType::PersistMemory,
            content,
            priority,
            vec!["memory".to_string(), "persist".to_string()],
        ).await
    }

    /// Create a nudge for recall
    pub async fn nudge_recall(&self, topic: String, priority: u8) -> String {
        self.add_nudge(
            NudgeType::RecallReminder,
            topic,
            priority,
            vec!["memory".to_string(), "recall".to_string()],
        ).await
    }

    /// Get statistics
    pub async fn stats(&self) -> NudgeStats {
        let nudges = self.nudges.read().await;
        let experiences = self.experience_log.read().await;
        let prefs = self.user_preferences.read().await;
        
        NudgeStats {
            total_nudges: nudges.len(),
            pending_nudges: nudges.iter().filter(|n| n.last_triggered.is_none()).count(),
            total_experiences: experiences.len(),
            user_preferences: prefs.len(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NudgeStats {
    pub total_nudges: usize,
    pub pending_nudges: usize,
    pub total_experiences: usize,
    pub user_preferences: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_nudge_engine() {
        let engine = NudgeEngine::new(NudgeConfig::default());
        
        let id = engine.nudge_persist("Remember this important fact".to_string(), 7).await;
        assert!(!id.is_empty());
        
        let nudges = engine.check_nudges().await;
        assert_eq!(nudges.len(), 1);
        assert_eq!(nudges[0].trigger_count, 1);
    }

    #[tokio::test]
    async fn test_experience_logging() {
        let engine = NudgeEngine::new(NudgeConfig::default());
        
        engine.log_experience(
            "test task".to_string(),
            "success".to_string(),
            vec!["tool1".to_string()],
            true,
            vec!["learned something".to_string()],
        ).await;
        
        let stats = engine.stats().await;
        assert_eq!(stats.total_experiences, 1);
    }
}

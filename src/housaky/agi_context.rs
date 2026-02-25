use crate::housaky::alignment::ValueDriftDetector;
use crate::housaky::goal_engine::GoalEngine;
use crate::housaky::meta_cognition::MetaCognitionEngine;
use crate::housaky::self_improvement_mod::ContinuousLearningEngine;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub context: String,
    pub options_considered: Vec<String>,
    pub chosen_action: String,
    pub reasoning: String,
    pub confidence: f64,
    pub outcome: Option<DecisionOutcome>,
    pub goal_id: Option<String>,
    pub tool_name: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOutcome {
    pub success: bool,
    pub result_summary: String,
    pub feedback_score: Option<f64>,
    pub lessons_learned: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

#[async_trait]
pub trait DecisionJournal: Send + Sync {
    async fn record_decision(&self, record: DecisionRecord) -> Result<String>;
    async fn record_outcome(&self, decision_id: &str, outcome: DecisionOutcome) -> Result<()>;
    async fn get_decision(&self, id: &str) -> Option<DecisionRecord>;
    async fn get_recent_decisions(&self, limit: usize) -> Vec<DecisionRecord>;
    async fn get_decisions_for_goal(&self, goal_id: &str) -> Vec<DecisionRecord>;
    async fn analyze_patterns(&self) -> DecisionPatternAnalysis;
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DecisionPatternAnalysis {
    pub total_decisions: usize,
    pub success_rate: f64,
    pub common_patterns: Vec<DecisionPattern>,
    pub tool_usage_stats: HashMap<String, ToolUsageStats>,
    pub confidence_accuracy_correlation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionPattern {
    pub context_pattern: String,
    pub typical_action: String,
    pub frequency: usize,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageStats {
    pub uses: usize,
    pub success_rate: f64,
    pub avg_confidence: f64,
}

pub struct InMemoryDecisionJournal {
    decisions: Arc<Mutex<Vec<DecisionRecord>>>,
    max_entries: usize,
}

impl InMemoryDecisionJournal {
    pub fn new(max_entries: usize) -> Self {
        Self {
            decisions: Arc::new(Mutex::new(Vec::new())),
            max_entries,
        }
    }
}

#[async_trait]
impl DecisionJournal for InMemoryDecisionJournal {
    async fn record_decision(&self, mut record: DecisionRecord) -> Result<String> {
        let id = format!("decision_{}", uuid::Uuid::new_v4());
        record.id = id.clone();
        
        let mut decisions = self.decisions.lock().await;
        if decisions.len() >= self.max_entries {
            decisions.remove(0);
        }
        decisions.push(record);
        
        Ok(id)
    }

    async fn record_outcome(&self, decision_id: &str, outcome: DecisionOutcome) -> Result<()> {
        let mut decisions = self.decisions.lock().await;
        if let Some(decision) = decisions.iter_mut().find(|d| d.id == decision_id) {
            decision.outcome = Some(outcome);
        }
        Ok(())
    }

    async fn get_decision(&self, id: &str) -> Option<DecisionRecord> {
        let decisions = self.decisions.lock().await;
        decisions.iter().find(|d| d.id == id).cloned()
    }

    async fn get_recent_decisions(&self, limit: usize) -> Vec<DecisionRecord> {
        let decisions = self.decisions.lock().await;
        decisions.iter().rev().take(limit).cloned().collect()
    }

    async fn get_decisions_for_goal(&self, goal_id: &str) -> Vec<DecisionRecord> {
        let decisions = self.decisions.lock().await;
        decisions
            .iter()
            .filter(|d| d.goal_id.as_deref() == Some(goal_id))
            .cloned()
            .collect()
    }

    async fn analyze_patterns(&self) -> DecisionPatternAnalysis {
        let decisions = self.decisions.lock().await;
        
        if decisions.is_empty() {
            return DecisionPatternAnalysis::default();
        }

        let total = decisions.len();
        let successful = decisions.iter().filter(|d| {
            d.outcome.as_ref().map(|o| o.success).unwrap_or(false)
        }).count();
        
        let success_rate = successful as f64 / total as f64;

        let mut tool_stats: HashMap<String, (usize, usize, f64)> = HashMap::new();
        for decision in decisions.iter() {
            if let Some(ref tool) = decision.tool_name {
                let entry = tool_stats.entry(tool.clone()).or_insert((0, 0, 0.0));
                entry.0 += 1;
                if decision.outcome.as_ref().map(|o| o.success).unwrap_or(false) {
                    entry.1 += 1;
                }
                entry.2 += decision.confidence;
            }
        }

        let tool_usage_stats: HashMap<String, ToolUsageStats> = tool_stats
            .into_iter()
            .map(|(tool, (uses, successes, total_conf))| {
                (
                    tool,
                    ToolUsageStats {
                        uses,
                        success_rate: successes as f64 / uses as f64,
                        avg_confidence: total_conf / uses as f64,
                    },
                )
            })
            .collect();

        DecisionPatternAnalysis {
            total_decisions: total,
            success_rate,
            common_patterns: Vec::new(),
            tool_usage_stats,
            confidence_accuracy_correlation: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct AGIConfig {
    pub enable_goal_tracking: bool,
    pub enable_learning: bool,
    pub enable_meta_cognition: bool,
    pub enable_decision_journal: bool,
    pub enable_drift_detection: bool,
    pub reflection_interval_turns: u32,
    pub drift_check_interval_turns: u32,
    pub max_decisions_in_journal: usize,
}

impl Default for AGIConfig {
    fn default() -> Self {
        Self {
            enable_goal_tracking: true,
            enable_learning: true,
            enable_meta_cognition: true,
            enable_decision_journal: true,
            enable_drift_detection: true,
            reflection_interval_turns: 10,
            drift_check_interval_turns: 20,
            max_decisions_in_journal: 1000,
        }
    }
}

pub struct AGIContext {
    pub goal_engine: Option<Arc<Mutex<GoalEngine>>>,
    pub learning_engine: Option<Arc<Mutex<ContinuousLearningEngine>>>,
    pub meta_cognition: Option<Arc<Mutex<MetaCognitionEngine>>>,
    pub decision_journal: Option<Arc<dyn DecisionJournal>>,
    pub drift_detector: Option<Arc<Mutex<ValueDriftDetector>>>,
    pub config: AGIConfig,
    workspace_dir: PathBuf,
    turn_counter: Arc<Mutex<u32>>,
}

impl AGIContext {
    pub fn new(workspace_dir: &PathBuf, config: AGIConfig) -> Self {
        let goal_engine = if config.enable_goal_tracking {
            Some(Arc::new(Mutex::new(GoalEngine::new(workspace_dir))))
        } else {
            None
        };

        let learning_engine = if config.enable_learning {
            Some(Arc::new(Mutex::new(ContinuousLearningEngine::new())))
        } else {
            None
        };

        let meta_cognition = if config.enable_meta_cognition {
            Some(Arc::new(Mutex::new(MetaCognitionEngine::new())))
        } else {
            None
        };

        let decision_journal: Option<Arc<dyn DecisionJournal>> = if config.enable_decision_journal {
            Some(Arc::new(InMemoryDecisionJournal::new(config.max_decisions_in_journal)))
        } else {
            None
        };

        let drift_detector = if config.enable_drift_detection {
            let mut detector = ValueDriftDetector::new();
            detector.establish_baseline(vec![
                crate::housaky::alignment::ValueBaseline::new("Safety", "Avoid harm", 0.95)
                    .with_constraints(vec!["minimum 0.8".to_string()]),
                crate::housaky::alignment::ValueBaseline::new("Truth", "Be honest", 0.9),
                crate::housaky::alignment::ValueBaseline::new("Helpfulness", "Assist users", 0.85),
            ]);
            Some(Arc::new(Mutex::new(detector)))
        } else {
            None
        };

        Self {
            goal_engine,
            learning_engine,
            meta_cognition,
            decision_journal,
            drift_detector,
            config,
            workspace_dir: workspace_dir.clone(),
            turn_counter: Arc::new(Mutex::new(0)),
        }
    }

    pub fn disabled() -> Self {
        Self {
            goal_engine: None,
            learning_engine: None,
            meta_cognition: None,
            decision_journal: None,
            drift_detector: None,
            config: AGIConfig {
                enable_goal_tracking: false,
                enable_learning: false,
                enable_meta_cognition: false,
                enable_decision_journal: false,
                enable_drift_detection: false,
                reflection_interval_turns: 0,
                drift_check_interval_turns: 0,
                max_decisions_in_journal: 0,
            },
            workspace_dir: PathBuf::new(),
            turn_counter: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn increment_turn(&self) -> u32 {
        let mut counter = self.turn_counter.lock().await;
        *counter += 1;
        *counter
    }

    pub async fn get_turn(&self) -> u32 {
        *self.turn_counter.lock().await
    }

    pub async fn should_reflect(&self) -> bool {
        if !self.config.enable_meta_cognition {
            return false;
        }
        let turn = self.get_turn().await;
        turn > 0 && turn % self.config.reflection_interval_turns == 0
    }

    pub async fn should_check_drift(&self) -> bool {
        if !self.config.enable_drift_detection {
            return false;
        }
        let turn = self.get_turn().await;
        turn > 0 && turn % self.config.drift_check_interval_turns == 0
    }

    pub async fn record_tool_feedback(
        &self,
        tool_name: &str,
        success: bool,
        context: HashMap<String, String>,
    ) {
        if let Some(ref engine) = self.learning_engine {
            let feedback = crate::housaky::self_improvement_mod::LearningFeedback {
                action: tool_name.to_string(),
                outcome: if success { "success" } else { "failure" }.to_string(),
                success,
                context,
                timestamp: Utc::now(),
                reward: if success { 1.0 } else { -0.5 },
            };

            let mut engine = engine.lock().await;
            engine.record_feedback(feedback);

            // Persist the learning model so self-improvement survives restarts.
            // This is intentionally best-effort; a failure to persist should not break runtime.
            if let Err(e) = self.persist_learning_model(&engine).await {
                tracing::warn!("Failed to persist ContinuousLearningEngine model: {e}");
            }
        }
    }

    async fn persist_learning_model(
        &self,
        engine: &ContinuousLearningEngine,
    ) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        // If disabled() was used, workspace_dir may be empty.
        if self.workspace_dir.as_os_str().is_empty() {
            return Ok(());
        }

        let dir = self.workspace_dir.join(".housaky").join("learning");
        tokio::fs::create_dir_all(&dir).await?;

        let model = engine.export_model();
        let json = serde_json::to_vec_pretty(&model)?;

        let tmp_path = dir.join("continuous_learning.json.tmp");
        let final_path = dir.join("continuous_learning.json");

        let mut f = tokio::fs::File::create(&tmp_path).await?;
        f.write_all(&json).await?;
        f.flush().await?;
        drop(f);

        // Atomic-ish replace.
        tokio::fs::rename(&tmp_path, &final_path).await?;

        Ok(())
    }

    pub async fn try_load_learning_model(&self) -> Result<()> {
        if let Some(ref engine) = self.learning_engine {
            if self.workspace_dir.as_os_str().is_empty() {
                return Ok(());
            }

            let path = self
                .workspace_dir
                .join(".housaky")
                .join("learning")
                .join("continuous_learning.json");

            if !path.exists() {
                return Ok(());
            }

            let bytes = tokio::fs::read(&path).await?;
            let model: crate::housaky::self_improvement_mod::LearningModel =
                serde_json::from_slice(&bytes)?;

            let mut engine = engine.lock().await;
            engine.import_model(model);
        }

        Ok(())
    }

    pub async fn record_decision(
        &self,
        context: &str,
        chosen_action: &str,
        reasoning: &str,
        confidence: f64,
        goal_id: Option<&str>,
        tool_name: Option<&str>,
    ) -> Option<String> {
        if let Some(ref journal) = self.decision_journal {
            let record = DecisionRecord {
                id: String::new(),
                timestamp: Utc::now(),
                context: context.to_string(),
                options_considered: vec![chosen_action.to_string()],
                chosen_action: chosen_action.to_string(),
                reasoning: reasoning.to_string(),
                confidence,
                outcome: None,
                goal_id: goal_id.map(|s| s.to_string()),
                tool_name: tool_name.map(|s| s.to_string()),
                metadata: HashMap::new(),
            };
            Some(journal.record_decision(record).await.ok()?)
        } else {
            None
        }
    }

    pub async fn record_decision_outcome(
        &self,
        decision_id: &str,
        success: bool,
        result_summary: &str,
    ) {
        if let Some(ref journal) = self.decision_journal {
            let outcome = DecisionOutcome {
                success,
                result_summary: result_summary.to_string(),
                feedback_score: Some(if success { 1.0 } else { 0.0 }),
                lessons_learned: Vec::new(),
                timestamp: Utc::now(),
            };
            let _ = journal.record_outcome(decision_id, outcome).await;
        }
    }

    pub async fn run_reflection(&self, trigger: &str) -> Option<String> {
        if let Some(ref meta) = self.meta_cognition {
            let meta_guard = meta.lock().await;
            if let Ok(reflection) = meta_guard.reflect(trigger).await {
                let summary = format!(
                    "Reflection: {} observations, {} insights. Mood: {:?}",
                    reflection.observations.len(),
                    reflection.insights.len(),
                    reflection.mood
                );
                return Some(summary);
            }
        }
        None
    }

    pub async fn check_value_alignment(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if let Some(ref detector) = self.drift_detector {
            let mut detector = detector.lock().await;
            let mut current_values = HashMap::new();
            current_values.insert("Safety".to_string(), 0.95);
            current_values.insert("Truth".to_string(), 0.9);
            current_values.insert("Helpfulness".to_string(), 0.85);

            let drift_events = detector.check_drift(&current_values);
            for event in drift_events {
                if event.severity >= crate::housaky::alignment::DriftSeverity::Moderate {
                    warnings.push(format!(
                        "Value drift detected for '{}': {}",
                        event.value_name, event.potential_cause
                    ));
                }
            }
        }
        warnings
    }
}

impl Default for AGIContext {
    fn default() -> Self {
        Self::disabled()
    }
}

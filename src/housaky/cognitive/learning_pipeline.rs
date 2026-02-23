use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{error, info};

use super::information_gap::InformationGapEngine;
use super::meta_learning::MetaLearningEngine;
use super::world_model::WorldModel;
use crate::housaky::memory::hierarchical::{ActionRecord, EventType, OutcomeRecord};
use crate::housaky::goal_engine::GoalEngine;
use crate::housaky::memory::hierarchical::HierarchicalMemory;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentInteraction {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub input: String,
    pub output: String,
    pub tool_calls: Vec<String>,
    pub tool_results: Vec<ToolResult>,
    pub success: bool,
    pub duration_ms: u64,
    pub context: InteractionContext,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_name: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct InteractionContext {
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub domain: Option<String>,
    pub task_type: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LearningReport {
    pub interactions_processed: usize,
    pub facts_extracted: usize,
    pub beliefs_updated: usize,
    pub gaps_identified: usize,
    pub new_goals_created: usize,
    pub world_model_updates: usize,
    pub timestamp: DateTime<Utc>,
}

pub struct LearningPipeline {
    unified_memory: Option<Arc<HierarchicalMemory>>,
    information_gap: Option<Arc<InformationGapEngine>>,
    world_model: Option<Arc<WorldModel>>,
    meta_learning: Option<Arc<MetaLearningEngine>>,
    goal_engine: Option<Arc<GoalEngine>>,
    interaction_buffer: Arc<RwLock<VecDeque<AgentInteraction>>>,
    config: LearningConfig,
    running: Arc<RwLock<bool>>,
    storage_path: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LearningConfig {
    pub enabled: bool,
    pub batch_size: usize,
    pub learning_interval_secs: u64,
    pub min_confidence_to_learn: f64,
    pub max_gaps_per_cycle: usize,
    pub background_learning_enabled: bool,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            batch_size: 10,
            learning_interval_secs: 300,
            min_confidence_to_learn: 0.5,
            max_gaps_per_cycle: 3,
            background_learning_enabled: true,
        }
    }
}

impl LearningPipeline {
    pub fn new(workspace_dir: &PathBuf) -> Self {
        let storage_path = workspace_dir.join(".housaky").join("learning");
        Self {
            unified_memory: None,
            information_gap: None,
            world_model: None,
            meta_learning: None,
            goal_engine: None,
            interaction_buffer: Arc::new(RwLock::new(VecDeque::new())),
            config: LearningConfig::default(),
            running: Arc::new(RwLock::new(false)),
            storage_path,
        }
    }

    pub fn with_memory(mut self, memory: Arc<HierarchicalMemory>) -> Self {
        self.unified_memory = Some(memory);
        self
    }

    pub fn with_information_gap(mut self, gap_engine: Arc<InformationGapEngine>) -> Self {
        self.information_gap = Some(gap_engine);
        self
    }

    pub fn with_world_model(mut self, world_model: Arc<WorldModel>) -> Self {
        self.world_model = Some(world_model);
        self
    }

    pub fn with_meta_learning(mut self, meta: Arc<MetaLearningEngine>) -> Self {
        self.meta_learning = Some(meta);
        self
    }

    pub fn with_goal_engine(mut self, goals: Arc<GoalEngine>) -> Self {
        self.goal_engine = Some(goals);
        self
    }

    pub fn with_config(mut self, config: LearningConfig) -> Self {
        self.config = config;
        self
    }

    pub async fn initialize(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.storage_path).await?;
        info!("Learning pipeline initialized");
        Ok(())
    }

    pub async fn record_interaction(&self, interaction: AgentInteraction) {
        let mut buffer = self.interaction_buffer.write().await;
        buffer.push_back(interaction);

        if buffer.len() > self.config.batch_size * 10 {
            buffer.pop_front();
        }

        info!("Recorded interaction, buffer size: {}", buffer.len());
    }

    pub async fn process_interaction(&self, interaction: AgentInteraction) -> Result<LearningReport> {
        let mut report = LearningReport {
            interactions_processed: 1,
            facts_extracted: 0,
            beliefs_updated: 0,
            gaps_identified: 0,
            new_goals_created: 0,
            world_model_updates: 0,
            timestamp: Utc::now(),
        };

        self.record_interaction(interaction.clone()).await;

        if let Some(ref memory) = self.unified_memory {
            let _ = memory.store_episode(
                EventType::Learning,
                &interaction.input,
                interaction
                    .tool_calls
                    .iter()
                    .map(|t| ActionRecord {
                        action_type: t.clone(),
                        description: t.clone(),
                        timestamp: interaction.timestamp,
                        success: interaction.success,
                        duration_ms: interaction.duration_ms,
                    })
                    .collect(),
                OutcomeRecord {
                    result: interaction.output.clone(),
                    success: interaction.success,
                    side_effects: vec![],
                    user_feedback: None,
                },
            ).await;
            report.facts_extracted = 1;
        }

        if let Some(ref gap_engine) = self.information_gap {
            let context = super::information_gap::CuriosityContext {
                current_goals: vec![],
                recent_events: vec![interaction.input.clone()],
                uncertain_topics: vec![],
                existing_knowledge: vec![],
                active_tasks: vec![],
            };

            let gaps = gap_engine.identify_gaps(&context).await;
            report.gaps_identified = gaps.len();

            for gap in gaps.iter().take(self.config.max_gaps_per_cycle) {
                if let Some(ref goals) = self.goal_engine {
                    let goal = gap_engine.create_learning_goal(gap).await?;
                    goals.add_goal(goal).await?;
                    report.new_goals_created += 1;
                }
            }
        }

        if let Some(ref world_model) = self.world_model {
            if interaction.success {
                let mut state = super::world_model::WorldState::default();
                state.context.insert("success".to_string(), "true".to_string());
                world_model.update_state(state).await;
                report.world_model_updates += 1;
            }
        }

        if let Some(ref meta) = self.meta_learning {
            if let Some(_goal) = interaction.context.task_type.as_ref() {
                let outcome = super::meta_learning::LearningOutcome {
                    id: uuid::Uuid::new_v4().to_string(),
                    strategy_id: meta.get_current_strategy().await.unwrap_or_default(),
                    task: super::meta_learning::LearningTask {
                        id: interaction.id.clone(),
                        task_type: super::meta_learning::TaskType::ProblemSolving,
                        domain: interaction.context.domain.clone().unwrap_or_default(),
                        difficulty: 0.5,
                        context: interaction.context.task_type.clone().map(|t| (t, "1".to_string())).into_iter().collect(),
                        required_skills: vec![],
                    },
                    success: interaction.success,
                    time_taken_ms: interaction.duration_ms,
                    improvement: if interaction.success { 0.1 } else { 0.0 },
                    feedback: interaction.output.clone(),
                    timestamp: interaction.timestamp,
                };
                meta.record_outcome(outcome).await;
            }
        }

        Ok(report)
    }

    pub async fn process_batch(&self) -> Result<LearningReport> {
        let mut batch = Vec::new();
        {
            let mut buffer = self.interaction_buffer.write().await;
            for _ in 0..self.config.batch_size {
                if let Some(interaction) = buffer.pop_front() {
                    batch.push(interaction);
                }
            }
        }

        let mut report = LearningReport {
            interactions_processed: 0,
            facts_extracted: 0,
            beliefs_updated: 0,
            gaps_identified: 0,
            new_goals_created: 0,
            world_model_updates: 0,
            timestamp: Utc::now(),
        };

        for interaction in batch {
            let interaction_report = self.process_interaction(interaction).await?;
            report.interactions_processed += interaction_report.interactions_processed;
            report.facts_extracted += interaction_report.facts_extracted;
            report.gaps_identified += interaction_report.gaps_identified;
            report.new_goals_created += interaction_report.new_goals_created;
            report.world_model_updates += interaction_report.world_model_updates;
        }

        info!(
            "Batch processing complete: {} interactions, {} facts, {} goals",
            report.interactions_processed,
            report.facts_extracted,
            report.new_goals_created
        );

        Ok(report)
    }

    pub async fn background_learning(&self) {
        if !self.config.background_learning_enabled {
            return;
        }

        *self.running.write().await = true;

        let mut interval = interval(Duration::from_secs(self.config.learning_interval_secs));

        while *self.running.read().await {
            interval.tick().await;

            if let Err(e) = self.process_batch().await {
                error!("Background learning error: {}", e);
            }

            if let Some(ref meta) = self.meta_learning {
                if let Err(e) = meta.save().await {
                    error!("Failed to save meta-learning state: {}", e);
                }
            }

            info!("Background learning cycle complete");
        }
    }

    pub async fn start_background_learning(&self) -> tokio::task::JoinHandle<()> {
        let pipeline = Arc::new(self.clone_inner());
        let config = self.config.clone();

        tokio::spawn(async move {
            if !config.background_learning_enabled {
                return;
            }

            let mut interval = interval(Duration::from_secs(config.learning_interval_secs));

            loop {
                interval.tick().await;

                if let Err(e) = pipeline.process_batch().await {
                    error!("Background learning error: {}", e);
                }

                info!("Background learning cycle complete");
            }
        })
    }

    fn clone_inner(&self) -> Self {
        Self {
            unified_memory: self.unified_memory.clone(),
            information_gap: self.information_gap.clone(),
            world_model: self.world_model.clone(),
            meta_learning: self.meta_learning.clone(),
            goal_engine: self.goal_engine.clone(),
            interaction_buffer: self.interaction_buffer.clone(),
            config: self.config.clone(),
            running: self.running.clone(),
            storage_path: self.storage_path.clone(),
        }
    }

    pub async fn stop(&self) {
        *self.running.write().await = false;
    }

    pub async fn get_pending_count(&self) -> usize {
        self.interaction_buffer.read().await.len()
    }
}

impl Clone for LearningPipeline {
    fn clone(&self) -> Self {
        self.clone_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_interaction() {
        let temp_dir = std::env::temp_dir();
        let pipeline = LearningPipeline::new(&temp_dir);
        pipeline.initialize().await.unwrap();

        let interaction = AgentInteraction {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            input: "Test input".to_string(),
            output: "Test output".to_string(),
            tool_calls: vec![],
            tool_results: vec![],
            success: true,
            duration_ms: 100,
            context: InteractionContext::default(),
        };

        pipeline.record_interaction(interaction).await;
        assert_eq!(pipeline.get_pending_count().await, 1);
    }
}

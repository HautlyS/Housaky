use crate::housaky::goal_engine::{Goal, GoalEngine, GoalPriority, GoalStatus};
use crate::housaky::gsd_orchestration::{
    DecompositionContext, GSDOrchestrator, GSDTaskStatus, PhaseStatus,
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone)]
pub struct GoalPhaseMapping {
    pub goal_id: String,
    pub phase_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_sync: chrono::DateTime<chrono::Utc>,
    pub progress_before: f64,
}

pub struct GoalTaskBridge {
    goal_engine: Arc<GoalEngine>,
    gsd_orchestrator: Arc<GSDOrchestrator>,
    mappings: Arc<RwLock<HashMap<String, GoalPhaseMapping>>>,
    auto_spawn_threshold: GoalPriority,
}

impl GoalTaskBridge {
    pub fn new(
        goal_engine: Arc<GoalEngine>,
        gsd_orchestrator: Arc<GSDOrchestrator>,
    ) -> Self {
        Self {
            goal_engine,
            gsd_orchestrator,
            mappings: Arc::new(RwLock::new(HashMap::new())),
            auto_spawn_threshold: GoalPriority::High,
        }
    }

    pub fn with_auto_spawn_threshold(mut self, threshold: GoalPriority) -> Self {
        self.auto_spawn_threshold = threshold;
        self
    }

    pub async fn check_and_spawn_phases(&self) -> Result<Vec<String>> {
        let goals = self.goal_engine.get_active_goals().await;
        let mut spawned = Vec::new();
        let mappings = self.mappings.read().await;

        for goal in goals {
            if mappings.contains_key(&goal.id) {
                continue;
            }

            if goal.status != GoalStatus::InProgress {
                continue;
            }

            let should_spawn = match self.auto_spawn_threshold {
                GoalPriority::Critical => goal.priority == GoalPriority::Critical,
                GoalPriority::High => {
                    matches!(goal.priority, GoalPriority::Critical | GoalPriority::High)
                }
                GoalPriority::Medium => matches!(
                    goal.priority,
                    GoalPriority::Critical | GoalPriority::High | GoalPriority::Medium
                ),
                GoalPriority::Low => {
                    matches!(
                        goal.priority,
                        GoalPriority::Critical | GoalPriority::High | GoalPriority::Medium | GoalPriority::Low
                    )
                }
                GoalPriority::Background => true,
            };

            if should_spawn {
                drop(mappings);
                let phase_id = self.spawn_phase_for_goal(&goal).await?;
                spawned.push(phase_id);
                return Ok(spawned);
            }
        }

        Ok(spawned)
    }

    pub async fn spawn_phase_for_goal(&self, goal: &Goal) -> Result<String> {
        info!("Spawning GSD phase for goal: {} ({})", goal.title, goal.id);

        let phase_name = format!("Goal: {}", goal.title);
        let phase_description = goal.description.clone();
        let goals = vec![goal.title.clone()];

        let phase_id = self
            .gsd_orchestrator
            .create_phase(phase_name, phase_description, goals)
            .await?;

        let mapping = GoalPhaseMapping {
            goal_id: goal.id.clone(),
            phase_id: phase_id.clone(),
            created_at: chrono::Utc::now(),
            last_sync: chrono::Utc::now(),
            progress_before: goal.progress,
        };

        self.mappings.write().await.insert(goal.id.clone(), mapping);

        info!("Created phase {} for goal {}", phase_id, goal.id);
        Ok(phase_id)
    }

    pub async fn sync_progress_to_goals(&self) -> Result<()> {
        let mappings = self.mappings.read().await.clone();

        for (_, mapping) in mappings {
            if let Some(phase_status) = self
                .gsd_orchestrator
                .get_phase_status(&mapping.phase_id)
                .await
            {
                let progress = match phase_status {
                    PhaseStatus::Pending => 0.0,
                    PhaseStatus::InProgress => 0.5,
                    PhaseStatus::Completed => 1.0,
                    PhaseStatus::Failed => mapping.progress_before,
                    PhaseStatus::Verified => 1.0,
                };

                self.goal_engine
                    .update_progress(&mapping.goal_id, progress, "GSD phase progress sync")
                    .await?;

                if matches!(phase_status, PhaseStatus::Completed | PhaseStatus::Verified) {
                    self.goal_engine
                        .update_progress(&mapping.goal_id, 1.0, "GSD phase completed")
                        .await?;
                    info!(
                        "Goal {} marked as completed via phase {}",
                        mapping.goal_id, mapping.phase_id
                    );
                }
            }
        }

        Ok(())
    }

    pub async fn decompose_goal_into_tasks(
        &self,
        goal_id: &str,
        subtasks: Vec<String>,
    ) -> Result<Vec<String>> {
        let mappings = self.mappings.read().await;
        let mapping = mappings
            .get(goal_id)
            .ok_or_else(|| anyhow::anyhow!("No phase mapping for goal {}", goal_id))?;

        let context = DecompositionContext {
            technology: Some("Rust".to_string()),
            requirements: vec![],
            constraints: vec![],
            existing_files: vec![],
            project_type: Some("CLI Tool".to_string()),
        };

        let phase_id = mapping.phase_id.clone();
        drop(mappings);

        let task_ids = self
            .gsd_orchestrator
            .plan_phase(&phase_id, subtasks)
            .await?;

        info!(
            "Created {} tasks for goal {} in phase {}",
            task_ids.len(),
            goal_id,
            phase_id
        );

        Ok(task_ids)
    }

    pub async fn execute_goal_phase(&self, goal_id: &str) -> Result<Option<String>> {
        let mappings = self.mappings.read().await;
        let mapping = mappings
            .get(goal_id)
            .ok_or_else(|| anyhow::anyhow!("No phase mapping for goal {}", goal_id))?;

        let phase_id = mapping.phase_id.clone();
        drop(mappings);

        let summary = self.gsd_orchestrator.execute_phase(&phase_id).await?;

        self.sync_progress_to_goals().await?;

        if summary.successful_tasks == summary.total_tasks {
            Ok(Some(format!(
                "Phase {} completed: {} tasks successful",
                phase_id, summary.successful_tasks
            )))
        } else {
            Ok(Some(format!(
                "Phase {} partial: {}/{} tasks successful",
                phase_id, summary.successful_tasks, summary.total_tasks
            )))
        }
    }

    pub async fn get_goal_phase_id(&self, goal_id: &str) -> Option<String> {
        let mappings = self.mappings.read().await;
        mappings.get(goal_id).map(|m| m.phase_id.clone())
    }

    pub async fn get_phase_goal_id(&self, phase_id: &str) -> Option<String> {
        let mappings = self.mappings.read().await;
        for mapping in mappings.values() {
            if mapping.phase_id == phase_id {
                return Some(mapping.goal_id.clone());
            }
        }
        None
    }

    pub async fn get_pending_executable_phases(&self) -> Vec<String> {
        let mappings = self.mappings.read().await;
        let mut executable = Vec::new();

        for mapping in mappings.values() {
            if let Some(status) = self
                .gsd_orchestrator
                .get_phase_status(&mapping.phase_id)
                .await
            {
                if matches!(status, PhaseStatus::Pending | PhaseStatus::InProgress) {
                    executable.push(mapping.phase_id.clone());
                }
            }
        }

        executable
    }

    pub async fn link_existing_goal_phase(&self, goal_id: &str, phase_id: &str) -> Result<()> {
        let mapping = GoalPhaseMapping {
            goal_id: goal_id.to_string(),
            phase_id: phase_id.to_string(),
            created_at: chrono::Utc::now(),
            last_sync: chrono::Utc::now(),
            progress_before: 0.0,
        };

        self.mappings.write().await.insert(goal_id.to_string(), mapping);
        info!("Linked goal {} to phase {}", goal_id, phase_id);
        Ok(())
    }

    pub async fn get_bridge_stats(&self) -> BridgeStats {
        let mappings = self.mappings.read().await;
        let total_links = mappings.len();
        
        let mut active_links = 0;
        for mapping in mappings.values() {
            if self.gsd_orchestrator
                .get_phase_status(&mapping.phase_id)
                .await
                .map(|s| matches!(s, PhaseStatus::InProgress))
                .unwrap_or(false)
            {
                active_links += 1;
            }
        }
        
        BridgeStats {
            total_links,
            active_links,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeStats {
    pub total_links: usize,
    pub active_links: usize,
}

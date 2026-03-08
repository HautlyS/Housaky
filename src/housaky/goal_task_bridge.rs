use crate::housaky::goal_engine::{Goal, GoalEngine, GoalPriority, GoalStatus};
use crate::housaky::gsd_orchestration::{
    DecompositionContext, GSDOrchestrator, PhaseStatus,
};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct GoalPhaseMapping {
    pub goal_id: String,
    pub phase_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_sync: chrono::DateTime<chrono::Utc>,
    pub progress_before: f64,
    pub last_error: Option<String>,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEvent {
    pub event_type: SyncEventType,
    pub goal_id: String,
    pub phase_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub details: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncEventType {
    GoalStarted,
    GoalProgress,
    GoalCompleted,
    GoalFailed,
    PhaseStarted,
    PhaseProgress,
    PhaseCompleted,
    PhaseFailed,
    Error,
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
            last_error: None,
            retry_count: 0,
        };

        self.mappings.write().await.insert(goal.id.clone(), mapping);

        info!("Created phase {} for goal {}", phase_id, goal.id);
        Ok(phase_id)
    }

    /// Sync progress from phases to goals (phase -> goal)
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

    /// Sync status from goals to phases (goal -> phase)
    pub async fn sync_goals_to_phases(&self) -> Result<()> {
        let mappings = self.mappings.read().await.clone();
        
        for (goal_id, mapping) in &mappings {
            // Get current goal status
            let goals = self.goal_engine.get_goals_filtered(|g| &g.id == goal_id).await;
            
            if let Some(goal) = goals.first() {
                match goal.status {
                    GoalStatus::InProgress => {
                        // Goal started - ensure phase is running
                        if let Some(phase_status) = self.gsd_orchestrator.get_phase_status(&mapping.phase_id).await {
                            if matches!(phase_status, PhaseStatus::Pending) {
                                info!("Goal {} is in progress, phase {} should be started", goal_id, mapping.phase_id);
                            }
                        }
                    }
                    GoalStatus::Completed | GoalStatus::Cancelled => {
                        // Goal ended - mark phase complete or cancelled
                        info!("Goal {} completed/cancelled, phase {} should reflect this", goal_id, mapping.phase_id);
                    }
                    GoalStatus::Failed => {
                        // Goal failed - propagate to phase
                        warn!("Goal {} failed, propagating failure to phase {}", goal_id, mapping.phase_id);
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// Propagate errors from phases to goals
    pub async fn propagate_errors_to_goals(&self, error_message: &str) -> Result<()> {
        let mut mappings = self.mappings.write().await;
        
        for (goal_id, mapping) in mappings.iter_mut() {
            if let Some(phase_status) = self.gsd_orchestrator.get_phase_status(&mapping.phase_id).await {
                if matches!(phase_status, PhaseStatus::Failed) {
                    mapping.last_error = Some(error_message.to_string());
                    mapping.retry_count += 1;
                    
                    // Mark goal as failed if too many retries
                    if mapping.retry_count >= 3 {
                        self.goal_engine.mark_failed(goal_id, error_message).await?;
                        warn!("Goal {} marked as failed after {} retries", goal_id, mapping.retry_count);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get bidirectional sync status
    pub async fn get_sync_status(&self) -> Vec<SyncStatus> {
        let mappings = self.mappings.read().await;
        
        let mut status = Vec::new();
        
        for (goal_id, mapping) in mappings.iter() {
            let phase_status = self.gsd_orchestrator.get_phase_status(&mapping.phase_id).await;
            
            status.push(SyncStatus {
                goal_id: goal_id.clone(),
                phase_id: mapping.phase_id.clone(),
                last_sync: mapping.last_sync,
                goal_progress: self.goal_engine.get_goals_filtered(|g| &g.id == goal_id).await.first().map(|g| g.progress),
                phase_status: phase_status.clone(),
                has_error: mapping.last_error.is_some(),
                retry_count: mapping.retry_count,
            });
        }
        
        status
    }

    /// Force resync of a specific goal-phase pair
    pub async fn resync_goal_phase(&self, goal_id: &str) -> Result<()> {
        let mapping = {
            let mappings = self.mappings.read().await;
            mappings.get(goal_id).cloned()
        };
        
        if let Some(mapping) = mapping {
            // Update last sync time
            {
                let mut mappings = self.mappings.write().await;
                if let Some(m) = mappings.get_mut(goal_id) {
                    m.last_sync = Utc::now();
                }
            }
            
            // Sync phase -> goal
            if let Some(phase_status) = self.gsd_orchestrator.get_phase_status(&mapping.phase_id).await {
                let progress = match phase_status {
                    PhaseStatus::Pending => 0.0,
                    PhaseStatus::InProgress => 0.5,
                    PhaseStatus::Completed | PhaseStatus::Verified => 1.0,
                    PhaseStatus::Failed => mapping.progress_before,
                };
                
                self.goal_engine.update_progress(goal_id, progress, "Manual resync").await?;
            }
            
            info!("Resynced goal {} with phase {}", goal_id, mapping.phase_id);
        }
        
        Ok(())
    }

    /// Start watching for changes (returns a channel to receive sync events)
    pub async fn start_watching(&self) -> mpsc::Receiver<SyncEvent> {
        let (tx, rx) = mpsc::channel(100);
        
        // In a real implementation, this would spawn a background task
        // that periodically checks for changes and sends events
        let _ = tx;
        
        rx
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

        let _context = DecompositionContext {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub goal_id: String,
    pub phase_id: String,
    pub last_sync: chrono::DateTime<chrono::Utc>,
    pub goal_progress: Option<f64>,
    pub phase_status: Option<PhaseStatus>,
    pub has_error: bool,
    pub retry_count: u32,
}

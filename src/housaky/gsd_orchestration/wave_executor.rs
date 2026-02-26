use super::task::{GSDTask, GSDTaskStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wave {
    pub number: u32,
    pub task_ids: Vec<String>,
    pub status: WaveStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WaveStatus {
    Pending,
    Running,
    Completed,
    Failed,
    PartiallyCompleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub task_id: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: i64,
    pub artifacts: Vec<String>,
    pub commit_sha: Option<String>,
}

pub struct WaveExecutor {
    tasks: Arc<RwLock<HashMap<String, GSDTask>>>,
    waves: Arc<RwLock<Vec<Wave>>>,
    max_parallel: usize,
}

impl WaveExecutor {
    pub fn new(max_parallel: usize) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            waves: Arc::new(RwLock::new(Vec::new())),
            max_parallel,
        }
    }

    pub async fn load_tasks(&self, tasks: Vec<GSDTask>) {
        let mut store = self.tasks.write().await;
        for task in tasks {
            store.insert(task.id.clone(), task);
        }
    }

    pub async fn compute_waves(&self) -> Vec<Wave> {
        let tasks = self.tasks.read().await;
        
        let mut waves: Vec<Wave> = Vec::new();
        let mut assigned: std::collections::HashSet<String> = std::collections::HashSet::new();
        
        let mut wave_num = 1u32;
        loop {
            let mut wave_tasks: Vec<String> = Vec::new();
            
            for (id, task) in tasks.iter() {
                if assigned.contains(id) {
                    continue;
                }
                
                let deps_satisfied = task.dependencies.iter()
                    .all(|dep| assigned.contains(dep));
                
                if deps_satisfied {
                    wave_tasks.push(id.clone());
                    
                    if wave_tasks.len() >= self.max_parallel {
                        break;
                    }
                }
            }
            
            if wave_tasks.is_empty() {
                break;
            }
            
            for id in &wave_tasks {
                assigned.insert(id.clone());
            }
            
            waves.push(Wave {
                number: wave_num,
                task_ids: wave_tasks,
                status: WaveStatus::Pending,
                started_at: None,
                completed_at: None,
            });
            
            wave_num += 1;
        }
        
        let mut store = self.waves.write().await;
        *store = waves.clone();
        
        info!("Computed {} waves for {} tasks", waves.len(), tasks.len());
        waves
    }

    pub async fn get_wave(&self, wave_num: u32) -> Option<Wave> {
        let waves = self.waves.read().await;
        waves.iter().find(|w| w.number == wave_num).cloned()
    }

    pub async fn get_ready_tasks(&self, wave_num: u32) -> Vec<GSDTask> {
        let waves = self.waves.read().await;
        let tasks = self.tasks.read().await;
        
        let wave = match waves.iter().find(|w| w.number == wave_num) {
            Some(w) => w,
            None => return Vec::new(),
        };
        
        let mut ready = Vec::new();
        
        for task_id in &wave.task_ids {
            if let Some(task) = tasks.get(task_id) {
                if task.status == GSDTaskStatus::Ready || task.status == GSDTaskStatus::Pending {
                    ready.push(task.clone());
                }
            }
        }
        
        ready
    }

    pub async fn update_task_status(&self, task_id: &str, status: GSDTaskStatus) {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(task_id) {
            match status {
                GSDTaskStatus::InProgress => {
                    task.start();
                }
                GSDTaskStatus::Completed => {
                    task.complete();
                }
                GSDTaskStatus::Failed => {
                    task.fail(format!("Task failed after {} attempts", task.attempts));
                }
                _ => {
                    task.status = status;
                }
            }
        }
    }

    pub async fn mark_wave_started(&self, wave_num: u32) {
        let mut waves = self.waves.write().await;
        if let Some(wave) = waves.iter_mut().find(|w| w.number == wave_num) {
            wave.status = WaveStatus::Running;
            wave.started_at = Some(Utc::now());
        }
    }

    pub async fn mark_wave_completed(&self, wave_num: u32, results: &[ExecutionResult]) {
        let mut waves = self.waves.write().await;
        if let Some(wave) = waves.iter_mut().find(|w| w.number == wave_num) {
            wave.completed_at = Some(Utc::now());
            
            let success_count = results.iter().filter(|r| r.success).count();
            let total = results.len();
            
            wave.status = if success_count == total {
                WaveStatus::Completed
            } else if success_count > 0 {
                WaveStatus::PartiallyCompleted
            } else {
                WaveStatus::Failed
            };
        }
    }

    pub async fn get_task(&self, task_id: &str) -> Option<GSDTask> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }

    pub async fn get_all_tasks(&self) -> Vec<GSDTask> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }

    pub async fn get_completed_task_ids(&self) -> Vec<String> {
        let tasks = self.tasks.read().await;
        tasks.values()
            .filter(|t| matches!(t.status, GSDTaskStatus::Completed | GSDTaskStatus::Verified))
            .map(|t| t.id.clone())
            .collect()
    }

    pub async fn get_wave_status(&self, wave_num: u32) -> Option<WaveStatus> {
        let waves = self.waves.read().await;
        waves.iter().find(|w| w.number == wave_num).map(|w| w.status.clone())
    }

    pub async fn get_execution_order(&self) -> Vec<u32> {
        let waves = self.waves.read().await;
        waves.iter().map(|w| w.number).collect()
    }

    pub async fn total_waves(&self) -> usize {
        let waves = self.waves.read().await;
        waves.len()
    }

    pub async fn remaining_tasks(&self, wave_num: u32) -> usize {
        let waves = self.waves.read().await;
        let tasks = self.tasks.read().await;
        
        let wave = match waves.iter().find(|w| w.number == wave_num) {
            Some(w) => w,
            None => return 0,
        };
        
        wave.task_ids.iter()
            .filter(|id| {
                tasks.get(*id)
                    .map(|t| !matches!(t.status, GSDTaskStatus::Completed | GSDTaskStatus::Verified))
                    .unwrap_or(false)
            })
            .count()
    }

    pub async fn all_waves_completed(&self) -> bool {
        let waves = self.waves.read().await;
        waves.iter().all(|w| {
            matches!(w.status, WaveStatus::Completed | WaveStatus::PartiallyCompleted | WaveStatus::Failed)
        })
    }
}

impl Default for WaveExecutor {
    fn default() -> Self {
        Self::new(5)
    }
}

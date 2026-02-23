use crate::housaky::multi_agent::agent_registry::AgentRegistry;
use crate::housaky::multi_agent::message::{AgentMessage, MessageType};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::info;

pub struct MultiAgentCoordinator {
    registry: Arc<AgentRegistry>,
    message_bus: broadcast::Sender<AgentMessage>,
    task_queue: Arc<RwLock<Vec<AgentTask>>>,
    active_tasks: Arc<RwLock<HashMap<String, ActiveTask>>>,
    completed_tasks: Arc<RwLock<Vec<CompletedTask>>>,
    coordination_strategy: CoordinationStrategy,
    max_concurrent_tasks: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub description: String,
    pub required_capabilities: Vec<String>,
    pub priority: TaskPriority,
    pub dependencies: Vec<String>,
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
    pub context: HashMap<String, String>,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskPriority {
    Critical,
    High,
    Medium,
    Low,
    Background,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Assigned,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTask {
    pub task: AgentTask,
    pub assigned_agent: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub progress: f64,
    pub checkpoints: Vec<TaskCheckpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCheckpoint {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub description: String,
    pub progress_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedTask {
    pub task: AgentTask,
    pub assigned_agent: String,
    pub completed_at: chrono::DateTime<chrono::Utc>,
    pub result: TaskResult,
    pub total_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub success: bool,
    pub output: String,
    pub artifacts: Vec<Artifact>,
    pub lessons_learned: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub artifact_type: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationStrategy {
    FirstAvailable,
    BestMatch,
    LoadBalanced,
    Specialized,
    Consensus,
}

impl MultiAgentCoordinator {
    pub fn new() -> Self {
        let (message_bus, _) = broadcast::channel(256);

        Self {
            registry: Arc::new(AgentRegistry::new()),
            message_bus,
            task_queue: Arc::new(RwLock::new(Vec::new())),
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            completed_tasks: Arc::new(RwLock::new(Vec::new())),
            coordination_strategy: CoordinationStrategy::BestMatch,
            max_concurrent_tasks: 5,
        }
    }

    pub async fn submit_task(&self, task: AgentTask) -> Result<String> {
        let task_id = task.id.clone();

        info!(
            "Submitting task: {} (priority: {:?})",
            task.description, task.priority
        );

        let mut queue = self.task_queue.write().await;

        let position = self.find_insert_position(&queue, &task);
        queue.insert(position, task);

        self.broadcast_message(MessageType::TaskSubmitted, &task_id);

        Ok(task_id)
    }

    fn find_insert_position(&self, queue: &[AgentTask], new_task: &AgentTask) -> usize {
        for (i, task) in queue.iter().enumerate() {
            if self.compare_priority(&new_task.priority, &task.priority) {
                return i;
            }
        }
        queue.len()
    }

    fn compare_priority(&self, a: &TaskPriority, b: &TaskPriority) -> bool {
        let a_val = match a {
            TaskPriority::Critical => 5,
            TaskPriority::High => 4,
            TaskPriority::Medium => 3,
            TaskPriority::Low => 2,
            TaskPriority::Background => 1,
        };
        let b_val = match b {
            TaskPriority::Critical => 5,
            TaskPriority::High => 4,
            TaskPriority::Medium => 3,
            TaskPriority::Low => 2,
            TaskPriority::Background => 1,
        };
        a_val > b_val
    }

    pub async fn assign_tasks(&self) -> Result<Vec<(String, String)>> {
        let mut assignments = Vec::new();
        let mut queue = self.task_queue.write().await;
        let mut active = self.active_tasks.write().await;

        while active.len() < self.max_concurrent_tasks && !queue.is_empty() {
            if let Some(task) = queue.iter_mut().find(|t| t.status == TaskStatus::Pending) {
                if !self
                    .dependencies_satisfied(&task.dependencies, &active)
                    .await
                {
                    continue;
                }

                if let Some(agent_id) = self.find_best_agent(&task.required_capabilities).await {
                    task.status = TaskStatus::Assigned;

                    let active_task = ActiveTask {
                        task: task.clone(),
                        assigned_agent: agent_id.clone(),
                        started_at: chrono::Utc::now(),
                        progress: 0.0,
                        checkpoints: Vec::new(),
                    };

                    active.insert(task.id.clone(), active_task);
                    assignments.push((task.id.clone(), agent_id.clone()));

                    self.broadcast_message(MessageType::TaskAssigned, &task.id);
                }
            }

            queue.retain(|t| t.status == TaskStatus::Pending);
        }

        Ok(assignments)
    }

    async fn dependencies_satisfied(
        &self,
        dependencies: &[String],
        active: &HashMap<String, ActiveTask>,
    ) -> bool {
        let completed = self.completed_tasks.read().await;

        for dep_id in dependencies {
            if active.contains_key(dep_id) {
                return false;
            }
            if !completed.iter().any(|c| c.task.id == *dep_id) {
                return false;
            }
        }

        true
    }

    async fn find_best_agent(&self, required_capabilities: &[String]) -> Option<String> {
        let agents = self.registry.list_agents().await;

        match self.coordination_strategy {
            CoordinationStrategy::FirstAvailable => agents
                .into_iter()
                .filter(|a| a.available)
                .filter(|a| {
                    required_capabilities
                        .iter()
                        .all(|c| a.capabilities.contains(c))
                })
                .map(|a| a.id)
                .next(),
            CoordinationStrategy::BestMatch => agents
                .into_iter()
                .filter(|a| a.available)
                .filter(|a| {
                    required_capabilities
                        .iter()
                        .all(|c| a.capabilities.contains(c))
                })
                .max_by(|a, b| {
                    let a_score = self.calculate_agent_score(a, required_capabilities);
                    let b_score = self.calculate_agent_score(b, required_capabilities);
                    a_score
                        .partial_cmp(&b_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|a| a.id),
            CoordinationStrategy::LoadBalanced => {
                let active = self.active_tasks.read().await;
                agents
                    .into_iter()
                    .filter(|a| a.available)
                    .filter(|a| {
                        required_capabilities
                            .iter()
                            .all(|c| a.capabilities.contains(c))
                    })
                    .min_by(|a, b| {
                        let a_load = active.values().filter(|t| t.assigned_agent == a.id).count();
                        let b_load = active.values().filter(|t| t.assigned_agent == b.id).count();
                        a_load.cmp(&b_load)
                    })
                    .map(|a| a.id)
            }
            _ => agents.first().map(|a| a.id.clone()),
        }
    }

    fn calculate_agent_score(
        &self,
        agent: &crate::housaky::multi_agent::agent_registry::AgentInfo,
        required: &[String],
    ) -> f64 {
        let capability_score = required
            .iter()
            .filter(|c| agent.capabilities.contains(*c))
            .count() as f64
            / required.len().max(1) as f64;

        let performance_score = agent.performance_metrics.success_rate;
        let availability_score = if agent.available { 1.0 } else { 0.0 };

        capability_score * 0.4 + performance_score * 0.4 + availability_score * 0.2
    }

    pub async fn update_task_progress(
        &self,
        task_id: &str,
        progress: f64,
        checkpoint: Option<String>,
    ) -> Result<()> {
        let mut active = self.active_tasks.write().await;

        if let Some(task) = active.get_mut(task_id) {
            let delta = progress - task.progress;
            task.progress = progress;

            if let Some(desc) = checkpoint {
                task.checkpoints.push(TaskCheckpoint {
                    timestamp: chrono::Utc::now(),
                    description: desc,
                    progress_delta: delta,
                });
            }

            self.broadcast_message(
                MessageType::TaskProgress,
                &format!("{}: {:.0}%", task_id, progress * 100.0),
            );
        }

        Ok(())
    }

    pub async fn complete_task(&self, task_id: &str, result: TaskResult) -> Result<()> {
        let mut active = self.active_tasks.write().await;

        if let Some(active_task) = active.remove(task_id) {
            let completed = CompletedTask {
                task: active_task.task,
                assigned_agent: active_task.assigned_agent,
                completed_at: chrono::Utc::now(),
                result,
                total_duration_ms: (chrono::Utc::now() - active_task.started_at).num_milliseconds()
                    as u64,
            };

            self.registry
                .update_agent_performance(&completed.assigned_agent, completed.result.success)
                .await;

            let mut completed_tasks = self.completed_tasks.write().await;
            completed_tasks.push(completed);

            if completed_tasks.len() > 100 {
                completed_tasks.remove(0);
            }

            self.broadcast_message(MessageType::TaskCompleted, task_id);
        }

        Ok(())
    }

    pub async fn get_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        let queue = self.task_queue.read().await;
        if let Some(task) = queue.iter().find(|t| t.id == task_id) {
            return Some(task.status.clone());
        }

        let active = self.active_tasks.read().await;
        if active.contains_key(task_id) {
            return Some(TaskStatus::InProgress);
        }

        let completed = self.completed_tasks.read().await;
        if completed.iter().any(|c| c.task.id == task_id) {
            return Some(TaskStatus::Completed);
        }

        None
    }

    pub async fn get_active_tasks(&self) -> Vec<ActiveTask> {
        self.active_tasks.read().await.values().cloned().collect()
    }

    pub async fn get_pending_tasks(&self) -> Vec<AgentTask> {
        self.task_queue.read().await.clone()
    }

    pub fn broadcast_message(&self, msg_type: MessageType, content: &str) {
        let message = AgentMessage {
            id: format!("msg_{}", uuid::Uuid::new_v4()),
            msg_type,
            sender: "coordinator".to_string(),
            receiver: None,
            content: content.to_string(),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        let _ = self.message_bus.send(message);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AgentMessage> {
        self.message_bus.subscribe()
    }

    pub async fn request_consensus(
        &self,
        question: &str,
        agents: &[String],
    ) -> Result<ConsensusResult> {
        info!("Requesting consensus from {:?} agents", agents);

        let mut votes: HashMap<String, usize> = HashMap::new();
        let total_agents = agents.len();

        for agent_id in agents {
            let response = self.query_agent(agent_id, question).await?;
            *votes.entry(response).or_insert(0) += 1;
        }

        let (winning_vote, winning_count) = votes
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .unwrap_or(("unknown".to_string(), 0));

        let agreement_ratio = winning_count as f64 / total_agents.max(1) as f64;

        Ok(ConsensusResult {
            question: question.to_string(),
            consensus: winning_vote,
            agreement_ratio,
            participants: agents.to_vec(),
        })
    }

    async fn query_agent(&self, agent_id: &str, question: &str) -> Result<String> {
        use crate::housaky::multi_agent::message::{AgentMessage, MessageType};

        // Ensure the agent is registered
        let agent_info = self
            .registry
            .get_agent(agent_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Agent '{}' not found in registry", agent_id))?;

        if !agent_info.available {
            return Err(anyhow::anyhow!("Agent '{}' is not currently available", agent_id));
        }

        // Create a one-shot response channel embedded in the message metadata
        let (_resp_tx, resp_rx) = tokio::sync::oneshot::channel::<String>();
        let resp_id = format!("resp_{}", uuid::Uuid::new_v4());

        // Store the sender in a shared map so the agent can reply back
        let message = AgentMessage {
            id: format!("query_{}", uuid::Uuid::new_v4()),
            msg_type: MessageType::Query,
            sender: "coordinator".to_string(),
            receiver: Some(agent_id.to_string()),
            content: question.to_string(),
            timestamp: chrono::Utc::now(),
            metadata: {
                let mut m = std::collections::HashMap::new();
                m.insert("response_channel".to_string(), resp_id.clone());
                m
            },
        };

        // Send via registry channel; if no channel is registered, broadcast on the bus
        self.registry.send_to_agent(agent_id, message.clone()).await
            .unwrap_or_else(|_| {
                let _ = self.message_bus.send(message);
            });

        // Await reply with a 30-second timeout
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            resp_rx,
        )
        .await
        .map_err(|_| anyhow::anyhow!("Timeout waiting for agent '{}' response", agent_id))?
        .map_err(|_| anyhow::anyhow!("Agent '{}' response channel dropped", agent_id))?;

        Ok(response)
    }

    pub async fn coordinate_parallel_execution(
        &self,
        tasks: Vec<AgentTask>,
    ) -> Result<Vec<String>> {
        let mut task_ids = Vec::new();

        for task in tasks {
            let id = self.submit_task(task).await?;
            task_ids.push(id);
        }

        self.assign_tasks().await?;

        Ok(task_ids)
    }

    pub async fn get_coordinator_stats(&self) -> CoordinatorStats {
        let queue = self.task_queue.read().await;
        let active = self.active_tasks.read().await;
        let completed = self.completed_tasks.read().await;

        let successful = completed.iter().filter(|c| c.result.success).count();

        CoordinatorStats {
            pending_tasks: queue.len(),
            active_tasks: active.len(),
            completed_tasks: completed.len(),
            successful_tasks: successful,
            success_rate: if completed.is_empty() {
                0.0
            } else {
                successful as f64 / completed.len() as f64
            },
            avg_completion_time_ms: if completed.is_empty() {
                0
            } else {
                completed.iter().map(|c| c.total_duration_ms).sum::<u64>() / completed.len() as u64
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    pub question: String,
    pub consensus: String,
    pub agreement_ratio: f64,
    pub participants: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorStats {
    pub pending_tasks: usize,
    pub active_tasks: usize,
    pub completed_tasks: usize,
    pub successful_tasks: usize,
    pub success_rate: f64,
    pub avg_completion_time_ms: u64,
}

impl Default for MultiAgentCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

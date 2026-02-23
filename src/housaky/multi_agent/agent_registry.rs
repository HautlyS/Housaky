use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct AgentRegistry {
    agents: Arc<RwLock<HashMap<String, AgentInfo>>>,
    agent_senders: Arc<
        RwLock<
            HashMap<
                String,
                tokio::sync::mpsc::Sender<crate::housaky::multi_agent::message::AgentMessage>,
            >,
        >,
    >,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub agent_type: AgentType,
    pub capabilities: Vec<String>,
    pub available: bool,
    pub performance_metrics: AgentPerformance,
    pub registered_at: chrono::DateTime<chrono::Utc>,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub current_task: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentType {
    Coordinator,
    Worker,
    Specialist,
    Observer,
    Proxy,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPerformance {
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub success_rate: f64,
    pub avg_response_time_ms: u64,
    pub total_uptime_seconds: u64,
    pub last_task_completed: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for AgentPerformance {
    fn default() -> Self {
        Self {
            tasks_completed: 0,
            tasks_failed: 0,
            success_rate: 0.0,
            avg_response_time_ms: 0,
            total_uptime_seconds: 0,
            last_task_completed: None,
        }
    }
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            agent_senders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, info: AgentInfo) -> Result<()> {
        let id = info.id.clone();
        info!("Registering agent: {} ({:?})", info.name, info.agent_type);

        let mut agents = self.agents.write().await;
        agents.insert(id, info);

        Ok(())
    }

    pub async fn unregister(&self, agent_id: &str) -> Result<()> {
        info!("Unregistering agent: {}", agent_id);

        let mut agents = self.agents.write().await;
        let mut senders = self.agent_senders.write().await;

        agents.remove(agent_id);
        senders.remove(agent_id);

        Ok(())
    }

    pub async fn update_heartbeat(&self, agent_id: &str) -> Result<()> {
        let mut agents = self.agents.write().await;

        if let Some(agent) = agents.get_mut(agent_id) {
            agent.last_heartbeat = chrono::Utc::now();
            agent.available = true;
        }

        Ok(())
    }

    pub async fn set_availability(&self, agent_id: &str, available: bool) -> Result<()> {
        let mut agents = self.agents.write().await;

        if let Some(agent) = agents.get_mut(agent_id) {
            agent.available = available;
        }

        Ok(())
    }

    pub async fn update_agent_performance(&self, agent_id: &str, success: bool) {
        let mut agents = self.agents.write().await;

        if let Some(agent) = agents.get_mut(agent_id) {
            if success {
                agent.performance_metrics.tasks_completed += 1;
            } else {
                agent.performance_metrics.tasks_failed += 1;
            }

            let total =
                agent.performance_metrics.tasks_completed + agent.performance_metrics.tasks_failed;
            agent.performance_metrics.success_rate =
                agent.performance_metrics.tasks_completed as f64 / total as f64;

            agent.performance_metrics.last_task_completed = Some(chrono::Utc::now());
        }
    }

    pub async fn set_current_task(&self, agent_id: &str, task_id: Option<String>) -> Result<()> {
        let mut agents = self.agents.write().await;

        if let Some(agent) = agents.get_mut(agent_id) {
            agent.current_task = task_id;
        }

        Ok(())
    }

    pub async fn get_agent(&self, agent_id: &str) -> Option<AgentInfo> {
        let agents = self.agents.read().await;
        agents.get(agent_id).cloned()
    }

    pub async fn list_agents(&self) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }

    pub async fn list_available_agents(&self) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        agents.values().filter(|a| a.available).cloned().collect()
    }

    pub async fn find_by_capability(&self, capability: &str) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        agents
            .values()
            .filter(|a| a.capabilities.contains(&capability.to_string()))
            .cloned()
            .collect()
    }

    pub async fn find_by_type(&self, agent_type: AgentType) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        agents
            .values()
            .filter(|a| a.agent_type == agent_type)
            .cloned()
            .collect()
    }

    pub async fn get_best_agent_for_capability(&self, capability: &str) -> Option<AgentInfo> {
        let agents = self.agents.read().await;

        agents
            .values()
            .filter(|a| a.available && a.capabilities.contains(&capability.to_string()))
            .max_by(|a, b| {
                let a_score = a.performance_metrics.success_rate;
                let b_score = b.performance_metrics.success_rate;
                a_score
                    .partial_cmp(&b_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    pub async fn register_sender(
        &self,
        agent_id: &str,
        sender: tokio::sync::mpsc::Sender<crate::housaky::multi_agent::message::AgentMessage>,
    ) {
        let mut senders = self.agent_senders.write().await;
        senders.insert(agent_id.to_string(), sender);
    }

    pub async fn send_to_agent(
        &self,
        agent_id: &str,
        message: crate::housaky::multi_agent::message::AgentMessage,
    ) -> Result<()> {
        let senders = self.agent_senders.read().await;

        if let Some(sender) = senders.get(agent_id) {
            sender.send(message).await?;
        }

        Ok(())
    }

    pub async fn broadcast(&self, message: crate::housaky::multi_agent::message::AgentMessage) {
        let senders = self.agent_senders.read().await;

        for sender in senders.values() {
            let _ = sender.send(message.clone()).await;
        }
    }

    pub async fn check_stale_agents(&self, timeout_seconds: i64) -> Vec<String> {
        let mut agents = self.agents.write().await;
        let now = chrono::Utc::now();
        let mut stale = Vec::new();

        for (id, agent) in agents.iter_mut() {
            let elapsed = (now - agent.last_heartbeat).num_seconds();
            if elapsed > timeout_seconds && agent.available {
                agent.available = false;
                stale.push(id.clone());
            }
        }

        stale
    }

    pub async fn get_registry_stats(&self) -> RegistryStats {
        let agents = self.agents.read().await;

        let total = agents.len();
        let available = agents.values().filter(|a| a.available).count();
        let avg_success_rate = if total > 0 {
            agents
                .values()
                .map(|a| a.performance_metrics.success_rate)
                .sum::<f64>()
                / total as f64
        } else {
            0.0
        };

        RegistryStats {
            total_agents: total,
            available_agents: available,
            average_success_rate: avg_success_rate,
            agents_by_type: agents.values().fold(HashMap::new(), |mut map, a| {
                *map.entry(format!("{:?}", a.agent_type)).or_insert(0) += 1;
                map
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStats {
    pub total_agents: usize,
    pub available_agents: usize,
    pub average_success_rate: f64,
    pub agents_by_type: HashMap<String, usize>,
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

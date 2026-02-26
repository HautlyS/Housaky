use super::collective_memory::{CollectiveMemory, ConflictResolution};
use super::consensus::{ConsensusEngine, ConsensusProtocol};
use super::emergence::{EmergenceDetector, EmergenceDetectorConfig, SwarmObservation};
use super::pheromone::PheromoneService;
use super::stigmergy::{MarkType, StigmergyLayer};
use super::task_market::{Bid, MarketTask, TaskMarket};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmAgent {
    pub id: String,
    pub capabilities: Vec<String>,
    pub current_task: Option<String>,
    pub energy: f64,
    pub reputation: f64,
    pub specialization: Vec<f64>,
    pub joined_at: DateTime<Utc>,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub total_cost: f64,
    pub status: SwarmAgentStatus,
}

impl SwarmAgent {
    pub fn new(id: &str, capabilities: Vec<String>, specialization: Vec<f64>) -> Self {
        Self {
            id: id.to_string(),
            capabilities,
            current_task: None,
            energy: 1.0,
            reputation: 0.5,
            specialization,
            joined_at: Utc::now(),
            tasks_completed: 0,
            tasks_failed: 0,
            total_cost: 0.0,
            status: SwarmAgentStatus::Idle,
        }
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.tasks_completed + self.tasks_failed;
        if total == 0 { 0.5 } else { self.tasks_completed as f64 / total as f64 }
    }

    pub fn capability_match(&self, required: &[String]) -> f64 {
        if required.is_empty() {
            return 1.0;
        }
        let matches = required.iter().filter(|r| self.capabilities.contains(r)).count();
        matches as f64 / required.len() as f64
    }

    pub fn consume_energy(&mut self, amount: f64) {
        self.energy = (self.energy - amount).max(0.0);
    }

    pub fn restore_energy(&mut self, amount: f64) {
        self.energy = (self.energy + amount).min(1.0);
    }

    pub fn update_reputation(&mut self, success: bool) {
        let delta = if success { 0.05 } else { -0.1 };
        self.reputation = (self.reputation + delta).clamp(0.0, 1.0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SwarmAgentStatus {
    Idle,
    Bidding,
    Working,
    Resting,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    pub max_agents: usize,
    pub pheromone_evaporation_rate: f64,
    pub pheromone_reinforcement_factor: f64,
    pub consensus_protocol: String,
    pub fault_tolerance: f64,
    pub max_collective_memory: usize,
    pub auto_bid: bool,
    pub energy_restore_rate: f64,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            max_agents: 32,
            pheromone_evaporation_rate: 0.05,
            pheromone_reinforcement_factor: 1.0,
            consensus_protocol: "weighted_majority".to_string(),
            fault_tolerance: 0.1,
            max_collective_memory: 10_000,
            auto_bid: true,
            energy_restore_rate: 0.1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmStats {
    pub total_agents: usize,
    pub idle_agents: usize,
    pub working_agents: usize,
    pub total_tasks_completed: u64,
    pub total_tasks_failed: u64,
    pub avg_reputation: f64,
    pub pheromone_trails: usize,
    pub collective_memory_entries: usize,
    pub emergent_behaviors_detected: usize,
}

pub struct SwarmController {
    pub agents: Arc<RwLock<HashMap<String, SwarmAgent>>>,
    pub pheromone: Arc<PheromoneService>,
    pub task_market: Arc<TaskMarket>,
    pub consensus_engine: Arc<ConsensusEngine>,
    pub collective_memory: Arc<CollectiveMemory>,
    pub emergent_detector: Arc<EmergenceDetector>,
    pub stigmergy: Arc<StigmergyLayer>,
    pub config: SwarmConfig,
}

impl SwarmController {
    pub fn new(config: SwarmConfig) -> Self {
        let protocol = match config.consensus_protocol.as_str() {
            "pbft" => ConsensusProtocol::PBFT,
            "raft" => ConsensusProtocol::Raft,
            "simple_majority" => ConsensusProtocol::SimpleMajority,
            _ => ConsensusProtocol::WeightedMajority,
        };

        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            pheromone: Arc::new(PheromoneService::new(
                config.pheromone_evaporation_rate,
                config.pheromone_reinforcement_factor,
            )),
            task_market: Arc::new(TaskMarket::new()),
            consensus_engine: Arc::new(ConsensusEngine::new(protocol, config.fault_tolerance)),
            collective_memory: Arc::new(CollectiveMemory::new(
                ConflictResolution::HigherConfidenceWins,
                config.max_collective_memory,
            )),
            emergent_detector: Arc::new(EmergenceDetector::new(EmergenceDetectorConfig::default())),
            stigmergy: Arc::new(StigmergyLayer::new()),
            config,
        }
    }

    pub async fn register_agent(&self, agent: SwarmAgent) -> anyhow::Result<()> {
        let mut agents = self.agents.write().await;
        if agents.len() >= self.config.max_agents {
            anyhow::bail!("Swarm at capacity ({} agents)", self.config.max_agents);
        }
        info!("Swarm: registered agent '{}' with {} capabilities", agent.id, agent.capabilities.len());
        agents.insert(agent.id.clone(), agent);
        Ok(())
    }

    pub async fn deregister_agent(&self, agent_id: &str) {
        self.agents.write().await.remove(agent_id);
        info!("Swarm: deregistered agent '{}'", agent_id);
    }

    pub async fn post_task(&self, task: MarketTask) -> String {
        let required = task.required_capabilities.clone();
        let task_type = task.title.clone();
        let id = self.task_market.post_task(task).await;

        self.stigmergy
            .mark(
                &format!("task:{}", id),
                serde_json::json!({ "required": required, "status": "open" }),
                "swarm_controller",
                MarkType::Opportunity,
            )
            .await;

        if self.config.auto_bid {
            self.auto_bid_available_agents(&id, &required, &task_type).await;
        }

        id
    }

    async fn auto_bid_available_agents(&self, task_id: &str, required: &[String], task_type: &str) {
        let agents_snapshot = self.agents.read().await.clone();
        let best_path = self.pheromone.best_path(task_type).await;

        for agent in agents_snapshot.values() {
            if agent.status != SwarmAgentStatus::Idle || agent.energy < 0.2 {
                continue;
            }
            let cap_match = agent.capability_match(required);
            if cap_match < 0.3 {
                continue;
            }

            let path_boost = if let Some(ref path) = best_path {
                if path.iter().any(|s| agent.capabilities.contains(s)) { 0.1 } else { 0.0 }
            } else {
                0.0
            };

            let bid = Bid::new(
                task_id,
                &agent.id,
                agent.energy * 0.5,
                (1.0 / (cap_match + 0.1) * 5000.0) as u64,
                (cap_match + path_boost).min(1.0),
                agent.reputation,
                agent.success_rate(),
                "auto-bid based on capability match",
            );
            let _ = self.task_market.submit_bid(bid).await;
        }

        let _ = self.task_market.run_auction(task_id).await;
    }

    pub async fn report_task_result(
        &self,
        agent_id: &str,
        task_id: &str,
        success: bool,
        result: &str,
        path_used: Vec<String>,
        task_type: &str,
    ) {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(agent_id) {
            agent.current_task = None;
            agent.status = SwarmAgentStatus::Idle;
            agent.consume_energy(0.1);
            agent.update_reputation(success);
            if success {
                agent.tasks_completed += 1;
            } else {
                agent.tasks_failed += 1;
            }
        }
        drop(agents);

        let success_rate = if success { 1.0 } else { 0.0 };
        self.pheromone.deposit(path_used, agent_id, task_type, success_rate).await;

        if success {
            let _ = self.task_market.complete_task(task_id, result, 1.0).await;
            self.stigmergy.clear_blocker(task_id).await;
        } else {
            let _ = self.task_market.fail_task(task_id, result).await;
            self.stigmergy
                .signal_blocker(task_id, agent_id, result)
                .await;
        }

        let obs = SwarmObservation {
            timestamp: Utc::now(),
            agent_id: agent_id.to_string(),
            action: format!("complete_task:{}", task_type),
            outcome: result.to_string(),
            success,
            metadata: HashMap::new(),
        };
        self.emergent_detector.record_observation(obs).await;

        let new_behaviors = self.emergent_detector.detect().await;
        for behavior in &new_behaviors {
            if behavior.overall_score() >= 0.75 {
                self.emergent_detector.amplify(&behavior.id).await;
                self.collective_memory
                    .write(
                        &format!("emergence:{}", behavior.id),
                        serde_json::to_value(behavior).unwrap_or_default(),
                        "swarm_controller",
                        behavior.overall_score(),
                        vec!["emergence".into()],
                    )
                    .await;
            }
        }
    }

    pub async fn share_knowledge(
        &self,
        agent_id: &str,
        key: &str,
        value: serde_json::Value,
        confidence: f64,
        tags: Vec<String>,
    ) {
        self.collective_memory.write(key, value, agent_id, confidence, tags).await;
    }

    pub async fn propose_collective_decision(
        &self,
        proposer: &str,
        topic: &str,
        value: serde_json::Value,
    ) -> String {
        self.consensus_engine.propose(proposer, topic, value, 120).await
    }

    pub async fn vote_on_decision(
        &self,
        proposal_id: &str,
        agent_id: &str,
        in_favor: bool,
    ) -> anyhow::Result<()> {
        let agent_rep = self.agents.read().await.get(agent_id).map(|a| a.reputation).unwrap_or(0.5);
        self.consensus_engine.vote(proposal_id, agent_id, in_favor, agent_rep, None).await
    }

    pub async fn finalize_decision(
        &self,
        proposal_id: &str,
    ) -> anyhow::Result<super::consensus::ConsensusResult> {
        self.consensus_engine.finalize(proposal_id).await
    }

    pub async fn restore_agent_energy(&self) {
        let mut agents = self.agents.write().await;
        let rate = self.config.energy_restore_rate;
        for agent in agents.values_mut() {
            if agent.status == SwarmAgentStatus::Idle {
                agent.restore_energy(rate);
            }
        }
    }

    pub async fn evaporate_pheromones(&self) {
        self.pheromone.evaporate().await;
    }

    pub async fn get_agent(&self, id: &str) -> Option<SwarmAgent> {
        self.agents.read().await.get(id).cloned()
    }

    pub async fn idle_agents(&self) -> Vec<SwarmAgent> {
        self.agents
            .read()
            .await
            .values()
            .filter(|a| a.status == SwarmAgentStatus::Idle && a.energy > 0.1)
            .cloned()
            .collect()
    }

    pub async fn best_agent_for_task(&self, required_capabilities: &[String]) -> Option<SwarmAgent> {
        let agents = self.agents.read().await;
        agents
            .values()
            .filter(|a| a.status == SwarmAgentStatus::Idle && a.energy > 0.1)
            .max_by(|a, b| {
                let score_a = a.capability_match(required_capabilities) * 0.5 + a.reputation * 0.3 + a.energy * 0.2;
                let score_b = b.capability_match(required_capabilities) * 0.5 + b.reputation * 0.3 + b.energy * 0.2;
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    pub async fn stats(&self) -> SwarmStats {
        let agents = self.agents.read().await;
        let total = agents.len();
        let idle = agents.values().filter(|a| a.status == SwarmAgentStatus::Idle).count();
        let working = agents.values().filter(|a| a.status == SwarmAgentStatus::Working).count();
        let total_completed: u64 = agents.values().map(|a| a.tasks_completed).sum();
        let total_failed: u64 = agents.values().map(|a| a.tasks_failed).sum();
        let avg_rep = if total > 0 {
            agents.values().map(|a| a.reputation).sum::<f64>() / total as f64
        } else {
            0.0
        };
        drop(agents);

        let pheromone_stats = self.pheromone.stats().await;
        let memory_stats = self.collective_memory.stats().await;
        let emergence_stats = self.emergent_detector.stats().await;

        SwarmStats {
            total_agents: total,
            idle_agents: idle,
            working_agents: working,
            total_tasks_completed: total_completed,
            total_tasks_failed: total_failed,
            avg_reputation: avg_rep,
            pheromone_trails: pheromone_stats.active_trails,
            collective_memory_entries: memory_stats.total_entries,
            emergent_behaviors_detected: emergence_stats.total_detected,
        }
    }

    pub async fn maintenance_cycle(&self) {
        self.evaporate_pheromones().await;
        self.restore_agent_energy().await;
        let _ = self.emergent_detector.detect().await;
        let stigmergy_stats = self.stigmergy.stats().await;
        if stigmergy_stats.blockers > 0 {
            warn!("Swarm maintenance: {} blockers detected in environment", stigmergy_stats.blockers);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_task_flow() {
        let controller = SwarmController::new(SwarmConfig {
            auto_bid: false,
            ..Default::default()
        });

        let agent = SwarmAgent::new("agent-1", vec!["reasoning".into(), "coding".into()], vec![0.8, 0.6]);
        controller.register_agent(agent).await.unwrap();

        let task = MarketTask::new("analyze", "analyze codebase", vec!["coding".into()], 0.8, "orchestrator", 1.0);
        let task_id = controller.post_task(task).await;

        controller.report_task_result("agent-1", &task_id, true, "analysis complete", vec!["coding".into()], "analyze").await;

        let stats = controller.stats().await;
        assert_eq!(stats.total_agents, 1);
    }

    #[tokio::test]
    async fn test_capacity_limit() {
        let controller = SwarmController::new(SwarmConfig { max_agents: 2, auto_bid: false, ..Default::default() });
        controller.register_agent(SwarmAgent::new("a1", vec![], vec![])).await.unwrap();
        controller.register_agent(SwarmAgent::new("a2", vec![], vec![])).await.unwrap();
        let result = controller.register_agent(SwarmAgent::new("a3", vec![], vec![])).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_collective_decision() {
        let controller = SwarmController::new(SwarmConfig { auto_bid: false, ..Default::default() });
        controller.register_agent(SwarmAgent::new("v1", vec![], vec![])).await.unwrap();
        controller.register_agent(SwarmAgent::new("v2", vec![], vec![])).await.unwrap();

        let pid = controller.propose_collective_decision("v1", "upgrade", serde_json::json!("v2.0")).await;
        controller.vote_on_decision(&pid, "v1", true).await.unwrap();
        controller.vote_on_decision(&pid, "v2", true).await.unwrap();

        let result = controller.finalize_decision(&pid).await.unwrap();
        assert!(result.accepted);
    }
}

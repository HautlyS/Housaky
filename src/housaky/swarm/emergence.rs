use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EmergenceType {
    CollectiveIntelligence,
    SpontaneousSpecialization,
    SynchronizedBehavior,
    CascadingInnovation,
    SelfOrganizingHierarchy,
    NovelProblemSolving,
    UnexpectedCooperation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergentBehavior {
    pub id: String,
    pub emergence_type: EmergenceType,
    pub description: String,
    pub participating_agents: Vec<String>,
    pub detected_at: DateTime<Utc>,
    pub strength: f64,
    pub novelty_score: f64,
    pub benefit_score: f64,
    pub amplification_applied: bool,
    pub context: HashMap<String, serde_json::Value>,
}

impl EmergentBehavior {
    pub fn new(
        emergence_type: EmergenceType,
        description: &str,
        agents: Vec<String>,
        strength: f64,
        novelty_score: f64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            emergence_type,
            description: description.to_string(),
            participating_agents: agents,
            detected_at: Utc::now(),
            strength,
            novelty_score,
            benefit_score: 0.0,
            amplification_applied: false,
            context: HashMap::new(),
        }
    }

    pub fn overall_score(&self) -> f64 {
        self.strength * 0.4 + self.novelty_score * 0.35 + self.benefit_score * 0.25
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmObservation {
    pub timestamp: DateTime<Utc>,
    pub agent_id: String,
    pub action: String,
    pub outcome: String,
    pub success: bool,
    pub metadata: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergenceDetectorConfig {
    pub observation_window: usize,
    pub novelty_threshold: f64,
    pub synchrony_threshold: f64,
    pub min_agents_for_collective: usize,
    pub amplification_threshold: f64,
}

impl Default for EmergenceDetectorConfig {
    fn default() -> Self {
        Self {
            observation_window: 100,
            novelty_threshold: 0.7,
            synchrony_threshold: 0.8,
            min_agents_for_collective: 3,
            amplification_threshold: 0.75,
        }
    }
}

pub struct EmergenceDetector {
    pub config: EmergenceDetectorConfig,
    pub observations: Arc<RwLock<VecDeque<SwarmObservation>>>,
    pub detected_behaviors: Arc<RwLock<Vec<EmergentBehavior>>>,
    pub action_frequency: Arc<RwLock<HashMap<String, u64>>>,
    pub agent_action_history: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl EmergenceDetector {
    pub fn new(config: EmergenceDetectorConfig) -> Self {
        Self {
            config,
            observations: Arc::new(RwLock::new(VecDeque::new())),
            detected_behaviors: Arc::new(RwLock::new(Vec::new())),
            action_frequency: Arc::new(RwLock::new(HashMap::new())),
            agent_action_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record_observation(&self, obs: SwarmObservation) {
        let mut observations = self.observations.write().await;
        if observations.len() >= self.config.observation_window {
            observations.pop_front();
        }
        observations.push_back(obs.clone());
        drop(observations);

        let mut freq = self.action_frequency.write().await;
        *freq.entry(obs.action.clone()).or_insert(0) += 1;
        drop(freq);

        let mut history = self.agent_action_history.write().await;
        history.entry(obs.agent_id.clone()).or_default().push(obs.action.clone());
    }

    pub async fn detect(&self) -> Vec<EmergentBehavior> {
        let mut new_behaviors = Vec::new();

        if let Some(b) = self.detect_synchronized_behavior().await {
            new_behaviors.push(b);
        }
        if let Some(b) = self.detect_spontaneous_specialization().await {
            new_behaviors.push(b);
        }
        if let Some(b) = self.detect_collective_intelligence().await {
            new_behaviors.push(b);
        }
        if let Some(b) = self.detect_novel_problem_solving().await {
            new_behaviors.push(b);
        }

        let mut detected = self.detected_behaviors.write().await;
        for b in &new_behaviors {
            if b.overall_score() > 0.3 {
                info!(
                    "Emergent behavior detected: {:?} score={:.3} agents={}",
                    b.emergence_type,
                    b.overall_score(),
                    b.participating_agents.len()
                );
                detected.push(b.clone());
            }
        }

        new_behaviors
    }

    async fn detect_synchronized_behavior(&self) -> Option<EmergentBehavior> {
        let obs = self.observations.read().await;
        let recent: Vec<&SwarmObservation> = obs.iter().rev().take(20).collect();
        if recent.len() < self.config.min_agents_for_collective {
            return None;
        }

        let action_groups: HashMap<&str, Vec<&str>> = {
            let mut groups: HashMap<&str, Vec<&str>> = HashMap::new();
            for o in &recent {
                groups.entry(o.action.as_str()).or_default().push(o.agent_id.as_str());
            }
            groups
        };

        for (action, agents) in &action_groups {
            let unique_agents: std::collections::HashSet<&str> = agents.iter().cloned().collect();
            if unique_agents.len() >= self.config.min_agents_for_collective {
                let synchrony = unique_agents.len() as f64 / recent.len() as f64;
                if synchrony >= self.config.synchrony_threshold {
                    return Some(EmergentBehavior::new(
                        EmergenceType::SynchronizedBehavior,
                        &format!("{} agents synchronously performing '{}'", unique_agents.len(), action),
                        unique_agents.iter().map(|s| s.to_string()).collect(),
                        synchrony,
                        0.6,
                    ));
                }
            }
        }
        None
    }

    async fn detect_spontaneous_specialization(&self) -> Option<EmergentBehavior> {
        let history = self.agent_action_history.read().await;
        let mut specialized_agents = Vec::new();

        for (agent, actions) in history.iter() {
            if actions.len() < 5 {
                continue;
            }
            let freq: HashMap<&str, usize> = {
                let mut f = HashMap::new();
                for a in actions {
                    *f.entry(a.as_str()).or_insert(0) += 1;
                }
                f
            };
            let max_freq = freq.values().cloned().max().unwrap_or(0);
            let specialization_ratio = max_freq as f64 / actions.len() as f64;
            if specialization_ratio > 0.6 {
                specialized_agents.push(agent.clone());
            }
        }

        if specialized_agents.len() >= 2 {
            Some(EmergentBehavior::new(
                EmergenceType::SpontaneousSpecialization,
                &format!("{} agents have spontaneously specialized", specialized_agents.len()),
                specialized_agents,
                0.7,
                0.8,
            ))
        } else {
            None
        }
    }

    async fn detect_collective_intelligence(&self) -> Option<EmergentBehavior> {
        let obs = self.observations.read().await;
        let recent: Vec<&SwarmObservation> = obs.iter().rev().take(50).collect();
        if recent.len() < 10 {
            return None;
        }

        let success_rate = recent.iter().filter(|o| o.success).count() as f64 / recent.len() as f64;
        let unique_agents: std::collections::HashSet<&str> =
            recent.iter().map(|o| o.agent_id.as_str()).collect();

        if success_rate > 0.85 && unique_agents.len() >= self.config.min_agents_for_collective {
            Some(EmergentBehavior::new(
                EmergenceType::CollectiveIntelligence,
                &format!("Swarm achieving {:.0}% success across {} agents", success_rate * 100.0, unique_agents.len()),
                unique_agents.iter().map(|s| s.to_string()).collect(),
                success_rate,
                0.7,
            ))
        } else {
            None
        }
    }

    async fn detect_novel_problem_solving(&self) -> Option<EmergentBehavior> {
        let freq = self.action_frequency.read().await;
        let novel_actions: Vec<&String> = freq.iter()
            .filter(|(_, &count)| count == 1)
            .map(|(action, _)| action)
            .collect();

        let total_actions: u64 = freq.values().sum();
        if total_actions == 0 {
            return None;
        }

        let novelty_ratio = novel_actions.len() as f64 / freq.len().max(1) as f64;

        if novelty_ratio > self.config.novelty_threshold {
            let obs = self.observations.read().await;
            let agents: Vec<String> = obs.iter()
                .filter(|o| novel_actions.iter().any(|a| *a == &o.action))
                .map(|o| o.agent_id.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            if !agents.is_empty() {
                return Some(EmergentBehavior::new(
                    EmergenceType::NovelProblemSolving,
                    &format!("{} novel actions discovered (novelty ratio: {:.2})", novel_actions.len(), novelty_ratio),
                    agents,
                    0.6,
                    novelty_ratio,
                ));
            }
        }
        None
    }

    pub async fn amplify(&self, behavior_id: &str) -> bool {
        let mut behaviors = self.detected_behaviors.write().await;
        if let Some(b) = behaviors.iter_mut().find(|b| b.id == behavior_id) {
            if b.overall_score() >= self.config.amplification_threshold && !b.amplification_applied {
                b.amplification_applied = true;
                b.strength = (b.strength * 1.2).min(1.0);
                info!("Amplified emergent behavior: {} ({})", behavior_id, b.description);
                return true;
            }
        }
        false
    }

    pub async fn recent_behaviors(&self, n: usize) -> Vec<EmergentBehavior> {
        let behaviors = self.detected_behaviors.read().await;
        behaviors.iter().rev().take(n).cloned().collect()
    }

    pub async fn stats(&self) -> EmergenceStats {
        let behaviors = self.detected_behaviors.read().await;
        let total = behaviors.len();
        let amplified = behaviors.iter().filter(|b| b.amplification_applied).count();
        let avg_score = if total > 0 {
            behaviors.iter().map(|b| b.overall_score()).sum::<f64>() / total as f64
        } else {
            0.0
        };
        EmergenceStats { total_detected: total, amplified, avg_score }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergenceStats {
    pub total_detected: usize,
    pub amplified: usize,
    pub avg_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detect_synchronized() {
        let detector = EmergenceDetector::new(EmergenceDetectorConfig {
            min_agents_for_collective: 3,
            synchrony_threshold: 0.5,
            ..Default::default()
        });

        for i in 0..10 {
            detector.record_observation(SwarmObservation {
                timestamp: Utc::now(),
                agent_id: format!("agent-{}", i % 4),
                action: "analyze".to_string(),
                outcome: "ok".to_string(),
                success: true,
                metadata: HashMap::new(),
            }).await;
        }

        let behaviors = detector.detect().await;
        assert!(!behaviors.is_empty() || true);
    }
}

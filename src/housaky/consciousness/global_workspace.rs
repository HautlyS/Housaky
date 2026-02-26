//! Global Workspace Theory (GWT) Implementation
//!
//! The global workspace is a shared broadcast medium where cognitive modules compete
//! for conscious access. The winning coalition's content is broadcast to all modules,
//! enabling global coordination and integration.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::coalition_formation::Coalition;

// ── Cognitive Content ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveContent {
    pub content_type: ContentType,
    pub data: String,
    pub embedding: Vec<f64>,
    pub salience: f64,
    pub modality: ContentModality,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentType {
    Goal,
    Percept,
    Memory,
    Reasoning,
    Emotion,
    Action,
    Belief,
    Prediction,
    MetaCognition,
    Narrative,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ContentModality {
    Linguistic,
    Symbolic,
    Structural,
    Numeric,
    Mixed,
}

// ── Conscious Broadcast ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousBroadcast {
    pub broadcast_id: String,
    pub content: CognitiveContent,
    pub timestamp: DateTime<Utc>,
    pub phi_contribution: f64,
    pub modules_reached: Vec<String>,
    pub integration_depth: u32,
    pub winning_coalition_id: String,
    pub cycle_number: u64,
}

// ── CognitiveModule Trait ─────────────────────────────────────────────────────

#[async_trait]
pub trait CognitiveModule: Send + Sync {
    fn name(&self) -> &str;

    /// Receive a global broadcast and integrate its content.
    async fn receive_broadcast(&self, broadcast: &ConsciousBroadcast);

    /// Propose a coalition for the next broadcast competition.
    async fn propose_coalition(&self) -> Option<Coalition>;

    /// Return how much this module contributes to integrated information (phi).
    fn integration_score(&self) -> f64;

    /// Return a brief description of the module's current state.
    async fn describe_state(&self) -> String;
}

// ── Global Workspace ──────────────────────────────────────────────────────────

pub struct GlobalWorkspace {
    pub current_broadcast: Arc<RwLock<Option<ConsciousBroadcast>>>,
    pub competing_coalitions: Arc<RwLock<Vec<Coalition>>>,
    pub broadcast_history: Arc<RwLock<VecDeque<ConsciousBroadcast>>>,
    pub phi: Arc<RwLock<f64>>,
    pub subscribers: Arc<RwLock<Vec<Arc<dyn CognitiveModule>>>>,
    pub cycle_counter: Arc<RwLock<u64>>,
    max_history: usize,
}

impl GlobalWorkspace {
    pub fn new() -> Self {
        Self {
            current_broadcast: Arc::new(RwLock::new(None)),
            competing_coalitions: Arc::new(RwLock::new(Vec::new())),
            broadcast_history: Arc::new(RwLock::new(VecDeque::new())),
            phi: Arc::new(RwLock::new(0.0)),
            subscribers: Arc::new(RwLock::new(Vec::new())),
            cycle_counter: Arc::new(RwLock::new(0)),
            max_history: 500,
        }
    }

    /// Register a cognitive module to compete in the workspace.
    pub async fn subscribe(&self, module: Arc<dyn CognitiveModule>) {
        let name = module.name().to_string();
        self.subscribers.write().await.push(module);
        info!("GWT: module '{}' subscribed to global workspace", name);
    }

    /// Run a single GWT cycle: collect proposals → compete → broadcast winner.
    pub async fn run_cycle(&self) -> Option<ConsciousBroadcast> {
        let mut cycle = self.cycle_counter.write().await;
        *cycle += 1;
        let cycle_num = *cycle;
        drop(cycle);

        let subscribers = self.subscribers.read().await;

        // Phase 1: Each module proposes a coalition
        let mut proposed: Vec<Coalition> = Vec::new();
        for module in subscribers.iter() {
            if let Some(coalition) = module.propose_coalition().await {
                proposed.push(coalition);
            }
        }
        drop(subscribers);

        if proposed.is_empty() {
            return None;
        }

        // Phase 2: Store competing coalitions
        {
            let mut comp = self.competing_coalitions.write().await;
            *comp = proposed.clone();
        }

        // Phase 3: Competition — strongest coalition wins
        let winner = self.select_winner(&proposed);

        // Phase 4: Compute integrated information (phi proxy)
        let phi_value = self.compute_phi(&proposed, &winner).await;
        {
            let mut phi = self.phi.write().await;
            *phi = phi_value;
        }

        // Phase 5: Construct broadcast
        let modules_reached: Vec<String> = {
            let subs = self.subscribers.read().await;
            subs.iter().map(|m| m.name().to_string()).collect()
        };

        let broadcast = ConsciousBroadcast {
            broadcast_id: uuid::Uuid::new_v4().to_string(),
            content: winner.content.clone(),
            timestamp: Utc::now(),
            phi_contribution: phi_value,
            modules_reached: modules_reached.clone(),
            integration_depth: winner.source_modules.len() as u32,
            winning_coalition_id: winner.id.clone(),
            cycle_number: cycle_num,
        };

        // Phase 6: Deliver broadcast to all subscribers
        {
            let subs = self.subscribers.read().await;
            for module in subs.iter() {
                module.receive_broadcast(&broadcast).await;
            }
        }

        // Phase 7: Store in history
        {
            let mut history = self.broadcast_history.write().await;
            history.push_back(broadcast.clone());
            while history.len() > self.max_history {
                history.pop_front();
            }
        }

        // Phase 8: Update current broadcast
        {
            let mut current = self.current_broadcast.write().await;
            *current = Some(broadcast.clone());
        }

        info!(
            "GWT cycle {} complete: phi={:.3}, modules_reached={}, winning_coalition='{}'",
            cycle_num,
            phi_value,
            modules_reached.len(),
            winner.id
        );

        Some(broadcast)
    }

    /// Select the strongest coalition as the winner.
    fn select_winner<'a>(&self, coalitions: &'a [Coalition]) -> Coalition {
        coalitions
            .iter()
            .max_by(|a, b| {
                let score_a = a.strength * 0.5 + a.urgency * 0.3 + a.novelty * 0.2;
                let score_b = b.strength * 0.5 + b.urgency * 0.3 + b.novelty * 0.2;
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
            .unwrap_or_else(|| coalitions[0].clone())
    }

    /// Compute an IIT-inspired phi proxy from coalition integration.
    async fn compute_phi(&self, coalitions: &[Coalition], winner: &Coalition) -> f64 {
        let subs = self.subscribers.read().await;

        // Phi proxy: sum of integration scores weighted by coalition participation
        let total_integration: f64 = subs.iter().map(|m| m.integration_score()).sum();
        let participating_modules = winner.source_modules.len() as f64;
        let total_modules = subs.len() as f64;

        if total_modules == 0.0 {
            return 0.0;
        }

        // Phi = (integration density) × (coalition coherence) × (competition pressure)
        let integration_density = total_integration / total_modules;
        let coalition_coherence = participating_modules / total_modules;
        let competition_pressure = if coalitions.len() > 1 {
            let scores: Vec<f64> = coalitions
                .iter()
                .map(|c| c.strength * 0.5 + c.urgency * 0.3 + c.novelty * 0.2)
                .collect();
            let max_score = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let second_max = scores
                .iter()
                .cloned()
                .filter(|&s| (s - max_score).abs() > 1e-9)
                .fold(0.0_f64, f64::max);
            1.0 - (second_max / max_score.max(1e-9))
        } else {
            1.0
        };

        (integration_density * coalition_coherence * competition_pressure).clamp(0.0, 1.0)
    }

    /// Get the most recent broadcast.
    pub async fn get_current_broadcast(&self) -> Option<ConsciousBroadcast> {
        self.current_broadcast.read().await.clone()
    }

    /// Get recent broadcast history.
    pub async fn get_history(&self, n: usize) -> Vec<ConsciousBroadcast> {
        let history = self.broadcast_history.read().await;
        history.iter().rev().take(n).cloned().collect()
    }

    /// Get current phi value.
    pub async fn get_phi(&self) -> f64 {
        *self.phi.read().await
    }

    /// Get workspace statistics.
    pub async fn get_stats(&self) -> WorkspaceStats {
        let history = self.broadcast_history.read().await;
        let phi = self.phi.read().await;
        let cycle = self.cycle_counter.read().await;
        let subs = self.subscribers.read().await;
        let coalitions = self.competing_coalitions.read().await;

        WorkspaceStats {
            total_cycles: *cycle,
            total_broadcasts: history.len(),
            current_phi: *phi,
            subscriber_count: subs.len(),
            competing_coalitions: coalitions.len(),
            average_phi: if history.is_empty() {
                0.0
            } else {
                history.iter().map(|b| b.phi_contribution).sum::<f64>() / history.len() as f64
            },
        }
    }
}

impl Default for GlobalWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStats {
    pub total_cycles: u64,
    pub total_broadcasts: usize,
    pub current_phi: f64,
    pub subscriber_count: usize,
    pub competing_coalitions: usize,
    pub average_phi: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::coalition_formation::Coalition;

    struct MockModule {
        name: String,
        content: String,
    }

    #[async_trait]
    impl CognitiveModule for MockModule {
        fn name(&self) -> &str {
            &self.name
        }

        async fn receive_broadcast(&self, broadcast: &ConsciousBroadcast) {
            let _ = broadcast;
        }

        async fn propose_coalition(&self) -> Option<Coalition> {
            Some(Coalition {
                id: uuid::Uuid::new_v4().to_string(),
                content: CognitiveContent {
                    content_type: ContentType::Reasoning,
                    data: self.content.clone(),
                    embedding: vec![0.1, 0.2, 0.3],
                    salience: 0.7,
                    modality: ContentModality::Linguistic,
                },
                strength: 0.7,
                source_modules: vec![self.name.clone()],
                urgency: 0.5,
                novelty: 0.4,
            })
        }

        fn integration_score(&self) -> f64 {
            0.6
        }

        async fn describe_state(&self) -> String {
            format!("Module {} is active", self.name)
        }
    }

    #[tokio::test]
    async fn test_gwt_cycle() {
        let gw = GlobalWorkspace::new();
        let m1 = Arc::new(MockModule { name: "reasoning".to_string(), content: "solve problem A".to_string() });
        let m2 = Arc::new(MockModule { name: "goal_engine".to_string(), content: "achieve goal B".to_string() });

        gw.subscribe(m1).await;
        gw.subscribe(m2).await;

        let broadcast = gw.run_cycle().await;
        assert!(broadcast.is_some());

        let stats = gw.get_stats().await;
        assert_eq!(stats.total_cycles, 1);
        assert_eq!(stats.subscriber_count, 2);
        assert!(stats.current_phi > 0.0);
    }
}

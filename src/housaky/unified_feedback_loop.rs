use crate::housaky::goal_engine::GoalEngine;
use crate::housaky::meta_cognition::MetaCognitionEngine;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEdge {
    pub from_component: String,
    pub to_component: String,
    pub edge_type: FeedbackType,
    pub strength: f64,
    pub latency_ms: u64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FeedbackType {
    Success,
    Failure,
    Learning,
    Reasoning,
    GoalUpdate,
    ToolResult,
    CapabilityChange,
    KnowledgeUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedFeedbackLoopConfig {
    pub enable_goal_to_reasoning: bool,
    pub enable_reasoning_to_tool: bool,
    pub enable_tool_to_learning: bool,
    pub enable_learning_to_goal: bool,
    pub enable_capability_feedback: bool,
    pub enable_knowledge_feedback: bool,
    pub feedback_decay: f64,
    pub max_feedback_history: usize,
}

impl Default for UnifiedFeedbackLoopConfig {
    fn default() -> Self {
        Self {
            enable_goal_to_reasoning: true,
            enable_reasoning_to_tool: true,
            enable_tool_to_learning: true,
            enable_learning_to_goal: true,
            enable_capability_feedback: true,
            enable_knowledge_feedback: true,
            feedback_decay: 0.95,
            max_feedback_history: 1000,
        }
    }
}

pub struct UnifiedFeedbackLoop {
    goal_engine: Arc<GoalEngine>,
    meta_cognition: Arc<MetaCognitionEngine>,
    config: UnifiedFeedbackLoopConfig,
    feedback_edges: Arc<RwLock<Vec<FeedbackEdge>>>,
    feedback_history: Arc<RwLock<VecDeque<FeedbackEvent>>>,
    component_states: Arc<RwLock<HashMap<String, ComponentState>>>,
    loop_closures: Arc<RwLock<Vec<LoopClosure>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentState {
    pub name: String,
    pub health: f64,
    pub last_update: DateTime<Utc>,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: FeedbackType,
    pub source: String,
    pub target: String,
    pub data: serde_json::Value,
    pub success: bool,
    pub impact: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopClosure {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub loop_type: String,
    pub components_involved: Vec<String>,
    pub closure_efficiency: f64,
    pub iterations: u32,
}

impl UnifiedFeedbackLoop {
    pub fn new(
        goal_engine: Arc<GoalEngine>,
        meta_cognition: Arc<MetaCognitionEngine>,
    ) -> Self {
        let edges = Self::create_default_edges();
        
        Self {
            goal_engine,
            meta_cognition,
            config: UnifiedFeedbackLoopConfig::default(),
            feedback_edges: Arc::new(RwLock::new(edges)),
            feedback_history: Arc::new(RwLock::new(VecDeque::new())),
            component_states: Arc::new(RwLock::new(HashMap::new())),
            loop_closures: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn create_default_edges() -> Vec<FeedbackEdge> {
        vec![
            FeedbackEdge {
                from_component: "goal_engine".to_string(),
                to_component: "reasoning".to_string(),
                edge_type: FeedbackType::GoalUpdate,
                strength: 0.8,
                latency_ms: 10,
                enabled: true,
            },
            FeedbackEdge {
                from_component: "reasoning".to_string(),
                to_component: "tools".to_string(),
                edge_type: FeedbackType::Reasoning,
                strength: 0.7,
                latency_ms: 5,
                enabled: true,
            },
            FeedbackEdge {
                from_component: "tools".to_string(),
                to_component: "learning".to_string(),
                edge_type: FeedbackType::ToolResult,
                strength: 0.9,
                latency_ms: 20,
                enabled: true,
            },
            FeedbackEdge {
                from_component: "learning".to_string(),
                to_component: "goal_engine".to_string(),
                edge_type: FeedbackType::Learning,
                strength: 0.6,
                latency_ms: 50,
                enabled: true,
            },
            FeedbackEdge {
                from_component: "learning".to_string(),
                to_component: "meta_cognition".to_string(),
                edge_type: FeedbackType::CapabilityChange,
                strength: 0.85,
                latency_ms: 30,
                enabled: true,
            },
            FeedbackEdge {
                from_component: "meta_cognition".to_string(),
                to_component: "goal_engine".to_string(),
                edge_type: FeedbackType::CapabilityChange,
                strength: 0.75,
                latency_ms: 15,
                enabled: true,
            },
        ]
    }

    pub async fn register_feedback(
        &self,
        from: &str,
        to: &str,
        event_type: FeedbackType,
        data: serde_json::Value,
        success: bool,
    ) -> Result<()> {
        let event = FeedbackEvent {
            id: format!("fb_{}", uuid::Uuid::new_v4()),
            timestamp: Utc::now(),
            event_type: event_type.clone(),
            source: from.to_string(),
            target: to.to_string(),
            data: data.clone(),
            success,
            impact: if success { 0.8 } else { -0.5 },
        };

        self.process_feedback(event.clone()).await?;

        let mut history = self.feedback_history.write().await;
        if history.len() >= self.config.max_feedback_history {
            history.pop_front();
        }
        history.push_back(event);

        Ok(())
    }

    async fn process_feedback(&self, event: FeedbackEvent) -> Result<()> {
        match event.event_type {
            FeedbackType::ToolResult => {
                if event.success {
                    if let Err(e) = self.handle_successful_tool(event.clone()).await {
                        info!("Error processing tool success feedback: {}", e);
                    }
                } else {
                    if let Err(e) = self.handle_failed_tool(event.clone()).await {
                        info!("Error processing tool failure feedback: {}", e);
                    }
                }
            }
            FeedbackType::Learning => {
                self.handle_learning_update(event.clone()).await;
            }
            FeedbackType::GoalUpdate => {
                self.handle_goal_update(event.clone()).await;
            }
            FeedbackType::CapabilityChange => {
                self.handle_capability_change(event.clone()).await;
            }
            _ => {}
        }

        self.check_for_loop_closure(&event).await;

        Ok(())
    }

    async fn handle_successful_tool(&self, event: FeedbackEvent) -> Result<()> {
        self.meta_cognition
            .update_capability("tool_mastery", 0.01)
            .await?;

        if let Some(goal_id) = event.data.get("goal_id").and_then(|v| v.as_str()) {
            let goals = self.goal_engine.get_active_goals().await;
            if let Some(goal) = goals.iter().find(|g| g.id == goal_id) {
                let new_progress = (goal.progress + 0.1).min(1.0);
                self.goal_engine
                    .update_progress(goal_id, new_progress, "Tool executed successfully")
                    .await?;
            }
        }

        Ok(())
    }

    async fn handle_failed_tool(&self, event: FeedbackEvent) -> Result<()> {
        let reason = event.data.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown");

        let _ = self.meta_cognition
            .reflect(&format!("Tool failure: {}", reason))
            .await;

        if let Some(goal_id) = event.data.get("goal_id").and_then(|v| v.as_str()) {
            let _ = self.goal_engine
                .mark_failed(goal_id, reason)
                .await;
        }

        Ok(())
    }

    async fn handle_learning_update(&self, event: FeedbackEvent) {
        if let Some(learning_type) = event.data.get("type").and_then(|v| v.as_str()) {
            let capability = match learning_type {
                "reasoning" => "reasoning",
                "meta" => "meta_cognition",
                "knowledge" => "knowledge_depth",
                _ => "learning",
            };

            let _ = self.meta_cognition
                .update_capability(capability, 0.02)
                .await;
        }
    }

    async fn handle_goal_update(&self, event: FeedbackEvent) {
        if let Some(goal_status) = event.data.get("status").and_then(|v| v.as_str()) {
            if goal_status == "completed" {
                let _ = self.meta_cognition
                    .update_capability("goal_achievement", 0.01)
                    .await;
            }
        }
    }

    async fn handle_capability_change(&self, event: FeedbackEvent) {
        if let Some(capability) = event.data.get("capability").and_then(|v| v.as_str()) {
            if let Some(delta) = event.data.get("delta").and_then(|v| v.as_f64()) {
                let _ = self.meta_cognition
                    .update_capability(capability, delta)
                    .await;
            }
        }
    }

    async fn check_for_loop_closure(&self, _event: &FeedbackEvent) {
        let history = self.feedback_history.read().await;
        
        let recent_events: Vec<_> = history
            .iter()
            .rev()
            .take(10)
            .collect();

        let loop_components: Vec<String> = recent_events
            .iter()
            .map(|e| e.target.clone())
            .collect();

        let has_cycle = loop_components.windows(2).any(|w| {
            recent_events.iter().any(|e| e.source == w[1] && e.target == w[0])
        });

        if has_cycle && loop_components.len() >= 4 {
            let closure = LoopClosure {
                id: format!("closure_{}", uuid::Uuid::new_v4()),
                timestamp: Utc::now(),
                loop_type: "feedback_cycle".to_string(),
                components_involved: loop_components,
                closure_efficiency: 0.7,
                iterations: 1,
            };

            let mut closures = self.loop_closures.write().await;
            closures.push(closure);
        }
    }

    pub async fn propagate_goal_to_reasoning(&self, goals: Vec<String>) -> Result<()> {
        for goal in goals {
            self.register_feedback(
                "goal_engine",
                "reasoning",
                FeedbackType::GoalUpdate,
                serde_json::json!({ "goal_id": goal }),
                true,
            ).await?;
        }
        Ok(())
    }

    pub async fn propagate_reasoning_to_tools(&self, reasoning_result: &str, suggested_tools: &[String]) -> Result<()> {
        for tool in suggested_tools {
            self.register_feedback(
                "reasoning",
                "tools",
                FeedbackType::Reasoning,
                serde_json::json!({
                    "reasoning": reasoning_result,
                    "tool": tool
                }),
                true,
            ).await?;
        }
        Ok(())
    }

    pub async fn propagate_tool_to_learning(&self, tool_name: &str, success: bool, result: &str) -> Result<()> {
        self.register_feedback(
            "tools",
            "learning",
            FeedbackType::ToolResult,
            serde_json::json!({
                "tool": tool_name,
                "success": success,
                "result": result
            }),
            success,
        ).await
    }

    pub async fn propagate_learning_to_goals(&self, learning_outcome: &str) -> Result<()> {
        self.register_feedback(
            "learning",
            "goal_engine",
            FeedbackType::Learning,
            serde_json::json!({ "outcome": learning_outcome }),
            true,
        ).await
    }

    pub async fn get_feedback_metrics(&self) -> FeedbackMetrics {
        let history = self.feedback_history.read().await;
        let edges = self.feedback_edges.read().await;
        let closures = self.loop_closures.read().await;

        let total_events = history.len();
        let success_events = history.iter().filter(|e| e.success).count();
        
        let avg_latency: u64 = if !edges.is_empty() {
            edges.iter().map(|e| e.latency_ms).sum::<u64>() / edges.len() as u64
        } else {
            0
        };

        FeedbackMetrics {
            total_feedback_events: total_events,
            success_rate: if total_events > 0 { success_events as f64 / total_events as f64 } else { 0.0 },
            active_edges: edges.iter().filter(|e| e.enabled).count(),
            loop_closures_detected: closures.len(),
            average_latency_ms: avg_latency,
        }
    }

    pub async fn optimize_feedback_paths(&self) {
        let mut edges = self.feedback_edges.write().await;
        
        for edge in edges.iter_mut() {
            let history = self.feedback_history.read().await;
            
            let relevant_feedback: Vec<_> = history
                .iter()
                .filter(|e| e.source == edge.from_component && e.target == edge.to_component)
                .collect();

            if !relevant_feedback.is_empty() {
                let avg_impact: f64 = relevant_feedback.iter().map(|e| e.impact).sum::<f64>() 
                    / relevant_feedback.len() as f64;
                
                edge.strength = (edge.strength * self.config.feedback_decay + avg_impact * (1.0 - self.config.feedback_decay))
                    .clamp(0.0, 1.0);
            }
        }

        info!("Optimized feedback paths");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackMetrics {
    pub total_feedback_events: usize,
    pub success_rate: f64,
    pub active_edges: usize,
    pub loop_closures_detected: usize,
    pub average_latency_ms: u64,
}

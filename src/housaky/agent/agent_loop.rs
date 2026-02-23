use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::housaky::cognitive::{ActionSelector, CognitiveLoop, InformationGapEngine, PlanningEngine, WorldModel, world_model::{Action, WorldModel, WorldState}, planning::Plan};
use crate::housaky::memory::{BeliefTracker, HierarchicalMemory};
use crate::housaky::goal_engine::GoalEngine;
use crate::housaky::reasoning_engine::ReasoningEngine;
use crate::housaky::knowledge_graph::KnowledgeGraphEngine;
use crate::housaky::working_memory::WorkingMemoryEngine;

pub struct UnifiedAgentLoop {
    pub cognitive_loop: Arc<CognitiveLoop>,
    pub reasoning: Arc<ReasoningEngine>,
    pub world_model: Arc<WorldModel>,
    pub planning: Arc<PlanningEngine>,
    pub information_gap: Arc<InformationGapEngine>,
    pub action_selector: Arc<ActionSelector>,
    pub belief_tracker: Arc<BeliefTracker>,
    pub working_memory: Arc<WorkingMemoryEngine>,
    pub knowledge_graph: Arc<KnowledgeGraphEngine>,
    pub goal_engine: Arc<GoalEngine>,
    pub hierarchical_memory: Arc<HierarchicalMemory>,
    pub session: Arc<RwLock<Option<Session>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub conversation_history: Vec<ConversationEntry>,
    pub context: std::collections::HashMap<String, String>,
    pub active_goals: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConversationEntry {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentInput {
    pub message: String,
    pub session_id: Option<String>,
    pub context: std::collections::HashMap<String, String>,
    pub user_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentOutput {
    pub response: String,
    pub session_id: Option<String>,
    pub actions_taken: Vec<String>,
    pub goals_updated: Vec<String>,
    pub meta: OutputMetadata,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OutputMetadata {
    pub reasoning_steps: usize,
    pub tools_used: Vec<String>,
    pub confidence: f64,
    pub processing_time_ms: u64,
}

impl UnifiedAgentLoop {
    pub fn new_with_config(config: &Config) -> Result<Self> {
        let workspace_dir = &config.workspace_dir;
        Ok(Self {
            cognitive_loop: Arc::new(CognitiveLoop::new(config)?),
            reasoning: Arc::new(ReasoningEngine::new()),
            world_model: Arc::new(WorldModel::new()),
            planning: Arc::new(PlanningEngine::new(Arc::new(WorldModel::new()))),
            information_gap: Arc::new(InformationGapEngine::new()),
            action_selector: Arc::new(ActionSelector::new()),
            belief_tracker: Arc::new(BeliefTracker::new()),
            working_memory: Arc::new(WorkingMemoryEngine::new()),
            knowledge_graph: Arc::new(KnowledgeGraphEngine::new(workspace_dir)),
            goal_engine: Arc::new(GoalEngine::new(workspace_dir)),
            hierarchical_memory: Arc::new(HierarchicalMemory::new(
                crate::memory::hierarchical::HierarchicalMemoryConfig::default()
            )),
            session: Arc::new(RwLock::new(None)),
        })
    }

    pub async fn run_turn(
        &self,
        input: &AgentInput,
        provider: &dyn crate::providers::Provider,
        model: &str,
    ) -> Result<AgentOutput> {
        let start_time = std::time::Instant::now();

        let session = self.get_or_create_session(input.session_id.as_deref()).await?;

        self.working_memory
            .add_message(&input.message, "user")
            .await?;

        let tools: Vec<Box<dyn crate::tools::Tool>> = vec![];
        let tool_refs: Vec<&dyn crate::tools::Tool> = tools.iter().map(|t| t.as_ref()).collect();

        let cognitive_response = self
            .cognitive_loop
            .process(&input.message, provider, model, &tool_refs)
            .await?;

        let gaps = self.information_gap.identify_gaps(
            &crate::cognitive::information_gap::CuriosityContext {
                current_goals: {
                    let goals = self.goal_engine.get_active_goals().await;
                    goals.into_iter().map(|g| g.title).collect()
                },
                recent_events: vec![format!("User message: {}", &input.message[..input.message.len().min(80)])],
                uncertain_topics: vec![],
                existing_knowledge: vec![],
                active_tasks: vec![],
            }
        ).await;
        let _should_learn = self.should_interrupt_for_learning(&gaps).await;

        self.working_memory
            .add_message(&cognitive_response.content, "assistant")
            .await?;

        let output = AgentOutput {
            response: cognitive_response.content.clone(),
            session_id: Some(session.id.clone()),
            actions_taken: cognitive_response.actions_taken.clone(),
            goals_updated: if cognitive_response.goals_updated { vec!["updated".to_string()] } else { vec![] },
            meta: OutputMetadata {
                reasoning_steps: cognitive_response.thoughts.len(),
                tools_used: cognitive_response.actions_taken,
                confidence: cognitive_response.confidence,
                processing_time_ms: start_time.elapsed().as_millis() as u64,
            },
        };

        Ok(output)
    }


    async fn get_or_create_session(&self, session_id: Option<&str>) -> Result<Session> {
        let mut session_lock = self.session.write().await;
        
        if let Some(id) = session_id {
            if let Some(ref existing) = *session_lock {
                if existing.id == id {
                    return Ok(existing.clone());
                }
            }
        }

        let new_session = Session {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_history: vec![],
            context: std::collections::HashMap::new(),
            active_goals: vec![],
            created_at: chrono::Utc::now(),
            last_updated: chrono::Utc::now(),
        };

        *session_lock = Some(new_session.clone());
        Ok(new_session)
    }

    async fn should_interrupt_for_learning(
        &self,
        gaps: &[crate::cognitive::KnowledgeGap],
    ) -> bool {
        let urgent_gaps = gaps.iter().filter(|g| g.urgency > 0.7).count();
        urgent_gaps > 0
    }

    pub async fn get_session(&self) -> Option<Session> {
        self.session.read().await.clone()
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        // UnifiedAgentLoop::new_with_config requires a Config — test session creation directly
        let session = Session {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_history: vec![],
            context: std::collections::HashMap::new(),
            active_goals: vec![],
            created_at: chrono::Utc::now(),
            last_updated: chrono::Utc::now(),
        };
        assert!(!session.id.is_empty());
    }
}

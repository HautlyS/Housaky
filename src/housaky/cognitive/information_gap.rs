use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KnowledgeGap {
    pub id: String,
    pub topic: String,
    pub question: String,
    pub urgency: f64,
    pub importance: f64,
    pub source: GapSource,
    pub related_beliefs: Vec<String>,
    pub discovered_at: DateTime<Utc>,
    pub status: GapStatus,
    pub attempts: Vec<GapAttempt>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GapSource {
    BeliefUncertainty,
    ReasoningFailure,
    GoalPrerequisite,
    ExternalQuery,
    PredictionError,
    UserQuestion,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GapStatus {
    Identified,
    Investigating,
    Resolved,
    Unresolvable,
    Ignored,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GapAttempt {
    pub timestamp: DateTime<Utc>,
    pub strategy: String,
    pub result: String,
    pub success: bool,
}

pub struct CuriosityEngine {
    novelty_weight: f64,
    relevance_weight: f64,
    surprise_weight: f64,
    information_gain_weight: f64,
    storage_path: Option<PathBuf>,
}

impl CuriosityEngine {
    pub fn new() -> Self {
        Self {
            novelty_weight: 0.3,
            relevance_weight: 0.3,
            surprise_weight: 0.2,
            information_gain_weight: 0.2,
            storage_path: None,
        }
    }

    pub fn with_storage(workspace_dir: &PathBuf) -> Self {
        let storage_path = workspace_dir.join(".housaky").join("curiosity.json");
        Self {
            novelty_weight: 0.3,
            relevance_weight: 0.3,
            surprise_weight: 0.2,
            information_gain_weight: 0.2,
            storage_path: Some(storage_path),
        }
    }

    pub async fn load(&self) -> Result<CuriosityState> {
        if let Some(ref path) = self.storage_path {
            if path.exists() {
                let content = tokio::fs::read_to_string(path).await?;
                let state: CuriosityState = serde_json::from_str(&content)?;
                return Ok(state);
            }
        }
        Ok(CuriosityState::default())
    }

    pub async fn save(&self, state: &CuriosityState) -> Result<()> {
        if let Some(ref path) = self.storage_path {
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            let content = serde_json::to_string_pretty(state)?;
            tokio::fs::write(path, content).await?;
        }
        Ok(())
    }

    pub fn calculate_curiosity(
        &self,
        topic: &str,
        context: &CuriosityContext,
        existing_knowledge: &[String],
    ) -> f64 {
        let novelty = self.calculate_novelty(topic, existing_knowledge);
        let relevance = self.calculate_relevance(topic, &context.current_goals);
        let surprise = self.calculate_surprise(topic, &context.recent_events);
        let info_gain = self.estimate_information_gain(topic, &context.uncertain_topics);

        (novelty * self.novelty_weight)
            + (relevance * self.relevance_weight)
            + (surprise * self.surprise_weight)
            + (info_gain * self.information_gain_weight)
    }

    fn calculate_novelty(&self, topic: &str, existing_knowledge: &[String]) -> f64 {
        if existing_knowledge.is_empty() {
            return 1.0;
        }

        let topic_lower = topic.to_lowercase();
        let known_count = existing_knowledge
            .iter()
            .filter(|k| k.to_lowercase().contains(&topic_lower))
            .count();

        1.0 - (known_count as f64 / existing_knowledge.len() as f64)
    }

    fn calculate_relevance(&self, topic: &str, goals: &[String]) -> f64 {
        if goals.is_empty() {
            return 0.5;
        }

        let topic_lower = topic.to_lowercase();
        let relevant_goals = goals
            .iter()
            .filter(|g| g.to_lowercase().contains(&topic_lower))
            .count();

        relevant_goals as f64 / goals.len() as f64
    }

    fn calculate_surprise(&self, topic: &str, recent_events: &[String]) -> f64 {
        if recent_events.is_empty() {
            return 0.0;
        }

        let topic_lower = topic.to_lowercase();
        let unexpected = recent_events
            .iter()
            .filter(|e| {
                let event_lower = e.to_lowercase();
                !event_lower.contains(&topic_lower) && !topic_lower.contains(&event_lower)
            })
            .count();

        unexpected as f64 / recent_events.len() as f64
    }

    fn estimate_information_gain(&self, topic: &str, uncertain_topics: &[String]) -> f64 {
        if uncertain_topics.is_empty() {
            return 0.5;
        }

        let topic_lower = topic.to_lowercase();
        let uncertain_count = uncertain_topics
            .iter()
            .filter(|u| u.to_lowercase().contains(&topic_lower))
            .count();

        uncertain_count as f64 / uncertain_topics.len() as f64
    }

    pub fn rank_topics(&self, topics: &[String], context: &CuriosityContext) -> Vec<ScoredTopic> {
        let existing: Vec<String> = context.existing_knowledge.clone();

        let mut scored: Vec<ScoredTopic> = topics
            .iter()
            .map(|t| ScoredTopic {
                topic: t.clone(),
                score: self.calculate_curiosity(t, context, &existing),
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        scored
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CuriosityContext {
    pub current_goals: Vec<String>,
    pub recent_events: Vec<String>,
    pub uncertain_topics: Vec<String>,
    pub existing_knowledge: Vec<String>,
    pub active_tasks: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoredTopic {
    pub topic: String,
    pub score: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CuriosityState {
    pub total_explorations: u64,
    pub successful_learnings: u64,
    pub topic_history: Vec<String>,
    pub curiosity_score: f64,
}

pub struct InformationGapEngine {
    curiosity: Arc<CuriosityEngine>,
    belief_tracker: Option<Arc<crate::housaky::memory::BeliefTracker>>,
    knowledge_graph: Option<Arc<crate::housaky::knowledge_graph::KnowledgeGraphEngine>>,
    gaps: Arc<RwLock<HashMap<String, KnowledgeGap>>>,
    storage_path: Option<PathBuf>,
}

impl InformationGapEngine {
    pub fn new() -> Self {
        Self {
            curiosity: Arc::new(CuriosityEngine::new()),
            belief_tracker: None,
            knowledge_graph: None,
            gaps: Arc::new(RwLock::new(HashMap::new())),
            storage_path: None,
        }
    }

    pub fn with_belief_tracker(
        mut self,
        tracker: Arc<crate::housaky::memory::BeliefTracker>,
    ) -> Self {
        self.belief_tracker = Some(tracker);
        self
    }

    pub fn with_knowledge_graph(
        mut self,
        kg: Arc<crate::housaky::knowledge_graph::KnowledgeGraphEngine>,
    ) -> Self {
        self.knowledge_graph = Some(kg);
        self
    }

    pub fn with_storage(mut self, workspace_dir: &PathBuf) -> Self {
        let storage_path = workspace_dir.join(".housaky").join("information_gaps.json");
        self.storage_path = Some(storage_path);
        self
    }

    pub async fn load(&self) -> Result<()> {
        if let Some(ref path) = self.storage_path {
            if path.exists() {
                let content = tokio::fs::read_to_string(path).await?;
                let loaded: HashMap<String, KnowledgeGap> = serde_json::from_str(&content)?;
                let mut gaps = self.gaps.write().await;
                *gaps = loaded;
                info!("Loaded {} knowledge gaps from storage", gaps.len());
            }
        }
        Ok(())
    }

    pub async fn save(&self) -> Result<()> {
        if let Some(ref path) = self.storage_path {
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            let gaps = self.gaps.read().await;
            let content = serde_json::to_string_pretty(&*gaps)?;
            tokio::fs::write(path, content).await?;
        }
        Ok(())
    }

    pub async fn identify_gaps(&self, context: &CuriosityContext) -> Vec<KnowledgeGap> {
        let mut identified_gaps = Vec::new();

        if let Some(ref tracker) = self.belief_tracker {
            let uncertain = tracker.get_uncertain_beliefs().await;
            for belief in uncertain {
                let gap = KnowledgeGap {
                    id: uuid::Uuid::new_v4().to_string(),
                    topic: belief.content.clone(),
                    question: format!("Verify: {}", belief.content),
                    urgency: 1.0 - belief.confidence,
                    importance: belief.importance,
                    source: GapSource::BeliefUncertainty,
                    related_beliefs: vec![belief.id.clone()],
                    discovered_at: Utc::now(),
                    status: GapStatus::Identified,
                    attempts: vec![],
                };
                identified_gaps.push(gap);
            }
        }

        if let Some(ref kg) = self.knowledge_graph {
            let recent_entities = kg
                .query(crate::housaky::knowledge_graph::GraphQuery::Recent(10))
                .await;
            for entity in recent_entities.iter().take(5) {
                if entity.confidence < 0.6 {
                    let gap = KnowledgeGap {
                        id: uuid::Uuid::new_v4().to_string(),
                        topic: entity.name.clone(),
                        question: format!("Learn more about: {}", entity.name),
                        urgency: 0.5,
                        importance: entity.importance,
                        source: GapSource::GoalPrerequisite,
                        related_beliefs: vec![],
                        discovered_at: Utc::now(),
                        status: GapStatus::Identified,
                        attempts: vec![],
                    };
                    identified_gaps.push(gap);
                }
            }
        }

        let scored_topics = self
            .curiosity
            .rank_topics(
                &context.uncertain_topics,
                &CuriosityContext {
                    current_goals: context.current_goals.clone(),
                    recent_events: context.recent_events.clone(),
                    uncertain_topics: context.uncertain_topics.clone(),
                    existing_knowledge: context.existing_knowledge.clone(),
                    active_tasks: context.active_tasks.clone(),
                },
            )
            .into_iter()
            .take(3)
            .collect::<Vec<_>>();

        for scored in scored_topics {
            let gap = KnowledgeGap {
                id: uuid::Uuid::new_v4().to_string(),
                topic: scored.topic.clone(),
                question: format!("Explore: {}", scored.topic),
                urgency: scored.score,
                importance: scored.score,
                source: GapSource::ExternalQuery,
                related_beliefs: vec![],
                discovered_at: Utc::now(),
                status: GapStatus::Identified,
                attempts: vec![],
            };
            identified_gaps.push(gap);
        }

        identified_gaps.sort_by(|a, b| b.urgency.partial_cmp(&a.urgency).unwrap());

        let mut gaps = self.gaps.write().await;
        for gap in &identified_gaps {
            gaps.insert(gap.id.clone(), gap.clone());
        }

        identified_gaps
    }

    pub async fn create_learning_goal(
        &self,
        gap: &KnowledgeGap,
    ) -> Result<crate::housaky::goal_engine::Goal> {
        let goal = crate::housaky::goal_engine::Goal {
            id: uuid::Uuid::new_v4().to_string(),
            title: format!("Learn about {}", gap.topic),
            description: gap.question.clone(),
            priority: match gap.urgency {
                u if u > 0.8 => crate::housaky::goal_engine::GoalPriority::Critical,
                u if u > 0.6 => crate::housaky::goal_engine::GoalPriority::High,
                u if u > 0.4 => crate::housaky::goal_engine::GoalPriority::Medium,
                _ => crate::housaky::goal_engine::GoalPriority::Low,
            },
            category: crate::housaky::goal_engine::GoalCategory::KnowledgeExpansion,
            progress: 0.0,
            status: crate::housaky::goal_engine::GoalStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deadline: None,
            parent_id: None,
            subtask_ids: vec![],
            dependencies: gap.related_beliefs.clone(),
            blockers: vec![],
            metrics: HashMap::new(),
            checkpoints: vec![],
            attempts: 0,
            max_attempts: 3,
            estimated_complexity: 0.5,
            actual_complexity: None,
            learning_value: gap.importance,
            tags: vec![gap.topic.clone()],
            context: HashMap::new(),
        };

        Ok(goal)
    }

    pub async fn get_active_gaps(&self) -> Vec<KnowledgeGap> {
        let gaps = self.gaps.read().await;
        gaps.values()
            .filter(|g| g.status == GapStatus::Identified || g.status == GapStatus::Investigating)
            .cloned()
            .collect()
    }

    pub async fn mark_resolved(&self, gap_id: &str, result: &str) {
        let mut gaps = self.gaps.write().await;
        if let Some(gap) = gaps.get_mut(gap_id) {
            gap.status = GapStatus::Resolved;
            gap.attempts.push(GapAttempt {
                timestamp: Utc::now(),
                strategy: "Research".to_string(),
                result: result.to_string(),
                success: true,
            });
        }
    }

    pub async fn mark_failed(&self, gap_id: &str, reason: &str) {
        let mut gaps = self.gaps.write().await;
        if let Some(gap) = gaps.get_mut(gap_id) {
            gap.attempts.push(GapAttempt {
                timestamp: Utc::now(),
                strategy: "Research".to_string(),
                result: reason.to_string(),
                success: false,
            });

            if gap.attempts.len() >= 3 {
                gap.status = GapStatus::Unresolvable;
            }
        }
    }
}

impl Default for CuriosityEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for InformationGapEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curiosity_calculation() {
        let engine = CuriosityEngine::new();
        let context = CuriosityContext {
            current_goals: vec!["rust programming".to_string()],
            recent_events: vec!["error occurred".to_string()],
            uncertain_topics: vec!["async programming".to_string()],
            existing_knowledge: vec!["rust".to_string()],
            active_tasks: vec![],
        };

        let score = engine.calculate_curiosity("rust programming", &context, &[]);
        assert!(score > 0.0);
    }

    #[tokio::test]
    async fn test_gap_identification() {
        let engine = InformationGapEngine::new();
        let context = CuriosityContext {
            current_goals: vec![],
            recent_events: vec![],
            uncertain_topics: vec!["new topic".to_string()],
            existing_knowledge: vec![],
            active_tasks: vec![],
        };

        let gaps = engine.identify_gaps(&context).await;
        assert!(!gaps.is_empty());
    }
}

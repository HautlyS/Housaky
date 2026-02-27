use crate::housaky::goal_engine::{Goal, GoalCategory, GoalEngine, GoalPriority, GoalStatus};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalSelectionContext {
    pub user_request: Option<String>,
    pub active_goals: Vec<String>,
    pub knowledge_gaps: Vec<String>,
    pub recent_successes: Vec<String>,
    pub recent_failures: Vec<String>,
    pub capability_levels: HashMap<String, f64>,
    pub available_resources: HashMap<String, f64>,
    pub time_constraints: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalScore {
    pub goal: Goal,
    pub score: f64,
    pub reasoning: String,
    pub alignment_with_knowledge: f64,
    pub capability_match: f64,
    pub priority_bonus: f64,
    pub resource_availability: f64,
}

pub struct KnowledgeGuidedGoalSelector {
    goal_engine: Arc<GoalEngine>,
    knowledge_weights: Arc<RwLock<KnowledgeWeights>>,
    selection_history: Arc<RwLock<Vec<GoalSelectionRecord>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeWeights {
    pub knowledge_alignment: f64,
    pub capability_match: f64,
    pub urgency: f64,
    pub learning_value: f64,
    pub resource_efficiency: f64,
}

impl Default for KnowledgeWeights {
    fn default() -> Self {
        Self {
            knowledge_alignment: 0.3,
            capability_match: 0.25,
            urgency: 0.2,
            learning_value: 0.15,
            resource_efficiency: 0.1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalSelectionRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub context: GoalSelectionContext,
    pub selected_goals: Vec<GoalScore>,
    pub rejected_goals: Vec<GoalScore>,
    pub reasoning_summary: String,
}

impl KnowledgeGuidedGoalSelector {
    pub fn new(goal_engine: Arc<GoalEngine>) -> Self {
        Self {
            goal_engine,
            knowledge_weights: Arc::new(RwLock::new(KnowledgeWeights::default())),
            selection_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn select_goals(&self, context: GoalSelectionContext, max_goals: usize) -> Result<Vec<GoalScore>> {
        let pending_goals = self.goal_engine.get_active_goals().await;
        
        let mut scored_goals = Vec::new();

        for goal in pending_goals {
            let score = self.score_goal(&goal, &context).await;
            scored_goals.push(score);
        }

        scored_goals.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        let selected: Vec<GoalScore> = scored_goals.into_iter().take(max_goals).collect();

        self.record_selection(context, selected.clone()).await;

        Ok(selected)
    }

    async fn score_goal(&self, goal: &Goal, context: &GoalSelectionContext) -> GoalScore {
        let knowledge_alignment = self.calculate_knowledge_alignment(goal, &context.knowledge_gaps);
        let capability_match = self.calculate_capability_match(goal, &context.capability_levels);
        let priority_bonus = self.calculate_priority_bonus(goal);
        let resource_availability = self.calculate_resource_availability(goal, &context.available_resources);

        let weights = self.knowledge_weights.read().await;

        let score = 
            (knowledge_alignment * weights.knowledge_alignment) +
            (capability_match * weights.capability_match) +
            (priority_bonus * weights.urgency) +
            (goal.learning_value * weights.learning_value) +
            (resource_availability * weights.resource_efficiency);

        let reasoning = format!(
            "Knowledge alignment: {:.2}, Capability match: {:.2}, Priority: {:?}, Resources: {:.2}",
            knowledge_alignment, capability_match, goal.priority, resource_availability
        );

        GoalScore {
            goal: goal.clone(),
            score,
            reasoning,
            alignment_with_knowledge: knowledge_alignment,
            capability_match,
            priority_bonus,
            resource_availability,
        }
    }

    fn calculate_knowledge_alignment(&self, goal: &Goal, knowledge_gaps: &[String]) -> f64 {
        let goal_lower = goal.title.to_lowercase();
        
        let mut alignment_score: f64 = 0.5;

        for gap in knowledge_gaps {
            let gap_lower = gap.to_lowercase();
            if goal_lower.contains(&gap_lower) || gap_lower.contains(&goal_lower) {
                alignment_score += 0.3;
            }
        }

        for tag in &goal.tags {
            if knowledge_gaps.iter().any(|g| g.to_lowercase().contains(&tag.to_lowercase())) {
                alignment_score += 0.1;
            }
        }

        alignment_score.min(1.0)
    }

    fn calculate_capability_match(&self, goal: &Goal, capability_levels: &HashMap<String, f64>) -> f64 {
        let complexity = goal.estimated_complexity;

        let avg_capability: f64 = if capability_levels.is_empty() {
            0.5
        } else {
            capability_levels.values().sum::<f64>() / capability_levels.len() as f64
        };

        let match_score = if complexity <= 3.0 && avg_capability >= 0.7 {
            0.9
        } else if complexity <= 5.0 && avg_capability >= 0.5 {
            0.7
        } else if complexity <= 8.0 && avg_capability >= 0.3 {
            0.5
        } else {
            0.3
        };

        match_score
    }

    fn calculate_priority_bonus(&self, goal: &Goal) -> f64 {
        match goal.priority {
            GoalPriority::Critical => 1.0,
            GoalPriority::High => 0.8,
            GoalPriority::Medium => 0.5,
            GoalPriority::Low => 0.3,
            GoalPriority::Background => 0.1,
        }
    }

    fn calculate_resource_availability(&self, goal: &Goal, available_resources: &HashMap<String, f64>) -> f64 {
        let required_resources = goal.estimated_complexity;

        let total_available: f64 = available_resources.values().sum();

        if total_available >= required_resources * 2.0 {
            1.0
        } else if total_available >= required_resources {
            0.7
        } else if total_available >= required_resources * 0.5 {
            0.4
        } else {
            0.2
        }
    }

    async fn record_selection(&self, context: GoalSelectionContext, selected: Vec<GoalScore>) {
        let record = GoalSelectionRecord {
            id: format!("selection_{}", uuid::Uuid::new_v4()),
            timestamp: Utc::now(),
            context,
            selected_goals: selected,
            rejected_goals: Vec::new(),
            reasoning_summary: "Goal selection completed".to_string(),
        };

        let mut history = self.selection_history.write().await;
        history.push(record);
    }

    pub async fn update_weights(&self, weights: KnowledgeWeights) {
        let mut current = self.knowledge_weights.write().await;
        *current = weights;
        info!("Updated knowledge-guided goal selection weights");
    }

    pub async fn get_selection_history(&self) -> Vec<GoalSelectionRecord> {
        self.selection_history.read().await.clone()
    }

    pub async fn suggest_new_goals(&self, context: &GoalSelectionContext) -> Vec<Goal> {
        let mut suggested = Vec::new();

        for gap in &context.knowledge_gaps {
            let goal = Goal {
                id: String::new(),
                title: format!("Address knowledge gap: {}", gap),
                description: format!("Close the knowledge gap in {}", gap),
                priority: GoalPriority::High,
                status: GoalStatus::Pending,
                category: GoalCategory::KnowledgeExpansion,
                progress: 0.0,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                deadline: Some(Utc::now() + chrono::Duration::days(7)),
                parent_id: None,
                subtask_ids: Vec::new(),
                dependencies: Vec::new(),
                blockers: Vec::new(),
                metrics: HashMap::new(),
                checkpoints: Vec::new(),
                attempts: 0,
                max_attempts: 3,
                estimated_complexity: 5.0,
                actual_complexity: None,
                learning_value: 0.8,
                tags: vec!["knowledge_gap".to_string(), gap.clone()],
                context: HashMap::new(),
                temporal_constraints: Vec::new(),
            };
            suggested.push(goal);
        }

        if context.capability_levels.iter().any(|(_, v)| *v < 0.7) {
            let low_caps: Vec<String> = context.capability_levels
                .iter()
                .filter(|(_, v)| **v < 0.7)
                .map(|(k, _)| k.clone())
                .collect();

            if !low_caps.is_empty() {
                suggested.push(Goal {
                    id: String::new(),
                    title: format!("Improve capabilities: {}", low_caps.join(", ")),
                    description: "Address low capability areas".to_string(),
                    priority: GoalPriority::Medium,
                    status: GoalStatus::Pending,
                    category: GoalCategory::Intelligence,
                    progress: 0.0,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    deadline: Some(Utc::now() + chrono::Duration::days(14)),
                    parent_id: None,
                    subtask_ids: Vec::new(),
                    dependencies: Vec::new(),
                    blockers: Vec::new(),
                    metrics: HashMap::new(),
                    checkpoints: Vec::new(),
                    attempts: 0,
                    max_attempts: 3,
                    estimated_complexity: 8.0,
                    actual_complexity: None,
                    learning_value: 1.0,
                    tags: vec!["capability_improvement".to_string()],
                    context: HashMap::new(),
                    temporal_constraints: Vec::new(),
                });
            }
        }

        suggested
    }
}

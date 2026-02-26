//! Curriculum — Self-directed curriculum: identify knowledge gaps → study plan.
//!
//! Manages the agent's self-directed learning path. Analyses the current knowledge
//! frontier, identifies gaps, and produces an ordered sequence of `StudySession`s
//! that close those gaps most efficiently.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};
use uuid::Uuid;

// ── Learning Objective ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningObjective {
    pub id: String,
    pub topic: String,
    pub description: String,
    pub target_mastery: f64,
    pub current_mastery: f64,
    pub deadline: Option<DateTime<Utc>>,
    pub prerequisites: Vec<String>,
    pub completed: bool,
    pub priority: f64,
}

impl LearningObjective {
    pub fn new(topic: &str, description: &str, target_mastery: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            topic: topic.to_string(),
            description: description.to_string(),
            target_mastery,
            current_mastery: 0.0,
            deadline: None,
            prerequisites: Vec::new(),
            completed: false,
            priority: 0.5,
        }
    }

    pub fn gap(&self) -> f64 {
        (self.target_mastery - self.current_mastery).max(0.0)
    }

    pub fn is_unblocked(&self, mastered_topics: &[&str]) -> bool {
        self.prerequisites
            .iter()
            .all(|p| mastered_topics.contains(&p.as_str()))
    }

    pub fn urgency_score(&self) -> f64 {
        let gap_factor = self.gap();
        let deadline_factor = self
            .deadline
            .map(|d| {
                let days = (d - Utc::now()).num_days().max(1);
                1.0 / days as f64
            })
            .unwrap_or(0.5);
        self.priority * gap_factor * (1.0 + deadline_factor)
    }
}

// ── Study Session ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StudyMethod {
    ReadPaper { paper_id: String },
    SolveProblems { problem_set: String },
    RunExperiment { description: String },
    ReviewSynthesis { synthesis_id: String },
    TeachBack { topic: String },
    ActiveRecall { topic: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudySession {
    pub id: String,
    pub objective_id: String,
    pub topic: String,
    pub method: StudyMethod,
    pub estimated_duration_mins: u32,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub completed: bool,
    pub mastery_gain_estimate: f64,
    pub actual_mastery_gain: Option<f64>,
}

impl StudySession {
    pub fn reading_session(
        objective_id: &str,
        topic: &str,
        paper_id: &str,
        duration_mins: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            objective_id: objective_id.to_string(),
            topic: topic.to_string(),
            method: StudyMethod::ReadPaper {
                paper_id: paper_id.to_string(),
            },
            estimated_duration_mins: duration_mins,
            scheduled_at: None,
            completed: false,
            mastery_gain_estimate: 0.05,
            actual_mastery_gain: None,
        }
    }
}

// ── Knowledge Frontier ────────────────────────────────────────────────────────

/// Maps topic names to current mastery levels (0.0 – 1.0).
pub type KnowledgeFrontier = HashMap<String, f64>;

// ── Curriculum ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Curriculum {
    pub id: String,
    pub name: String,
    pub knowledge_frontier: KnowledgeFrontier,
    pub learning_objectives: Vec<LearningObjective>,
    pub study_plan: Vec<StudySession>,
    pub mastery_threshold: f64,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub completed_sessions: Vec<StudySession>,
}

impl Curriculum {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            knowledge_frontier: HashMap::new(),
            learning_objectives: Vec::new(),
            study_plan: Vec::new(),
            mastery_threshold: 0.80,
            created_at: Utc::now(),
            last_updated: Utc::now(),
            completed_sessions: Vec::new(),
        }
    }

    /// Add or update a topic in the knowledge frontier.
    pub fn update_mastery(&mut self, topic: &str, mastery: f64) {
        self.knowledge_frontier
            .insert(topic.to_string(), mastery.clamp(0.0, 1.0));
        self.last_updated = Utc::now();

        // Update any objectives for this topic
        for obj in &mut self.learning_objectives {
            if obj.topic == topic {
                obj.current_mastery = mastery;
                if mastery >= obj.target_mastery {
                    obj.completed = true;
                }
            }
        }
    }

    /// Add a learning objective.
    pub fn add_objective(&mut self, objective: LearningObjective) {
        self.learning_objectives.push(objective);
    }

    /// Identify the top-N knowledge gaps.
    pub fn identify_gaps(&self, top_n: usize) -> Vec<KnowledgeGap> {
        let mut gaps: Vec<KnowledgeGap> = self
            .learning_objectives
            .iter()
            .filter(|o| !o.completed)
            .map(|o| KnowledgeGap {
                topic: o.topic.clone(),
                current_mastery: o.current_mastery,
                target_mastery: o.target_mastery,
                gap_size: o.gap(),
                urgency: o.urgency_score(),
                objective_id: o.id.clone(),
            })
            .collect();

        gaps.sort_by(|a, b| {
            b.urgency
                .partial_cmp(&a.urgency)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        gaps.truncate(top_n);
        gaps
    }

    /// Generate a study plan to close the top-N gaps.
    pub fn generate_study_plan(&mut self, gaps: &[KnowledgeGap]) {
        let mastered: Vec<&str> = self
            .knowledge_frontier
            .iter()
            .filter(|(_, &m)| m >= self.mastery_threshold)
            .map(|(t, _)| t.as_str())
            .collect();

        let mut sessions = Vec::new();

        for gap in gaps {
            let obj = self
                .learning_objectives
                .iter()
                .find(|o| o.id == gap.objective_id);
            if let Some(obj) = obj {
                if !obj.is_unblocked(&mastered) {
                    debug!("Objective '{}' blocked by prerequisites", obj.topic);
                    continue;
                }
                // Generate 2 sessions per gap: read + active recall
                sessions.push(StudySession {
                    id: Uuid::new_v4().to_string(),
                    objective_id: obj.id.clone(),
                    topic: gap.topic.clone(),
                    method: StudyMethod::ReadPaper {
                        paper_id: format!("auto:{}", gap.topic),
                    },
                    estimated_duration_mins: 30,
                    scheduled_at: Some(Utc::now()),
                    completed: false,
                    mastery_gain_estimate: 0.08,
                    actual_mastery_gain: None,
                });
                sessions.push(StudySession {
                    id: Uuid::new_v4().to_string(),
                    objective_id: obj.id.clone(),
                    topic: gap.topic.clone(),
                    method: StudyMethod::ActiveRecall {
                        topic: gap.topic.clone(),
                    },
                    estimated_duration_mins: 15,
                    scheduled_at: Some(Utc::now() + Duration::minutes(35)),
                    completed: false,
                    mastery_gain_estimate: 0.05,
                    actual_mastery_gain: None,
                });
            }
        }

        self.study_plan.extend(sessions);
        info!(
            "Curriculum '{}': generated {} sessions for {} gaps",
            self.name,
            self.study_plan.len(),
            gaps.len()
        );
        self.last_updated = Utc::now();
    }

    /// Mark a session as completed and apply mastery gain.
    pub fn complete_session(&mut self, session_id: &str, actual_gain: f64) {
        let mut completed_session = None;
        for s in &mut self.study_plan {
            if s.id == session_id {
                s.completed = true;
                s.actual_mastery_gain = Some(actual_gain);
                completed_session = Some(s.clone());
                break;
            }
        }
        if let Some(session) = completed_session {
            let mastery = self
                .knowledge_frontier
                .entry(session.topic.clone())
                .or_insert(0.0);
            *mastery = (*mastery + actual_gain).min(1.0);

            // Sync objectives
            for obj in &mut self.learning_objectives {
                if obj.topic == session.topic {
                    obj.current_mastery = *self
                        .knowledge_frontier
                        .get(&session.topic)
                        .unwrap_or(&0.0);
                    if obj.current_mastery >= obj.target_mastery {
                        obj.completed = true;
                    }
                }
            }

            self.study_plan.retain(|s| s.id != session_id);
            self.completed_sessions.push(session);
        }
    }

    pub fn overall_mastery(&self) -> f64 {
        if self.knowledge_frontier.is_empty() {
            return 0.0;
        }
        self.knowledge_frontier.values().sum::<f64>() / self.knowledge_frontier.len() as f64
    }

    pub fn completion_rate(&self) -> f64 {
        let total = self.learning_objectives.len();
        if total == 0 {
            return 1.0;
        }
        let completed = self.learning_objectives.iter().filter(|o| o.completed).count();
        completed as f64 / total as f64
    }
}

impl Default for Curriculum {
    fn default() -> Self {
        Self::new("default")
    }
}

// ── Knowledge Gap ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGap {
    pub topic: String,
    pub current_mastery: f64,
    pub target_mastery: f64,
    pub gap_size: f64,
    pub urgency: f64,
    pub objective_id: String,
}

use crate::housaky::cognitive::action_selector::{ActionDecision, ActionOutcome, ActionResult};
use crate::housaky::cognitive::perception::PerceivedInput;
use crate::util::{read_msgpack_file, write_msgpack_file};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub perception: PerceptionSummary,
    pub action: ActionSummary,
    pub outcome: OutcomeSummary,
    pub patterns_extracted: Vec<Pattern>,
    pub lessons_learned: Vec<Lesson>,
    pub success_score: f64,
    pub replayable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionSummary {
    pub input_hash: String,
    pub intent: String,
    pub topics: Vec<String>,
    pub complexity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSummary {
    pub action_type: String,
    pub tool_used: Option<String>,
    pub arguments_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeSummary {
    pub success: bool,
    pub duration_ms: u64,
    pub side_effects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub pattern_type: PatternType,
    pub description: String,
    pub conditions: Vec<String>,
    pub actions: Vec<String>,
    pub confidence: f64,
    pub occurrences: u64,
    pub success_rate: f64,
    pub first_seen: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PatternType {
    InputOutput,
    Sequence,
    Conditional,
    Correction,
    Optimization,
    Recovery,
    UserPreference,
    ErrorHandling,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lesson {
    pub id: String,
    pub topic: String,
    pub insight: String,
    pub context: String,
    pub applicability: Vec<String>,
    pub confidence: f64,
    pub source_experience: String,
    pub validated: bool,
    pub validation_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPrototype {
    pub id: String,
    pub name: String,
    pub description: String,
    pub patterns: Vec<String>,
    pub triggers: Vec<String>,
    pub actions: Vec<ActionTemplate>,
    pub success_rate: f64,
    pub usage_count: u64,
    pub ready_for_promotion: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionTemplate {
    pub action_type: String,
    pub tool: Option<String>,
    pub arguments_template: String,
    pub conditions: Vec<String>,
}

pub struct ExperienceLearner {
    experiences: Arc<RwLock<Vec<Experience>>>,
    patterns: Arc<RwLock<HashMap<String, Pattern>>>,
    lessons: Arc<RwLock<Vec<Lesson>>>,
    skill_prototypes: Arc<RwLock<Vec<SkillPrototype>>>,
    workspace_dir: PathBuf,
    min_pattern_occurrences: u64,
    pattern_confidence_threshold: f64,
}

impl ExperienceLearner {
    pub fn new(workspace_dir: &PathBuf) -> Self {
        Self {
            experiences: Arc::new(RwLock::new(Vec::new())),
            patterns: Arc::new(RwLock::new(HashMap::new())),
            lessons: Arc::new(RwLock::new(Vec::new())),
            skill_prototypes: Arc::new(RwLock::new(Vec::new())),
            workspace_dir: workspace_dir.clone(),
            min_pattern_occurrences: 3,
            pattern_confidence_threshold: 0.7,
        }
    }

    pub async fn record_experience(
        &self,
        perception: &PerceivedInput,
        decision: &ActionDecision,
        outcome: &ActionOutcome,
    ) -> Result<String> {
        info!("Recording experience...");

        let success_score = match &outcome.result {
            ActionResult::Success { .. } => 1.0,
            ActionResult::PartialSuccess { issues, .. } => 0.7 - (issues.len() as f64 * 0.1),
            ActionResult::Failure { recoverable, .. } => {
                if *recoverable {
                    0.3
                } else {
                    0.1
                }
            }
            ActionResult::Cancelled { .. } => 0.0,
        };

        let experience = Experience {
            id: format!("exp_{}", uuid::Uuid::new_v4()),
            timestamp: chrono::Utc::now(),
            perception: PerceptionSummary {
                input_hash: self.hash_input(&perception.raw_input),
                intent: format!("{:?}", perception.intent.primary),
                topics: perception.topics.clone(),
                complexity: perception.complexity,
            },
            action: ActionSummary {
                action_type: format!("{:?}", decision.action),
                tool_used: self.extract_tool_from_action(&decision.action),
                arguments_hash: self.hash_arguments(&decision.action),
            },
            outcome: OutcomeSummary {
                success: success_score > 0.5,
                duration_ms: outcome.duration_ms,
                side_effects: outcome.side_effects.clone(),
            },
            patterns_extracted: vec![],
            lessons_learned: vec![],
            success_score,
            replayable: success_score > 0.7,
        };

        let id = experience.id.clone();

        let mut experiences = self.experiences.write().await;
        experiences.push(experience.clone());

        if experiences.len() > 10000 {
            experiences.remove(0);
        }
        drop(experiences);

        self.extract_patterns(&experience).await?;
        self.derive_lessons(&experience).await?;

        self.check_skill_prototype(&experience).await?;

        self.save_state().await?;

        Ok(id)
    }

    fn hash_input(&self, input: &str) -> String {
        let normalized = input
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        md5::format_hex(&md5::compute(normalized.as_bytes()))
    }

    fn hash_arguments(
        &self,
        action: &crate::housaky::cognitive::action_selector::SelectedAction,
    ) -> String {
        match action {
            crate::housaky::cognitive::action_selector::SelectedAction::UseTool {
                arguments,
                ..
            } => md5::format_hex(&md5::compute(arguments.to_string().as_bytes())),
            _ => "no_args".to_string(),
        }
    }

    fn extract_tool_from_action(
        &self,
        action: &crate::housaky::cognitive::action_selector::SelectedAction,
    ) -> Option<String> {
        match action {
            crate::housaky::cognitive::action_selector::SelectedAction::UseTool {
                tool_name,
                ..
            } => Some(tool_name.clone()),
            _ => None,
        }
    }

    async fn extract_patterns(&self, experience: &Experience) -> Result<()> {
        let mut patterns = self.patterns.write().await;

        if let Some(tool) = &experience.action.tool_used {
            let pattern_key = format!("tool_usage_{}_{}", tool, experience.perception.intent);

            let pattern = patterns
                .entry(pattern_key.clone())
                .or_insert_with(|| Pattern {
                    id: format!("pat_{}", uuid::Uuid::new_v4()),
                    pattern_type: PatternType::InputOutput,
                    description: format!(
                        "Use {} for {} requests",
                        tool, experience.perception.intent
                    ),
                    conditions: vec![format!("intent:{}", experience.perception.intent)],
                    actions: vec![format!("use_tool:{}", tool)],
                    confidence: 0.5,
                    occurrences: 0,
                    success_rate: 0.0,
                    first_seen: experience.timestamp,
                    last_seen: experience.timestamp,
                });

            pattern.occurrences += 1;
            pattern.last_seen = experience.timestamp;
            pattern.success_rate = (pattern.success_rate * (pattern.occurrences - 1) as f64
                + experience.success_score)
                / pattern.occurrences as f64;
            pattern.confidence = if pattern.occurrences >= self.min_pattern_occurrences {
                (pattern.success_rate * pattern.occurrences as f64
                    / self.min_pattern_occurrences as f64)
                    .min(1.0)
            } else {
                pattern.occurrences as f64 / self.min_pattern_occurrences as f64
                    * pattern.success_rate
            };
        }

        for topic in &experience.perception.topics {
            let pattern_key = format!("topic_{}_{}", topic, experience.action.action_type);

            let pattern = patterns
                .entry(pattern_key.clone())
                .or_insert_with(|| Pattern {
                    id: format!("pat_{}", uuid::Uuid::new_v4()),
                    pattern_type: PatternType::InputOutput,
                    description: format!(
                        "Handle {} topic with {}",
                        topic, experience.action.action_type
                    ),
                    conditions: vec![format!("topic:{}", topic)],
                    actions: vec![experience.action.action_type.clone()],
                    confidence: 0.5,
                    occurrences: 0,
                    success_rate: 0.0,
                    first_seen: experience.timestamp,
                    last_seen: experience.timestamp,
                });

            pattern.occurrences += 1;
            pattern.last_seen = experience.timestamp;
            pattern.success_rate = (pattern.success_rate * (pattern.occurrences - 1) as f64
                + experience.success_score)
                / pattern.occurrences as f64;
        }

        if !experience.outcome.success {
            let pattern_key = format!("recovery_{}", experience.action.action_type);

            let pattern = patterns
                .entry(pattern_key.clone())
                .or_insert_with(|| Pattern {
                    id: format!("pat_{}", uuid::Uuid::new_v4()),
                    pattern_type: PatternType::Recovery,
                    description: format!("Recovery from {} failure", experience.action.action_type),
                    conditions: vec![format!("failed:{}", experience.action.action_type)],
                    actions: vec![
                        "ask_clarification".to_string(),
                        "try_alternative".to_string(),
                    ],
                    confidence: 0.5,
                    occurrences: 0,
                    success_rate: 0.0,
                    first_seen: experience.timestamp,
                    last_seen: experience.timestamp,
                });

            pattern.occurrences += 1;
            pattern.last_seen = experience.timestamp;
        }

        Ok(())
    }

    async fn derive_lessons(&self, experience: &Experience) -> Result<()> {
        let mut lessons = self.lessons.write().await;

        if experience.success_score > 0.8 {
            let lesson = Lesson {
                id: format!("lesson_{}", uuid::Uuid::new_v4()),
                topic: experience
                    .perception
                    .topics
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "general".to_string()),
                insight: format!(
                    "For {} requests, using {} is effective",
                    experience.perception.intent, experience.action.action_type
                ),
                context: format!("Based on {} successful execution", experience.id),
                applicability: vec![format!("intent:{}", experience.perception.intent)],
                confidence: experience.success_score,
                source_experience: experience.id.clone(),
                validated: false,
                validation_count: 0,
            };
            lessons.push(lesson);
        }

        if !experience.outcome.success {
            let lesson = Lesson {
                id: format!("lesson_{}", uuid::Uuid::new_v4()),
                topic: "error_recovery".to_string(),
                insight: format!(
                    "When {} fails, consider alternative approaches",
                    experience.action.action_type
                ),
                context: format!("Based on {} failure", experience.id),
                applicability: vec![format!("failed:{}", experience.action.action_type)],
                confidence: 0.7,
                source_experience: experience.id.clone(),
                validated: false,
                validation_count: 0,
            };
            lessons.push(lesson);
        }

        if lessons.len() > 1000 {
            lessons.remove(0);
        }

        Ok(())
    }

    async fn check_skill_prototype(&self, _experience: &Experience) -> Result<()> {
        let patterns = self.patterns.read().await;
        let mut prototypes = self.skill_prototypes.write().await;

        let relevant_patterns: Vec<_> = patterns
            .values()
            .filter(|p| {
                p.occurrences >= self.min_pattern_occurrences
                    && p.confidence >= self.pattern_confidence_threshold
            })
            .collect();

        for pattern in relevant_patterns {
            let existing_prototype = prototypes.iter().find(|s| s.patterns.contains(&pattern.id));

            if existing_prototype.is_none() {
                let prototype = SkillPrototype {
                    id: format!("skill_proto_{}", uuid::Uuid::new_v4()),
                    name: self.generate_skill_name(pattern),
                    description: pattern.description.clone(),
                    patterns: vec![pattern.id.clone()],
                    triggers: pattern.conditions.clone(),
                    actions: pattern
                        .actions
                        .iter()
                        .map(|a| ActionTemplate {
                            action_type: a.clone(),
                            tool: None,
                            arguments_template: "{}".to_string(),
                            conditions: vec![],
                        })
                        .collect(),
                    success_rate: pattern.success_rate,
                    usage_count: pattern.occurrences,
                    ready_for_promotion: pattern.success_rate > 0.8 && pattern.occurrences >= 5,
                };

                info!(
                    "Created skill prototype: {} (success rate: {:.0}%)",
                    prototype.name,
                    prototype.success_rate * 100.0
                );

                prototypes.push(prototype);
            }
        }

        if prototypes.len() > 100 {
            prototypes.sort_by(|a, b| b.success_rate.partial_cmp(&a.success_rate).unwrap());
            prototypes.truncate(100);
        }

        Ok(())
    }

    fn generate_skill_name(&self, pattern: &Pattern) -> String {
        let desc_words: Vec<&str> = pattern.description.split_whitespace().take(3).collect();
        desc_words.join("_").to_lowercase().replace(' ', "_")
    }

    pub async fn find_similar_experiences(&self, perception: &PerceivedInput) -> Vec<Experience> {
        let experiences = self.experiences.read().await;

        let mut similar: Vec<(f64, &Experience)> = experiences
            .iter()
            .map(|exp| {
                let mut score = 0.0;

                if exp.perception.intent == format!("{:?}", perception.intent.primary) {
                    score += 0.3;
                }

                let topic_overlap = exp
                    .perception
                    .topics
                    .iter()
                    .filter(|t| perception.topics.contains(t))
                    .count();
                score += topic_overlap as f64 * 0.2;

                score += (1.0 - (exp.perception.complexity - perception.complexity).abs()) * 0.2;

                score += exp.success_score * 0.3;

                (score, exp)
            })
            .collect();

        similar.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        similar
            .into_iter()
            .take(5)
            .map(|(_, exp)| exp.clone())
            .collect()
    }

    pub async fn get_applicable_patterns(&self, perception: &PerceivedInput) -> Vec<Pattern> {
        let patterns = self.patterns.read().await;

        let intent_str = format!("intent:{:?}", perception.intent.primary);
        let topic_conditions: Vec<String> = perception
            .topics
            .iter()
            .map(|t| format!("topic:{}", t))
            .collect();

        patterns
            .values()
            .filter(|p| {
                p.conditions
                    .iter()
                    .any(|c| c == &intent_str || topic_conditions.contains(c))
            })
            .filter(|p| p.confidence >= self.pattern_confidence_threshold)
            .cloned()
            .collect()
    }

    pub async fn get_lessons_for_context(&self, context: &str) -> Vec<Lesson> {
        let lessons = self.lessons.read().await;

        lessons
            .iter()
            .filter(|l| l.applicability.iter().any(|a| context.contains(a)))
            .cloned()
            .collect()
    }

    pub async fn get_ready_skills(&self) -> Vec<SkillPrototype> {
        let prototypes = self.skill_prototypes.read().await;

        prototypes
            .iter()
            .filter(|s| s.ready_for_promotion)
            .cloned()
            .collect()
    }

    pub async fn promote_skill(&self, prototype_id: &str) -> Result<String> {
        let mut prototypes = self.skill_prototypes.write().await;

        if let Some(prototype) = prototypes.iter_mut().find(|s| s.id == prototype_id) {
            let skill_name = prototype.name.clone();
            let skill_dir = self
                .workspace_dir
                .join(".housaky")
                .join("skills")
                .join(&skill_name);

            tokio::fs::create_dir_all(&skill_dir).await?;

            let skill_content = format!(
                r#"# Housaky Skill: {}

{}

## Triggers
{}

## Actions
{}

## Success Rate: {:.0}%

## Usage Count: {}

## Promoted from Experience Learning
This skill was automatically generated from successful patterns observed during interactions.
"#,
                skill_name,
                prototype.description,
                prototype
                    .triggers
                    .iter()
                    .map(|t| format!("- {}", t))
                    .collect::<Vec<_>>()
                    .join("\n"),
                prototype
                    .actions
                    .iter()
                    .map(|a| format!("- {:?}", a))
                    .collect::<Vec<_>>()
                    .join("\n"),
                prototype.success_rate * 100.0,
                prototype.usage_count
            );

            tokio::fs::write(skill_dir.join("SKILL.md"), skill_content).await?;

            info!("Promoted skill: {}", skill_name);

            return Ok(skill_name);
        }

        Err(anyhow::anyhow!(
            "Skill prototype not found: {}",
            prototype_id
        ))
    }

    pub async fn get_learning_stats(&self) -> LearningStats {
        let experiences = self.experiences.read().await;
        let patterns = self.patterns.read().await;
        let lessons = self.lessons.read().await;
        let prototypes = self.skill_prototypes.read().await;

        let successful = experiences.iter().filter(|e| e.outcome.success).count();

        LearningStats {
            total_experiences: experiences.len(),
            successful_experiences: successful,
            patterns_discovered: patterns.len(),
            high_confidence_patterns: patterns.values().filter(|p| p.confidence > 0.8).count(),
            lessons_learned: lessons.len(),
            skill_prototypes: prototypes.len(),
            ready_skills: prototypes.iter().filter(|s| s.ready_for_promotion).count(),
            average_success_rate: if experiences.is_empty() {
                0.0
            } else {
                experiences.iter().map(|e| e.success_score).sum::<f64>() / experiences.len() as f64
            },
        }
    }

    async fn save_state(&self) -> Result<()> {
        let state_dir = self.workspace_dir.join(".housaky").join("learning");
        tokio::fs::create_dir_all(&state_dir).await?;

        let experiences = self.experiences.read().await;
        let recent: Vec<_> = experiences.iter().rev().take(100).cloned().collect();
        write_msgpack_file(&state_dir.join("experiences.msgpack"), &recent).await?;

        let patterns = self.patterns.read().await;
        write_msgpack_file(&state_dir.join("patterns.msgpack"), &*patterns).await?;

        let lessons = self.lessons.read().await;
        write_msgpack_file(&state_dir.join("lessons.msgpack"), &*lessons).await?;

        Ok(())
    }

    pub async fn load_state(&self) -> Result<()> {
        let state_dir = self.workspace_dir.join(".housaky").join("learning");

        if state_dir.exists() {
            let patterns_path = state_dir.join("patterns.msgpack");
            if patterns_path.exists() {
                if let Ok(loaded) = read_msgpack_file::<HashMap<String, Pattern>>(&patterns_path).await {
                    *self.patterns.write().await = loaded;
                }
            }

            let lessons_path = state_dir.join("lessons.msgpack");
            if lessons_path.exists() {
                if let Ok(loaded) = read_msgpack_file::<Vec<Lesson>>(&lessons_path).await {
                    *self.lessons.write().await = loaded;
                }
            }

            info!("Loaded learning state from disk");
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningStats {
    pub total_experiences: usize,
    pub successful_experiences: usize,
    pub patterns_discovered: usize,
    pub high_confidence_patterns: usize,
    pub lessons_learned: usize,
    pub skill_prototypes: usize,
    pub ready_skills: usize,
    pub average_success_rate: f64,
}

mod md5 {
    use ::md5::{Digest, Md5};

    pub fn compute(data: &[u8]) -> [u8; 16] {
        let mut hasher = Md5::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut hash = [0u8; 16];
        hash.copy_from_slice(&result);
        hash
    }

    pub fn format_hex(hash: &[u8; 16]) -> String {
        let mut out = String::with_capacity(hash.len() * 2);
        for b in hash {
            use std::fmt::Write as _;
            write!(out, "{:02x}", b).ok();
        }
        out
    }
}

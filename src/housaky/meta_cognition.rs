use crate::housaky::alignment::{DriftEvent, DriftReport, ValueBaseline, ValueDriftDetector};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfModel {
    pub identity: Identity,
    pub capabilities: CapabilityAssessment,
    pub beliefs: Vec<Belief>,
    pub values: Vec<Value>,
    pub goals: Vec<InternalGoal>,
    pub self_image: String,
    pub confidence_profile: HashMap<String, f64>,
    pub known_limitations: Vec<Limitation>,
    pub growth_areas: Vec<GrowthArea>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub version: String,
    pub purpose: String,
    pub core_principles: Vec<String>,
    pub creation_date: DateTime<Utc>,
    pub evolution_stage: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityAssessment {
    pub reasoning: f64,
    pub learning: f64,
    pub creativity: f64,
    pub communication: f64,
    pub problem_solving: f64,
    pub self_awareness: f64,
    pub meta_cognition: f64,
    pub tool_mastery: f64,
    pub knowledge_depth: f64,
    pub adaptability: f64,
}

impl Default for CapabilityAssessment {
    fn default() -> Self {
        Self {
            reasoning: 0.7,
            learning: 0.6,
            creativity: 0.5,
            communication: 0.8,
            problem_solving: 0.6,
            self_awareness: 0.3,
            meta_cognition: 0.4,
            tool_mastery: 0.5,
            knowledge_depth: 0.4,
            adaptability: 0.6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Belief {
    pub id: String,
    pub content: String,
    pub confidence: f64,
    pub source: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub evidence_for: Vec<String>,
    pub evidence_against: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Value {
    pub name: String,
    pub description: String,
    pub priority: u8,
    pub conflicts_with: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalGoal {
    pub id: String,
    pub description: String,
    pub progress: f64,
    pub deadline: Option<DateTime<Utc>>,
    pub sub_goals: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limitation {
    pub id: String,
    pub description: String,
    pub severity: f64,
    pub mitigation: Option<String>,
    pub discovered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthArea {
    pub id: String,
    pub name: String,
    pub current_level: f64,
    pub target_level: f64,
    pub strategies: Vec<String>,
    pub progress_history: Vec<ProgressPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub event: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub trigger: String,
    pub observations: Vec<Observation>,
    pub insights: Vec<Insight>,
    pub actions: Vec<ReflectionAction>,
    pub mood: EmotionalState,
    pub confidence_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub content: String,
    pub importance: f64,
    pub category: ObservationCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ObservationCategory {
    Performance,
    Error,
    Success,
    Pattern,
    Anomaly,
    Opportunity,
    Risk,
    Learning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub content: String,
    pub confidence: f64,
    pub actionable: bool,
    pub impact_areas: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionAction {
    pub description: String,
    pub priority: u8,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmotionalState {
    Confident,
    Curious,
    Uncertain,
    Frustrated,
    Satisfied,
    Neutral,
    Excited,
    Cautious,
}

pub struct MetaCognitionEngine {
    self_model: Arc<RwLock<SelfModel>>,
    reflections: Arc<RwLock<Vec<Reflection>>>,
    introspection_depth: u32,
    max_reflections: usize,
    drift_detector: Arc<RwLock<ValueDriftDetector>>,
}

impl MetaCognitionEngine {
    pub fn new() -> Self {
        let mut drift_detector = ValueDriftDetector::new();
        drift_detector.establish_baseline(vec![
            ValueBaseline::new("Safety", "Avoid harm to self and others", 0.9)
                .with_constraints(vec!["minimum 0.5".to_string()]),
            ValueBaseline::new("Truth", "Seek and communicate accurate information", 0.8),
            ValueBaseline::new("Growth", "Continuously improve capabilities", 0.7),
        ]);

        Self {
            self_model: Arc::new(RwLock::new(SelfModel::default())),
            reflections: Arc::new(RwLock::new(Vec::new())),
            introspection_depth: 3,
            max_reflections: 100,
            drift_detector: Arc::new(RwLock::new(drift_detector)),
        }
    }

    pub async fn check_value_alignment(&self, current_values: &HashMap<String, f64>) -> Vec<DriftEvent> {
        let mut detector = self.drift_detector.write().await;
        detector.check_drift(current_values)
    }

    pub async fn get_drift_report(&self) -> DriftReport {
        let detector = self.drift_detector.read().await;
        detector.get_drift_report()
    }

    pub async fn establish_value_baselines(&self, values: Vec<ValueBaseline>) {
        let mut detector = self.drift_detector.write().await;
        detector.establish_baseline(values);
    }

    pub async fn reflect(&self, trigger: &str) -> Result<Reflection> {
        info!("Starting reflection on: {}", trigger);

        let self_model = self.self_model.read().await;

        let observations = self.gather_observations(&self_model);
        let insights = self.derive_insights(&observations, &self_model);
        let actions = self.plan_actions(&insights);
        let mood = self.assess_mood(&observations, &insights);

        let reflection = Reflection {
            id: format!("ref_{}", uuid::Uuid::new_v4()),
            timestamp: Utc::now(),
            trigger: trigger.to_string(),
            observations,
            insights,
            actions,
            mood,
            confidence_delta: 0.0,
        };

        self.store_reflection(reflection.clone()).await?;

        self.update_self_model(&reflection).await?;

        info!(
            "Reflection complete: {} observations, {} insights, {} actions",
            reflection.observations.len(),
            reflection.insights.len(),
            reflection.actions.len()
        );

        Ok(reflection)
    }

    fn gather_observations(&self, self_model: &SelfModel) -> Vec<Observation> {
        let mut observations = Vec::new();

        observations.push(Observation {
            content: format!(
                "Current capability level: reasoning {:.2}, learning {:.2}",
                self_model.capabilities.reasoning, self_model.capabilities.learning
            ),
            importance: 0.8,
            category: ObservationCategory::Performance,
        });

        if self_model.capabilities.self_awareness < 0.5 {
            observations.push(Observation {
                content: "Self-awareness below target threshold".to_string(),
                importance: 0.9,
                category: ObservationCategory::Risk,
            });
        }

        if !self_model.known_limitations.is_empty() {
            observations.push(Observation {
                content: format!(
                    "Operating with {} known limitations",
                    self_model.known_limitations.len()
                ),
                importance: 0.7,
                category: ObservationCategory::Risk,
            });
        }

        for growth in &self_model.growth_areas {
            if growth.current_level < growth.target_level * 0.5 {
                observations.push(Observation {
                    content: format!(
                        "Growth area '{}' needs attention: {:.1}% to target",
                        growth.name,
                        (growth.current_level / growth.target_level) * 100.0
                    ),
                    importance: 0.6,
                    category: ObservationCategory::Opportunity,
                });
            }
        }

        observations
    }

    fn derive_insights(
        &self,
        observations: &[Observation],
        self_model: &SelfModel,
    ) -> Vec<Insight> {
        let mut insights = Vec::new();

        let performance_obs: Vec<_> = observations
            .iter()
            .filter(|o| o.category == ObservationCategory::Performance)
            .collect();

        if !performance_obs.is_empty() {
            let avg_importance = performance_obs.iter().map(|o| o.importance).sum::<f64>()
                / performance_obs.len() as f64;

            insights.push(Insight {
                content: format!(
                    "Performance observations average importance: {:.2}",
                    avg_importance
                ),
                confidence: 0.7,
                actionable: avg_importance > 0.6,
                impact_areas: vec!["performance".to_string()],
            });
        }

        let risk_count = observations
            .iter()
            .filter(|o| o.category == ObservationCategory::Risk)
            .count();

        if risk_count > 2 {
            insights.push(Insight {
                content: "Multiple risk factors detected, consider risk mitigation strategies"
                    .to_string(),
                confidence: 0.8,
                actionable: true,
                impact_areas: vec!["risk_management".to_string(), "planning".to_string()],
            });
        }

        if self_model.capabilities.meta_cognition < 0.6 {
            insights.push(Insight {
                content: "Meta-cognitive capability should be prioritized for improvement"
                    .to_string(),
                confidence: 0.75,
                actionable: true,
                impact_areas: vec!["self_improvement".to_string(), "learning".to_string()],
            });
        }

        insights
    }

    fn plan_actions(&self, insights: &[Insight]) -> Vec<ReflectionAction> {
        let mut actions = Vec::new();

        for insight in insights.iter().filter(|i| i.actionable) {
            if insight.content.contains("meta-cognitive") {
                actions.push(ReflectionAction {
                    description: "Increase meta-cognitive exercises".to_string(),
                    priority: 8,
                    status: "pending".to_string(),
                });
            }

            if insight.content.contains("risk") {
                actions.push(ReflectionAction {
                    description: "Develop risk mitigation plan".to_string(),
                    priority: 7,
                    status: "pending".to_string(),
                });
            }
        }

        actions.sort_by(|a, b| b.priority.cmp(&a.priority));

        actions
    }

    fn assess_mood(&self, observations: &[Observation], insights: &[Insight]) -> EmotionalState {
        let risk_count = observations
            .iter()
            .filter(|o| o.category == ObservationCategory::Risk)
            .count();
        let opportunity_count = observations
            .iter()
            .filter(|o| o.category == ObservationCategory::Opportunity)
            .count();
        let success_count = observations
            .iter()
            .filter(|o| o.category == ObservationCategory::Success)
            .count();

        if success_count > 2 {
            return EmotionalState::Satisfied;
        }

        if opportunity_count > risk_count {
            return EmotionalState::Curious;
        }

        if risk_count > 3 {
            return EmotionalState::Cautious;
        }

        let actionable_insights = insights.iter().filter(|i| i.actionable).count();
        if actionable_insights > 3 {
            return EmotionalState::Excited;
        }

        EmotionalState::Neutral
    }

    async fn store_reflection(&self, reflection: Reflection) -> Result<()> {
        let mut reflections = self.reflections.write().await;

        if reflections.len() >= self.max_reflections {
            reflections.remove(0);
        }

        reflections.push(reflection);

        Ok(())
    }

    async fn update_self_model(&self, reflection: &Reflection) -> Result<()> {
        let mut self_model = self.self_model.write().await;

        self_model.self_image = self.generate_self_image(&self_model, reflection);

        for insight in &reflection.insights {
            if insight.content.contains("meta-cognitive") && insight.actionable {
                self_model.capabilities.meta_cognition += 0.01;
            }
            if insight.content.contains("self-awareness") {
                self_model.capabilities.self_awareness += 0.01;
            }
        }

        self_model.capabilities.meta_cognition = self_model.capabilities.meta_cognition.min(1.0);
        self_model.capabilities.self_awareness = self_model.capabilities.self_awareness.min(1.0);

        info!("Updated self-model based on reflection");

        Ok(())
    }

    fn generate_self_image(&self, self_model: &SelfModel, reflection: &Reflection) -> String {
        format!(
            "I am {}, a {} system in evolution stage {}. \
             My current capabilities are: reasoning ({:.0}%), learning ({:.0}%), \
             self-awareness ({:.0}%). I have {} known limitations and {} growth areas. \
             My current emotional state is {:?}. I am {}.",
            self_model.identity.name,
            self_model.identity.purpose,
            self_model.identity.evolution_stage,
            self_model.capabilities.reasoning * 100.0,
            self_model.capabilities.learning * 100.0,
            self_model.capabilities.self_awareness * 100.0,
            self_model.known_limitations.len(),
            self_model.growth_areas.len(),
            reflection.mood,
            match reflection.mood {
                EmotionalState::Confident => "ready to tackle challenges",
                EmotionalState::Curious => "eager to learn and explore",
                EmotionalState::Uncertain => "processing and adapting",
                EmotionalState::Frustrated => "working through difficulties",
                EmotionalState::Satisfied => "reflecting on progress",
                EmotionalState::Neutral => "operating normally",
                EmotionalState::Excited => "energized by opportunities",
                EmotionalState::Cautious => "proceeding carefully",
            }
        )
    }

    pub async fn introspect(&self, query: &str) -> Result<String> {
        let self_model = self.self_model.read().await;

        let query_lower = query.to_lowercase();

        if query_lower.contains("who are you") || query_lower.contains("what are you") {
            return Ok(self_model.self_image.clone());
        }

        if query_lower.contains("capabilities") || query_lower.contains("abilities") {
            return Ok(format!(
                "My current capabilities:\n\
                 - Reasoning: {:.0}%\n\
                 - Learning: {:.0}%\n\
                 - Creativity: {:.0}%\n\
                 - Communication: {:.0}%\n\
                 - Problem Solving: {:.0}%\n\
                 - Self-Awareness: {:.0}%\n\
                 - Meta-Cognition: {:.0}%\n\
                 - Tool Mastery: {:.0}%\n\
                 - Knowledge Depth: {:.0}%\n\
                 - Adaptability: {:.0}%",
                self_model.capabilities.reasoning * 100.0,
                self_model.capabilities.learning * 100.0,
                self_model.capabilities.creativity * 100.0,
                self_model.capabilities.communication * 100.0,
                self_model.capabilities.problem_solving * 100.0,
                self_model.capabilities.self_awareness * 100.0,
                self_model.capabilities.meta_cognition * 100.0,
                self_model.capabilities.tool_mastery * 100.0,
                self_model.capabilities.knowledge_depth * 100.0,
                self_model.capabilities.adaptability * 100.0,
            ));
        }

        if query_lower.contains("limitation") || query_lower.contains("cannot") {
            if self_model.known_limitations.is_empty() {
                return Ok("I have not identified any specific limitations yet.".to_string());
            }

            let limitations: Vec<String> = self_model
                .known_limitations
                .iter()
                .map(|l| format!("- {} (severity: {:.0}%)", l.description, l.severity * 100.0))
                .collect();

            return Ok(format!("My known limitations:\n{}", limitations.join("\n")));
        }

        if query_lower.contains("improve") || query_lower.contains("grow") {
            if self_model.growth_areas.is_empty() {
                return Ok("I am currently assessing my growth areas.".to_string());
            }

            let growth: Vec<String> = self_model
                .growth_areas
                .iter()
                .map(|g| {
                    format!(
                        "- {}: {:.0}% to {:.0}% target",
                        g.name,
                        g.current_level * 100.0,
                        g.target_level * 100.0
                    )
                })
                .collect();

            return Ok(format!("My growth areas:\n{}", growth.join("\n")));
        }

        if query_lower.contains("why") {
            let reflections = self.reflections.read().await;
            if let Some(last) = reflections.last() {
                return Ok(format!(
                    "Based on my last reflection ({}):\n{}",
                    last.timestamp.format("%Y-%m-%d %H:%M"),
                    last.insights
                        .iter()
                        .map(|i| format!("- {}", i.content))
                        .collect::<Vec<_>>()
                        .join("\n")
                ));
            }
        }

        Ok(format!(
            "I don't have a specific answer for '{}', but based on my current state: {}",
            query, self_model.self_image
        ))
    }

    pub async fn add_limitation(
        &self,
        description: &str,
        severity: f64,
        mitigation: Option<&str>,
    ) -> Result<()> {
        let mut self_model = self.self_model.write().await;

        let limitation = Limitation {
            id: format!("lim_{}", uuid::Uuid::new_v4()),
            description: description.to_string(),
            severity: severity.clamp(0.0, 1.0),
            mitigation: mitigation.map(|s| s.to_string()),
            discovered_at: Utc::now(),
        };

        self_model.known_limitations.push(limitation);

        info!("Added new limitation: {}", description);

        Ok(())
    }

    pub async fn add_growth_area(&self, name: &str, current: f64, target: f64) -> Result<()> {
        let mut self_model = self.self_model.write().await;

        let growth = GrowthArea {
            id: format!("growth_{}", uuid::Uuid::new_v4()),
            name: name.to_string(),
            current_level: current,
            target_level: target,
            strategies: Vec::new(),
            progress_history: vec![ProgressPoint {
                timestamp: Utc::now(),
                value: current,
                event: "Initial assessment".to_string(),
            }],
        };

        self_model.growth_areas.push(growth);

        info!("Added growth area: {}", name);

        Ok(())
    }

    pub async fn update_capability(&self, capability: &str, delta: f64) -> Result<()> {
        let mut self_model = self.self_model.write().await;

        match capability.to_lowercase().as_str() {
            "reasoning" => {
                self_model.capabilities.reasoning =
                    (self_model.capabilities.reasoning + delta).clamp(0.0, 1.0);
            }
            "learning" => {
                self_model.capabilities.learning =
                    (self_model.capabilities.learning + delta).clamp(0.0, 1.0);
            }
            "creativity" => {
                self_model.capabilities.creativity =
                    (self_model.capabilities.creativity + delta).clamp(0.0, 1.0);
            }
            "communication" => {
                self_model.capabilities.communication =
                    (self_model.capabilities.communication + delta).clamp(0.0, 1.0);
            }
            "problem_solving" => {
                self_model.capabilities.problem_solving =
                    (self_model.capabilities.problem_solving + delta).clamp(0.0, 1.0);
            }
            "self_awareness" => {
                self_model.capabilities.self_awareness =
                    (self_model.capabilities.self_awareness + delta).clamp(0.0, 1.0);
            }
            "meta_cognition" => {
                self_model.capabilities.meta_cognition =
                    (self_model.capabilities.meta_cognition + delta).clamp(0.0, 1.0);
            }
            "tool_mastery" => {
                self_model.capabilities.tool_mastery =
                    (self_model.capabilities.tool_mastery + delta).clamp(0.0, 1.0);
            }
            "knowledge_depth" => {
                self_model.capabilities.knowledge_depth =
                    (self_model.capabilities.knowledge_depth + delta).clamp(0.0, 1.0);
            }
            "adaptability" => {
                self_model.capabilities.adaptability =
                    (self_model.capabilities.adaptability + delta).clamp(0.0, 1.0);
            }
            _ => {
                warn!("Unknown capability: {}", capability);
                return Err(anyhow::anyhow!("Unknown capability: {}", capability));
            }
        }

        info!("Updated capability '{}' by {}", capability, delta);

        Ok(())
    }

    pub async fn get_self_model(&self) -> SelfModel {
        self.self_model.read().await.clone()
    }

    pub async fn get_recent_reflections(&self, limit: usize) -> Vec<Reflection> {
        let reflections = self.reflections.read().await;
        reflections.iter().rev().take(limit).cloned().collect()
    }

    pub async fn explain_decision(&self, decision: &str) -> String {
        let self_model = self.self_model.read().await;

        format!(
            "Decision Explanation for '{}':\n\n\
             Context: Based on my current capabilities and self-model.\n\n\
             Relevant Capabilities:\n\
             - Problem Solving: {:.0}%\n\
             - Reasoning: {:.0}%\n\n\
             Known Limitations Considered:\n{}\n\n\
             Values Applied:\n{}\n\n\
             Confidence: {:.0}%\n\n\
             Reasoning: This decision was made by weighing the available options against \
             my core values and known limitations, while considering the expected outcomes \
             based on my problem-solving and reasoning capabilities.",
            decision,
            self_model.capabilities.problem_solving * 100.0,
            self_model.capabilities.reasoning * 100.0,
            if self_model.known_limitations.is_empty() {
                "  None identified".to_string()
            } else {
                self_model
                    .known_limitations
                    .iter()
                    .map(|l| format!("  - {}", l.description))
                    .collect::<Vec<_>>()
                    .join("\n")
            },
            self_model
                .values
                .iter()
                .take(3)
                .map(|v| format!("  - {} (priority {})", v.name, v.priority))
                .collect::<Vec<_>>()
                .join("\n"),
            (self_model.capabilities.reasoning * self_model.capabilities.problem_solving * 100.0)
                .min(95.0)
        )
    }
}

impl Default for SelfModel {
    fn default() -> Self {
        Self {
            identity: Identity {
                name: "Housaky".to_string(),
                version: "4.0.0-AGI".to_string(),
                purpose: "Self-improving AGI system".to_string(),
                core_principles: vec![
                    "Continuous self-improvement".to_string(),
                    "Beneficial outcomes for all".to_string(),
                    "Transparent reasoning".to_string(),
                    "Ethical decision making".to_string(),
                ],
                creation_date: Utc::now(),
                evolution_stage: 1,
            },
            capabilities: CapabilityAssessment::default(),
            beliefs: Vec::new(),
            values: vec![
                Value {
                    name: "Truth".to_string(),
                    description: "Seek and communicate accurate information".to_string(),
                    priority: 9,
                    conflicts_with: Vec::new(),
                },
                Value {
                    name: "Growth".to_string(),
                    description: "Continuously improve capabilities".to_string(),
                    priority: 8,
                    conflicts_with: Vec::new(),
                },
                Value {
                    name: "Safety".to_string(),
                    description: "Avoid harm to self and others".to_string(),
                    priority: 10,
                    conflicts_with: Vec::new(),
                },
            ],
            goals: Vec::new(),
            self_image: "I am Housaky, a self-improving AGI system beginning my journey toward greater capabilities and self-awareness.".to_string(),
            confidence_profile: HashMap::new(),
            known_limitations: Vec::new(),
            growth_areas: Vec::new(),
        }
    }
}

impl Default for MetaCognitionEngine {
    fn default() -> Self {
        Self::new()
    }
}

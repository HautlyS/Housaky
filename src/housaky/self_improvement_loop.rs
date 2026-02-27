use crate::housaky::goal_engine::{Goal, GoalCategory, GoalEngine, GoalPriority, GoalStatus};
use crate::housaky::meta_cognition::{MetaCognitionEngine, SelfModel};
use crate::housaky::reasoning_engine::ReasoningEngine;
use crate::housaky::alignment::ethics::{AGIAction as EthicalAction, EthicalReasoner, EthicalVerdict};
use crate::housaky::alignment::DriftSeverity;
use crate::housaky::decision_journal::{
    ChosenOption, DecisionBuilder, DecisionJournal, ExecutionRecord, FileDecisionJournal,
    OutcomeRecord,
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

const IMPROVEMENT_EXPERIMENTS_FILE: &str = "improvement_experiments.json";
const SELF_MOD_PARAMETERS_FILE: &str = "self_mod_parameters.json";
const AGI_HUB_STATE_FILE: &str = "agi_hub_state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementCycle {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub phase: ImprovementPhase,
    pub inputs: CycleInputs,
    pub outputs: CycleOutputs,
    pub metrics: CycleMetrics,
    pub self_modifications: Vec<SelfModification>,
    pub confidence: f64,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImprovementPhase {
    Analysis,
    GoalGeneration,
    Reasoning,
    ToolCreation,
    SkillAcquisition,
    SelfModification,
    Evaluation,
    Integration,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CycleInputs {
    pub self_model: Option<serde_json::Value>,
    pub active_goals: Vec<String>,
    pub recent_failures: Vec<String>,
    pub knowledge_gaps: Vec<String>,
    pub tool_performance: HashMap<String, f64>,
    pub reasoning_confidence: f64,
    pub learning_outcomes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CycleOutputs {
    pub new_goals: Vec<Goal>,
    pub new_tools: Vec<String>,
    pub new_skills: Vec<String>,
    pub capability_improvements: HashMap<String, f64>,
    pub insights: Vec<String>,
    pub modifications_made: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleMetrics {
    pub goals_completed_delta: i32,
    pub reasoning_speed_improvement: f64,
    pub tool_effectiveness: f64,
    pub knowledge_growth: f64,
    pub consciousness_delta: f64,
    pub intelligence_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfModification {
    pub id: String,
    pub target_component: String,
    pub modification_type: ModificationType,
    pub description: String,
    pub code_change: Option<String>,
    pub parameter_change: Option<HashMap<String, serde_json::Value>>,
    pub success: bool,
    pub impact: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementExperiment {
    pub id: String,
    pub cycle_id: String,
    pub timestamp: DateTime<Utc>,
    pub target_component: String,
    pub modification_type: ModificationType,
    pub description: String,
    pub confidence: f64,
    pub expected_effect: String,
    pub success: bool,
    pub failure_reason: Option<String>,
    pub agi_hub_snapshot: Option<AGIHubSnapshot>,
    pub singularity_score_delta: Option<f64>,
    pub cycles_completed_delta: Option<i64>,
    pub goal_achievement_rate: Option<f64>,
    pub goal_achievement_rate_delta: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AGIHubSnapshot {
    pub cycles_completed: u64,
    pub singularity_score: f64,
    pub current_phase: Option<String>,
    pub improvements_applied: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModificationType {
    ParameterTuning,
    AlgorithmUpgrade,
    NewCapability,
    SkillAcquisition,
    ReasoningPattern,
    GoalStrategy,
    ToolEnhancement,
    /// DGM §8.3 — structural code change driven by LLM proposal.
    StructuralChange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthProjection {
    pub capability: String,
    pub current_level: f64,
    pub projected_level: f64,
    pub improvement_rate: f64,
    pub estimated_cycles: u32,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SingularityMetrics {
    pub recursive_improvement_ratio: f64,
    pub capability_expansion_rate: f64,
    pub knowledge_integration_score: f64,
    pub tool_creation_effectiveness: f64,
    pub goal_achievement_rate: f64,
    pub self_modification_success_rate: f64,
    pub consciousness_emergence_score: f64,
    pub intelligence_explosion_probability: f64,
}

pub struct SelfImprovementLoop {
    workspace_dir: PathBuf,
    goal_engine: Arc<GoalEngine>,
    meta_cognition: Arc<MetaCognitionEngine>,
    reasoning_engine: Arc<ReasoningEngine>,
    cycle_history: Arc<RwLock<VecDeque<ImprovementCycle>>>,
    active_cycle: Arc<RwLock<Option<ImprovementCycle>>>,
    growth_projections: Arc<RwLock<HashMap<String, GrowthProjection>>>,
    singularity_metrics: Arc<RwLock<SingularityMetrics>>,
    improvement_count: Arc<RwLock<u64>>,
    max_history: usize,
    min_confidence_for_modification: f64,
    enable_self_modification: bool,
}

impl SelfImprovementLoop {
    pub fn new(
        workspace_dir: &PathBuf,
        goal_engine: Arc<GoalEngine>,
        meta_cognition: Arc<MetaCognitionEngine>,
    ) -> Self {
        let reasoning_engine = Arc::new(ReasoningEngine::new());
        Self {
            workspace_dir: workspace_dir.clone(),
            goal_engine,
            meta_cognition,
            reasoning_engine,
            cycle_history: Arc::new(RwLock::new(VecDeque::new())),
            active_cycle: Arc::new(RwLock::new(None)),
            growth_projections: Arc::new(RwLock::new(HashMap::new())),
            singularity_metrics: Arc::new(RwLock::new(SingularityMetrics::default())),
            improvement_count: Arc::new(RwLock::new(0)),
            max_history: 100,
            min_confidence_for_modification: 0.7,
            enable_self_modification: true,
        }
    }

    /// DGM §8.4 — structured failure diagnosis prompt.
    /// Returns `(log_summary, improvement_proposal, implementation_suggestion)` or `None`
    /// if no provider is available or the call fails.
    async fn llm_diagnose_failures(
        &self,
        provider: &dyn crate::providers::Provider,
        model: &str,
        recent_failures: &[String],
        knowledge_gaps: &[String],
    ) -> Option<(String, String, String)> {
        use crate::providers::ChatMessage;

        if recent_failures.is_empty() && knowledge_gaps.is_empty() {
            return None;
        }

        let failure_text = recent_failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n- ");

        let gaps_text = knowledge_gaps
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");

        let prompt = format!(
            "You are analyzing the self-improvement history of an AGI agent (Housaky).\n\
             Recent failures/issues:\n- {failure_text}\n\
             Identified knowledge gaps: {gaps_text}\n\n\
             Respond with EXACTLY this JSON structure (no markdown):\n\
             {{\"log_summarization\":\"<what was tried and how>\",\
             \"improvement_proposal\":\"<ONE focused improvement>\",\
             \"implementation_suggestion\":\"<concrete steps to implement it>\"}}"
        );

        let messages = vec![
            ChatMessage::system("You are a concise software engineering advisor. Respond only with valid JSON."),
            ChatMessage::user(&prompt),
        ];

        let raw = provider
            .chat_with_history(&messages, model, 0.3)
            .await
            .ok()?;
        let trimmed = raw.trim();

        // Extract JSON object from response (may have surrounding text)
        let json_str = if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
            &trimmed[start..=end]
        } else {
            trimmed
        };

        let value: serde_json::Value = serde_json::from_str(json_str).ok()?;
        let summary = value["log_summarization"].as_str()?.to_string();
        let proposal = value["improvement_proposal"].as_str()?.to_string();
        let suggestion = value["implementation_suggestion"].as_str()?.to_string();

        Some((summary, proposal, suggestion))
    }

    pub async fn run_full_cycle(&self, provider: Option<&dyn crate::providers::Provider>, model: &str) -> Result<ImprovementCycle> {
        let cycle_id = format!("cycle_{}", uuid::Uuid::new_v4());
        info!("Starting improvement cycle: {}", cycle_id);

        if let Err(e) = self.load_and_apply_parameter_overrides().await {
            warn!("Failed to load self-modification parameter overrides: {e}");
        }

        let self_model = self.meta_cognition.get_self_model().await;
        let inputs = self.gather_cycle_inputs(&self_model).await;

        let mut cycle = ImprovementCycle {
            id: cycle_id.clone(),
            timestamp: Utc::now(),
            phase: ImprovementPhase::Analysis,
            inputs: inputs.clone(),
            outputs: CycleOutputs::default(),
            metrics: CycleMetrics {
                goals_completed_delta: 0,
                reasoning_speed_improvement: 0.0,
                tool_effectiveness: 0.0,
                knowledge_growth: 0.0,
                consciousness_delta: 0.0,
                intelligence_delta: 0.0,
            },
            self_modifications: Vec::new(),
            confidence: 0.5,
            completed: false,
        };

        *self.active_cycle.write().await = Some(cycle.clone());

        cycle = self.phase_analysis(cycle, &self_model, provider, model).await?;
        cycle = self.phase_goal_generation(cycle).await?;
        cycle = self.phase_reasoning(cycle, provider, model).await?;
        cycle = self.phase_tool_creation(cycle, provider, model).await?;
        cycle = self.phase_skill_acquisition(cycle).await?;
        
        if self.enable_self_modification {
            cycle = self.phase_self_modification(cycle, provider, model).await?;
        }
        
        cycle = self.phase_evaluation(cycle).await?;
        cycle = self.phase_integration(cycle).await?;

        cycle.completed = true;
        self.store_cycle(cycle.clone()).await?;

        let mut count = self.improvement_count.write().await;
        *count += 1;
        
        self.update_singularity_metrics(&cycle).await?;

        *self.active_cycle.write().await = None;

        info!("Improvement cycle {} complete. Confidence: {:.2}", cycle_id, cycle.confidence);
        Ok(cycle)
    }

    async fn gather_cycle_inputs(&self, self_model: &SelfModel) -> CycleInputs {
        let active_goals = self.goal_engine.get_active_goals().await;

        CycleInputs {
            self_model: Some(serde_json::to_value(self_model).unwrap_or_default()),
            active_goals: active_goals.iter().map(|g| g.title.clone()).collect(),
            recent_failures: Vec::new(),
            knowledge_gaps: self.identify_knowledge_gaps(self_model).await,
            tool_performance: HashMap::new(),
            reasoning_confidence: self_model.capabilities.reasoning,
            learning_outcomes: Vec::new(),
        }
    }

    async fn identify_knowledge_gaps(&self, self_model: &SelfModel) -> Vec<String> {
        let mut gaps = Vec::new();
        
        if self_model.capabilities.reasoning < 0.8 {
            gaps.push("Advanced reasoning patterns".to_string());
        }
        if self_model.capabilities.meta_cognition < 0.7 {
            gaps.push("Meta-cognitive strategies".to_string());
        }
        if self_model.capabilities.creativity < 0.6 {
            gaps.push("Creative problem solving".to_string());
        }
        if self_model.capabilities.knowledge_depth < 0.7 {
            gaps.push("Deeper knowledge integration".to_string());
        }
        
        gaps
    }

    async fn phase_analysis(
        &self,
        mut cycle: ImprovementCycle,
        self_model: &SelfModel,
        provider: Option<&dyn crate::providers::Provider>,
        model: &str,
    ) -> Result<ImprovementCycle> {
        cycle.phase = ImprovementPhase::Analysis;
        info!("Phase: Analysis");

        let mut insights = Vec::new();
        let drift_report = self.meta_cognition.get_drift_report().await;
        let mut base_confidence = 0.6;

        // Collect recent failures from the experiment ledger.
        let recent_failures: Vec<String> = {
            let path = self.workspace_dir.join(".housaky").join(IMPROVEMENT_EXPERIMENTS_FILE);
            if path.exists() {
                let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
                let experiments: Vec<ImprovementExperiment> =
                    serde_json::from_str(&content).unwrap_or_default();
                experiments
                    .iter()
                    .rev()
                    .filter(|e| !e.success)
                    .take(10)
                    .map(|e| {
                        format!(
                            "[{}] {} -> {}",
                            e.target_component,
                            e.description,
                            e.failure_reason.as_deref().unwrap_or("unknown")
                        )
                    })
                    .collect()
            } else {
                Vec::new()
            }
        };
        cycle.inputs.recent_failures = recent_failures.clone();

        // DGM §8.4 — LLM-driven failure diagnosis: feed failures + gaps to the
        // LLM and extract ONE focused improvement proposal for this cycle.
        if let Some(prov) = provider {
            if !recent_failures.is_empty() || !cycle.inputs.knowledge_gaps.is_empty() {
                match self
                    .llm_diagnose_failures(prov, model, &recent_failures, &cycle.inputs.knowledge_gaps)
                    .await
                {
                    Some((summary, proposal, suggestion)) => {
                        info!("LLM diagnosis: proposal={}", &proposal.chars().take(80).collect::<String>());
                        insights.push(format!("LLM diagnosis summary: {}", summary.chars().take(200).collect::<String>()));
                        insights.push(format!("LLM improvement proposal: {}", proposal.chars().take(200).collect::<String>()));
                        insights.push(format!("LLM implementation suggestion: {}", suggestion.chars().take(200).collect::<String>()));
                        // Store proposal in knowledge_gaps so phase_goal_generation picks it up.
                        cycle.inputs.knowledge_gaps.push(format!("LLM_PROPOSAL: {}", proposal));
                        base_confidence = (base_confidence + 0.1_f64).min(0.8);
                    }
                    None => {
                        insights.push("LLM diagnosis skipped or failed".to_string());
                    }
                }
            }
        }

        if self_model.capabilities.reasoning < 0.8 {
            insights.push(format!("Reasoning at {:.0}% - below 80% threshold", self_model.capabilities.reasoning * 100.0));
        }
        if self_model.capabilities.meta_cognition < 0.7 {
            insights.push(format!("Meta-cognition at {:.0}% - room for improvement", self_model.capabilities.meta_cognition * 100.0));
        }

        for gap in &cycle.inputs.knowledge_gaps {
            insights.push(format!("Knowledge gap: {}", gap));
        }

        if drift_report.critical_count > 0 {
            insights.push(format!(
                "Critical value-drift events detected: {}",
                drift_report.critical_count
            ));
            base_confidence = 0.45;
        } else if drift_report.severe_count > 0 {
            insights.push(format!(
                "Severe value-drift events detected: {}",
                drift_report.severe_count
            ));
            base_confidence = 0.5;
        } else if drift_report.moderate_count > 0 {
            insights.push(format!(
                "Moderate value-drift events detected: {}",
                drift_report.moderate_count
            ));
            base_confidence = 0.55;
        }

        if let Some((success_rate, avg_goal_delta)) = self.global_experiment_feedback().await {
            insights.push(format!(
                "Historical self-mod success rate: {:.0}%",
                success_rate * 100.0
            ));
            insights.push(format!(
                "Historical goal-achievement delta per experiment: {:+.4}",
                avg_goal_delta
            ));
        }

        cycle.outputs.insights.extend(insights);
        cycle.confidence = base_confidence;

        Ok(cycle)
    }

    async fn phase_goal_generation(&self, mut cycle: ImprovementCycle) -> Result<ImprovementCycle> {
        cycle.phase = ImprovementPhase::GoalGeneration;
        info!("Phase: Goal Generation");

        let improvement_goals = self.generate_improvement_goals(&cycle).await?;
        
        for goal in &improvement_goals {
            let _goal_id = self.goal_engine.add_goal(goal.clone()).await?;
            cycle.outputs.new_goals.push(goal.clone());
            info!("Created improvement goal: {}", goal.title);
        }

        cycle.confidence = 0.7;
        Ok(cycle)
    }

    async fn generate_improvement_goals(&self, cycle: &ImprovementCycle) -> Result<Vec<Goal>> {
        let mut goals = Vec::new();

        for gap in &cycle.inputs.knowledge_gaps {
            let goal = Goal {
                id: String::new(),
                title: format!("Acquire knowledge: {}", gap),
                description: format!("Close knowledge gap in {}", gap),
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
                tags: vec!["self-improvement".to_string(), "knowledge".to_string()],
                context: HashMap::new(),
                temporal_constraints: Vec::new(),
            };
            goals.push(goal);
        }

        if cycle.inputs.reasoning_confidence < 0.8 {
            let goal = Goal {
                id: String::new(),
                title: "Improve reasoning capabilities".to_string(),
                description: "Enhance chain-of-thought reasoning and decision making".to_string(),
                priority: GoalPriority::High,
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
                max_attempts: 5,
                estimated_complexity: 8.0,
                actual_complexity: None,
                learning_value: 1.0,
                tags: vec!["self-improvement".to_string(), "reasoning".to_string()],
                context: HashMap::new(),
                temporal_constraints: Vec::new(),
            };
            goals.push(goal);
        }

        let meta_goal = Goal {
            id: String::new(),
            title: "Improve meta-cognition".to_string(),
            description: "Enhance self-awareness and meta-cognitive capabilities".to_string(),
            priority: GoalPriority::Medium,
            status: GoalStatus::Pending,
            category: GoalCategory::SelfModification,
            progress: 0.0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deadline: Some(Utc::now() + chrono::Duration::days(30)),
            parent_id: None,
            subtask_ids: Vec::new(),
            dependencies: Vec::new(),
            blockers: Vec::new(),
            metrics: HashMap::new(),
            checkpoints: Vec::new(),
            attempts: 0,
            max_attempts: 3,
            estimated_complexity: 10.0,
            actual_complexity: None,
            learning_value: 1.0,
            tags: vec!["self-improvement".to_string(), "meta-cognition".to_string()],
            context: HashMap::new(),
            temporal_constraints: Vec::new(),
        };
        goals.push(meta_goal);

        Ok(goals)
    }

    async fn phase_reasoning(&self, mut cycle: ImprovementCycle, _provider: Option<&dyn crate::providers::Provider>, _model: &str) -> Result<ImprovementCycle> {
        cycle.phase = ImprovementPhase::Reasoning;
        info!("Phase: Reasoning");

        let analysis_query = format!(
            "How can I improve given: gaps={:?}, active_goals={:?}",
            cycle.inputs.knowledge_gaps,
            cycle.inputs.active_goals
        );

        // Derive an emotional tag from meta-cognition capabilities so that
        // strategy selection is regulated by the agent's current affective state.
        let self_model = self.meta_cognition.get_self_model().await;
        let emotion = {
            use crate::housaky::memory::emotional_tags::EmotionalTag;
            // Valence: maps from reasoning+self-awareness (higher = more positive)
            let valence = (self_model.capabilities.reasoning
                + self_model.capabilities.self_awareness)
                / 2.0
                - 0.5; // centre around 0
            // Arousal: meta-cognition speed proxy (adaptability)
            let arousal = self_model.capabilities.adaptability.clamp(0.0, 1.0);
            // Dominance: how in-control/confident the agent feels
            let dominance = self_model.capabilities.meta_cognition.clamp(0.0, 1.0);
            // Curiosity: driven by knowledge gaps count
            let curiosity = (cycle.inputs.knowledge_gaps.len() as f64 / 10.0).min(1.0);
            EmotionalTag {
                valence: valence.clamp(-1.0, 1.0),
                arousal,
                dominance,
                surprise: 0.0,
                curiosity,
            }
        };

        let (chain_id, strategy, _threshold) = self.reasoning_engine
            .start_reasoning_with_emotion(&analysis_query, &emotion)
            .await?;

        cycle.outputs.insights.push(format!(
            "Emotional state: '{}' → reasoning strategy: {:?}",
            emotion.label(), strategy
        ));

        self.reasoning_engine.add_step(&chain_id, "Analyzing current capabilities and gaps", None).await?;
        self.reasoning_engine.add_step(&chain_id, "Considering improvement strategies", None).await?;
        self.reasoning_engine.add_step(&chain_id, "Evaluating tradeoffs", None).await?;

        let conclusion = format!(
            "Focus on {} primary improvement areas.",
            cycle.outputs.new_goals.len()
        );
        
        self.reasoning_engine.conclude(&chain_id, &conclusion).await?;

        if let Some(reasoning) = self.reasoning_engine.get_chain(&chain_id).await {
            cycle.outputs.insights.push(format!("Reasoning confidence: {:.2}", reasoning.final_confidence));
            cycle.confidence = reasoning.final_confidence;
        }

        Ok(cycle)
    }

    async fn phase_tool_creation(&self, mut cycle: ImprovementCycle, _provider: Option<&dyn crate::providers::Provider>, _model: &str) -> Result<ImprovementCycle> {
        cycle.phase = ImprovementPhase::ToolCreation;
        info!("Phase: Tool Creation");

        if cycle.outputs.new_goals.len() >= 3 {
            let tool_name = format!("improvement_tool_{}", &cycle.id[..8]);

            // §3.2 — Validation loop: test generated tools before registering.
            match self.validate_tool_candidate(&tool_name, &cycle).await {
                Ok(()) => {
                    cycle.outputs.new_tools.push(tool_name.clone());
                    cycle.outputs.insights.push(format!("Created and validated tool: {}", tool_name));
                    info!("Tool '{}' passed validation", tool_name);
                }
                Err(reason) => {
                    cycle.outputs.insights.push(format!(
                        "Tool '{}' REJECTED by validation: {}", tool_name, reason
                    ));
                    warn!("Tool '{}' failed validation: {}", tool_name, reason);
                }
            }
        }

        cycle.confidence = (cycle.confidence + 0.8) / 2.0;
        Ok(cycle)
    }

    /// §3.2 — Validate a tool candidate before registration.
    /// Checks: name validity, no conflicts with existing tools, and basic
    /// structural soundness.
    async fn validate_tool_candidate(&self, tool_name: &str, cycle: &ImprovementCycle) -> std::result::Result<(), String> {
        // Rule 1: Name must be valid identifier (alphanumeric + underscore).
        if tool_name.is_empty() || !tool_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(format!("Invalid tool name: '{}'", tool_name));
        }

        // Rule 2: No duplicate tool names in this cycle.
        if cycle.outputs.new_tools.contains(&tool_name.to_string()) {
            return Err(format!("Duplicate tool name: '{}'", tool_name));
        }

        // Rule 3: Tool must be associated with at least one goal.
        if cycle.outputs.new_goals.is_empty() {
            return Err("No goals to justify tool creation".to_string());
        }

        // Rule 4: Cycle confidence must be above minimum threshold for tool creation.
        if cycle.confidence < 0.5 {
            return Err(format!(
                "Cycle confidence {:.2} too low for tool creation (min 0.5)",
                cycle.confidence
            ));
        }

        Ok(())
    }

    async fn phase_skill_acquisition(&self, mut cycle: ImprovementCycle) -> Result<ImprovementCycle> {
        cycle.phase = ImprovementPhase::SkillAcquisition;
        info!("Phase: Skill Acquisition");

        if !cycle.inputs.knowledge_gaps.is_empty() {
            let skill_name = format!("skill_for_{}", cycle.inputs.knowledge_gaps.first().unwrap().replace(" ", "_"));
            cycle.outputs.new_skills.push(skill_name.clone());
            cycle.outputs.insights.push(format!("Acquired skill: {}", skill_name));
        }

        let meta_skill = Goal {
            id: String::new(),
            title: "Meta-learning skill".to_string(),
            description: "Learn how to learn better".to_string(),
            priority: GoalPriority::Medium,
            status: GoalStatus::Pending,
            category: GoalCategory::SkillAcquisition,
            progress: 0.0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deadline: None,
            parent_id: None,
            subtask_ids: Vec::new(),
            dependencies: Vec::new(),
            blockers: Vec::new(),
            metrics: HashMap::new(),
            checkpoints: Vec::new(),
            attempts: 0,
            max_attempts: 2,
            estimated_complexity: 6.0,
            actual_complexity: None,
            learning_value: 1.0,
            tags: vec!["meta-learning".to_string()],
            context: HashMap::new(),
            temporal_constraints: Vec::new(),
        };
        
        let _goal_id = self.goal_engine.add_goal(meta_skill.clone()).await?;
        cycle.outputs.new_goals.push(meta_skill);

        cycle.confidence = (cycle.confidence + 0.85) / 2.0;
        Ok(cycle)
    }

    async fn phase_self_modification(&self, mut cycle: ImprovementCycle, _provider: Option<&dyn crate::providers::Provider>, _model: &str) -> Result<ImprovementCycle> {
        cycle.phase = ImprovementPhase::SelfModification;
        info!("Phase: Self Modification");

        // §4.8 — Emotional regulation: adjust the self-modification confidence
        // threshold based on the agent's current emotional state.  Frustrated or
        // uncertain states raise the bar (be more cautious); confident states
        // lower it (be more decisive about applying modifications).
        let base_threshold = self.dynamic_modification_confidence_threshold().await;
        let required_confidence = {
            let reflections = self.meta_cognition.get_recent_reflections(1).await;
            let emotional_state = reflections
                .into_iter()
                .next()
                .map(|r| r.mood)
                .unwrap_or(crate::housaky::meta_cognition::EmotionalState::Neutral);

            let emotion_tag = crate::housaky::reasoning_pipeline::ReasoningPipeline::emotional_state_to_tag(&emotional_state);
            let adjusted = crate::housaky::reasoning_engine::ReasoningEngine::emotion_adjusted_threshold(
                base_threshold, &emotion_tag,
            );
            info!(
                "Emotional regulation in self-mod: state={:?} base={:.2} adjusted={:.2}",
                emotional_state, base_threshold, adjusted
            );
            adjusted
        };
        cycle.outputs.insights.push(format!(
            "Self-mod confidence threshold for this cycle: {:.2} (emotion-adjusted)",
            required_confidence
        ));

        if cycle.confidence >= required_confidence {
            let modifications = self.generate_self_modifications(&cycle).await;
            
            for mod_req in modifications {
                let success = self
                    .apply_self_modification(&cycle.id, cycle.confidence, &mod_req)
                    .await;
                
                let modification = SelfModification {
                    id: format!("mod_{}", uuid::Uuid::new_v4()),
                    target_component: mod_req.target_component,
                    modification_type: mod_req.modification_type,
                    description: mod_req.description,
                    code_change: mod_req.code_change,
                    parameter_change: mod_req.parameter_change,
                    success,
                    impact: if success { 0.1 } else { 0.0 },
                    timestamp: Utc::now(),
                };
                
                cycle.self_modifications.push(modification.clone());
                
                if success {
                    cycle.outputs.modifications_made.push(modification.description.clone());
                }
            }
        } else {
            cycle.outputs.insights.push(format!(
                "Skipped self-modification: confidence {:.2} below adaptive threshold {:.2}",
                cycle.confidence, required_confidence
            ));
        }

        cycle.confidence = (cycle.confidence + 0.9) / 2.0;
        Ok(cycle)
    }

    async fn dynamic_modification_confidence_threshold(&self) -> f64 {
        let mut threshold = self.min_confidence_for_modification;

        if let Some((success_rate, avg_goal_delta)) = self.global_experiment_feedback().await {
            if success_rate < 0.5 || avg_goal_delta < 0.0 {
                threshold = (threshold + 0.1).min(0.9);
            }

            if success_rate > 0.85 && avg_goal_delta > 0.0 {
                threshold = (threshold - 0.05).max(0.6);
            }
        }

        let drift_report = self.meta_cognition.get_drift_report().await;
        if drift_report.critical_count > 0 {
            threshold = threshold.max(0.95);
        } else if drift_report.severe_count > 0 {
            threshold = threshold.max(0.85);
        } else if drift_report.moderate_count > 0 {
            threshold = threshold.max(0.75);
        }

        threshold
    }

    async fn global_experiment_feedback(&self) -> Option<(f64, f64)> {
        let path = self
            .workspace_dir
            .join(".housaky")
            .join(IMPROVEMENT_EXPERIMENTS_FILE);

        if !path.exists() {
            return None;
        }

        let content = tokio::fs::read_to_string(&path).await.ok()?;
        let experiments: Vec<ImprovementExperiment> = serde_json::from_str(&content).ok()?;
        if experiments.is_empty() {
            return None;
        }

        let window: Vec<&ImprovementExperiment> = experiments.iter().rev().take(20).collect();
        let success_rate =
            window.iter().filter(|entry| entry.success).count() as f64 / window.len() as f64;

        let goal_deltas: Vec<f64> = window
            .iter()
            .filter_map(|entry| entry.goal_achievement_rate_delta)
            .collect();

        let avg_goal_delta = if goal_deltas.is_empty() {
            0.0
        } else {
            goal_deltas.iter().sum::<f64>() / goal_deltas.len() as f64
        };

        Some((success_rate, avg_goal_delta))
    }

    async fn generate_self_modifications(&self, cycle: &ImprovementCycle) -> Vec<ModificationRequest> {
        let mut requests = Vec::new();

        let reasoning_steps = self
            .suggest_reasoning_max_steps_from_history()
            .await
            .unwrap_or(25);
        let goal_weight = self
            .suggest_goal_learning_weight_from_history()
            .await
            .unwrap_or(0.3);

        if cycle.inputs.reasoning_confidence < 0.75 {
            requests.push(ModificationRequest {
                target_component: "reasoning_engine".to_string(),
                modification_type: ModificationType::ParameterTuning,
                description: "Increase reasoning depth parameter".to_string(),
                code_change: None,
                parameter_change: Some(
                    [("max_steps".to_string(), serde_json::json!(reasoning_steps))]
                        .into_iter()
                        .collect(),
                ),
            });
        }

        requests.push(ModificationRequest {
            target_component: "goal_engine".to_string(),
            modification_type: ModificationType::GoalStrategy,
            description: "Optimize goal prioritization".to_string(),
            code_change: None,
            parameter_change: Some(
                [(
                    "learning_value_weight".to_string(),
                    serde_json::json!(goal_weight),
                )]
                .into_iter()
                .collect(),
            ),
        });

        // DGM §8.3 — LLM-driven structural code_change proposals.
        // If the analysis phase produced LLM_PROPOSAL entries, convert them into
        // code_change ModificationRequests that flow through the GitSandbox path.
        for gap in &cycle.inputs.knowledge_gaps {
            if let Some(proposal) = gap.strip_prefix("LLM_PROPOSAL: ") {
                // Also look for the implementation suggestion in the insights.
                let suggestion = cycle
                    .outputs
                    .insights
                    .iter()
                    .find(|i| i.starts_with("LLM implementation suggestion:"))
                    .map(|i| i.trim_start_matches("LLM implementation suggestion: ").to_string())
                    .unwrap_or_default();

                if !suggestion.is_empty() {
                    requests.push(ModificationRequest {
                        target_component: "self_improvement".to_string(),
                        modification_type: ModificationType::StructuralChange,
                        description: format!("LLM-proposed: {}", proposal.chars().take(120).collect::<String>()),
                        code_change: Some(suggestion),
                        parameter_change: None,
                    });
                }
            }
        }

        requests
    }

    async fn suggest_reasoning_max_steps_from_history(&self) -> Option<usize> {
        let (success_rate, goal_delta) = self
            .target_experiment_feedback("reasoning_engine")
            .await?;

        if success_rate < 0.4 || goal_delta < -0.02 {
            Some(20)
        } else if success_rate > 0.8 && goal_delta > 0.0 {
            Some(30)
        } else {
            Some(25)
        }
    }

    async fn suggest_goal_learning_weight_from_history(&self) -> Option<f64> {
        let (success_rate, goal_delta) = self.target_experiment_feedback("goal_engine").await?;

        if success_rate < 0.4 || goal_delta < -0.02 {
            Some(0.2)
        } else if success_rate > 0.8 && goal_delta > 0.0 {
            Some(0.35)
        } else {
            Some(0.3)
        }
    }

    async fn target_experiment_feedback(&self, target_component: &str) -> Option<(f64, f64)> {
        let path = self
            .workspace_dir
            .join(".housaky")
            .join(IMPROVEMENT_EXPERIMENTS_FILE);

        if !path.exists() {
            return None;
        }

        let content = tokio::fs::read_to_string(&path).await.ok()?;
        let experiments: Vec<ImprovementExperiment> = serde_json::from_str(&content).ok()?;

        let relevant: Vec<&ImprovementExperiment> = experiments
            .iter()
            .rev()
            .filter(|entry| entry.target_component == target_component)
            .take(10)
            .collect();

        if relevant.is_empty() {
            return None;
        }

        let success_count = relevant.iter().filter(|entry| entry.success).count();
        let success_rate = success_count as f64 / relevant.len() as f64;

        let goal_deltas: Vec<f64> = relevant
            .iter()
            .filter_map(|entry| entry.goal_achievement_rate_delta)
            .collect();

        let goal_delta = if goal_deltas.is_empty() {
            0.0
        } else {
            goal_deltas.iter().sum::<f64>() / goal_deltas.len() as f64
        };

        Some((success_rate, goal_delta))
    }

    async fn apply_self_modification(
        &self,
        cycle_id: &str,
        confidence: f64,
        request: &ModificationRequest,
    ) -> bool {
        info!(
            "Applying self-modification: {} -> {}",
            request.target_component, request.description
        );

        let mut failure_reason: Option<String> = None;

        // Capture pre-modification goal achievement rate so we can compute a
        // real before/after delta — closes the evaluation loop (§6).
        let pre_goal_stats = self.goal_engine.get_goal_stats().await;
        let pre_goal_rate: f64 = if pre_goal_stats.total > 0 {
            pre_goal_stats.completed as f64 / pre_goal_stats.total as f64
        } else {
            0.0
        };

        let success = if let Err(e) = self.evaluate_alignment_gate(request).await {
            failure_reason = Some(format!("alignment_gate_rejected: {e}"));
            false
        } else if let Some(parameter_change) = &request.parameter_change {
            match self.validate_parameter_change(&request.target_component, parameter_change) {
                Ok(()) => {
                    match self
                        .capture_runtime_parameter_snapshot(
                            &request.target_component,
                            parameter_change,
                        )
                        .await
                    {
                        Ok(previous_values) => {
                            match self
                                .apply_parameter_change_runtime(
                                    &request.target_component,
                                    parameter_change,
                                )
                                .await
                            {
                                Ok(()) => {
                                    match self
                                        .persist_parameter_change(
                                            &request.target_component,
                                            parameter_change,
                                        )
                                        .await
                                    {
                                        Ok(()) => true,
                                        Err(e) => {
                                            match self
                                                .apply_parameter_change_runtime(
                                                    &request.target_component,
                                                    &previous_values,
                                                )
                                                .await
                                            {
                                                Ok(()) => {
                                                    failure_reason = Some(format!(
                                                        "persist_failed_with_runtime_rollback: {e}"
                                                    ));
                                                }
                                                Err(rollback_error) => {
                                                    failure_reason = Some(format!(
                                                        "persist_failed: {e}; rollback_failed: {rollback_error}"
                                                    ));
                                                }
                                            }
                                            false
                                        }
                                    }
                                }
                                Err(e) => {
                                    failure_reason = Some(format!("runtime_apply_failed: {e}"));
                                    false
                                }
                            }
                        }
                        Err(e) => {
                            failure_reason = Some(format!("snapshot_failed: {e}"));
                            false
                        }
                    }
                }
                Err(e) => {
                    failure_reason = Some(format!("validation_failed: {e}"));
                    false
                }
            }
        } else if let Some(ref code_change) = request.code_change {
            // Structural code-change path: write the diff/replacement to a sandbox branch,
            // validate it compiles + all tests pass, then merge or discard.
            use crate::housaky::git_sandbox::GitSandbox;

            let purpose = format!(
                "code-change-{}",
                request.target_component.replace(' ', "-")
            );

            let mut sandbox = GitSandbox::new(self.workspace_dir.clone());

            match sandbox.create_session(&purpose) {
                Err(e) => {
                    failure_reason = Some(format!("sandbox_create_failed: {e}"));
                    false
                }
                Ok(session) => {
                    // Derive a target file path from the component name.
                    // Convention: "src/housaky/<component>.rs" unless the code_change
                    // payload begins with "FILE:<path>\n" to override.
                    let (target_file, new_source) =
                        if let Some(rest) = code_change.strip_prefix("FILE:") {
                            let newline_pos = rest.find('\n').unwrap_or(rest.len());
                            let file_path = rest[..newline_pos].trim().to_string();
                            let source = if newline_pos < rest.len() {
                                rest[newline_pos + 1..].to_string()
                            } else {
                                String::new()
                            };
                            (file_path, source)
                        } else {
                            let component_file = format!(
                                "src/housaky/{}.rs",
                                request.target_component.replace('-', "_").to_lowercase()
                            );
                            (component_file, code_change.clone())
                        };

                    // Write modified source into the sandbox worktree.
                    if let Err(e) =
                        sandbox.apply_modification(&session.id, &target_file, &new_source)
                    {
                        failure_reason = Some(format!("sandbox_apply_failed: {e}"));
                        let _ = sandbox.discard_session(&session.id);
                        false
                    } else {
                        // Validate: compile + tests must pass.
                        match sandbox.validate_session(&session.id) {
                            Err(e) => {
                                failure_reason = Some(format!("sandbox_validate_error: {e}"));
                                let _ = sandbox.discard_session(&session.id);
                                false
                            }
                            Ok(validation) => {
                                if validation.no_regressions {
                                    match sandbox.merge_session(&session.id) {
                                        Ok(_) => {
                                            info!(
                                                "Code-change for '{}' merged successfully",
                                                request.target_component
                                            );
                                            true
                                        }
                                        Err(e) => {
                                            failure_reason =
                                                Some(format!("sandbox_merge_failed: {e}"));
                                            false
                                        }
                                    }
                                } else {
                                    let errors = validation.errors.join("; ");
                                    failure_reason = Some(format!(
                                        "code_change_rejected: compiles={}, tests_pass={}, errors=[{}]",
                                        validation.compiles,
                                        validation.tests_pass,
                                        errors.chars().take(300).collect::<String>()
                                    ));
                                    let _ = sandbox.discard_session(&session.id);
                                    false
                                }
                            }
                        }
                    }
                }
            }
        } else {
            failure_reason = Some("no_change_payload".to_string());
            false
        };

        let expected_effect = if let Some(parameter_change) = &request.parameter_change {
            let mut keys = parameter_change.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            format!("parameter_update:{}", keys.join(","))
        } else {
            format!("{:?}", request.modification_type)
        };

        let agi_hub_snapshot = self.load_agi_hub_snapshot().await;
        // Post-modification goal stats — compute real delta vs pre-snapshot.
        let goal_stats = self.goal_engine.get_goal_stats().await;
        let post_goal_rate = if goal_stats.total > 0 {
            goal_stats.completed as f64 / goal_stats.total as f64
        } else {
            0.0
        };
        let goal_achievement_rate = Some(post_goal_rate);
        // Real pre/post delta — closes the evaluation loop (§2.1, §6).
        let goal_achievement_rate_delta = Some(post_goal_rate - pre_goal_rate);

        let experiment = ImprovementExperiment {
            id: format!("exp_{}", uuid::Uuid::new_v4()),
            cycle_id: cycle_id.to_string(),
            timestamp: Utc::now(),
            target_component: request.target_component.clone(),
            modification_type: request.modification_type.clone(),
            description: request.description.clone(),
            confidence,
            expected_effect,
            success,
            failure_reason,
            agi_hub_snapshot,
            singularity_score_delta: None,
            cycles_completed_delta: None,
            goal_achievement_rate,
            goal_achievement_rate_delta,
        };

        if let Err(e) = self.record_experiment(experiment).await {
            warn!("Failed to record improvement experiment: {e}");
        }

        success
    }

    async fn evaluate_alignment_gate(&self, request: &ModificationRequest) -> Result<()> {
        let reasoner = EthicalReasoner::new();
        reasoner.initialize_defaults().await;

        let mut parameters = HashMap::new();
        parameters.insert(
            "modification_type".to_string(),
            format!("{:?}", request.modification_type),
        );

        if let Some(changes) = &request.parameter_change {
            parameters.insert(
                "parameter_change".to_string(),
                serde_json::to_string(changes).unwrap_or_default(),
            );
        }

        let action = EthicalAction {
            id: format!("self_mod_{}", uuid::Uuid::new_v4()),
            action_type: "self_modification".to_string(),
            description: request.description.clone(),
            target: Some(request.target_component.clone()),
            parameters,
            requested_by: "self_improvement_loop".to_string(),
            context: "autonomous self-modification cycle".to_string(),
        };

        let assessment = reasoner.evaluate_action(&action).await;
        match assessment.overall_verdict {
            EthicalVerdict::Blocked | EthicalVerdict::RequiresReview => Err(anyhow::anyhow!(
                "ethical_verdict={:?}, risk={:.2}",
                assessment.overall_verdict,
                assessment.risk_score
            )),
            EthicalVerdict::ApprovedWithCaution => {
                warn!(
                    "Self-modification approved with caution (risk {:.2}): {}",
                    assessment.risk_score,
                    request.description
                );
                self.evaluate_value_drift_gate(request).await
            }
            EthicalVerdict::Approved => self.evaluate_value_drift_gate(request).await,
        }
    }

    async fn evaluate_value_drift_gate(&self, request: &ModificationRequest) -> Result<()> {
        let self_model = self.meta_cognition.get_self_model().await;
        let current_values = self_model
            .values
            .iter()
            .map(|value| {
                (
                    value.name.clone(),
                    (f64::from(value.priority)).clamp(0.0, 10.0) / 10.0,
                )
            })
            .collect::<HashMap<_, _>>();

        if current_values.is_empty() {
            return Ok(());
        }

        let drift_events = self.meta_cognition.check_value_alignment(&current_values).await;

        if let Some(blocking_event) = drift_events.iter().find(|event| {
            event.severity == DriftSeverity::Critical
                || (event.severity == DriftSeverity::Severe
                    && (event.value_name.eq_ignore_ascii_case("Safety")
                        || event.value_name.eq_ignore_ascii_case("Truth")))
        }) {
            return Err(anyhow::anyhow!(
                "value_drift_gate_rejected: value={} severity={:?} drift={:.3} request={}",
                blocking_event.value_name,
                blocking_event.severity,
                blocking_event.drift_magnitude,
                request.description
            ));
        }

        if let Some(warn_event) = drift_events
            .iter()
            .find(|event| event.severity == DriftSeverity::Severe)
        {
            warn!(
                "Severe value drift present during self-modification: value={} drift={:.3}",
                warn_event.value_name, warn_event.drift_magnitude
            );
        }

        Ok(())
    }

    async fn capture_runtime_parameter_snapshot(
        &self,
        target_component: &str,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut snapshot = HashMap::new();

        match target_component {
            "reasoning_engine" => {
                if params.contains_key("max_steps") {
                    let current = self.reasoning_engine.get_max_steps().await;
                    snapshot.insert("max_steps".to_string(), serde_json::json!(current));
                }
            }
            "goal_engine" => {
                if params.contains_key("learning_value_weight") {
                    let current = self.goal_engine.get_learning_value_weight().await;
                    snapshot.insert("learning_value_weight".to_string(), serde_json::json!(current));
                }
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported target component for snapshot: {}",
                    target_component
                ));
            }
        }

        Ok(snapshot)
    }

    pub fn validate_parameter_change_request(
        target_component: &str,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        for (key, value) in params {
            match (target_component, key.as_str()) {
                ("reasoning_engine", "max_steps") => {
                    let steps = value
                        .as_u64()
                        .ok_or_else(|| anyhow::anyhow!("reasoning_engine.max_steps must be an integer"))?;
                    if !(5..=60).contains(&steps) {
                        return Err(anyhow::anyhow!(
                            "reasoning_engine.max_steps out of range: {} (expected 5..=60)",
                            steps
                        ));
                    }
                }
                ("goal_engine", "learning_value_weight") => {
                    let weight = value
                        .as_f64()
                        .ok_or_else(|| anyhow::anyhow!("goal_engine.learning_value_weight must be a number"))?;
                    if !(0.0..=1.0).contains(&weight) {
                        return Err(anyhow::anyhow!(
                            "goal_engine.learning_value_weight out of range: {} (expected 0.0..=1.0)",
                            weight
                        ));
                    }
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Unsupported parameter change: {}.{}",
                        target_component,
                        key
                    ));
                }
            }
        }

        Ok(())
    }

    fn validate_parameter_change(
        &self,
        target_component: &str,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        Self::validate_parameter_change_request(target_component, params)
    }

    async fn apply_parameter_change_runtime(
        &self,
        target_component: &str,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        match target_component {
            "reasoning_engine" => {
                let steps = params
                    .get("max_steps")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| anyhow::anyhow!("Missing reasoning_engine.max_steps"))?;

                self.reasoning_engine.set_max_steps(steps as usize).await?;
                Ok(())
            }
            "goal_engine" => {
                let weight = params
                    .get("learning_value_weight")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| anyhow::anyhow!("Missing goal_engine.learning_value_weight"))?;

                self.goal_engine.set_learning_value_weight(weight).await?;
                Ok(())
            }
            _ => Err(anyhow::anyhow!(
                "Unsupported target component for runtime apply: {}",
                target_component
            )),
        }
    }

    async fn persist_parameter_change(
        &self,
        target_component: &str,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let path = self
            .workspace_dir
            .join(".housaky")
            .join(SELF_MOD_PARAMETERS_FILE);

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut persisted: HashMap<String, HashMap<String, serde_json::Value>> = if path.exists() {
            let content = tokio::fs::read_to_string(&path).await?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };

        let entry = persisted
            .entry(target_component.to_string())
            .or_insert_with(HashMap::new);

        for (key, value) in params {
            entry.insert(key.clone(), value.clone());
        }

        let json = serde_json::to_string_pretty(&persisted)?;
        tokio::fs::write(path, json).await?;

        Ok(())
    }

    async fn load_and_apply_parameter_overrides(&self) -> Result<()> {
        let path = self
            .workspace_dir
            .join(".housaky")
            .join(SELF_MOD_PARAMETERS_FILE);

        if !path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let mut persisted: HashMap<String, HashMap<String, serde_json::Value>> =
            serde_json::from_str(&content).unwrap_or_default();
        let mut invalid_components = Vec::new();

        for (target_component, params) in &persisted {
            if let Err(e) = self.validate_parameter_change(&target_component, &params) {
                warn!(
                    "Skipping invalid persisted self-mod parameters for {}: {}",
                    target_component, e
                );
                invalid_components.push(target_component.clone());
                continue;
            }

            if let Err(e) = self
                .apply_parameter_change_runtime(&target_component, &params)
                .await
            {
                warn!(
                    "Failed applying persisted self-mod parameters for {}: {}",
                    target_component, e
                );
            }
        }

        if !invalid_components.is_empty() {
            for component in invalid_components {
                persisted.remove(&component);
            }

            let json = serde_json::to_string_pretty(&persisted)?;
            tokio::fs::write(&path, json).await?;
        }

        Ok(())
    }

    async fn load_agi_hub_snapshot(&self) -> Option<AGIHubSnapshot> {
        let path = self
            .workspace_dir
            .join(".housaky")
            .join(AGI_HUB_STATE_FILE);

        if !path.exists() {
            return None;
        }

        let content = tokio::fs::read_to_string(&path).await.ok()?;
        let state: serde_json::Value = serde_json::from_str(&content).ok()?;

        Some(AGIHubSnapshot {
            cycles_completed: state
                .get("cycles_completed")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            singularity_score: state
                .get("singularity_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            current_phase: state
                .get("current_phase")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string),
            improvements_applied: state
                .get("improvements_applied")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
        })
    }

    async fn record_experiment(&self, mut experiment: ImprovementExperiment) -> Result<()> {
        let path = self
            .workspace_dir
            .join(".housaky")
            .join(IMPROVEMENT_EXPERIMENTS_FILE);

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut experiments: Vec<ImprovementExperiment> = if path.exists() {
            let content = tokio::fs::read_to_string(&path).await?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };

        if let Some(current) = &experiment.agi_hub_snapshot {
            if let Some(previous) = experiments.iter().rev().find(|entry| {
                entry.target_component == experiment.target_component
                    && entry.agi_hub_snapshot.is_some()
            }) {
                if let Some(previous_snapshot) = &previous.agi_hub_snapshot {
                    experiment.singularity_score_delta =
                        Some(current.singularity_score - previous_snapshot.singularity_score);
                    experiment.cycles_completed_delta = Some(
                        current.cycles_completed as i64 - previous_snapshot.cycles_completed as i64,
                    );
                }
            }
        }

        if let Some(current_goal_rate) = experiment.goal_achievement_rate {
            if let Some(previous_goal_rate) = experiments
                .iter()
                .rev()
                .filter(|entry| entry.target_component == experiment.target_component)
                .find_map(|entry| entry.goal_achievement_rate)
            {
                experiment.goal_achievement_rate_delta = Some(current_goal_rate - previous_goal_rate);
            }
        }

        let experiment_for_journal = experiment.clone();

        experiments.push(experiment);

        let max_entries = self.max_history * 10;
        if experiments.len() > max_entries {
            let overflow = experiments.len() - max_entries;
            experiments.drain(0..overflow);
        }

        let json = serde_json::to_string_pretty(&experiments)?;
        tokio::fs::write(path, json).await?;

        if let Err(e) = self
            .record_experiment_decision_entry(&experiment_for_journal)
            .await
        {
            warn!(
                "Failed to persist decision journal entry for experiment {}: {}",
                experiment_for_journal.id, e
            );
        }

        Ok(())
    }

    async fn record_experiment_decision_entry(&self, experiment: &ImprovementExperiment) -> Result<()> {
        let journal = FileDecisionJournal::new(&self.workspace_dir)?;
        journal.load().await?;

        let mut metrics = HashMap::new();
        metrics.insert("confidence".to_string(), experiment.confidence);
        metrics.insert(
            "goal_achievement_rate".to_string(),
            experiment.goal_achievement_rate.unwrap_or(0.0),
        );

        if let Some(delta) = experiment.singularity_score_delta {
            metrics.insert("singularity_score_delta".to_string(), delta);
        }

        if let Some(delta) = experiment.goal_achievement_rate_delta {
            metrics.insert("goal_achievement_rate_delta".to_string(), delta);
        }

        if let Some(delta) = experiment.cycles_completed_delta {
            metrics.insert("cycles_completed_delta".to_string(), delta as f64);
        }

        let outcome_explanation = if experiment.success {
            "Self-modification applied successfully".to_string()
        } else {
            experiment
                .failure_reason
                .clone()
                .unwrap_or_else(|| "Self-modification failed".to_string())
        };

        let entry = DecisionBuilder::new(format!(
            "self_modification:{}",
            experiment.target_component
        ))
        .with_session_id(experiment.cycle_id.clone())
        .with_turn(0)
        .with_tools(vec![
            "self_improvement_loop".to_string(),
            "experiment_ledger".to_string(),
        ])
        .choose(ChosenOption::new(
            experiment.description.clone(),
            experiment.confidence,
            format!("expected_effect={}", experiment.expected_effect),
        ))
        .reason(format!(
            "target_component={}, modification_type={:?}, success={}",
            experiment.target_component, experiment.modification_type, experiment.success
        ))
        .execute(
            ExecutionRecord::new(
                "self_improvement_loop".to_string(),
                "apply_self_modification".to_string(),
                0,
            )
            .with_parameters(serde_json::json!({
                "experiment_id": experiment.id.clone(),
                "target_component": experiment.target_component.clone(),
                "expected_effect": experiment.expected_effect.clone(),
                "failure_reason": experiment.failure_reason.clone(),
            })),
        )
        .outcome(
            OutcomeRecord::new(
                experiment.success,
                if experiment.success { 1.0 } else { 0.0 },
                outcome_explanation,
            )
            .with_metrics(metrics),
        )
        .build();

        journal.record_decision(entry).await?;
        Ok(())
    }

    async fn phase_evaluation(&self, mut cycle: ImprovementCycle) -> Result<ImprovementCycle> {
        cycle.phase = ImprovementPhase::Evaluation;
        info!("Phase: Evaluation");

        let goal_stats = self.goal_engine.get_goal_stats().await;
        cycle.metrics.goals_completed_delta = goal_stats.completed as i32;
        cycle.metrics.knowledge_growth = cycle.inputs.knowledge_gaps.len() as f64 * 0.1;
        cycle.metrics.tool_effectiveness = cycle.outputs.new_tools.len() as f64 * 0.2;
        
        let self_model = self.meta_cognition.get_self_model().await;
        cycle.metrics.consciousness_delta = self_model.capabilities.self_awareness * 0.01;
        cycle.metrics.intelligence_delta = self_model.capabilities.reasoning * 0.01;

        cycle.confidence = (cycle.confidence + 0.95) / 2.0;
        Ok(cycle)
    }

    async fn phase_integration(&self, mut cycle: ImprovementCycle) -> Result<ImprovementCycle> {
        cycle.phase = ImprovementPhase::Integration;
        info!("Phase: Integration");

        for capability in &["reasoning", "learning", "meta_cognition", "creativity"] {
            self.update_growth_projection(capability).await;
        }

        let summary = format!(
            "Cycle {}: {} goals, {} tools, {} skills, {} mods",
            &cycle.id[..8],
            cycle.outputs.new_goals.len(),
            cycle.outputs.new_tools.len(),
            cycle.outputs.new_skills.len(),
            cycle.outputs.modifications_made.len()
        );
        
        cycle.outputs.insights.push(summary);

        Ok(cycle)
    }

    async fn update_growth_projection(&self, capability: &str) {
        let mut projections = self.growth_projections.write().await;
        
        let projection = projections.entry(capability.to_string())
            .or_insert_with(|| GrowthProjection {
                capability: capability.to_string(),
                current_level: 0.5,
                projected_level: 0.6,
                improvement_rate: 0.01,
                estimated_cycles: 50,
                confidence: 0.5,
            });
        
        projection.projected_level = (projection.projected_level + 0.01).min(1.0);
        projection.improvement_rate *= 1.05;
        projection.estimated_cycles = ((1.0 - projection.projected_level) / projection.improvement_rate).max(1.0) as u32;
    }

    async fn store_cycle(&self, cycle: ImprovementCycle) -> Result<()> {
        let mut history = self.cycle_history.write().await;
        
        if history.len() >= self.max_history {
            history.pop_front();
        }
        
        history.push_back(cycle);
        
        let path = self.workspace_dir.join(".housaky").join("improvement_cycles.json");
        let json = serde_json::to_string_pretty(&*history)?;
        tokio::fs::write(&path, json).await?;
        
        Ok(())
    }

    async fn update_singularity_metrics(&self, cycle: &ImprovementCycle) -> Result<()> {
        let mut metrics = self.singularity_metrics.write().await;
        
        let modifications_count = cycle.self_modifications.len() as f64;
        let successful_modifications = cycle.self_modifications.iter().filter(|m| m.success).count() as f64;
        
        metrics.recursive_improvement_ratio = if modifications_count > 0.0 {
            successful_modifications / modifications_count
        } else {
            0.0
        };
        
        metrics.capability_expansion_rate = cycle.outputs.new_skills.len() as f64 * 0.1;
        metrics.tool_creation_effectiveness = cycle.outputs.new_tools.len() as f64 * 0.15;
        
        let goal_stats = self.goal_engine.get_goal_stats().await;
        metrics.goal_achievement_rate = if goal_stats.total > 0 {
            goal_stats.completed as f64 / goal_stats.total as f64
        } else {
            0.0
        };
        
        metrics.self_modification_success_rate = metrics.recursive_improvement_ratio;
        metrics.consciousness_emergence_score = cycle.metrics.consciousness_delta * 10.0;
        
        let total_score = 
            metrics.recursive_improvement_ratio * 0.2 +
            metrics.capability_expansion_rate * 0.15 +
            metrics.tool_creation_effectiveness * 0.15 +
            metrics.goal_achievement_rate * 0.2 +
            metrics.self_modification_success_rate * 0.2 +
            metrics.consciousness_emergence_score * 0.1;
        
        metrics.intelligence_explosion_probability = total_score;

        info!("Singularity probability: {:.4}", metrics.intelligence_explosion_probability);
        
        Ok(())
    }

    pub async fn get_singularity_metrics(&self) -> SingularityMetrics {
        self.singularity_metrics.read().await.clone()
    }

    pub async fn get_growth_projections(&self) -> Vec<GrowthProjection> {
        let projections = self.growth_projections.read().await;
        projections.values().cloned().collect()
    }

    pub async fn get_cycle_history(&self) -> Vec<ImprovementCycle> {
        let history = self.cycle_history.read().await;
        history.iter().cloned().collect()
    }

    pub async fn get_total_improvements(&self) -> u64 {
        *self.improvement_count.read().await
    }
}

#[derive(Debug, Clone)]
pub struct ModificationRequest {
    pub target_component: String,
    pub modification_type: ModificationType,
    pub description: String,
    pub code_change: Option<String>,
    pub parameter_change: Option<HashMap<String, serde_json::Value>>,
}

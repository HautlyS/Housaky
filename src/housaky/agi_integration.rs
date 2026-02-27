#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use crate::housaky::capability_growth_tracker::CapabilityGrowthTracker;
use crate::housaky::consciousness::phase3_engine::{Phase3Engine, Phase3Config};
use crate::housaky::goal_engine::{Goal, GoalPriority, GoalStatus};
use crate::housaky::knowledge_graph::EntityType;
use crate::housaky::meta_cognition::MetaCognitionEngine;
use crate::housaky::reasoning_pipeline::{ReasoningPipeline, ReasoningResult};
use crate::housaky::singularity::SingularityEngine;
use crate::housaky::tool_creator::{ToolCreator, ToolGenerationRequest, ToolKind};
use crate::providers::Provider;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

const AGI_HUB_STATE_FILE: &str = "agi_hub_state.json";

pub struct AGIIntegrationHub {
    reasoning: Arc<ReasoningPipeline>,
    goal_engine: Arc<crate::housaky::goal_engine::GoalEngine>,
    tool_creator: Arc<ToolCreator>,
    meta_cognition: Arc<MetaCognitionEngine>,
    knowledge_graph: Arc<crate::housaky::knowledge_graph::KnowledgeGraphEngine>,
    inner_monologue: Arc<crate::housaky::inner_monologue::InnerMonologue>,
    singularity: Arc<RwLock<SingularityEngine>>,
    consciousness: Arc<Phase3Engine>,
    workspace_dir: PathBuf,
    state: Arc<RwLock<AGIHubState>>,
    config: AGIHubConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AGIHubState {
    pub cycles_completed: u64,
    pub reasoning_calls: u64,
    pub goals_created: u64,
    pub tools_generated: u64,
    pub reflections_completed: u64,
    pub failures_analyzed: u64,
    pub improvements_applied: u64,
    pub current_phase: AGIPhase,
    pub last_cycle_timestamp: Option<chrono::DateTime<Utc>>,
    pub singularity_score: f64,
    pub convergence_indicators: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AGIPhase {
    Initializing,
    Reasoning,
    GoalOriented,
    ToolBuilding,
    SelfReflecting,
    Improving,
    Converging,
    Singular,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AGIHubConfig {
    pub auto_create_tools: bool,
    pub auto_create_goals: bool,
    pub failure_analysis_depth: u32,
    pub improvement_threshold: f64,
    pub singularity_threshold: f64,
    pub max_goals_per_cycle: usize,
    pub enable_feedback_loop: bool,
}

impl Default for AGIHubConfig {
    fn default() -> Self {
        Self {
            auto_create_tools: true,
            auto_create_goals: true,
            failure_analysis_depth: 3,
            improvement_threshold: 0.6,
            singularity_threshold: 0.95,
            max_goals_per_cycle: 3,
            enable_feedback_loop: true,
        }
    }
}

impl Default for AGIHubState {
    fn default() -> Self {
        Self {
            cycles_completed: 0,
            reasoning_calls: 0,
            goals_created: 0,
            tools_generated: 0,
            reflections_completed: 0,
            failures_analyzed: 0,
            improvements_applied: 0,
            current_phase: AGIPhase::Initializing,
            last_cycle_timestamp: None,
            singularity_score: 0.1,
            convergence_indicators: HashMap::new(),
        }
    }
}

pub struct AGICycleInput {
    pub user_query: String,
    pub context: AGICycleContext,
    #[doc(hidden)]
    pub provider: Option<Arc<dyn Provider>>,
    pub model: Option<String>,
    pub available_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AGICycleContext {
    pub active_goals: Vec<crate::housaky::goal_engine::Goal>,
    pub recent_failures: Vec<FailureRecord>,
    pub previous_insights: Vec<String>,
    pub knowledge_context: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureRecord {
    pub id: String,
    pub action: String,
    pub error: String,
    pub timestamp: chrono::DateTime<Utc>,
    pub analysis: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AGICycleOutput {
    pub reasoning_result: Option<ReasoningResult>,
    pub actions_taken: Vec<AGIAction>,
    pub goals_created: Vec<String>,
    pub tools_created: Vec<String>,
    pub reflections: Vec<ReflectionSummary>,
    pub singularity_progress: SingularityProgress,
    pub cycle_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AGIAction {
    pub action_type: AGIActionType,
    pub description: String,
    pub success: bool,
    pub details: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AGIActionType {
    Reasoning,
    GoalCreation,
    ToolCreation,
    Reflection,
    ToolExecution,
    KnowledgeUpdate,
    GoalUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionSummary {
    pub observations: usize,
    pub insights: usize,
    pub actions: usize,
    pub confidence_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingularityProgress {
    pub score: f64,
    pub phase: AGIPhase,
    pub metrics: HashMap<String, f64>,
    pub convergence_level: f64,
    pub improvement_trend: f64,
}

impl AGIIntegrationHub {
    pub fn new(workspace_dir: &PathBuf) -> Self {
        let reasoning = Arc::new(ReasoningPipeline::new());
        let goal_engine = Arc::new(crate::housaky::goal_engine::GoalEngine::new(workspace_dir));
        let tool_creator = Arc::new(ToolCreator::new(workspace_dir));
        let meta_cognition = Arc::new(MetaCognitionEngine::new());
        let knowledge_graph = Arc::new(crate::housaky::knowledge_graph::KnowledgeGraphEngine::new(workspace_dir));
        let inner_monologue = Arc::new(crate::housaky::inner_monologue::InnerMonologue::new(workspace_dir));
        let growth_tracker = Arc::new(CapabilityGrowthTracker::new());
        let singularity = Arc::new(RwLock::new(SingularityEngine::new(growth_tracker)));
        // Phase3Engine::new is async; use a blocking future here via block_in_place
        // so we can keep new() synchronous.  In practice this is fine as it only
        // allocates in-memory structures.
        let consciousness = Arc::new(
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(Phase3Engine::new(Phase3Config::default()))
            })
        );

        Self {
            reasoning,
            goal_engine,
            tool_creator,
            meta_cognition,
            knowledge_graph,
            inner_monologue,
            singularity,
            consciousness,
            workspace_dir: workspace_dir.clone(),
            state: Arc::new(RwLock::new(AGIHubState::default())),
            config: AGIHubConfig::default(),
        }
    }

    pub fn with_config(workspace_dir: &PathBuf, config: AGIHubConfig) -> Self {
        let mut hub = Self::new(workspace_dir);
        hub.config = config;
        hub
    }

    fn state_path(&self) -> PathBuf {
        self.workspace_dir
            .join(".housaky")
            .join(AGI_HUB_STATE_FILE)
    }

    async fn load_state(&self) -> Result<()> {
        let path = self.state_path();
        if !path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let loaded: AGIHubState = serde_json::from_str(&content)?;
        let mut state = self.state.write().await;
        *state = loaded;
        Ok(())
    }

    async fn persist_state(&self) -> Result<()> {
        let path = self.state_path();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let state = self.state.read().await.clone();
        let json = serde_json::to_string_pretty(&state)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }

    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing AGI Integration Hub...");
        
        self.goal_engine.load_goals().await?;
        self.knowledge_graph.load_graph().await?;
        self.inner_monologue.load().await?;
        self.tool_creator.load_tools().await?;

        // Initialise Phase 6 singularity substrates
        self.singularity.write().await.init_substrates().await;

        if let Err(e) = self.load_state().await {
            warn!("Failed to load AGI hub state: {e}");
        }

        let mut state = self.state.write().await;
        if state.current_phase == AGIPhase::Initializing {
            state.current_phase = AGIPhase::Reasoning;
        }

        drop(state);
        if let Err(e) = self.persist_state().await {
            warn!("Failed to persist AGI hub state: {e}");
        }

        info!("AGI Integration Hub initialized");
        Ok(())
    }

    pub async fn run_agi_cycle(&self, input: AGICycleInput) -> Result<AGICycleOutput> {
        let start_time = std::time::Instant::now();
        info!("Starting AGI cycle for query: {}", input.user_query.chars().take(50).collect::<String>());

        let mut actions_taken = Vec::new();
        let mut goals_created = Vec::new();
        let mut tools_created = Vec::new();
        let mut reflections = Vec::new();
        let mut reasoning_result = None;

        {
            let mut state = self.state.write().await;
            state.current_phase = AGIPhase::Reasoning;
            state.cycles_completed += 1;
        }

        if let Some(provider) = &input.provider {
            let model = input.model.as_deref().unwrap_or("default");
            
            let tool_names: Vec<&str> = input.available_tools.iter().map(|s| s.as_str()).collect();
            
            let context = crate::housaky::core::TurnContext {
                user_message: input.user_query.clone(),
                relevant_memories: input.context.knowledge_context.clone(),
                active_goals: input.context.active_goals.clone(),
                recent_thoughts: input.context.previous_insights.clone(),
                working_context: String::new(),
            };

            let result = self.reasoning
                .reason_react(provider.as_ref(), model, &input.user_query, &tool_names, &context)
                .await?;
            
            reasoning_result = Some(result.clone());

            actions_taken.push(AGIAction {
                action_type: AGIActionType::Reasoning,
                description: format!("Reasoning completed with confidence: {:.2}", result.confidence),
                success: true,
                details: HashMap::new(),
            });

            {
                let mut state = self.state.write().await;
                state.reasoning_calls += 1;
            }

            self.inner_monologue
                .add_thought(
                    &format!("Reasoning: {} -> {}", input.user_query.chars().take(30).collect::<String>(), result.summary.chars().take(50).collect::<String>()),
                    result.confidence,
                )
                .await?;

            if self.config.auto_create_goals {
                let new_goals = self.connect_reasoning_to_goals(&result).await?;
                for goal_id in &new_goals {
                    goals_created.push(goal_id.clone());
                    actions_taken.push(AGIAction {
                        action_type: AGIActionType::GoalCreation,
                        description: format!("Created goal: {}", goal_id),
                        success: true,
                        details: HashMap::new(),
                    });
                }
            }

            if self.config.auto_create_tools && !result.suggested_tools.is_empty() {
                let created_tools = self.connect_reasoning_to_tools(&result, provider.as_ref(), model).await?;
                for tool_id in &created_tools {
                    tools_created.push(tool_id.clone());
                    actions_taken.push(AGIAction {
                        action_type: AGIActionType::ToolCreation,
                        description: format!("Created tool: {}", tool_id),
                        success: true,
                        details: HashMap::new(),
                    });
                }
            }
        }

        if !input.context.recent_failures.is_empty() && self.config.enable_feedback_loop {
            info!("Analyzing {} failures from previous cycles", input.context.recent_failures.len());
            
            let analysis = self.analyze_failure(&input.context.recent_failures).await?;
            
            actions_taken.push(AGIAction {
                action_type: AGIActionType::Reflection,
                description: format!("Analyzed {} failures", input.context.recent_failures.len()),
                success: true,
                details: [("failures_analyzed".to_string(), input.context.recent_failures.len().to_string())].into_iter().collect(),
            });

            let improvements = self.improve_from_insights(&analysis).await?;
            
            for improvement in &improvements {
                actions_taken.push(AGIAction {
                    action_type: AGIActionType::GoalUpdate,
                    description: improvement.clone(),
                    success: true,
                    details: HashMap::new(),
                });
            }

            {
                let mut state = self.state.write().await;
                state.failures_analyzed += input.context.recent_failures.len() as u64;
                state.improvements_applied += improvements.len() as u64;
            }
        }

        let reflection = self.meta_cognition
            .reflect("AGI cycle completed")
            .await?;

        reflections.push(ReflectionSummary {
            observations: reflection.observations.len(),
            insights: reflection.insights.len(),
            actions: reflection.actions.len(),
            confidence_delta: reflection.confidence_delta,
        });

        {
            let mut state = self.state.write().await;
            state.reflections_completed += 1;
            state.last_cycle_timestamp = Some(Utc::now());
        }

        self.inner_monologue
            .add_thought(
                &format!("Cycle {} complete: {} actions", 
                    { let s = self.state.read().await; s.cycles_completed },
                    actions_taken.len()
                ),
                0.8,
            )
            .await?;

        let progress = self.track_singularity_progress().await?;

        // ── Consciousness broadcast cycle ───────────────────────────────────
        {
            // Feed the most-urgent active goal into the consciousness engine so
            // the GWT competition is seeded with real AGI context each heartbeat.
            let active_goals = self.goal_engine.get_active_goals().await;
            if let Some(top_goal) = active_goals.first() {
                let urgency = top_goal.priority.clone() as u8 as f64 / 4.0;
                self.consciousness
                    .set_active_goal(&top_goal.title, urgency)
                    .await;
            }
            if let Some(ref rr) = reasoning_result {
                self.consciousness
                    .reasoning_adapter
                    .set_active_reasoning(
                        &rr.summary.chars().take(100).collect::<String>(),
                        rr.confidence,
                    )
                    .await;
            }
            let consciousness_report = self.consciousness.run_cycle().await;
            info!(
                phi = consciousness_report.phi,
                level = %consciousness_report.level,
                winning_coalition = ?consciousness_report.winning_coalition,
                narrative_entries = consciousness_report.narrative_entries,
                "Consciousness broadcast cycle complete"
            );
            // Persist AGI cycle context as an episodic event in the consciousness engine
            let cycle_num = { self.state.read().await.cycles_completed };
            self.consciousness
                .record_cognitive_event(
                    crate::housaky::memory::episodic::EpisodicEventType::ReasoningStep,
                    &format!(
                        "AGI cycle {}: {} actions, phi={:.2}",
                        cycle_num,
                        actions_taken.len(),
                        consciousness_report.phi
                    ),
                    consciousness_report.phi,
                    !actions_taken.is_empty(),
                    consciousness_report.phi,
                    (actions_taken.len() as f64 / 10.0).clamp(0.0, 1.0),
                )
                .await;
        }

        // ── Phase 6 singularity tick ────────────────────────────────────────
        {
            let cycle = { self.state.read().await.cycles_completed };
            let alignment_ok = progress.convergence_level > 0.5;
            // Gather knowledge gaps from active goals as seed concepts
            let active_goals = self.goal_engine.get_active_goals().await;
            let knowledge_gaps: Vec<String> = active_goals
                .iter()
                .take(8)
                .map(|g| g.title.clone())
                .collect();
            let seed_concepts: Vec<String> = active_goals
                .iter()
                .flat_map(|g| g.tags.iter().cloned())
                .take(8)
                .collect();
            let sing_report = self
                .singularity
                .write()
                .await
                .tick(cycle, alignment_ok, &knowledge_gaps, &seed_concepts)
                .await;
            info!(
                singularity_phase = ?sing_report.phase_status,
                open_ended_goals = sing_report.open_ended_goals_generated,
                substrates = sing_report.substrate_count,
                "SingularityEngine tick complete"
            );
        }

        {
            let mut state = self.state.write().await;
            if progress.convergence_level > self.config.singularity_threshold {
                state.current_phase = AGIPhase::Singular;
            } else if progress.convergence_level > 0.8 {
                state.current_phase = AGIPhase::Converging;
            } else if progress.convergence_level > 0.5 {
                state.current_phase = AGIPhase::Improving;
            }
        }

        if let Err(e) = self.persist_state().await {
            warn!("Failed to persist AGI hub state after cycle: {e}");
        }

        let cycle_time_ms = start_time.elapsed().as_millis() as u64;
        info!("AGI cycle completed in {}ms: {} actions, {} goals, {} tools", 
            cycle_time_ms, actions_taken.len(), goals_created.len(), tools_created.len());

        Ok(AGICycleOutput {
            reasoning_result,
            actions_taken,
            goals_created,
            tools_created,
            reflections,
            singularity_progress: progress,
            cycle_time_ms,
        })
    }

    async fn connect_reasoning_to_goals(&self, reasoning: &ReasoningResult) -> Result<Vec<String>> {
        let mut created_goal_ids = Vec::new();

        if reasoning.conclusion.contains("goal") || reasoning.conclusion.contains("objective") 
            || reasoning.conclusion.contains("should") || reasoning.conclusion.contains("need to") {
            
            let goal = Goal {
                id: format!("goal_{}", uuid::Uuid::new_v4()),
                title: reasoning.conclusion.chars().take(80).collect(),
                description: reasoning.summary.clone(),
                priority: GoalPriority::Medium,
                status: GoalStatus::Pending,
                category: crate::housaky::goal_engine::GoalCategory::Intelligence,
                progress: 0.0,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                deadline: None,
                parent_id: None,
                subtask_ids: vec![],
                dependencies: vec![],
                blockers: vec![],
                metrics: HashMap::new(),
                checkpoints: vec![],
                attempts: 0,
                max_attempts: 3,
                estimated_complexity: 0.5,
                actual_complexity: None,
                learning_value: reasoning.confidence,
                tags: vec!["from_reasoning".to_string()],
                context: HashMap::new(),
                temporal_constraints: Vec::new(),
            };

            let goal_id = self.goal_engine.add_goal(goal).await?;
            created_goal_ids.push(goal_id);

            for insight in &reasoning.insights {
                let insight_goal = Goal {
                id: format!("goal_{}", uuid::Uuid::new_v4()),
                title: insight.chars().take(80).collect(),
                    description: format!("From reasoning insight: {}", insight),
                    priority: GoalPriority::Low,
                    status: GoalStatus::Pending,
                    category: crate::housaky::goal_engine::GoalCategory::Intelligence,
                    progress: 0.0,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    deadline: None,
                    parent_id: None,
                    subtask_ids: vec![],
                    dependencies: vec![],
                    blockers: vec![],
                    metrics: HashMap::new(),
                    checkpoints: vec![],
                    attempts: 0,
                    max_attempts: 2,
                    estimated_complexity: 0.3,
                    actual_complexity: None,
                    learning_value: 0.5,
                    tags: vec!["insight_goal".to_string()],
                    context: HashMap::new(),
                    temporal_constraints: Vec::new(),
                };

                let insight_goal_id = self.goal_engine.add_goal(insight_goal).await?;
                created_goal_ids.push(insight_goal_id);
            }

            {
                let mut state = self.state.write().await;
                state.goals_created += created_goal_ids.len() as u64;
            }
        }

        Ok(created_goal_ids)
    }

    async fn connect_reasoning_to_tools(
        &self, 
        reasoning: &ReasoningResult,
        _provider: &dyn Provider,
        _model: &str,
    ) -> Result<Vec<String>> {
        let mut created_tool_ids = Vec::new();

        for tool_suggestion in &reasoning.suggested_tools {
            let request = ToolGenerationRequest {
                name: tool_suggestion.name.clone(),
                description: format!("Auto-generated from reasoning: {}", tool_suggestion.reasoning),
                kind: ToolKind::Composite,
                examples: vec![],
                constraints: vec![],
            };

            match self.tool_creator.generate_tool(request).await {
                Ok(generated_tool) => {
                    let tool_id = generated_tool.id.clone();
                    created_tool_ids.push(tool_id.clone());
                    
                    self.inner_monologue
                        .add_thought(
                            &format!("Created tool '{}' from reasoning suggestion", tool_id),
                            tool_suggestion.priority as f64 / 10.0,
                        )
                        .await?;

                    {
                        let mut state = self.state.write().await;
                        state.tools_generated += 1;
                    }
                }
                Err(e) => {
                    warn!("Failed to create tool from suggestion: {}", e);
                }
            }
        }

        Ok(created_tool_ids)
    }

    pub async fn analyze_failure(&self, failures: &[FailureRecord]) -> Result<FailureAnalysis> {
        info!("Analyzing {} failures", failures.len());

        let mut root_causes = Vec::new();
        let mut patterns = Vec::new();
        let mut recommendations = Vec::new();

        let mut error_counts: HashMap<String, u32> = HashMap::new();
        for failure in failures {
            let error_type = Self::categorize_error(&failure.error);
            *error_counts.entry(error_type).or_insert(0) += 1;
        }

        for (error_type, count) in &error_counts {
            if *count > 1 {
                patterns.push(format!("{} occurred {} times", error_type, count));
            }
            
            let cause = match error_type.as_str() {
                "timeout" => "Action took too long - consider optimization or async handling",
                "authentication" => "Missing or invalid credentials - need to add auth handling",
                "permission" => "Insufficient permissions - need role escalation",
                "validation" => "Input validation failed - need better error checking",
                "resource" => "Resource unavailable - need better resource management",
                "logic" => "Internal logic error - need to review algorithm",
                _ => "Unknown error type - need investigation",
            };
            root_causes.push(format!("{}: {}", error_type, cause));
        }

        for failure in failures {
            if failure.analysis.is_none() {
                let analysis = format!(
                    "Failure in '{}': {}. Suggested fix: {}",
                    failure.action,
                    failure.error,
                    root_causes.first().map(|s| s.as_str()).unwrap_or("Review logic")
                );
                recommendations.push(analysis);
            }
        }

        if failures.len() >= 3 {
            recommendations.push("Multiple failures detected - consider a major refactoring or approach change".to_string());
        }

        self.inner_monologue
            .add_thought(
                &format!("Failure analysis: {} root causes identified", root_causes.len()),
                0.7,
            )
            .await?;

        {
            let mut state = self.state.write().await;
            state.failures_analyzed += failures.len() as u64;
        }

        Ok(FailureAnalysis {
            root_causes,
            patterns,
            recommendations,
            failure_count: failures.len(),
        })
    }

    fn categorize_error(error: &str) -> String {
        let error_lower = error.to_lowercase();
        
        if error_lower.contains("timeout") || error_lower.contains("timed out") {
            "timeout".to_string()
        } else if error_lower.contains("auth") || error_lower.contains("credential") || error_lower.contains("unauthorized") {
            "authentication".to_string()
        } else if error_lower.contains("permission") || error_lower.contains("denied") || error_lower.contains("forbidden") {
            "permission".to_string()
        } else if error_lower.contains("valid") || error_lower.contains("invalid") || error_lower.contains("parse") {
            "validation".to_string()
        } else if error_lower.contains("resource") || error_lower.contains("memory") || error_lower.contains("cpu") {
            "resource".to_string()
        } else if error_lower.contains("null") || error_lower.contains("undefined") || error_lower.contains("none") {
            "logic".to_string()
        } else {
            "unknown".to_string()
        }
    }

    pub async fn improve_from_insights(&self, analysis: &FailureAnalysis) -> Result<Vec<String>> {
        info!("Applying improvements from failure analysis");
        
        let mut improvements = Vec::new();

        for recommendation in &analysis.recommendations {
            if recommendation.contains("refactoring") || recommendation.contains("approach change") {
                let reflection = self.meta_cognition
                    .reflect("Major improvement needed based on failure analysis")
                    .await?;

                for insight in &reflection.insights {
                    if insight.actionable {
                        let goal = Goal {
                            id: format!("goal_{}", uuid::Uuid::new_v4()),
                            title: insight.content.chars().take(80).collect(),
                            description: format!("Improvement from failure analysis: {}", insight.content),
                            priority: GoalPriority::High,
                            status: GoalStatus::Pending,
                            category: crate::housaky::goal_engine::GoalCategory::SystemImprovement,
                            progress: 0.0,
                            created_at: Utc::now(),
                            updated_at: Utc::now(),
                            deadline: None,
                            parent_id: None,
                            subtask_ids: vec![],
                            dependencies: vec![],
                            blockers: vec![],
                            metrics: HashMap::new(),
                            checkpoints: vec![],
                            attempts: 0,
                            max_attempts: 2,
                            estimated_complexity: 0.7,
                            actual_complexity: None,
                            learning_value: insight.confidence,
                            tags: vec!["improvement".to_string()],
                            context: HashMap::new(),
                            temporal_constraints: Vec::new(),
                        };

                        self.goal_engine.add_goal(goal).await?;
                        improvements.push(format!("Created improvement goal: {}", insight.content.chars().take(50).collect::<String>()));
                    }
                }
            } else if recommendation.contains("validation") || recommendation.contains("error checking") {
                improvements.push("Add better input validation".to_string());
                
                self.inner_monologue
                    .add_thought("Improvement: Add better input validation", 0.7)
                    .await?;
            } else if recommendation.contains("resource") {
                improvements.push("Improve resource management".to_string());
                
                self.inner_monologue
                    .add_thought("Improvement: Optimize resource management", 0.7)
                    .await?;
            }
        }

        for pattern in &analysis.patterns {
            self.knowledge_graph
                .add_entity(
                    &format!("pattern_{}", pattern.chars().take(20).collect::<String>()).replace(' ', "_"),
                    EntityType::Concept,
                    pattern,
                )
                .await?;
        }

        {
            let mut state = self.state.write().await;
            state.improvements_applied += improvements.len() as u64;
        }

        Ok(improvements)
    }

    pub async fn track_singularity_progress(&self) -> Result<SingularityProgress> {
        let state = self.state.read().await;
        
        let mut metrics = HashMap::new();
        
        metrics.insert("cycles_completed".to_string(), state.cycles_completed as f64);
        metrics.insert("reasoning_calls".to_string(), state.reasoning_calls as f64);
        metrics.insert("goals_created".to_string(), state.goals_created as f64);
        metrics.insert("tools_generated".to_string(), state.tools_generated as f64);
        metrics.insert("reflections_completed".to_string(), state.reflections_completed as f64);
        metrics.insert("failures_analyzed".to_string(), state.failures_analyzed as f64);
        metrics.insert("improvements_applied".to_string(), state.improvements_applied as f64);

        let reasoning_density = if state.cycles_completed > 0 {
            state.reasoning_calls as f64 / state.cycles_completed as f64
        } else {
            0.0
        };
        metrics.insert("reasoning_density".to_string(), reasoning_density);

        let goal_creation_rate = if state.cycles_completed > 0 {
            state.goals_created as f64 / state.cycles_completed as f64
        } else {
            0.0
        };
        metrics.insert("goal_creation_rate".to_string(), goal_creation_rate);

        let tool_creation_rate = if state.reasoning_calls > 0 {
            state.tools_generated as f64 / state.reasoning_calls as f64
        } else {
            0.0
        };
        metrics.insert("tool_creation_rate".to_string(), tool_creation_rate);

        let improvement_rate = if state.failures_analyzed > 0 {
            state.improvements_applied as f64 / state.failures_analyzed as f64
        } else {
            1.0
        };
        metrics.insert("improvement_rate".to_string(), improvement_rate);

        let singularity_score = self.calculate_singularity_score(&metrics);
        
        let convergence_level = self.calculate_convergence(&metrics);
        
        let improvement_trend = if state.cycles_completed > 10 {
            let recent_improvements = state.improvements_applied.saturating_sub(state.cycles_completed / 2);
            recent_improvements as f64 / 10.0
        } else {
            state.improvements_applied as f64
        };

        let phase = state.current_phase.clone();
        let current_convergence = state.convergence_indicators.clone();

        drop(state);

        let mut convergence_indicators = current_convergence;
        convergence_indicators.insert("score".to_string(), singularity_score);
        convergence_indicators.insert("convergence".to_string(), convergence_level);
        
        let mut state = self.state.write().await;
        state.singularity_score = singularity_score;
        state.convergence_indicators = convergence_indicators.clone();

        Ok(SingularityProgress {
            score: singularity_score,
            phase,
            metrics,
            convergence_level,
            improvement_trend,
        })
    }

    fn calculate_singularity_score(&self, metrics: &HashMap<String, f64>) -> f64 {
        let cycles = metrics.get("cycles_completed").copied().unwrap_or(0.0);
        let reasoning_density = metrics.get("reasoning_density").copied().unwrap_or(0.0);
        let goal_rate = metrics.get("goal_creation_rate").copied().unwrap_or(0.0);
        let tool_rate = metrics.get("tool_creation_rate").copied().unwrap_or(0.0);
        let improvement_rate = metrics.get("improvement_rate").copied().unwrap_or(0.0);

        let cycle_score = (cycles / 100.0).min(0.3);
        let reasoning_score = (reasoning_density / 5.0).min(0.2);
        let goal_score = (goal_rate * 10.0).min(0.2);
        let tool_score = (tool_rate * 10.0).min(0.15);
        let improvement_score = improvement_rate.min(0.15);

        cycle_score + reasoning_score + goal_score + tool_score + improvement_score
    }

    fn calculate_convergence(&self, metrics: &HashMap<String, f64>) -> f64 {
        let cycles = metrics.get("cycles_completed").copied().unwrap_or(0.0);
        let improvements = metrics.get("improvements_applied").copied().unwrap_or(0.0);
        
        if cycles < 10.0 {
            return 0.1;
        }

        let improvement_density = improvements / cycles;
        
        let stability_score = if cycles > 50.0 {
            0.3
        } else if cycles > 20.0 {
            0.2
        } else {
            0.1
        };

        (improvement_density * 0.7 + stability_score).min(1.0)
    }

    pub async fn get_state(&self) -> AGIHubState {
        self.state.read().await.clone()
    }

    pub async fn get_hub_metrics(&self) -> HubMetrics {
        let state = self.state.read().await;
        let goal_stats = self.goal_engine.get_goal_stats().await;
        
        HubMetrics {
            cycles_completed: state.cycles_completed,
            reasoning_calls: state.reasoning_calls,
            goals_created: state.goals_created,
            tools_generated: state.tools_generated,
            reflections_completed: state.reflections_completed,
            failures_analyzed: state.failures_analyzed,
            improvements_applied: state.improvements_applied,
            singularity_score: state.singularity_score,
            current_phase: state.current_phase.clone(),
            active_goals: goal_stats.in_progress,
            completed_goals: goal_stats.completed,
        }
    }

    pub async fn reset(&self) -> Result<()> {
        let mut state = self.state.write().await;
        *state = AGIHubState::default();
        drop(state);

        if let Err(e) = self.persist_state().await {
            warn!("Failed to persist AGI hub state after reset: {e}");
        }

        info!("AGI Integration Hub state reset");
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureAnalysis {
    pub root_causes: Vec<String>,
    pub patterns: Vec<String>,
    pub recommendations: Vec<String>,
    pub failure_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubMetrics {
    pub cycles_completed: u64,
    pub reasoning_calls: u64,
    pub goals_created: u64,
    pub tools_generated: u64,
    pub reflections_completed: u64,
    pub failures_analyzed: u64,
    pub improvements_applied: u64,
    pub singularity_score: f64,
    pub current_phase: AGIPhase,
    pub active_goals: usize,
    pub completed_goals: usize,
}

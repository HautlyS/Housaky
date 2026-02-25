use crate::config::Config;
use crate::housaky::cognitive::action_selector::{
    ActionDecision, ActionOutcome, ActionResult, ActionSelector,
};
use crate::housaky::cognitive::experience_learner::ExperienceLearner;
use crate::housaky::cognitive::perception::{PerceivedInput, PerceptionEngine};
use crate::housaky::cognitive::uncertainty::UncertaintyAssessment;
use crate::housaky::goal_engine::GoalEngine;
use crate::housaky::inner_monologue::InnerMonologue;
use crate::housaky::knowledge_graph::KnowledgeGraphEngine;
use crate::housaky::meta_cognition::MetaCognitionEngine;
use crate::housaky::reasoning_pipeline::ReasoningPipeline;
use crate::housaky::working_memory::WorkingMemoryEngine;
use crate::providers::{create_provider, ChatMessage, Provider};
use crate::tools::Tool;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::info;
use std::fmt::Write as _;

pub struct CognitiveLoop {
    pub perception: PerceptionEngine,
    pub action_selector: ActionSelector,
    pub uncertainty: crate::housaky::cognitive::uncertainty::UncertaintyDetector,
    pub experience_learner: ExperienceLearner,
    pub working_memory: Arc<WorkingMemoryEngine>,
    pub goal_engine: Arc<GoalEngine>,
    pub reasoning: Arc<ReasoningPipeline>,
    pub meta_cognition: Arc<MetaCognitionEngine>,
    pub knowledge_graph: Arc<KnowledgeGraphEngine>,
    pub inner_monologue: Arc<InnerMonologue>,
    pub config: CognitiveLoopConfig,
    state: Arc<RwLock<CognitiveState>>,
    workspace_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct CognitiveLoopConfig {
    pub enable_reflection: bool,
    pub enable_learning: bool,
    pub enable_uncertainty_detection: bool,
    pub reasoning_depth: u32,
    pub max_iterations: u32,
    pub reflection_frequency: u32,
    pub auto_goal_creation: bool,
}

impl Default for CognitiveLoopConfig {
    fn default() -> Self {
        Self {
            enable_reflection: true,
            enable_learning: true,
            enable_uncertainty_detection: true,
            reasoning_depth: 3,
            max_iterations: 10,
            reflection_frequency: 5,
            auto_goal_creation: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveState {
    pub total_turns: u64,
    pub successful_actions: u64,
    pub failed_actions: u64,
    pub reflections_count: u64,
    pub skills_learned: u64,
    pub current_session_id: String,
    pub last_perception: Option<PerceivedInput>,
    pub last_decision: Option<ActionDecision>,
    pub conversation_history: Vec<ConversationTurn>,
    pub active_context: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub turn_id: String,
    pub user_input: String,
    pub perception: Option<PerceivedInput>,
    pub decision: Option<ActionDecision>,
    pub response: String,
    pub success: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveResponse {
    pub content: String,
    pub thoughts: Vec<String>,
    pub actions_taken: Vec<String>,
    pub confidence: f64,
    pub needs_clarification: bool,
    pub suggested_follow_ups: Vec<String>,
    pub learning_occurred: bool,
    pub goals_updated: bool,
}

impl CognitiveLoop {
    pub fn new(config: &Config) -> Result<Self> {
        Self::with_inner_monologue(config, None)
    }
    
    pub fn with_inner_monologue(config: &Config, inner_monologue: Option<Arc<InnerMonologue>>) -> Result<Self> {
        let workspace_dir = config.workspace_dir.clone();

        Ok(Self {
            perception: PerceptionEngine::new(),
            action_selector: ActionSelector::new(),
            uncertainty: crate::housaky::cognitive::uncertainty::UncertaintyDetector::new(),
            experience_learner: ExperienceLearner::new(&workspace_dir),
            working_memory: Arc::new(WorkingMemoryEngine::new()),
            goal_engine: Arc::new(GoalEngine::new(&workspace_dir)),
            reasoning: Arc::new(ReasoningPipeline::new()),
            meta_cognition: Arc::new(MetaCognitionEngine::new()),
            knowledge_graph: Arc::new(KnowledgeGraphEngine::new(&workspace_dir)),
            inner_monologue: inner_monologue.unwrap_or_else(|| Arc::new(InnerMonologue::new(&workspace_dir))),
            config: CognitiveLoopConfig::default(),
            state: Arc::new(RwLock::new(CognitiveState {
                total_turns: 0,
                successful_actions: 0,
                failed_actions: 0,
                reflections_count: 0,
                skills_learned: 0,
                current_session_id: format!("session_{}", uuid::Uuid::new_v4()),
                last_perception: None,
                last_decision: None,
                conversation_history: Vec::new(),
                active_context: HashMap::new(),
            })),
            workspace_dir,
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing Cognitive Loop...");

        self.goal_engine.load_goals().await?;
        self.knowledge_graph.load_graph().await?;
        self.inner_monologue.load().await?;
        self.experience_learner.load_state().await?;

        info!("Cognitive Loop initialized successfully");
        Ok(())
    }

    pub async fn process(
        &self,
        user_input: &str,
        provider: &dyn Provider,
        model: &str,
        available_tools: &[&dyn Tool],
    ) -> Result<CognitiveResponse> {
        let start_time = std::time::Instant::now();
        info!(
            "Processing user input: {}",
            user_input.chars().take(50).collect::<String>()
        );

        let perception = self.perceive(user_input, provider, model).await?;

        let uncertainty = if self.config.enable_uncertainty_detection {
            self.assess_uncertainty(user_input, provider, model).await?
        } else {
            self.default_uncertainty()
        };

        let similar_experiences = self
            .experience_learner
            .find_similar_experiences(&perception)
            .await;
        let applicable_patterns = self
            .experience_learner
            .get_applicable_patterns(&perception)
            .await;

        let decision = self
            .decide_action(
                &perception,
                &uncertainty,
                available_tools,
                provider,
                model,
                &similar_experiences,
                &applicable_patterns,
            )
            .await?;

        let (response, outcome) = self
            .execute_decision(&decision, &perception, provider, model, available_tools)
            .await?;

        if self.config.enable_learning {
            self.experience_learner
                .record_experience(&perception, &decision, &outcome)
                .await?;
        }
        
        let thought = format!(
            "Processed {}: intent={:?}, action={:?}, confidence={:.0}%",
            user_input,
            perception.intent.primary,
            decision.action,
            response.confidence * 100.0
        );
        let _ = self.inner_monologue.add_thought(&thought, response.confidence).await;

        self.update_state(
            &perception,
            &decision,
            matches!(
                outcome.result,
                crate::housaky::cognitive::action_selector::ActionResult::Success { .. }
            ),
            crate::util::time::duration_ms_u64(start_time.elapsed()),
        )
        .await;

        if self.config.enable_reflection
            && self.state.read().await.total_turns % u64::from(self.config.reflection_frequency)
                == 0
        {
            self.reflect(&perception, &decision, &response).await?;
        }

        Ok(response)
    }

    async fn perceive(
        &self,
        input: &str,
        provider: &dyn Provider,
        model: &str,
    ) -> Result<PerceivedInput> {
        let basic_perception = self.perception.perceive(input).await?;

        if basic_perception.ambiguity_level > 0.5 || basic_perception.intent.confidence < 0.6 {
            self.perception
                .perceive_with_llm(input, provider, model)
                .await
        } else {
            Ok(basic_perception)
        }
    }

    async fn assess_uncertainty(
        &self,
        input: &str,
        provider: &dyn Provider,
        model: &str,
    ) -> Result<UncertaintyAssessment> {
        let basic = self.uncertainty.assess(input, 0.7, &[]).await?;

        if basic.overall_uncertainty > 0.4 {
            self.uncertainty
                .assess_with_llm(input, "Processing input", provider, model)
                .await
        } else {
            Ok(basic)
        }
    }

    fn default_uncertainty(&self) -> UncertaintyAssessment {
        UncertaintyAssessment {
            overall_uncertainty: 0.2,
            sources: vec![],
            confidence_intervals: HashMap::new(),
            calibration_score: 0.7,
            should_ask_clarification: false,
            clarification_questions: vec![],
            alternative_interpretations: vec![],
            knowledge_gaps: vec![],
        }
    }

    async fn decide_action(
        &self,
        perception: &PerceivedInput,
        uncertainty: &UncertaintyAssessment,
        tools: &[&dyn Tool],
        provider: &dyn Provider,
        model: &str,
        _experiences: &[crate::housaky::cognitive::experience_learner::Experience],
        patterns: &[crate::housaky::cognitive::experience_learner::Pattern],
    ) -> Result<ActionDecision> {
        let basic_decision = self
            .action_selector
            .select_action(perception, uncertainty, tools)
            .await?;

        if !patterns.is_empty() {
            if let Some(best_pattern) = patterns.first() {
                if best_pattern.confidence > basic_decision.confidence {
                    return Ok(ActionDecision {
                        reasoning: format!(
                            "Based on learned pattern: {}",
                            best_pattern.description
                        ),
                        confidence: best_pattern.confidence,
                        ..basic_decision
                    });
                }
            }
        }

        if uncertainty.overall_uncertainty > 0.3 {
            self.action_selector
                .select_action_with_llm(perception, uncertainty, tools, provider, model)
                .await
        } else {
            Ok(basic_decision)
        }
    }

    async fn execute_decision(
        &self,
        decision: &ActionDecision,
        perception: &PerceivedInput,
        provider: &dyn Provider,
        model: &str,
        tools: &[&dyn Tool],
    ) -> Result<(CognitiveResponse, ActionOutcome)> {
        let start_time = std::time::Instant::now();
        let mut thoughts = Vec::new();
        let mut actions_taken = Vec::new();

        thoughts.push(format!("Intent detected: {:?}", perception.intent.primary));
        thoughts.push(format!("Confidence: {:.0}%", decision.confidence * 100.0));

        let (content, success, needs_clarification) = match &decision.action {
            crate::housaky::cognitive::action_selector::SelectedAction::Respond {
                content,
                needs_clarification,
                suggested_follow_ups: _,
            } => {
                let response = if content.is_empty() {
                    self.generate_response(perception, decision, provider, model)
                        .await?
                } else {
                    content.clone()
                };

                thoughts.push("Generated response".to_string());
                (response, true, *needs_clarification)
            }

            crate::housaky::cognitive::action_selector::SelectedAction::UseTool {
                tool_name,
                arguments,
                ..
            } => {
                thoughts.push(format!("Using tool: {}", tool_name));
                actions_taken.push(format!("tool:{}", tool_name));

                if let Some(tool) = tools.iter().find(|t| t.name() == tool_name) {
                    match tool.execute(arguments.clone()).await {
                        Ok(result) => {
                            let response = format!(
                                "Tool {} executed successfully:\n{}",
                                tool_name, result.output
                            );
                            thoughts.push(format!(
                                "Tool result: {}",
                                result.output.chars().take(100).collect::<String>()
                            ));
                            (response, result.success, false)
                        }
                        Err(e) => {
                            let response = format!("Tool {} failed: {}", tool_name, e);
                            thoughts.push(format!("Tool error: {}", e));
                            (response, false, false)
                        }
                    }
                } else {
                    let response = format!("Tool '{}' not found", tool_name);
                    (response, false, false)
                }
            }

            crate::housaky::cognitive::action_selector::SelectedAction::Clarify {
                questions,
                assumptions,
            } => {
                let mut response = String::from("I need some clarification:\n");
                for q in questions {
                    writeln!(response, "- {q}").ok();
                }
                if !assumptions.is_empty() {
                    response.push_str("\nI'm assuming:\n");
                    for a in assumptions {
                        writeln!(response, "- {a}").ok();
                    }
                }
                thoughts.push("Requesting clarification".to_string());
                (response, true, true)
            }

            crate::housaky::cognitive::action_selector::SelectedAction::CreateGoal {
                title,
                description,
                priority,
            } => {
                let goal = crate::housaky::goal_engine::Goal {
                    title: title.clone(),
                    description: description.clone(),
                    priority: priority.clone(),
                    ..Default::default()
                };
                self.goal_engine.add_goal(goal).await?;
                let response = format!("Created goal: {}", title);
                thoughts.push("Created new goal".to_string());
                actions_taken.push("create_goal".to_string());
                (response, true, false)
            }

            crate::housaky::cognitive::action_selector::SelectedAction::Learn {
                topic,
                source,
                ..
            } => {
                let response = format!("Learning about {} from {}", topic, source);
                thoughts.push(format!("Initiated learning: {}", topic));
                actions_taken.push(format!("learn:{}", topic));
                (response, true, false)
            }

            crate::housaky::cognitive::action_selector::SelectedAction::Reflect {
                trigger, ..
            } => {
                let reflection = self.meta_cognition.reflect(trigger).await?;
                let response = format!("Reflection complete. Mood: {:?}", reflection.mood);
                thoughts.push("Performed reflection".to_string());
                actions_taken.push("reflect".to_string());
                (response, true, false)
            }

            crate::housaky::cognitive::action_selector::SelectedAction::Delegate {
                agent_type,
                task_description,
                ..
            } => {
                let response = format!("Delegating to {}: {}", agent_type, task_description);
                thoughts.push(format!("Delegated to: {}", agent_type));
                actions_taken.push(format!("delegate:{}", agent_type));
                (response, true, false)
            }

            crate::housaky::cognitive::action_selector::SelectedAction::Wait { reason, .. } => {
                let response = format!("Waiting: {}", reason);
                thoughts.push("Waiting".to_string());
                (response, true, false)
            }
        };

        let duration_ms = crate::util::time::duration_ms_u64(start_time.elapsed());

        let outcome = ActionOutcome {
            decision_id: format!("outcome_{}", uuid::Uuid::new_v4()),
            action: decision.action.clone(),
            result: if success {
                ActionResult::Success {
                    output: content.clone(),
                }
            } else {
                ActionResult::Failure {
                    error: content.clone(),
                    recoverable: true,
                }
            },
            duration_ms,
            side_effects: actions_taken.clone(),
            user_feedback: None,
        };

        let response = CognitiveResponse {
            content,
            thoughts,
            actions_taken: actions_taken.clone(),
            confidence: decision.confidence,
            needs_clarification,
            suggested_follow_ups: match &decision.action {
                crate::housaky::cognitive::action_selector::SelectedAction::Respond {
                    suggested_follow_ups,
                    ..
                } => suggested_follow_ups.clone(),
                _ => vec![],
            },
            learning_occurred: self.config.enable_learning,
            goals_updated: actions_taken.iter().any(|a| a.contains("goal")),
        };

        Ok((response, outcome))
    }

    async fn generate_response(
        &self,
        perception: &PerceivedInput,
        decision: &ActionDecision,
        provider: &dyn Provider,
        model: &str,
    ) -> Result<String> {
        let memory_context = self.working_memory.get_context(2000).await;
        let active_goals = self.goal_engine.get_active_goals().await;

        let mut system_prompt = String::from(
            "You are Housaky, an AGI assistant with advanced reasoning capabilities.\n\n",
        );

        if !memory_context.is_empty() {
            writeln!(system_prompt, "## Relevant Context\n{memory_context}\n").ok();
        }

        if !active_goals.is_empty() {
            system_prompt.push_str("## Active Goals\n");
            for goal in active_goals.iter().take(3) {
                #[allow(clippy::cast_possible_truncation)]
                let progress_pct = (goal.progress * 100.0).round() as i32;
                writeln!(system_prompt, "- {} ({}% complete)", goal.title, progress_pct).ok();
            }
            system_prompt.push('\n');
        }

        writeln!(system_prompt, "## User Intent: {:?}", perception.intent.primary).ok();
        writeln!(system_prompt, "## Detected Topics: {:?}", perception.topics).ok();
        writeln!(system_prompt, "## Confidence: {:.0}%\n", decision.confidence * 100.0).ok();
        system_prompt.push_str("Respond helpfully and accurately. If uncertain, say so.");

        let state = self.state.read().await;
        let history: Vec<ChatMessage> = state
            .conversation_history
            .iter()
            .rev()
            .take(10)
            .rev()
            .flat_map(|turn| {
                let mut msgs = vec![ChatMessage {
                    role: "user".to_string(),
                    content: turn.user_input.clone(),
                }];
                if !turn.response.is_empty() {
                    msgs.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: turn.response.clone(),
                    });
                }
                msgs
            })
            .collect();
        drop(state);

        let response = if history.is_empty() {
            provider
                .chat_with_system(Some(&system_prompt), &perception.raw_input, model, 0.7)
                .await?
        } else {
            let mut full_history = vec![ChatMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            }];
            full_history.extend(history);
            full_history.push(ChatMessage {
                role: "user".to_string(),
                content: perception.raw_input.clone(),
            });
            provider
                .chat_with_history(&full_history, model, 0.7)
                .await?
        };

        self.working_memory
            .add(
                &format!("User: {} | Assistant: {}", perception.raw_input, response),
                crate::housaky::working_memory::MemoryImportance::Normal,
                [(
                    "intent".to_string(),
                    format!("{:?}", perception.intent.primary),
                )]
                .into_iter()
                .collect(),
            )
            .await?;

        Ok(response)
    }

    async fn update_state(
        &self,
        perception: &PerceivedInput,
        decision: &ActionDecision,
        success: bool,
        duration_ms: u64,
    ) {
        let mut state = self.state.write().await;

        state.total_turns += 1;
        if success {
            state.successful_actions += 1;
        } else {
            state.failed_actions += 1;
        }

        let turn = ConversationTurn {
            turn_id: format!("turn_{}", uuid::Uuid::new_v4()),
            user_input: perception.raw_input.clone(),
            perception: Some(perception.clone()),
            decision: Some(decision.clone()),
            response: String::new(),
            success,
            duration_ms,
        };

        state.conversation_history.push(turn);

        if state.conversation_history.len() > 100 {
            state.conversation_history.remove(0);
        }

        state.last_perception = Some(perception.clone());
        state.last_decision = Some(decision.clone());
    }

    async fn reflect(
        &self,
        perception: &PerceivedInput,
        decision: &ActionDecision,
        response: &CognitiveResponse,
    ) -> Result<()> {
        info!("Performing periodic reflection...");

        let trigger = format!(
            "Turn {} - Intent: {:?}, Action: {:?}, Success: {}",
            self.state.read().await.total_turns,
            perception.intent.primary,
            decision.action,
            response.confidence > 0.5
        );

        let _ = self.meta_cognition.reflect(&trigger).await?;

        self.inner_monologue
            .add_thought(
                &format!(
                    "Reflected on {:?} - Confidence: {:.0}%",
                    perception.intent.primary,
                    response.confidence * 100.0
                ),
                response.confidence,
            )
            .await?;

        let mut state = self.state.write().await;
        state.reflections_count += 1;

        Ok(())
    }

    pub async fn get_state(&self) -> CognitiveState {
        self.state.read().await.clone()
    }

    pub async fn get_metrics(&self) -> CognitiveMetrics {
        let state = self.state.read().await;
        let learning_stats = self.experience_learner.get_learning_stats().await;
        let memory_stats = self.working_memory.get_stats().await;
        let goal_stats = self.goal_engine.get_goal_stats().await;

        CognitiveMetrics {
            total_turns: state.total_turns,
            successful_actions: state.successful_actions,
            failed_actions: state.failed_actions,
            success_rate: if state.total_turns > 0 {
                state.successful_actions as f64 / state.total_turns as f64
            } else {
                0.0
            },
            reflections_count: state.reflections_count,
            learning_stats,
            memory_stats,
            goal_stats,
        }
    }

    pub async fn run_continuous(
        self: Arc<Self>,
        mut receiver: mpsc::Receiver<CognitiveRequest>,
        sender: mpsc::Sender<CognitiveResponse>,
        provider_name: String,
        model: String,
        api_key: Option<String>,
    ) -> Result<()> {
        info!("Starting continuous cognitive loop");

        let provider = create_provider(&provider_name, api_key.as_deref())?;

        // In continuous mode we still want full tool access (subject to SecurityPolicy).
        // This is required for genuine agentic behavior (plan → act → observe loops).
        let security = Arc::new(crate::security::SecurityPolicy::default());
        let tools: Vec<Box<dyn Tool>> = crate::tools::default_tools(security);
        let tool_refs: Vec<&dyn Tool> = tools.iter().map(|t| t.as_ref()).collect();

        while let Some(request) = receiver.recv().await {
            let response = self
                .process(&request.input, provider.as_ref(), &model, &tool_refs)
                .await?;

            sender.send(response).await?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveRequest {
    pub input: String,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveMetrics {
    pub total_turns: u64,
    pub successful_actions: u64,
    pub failed_actions: u64,
    pub success_rate: f64,
    pub reflections_count: u64,
    pub learning_stats: crate::housaky::cognitive::experience_learner::LearningStats,
    pub memory_stats: crate::housaky::working_memory::WorkingMemoryStats,
    pub goal_stats: crate::housaky::goal_engine::GoalStats,
}

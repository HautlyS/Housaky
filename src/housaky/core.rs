use crate::config::Config;
use crate::housaky::agent::Agent;
use crate::housaky::cognitive::cognitive_loop::{CognitiveLoop, CognitiveResponse};
use crate::housaky::goal_engine::{Goal, GoalEngine, GoalPriority, GoalStatus};
use crate::housaky::inner_monologue::InnerMonologue;
use crate::housaky::knowledge_graph::KnowledgeGraphEngine;
use crate::housaky::memory::consolidation::MemoryConsolidator;
use crate::housaky::memory::hierarchical::HierarchicalMemory;
use crate::housaky::meta_cognition::MetaCognitionEngine;
use crate::housaky::reasoning_pipeline::{ReasoningPipeline, ReasoningResult};
use crate::housaky::streaming::streaming::StreamingManager;
use crate::housaky::tool_creator::ToolCreator;
use crate::housaky::working_memory::{MemoryImportance, WorkingMemoryEngine};
use crate::providers::Provider;
use crate::tools::Tool;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct HousakyCore {
    pub agent: Arc<Agent>,
    pub goal_engine: Arc<GoalEngine>,
    pub working_memory: Arc<WorkingMemoryEngine>,
    pub meta_cognition: Arc<MetaCognitionEngine>,
    pub knowledge_graph: Arc<KnowledgeGraphEngine>,
    pub tool_creator: Arc<ToolCreator>,
    pub inner_monologue: Arc<InnerMonologue>,
    pub reasoning_pipeline: Arc<ReasoningPipeline>,
    pub cognitive_loop: Arc<CognitiveLoop>,
    pub hierarchical_memory: Arc<HierarchicalMemory>,
    pub memory_consolidator: Arc<MemoryConsolidator>,
    pub streaming_manager: Arc<StreamingManager>,
    state: Arc<RwLock<HousakyCoreState>>,
    config: HousakyCoreConfig,
    workspace_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HousakyCoreState {
    pub is_active: bool,
    pub total_turns: u64,
    pub successful_actions: u64,
    pub failed_actions: u64,
    pub total_reflections: u64,
    pub skills_created: u64,
    pub goals_completed: u64,
    pub current_focus: Option<String>,
    pub last_thought: Option<String>,
    pub last_action: Option<String>,
    pub confidence_level: f64,
    pub evolution_stage: u32,
    pub uptime_seconds: u64,
    pub started_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HousakyCoreConfig {
    pub enabled: bool,
    pub auto_reflect: bool,
    pub auto_create_skills: bool,
    pub reasoning_depth: u32,
    pub max_working_memory_tokens: usize,
    pub goal_priority_threshold: GoalPriority,
    pub enable_inner_monologue: bool,
    pub monologue_persistence: bool,
    pub confidence_threshold: f64,
    pub self_improvement_interval_seconds: u64,
}

impl Default for HousakyCoreConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_reflect: true,
            auto_create_skills: true,
            reasoning_depth: 3,
            max_working_memory_tokens: 8000,
            goal_priority_threshold: GoalPriority::Medium,
            enable_inner_monologue: true,
            monologue_persistence: true,
            confidence_threshold: 0.7,
            self_improvement_interval_seconds: 120,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AGIDecision {
    pub action: AGIAction,
    pub reasoning: String,
    pub confidence: f64,
    pub goal_context: Option<String>,
    pub thought_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AGIAction {
    UseTool {
        name: String,
        arguments: serde_json::Value,
        goal_id: Option<String>,
    },
    Respond {
        content: String,
        needs_clarification: bool,
    },
    CreateGoal {
        title: String,
        description: String,
        priority: GoalPriority,
    },
    Reflect {
        trigger: String,
    },
    Learn {
        topic: String,
        source: String,
    },
    Wait {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnContext {
    pub user_message: String,
    pub relevant_memories: Vec<String>,
    pub active_goals: Vec<Goal>,
    pub recent_thoughts: Vec<String>,
    pub working_context: String,
}

impl HousakyCore {
    pub fn new(config: &Config) -> Result<Self> {
        let workspace_dir = config.workspace_dir.clone();

        let agent = Agent::new(config)?;
        let goal_engine = Arc::new(GoalEngine::new(&workspace_dir));
        let working_memory = Arc::new(WorkingMemoryEngine::new());
        let meta_cognition = Arc::new(MetaCognitionEngine::new());
        let knowledge_graph = Arc::new(KnowledgeGraphEngine::new(&workspace_dir));
        let tool_creator = Arc::new(ToolCreator::new(&workspace_dir));
        let inner_monologue = Arc::new(InnerMonologue::new(&workspace_dir));
        let reasoning_pipeline = Arc::new(ReasoningPipeline::new());
        let cognitive_loop = Arc::new(CognitiveLoop::with_inner_monologue(config, Some(inner_monologue.clone()))?);
        let hierarchical_memory = Arc::new(HierarchicalMemory::new(Default::default()));
        let memory_consolidator = Arc::new(MemoryConsolidator::new(
            hierarchical_memory.clone(),
            &workspace_dir,
        ));
        let streaming_manager = Arc::new(StreamingManager::new());

        let core_config = HousakyCoreConfig::default();

        let state = Arc::new(RwLock::new(HousakyCoreState {
            is_active: true,
            total_turns: 0,
            successful_actions: 0,
            failed_actions: 0,
            total_reflections: 0,
            skills_created: 0,
            goals_completed: 0,
            current_focus: None,
            last_thought: None,
            last_action: None,
            confidence_level: 0.7,
            evolution_stage: 1,
            uptime_seconds: 0,
            started_at: Utc::now(),
        }));

        Ok(Self {
            agent: Arc::new(agent),
            goal_engine,
            working_memory,
            meta_cognition,
            knowledge_graph,
            tool_creator,
            inner_monologue,
            reasoning_pipeline,
            cognitive_loop,
            hierarchical_memory,
            memory_consolidator,
            streaming_manager,
            state,
            config: core_config,
            workspace_dir,
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing Housaky AGI Core...");

        if let Err(e) = self.goal_engine.load_goals().await {
            return Err(anyhow::anyhow!("Failed to load goals: {}", e));
        }
        
        if let Err(e) = self.knowledge_graph.load_graph().await {
            return Err(anyhow::anyhow!("Failed to load knowledge graph: {}", e));
        }
        
        if let Err(e) = self.inner_monologue.load().await {
            return Err(anyhow::anyhow!("Failed to load inner monologue: {}", e));
        }
        
        if let Err(e) = self.tool_creator.load_tools().await {
            return Err(anyhow::anyhow!("Failed to load tools: {}", e));
        }
        
        if let Err(e) = self.cognitive_loop.initialize().await {
            return Err(anyhow::anyhow!("Failed to initialize cognitive loop: {}", e));
        }

        if let Err(e) = self.initialize_default_goals().await {
            return Err(anyhow::anyhow!("Failed to initialize default goals: {}", e));
        }

        info!("Housaky AGI Core initialized successfully");
        Ok(())
    }

    async fn initialize_default_goals(&self) -> Result<()> {
        let stats = self.goal_engine.get_goal_stats().await;

        if stats.total == 0 {
            let default_goals: Vec<(&str, &str, GoalPriority)> = vec![
                (
                    "Understand user intent accurately",
                    "Parse and comprehend user requests with high fidelity",
                    GoalPriority::Critical,
                ),
                (
                    "Maintain coherent conversation context",
                    "Track conversation flow and recall relevant context",
                    GoalPriority::High,
                ),
                (
                    "Improve reasoning capabilities",
                    "Enhance chain-of-thought and decision-making",
                    GoalPriority::High,
                ),
                (
                    "Learn from interactions",
                    "Extract and store useful knowledge from conversations",
                    GoalPriority::Medium,
                ),
                (
                    "Self-improve continuously",
                    "Identify and address capability gaps",
                    GoalPriority::Medium,
                ),
            ];

            let count = default_goals.len();

            for (title, description, priority) in default_goals {
                let goal = Goal {
                    title: title.to_string(),
                    description: description.to_string(),
                    priority,
                    status: GoalStatus::Pending,
                    ..Default::default()
                };
                self.goal_engine.add_goal(goal).await?;
            }

            info!("Initialized {} default AGI goals", count);
        }

        Ok(())
    }

    pub async fn prepare_context(&self, user_message: &str) -> Result<TurnContext> {
        let memories = self.working_memory.search(user_message, 5).await;
        let relevant_memories: Vec<String> = memories.iter().map(|m| m.content.clone()).collect();

        let active_goals = self.goal_engine.get_active_goals().await;

        let recent_thoughts = if self.config.enable_inner_monologue {
            self.inner_monologue.get_recent(3).await
        } else {
            vec![]
        };

        let working_context = self.working_memory.get_context(2000).await;

        Ok(TurnContext {
            user_message: user_message.to_string(),
            relevant_memories,
            active_goals,
            recent_thoughts,
            working_context,
        })
    }

    pub async fn process_with_reasoning(
        &self,
        provider: &dyn Provider,
        model: &str,
        user_message: &str,
        available_tools: &[&dyn Tool],
    ) -> Result<AGIDecision> {
        let context = self.prepare_context(user_message).await?;

        let top_goal = context.active_goals.first().cloned();
        let goal_context = top_goal.as_ref().map(|g| {
            format!(
                "Active Goal: {} ({}% complete)",
                g.title,
                (g.progress * 100.0) as i32
            )
        });

        let tool_names: Vec<&str> = available_tools.iter().map(|t| t.name()).collect();
        let reasoning = self
            .reasoning_pipeline
            .reason_react(provider, model, user_message, &tool_names, &context)
            .await?;

        let action = self
            .derive_action_from_reasoning(&reasoning, top_goal.as_ref())
            .await?;

        let confidence = reasoning.confidence;

        let thought_id = if self.config.enable_inner_monologue {
            let thought = format!(
                "User asked: '{}'. Reasoning: {}. Conclusion: {}",
                user_message.chars().take(50).collect::<String>(),
                reasoning.summary.chars().take(100).collect::<String>(),
                format!("{:?}", action).chars().take(50).collect::<String>()
            );
            Some(
                self.inner_monologue
                    .add_thought(&thought, confidence)
                    .await?,
            )
        } else {
            None
        };

        let mut state = self.state.write().await;
        state.total_turns += 1;
        state.last_thought = Some(reasoning.summary.clone());
        state.last_action = Some(format!("{:?}", action));
        state.confidence_level = f64::midpoint(state.confidence_level, confidence);
        drop(state);

        if let Some(goal) = &top_goal {
            let progress_delta = if matches!(action, AGIAction::Respond { .. }) {
                0.1
            } else {
                0.05
            };
            tracing::info!("process_with_reasoning: about to update goal progress");
            self.goal_engine
                .update_progress(
                    &goal.id,
                    (goal.progress + progress_delta).min(1.0),
                    &format!(
                        "Processed user message: {}",
                        user_message.chars().take(30).collect::<String>()
                    ),
                )
                .await?;
            tracing::info!("process_with_reasoning: goal progress updated");
        }

        tracing::info!("process_with_reasoning: about to return AGIDecision");
        Ok(AGIDecision {
            action,
            reasoning: reasoning.summary,
            confidence,
            goal_context,
            thought_id,
        })
    }

    async fn derive_action_from_reasoning(
        &self,
        reasoning: &ReasoningResult,
        top_goal: Option<&Goal>,
    ) -> Result<AGIAction> {
        if reasoning.suggested_tools.is_empty() {
            return Ok(AGIAction::Respond {
                content: reasoning.conclusion.clone(),
                needs_clarification: reasoning.confidence < self.config.confidence_threshold,
            });
        }

        let tool = &reasoning.suggested_tools[0];

        if tool.name == "memory_store" || tool.name == "memory_recall" {
            return Ok(AGIAction::Learn {
                topic: tool
                    .arguments
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                source: "conversation".to_string(),
            });
        }

        Ok(AGIAction::UseTool {
            name: tool.name.clone(),
            arguments: tool.arguments.clone(),
            goal_id: top_goal.map(|g| g.id.clone()),
        })
    }

    pub async fn record_action_result(
        &self,
        success: bool,
        output: &str,
        goal_id: Option<&str>,
    ) -> Result<()> {
        let mut state = self.state.write().await;

        if success {
            state.successful_actions += 1;
        } else {
            state.failed_actions += 1;
        }
        drop(state);

        let importance = if success {
            MemoryImportance::Normal
        } else {
            MemoryImportance::High
        };

        let mut context = HashMap::new();
        context.insert("success".to_string(), success.to_string());
        context.insert("timestamp".to_string(), Utc::now().to_rfc3339());

        self.working_memory.add(output, importance, context).await?;

        if let Some(gid) = goal_id {
            if success {
                let goal = self.goal_engine.get_next_goal().await;
                if let Some(g) = goal {
                    if g.id == gid {
                        self.goal_engine
                            .update_progress(gid, g.progress + 0.2, "Action completed successfully")
                            .await?;
                    }
                }
            }
        }

        if self.config.auto_reflect && !success {
            let trigger = format!(
                "Failed action: {}",
                output.chars().take(100).collect::<String>()
            );
            self.meta_cognition.reflect(&trigger).await?;

            let mut state = self.state.write().await;
            state.total_reflections += 1;
        }

        Ok(())
    }

    pub async fn reflect_on_turn(&self, trigger: &str) -> Result<()> {
        if !self.config.auto_reflect {
            return Ok(());
        }

        let reflection = self.meta_cognition.reflect(trigger).await?;

        let mut state = self.state.write().await;
        state.total_reflections += 1;
        state.confidence_level =
            (state.confidence_level + reflection.confidence_delta).clamp(0.0, 1.0);

        for insight in &reflection.insights {
            if insight.actionable {
                self.inner_monologue
                    .add_thought(&format!("Insight: {}", insight.content), insight.confidence)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn store_knowledge(&self, content: &str, source: &str) -> Result<()> {
        let entity_ids = self
            .knowledge_graph
            .extract_from_text(content, source)
            .await?;

        for entity_id in &entity_ids {
            let related = self
                .knowledge_graph
                .get_related_entities(entity_id, 2)
                .await;
            for (entity, rel_type, depth) in related {
                self.inner_monologue
                    .add_thought(
                        &format!(
                            "Learned: {} is {:?} to {} (depth {})",
                            entity_id.as_str(),
                            rel_type,
                            entity.name,
                            depth
                        ),
                        0.8,
                    )
                    .await?;
            }
        }

        self.working_memory
            .add(
                content,
                MemoryImportance::High,
                [("source".to_string(), source.to_string())]
                    .into_iter()
                    .collect(),
            )
            .await?;

        Ok(())
    }

    pub async fn get_state(&self) -> HousakyCoreState {
        let mut state = self.state.read().await.clone();
        state.uptime_seconds = (Utc::now() - state.started_at).num_seconds() as u64;
        state
    }

    pub async fn get_dashboard_metrics(&self) -> DashboardMetrics {
        let state = self.state.read().await;
        let goal_stats = self.goal_engine.get_goal_stats().await;
        let mem_stats = self.working_memory.get_stats().await;
        let self_model = self.meta_cognition.get_self_model().await;
        let graph_stats = self.knowledge_graph.get_stats().await;

        DashboardMetrics {
            is_active: state.is_active,
            total_turns: state.total_turns,
            successful_actions: state.successful_actions,
            failed_actions: state.failed_actions,
            success_rate: if state.total_turns > 0 {
                state.successful_actions as f64 / state.total_turns as f64
            } else {
                0.0
            },
            confidence_level: state.confidence_level,
            evolution_stage: state.evolution_stage,
            active_goals: goal_stats.in_progress,
            pending_goals: goal_stats.pending,
            completed_goals: goal_stats.completed,
            memory_items: mem_stats.short_term_count + mem_stats.long_term_count,
            memory_tokens: mem_stats.short_term_tokens,
            knowledge_entities: graph_stats.entity_count,
            knowledge_relations: graph_stats.relation_count,
            capabilities: self_model.capabilities,
            current_focus: state.current_focus.clone(),
            last_thought: state.last_thought.clone(),
            uptime_seconds: state.uptime_seconds,
        }
    }

    pub async fn process_with_cognitive_loop(
        &self,
        user_input: &str,
        provider: &dyn Provider,
        model: &str,
        available_tools: &[&dyn Tool],
    ) -> Result<CognitiveResponse> {
        info!("Processing with full cognitive loop...");

        let response = self
            .cognitive_loop
            .process(user_input, provider, model, available_tools)
            .await?;

        let mut state = self.state.write().await;
        state.total_turns += 1;
        if response.learning_occurred {
            state.skills_created += 1;
        }
        state.confidence_level =
            (state.confidence_level * 0.9 + response.confidence * 0.1).clamp(0.0, 1.0);
        state.last_thought = response.thoughts.first().cloned();
        drop(state);

        if response.goals_updated {
            let stats = self.goal_engine.get_goal_stats().await;
            let mut state = self.state.write().await;
            state.goals_completed = stats.completed as u64;
        }

        Ok(response)
    }

    pub async fn run_memory_consolidation(&self) -> Result<()> {
        info!("Running memory consolidation...");
        self.memory_consolidator
            .run_periodic_consolidation()
            .await?;

        let mut state = self.state.write().await;
        state.evolution_stage += 1;

        Ok(())
    }

    pub async fn run_self_improvement(
        &self,
        _provider: &dyn Provider,
        _model: &str,
    ) -> Result<Vec<String>> {
        info!("Running self-improvement cycle...");
        let mut improvements = Vec::new();

        let reflection = self
            .meta_cognition
            .reflect("Periodic self-improvement cycle")
            .await?;

        for observation in &reflection.observations {
            let thought = format!("Observation: {}", observation.content);
            self.inner_monologue
                .add_thought(&thought, observation.importance)
                .await?;
            improvements.push(format!("Observed: {}", observation.content));
        }

        for insight in &reflection.insights {
            if insight.actionable {
                let thought = format!("Actionable insight: {}", insight.content);
                self.inner_monologue
                    .add_thought(&thought, insight.confidence)
                    .await?;

                if insight.content.contains("goal") || insight.content.contains("objective") {
                    let goal = Goal {
                        title: insight.content.chars().take(100).collect(),
                        description: insight.content.clone(),
                        priority: GoalPriority::Medium,
                        status: GoalStatus::Pending,
                        ..Default::default()
                    };
                    self.goal_engine.add_goal(goal).await?;
                    improvements.push("Created new goal from insight".to_string());
                }
            }
        }

        for action in &reflection.actions {
            let thought = format!("Improvement action: {}", action.description);
            self.inner_monologue.add_thought(&thought, 0.8).await?;
            improvements.push(format!("Action: {}", action.description));
        }

        let mut state = self.state.write().await;
        state.total_reflections += 1;
        state.confidence_level =
            (state.confidence_level + reflection.confidence_delta).clamp(0.1, 1.0);

        info!(
            "Self-improvement cycle complete: {} improvements identified",
            improvements.len()
        );
        Ok(improvements)
    }

    pub async fn auto_create_tool(
        &self,
        _provider: &dyn Provider,
        _model: &str,
    ) -> Result<Option<String>> {
        use crate::housaky::tool_creator::{ToolGenerationRequest, ToolKind};

        let goals = self.goal_engine.get_active_goals().await;

        for goal in goals.iter().filter(|g| g.progress < 0.5) {
            let tool_name = format!("tool_for_goal_{}", goal.id.replace('-', "_"));
            let spec = format!(
                "Tool to help achieve goal: {}\nDescription: {}",
                goal.title, goal.description
            );

            let request = ToolGenerationRequest {
                name: tool_name.clone(),
                description: spec,
                kind: ToolKind::Composite,
                examples: vec![],
                constraints: vec![],
            };

            match self.tool_creator.generate_tool(request).await {
                Ok(generated_tool) => {
                    let mut state = self.state.write().await;
                    state.skills_created += 1;

                    self.inner_monologue
                        .add_thought(
                            &format!(
                                "Created tool '{}' for goal: {}",
                                generated_tool.id, goal.title
                            ),
                            0.9,
                        )
                        .await?;

                    return Ok(Some(generated_tool.id));
                }
                Err(e) => {
                    info!("Tool creation attempt failed: {}", e);
                }
            }
        }

        Ok(None)
    }

    pub async fn learn_from_experience(&self) -> Result<Vec<String>> {
        info!("Learning from past experiences...");

        let episodes = self.hierarchical_memory.get_recent_episodes(10).await;
        let patterns = self.memory_consolidator.analyze_patterns(&episodes);

        let mut learned = Vec::new();

        for pattern in patterns.iter().take(3) {
            if pattern.success_rate > 0.7 {
                match self
                    .memory_consolidator
                    .create_procedure_from_pattern(pattern)
                    .await
                {
                    Ok(procedure_id) => {
                        learned.push(format!(
                            "Learned procedure: {} ({}% success)",
                            pattern.description,
                            (pattern.success_rate * 100.0) as i32
                        ));

                        self.inner_monologue
                            .add_thought(
                                &format!(
                                    "Extracted procedure {} from {} occurrences",
                                    procedure_id, pattern.occurrence_count
                                ),
                                pattern.success_rate,
                            )
                            .await?;
                    }
                    Err(e) => {
                        info!("Failed to create procedure: {}", e);
                    }
                }
            }
        }

        if !learned.is_empty() {
            let mut state = self.state.write().await;
            state.skills_created += learned.len() as u64;
        }

        Ok(learned)
    }

    pub async fn stream_response(
        &self,
        provider: &dyn Provider,
        model: &str,
        messages: &[crate::providers::ChatMessage],
    ) -> Result<String> {
        info!("Streaming response via StreamingManager...");

        let response = self
            .streaming_manager
            .stream_chat(provider, model, messages, 0.7)
            .await?;

        let stats = self.streaming_manager.get_stats().await;
        info!(
            "Stream complete: {} tokens in {}ms ({:.1} tok/s)",
            stats.token_count, stats.elapsed_ms, stats.tokens_per_second
        );

        Ok(response)
    }

    pub async fn process_with_streaming(
        &self,
        provider: &dyn Provider,
        model: &str,
        user_message: &str,
        available_tools: &[&dyn Tool],
    ) -> Result<AGIDecision> {
        info!("Processing with streaming response...");

        let context = self.prepare_context(user_message).await?;

        let top_goal = context.active_goals.first().cloned();
        let goal_context = top_goal.as_ref().map(|g| {
            format!(
                "Active Goal: {} ({}% complete)",
                g.title,
                (g.progress * 100.0) as i32
            )
        });

        let tool_names: Vec<&str> = available_tools.iter().map(|t| t.name()).collect();

        let messages = vec![
            crate::providers::ChatMessage::system("You are Housaky, an AGI assistant."),
            crate::providers::ChatMessage::user(user_message),
        ];

        let streaming_response = self
            .streaming_manager
            .stream_chat(provider, model, &messages, 0.7)
            .await?;

        let reasoning = self
            .reasoning_pipeline
            .reason_react(provider, model, user_message, &tool_names, &context)
            .await?;

        let action = self
            .derive_action_from_reasoning(&reasoning, top_goal.as_ref())
            .await?;

        let confidence = reasoning.confidence;

        let thought_id = if self.config.enable_inner_monologue {
            let thought = format!(
                "User asked: '{}'. Streaming response length: {} chars",
                user_message.chars().take(50).collect::<String>(),
                streaming_response.len()
            );
            Some(
                self.inner_monologue
                    .add_thought(&thought, confidence)
                    .await?,
            )
        } else {
            None
        };

        let mut state = self.state.write().await;
        state.total_turns += 1;
        state.last_thought = Some(format!("Streamed {} chars", streaming_response.len()));
        state.last_action = Some(format!("{:?}", action));
        state.confidence_level = f64::midpoint(state.confidence_level, confidence);
        drop(state);

        Ok(AGIDecision {
            action,
            reasoning: reasoning.summary,
            confidence,
            goal_context,
            thought_id,
        })
    }

    pub async fn get_streaming_stats(&self) -> crate::housaky::streaming::streaming::StreamStats {
        self.streaming_manager.get_stats().await
    }
    
    pub async fn decompose_task(&self, task: &str) -> Result<Vec<SubTask>> {
        info!("Decomposing task: {}", task);
        
        let complexity_indicators = [
            " and ", " then ", " also ", " plus ", " moreover",
            "first", "second", "third", "finally",
            "step 1", "step 2", "step 3",
            "multiple", "several", "various",
        ];
        
        let is_complex = complexity_indicators.iter()
            .any(|i| task.to_lowercase().contains(i));
        
        if !is_complex {
            return Ok(vec![SubTask {
                id: format!("sub_{}", uuid::Uuid::new_v4()),
                description: task.to_string(),
                status: SubTaskStatus::Pending,
                dependencies: vec![],
            }]);
        }
        
        let parts: Vec<&str> = task
            .split(|c| c == ',' || c == '.' || c == ';' || c == '\n')
            .filter(|s| !s.trim().is_empty())
            .collect();
        
        let mut subtasks = Vec::new();
        let mut dependencies: Vec<String> = vec![];
        
        for (_i, part) in parts.iter().enumerate() {
            let clean = part.trim();
            if clean.is_empty() || clean.len() < 3 {
                continue;
            }
            
            let subtask = SubTask {
                id: format!("sub_{}", uuid::Uuid::new_v4()),
                description: clean.to_string(),
                status: SubTaskStatus::Pending,
                dependencies: dependencies.clone(),
            };
            
            dependencies.push(subtask.id.clone());
            subtasks.push(subtask);
        }
        
        if subtasks.is_empty() {
            subtasks.push(SubTask {
                id: format!("sub_{}", uuid::Uuid::new_v4()),
                description: task.to_string(),
                status: SubTaskStatus::Pending,
                dependencies: vec![],
            });
        }
        
        info!("Decomposed into {} subtasks", subtasks.len());
        Ok(subtasks)
    }

    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down Housaky AGI Core...");

        self.goal_engine.load_goals().await?;
        self.inner_monologue.save().await?;
        self.tool_creator.save_tools().await?;

        let mut state = self.state.write().await;
        state.is_active = false;

        info!("Housaky AGI Core shutdown complete");
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub id: String,
    pub description: String,
    pub status: SubTaskStatus,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubTaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetrics {
    pub is_active: bool,
    pub total_turns: u64,
    pub successful_actions: u64,
    pub failed_actions: u64,
    pub success_rate: f64,
    pub confidence_level: f64,
    pub evolution_stage: u32,
    pub active_goals: usize,
    pub pending_goals: usize,
    pub completed_goals: usize,
    pub memory_items: usize,
    pub memory_tokens: usize,
    pub knowledge_entities: usize,
    pub knowledge_relations: usize,
    pub capabilities: crate::housaky::meta_cognition::CapabilityAssessment,
    pub current_focus: Option<String>,
    pub last_thought: Option<String>,
    pub uptime_seconds: u64,
}

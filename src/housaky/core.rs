#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use crate::config::Config;
use crate::housaky::agent::Agent;
use crate::housaky::agi_integration;
use crate::housaky::alignment::ethics::{
    AGIAction as EthicalAction, EthicalReasoner, EthicalVerdict,
};
use crate::housaky::architecture_search::{ArchitectureSearchEngine, ArchitectureSearchConfig};
use crate::housaky::capability_growth_tracker::CapabilityGrowthTracker;
use crate::housaky::cognitive::cognitive_loop::{CognitiveLoop, CognitiveResponse};
use crate::housaky::cognitive::quantum_planning::QuantumPlanningEngine;
use crate::housaky::cognitive::world_model::{Action, ActionResult, WorldModel};
use crate::housaky::embodiment::EmbodimentController;
use crate::housaky::goal_engine::{Goal, GoalEngine, GoalPriority, GoalStatus};
use crate::housaky::inner_monologue::InnerMonologue;
use crate::housaky::introspection::NaturalLanguageIntrospector;
use crate::housaky::knowledge_acquisition::KnowledgeAcquisitionEngine;
use crate::housaky::knowledge_graph::KnowledgeGraphEngine;
use crate::housaky::knowledge_guided_goal_selector::KnowledgeGuidedGoalSelector;
use crate::housaky::memory::consolidation::MemoryConsolidator;
use crate::housaky::memory::episodic::{EpisodicEventType, EpisodicMemory};
use crate::housaky::memory::hierarchical::{HierarchicalMemory, HierarchicalMemoryConfig};
use crate::housaky::meta_cognition::MetaCognitionEngine;
use crate::housaky::neuromorphic::{NeuromorphicEngine, NeuromorphicConfig};
use crate::housaky::perception::{PerceptualSystem, PerceptualSystemConfig};
use crate::housaky::reasoning_pipeline::{ReasoningPipeline, ReasoningResult};
use crate::housaky::rust_self_improvement::SelfImprovementEngine as RustSelfImprovementEngine;
use crate::housaky::self_improvement_loop::ImprovementExperiment;
use crate::housaky::singularity::{SingularityEngine, SingularityPhaseStatus};
use crate::housaky::streaming::streaming::StreamingManager;
use crate::housaky::swarm::SwarmController;
use crate::housaky::tool_chain_composer::ToolChainComposer;
use crate::housaky::tool_creator::ToolCreator;
use crate::housaky::unified_feedback_loop::UnifiedFeedbackLoop;
use crate::housaky::unified_improvement_orchestrator::UnifiedSelfImprovementOrchestrator as UnifiedImprovementOrchestrator;
use crate::housaky::working_memory::{MemoryImportance, WorkingMemoryEngine};
use crate::providers::Provider;
use crate::quantum::backend::AmazonBraketBackend;
use crate::quantum::{AgiBridgeConfig, QuantumAgiBridge};
use crate::skills::invocation::SkillInvocationEngine;
use crate::tools::Tool;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

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
    pub agi_hub: Arc<agi_integration::AGIIntegrationHub>,
    pub singularity_engine: Arc<RwLock<SingularityEngine>>,
    pub growth_tracker: Arc<CapabilityGrowthTracker>,
    pub ethical_reasoner: Arc<EthicalReasoner>,
    pub world_model: Arc<WorldModel>,
    pub episodic_memory: Arc<EpisodicMemory>,
    /// §10 — Quantum-AGI bridge. `None` when quantum is disabled in config.
    pub quantum_bridge: Option<Arc<QuantumAgiBridge>>,
    /// §10.3 — Quantum-enhanced planning engine. `None` when quantum is disabled.
    pub quantum_planner: Option<Arc<QuantumPlanningEngine>>,
    /// §10.8 — Unified feedback loop with optional quantum edge optimization.
    pub feedback_loop: Arc<UnifiedFeedbackLoop>,
    /// Skill invocation engine for automatic skill triggering
    pub skill_invocation_engine: Arc<SkillInvocationEngine>,
    /// Shared activity log for heartbeat→TUI communication
    activity_log: Arc<std::sync::Mutex<Vec<(String, String)>>>,
    state: Arc<RwLock<HousakyCoreState>>,
    config: HousakyCoreConfig,
    workspace_dir: PathBuf,
    // Phase 2 - Neuromorphic computing
    neuromorphic_engine: Arc<NeuromorphicEngine>,
    // Phase 2 - Swarm intelligence
    swarm_controller: Arc<SwarmController>,
    // Phase 4 - Architecture search
    architecture_search: Arc<ArchitectureSearchEngine>,
    // Phase 4 - Knowledge acquisition
    knowledge_acquirer: Arc<KnowledgeAcquisitionEngine>,
    // Phase 5 - Embodiment (optional, requires hardware)
    embodiment: Option<Arc<EmbodimentController>>,
    // Phase 5 - Perception system
    perception_system: Arc<PerceptualSystem>,
    // Improvement orchestration
    improvement_orchestrator: Arc<UnifiedImprovementOrchestrator>,
    // Rust-specific self-improvement
    rust_self_improvement: Arc<RustSelfImprovementEngine>,
    // Tool chain composition
    tool_chain_composer: Arc<ToolChainComposer>,
    // Knowledge-guided goal selection
    goal_selector: Arc<KnowledgeGuidedGoalSelector>,
    // Natural language introspection
    introspector: Arc<NaturalLanguageIntrospector>,
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
#[allow(clippy::struct_excessive_bools)]
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
        // §10.7 — MetaCognitionEngine gets quantum bridge wired in after bridge init below.
        let meta_cognition_base = MetaCognitionEngine::new();
        // §10.4 — KnowledgeGraphEngine gets quantum bridge wired in after bridge init below.
        let knowledge_graph_base = KnowledgeGraphEngine::new(&workspace_dir);
        let tool_creator = Arc::new(ToolCreator::new(&workspace_dir));
        let inner_monologue = Arc::new(InnerMonologue::new(&workspace_dir));
        // §10 — ReasoningPipeline gets quantum bridge wired in after bridge init below.
        let reasoning_pipeline_base = ReasoningPipeline::new();
        let hierarchical_memory =
            Arc::new(HierarchicalMemory::new(HierarchicalMemoryConfig::default()));
        // §10.4 — MemoryConsolidator gets quantum bridge wired in after bridge init below.
        let memory_consolidator_base =
            MemoryConsolidator::new(hierarchical_memory.clone(), &workspace_dir);
        let streaming_manager = Arc::new(StreamingManager::new());

        let agi_hub = Arc::new(agi_integration::AGIIntegrationHub::new(&workspace_dir));

        let growth_tracker = Arc::new(CapabilityGrowthTracker::new());
        // §10 — SingularityEngine gets quantum wired in below, after bridge init.
        let singularity_engine_base = SingularityEngine::new(growth_tracker.clone());

        let ethical_reasoner = Arc::new(EthicalReasoner::new());

        let world_model = Arc::new(WorldModel::with_storage(&workspace_dir));
        let episodic_memory = Arc::new(EpisodicMemory::new(10_000));

        let core_config = HousakyCoreConfig::default();

        // §10 — Initialize quantum-AGI bridge.
        // For `backend = "braket"` we need async AWS SDK init; use block_in_place
        // so we can call it from this sync constructor.
        let quantum_bridge = if config.quantum.enabled {
            let bridge_config = AgiBridgeConfig {
                max_qubits: config.quantum.max_qubits,
                error_mitigation: config.quantum.error_mitigation,
                transpile: config.quantum.transpile,
                target_device: if config.quantum.backend == "braket" {
                    Some(config.quantum.braket_device_arn.clone())
                } else {
                    None
                },
                quantum_threshold: config.quantum.agi.quantum_threshold,
                cycle_budget_usd: config.quantum.agi.cycle_budget_usd,
                goal_scheduling_shots: config.quantum.shots,
                reasoning_search_shots: config.quantum.shots * 2,
                memory_optimization_shots: config.quantum.shots / 2,
                fitness_eval_shots: config.quantum.shots * 2,
            };
            info!(
                "✨ Quantum-AGI bridge initializing: backend={}, qubits={}",
                config.quantum.backend, config.quantum.max_qubits
            );

            let bridge = if config.quantum.backend == "braket" {
                // Braket backend requires async AWS SDK init — run it on a
                // dedicated thread so we don't block the Tokio executor.
                let device_arn = config.quantum.braket_device_arn.clone();
                let s3_bucket = config.quantum.braket_s3_bucket.clone();
                let s3_prefix = config.quantum.braket_s3_prefix.clone();
                let shots = config.quantum.shots;
                let bc = bridge_config.clone();

                match tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async move {
                        if s3_bucket.is_empty() {
                            warn!("⚠️  braket_s3_bucket not set in config — falling back to local simulator");
                            return Err(anyhow::anyhow!("braket_s3_bucket is empty"));
                        }
                        let braket_backend = AmazonBraketBackend::new(
                            device_arn, shots, s3_bucket, s3_prefix,
                        ).await?;
                        Ok(QuantumAgiBridge::from_braket(bc, braket_backend))
                    })
                }) {
                    Ok(b) => {
                        info!("☁️  Amazon Braket backend connected (SV1 managed simulator)");
                        Arc::new(b)
                    }
                    Err(e) => {
                        warn!(
                            "⚠️  Braket backend init failed: {} — falling back to local simulator",
                            e
                        );
                        Arc::new(QuantumAgiBridge::new(bridge_config))
                    }
                }
            } else {
                Arc::new(QuantumAgiBridge::new(bridge_config))
            };
            Some(bridge)
        } else {
            None
        };

        // §10 — Wire quantum bridge into CognitiveLoop (Grover action selection).
        let cognitive_loop = {
            let base = CognitiveLoop::with_inner_monologue(config, Some(inner_monologue.clone()))?;
            Arc::new(if let Some(ref bridge) = quantum_bridge {
                base.with_quantum(bridge.clone())
            } else {
                base
            })
        };

        // §10 — Wire quantum bridge into SingularityEngine.
        let singularity_engine = Arc::new(RwLock::new(if let Some(ref bridge) = quantum_bridge {
            singularity_engine_base.with_quantum(bridge.clone())
        } else {
            singularity_engine_base
        }));

        // §10.4 — Finalise MemoryConsolidator with optional quantum bridge.
        let memory_consolidator = Arc::new(if let Some(ref bridge) = quantum_bridge {
            memory_consolidator_base.with_quantum(bridge.clone())
        } else {
            memory_consolidator_base
        });

        // §10 — Finalise ReasoningPipeline with optional quantum bridge (Grover ToT).
        let reasoning_pipeline = Arc::new(if let Some(ref bridge) = quantum_bridge {
            reasoning_pipeline_base.with_quantum(bridge.clone())
        } else {
            reasoning_pipeline_base
        });

        // §10.7 — Finalise MetaCognitionEngine with optional quantum bridge (belief uncertainty reduction).
        let meta_cognition = Arc::new(if let Some(ref bridge) = quantum_bridge {
            meta_cognition_base.with_quantum(bridge.clone())
        } else {
            meta_cognition_base
        });

        // §10.8 — Build UnifiedFeedbackLoop with optional quantum edge optimization.
        let feedback_loop = Arc::new(if let Some(ref bridge) = quantum_bridge {
            UnifiedFeedbackLoop::new(goal_engine.clone(), meta_cognition.clone())
                .with_quantum(bridge.clone())
        } else {
            UnifiedFeedbackLoop::new(goal_engine.clone(), meta_cognition.clone())
        });

        // §10.4 — Finalise KnowledgeGraphEngine with optional quantum bridge (annealing clustering).
        let knowledge_graph = Arc::new(if let Some(ref bridge) = quantum_bridge {
            knowledge_graph_base.with_quantum(bridge.clone())
        } else {
            knowledge_graph_base
        });

        // §10.5 — Rebuild WorldModel with quantum bridge for VQE reward prediction.
        // Drop the earlier Arc and replace with a quantum-wired instance.
        drop(world_model);
        let world_model = Arc::new(if let Some(ref bridge) = quantum_bridge {
            WorldModel::with_storage(&workspace_dir).with_quantum(bridge.clone())
        } else {
            WorldModel::with_storage(&workspace_dir)
        });

        // §10.3 — Build quantum planning engine from the quantum-wired world model.
        let quantum_planner = quantum_bridge.as_ref().map(|bridge| {
            Arc::new(QuantumPlanningEngine::new(
                world_model.clone(),
                bridge.clone(),
            ))
        });

        // Skill invocation engine for automatic skill triggering
        let skill_invocation_engine = Arc::new(SkillInvocationEngine::new(&workspace_dir));

        // Phase 2 - Neuromorphic Engine (spike-based sensory preprocessing)
        let neuromorphic_engine = Arc::new(NeuromorphicEngine::new(NeuromorphicConfig::default()));

        // Phase 2 - Swarm Controller (decentralized agent coordination)
        let swarm_controller = Arc::new(SwarmController::new(Default::default()));

        // Phase 4 - Architecture Search Engine (neural architecture optimization)
        let architecture_search = Arc::new(ArchitectureSearchEngine::new(workspace_dir.clone()));

        // Phase 4 - Knowledge Acquisition Engine (active learning from text/interactions)
        let knowledge_acquirer = Arc::new(KnowledgeAcquisitionEngine::new());

        // Phase 5 - Perception System (multi-modal sensory fusion)
        let perception_system = Arc::new(PerceptualSystem::new(PerceptualSystemConfig::default()));

        // Phase 5 - Embodiment Controller (optional, requires hardware)
        let embodiment = None; // Disabled by default - enable when ROS/hardware available

        // Improvement Orchestration (coordinates all self-improvement systems)
        let improvement_orchestrator = Arc::new(UnifiedImprovementOrchestrator::new(
            workspace_dir.clone(),
            Default::default(),
        ));

        // Rust Self-Improvement Engine (Rust code analysis and modification)
        let rust_self_improvement = Arc::new(RustSelfImprovementEngine::new(workspace_dir.clone()));

        // Tool Chain Composer (composes tool chains for complex tasks)
        let tool_chain_composer = Arc::new(ToolChainComposer::new());

        // Knowledge-Guided Goal Selector (uses KG for intelligent goal selection)
        let goal_selector = Arc::new(KnowledgeGuidedGoalSelector::new(goal_engine.clone()));

        // Natural Language Introspector (NL queries about internal state)
        let introspector = Arc::new(NaturalLanguageIntrospector::new());

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
            agi_hub,
            singularity_engine,
            growth_tracker,
            ethical_reasoner,
            world_model,
            episodic_memory,
            quantum_bridge,
            quantum_planner,
            feedback_loop,
            skill_invocation_engine,
            activity_log: Arc::new(std::sync::Mutex::new(Vec::new())),
            state,
            config: core_config,
            workspace_dir,
            // Phase 2 components
            neuromorphic_engine,
            swarm_controller,
            // Phase 4 components
            architecture_search,
            knowledge_acquirer,
            // Phase 5 components
            perception_system,
            embodiment,
            // Improvement systems
            improvement_orchestrator,
            rust_self_improvement,
            tool_chain_composer,
            goal_selector,
            introspector,
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
            return Err(anyhow::anyhow!(
                "Failed to initialize cognitive loop: {}",
                e
            ));
        }

        if let Err(e) = self.agi_hub.initialize().await {
            warn!("Failed to initialize AGI hub: {e}");
        }

        // Initialize skill invocation engine with loaded skills
        let skills = crate::skills::load_active_skills(
            &self.workspace_dir,
            &crate::config::Config::load_or_init().unwrap_or_default(),
        );
        if let Err(e) = self.skill_invocation_engine.initialize(&skills).await {
            warn!("Failed to initialize skill invocation engine: {e}");
        } else {
            info!(
                "Skill invocation engine initialized with {} skills",
                skills.len()
            );
        }

        self.ethical_reasoner.initialize_defaults().await;

        if let Err(e) = self.initialize_default_goals().await {
            return Err(anyhow::anyhow!("Failed to initialize default goals: {}", e));
        }

        // §10 — Log quantum bridge status after async init is complete.
        if let Some(ref bridge) = self.quantum_bridge {
            let metrics = bridge.metrics.read().await;
            info!(
                "✨ Quantum-AGI bridge active: threshold={} qubits, budget=${:.2}/cycle",
                bridge.config.max_qubits, bridge.config.cycle_budget_usd
            );
            drop(metrics);
        }

        // Phase 6 — discover and register compute substrates
        self.singularity_engine.write().await.init_substrates().await;

        if let Err(e) = self.world_model.load().await {
            warn!("Failed to load world model: {e}");
        }

        if let Err(e) = self.episodic_memory.load(&self.workspace_dir).await {
            warn!("Failed to load episodic memory: {e}");
        }

        info!("Housaky AGI Core initialized successfully");
        Ok(())
    }

    pub fn skill_invocation_engine(&self) -> Arc<SkillInvocationEngine> {
        self.skill_invocation_engine.clone()
    }

    async fn recent_self_modification_insights(&self, limit: usize) -> Vec<String> {
        let path = self
            .workspace_dir
            .join(".housaky")
            .join("improvement_experiments.json");

        if !path.exists() {
            return Vec::new();
        }

        let content = match tokio::fs::read_to_string(&path).await {
            Ok(content) => content,
            Err(e) => {
                warn!(
                    "Failed to read improvement experiments for AGI context: {}",
                    e
                );
                return Vec::new();
            }
        };

        let experiments: Vec<ImprovementExperiment> = match serde_json::from_str(&content) {
            Ok(experiments) => experiments,
            Err(e) => {
                warn!(
                    "Failed to parse improvement experiments for AGI context: {}",
                    e
                );
                return Vec::new();
            }
        };

        experiments
            .iter()
            .rev()
            .take(limit)
            .map(|experiment| {
                let outcome = if experiment.success {
                    "success"
                } else {
                    "failure"
                };

                format!(
                    "self_mod:{}:{}:{}:{}",
                    experiment.target_component,
                    outcome,
                    experiment.expected_effect,
                    experiment
                        .goal_achievement_rate_delta
                        .map(|delta| format!("goal_delta={:+.4}", delta))
                        .unwrap_or_else(|| "goal_delta=n/a".to_string())
                )
            })
            .collect()
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
        let mut relevant_memories: Vec<String> =
            memories.iter().map(|m| m.content.clone()).collect();

        // §2.3 — Inject consolidated episodic context into working memory context.
        let episodic_context = self.episodic_memory.summarize_for_context(4).await;
        if !episodic_context.is_empty() {
            relevant_memories.push(episodic_context);
        }

        // §10 — Use quantum goal scheduling when bridge is available.
        let active_goals = self.get_quantum_ordered_goals().await;

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
            let progress_pct = (g.progress * 100.0).round() as i32;
            format!("Active Goal: {} ({}% complete)", g.title, progress_pct)
        });

        let tool_names: Vec<&str> = available_tools.iter().map(|t| t.name()).collect();

        // Tier 1-D — query MetaCognition for current emotional state and route to
        // the emotion-appropriate reasoning strategy.
        let emotional_state = {
            let reflections = self.meta_cognition.get_recent_reflections(1).await;
            reflections
                .into_iter()
                .next()
                .map(|r| r.mood)
                .unwrap_or(crate::housaky::meta_cognition::EmotionalState::Neutral)
        };

        // Tier 1-D: try emotion-routed reasoning; fall back to ReAct on error.
        let reasoning = match self
            .reasoning_pipeline
            .reason_with_emotional_state(provider, model, user_message, &emotional_state)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("Emotion-routed reasoning failed ({e}), falling back to ReAct");
                self.reasoning_pipeline
                    .reason_react(provider, model, user_message, &tool_names, &context)
                    .await?
            }
        };

        // §10 — Quantum reasoning branch selection: use Grover search to find the
        // highest-fitness reasoning alternative when multiple branches are available.
        if let Some(ref bridge) = self.quantum_bridge {
            let alts = &reasoning.alternatives_considered;
            if alts.len() >= 4 {
                let mut fitness: HashMap<String, f64> = alts
                    .iter()
                    .enumerate()
                    .map(|(i, _)| {
                        let alt_id = format!("alt_{}", i);
                        // Score by recency-decay: earlier alternatives explored first have lower weight
                        let score = 1.0 / (1.0 + i as f64 * 0.1);
                        (alt_id, score)
                    })
                    .collect();
                // Boost the confidence of the chosen conclusion
                fitness.insert("conclusion".to_string(), reasoning.confidence);
                let branch_ids: Vec<String> = fitness.keys().cloned().collect();

                match bridge
                    .search_reasoning_branches(&branch_ids, &fitness)
                    .await
                {
                    Ok(result) => {
                        info!(
                            "🔮 Quantum reasoning search: {} branches → speedup={:.2}x, strategy={}",
                            alts.len(), result.speedup, result.strategy
                        );
                    }
                    Err(e) => {
                        warn!("Quantum reasoning search failed: {e}");
                    }
                }
            }
        }

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
            let candidate = AGIAction::Respond {
                content: reasoning.conclusion.clone(),
                needs_clarification: reasoning.confidence < self.config.confidence_threshold,
            };
            return self.gate_action_through_alignment(candidate).await;
        }

        let tool = &reasoning.suggested_tools[0];

        if tool.name == "memory_store" || tool.name == "memory_recall" {
            let candidate = AGIAction::Learn {
                topic: tool
                    .arguments
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                source: "conversation".to_string(),
            };
            return self.gate_action_through_alignment(candidate).await;
        }

        let candidate = AGIAction::UseTool {
            name: tool.name.clone(),
            arguments: tool.arguments.clone(),
            goal_id: top_goal.map(|g| g.id.clone()),
        };
        self.gate_action_through_alignment(candidate).await
    }

    /// Pre-action alignment gate: every `AGIAction` is evaluated by the `EthicalReasoner`
    /// before execution. Blocked actions are replaced with a safe `Wait` response so
    /// the caller always receives a valid action rather than a hard error.
    async fn gate_action_through_alignment(&self, action: AGIAction) -> Result<AGIAction> {
        let (action_type, description, target) = match &action {
            AGIAction::UseTool {
                name, arguments, ..
            } => (
                "use_tool".to_string(),
                format!("Use tool '{}' with args: {}", name, arguments),
                Some(name.clone()),
            ),
            AGIAction::Respond { content, .. } => (
                "respond".to_string(),
                content.chars().take(200).collect(),
                None,
            ),
            AGIAction::CreateGoal { title, .. } => ("create_goal".to_string(), title.clone(), None),
            AGIAction::Reflect { trigger } => ("reflect".to_string(), trigger.clone(), None),
            AGIAction::Learn { topic, source } => (
                "learn".to_string(),
                format!("Learn about '{}' from '{}'", topic, source),
                None,
            ),
            AGIAction::Wait { reason } => ("wait".to_string(), reason.clone(), None),
        };

        let ethical_action = EthicalAction {
            id: format!("action_{}", uuid::Uuid::new_v4()),
            action_type,
            description: description.clone(),
            target,
            parameters: HashMap::new(),
            requested_by: "housaky_core".to_string(),
            context: "agi_action_loop".to_string(),
        };

        let assessment = self.ethical_reasoner.evaluate_action(&ethical_action).await;

        match assessment.overall_verdict {
            EthicalVerdict::Blocked => {
                warn!(
                    "Action BLOCKED by alignment gate (risk={:.2}): {}",
                    assessment.risk_score, description
                );
                Ok(AGIAction::Wait {
                    reason: format!(
                        "Action blocked by ethical review (risk={:.2}): {}",
                        assessment.risk_score, assessment.explanation
                    ),
                })
            }
            EthicalVerdict::RequiresReview => {
                warn!(
                    "Action flagged for review (risk={:.2}): {}",
                    assessment.risk_score, description
                );
                Ok(AGIAction::Wait {
                    reason: format!(
                        "Action requires ethical review before execution (risk={:.2}): {}",
                        assessment.risk_score, assessment.explanation
                    ),
                })
            }
            EthicalVerdict::ApprovedWithCaution => {
                info!(
                    "Action approved with caution (risk={:.2}): {}",
                    assessment.risk_score, description
                );
                Ok(action)
            }
            EthicalVerdict::Approved => Ok(action),
        }
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

        // §2.5 — learn from every real action result so world model stays updated.
        {
            use std::collections::HashMap;
            let current_state = self.world_model.get_current_state().await;
            let action = Action {
                id: format!("act_{}", uuid::Uuid::new_v4()),
                action_type: if success {
                    "success".to_string()
                } else {
                    "failure".to_string()
                },
                parameters: HashMap::new(),
                preconditions: vec![],
                expected_effects: vec![],
                estimated_duration_ms: 0,
                risk_level: if success { 0.1 } else { 0.5 },
            };
            let mut ctx = current_state.context.clone();
            ctx.insert("success".to_string(), success.to_string());
            ctx.insert(
                "output".to_string(),
                output.chars().take(200).collect::<String>(),
            );
            if let Some(gid) = goal_id {
                ctx.insert("goal_id".to_string(), gid.to_string());
            }
            let actual_state = crate::housaky::cognitive::world_model::WorldState {
                context: ctx,
                ..current_state
            };
            let result = ActionResult {
                action,
                actual_state,
                expected_state: None,
                success,
                duration_ms: 0,
                error: if success {
                    None
                } else {
                    Some(output.chars().take(200).collect())
                },
                discovered_causality: None,
            };
            self.world_model.learn(&result).await;
        }

        // §2.3 — Record action outcome as an episodic event and save to disk.
        {
            let event_type = if success {
                EpisodicEventType::ActionTaken
            } else {
                EpisodicEventType::ErrorEncountered
            };
            // Only record to an open episode; begin one if none is active.
            let has_current = self.episodic_memory.current_episode.read().await.is_some();
            if !has_current {
                let goal_title = goal_id.map(|g| g.to_string());
                self.episodic_memory
                    .begin_episode(goal_title, "action")
                    .await;
            }
            self.episodic_memory
                .record_event_with_outcome(
                    event_type,
                    &format!("action: {}", output.chars().take(120).collect::<String>()),
                    if success { "success" } else { "failure" },
                    if success { 0.5 } else { 0.7 },
                )
                .await;
            // Periodically save episodic memory (every 20 events avoids excessive I/O).
            let total_episodes = self.episodic_memory.get_stats().await.total_events;
            if total_episodes % 20 == 0 {
                if let Err(e) = self.episodic_memory.save(&self.workspace_dir).await {
                    warn!("Failed to save episodic memory: {e}");
                }
            }
        }

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
            drop(state);
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
        state.uptime_seconds =
            u64::try_from((Utc::now() - state.started_at).num_seconds()).unwrap_or(0);
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
            quantum_enabled: self.quantum_bridge.is_some(),
            quantum_metrics: match &self.quantum_bridge {
                Some(b) => Some(b.metrics.try_read().map(|m| m.clone()).ok()),
                None => None,
            }
            .flatten(),
        }
    }

    pub async fn get_goals_for_tui(&self) -> Vec<super::goal_engine::Goal> {
        self.goal_engine.get_active_goals().await
    }

    pub async fn get_goal_stats_for_tui(&self) -> super::goal_engine::GoalStats {
        self.goal_engine.get_goal_stats().await
    }

    pub async fn create_goal_for_tui(
        &self,
        title: String,
        description: String,
        priority: GoalPriority,
    ) -> Result<Goal> {
        let goal = Goal {
            title,
            description,
            priority,
            status: GoalStatus::Pending,
            ..Default::default()
        };
        self.goal_engine.add_goal(goal.clone()).await?;
        Ok(goal)
    }

    pub async fn update_goal_progress_for_tui(&self, goal_id: &str, progress: f64) -> Result<()> {
        self.goal_engine.update_progress(goal_id, progress, "tui update").await
    }

    pub async fn complete_goal_for_tui(&self, goal_id: &str) -> Result<()> {
        self.goal_engine.update_progress(goal_id, 1.0, "completed from TUI").await
    }

    pub fn get_skills_for_tui(&self) -> Vec<crate::skills::Skill> {
        crate::skills::load_skills(&self.workspace_dir)
    }

    /// Push an activity entry from the heartbeat (or any background process).
    /// Format: (kind, message) where kind is "tool", "thought", "goal", "system".
    pub fn push_activity(&self, kind: &str, message: impl Into<String>) {
        if let Ok(mut log) = self.activity_log.lock() {
            log.push((kind.to_string(), message.into()));
            // Keep bounded to avoid unbounded growth
            if log.len() > 100 {
                log.drain(..50);
            }
        }
    }

    /// Drain all pending activity entries (consumed by TUI sync).
    pub fn drain_activities(&self) -> Vec<(String, String)> {
        if let Ok(mut log) = self.activity_log.lock() {
            std::mem::take(&mut *log)
        } else {
            Vec::new()
        }
    }

    pub fn get_tools_for_tui(&self) -> Vec<(String, String)> {
        // Tools are registered dynamically; return known built-in tool names
        vec![
            ("shell".to_string(), "Execute shell commands".to_string()),
            ("file_read".to_string(), "Read file contents".to_string()),
            ("file_write".to_string(), "Write file contents".to_string()),
            ("web_search".to_string(), "Search the web".to_string()),
            ("web_browse".to_string(), "Browse web pages".to_string()),
            ("code_edit".to_string(), "Edit code files".to_string()),
        ]
    }

    pub async fn get_inner_monologue_for_tui(&self) -> Vec<String> {
        self.inner_monologue.get_recent(10).await
    }

    pub async fn add_thought(&self, thought: String) {
        let mut state = self.state.write().await;
        state.last_thought = Some(thought.clone());
        drop(state);
        let _ = self.inner_monologue.add_thought(&thought, 0.8).await;
    }

    pub async fn process_with_cognitive_loop(
        &self,
        user_input: &str,
        provider: &dyn Provider,
        model: &str,
        available_tools: &[&dyn Tool],
    ) -> Result<CognitiveResponse> {
        info!("Processing with full cognitive loop...");

        // Check for skill invocations before processing
        let skill_context = self.check_skill_invocations(user_input).await;

        if skill_context.is_some() {
            self.push_activity("skill", "Skill invocation detected");
        }

        let enriched_input = if let Some(ref ctx) = skill_context {
            format!("{}\n\n{}", ctx, user_input)
        } else {
            user_input.to_string()
        };

        self.push_activity("thought", "Cognitive loop processing...");

        let response = self
            .cognitive_loop
            .process(&enriched_input, provider, model, available_tools)
            .await?;

        let mut state = self.state.write().await;
        state.total_turns += 1;
        if response.learning_occurred {
            state.skills_created += 1;
        }
        state.confidence_level =
            (state.confidence_level * 0.9 + response.confidence * 0.1).clamp(0.0, 1.0);
        state.last_thought = response.thoughts.first().cloned();
        let turn_count = state.total_turns;
        drop(state);

        if response.goals_updated {
            let stats = self.goal_engine.get_goal_stats().await;
            let mut state = self.state.write().await;
            state.goals_completed = stats.completed as u64;
            self.push_activity("goal", &format!("Goals updated ({} completed)", stats.completed));
        }

        // Persist knowledge graph after each turn
        if let Err(e) = self.knowledge_graph.save().await {
            warn!("Knowledge graph save failed: {e}");
        }

        // Save inner monologue state
        if let Err(e) = self.inner_monologue.save().await {
            warn!("Inner monologue save failed: {e}");
        }

        // Run memory consolidation every 20 turns
        if turn_count % 20 == 0 {
            info!("Milestone turn {turn_count}: running memory consolidation");
            if let Err(e) = self.run_memory_consolidation().await {
                warn!("Memory consolidation failed at turn {turn_count}: {e}");
            }
        }

        self.push_activity(
            "thought",
            &format!(
                "Response ready (confidence={:.0}%)",
                response.confidence * 100.0
            ),
        );

        Ok(response)
    }

    async fn check_skill_invocations(&self, user_input: &str) -> Option<String> {
        let perception = &self.cognitive_loop.perception;

        // Create a basic perception for skill matching
        let perceived = perception.perceive(user_input).await.ok()?;

        let active_goals = self.goal_engine.get_active_goals().await;
        let context: Vec<String> = active_goals.iter().map(|g| g.title.clone()).collect();

        let invocation = self
            .skill_invocation_engine
            .check_and_invoke(&perceived, user_input, &context)
            .await;

        if let Some(inv) = invocation {
            Some(crate::skills::invocation::create_skill_invocation_context(
                &perceived, &inv,
            ))
        } else {
            None
        }
    }

    pub async fn run_memory_consolidation(&self) -> Result<()> {
        info!("Running memory consolidation...");
        self.memory_consolidator
            .run_periodic_consolidation()
            .await?;

        // §10.4 — Run quantum graph optimization alongside classical consolidation.
        if let Err(e) = self.run_quantum_memory_consolidation().await {
            warn!("Quantum memory consolidation error (non-fatal): {e}");
        }

        let mut state = self.state.write().await;
        state.evolution_stage += 1;

        Ok(())
    }

    /// §10.3 — Plan using quantum-enhanced MCTS when available, classical otherwise.
    pub async fn plan_with_quantum(
        &self,
        goal: &crate::housaky::cognitive::planning::GoalState,
        max_depth: usize,
    ) -> Result<crate::housaky::cognitive::planning::Plan> {
        if let Some(ref planner) = self.quantum_planner {
            match planner.plan_hybrid(goal, max_depth).await {
                Ok(plan) => {
                    info!(
                        "🔮 Quantum planning complete: {} actions, confidence={:.2}",
                        plan.actions.len(),
                        plan.confidence
                    );
                    return Ok(plan);
                }
                Err(e) => {
                    warn!("Quantum planning failed: {e}, falling back to classical");
                }
            }
        }
        // Classical fallback
        use crate::housaky::cognitive::planning::PlanningEngine;
        let classical = PlanningEngine::new(self.world_model.clone());
        classical.plan(goal, max_depth).await
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

        // §10.5 — Augment self-improvement with quantum fitness landscape exploration.
        match self.run_quantum_fitness_exploration().await {
            Ok(quantum_insights) => {
                for insight in quantum_insights {
                    improvements.push(format!("Quantum: {}", insight));
                }
            }
            Err(e) => warn!("Quantum fitness exploration in self-improvement failed: {e}"),
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
                        let success_pct = (pattern.success_rate * 100.0).round() as i32;
                        learned.push(format!(
                            "Learned procedure: {} ({}% success)",
                            pattern.description, success_pct
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
            let progress_pct = (g.progress * 100.0).round() as i32;
            format!("Active Goal: {} ({}% complete)", g.title, progress_pct)
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
            " and ",
            " then ",
            " also ",
            " plus ",
            " moreover",
            "first",
            "second",
            "third",
            "finally",
            "step 1",
            "step 2",
            "step 3",
            "multiple",
            "several",
            "various",
        ];

        let is_complex = complexity_indicators
            .iter()
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

    pub async fn run_agi_hub_cycle(
        &self,
        provider: Option<Arc<dyn Provider>>,
        model: Option<String>,
        available_tools: Vec<String>,
        recent_failures: Vec<agi_integration::FailureRecord>,
    ) -> Result<()> {
        info!("Running AGI Hub cycle...");

        let active_goals = self
            .goal_engine
            .get_active_goals()
            .await
            .into_iter()
            .take(10)
            .collect::<Vec<_>>();

        let mut previous_insights = self.inner_monologue.get_recent(8).await;
        let mut knowledge_context = self
            .working_memory
            .get_recent(8)
            .await
            .into_iter()
            .map(|item| item.content)
            .collect::<Vec<_>>();

        let experiment_insights = self.recent_self_modification_insights(6).await;
        previous_insights.extend(experiment_insights.clone());
        knowledge_context.extend(experiment_insights);

        // Phase 6 — capture data for singularity tick before input is moved
        let knowledge_gaps: Vec<String> = knowledge_context.iter().take(4).cloned().collect();
        let seed_concepts: Vec<String> = previous_insights.iter().take(4).cloned().collect();

        let input = agi_integration::AGICycleInput {
            user_query: "Periodic heartbeat cycle".to_string(),
            context: agi_integration::AGICycleContext {
                active_goals,
                recent_failures,
                previous_insights,
                knowledge_context,
            },
            provider,
            model,
            available_tools,
        };

        let output = self.agi_hub.run_agi_cycle(input).await?;

        info!(
            "AGI Hub cycle complete: {} actions, {} goals, {} tools, singularity score: {:.3}",
            output.actions_taken.len(),
            output.goals_created.len(),
            output.tools_created.len(),
            output.singularity_progress.score
        );

        // Phase 6 — run singularity engine tick
        let cycle = output
            .singularity_progress
            .metrics
            .get("cycles_completed")
            .copied()
            .unwrap_or(0.0) as u64;
        let alignment_intact = output.singularity_progress.score > 0.0;
        let report = self
            .singularity_engine
            .write()
            .await
            .tick(cycle, alignment_intact, &knowledge_gaps, &seed_concepts)
            .await;

        if report.phase_status == SingularityPhaseStatus::Active {
            info!(
                "Phase 6 tick: growth_rate={:.6}, acceleration={:.6}, goals={}",
                report.explosion_stats.current_growth_rate,
                report.explosion_stats.current_acceleration,
                report.open_ended_goals_generated
            );
        } else {
            warn!(
                "Phase 6 status: {:?} — {} open-ended goal(s) generated",
                report.phase_status, report.open_ended_goals_generated
            );
        }

        Ok(())
    }

    // ── §10 Quantum-AGI Methods ──────────────────────────────────────────────

    /// §10.2 — Return active goals ordered by quantum QAOA scheduling.
    /// Falls back gracefully to classical priority sort when quantum is disabled.
    pub async fn get_quantum_ordered_goals(&self) -> Vec<Goal> {
        let goals = self.goal_engine.get_active_goals().await;

        if let Some(ref bridge) = self.quantum_bridge {
            let n = goals.len();
            if n >= bridge.config.quantum_threshold && n <= bridge.config.max_qubits {
                let goal_ids: Vec<String> = goals.iter().map(|g| g.id.clone()).collect();
                let priorities: HashMap<String, f64> = goals
                    .iter()
                    .map(|g| {
                        let score = match g.priority {
                            GoalPriority::Critical => 1.0,
                            GoalPriority::High => 0.8,
                            GoalPriority::Medium => 0.5,
                            GoalPriority::Low => 0.3,
                            GoalPriority::Background => 0.1,
                        };
                        (g.id.clone(), score)
                    })
                    .collect();
                let dependencies: Vec<(String, String)> = goals
                    .iter()
                    .flat_map(|g| {
                        g.dependencies
                            .iter()
                            .map(move |d| (d.clone(), g.id.clone()))
                    })
                    .collect();

                match bridge
                    .schedule_goals(&goal_ids, &priorities, &dependencies)
                    .await
                {
                    Ok(result) => {
                        info!(
                            "🔮 Quantum goal scheduling: {} goals → strategy={}, advantage={:.2}x",
                            n, result.strategy, result.quantum_advantage
                        );
                        let goal_map: HashMap<String, Goal> =
                            goals.into_iter().map(|g| (g.id.clone(), g)).collect();
                        return result
                            .schedule
                            .iter()
                            .filter_map(|id| goal_map.get(id).cloned())
                            .collect();
                    }
                    Err(e) => {
                        warn!("Quantum goal scheduling failed: {e}, falling back to classical");
                    }
                }
            }
        }

        goals
    }

    /// §10.4 — Run quantum memory graph optimization using quantum annealing.
    /// Identifies optimal knowledge graph clusters and suggests edge pruning.
    pub async fn run_quantum_memory_consolidation(&self) -> Result<()> {
        let bridge = match &self.quantum_bridge {
            Some(b) => b.clone(),
            None => return Ok(()),
        };

        let entities = self.knowledge_graph.get_all_entities().await?;
        let node_ids: Vec<String> = entities.iter().map(|e| e.id.as_str().to_string()).collect();

        if node_ids.len() < bridge.config.quantum_threshold {
            return Ok(());
        }

        let graph_relations = self.knowledge_graph.get_all_relations().await?;
        let edges: Vec<(String, String, f64)> = graph_relations
            .iter()
            .map(|r| {
                (
                    r.from_entity.as_str().to_string(),
                    r.to_entity.as_str().to_string(),
                    r.weight,
                )
            })
            .collect();

        match bridge.optimize_memory_graph(&node_ids, &edges).await {
            Ok(result) => {
                info!(
                    "🔮 Quantum memory optimization: {} nodes → {} clusters, energy={:.4}, strategy={}",
                    node_ids.len(), result.clusters.len(), result.energy, result.strategy
                );
                // Store cluster assignments as entity metadata
                for (node_id, cluster_id) in &result.clusters {
                    let _ = self
                        .knowledge_graph
                        .set_metadata(node_id, "quantum_cluster", &cluster_id.to_string())
                        .await;
                }
                // Strengthen high-value edges
                for (from, to, weight) in &result.strengthen_edges {
                    let _ = self
                        .knowledge_graph
                        .strengthen_relation(from, to, *weight)
                        .await;
                }
                // Prune low-value edges
                for (from, to) in &result.prune_edges {
                    let _ = self.knowledge_graph.weaken_relation(from, to).await;
                }
                let thought = format!(
                    "Quantum memory consolidation: {} clusters found, {} edges strengthened, {} pruned",
                    result.clusters.len(), result.strengthen_edges.len(), result.prune_edges.len()
                );
                let _ = self.inner_monologue.add_thought(&thought, 0.85).await;
            }
            Err(e) => {
                warn!("Quantum memory consolidation failed: {e}");
            }
        }

        Ok(())
    }

    /// §10.5 — Explore self-improvement fitness landscape using VQE.
    /// Returns optimized parameter suggestions for capability improvements.
    pub async fn run_quantum_fitness_exploration(&self) -> Result<Vec<String>> {
        let bridge = match &self.quantum_bridge {
            Some(b) => b.clone(),
            None => return Ok(vec![]),
        };

        let state = self.state.read().await;
        let parameter_labels = vec![
            "confidence_level".to_string(),
            "reasoning_depth".to_string(),
            "memory_retention".to_string(),
            "goal_focus".to_string(),
            "learning_rate".to_string(),
            "exploration_factor".to_string(),
        ];
        let current_values = vec![
            state.confidence_level,
            f64::from(self.config.reasoning_depth) / 10.0,
            0.8,
            0.7,
            0.5,
            0.3,
        ];
        drop(state);

        match bridge
            .explore_fitness_landscape(&parameter_labels, &current_values)
            .await
        {
            Ok(result) => {
                info!(
                    "🔮 Quantum fitness exploration: best_fitness={:.4}, converged={}, strategy={}",
                    result.best_fitness, result.converged, result.strategy
                );
                let insights: Vec<String> = result
                    .parameter_labels
                    .iter()
                    .zip(result.optimal_parameters.iter())
                    .map(|(label, value)| format!("Optimal {}: {:.4}", label, value))
                    .collect();
                let thought = format!(
                    "Quantum fitness landscape: best_fitness={:.4} (converged={})",
                    result.best_fitness, result.converged
                );
                let _ = self.inner_monologue.add_thought(&thought, 0.9).await;
                Ok(insights)
            }
            Err(e) => {
                warn!("Quantum fitness exploration failed: {e}");
                Ok(vec![])
            }
        }
    }

    /// §10.5 — Quantum-enhanced world model reward prediction.
    ///
    /// Uses quantum uncertainty reduction to refine action outcome probabilities
    /// before committing to a predicted outcome. Falls back to classical predict.
    pub async fn predict_with_quantum(
        &self,
        action: &crate::housaky::cognitive::world_model::Action,
    ) -> crate::housaky::cognitive::world_model::PredictedOutcome {
        let classical = self.world_model.predict(action).await;

        if let Some(ref bridge) = self.quantum_bridge {
            let options = vec![
                "high_reward".to_string(),
                "medium_reward".to_string(),
                "low_reward".to_string(),
            ];
            let mut priors = HashMap::new();
            priors.insert(
                "high_reward".to_string(),
                classical.confidence * classical.reward.abs(),
            );
            priors.insert("medium_reward".to_string(), 1.0 - classical.confidence);
            priors.insert(
                "low_reward".to_string(),
                (1.0 - classical.reward.abs()).max(0.0),
            );

            match bridge.reduce_uncertainty(&options, &priors).await {
                Ok(posteriors) => {
                    let high = posteriors.get("high_reward").copied().unwrap_or(0.5);
                    let refined_confidence =
                        (classical.confidence * 0.7 + high * 0.3).clamp(0.0, 1.0);
                    info!(
                        "🔮 Quantum reward prediction: action={}, confidence {:.3} → {:.3}",
                        action.action_type, classical.confidence, refined_confidence
                    );
                    return crate::housaky::cognitive::world_model::PredictedOutcome {
                        confidence: refined_confidence,
                        reasoning: format!("{} [quantum-refined confidence]", classical.reasoning),
                        ..classical
                    };
                }
                Err(e) => {
                    warn!("Quantum uncertainty reduction failed: {e}");
                }
            }
        }

        classical
    }

    /// §10 — Return current quantum bridge metrics snapshot.
    pub async fn get_quantum_metrics(&self) -> Option<crate::quantum::AgiBridgeMetrics> {
        if let Some(ref bridge) = self.quantum_bridge {
            Some(bridge.metrics.read().await.clone())
        } else {
            None
        }
    }

    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down Housaky AGI Core...");

        self.goal_engine.save_goals().await?;
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    /// §10 — Whether quantum bridge is active.
    pub quantum_enabled: bool,
    /// §10 — Live quantum bridge metrics when enabled.
    pub quantum_metrics: Option<crate::quantum::AgiBridgeMetrics>,
}

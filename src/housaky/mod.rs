//! Housaky AGI Integration for Housaky
//!
//! This module provides:
//! - Housaky as the default autonomous agent
//! - 2-minute heartbeat with self-improvement
//! - Kowalski multi-agent integration
//! - EC2/VPS awareness and infrastructure management
//! - Infinite self-improvement toward AGI singularity
//!
//! AGI Core Components:
//! - Goal Engine: Persistent goal management with decomposition
//! - Reasoning Engine: CoT, ReAct, Tree of Thoughts reasoning
//! - Knowledge Graph: Entity-relationship knowledge storage
//! - Tool Creator: Automatic tool generation and testing
//! - Working Memory: Token-budgeted context management
//! - Meta-Cognition: Self-reflection and introspection
//! - Web Browser: Safe web content fetching and search

pub mod agent;
pub mod agi_context;
pub mod agi_integration;
pub mod agi_loop;
pub mod alignment;
pub mod cognitive;
pub mod core;
pub mod decision_journal;
pub mod goal_engine;
pub mod heartbeat;
pub mod housaky_agent;
pub mod inner_monologue;
pub mod knowledge_graph;
pub mod kowalski_integration;
pub mod memory;
pub mod meta_cognition;
pub mod multi_agent;
pub mod reasoning_engine;
pub mod reasoning_pipeline;
pub mod self_improvement_mod;
pub mod session_manager;
pub mod skills;
pub mod streaming;
pub mod tool_creator;
pub mod web_browser;
pub mod working_memory;

// New self-improving infrastructure
pub mod self_improvement_loop;
pub mod recursive_self_modifier;
pub mod tool_chain_composer;
pub mod knowledge_guided_goal_selector;
pub mod unified_feedback_loop;
pub mod capability_growth_tracker;

// Self-improvement code modification
pub mod rust_code_modifier;
pub mod git_sandbox;
pub mod self_improve_interface;

// GSD-inspired orchestration system
pub mod gsd_orchestration;

// Introspection for NL queries
pub mod introspection;

// Phase 1 — AGI Singularity Foundation
pub mod self_replication;
pub mod self_modification;
pub mod learning;

// Phase 2 — Quantum-Hybrid & Distributed Cognition
pub mod swarm;
pub mod neuromorphic;
pub mod federation;
pub mod quantum;

// Phase 3 — Consciousness Substrate & Self-Awareness
pub mod consciousness;

// Phase 4 — Unbounded Self-Improvement
pub mod architecture_search;
pub mod verification;
pub mod knowledge_acquisition;

// Phase 5 — Physical Embodiment & World Interaction
pub mod embodiment;
pub mod perception;

// Phase 6 — Singularity Convergence (Cycles 10 000–∞)
pub mod singularity;

// Re-export runtime WASM plugin functionality
pub use crate::runtime::wasm::{WasmRuntime, WasmExecutionResult, WasmCapabilities};

pub use gsd_orchestration::{
    GSDOrchestrator, GSDExecutionEngine, ExecutionSummary, VerificationReport, TaskAwareness, TaskAwarenessReport,
    CapabilityProfile, TaskPerformance, SelfImprovementIntegration, CapabilityUpdate,
};

pub use agent::{AgentInput, AgentOutput, Session as AgentSession, UnifiedAgentLoop};
pub use agi_integration::{
    AGIAction, AGIActionType, AGICycleContext, AGICycleInput, AGICycleOutput, AGIHubState,
    AGIIntegrationHub, AGIPhase, FailureAnalysis, FailureRecord, HubMetrics, ReflectionSummary,
    SingularityProgress,
};
pub use decision_journal::{
    ChosenOption, ConsideredOption, DecisionBuilder, DecisionContext, DecisionEntry,
    DecisionJournal, DecisionJournalError, ExecutionRecord, FileDecisionJournal, JournalStats,
    OutcomeRecord,
};
pub use housaky_agent::{
    Agent, Capability, KowalskiIntegrationConfig, Task, TaskCategory, TaskPriority, TaskStatus,
};
pub use session_manager::{Session, SessionManager, SessionSummary};

use crate::commands::{GoalCommands, GSDCommands, HousakyCommands, SelfModCommands};
use crate::config::Config;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn handle_command(command: HousakyCommands, config: &Config) -> Result<()> {
    match command {
        HousakyCommands::Status => {
            println!("🤖 Housaky AGI v4.0 Status");
            println!();

            let core = core::HousakyCore::new(config)?;
            core.initialize().await?;
            let metrics = core.get_dashboard_metrics().await;

            println!("Activity:");
            println!("  Total Turns:        {}", metrics.total_turns);
            println!("  Successful Actions: {}", metrics.successful_actions);
            println!("  Success Rate:       {:.1}%", metrics.success_rate * 100.0);
            println!();

            println!("State:");
            println!(
                "  Confidence Level:   {:.1}%",
                metrics.confidence_level * 100.0
            );
            println!("  Evolution Stage:    {}", metrics.evolution_stage);
            println!("  Uptime:             {}s", metrics.uptime_seconds);
            println!();

            println!("Capabilities:");
            println!(
                "  Reasoning:      {:.0}%",
                metrics.capabilities.reasoning * 100.0
            );
            println!(
                "  Learning:       {:.0}%",
                metrics.capabilities.learning * 100.0
            );
            println!(
                "  Self-Awareness: {:.0}%",
                metrics.capabilities.self_awareness * 100.0
            );
            println!(
                "  Meta-Cognition: {:.0}%",
                metrics.capabilities.meta_cognition * 100.0
            );
            println!();

            println!("Memory:");
            println!("  Items:   {}", metrics.memory_items);
            println!("  Tokens:  {}", metrics.memory_tokens);
            println!();

            println!("Knowledge:");
            println!("  Entities:  {}", metrics.knowledge_entities);
            println!("  Relations: {}", metrics.knowledge_relations);
            println!();

            let hub_metrics = core.agi_hub.get_hub_metrics().await;
            println!("AGI Hub:");
            println!("  Phase:              {:?}", hub_metrics.current_phase);
            println!("  Singularity Score:  {:.3}", hub_metrics.singularity_score);
            println!("  Cycles Completed:   {}", hub_metrics.cycles_completed);
            println!("  Goals Created:      {}", hub_metrics.goals_created);
            println!("  Tools Generated:    {}", hub_metrics.tools_generated);
        }

        HousakyCommands::Init => {
            println!("🚀 Initializing Housaky AGI...");

            let core = core::HousakyCore::new(config)?;
            core.initialize().await?;

            println!("✓ Housaky AGI Core initialized successfully!");
            println!("  Workspace: {}", config.workspace_dir.display());
            println!();

            let agent = Arc::new(agent::Agent::new(config)?);
            let heartbeat = heartbeat::HousakyHeartbeat::new(agent.clone());

            tokio::spawn(async move {
                if let Err(e) = heartbeat.run().await {
                    tracing::error!("Housaky heartbeat error: {}", e);
                }
            });

            println!("✓ AGI background process started (heartbeat every 2 minutes)");
            println!("✓ Self-improvement engine active");
            println!();
            println!("Run 'housaky housaky status' to see current state.");
            println!("Run 'housaky housaky goals' to manage goals.");
            println!("Run 'housaky agent' for interactive AI chat.");
        }

        HousakyCommands::Heartbeat => {
            println!("💓 Triggering manual heartbeat...");

            let agent = Arc::new(agent::Agent::new(config)?);
            let heartbeat = heartbeat::HousakyHeartbeat::new(agent);

            println!("Heartbeat cycle initiated. Running single cycle...");

            match heartbeat.run_single_cycle().await {
                Ok(()) => println!("✓ Heartbeat cycle completed successfully"),
                Err(e) => println!("✗ Heartbeat cycle failed: {}", e),
            }
        }

        HousakyCommands::Tasks => {
            println!("📋 Housaky Tasks");
            println!();

            let tasks_path = config.workspace_dir.join(".housaky").join("TASKS.md");
            if tasks_path.exists() {
                let content = tokio::fs::read_to_string(&tasks_path).await?;
                println!("{}", content);
            } else {
                println!("No tasks file found. Run 'housaky housaky init' first.");
            }
        }

        HousakyCommands::Review => {
            println!("📝 Housaky State Review");
            println!();

            let review_path = config.workspace_dir.join(".housaky").join("REVIEW.md");
            if review_path.exists() {
                let content = tokio::fs::read_to_string(&review_path).await?;
                println!("{}", content);
            } else {
                println!("No review file found. Run 'housaky housaky init' first.");
            }
        }

        HousakyCommands::Improve => {
            println!("🔧 Forcing self-improvement cycle...");

            let meta = meta_cognition::MetaCognitionEngine::new();
            let reflection = meta.reflect("Manual improvement trigger").await?;

            println!("Reflection complete:");
            println!("  Observations: {}", reflection.observations.len());
            println!("  Insights:     {}", reflection.insights.len());
            println!("  Actions:      {}", reflection.actions.len());
            println!("  Mood:         {:?}", reflection.mood);
        }

        HousakyCommands::ConnectKowalski => {
            println!("🔗 Connecting to Kowalski agents...");

            let bridge =
                kowalski_integration::KowalskiBridge::new(&agent::KowalskiIntegrationConfig {
                    enabled: true,
                    kowalski_path: std::path::PathBuf::from("/home/ubuntu/kowalski"),
                    enable_federation: true,
                    enable_code_agent: true,
                    enable_web_agent: true,
                    enable_academic_agent: true,
                    enable_data_agent: true,
                });

            if bridge.check_kowalski().await? {
                println!("✓ Kowalski connection successful!");
                let health = bridge.get_health();
                println!("  Available agents: {}", health.available_agents.len());
                for agent in &health.available_agents {
                    println!("    - {}", agent);
                }
            } else {
                println!("⚠ Kowalski not available at configured path");
                println!("  Check that Kowalski is installed and built.");
            }
        }

        HousakyCommands::Agi {
            message,
            provider,
            model,
        } => {
            println!("🧠 Starting Housaky AGI Mode (autonomous, parallel)...");
            let cfg = config.clone();
            tokio::spawn(async move {
                if let Err(e) = agi_loop::run_agi_loop(cfg, message, provider, model, 0.7).await {
                    tracing::error!("AGI loop error: {}", e);
                }
            });
            println!("✓ AGI running in background with self-improvement");
        }

        HousakyCommands::Run {
            message,
            provider,
            model,
            verbose,
        } => {
            println!("🚀 Starting Full Housaky AGI System (daemon + channels + heartbeat)...");
            println!("   Verbose mode: {}", verbose);
            
            heartbeat::run_agi_background(config.clone(), message, provider, model).await?;
        }

        HousakyCommands::Dashboard { provider, model } => {
            println!("📊 Launching AGI Dashboard...");
            crate::tui::run_agi_tui(config.clone(), provider, model, None)?;
        }

        HousakyCommands::Thoughts { count } => {
            let monologue = inner_monologue::InnerMonologue::new(&config.workspace_dir);
            monologue.load().await?;
            let thoughts = monologue.get_recent_thoughts(count).await;

            println!("💭 Recent Thoughts ({} shown)", thoughts.len());
            println!();

            for (i, thought) in thoughts.iter().enumerate() {
                println!(
                    "{:3}. [{}] {}",
                    i + 1,
                    thought.thought_type,
                    thought.content.chars().take(100).collect::<String>()
                );
            }
        }

        HousakyCommands::Goals { goal_command } => {
            handle_goal_command(goal_command, config).await?;
        }

        HousakyCommands::SelfMod { self_mod_command } => {
            handle_self_mod_command(self_mod_command, config).await?;
        }

        HousakyCommands::GSD { gsd_command } => {
            handle_gsd_command(gsd_command, config).await?;
        }
    }

    Ok(())
}

async fn handle_goal_command(command: GoalCommands, config: &Config) -> Result<()> {
    let engine = goal_engine::GoalEngine::new(&config.workspace_dir);
    engine.load_goals().await?;

    match command {
        GoalCommands::List => {
            let stats = engine.get_goal_stats().await;
            let goals = engine.get_active_goals().await;

            println!("📋 Goals Overview");
            println!("  Total:      {}", stats.total);
            println!("  Pending:    {}", stats.pending);
            println!("  In Progress: {}", stats.in_progress);
            println!("  Completed:  {}", stats.completed);
            println!();

            if !goals.is_empty() {
                println!("Active Goals:");
                for goal in goals {
                    println!(
                        "  [{}] {} - {}% complete",
                        format!("{:?}", goal.priority),
                        goal.title,
                        (goal.progress * 100.0) as i32
                    );
                    if !goal.description.is_empty() {
                        println!(
                            "    {}",
                            goal.description.chars().take(60).collect::<String>()
                        );
                    }
                }
            }
        }

        GoalCommands::Add {
            title,
            description,
            priority,
        } => {
            let prio = match priority.to_lowercase().as_str() {
                "critical" => goal_engine::GoalPriority::Critical,
                "high" => goal_engine::GoalPriority::High,
                "low" => goal_engine::GoalPriority::Low,
                _ => goal_engine::GoalPriority::Medium,
            };

            let goal = goal_engine::Goal {
                title,
                description: description.unwrap_or_default(),
                priority: prio,
                ..Default::default()
            };

            let id = engine.add_goal(goal).await?;
            println!("✓ Created goal: {}", id);
        }

        GoalCommands::Complete { id } => {
            engine
                .update_progress(&id, 1.0, "Manually completed")
                .await?;
            println!("✓ Goal {} marked as complete", id);
        }
    }

    Ok(())
}

async fn handle_self_mod_command(command: SelfModCommands, config: &Config) -> Result<()> {
    let housaky_dir = config.workspace_dir.join(".housaky");
    let params_path = housaky_dir.join("self_mod_parameters.json");
    let experiments_path = housaky_dir.join("improvement_experiments.json");

    match command {
        SelfModCommands::Run { provider, model } => {
            let core = core::HousakyCore::new(config)?;
            core.initialize().await?;

            let provider_name = provider
                .or_else(|| config.default_provider.clone())
                .unwrap_or_else(|| "openrouter".to_string());
            let model_name = model
                .or_else(|| config.default_model.clone())
                .unwrap_or_else(|| "arcee-ai/trinity-large-preview:free".to_string());

            let provider_instance = match crate::providers::create_provider(
                &provider_name,
                config.api_key.as_deref(),
            ) {
                Ok(provider_instance) => Some(provider_instance),
                Err(e) => {
                    println!(
                        "⚠ Could not initialize provider '{}': {}",
                        provider_name, e
                    );
                    println!("  Proceeding in offline mode for this cycle.");
                    None
                }
            };

            let recursive_loop = self_improvement_loop::SelfImprovementLoop::new(
                &config.workspace_dir,
                core.goal_engine.clone(),
                core.meta_cognition.clone(),
            );

            println!("🧠 Running one recursive self-improvement cycle...");
            let cycle = recursive_loop
                .run_full_cycle(
                    provider_instance.as_ref().map(|provider| provider.as_ref()),
                    &model_name,
                )
                .await?;

            println!("✓ Cycle completed");
            println!("  ID:               {}", cycle.id);
            println!("  Confidence:       {:.2}", cycle.confidence);
            println!("  New goals:        {}", cycle.outputs.new_goals.len());
            println!("  New tools:        {}", cycle.outputs.new_tools.len());
            println!("  New skills:       {}", cycle.outputs.new_skills.len());
            println!("  Modifications:    {}", cycle.self_modifications.len());
            println!(
                "  Successful mods:  {}",
                cycle.self_modifications.iter().filter(|m| m.success).count()
            );
        }

        SelfModCommands::Status => {
            let overrides: HashMap<String, HashMap<String, serde_json::Value>> =
                if params_path.exists() {
                    let content = tokio::fs::read_to_string(&params_path).await?;
                    serde_json::from_str(&content).unwrap_or_default()
                } else {
                    HashMap::new()
                };

            let experiments: Vec<self_improvement_loop::ImprovementExperiment> =
                if experiments_path.exists() {
                    let content = tokio::fs::read_to_string(&experiments_path).await?;
                    serde_json::from_str(&content).unwrap_or_default()
                } else {
                    Vec::new()
                };

            println!("🔧 Self-Modification Status");
            println!();

            println!("Overrides:");
            if overrides.is_empty() {
                println!("  (none)");
            } else {
                for (component, params) in &overrides {
                    println!("  {}", component);
                    for (key, value) in params {
                        println!("    - {} = {}", key, value);
                    }
                }
            }

            println!();
            let success_count = experiments.iter().filter(|exp| exp.success).count();
            let total_count = experiments.len();
            let success_rate = if total_count > 0 {
                (success_count as f64 / total_count as f64) * 100.0
            } else {
                0.0
            };

            println!("Experiments:");
            println!("  Total:        {}", total_count);
            println!("  Successful:   {}", success_count);
            println!("  Success Rate: {:.1}%", success_rate);

            if let Some(last) = experiments.last() {
                println!();
                println!("Latest:");
                println!("  ID:         {}", last.id);
                println!("  Target:     {}", last.target_component);
                println!("  Success:    {}", last.success);
                println!("  Timestamp:  {}", last.timestamp.to_rfc3339());
            }
        }

        SelfModCommands::Experiments { count } => {
            let experiments: Vec<self_improvement_loop::ImprovementExperiment> =
                if experiments_path.exists() {
                    let content = tokio::fs::read_to_string(&experiments_path).await?;
                    serde_json::from_str(&content).unwrap_or_default()
                } else {
                    Vec::new()
                };

            if experiments.is_empty() {
                println!("No self-modification experiments recorded yet.");
                return Ok(());
            }

            let shown = count.min(experiments.len());
            println!("🧪 Recent Self-Modification Experiments ({} shown)", shown);
            println!();

            for experiment in experiments.iter().rev().take(shown) {
                println!("- {}", experiment.id);
                println!("  Target:      {}", experiment.target_component);
                println!("  Type:        {:?}", experiment.modification_type);
                println!("  Success:     {}", experiment.success);
                println!("  Confidence:  {:.2}", experiment.confidence);
                println!("  Expected:    {}", experiment.expected_effect);
                println!("  Timestamp:   {}", experiment.timestamp.to_rfc3339());

                if let Some(delta) = experiment.singularity_score_delta {
                    println!("  ΔScore:      {:+.4}", delta);
                }

                if let Some(delta) = experiment.goal_achievement_rate_delta {
                    println!("  ΔGoal Rate:  {:+.4}", delta);
                }

                if let Some(reason) = &experiment.failure_reason {
                    println!("  Failure:     {}", reason);
                }

                println!();
            }
        }

        SelfModCommands::Set { target, key, value } => {
            let parsed_value = serde_json::from_str::<serde_json::Value>(&value)
                .unwrap_or(serde_json::Value::String(value.clone()));

            let mut params = HashMap::new();
            params.insert(key.clone(), parsed_value.clone());

            self_improvement_loop::SelfImprovementLoop::validate_parameter_change_request(
                &target,
                &params,
            )?;

            tokio::fs::create_dir_all(&housaky_dir).await?;

            let mut overrides: HashMap<String, HashMap<String, serde_json::Value>> =
                if params_path.exists() {
                    let content = tokio::fs::read_to_string(&params_path).await?;
                    serde_json::from_str(&content).unwrap_or_default()
                } else {
                    HashMap::new()
                };

            overrides
                .entry(target.clone())
                .or_insert_with(HashMap::new)
                .insert(key.clone(), parsed_value);

            let json = serde_json::to_string_pretty(&overrides)?;
            tokio::fs::write(&params_path, json).await?;

            println!("✓ Saved self-mod override: {}.{}", target, key);
            println!("  This will be applied on the next self-improvement cycle.");
        }

        SelfModCommands::Unset { target, key } => {
            if !params_path.exists() {
                println!("No self-mod override file found.");
                return Ok(());
            }

            let content = tokio::fs::read_to_string(&params_path).await?;
            let mut overrides: HashMap<String, HashMap<String, serde_json::Value>> =
                serde_json::from_str(&content).unwrap_or_default();

            let mut removed = false;

            if let Some(component_params) = overrides.get_mut(&target) {
                removed = component_params.remove(&key).is_some();
                if component_params.is_empty() {
                    overrides.remove(&target);
                }
            }

            if removed {
                let json = serde_json::to_string_pretty(&overrides)?;
                tokio::fs::write(&params_path, json).await?;
                println!("✓ Removed self-mod override: {}.{}", target, key);
            } else {
                println!("No override found for {}.{}", target, key);
            }
        }
    }

    Ok(())
}

async fn handle_gsd_command(command: GSDCommands, config: &Config) -> Result<()> {
        use crate::housaky::gsd_orchestration::GSDExecutionEngine;
    use crate::providers::create_provider;
    use crate::providers::Provider;

    println!("🚀 GSD Orchestration System");
    println!("============================\n");

    let provider = create_provider(
        config.default_provider.as_deref().unwrap_or("openrouter"),
        config.api_key.as_deref(),
    )?;

    let model = config.default_model.clone()
        .unwrap_or_else(|| "arcee-ai/trinity-large-preview:free".to_string());

    let boxed_provider: Box<dyn Provider> = provider;
    let engine = GSDExecutionEngine::new(
        config.workspace_dir.clone(),
        Some(boxed_provider),
        model,
    );

    engine.initialize().await?;

    match command {
        GSDCommands::NewProject { name, vision } => {
            println!("📁 Creating new GSD project: {}", name);
            let content = engine.create_project(name.clone(), vision).await?;
            println!("✓ Project '{}' created", name);
            println!("\nProject context:");
            println!("{}", content);
        }

        GSDCommands::Phase { name, description, goals } => {
            println!("📋 Creating phase: {}", name);
            let phase_id = engine.create_phase(name.clone(), description, goals.clone()).await?;
            println!("✓ Phase '{}' created (ID: {})", name, phase_id);
        }

        GSDCommands::Discuss { phase_id, answers } => {
            println!("💬 Discussing phase: {}", phase_id);
            let content = engine.discuss_phase(&phase_id, answers).await?;
            println!("✓ Phase context saved");
            println!("\nContext:\n{}", content);
        }

        GSDCommands::Execute { phase_id, task } => {
            println!("⚡ Executing phase: {}", phase_id);
            println!("Task: {}\n", task);
            
            let summary = engine.execute_with_llm(&phase_id, &task).await?;
            
            println!("\n📊 Execution Summary:");
            println!("  Total tasks: {}", summary.total_tasks);
            println!("  Successful:  {}", summary.successful_tasks);
            println!("  Failed:      {}", summary.failed_tasks);
            println!("  Duration:    {}ms", summary.total_duration_ms);
        }

        GSDCommands::Quick { task } => {
            println!("⚡ Quick execute: {}", task);
            println!();
            
            let summary = engine.quick_execute(&task).await?;
            
            println!("\n📊 Execution Summary:");
            println!("  Total tasks: {}", summary.total_tasks);
            println!("  Successful:  {}", summary.successful_tasks);
            println!("  Failed:      {}", summary.failed_tasks);
            println!("  Duration:    {}ms", summary.total_duration_ms);
        }

        GSDCommands::Verify { phase_id } => {
            println!("🔍 Verifying phase: {}", phase_id);
            let report = engine.verify_phase(&phase_id).await?;
            
            println!("\n📋 Verification Report:");
            println!("  Total items: {}", report.total_items);
            println!("  Verified:     {}", report.verified);
            println!("  Failed:       {}", report.failed);
            
            if !report.recommendations.is_empty() {
                println!("\n💡 Recommendations:");
                for rec in &report.recommendations {
                    println!("  - {}", rec);
                }
            }
        }

        GSDCommands::Status => {
            if let Some(phase) = engine.get_current_phase().await {
                println!("📍 Current Phase:");
                println!("  Name:        {}", phase.name);
                println!("  Description: {}", phase.description);
                println!("  Status:      {:?}", phase.status);
                println!("  Tasks:       {}", phase.tasks.len());
            } else {
                println!("No active phase");
            }
        }

        GSDCommands::Analyze { task } => {
            let decomposer = crate::housaky::gsd_orchestration::StepDecomposer::new();
            let analysis = decomposer.analyze_complexity(&task);
            
            println!("🔍 Task Complexity Analysis:");
            println!("  Score:     {:.2}", analysis.score);
            println!("  Category:  {:?}", analysis.category);
            println!("\n📌 Indicators:");
            for ind in &analysis.indicators {
                println!("  - {}", ind);
            }
        }

        GSDCommands::Awareness => {
            let report = engine.get_awareness_report().await;
            
            println!("🧠 Task Awareness Report:");
            println!("\n📊 Capability Profile:");
            println!("  Code Generation: {:.0}%", report.capability_profile.code_generation * 100.0);
            println!("  Testing:         {:.0}%", report.capability_profile.testing * 100.0);
            println!("  Debugging:       {:.0}%", report.capability_profile.debugging * 100.0);
            println!("  Refactoring:     {:.0}%", report.capability_profile.refactoring * 100.0);
            println!("  Architecture:    {:.0}%", report.capability_profile.architecture * 100.0);
            println!("  API Design:      {:.0}%", report.capability_profile.api_design * 100.0);
            println!("  Security:        {:.0}%", report.capability_profile.security * 100.0);
            println!("\n📈 Performance:");
            println!("  Tasks analyzed:     {}", report.total_tasks_analyzed);
            println!("  Avg success rate:  {:.1}%", report.avg_success_rate * 100.0);
            println!("  Complexity bias:   {:.2}", report.complexity_bias);
        }
    }

    Ok(())
}

/// Initialize Housaky integration
pub async fn initialize(config: &Config) -> Result<Arc<agent::Agent>> {
    let mut agent = agent::Agent::new(config)?;
    agent.initialize().await?;

    let agent = Arc::new(agent);

    // Start heartbeat in background
    let heartbeat = heartbeat::HousakyHeartbeat::new(agent.clone());
    tokio::spawn(async move {
        if let Err(e) = heartbeat.run().await {
            tracing::error!("Housaky heartbeat error: {}", e);
        }
    });

    Ok(agent)
}

/// Check if Housaky is enabled (always true in this integration)
pub fn is_enabled() -> bool {
    true
}

/// Get Housaky version
pub fn version() -> &'static str {
    "4.0.0-AGI"
}

/// Get Housaky description
pub fn description() -> &'static str {
    "Housaky AGI - Self-improving autonomous agent with infinite capability expansion"
}

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

pub use agent::{AgentInput, AgentOutput, Session as AgentSession, UnifiedAgentLoop};
pub use decision_journal::{
    ChosenOption, ConsideredOption, DecisionBuilder, DecisionContext, DecisionEntry,
    DecisionJournal, DecisionJournalError, ExecutionRecord, FileDecisionJournal, JournalStats,
    OutcomeRecord,
};
pub use housaky_agent::{
    Agent, Capability, KowalskiIntegrationConfig, Task, TaskCategory, TaskPriority, TaskStatus,
};
pub use session_manager::{Session, SessionManager, SessionSummary};

use crate::commands::{GoalCommands, HousakyCommands};
use crate::config::Config;
use anyhow::Result;
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
            crate::tui::run_agi_tui(config.clone(), provider, model)?;
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

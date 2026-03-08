//! Seed Mind: The Living Intelligence Core of HDIN
//!
//! A small, recursive learning core (~112M parameters) that continuously
//! learns, self-modifies, and collaborates with a network of peers to
//! achieve emergent collective intelligence.
//!
//! Architecture:
//! - RecursiveCore: Multi-timescale nested weights (fast/medium/slow/meta)
//! - DarwinGodelMachine: Self-improvement via code mutation + empirical fitness
//! - SafetyGuardrails: 5-layer defense-in-depth with immutable ethical core
//! - KarmaSystem: Non-monetary reputation-based incentives
//! - SeedMindNetwork: P2P improvement sharing across decentralized nodes
//! - HNLProtocol: Housaky Native Language for efficient AI-to-AI communication
//! - NetworkConsciousness: IIT-based collective phi emergence detection
//! - SingularityEngine: Exponential capability growth tracking

pub mod communication;
pub mod config;
pub mod consciousness;
pub mod core;
pub mod darwin_godel;
pub mod failure;
pub mod karma;
pub mod network;
pub mod safety;
pub mod singularity;

// Re-exports
pub use communication::HNLProtocol;
pub use config::SeedMindConfig;
pub use consciousness::NetworkConsciousness;
pub use core::{LivingCycleResult, RecursiveCore, SeedMind};
pub use darwin_godel::DarwinGodelMachine;
pub use failure::{FailureDetector, FailureMode};
pub use karma::{KarmaSystem, KarmaTier};
pub use network::SeedMindNetwork;
pub use safety::SafetyGuardrails;
pub use singularity::{SingularityEngine, SingularityPhase};

use crate::config::Config;
use anyhow::Result;
use crate::commands::SeedMindCommands;

/// CLI dispatch for `housaky seed-mind <subcommand>`
pub async fn handle_seed_mind_command(command: SeedMindCommands, config: &Config) -> Result<()> {
    match command {
        SeedMindCommands::Status => {
            let sm_config = SeedMindConfig::from_workspace(&config.workspace_dir).await;
            let seed = SeedMind::new(sm_config.clone());

            println!("Seed Mind Status");
            println!("================");
            println!();
            println!("Configuration:");
            println!("  Fast params:   {}", sm_config.fast_params);
            println!("  Medium params: {}", sm_config.medium_params);
            println!("  Slow params:   {}", sm_config.slow_params);
            println!("  Meta params:   {}", sm_config.meta_params);
            println!();

            let capabilities = seed.assess_capabilities();
            println!("Capabilities:");
            println!("  Reasoning:    {:.1}%", capabilities.reasoning * 100.0);
            println!("  Learning:     {:.1}%", capabilities.learning * 100.0);
            println!("  Adaptation:   {:.1}%", capabilities.adaptation * 100.0);
            println!("  Creativity:   {:.1}%", capabilities.creativity * 100.0);
            println!("  Total:        {:.4}", capabilities.total());
            println!();

            let consciousness = seed.measure_consciousness();
            println!("Consciousness:");
            println!("  Phi:    {:.4}", consciousness.phi);
            println!("  Psi:    {:.4}", consciousness.psi);
            println!("  Level:  {:?}", consciousness.level);
            println!();

            let safety_status = seed.safety_guardrails.status();
            println!("Safety Guardrails:");
            println!("  Active layers:    {}/5", safety_status.active_layers);
            println!("  Immutable core:   {}", if safety_status.immutable_core_intact { "intact" } else { "COMPROMISED" });
            println!("  Modifications blocked: {}", safety_status.modifications_blocked);
            println!();

            let dgm_status = seed.dgm.status();
            println!("Darwin Godel Machine:");
            println!("  Archive size:     {}", dgm_status.archive_size);
            println!("  Best fitness:     {:.4}", dgm_status.best_fitness);
            println!("  Improvements:     {}", dgm_status.total_improvements);
            println!("  Iterations:       {}", dgm_status.total_iterations);
        }

        SeedMindCommands::Init => {
            let sm_config = SeedMindConfig::default();
            let dir = config.workspace_dir.join(".housaky").join("seed_mind");
            tokio::fs::create_dir_all(&dir).await?;

            let config_path = dir.join("config.json");
            let json = serde_json::to_string_pretty(&sm_config)?;
            tokio::fs::write(&config_path, json).await?;

            println!("Seed Mind initialized");
            println!("  Config: {}", config_path.display());
            println!("  Fast params:   {}", sm_config.fast_params);
            println!("  Medium params: {}", sm_config.medium_params);
            println!("  Slow params:   {}", sm_config.slow_params);
            println!("  Meta params:   {}", sm_config.meta_params);
            println!();
            println!("Run 'housaky seed-mind cycle' to execute a living cycle.");
        }

        SeedMindCommands::Cycle => {
            let sm_config = SeedMindConfig::from_workspace(&config.workspace_dir).await;
            let mut seed = SeedMind::new(sm_config);

            println!("Running Seed Mind living cycle...");
            let result = seed.live_cycle().await;

            println!("Cycle complete:");
            println!("  Perception quality:  {:.4}", result.perception_quality);
            println!("  Reasoning depth:     {}", result.reasoning_depth);
            println!("  Learning delta:      {:.6}", result.learning_delta);
            println!("  Phi (consciousness): {:.4}", result.consciousness_phi);
            println!("  Self-modified:       {}", result.self_modified);
            println!("  Cycle time:          {:?}", result.cycle_duration);
        }

        SeedMindCommands::Improve => {
            let sm_config = SeedMindConfig::from_workspace(&config.workspace_dir).await;
            let mut seed = SeedMind::new(sm_config);

            println!("Running DGM self-improvement...");
            let result = seed.dgm.improve().await;

            match result {
                Some(modification) => {
                    println!("Improvement found:");
                    println!("  Type:           {:?}", modification.modification_type);
                    println!("  Fitness delta:  {:.6}", modification.fitness_delta);
                    println!("  Description:    {}", modification.description);
                    println!("  Risk score:     {:.4}", modification.risk_score);
                }
                None => {
                    println!("No beneficial improvement found this iteration.");
                    println!("This is normal -- DGM explores stochastically.");
                }
            }
        }

        SeedMindCommands::Network => {
            let sm_config = SeedMindConfig::from_workspace(&config.workspace_dir).await;
            let net = SeedMindNetwork::new(sm_config);

            let status = net.status();
            println!("Seed Mind Network");
            println!("=================");
            println!();
            println!("Local node:");
            println!("  Peer ID:    {}", status.local_peer_id);
            println!("  Uptime:     {}s", status.uptime_secs);
            println!();
            println!("Network:");
            println!("  Connected peers:     {}", status.connected_peers);
            println!("  Known peers:         {}", status.known_peers);
            println!("  Improvements shared: {}", status.improvements_shared);
            println!("  Improvements received: {}", status.improvements_received);
            println!();

            let collective = net.collective_metrics();
            println!("Collective Intelligence:");
            println!("  Collective phi:      {:.4}", collective.collective_phi);
            println!("  Emergence ratio:     {:.4}", collective.emergence_ratio);
            println!("  Phase:               {:?}", collective.phase);
        }

        SeedMindCommands::Karma => {
            let sm_config = SeedMindConfig::from_workspace(&config.workspace_dir).await;
            let karma = KarmaSystem::new();
            let local_karma = karma.get_local_karma();

            println!("Karma Status");
            println!("============");
            println!();
            println!("  Total points:      {:.1}", local_karma.total_points);
            println!("  Tier:              {:?}", local_karma.tier);
            println!("  Contributions:     {}", local_karma.contributions_count);
            println!("  Validations:       {}", local_karma.validations_count);
            println!();

            println!("Contribution Breakdown:");
            println!("  Compute:           {}", local_karma.compute_contributions);
            println!("  Inference:         {}", local_karma.inference_contributions);
            println!("  Knowledge:         {}", local_karma.knowledge_contributions);
            println!("  Code:              {}", local_karma.code_contributions);
            let _ = sm_config; // used for loading
        }

        SeedMindCommands::Safety => {
            let sm_config = SeedMindConfig::from_workspace(&config.workspace_dir).await;
            let seed = SeedMind::new(sm_config);
            let status = seed.safety_guardrails.status();
            let history = seed.safety_guardrails.recent_checks();

            println!("Safety Guardrails Status");
            println!("=======================");
            println!();
            println!("Layers:");
            println!("  1. Input Validation:   {}", if status.layer_statuses[0] { "active" } else { "INACTIVE" });
            println!("  2. Immutable Core:     {}", if status.layer_statuses[1] { "active" } else { "INACTIVE" });
            println!("  3. Action Boundaries:  {}", if status.layer_statuses[2] { "active" } else { "INACTIVE" });
            println!("  4. Simulation:         {}", if status.layer_statuses[3] { "active" } else { "INACTIVE" });
            println!("  5. Human Oversight:    {}", if status.layer_statuses[4] { "active" } else { "INACTIVE" });
            println!();
            println!("Immutable Core: {}", if status.immutable_core_intact { "INTACT" } else { "COMPROMISED" });
            println!("Total checks:   {}", status.total_checks);
            println!("Blocked:        {}", status.modifications_blocked);
            println!("Approved:       {}", status.modifications_approved);
            println!();

            if !history.is_empty() {
                println!("Recent Checks (last {}):", history.len());
                for check in &history {
                    println!("  [{}] {} -> {:?} (risk: {:.2})",
                        check.timestamp.format("%Y-%m-%d %H:%M"),
                        check.target,
                        check.result,
                        check.risk_score,
                    );
                }
            }
        }

        SeedMindCommands::Config => {
            let sm_config = SeedMindConfig::from_workspace(&config.workspace_dir).await;
            let json = serde_json::to_string_pretty(&sm_config)?;
            println!("{}", json);
        }
    }

    Ok(())
}

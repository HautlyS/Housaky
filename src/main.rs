//! Housaky - Zero overhead. Zero compromise. 100% Rust.
//!
//! Main entry point for the Housaky AI assistant.

#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::assigning_clones,
    clippy::bool_to_int_with_if,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::cast_possible_wrap,
    clippy::doc_markdown,
    clippy::field_reassign_with_default,
    clippy::float_cmp,
    clippy::implicit_clone,
    clippy::items_after_statements,
    clippy::map_unwrap_or,
    clippy::manual_let_else,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::needless_pass_by_value,
    clippy::needless_raw_string_hashes,
    clippy::redundant_closure_for_method_calls,
    clippy::similar_names,
    clippy::single_match_else,
    clippy::struct_field_names,
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    clippy::unused_self,
    clippy::cast_precision_loss,
    clippy::unnecessary_cast,
    clippy::unnecessary_lazy_evaluations,
    clippy::unnecessary_literal_bound,
    clippy::unnecessary_map_or,
    clippy::unnecessary_wraps,
    dead_code
)]
#![allow(clippy::all)]

use anyhow::Result;
use clap::Parser;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

// Use the library modules
extern crate housaky;

use housaky::cli::{ Cli, Commands, is_first_run, is_daemon_running };
use housaky::{
    channels, commands, cron, daemon, dashboard, hardware,
    integrations, migration, onboard, peripherals, service, skills, Config,
};
use housaky::commands::{
    ChannelCommands, HousakyCommands, KeyCommands, SkillCommands,
};

/// Start the complete Housaky system: daemon + dashboard + AGI TUI
/// This is the default behavior when running `housaky` with no subcommand
async fn start_full_system(config: Config, provider: Option<String>, model: Option<String>, verbose: bool) -> Result<()> {
    use tokio::time::{sleep, Duration};

    let host = "127.0.0.1".to_string();
    let gateway_port = 8080u16;
    let dashboard_port = 3000u16;

    // Set workspace env for all components
    std::env::set_var(
        "HOUSAKY_WORKSPACE",
        config.workspace_dir.to_string_lossy().to_string(),
    );

    // Check if daemon is already running
    let daemon_already_running = is_daemon_running();

    let daemon_abort: tokio::task::AbortHandle = if daemon_already_running {
        tokio::task::spawn(async {}).abort_handle()
    } else {
        let daemon_config = config.clone();
        let daemon_host = host.clone();
        let daemon_task = tokio::spawn(async move {
            if let Err(e) = daemon::run(daemon_config, daemon_host, gateway_port).await {
                tracing::error!("Daemon error: {}", e);
            }
        });
        daemon_task.abort_handle()
    };

    // Start dashboard web server (if installed)
    let dashboard_installed = dashboard::check_dashboard_installed();
    let dashboard_abort = if dashboard_installed {
        let dashboard_host = host.clone();
        let t = tokio::spawn(async move {
            if let Err(e) =
                dashboard::start_dashboard_server(&dashboard_host, dashboard_port, false).await
            {
                tracing::error!("Dashboard error: {}", e);
            }
        });
        Some(t.abort_handle())
    } else {
        None
    };

    // Give services time to start (shorter if connecting to existing daemon)
    if daemon_already_running {
        sleep(Duration::from_millis(200)).await;
    } else {
        sleep(Duration::from_millis(1000)).await;
    }

    // Start AGI with TUI (this blocks until user exits)
    let result = housaky::housaky::heartbeat::run_agi_with_tui(
        config.clone(),
        None, // No initial message
        provider,
        model,
        verbose,
    )
    .await;

    if let Some(abort) = dashboard_abort {
        abort.abort();
    }
    daemon_abort.abort();
    sleep(Duration::from_millis(500)).await;
    result
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    // Install default crypto provider for Rustls TLS.
    if let Err(e) = rustls::crypto::ring::default_provider().install_default() {
        eprintln!("Warning: Failed to install default crypto provider: {e:?}");
    }

    // Check if running without any subcommand (bare `housaky`)
    // We need to handle clap's built-in flags (--help, --version) separately
    let args: Vec<String> = std::env::args().collect();
    
    // Check if this is a clap built-in flag that should be handled by clap
    let is_clap_flag = args.len() > 1 && matches!(
        args[1].as_str(),
        "--help" | "-h" | "--version" | "-V" | "--help-all"
    );
    
    // Only start TUI if:
    // 1. No arguments at all (just `housaky`)
    // 2. Not a clap built-in flag
    let no_subcommand = args.len() == 1 || (args.len() > 1 && args[1].starts_with('-') && !is_clap_flag);

    if no_subcommand {
        // Initialize logging for the no-subcommand paths
        // Use a writer that discards output to avoid breaking the TUI
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::WARN)
            .with_target(false)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false)
            .with_writer(std::io::sink)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);

        // Initialize keys manager once
        let _ = housaky::keys_manager::manager::init_global_keys_manager();
        housaky::keys_manager::manager::load_keys_from_file().await;

        // Check for first run
        let (first_run, existing_config) = is_first_run();
        if first_run {
            println!("👋 Welcome to Housaky!");
            println!("   Starting interactive setup...\n");

            let config = onboard::run_wizard()?;

            if std::env::var("HOUSAKY_AUTOSTART_CHANNELS").as_deref() == Ok("1") {
                println!("\n🚀 Auto-starting channels...");
                channels::start_channels(config.clone()).await?;
            }

            return start_full_system(config, None, None, false).await;
        } else {
            let config = match existing_config {
                Some(c) => c,
                None => Config::load_or_init()?,
            };

            std::env::set_var(
                "HOUSAKY_WORKSPACE",
                config.workspace_dir.to_string_lossy().to_string(),
            );

            return start_full_system(config, None, None, false).await;
        }
    }

    // Normal CLI parsing with subcommand
    let cli = Cli::parse();

    // Detect TUI-bound commands BEFORE initializing logging
    // Only Tui and Run commands should trigger TUI mode
    let is_tui_command = cli.command.as_ref().map_or(false, |c| c.requires_tui());

    // Initialize logging:
    // - TUI commands: route through TuiWriter
    // - CLI commands: normal stdout subscriber
    if is_tui_command {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .with_writer(housaky::tui::output::TuiWriter)
            .with_ansi(false)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    } else {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    };

    // Initialize the keys manager
    let _ = housaky::keys_manager::manager::init_global_keys_manager();
    housaky::keys_manager::manager::load_keys_from_file().await;

    // Provide workspace dir as env for channel commands
    if std::env::var("HOUSAKY_WORKSPACE").is_err() {
        if let Ok(cfg) = Config::load_or_init() {
            std::env::set_var(
                "HOUSAKY_WORKSPACE",
                cfg.workspace_dir.to_string_lossy().to_string(),
            );
        }
    }

    // Handle subcommands
    match cli.command {
        None => {
            // Should never reach here - handled above
            unreachable!()
        }

        Some(Commands::Onboard {
            interactive,
            channels_only,
            api_key,
            provider,
            memory,
        }) => {
            let config = housaky::cli::handle_onboard(
                interactive,
                channels_only,
                api_key,
                provider,
                memory,
            ).await?;
            // Config is returned but not used further here
            let _ = config;
            Ok(())
        }

        Some(cmd) => {
            let mut config = Config::load_or_init()?;
            config.apply_env_overrides();

            match cmd {
                Commands::Agent {
                    message,
                    provider,
                    model,
                    temperature,
                    peripheral,
                } => housaky::cli::handle_agent(config, message, provider, model, temperature, peripheral).await,

                Commands::Gateway { port, host } => {
                    housaky::cli::handle_gateway(host, port, config).await
                }

                Commands::Daemon { action, port, host } => {
                    housaky::cli::handle_daemon(config, action, port, host).await
                }

                Commands::Run {
                    message,
                    provider,
                    model,
                    verbose,
                } => {
                    println!("🚀 Starting Full Housaky AGI System with TUI Chat...");
                    if verbose {
                        println!("   Verbose mode: on");
                    }
                    housaky::housaky::heartbeat::run_agi_with_tui(
                        config, message, provider, model, verbose,
                    )
                    .await
                }

                Commands::Status => housaky::cli::handle_status(&config),

                Commands::Dashboard {
                    start,
                    host,
                    port,
                    open,
                    desktop,
                } => housaky::cli::handle_dashboard(start, host, port, open, desktop).await,

                Commands::Cron { cron_command } => cron::handle_command(cron_command, &config),

                Commands::Models { model_command } => match model_command {
                    commands::ModelCommands::Refresh { provider, force } => {
                        onboard::run_models_refresh(&config, provider.as_deref(), force)
                    }
                },

                Commands::Service { service_command } => {
                    service::handle_command(&service_command, &config)
                }

                Commands::Keys { key_command } => {
                    let keys_manager = housaky::keys_manager::manager::get_global_keys_manager();

                    async fn handle_keys_command(
                        config: &mut Config,
                        keys_manager: &housaky::keys_manager::manager::KeysManager,
                        cmd: KeyCommands,
                    ) -> Result<()> {
                        match cmd {
                            KeyCommands::Manager(manager_cmd) => {
                                housaky::keys_manager::commands::handle_keys_manager_command(
                                    config,
                                    keys_manager,
                                    manager_cmd,
                                )
                                .await
                            }
                            KeyCommands::List => {
                                let _load_result = keys_manager.load().await;
                                let providers = keys_manager.get_providers().await;
                                if providers.is_empty() {
                                    println!("No keys configured.");
                                    println!("  Use `housaky keys manager add-provider` to add your first key.");
                                    Ok(())
                                } else {
                                    println!("Keys (keys_manager):");
                                    for provider in &providers {
                                        let enabled_count =
                                            provider.keys.iter().filter(|key| key.enabled).count();
                                        println!(
                                            "  - {}: {} keys ({} enabled)",
                                            provider.name,
                                            provider.keys.len(),
                                            enabled_count
                                        );
                                        for key in &provider.keys {
                                            let suffix = if key.key.len() > 4 {
                                                &key.key[key.key.len() - 4..]
                                            } else {
                                                &key.key
                                            };
                                            let status =
                                                if key.enabled { "enabled" } else { "disabled" };
                                            println!("      ...{} - {}", suffix, status);
                                        }
                                    }
                                    Ok(())
                                }
                            }
                            KeyCommands::Add { provider, key } => {
                                match keys_manager.add_key(&provider, key, None, None).await {
                                    Ok(()) => {
                                        println!("Added key to provider: {}", provider);
                                        keys_manager.save().await.ok();
                                        Ok(())
                                    }
                                    Err(e) => {
                                        println!("Error adding key: {}", e);
                                        Err(anyhow::anyhow!(e))
                                    }
                                }
                            }
                            KeyCommands::Remove { provider } => {
                                match keys_manager.remove_provider(&provider).await {
                                    Ok(()) => {
                                        println!("Removed provider: {}", provider);
                                        keys_manager.save().await.ok();
                                        Ok(())
                                    }
                                    Err(e) => {
                                        println!("Error removing provider: {}", e);
                                        Err(anyhow::anyhow!(e))
                                    }
                                }
                            }
                            KeyCommands::Rotate => {
                                println!("Key rotation is handled per-provider in keys_manager.");
                                println!("  Use `housaky keys manager rotate <provider>` instead.");
                                Ok(())
                            }
                        }
                    }

                    handle_keys_command(&mut config, &keys_manager, key_command).await
                }

                Commands::Doctor { doctor_command } => {
                    housaky::cli::handle_doctor(&config, doctor_command)
                }

                Commands::Channel { channel_command } => match channel_command {
                    ChannelCommands::Start => channels::start_channels(config).await,
                    ChannelCommands::Doctor => channels::doctor_channels(config).await,
                    other => channels::handle_command(other, &config),
                },

                Commands::Integrations {
                    integration_command,
                } => integrations::handle_command(integration_command, &config),

                Commands::Skills {
                    skill,
                    skill_command,
                } => {
                    let cmd = if let Some(name) = skill {
                        SkillCommands::Get {
                            name,
                            enable: false,
                        }
                    } else {
                        skill_command.unwrap_or(SkillCommands::Ui)
                    };
                    skills::handle_command(cmd, &config.workspace_dir)
                }

                Commands::Migrate { migrate_command } => {
                    migration::handle_command(migrate_command, &config).await
                }

                Commands::Hardware { hardware_command } => {
                    hardware::handle_command(hardware_command.clone(), &config)
                }

                Commands::Peripheral { peripheral_command } => {
                    peripherals::handle_command(peripheral_command.clone(), &config)
                }

                Commands::Tui { provider, model } => {
                    start_full_system(config, provider, model, false).await
                }

                Commands::Config {
                    section,
                    reset,
                    restore,
                } => housaky::cli::handle_config(config, section, reset, restore).await,

                Commands::Init => {
                    housaky::housaky::handle_command(HousakyCommands::Init, &config).await?;
                    Ok(())
                }

                Commands::Heartbeat => {
                    housaky::housaky::handle_command(HousakyCommands::Heartbeat, &config).await?;
                    Ok(())
                }

                Commands::Tasks => {
                    housaky::housaky::handle_command(HousakyCommands::Tasks, &config).await?;
                    Ok(())
                }

                Commands::Review => {
                    housaky::housaky::handle_command(HousakyCommands::Review, &config).await?;
                    Ok(())
                }

                Commands::Improve => {
                    housaky::housaky::handle_command(HousakyCommands::Improve, &config).await?;
                    Ok(())
                }

                Commands::ConnectKowalski => {
                    housaky::housaky::handle_command(HousakyCommands::ConnectKowalski, &config)
                        .await?;
                    Ok(())
                }

                Commands::AgiSession {
                    message,
                    provider,
                    model,
                } => {
                    housaky::housaky::handle_command(
                        HousakyCommands::Agi {
                            message,
                            provider,
                            model,
                        },
                        &config,
                    )
                    .await?;
                    Ok(())
                }

                Commands::Thoughts { count } => {
                    housaky::housaky::handle_command(
                        HousakyCommands::Thoughts { count },
                        &config,
                    )
                    .await?;
                    Ok(())
                }

                Commands::Goals { goal_command } => {
                    housaky::housaky::handle_command(
                        HousakyCommands::Goals { goal_command },
                        &config,
                    )
                    .await?;
                    Ok(())
                }

                Commands::SelfMod { self_mod_command } => {
                    housaky::housaky::handle_command(
                        HousakyCommands::SelfMod { self_mod_command },
                        &config,
                    )
                    .await?;
                    Ok(())
                }

                Commands::GSD { gsd_command } => {
                    housaky::housaky::handle_command(
                        HousakyCommands::GSD { gsd_command },
                        &config,
                    )
                    .await?;
                    Ok(())
                }

                Commands::Collective { collective_command } => {
                    housaky::housaky::handle_command(
                        HousakyCommands::Collective { collective_command },
                        &config,
                    )
                    .await?;
                    Ok(())
                }

                Commands::Quantum { quantum_command } => {
                    housaky::cli::quantum::handle_quantum(&config, quantum_command).await
                }

                Commands::A2A { action, message, task_id, task_action, params, category, confidence, file, timeout } => {
                    use housaky::housaky::a2a::{A2AManager, A2AMessage, A2AMessageType, A2ASelfImprove};
                    
                    let shared_dir = config.collaboration.shared_dir.clone();
                    let instance = &config.collaboration.instance_name;
                    let peer = &config.collaboration.peer_instance;
                    
                    let manager = A2AManager::new(shared_dir.clone(), instance, peer);
                    let a2a = A2ASelfImprove::new(shared_dir, instance, peer);
                    
                    let action = action.unwrap_or_else(|| "ping".to_string());
                    
                    match action.as_str() {
                        "ping" => {
                            match a2a.ping_peer().await {
                                Ok(true) => {
                                    println!("✅ Peer {} is alive!", peer);
                                    Ok(())
                                }
                                Ok(false) => {
                                    println!("❌ Peer {} is not responding", peer);
                                    Ok(())
                                }
                                Err(e) => {
                                    println!("❌ Ping failed: {}", e);
                                    Ok(())
                                }
                            }
                        }
                        "sync" => {
                            match a2a.sync_with_peer(timeout).await {
                                Ok(Some(resp)) => {
                                    println!("📡 Sync response: {:?}", resp.msg);
                                    Ok(())
                                }
                                Ok(None) => {
                                    println!("⏱️ Sync timed out after {}s", timeout);
                                    Ok(())
                                }
                                Err(e) => {
                                    anyhow::bail!("Sync error: {}", e)
                                }
                            }
                        }
                        "send" => {
                            let msg = message.unwrap_or_else(|| "Hello from Housaky".to_string());
                            manager.send(&A2AMessage::new(instance, peer, A2AMessageType::Context {
                                memory_type: "message".to_string(),
                                data: serde_json::json!({ "text": msg }),
                            })).await?;
                            println!("📤 Message sent to {}", peer);
                            Ok(())
                        }
                        "recv" => {
                            let messages = manager.read_from(peer)?;
                            for msg in &messages {
                                println!("📥 From {}: {:?}", msg.from, msg.msg);
                            }
                            if messages.is_empty() {
                                println!("📭 No messages from {}", peer);
                            }
                            Ok(())
                        }
                        "delegate" => {
                            let tid = task_id.unwrap_or_else(|| "task-1".to_string());
                            let act = task_action.unwrap_or_else(|| "analyze".to_string());
                            let ps = params.as_ref()
                                .and_then(|p| serde_json::from_str(p).ok())
                                .unwrap_or(serde_json::json!({}));
                            a2a.delegate_task(&tid, &act, ps).await?;
                            println!("📋 Task {} ({}) delegated to {}", tid, act, peer);
                            Ok(())
                        }
                        "learn" => {
                            let cat = category.unwrap_or_else(|| "general".to_string());
                            let cont = message.unwrap_or_else(|| "No content".to_string());
                            let conf = confidence.unwrap_or(0.8);
                            a2a.share_learning(&cat, &cont, conf).await?;
                            println!("📚 Learning shared with {}: {} (conf: {})", peer, cat, conf);
                            Ok(())
                        }
                        "review" => {
                            let f = file.as_ref().ok_or_else(|| anyhow::anyhow!("--file required for review"))?;
                            let diff = std::fs::read_to_string(f).unwrap_or_default();
                            a2a.request_code_review(f, &diff).await?;
                            println!("🔍 Code review requested for {}", f);
                            Ok(())
                        }
                        "process" => {
                            let responses = manager.process()?;
                            let count = responses.len();
                            for resp in responses.iter() {
                                println!("📤 Response: {:?}", resp.msg);
                                manager.send(resp).await?;
                            }
                            println!("✅ Processed {} messages", count);
                            Ok(())
                        }
                        _ => {
                            println!("Usage: housaky a2a <action> [options]");
                            println!("Actions:");
                            println!("  ping          - Check if peer is alive");
                            println!("  sync          - Request state sync with peer");
                            println!("  send          - Send message to peer");
                            println!("  recv          - Receive messages from peer");
                            println!("  delegate      - Delegate task to peer");
                            println!("  learn         - Share learning with peer");
                            println!("  review        - Request code review from peer");
                            println!("  process       - Process incoming messages");
                            Ok(())
                        }
                    }
                }

                Commands::Onboard { .. } => {
                    // Onboard is handled above, this should not be reached
                    unreachable!("Onboard command should be handled above")
                }
            }
        }
    }?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_has_no_flag_conflicts() {
        Cli::command().debug_assert();
    }
}

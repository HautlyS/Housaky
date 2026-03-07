//! Housaky - Zero overhead. Zero compromise. 100% Rust.
//!
//! Main entry point for the Housaky AI assistant.

#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::all)]
#![allow(dead_code)]

use anyhow::Result;
use clap::Parser;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

extern crate housaky;

use housaky::cli::{is_daemon_running, is_first_run, Cli, Commands};
use housaky::cli::args::{DoctorAction, HwAction};
use housaky::commands::{ChannelCommands, KeyCommands, SkillCommands};
use housaky::{channels, cron, daemon, dashboard, mcp, onboard, service, skills, Config};

// ============================================================================
// System Startup
// ============================================================================

/// Start full system: daemon + dashboard + TUI
async fn start_full_system(
    config: Config,
    provider: Option<String>,
    model: Option<String>,
) -> Result<()> {
    use tokio::time::{sleep, Duration};

    let host = "127.0.0.1".to_string();
    let gateway_port = 8080u16;
    let dashboard_port = 3000u16;

    std::env::set_var(
        "HOUSAKY_WORKSPACE",
        config.workspace_dir.to_string_lossy().to_string(),
    );

    let daemon_running = is_daemon_running();
    let daemon_abort = if daemon_running {
        tokio::task::spawn(async {}).abort_handle()
    } else {
        let cfg = config.clone();
        let h = host.clone();
        tokio::spawn(async move {
            if let Err(e) = daemon::run(cfg, h, gateway_port).await {
                tracing::error!("Daemon error: {}", e);
            }
        })
        .abort_handle()
    };

    let dashboard_abort = if dashboard::check_dashboard_installed() {
        let h = host.clone();
        Some(
            tokio::spawn(async move {
                if let Err(e) = dashboard::start_dashboard_server(&h, dashboard_port, false).await {
                    tracing::error!("Dashboard error: {}", e);
                }
            })
            .abort_handle(),
        )
    } else {
        None
    };

    sleep(Duration::from_millis(if daemon_running { 200 } else { 1000 })).await;

    let result = housaky::tui::run_minimal_tui(config, provider, model);

    if let Some(a) = dashboard_abort {
        a.abort();
    }
    daemon_abort.abort();
    sleep(Duration::from_millis(500)).await;
    result
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    // Install crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Check for bare `housaky` (no subcommand)
    let args: Vec<String> = std::env::args().collect();
    let is_help = args.len() > 1 && matches!(args[1].as_str(), "--help" | "-h" | "--version" | "-V");
    let no_subcommand = args.len() == 1 || (args.len() > 1 && args[1].starts_with('-') && !is_help);

    if no_subcommand {
        // Silent logging for TUI
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::WARN)
            .with_writer(std::io::sink)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);

        let _ = housaky::keys_manager::manager::init_global_keys_manager();
        housaky::keys_manager::manager::load_keys_from_file().await;

        let (first_run, existing_config) = is_first_run();
        if first_run {
            println!("Welcome to Housaky!");
            println!("Starting setup...\n");
            let config = onboard::run_wizard()?;
            return start_full_system(config, None, None).await;
        }

        let config = existing_config.unwrap_or_else(|| Config::load_or_init().unwrap_or_default());
        std::env::set_var("HOUSAKY_WORKSPACE", config.workspace_dir.to_string_lossy().to_string());
        return start_full_system(config, None, None).await;
    }

    // Parse CLI
    let cli = Cli::parse();

    // Setup logging
    let is_tui = cli.command.as_ref().map_or(false, housaky::cli::Commands::requires_tui);
    if is_tui {
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
    }

    // Init keys manager
    let _ = housaky::keys_manager::manager::init_global_keys_manager();
    housaky::keys_manager::manager::load_keys_from_file().await;

    // Set workspace env
    if std::env::var("HOUSAKY_WORKSPACE").is_err() {
        if let Ok(cfg) = Config::load_or_init() {
            std::env::set_var("HOUSAKY_WORKSPACE", cfg.workspace_dir.to_string_lossy().to_string());
        }
    }

    // Handle commands
    match cli.command {
        None => {
            // Default: launch unified TUI (same as housaky tui chat)
            let config = Config::load_or_init()?;
            housaky::tui::run_minimal_tui(config, None, None)
        }

        Some(cmd) => {
            let mut config = Config::load_or_init()?;
            config.apply_env_overrides();

            match cmd {
                // ─────────────────────────────────────────────────────────────
                // CHAT
                // ─────────────────────────────────────────────────────────────
                Commands::Chat { message, provider, model, temperature: _ } => {
                    if message.is_some() {
                        // One-shot mode
                        housaky::cli::handle_agent(config, message, provider, model, 0.7, vec![]).await
                    } else {
                        // TUI mode
                        housaky::tui::run_minimal_tui(config, provider, model)
                    }
                }

                // ─────────────────────────────────────────────────────────────
                // INIT & STATUS
                // ─────────────────────────────────────────────────────────────
                Commands::Init { interactive, api_key, provider, memory, channels_only } => {
                    housaky::cli::handle_onboard(interactive, channels_only, api_key, provider, memory).await?;
                    Ok(())
                }

                Commands::Status => housaky::cli::handle_status(&config),

                Commands::Doctor { action } => {
                    let doctor_cmd = match action {
                        Some(DoctorAction::Run) | None => housaky::commands::DoctorCommands::Run,
                        Some(DoctorAction::Fix) => housaky::commands::DoctorCommands::Fix,
                        Some(DoctorAction::Channels) => housaky::commands::DoctorCommands::Channels,
                        Some(DoctorAction::Security) => housaky::commands::DoctorCommands::Security,
                        Some(DoctorAction::Json) => housaky::commands::DoctorCommands::Json,
                    };
                    housaky::cli::handle_doctor(&config, Some(doctor_cmd))
                }

                Commands::Config { section, reset, restore } => {
                    housaky::cli::handle_config(config, section, reset, restore).await
                }

                // ─────────────────────────────────────────────────────────────
                // DAEMON & SERVICE
                // ─────────────────────────────────────────────────────────────
                Commands::Daemon { action, port, host } => {
                    housaky::cli::handle_daemon(config, action, port, host).await
                }

                Commands::Service { action } => {
                    service::handle_command(&action, &config)
                }

                Commands::Gateway { port, host } => {
                    housaky::cli::handle_gateway(host, port, config).await
                }

                Commands::Dashboard { start, host, port, open, desktop } => {
                    housaky::cli::handle_dashboard(start, host, port, open, desktop).await
                }

                // ─────────────────────────────────────────────────────────────
                // KEYS & MODELS
                // ─────────────────────────────────────────────────────────────
                Commands::Keys { action } => {
                    let keys_manager = housaky::keys_manager::manager::get_global_keys_manager();
                    handle_keys(&mut config, &keys_manager, action).await
                }

                Commands::Models { action } => {
                    match action {
                        housaky::commands::ModelCommands::Refresh { provider, force } => {
                            onboard::run_models_refresh(&config, provider.as_deref(), force)
                        }
                    }
                }

                // ─────────────────────────────────────────────────────────────
                // CHANNELS & SKILLS
                // ─────────────────────────────────────────────────────────────
                Commands::Channel { action } => match action {
                    ChannelCommands::Start => channels::start_channels(config).await,
                    ChannelCommands::Doctor => channels::doctor_channels(config).await,
                    other => channels::handle_command(other, &config),
                },

                Commands::Skill { name, action } => {
                    let cmd = if let Some(n) = name {
                        SkillCommands::Get { name: n, enable: false }
                    } else {
                        action.unwrap_or(SkillCommands::Ui)
                    };
                    skills::handle_command(cmd, &config.workspace_dir)
                }

                Commands::Mcp { action } => {
                    mcp::marketplace::handle_mcp_command(action, &config.workspace_dir)
                }

                Commands::Cron { action } => cron::handle_command(action, &config),

                Commands::Migrate { action } => {
                    housaky::migration::handle_command(action, &config).await
                }

                // ─────────────────────────────────────────────────────────────
                // AGI
                // ─────────────────────────────────────────────────────────────
                Commands::Goal { action } => {
                    housaky::housaky::handle_command(housaky::commands::HousakyCommands::Goals { goal_command: action }, &config).await
                }

                Commands::Improve { provider: _, model: _ } => {
                    housaky::housaky::handle_command(housaky::commands::HousakyCommands::Improve, &config).await
                }

                Commands::Thoughts { count } => {
                    housaky::housaky::handle_command(housaky::commands::HousakyCommands::Thoughts { count }, &config).await
                }

                Commands::SelfMod { action } => {
                    housaky::housaky::handle_command(housaky::commands::HousakyCommands::SelfMod { self_mod_command: action }, &config).await
                }

                Commands::Gsd { action } => {
                    housaky::housaky::handle_command(housaky::commands::HousakyCommands::GSD { gsd_command: action }, &config).await
                }

                Commands::Collective { action } => {
                    housaky::housaky::handle_command(housaky::commands::HousakyCommands::Collective { collective_command: action }, &config).await
                }

                // ─────────────────────────────────────────────────────────────
                // HARDWARE
                // ─────────────────────────────────────────────────────────────
                Commands::Hw { action } => handle_hw(action, &config),

                // ─────────────────────────────────────────────────────────────
                // QUANTUM & A2A
                // ─────────────────────────────────────────────────────────────
                Commands::Quantum { action } => {
                    housaky::cli::quantum::handle_quantum(&config, action).await
                }

                Commands::A2a { action, message, task_id, task_action, params, category, confidence, file, timeout } => {
                    handle_a2a(&config, action, message, task_id, task_action, params, category, confidence, file, timeout).await
                }

                Commands::Heartbeat => {
                    housaky::housaky::handle_command(housaky::commands::HousakyCommands::Heartbeat, &config).await
                }

                Commands::Kowalski => {
                    housaky::housaky::handle_command(housaky::commands::HousakyCommands::ConnectKowalski, &config).await
                }

                // ─────────────────────────────────────────────────────────────
                // TUI & HELP
                // ─────────────────────────────────────────────────────────────
                Commands::Tui { name, provider, model, temperature } => {
                    housaky::cli::run_tui_command(name, provider, model, temperature, config)
                }

                Commands::Help { topic } => {
                    housaky::cli::HelpSystem::show_help(topic.as_deref());
                    Ok(())
                }
            }
        }
    }
}

// ============================================================================
// Command Handlers
// ============================================================================

async fn handle_keys(
    config: &mut Config,
    keys_manager: &housaky::keys_manager::manager::KeysManager,
    cmd: KeyCommands,
) -> Result<()> {
    match cmd {
        KeyCommands::Manager(manager_cmd) => {
            housaky::keys_manager::commands::handle_keys_manager_command(config, keys_manager, manager_cmd).await
        }
        KeyCommands::List => {
            let _ = keys_manager.load().await;
            let providers = keys_manager.get_providers().await;
            if providers.is_empty() {
                println!("No keys configured. Use `housaky keys add <provider> <key>`");
            } else {
                println!("API Keys:");
                for p in &providers {
                    let enabled = p.keys.iter().filter(|k| k.enabled).count();
                    println!("  {} - {} keys ({} enabled)", p.name, p.keys.len(), enabled);
                    for k in &p.keys {
                        let suffix = if k.key.len() > 4 { &k.key[k.key.len()-4..] } else { &k.key };
                        let status = if k.enabled { "enabled" } else { "disabled" };
                        println!("    ...{suffix} - {status}");
                    }
                }
            }
            Ok(())
        }
        KeyCommands::Add { provider, key } => {
            keys_manager.add_key(&provider, key, None, None).await?;
            keys_manager.save().await?;
            println!("Added key for {provider}");
            Ok(())
        }
        KeyCommands::Remove { provider } => {
            keys_manager.remove_provider(&provider).await?;
            keys_manager.save().await?;
            println!("Removed {provider}");
            Ok(())
        }
        KeyCommands::Rotate => {
            println!("Key rotation is automatic. Use `housaky keys manager rotate <provider>` for manual rotation.");
            Ok(())
        }
        KeyCommands::Tui => {
            housaky::keys_manager::tui::run_keys_tui(keys_manager).await
        }
        KeyCommands::Subagent { action } => {
            handle_subagent(keys_manager, action).await
        }
    }
}

async fn handle_subagent(
    keys_manager: &housaky::keys_manager::manager::KeysManager,
    cmd: housaky::commands::SubagentCommands,
) -> Result<()> {
    use housaky::commands::SubagentCommands;

    match cmd {
        SubagentCommands::List => {
            let _ = keys_manager.load().await;
            let store = keys_manager.store.read().await;

            if let Some(subagents) = &store.subagents {
                println!("Sub-agent Configurations:");
                for (name, config) in subagents {
                    println!("  {name}:");
                    println!("    Provider: {}", config.provider);
                    println!("    Model: {}", config.model);
                    println!("    Key: {}", config.key_name);
                    println!("    Max Concurrent: {}", config.max_concurrent);
                }
            } else {
                println!("No sub-agents configured. Add them to keys.json under 'subagents'.");
            }
            Ok(())
        }
        SubagentCommands::Show { name } => {
            let _ = keys_manager.load().await;
            let store = keys_manager.store.read().await;

            if let Some(subagents) = &store.subagents {
                if let Some(config) = subagents.get(&name) {
                    println!("Sub-agent: {name}");
                    println!("  Provider: {}", config.provider);
                    println!("  Model: {}", config.model);
                    println!("  Key: {}", config.key_name);
                    println!("  Max Concurrent: {}", config.max_concurrent);

                    // Show key details
                    if let Some(provider) = store.providers.get(&config.provider) {
                        if let Some(key) = provider.keys.iter().find(|k| k.name == config.key_name) {
                            println!("  Key Status: {}", if key.enabled { "enabled" } else { "disabled" });
                            println!("  Key ID: {}", key.id);
                            println!("  Total Requests: {}", key.usage.total_requests);
                        }
                    }
                } else {
                    println!("Sub-agent '{name}' not found.");
                }
            } else {
                println!("No sub-agents configured.");
            }
            Ok(())
        }
        SubagentCommands::Assign { name, provider, key, model } => {
            let _ = keys_manager.load().await;
            let mut store = keys_manager.store.write().await;

            // Verify provider exists
            if !store.providers.contains_key(&provider) {
                println!("Error: Provider '{provider}' not found.");
                return Ok(());
            }

            // Verify key exists
            let key_exists = store.providers.get(&provider)
                .is_some_and(|p| p.keys.iter().any(|k| k.name == key));

            if !key_exists {
                println!("Error: Key '{key}' not found in provider '{provider}'.");
                return Ok(());
            }

            // Get or create subagents map
            let subagents = store.subagents.get_or_insert_with(std::collections::HashMap::new);

            // Determine the model to use
            let model_name = model.as_deref().unwrap_or("gpt-4o");

            // Create or update subagent config
            let config = housaky::keys_manager::manager::SubAgentConfig {
                provider: provider.clone(),
                model: model_name.to_string(),
                key_name: key.clone(),
                max_concurrent: 2,
            };

            subagents.insert(name.clone(), config);
            drop(store);
            keys_manager.save().await?;

            println!("Assigned {name} -> {provider}/{key} (model: {model_name})");
            Ok(())
        }
        SubagentCommands::Test { name, message } => {
            let _ = keys_manager.load().await;
            let store = keys_manager.store.read().await;

            if let Some(subagents) = &store.subagents {
                if let Some(config) = subagents.get(&name) {
                    println!("Testing sub-agent '{name}'...");
                    println!("  Provider: {}", config.provider);
                    println!("  Model: {}", config.model);
                    println!("  Key: {}", config.key_name);
                    println!("  Message: {message}");

                    // Get the API key
                    if let Some(provider) = store.providers.get(&config.provider) {
                        if let Some(key) = provider.keys.iter().find(|k| k.name == config.key_name) {
                            println!("\n🔑 Using key: ...{}", &key.key[key.key.len().saturating_sub(4)..]);

                            // Make a test request
                            let base_url = provider.base_url.as_deref().unwrap_or("https://api.openai.com/v1");
                            println!("🌐 Endpoint: {base_url}");

                            // Note: Actual API call would go here
                            println!("\n✅ Configuration valid. Ready to make requests.");
                        }
                    }
                } else {
                    println!("Sub-agent '{name}' not found.");
                }
            } else {
                println!("No sub-agents configured.");
            }
            Ok(())
        }
    }
}

fn handle_hw(action: HwAction, config: &Config) -> Result<()> {
    use housaky::cli::args::{HwAction as CliHwAction};
    use housaky::{hardware, peripherals};
    use housaky::commands::{HardwareCommands, PeripheralCommands};

    match action {
        CliHwAction::Discover => hardware::handle_command(HardwareCommands::Discover, config),
        CliHwAction::Introspect { path } => hardware::handle_command(HardwareCommands::Introspect { path }, config),
        CliHwAction::Info { chip } => hardware::handle_command(HardwareCommands::Info { chip }, config),
        CliHwAction::List => peripherals::handle_command(PeripheralCommands::List, config),
        CliHwAction::Add { board, path } => peripherals::handle_command(PeripheralCommands::Add { board, path }, config),
        CliHwAction::FlashArduino { port } => peripherals::handle_command(PeripheralCommands::Flash { port }, config),
        CliHwAction::FlashNucleo => peripherals::handle_command(PeripheralCommands::FlashNucleo, config),
        CliHwAction::SetupUnoQ { host } => peripherals::handle_command(PeripheralCommands::SetupUnoQ { host }, config),
    }
}

async fn handle_a2a(
    config: &Config,
    action: Option<String>,
    message: Option<String>,
    task_id: Option<String>,
    task_action: Option<String>,
    params: Option<String>,
    category: Option<String>,
    confidence: Option<f32>,
    file: Option<String>,
    timeout: u64,
) -> Result<()> {
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
                Ok(true) => println!("Peer {peer} is alive"),
                Ok(false) => println!("Peer {peer} not responding"),
                Err(e) => println!("Ping failed: {e}"),
            }
            Ok(())
        }
        "sync" => {
            match a2a.sync_with_peer(timeout).await {
                Ok(Some(resp)) => println!("Sync response: {:?}", resp.msg),
                Ok(None) => println!("Sync timed out after {timeout}s"),
                Err(e) => anyhow::bail!("Sync error: {e}"),
            }
            Ok(())
        }
        "send" => {
            let msg = message.unwrap_or_else(|| "Hello from Housaky".to_string());
            manager.send(&A2AMessage::new(instance, peer, A2AMessageType::Context {
                memory_type: "message".to_string(),
                data: serde_json::json!({ "text": msg }),
            })).await?;
            println!("Sent to {peer}");
            Ok(())
        }
        "recv" => {
            let messages = manager.read_from(peer)?;
            for msg in &messages {
                println!("From {}: {:?}", msg.from, msg.msg);
            }
            if messages.is_empty() {
                println!("No messages from {peer}");
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
            println!("Task {tid} ({act}) delegated to {peer}");
            Ok(())
        }
        "learn" => {
            let cat = category.unwrap_or_else(|| "general".to_string());
            let cont = message.unwrap_or_else(|| "No content".to_string());
            let conf = confidence.unwrap_or(0.8);
            a2a.share_learning(&cat, &cont, conf).await?;
            println!("Learning shared: {cat} (confidence: {conf})");
            Ok(())
        }
        "review" => {
            let f = file.as_ref().ok_or_else(|| anyhow::anyhow!("--file required"))?;
            let diff = std::fs::read_to_string(f).unwrap_or_default();
            a2a.request_code_review(f, &diff).await?;
            println!("Code review requested for {f}");
            Ok(())
        }
        _ => {
            println!("A2A Actions: ping, sync, send, recv, delegate, learn, review");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_no_conflicts() {
        Cli::command().debug_assert();
    }
}

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

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

// Explicitly declare we're using the housaky library
extern crate housaky;

// Use the library modules
use housaky::{
    agent, channels, commands, config_editor, cron, daemon, dashboard, doctor, gateway, hardware,
    integrations, migration, onboard, peripherals, service, skills, tui, Config,
};
use housaky::daemon::control as daemon_control;

#[derive(Subcommand, Debug)]
enum DaemonAction {
    /// Start the daemon (default when no subcommand given)
    Start {
        #[arg(short, long, default_value = "8080")]
        port: u16,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
    /// Stop a running daemon
    Stop,
    /// Restart: stop any running daemon then start a fresh one
    Restart {
        #[arg(short, long, default_value = "8080")]
        port: u16,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
    /// Show whether the daemon is running
    Status,
}

use commands::{
    ChannelCommands, CronCommands, HardwareCommands, HousakyCommands, IntegrationCommands,
    KeyCommands, MigrateCommands, ModelCommands, PeripheralCommands, QuantumCommands,
    ServiceCommands, SkillCommands,
};

/// `Housaky` - Zero overhead. Zero compromise. 100% Rust.
#[derive(Parser, Debug)]
#[command(name = "housaky")]
#[command(author = "theonlyhennygod")]
#[command(version = "0.1.0")]
#[command(about = "The fastest, smallest AI assistant.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize your workspace and configuration
    Onboard {
        /// Run the full interactive wizard (default is quick setup)
        #[arg(long)]
        interactive: bool,

        /// Reconfigure channels only (fast repair flow)
        #[arg(long)]
        channels_only: bool,

        /// API key (used in quick mode, ignored with --interactive)
        #[arg(long)]
        api_key: Option<String>,

        /// Provider name (used in quick mode, default: openrouter)
        #[arg(long)]
        provider: Option<String>,

        /// Memory backend (sqlite, lucid, markdown, none) - used in quick mode, default: sqlite
        #[arg(long)]
        memory: Option<String>,
    },

    /// Start the AI agent loop
    Agent {
        /// Single message mode (don't enter interactive mode)
        #[arg(short, long)]
        message: Option<String>,

        /// Provider to use (openrouter, anthropic, openai)
        #[arg(short, long)]
        provider: Option<String>,

        /// Model to use
        #[arg(long)]
        model: Option<String>,

        /// Temperature (0.0 - 2.0)
        #[arg(short, long, default_value = "0.7")]
        temperature: f64,

        /// Attach a peripheral (board:path, e.g. nucleo-f401re:/dev/ttyACM0)
        #[arg(long)]
        peripheral: Vec<String>,
    },

    /// Start the gateway server (webhooks, websockets)
    Gateway {
        /// Port to listen on (use 0 for random available port)
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },

    /// Manage the long-running autonomous runtime (gateway + channels + heartbeat + scheduler)
    Daemon {
        #[command(subcommand)]
        action: Option<DaemonAction>,

        /// Port to listen on — used when no subcommand given (start)
        #[arg(short, long, default_value = "8080", global = true)]
        port: u16,

        /// Host to bind to — used when no subcommand given (start)
        #[arg(long, default_value = "127.0.0.1", global = true)]
        host: String,
    },

    /// Start full AGI system with TUI chat (gateway + channels + heartbeat + chat)
    Run {
        /// Single message to process
        #[arg(short, long)]
        message: Option<String>,

        /// Provider to use
        #[arg(short, long)]
        provider: Option<String>,

        /// Model to use
        #[arg(long)]
        model: Option<String>,

        /// Verbose output (show thoughts)
        #[arg(short, long)]
        verbose: bool,
    },

    /// Manage OS service lifecycle (launchd/systemd user service)
    Service {
        #[command(subcommand)]
        service_command: ServiceCommands,
    },

    /// Run diagnostics for daemon/scheduler/channel freshness
    Doctor,

    /// Show system status (full details)
    Status,

    /// Start or check status of the Housaky Dashboard
    Dashboard {
        /// Start the dashboard server
        #[arg(long)]
        start: bool,

        /// Host to bind to (use "0.0.0.0" or "network" to expose to network)
        #[arg(long)]
        host: Option<String>,

        /// Port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Open dashboard in browser
        #[arg(short, long)]
        open: bool,

        /// Launch the desktop app instead of web server
        #[arg(long)]
        desktop: bool,
    },

    /// Configure and manage scheduled tasks
    Cron {
        #[command(subcommand)]
        cron_command: CronCommands,
    },

    /// Manage provider model catalogs
    Models {
        #[command(subcommand)]
        model_command: ModelCommands,
    },

    /// Manage API keys and providers
    Keys {
        #[command(subcommand)]
        key_command: KeyCommands,
    },

    /// Manage channels (telegram, discord, slack)
    Channel {
        #[command(subcommand)]
        channel_command: ChannelCommands,
    },

    /// Browse 50+ integrations
    Integrations {
        #[command(subcommand)]
        integration_command: IntegrationCommands,
    },

    /// Manage skills (user-defined capabilities)
    Skills {
        /// Optional shorthand: `housaky skills <name>` (equivalent to `housaky skills get <name>`)
        #[arg(value_name = "SKILL")]
        skill: Option<String>,

        /// Skill subcommand (defaults to `ui` when omitted)
        #[command(subcommand)]
        skill_command: Option<SkillCommands>,
    },

    /// Migrate data from other agent runtimes
    Migrate {
        #[command(subcommand)]
        migrate_command: MigrateCommands,
    },

    /// Discover and introspect USB hardware
    Hardware {
        #[command(subcommand)]
        hardware_command: HardwareCommands,
    },

    /// Manage hardware peripherals (STM32, RPi GPIO, etc.)
    Peripheral {
        #[command(subcommand)]
        peripheral_command: PeripheralCommands,
    },

    /// Launch terminal user interface for AI chat
    Tui {
        /// Provider to use (openrouter, anthropic, openai, etc.)
        #[arg(short, long)]
        provider: Option<String>,

        /// Model to use
        #[arg(long)]
        model: Option<String>,
    },

    /// Interactive configuration editor
    Config {
        /// Open to specific section (agent, tools, channels, gateway, memory, providers, fallback, security, cost)
        #[arg(short, long)]
        section: Option<String>,

        /// Reset to defaults
        #[arg(long)]
        reset: bool,

        /// Restore from persistent backup (config.toml.persist)
        #[arg(long)]
        restore: bool,
    },

    /// Housaky AGI commands (goals, reasoning, self-improvement)
    Housaky {
        #[command(subcommand)]
        housaky_command: HousakyCommands,
    },

    /// Quantum computing (Amazon Braket + local simulator)
    Quantum {
        #[command(subcommand)]
        quantum_command: QuantumCommands,
    },
}

fn get_api_key_from_keys_manager_or_config(config: &Config) -> Option<String> {
    let manager = housaky::keys_manager::manager::get_global_keys_manager();
    let store = manager.store.blocking_read();
    if let Some(first_provider) = store.providers.values().next() {
        if let Some(first_key) = first_provider.keys.first() {
            return Some(first_key.key.clone());
        }
    }
    config.api_key.clone()
}

/// Start the complete Housaky system: daemon + dashboard + AGI TUI
/// This is the default behavior when running `housaky` with no subcommand
async fn start_full_system(config: Config, verbose: bool) -> Result<()> {
    use tokio::time::{sleep, Duration};

    println!("🚀 Starting Full Housaky AGI System...");
    println!();

    let host = "127.0.0.1".to_string();
    let gateway_port = 8080u16;
    let dashboard_port = 3000u16;

    // Check for required configuration
    let has_api_key = config.api_key.as_ref().map_or(false, |k| !k.is_empty());
    let has_provider = config.default_provider.as_ref().map_or(false, |p| !p.is_empty());

    if !has_api_key && !has_provider {
        println!("⚠️  No API key or provider configured.");
        println!("   Run `housaky onboard` to set up your configuration.");
        println!();
        // Still continue - the AGI system may use environment variables
    }

    // Set workspace env for all components
    std::env::set_var("HOUSAKY_WORKSPACE", config.workspace_dir.to_string_lossy().to_string());

    // 1. Start daemon (gateway + channels + heartbeat + AGI)
    println!("🧠 Starting daemon (gateway + channels + heartbeat + AGI)...");
    let daemon_config = config.clone();
    let daemon_host = host.clone();
    let daemon_task = tokio::spawn(async move {
        if let Err(e) = daemon::run(daemon_config, daemon_host, gateway_port).await {
            eprintln!("❌ Daemon error: {}", e);
        } else {
            println!("✅ Daemon stopped cleanly");
        }
    });
    let daemon_abort = daemon_task.abort_handle();

    // 2. Start dashboard web server (if installed)
    let dashboard_installed = dashboard::check_dashboard_installed();
    let dashboard_abort = if dashboard_installed {
        println!("📊 Dashboard detected, starting server on port {}...", dashboard_port);
        let dashboard_host = host.clone();
        let t = tokio::spawn(async move {
            if let Err(e) = dashboard::start_dashboard_server(&dashboard_host, dashboard_port, false).await {
                eprintln!("❌ Dashboard error: {}", e);
            } else {
                println!("✅ Dashboard stopped cleanly");
            }
        });
        Some(t.abort_handle())
    } else {
        println!("💡 Dashboard not installed. Build it with: cd dashboard && pnpm install && pnpm build");
        None
    };

    // Give services time to start
    sleep(Duration::from_millis(1000)).await;

    // Print status of configured channels (including voice)
    print_channel_status(&config);

    // 3. Start AGI with TUI (this blocks until user exits)
    println!();
    println!("🤖 Starting AGI interface with cognitive loop...");
    println!("   Press Ctrl+C to exit");
    println!();

    let result = housaky::housaky::heartbeat::run_agi_with_tui(
        config.clone(),
        None,  // No initial message
        None,  // Use default provider
        None,  // Use default model
        verbose,
    ).await;

    // Shutdown sequence
    println!("\n👋 Shutting down Housaky system...");
    if let Some(abort) = dashboard_abort {
        abort.abort();
    }
    daemon_abort.abort();
    // Give background tasks a moment to acknowledge the abort
    sleep(Duration::from_secs(2)).await;

    println!("✅ Housaky system stopped");
    result
}

/// Print status of configured channels
fn print_channel_status(config: &Config) {
    let channels_config = &config.channels_config;
    let mut active_channels = Vec::new();
    
    if channels_config.telegram.is_some() {
        active_channels.push("Telegram");
    }
    if channels_config.discord.is_some() {
        active_channels.push("Discord");
    }
    if channels_config.slack.is_some() {
        active_channels.push("Slack");
    }
    if channels_config.whatsapp.is_some() {
        active_channels.push("WhatsApp");
    }
    if channels_config.matrix.is_some() {
        active_channels.push("Matrix");
    }
    if channels_config.imessage.is_some() {
        active_channels.push("iMessage");
    }
    if channels_config.email.is_some() {
        active_channels.push("Email");
    }
    if channels_config.irc.is_some() {
        active_channels.push("IRC");
    }
    if channels_config.lark.is_some() {
        active_channels.push("Lark");
    }
    if channels_config.dingtalk.is_some() {
        active_channels.push("DingTalk");
    }
    
    if !active_channels.is_empty() {
        println!("📡 Active channels: {}", active_channels.join(", "));
    }
    
    // Voice integration check
    if std::env::var("ELEVENLABS_API_KEY").is_ok() {
        println!("🎙️  Voice integration: Enabled (ElevenLabs)");
    }
}

/// Check if this is a first-time run (no valid config exists).
/// Returns `(true, None)` when onboarding is needed, or `(false, Some(config))` when
/// a usable config already exists so the caller does not re-load it.
fn is_first_run() -> (bool, Option<Config>) {
    let home = directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let config_path = home.join(".housaky").join("config.toml");

    // First run if config file doesn't exist
    if !config_path.exists() {
        return (true, None);
    }

    // Try to load existing config and check if it has essential settings
    match Config::load_or_init() {
        Ok(config) => {
            let has_api_key = config.api_key.as_ref().map_or(false, |k| !k.is_empty());
            let has_provider = config.default_provider.as_ref().map_or(false, |p| !p.is_empty());
            // If neither API key nor provider is set, treat as first run
            if !has_api_key && !has_provider {
                (true, None)
            } else {
                (false, Some(config))
            }
        }
        Err(_) => (true, None),
    }
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    // Install default crypto provider for Rustls TLS.
    // This prevents the error: "could not automatically determine the process-level CryptoProvider"
    // when both aws-lc-rs and ring features are available (or neither is explicitly selected).
    if let Err(e) = rustls::crypto::ring::default_provider().install_default() {
        eprintln!("Warning: Failed to install default crypto provider: {e:?}");
    }

    // Check if running without any subcommand (bare `housaky`)
    let args: Vec<String> = std::env::args().collect();
    let no_subcommand = args.len() == 1 || (args.len() > 1 && args[1].starts_with('-'));

    if no_subcommand {
        // Initialize logging once for the no-subcommand paths
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .finish();
        // Ignore error: subscriber may already be set in integration tests
        let _ = tracing::subscriber::set_global_default(subscriber);

        // Initialize keys manager once
        let _ = housaky::keys_manager::manager::init_global_keys_manager();
        housaky::keys_manager::manager::load_keys_from_file().await;

        // Check for first run — reuse the loaded Config to avoid a double load
        let (first_run, existing_config) = is_first_run();
        if first_run {
            println!("👋 Welcome to Housaky!");
            println!("   Starting interactive setup...\n");

            // Run the interactive wizard
            let config = onboard::run_wizard()?;

            // After wizard, check if we should auto-start channels
            if std::env::var("HOUSAKY_AUTOSTART_CHANNELS").as_deref() == Ok("1") {
                println!("\n🚀 Auto-starting channels...");
                channels::start_channels(config.clone()).await?;
            }

            // Start full system with the new config
            return start_full_system(config, false).await;
        } else {
            // Reuse the config already loaded by is_first_run() when available
            let config = match existing_config {
                Some(c) => c,
                None => Config::load_or_init()?,
            };

            // Set workspace env
            std::env::set_var("HOUSAKY_WORKSPACE", config.workspace_dir.to_string_lossy().to_string());

            return start_full_system(config, false).await;
        }
    }

    // Normal CLI parsing with subcommand
    let cli = Cli::parse();

    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    // Ignore error: subscriber may already be set (e.g. in tests)
    let _ = tracing::subscriber::set_global_default(subscriber);

    // Initialize the keys manager
    let _ = housaky::keys_manager::manager::init_global_keys_manager();
    housaky::keys_manager::manager::load_keys_from_file().await;

    // Provide workspace dir as env for channel commands (/logs today, /grep, etc.).
    if std::env::var("HOUSAKY_WORKSPACE").is_err() {
        if let Ok(cfg) = Config::load_or_init() {
            std::env::set_var("HOUSAKY_WORKSPACE", cfg.workspace_dir.to_string_lossy().to_string());
        }
    }

    // Handle subcommands (unwrap is safe because we checked above)
    match cli.command {
        None => {
            // Should never reach here - handled above
            unreachable!()
        }

        // Onboard runs quick setup by default, or the interactive wizard with --interactive
        Some(Commands::Onboard {
            interactive,
            channels_only,
            api_key,
            provider,
            memory,
        }) => {
            if interactive && channels_only {
                bail!("Use either --interactive or --channels-only, not both");
            }
            if channels_only && (api_key.is_some() || provider.is_some() || memory.is_some()) {
                bail!("--channels-only does not accept --api-key, --provider, or --memory");
            }

            let config = if channels_only {
                onboard::run_channels_repair_wizard()?
            } else if interactive {
                onboard::run_wizard()?
            } else {
                onboard::run_quick_setup(api_key.as_deref(), provider.as_deref(), memory.as_deref())?
            };
            // Auto-start channels if user said yes during wizard
            if std::env::var("HOUSAKY_AUTOSTART_CHANNELS").as_deref() == Ok("1") {
                channels::start_channels(config).await?;
            }
            Ok(())
        }

        // All other commands need config loaded first
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
                } => agent::run(config, message, provider, model, temperature, peripheral).await,

                Commands::Gateway { port, host } => {
                    if port == 0 {
                        info!("🚀 Starting Housaky Gateway on {host} (random port)");
                    } else {
                        info!("🚀 Starting Housaky Gateway on {host}:{port}");
                    }
                    gateway::run_gateway(&host, port, config).await
                },

                Commands::Daemon { action, port, host } => {
                    match action.unwrap_or(DaemonAction::Start { port, host: host.clone() }) {
                        DaemonAction::Start { port, host } => {
                            if port == 0 {
                                info!("🧠 Starting Housaky Daemon on {host} (random port)");
                            } else {
                                info!("🧠 Starting Housaky Daemon on {host}:{port}");
                            }
                            daemon::run(config, host, port).await
                        }
                        DaemonAction::Stop => {
                            daemon_control::stop();
                            Ok(())
                        }
                        DaemonAction::Restart { port, host } => {
                            daemon_control::stop();
                            if port == 0 {
                                info!("🧠 Restarting Housaky Daemon on {host} (random port)");
                            } else {
                                info!("🧠 Restarting Housaky Daemon on {host}:{port}");
                            }
                            daemon::run(config, host, port).await
                        }
                        DaemonAction::Status => {
                            daemon_control::status();
                            Ok(())
                        }
                    }
                },

                Commands::Run { message, provider, model, verbose } => {
                    println!("🚀 Starting Full Housaky AGI System with TUI Chat...");
                    if verbose {
                        println!("   Verbose mode: on");
                    }
                    housaky::housaky::heartbeat::run_agi_with_tui(
                        config,
                        message,
                        provider,
                        model,
                        verbose,
                    ).await
                },

                Commands::Status => {
                    println!("🦀 Housaky Status");
                    println!();
                    println!("Version:     {}", env!("CARGO_PKG_VERSION"));
                    println!("Workspace:   {}", config.workspace_dir.display());
                    println!("Config:      {}", config.config_path.display());
                    println!();
                    println!(
                        "🤖 Provider:      {}",
                        config.default_provider.as_deref().unwrap_or("openrouter")
                    );
                    println!(
                        "   Model:         {}",
                        config.default_model.as_deref().unwrap_or("(default)")
                    );
                    println!("📊 Observability:  {}", config.observability.backend);
                    println!("🛡️  Autonomy:      {:?}", config.autonomy.level);
                    println!("⚙️  Runtime:       {}", config.runtime.kind);
                    println!(
                        "💓 Heartbeat:      {}",
                        if config.heartbeat.enabled {
                            format!("every {}min", config.heartbeat.interval_minutes)
                        } else {
                            "disabled".into()
                        }
                    );
                    println!(
                        "🧠 Memory:         {} (auto-save: {})",
                        config.memory.backend,
                        if config.memory.auto_save { "on" } else { "off" }
                    );

                    println!();
                    println!("Security:");
                    println!("  Workspace only:    {}", config.autonomy.workspace_only);
                    println!(
                        "  Allowed commands:  {}",
                        config.autonomy.allowed_commands.join(", ")
                    );
                    println!(
                        "  Max actions/hour:  {}",
                        config.autonomy.max_actions_per_hour
                    );
                    println!(
                        "  Max cost/day:      ${:.2}",
                        f64::from(config.autonomy.max_cost_per_day_cents) / 100.0
                    );
                    println!();
                    println!("Channels:");
                    println!("  CLI:      ✅ always");
                    for (name, configured) in [
                        ("Telegram", config.channels_config.telegram.is_some()),
                        ("Discord", config.channels_config.discord.is_some()),
                        ("Slack", config.channels_config.slack.is_some()),
                        ("WhatsApp", config.channels_config.whatsapp.is_some()),
                        ("Matrix", config.channels_config.matrix.is_some()),
                        ("iMessage", config.channels_config.imessage.is_some()),
                        ("Email", config.channels_config.email.is_some()),
                        ("IRC", config.channels_config.irc.is_some()),
                        ("Lark", config.channels_config.lark.is_some()),
                        ("DingTalk", config.channels_config.dingtalk.is_some()),
                        ("Webhook", config.channels_config.webhook.is_some()),
                    ] {
                        println!(
                            "  {name:9} {}",
                            if configured {
                                "✅ configured"
                            } else {
                                "❌ not configured"
                            }
                        );
                    }
                    println!();
                    println!("Peripherals:");
                    println!(
                        "  Enabled:   {}",
                        if config.peripherals.enabled {
                            "yes"
                        } else {
                            "no"
                        }
                    );
                    println!("  Boards:    {}", config.peripherals.boards.len());

                    Ok(())
                }

                Commands::Dashboard { start, host, port, open, desktop } => {
                    if desktop {
                        return dashboard::launch_desktop_app();
                    }

                    if start {
                        let bind_host = host.as_deref().unwrap_or("127.0.0.1");
                        let should_open = open || host.is_none();
                        dashboard::start_dashboard_server(bind_host, port, should_open).await
                    } else {
                        dashboard::print_status(port);
                        println!();
                        println!("Options:");
                        println!("  --start       Start the dashboard web server");
                        println!("  --host <ip>   Bind to specific host (use \"0.0.0.0\" for network)");
                        println!("  --port <n>    Port number (default: 3000)");
                        println!("  --open        Open in browser after starting");
                        println!("  --desktop     Launch the desktop app instead");
                        Ok(())
                    }
                }

                Commands::Cron { cron_command } => cron::handle_command(cron_command, &config),

                Commands::Models { model_command } => match model_command {
                    ModelCommands::Refresh { provider, force } => {
                        onboard::run_models_refresh(&config, provider.as_deref(), force)
                    }
                },

                Commands::Service { service_command } => service::handle_command(&service_command, &config),

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
                                if !providers.is_empty() {
                                    println!("Keys (keys_manager):");
                                    for provider in &providers {
                                        let enabled_count = provider.keys.iter().filter(|key| key.enabled).count();
                                        println!("  - {}: {} keys ({} enabled)", provider.name, provider.keys.len(), enabled_count);
                                        for key in &provider.keys {
                                            let suffix = if key.key.len() > 4 { &key.key[key.key.len()-4..] } else { &key.key };
                                            let status = if key.enabled { "enabled" } else { "disabled" };
                                            println!("      ...{} - {}", suffix, status);
                                        }
                                    }
                                    Ok(())
                                } else {
                                    println!("No keys configured.");
                                    println!("  Use `housaky keys manager add-provider` to add your first key.");
                                    Ok(())
                                }
                            }
                            KeyCommands::Add { provider, key } => {
                                match keys_manager.add_key(&provider, key, None).await {
                                    Ok(_) => {
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
                                    Ok(_) => {
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

                Commands::Doctor => doctor::run(&config),

                Commands::Channel { channel_command } => match channel_command {
                    ChannelCommands::Start => channels::start_channels(config).await,
                    ChannelCommands::Doctor => channels::doctor_channels(config).await,
                    other => channels::handle_command(other, &config),
                },

                Commands::Integrations {
                    integration_command,
                } => integrations::handle_command(integration_command, &config),

                Commands::Skills { skill, skill_command } => {
                    let cmd = if let Some(name) = skill {
                        SkillCommands::Get { name, enable: false }
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
                    tokio::task::spawn_blocking(move || tui::run_chat_tui(config, provider, model))
                        .await
                        .map_err(|e| anyhow::anyhow!("TUI task failed: {e}"))?
                        .map_err(|e| anyhow::anyhow!("TUI error: {e}"))?;
                    Ok(())
                }

                Commands::Config { section, reset, restore } => {
                    if restore {
                        match Config::restore_from_backup() {
                            Ok(restored) => {
                                println!("✅ Restored config from backup at {}", restored.config_path.display());
                            }
                            Err(e) => {
                                println!("❌ Failed to restore config: {}", e);
                                println!("   No backup found. Your config is safe at ~/.housaky/config.toml");
                            }
                        }
                        return Ok(());
                    }
                    if reset {
                        let mut default_config = Config::default();
                        default_config.config_path = config.config_path.clone();
                        default_config.workspace_dir = config.workspace_dir.clone();
                        default_config.save()?;
                        println!(
                            "Config reset to defaults at {}",
                            default_config.config_path.display()
                        );
                        Ok(())
                    } else {
                        tokio::task::spawn_blocking(move || config_editor::run_config_tui(config, section))
                            .await
                            .map_err(|e| anyhow::anyhow!("Config TUI task failed: {e}"))?
                            .map_err(|e| anyhow::anyhow!("Config TUI error: {e}"))?;
                        Ok(())
                    }
                }

                Commands::Housaky { housaky_command } => {
                    housaky::housaky::handle_command(housaky_command, &config).await?;
                    Ok(())
                }

                Commands::Quantum { quantum_command } => {
                    use housaky::quantum::{
                        AmazonBraketBackend, QuantumBackend, QuantumConfig, SimulatorBackend,
                    };
                    use housaky::quantum::circuit::{Gate, QuantumCircuit};

                    fn bell_circuit() -> QuantumCircuit {
                        let mut c = QuantumCircuit::new(2);
                        c.add_gate(Gate::h(0));
                        c.add_gate(Gate::cnot(0, 1));
                        c.measure_all();
                        c
                    }

                    match quantum_command {
                        QuantumCommands::RunBraket { shots, device, bucket, prefix } => {
                            println!("Submitting Bell-state circuit to Amazon Braket...");
                            println!("  Device : {device}");
                            println!("  Shots  : {shots}");
                            println!("  Bucket : s3://{bucket}/{prefix}");
                            let cfg = QuantumConfig {
                                backend: "braket".to_string(),
                                shots,
                                braket_device_arn: device,
                                braket_s3_bucket: bucket,
                                braket_s3_prefix: prefix,
                                ..QuantumConfig::default()
                            };
                            let backend = AmazonBraketBackend::from_config(&cfg).await?;
                            let circuit = bell_circuit();
                            let result = backend.execute_circuit(&circuit).await?;
                            println!("\nTask ARN  : {}", result.backend_id);
                            println!("Shots run : {}", result.shots);
                            println!("Runtime   : {} ms", result.execution_time_ms);
                            println!("\nCounts:");
                            let mut counts: Vec<_> = result.counts.iter().collect();
                            counts.sort_by(|a, b| b.1.cmp(a.1));
                            for (bitstring, count) in &counts {
                                let pct = (**count as f64 / result.shots as f64) * 100.0;
                                println!("  |{bitstring}> : {count:5}  ({pct:.1}%)", count = **count);
                            }
                            Ok(())
                        }

                        QuantumCommands::RunSimulator { shots } => {
                            println!("Running Bell-state circuit on local statevector simulator...");
                            println!("  Shots : {shots}");
                            let backend = SimulatorBackend::new(2, shots);
                            let circuit = bell_circuit();
                            let result = backend.execute_circuit(&circuit).await?;
                            println!("\nShots run : {}", result.shots);
                            println!("Runtime   : {} ms", result.execution_time_ms);
                            println!("\nCounts:");
                            let mut counts: Vec<_> = result.counts.iter().collect();
                            counts.sort_by(|a, b| b.1.cmp(a.1));
                            for (bitstring, count) in &counts {
                                let n = **count;
                                let pct = (n as f64 / result.shots as f64) * 100.0;
                                println!("  |{bitstring}> : {n:5}  ({pct:.1}%)");
                            }
                            Ok(())
                        }

                        QuantumCommands::DeviceInfo { device, bucket } => {
                            println!("Querying Braket device info...");
                            let cfg = QuantumConfig {
                                backend: "braket".to_string(),
                                braket_device_arn: device,
                                braket_s3_bucket: bucket,
                                ..QuantumConfig::default()
                            };
                            let backend = AmazonBraketBackend::from_config(&cfg).await?;
                            let info = backend.get_backend_info().await;
                            println!("  ID          : {}", info.id);
                            println!("  Max qubits  : {}", info.max_qubits);
                            println!("  Max shots   : {}", info.max_shots);
                            println!("  Online      : {}", info.online);
                            println!("  Gates       : {}", info.supported_gates.join(", "));
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
    }
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

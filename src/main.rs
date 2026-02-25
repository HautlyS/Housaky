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
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

// Explicitly declare we're using the housaky library
extern crate housaky;

// Use the library modules
use housaky::{
    agent, channels, commands, config_editor, cron, daemon, dashboard, doctor, gateway, hardware,
    integrations, key_management, migration, onboard, peripherals, service, skills, tui, Config,
};

use commands::{
    ChannelCommands, CronCommands, HardwareCommands, HousakyCommands, IntegrationCommands,
    KeyCommands, KvmCommands, MigrateCommands, ModelCommands, PeripheralCommands, ServiceCommands, SkillCommands,
};

/// `Housaky` - Zero overhead. Zero compromise. 100% Rust.
#[derive(Parser, Debug)]
#[command(name = "housaky")]
#[command(author = "theonlyhennygod")]
#[command(version = "0.1.0")]
#[command(about = "The fastest, smallest AI assistant.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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

    /// Start long-running autonomous runtime (gateway + channels + heartbeat + scheduler)
    Daemon {
        /// Port to listen on (use 0 for random available port)
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
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

    /// KVM (Key Virtual Management) - manage multiple API keys with intelligent rotation
    Kvm {
        #[command(subcommand)]
        kvm_command: KvmCommands,
    },
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

    let cli = Cli::parse();

    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Initialize KVM manager and load keys from disk
    let _ = key_management::init_global_kvm_manager();

    // Provide workspace dir as env for channel commands (/logs today, /grep, etc.).
    if std::env::var("HOUSAKY_WORKSPACE").is_err() {
        if let Ok(cfg) = Config::load_or_init() {
            std::env::set_var("HOUSAKY_WORKSPACE", cfg.workspace_dir.to_string_lossy().to_string());
        }
    }

    // Onboard runs quick setup by default, or the interactive wizard with --interactive
    if let Commands::Onboard {
        interactive,
        channels_only,
        api_key,
        provider,
        memory,
    } = &cli.command
    {
        if *interactive && *channels_only {
            bail!("Use either --interactive or --channels-only, not both");
        }
        if *channels_only && (api_key.is_some() || provider.is_some() || memory.is_some()) {
            bail!("--channels-only does not accept --api-key, --provider, or --memory");
        }

        let config = if *channels_only {
            onboard::run_channels_repair_wizard()?
        } else if *interactive {
            onboard::run_wizard()?
        } else {
            onboard::run_quick_setup(api_key.as_deref(), provider.as_deref(), memory.as_deref())?
        };
        // Auto-start channels if user said yes during wizard
        if std::env::var("HOUSAKY_AUTOSTART_CHANNELS").as_deref() == Ok("1") {
            channels::start_channels(config).await?;
        }
        return Ok(());
    }

    // All other commands need config loaded first
    let mut config = Config::load_or_init()?;
    config.apply_env_overrides();

    match cli.command {
        Commands::Onboard { .. } => unreachable!(),

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
        }

        Commands::Daemon { port, host } => {
            if port == 0 {
                info!("🧠 Starting Housaky Daemon on {host} (random port)");
            } else {
                info!("🧠 Starting Housaky Daemon on {host}:{port}");
            }
            daemon::run(config, host, port).await
        }

        Commands::Run { message, provider, model, verbose } => {
            println!("🚀 Starting Full Housaky AGI System with TUI Chat...");
            println!("   Verbose mode: {}", verbose);
            
            housaky::housaky::heartbeat::run_agi_with_tui(
                config, 
                message, 
                provider, 
                model, 
                verbose
            ).await
        }

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
            // New centralized manager (keys.json)
            let keys_manager = housaky::keys_manager::manager::get_global_keys_manager();
            // Legacy KVM manager (kvm_keys.json)
            let kvm = key_management::get_global_kvm_manager();
            
            async fn handle_keys_command(
                config: &mut Config,
                keys_manager: &housaky::keys_manager::manager::KeysManager,
                kvm: Arc<key_management::KvmKeyManager>,
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
                    // Legacy KVM-based commands:
                    _ => {
                        // Load keys from disk if not already loaded
                        key_management::load_kvm_from_file().await;
                        
                        match cmd {
                            KeyCommands::List => {
                                let store = kvm.store.read().await;
                        if store.providers.is_empty() {
                            println!("No KVM providers configured.");
                            println!("  Use `housaky kvm add-provider` to add providers with keys.");
                            println!("  Or use `housaky kvm interactive` for interactive mode.");
                        } else {
                            println!("KVM Providers:");
                            for (name, provider) in &store.providers {
                                let enabled_count = provider.keys.iter().filter(|k| k.enabled).count();
                                println!("  - {}: {} keys ({} enabled)", name, provider.keys.len(), enabled_count);
                                for key in &provider.keys {
                                    let status = if key.enabled { "enabled" } else { "disabled" };
                                    let suffix = if key.key.len() > 4 { &key.key[key.key.len()-4..] } else { &key.key };
                                    println!("      [{}] ...{} - {}", key.id.chars().take(8).collect::<String>(), suffix, status);
                                }
                            }
                        }
                        Ok(())
                    }
                    KeyCommands::Add { provider, key } => {
                        match kvm.add_provider_with_template(&provider, "custom", vec![key]).await {
                            Ok(_) => {
                                println!("Added key to provider: {}", provider);
                                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                Ok(())
                            }
                            Err(e) => {
                                println!("Error adding key: {}", e);
                                Err(anyhow::anyhow!(e))
                            }
                        }
                    }
                    KeyCommands::Remove { provider } => {
                        let mut store = kvm.store.write().await;
                        if store.providers.remove(&provider).is_some() {
                            let mut rs = kvm.rotation_states.write().await;
                            rs.remove(&provider);
                            println!("Removed provider: {}", provider);
                            Ok(())
                        } else {
                            println!("Provider not found: {}", provider);
                            Err(anyhow::anyhow!("Provider not found"))
                        }
                    }
                    KeyCommands::Rotate => {
                        let store = kvm.store.read().await;
                        let names: Vec<_> = store.providers.keys().cloned().collect();
                        drop(store);
                        for name in names {
                            if let Some(key) = kvm.rotate_key(&name).await {
                                println!("Rotated key for {}: ...{}", name, &key.key[key.key.len().saturating_sub(4)..]);
                            }
                        }
                        Ok(())
                    }
                    KeyCommands::Manager(_) => unreachable!("handled above"),
                        }
                    }
                }
            }
            
            handle_keys_command(&mut config, &keys_manager, kvm, key_command).await
        }

        Commands::Kvm { kvm_command } => {
            let kvm = key_management::get_global_kvm_manager();
            
            async fn handle_kvm_command(kvm: Arc<key_management::KvmKeyManager>, cmd: KvmCommands) -> Result<()> {
                // Load keys from disk if not already loaded
                key_management::load_kvm_from_file().await;
                
                match cmd {
                    KvmCommands::List => {
                        let store = kvm.store.read().await;
                        if store.providers.is_empty() {
                            println!("No KVM providers configured.");
                            println!("  Use `housaky kvm add-provider` to add providers.");
                            println!("  Or use `housaky kvm interactive` for interactive mode.");
                        } else {
                            println!("KVM Providers:");
                            for (name, provider) in &store.providers {
                                let enabled_count = provider.keys.iter().filter(|k| k.enabled).count();
                                println!("\n  [{}]", name);
                                println!("      Base URL: {:?}", provider.base_url);
                                println!("      Auth: {}", provider.auth_method);
                                println!("      Keys: {} total, {} enabled", provider.keys.len(), enabled_count);
                                for key in &provider.keys {
                                    let status = if key.enabled { "✓" } else { "✗" };
                                    let suffix = if key.key.len() > 4 { &key.key[key.key.len()-4..] } else { &key.key };
                                    println!("        {} ...{} - {}", status, suffix, key.id);
                                }
                            }
                        }
                        Ok(())
                    }
                    KvmCommands::AddProvider { name, template, base_url, auth_method, keys, models } => {
                        let tmpl = template.as_deref().unwrap_or("custom");
                        
                        if keys.is_empty() {
                            println!("Error: At least one key is required");
                            return Err(anyhow::anyhow!("No keys provided"));
                        }
                        
                        match kvm.add_provider_with_template(&name, tmpl, keys.clone()).await {
                            Ok(_) => {
                                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                
                                if let Some(url) = base_url {
                                    let mut store = kvm.store.write().await;
                                    if let Some(provider) = store.providers.get_mut(&name) {
                                        provider.base_url = Some(url);
                                    }
                                }
                                
                                if let Some(auth) = auth_method {
                                    let mut store = kvm.store.write().await;
                                    if let Some(provider) = store.providers.get_mut(&name) {
                                        provider.auth_method = auth;
                                    }
                                }
                                
                                println!("Added provider '{}' with {} keys", name, keys.len());
                                if !models.is_empty() {
                                    println!("  Models: {}", models.join(", "));
                                }
                                let _ = kvm.save().await;
                                Ok(())
                            }
                            Err(e) => {
                                println!("Error adding provider: {}", e);
                                Err(anyhow::anyhow!(e))
                            }
                        }
                    }
                    KvmCommands::AddKey { provider, key } => {
                        match kvm.add_key(&provider, key.clone()).await {
                            Ok(_) => {
                                println!("Added key to provider: {}", provider);
                                let _ = kvm.save().await;
                                Ok(())
                            }
                            Err(e) => {
                                println!("Error adding key: {}", e);
                                Err(anyhow::anyhow!(e))
                            }
                        }
                    }
                    KvmCommands::RemoveProvider { name } => {
                        let mut store = kvm.store.write().await;
                        if store.providers.remove(&name).is_some() {
                            let mut rs = kvm.rotation_states.write().await;
                            rs.remove(&name);
                            println!("Removed provider: {}", name);
                            drop(store);
                            let _ = kvm.save().await;
                            Ok(())
                        } else {
                            println!("Provider not found: {}", name);
                            Err(anyhow::anyhow!("Provider not found"))
                        }
                    }
                    KvmCommands::RemoveKey { provider, key } => {
                        let mut store = kvm.store.write().await;
                        if let Some(p) = store.providers.get_mut(&provider) {
                            let initial_len = p.keys.len();
                            p.keys.retain(|k| !k.key.contains(&key) && !k.id.contains(&key));
                            if p.keys.len() < initial_len {
                                println!("Removed key from provider: {}", provider);
                                drop(store);
                                let _ = kvm.save().await;
                                Ok(())
                            } else {
                                println!("Key not found in provider: {}", provider);
                                Err(anyhow::anyhow!("Key not found"))
                            }
                        } else {
                            println!("Provider not found: {}", provider);
                            Err(anyhow::anyhow!("Provider not found"))
                        }
                    }
                    KvmCommands::Rotate { provider } => {
                        match provider {
                            Some(name) => {
                                if let Some(key) = kvm.rotate_key(&name).await {
                                    println!("Rotated key for {}: ...{}", name, &key.key[key.key.len().saturating_sub(4)..]);
                                } else {
                                    println!("Provider not found: {}", name);
                                }
                            }
                            None => {
                                let store = kvm.store.read().await;
                                let names: Vec<_> = store.providers.keys().cloned().collect();
                                drop(store);
                                for name in names {
                                    if let Some(key) = kvm.rotate_key(&name).await {
                                        println!("Rotated key for {}: ...{}", name, &key.key[key.key.len().saturating_sub(4)..]);
                                    }
                                }
                            }
                        }
                        Ok(())
                    }
                    KvmCommands::Stats { provider } => {
                        let store = kvm.store.read().await;
                        let providers: Vec<_> = match provider {
                            Some(p) => store.providers.get(&p).map(|pr| (p.clone(), pr)).into_iter().collect(),
                            None => store.providers.iter().map(|(n, p)| (n.clone(), p)).collect(),
                        };
                        
                        for (name, prov) in providers {
                            println!("\n[{}]", name);
                            for key in &prov.keys {
                                let usage = &key.usage;
                                println!("  Key: ...{}", &key.key[key.key.len().saturating_sub(4)..]);
                                println!("    Total requests: {}", usage.total_requests);
                                println!("    Successful: {}", usage.successful_requests);
                                println!("    Failed: {}", usage.failed_requests);
                                println!("    Rate limited: {}", usage.rate_limited_count);
                                println!("    Enabled: {}", key.enabled);
                            }
                        }
                        Ok(())
                    }
                    KvmCommands::SetStrategy { provider, strategy } => {
                        use key_management::KvmRotationStrategy;
                        
                        let strat = match strategy.as_str() {
                            "RoundRobin" => KvmRotationStrategy::RoundRobin,
                            "Priority" => KvmRotationStrategy::Priority,
                            "UsageBased" => KvmRotationStrategy::UsageBased,
                            "ErrorBased" => KvmRotationStrategy::ErrorBased,
                            "HealthBased" => KvmRotationStrategy::HealthBased,
                            "Adaptive" => KvmRotationStrategy::Adaptive,
                            _ => {
                                println!("Unknown strategy: {}", strategy);
                                return Err(anyhow::anyhow!("Unknown strategy"));
                            }
                        };
                        
                        kvm.set_rotation_strategy(strat).await;
                        println!("Set rotation strategy to {} for {}", strategy, provider);
                        Ok(())
                    }
                    KvmCommands::EnableKey { provider, key } => {
                        match kvm.enable_key(&provider, &key).await {
                            Ok(_) => {
                                println!("Enabled key in provider: {}", provider);
                                let _ = kvm.save().await;
                                Ok(())
                            }
                            Err(e) => {
                                println!("Error enabling key: {}", e);
                                Err(anyhow::anyhow!(e))
                            }
                        }
                    }
                    KvmCommands::DisableKey { provider, key } => {
                        match kvm.disable_key(&provider, &key).await {
                            Ok(_) => {
                                println!("Disabled key in provider: {}", provider);
                                let _ = kvm.save().await;
                                Ok(())
                            }
                            Err(e) => {
                                println!("Error disabling key: {}", e);
                                Err(anyhow::anyhow!(e))
                            }
                        }
                    }
                    KvmCommands::Export { path } => {
                        let store = kvm.store.read().await;
                        let json = serde_json::to_string_pretty(&*store).unwrap();
                        match path {
                            Some(p) => {
                                std::fs::write(&p, &json).unwrap();
                                println!("Exported KVM config to: {}", p);
                            }
                            None => {
                                println!("{}", json);
                            }
                        }
                        Ok(())
                    }
                    KvmCommands::Import { path } => {
                        let content = std::fs::read_to_string(&path).unwrap();
                        let new_store: key_management::KvmKeyStore = serde_json::from_str(&content).unwrap();
                        
                        let mut store = kvm.store.write().await;
                        *store = new_store;
                        
                        let mut rs = kvm.rotation_states.write().await;
                        rs.clear();
                        for name in store.providers.keys() {
                            rs.insert(name.clone(), Arc::new(key_management::KeyRotationState::new()));
                        }
                        
                        println!("Imported KVM config from: {}", path);
                        Ok(())
                    }
                    KvmCommands::Interactive => {
                        println!("Starting interactive KVM TUI...");
                        println!("Not implemented yet - use CLI commands for now.");
                        Ok(())
                    }
                }
            }
            
            handle_kvm_command(kvm, kvm_command).await
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
            // Run TUI in a blocking task to avoid nested runtime issues
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
                // Run config editor TUI in a blocking task
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

//! Command handlers for Housaky CLI
//!
//! This module contains all the command handling logic extracted from main.rs.

use anyhow::{bail, Result};
use tracing::info;

use crate::cli::args::DaemonAction;
use crate::config::Config;
use crate::commands::DoctorCommands;

/// Handle the Onboard command
pub async fn handle_onboard(
    interactive: bool,
    channels_only: bool,
    api_key: Option<String>,
    provider: Option<String>,
    memory: Option<String>,
) -> Result<Config> {
    if interactive && channels_only {
        bail!("Use either --interactive or --channels-only, not both");
    }
    if channels_only && (api_key.is_some() || provider.is_some() || memory.is_some()) {
        bail!("--channels-only does not accept --api-key, --provider, or --memory");
    }

    let config = if channels_only {
        crate::onboard::run_channels_repair_wizard()?
    } else if interactive {
        crate::onboard::run_wizard()?
    } else {
        crate::onboard::run_quick_setup(
            api_key.as_deref(),
            provider.as_deref(),
            memory.as_deref(),
        )?
    };
    
    // Auto-start channels if user said yes during wizard
    if std::env::var("HOUSAKY_AUTOSTART_CHANNELS").as_deref() == Ok("1") {
        crate::channels::start_channels(config.clone()).await?;
    }
    
    Ok(config)
}

/// Handle the Agent command
pub async fn handle_agent(
    config: Config,
    message: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    temperature: f64,
    peripheral: Vec<String>,
) -> Result<()> {
    crate::agent::run(config, message, provider, model, temperature, peripheral).await
}

/// Handle the Gateway command
pub async fn handle_gateway(host: String, port: u16, config: Config) -> Result<()> {
    if port == 0 {
        info!("🚀 Starting Housaky Gateway on {host} (random port)");
    } else {
        info!("🚀 Starting Housaky Gateway on {host}:{port}");
    }
    crate::gateway::run_gateway(&host, port, config).await
}

/// Handle the Daemon command
pub async fn handle_daemon(
    config: Config,
    action: Option<DaemonAction>,
    port: u16,
    host: String,
) -> Result<()> {
    use crate::daemon::control as daemon_control;
    
    match action.unwrap_or(DaemonAction::Start {
        port,
        host: host.clone(),
    }) {
        DaemonAction::Start { port, host } => {
            if port == 0 {
                info!("🧠 Starting Housaky Daemon on {host} (random port)");
            } else {
                info!("🧠 Starting Housaky Daemon on {host}:{port}");
            }
            crate::daemon::run(config, host, port).await
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
            crate::daemon::run(config, host, port).await
        }
        DaemonAction::Status => {
            daemon_control::status();
            Ok(())
        }
    }
}

/// Handle the Status command
pub fn handle_status(config: &Config) -> Result<()> {
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

    println!();
    println!("⚛️  Quantum:");
    println!(
        "  Enabled:   {}",
        if config.quantum.enabled {
            "✅ yes"
        } else {
            "❌ no  (set [quantum] enabled = true)"
        }
    );
    if config.quantum.enabled {
        println!("  Backend:   {}", config.quantum.backend);
        println!("  Max qubits: {}", config.quantum.max_qubits);
        println!("  Shots:     {}", config.quantum.shots);
        if config.quantum.backend == "braket" {
            let arn = if config.quantum.braket_device_arn.is_empty() {
                "arn:aws:braket:::device/quantum-simulator/amazon/sv1 (default)"
                    .to_string()
            } else {
                config.quantum.braket_device_arn.clone()
            };
            println!("  Device:    {}", arn);
            println!(
                "  S3 bucket: {}",
                if config.quantum.braket_s3_bucket.is_empty() {
                    "⚠️  not set".to_string()
                } else {
                    config.quantum.braket_s3_bucket.clone()
                }
            );
            println!("  Activate:  https://console.aws.amazon.com/braket/home");
        }
        println!("  Features:  QAOA goal-sched={} | QA mem-opt={} | Grover search={} | VQE fit={}",
            config.quantum.agi.enable_goal_scheduling,
            config.quantum.agi.enable_memory_optimization,
            config.quantum.agi.enable_reasoning_search,
            config.quantum.agi.enable_fitness_exploration,
        );
        if config.quantum.agi.cycle_budget_usd > 0.0 {
            println!(
                "  Budget:    ${:.4}/day",
                config.quantum.agi.cycle_budget_usd
            );
        }
    }

    Ok(())
}

/// Handle the Dashboard command
pub async fn handle_dashboard(
    start: bool,
    host: Option<String>,
    port: u16,
    open: bool,
    desktop: bool,
) -> Result<()> {
    if desktop {
        return crate::dashboard::launch_desktop_app();
    }

    if start {
        let bind_host = host.as_deref().unwrap_or("127.0.0.1");
        let should_open = open || host.is_none();
        crate::dashboard::start_dashboard_server(bind_host, port, should_open).await
    } else {
        crate::dashboard::print_status(port);
        println!();
        println!("Options:");
        println!("  --start       Start the dashboard web server");
        println!(
            "  --host <ip>   Bind to specific host (use \"0.0.0.0\" for network)"
        );
        println!("  --port <n>    Port number (default: 3000)");
        println!("  --open        Open in browser after starting");
        println!("  --desktop     Launch the desktop app instead");
        Ok(())
    }
}

/// Handle the Doctor command
pub fn handle_doctor(config: &Config, doctor_command: Option<DoctorCommands>) -> Result<()> {
    match doctor_command.unwrap_or(DoctorCommands::Run) {
        DoctorCommands::Run => crate::doctor::run(config)?,
        DoctorCommands::Fix => crate::doctor::run_fix(config)?,
        DoctorCommands::Channels => crate::doctor::run_channels(config)?,
        DoctorCommands::Security => crate::doctor::run_security(config)?,
        DoctorCommands::Json => {
            let report = crate::doctor::collect(config);
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }
    Ok(())
}

/// Handle the Config command
pub async fn handle_config(
    config: Config,
    section: Option<String>,
    reset: bool,
    restore: bool,
) -> Result<()> {
    if restore {
        match Config::restore_from_backup() {
            Ok(restored) => {
                println!(
                    "✅ Restored config from backup at {}",
                    restored.config_path.display()
                );
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
        // Run config TUI in blocking context
        tokio::task::spawn_blocking(move || {
            crate::config_editor::run_config_tui(config, section)
        })
        .await
        .map_err(|e| anyhow::anyhow!("Config TUI task failed: {e}"))?
        .map_err(|e| anyhow::anyhow!("Config TUI error: {e}"))?;
        Ok(())
    }
}

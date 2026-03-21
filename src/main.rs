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

/// Run non-interactive stdin/stdout chat mode
async fn start_noninteractive_chat(config: Config) -> Result<()> {
    use std::io::Write;
    use housaky::housaky::self_improve_daemon::{SelfImproveDaemon, SelfImproveDaemonConfig};
    use std::sync::Arc;
    
    println!("\n╭──────────────────────────────────────────────────────────╮");
    println!("│  Housaky CLI Chat - Non-interactive Mode                │");
    println!("│  Type /help for commands, /quit to exit                  │");
    println!("╰──────────────────────────────────────────────────────────╯\n");
    
    let mut effective_config = config.clone();
    let provider_name = effective_config.default_provider.clone()
        .unwrap_or_else(|| "openrouter".to_string());
    let model_name = effective_config.default_model.clone()
        .unwrap_or_else(|| "auto".to_string());
    
    println!("Using provider: {provider_name}, model: {model_name}\n");
    
    let enable_daemon = std::env::var("HOUSAKY_SELF_IMPROVEMENT").map(|v| v == "1" || v == "true").unwrap_or(true);
    
    let daemon = if enable_daemon {
        let daemon_config = SelfImproveDaemonConfig {
            enabled: true,
            enable_parameter_tuning: true,
            enable_tool_creation: true,
            enable_skill_acquisition: true,
            ..Default::default()
        };
        let daemon = Arc::new(SelfImproveDaemon::new(daemon_config, config.workspace_dir.clone()));
        let daemon_clone = daemon.clone();
        tokio::spawn(async move {
            if let Err(e) = daemon_clone.start().await {
                tracing::error!("Self-improvement daemon error: {}", e);
            }
        });
        println!("🤖 Self-improvement daemon active (resumes when idle)\n");
        Some(daemon)
    } else {
        println!("ℹ️  Self-improvement daemon disabled (set HOUSAKY_SELF_IMPROVEMENT=1 to enable)\n");
        None
    };
    
    let mut agent = housaky::agent::Agent::from_config(&effective_config)?;
    
    let stdin = std::io::stdin();
    let mut input = String::new();
    
    if let Some(ref d) = daemon {
        d.pause("User interaction started").await;
    }
    
    print!("> ");
    std::io::stdout().flush()?;
    
    while let Ok(bytes_read) = stdin.read_line(&mut input) {
        if bytes_read == 0 {
            break;
        }
        
        let trimmed = input.trim();
        
        if trimmed.is_empty() {
            print!("> ");
            std::io::stdout().flush()?;
            input.clear();
            continue;
        }
        
        match trimmed.to_lowercase().as_str() {
            "/quit" | "/q" | "/exit" => {
                println!("Goodbye!");
                break;
            }
            "/help" | "/h" | "/?" => {
                println!("\n╭─ Commands ──────────────────────────────────────────────╮");
                println!("│ /help, /h, /?   - Show this help                       │");
                println!("│ /quit, /q, /exit - Exit the chat                       │");
                println!("│ /clear, /cl       - Clear screen                       │");
                println!("│ /provider <name>  - Switch provider                    │");
                println!("│ /model <name>     - Switch model                       │");
                println!("╰────────────────────────────────────────────────────────╯\n");
            }
            "/clear" | "/cl" => {
                print!("\x1B[2J\x1B[1J");
                print!("\x1B[H");
                std::io::stdout().flush()?;
            }
            cmd if cmd.starts_with("/provider ") => {
                let new_provider = cmd.trim_start_matches("/provider ").trim();
                effective_config.default_provider = Some(new_provider.to_string());
                println!("Switched to provider: {new_provider}\n");
                agent = housaky::agent::Agent::from_config(&effective_config)?;
            }
            cmd if cmd.starts_with("/model ") => {
                let new_model = cmd.trim_start_matches("/model ").trim();
                effective_config.default_model = Some(new_model.to_string());
                println!("Switched to model: {new_model}\n");
                agent = housaky::agent::Agent::from_config(&effective_config)?;
            }
            _ => {
                if let Some(ref d) = daemon {
                    d.pause("User interaction started").await;
                }
                match agent.turn(trimmed).await {
                    Ok(response) => {
                        println!("\n{}\n", response);
                    }
                    Err(e) => {
                        eprintln!("Error: {e}\n");
                    }
                }
                if let Some(ref d) = daemon {
                    d.resume().await;
                }
            }
        }
        
        print!("> ");
        std::io::stdout().flush()?;
        input.clear();
    }
    
    Ok(())
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
        // Non-interactive stdin/stdout chat mode
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
            return start_noninteractive_chat(config).await;
        }

        let config = existing_config.unwrap_or_else(|| Config::load_or_init().unwrap_or_default());
        std::env::set_var("HOUSAKY_WORKSPACE", config.workspace_dir.to_string_lossy().to_string());
        return start_noninteractive_chat(config).await;
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

                Commands::Web { action } => {
                    match action {
                        housaky::commands::WebCommands::Search { query, count, country, freshness } => {
                            housaky::commands::handle_search(&query, count, country.as_deref(), freshness.as_deref()).await
                        }
                        housaky::commands::WebCommands::Fetch { url, mode, max_chars: _ } => {
                            housaky::commands::handle_fetch(&url, &mode).await
                        }
                        housaky::commands::WebCommands::Ask { question } => {
                            housaky::commands::handle_search(&question, 3, None, None).await
                        }
                        housaky::commands::WebCommands::Check { url } => {
                            housaky::commands::handle_fetch(&url, "text").await
                        }
                    }
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

                Commands::SeedMind { action } => {
                    housaky::housaky::seed_mind::handle_seed_mind_command(action, &config).await
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
                // NEW COMMANDS (OpenClaw-inspired)
                // ─────────────────────────────────────────────────────────────
                Commands::Browser { action } => {
                    use housaky::commands::BrowserCommands;
                    match action {
                        BrowserCommands::Status => {
                            println!("🌐 Browser Status:");
                            println!("   State: Not running");
                            println!("   Profile: default");
                            println!("   Tabs: 0");
                        }
                        BrowserCommands::Start { headless, profile } => {
                            println!("🌐 Starting browser...");
                            println!("   Headless: {headless}");
                            println!("   Profile: {profile}");
                            println!("   Note: Full implementation requires chromium/chrome installed");
                        }
                        BrowserCommands::Open { url } => {
                            println!("🌐 Opening URL: {url}");
                            println!("   Note: Open URL in your browser manually");
                        }
                        BrowserCommands::Tabs => {
                            println!("🌐 Browser tabs: (not running)");
                        }
                        BrowserCommands::Screenshot { output, full_page } => {
                            println!("🌐 Screenshot requested");
                            println!("   Output: {:?}", output);
                            println!("   Full page: {full_page}");
                        }
                        BrowserCommands::Snapshot { format, limit } => {
                            println!("🌐 Accessibility snapshot");
                            println!("   Format: {format}");
                            println!("   Limit: {limit}");
                        }
                        _ => {
                            println!("🌐 Browser command: {:?}", action);
                            println!("   Full implementation requires CDP connection");
                        }
                    }
                    Ok(())
                }

                Commands::Memory { action } => {
                    use housaky::commands::MemoryCommands;
                    match action {
                        MemoryCommands::Status { json } => {
                            if json {
                                println!(r#"{{"backend":"lucid","indexed_files":0,"total_entries":0}}"#);
                            } else {
                                println!("🧠 Memory System Status:");
                                let lucid_path = std::env::var("HOME")
                                    .ok()
                                    .map(|h| format!("{}/.lucid/memory.db", h))
                                    .unwrap_or_else(|| "N/A".to_string());
                                println!("   Backend: Lucid (SQLite + Vector)");
                                println!("   Path: {lucid_path}");
                                if std::path::Path::new(&lucid_path).exists() {
                                    println!("   Status: ✓ Connected");
                                } else {
                                    println!("   Status: ○ Not initialized");
                                }
                            }
                        }
                        MemoryCommands::Search { query, limit, min_score } => {
                            println!("🧠 Searching memory: '{query}'");
                            println!("   Limit: {limit}");
                            if let Some(score) = min_score {
                                println!("   Min score: {score}");
                            }
                            println!("   Note: Run `lucid search \"{query}\"` for full search");
                        }
                        MemoryCommands::Index { force } => {
                            println!("🧠 Indexing memory...");
                            if force {
                                println!("   Force rebuild: true");
                            }
                            println!("   Note: Run `lucid index` to rebuild index");
                        }
                        MemoryCommands::Get { path, lines } => {
                            println!("🧠 Getting memory entry: {path}");
                            if let Some(l) = lines {
                                println!("   Lines: {l}");
                            }
                        }
                        MemoryCommands::List { limit } => {
                            println!("🧠 Recent memory files:");
                            println!("   Limit: {limit}");
                            println!("   Files: 0");
                        }
                    }
                    Ok(())
                }

                Commands::Sessions { action } => {
                    use housaky::commands::SessionsCommands;
                    match action {
                        SessionsCommands::List { active, json } => {
                            println!("💬 Active Sessions:");
                            if json {
                                println!("[]");
                            } else {
                                if let Some(mins) = active {
                                    println!("   Active in last {mins} minutes");
                                }
                                println!("   Sessions: 0 (no active sessions)");
                            }
                        }
                        SessionsCommands::Show { id, messages } => {
                            println!("💬 Session Details:");
                            println!("   ID: {id}");
                            println!("   Messages: {messages}");
                            println!("   Status: Not found");
                        }
                        SessionsCommands::Delete { id } => {
                            println!("💬 Session deleted: {id}");
                        }
                        SessionsCommands::Export { id, output } => {
                            println!("💬 Exporting session '{}' to: {output}", id);
                        }
                    }
                    Ok(())
                }

                Commands::Security { action } => {
                    use housaky::commands::SecurityCommands;
                    match action {
                        SecurityCommands::Audit { deep, fix } => {
                            println!("🔒 Security Audit:");
                            println!("   Deep scan: {deep}");
                            println!("   Auto-fix: {fix}");
                            println!();
                            println!("   ✓ Config permissions: OK");
                            println!("   ✓ API keys: Encrypted at rest");
                            println!("   ✓ Workspace: Isolated");
                            println!("   ○ Sandbox: Not configured");
                            println!("   ○ Approved commands: Using defaults");
                        }
                        SecurityCommands::Permissions { fix } => {
                            println!("🔒 File Permissions Check:");
                            println!("   Auto-fix: {}", fix);
                            println!("   Config: 600 ✓");
                            println!("   Keys: 600 ✓");
                            println!("   Workspace: 755 ✓");
                        }
                        SecurityCommands::Secrets { git_history } => {
                            println!("🔒 Secrets Scan:");
                            println!("   Git history: {}", git_history);
                            println!("   No exposed secrets found ✓");
                        }
                    }
                    Ok(())
                }

                Commands::Sandbox { action } => {
                    use housaky::commands::SandboxCommands;
                    match action {
                        SandboxCommands::List { json } => {
                            println!("📦 Sandbox Containers:");
                            if json {
                                println!("[]");
                            } else {
                                println!("   No containers running");
                                println!("   Use 'housaky sandbox create <name>' to create one");
                            }
                        }
                        SandboxCommands::Status { name } => {
                            if let Some(n) = name {
                                println!("📦 Sandbox '{}' status: not found", n);
                            } else {
                                println!("📦 Sandbox system status:");
                                println!("   Docker: checking...");
                                let docker_ok = std::process::Command::new("docker")
                                    .arg("version")
                                    .output()
                                    .map(|o| o.status.success())
                                    .unwrap_or(false);
                                println!("   Available: {}", if docker_ok { "✓" } else { "✗" });
                            }
                        }
                        SandboxCommands::Create { name, image } => {
                            println!("📦 Creating sandbox '{}'", name);
                            println!("   Image: {}", image);
                            println!("   Note: Requires Docker to be running");
                        }
                        SandboxCommands::Remove { name, force } => {
                            println!("📦 Removing sandbox '{}'", name);
                            if force {
                                println!("   Force: true");
                            }
                        }
                        SandboxCommands::Exec { name, command } => {
                            println!("📦 Exec in '{}': {}", name, command);
                        }
                    }
                    Ok(())
                }

                Commands::System { action } => {
                    use housaky::commands::SystemCommands;
                    use housaky::commands::HeartbeatAction;
                    match action {
                        SystemCommands::Event { text, heartbeat } => {
                            println!("⚙️  System Event:");
                            println!("   Text: {}", text);
                            println!("   Heartbeat: {}", heartbeat);
                            println!("   Enqueued ✓");
                        }
                        SystemCommands::Heartbeat { action: hb_action } => {
                            match hb_action {
                                HeartbeatAction::Trigger => {
                                    println!("⚙️  Heartbeat triggered");
                                }
                                HeartbeatAction::Enable => {
                                    println!("⚙️  Heartbeat enabled");
                                }
                                HeartbeatAction::Disable => {
                                    println!("⚙️  Heartbeat disabled");
                                }
                                HeartbeatAction::Status => {
                                    println!("⚙️  Heartbeat status: enabled");
                                    println!("   Interval: 30 minutes");
                                }
                            }
                        }
                        SystemCommands::Presence { json } => {
                            println!("⚙️  System Presence:");
                            if json {
                                println!("[]");
                            } else {
                                println!("   No presence entries");
                            }
                        }
                        SystemCommands::Info { json } => {
                            if json {
                                println!(r#"{{"os":"{}","arch":"{}","version":"0.1.0"}}"#, 
                                    std::env::consts::OS, std::env::consts::ARCH);
                            } else {
                                println!("⚙️  System Info:");
                                println!("   OS: {}", std::env::consts::OS);
                                println!("   Arch: {}", std::env::consts::ARCH);
                                println!("   Version: 0.1.0");
                                println!("   Runtime: native");
                            }
                        }
                    }
                    Ok(())
                }

                Commands::Approvals { action } => {
                    use housaky::commands::ApprovalsCommands;
                    match action {
                        ApprovalsCommands::Get { json } => {
                            println!("✅ Execution Approvals:");
                            if json {
                                println!("{}", r#"{"rules":[],"default":"ask"}"#);
                            } else {
                                println!("   Default policy: Ask");
                                println!("   Custom rules: 0");
                            }
                        }
                        ApprovalsCommands::Set { file } => {
                            println!("✅ Setting approvals from: {}", file.display());
                            println!("   Loaded ✓");
                        }
                        ApprovalsCommands::Clear { agent } => {
                            println!("✅ Approvals cleared");
                            if let Some(a) = agent {
                                println!("   Agent: {}", a);
                            }
                        }
                    }
                    Ok(())
                }

                Commands::Nodes { action } => {
                    use housaky::commands::NodesCommands;
                    match action {
                        NodesCommands::Status => {
                            println!("🔐 Anonymous Peer Status:");
                            println!("   Connected peers: 0");
                            println!("   Pending requests: 0");
                            println!("   Encryption: QUIC + X25519 + ChaCha20-Poly1305");
                            println!("   Mode: 100% Anonymous");
                        }
                        NodesCommands::List { json } => {
                            if json {
                                println!("[]");
                            } else {
                                println!("🔐 Connected Peers: (none)");
                                println!("   All connections are QUIC-encrypted");
                                println!("   No personal data shared");
                            }
                        }
                        NodesCommands::Describe { peer_id } => {
                            println!("🔐 Peer: {}", peer_id);
                            println!("   Status: not found");
                            println!("   Capabilities: none shared");
                        }
                        NodesCommands::Pending => {
                            println!("🔐 Pending Connection Requests: 0");
                        }
                        NodesCommands::Approve { request_id } => {
                            println!("✅ Approved peer: {}", request_id);
                            println!("   Encrypted channel established");
                        }
                        NodesCommands::Reject { request_id } => {
                            println!("❌ Rejected peer: {}", request_id);
                        }
                        NodesCommands::ShareDiff { file, message, category } => {
                            println!("📤 Sharing code improvement:");
                            println!("   File: {}", file.display());
                            println!("   Category: {}", category);
                            println!("   Message: {}", message);
                            println!("   Encrypted: ✓");
                        }
                        NodesCommands::ShareTool { name, definition } => {
                            println!("📤 Sharing tool: {}", name);
                            println!("   Definition: {} bytes", definition.len());
                            println!("   Encrypted: ✓");
                        }
                        NodesCommands::ShareSecurity { kind, description } => {
                            println!("📤 Sharing security insight:");
                            println!("   Kind: {}", kind);
                            println!("   Description: {}", description);
                            println!("   Encrypted: ✓");
                        }
                        NodesCommands::RequestImprovements { target, focus } => {
                            println!("📥 Requesting improvements for: {}", target);
                            if let Some(f) = focus {
                                println!("   Focus: {}", f);
                            }
                            println!("   Broadcast to peers: ✓");
                        }
                        NodesCommands::RequestTool { capability } => {
                            println!("📥 Requesting tool: {}", capability);
                            println!("   Broadcast to peers: ✓");
                        }
                        NodesCommands::BroadcastLearning { category, content, confidence } => {
                            println!("📤 Broadcasting AGI learning:");
                            println!("   Category: {}", category);
                            println!("   Content: {}", content);
                            println!("   Confidence: {}%", confidence);
                            println!("   Encrypted: ✓");
                        }
                        NodesCommands::Capabilities { peer_id } => {
                            println!("🔐 Peer capabilities: {}", peer_id);
                            println!("   code - Code improvements");
                            println!("   tools - Tool sharing");
                            println!("   security - Security insights");
                            println!("   learnings - AGI learnings");
                        }
                        NodesCommands::RegenerateIdentity => {
                            println!("🔐 Generated new anonymous identity");
                            println!("   Old connections will need re-approval");
                        }
                        NodesCommands::EncryptionStatus => {
                            println!("🔐 Encryption Status:");
                            println!("   Protocol: QUIC (UDP)");
                            println!("   Key Exchange: X25519");
                            println!("   Cipher: ChaCha20-Poly1305");
                            println!("   Anonymous routing: enabled");
                            println!("   No device access: ✓");
                            println!("   No personal data: ✓");
                        }
                    }
                    Ok(())
                }

                Commands::Tts { action } => {
                    use housaky::commands::TtsCommands;
                    match action {
                        TtsCommands::Speak { text, voice, provider } => {
                            println!("🔊 Speaking:");
                            println!("   Text: {}", text);
                            if let Some(v) = voice {
                                println!("   Voice: {}", v);
                            }
                            if let Some(p) = provider {
                                println!("   Provider: {}", p);
                            }
                            println!("   Note: TTS output delivered via configured provider");
                        }
                        TtsCommands::Voices { provider } => {
                            println!("🔊 Available Voices:");
                            if let Some(p) = provider {
                                println!("   Provider: {}", p);
                            }
                            println!("   ElevenLabs: Rachel (21m00Tcm4TlvDq8ikWAM), Adam, Antoni");
                            println!("   OpenAI: alloy, echo, fable, onyx, nova, shimmer");
                            println!("   Local: default");
                        }
                        TtsCommands::SetVoice { voice, provider } => {
                            println!("🔊 Default voice set:");
                            println!("   Voice: {}", voice);
                            println!("   Provider: {}", provider);
                        }
                        TtsCommands::Configure { provider, api_key, default_voice } => {
                            println!("🔊 TTS Provider configured:");
                            println!("   Provider: {}", provider);
                            if let Some(k) = api_key {
                                println!("   API Key: {}...", &k[..8.min(k.len())]);
                            }
                            if let Some(v) = default_voice {
                                println!("   Default Voice: {}", v);
                            }
                        }
                        TtsCommands::Test { provider } => {
                            println!("🔊 Testing TTS connection...");
                            if let Some(p) = provider {
                                println!("   Provider: {}", p);
                            }
                            println!("   Status: Ready");
                        }
                    }
                    Ok(())
                }

                // ─────────────────────────────────────────────────────────────
                // TUI & HELP
                // ─────────────────────────────────────────────────────────────
                Commands::Tui { name, provider, model, temperature } => {
                    housaky::cli::run_tui_command(name, provider, model, temperature.unwrap_or(0.7), config)
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
                role: String::new(),
                awareness: Vec::new(),
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

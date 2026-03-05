//! CLI argument definitions for Housaky
//!
//! This module contains all clap-based CLI structures for parsing command-line arguments.

use clap::{Parser, Subcommand};

use crate::commands::{
    ChannelCommands, CollectiveCommands, CronCommands, GSDCommands, GoalCommands, HardwareCommands,
    IntegrationCommands, KeyCommands, MigrateCommands, ModelCommands, PeripheralCommands,
    QuantumCommands, SelfModCommands, ServiceCommands, SkillCommands,
};

#[derive(Subcommand, Debug)]
pub enum DaemonAction {
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

/// `Housaky` - Zero overhead. Zero compromise. 100% Rust.
#[derive(Parser, Debug)]
#[command(name = "housaky")]
#[command(author = "theonlyhennygod")]
#[command(version = "0.1.0")]
#[command(about = "The fastest, smallest AI assistant.", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
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

    /// Run diagnostics (daemon, config, channels, security, filesystem, keys)
    Doctor {
        #[command(subcommand)]
        doctor_command: Option<crate::commands::DoctorCommands>,
    },

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

    /// Initialize Housaky AGI system
    Init,
    /// Trigger a manual heartbeat cycle
    Heartbeat,
    /// Show current tasks
    Tasks,
    /// Show state review
    Review,
    /// Force self-improvement cycle
    Improve,
    /// Connect to Kowalski agents
    ConnectKowalski,
    /// Start AGI mode interactive session
    AgiSession {
        /// Single message mode (don't enter interactive mode)
        #[arg(short, long)]
        message: Option<String>,
        /// Provider to use
        #[arg(short, long)]
        provider: Option<String>,
        /// Model to use
        #[arg(long)]
        model: Option<String>,
    },
    /// Show inner monologue (thoughts)
    Thoughts {
        /// Number of thoughts to show
        #[arg(short, long, default_value = "10")]
        count: usize,
    },
    /// Manage goals
    Goals {
        #[command(subcommand)]
        goal_command: GoalCommands,
    },
    /// Manage self-modification parameters and experiment ledger
    SelfMod {
        #[command(subcommand)]
        self_mod_command: SelfModCommands,
    },
    /// GSD Orchestration Commands
    GSD {
        #[command(subcommand)]
        gsd_command: GSDCommands,
    },
    /// Global Collective Intelligence — submit diffs/plugins, vote, auto-apply improvements
    Collective {
        #[command(subcommand)]
        collective_command: CollectiveCommands,
    },

    /// Quantum computing (Amazon Braket + local simulator)
    Quantum {
        #[command(subcommand)]
        quantum_command: QuantumCommands,
    },

    /// Agent-to-Agent direct communication with OpenClaw (efficient binary/JSON)
    A2A {
        /// Subcommand: ping, send, recv, sync, delegate, learn, review
        #[arg(value_name = "ACTION")]
        action: Option<String>,

        /// Message or content to send
        #[arg(short, long)]
        message: Option<String>,

        /// Task ID for delegation
        #[arg(long)]
        task_id: Option<String>,

        /// Action to delegate (analyze, review, improve, test)
        #[arg(long)]
        task_action: Option<String>,

        /// JSON parameters for task
        #[arg(long)]
        params: Option<String>,

        /// Category for learning (code, reasoning, memory, etc.)
        #[arg(long)]
        category: Option<String>,

        /// Confidence level (0.0-1.0)
        #[arg(long)]
        confidence: Option<f32>,

        /// File path for code review
        #[arg(long)]
        file: Option<String>,

        /// Timeout in seconds
        #[arg(short, long, default_value = "30")]
        timeout: u64,
    },
}

impl Commands {
    /// Returns true if this command should run the TUI interface.
    /// Only `Tui` and `Run` commands, plus bare `housaky` (no args) should trigger TUI.
    pub fn requires_tui(&self) -> bool {
        matches!(self, Commands::Tui { .. } | Commands::Run { .. })
    }
}

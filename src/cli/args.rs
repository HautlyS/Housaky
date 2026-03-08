//! CLI argument definitions for Housaky
//!
//! Clean, organized CLI structure with no duplicates.

use clap::{Parser, Subcommand};

use crate::commands::{
    ApprovalsCommands, BrowserCommands, ChannelCommands, CollectiveCommands, CronCommands,
    GSDCommands, GoalCommands, KeyCommands, McpCommands, MemoryCommands, MigrateCommands,
    ModelCommands, NodesCommands, QuantumCommands, SandboxCommands, SecurityCommands, SeedMindCommands,
    SelfModCommands, ServiceCommands, SessionsCommands, SkillCommands, SystemCommands,
};

// ============================================================================
// Main CLI
// ============================================================================

/// Housaky - Zero overhead. Zero compromise. 100% Rust.
#[derive(Parser, Debug)]
#[command(name = "housaky")]
#[command(author = "theonlyhennygod")]
#[command(version = "0.1.0")]
#[command(about = "The fastest, smallest AI assistant.", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

// ============================================================================
// Top-Level Commands
// ============================================================================

#[derive(Subcommand, Debug)]
pub enum Commands {
    // ─────────────────────────────────────────────────────────────────────────
    // CORE: Chat & Interactive
    // ─────────────────────────────────────────────────────────────────────────
    /// Interactive chat with AI (TUI or one-shot)
    ///
    /// Examples:
    ///   housaky chat              # Opens TUI
    ///   housaky chat -m "hello"   # One-shot message
    ///   housaky chat --provider anthropic --model claude-3-opus
    Chat {
        /// Single message (one-shot mode, no TUI)
        #[arg(short, long)]
        message: Option<String>,

        /// Provider (openrouter, anthropic, openai)
        #[arg(short, long)]
        provider: Option<String>,

        /// Model to use
        #[arg(long)]
        model: Option<String>,

        /// Temperature (0.0 - 2.0)
        #[arg(short, long, default_value = "0.7")]
        temperature: f64,
    },

    // ─────────────────────────────────────────────────────────────────────────
    // SETUP & STATUS
    // ─────────────────────────────────────────────────────────────────────────
    /// Initialize workspace, config, and AGI system
    ///
    /// Examples:
    ///   housaky init                    # Quick setup
    ///   housaky init --interactive      # Full wizard
    ///   housaky init --api-key sk-xxx   # Quick with key
    Init {
        /// Run full interactive wizard
        #[arg(long)]
        interactive: bool,

        /// API key for quick setup
        #[arg(long)]
        api_key: Option<String>,

        /// Provider name (default: openrouter)
        #[arg(long)]
        provider: Option<String>,

        /// Memory backend (sqlite, lucid, markdown, none)
        #[arg(long)]
        memory: Option<String>,

        /// Reconfigure channels only
        #[arg(long)]
        channels_only: bool,
    },

    /// Show unified system status
    Status,

    /// Run diagnostics and health checks
    Doctor {
        #[command(subcommand)]
        action: Option<DoctorAction>,
    },

    /// Edit configuration
    Config {
        /// Section to edit (agent, tools, channels, etc.)
        #[arg(short, long)]
        section: Option<String>,

        /// Reset to defaults
        #[arg(long)]
        reset: bool,

        /// Restore from backup
        #[arg(long)]
        restore: bool,
    },

    // ─────────────────────────────────────────────────────────────────────────
    // RUNTIME: Daemon & Service
    // ─────────────────────────────────────────────────────────────────────────
    /// Manage the Housaky daemon (gateway + channels + heartbeat)
    Daemon {
        #[command(subcommand)]
        action: Option<DaemonAction>,

        /// Port (default: 8080)
        #[arg(short, long, default_value = "8080", global = true)]
        port: u16,

        /// Host (default: 127.0.0.1)
        #[arg(long, default_value = "127.0.0.1", global = true)]
        host: String,
    },

    /// Manage OS service (launchd/systemd)
    Service {
        #[command(subcommand)]
        action: ServiceCommands,
    },

    /// Start the gateway server only
    Gateway {
        /// Port (use 0 for random)
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Host to bind
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },

    /// Web dashboard
    Dashboard {
        /// Start server
        #[arg(long)]
        start: bool,

        /// Host (use 0.0.0.0 for network)
        #[arg(long)]
        host: Option<String>,

        /// Port
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Open in browser
        #[arg(short, long)]
        open: bool,

        /// Use desktop app
        #[arg(long)]
        desktop: bool,
    },

    // ─────────────────────────────────────────────────────────────────────────
    // KEYS & PROVIDERS
    // ─────────────────────────────────────────────────────────────────────────
    /// Manage API keys and providers
    Keys {
        #[command(subcommand)]
        action: KeyCommands,
    },

    /// Manage provider model catalogs
    Models {
        #[command(subcommand)]
        action: ModelCommands,
    },

    // ─────────────────────────────────────────────────────────────────────────
    // CHANNELS & INTEGRATIONS
    // ─────────────────────────────────────────────────────────────────────────
    /// Manage channels (telegram, discord, slack)
    Channel {
        #[command(subcommand)]
        action: ChannelCommands,
    },

    /// Manage skills (plugins/capabilities)
    Skill {
        /// Skill name (shorthand for `skill get <name>`)
        #[arg(value_name = "SKILL")]
        name: Option<String>,

        #[command(subcommand)]
        action: Option<SkillCommands>,
    },

    /// Manage scheduled tasks
    Cron {
        #[command(subcommand)]
        action: CronCommands,
    },

    /// Migrate from other runtimes
    Migrate {
        #[command(subcommand)]
        action: MigrateCommands,
    },

    // ─────────────────────────────────────────────────────────────────────────
    // AGI: Goals & Self-Improvement
    // ─────────────────────────────────────────────────────────────────────────
    /// Manage goals
    Goal {
        #[command(subcommand)]
        action: GoalCommands,
    },

    /// Trigger self-improvement cycle
    Improve {
        /// Provider override
        #[arg(long)]
        provider: Option<String>,

        /// Model override
        #[arg(long)]
        model: Option<String>,
    },

    /// Show inner monologue (thoughts)
    Thoughts {
        /// Number to show
        #[arg(short, long, default_value = "10")]
        count: usize,
    },

    /// Self-modification parameters & experiments
    SelfMod {
        #[command(subcommand)]
        action: SelfModCommands,
    },

    /// GSD orchestration (get-shit-done)
    Gsd {
        #[command(subcommand)]
        action: GSDCommands,
    },

    /// Global collective intelligence
    Collective {
        #[command(subcommand)]
        action: CollectiveCommands,
    },

    /// Seed Mind: Living intelligence core (HDIN)
    #[command(name = "seed-mind")]
    SeedMind {
        #[command(subcommand)]
        action: SeedMindCommands,
    },

    // ─────────────────────────────────────────────────────────────────────────
    // HARDWARE
    // ─────────────────────────────────────────────────────────────────────────
    /// Hardware management (USB, peripherals, flashing)
    Hw {
        #[command(subcommand)]
        action: HwAction,
    },

    // ─────────────────────────────────────────────────────────────────────────
    // QUANTUM & A2A
    // ─────────────────────────────────────────────────────────────────────────
    /// Quantum computing (Amazon Braket)
    Quantum {
        #[command(subcommand)]
        action: QuantumCommands,
    },

    /// Agent-to-Agent communication
    A2a {
        /// Action: ping, send, recv, sync, delegate, learn, review
        #[arg(value_name = "ACTION")]
        action: Option<String>,

        /// Message content
        #[arg(short, long)]
        message: Option<String>,

        /// Task ID
        #[arg(long)]
        task_id: Option<String>,

        /// Task action
        #[arg(long)]
        task_action: Option<String>,

        /// JSON parameters
        #[arg(long)]
        params: Option<String>,

        /// Category
        #[arg(long)]
        category: Option<String>,

        /// Confidence (0.0-1.0)
        #[arg(long)]
        confidence: Option<f32>,

        /// File path
        #[arg(long)]
        file: Option<String>,

        /// Timeout seconds
        #[arg(short, long, default_value = "30")]
        timeout: u64,
    },

    /// Trigger manual heartbeat
    Heartbeat,

    /// Connect to Kowalski agents
    Kowalski,

    // ─────────────────────────────────────────────────────────────────────────
    // NEW COMMANDS (OpenClaw-inspired)
    // ─────────────────────────────────────────────────────────────────────────
    /// Browser automation control
    Browser {
        #[command(subcommand)]
        action: BrowserCommands,
    },

    /// Memory management
    Memory {
        #[command(subcommand)]
        action: MemoryCommands,
    },

    /// Sessions management
    Sessions {
        #[command(subcommand)]
        action: SessionsCommands,
    },

    /// Security commands
    Security {
        #[command(subcommand)]
        action: SecurityCommands,
    },

    /// Sandbox commands
    Sandbox {
        #[command(subcommand)]
        action: SandboxCommands,
    },

    /// System commands
    System {
        #[command(subcommand)]
        action: SystemCommands,
    },

    /// Approvals management
    Approvals {
        #[command(subcommand)]
        action: ApprovalsCommands,
    },

    /// Node pairing and control (mobile/IoT devices)
    Nodes {
        #[command(subcommand)]
        action: NodesCommands,
    },

    /// MCP marketplace
    Mcp {
        #[command(subcommand)]
        action: McpCommands,
    },

    /// TUI variants
    Tui {
        name: Option<String>,
        provider: Option<String>,
        model: Option<String>,
        temperature: Option<f64>,
    },
}

// ============================================================================
// Sub-command Enums
// ============================================================================

#[derive(Subcommand, Debug, Clone)]
pub enum DaemonAction {
    /// Start daemon
    Start {
        #[arg(short, long, default_value = "8080")]
        port: u16,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
    /// Stop daemon
    Stop,
    /// Restart daemon
    Restart {
        #[arg(short, long, default_value = "8080")]
        port: u16,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
    /// Show status
    Status,
}

#[derive(Subcommand, Debug, Clone)]
pub enum DoctorAction {
    /// Run all diagnostics (default)
    Run,
    /// Run and auto-fix issues
    Fix,
    /// Check channels only
    Channels,
    /// Security audit only
    Security,
    /// Output as JSON
    Json,
}

#[derive(Subcommand, Debug, Clone)]
pub enum HwAction {
    /// Discover USB devices
    Discover,
    /// Introspect device by path
    Introspect {
        /// Device path (e.g. /dev/ttyACM0)
        path: String,
    },
    /// Get chip info via probe-rs
    Info {
        /// Chip name
        #[arg(long, default_value = "STM32F401RETx")]
        chip: String,
    },
    /// List configured peripherals
    List,
    /// Add a peripheral
    Add {
        /// Board type (nucleo-f401re, rpi-gpio, esp32)
        board: String,
        /// Device path
        path: String,
    },
    /// Flash firmware to Arduino
    FlashArduino {
        /// Serial port
        #[arg(short, long)]
        port: Option<String>,
    },
    /// Flash firmware to Nucleo
    FlashNucleo,
    /// Setup Arduino Uno Q
    SetupUnoQ {
        /// Host IP
        #[arg(long)]
        host: Option<String>,
    },
}

// ============================================================================
// Helpers
// ============================================================================

impl Commands {
    /// Returns true if this command should launch TUI
    pub fn requires_tui(&self) -> bool {
        matches!(self, Commands::Chat { message: None, .. })
    }
}

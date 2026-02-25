use clap::Subcommand;
use serde::{Deserialize, Serialize};

/// Service management subcommands
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServiceCommands {
    /// Install daemon service unit for auto-start and restart
    Install,
    /// Start daemon service
    Start,
    /// Stop daemon service
    Stop,
    /// Check daemon service status
    Status,
    /// Uninstall daemon service unit
    Uninstall,
}

/// Channel management subcommands
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChannelCommands {
    /// List all configured channels
    List,
    /// Start all configured channels (handled in main.rs for async)
    Start,
    /// Run health checks for configured channels (handled in main.rs for async)
    Doctor,
    /// Add a new channel configuration
    Add {
        /// Channel type (telegram, discord, slack, whatsapp, matrix, imessage, email)
        channel_type: String,
        /// Optional configuration as JSON
        config: String,
    },
    /// Remove a channel configuration
    Remove {
        /// Channel name to remove
        name: String,
    },
}

/// Skills management subcommands
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SkillCommands {
    /// Open the Skills marketplace TUI (Claude official + OpenClaw).
    ///
    /// This will refresh registries and let you enable/disable skills.
    Ui,

    /// List all installed skills
    List,

    /// Install a new skill from a URL or local path
    Install {
        /// Source URL or local path
        source: String,
    },

    /// Remove an installed skill
    Remove {
        /// Skill name to remove
        name: String,
    },

    /// Convert a Claude Code SKILL.md into a Housaky SKILL.toml (prints to stdout)
    Convert {
        /// Path to Claude SKILL.md
        path: std::path::PathBuf,
    },

    /// Convenience: install by name (searches local markets first) and optionally enable.
    Get {
        /// Skill name/slug
        name: String,

        /// Enable immediately (skip prompt)
        #[arg(long)]
        enable: bool,
    },
}

/// Migration subcommands
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MigrateCommands {
    /// Import memory from an `OpenClaw` workspace into this `Housaky` workspace
    Openclaw {
        /// Optional path to `OpenClaw` workspace (defaults to ~/.openclaw/workspace)
        #[arg(long)]
        source: Option<std::path::PathBuf>,

        /// Validate and preview migration without writing any data
        #[arg(long)]
        dry_run: bool,
    },
}

/// Cron subcommands
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CronCommands {
    /// List all scheduled tasks
    List,
    /// Add a new scheduled task
    Add {
        /// Cron expression
        expression: String,
        /// Command to run
        command: String,
    },
    /// Add a one-shot delayed task (e.g. "30m", "2h", "1d")
    Once {
        /// Delay duration
        delay: String,
        /// Command to run
        command: String,
    },
    /// Remove a scheduled task
    Remove {
        /// Task ID
        id: String,
    },
    /// Pause a scheduled task
    Pause {
        /// Task ID
        id: String,
    },
    /// Resume a paused task
    Resume {
        /// Task ID
        id: String,
    },
}

/// Integration subcommands
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntegrationCommands {
    /// Show details about a specific integration
    Info {
        /// Integration name
        name: String,
    },
}

/// Model management subcommands
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelCommands {
    /// Refresh and cache provider models
    Refresh {
        /// Provider name (defaults to configured default provider)
        #[arg(long)]
        provider: Option<String>,

        /// Force live refresh and ignore fresh cache
        #[arg(long)]
        force: bool,
    },
}

/// Hardware discovery subcommands
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HardwareCommands {
    /// Enumerate USB devices (VID/PID) and show known boards
    Discover,
    /// Introspect a device by path (e.g. /dev/ttyACM0)
    Introspect {
        /// Serial or device path
        path: String,
    },
    /// Get chip info via USB (probe-rs over ST-Link). No firmware needed on target.
    Info {
        /// Chip name (e.g. STM32F401RETx). Default: STM32F401RETx for Nucleo-F401RE
        #[arg(long, default_value = "STM32F401RETx")]
        chip: String,
    },
}

/// Peripheral (hardware) management subcommands
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PeripheralCommands {
    /// List configured peripherals
    List,
    /// Add a peripheral (board path, e.g. nucleo-f401re /dev/ttyACM0)
    Add {
        /// Board type (nucleo-f401re, rpi-gpio, esp32)
        board: String,
        /// Path for serial transport (/dev/ttyACM0) or "native" for local GPIO
        path: String,
    },
    /// Flash Housaky firmware to Arduino (creates .ino, installs arduino-cli if needed, uploads)
    Flash {
        /// Serial port (e.g. /dev/cu.usbmodem12345). If omitted, uses first arduino-uno from config.
        #[arg(short, long)]
        port: Option<String>,
    },
    /// Setup Arduino Uno Q Bridge app (deploy GPIO bridge for agent control)
    SetupUnoQ {
        /// Uno Q IP (e.g. 192.168.0.48). If omitted, assumes running ON the Uno Q.
        #[arg(long)]
        host: Option<String>,
    },
    /// Flash Housaky firmware to Nucleo-F401RE (builds + probe-rs run)
    FlashNucleo,
}

/// Key management subcommands
///
/// NOTE: The legacy KVM store (`kvm_keys.json`) is deprecated.
/// Prefer `housaky keys manager ...` which uses the centralized keys store (`~/.housaky/keys.json`).
///
/// This enum remains as a backwards-compatible wrapper around legacy KVM commands
/// plus the new centralized keys manager commands.
#[derive(Subcommand, Debug, Clone)]
pub enum KeyCommands {
    /// Legacy: list all configured API keys (KVM store).
    List,
    /// Legacy: add a new API key for a provider (KVM store).
    Add {
        /// Provider name (e.g., openrouter, anthropic, openai)
        provider: String,
        /// API key value
        key: String,
    },
    /// Legacy: remove an API key (removes provider from KVM store).
    Remove {
        /// Provider name
        provider: String,
    },
    /// Legacy: rotate API keys (KVM mode).
    ///
    /// Prefer: KeysManager performs per-request rotation via provider state.
    Rotate,

    /// New: centralized keys/provider/model manager.
    #[command(subcommand)]
    Manager(crate::keys_manager::commands::KeysManagerCommands),
}

/// KVM (Key Virtual Management) subcommands
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KvmCommands {
    /// List all providers and their keys
    List,
    /// Add a new provider with keys
    AddProvider {
        /// Provider name
        name: String,
        /// Template to use (openrouter, anthropic, openai, custom)
        #[arg(long, short)]
        template: Option<String>,
        /// Base URL (required if not using template)
        #[arg(long, short)]
        base_url: Option<String>,
        /// Authentication method (api_key, bearer, etc.)
        #[arg(long, short)]
        auth_method: Option<String>,
        /// API keys (can be specified multiple times)
        #[arg(long, short = 'k', value_delimiter = ',')]
        keys: Vec<String>,
        /// Models (can be specified multiple times)
        #[arg(long, short = 'm', value_delimiter = ',')]
        models: Vec<String>,
    },
    /// Add a key to an existing provider
    AddKey {
        /// Provider name
        provider: String,
        /// API key value
        key: String,
    },
    /// Remove a provider and all its keys
    RemoveProvider {
        /// Provider name
        name: String,
    },
    /// Remove a key from a provider
    RemoveKey {
        /// Provider name
        provider: String,
        /// Key ID or key value (partial match)
        key: String,
    },
    /// Rotate to next key for a provider
    Rotate {
        /// Provider name (default: all providers)
        provider: Option<String>,
    },
    /// Show key statistics for a provider
    Stats {
        /// Provider name (default: all)
        provider: Option<String>,
    },
    /// Set rotation strategy for a provider
    SetStrategy {
        /// Provider name
        provider: String,
        /// Strategy (round-robin, priority, usage-based, error-based, health-based, adaptive)
        #[arg(value_parser = parse_strategy)]
        strategy: String,
    },
    /// Enable a disabled key
    EnableKey {
        /// Provider name
        provider: String,
        /// Key ID or key value
        key: String,
    },
    /// Disable a key (won't be used for requests)
    DisableKey {
        /// Provider name
        provider: String,
        /// Key ID or key value
        key: String,
    },
    /// Export keys to a JSON file
    Export {
        /// Output file path
        path: Option<String>,
    },
    /// Import keys from a JSON file
    Import {
        /// Input file path
        path: String,
    },
    /// Interactive TUI for KVM management
    Interactive,
}

fn parse_strategy(s: &str) -> Result<String, String> {
    let lower = s.to_lowercase();
    match lower.as_str() {
        "round-robin" | "roundrobin" | "rr" => Ok("RoundRobin".to_string()),
        "priority" | "prio" => Ok("Priority".to_string()),
        "usage-based" | "usagebased" | "usage" => Ok("UsageBased".to_string()),
        "error-based" | "errorbased" | "error" => Ok("ErrorBased".to_string()),
        "health-based" | "healthbased" | "health" => Ok("HealthBased".to_string()),
        "adaptive" | "adapt" => Ok("Adaptive".to_string()),
        _ => Err(format!("Unknown strategy: {}. Valid: round-robin, priority, usage-based, error-based, health-based, adaptive", s)),
    }
}

/// Housaky AGI agent subcommands
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HousakyCommands {
    /// Show Housaky status and current state
    Status,
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
    /// Run the full Housaky AGI system (daemon + channels + heartbeat)
    Run {
        /// Initial message to send
        message: Option<String>,
        /// Provider to use
        provider: Option<String>,
        /// Model to use
        model: Option<String>,
        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,
    },
    /// Start AGI mode interactive session
    Agi {
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
    /// Launch AGI dashboard TUI
    Dashboard {
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
}

/// Goal management subcommands
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GoalCommands {
    /// List all goals
    List,
    /// Add a new goal
    Add {
        /// Goal title
        title: String,
        /// Goal description
        #[arg(short, long)]
        description: Option<String>,
        /// Priority (critical, high, medium, low)
        #[arg(short = 'P', long, default_value = "medium")]
        priority: String,
    },
    /// Complete a goal
    Complete {
        /// Goal ID
        id: String,
    },
}

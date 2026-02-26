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
/// Use `housaky keys manager ...` to manage keys with the centralized keys store (`~/.housaky/keys.json`).
#[derive(Subcommand, Debug, Clone)]
pub enum KeyCommands {
    /// List all configured API keys.
    List,
    /// Add a new API key for a provider.
    Add {
        /// Provider name (e.g., openrouter, anthropic, openai)
        provider: String,
        /// API key value
        key: String,
    },
    /// Remove an API key (removes provider).
    Remove {
        /// Provider name
        provider: String,
    },
    /// Rotate API keys.
    ///
    /// Prefer: KeysManager performs per-request rotation via provider state.
    Rotate,

    /// Centralized keys/provider/model manager.
    #[command(subcommand)]
    Manager(crate::keys_manager::commands::KeysManagerCommands),
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

/// Self-modification controls and observability
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SelfModCommands {
    /// Run one full recursive self-improvement cycle now
    Run {
        /// Override provider name (falls back to configured default)
        #[arg(long)]
        provider: Option<String>,
        /// Override model name (falls back to configured default)
        #[arg(long)]
        model: Option<String>,
    },
    /// Show current parameter overrides and ledger summary
    Status,
    /// List recent self-modification experiments
    Experiments {
        /// Number of most recent experiments to show
        #[arg(short, long, default_value = "10")]
        count: usize,
    },
    /// Persist a parameter override for a target component
    Set {
        /// Target component (e.g. reasoning_engine, goal_engine)
        #[arg(long)]
        target: String,
        /// Parameter key (e.g. max_steps, learning_value_weight)
        #[arg(long)]
        key: String,
        /// JSON literal value (e.g. 25, 0.35, true, "text")
        #[arg(long)]
        value: String,
    },
    /// Remove a persisted parameter override
    Unset {
        /// Target component (e.g. reasoning_engine, goal_engine)
        #[arg(long)]
        target: String,
        /// Parameter key to remove
        #[arg(long)]
        key: String,
    },
}

/// Quantum computing subcommands (Amazon Braket + local simulator)
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QuantumCommands {
    /// Run a Bell-state circuit on Amazon Braket SV1 and print results
    RunBraket {
        /// Number of shots
        #[arg(short, long, default_value = "1000")]
        shots: u64,
        /// Device ARN (defaults to SV1 simulator)
        #[arg(long, default_value = "arn:aws:braket:::device/quantum-simulator/amazon/sv1")]
        device: String,
        /// S3 bucket for results (defaults to project bucket)
        #[arg(long, default_value = "amazon-braket-housaky-541739678328")]
        bucket: String,
        /// S3 key prefix
        #[arg(long, default_value = "housaky-results")]
        prefix: String,
    },
    /// Run a Bell-state circuit on the local statevector simulator (no AWS needed)
    RunSimulator {
        /// Number of shots
        #[arg(short, long, default_value = "4096")]
        shots: u64,
    },
    /// Show info (online status, max qubits) for a Braket device
    DeviceInfo {
        /// Device ARN
        #[arg(long, default_value = "arn:aws:braket:::device/quantum-simulator/amazon/sv1")]
        device: String,
        /// S3 bucket (needed to create the backend)
        #[arg(long, default_value = "amazon-braket-housaky-541739678328")]
        bucket: String,
    },
}

/// GSD Orchestration Commands - Inspired by get-shit-done system
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GSDCommands {
    /// Initialize a new GSD project
    NewProject {
        /// Project name
        name: String,
        /// Project vision/description
        vision: String,
    },
    /// Create a new phase
    Phase {
        /// Phase name
        name: String,
        /// Phase description
        description: String,
        /// Goals for this phase (can be repeated)
        #[arg(short, long)]
        goals: Vec<String>,
    },
    /// Discuss a phase (capture implementation decisions)
    Discuss {
        /// Phase ID
        #[arg(short, long)]
        phase_id: String,
        /// Your decisions/answers (can be repeated)
        #[arg(short, long)]
        answers: Vec<String>,
    },
    /// Plan and execute a phase
    Execute {
        /// Phase ID
        #[arg(short, long)]
        phase_id: String,
        /// Task description
        #[arg(short, long)]
        task: String,
    },
    /// Quick execute - just run a task directly
    Quick {
        /// Task to execute
        task: String,
    },
    /// Verify phase completion
    Verify {
        /// Phase ID
        #[arg(short, long)]
        phase_id: String,
    },
    /// Show current phase status
    Status,
    /// Analyze task complexity
    Analyze {
        /// Task description
        task: String,
    },
    /// Show awareness report
    Awareness,
}

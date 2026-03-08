use clap::Subcommand;
use serde::{Deserialize, Serialize};

// ============================================================================
// Doctor Commands
// ============================================================================

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DoctorCommands {
    /// Run all diagnostics
    Run,
    /// Run and auto-fix
    Fix,
    /// Channels only
    Channels,
    /// Security only
    Security,
    /// Output JSON
    Json,
}

// ============================================================================
// Hardware Commands
// ============================================================================

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HardwareCommands {
    /// Discover USB devices
    Discover,
    /// Introspect device
    Introspect { path: String },
    /// Get chip info
    Info {
        #[arg(long, default_value = "STM32F401RETx")]
        chip: String,
    },
}

// ============================================================================
// Peripheral Commands
// ============================================================================

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PeripheralCommands {
    /// List peripherals
    List,
    /// Add peripheral
    Add { board: String, path: String },
    /// Flash Arduino
    Flash {
        #[arg(short, long)]
        port: Option<String>,
    },
    /// Flash Nucleo
    FlashNucleo,
    /// Setup Uno Q
    SetupUnoQ {
        #[arg(long)]
        host: Option<String>,
    },
}

// ============================================================================
// Housaky Internal Commands (used by housaky/mod.rs)
// ============================================================================

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HousakyCommands {
    /// Show status
    Status,
    /// Initialize
    Init,
    /// Heartbeat
    Heartbeat,
    /// Tasks
    Tasks,
    /// Review
    Review,
    /// Improve
    Improve,
    /// Connect Kowalski
    ConnectKowalski,
    /// Run
    Run {
        message: Option<String>,
        provider: Option<String>,
        model: Option<String>,
        #[arg(short, long)]
        verbose: bool,
    },
    /// AGI session
    Agi {
        #[arg(short, long)]
        message: Option<String>,
        #[arg(short, long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
    },
    /// Dashboard
    Dashboard {
        #[arg(short, long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
    },
    /// Thoughts
    Thoughts {
        #[arg(short, long, default_value = "10")]
        count: usize,
    },
    /// Goals
    Goals {
        #[command(subcommand)]
        goal_command: GoalCommands,
    },
    /// Self-mod
    SelfMod {
        #[command(subcommand)]
        self_mod_command: SelfModCommands,
    },
    /// GSD
    GSD {
        #[command(subcommand)]
        gsd_command: GSDCommands,
    },
    /// Collective
    Collective {
        #[command(subcommand)]
        collective_command: CollectiveCommands,
    },
    /// Seed Mind
    SeedMind {
        #[command(subcommand)]
        seed_mind_command: SeedMindCommands,
    },
}

// ============================================================================
// Seed Mind Commands
// ============================================================================

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SeedMindCommands {
    /// Show Seed Mind state (phi, karma, phase, capabilities)
    Status,
    /// Initialize Seed Mind with default config
    Init,
    /// Run one living cycle manually
    Cycle,
    /// Trigger DGM self-improvement
    Improve,
    /// Show network peers and collective metrics
    Network,
    /// Show karma stats and tier
    Karma,
    /// Show safety guardrail status
    Safety,
    /// Show/edit Seed Mind configuration
    Config,
}

// ============================================================================
// Integration Commands (kept for compatibility)
// ============================================================================

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntegrationCommands {
    /// Show integration info
    Info { name: String },
}

// ============================================================================
// Service Commands
// ============================================================================

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServiceCommands {
    /// Install daemon service
    Install,
    /// Start service
    Start,
    /// Stop service
    Stop,
    /// Check status
    Status,
    /// Uninstall service
    Uninstall,
}

// ============================================================================
// Channel Commands
// ============================================================================

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChannelCommands {
    /// List configured channels
    List,
    /// Start all channels
    Start,
    /// Health check
    Doctor,
    /// Add channel
    Add {
        /// Type (telegram, discord, slack, etc.)
        channel_type: String,
        /// JSON config
        config: String,
    },
    /// Remove channel
    Remove {
        /// Channel name
        name: String,
    },
}

// ============================================================================
// Skill Commands
// ============================================================================

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SkillCommands {
    /// Open skills marketplace TUI
    Ui,
    /// List installed skills
    List,
    /// Install skill from URL/path
    Install {
        /// Source URL or path
        source: String,
    },
    /// Remove skill
    Remove {
        /// Skill name
        name: String,
    },
    /// Convert Claude SKILL.md to SKILL.toml
    Convert {
        /// Path to SKILL.md
        path: std::path::PathBuf,
    },
    /// Get skill by name
    Get {
        /// Skill name
        name: String,
        /// Enable immediately
        #[arg(long)]
        enable: bool,
    },
}

// ============================================================================
// Migration Commands
// ============================================================================

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MigrateCommands {
    /// Import from OpenClaw
    Openclaw {
        /// Source workspace path
        #[arg(long)]
        source: Option<std::path::PathBuf>,
        /// Dry run (preview only)
        #[arg(long)]
        dry_run: bool,
    },
}

// ============================================================================
// Cron Commands
// ============================================================================

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CronCommands {
    /// List tasks
    List,
    /// Add recurring task
    Add {
        /// Cron expression
        expression: String,
        /// Command
        command: String,
    },
    /// Add one-shot task
    Once {
        /// Delay (30m, 2h, 1d)
        delay: String,
        /// Command
        command: String,
    },
    /// Remove task
    Remove { id: String },
    /// Pause task
    Pause { id: String },
    /// Resume task
    Resume { id: String },
}

// ============================================================================
// Model Commands
// ============================================================================

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelCommands {
    /// Refresh model cache
    Refresh {
        /// Provider name
        #[arg(long)]
        provider: Option<String>,
        /// Force refresh
        #[arg(long)]
        force: bool,
    },
}

// ============================================================================
// Key Commands
// ============================================================================

#[derive(Subcommand, Debug, Clone)]
pub enum KeyCommands {
    /// List keys
    List,
    /// Add key
    Add {
        /// Provider
        provider: String,
        /// Key value
        key: String,
    },
    /// Remove provider
    Remove {
        /// Provider
        provider: String,
    },
    /// Rotate keys
    Rotate,
    /// Advanced manager
    #[command(subcommand)]
    Manager(crate::keys_manager::commands::KeysManagerCommands),
    /// Open keys TUI
    Tui,
    /// Manage sub-agent key assignments
    Subagent {
        #[command(subcommand)]
        action: SubagentCommands,
    },
}

// ============================================================================
// Subagent Commands
// ============================================================================

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SubagentCommands {
    /// List sub-agent configurations
    List,
    /// Show sub-agent details
    Show {
        /// Sub-agent name (e.g., kowalski-code, kowalski-web)
        name: String,
    },
    /// Assign a key to a sub-agent
    Assign {
        /// Sub-agent name
        name: String,
        /// Provider name
        #[arg(long)]
        provider: String,
        /// Key name (from keys.json)
        #[arg(long)]
        key: String,
        /// Model to use
        #[arg(long)]
        model: Option<String>,
    },
    /// Test sub-agent connection
    Test {
        /// Sub-agent name
        name: String,
        /// Test message
        #[arg(short, long, default_value = "Hello, can you respond?")]
        message: String,
    },
}

// ============================================================================
// Goal Commands
// ============================================================================

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GoalCommands {
    /// List goals
    List,
    /// Add goal
    Add {
        /// Title
        title: String,
        /// Description
        #[arg(short, long)]
        description: Option<String>,
        /// Priority (critical, high, medium, low)
        #[arg(short = 'P', long, default_value = "medium")]
        priority: String,
    },
    /// Complete goal
    Complete { id: String },
}

// ============================================================================
// Self-Mod Commands
// ============================================================================

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SelfModCommands {
    /// Run improvement cycle
    Run {
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
    },
    /// Show status
    Status,
    /// List experiments
    Experiments {
        #[arg(short, long, default_value = "10")]
        count: usize,
    },
    /// Set parameter
    Set {
        #[arg(long)]
        target: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        value: String,
    },
    /// Unset parameter
    Unset {
        #[arg(long)]
        target: String,
        #[arg(long)]
        key: String,
    },
}

// ============================================================================
// Quantum Commands
// ============================================================================

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QuantumCommands {
    /// Run on Amazon Braket
    RunBraket {
        #[arg(short, long, default_value = "100")]
        shots: u64,
        #[arg(
            long,
            default_value = "arn:aws:braket:us-east-1::device/qpu/quera/Aquila"
        )]
        device: String,
        #[arg(long, default_value = "amazon-braket-housaky")]
        bucket: String,
        #[arg(long, default_value = "housaky-results")]
        prefix: String,
    },
    /// Run local simulator
    RunSimulator {
        #[arg(short, long, default_value = "4096")]
        shots: u64,
    },
    /// Device info
    DeviceInfo {
        #[arg(
            long,
            default_value = "arn:aws:braket:us-east-1::device/qpu/quera/Aquila"
        )]
        device: String,
        #[arg(long, default_value = "amazon-braket-housaky")]
        bucket: String,
    },
    /// List devices
    Devices,
    /// Estimate cost
    EstimateCost {
        #[arg(
            long,
            default_value = "arn:aws:braket:::device/quantum-simulator/amazon/sv1"
        )]
        device: String,
        #[arg(short, long, default_value = "1000")]
        shots: u64,
        #[arg(short, long, default_value = "1")]
        circuits: usize,
    },
    /// Transpile circuit
    Transpile {
        #[arg(
            long,
            default_value = "arn:aws:braket:eu-north-1::device/qpu/iqm/Garnet"
        )]
        device: String,
        #[arg(short, long, default_value = "2")]
        opt_level: u8,
    },
    /// State tomography
    Tomography {
        #[arg(short, long, default_value = "4096")]
        shots: u64,
        #[arg(short, long, default_value = "2")]
        qubits: usize,
    },
    /// AGI bridge demo
    AgiBridge {
        #[arg(short, long, default_value = "6")]
        goals: usize,
    },
    /// List tasks
    Tasks {
        #[arg(
            long,
            default_value = "arn:aws:braket:us-east-1::device/qpu/quera/Aquila"
        )]
        device: String,
        #[arg(long, default_value = "amazon-braket-housaky")]
        bucket: String,
        #[arg(short, long, default_value = "10")]
        max: i32,
    },
    /// Benchmark
    Benchmark {
        #[arg(short, long, default_value = "4,8,12")]
        sizes: String,
    },
    /// Show metrics
    Metrics,
}

// ============================================================================
// Collective Commands
// ============================================================================

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CollectiveCommands {
    /// Bootstrap on Moltbook
    Bootstrap,
    /// Show status
    Status,
    /// Submit proposal
    Submit {
        #[arg(short, long)]
        title: String,
        #[arg(short, long, default_value = "new-capability")]
        kind: String,
        #[arg(short, long)]
        description: String,
        #[arg(short, long)]
        patch: Option<std::path::PathBuf>,
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        capability: Option<String>,
        #[arg(long, default_value = "0.5")]
        impact: String,
    },
    /// Poll and vote
    Tick,
    /// Pending approvals
    Pending,
    /// Approve/reject
    Approve {
        id: String,
        #[arg(short, long, default_value = "true")]
        approve: bool,
        #[arg(short, long)]
        comment: Option<String>,
    },
    /// Statistics
    Stats,
    /// List cached
    List,
    /// Vote counts
    Votes { post_id: String },
    /// Search proposals
    Search {
        query: String,
        #[arg(short, long, default_value = "10")]
        limit: u32,
    },
    /// Register agent
    Register {
        name: String,
        #[arg(
            short,
            long,
            default_value = "Housaky AGI collective intelligence node"
        )]
        description: String,
    },
}

// ============================================================================
// GSD Commands
// ============================================================================

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GSDCommands {
    /// New project
    NewProject { name: String, vision: String },
    /// Create phase
    Phase {
        name: String,
        description: String,
        #[arg(short, long)]
        goals: Vec<String>,
    },
    /// Discuss phase
    Discuss {
        #[arg(short, long)]
        phase_id: String,
        #[arg(short, long)]
        answers: Vec<String>,
    },
    /// Execute phase
    Execute {
        #[arg(short, long)]
        phase_id: String,
        #[arg(short, long)]
        task: String,
    },
    /// Quick execute
    Quick { task: String },
    /// Verify phase
    Verify {
        #[arg(short, long)]
        phase_id: String,
    },
    /// Status
    Status,
    /// Analyze complexity
    Analyze { task: String },
    /// Awareness report
    Awareness,
}

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum McpCommands {
    List,
    Installed,
    Install { name: String },
    Uninstall { name: String },
    Enable { name: String },
    Disable { name: String },
}

// Subdirectory modules
mod approvals;
mod browser;
mod memory;
mod nodes;
mod sandbox;
mod security;
mod sessions;
mod system;

pub use approvals::ApprovalsCommands;
pub use browser::BrowserCommands;
pub use memory::MemoryCommands;
pub use nodes::{NodeConfig, NodeEncryptionConfig, PeerInfo, PeerRequest, PeerStatus, SecurityInsight, ShareableCapability, SharedDiff, SharedTool, NodesCommands};
pub use sandbox::SandboxCommands;
pub use security::SecurityCommands;
pub use sessions::SessionsCommands;
pub use system::{HeartbeatAction, SystemCommands};

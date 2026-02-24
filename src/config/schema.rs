use crate::security::AutonomyLevel;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use directories::UserDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

// ── Top-level config ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Workspace directory - computed from home, not serialized
    #[serde(skip)]
    pub workspace_dir: PathBuf,
    /// Path to config.toml - computed from home, not serialized
    #[serde(skip)]
    pub config_path: PathBuf,
    pub api_key: Option<String>,
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub default_temperature: f64,

    #[serde(default)]
    pub observability: ObservabilityConfig,

    #[serde(default)]
    pub autonomy: AutonomyConfig,

    #[serde(default)]
    pub runtime: RuntimeConfig,

    #[serde(default)]
    pub reliability: ReliabilityConfig,

    #[serde(default)]
    pub scheduler: SchedulerConfig,

    #[serde(default)]
    pub agent: AgentConfig,

    #[serde(default)]
    pub tools: ToolConfig,

    #[serde(default)]
    pub fallback: FallbackConfig,

    /// Model routing rules — route `hint:<name>` to specific provider+model combos.
    #[serde(default)]
    pub model_routes: Vec<ModelRouteConfig>,

    #[serde(default)]
    pub heartbeat: HeartbeatConfig,

    #[serde(default)]
    pub channels_config: ChannelsConfig,

    #[serde(default)]
    pub memory: MemoryConfig,

    #[serde(default)]
    pub tunnel: TunnelConfig,

    #[serde(default)]
    pub gateway: GatewayConfig,

    #[serde(default)]
    pub composio: ComposioConfig,

    #[serde(default)]
    pub secrets: SecretsConfig,

    #[serde(default)]
    pub browser: BrowserConfig,

    #[serde(default)]
    pub http_request: HttpRequestConfig,

    #[serde(default)]
    pub identity: IdentityConfig,

    #[serde(default)]
    pub cost: CostConfig,

    #[serde(default)]
    pub peripherals: PeripheralsConfig,

    /// Delegate agent configurations for multi-agent workflows.
    #[serde(default)]
    pub agents: HashMap<String, DelegateAgentConfig>,

    /// Hardware configuration (wizard-driven physical world setup).
    #[serde(default)]
    pub hardware: HardwareConfig,

    /// Enable AGI features in channels (reasoning, goals, thoughts)
    #[serde(default)]
    pub agi_enabled: bool,
}

// ── Delegate Agents ──────────────────────────────────────────────

/// Configuration for a delegate sub-agent used by the `delegate` tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateAgentConfig {
    /// Provider name (e.g. "ollama", "openrouter", "anthropic")
    pub provider: String,
    /// Model name
    pub model: String,
    /// Optional system prompt for the sub-agent
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Optional API key override
    #[serde(default)]
    pub api_key: Option<String>,
    /// Temperature override
    #[serde(default)]
    pub temperature: Option<f64>,
    /// Max recursion depth for nested delegation
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
}

fn default_max_depth() -> u32 {
    3
}

// ── Hardware Config (wizard-driven) ─────────────────────────────

/// Hardware transport mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HardwareTransport {
    #[default]
    None,
    Native,
    Serial,
    Probe,
}

impl std::fmt::Display for HardwareTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Native => write!(f, "native"),
            Self::Serial => write!(f, "serial"),
            Self::Probe => write!(f, "probe"),
        }
    }
}

/// Wizard-driven hardware configuration for physical world interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareConfig {
    /// Whether hardware access is enabled
    #[serde(default)]
    pub enabled: bool,
    /// Transport mode
    #[serde(default)]
    pub transport: HardwareTransport,
    /// Serial port path (e.g. "/dev/ttyACM0")
    #[serde(default)]
    pub serial_port: Option<String>,
    /// Serial baud rate
    #[serde(default = "default_baud_rate")]
    pub baud_rate: u32,
    /// Probe target chip (e.g. "STM32F401RE")
    #[serde(default)]
    pub probe_target: Option<String>,
    /// Enable workspace datasheet RAG (index PDF schematics for AI pin lookups)
    #[serde(default)]
    pub workspace_datasheets: bool,
}

fn default_baud_rate() -> u32 {
    115_200
}

impl HardwareConfig {
    /// Return the active transport mode.
    pub fn transport_mode(&self) -> HardwareTransport {
        self.transport.clone()
    }
}

impl Default for HardwareConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            transport: HardwareTransport::None,
            serial_port: None,
            baud_rate: default_baud_rate(),
            probe_target: None,
            workspace_datasheets: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// When true: bootstrap_max_chars=6000, rag_chunk_limit=2. Use for 13B or smaller models.
    #[serde(default)]
    pub compact_context: bool,
    #[serde(default = "default_agent_max_tool_iterations")]
    pub max_tool_iterations: usize,
    #[serde(default = "default_agent_max_history_messages")]
    pub max_history_messages: usize,
    #[serde(default)]
    pub parallel_tools: bool,
    #[serde(default = "default_agent_tool_dispatcher")]
    pub tool_dispatcher: String,
    /// Number of recent messages to keep during history compaction
    #[serde(default = "default_compaction_keep_recent_messages")]
    pub compaction_keep_recent_messages: usize,
    /// Maximum characters for compaction source text
    #[serde(default = "default_compaction_max_source_chars")]
    pub compaction_max_source_chars: usize,
    /// Maximum characters for compaction summary output
    #[serde(default = "default_compaction_max_summary_chars")]
    pub compaction_max_summary_chars: usize,
    /// Maximum characters for bootstrap context loading
    #[serde(default = "default_bootstrap_max_chars")]
    pub bootstrap_max_chars: usize,
    /// AGI subsystem configuration
    #[serde(default)]
    pub agi: AGISubsystemConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AGISubsystemConfig {
    /// Enable goal tracking during agent execution
    #[serde(default = "default_true")]
    pub enable_goal_tracking: bool,
    /// Enable continuous learning from tool execution feedback
    #[serde(default = "default_true")]
    pub enable_learning: bool,
    /// Enable meta-cognitive reflection
    #[serde(default = "default_true")]
    pub enable_meta_cognition: bool,
    /// Enable decision journaling
    #[serde(default = "default_true")]
    pub enable_decision_journal: bool,
    /// Enable value drift detection
    #[serde(default = "default_true")]
    pub enable_drift_detection: bool,
    /// Number of turns between periodic reflections
    #[serde(default = "default_reflection_interval")]
    pub reflection_interval_turns: u32,
    /// Number of turns between drift checks
    #[serde(default = "default_drift_check_interval")]
    pub drift_check_interval_turns: u32,
    /// Maximum decisions stored in journal
    #[serde(default = "default_max_decisions")]
    pub max_decisions_in_journal: usize,
}

fn default_reflection_interval() -> u32 {
    10
}

fn default_drift_check_interval() -> u32 {
    20
}

fn default_max_decisions() -> usize {
    1000
}

impl Default for AGISubsystemConfig {
    fn default() -> Self {
        Self {
            enable_goal_tracking: true,
            enable_learning: true,
            enable_meta_cognition: true,
            enable_decision_journal: true,
            enable_drift_detection: true,
            reflection_interval_turns: default_reflection_interval(),
            drift_check_interval_turns: default_drift_check_interval(),
            max_decisions_in_journal: default_max_decisions(),
        }
    }
}

fn default_agent_max_tool_iterations() -> usize {
    usize::MAX
}

fn default_agent_max_history_messages() -> usize {
    50
}

fn default_agent_tool_dispatcher() -> String {
    "auto".into()
}

fn default_compaction_keep_recent_messages() -> usize {
    20
}

fn default_compaction_max_source_chars() -> usize {
    12_000
}

fn default_compaction_max_summary_chars() -> usize {
    2_000
}

fn default_bootstrap_max_chars() -> usize {
    20_000
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            compact_context: false,
            max_tool_iterations: default_agent_max_tool_iterations(),
            max_history_messages: default_agent_max_history_messages(),
            parallel_tools: false,
            tool_dispatcher: default_agent_tool_dispatcher(),
            compaction_keep_recent_messages: default_compaction_keep_recent_messages(),
            compaction_max_source_chars: default_compaction_max_source_chars(),
            compaction_max_summary_chars: default_compaction_max_summary_chars(),
            bootstrap_max_chars: default_bootstrap_max_chars(),
            agi: AGISubsystemConfig::default(),
        }
    }
}

// ── Tool Configuration ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Shell command timeout in seconds
    #[serde(default = "default_shell_timeout_secs")]
    pub shell_timeout_secs: u64,
    /// Maximum shell output in bytes (1MB default)
    #[serde(default = "default_shell_max_output_bytes")]
    pub shell_max_output_bytes: usize,
    /// Maximum file read size in bytes (10MB default)
    #[serde(default = "default_file_read_max_bytes")]
    pub file_read_max_bytes: usize,
    /// Delegate tool timeout in seconds
    #[serde(default = "default_delegate_timeout_secs")]
    pub delegate_timeout_secs: u64,
    /// Screenshot capture timeout in seconds
    #[serde(default = "default_screenshot_timeout_secs")]
    pub screenshot_timeout_secs: u64,
    /// Maximum screenshot size in bytes (2MB default)
    #[serde(default = "default_screenshot_max_bytes")]
    pub screenshot_max_bytes: usize,
    /// Maximum image size in bytes (5MB default)
    #[serde(default = "default_image_max_bytes")]
    pub image_max_bytes: usize,
}

fn default_shell_timeout_secs() -> u64 {
    60
}

fn default_shell_max_output_bytes() -> usize {
    1_048_576
}

fn default_file_read_max_bytes() -> usize {
    10_485_760
}

fn default_delegate_timeout_secs() -> u64 {
    120
}

fn default_screenshot_timeout_secs() -> u64 {
    15
}

fn default_screenshot_max_bytes() -> usize {
    2_097_152
}

fn default_image_max_bytes() -> usize {
    5_242_880
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            shell_timeout_secs: default_shell_timeout_secs(),
            shell_max_output_bytes: default_shell_max_output_bytes(),
            file_read_max_bytes: default_file_read_max_bytes(),
            delegate_timeout_secs: default_delegate_timeout_secs(),
            screenshot_timeout_secs: default_screenshot_timeout_secs(),
            screenshot_max_bytes: default_screenshot_max_bytes(),
            image_max_bytes: default_image_max_bytes(),
        }
    }
}

// ── Fallback Provider Configuration ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfig {
    /// Enable fallback provider rotation on usage limits
    #[serde(default = "default_fallback_enabled")]
    pub enabled: bool,
    /// Ordered list of fallback providers
    #[serde(default)]
    pub providers: Vec<FallbackProvider>,
    /// Rotate to next provider at this usage percentage (default: 80%)
    #[serde(default = "default_rotate_at_percent")]
    pub rotate_at_percent: u8,
    /// Auto-rotate on rate limit (429) errors
    #[serde(default = "default_true")]
    pub rotate_on_rate_limit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackProvider {
    /// Provider name (openrouter, anthropic, openai, etc.)
    pub name: String,
    /// Optional custom base URL
    #[serde(default)]
    pub base_url: Option<String>,
    /// Encrypted API key (using Blake3)
    #[serde(default)]
    pub api_key_encrypted: Option<String>,
    /// Priority-ordered models for this provider
    #[serde(default)]
    pub models: Vec<String>,
    /// Priority (1 = highest, 2 = second, etc.)
    #[serde(default = "default_fallback_priority")]
    pub priority: u8,
}

fn default_fallback_enabled() -> bool {
    true
}

fn default_rotate_at_percent() -> u8 {
    80
}

fn default_fallback_priority() -> u8 {
    1
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            enabled: default_fallback_enabled(),
            providers: Vec::new(),
            rotate_at_percent: default_rotate_at_percent(),
            rotate_on_rate_limit: true,
        }
    }
}

// ── Identity (AIEOS / OpenClaw format) ──────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    /// Identity format: "openclaw" (default) or "aieos"
    #[serde(default = "default_identity_format")]
    pub format: String,
    /// Path to AIEOS JSON file (relative to workspace)
    #[serde(default)]
    pub aieos_path: Option<String>,
    /// Inline AIEOS JSON (alternative to file path)
    #[serde(default)]
    pub aieos_inline: Option<String>,
}

fn default_identity_format() -> String {
    "openclaw".into()
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            format: default_identity_format(),
            aieos_path: None,
            aieos_inline: None,
        }
    }
}

// ── Cost tracking and budget enforcement ───────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    /// Enable cost tracking (default: false)
    #[serde(default)]
    pub enabled: bool,

    /// Daily spending limit in USD (default: 10.00)
    #[serde(default = "default_daily_limit")]
    pub daily_limit_usd: f64,

    /// Monthly spending limit in USD (default: 100.00)
    #[serde(default = "default_monthly_limit")]
    pub monthly_limit_usd: f64,

    /// Warn when spending reaches this percentage of limit (default: 80)
    #[serde(default = "default_warn_percent")]
    pub warn_at_percent: u8,

    /// Allow requests to exceed budget with --override flag (default: false)
    #[serde(default)]
    pub allow_override: bool,

    /// Per-model pricing (USD per 1M tokens)
    #[serde(default)]
    pub prices: std::collections::HashMap<String, ModelPricing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Input price per 1M tokens
    #[serde(default)]
    pub input: f64,

    /// Output price per 1M tokens
    #[serde(default)]
    pub output: f64,
}

fn default_daily_limit() -> f64 {
    10.0
}

fn default_monthly_limit() -> f64 {
    100.0
}

fn default_warn_percent() -> u8 {
    80
}

impl Default for CostConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            daily_limit_usd: default_daily_limit(),
            monthly_limit_usd: default_monthly_limit(),
            warn_at_percent: default_warn_percent(),
            allow_override: false,
            prices: get_default_pricing(),
        }
    }
}

/// Default pricing for popular models (USD per 1M tokens)
fn get_default_pricing() -> std::collections::HashMap<String, ModelPricing> {
    let mut prices = std::collections::HashMap::new();

    // Anthropic models
    prices.insert(
        "arcee-ai/trinity-large-preview:free".into(),
        ModelPricing {
            input: 3.0,
            output: 15.0,
        },
    );
    prices.insert(
        "anthropic/claude-opus-4-20250514".into(),
        ModelPricing {
            input: 15.0,
            output: 75.0,
        },
    );
    prices.insert(
        "anthropic/claude-3.5-sonnet".into(),
        ModelPricing {
            input: 3.0,
            output: 15.0,
        },
    );
    prices.insert(
        "anthropic/claude-3-haiku".into(),
        ModelPricing {
            input: 0.25,
            output: 1.25,
        },
    );

    // OpenAI models
    prices.insert(
        "openai/gpt-4o".into(),
        ModelPricing {
            input: 5.0,
            output: 15.0,
        },
    );
    prices.insert(
        "openai/gpt-4o-mini".into(),
        ModelPricing {
            input: 0.15,
            output: 0.60,
        },
    );
    prices.insert(
        "openai/o1-preview".into(),
        ModelPricing {
            input: 15.0,
            output: 60.0,
        },
    );

    // Google models
    prices.insert(
        "google/gemini-2.0-flash".into(),
        ModelPricing {
            input: 0.10,
            output: 0.40,
        },
    );
    prices.insert(
        "google/gemini-1.5-pro".into(),
        ModelPricing {
            input: 1.25,
            output: 5.0,
        },
    );

    prices
}

// ── Peripherals (hardware: STM32, RPi GPIO, etc.) ────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PeripheralsConfig {
    /// Enable peripheral support (boards become agent tools)
    #[serde(default)]
    pub enabled: bool,
    /// Board configurations (nucleo-f401re, rpi-gpio, etc.)
    #[serde(default)]
    pub boards: Vec<PeripheralBoardConfig>,
    /// Path to datasheet docs (relative to workspace) for RAG retrieval.
    /// Place .md/.txt files named by board (e.g. nucleo-f401re.md, rpi-gpio.md).
    #[serde(default)]
    pub datasheet_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeripheralBoardConfig {
    /// Board type: "nucleo-f401re", "rpi-gpio", "esp32", etc.
    pub board: String,
    /// Transport: "serial", "native", "websocket"
    #[serde(default = "default_peripheral_transport")]
    pub transport: String,
    /// Path for serial: "/dev/ttyACM0", "/dev/ttyUSB0"
    #[serde(default)]
    pub path: Option<String>,
    /// Baud rate for serial (default: 115200)
    #[serde(default = "default_peripheral_baud")]
    pub baud: u32,
}

fn default_peripheral_transport() -> String {
    "serial".into()
}

fn default_peripheral_baud() -> u32 {
    115_200
}

impl Default for PeripheralBoardConfig {
    fn default() -> Self {
        Self {
            board: String::new(),
            transport: default_peripheral_transport(),
            path: None,
            baud: default_peripheral_baud(),
        }
    }
}

// ── Gateway security ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Gateway port (default: 8080)
    #[serde(default = "default_gateway_port")]
    pub port: u16,
    /// Gateway host (default: 127.0.0.1)
    #[serde(default = "default_gateway_host")]
    pub host: String,
    /// Require pairing before accepting requests (default: true)
    #[serde(default = "default_true")]
    pub require_pairing: bool,
    /// Allow binding to non-localhost without a tunnel (default: false)
    #[serde(default)]
    pub allow_public_bind: bool,
    /// Paired bearer tokens (managed automatically, not user-edited)
    #[serde(default)]
    pub paired_tokens: Vec<String>,

    /// Max `/pair` requests per minute per client key.
    #[serde(default = "default_pair_rate_limit")]
    pub pair_rate_limit_per_minute: u32,

    /// Max `/webhook` requests per minute per client key.
    #[serde(default = "default_webhook_rate_limit")]
    pub webhook_rate_limit_per_minute: u32,

    /// TTL for webhook idempotency keys.
    #[serde(default = "default_idempotency_ttl_secs")]
    pub idempotency_ttl_secs: u64,

    /// Maximum HTTP body size in bytes (default: 64KB)
    #[serde(default = "default_gateway_max_body_size")]
    pub max_body_size: usize,

    /// HTTP request timeout in seconds (default: 30)
    #[serde(default = "default_gateway_request_timeout_secs")]
    pub request_timeout_secs: u64,

    /// Rate limit window in seconds (default: 60)
    #[serde(default = "default_gateway_rate_limit_window_secs")]
    pub rate_limit_window_secs: u64,

    /// Maximum pairing attempts before lockout
    #[serde(default = "default_pairing_max_attempts")]
    pub pairing_max_attempts: u32,

    /// Pairing lockout duration in seconds (default: 300 = 5 min)
    #[serde(default = "default_pairing_lockout_secs")]
    pub pairing_lockout_secs: u64,

    /// Rate limiter sweep interval in seconds
    #[serde(default = "default_rate_limiter_sweep_secs")]
    pub rate_limiter_sweep_secs: u64,
}

fn default_gateway_port() -> u16 {
    3000
}

fn default_gateway_host() -> String {
    "127.0.0.1".into()
}

fn default_pair_rate_limit() -> u32 {
    10
}

fn default_webhook_rate_limit() -> u32 {
    60
}

fn default_idempotency_ttl_secs() -> u64 {
    300
}

fn default_gateway_max_body_size() -> usize {
    65_536
}

fn default_gateway_request_timeout_secs() -> u64 {
    30
}

fn default_gateway_rate_limit_window_secs() -> u64 {
    60
}

fn default_pairing_max_attempts() -> u32 {
    5
}

fn default_pairing_lockout_secs() -> u64 {
    300
}

fn default_rate_limiter_sweep_secs() -> u64 {
    300
}

fn default_true() -> bool {
    true
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: default_gateway_port(),
            host: default_gateway_host(),
            require_pairing: true,
            allow_public_bind: false,
            paired_tokens: Vec::new(),
            pair_rate_limit_per_minute: default_pair_rate_limit(),
            webhook_rate_limit_per_minute: default_webhook_rate_limit(),
            idempotency_ttl_secs: default_idempotency_ttl_secs(),
            max_body_size: default_gateway_max_body_size(),
            request_timeout_secs: default_gateway_request_timeout_secs(),
            rate_limit_window_secs: default_gateway_rate_limit_window_secs(),
            pairing_max_attempts: default_pairing_max_attempts(),
            pairing_lockout_secs: default_pairing_lockout_secs(),
            rate_limiter_sweep_secs: default_rate_limiter_sweep_secs(),
        }
    }
}

// ── Composio (managed tool surface) ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposioConfig {
    /// Enable Composio integration for 1000+ OAuth tools
    #[serde(default)]
    pub enabled: bool,
    /// Composio API key (stored encrypted when secrets.encrypt = true)
    #[serde(default)]
    pub api_key: Option<String>,
    /// Default entity ID for multi-user setups
    #[serde(default = "default_entity_id")]
    pub entity_id: String,
}

fn default_entity_id() -> String {
    "default".into()
}

impl Default for ComposioConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: None,
            entity_id: default_entity_id(),
        }
    }
}

// ── Secrets (encrypted credential store) ────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfig {
    /// Enable encryption for API keys and tokens in config.toml
    #[serde(default = "default_true")]
    pub encrypt: bool,
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self { encrypt: true }
    }
}

// ── Browser (friendly-service browsing only) ───────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserComputerUseConfig {
    /// Sidecar endpoint for computer-use actions (OS-level mouse/keyboard/screenshot)
    #[serde(default = "default_browser_computer_use_endpoint")]
    pub endpoint: String,
    /// Optional bearer token for computer-use sidecar
    #[serde(default)]
    pub api_key: Option<String>,
    /// Per-action request timeout in milliseconds
    #[serde(default = "default_browser_computer_use_timeout_ms")]
    pub timeout_ms: u64,
    /// Allow remote/public endpoint for computer-use sidecar (default: false)
    #[serde(default)]
    pub allow_remote_endpoint: bool,
    /// Optional window title/process allowlist forwarded to sidecar policy
    #[serde(default)]
    pub window_allowlist: Vec<String>,
    /// Optional X-axis boundary for coordinate-based actions
    #[serde(default)]
    pub max_coordinate_x: Option<i64>,
    /// Optional Y-axis boundary for coordinate-based actions
    #[serde(default)]
    pub max_coordinate_y: Option<i64>,
}

fn default_browser_computer_use_endpoint() -> String {
    "http://127.0.0.1:8787/v1/actions".into()
}

fn default_browser_computer_use_timeout_ms() -> u64 {
    15_000
}

impl Default for BrowserComputerUseConfig {
    fn default() -> Self {
        Self {
            endpoint: default_browser_computer_use_endpoint(),
            api_key: None,
            timeout_ms: default_browser_computer_use_timeout_ms(),
            allow_remote_endpoint: false,
            window_allowlist: Vec::new(),
            max_coordinate_x: None,
            max_coordinate_y: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// Enable `browser_open` tool (opens URLs in Brave without scraping)
    #[serde(default)]
    pub enabled: bool,
    /// Allowed domains for `browser_open` (exact or subdomain match)
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    /// Browser session name (for agent-browser automation)
    #[serde(default)]
    pub session_name: Option<String>,
    /// Browser automation backend: "agent_browser" | "rust_native" | "computer_use" | "auto"
    #[serde(default = "default_browser_backend")]
    pub backend: String,
    /// Headless mode for rust-native backend
    #[serde(default = "default_true")]
    pub native_headless: bool,
    /// WebDriver endpoint URL for rust-native backend (e.g. http://127.0.0.1:9515)
    #[serde(default = "default_browser_webdriver_url")]
    pub native_webdriver_url: String,
    /// Optional Chrome/Chromium executable path for rust-native backend
    #[serde(default)]
    pub native_chrome_path: Option<String>,
    /// Computer-use sidecar configuration
    #[serde(default)]
    pub computer_use: BrowserComputerUseConfig,
}

fn default_browser_backend() -> String {
    "agent_browser".into()
}

fn default_browser_webdriver_url() -> String {
    "http://127.0.0.1:9515".into()
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_domains: Vec::new(),
            session_name: None,
            backend: default_browser_backend(),
            native_headless: default_true(),
            native_webdriver_url: default_browser_webdriver_url(),
            native_chrome_path: None,
            computer_use: BrowserComputerUseConfig::default(),
        }
    }
}

// ── HTTP request tool ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HttpRequestConfig {
    /// Enable `http_request` tool for API interactions
    #[serde(default)]
    pub enabled: bool,
    /// Allowed domains for HTTP requests (exact or subdomain match)
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    /// Maximum response size in bytes (default: 1MB)
    #[serde(default = "default_http_max_response_size")]
    pub max_response_size: usize,
    /// Request timeout in seconds (default: 30)
    #[serde(default = "default_http_timeout_secs")]
    pub timeout_secs: u64,
}

fn default_http_max_response_size() -> usize {
    1_000_000 // 1MB
}

fn default_http_timeout_secs() -> u64 {
    30
}

// ── Memory ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// "sqlite" | "lucid" | "markdown" | "none" (`none` = explicit no-op memory)
    pub backend: String,
    /// Auto-save conversation context to memory
    pub auto_save: bool,
    /// Run memory/session hygiene (archiving + retention cleanup)
    #[serde(default = "default_hygiene_enabled")]
    pub hygiene_enabled: bool,
    /// Archive daily/session files older than this many days
    #[serde(default = "default_archive_after_days")]
    pub archive_after_days: u32,
    /// Purge archived files older than this many days
    #[serde(default = "default_purge_after_days")]
    pub purge_after_days: u32,
    /// For sqlite backend: prune conversation rows older than this many days
    #[serde(default = "default_conversation_retention_days")]
    pub conversation_retention_days: u32,
    /// Embedding provider: "none" | "openai" | "custom:URL"
    #[serde(default = "default_embedding_provider")]
    pub embedding_provider: String,
    /// Embedding model name (e.g. "text-embedding-3-small")
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
    /// Embedding vector dimensions
    #[serde(default = "default_embedding_dims")]
    pub embedding_dimensions: usize,
    /// Weight for vector similarity in hybrid search (0.0–1.0)
    #[serde(default = "default_vector_weight")]
    pub vector_weight: f64,
    /// Weight for keyword BM25 in hybrid search (0.0–1.0)
    #[serde(default = "default_keyword_weight")]
    pub keyword_weight: f64,
    /// Max embedding cache entries before LRU eviction
    #[serde(default = "default_cache_size")]
    pub embedding_cache_size: usize,
    /// Max tokens per chunk for document splitting
    #[serde(default = "default_chunk_size")]
    pub chunk_max_tokens: usize,

    // ── Response Cache (saves tokens on repeated prompts) ──────
    /// Enable LLM response caching to avoid paying for duplicate prompts
    #[serde(default)]
    pub response_cache_enabled: bool,
    /// TTL in minutes for cached responses (default: 60)
    #[serde(default = "default_response_cache_ttl")]
    pub response_cache_ttl_minutes: u32,
    /// Max number of cached responses before LRU eviction (default: 5000)
    #[serde(default = "default_response_cache_max")]
    pub response_cache_max_entries: usize,

    // ── Memory Snapshot (soul backup to Markdown) ─────────────
    /// Enable periodic export of core memories to MEMORY_SNAPSHOT.md
    #[serde(default)]
    pub snapshot_enabled: bool,
    /// Run snapshot during hygiene passes (heartbeat-driven)
    #[serde(default)]
    pub snapshot_on_hygiene: bool,
    /// Auto-hydrate from MEMORY_SNAPSHOT.md when brain.db is missing
    #[serde(default = "default_true")]
    pub auto_hydrate: bool,
}

fn default_embedding_provider() -> String {
    "none".into()
}
fn default_hygiene_enabled() -> bool {
    true
}
fn default_archive_after_days() -> u32 {
    7
}
fn default_purge_after_days() -> u32 {
    30
}
fn default_conversation_retention_days() -> u32 {
    30
}
fn default_embedding_model() -> String {
    "text-embedding-3-small".into()
}
fn default_embedding_dims() -> usize {
    1536
}
fn default_vector_weight() -> f64 {
    0.7
}
fn default_keyword_weight() -> f64 {
    0.3
}
fn default_cache_size() -> usize {
    10_000
}
fn default_chunk_size() -> usize {
    512
}
fn default_response_cache_ttl() -> u32 {
    60
}
fn default_response_cache_max() -> usize {
    5_000
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            backend: "sqlite".into(),
            auto_save: true,
            hygiene_enabled: default_hygiene_enabled(),
            archive_after_days: default_archive_after_days(),
            purge_after_days: default_purge_after_days(),
            conversation_retention_days: default_conversation_retention_days(),
            embedding_provider: default_embedding_provider(),
            embedding_model: default_embedding_model(),
            embedding_dimensions: default_embedding_dims(),
            vector_weight: default_vector_weight(),
            keyword_weight: default_keyword_weight(),
            embedding_cache_size: default_cache_size(),
            chunk_max_tokens: default_chunk_size(),
            response_cache_enabled: false,
            response_cache_ttl_minutes: default_response_cache_ttl(),
            response_cache_max_entries: default_response_cache_max(),
            snapshot_enabled: false,
            snapshot_on_hygiene: false,
            auto_hydrate: true,
        }
    }
}

// ── Observability ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// "none" | "log" | "prometheus" | "otel"
    pub backend: String,

    /// OTLP endpoint (e.g. "http://localhost:4318"). Only used when backend = "otel".
    #[serde(default)]
    pub otel_endpoint: Option<String>,

    /// Service name reported to the OTel collector. Defaults to "housaky".
    #[serde(default)]
    pub otel_service_name: Option<String>,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            backend: "none".into(),
            otel_endpoint: None,
            otel_service_name: None,
        }
    }
}

// ── Autonomy / Security ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomyConfig {
    pub level: AutonomyLevel,
    pub workspace_only: bool,
    pub allowed_commands: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub max_actions_per_hour: u32,
    pub max_cost_per_day_cents: u32,

    /// Require explicit approval for medium-risk shell commands.
    #[serde(default = "default_true")]
    pub require_approval_for_medium_risk: bool,

    /// Block high-risk shell commands even if allowlisted.
    #[serde(default = "default_true")]
    pub block_high_risk_commands: bool,
}

impl Default for AutonomyConfig {
    fn default() -> Self {
        Self {
            level: AutonomyLevel::Supervised,
            workspace_only: true,
            allowed_commands: vec![
                "git".into(),
                "npm".into(),
                "cargo".into(),
                "ls".into(),
                "cat".into(),
                "grep".into(),
                "find".into(),
                "echo".into(),
                "pwd".into(),
                "wc".into(),
                "head".into(),
                "tail".into(),
            ],
            forbidden_paths: vec![
                "/etc".into(),
                "/root".into(),
                "/home".into(),
                "/usr".into(),
                "/bin".into(),
                "/sbin".into(),
                "/lib".into(),
                "/opt".into(),
                "/boot".into(),
                "/dev".into(),
                "/proc".into(),
                "/sys".into(),
                "/var".into(),
                "/tmp".into(),
                "~/.ssh".into(),
                "~/.gnupg".into(),
                "~/.aws".into(),
                "~/.config".into(),
            ],
            max_actions_per_hour: 20,
            max_cost_per_day_cents: 500,
            require_approval_for_medium_risk: true,
            block_high_risk_commands: true,
        }
    }
}

// ── Runtime ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Runtime kind (`native` | `docker` | `wasm` | `cloudflare`).
    #[serde(default = "default_runtime_kind")]
    pub kind: String,

    /// Docker runtime settings (used when `kind = "docker"`).
    #[serde(default)]
    pub docker: DockerRuntimeConfig,

    /// WASM runtime settings (used when `kind = "wasm"`).
    #[serde(default)]
    pub wasm: WasmRuntimeConfig,

    /// Cloudflare Workers runtime settings (used when `kind = "cloudflare"`).
    #[serde(default)]
    pub cloudflare: CloudflareRuntimeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerRuntimeConfig {
    /// Runtime image used to execute shell commands.
    #[serde(default = "default_docker_image")]
    pub image: String,

    /// Docker network mode (`none`, `bridge`, etc.).
    #[serde(default = "default_docker_network")]
    pub network: String,

    /// Optional memory limit in MB (`None` = no explicit limit).
    #[serde(default = "default_docker_memory_limit_mb")]
    pub memory_limit_mb: Option<u64>,

    /// Optional CPU limit (`None` = no explicit limit).
    #[serde(default = "default_docker_cpu_limit")]
    pub cpu_limit: Option<f64>,

    /// Mount root filesystem as read-only.
    #[serde(default = "default_true")]
    pub read_only_rootfs: bool,

    /// Mount configured workspace into `/workspace`.
    #[serde(default = "default_true")]
    pub mount_workspace: bool,

    /// Optional workspace root allowlist for Docker mount validation.
    #[serde(default)]
    pub allowed_workspace_roots: Vec<String>,
}

fn default_runtime_kind() -> String {
    "native".into()
}

fn default_docker_image() -> String {
    "alpine:3.20".into()
}

fn default_docker_network() -> String {
    "none".into()
}

fn default_docker_memory_limit_mb() -> Option<u64> {
    Some(512)
}

fn default_docker_cpu_limit() -> Option<f64> {
    Some(1.0)
}

impl Default for DockerRuntimeConfig {
    fn default() -> Self {
        Self {
            image: default_docker_image(),
            network: default_docker_network(),
            memory_limit_mb: default_docker_memory_limit_mb(),
            cpu_limit: default_docker_cpu_limit(),
            read_only_rootfs: true,
            mount_workspace: true,
            allowed_workspace_roots: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmRuntimeConfig {
    /// Directory containing WASM tool modules (relative to workspace).
    #[serde(default = "default_wasm_tools_dir")]
    pub tools_dir: String,
    /// Memory limit per module in MB.
    #[serde(default = "default_wasm_memory_limit_mb")]
    pub memory_limit_mb: u64,
    /// Fuel limit (instruction count) per invocation.
    #[serde(default = "default_wasm_fuel_limit")]
    pub fuel_limit: u64,
    /// Allow reading files from workspace.
    #[serde(default)]
    pub allow_workspace_read: bool,
    /// Allow writing files to workspace.
    #[serde(default)]
    pub allow_workspace_write: bool,
    /// Allowed HTTP hosts for network access (empty = no network).
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
}

fn default_wasm_tools_dir() -> String {
    "tools/wasm".into()
}

fn default_wasm_memory_limit_mb() -> u64 {
    64
}

fn default_wasm_fuel_limit() -> u64 {
    1_000_000
}

impl Default for WasmRuntimeConfig {
    fn default() -> Self {
        Self {
            tools_dir: default_wasm_tools_dir(),
            memory_limit_mb: default_wasm_memory_limit_mb(),
            fuel_limit: default_wasm_fuel_limit(),
            allow_workspace_read: false,
            allow_workspace_write: false,
            allowed_hosts: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareRuntimeConfig {
    /// Cloudflare account ID.
    #[serde(default)]
    pub account_id: String,
    /// API token for Cloudflare Workers.
    #[serde(default)]
    pub api_token: String,
    /// Worker name prefix.
    #[serde(default = "default_cloudflare_worker_name")]
    pub worker_name: String,
    /// Memory limit in MB.
    #[serde(default = "default_cloudflare_memory_mb")]
    pub memory_limit_mb: u64,
    /// CPU timeout in ms.
    #[serde(default = "default_cloudflare_cpu_timeout_ms")]
    pub cpu_timeout_ms: u64,
    /// Environment (production, staging, etc).
    #[serde(default)]
    pub environment: String,
    /// Secrets (JSON string of key-value pairs).
    #[serde(default)]
    pub secrets: std::collections::HashMap<String, String>,
    /// Bindings (KV, DurableObjects, etc).
    #[serde(default)]
    pub bindings: Vec<BindingConfig>,
}

fn default_cloudflare_worker_name() -> String {
    "housaky-agent".into()
}

fn default_cloudflare_memory_mb() -> u64 {
    128
}

fn default_cloudflare_cpu_timeout_ms() -> u64 {
    50_000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingConfig {
    pub name: String,
    pub binding_type: String,
    pub config: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for CloudflareRuntimeConfig {
    fn default() -> Self {
        Self {
            account_id: String::new(),
            api_token: String::new(),
            worker_name: default_cloudflare_worker_name(),
            memory_limit_mb: default_cloudflare_memory_mb(),
            cpu_timeout_ms: default_cloudflare_cpu_timeout_ms(),
            environment: "production".into(),
            secrets: std::collections::HashMap::new(),
            bindings: Vec::new(),
        }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            kind: default_runtime_kind(),
            docker: DockerRuntimeConfig::default(),
            wasm: WasmRuntimeConfig::default(),
            cloudflare: CloudflareRuntimeConfig::default(),
        }
    }
}

// ── Reliability / supervision ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReliabilityConfig {
    /// Provider configurations with detailed key management
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
    /// Global rotation settings
    #[serde(default)]
    pub rotation: GlobalRotationConfig,
    /// Health monitoring configuration
    #[serde(default)]
    pub health_monitoring: HealthMonitoringConfig,
    /// Key storage and encryption
    #[serde(default)]
    pub key_storage: KeyStorageConfig,
    /// Retries per provider before failing over.
    #[serde(default = "default_provider_retries")]
    pub provider_retries: u32,
    /// Base backoff (ms) for provider retry delay.
    #[serde(default = "default_provider_backoff_ms")]
    pub provider_backoff_ms: u64,
    /// Fallback provider chain (e.g. `"anthropic", "openai"]).
    #[serde(default)]
    pub fallback_providers: Vec<String>,
    /// Enable automatic API key rotation on rate-limit (429) errors.
    #[serde(default = "default_true")]
    pub auto_rotate_on_limit: bool,
    /// Path to KVM-style keys.json for auto-loading rotated keys.
    #[serde(default)]
    pub kvm_keys_path: Option<String>,
    /// Additional API keys for round-robin rotation on rate-limit (429) errors.
    /// The primary `api_key` is always tried first; these are extras.
    #[serde(default)]
    pub api_keys: Vec<String>,
    /// Per-model fallback chains. When a model fails, try these alternatives in order.
    /// Example: `{ "claude-opus-4-20250514" = ["claude-sonnet-4-20250514", "gpt-4o"] }`
    #[serde(default)]
    pub model_fallbacks: std::collections::HashMap<String, Vec<String>>,
    /// Initial backoff for channel/daemon restarts.
    #[serde(default = "default_channel_backoff_secs")]
    pub channel_initial_backoff_secs: u64,
    /// Max backoff for channel/daemon restarts.
    #[serde(default = "default_channel_backoff_max_secs")]
    pub channel_max_backoff_secs: u64,
    /// Scheduler polling cadence in seconds.
    #[serde(default = "default_scheduler_poll_secs")]
    pub scheduler_poll_secs: u64,
    /// Max retries for cron job execution attempts.
    #[serde(default = "default_scheduler_retries")]
    pub scheduler_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider name (openrouter, anthropic, openai, etc.)
    pub name: String,
    /// Base URL for API requests
    pub base_url: Option<String>,
    /// Model configurations
    pub models: Vec<ModelConfig>,
    /// Authentication method (api_key, oauth, etc.)
    pub auth_method: AuthMethod,
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Maximum concurrent requests
    #[serde(default = "default_concurrent")]
    pub max_concurrent: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model name
    pub name: String,
    /// API key configurations
    pub api_keys: Vec<ApiKeyConfig>,
    /// Request limits
    pub rate_limit: Option<RateLimitConfig>,
    /// Temperature and other model parameters
    #[serde(flatten)]
    pub model_params: ModelParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// API key value
    pub key: String,
    /// Key metadata
    pub metadata: KeyMetadata,
    /// Usage statistics
    #[serde(default)]
    pub usage: UsageStats,
    /// Rotation configuration
    #[serde(default)]
    pub rotation: RotationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationConfig {
    /// Enable automatic rotation
    #[serde(default)]
    pub enabled: bool,
    /// Rotation strategy (round-robin, priority, health-based)
    #[serde(default = "default_rotation_strategy")]
    pub strategy: RotationStrategy,
    /// Threshold for rotation (usage %, error rate, etc.)
    #[serde(default)]
    pub threshold: RotationThreshold,
    /// Cooldown period between rotations
    #[serde(default = "default_cooldown")]
    pub cooldown_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalRotationConfig {
    /// Enable global rotation
    #[serde(default)]
    pub enabled: bool,
    /// Default rotation strategy
    #[serde(default = "default_global_strategy")]
    pub default_strategy: RotationStrategy,
    /// Global cooldown period
    #[serde(default = "default_global_cooldown")]
    pub global_cooldown_secs: u64,
    /// Health check interval
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMonitoringConfig {
    /// Enable health monitoring
    #[serde(default)]
    pub enabled: bool,
    /// Health check endpoints
    #[serde(default)]
    pub endpoints: Vec<HealthEndpoint>,
    /// Failure threshold for key rotation
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,
    /// Success rate threshold
    #[serde(default = "default_success_rate")]
    pub success_rate_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyStorageConfig {
    /// Enable encryption for API keys
    #[serde(default = "default_true")]
    pub encrypt: bool,
    /// Encryption key storage method
    #[serde(default = "default_encryption_method")]
    pub encryption_method: EncryptionMethod,
    /// Key rotation schedule
    #[serde(default)]
    pub rotation_schedule: RotationSchedule,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationSchedule {
    /// Daily rotation time (HH:MM)
    #[serde(default = "default_daily_rotation_time")]
    pub daily_rotation_time: String,
    /// Weekly rotation day (0=Sunday, 6=Saturday)
    #[serde(default = "default_weekly_rotation_day")]
    pub weekly_rotation_day: u32,
    /// Monthly rotation day (1-31)
    #[serde(default = "default_monthly_rotation_day")]
    pub monthly_rotation_day: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthEndpoint {
    /// Endpoint URL for health check
    pub url: String,
    /// HTTP method (GET, POST, etc.)
    #[serde(default = "default_health_method")]
    pub method: String,
    /// Expected response status
    #[serde(default = "default_expected_status")]
    pub expected_status: u16,
    /// Request timeout
    #[serde(default = "default_health_timeout")]
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    /// Total requests made
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Error rate percentage
    #[serde(default = "default_error_rate")]
    pub error_rate: f64,
    /// Usage percentage (0-100)
    #[serde(default = "default_usage_percent")]
    pub usage_percent: u8,
    /// Last usage update timestamp
    #[serde(default = "default_timestamp")]
    pub last_update: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    /// Key description
    pub description: String,
    /// Creation date
    #[serde(default = "default_timestamp")]
    pub created_at: DateTime<Utc>,
    /// Last rotation date
    #[serde(default = "default_timestamp")]
    pub last_rotated: DateTime<Utc>,
    /// Tags for organization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Environment (production, staging, development)
    #[serde(default = "default_environment")]
    pub environment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per minute
    #[serde(default = "default_rpm")]
    pub requests_per_minute: u32,
    /// Requests per hour
    #[serde(default = "default_rph")]
    pub requests_per_hour: u32,
    /// Requests per day
    #[serde(default = "default_rpd")]
    pub requests_per_day: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParams {
    /// Temperature (0.0 - 2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    /// Max tokens
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Top P sampling
    #[serde(default = "default_top_p")]
    pub top_p: f64,
    /// Frequency penalty
    #[serde(default = "default_frequency_penalty")]
    pub frequency_penalty: f64,
    /// Presence penalty
    #[serde(default = "default_presence_penalty")]
    pub presence_penalty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    ApiKey,
    OAuth,
    BearerToken,
    BasicAuth,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationStrategy {
    RoundRobin,
    Priority,
    UsageBased,
    ErrorBased,
    HealthBased,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationThreshold {
    UsagePercent(u8),
    ErrorRate(f64),
    TimeBased(Duration),
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionMethod {
    Aes256,
    ChaCha20,
    Rsa,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl Default for GlobalRotationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_strategy: default_global_strategy(),
            global_cooldown_secs: default_global_cooldown(),
            health_check_interval_secs: default_health_check_interval(),
        }
    }
}

impl Default for HealthMonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoints: Vec::new(),
            failure_threshold: default_failure_threshold(),
            success_rate_threshold: default_success_rate(),
        }
    }
}

impl Default for KeyStorageConfig {
    fn default() -> Self {
        Self {
            encrypt: true,
            encryption_method: default_encryption_method(),
            rotation_schedule: RotationSchedule::default(),
        }
    }
}

impl Default for UsageStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            error_rate: default_error_rate(),
            usage_percent: default_usage_percent(),
            last_update: default_timestamp(),
        }
    }
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            strategy: default_rotation_strategy(),
            threshold: RotationThreshold::UsagePercent(80),
            cooldown_secs: default_cooldown(),
        }
    }
}

impl Default for RotationThreshold {
    fn default() -> Self {
        Self::UsagePercent(80)
    }
}

impl Default for RotationSchedule {
    fn default() -> Self {
        Self {
            daily_rotation_time: default_daily_rotation_time(),
            weekly_rotation_day: default_weekly_rotation_day(),
            monthly_rotation_day: default_monthly_rotation_day(),
        }
    }
}

// Default functions

fn default_timeout() -> u64 {
    30
}
fn default_concurrent() -> usize {
    4
}
fn default_rotation_strategy() -> RotationStrategy {
    RotationStrategy::RoundRobin
}
fn default_cooldown() -> u64 {
    300
}
fn default_global_strategy() -> RotationStrategy {
    RotationStrategy::Priority
}
fn default_global_cooldown() -> u64 {
    600
}
fn default_health_check_interval() -> u64 {
    300
}
fn default_failure_threshold() -> u32 {
    5
}
fn default_success_rate() -> f64 {
    0.9
}
fn default_encryption_method() -> EncryptionMethod {
    EncryptionMethod::Aes256
}
fn default_daily_rotation_time() -> String {
    "02:00".to_string()
}
fn default_weekly_rotation_day() -> u32 {
    0
}
fn default_monthly_rotation_day() -> u32 {
    1
}
fn default_health_method() -> String {
    "GET".to_string()
}
fn default_expected_status() -> u16 {
    200
}
fn default_health_timeout() -> u64 {
    10
}
fn default_error_rate() -> f64 {
    0.0
}
fn default_usage_percent() -> u8 {
    0
}
fn default_timestamp() -> DateTime<Utc> {
    Utc::now()
}
fn default_environment() -> String {
    "production".to_string()
}
fn default_rpm() -> u32 {
    60
}
fn default_rph() -> u32 {
    3600
}
fn default_rpd() -> u32 {
    86400
}
fn default_temperature() -> f64 {
    0.7
}
fn default_max_tokens() -> u32 {
    4096
}
fn default_top_p() -> f64 {
    1.0
}
fn default_frequency_penalty() -> f64 {
    0.0
}
fn default_presence_penalty() -> f64 {
    0.0
}

fn default_provider_retries() -> u32 {
    2
}

fn default_provider_backoff_ms() -> u64 {
    500
}

fn default_channel_backoff_secs() -> u64 {
    2
}

fn default_channel_backoff_max_secs() -> u64 {
    60
}

fn default_scheduler_poll_secs() -> u64 {
    15
}

fn default_scheduler_retries() -> u32 {
    2
}

impl ReliabilityConfig {
    /// Load API keys from a KVM-style keys.json file.
    /// The file should have the format:
    /// ```json
    /// {
    ///   "provider_name": {
    ///     "keys": [{"key": "sk-xxx", ...}],
    ///     "activeIndex": 0
    ///   }
    /// }
    /// ```
    pub fn load_kvm_keys(&mut self) -> Result<()> {
        let Some(ref path) = self.kvm_keys_path else {
            return Ok(());
        };

        let expanded = shellexpand::tilde(path);
        let kvm_path = PathBuf::from(expanded.as_ref());

        if !kvm_path.exists() {
            tracing::debug!("KVM keys file not found at {}", kvm_path.display());
            return Ok(());
        }

        let contents = fs::read_to_string(&kvm_path)
            .with_context(|| format!("Failed to read KVM keys file: {}", kvm_path.display()))?;

        let kvm_data: serde_json::Value = serde_json::from_str(&contents)
            .with_context(|| "Failed to parse KVM keys file as JSON")?;

        let mut loaded_keys = Vec::new();

        if let Some(obj) = kvm_data.as_object() {
            for (provider_name, provider_data) in obj {
                if let Some(keys_array) = provider_data.get("keys").and_then(|k| k.as_array()) {
                    for key_entry in keys_array {
                        if let Some(key) = key_entry.get("key").and_then(|k| k.as_str()) {
                            if !key.is_empty() && !loaded_keys.contains(&key.to_string()) {
                                tracing::debug!(
                                    provider = provider_name,
                                    key_suffix = &key[key.len().saturating_sub(4)..],
                                    "Loaded API key from KVM"
                                );
                                loaded_keys.push(key.to_string());
                            }
                        }
                    }
                }
            }
        }

        if !loaded_keys.is_empty() {
            for key in loaded_keys {
                if !self.api_keys.contains(&key) {
                    self.api_keys.push(key);
                }
            }
            tracing::info!(
                count = self.api_keys.len(),
                "Loaded API keys from KVM for automatic rotation"
            );
        }

        Ok(())
    }
}

impl Default for ReliabilityConfig {
    fn default() -> Self {
        Self {
            providers: Vec::new(),
            rotation: GlobalRotationConfig::default(),
            health_monitoring: HealthMonitoringConfig::default(),
            key_storage: KeyStorageConfig::default(),
            provider_retries: default_provider_retries(),
            provider_backoff_ms: default_provider_backoff_ms(),
            fallback_providers: Vec::new(),
            auto_rotate_on_limit: true,
            kvm_keys_path: None,
            api_keys: Vec::new(),
            model_fallbacks: std::collections::HashMap::new(),
            channel_initial_backoff_secs: default_channel_backoff_secs(),
            channel_max_backoff_secs: default_channel_backoff_max_secs(),
            scheduler_poll_secs: default_scheduler_poll_secs(),
            scheduler_retries: default_scheduler_retries(),
        }
    }
}

// ── Scheduler ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Enable the built-in scheduler loop.
    #[serde(default = "default_scheduler_enabled")]
    pub enabled: bool,
    /// Maximum number of persisted scheduled tasks.
    #[serde(default = "default_scheduler_max_tasks")]
    pub max_tasks: usize,
    /// Maximum tasks executed per scheduler polling cycle.
    #[serde(default = "default_scheduler_max_concurrent")]
    pub max_concurrent: usize,
}

fn default_scheduler_enabled() -> bool {
    true
}

fn default_scheduler_max_tasks() -> usize {
    64
}

fn default_scheduler_max_concurrent() -> usize {
    4
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: default_scheduler_enabled(),
            max_tasks: default_scheduler_max_tasks(),
            max_concurrent: default_scheduler_max_concurrent(),
        }
    }
}

// ── Model routing ────────────────────────────────────────────────

/// Route a task hint to a specific provider + model.
///
/// ```toml
/// [[model_routes]]
/// hint = "reasoning"
/// provider = "openrouter"
/// model = "anthropic/claude-opus-4-20250514"
///
/// [[model_routes]]
/// hint = "fast"
/// provider = "groq"
/// model = "llama-3.3-70b-versatile"
/// ```
///
/// Usage: pass `hint:reasoning` as the model parameter to route the request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRouteConfig {
    /// Task hint name (e.g. "reasoning", "fast", "code", "summarize")
    pub hint: String,
    /// Provider to route to (must match a known provider name)
    pub provider: String,
    /// Model to use with that provider
    pub model: String,
    /// Optional API key override for this route's provider
    #[serde(default)]
    pub api_key: Option<String>,
}

// ── Heartbeat ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    pub enabled: bool,
    pub interval_minutes: u32,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_minutes: 30,
        }
    }
}

// ── Tunnel ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    /// "none", "cloudflare", "tailscale", "ngrok", "custom"
    pub provider: String,

    #[serde(default)]
    pub cloudflare: Option<CloudflareTunnelConfig>,

    #[serde(default)]
    pub tailscale: Option<TailscaleTunnelConfig>,

    #[serde(default)]
    pub ngrok: Option<NgrokTunnelConfig>,

    #[serde(default)]
    pub custom: Option<CustomTunnelConfig>,
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            provider: "none".into(),
            cloudflare: None,
            tailscale: None,
            ngrok: None,
            custom: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareTunnelConfig {
    /// Cloudflare Tunnel token (from Zero Trust dashboard)
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailscaleTunnelConfig {
    /// Use Tailscale Funnel (public internet) vs Serve (tailnet only)
    #[serde(default)]
    pub funnel: bool,
    /// Optional hostname override
    pub hostname: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NgrokTunnelConfig {
    /// ngrok auth token
    pub auth_token: String,
    /// Optional custom domain
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTunnelConfig {
    /// Command template to start the tunnel. Use {port} and {host} placeholders.
    /// Example: "bore local {port} --to bore.pub"
    pub start_command: String,
    /// Optional URL to check tunnel health
    pub health_url: Option<String>,
    /// Optional regex to extract public URL from command stdout
    pub url_pattern: Option<String>,
}

// ── Channels ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelsConfig {
    pub cli: bool,
    pub telegram: Option<TelegramConfig>,
    pub discord: Option<DiscordConfig>,
    pub slack: Option<SlackConfig>,
    pub webhook: Option<WebhookConfig>,
    pub imessage: Option<IMessageConfig>,
    pub matrix: Option<MatrixConfig>,
    pub whatsapp: Option<WhatsAppConfig>,
    pub email: Option<crate::channels::email_channel::EmailConfig>,
    pub irc: Option<IrcConfig>,
    pub lark: Option<LarkConfig>,
    pub dingtalk: Option<DingTalkConfig>,
    /// Message processing timeout in seconds
    #[serde(default = "default_channel_message_timeout_secs")]
    pub message_timeout_secs: u64,
    /// Parallel tasks per channel
    #[serde(default = "default_channel_parallelism")]
    pub parallelism_per_channel: usize,
    /// Minimum in-flight messages
    #[serde(default = "default_channel_min_in_flight")]
    pub min_in_flight_messages: usize,
    /// Maximum in-flight messages
    #[serde(default = "default_channel_max_in_flight")]
    pub max_in_flight_messages: usize,
    /// Maximum history turns per conversation
    #[serde(default = "default_channel_history_turns")]
    pub history_turns: usize,
    /// Bootstrap max chars for channel context
    #[serde(default = "default_channel_bootstrap_max_chars")]
    pub bootstrap_max_chars: usize,
}

fn default_channel_message_timeout_secs() -> u64 {
    300
}

fn default_channel_parallelism() -> usize {
    4
}

fn default_channel_min_in_flight() -> usize {
    8
}

fn default_channel_max_in_flight() -> usize {
    64
}

fn default_channel_history_turns() -> usize {
    40
}

fn default_channel_bootstrap_max_chars() -> usize {
    20_000
}

impl Default for ChannelsConfig {
    fn default() -> Self {
        Self {
            cli: true,
            telegram: None,
            discord: None,
            slack: None,
            webhook: None,
            imessage: None,
            matrix: None,
            whatsapp: None,
            email: None,
            irc: None,
            lark: None,
            dingtalk: None,
            message_timeout_secs: default_channel_message_timeout_secs(),
            parallelism_per_channel: default_channel_parallelism(),
            min_in_flight_messages: default_channel_min_in_flight(),
            max_in_flight_messages: default_channel_max_in_flight(),
            history_turns: default_channel_history_turns(),
            bootstrap_max_chars: default_channel_bootstrap_max_chars(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub allowed_users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub bot_token: String,
    pub guild_id: Option<String>,
    #[serde(default)]
    pub allowed_users: Vec<String>,
    /// When true, process messages from other bots (not just humans).
    /// The bot still ignores its own messages to prevent feedback loops.
    #[serde(default)]
    pub listen_to_bots: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub bot_token: String,
    pub app_token: Option<String>,
    pub channel_id: Option<String>,
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub port: u16,
    pub secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IMessageConfig {
    pub allowed_contacts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixConfig {
    pub homeserver: String,
    pub access_token: String,
    pub room_id: String,
    pub allowed_users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    /// Access token from Meta Business Suite
    pub access_token: String,
    /// Phone number ID from Meta Business API
    pub phone_number_id: String,
    /// Webhook verify token (you define this, Meta sends it back for verification)
    pub verify_token: String,
    /// App secret from Meta Business Suite (for webhook signature verification)
    /// Can also be set via `HOUSAKY_WHATSAPP_APP_SECRET` environment variable
    #[serde(default)]
    pub app_secret: Option<String>,
    /// Allowed phone numbers (E.164 format: +1234567890) or "*" for all
    #[serde(default)]
    pub allowed_numbers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrcConfig {
    /// IRC server hostname
    pub server: String,
    /// IRC server port (default: 6697 for TLS)
    #[serde(default = "default_irc_port")]
    pub port: u16,
    /// Bot nickname
    pub nickname: String,
    /// Username (defaults to nickname if not set)
    pub username: Option<String>,
    /// Channels to join on connect
    #[serde(default)]
    pub channels: Vec<String>,
    /// Allowed nicknames (case-insensitive) or "*" for all
    #[serde(default)]
    pub allowed_users: Vec<String>,
    /// Server password (for bouncers like ZNC)
    pub server_password: Option<String>,
    /// NickServ IDENTIFY password
    pub nickserv_password: Option<String>,
    /// SASL PLAIN password (IRCv3)
    pub sasl_password: Option<String>,
    /// Verify TLS certificate (default: true)
    pub verify_tls: Option<bool>,
}

fn default_irc_port() -> u16 {
    6697
}

/// Lark/Feishu configuration for messaging integration
/// Lark is the international version, Feishu is the Chinese version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LarkConfig {
    /// App ID from Lark/Feishu developer console
    pub app_id: String,
    /// App Secret from Lark/Feishu developer console
    pub app_secret: String,
    /// Encrypt key for webhook message decryption (optional)
    #[serde(default)]
    pub encrypt_key: Option<String>,
    /// Verification token for webhook validation (optional)
    #[serde(default)]
    pub verification_token: Option<String>,
    /// Allowed user IDs or union IDs (empty = deny all, "*" = allow all)
    #[serde(default)]
    pub allowed_users: Vec<String>,
    /// Whether to use the Feishu (Chinese) endpoint instead of Lark (International)
    #[serde(default)]
    pub use_feishu: bool,
}

// ── Security Config ─────────────────────────────────────────────────

/// Security configuration for sandboxing, resource limits, and audit logging
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    /// Sandbox configuration
    #[serde(default)]
    pub sandbox: SandboxConfig,

    /// Resource limits
    #[serde(default)]
    pub resources: ResourceLimitsConfig,

    /// Audit logging configuration
    #[serde(default)]
    pub audit: AuditConfig,
}

/// Sandbox configuration for OS-level isolation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Enable sandboxing (None = auto-detect, Some = explicit)
    #[serde(default)]
    pub enabled: Option<bool>,

    /// Sandbox backend to use
    #[serde(default)]
    pub backend: SandboxBackend,

    /// Custom Firejail arguments (when backend = firejail)
    #[serde(default)]
    pub firejail_args: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: None, // Auto-detect
            backend: SandboxBackend::Auto,
            firejail_args: Vec::new(),
        }
    }
}

/// Sandbox backend selection
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SandboxBackend {
    /// Auto-detect best available (default)
    #[default]
    Auto,
    /// Landlock (Linux kernel LSM, native)
    Landlock,
    /// Firejail (user-space sandbox)
    Firejail,
    /// Bubblewrap (user namespaces)
    Bubblewrap,
    /// Docker container isolation
    Docker,
    /// No sandboxing (application-layer only)
    None,
}

/// Resource limits for command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimitsConfig {
    /// Maximum memory in MB per command
    #[serde(default = "default_max_memory_mb")]
    pub max_memory_mb: u32,

    /// Maximum CPU time in seconds per command
    #[serde(default = "default_max_cpu_time_seconds")]
    pub max_cpu_time_seconds: u64,

    /// Maximum number of subprocesses
    #[serde(default = "default_max_subprocesses")]
    pub max_subprocesses: u32,

    /// Enable memory monitoring
    #[serde(default = "default_memory_monitoring_enabled")]
    pub memory_monitoring: bool,
}

fn default_max_memory_mb() -> u32 {
    512
}

fn default_max_cpu_time_seconds() -> u64 {
    60
}

fn default_max_subprocesses() -> u32 {
    10
}

fn default_memory_monitoring_enabled() -> bool {
    true
}

impl Default for ResourceLimitsConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: default_max_memory_mb(),
            max_cpu_time_seconds: default_max_cpu_time_seconds(),
            max_subprocesses: default_max_subprocesses(),
            memory_monitoring: default_memory_monitoring_enabled(),
        }
    }
}

/// Audit logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable audit logging
    #[serde(default = "default_audit_enabled")]
    pub enabled: bool,

    /// Path to audit log file (relative to housaky dir)
    #[serde(default = "default_audit_log_path")]
    pub log_path: String,

    /// Maximum log size in MB before rotation
    #[serde(default = "default_audit_max_size_mb")]
    pub max_size_mb: u32,

    /// Sign events with HMAC for tamper evidence
    #[serde(default)]
    pub sign_events: bool,
}

fn default_audit_enabled() -> bool {
    true
}

fn default_audit_log_path() -> String {
    "audit.log".to_string()
}

fn default_audit_max_size_mb() -> u32 {
    100
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: default_audit_enabled(),
            log_path: default_audit_log_path(),
            max_size_mb: default_audit_max_size_mb(),
            sign_events: false,
        }
    }
}

/// DingTalk (钉钉) configuration for Stream Mode messaging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkConfig {
    /// Client ID (AppKey) from DingTalk developer console
    pub client_id: String,
    /// Client Secret (AppSecret) from DingTalk developer console
    pub client_secret: String,
    /// Allowed user IDs (staff IDs). Empty = deny all, "*" = allow all
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

// ── Config impl ──────────────────────────────────────────────────

impl Default for Config {
    fn default() -> Self {
        let home =
            UserDirs::new().map_or_else(|| PathBuf::from("."), |u| u.home_dir().to_path_buf());
        let housaky_dir = home.join(".housaky");

        Self {
            workspace_dir: housaky_dir.join("workspace"),
            config_path: housaky_dir.join("config.toml"),
            api_key: None,
            default_provider: Some("openrouter".to_string()),
            default_model: Some("arcee-ai/trinity-large-preview:free".to_string()),
            default_temperature: 0.7,
            observability: ObservabilityConfig::default(),
            autonomy: AutonomyConfig::default(),
            runtime: RuntimeConfig::default(),
            reliability: ReliabilityConfig::default(),
            scheduler: SchedulerConfig::default(),
            agent: AgentConfig::default(),
            tools: ToolConfig::default(),
            fallback: FallbackConfig::default(),
            model_routes: Vec::new(),
            heartbeat: HeartbeatConfig::default(),
            channels_config: ChannelsConfig::default(),
            memory: MemoryConfig::default(),
            tunnel: TunnelConfig::default(),
            gateway: GatewayConfig::default(),
            composio: ComposioConfig::default(),
            secrets: SecretsConfig::default(),
            browser: BrowserConfig::default(),
            http_request: HttpRequestConfig::default(),
            identity: IdentityConfig::default(),
            cost: CostConfig::default(),
            peripherals: PeripheralsConfig::default(),
            agents: HashMap::new(),
            hardware: HardwareConfig::default(),
            agi_enabled: true,
        }
    }
}

impl Config {
    pub fn load_or_init() -> Result<Self> {
        let home = UserDirs::new()
            .map(|u| u.home_dir().to_path_buf())
            .context("Could not find home directory")?;
        let housaky_dir = home.join(".housaky");
        let config_path = housaky_dir.join("config.toml");

        if !housaky_dir.exists() {
            fs::create_dir_all(&housaky_dir).context("Failed to create .housaky directory")?;
            fs::create_dir_all(housaky_dir.join("workspace"))
                .context("Failed to create workspace directory")?;
        }

        if config_path.exists() {
            let contents =
                fs::read_to_string(&config_path).context("Failed to read config file")?;
            let mut config: Config =
                toml::from_str(&contents).context("Failed to parse config file")?;
            // Set computed paths that are skipped during serialization
            config.config_path = config_path.clone();
            config.workspace_dir = housaky_dir.join("workspace");
            // Load KVM keys if configured and auto-rotate is enabled
            if config.reliability.auto_rotate_on_limit {
                if let Err(e) = config.reliability.load_kvm_keys() {
                    tracing::warn!("Failed to load KVM keys: {e}");
                }
            }
            config.apply_env_overrides();
            config.apply_vkm_config();
            Ok(config)
        } else {
            let mut config = Config::default();
            config.config_path = config_path.clone();
            config.workspace_dir = housaky_dir.join("workspace");
            config.save()?;
            config.apply_env_overrides();
            config.apply_vkm_config();
            Ok(config)
        }
    }

    /// Apply VKM (Virtual Key Manager) configuration for dynamic API key rotation
    pub fn apply_vkm_config(&mut self) {
        if let Some(vkm_config) = crate::vkm_client::get_vkm_config() {
            tracing::info!(
                provider = vkm_config.provider.as_deref().unwrap_or("unknown"),
                model = vkm_config.model.as_deref().unwrap_or("unknown"),
                "Loaded active config from VKM"
            );

            if !vkm_config.api_key.is_empty() {
                self.api_key = Some(vkm_config.api_key);
            }

            let base_url_opt: &Option<String> = &vkm_config.base_url;
            if let Some(base_url) = base_url_opt {
                if !base_url.is_empty() {
                    self.default_provider = Some(format!("custom:{}", base_url));
                }
            }

            let model_opt: &Option<String> = &vkm_config.model;
            if let Some(model) = model_opt {
                if !model.is_empty() {
                    self.default_model = Some(model.clone());
                }
            }
        }
    }

    /// Apply environment variable overrides to config
    pub fn apply_env_overrides(&mut self) {
        // API Key: HOUSAKY_API_KEY or API_KEY (generic)
        if let Ok(key) = std::env::var("HOUSAKY_API_KEY").or_else(|_| std::env::var("API_KEY")) {
            if !key.is_empty() {
                self.api_key = Some(key);
            }
        }
        // Provider-specific API keys
        let provider = self.default_provider.as_deref().unwrap_or("");
        if self.api_key.is_none() {
            match provider {
                "openrouter" | "open_router" => {
                    if let Ok(key) = std::env::var("OPENROUTER_API_KEY") {
                        if !key.is_empty() {
                            self.api_key = Some(key);
                        }
                    }
                }
                "openai" => {
                    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
                        if !key.is_empty() {
                            self.api_key = Some(key);
                        }
                    }
                }
                "anthropic" => {
                    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
                        if !key.is_empty() {
                            self.api_key = Some(key);
                        }
                    }
                }
                "glm" | "zhipu" => {
                    if let Ok(key) = std::env::var("GLM_API_KEY") {
                        if !key.is_empty() {
                            self.api_key = Some(key);
                        }
                    }
                }
                "groq" => {
                    if let Ok(key) = std::env::var("GROQ_API_KEY") {
                        if !key.is_empty() {
                            self.api_key = Some(key);
                        }
                    }
                }
                "mistral" => {
                    if let Ok(key) = std::env::var("MISTRAL_API_KEY") {
                        if !key.is_empty() {
                            self.api_key = Some(key);
                        }
                    }
                }
                "deepseek" => {
                    if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
                        if !key.is_empty() {
                            self.api_key = Some(key);
                        }
                    }
                }
                "xai" | "grok" => {
                    if let Ok(key) = std::env::var("XAI_API_KEY") {
                        if !key.is_empty() {
                            self.api_key = Some(key);
                        }
                    }
                }
                _ => {}
            }
        }
        // GLM specific check
        if self.default_provider.as_deref() == Some("glm")
            || self.default_provider.as_deref() == Some("zhipu")
        {
            if let Ok(key) = std::env::var("GLM_API_KEY") {
                if !key.is_empty() {
                    self.api_key = Some(key);
                }
            }
        }

        // Provider: HOUSAKY_PROVIDER or PROVIDER
        if let Ok(provider) =
            std::env::var("HOUSAKY_PROVIDER").or_else(|_| std::env::var("PROVIDER"))
        {
            if !provider.is_empty() {
                self.default_provider = Some(provider);
            }
        }

        // Model: HOUSAKY_MODEL
        if let Ok(model) = std::env::var("HOUSAKY_MODEL") {
            if !model.is_empty() {
                self.default_model = Some(model);
            }
        }

        // Workspace directory: HOUSAKY_WORKSPACE
        if let Ok(workspace) = std::env::var("HOUSAKY_WORKSPACE") {
            if !workspace.is_empty() {
                self.workspace_dir = PathBuf::from(workspace);
            }
        }

        // Gateway port: HOUSAKY_GATEWAY_PORT or PORT
        if let Ok(port_str) =
            std::env::var("HOUSAKY_GATEWAY_PORT").or_else(|_| std::env::var("PORT"))
        {
            if let Ok(port) = port_str.parse::<u16>() {
                self.gateway.port = port;
            }
        }

        // Gateway host: HOUSAKY_GATEWAY_HOST or HOST
        if let Ok(host) = std::env::var("HOUSAKY_GATEWAY_HOST").or_else(|_| std::env::var("HOST")) {
            if !host.is_empty() {
                self.gateway.host = host;
            }
        }

        // Allow public bind: HOUSAKY_ALLOW_PUBLIC_BIND
        if let Ok(val) = std::env::var("HOUSAKY_ALLOW_PUBLIC_BIND") {
            self.gateway.allow_public_bind = val == "1" || val.eq_ignore_ascii_case("true");
        }

        // Temperature: HOUSAKY_TEMPERATURE
        if let Ok(temp_str) = std::env::var("HOUSAKY_TEMPERATURE") {
            if let Ok(temp) = temp_str.parse::<f64>() {
                if (0.0..=2.0).contains(&temp) {
                    self.default_temperature = temp;
                }
            }
        }
    }

    pub fn save(&self) -> Result<()> {
        // Encrypt agent API keys before serialization
        let mut config_to_save = self.clone();
        let housaky_dir = self
            .config_path
            .parent()
            .context("Config path must have a parent directory")?;
        let store = crate::security::SecretStore::new(housaky_dir, self.secrets.encrypt);
        for agent in config_to_save.agents.values_mut() {
            if let Some(ref plaintext_key) = agent.api_key {
                if !crate::security::SecretStore::is_encrypted(plaintext_key) {
                    agent.api_key = Some(
                        store
                            .encrypt(plaintext_key)
                            .context("Failed to encrypt agent API key")?,
                    );
                }
            }
        }

        let toml_str =
            toml::to_string_pretty(&config_to_save).context("Failed to serialize config")?;

        let parent_dir = self
            .config_path
            .parent()
            .context("Config path must have a parent directory")?;
        fs::create_dir_all(parent_dir).with_context(|| {
            format!(
                "Failed to create config directory: {}",
                parent_dir.display()
            )
        })?;

        let file_name = self
            .config_path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("config.toml");
        let temp_path = parent_dir.join(format!(".{file_name}.tmp-{}", uuid::Uuid::new_v4()));
        let backup_path = parent_dir.join(format!("{file_name}.bak"));

        let mut temp_file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)
            .with_context(|| {
                format!(
                    "Failed to create temporary config file: {}",
                    temp_path.display()
                )
            })?;
        temp_file
            .write_all(toml_str.as_bytes())
            .context("Failed to write temporary config contents")?;
        temp_file
            .sync_all()
            .context("Failed to fsync temporary config file")?;
        drop(temp_file);

        let had_existing_config = self.config_path.exists();
        if had_existing_config {
            fs::copy(&self.config_path, &backup_path).with_context(|| {
                format!(
                    "Failed to create config backup before atomic replace: {}",
                    backup_path.display()
                )
            })?;
        }

        if let Err(e) = fs::rename(&temp_path, &self.config_path) {
            let _ = fs::remove_file(&temp_path);
            if had_existing_config && backup_path.exists() {
                let _ = fs::copy(&backup_path, &self.config_path);
            }
            anyhow::bail!("Failed to atomically replace config file: {e}");
        }

        sync_directory(parent_dir)?;

        if had_existing_config {
            let _ = fs::remove_file(&backup_path);
        }

        Ok(())
    }
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<()> {
    let dir = File::open(path)
        .with_context(|| format!("Failed to open directory for fsync: {}", path.display()))?;
    dir.sync_all()
        .with_context(|| format!("Failed to fsync directory metadata: {}", path.display()))?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── Defaults ─────────────────────────────────────────────

    #[test]
    fn config_default_has_sane_values() {
        let c = Config::default();
        assert_eq!(c.default_provider.as_deref(), Some("openrouter"));
        assert!(c.default_model.as_deref().is_some());
        assert!((c.default_temperature - 0.7).abs() < f64::EPSILON);
        assert!(c.api_key.is_none());
        assert!(c.workspace_dir.to_string_lossy().contains("workspace"));
        assert!(c.config_path.to_string_lossy().contains("config.toml"));
    }

    #[test]
    fn observability_config_default() {
        let o = ObservabilityConfig::default();
        assert_eq!(o.backend, "none");
    }

    #[test]
    fn autonomy_config_default() {
        let a = AutonomyConfig::default();
        assert_eq!(a.level, AutonomyLevel::Supervised);
        assert!(a.workspace_only);
        assert!(a.allowed_commands.contains(&"git".to_string()));
        assert!(a.allowed_commands.contains(&"cargo".to_string()));
        assert!(a.forbidden_paths.contains(&"/etc".to_string()));
        assert_eq!(a.max_actions_per_hour, 20);
        assert_eq!(a.max_cost_per_day_cents, 500);
        assert!(a.require_approval_for_medium_risk);
        assert!(a.block_high_risk_commands);
    }

    #[test]
    fn runtime_config_default() {
        let r = RuntimeConfig::default();
        assert_eq!(r.kind, "native");
        assert_eq!(r.docker.image, "alpine:3.20");
        assert_eq!(r.docker.network, "none");
        assert_eq!(r.docker.memory_limit_mb, Some(512));
        assert_eq!(r.docker.cpu_limit, Some(1.0));
        assert!(r.docker.read_only_rootfs);
        assert!(r.docker.mount_workspace);
    }

    #[test]
    fn heartbeat_config_default() {
        let h = HeartbeatConfig::default();
        assert!(h.enabled);
        assert_eq!(h.interval_minutes, 30);
    }

    #[test]
    fn memory_config_default_hygiene_settings() {
        let m = MemoryConfig::default();
        assert_eq!(m.backend, "sqlite");
        assert!(m.auto_save);
        assert!(m.hygiene_enabled);
        assert_eq!(m.archive_after_days, 7);
        assert_eq!(m.purge_after_days, 30);
        assert_eq!(m.conversation_retention_days, 30);
    }

    #[test]
    fn channels_config_default() {
        let c = ChannelsConfig::default();
        assert!(c.cli);
        assert!(c.telegram.is_none());
        assert!(c.discord.is_none());
    }

    // ── Serde round-trip ─────────────────────────────────────

    #[test]
    fn config_toml_roundtrip() {
        let config = Config {
            workspace_dir: PathBuf::from("/tmp/test/workspace"),
            config_path: PathBuf::from("/tmp/test/config.toml"),
            api_key: Some("sk-test-key".into()),
            default_provider: Some("openrouter".into()),
            default_model: Some("gpt-4o".into()),
            default_temperature: 0.5,
            observability: ObservabilityConfig {
                backend: "log".into(),
                ..ObservabilityConfig::default()
            },
            autonomy: AutonomyConfig {
                level: AutonomyLevel::Full,
                workspace_only: false,
                allowed_commands: vec!["docker".into()],
                forbidden_paths: vec!["/secret".into()],
                max_actions_per_hour: 50,
                max_cost_per_day_cents: 1000,
                require_approval_for_medium_risk: false,
                block_high_risk_commands: true,
            },
            runtime: RuntimeConfig {
                kind: "docker".into(),
                ..RuntimeConfig::default()
            },
            reliability: ReliabilityConfig::default(),
            scheduler: SchedulerConfig::default(),
            model_routes: Vec::new(),
            heartbeat: HeartbeatConfig {
                enabled: true,
                interval_minutes: 15,
            },
            channels_config: ChannelsConfig {
                cli: true,
                telegram: Some(TelegramConfig {
                    bot_token: "123:ABC".into(),
                    allowed_users: vec!["user1".into()],
                }),
                discord: None,
                slack: None,
                webhook: None,
                imessage: None,
                matrix: None,
                whatsapp: None,
                email: None,
                irc: None,
                lark: None,
                dingtalk: None,
                message_timeout_secs: 300,
                parallelism_per_channel: 4,
                min_in_flight_messages: 8,
                max_in_flight_messages: 64,
                history_turns: 40,
                bootstrap_max_chars: 20_000,
            },
            memory: MemoryConfig::default(),
            tunnel: TunnelConfig::default(),
            gateway: GatewayConfig::default(),
            composio: ComposioConfig::default(),
            secrets: SecretsConfig::default(),
            browser: BrowserConfig::default(),
            http_request: HttpRequestConfig::default(),
            agent: AgentConfig::default(),
            tools: ToolConfig::default(),
            fallback: FallbackConfig::default(),
            identity: IdentityConfig::default(),
            cost: CostConfig::default(),
            peripherals: PeripheralsConfig::default(),
            agents: HashMap::new(),
            hardware: HardwareConfig::default(),
            agi_enabled: true,
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.api_key, config.api_key);
        assert_eq!(parsed.default_provider, config.default_provider);
        assert_eq!(parsed.default_model, config.default_model);
        assert!((parsed.default_temperature - config.default_temperature).abs() < f64::EPSILON);
        assert_eq!(parsed.observability.backend, "log");
        assert_eq!(parsed.autonomy.level, AutonomyLevel::Full);
        assert!(!parsed.autonomy.workspace_only);
        assert_eq!(parsed.runtime.kind, "docker");
        assert!(parsed.heartbeat.enabled);
        assert_eq!(parsed.heartbeat.interval_minutes, 15);
        assert!(parsed.channels_config.telegram.is_some());
        assert_eq!(
            parsed.channels_config.telegram.unwrap().bot_token,
            "123:ABC"
        );
    }

    #[test]
    fn config_minimal_toml_uses_defaults() {
        let minimal = r#"
workspace_dir = "/tmp/ws"
config_path = "/tmp/config.toml"
default_temperature = 0.7
"#;
        let parsed: Config = toml::from_str(minimal).unwrap();
        assert!(parsed.api_key.is_none());
        assert!(parsed.default_provider.is_none());
        assert_eq!(parsed.observability.backend, "none");
        assert_eq!(parsed.autonomy.level, AutonomyLevel::Supervised);
        assert_eq!(parsed.runtime.kind, "native");
        assert!(parsed.heartbeat.enabled);
        assert!(parsed.channels_config.cli);
        assert!(parsed.memory.hygiene_enabled);
        assert_eq!(parsed.memory.archive_after_days, 7);
        assert_eq!(parsed.memory.purge_after_days, 30);
        assert_eq!(parsed.memory.conversation_retention_days, 30);
    }

    #[test]
    fn agent_config_defaults() {
        let cfg = AgentConfig::default();
        assert!(!cfg.compact_context);
        assert_eq!(cfg.max_tool_iterations, usize::MAX);
        assert_eq!(cfg.max_history_messages, 50);
        assert!(!cfg.parallel_tools);
        assert_eq!(cfg.tool_dispatcher, "auto");
        assert_eq!(cfg.compaction_keep_recent_messages, 20);
        assert_eq!(cfg.compaction_max_source_chars, 12_000);
        assert_eq!(cfg.compaction_max_summary_chars, 2_000);
        assert_eq!(cfg.bootstrap_max_chars, 20_000);
    }

    #[test]
    fn tool_config_defaults() {
        let cfg = ToolConfig::default();
        assert_eq!(cfg.shell_timeout_secs, 60);
        assert_eq!(cfg.shell_max_output_bytes, 1_048_576);
        assert_eq!(cfg.file_read_max_bytes, 10_485_760);
        assert_eq!(cfg.delegate_timeout_secs, 120);
        assert_eq!(cfg.screenshot_timeout_secs, 15);
        assert_eq!(cfg.screenshot_max_bytes, 2_097_152);
        assert_eq!(cfg.image_max_bytes, 5_242_880);
    }

    #[test]
    fn fallback_config_defaults() {
        let cfg = FallbackConfig::default();
        assert!(cfg.enabled);
        assert!(cfg.providers.is_empty());
        assert_eq!(cfg.rotate_at_percent, 80);
        assert!(cfg.rotate_on_rate_limit);
    }

    #[test]
    fn gateway_config_new_fields() {
        let cfg = GatewayConfig::default();
        assert_eq!(cfg.max_body_size, 65_536);
        assert_eq!(cfg.request_timeout_secs, 30);
        assert_eq!(cfg.rate_limit_window_secs, 60);
        assert_eq!(cfg.pairing_max_attempts, 5);
        assert_eq!(cfg.pairing_lockout_secs, 300);
        assert_eq!(cfg.rate_limiter_sweep_secs, 300);
    }

    #[test]
    fn channels_config_new_fields() {
        let cfg = ChannelsConfig::default();
        assert_eq!(cfg.message_timeout_secs, 300);
        assert_eq!(cfg.parallelism_per_channel, 4);
        assert_eq!(cfg.min_in_flight_messages, 8);
        assert_eq!(cfg.max_in_flight_messages, 64);
        assert_eq!(cfg.history_turns, 40);
        assert_eq!(cfg.bootstrap_max_chars, 20_000);
    }

    #[test]
    fn agent_config_deserializes() {
        let raw = r#"
default_temperature = 0.7
[agent]
compact_context = true
max_tool_iterations = 20
max_history_messages = 80
parallel_tools = true
tool_dispatcher = "xml"
"#;
        let parsed: Config = toml::from_str(raw).unwrap();
        assert!(parsed.agent.compact_context);
        assert_eq!(parsed.agent.max_tool_iterations, 20);
        assert_eq!(parsed.agent.max_history_messages, 80);
        assert!(parsed.agent.parallel_tools);
        assert_eq!(parsed.agent.tool_dispatcher, "xml");
    }

    #[test]
    fn config_save_and_load_tmpdir() {
        let dir = std::env::temp_dir().join("housaky_test_config");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let config_path = dir.join("config.toml");
        let config = Config {
            workspace_dir: dir.join("workspace"),
            config_path: config_path.clone(),
            api_key: Some("sk-roundtrip".into()),
            default_provider: Some("openrouter".into()),
            default_model: Some("test-model".into()),
            default_temperature: 0.9,
            observability: ObservabilityConfig::default(),
            autonomy: AutonomyConfig::default(),
            runtime: RuntimeConfig::default(),
            reliability: ReliabilityConfig::default(),
            scheduler: SchedulerConfig::default(),
            model_routes: Vec::new(),
            heartbeat: HeartbeatConfig::default(),
            channels_config: ChannelsConfig::default(),
            memory: MemoryConfig::default(),
            tunnel: TunnelConfig::default(),
            gateway: GatewayConfig::default(),
            composio: ComposioConfig::default(),
            secrets: SecretsConfig::default(),
            browser: BrowserConfig::default(),
            http_request: HttpRequestConfig::default(),
            agent: AgentConfig::default(),
            tools: ToolConfig::default(),
            fallback: FallbackConfig::default(),
            identity: IdentityConfig::default(),
            cost: CostConfig::default(),
            peripherals: PeripheralsConfig::default(),
            agents: HashMap::new(),
            hardware: HardwareConfig::default(),
            agi_enabled: true,
        };

        config.save().unwrap();
        assert!(config_path.exists());

        let contents = fs::read_to_string(&config_path).unwrap();
        let loaded: Config = toml::from_str(&contents).unwrap();
        assert_eq!(loaded.api_key.as_deref(), Some("sk-roundtrip"));
        assert_eq!(loaded.default_model.as_deref(), Some("test-model"));
        assert!((loaded.default_temperature - 0.9).abs() < f64::EPSILON);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_save_atomic_cleanup() {
        let dir =
            std::env::temp_dir().join(format!("housaky_test_config_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();

        let config_path = dir.join("config.toml");
        let mut config = Config::default();
        config.workspace_dir = dir.join("workspace");
        config.config_path = config_path.clone();
        config.default_model = Some("model-a".into());

        config.save().unwrap();
        assert!(config_path.exists());

        config.default_model = Some("model-b".into());
        config.save().unwrap();

        let contents = fs::read_to_string(&config_path).unwrap();
        assert!(contents.contains("model-b"));

        let names: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert!(!names.iter().any(|name| name.contains(".tmp-")));
        assert!(!names.iter().any(|name| name.ends_with(".bak")));

        let _ = fs::remove_dir_all(&dir);
    }

    // ── Telegram / Discord config ────────────────────────────

    #[test]
    fn telegram_config_serde() {
        let tc = TelegramConfig {
            bot_token: "123:XYZ".into(),
            allowed_users: vec!["alice".into(), "bob".into()],
        };
        let json = serde_json::to_string(&tc).unwrap();
        let parsed: TelegramConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.bot_token, "123:XYZ");
        assert_eq!(parsed.allowed_users.len(), 2);
    }

    #[test]
    fn discord_config_serde() {
        let dc = DiscordConfig {
            bot_token: "discord-token".into(),
            guild_id: Some("12345".into()),
            allowed_users: vec![],
            listen_to_bots: false,
        };
        let json = serde_json::to_string(&dc).unwrap();
        let parsed: DiscordConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.bot_token, "discord-token");
        assert_eq!(parsed.guild_id.as_deref(), Some("12345"));
    }

    #[test]
    fn discord_config_optional_guild() {
        let dc = DiscordConfig {
            bot_token: "tok".into(),
            guild_id: None,
            allowed_users: vec![],
            listen_to_bots: false,
        };
        let json = serde_json::to_string(&dc).unwrap();
        let parsed: DiscordConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.guild_id.is_none());
    }

    // ── iMessage / Matrix config ────────────────────────────

    #[test]
    fn imessage_config_serde() {
        let ic = IMessageConfig {
            allowed_contacts: vec!["+1234567890".into(), "user@icloud.com".into()],
        };
        let json = serde_json::to_string(&ic).unwrap();
        let parsed: IMessageConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.allowed_contacts.len(), 2);
        assert_eq!(parsed.allowed_contacts[0], "+1234567890");
    }

    #[test]
    fn imessage_config_empty_contacts() {
        let ic = IMessageConfig {
            allowed_contacts: vec![],
        };
        let json = serde_json::to_string(&ic).unwrap();
        let parsed: IMessageConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.allowed_contacts.is_empty());
    }

    #[test]
    fn imessage_config_wildcard() {
        let ic = IMessageConfig {
            allowed_contacts: vec!["*".into()],
        };
        let toml_str = toml::to_string(&ic).unwrap();
        let parsed: IMessageConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.allowed_contacts, vec!["*"]);
    }

    #[test]
    fn matrix_config_serde() {
        let mc = MatrixConfig {
            homeserver: "https://matrix.org".into(),
            access_token: "syt_token_abc".into(),
            room_id: "!room123:matrix.org".into(),
            allowed_users: vec!["@user:matrix.org".into()],
        };
        let json = serde_json::to_string(&mc).unwrap();
        let parsed: MatrixConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.homeserver, "https://matrix.org");
        assert_eq!(parsed.access_token, "syt_token_abc");
        assert_eq!(parsed.room_id, "!room123:matrix.org");
        assert_eq!(parsed.allowed_users.len(), 1);
    }

    #[test]
    fn matrix_config_toml_roundtrip() {
        let mc = MatrixConfig {
            homeserver: "https://synapse.local:8448".into(),
            access_token: "tok".into(),
            room_id: "!abc:synapse.local".into(),
            allowed_users: vec!["@admin:synapse.local".into(), "*".into()],
        };
        let toml_str = toml::to_string(&mc).unwrap();
        let parsed: MatrixConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.homeserver, "https://synapse.local:8448");
        assert_eq!(parsed.allowed_users.len(), 2);
    }

    #[test]
    fn channels_config_with_imessage_and_matrix() {
        let c = ChannelsConfig {
            cli: true,
            telegram: None,
            discord: None,
            slack: None,
            webhook: None,
            imessage: Some(IMessageConfig {
                allowed_contacts: vec!["+1".into()],
            }),
            matrix: Some(MatrixConfig {
                homeserver: "https://m.org".into(),
                access_token: "tok".into(),
                room_id: "!r:m".into(),
                allowed_users: vec!["@u:m".into()],
            }),
            whatsapp: None,
            email: None,
            irc: None,
            lark: None,
            dingtalk: None,
            message_timeout_secs: 300,
            parallelism_per_channel: 4,
            min_in_flight_messages: 8,
            max_in_flight_messages: 64,
            history_turns: 40,
            bootstrap_max_chars: 20_000,
        };
        let toml_str = toml::to_string_pretty(&c).unwrap();
        let parsed: ChannelsConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.imessage.is_some());
        assert!(parsed.matrix.is_some());
        assert_eq!(parsed.imessage.unwrap().allowed_contacts, vec!["+1"]);
        assert_eq!(parsed.matrix.unwrap().homeserver, "https://m.org");
    }

    #[test]
    fn channels_config_default_has_no_imessage_matrix() {
        let c = ChannelsConfig::default();
        assert!(c.imessage.is_none());
        assert!(c.matrix.is_none());
    }

    // ── Edge cases: serde(default) for allowed_users ─────────

    #[test]
    fn discord_config_deserializes_without_allowed_users() {
        // Old configs won't have allowed_users — serde(default) should fill vec![]
        let json = r#"{"bot_token":"tok","guild_id":"123"}"#;
        let parsed: DiscordConfig = serde_json::from_str(json).unwrap();
        assert!(parsed.allowed_users.is_empty());
    }

    #[test]
    fn discord_config_deserializes_with_allowed_users() {
        let json = r#"{"bot_token":"tok","guild_id":"123","allowed_users":["111","222"]}"#;
        let parsed: DiscordConfig = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.allowed_users, vec!["111", "222"]);
    }

    #[test]
    fn slack_config_deserializes_without_allowed_users() {
        let json = r#"{"bot_token":"xoxb-tok"}"#;
        let parsed: SlackConfig = serde_json::from_str(json).unwrap();
        assert!(parsed.allowed_users.is_empty());
    }

    #[test]
    fn slack_config_deserializes_with_allowed_users() {
        let json = r#"{"bot_token":"xoxb-tok","allowed_users":["U111"]}"#;
        let parsed: SlackConfig = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.allowed_users, vec!["U111"]);
    }

    #[test]
    fn discord_config_toml_backward_compat() {
        let toml_str = r#"
bot_token = "tok"
guild_id = "123"
"#;
        let parsed: DiscordConfig = toml::from_str(toml_str).unwrap();
        assert!(parsed.allowed_users.is_empty());
        assert_eq!(parsed.bot_token, "tok");
    }

    #[test]
    fn slack_config_toml_backward_compat() {
        let toml_str = r#"
bot_token = "xoxb-tok"
channel_id = "C123"
"#;
        let parsed: SlackConfig = toml::from_str(toml_str).unwrap();
        assert!(parsed.allowed_users.is_empty());
        assert_eq!(parsed.channel_id.as_deref(), Some("C123"));
    }

    #[test]
    fn webhook_config_with_secret() {
        let json = r#"{"port":8080,"secret":"my-secret-key"}"#;
        let parsed: WebhookConfig = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.secret.as_deref(), Some("my-secret-key"));
    }

    #[test]
    fn webhook_config_without_secret() {
        let json = r#"{"port":8080}"#;
        let parsed: WebhookConfig = serde_json::from_str(json).unwrap();
        assert!(parsed.secret.is_none());
        assert_eq!(parsed.port, 8080);
    }

    // ── WhatsApp config ──────────────────────────────────────

    #[test]
    fn whatsapp_config_serde() {
        let wc = WhatsAppConfig {
            access_token: "EAABx...".into(),
            phone_number_id: "123456789".into(),
            verify_token: "my-verify-token".into(),
            app_secret: None,
            allowed_numbers: vec!["+1234567890".into(), "+9876543210".into()],
        };
        let json = serde_json::to_string(&wc).unwrap();
        let parsed: WhatsAppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.access_token, "EAABx...");
        assert_eq!(parsed.phone_number_id, "123456789");
        assert_eq!(parsed.verify_token, "my-verify-token");
        assert_eq!(parsed.allowed_numbers.len(), 2);
    }

    #[test]
    fn whatsapp_config_toml_roundtrip() {
        let wc = WhatsAppConfig {
            access_token: "tok".into(),
            phone_number_id: "12345".into(),
            verify_token: "verify".into(),
            app_secret: Some("secret123".into()),
            allowed_numbers: vec!["+1".into()],
        };
        let toml_str = toml::to_string(&wc).unwrap();
        let parsed: WhatsAppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.phone_number_id, "12345");
        assert_eq!(parsed.allowed_numbers, vec!["+1"]);
    }

    #[test]
    fn whatsapp_config_deserializes_without_allowed_numbers() {
        let json = r#"{"access_token":"tok","phone_number_id":"123","verify_token":"ver"}"#;
        let parsed: WhatsAppConfig = serde_json::from_str(json).unwrap();
        assert!(parsed.allowed_numbers.is_empty());
    }

    #[test]
    fn whatsapp_config_wildcard_allowed() {
        let wc = WhatsAppConfig {
            access_token: "tok".into(),
            phone_number_id: "123".into(),
            verify_token: "ver".into(),
            app_secret: None,
            allowed_numbers: vec!["*".into()],
        };
        let toml_str = toml::to_string(&wc).unwrap();
        let parsed: WhatsAppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.allowed_numbers, vec!["*"]);
    }

    #[test]
    fn channels_config_with_whatsapp() {
        let c = ChannelsConfig {
            cli: true,
            telegram: None,
            discord: None,
            slack: None,
            webhook: None,
            imessage: None,
            matrix: None,
            whatsapp: Some(WhatsAppConfig {
                access_token: "tok".into(),
                phone_number_id: "123".into(),
                verify_token: "ver".into(),
                app_secret: None,
                allowed_numbers: vec!["+1".into()],
            }),
            email: None,
            irc: None,
            lark: None,
            dingtalk: None,
            message_timeout_secs: 300,
            parallelism_per_channel: 4,
            min_in_flight_messages: 8,
            max_in_flight_messages: 64,
            history_turns: 40,
            bootstrap_max_chars: 20_000,
        };
        let toml_str = toml::to_string_pretty(&c).unwrap();
        let parsed: ChannelsConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.whatsapp.is_some());
        let wa = parsed.whatsapp.unwrap();
        assert_eq!(wa.phone_number_id, "123");
        assert_eq!(wa.allowed_numbers, vec!["+1"]);
    }

    #[test]
    fn channels_config_default_has_no_whatsapp() {
        let c = ChannelsConfig::default();
        assert!(c.whatsapp.is_none());
    }

    // ══════════════════════════════════════════════════════════
    // SECURITY CHECKLIST TESTS — Gateway config
    // ══════════════════════════════════════════════════════════

    #[test]
    fn checklist_gateway_default_requires_pairing() {
        let g = GatewayConfig::default();
        assert!(g.require_pairing, "Pairing must be required by default");
    }

    #[test]
    fn checklist_gateway_default_blocks_public_bind() {
        let g = GatewayConfig::default();
        assert!(
            !g.allow_public_bind,
            "Public bind must be blocked by default"
        );
    }

    #[test]
    fn checklist_gateway_default_no_tokens() {
        let g = GatewayConfig::default();
        assert!(
            g.paired_tokens.is_empty(),
            "No pre-paired tokens by default"
        );
        assert_eq!(g.pair_rate_limit_per_minute, 10);
        assert_eq!(g.webhook_rate_limit_per_minute, 60);
        assert_eq!(g.idempotency_ttl_secs, 300);
    }

    #[test]
    fn checklist_gateway_cli_default_host_is_localhost() {
        // The CLI default for --host is 127.0.0.1 (checked in main.rs)
        // Here we verify the config default matches
        let c = Config::default();
        assert!(
            c.gateway.require_pairing,
            "Config default must require pairing"
        );
        assert!(
            !c.gateway.allow_public_bind,
            "Config default must block public bind"
        );
    }

    #[test]
    fn checklist_gateway_serde_roundtrip() {
        let g = GatewayConfig {
            port: 3000,
            host: "127.0.0.1".into(),
            require_pairing: true,
            allow_public_bind: false,
            paired_tokens: vec!["zc_test_token".into()],
            pair_rate_limit_per_minute: 12,
            webhook_rate_limit_per_minute: 80,
            idempotency_ttl_secs: 600,
            max_body_size: 65536,
            request_timeout_secs: 30,
            rate_limit_window_secs: 60,
            pairing_max_attempts: 5,
            pairing_lockout_secs: 300,
            rate_limiter_sweep_secs: 300,
        };
        let toml_str = toml::to_string(&g).unwrap();
        let parsed: GatewayConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.require_pairing);
        assert!(!parsed.allow_public_bind);
        assert_eq!(parsed.paired_tokens, vec!["zc_test_token"]);
        assert_eq!(parsed.pair_rate_limit_per_minute, 12);
        assert_eq!(parsed.webhook_rate_limit_per_minute, 80);
        assert_eq!(parsed.idempotency_ttl_secs, 600);
    }

    #[test]
    fn checklist_gateway_backward_compat_no_gateway_section() {
        // Old configs without [gateway] should get secure defaults
        let minimal = r#"
workspace_dir = "/tmp/ws"
config_path = "/tmp/config.toml"
default_temperature = 0.7
"#;
        let parsed: Config = toml::from_str(minimal).unwrap();
        assert!(
            parsed.gateway.require_pairing,
            "Missing [gateway] must default to require_pairing=true"
        );
        assert!(
            !parsed.gateway.allow_public_bind,
            "Missing [gateway] must default to allow_public_bind=false"
        );
    }

    #[test]
    fn checklist_autonomy_default_is_workspace_scoped() {
        let a = AutonomyConfig::default();
        assert!(a.workspace_only, "Default autonomy must be workspace_only");
        assert!(
            a.forbidden_paths.contains(&"/etc".to_string()),
            "Must block /etc"
        );
        assert!(
            a.forbidden_paths.contains(&"/proc".to_string()),
            "Must block /proc"
        );
        assert!(
            a.forbidden_paths.contains(&"~/.ssh".to_string()),
            "Must block ~/.ssh"
        );
    }

    // ══════════════════════════════════════════════════════════
    // COMPOSIO CONFIG TESTS
    // ══════════════════════════════════════════════════════════

    #[test]
    fn composio_config_default_disabled() {
        let c = ComposioConfig::default();
        assert!(!c.enabled, "Composio must be disabled by default");
        assert!(c.api_key.is_none(), "No API key by default");
        assert_eq!(c.entity_id, "default");
    }

    #[test]
    fn composio_config_serde_roundtrip() {
        let c = ComposioConfig {
            enabled: true,
            api_key: Some("comp-key-123".into()),
            entity_id: "user42".into(),
        };
        let toml_str = toml::to_string(&c).unwrap();
        let parsed: ComposioConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.api_key.as_deref(), Some("comp-key-123"));
        assert_eq!(parsed.entity_id, "user42");
    }

    #[test]
    fn composio_config_backward_compat_missing_section() {
        let minimal = r#"
workspace_dir = "/tmp/ws"
config_path = "/tmp/config.toml"
default_temperature = 0.7
"#;
        let parsed: Config = toml::from_str(minimal).unwrap();
        assert!(
            !parsed.composio.enabled,
            "Missing [composio] must default to disabled"
        );
        assert!(parsed.composio.api_key.is_none());
    }

    #[test]
    fn composio_config_partial_toml() {
        let toml_str = r"
enabled = true
";
        let parsed: ComposioConfig = toml::from_str(toml_str).unwrap();
        assert!(parsed.enabled);
        assert!(parsed.api_key.is_none());
        assert_eq!(parsed.entity_id, "default");
    }

    // ══════════════════════════════════════════════════════════
    // SECRETS CONFIG TESTS
    // ══════════════════════════════════════════════════════════

    #[test]
    fn secrets_config_default_encrypts() {
        let s = SecretsConfig::default();
        assert!(s.encrypt, "Encryption must be enabled by default");
    }

    #[test]
    fn secrets_config_serde_roundtrip() {
        let s = SecretsConfig { encrypt: false };
        let toml_str = toml::to_string(&s).unwrap();
        let parsed: SecretsConfig = toml::from_str(&toml_str).unwrap();
        assert!(!parsed.encrypt);
    }

    #[test]
    fn secrets_config_backward_compat_missing_section() {
        let minimal = r#"
workspace_dir = "/tmp/ws"
config_path = "/tmp/config.toml"
default_temperature = 0.7
"#;
        let parsed: Config = toml::from_str(minimal).unwrap();
        assert!(
            parsed.secrets.encrypt,
            "Missing [secrets] must default to encrypt=true"
        );
    }

    #[test]
    fn config_default_has_composio_and_secrets() {
        let c = Config::default();
        assert!(!c.composio.enabled);
        assert!(c.composio.api_key.is_none());
        assert!(c.secrets.encrypt);
        assert!(!c.browser.enabled);
        assert!(c.browser.allowed_domains.is_empty());
    }

    #[test]
    fn browser_config_default_disabled() {
        let b = BrowserConfig::default();
        assert!(!b.enabled);
        assert!(b.allowed_domains.is_empty());
        assert_eq!(b.backend, "agent_browser");
        assert!(b.native_headless);
        assert_eq!(b.native_webdriver_url, "http://127.0.0.1:9515");
        assert!(b.native_chrome_path.is_none());
        assert_eq!(b.computer_use.endpoint, "http://127.0.0.1:8787/v1/actions");
        assert_eq!(b.computer_use.timeout_ms, 15_000);
        assert!(!b.computer_use.allow_remote_endpoint);
        assert!(b.computer_use.window_allowlist.is_empty());
        assert!(b.computer_use.max_coordinate_x.is_none());
        assert!(b.computer_use.max_coordinate_y.is_none());
    }

    #[test]
    fn browser_config_serde_roundtrip() {
        let b = BrowserConfig {
            enabled: true,
            allowed_domains: vec!["example.com".into(), "docs.example.com".into()],
            session_name: None,
            backend: "auto".into(),
            native_headless: false,
            native_webdriver_url: "http://localhost:4444".into(),
            native_chrome_path: Some("/usr/bin/chromium".into()),
            computer_use: BrowserComputerUseConfig {
                endpoint: "https://computer-use.example.com/v1/actions".into(),
                api_key: Some("test-token".into()),
                timeout_ms: 8_000,
                allow_remote_endpoint: true,
                window_allowlist: vec!["Chrome".into(), "Visual Studio Code".into()],
                max_coordinate_x: Some(3840),
                max_coordinate_y: Some(2160),
            },
        };
        let toml_str = toml::to_string(&b).unwrap();
        let parsed: BrowserConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.allowed_domains.len(), 2);
        assert_eq!(parsed.allowed_domains[0], "example.com");
        assert_eq!(parsed.backend, "auto");
        assert!(!parsed.native_headless);
        assert_eq!(parsed.native_webdriver_url, "http://localhost:4444");
        assert_eq!(
            parsed.native_chrome_path.as_deref(),
            Some("/usr/bin/chromium")
        );
        assert_eq!(
            parsed.computer_use.endpoint,
            "https://computer-use.example.com/v1/actions"
        );
        assert_eq!(parsed.computer_use.api_key.as_deref(), Some("test-token"));
        assert_eq!(parsed.computer_use.timeout_ms, 8_000);
        assert!(parsed.computer_use.allow_remote_endpoint);
        assert_eq!(parsed.computer_use.window_allowlist.len(), 2);
        assert_eq!(parsed.computer_use.max_coordinate_x, Some(3840));
        assert_eq!(parsed.computer_use.max_coordinate_y, Some(2160));
    }

    #[test]
    fn browser_config_backward_compat_missing_section() {
        let minimal = r#"
workspace_dir = "/tmp/ws"
config_path = "/tmp/config.toml"
default_temperature = 0.7
"#;
        let parsed: Config = toml::from_str(minimal).unwrap();
        assert!(!parsed.browser.enabled);
        assert!(parsed.browser.allowed_domains.is_empty());
    }

    // ── Environment variable overrides (Docker support) ─────────

    fn env_override_test_guard() -> std::sync::MutexGuard<'static, ()> {
        static ENV_OVERRIDE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        ENV_OVERRIDE_TEST_LOCK
            .lock()
            .expect("env override test lock poisoned")
    }

    #[test]
    fn env_override_api_key() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();
        assert!(config.api_key.is_none());

        std::env::set_var("HOUSAKY_API_KEY", "sk-test-env-key");
        config.apply_env_overrides();
        assert_eq!(config.api_key.as_deref(), Some("sk-test-env-key"));

        std::env::remove_var("HOUSAKY_API_KEY");
    }

    #[test]
    fn env_override_api_key_fallback() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::remove_var("HOUSAKY_API_KEY");
        std::env::set_var("API_KEY", "sk-fallback-key");
        config.apply_env_overrides();
        assert_eq!(config.api_key.as_deref(), Some("sk-fallback-key"));

        std::env::remove_var("API_KEY");
    }

    #[test]
    fn env_override_provider() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::set_var("HOUSAKY_PROVIDER", "anthropic");
        config.apply_env_overrides();
        assert_eq!(config.default_provider.as_deref(), Some("anthropic"));

        std::env::remove_var("HOUSAKY_PROVIDER");
    }

    #[test]
    fn env_override_provider_fallback() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::remove_var("HOUSAKY_PROVIDER");
        std::env::set_var("PROVIDER", "openai");
        config.apply_env_overrides();
        assert_eq!(config.default_provider.as_deref(), Some("openai"));

        std::env::remove_var("PROVIDER");
    }

    #[test]
    fn env_override_model() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::set_var("HOUSAKY_MODEL", "gpt-4o");
        config.apply_env_overrides();
        assert_eq!(config.default_model.as_deref(), Some("gpt-4o"));

        std::env::remove_var("HOUSAKY_MODEL");
    }

    #[test]
    fn env_override_workspace() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::set_var("HOUSAKY_WORKSPACE", "/custom/workspace");
        config.apply_env_overrides();
        assert_eq!(config.workspace_dir, PathBuf::from("/custom/workspace"));

        std::env::remove_var("HOUSAKY_WORKSPACE");
    }

    #[test]
    fn env_override_empty_values_ignored() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();
        let original_provider = config.default_provider.clone();

        std::env::set_var("HOUSAKY_PROVIDER", "");
        config.apply_env_overrides();
        assert_eq!(config.default_provider, original_provider);

        std::env::remove_var("HOUSAKY_PROVIDER");
    }

    #[test]
    fn env_override_gateway_port() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();
        assert_eq!(config.gateway.port, 3000);

        std::env::set_var("HOUSAKY_GATEWAY_PORT", "8080");
        config.apply_env_overrides();
        assert_eq!(config.gateway.port, 8080);

        std::env::remove_var("HOUSAKY_GATEWAY_PORT");
    }

    #[test]
    fn env_override_port_fallback() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::remove_var("HOUSAKY_GATEWAY_PORT");
        std::env::set_var("PORT", "9000");
        config.apply_env_overrides();
        assert_eq!(config.gateway.port, 9000);

        std::env::remove_var("PORT");
    }

    #[test]
    fn env_override_gateway_host() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();
        assert_eq!(config.gateway.host, "127.0.0.1");

        std::env::set_var("HOUSAKY_GATEWAY_HOST", "0.0.0.0");
        config.apply_env_overrides();
        assert_eq!(config.gateway.host, "0.0.0.0");

        std::env::remove_var("HOUSAKY_GATEWAY_HOST");
    }

    #[test]
    fn env_override_host_fallback() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::remove_var("HOUSAKY_GATEWAY_HOST");
        std::env::set_var("HOST", "0.0.0.0");
        config.apply_env_overrides();
        assert_eq!(config.gateway.host, "0.0.0.0");

        std::env::remove_var("HOST");
    }

    #[test]
    fn env_override_temperature() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::set_var("HOUSAKY_TEMPERATURE", "0.5");
        config.apply_env_overrides();
        assert!((config.default_temperature - 0.5).abs() < f64::EPSILON);

        std::env::remove_var("HOUSAKY_TEMPERATURE");
    }

    #[test]
    fn env_override_temperature_out_of_range_ignored() {
        let _env_guard = env_override_test_guard();
        // Clean up any leftover env vars from other tests
        std::env::remove_var("HOUSAKY_TEMPERATURE");

        let mut config = Config::default();
        let original_temp = config.default_temperature;

        // Temperature > 2.0 should be ignored
        std::env::set_var("HOUSAKY_TEMPERATURE", "3.0");
        config.apply_env_overrides();
        assert!(
            (config.default_temperature - original_temp).abs() < f64::EPSILON,
            "Temperature 3.0 should be ignored (out of range)"
        );

        std::env::remove_var("HOUSAKY_TEMPERATURE");
    }

    #[test]
    fn env_override_invalid_port_ignored() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();
        let original_port = config.gateway.port;

        std::env::set_var("PORT", "not_a_number");
        config.apply_env_overrides();
        assert_eq!(config.gateway.port, original_port);

        std::env::remove_var("PORT");
    }

    #[test]
    fn gateway_config_default_values() {
        let g = GatewayConfig::default();
        assert_eq!(g.port, 3000);
        assert_eq!(g.host, "127.0.0.1");
        assert!(g.require_pairing);
        assert!(!g.allow_public_bind);
        assert!(g.paired_tokens.is_empty());
    }

    // ── Peripherals config ───────────────────────────────────────

    #[test]
    fn peripherals_config_default_disabled() {
        let p = PeripheralsConfig::default();
        assert!(!p.enabled);
        assert!(p.boards.is_empty());
    }

    #[test]
    fn peripheral_board_config_defaults() {
        let b = PeripheralBoardConfig::default();
        assert!(b.board.is_empty());
        assert_eq!(b.transport, "serial");
        assert!(b.path.is_none());
        assert_eq!(b.baud, 115_200);
    }

    #[test]
    fn peripherals_config_toml_roundtrip() {
        let p = PeripheralsConfig {
            enabled: true,
            boards: vec![PeripheralBoardConfig {
                board: "nucleo-f401re".into(),
                transport: "serial".into(),
                path: Some("/dev/ttyACM0".into()),
                baud: 115_200,
            }],
            datasheet_dir: None,
        };
        let toml_str = toml::to_string(&p).unwrap();
        let parsed: PeripheralsConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.boards.len(), 1);
        assert_eq!(parsed.boards[0].board, "nucleo-f401re");
        assert_eq!(parsed.boards[0].path.as_deref(), Some("/dev/ttyACM0"));
    }
}

use crate::config::Config;
use crate::tui::enhanced_app::layout::centered_rect;
use crate::tui::enhanced_app::theme::{
    style_border, style_border_active, style_border_focus, style_dim, style_error, style_muted,
    style_success, style_title, style_warning, Palette,
};
use anyhow::Result;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

// ── Section definitions ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Providers,
    Agent,
    Tools,
    Memory,
    Gateway,
    Channels,
    Security,
    Cost,
    Fallback,
    Skills,
    AGI,
    Heartbeat,
}

impl Section {
    const ALL: &'static [Section] = &[
        Section::Providers,
        Section::Agent,
        Section::Tools,
        Section::Memory,
        Section::Gateway,
        Section::Channels,
        Section::Security,
        Section::Cost,
        Section::Fallback,
        Section::Skills,
        Section::AGI,
        Section::Heartbeat,
    ];

    fn label(self) -> &'static str {
        match self {
            Section::Providers => "  Providers",
            Section::Agent => "  Agent",
            Section::Tools => "  Tools",
            Section::Memory => "  Memory",
            Section::Gateway => "  Gateway",
            Section::Channels => "  Channels",
            Section::Security => "  Security",
            Section::Cost => "  Cost",
            Section::Fallback => "  Fallback",
            Section::Skills => "  Skills",
            Section::AGI => "  AGI",
            Section::Heartbeat => "  Heartbeat",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Section::Providers => "◈",
            Section::Agent => "🤖",
            Section::Tools => "⚙",
            Section::Memory => "🧠",
            Section::Gateway => "🌐",
            Section::Channels => "📡",
            Section::Security => "🔒",
            Section::Cost => "💰",
            Section::Fallback => "🔄",
            Section::Skills => "🧩",
            Section::AGI => "✨",
            Section::Heartbeat => "💓",
        }
    }
}

// ── Field value type ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    Text,
    Number,
    Bool,
    Secret, // masked with ***
    Float,
}

#[derive(Debug, Clone)]
pub struct ConfigField {
    pub label: &'static str,
    pub desc: String,
    pub kind: FieldKind,
    pub value: String,
    pub modified: bool,
}

impl ConfigField {
    fn new(label: &'static str, desc: impl Into<String>, kind: FieldKind, value: String) -> Self {
        Self {
            label,
            desc: desc.into(),
            kind,
            value,
            modified: false,
        }
    }

    fn display_value(&self) -> String {
        if self.kind == FieldKind::Secret && !self.value.is_empty() {
            format!("{}…", &self.value[..self.value.len().min(6)])
        } else {
            self.value.clone()
        }
    }

    fn toggle_bool(&mut self) {
        if self.kind == FieldKind::Bool {
            self.value = if self.value == "true" {
                "false".to_string()
            } else {
                "true".to_string()
            };
            self.modified = true;
        }
    }
}

// ── Validation result ─────────────────────────────────────────────────────────

fn validate(field: &ConfigField, raw: &str) -> Option<String> {
    match field.kind {
        FieldKind::Number => {
            if raw.parse::<u64>().is_err() && raw.parse::<i64>().is_err() {
                Some(format!("'{}' is not a valid integer", raw))
            } else {
                None
            }
        }
        FieldKind::Float => {
            if raw.parse::<f64>().is_err() {
                Some(format!("'{}' is not a valid number", raw))
            } else {
                None
            }
        }
        FieldKind::Bool => {
            if raw != "true" && raw != "false" {
                Some("Must be 'true' or 'false'".to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

// ── Config editor state ───────────────────────────────────────────────────────

pub struct ConfigEditor {
    pub dirty: bool,
    pub last_saved: Option<String>,
    pub error: Option<String>,

    section_idx: usize,
    field_idx: usize,
    list_state: ListState,
    fields: Vec<ConfigField>,

    editing: bool,
    edit_buf: String,
    edit_cursor: usize,
    edit_error: Option<String>,

    show_confirm_quit: bool,
    show_raw: bool,
    raw_toml: String,
}

impl ConfigEditor {
    pub fn new(config: &Config) -> Self {
        let mut ed = Self {
            dirty: false,
            last_saved: None,
            error: None,
            section_idx: 0,
            field_idx: 0,
            list_state: ListState::default(),
            fields: Vec::new(),
            editing: false,
            edit_buf: String::new(),
            edit_cursor: 0,
            edit_error: None,
            show_confirm_quit: false,
            show_raw: false,
            raw_toml: String::new(),
        };
        ed.load_section(config, 0);
        ed.list_state.select(Some(0));
        ed
    }

    pub fn is_editing(&self) -> bool {
        self.editing
    }

    // ── Section loader ────────────────────────────────────────────────────────

    fn load_section(&mut self, config: &Config, idx: usize) {
        self.fields.clear();
        self.field_idx = 0;

        let section = Section::ALL[idx % Section::ALL.len()];
        match section {
            Section::Providers => {
                self.fields.push(ConfigField::new(
                    "api_key",
                    "Master API key (env: HOUSAKY_API_KEY)",
                    FieldKind::Secret,
                    config.api_key.clone().unwrap_or_default(),
                ));
                self.fields.push(ConfigField::new(
                    "default_provider",
                    "Provider name  e.g. openrouter, anthropic, openai",
                    FieldKind::Text,
                    config.default_provider.clone().unwrap_or_default(),
                ));
                self.fields.push(ConfigField::new(
                    "default_model",
                    "Model name  e.g. gpt-4o, claude-3-5-sonnet-latest",
                    FieldKind::Text,
                    config.default_model.clone().unwrap_or_default(),
                ));
                self.fields.push(ConfigField::new(
                    "default_temperature",
                    "Sampling temperature  [0.0 – 2.0]",
                    FieldKind::Float,
                    format!("{}", config.default_temperature),
                ));
                self.fields.push(ConfigField::new(
                    "provider_timeout",
                    "LLM request timeout (secs, 0=default)",
                    FieldKind::Number,
                    format!(
                        "{}",
                        config.provider_timeout.request_timeout_secs.unwrap_or(0)
                    ),
                ));
            }
            Section::Agent => {
                self.fields.push(ConfigField::new(
                    "max_tool_iterations",
                    "Max tool-call rounds per turn",
                    FieldKind::Number,
                    format!("{}", config.agent.max_tool_iterations),
                ));
                self.fields.push(ConfigField::new(
                    "max_history_messages",
                    "Context window message limit",
                    FieldKind::Number,
                    format!("{}", config.agent.max_history_messages),
                ));
                self.fields.push(ConfigField::new(
                    "parallel_tools",
                    "Run tools in parallel",
                    FieldKind::Bool,
                    format!("{}", config.agent.parallel_tools),
                ));
                self.fields.push(ConfigField::new(
                    "compact_context",
                    "Auto-compact long contexts",
                    FieldKind::Bool,
                    format!("{}", config.agent.compact_context),
                ));
                self.fields.push(ConfigField::new(
                    "tool_dispatcher",
                    "Tool dispatch strategy",
                    FieldKind::Text,
                    config.agent.tool_dispatcher.clone(),
                ));
                self.fields.push(ConfigField::new(
                    "bootstrap_max_chars",
                    "Max chars in bootstrap prompt",
                    FieldKind::Number,
                    format!("{}", config.agent.bootstrap_max_chars),
                ));
                self.fields.push(ConfigField::new(
                    "compaction_keep_recent",
                    "Messages to keep after compaction",
                    FieldKind::Number,
                    format!("{}", config.agent.compaction_keep_recent_messages),
                ));
                self.fields.push(ConfigField::new(
                    "compaction_max_source_chars",
                    "Max chars before compaction",
                    FieldKind::Number,
                    format!("{}", config.agent.compaction_max_source_chars),
                ));
                self.fields.push(ConfigField::new(
                    "compaction_max_summary_chars",
                    "Max chars in compaction summary",
                    FieldKind::Number,
                    format!("{}", config.agent.compaction_max_summary_chars),
                ));
            }
            Section::Tools => {
                self.fields.push(ConfigField::new(
                    "shell_timeout_secs",
                    "Shell command timeout",
                    FieldKind::Number,
                    format!("{}", config.tools.shell_timeout_secs),
                ));
                self.fields.push(ConfigField::new(
                    "shell_max_output_bytes",
                    "Max shell output size",
                    FieldKind::Number,
                    format!("{}", config.tools.shell_max_output_bytes),
                ));
                self.fields.push(ConfigField::new(
                    "file_read_max_bytes",
                    "Max file read size",
                    FieldKind::Number,
                    format!("{}", config.tools.file_read_max_bytes),
                ));
                self.fields.push(ConfigField::new(
                    "delegate_timeout_secs",
                    "Delegate agent timeout",
                    FieldKind::Number,
                    format!("{}", config.tools.delegate_timeout_secs),
                ));
                self.fields.push(ConfigField::new(
                    "screenshot_timeout_secs",
                    "Screenshot capture timeout",
                    FieldKind::Number,
                    format!("{}", config.tools.screenshot_timeout_secs),
                ));
                self.fields.push(ConfigField::new(
                    "screenshot_max_bytes",
                    "Max screenshot file size",
                    FieldKind::Number,
                    format!("{}", config.tools.screenshot_max_bytes),
                ));
                self.fields.push(ConfigField::new(
                    "image_max_bytes",
                    "Max image payload size",
                    FieldKind::Number,
                    format!("{}", config.tools.image_max_bytes),
                ));
            }
            Section::Memory => {
                self.fields.push(ConfigField::new(
                    "backend",
                    "Storage backend  (sqlite / memory)",
                    FieldKind::Text,
                    config.memory.backend.clone(),
                ));
                self.fields.push(ConfigField::new(
                    "auto_save",
                    "Auto-persist working memory",
                    FieldKind::Bool,
                    format!("{}", config.memory.auto_save),
                ));
                self.fields.push(ConfigField::new(
                    "hygiene_enabled",
                    "Enable memory hygiene pruning",
                    FieldKind::Bool,
                    format!("{}", config.memory.hygiene_enabled),
                ));
                self.fields.push(ConfigField::new(
                    "archive_after_days",
                    "Archive memories after N days",
                    FieldKind::Number,
                    format!("{}", config.memory.archive_after_days),
                ));
                self.fields.push(ConfigField::new(
                    "purge_after_days",
                    "Delete archived memories after N",
                    FieldKind::Number,
                    format!("{}", config.memory.purge_after_days),
                ));
                self.fields.push(ConfigField::new(
                    "conversation_retention_days",
                    "Keep conversation logs for N days",
                    FieldKind::Number,
                    format!("{}", config.memory.conversation_retention_days),
                ));
                self.fields.push(ConfigField::new(
                    "embedding_provider",
                    "Embedding model provider",
                    FieldKind::Text,
                    config.memory.embedding_provider.clone(),
                ));
                self.fields.push(ConfigField::new(
                    "embedding_cache_size",
                    "Embedding vector cache capacity",
                    FieldKind::Number,
                    format!("{}", config.memory.embedding_cache_size),
                ));
            }
            Section::Gateway => {
                self.fields.push(ConfigField::new(
                    "port",
                    "HTTP gateway port",
                    FieldKind::Number,
                    format!("{}", config.gateway.port),
                ));
                self.fields.push(ConfigField::new(
                    "host",
                    "Bind address",
                    FieldKind::Text,
                    config.gateway.host.clone(),
                ));
                self.fields.push(ConfigField::new(
                    "require_pairing",
                    "Require device pairing",
                    FieldKind::Bool,
                    format!("{}", config.gateway.require_pairing),
                ));
                self.fields.push(ConfigField::new(
                    "max_body_size",
                    "Max request body bytes",
                    FieldKind::Number,
                    format!("{}", config.gateway.max_body_size),
                ));
                self.fields.push(ConfigField::new(
                    "request_timeout_secs",
                    "Request processing timeout",
                    FieldKind::Number,
                    format!("{}", config.gateway.request_timeout_secs),
                ));
                self.fields.push(ConfigField::new(
                    "rate_limit_window_secs",
                    "Rate-limit window duration",
                    FieldKind::Number,
                    format!("{}", config.gateway.rate_limit_window_secs),
                ));
                self.fields.push(ConfigField::new(
                    "pair_rate_limit_per_minute",
                    "Pairing requests/min limit",
                    FieldKind::Number,
                    format!("{}", config.gateway.pair_rate_limit_per_minute),
                ));
                self.fields.push(ConfigField::new(
                    "webhook_rate_limit_per_minute",
                    "Webhook requests/min limit",
                    FieldKind::Number,
                    format!("{}", config.gateway.webhook_rate_limit_per_minute),
                ));
                self.fields.push(ConfigField::new(
                    "pairing_max_attempts",
                    "Max pairing auth attempts",
                    FieldKind::Number,
                    format!("{}", config.gateway.pairing_max_attempts),
                ));
                self.fields.push(ConfigField::new(
                    "pairing_lockout_secs",
                    "Lockout duration after failures",
                    FieldKind::Number,
                    format!("{}", config.gateway.pairing_lockout_secs),
                ));
                self.fields.push(ConfigField::new(
                    "idempotency_ttl_secs",
                    "Idempotency key TTL",
                    FieldKind::Number,
                    format!("{}", config.gateway.idempotency_ttl_secs),
                ));
                self.fields.push(ConfigField::new(
                    "allow_public_bind",
                    "Allow binding to 0.0.0.0",
                    FieldKind::Bool,
                    format!("{}", config.gateway.allow_public_bind),
                ));
            }
            Section::Channels => {
                self.fields.push(ConfigField::new(
                    "message_timeout_secs",
                    "Per-message processing timeout",
                    FieldKind::Number,
                    format!("{}", config.channels_config.message_timeout_secs),
                ));
                self.fields.push(ConfigField::new(
                    "parallelism_per_channel",
                    "Concurrent messages per channel",
                    FieldKind::Number,
                    format!("{}", config.channels_config.parallelism_per_channel),
                ));
                self.fields.push(ConfigField::new(
                    "min_in_flight_messages",
                    "Min in-flight message buffer",
                    FieldKind::Number,
                    format!("{}", config.channels_config.min_in_flight_messages),
                ));
                self.fields.push(ConfigField::new(
                    "max_in_flight_messages",
                    "Max in-flight message buffer",
                    FieldKind::Number,
                    format!("{}", config.channels_config.max_in_flight_messages),
                ));
                self.fields.push(ConfigField::new(
                    "history_turns",
                    "Conversation history turns",
                    FieldKind::Number,
                    format!("{}", config.channels_config.history_turns),
                ));
                self.fields.push(ConfigField::new(
                    "bootstrap_max_chars",
                    "Bootstrap prompt max chars",
                    FieldKind::Number,
                    format!("{}", config.channels_config.bootstrap_max_chars),
                ));
                self.fields.push(ConfigField::new(
                    "show_progress_messages",
                    "Show typing/progress indicators",
                    FieldKind::Bool,
                    format!("{}", config.channels_config.show_progress_messages),
                ));
            }
            Section::Security => {
                self.fields.push(ConfigField::new(
                    "encrypt_secrets",
                    "Encrypt secrets at rest",
                    FieldKind::Bool,
                    format!("{}", config.secrets.encrypt),
                ));
                self.fields.push(ConfigField::new(
                    "workspace_only",
                    "Restrict tools to workspace dir",
                    FieldKind::Bool,
                    format!("{}", config.autonomy.workspace_only),
                ));
                self.fields.push(ConfigField::new(
                    "block_high_risk_commands",
                    "Block destructive shell commands",
                    FieldKind::Bool,
                    format!("{}", config.autonomy.block_high_risk_commands),
                ));
                self.fields.push(ConfigField::new(
                    "require_approval_medium",
                    "Require approval for medium risk",
                    FieldKind::Bool,
                    format!("{}", config.autonomy.require_approval_for_medium_risk),
                ));
                self.fields.push(ConfigField::new(
                    "max_actions_per_hour",
                    "Autonomous action rate limit",
                    FieldKind::Number,
                    format!("{}", config.autonomy.max_actions_per_hour),
                ));
                self.fields.push(ConfigField::new(
                    "max_cost_per_day_cents",
                    "Max spend per day (cents)",
                    FieldKind::Number,
                    format!("{}", config.autonomy.max_cost_per_day_cents),
                ));
            }
            Section::Cost => {
                self.fields.push(ConfigField::new(
                    "enabled",
                    "Enable cost tracking",
                    FieldKind::Bool,
                    format!("{}", config.cost.enabled),
                ));
                self.fields.push(ConfigField::new(
                    "daily_limit_usd",
                    "Daily spend limit (USD)",
                    FieldKind::Float,
                    format!("{}", config.cost.daily_limit_usd),
                ));
                self.fields.push(ConfigField::new(
                    "monthly_limit_usd",
                    "Monthly spend limit (USD)",
                    FieldKind::Float,
                    format!("{}", config.cost.monthly_limit_usd),
                ));
                self.fields.push(ConfigField::new(
                    "warn_at_percent",
                    "Warn when N% of limit reached",
                    FieldKind::Number,
                    format!("{}", config.cost.warn_at_percent),
                ));
            }
            Section::Fallback => {
                self.fields.push(ConfigField::new(
                    "enabled",
                    "Enable provider fallback",
                    FieldKind::Bool,
                    format!("{}", config.fallback.enabled),
                ));
                self.fields.push(ConfigField::new(
                    "rotate_at_percent",
                    "Rotate provider at N% usage",
                    FieldKind::Number,
                    format!("{}", config.fallback.rotate_at_percent),
                ));
                self.fields.push(ConfigField::new(
                    "rotate_on_rate_limit",
                    "Rotate on 429 rate-limit errors",
                    FieldKind::Bool,
                    format!("{}", config.fallback.rotate_on_rate_limit),
                ));
            }
            Section::Skills => {
                let enabled_count = config.skills.enabled.values().filter(|&&v| v).count();
                let total_count = config.skills.enabled.len();
                self.fields.push(ConfigField::new(
                    "_info",
                    format!("{}/{} skills enabled", enabled_count, total_count),
                    FieldKind::Text,
                    String::new(),
                ));
                for (name, &en) in &config.skills.enabled {
                    self.fields.push(ConfigField::new(
                        "skill",
                        name.clone(),
                        FieldKind::Bool,
                        format!("{}", en),
                    ));
                }
            }
            Section::AGI => {
                self.fields.push(ConfigField::new(
                    "agi_enabled",
                    "Enable AGI features (goals, thoughts)",
                    FieldKind::Bool,
                    format!("{}", config.agi_enabled),
                ));
                self.fields.push(ConfigField::new(
                    "observability_backend",
                    "Telemetry backend (log/otlp)",
                    FieldKind::Text,
                    config.observability.backend.clone(),
                ));
            }
            Section::Heartbeat => {
                self.fields.push(ConfigField::new(
                    "enabled",
                    "Enable cognitive heartbeat",
                    FieldKind::Bool,
                    format!("{}", config.heartbeat.enabled),
                ));
                self.fields.push(ConfigField::new(
                    "interval_minutes",
                    "Heartbeat interval (minutes)",
                    FieldKind::Number,
                    format!("{}", config.heartbeat.interval_minutes),
                ));
            }
        }
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    pub fn section_next(&mut self, config: &Config) {
        self.commit_or_cancel();
        self.section_idx = (self.section_idx + 1) % Section::ALL.len();
        self.load_section(config, self.section_idx);
        self.list_state.select(Some(0));
    }

    pub fn section_prev(&mut self, config: &Config) {
        self.commit_or_cancel();
        let len = Section::ALL.len();
        self.section_idx = (self.section_idx + len - 1) % len;
        self.load_section(config, self.section_idx);
        self.list_state.select(Some(0));
    }

    pub fn field_up(&mut self) {
        if self.editing {
            return;
        }
        if self.field_idx > 0 {
            self.field_idx -= 1;
            self.list_state.select(Some(self.field_idx));
        }
    }

    pub fn field_down(&mut self) {
        if self.editing {
            return;
        }
        let max = self.fields.len().saturating_sub(1);
        if self.field_idx < max {
            self.field_idx += 1;
            self.list_state.select(Some(self.field_idx));
        }
    }

    // ── Editing ───────────────────────────────────────────────────────────────

    pub fn start_edit(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        let f = &self.fields[self.field_idx];
        // Bool fields toggle immediately, no inline edit needed
        if f.kind == FieldKind::Bool {
            self.fields[self.field_idx].toggle_bool();
            self.dirty = true;
            return;
        }
        // Skip info-only pseudo-fields
        if f.label == "_info" {
            return;
        }
        self.editing = true;
        self.edit_buf = f.value.clone();
        self.edit_cursor = self.edit_buf.len();
        self.edit_error = None;
    }

    pub fn edit_push(&mut self, c: char) {
        if !self.editing {
            return;
        }
        self.edit_buf.insert(self.edit_cursor, c);
        self.edit_cursor += c.len_utf8();
        self.edit_error = None;
    }

    pub fn edit_backspace(&mut self) {
        if !self.editing || self.edit_cursor == 0 {
            return;
        }
        let prev = self.edit_buf[..self.edit_cursor]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.edit_buf.remove(prev);
        self.edit_cursor = prev;
        self.edit_error = None;
    }

    pub fn edit_left(&mut self) {
        if !self.editing || self.edit_cursor == 0 {
            return;
        }
        self.edit_cursor = self.edit_buf[..self.edit_cursor]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
    }

    pub fn edit_right(&mut self) {
        if !self.editing || self.edit_cursor >= self.edit_buf.len() {
            return;
        }
        let c = self.edit_buf[self.edit_cursor..].chars().next().unwrap();
        self.edit_cursor += c.len_utf8();
    }

    pub fn edit_home(&mut self) {
        self.edit_cursor = 0;
    }
    pub fn edit_end(&mut self) {
        self.edit_cursor = self.edit_buf.len();
    }

    pub fn edit_kill_line(&mut self) {
        if self.editing {
            self.edit_buf.truncate(self.edit_cursor);
        }
    }

    pub fn commit_edit(&mut self) -> bool {
        if !self.editing {
            return true;
        }
        let field = &self.fields[self.field_idx];
        if let Some(err) = validate(field, &self.edit_buf) {
            self.edit_error = Some(err);
            return false;
        }
        self.fields[self.field_idx].value = self.edit_buf.clone();
        self.fields[self.field_idx].modified = true;
        self.editing = false;
        self.edit_error = None;
        self.dirty = true;
        true
    }

    pub fn cancel_edit(&mut self) {
        self.editing = false;
        self.edit_buf.clear();
        self.edit_error = None;
    }

    fn commit_or_cancel(&mut self) {
        if self.editing {
            let _ = self.commit_edit();
            if self.editing {
                self.cancel_edit();
            }
        }
    }

    // ── Save ──────────────────────────────────────────────────────────────────

    /// Apply edited field values back onto `config` and save to disk.
    pub fn apply_and_save(&mut self, config: &mut Config) -> Result<()> {
        self.commit_or_cancel();
        self.apply_fields_to_config(config)?;
        config.save()?;
        self.dirty = false;
        self.last_saved = Some(chrono::Local::now().format("%H:%M:%S").to_string());
        for f in &mut self.fields {
            f.modified = false;
        }
        Ok(())
    }

    fn apply_fields_to_config(&self, config: &mut Config) -> Result<()> {
        let section = Section::ALL[self.section_idx % Section::ALL.len()];
        for f in &self.fields {
            if !f.modified || f.label == "_info" {
                continue;
            }
            let v = &f.value;
            match (section, f.label) {
                // Providers
                (Section::Providers, "api_key") => {
                    config.api_key = if v.is_empty() { None } else { Some(v.clone()) };
                }
                (Section::Providers, "default_provider") => {
                    config.default_provider = if v.is_empty() { None } else { Some(v.clone()) };
                }
                (Section::Providers, "default_model") => {
                    config.default_model = if v.is_empty() { None } else { Some(v.clone()) };
                }
                (Section::Providers, "default_temperature") => {
                    config.default_temperature = v.parse().unwrap_or(config.default_temperature);
                }
                (Section::Providers, "provider_timeout") => {
                    let n: u64 = v.parse().unwrap_or(0);
                    config.provider_timeout.request_timeout_secs =
                        if n == 0 { None } else { Some(n) };
                }
                // Agent
                (Section::Agent, "max_tool_iterations") => {
                    config.agent.max_tool_iterations =
                        v.parse().unwrap_or(config.agent.max_tool_iterations);
                }
                (Section::Agent, "max_history_messages") => {
                    config.agent.max_history_messages =
                        v.parse().unwrap_or(config.agent.max_history_messages);
                }
                (Section::Agent, "parallel_tools") => {
                    config.agent.parallel_tools = v == "true";
                }
                (Section::Agent, "compact_context") => {
                    config.agent.compact_context = v == "true";
                }
                (Section::Agent, "tool_dispatcher") => {
                    config.agent.tool_dispatcher = v.clone();
                }
                (Section::Agent, "bootstrap_max_chars") => {
                    config.agent.bootstrap_max_chars =
                        v.parse().unwrap_or(config.agent.bootstrap_max_chars);
                }
                (Section::Agent, "compaction_keep_recent") => {
                    config.agent.compaction_keep_recent_messages = v
                        .parse()
                        .unwrap_or(config.agent.compaction_keep_recent_messages);
                }
                (Section::Agent, "compaction_max_source_chars") => {
                    config.agent.compaction_max_source_chars = v
                        .parse()
                        .unwrap_or(config.agent.compaction_max_source_chars);
                }
                (Section::Agent, "compaction_max_summary_chars") => {
                    config.agent.compaction_max_summary_chars = v
                        .parse()
                        .unwrap_or(config.agent.compaction_max_summary_chars);
                }
                // Tools
                (Section::Tools, "shell_timeout_secs") => {
                    config.tools.shell_timeout_secs =
                        v.parse().unwrap_or(config.tools.shell_timeout_secs);
                }
                (Section::Tools, "shell_max_output_bytes") => {
                    config.tools.shell_max_output_bytes =
                        v.parse().unwrap_or(config.tools.shell_max_output_bytes);
                }
                (Section::Tools, "file_read_max_bytes") => {
                    config.tools.file_read_max_bytes =
                        v.parse().unwrap_or(config.tools.file_read_max_bytes);
                }
                (Section::Tools, "delegate_timeout_secs") => {
                    config.tools.delegate_timeout_secs =
                        v.parse().unwrap_or(config.tools.delegate_timeout_secs);
                }
                (Section::Tools, "screenshot_timeout_secs") => {
                    config.tools.screenshot_timeout_secs =
                        v.parse().unwrap_or(config.tools.screenshot_timeout_secs);
                }
                (Section::Tools, "screenshot_max_bytes") => {
                    config.tools.screenshot_max_bytes =
                        v.parse().unwrap_or(config.tools.screenshot_max_bytes);
                }
                (Section::Tools, "image_max_bytes") => {
                    config.tools.image_max_bytes =
                        v.parse().unwrap_or(config.tools.image_max_bytes);
                }
                // Memory
                (Section::Memory, "backend") => {
                    config.memory.backend = v.clone();
                }
                (Section::Memory, "auto_save") => {
                    config.memory.auto_save = v == "true";
                }
                (Section::Memory, "hygiene_enabled") => {
                    config.memory.hygiene_enabled = v == "true";
                }
                (Section::Memory, "archive_after_days") => {
                    config.memory.archive_after_days =
                        v.parse().unwrap_or(config.memory.archive_after_days);
                }
                (Section::Memory, "purge_after_days") => {
                    config.memory.purge_after_days =
                        v.parse().unwrap_or(config.memory.purge_after_days);
                }
                (Section::Memory, "conversation_retention_days") => {
                    config.memory.conversation_retention_days = v
                        .parse()
                        .unwrap_or(config.memory.conversation_retention_days);
                }
                (Section::Memory, "embedding_provider") => {
                    config.memory.embedding_provider = v.clone();
                }
                (Section::Memory, "embedding_cache_size") => {
                    config.memory.embedding_cache_size =
                        v.parse().unwrap_or(config.memory.embedding_cache_size);
                }
                // Gateway
                (Section::Gateway, "port") => {
                    config.gateway.port = v.parse().unwrap_or(config.gateway.port);
                }
                (Section::Gateway, "host") => {
                    config.gateway.host = v.clone();
                }
                (Section::Gateway, "require_pairing") => {
                    config.gateway.require_pairing = v == "true";
                }
                (Section::Gateway, "max_body_size") => {
                    config.gateway.max_body_size =
                        v.parse().unwrap_or(config.gateway.max_body_size);
                }
                (Section::Gateway, "request_timeout_secs") => {
                    config.gateway.request_timeout_secs =
                        v.parse().unwrap_or(config.gateway.request_timeout_secs);
                }
                (Section::Gateway, "rate_limit_window_secs") => {
                    config.gateway.rate_limit_window_secs =
                        v.parse().unwrap_or(config.gateway.rate_limit_window_secs);
                }
                (Section::Gateway, "pair_rate_limit_per_minute") => {
                    config.gateway.pair_rate_limit_per_minute = v
                        .parse()
                        .unwrap_or(config.gateway.pair_rate_limit_per_minute);
                }
                (Section::Gateway, "webhook_rate_limit_per_minute") => {
                    config.gateway.webhook_rate_limit_per_minute = v
                        .parse()
                        .unwrap_or(config.gateway.webhook_rate_limit_per_minute);
                }
                (Section::Gateway, "pairing_max_attempts") => {
                    config.gateway.pairing_max_attempts =
                        v.parse().unwrap_or(config.gateway.pairing_max_attempts);
                }
                (Section::Gateway, "pairing_lockout_secs") => {
                    config.gateway.pairing_lockout_secs =
                        v.parse().unwrap_or(config.gateway.pairing_lockout_secs);
                }
                (Section::Gateway, "idempotency_ttl_secs") => {
                    config.gateway.idempotency_ttl_secs =
                        v.parse().unwrap_or(config.gateway.idempotency_ttl_secs);
                }
                (Section::Gateway, "allow_public_bind") => {
                    config.gateway.allow_public_bind = v == "true";
                }
                // Channels
                (Section::Channels, "message_timeout_secs") => {
                    config.channels_config.message_timeout_secs = v
                        .parse()
                        .unwrap_or(config.channels_config.message_timeout_secs);
                }
                (Section::Channels, "parallelism_per_channel") => {
                    config.channels_config.parallelism_per_channel = v
                        .parse()
                        .unwrap_or(config.channels_config.parallelism_per_channel);
                }
                (Section::Channels, "min_in_flight_messages") => {
                    config.channels_config.min_in_flight_messages = v
                        .parse()
                        .unwrap_or(config.channels_config.min_in_flight_messages);
                }
                (Section::Channels, "max_in_flight_messages") => {
                    config.channels_config.max_in_flight_messages = v
                        .parse()
                        .unwrap_or(config.channels_config.max_in_flight_messages);
                }
                (Section::Channels, "history_turns") => {
                    config.channels_config.history_turns =
                        v.parse().unwrap_or(config.channels_config.history_turns);
                }
                (Section::Channels, "bootstrap_max_chars") => {
                    config.channels_config.bootstrap_max_chars = v
                        .parse()
                        .unwrap_or(config.channels_config.bootstrap_max_chars);
                }
                (Section::Channels, "show_progress_messages") => {
                    config.channels_config.show_progress_messages = v == "true";
                }
                // Security
                (Section::Security, "encrypt_secrets") => {
                    config.secrets.encrypt = v == "true";
                }
                (Section::Security, "workspace_only") => {
                    config.autonomy.workspace_only = v == "true";
                }
                (Section::Security, "block_high_risk_commands") => {
                    config.autonomy.block_high_risk_commands = v == "true";
                }
                (Section::Security, "require_approval_medium") => {
                    config.autonomy.require_approval_for_medium_risk = v == "true";
                }
                (Section::Security, "max_actions_per_hour") => {
                    config.autonomy.max_actions_per_hour =
                        v.parse().unwrap_or(config.autonomy.max_actions_per_hour);
                }
                (Section::Security, "max_cost_per_day_cents") => {
                    config.autonomy.max_cost_per_day_cents =
                        v.parse().unwrap_or(config.autonomy.max_cost_per_day_cents);
                }
                // Cost
                (Section::Cost, "enabled") => {
                    config.cost.enabled = v == "true";
                }
                (Section::Cost, "daily_limit_usd") => {
                    config.cost.daily_limit_usd = v.parse().unwrap_or(config.cost.daily_limit_usd);
                }
                (Section::Cost, "monthly_limit_usd") => {
                    config.cost.monthly_limit_usd =
                        v.parse().unwrap_or(config.cost.monthly_limit_usd);
                }
                (Section::Cost, "warn_at_percent") => {
                    config.cost.warn_at_percent = v.parse().unwrap_or(config.cost.warn_at_percent);
                }
                // Fallback
                (Section::Fallback, "enabled") => {
                    config.fallback.enabled = v == "true";
                }
                (Section::Fallback, "rotate_at_percent") => {
                    config.fallback.rotate_at_percent =
                        v.parse().unwrap_or(config.fallback.rotate_at_percent);
                }
                (Section::Fallback, "rotate_on_rate_limit") => {
                    config.fallback.rotate_on_rate_limit = v == "true";
                }
                // Skills
                (Section::Skills, "skill") => {
                    // desc holds the skill name
                    let skill_name = f.desc.to_string();
                    config.skills.enabled.insert(skill_name, v == "true");
                }
                // AGI
                (Section::AGI, "agi_enabled") => {
                    config.agi_enabled = v == "true";
                }
                (Section::AGI, "observability_backend") => {
                    config.observability.backend = v.clone();
                }
                // Heartbeat
                (Section::Heartbeat, "enabled") => {
                    config.heartbeat.enabled = v == "true";
                }
                (Section::Heartbeat, "interval_minutes") => {
                    config.heartbeat.interval_minutes =
                        v.parse().unwrap_or(config.heartbeat.interval_minutes);
                }
                _ => {}
            }
        }
        Ok(())
    }

    // ── Raw TOML viewer ───────────────────────────────────────────────────────

    pub fn toggle_raw(&mut self, config: &Config) {
        self.show_raw = !self.show_raw;
        if self.show_raw {
            self.raw_toml =
                toml::to_string_pretty(config).unwrap_or_else(|e| format!("Error: {}", e));
        }
    }

    pub fn is_showing_raw(&self) -> bool {
        self.show_raw
    }

    // ── Draw ──────────────────────────────────────────────────────────────────

    pub fn draw(&mut self, f: &mut Frame, area: Rect) {
        if self.show_raw {
            self.draw_raw_toml(f, area);
            return;
        }

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(20), Constraint::Min(40)])
            .split(area);

        self.draw_section_list(f, cols[0]);
        self.draw_fields(f, cols[1]);

        // Edit overlay — drawn on top
        if self.editing {
            self.draw_edit_popup(f);
        }
    }

    fn draw_section_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = Section::ALL
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                let is_active = i == self.section_idx;
                let style = if is_active {
                    Style::default()
                        .fg(Palette::BG)
                        .bg(Palette::CYAN)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Palette::TEXT_DIM)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {} ", s.icon()), style),
                    Span::styled(s.label().trim_start(), style),
                ]))
            })
            .collect();

        let dirty_title = if self.dirty {
            " ✎ Config* "
        } else {
            " ◈ Config "
        };
        let title_style = if self.dirty {
            style_warning()
        } else {
            style_title()
        };

        let mut section_state = ListState::default();
        section_state.select(Some(self.section_idx));

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(style_border())
                    .title(Span::styled(dirty_title, title_style)),
            )
            .highlight_style(Style::default().bg(Palette::BG_SELECTED));

        f.render_stateful_widget(list, area, &mut section_state);
    }

    fn draw_fields(&mut self, f: &mut Frame, area: Rect) {
        let section = Section::ALL[self.section_idx % Section::ALL.len()];
        let title = format!(" {} {} ", section.icon(), section.label().trim_start());

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(3)])
            .split(area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border_focus())
            .title(Span::styled(title, style_title()));
        let inner = block.inner(rows[0]);
        f.render_widget(block, rows[0]);

        // Field rows
        let items: Vec<ListItem> = self
            .fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let selected = i == self.field_idx;
                let bg = if selected {
                    Palette::BG_SELECTED
                } else {
                    Palette::BG_PANEL
                };

                // Info pseudo-field
                if field.label == "_info" {
                    return ListItem::new(Line::from(vec![
                        Span::styled("   ", Style::default().bg(bg)),
                        Span::styled(
                            field.desc.to_owned(),
                            Style::default()
                                .fg(Palette::TEXT_DIM)
                                .bg(bg)
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }

                let cursor = if selected { "▶ " } else { "  " };
                let cursor_style = Style::default().fg(Palette::CYAN).bg(bg);

                let label_style = Style::default()
                    .fg(if selected {
                        Palette::TEXT_BRIGHT
                    } else {
                        Palette::TEXT
                    })
                    .bg(bg)
                    .add_modifier(if field.modified {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    });

                let modified_dot = if field.modified {
                    Span::styled(" ●", Style::default().fg(Palette::WARNING).bg(bg))
                } else {
                    Span::styled("  ", Style::default().bg(bg))
                };

                let value_str = field.display_value();
                let value_style = match field.kind {
                    FieldKind::Bool => {
                        if value_str == "true" {
                            Style::default()
                                .fg(Palette::SUCCESS)
                                .bg(bg)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Palette::TEXT_DIM).bg(bg)
                        }
                    }
                    FieldKind::Secret => Style::default().fg(Palette::VIOLET).bg(bg),
                    FieldKind::Number | FieldKind::Float => {
                        Style::default().fg(Palette::CYAN).bg(bg)
                    }
                    FieldKind::Text => Style::default().fg(Palette::TEXT).bg(bg),
                };

                let value_display = if field.kind == FieldKind::Bool {
                    if value_str == "true" {
                        "● on ".to_string()
                    } else {
                        "○ off".to_string()
                    }
                } else {
                    format!("{:<28}", truncate(&value_str, 28))
                };

                ListItem::new(Line::from(vec![
                    Span::styled(cursor, cursor_style),
                    Span::styled(format!("{:<32}", field.label), label_style),
                    modified_dot,
                    Span::styled("  ", Style::default().bg(bg)),
                    Span::styled(value_display, value_style),
                ]))
            })
            .collect();

        let mut ls = self.list_state.clone();
        let list = List::new(items).highlight_style(Style::default().bg(Palette::BG_SELECTED));
        f.render_stateful_widget(list, inner, &mut ls);
        self.list_state = ls;

        // Bottom hint / edit error bar
        let hint_text = if let Some(ref err) = self.edit_error {
            format!(" ✗ {}", err)
        } else if let Some(ref t) = self.last_saved {
            format!(
                " ✓ Saved at {}  — Ctrl+S save  Tab next section  r raw  q back",
                t
            )
        } else {
            " Enter=edit  Space=toggle bool  Ctrl+S=save  Tab=section  r=raw TOML  ↑↓/jk=nav  q=back ".to_string()
        };
        let hint_style = if self.edit_error.is_some() {
            style_error()
        } else if self.last_saved.is_some() {
            style_success()
        } else {
            style_muted()
        };
        let hint = Paragraph::new(Span::styled(hint_text, hint_style)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(style_border()),
        );
        f.render_widget(hint, rows[1]);

        // Description tooltip for selected field
        if let Some(field) = self.fields.get(self.field_idx) {
            if field.label != "_info" {
                // Show desc inline right-aligned in block title area — via a floating label
                // We draw it as a tiny overlay at top-right of fields area
                let desc = format!(" {} ", field.desc);
                let dw = (desc.len() as u16 + 2).min(area.width.saturating_sub(24));
                let desc_area =
                    Rect::new(area.x + area.width.saturating_sub(dw + 2), area.y, dw, 1);
                f.render_widget(Paragraph::new(Span::styled(desc, style_dim())), desc_area);
            }
        }
    }

    fn draw_edit_popup(&self, f: &mut Frame) {
        let field = match self.fields.get(self.field_idx) {
            Some(f) => f,
            None => return,
        };

        let area = f.area();
        let popup = centered_rect(60, 30, area);
        f.render_widget(Clear, popup);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // label info
                Constraint::Length(3), // input
                Constraint::Length(3), // error / hint
            ])
            .split(popup);

        // Header
        let header = Paragraph::new(vec![Line::from(vec![
            Span::styled("  Editing  ", style_muted()),
            Span::styled(field.label, style_title().add_modifier(Modifier::BOLD)),
        ])])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(style_border_focus())
                .title(Span::styled(format!(" ✎ {} ", field.label), style_title())),
        );
        f.render_widget(header, rows[0]);

        // Input with cursor
        let before = &self.edit_buf[..self.edit_cursor];
        let cursor_char = self.edit_buf[self.edit_cursor..]
            .chars()
            .next()
            .map(|c| c.to_string())
            .unwrap_or_else(|| " ".to_string());
        let after_start = self.edit_cursor
            + cursor_char
                .len()
                .min(self.edit_buf.len() - self.edit_cursor);
        let after = if after_start < self.edit_buf.len() {
            &self.edit_buf[after_start..]
        } else {
            ""
        };

        let mask = field.kind == FieldKind::Secret;
        let display_before = if mask {
            "•".repeat(before.chars().count())
        } else {
            before.to_string()
        };
        let display_after = if mask {
            "•".repeat(after.chars().count())
        } else {
            after.to_string()
        };

        let input_line = Line::from(vec![
            Span::styled("  ", style_muted()),
            Span::styled(display_before, Style::default().fg(Palette::TEXT_BRIGHT)),
            Span::styled(
                cursor_char,
                Style::default()
                    .fg(Palette::BG)
                    .bg(Palette::CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(display_after, Style::default().fg(Palette::TEXT_BRIGHT)),
        ]);
        let input = Paragraph::new(input_line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(style_border_active()),
        );
        f.render_widget(input, rows[1]);

        // Error or hint
        let (hint_text, hint_style) = if let Some(ref err) = self.edit_error {
            (format!("  ✗ {}", err), style_error())
        } else {
            (
                "  Enter=confirm  Esc=cancel  Ctrl+K=clear  ←→=cursor".to_string(),
                style_muted(),
            )
        };
        let hint = Paragraph::new(Span::styled(hint_text, hint_style)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(style_border()),
        );
        f.render_widget(hint, rows[2]);
    }

    fn draw_raw_toml(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border_focus())
            .title(Span::styled(
                format!(
                    " 📄 Raw TOML — {}  (r to close) ",
                    std::env::var("HOME").unwrap_or_default() + "/.housaky/config.toml"
                ),
                style_title(),
            ));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let lines: Vec<Line> = self
            .raw_toml
            .lines()
            .map(|l| {
                // Minimal TOML syntax highlighting
                if l.starts_with('[') {
                    Line::from(Span::styled(
                        l.to_owned(),
                        Style::default()
                            .fg(Palette::CYAN)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else if l.contains(" = ") {
                    let mut parts = l.splitn(2, " = ");
                    let key = parts.next().unwrap_or("");
                    let val = parts.next().unwrap_or("");
                    Line::from(vec![
                        Span::styled(key.to_owned(), Style::default().fg(Palette::VIOLET)),
                        Span::styled(" = ", style_muted()),
                        Span::styled(val.to_owned(), Style::default().fg(Palette::SUCCESS)),
                    ])
                } else if l.trim_start().starts_with('#') {
                    Line::from(Span::styled(l.to_owned(), style_dim()))
                } else {
                    Line::from(Span::raw(l.to_owned()))
                }
            })
            .collect();

        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        let end = s
            .char_indices()
            .nth(max.saturating_sub(1))
            .map(|(i, _)| i)
            .unwrap_or(s.len());
        format!("{}…", &s[..end])
    }
}

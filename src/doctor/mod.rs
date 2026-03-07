use crate::config::Config;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const DAEMON_STALE_SECONDS: i64 = 30;
const SCHEDULER_STALE_SECONDS: i64 = 120;
const CHANNEL_STALE_SECONDS: i64 = 300;

// ── Check severity ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Ok,
    Warning,
    Error,
    Info,
}

impl Severity {
    pub fn icon(self) -> &'static str {
        match self {
            Severity::Ok => "✅",
            Severity::Warning => "⚠️ ",
            Severity::Error => "❌",
            Severity::Info => "ℹ️ ",
        }
    }

    pub fn is_problem(self) -> bool {
        matches!(self, Severity::Warning | Severity::Error)
    }
}

// ── Check category ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckCategory {
    Daemon,
    Scheduler,
    Channel,
    Config,
    Security,
    FileSystem,
    Keys,
}

impl CheckCategory {
    pub fn label(self) -> &'static str {
        match self {
            CheckCategory::Daemon => "Daemon",
            CheckCategory::Scheduler => "Scheduler",
            CheckCategory::Channel => "Channel",
            CheckCategory::Config => "Config",
            CheckCategory::Security => "Security",
            CheckCategory::FileSystem => "FileSystem",
            CheckCategory::Keys => "Keys",
        }
    }
}

// ── Individual check result ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub category: CheckCategory,
    pub name: String,
    pub severity: Severity,
    pub message: String,
    pub fix_hint: Option<String>,
    pub fix_command: Option<String>,
    pub auto_fixable: bool,
}

impl CheckResult {
    pub fn ok(category: CheckCategory, name: impl Into<String>, msg: impl Into<String>) -> Self {
        Self {
            category,
            name: name.into(),
            severity: Severity::Ok,
            message: msg.into(),
            fix_hint: None,
            fix_command: None,
            auto_fixable: false,
        }
    }

    pub fn warn(category: CheckCategory, name: impl Into<String>, msg: impl Into<String>) -> Self {
        Self {
            category,
            name: name.into(),
            severity: Severity::Warning,
            message: msg.into(),
            fix_hint: None,
            fix_command: None,
            auto_fixable: false,
        }
    }

    pub fn error(category: CheckCategory, name: impl Into<String>, msg: impl Into<String>) -> Self {
        Self {
            category,
            name: name.into(),
            severity: Severity::Error,
            message: msg.into(),
            fix_hint: None,
            fix_command: None,
            auto_fixable: false,
        }
    }

    pub fn info(category: CheckCategory, name: impl Into<String>, msg: impl Into<String>) -> Self {
        Self {
            category,
            name: name.into(),
            severity: Severity::Info,
            message: msg.into(),
            fix_hint: None,
            fix_command: None,
            auto_fixable: false,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.fix_hint = Some(hint.into());
        self
    }

    pub fn with_command(mut self, cmd: impl Into<String>) -> Self {
        self.fix_command = Some(cmd.into());
        self
    }

    pub fn auto_fixable(mut self) -> Self {
        self.auto_fixable = true;
        self
    }
}

// ── Full diagnostic report ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub checks: Vec<CheckResult>,
    pub generated_at: String,
    pub total_ok: usize,
    pub total_warnings: usize,
    pub total_errors: usize,
    pub auto_fixable: usize,
}

impl DoctorReport {
    fn new(checks: Vec<CheckResult>) -> Self {
        let total_ok = checks.iter().filter(|c| c.severity == Severity::Ok).count();
        let total_warnings = checks
            .iter()
            .filter(|c| c.severity == Severity::Warning)
            .count();
        let total_errors = checks
            .iter()
            .filter(|c| c.severity == Severity::Error)
            .count();
        let auto_fixable = checks.iter().filter(|c| c.auto_fixable).count();
        Self {
            generated_at: Utc::now().to_rfc3339(),
            checks,
            total_ok,
            total_warnings,
            total_errors,
            auto_fixable,
        }
    }

    pub fn problems(&self) -> Vec<&CheckResult> {
        self.checks
            .iter()
            .filter(|c| c.severity.is_problem())
            .collect()
    }

    pub fn by_category(&self, cat: CheckCategory) -> Vec<&CheckResult> {
        self.checks.iter().filter(|c| c.category == cat).collect()
    }

    pub fn is_healthy(&self) -> bool {
        self.total_errors == 0
    }
}

// ── Public entry points ───────────────────────────────────────────────────────

/// Run all checks and print a full human-readable report.
pub fn run(config: &Config) -> Result<()> {
    let report = collect(config);
    print_report(&report, false);
    Ok(())
}

/// Run checks and attempt to auto-apply fixes for fixable issues.
pub fn run_fix(config: &Config) -> Result<()> {
    let report = collect(config);
    print_report(&report, true);

    let fixable: Vec<&CheckResult> = report
        .checks
        .iter()
        .filter(|c| c.auto_fixable && c.severity.is_problem())
        .collect();
    if fixable.is_empty() {
        println!("\n  No auto-fixable issues found.");
        return Ok(());
    }

    println!("\n🔧 Applying {} auto-fix(es)…", fixable.len());
    for check in fixable {
        if let Some(ref cmd) = check.fix_command {
            println!("  → [{}] {}: {}", check.category.label(), check.name, cmd);
            apply_fix(config, check);
        }
    }

    println!("\n  Re-running checks after fixes…");
    let report2 = collect(config);
    print_report(&report2, false);
    Ok(())
}

/// Run only security-focused checks.
pub fn run_security(config: &Config) -> Result<()> {
    let report = collect(config);
    println!("🔐 Housaky Security Review");
    println!();
    for check in report.by_category(CheckCategory::Security) {
        print_check(check);
    }
    for check in report.by_category(CheckCategory::Keys) {
        print_check(check);
    }
    for check in report.by_category(CheckCategory::FileSystem) {
        print_check(check);
    }
    println!();
    let sec_errors: usize = report
        .checks
        .iter()
        .filter(|c| {
            matches!(
                c.category,
                CheckCategory::Security | CheckCategory::Keys | CheckCategory::FileSystem
            ) && c.severity == Severity::Error
        })
        .count();
    if sec_errors == 0 {
        println!("  ✅ No critical security issues found.");
    } else {
        println!(
            "  ❌ {} critical security issue(s) — review above.",
            sec_errors
        );
    }
    Ok(())
}

/// Run only channel checks.
pub fn run_channels(config: &Config) -> Result<()> {
    let report = collect(config);
    println!("📡 Housaky Channel Status");
    println!();
    for check in report.by_category(CheckCategory::Channel) {
        print_check(check);
    }
    println!();
    Ok(())
}

/// Collect all checks and return a structured `DoctorReport` (used by TUI).
pub fn collect(config: &Config) -> DoctorReport {
    let mut checks: Vec<CheckResult> = Vec::new();

    checks.extend(check_daemon(config));
    checks.extend(check_config(config));
    checks.extend(check_filesystem(config));
    checks.extend(check_channels_config(config));
    checks.extend(check_security(config));
    checks.extend(check_keys(config));

    DoctorReport::new(checks)
}

// ── Daemon / scheduler checks ─────────────────────────────────────────────────

fn check_daemon(config: &Config) -> Vec<CheckResult> {
    let mut checks = Vec::new();
    let state_file = crate::daemon::state_file_path(config);

    if !state_file.exists() {
        checks.push(
            CheckResult::error(
                CheckCategory::Daemon,
                "daemon-state",
                format!("State file not found: {}", state_file.display()),
            )
            .with_hint("Start the daemon first")
            .with_command("housaky daemon")
            .auto_fixable(),
        );
        return checks;
    }

    let raw = match std::fs::read_to_string(&state_file) {
        Ok(r) => r,
        Err(e) => {
            checks.push(CheckResult::error(
                CheckCategory::Daemon,
                "daemon-state-read",
                format!("Cannot read state file: {e}"),
            ));
            return checks;
        }
    };

    let snapshot: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            checks.push(
                CheckResult::error(
                    CheckCategory::Daemon,
                    "daemon-state-parse",
                    format!("State file corrupted (invalid JSON): {e}"),
                )
                .with_hint("Delete and restart the daemon to regenerate state")
                .with_command(format!("rm {} && housaky daemon", state_file.display())),
            );
            return checks;
        }
    };

    checks.push(CheckResult::ok(
        CheckCategory::Daemon,
        "daemon-state-file",
        format!("State file present: {}", state_file.display()),
    ));

    let updated_at = snapshot
        .get("updated_at")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    match DateTime::parse_from_rfc3339(updated_at) {
        Ok(ts) => {
            let age = Utc::now()
                .signed_duration_since(ts.with_timezone(&Utc))
                .num_seconds();
            if age <= DAEMON_STALE_SECONDS {
                checks.push(CheckResult::ok(
                    CheckCategory::Daemon,
                    "daemon-heartbeat",
                    format!("Daemon heartbeat fresh ({age}s ago)"),
                ));
            } else if age <= DAEMON_STALE_SECONDS * 10 {
                checks.push(
                    CheckResult::warn(
                        CheckCategory::Daemon,
                        "daemon-heartbeat",
                        format!("Daemon heartbeat stale ({age}s ago — expected ≤{DAEMON_STALE_SECONDS}s)"),
                    )
                    .with_hint("The daemon may be slow or overloaded")
                    .with_command("housaky daemon restart"),
                );
            } else {
                checks.push(
                    CheckResult::error(
                        CheckCategory::Daemon,
                        "daemon-heartbeat",
                        format!("Daemon appears dead — last heartbeat {age}s ago"),
                    )
                    .with_hint("Restart the daemon")
                    .with_command("housaky daemon restart")
                    .auto_fixable(),
                );
            }
        }
        Err(_) => {
            checks.push(
                CheckResult::warn(
                    CheckCategory::Daemon,
                    "daemon-timestamp",
                    format!("Invalid daemon timestamp: {updated_at:?}"),
                )
                .with_hint("State file may be from an older version — restart daemon")
                .with_command("housaky daemon restart"),
            );
        }
    }

    if let Some(components) = snapshot
        .get("components")
        .and_then(serde_json::Value::as_object)
    {
        if let Some(scheduler) = components.get("scheduler") {
            let ok = scheduler
                .get("status")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|s| s == "ok");
            let age = scheduler
                .get("last_ok")
                .and_then(serde_json::Value::as_str)
                .and_then(parse_rfc3339)
                .map_or(i64::MAX, |dt| {
                    Utc::now().signed_duration_since(dt).num_seconds()
                });

            if ok && age <= SCHEDULER_STALE_SECONDS {
                checks.push(CheckResult::ok(
                    CheckCategory::Scheduler,
                    "scheduler",
                    format!("Scheduler healthy (last ok {age}s ago)"),
                ));
            } else {
                checks.push(
                    CheckResult::error(
                        CheckCategory::Scheduler,
                        "scheduler",
                        format!("Scheduler unhealthy (status_ok={ok}, last_ok={age}s ago)"),
                    )
                    .with_hint("Check for cron panics or resource exhaustion")
                    .with_command("housaky daemon restart"),
                );
            }

            let restart_count = scheduler
                .get("restart_count")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            if restart_count > 5 {
                checks.push(
                    CheckResult::warn(
                        CheckCategory::Scheduler,
                        "scheduler-restarts",
                        format!(
                            "Scheduler has restarted {restart_count} times — possible crash loop"
                        ),
                    )
                    .with_hint("Check logs: housaky logs"),
                );
            }
        } else {
            checks.push(
                CheckResult::error(
                    CheckCategory::Scheduler,
                    "scheduler-missing",
                    "Scheduler component not found in state",
                )
                .with_command("housaky daemon restart"),
            );
        }

        let mut channel_count = 0_u32;
        let mut stale_channels = 0_u32;
        for (name, component) in components {
            if !name.starts_with("channel:") {
                continue;
            }
            channel_count += 1;
            let status_ok = component
                .get("status")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|s| s == "ok");
            let age = component
                .get("last_ok")
                .and_then(serde_json::Value::as_str)
                .and_then(parse_rfc3339)
                .map_or(i64::MAX, |dt| {
                    Utc::now().signed_duration_since(dt).num_seconds()
                });

            if status_ok && age <= CHANNEL_STALE_SECONDS {
                checks.push(CheckResult::ok(
                    CheckCategory::Channel,
                    name.clone(),
                    format!("Fresh (last ok {age}s ago)"),
                ));
            } else {
                stale_channels += 1;
                let short = name.trim_start_matches("channel:");
                checks.push(
                    CheckResult::error(
                        CheckCategory::Channel,
                        name.clone(),
                        format!("Stale/unhealthy (status_ok={status_ok}, last_ok={age}s ago)"),
                    )
                    .with_hint(format!(
                        "Check {short} token/credentials, then restart channel"
                    ))
                    .with_command(format!("housaky channel doctor && housaky channel start")),
                );
            }
        }

        if channel_count == 0 {
            checks.push(CheckResult::info(
                CheckCategory::Channel,
                "channels-runtime",
                "No channel components tracked in daemon state yet — channels not started",
            ));
        } else {
            let live = channel_count - stale_channels;
            checks.push(CheckResult::info(
                CheckCategory::Channel,
                "channels-summary",
                format!(
                    "{channel_count} channel(s) tracked: {live} healthy, {stale_channels} stale"
                ),
            ));
        }
    }

    checks
}

// ── Config checks ─────────────────────────────────────────────────────────────

fn check_config(config: &Config) -> Vec<CheckResult> {
    let mut checks = Vec::new();

    if config.config_path.exists() {
        checks.push(CheckResult::ok(
            CheckCategory::Config,
            "config-file",
            format!("Config file present: {}", config.config_path.display()),
        ));
    } else {
        checks.push(
            CheckResult::error(
                CheckCategory::Config,
                "config-file",
                format!("Config file missing: {}", config.config_path.display()),
            )
            .with_hint("Run the onboarding wizard to generate a config")
            .with_command("housaky onboard"),
        );
    }

    if config.api_key.is_none() && config.default_provider.is_none() {
        checks.push(
            CheckResult::error(
                CheckCategory::Config,
                "api-key",
                "No API key or default provider configured",
            )
            .with_hint("Add an API key for your preferred provider")
            .with_command("housaky keys manager add --provider openrouter --key YOUR_KEY"),
        );
    } else {
        checks.push(CheckResult::ok(
            CheckCategory::Config,
            "api-key",
            "Provider/API key configured",
        ));
    }

    if config.default_provider.is_none() {
        checks.push(
            CheckResult::warn(
                CheckCategory::Config,
                "default-provider",
                "No default_provider set in config — will fall back to openrouter",
            )
            .with_hint("Set default_provider in config or via: housaky config set default_provider openrouter"),
        );
    }

    if config.default_model.is_none() {
        checks.push(
            CheckResult::warn(
                CheckCategory::Config,
                "default-model",
                "No default_model set — will use provider default",
            )
            .with_hint("Set default_model in your config"),
        );
    }

    if config.default_temperature < 0.0 || config.default_temperature > 2.0 {
        checks.push(
            CheckResult::error(
                CheckCategory::Config,
                "temperature",
                format!(
                    "default_temperature={} is out of range [0.0, 2.0]",
                    config.default_temperature
                ),
            )
            .with_hint("Set default_temperature to a value between 0.0 and 2.0"),
        );
    }

    if !config.agi_enabled {
        checks.push(CheckResult::info(
            CheckCategory::Config,
            "agi-enabled",
            "AGI mode disabled (agi_enabled = false) — enable for full capabilities",
        ));
    }

    checks
}

// ── Filesystem checks ─────────────────────────────────────────────────────────

fn check_filesystem(config: &Config) -> Vec<CheckResult> {
    let mut checks = Vec::new();
    let ws = &config.workspace_dir;

    if ws.exists() {
        checks.push(CheckResult::ok(
            CheckCategory::FileSystem,
            "workspace-dir",
            format!("Workspace directory exists: {}", ws.display()),
        ));

        let perms_ok = check_dir_writable(ws);
        if perms_ok {
            checks.push(CheckResult::ok(
                CheckCategory::FileSystem,
                "workspace-writable",
                "Workspace directory is writable",
            ));
        } else {
            checks.push(
                CheckResult::error(
                    CheckCategory::FileSystem,
                    "workspace-writable",
                    format!("Workspace directory is not writable: {}", ws.display()),
                )
                .with_hint("Fix permissions")
                .with_command(format!("chmod 700 {}", ws.display())),
            );
        }
    } else {
        checks.push(
            CheckResult::warn(
                CheckCategory::FileSystem,
                "workspace-dir",
                format!("Workspace directory missing: {}", ws.display()),
            )
            .with_hint("Housaky will create it on first run, or run onboard")
            .with_command("housaky onboard"),
        );
    }

    let secret_key_path = ws.join(".secret_key");
    if secret_key_path.exists() {
        if let Ok(perms) = secret_key_permissions(&secret_key_path) {
            if perms {
                checks.push(CheckResult::ok(
                    CheckCategory::FileSystem,
                    "secret-key-perms",
                    ".secret_key file has correct permissions (0600)",
                ));
            } else {
                checks.push(
                    CheckResult::error(
                        CheckCategory::Security,
                        "secret-key-perms",
                        format!(
                            ".secret_key file has overly permissive permissions: {}",
                            secret_key_path.display()
                        ),
                    )
                    .with_hint("Restrict to owner-only read/write")
                    .with_command(format!("chmod 600 {}", secret_key_path.display()))
                    .auto_fixable(),
                );
            }
        }
    }

    let audit_log = ws.join("audit.log");
    if audit_log.exists() {
        let size = std::fs::metadata(&audit_log).map(|m| m.len()).unwrap_or(0);
        let size_mb = size / (1024 * 1024);
        if size_mb > 100 {
            checks.push(
                CheckResult::warn(
                    CheckCategory::FileSystem,
                    "audit-log-size",
                    format!("Audit log is large: {}MB — consider rotating", size_mb),
                )
                .with_hint("Rotation is automatic when audit logging is enabled")
                .with_command(format!("truncate -s 0 {}", audit_log.display())),
            );
        }
    }

    checks
}

// ── Channel config checks ─────────────────────────────────────────────────────

fn check_channels_config(config: &Config) -> Vec<CheckResult> {
    let mut checks = Vec::new();
    let cc = &config.channels_config;
    let mut any_configured = false;

    if let Some(ref tg) = cc.telegram {
        any_configured = true;
        if tg.bot_token.is_empty() || tg.bot_token == "YOUR_BOT_TOKEN" {
            checks.push(
                CheckResult::error(
                    CheckCategory::Channel,
                    "channel-telegram-token",
                    "Telegram bot_token is empty or placeholder",
                )
                .with_hint("Get a token from @BotFather on Telegram")
                .with_command("housaky onboard"),
            );
        } else {
            checks.push(CheckResult::ok(
                CheckCategory::Channel,
                "channel-telegram",
                "Telegram configured",
            ));
        }

        if tg.allowed_users.is_empty() {
            checks.push(
                CheckResult::warn(
                    CheckCategory::Channel,
                    "channel-telegram-acl",
                    "Telegram allowed_users is empty — ANY user can interact with the bot",
                )
                .with_hint("Add your Telegram user ID to allowed_users in config"),
            );
        }
    }

    if let Some(ref dc) = cc.discord {
        any_configured = true;
        if dc.bot_token.is_empty() || dc.bot_token == "YOUR_BOT_TOKEN" {
            checks.push(
                CheckResult::error(
                    CheckCategory::Channel,
                    "channel-discord-token",
                    "Discord bot_token is empty or placeholder",
                )
                .with_hint("Get a token from Discord Developer Portal")
                .with_command("housaky onboard"),
            );
        } else {
            checks.push(CheckResult::ok(
                CheckCategory::Channel,
                "channel-discord",
                "Discord configured",
            ));
        }

        if dc.allowed_users.is_empty() {
            checks.push(
                CheckResult::warn(
                    CheckCategory::Channel,
                    "channel-discord-acl",
                    "Discord allowed_users is empty — any server member can interact",
                )
                .with_hint("Add your Discord user ID to allowed_users in config"),
            );
        }
    }

    if let Some(ref sl) = cc.slack {
        any_configured = true;
        if sl.bot_token.is_empty() || sl.bot_token.starts_with("xoxb-PLACEHOLDER") {
            checks.push(
                CheckResult::error(
                    CheckCategory::Channel,
                    "channel-slack-token",
                    "Slack bot_token is empty or placeholder",
                )
                .with_hint("Create a Slack app and install it to get an xoxb- token")
                .with_command("housaky onboard"),
            );
        } else {
            checks.push(CheckResult::ok(
                CheckCategory::Channel,
                "channel-slack",
                "Slack configured",
            ));
        }
    }

    if let Some(ref wa) = cc.whatsapp {
        any_configured = true;
        if wa.access_token.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
            checks.push(
                CheckResult::error(
                    CheckCategory::Channel,
                    "channel-whatsapp-token",
                    "WhatsApp access_token is empty",
                )
                .with_hint("Get a permanent token from Meta Business API"),
            );
        } else {
            checks.push(CheckResult::ok(
                CheckCategory::Channel,
                "channel-whatsapp",
                "WhatsApp configured",
            ));
        }
        if wa.verify_token.as_ref().map(|s| s.is_empty()).unwrap_or(true) || wa.verify_token.as_deref() == Some("housaky_secret") {
            checks.push(
                CheckResult::warn(
                    CheckCategory::Channel,
                    "channel-whatsapp-verify",
                    "WhatsApp verify_token uses default/empty value — set a strong secret",
                )
                .with_hint("Set a unique verify_token in your WhatsApp config section"),
            );
        }
    }

    if let Some(ref mx) = cc.matrix {
        any_configured = true;
        if mx.access_token.is_empty() {
            checks.push(
                CheckResult::error(
                    CheckCategory::Channel,
                    "channel-matrix-token",
                    "Matrix access_token is empty",
                )
                .with_hint("Generate an access token via your Matrix homeserver"),
            );
        } else {
            checks.push(CheckResult::ok(
                CheckCategory::Channel,
                "channel-matrix",
                "Matrix configured",
            ));
        }
    }

    if let Some(ref email) = cc.email {
        any_configured = true;
        if email.password.is_empty() {
            checks.push(
                CheckResult::error(
                    CheckCategory::Channel,
                    "channel-email-password",
                    "Email password is empty — channel will fail to authenticate",
                )
                .with_hint("Set password in your email config section"),
            );
        } else {
            checks.push(CheckResult::ok(
                CheckCategory::Channel,
                "channel-email",
                "Email configured",
            ));
        }

        if email.smtp_host.is_empty() {
            checks.push(
                CheckResult::error(
                    CheckCategory::Channel,
                    "channel-email-smtp",
                    "Email smtp_host is empty",
                )
                .with_hint("Set smtp_host, e.g. smtp.gmail.com"),
            );
        }
    }

    if let Some(ref irc) = cc.irc {
        any_configured = true;
        checks.push(CheckResult::ok(
            CheckCategory::Channel,
            "channel-irc",
            format!("IRC configured ({}:{})", irc.server, irc.port),
        ));
        if irc.verify_tls.unwrap_or(true) {
            checks.push(CheckResult::ok(
                CheckCategory::Channel,
                "channel-irc-tls",
                "IRC TLS verification enabled",
            ));
        } else {
            checks.push(
                CheckResult::warn(
                    CheckCategory::Channel,
                    "channel-irc-tls",
                    "IRC TLS verification is disabled (verify_tls = false) — susceptible to MITM",
                )
                .with_hint("Set verify_tls = true unless connecting to a local server"),
            );
        }
    }

    if let Some(ref lk) = cc.lark {
        any_configured = true;
        if lk.app_id.is_empty() || lk.app_secret.is_empty() {
            checks.push(
                CheckResult::error(
                    CheckCategory::Channel,
                    "channel-lark-credentials",
                    "Lark app_id or app_secret is empty",
                )
                .with_hint("Create a Lark/Feishu app and fill in app_id + app_secret"),
            );
        } else {
            checks.push(CheckResult::ok(
                CheckCategory::Channel,
                "channel-lark",
                "Lark/Feishu configured",
            ));
        }
    }

    if let Some(ref dt) = cc.dingtalk {
        any_configured = true;
        if dt.client_id.is_empty() || dt.client_secret.is_empty() {
            checks.push(
                CheckResult::error(
                    CheckCategory::Channel,
                    "channel-dingtalk-credentials",
                    "DingTalk client_id or client_secret is empty",
                )
                .with_hint("Register a DingTalk app and fill in client_id + client_secret"),
            );
        } else {
            checks.push(CheckResult::ok(
                CheckCategory::Channel,
                "channel-dingtalk",
                "DingTalk configured",
            ));
        }
    }

    if !any_configured {
        checks.push(
            CheckResult::warn(
                CheckCategory::Channel,
                "channels-none",
                "No communication channels configured — Housaky can only be reached via CLI",
            )
            .with_hint("Run the onboarding wizard to configure your preferred channel(s)")
            .with_command("housaky onboard"),
        );
    }

    checks
}

// ── Security checks ───────────────────────────────────────────────────────────

fn check_security(config: &Config) -> Vec<CheckResult> {
    let mut checks = Vec::new();

    if config.secrets.encrypt {
        checks.push(CheckResult::ok(
            CheckCategory::Security,
            "secrets-encryption",
            "Secret encryption enabled (ChaCha20-Poly1305)",
        ));
    } else {
        checks.push(
            CheckResult::warn(
                CheckCategory::Security,
                "secrets-encryption",
                "Secret encryption disabled (secrets.encrypt = false) — API keys stored in plaintext",
            )
            .with_hint("Enable encryption: set [secrets] encrypt = true in config.toml"),
        );
    }

    match config.autonomy.level {
        crate::security::AutonomyLevel::Full => {
            checks.push(
                CheckResult::warn(
                    CheckCategory::Security,
                    "autonomy-level",
                    "Autonomy level is 'full' — agent executes without approval prompts",
                )
                .with_hint("Consider 'supervised' mode unless you trust the environment fully"),
            );
        }
        crate::security::AutonomyLevel::Supervised => {
            checks.push(CheckResult::ok(
                CheckCategory::Security,
                "autonomy-level",
                "Autonomy level: supervised (requires approval for risky operations)",
            ));
        }
        crate::security::AutonomyLevel::ReadOnly => {
            checks.push(CheckResult::ok(
                CheckCategory::Security,
                "autonomy-level",
                "Autonomy level: read-only (safest mode)",
            ));
        }
    }

    if config.autonomy.max_actions_per_hour > 500 {
        checks.push(
            CheckResult::warn(
                CheckCategory::Security,
                "rate-limit",
                format!(
                    "max_actions_per_hour={} is very high — could exhaust API quota or budget",
                    config.autonomy.max_actions_per_hour
                ),
            )
            .with_hint("Consider setting max_actions_per_hour ≤ 200 for normal usage"),
        );
    }

    if config.gateway.allow_public_bind {
        checks.push(
            CheckResult::warn(
                CheckCategory::Security,
                "gateway-public-bind",
                "Gateway allow_public_bind is true — accessible from all network interfaces",
            )
            .with_hint("Set allow_public_bind = false unless remote access is intentional"),
        );
    }

    if config.gateway.require_pairing {
        checks.push(CheckResult::ok(
            CheckCategory::Security,
            "gateway-pairing",
            "Gateway pairing required (authenticated)",
        ));
    } else {
        checks.push(
            CheckResult::error(
                CheckCategory::Security,
                "gateway-pairing",
                "Gateway pairing is disabled (require_pairing = false) — API is unauthenticated",
            )
            .with_hint("Set gateway.require_pairing = true in config.toml"),
        );
    }

    if config.gateway.paired_tokens.is_empty() {
        checks.push(CheckResult::info(
            CheckCategory::Security,
            "gateway-tokens",
            "No paired gateway tokens yet — pair a client with: housaky gateway pair",
        ));
    } else {
        checks.push(CheckResult::ok(
            CheckCategory::Security,
            "gateway-tokens",
            format!(
                "{} gateway token(s) paired",
                config.gateway.paired_tokens.len()
            ),
        ));
    }

    if config.gateway.host != "127.0.0.1" && config.gateway.host != "localhost" {
        checks.push(
            CheckResult::warn(
                CheckCategory::Security,
                "gateway-bind",
                format!(
                    "Gateway bound to '{}' — accessible beyond localhost",
                    config.gateway.host
                ),
            )
            .with_hint("Set gateway.host = \"127.0.0.1\" unless remote access is intentional"),
        );
    }

    let audit_log = config.workspace_dir.join("audit.log");
    if audit_log.exists() {
        checks.push(CheckResult::ok(
            CheckCategory::Security,
            "audit-logging",
            format!("Audit log present: {}", audit_log.display()),
        ));
    } else {
        checks.push(
            CheckResult::info(
                CheckCategory::Security,
                "audit-logging",
                "No audit.log found — audit logging may be disabled or daemon has not run yet",
            )
            .with_hint("Audit logging activates when the daemon runs with observability enabled"),
        );
    }

    let config_path = &config.config_path;
    if config_path.exists() {
        if let Ok(perms) = secret_key_permissions(config_path) {
            if perms {
                checks.push(CheckResult::ok(
                    CheckCategory::Security,
                    "config-file-perms",
                    "Config file permissions are appropriately restrictive",
                ));
            } else {
                checks.push(
                    CheckResult::warn(
                        CheckCategory::Security,
                        "config-file-perms",
                        format!(
                            "Config file may have broad permissions: {}",
                            config_path.display()
                        ),
                    )
                    .with_hint("Restrict config file to owner-only")
                    .with_command(format!("chmod 600 {}", config_path.display()))
                    .auto_fixable(),
                );
            }
        }
    }

    if config.tunnel.provider != "none" {
        checks.push(CheckResult::info(
            CheckCategory::Security,
            "tunnel-enabled",
            "Tunnel is enabled — ensure tunnel provider is trusted and auth is enforced",
        ));
    }

    checks
}

// ── Keys / provider checks ────────────────────────────────────────────────────

fn check_keys(config: &Config) -> Vec<CheckResult> {
    let mut checks = Vec::new();
    let keys_path = config.workspace_dir.join("keys.json");

    if keys_path.exists() {
        checks.push(CheckResult::ok(
            CheckCategory::Keys,
            "keys-file",
            format!("Keys file present: {}", keys_path.display()),
        ));

        if let Ok(perms) = secret_key_permissions(&keys_path) {
            if perms {
                checks.push(CheckResult::ok(
                    CheckCategory::Keys,
                    "keys-file-perms",
                    "Keys file permissions are appropriately restrictive (0600)",
                ));
            } else {
                checks.push(
                    CheckResult::error(
                        CheckCategory::Security,
                        "keys-file-perms",
                        format!(
                            "Keys file has overly permissive permissions: {}",
                            keys_path.display()
                        ),
                    )
                    .with_hint("Keys file should be owner-readable only")
                    .with_command(format!("chmod 600 {}", keys_path.display()))
                    .auto_fixable(),
                );
            }
        }

        if let Ok(raw) = std::fs::read_to_string(&keys_path) {
            if raw.contains("sk-") && !raw.contains("enc2:") && !raw.contains("enc:") {
                checks.push(
                    CheckResult::error(
                        CheckCategory::Security,
                        "keys-plaintext",
                        "Possible plaintext API key (sk-…) detected in keys.json — encryption not applied",
                    )
                    .with_hint("Enable secrets.encrypt = true and re-add keys to encrypt them"),
                );
            }
        }
    } else {
        checks.push(
            CheckResult::warn(
                CheckCategory::Keys,
                "keys-file",
                "No keys.json found — API keys may not be persisted",
            )
            .with_hint(
                "Add API keys: housaky keys manager add --provider openrouter --key YOUR_KEY",
            ),
        );
    }

    checks
}

// ── Fix applicator ────────────────────────────────────────────────────────────

fn apply_fix(config: &Config, check: &CheckResult) {
    match check.name.as_str() {
        "secret-key-perms" => {
            let path = config.workspace_dir.join(".secret_key");
            if let Err(e) = set_file_permissions_600(&path) {
                eprintln!("    ⚠️  Could not fix {}: {e}", path.display());
            } else {
                println!("    ✅ Fixed permissions on {}", path.display());
            }
        }
        "config-file-perms" => {
            if let Err(e) = set_file_permissions_600(&config.config_path) {
                eprintln!(
                    "    ⚠️  Could not fix {}: {e}",
                    config.config_path.display()
                );
            } else {
                println!(
                    "    ✅ Fixed permissions on {}",
                    config.config_path.display()
                );
            }
        }
        "keys-file-perms" => {
            let path = config.workspace_dir.join("keys.json");
            if let Err(e) = set_file_permissions_600(&path) {
                eprintln!("    ⚠️  Could not fix {}: {e}", path.display());
            } else {
                println!("    ✅ Fixed permissions on {}", path.display());
            }
        }
        _ => {
            if let Some(ref hint) = check.fix_hint {
                println!("    💡 Manual action required: {hint}");
            }
            if let Some(ref cmd) = check.fix_command {
                println!("    🔧 Run: {cmd}");
            }
        }
    }
}

// ── Print helpers ─────────────────────────────────────────────────────────────

fn print_report(report: &DoctorReport, with_fixes: bool) {
    println!("🩺 Housaky Doctor");
    println!();

    let mut last_cat: Option<CheckCategory> = None;
    for check in &report.checks {
        if last_cat != Some(check.category) {
            if last_cat.is_some() {
                println!();
            }
            println!("  ── {} ──", check.category.label());
            last_cat = Some(check.category);
        }
        print_check(check);
        if with_fixes && check.severity.is_problem() {
            if let Some(ref hint) = check.fix_hint {
                println!("       💡 {hint}");
            }
            if let Some(ref cmd) = check.fix_command {
                println!("       🔧 {cmd}");
            }
        }
    }

    println!();
    println!(
        "  Summary: {} ok  {} warning(s)  {} error(s){}",
        report.total_ok,
        report.total_warnings,
        report.total_errors,
        if report.auto_fixable > 0 {
            format!(
                "  ({} auto-fixable — run `housaky doctor fix`)",
                report.auto_fixable
            )
        } else {
            String::new()
        }
    );
}

pub fn print_check(check: &CheckResult) {
    println!(
        "  {} [{}] {}",
        check.severity.icon(),
        check.name,
        check.message,
    );
}

// ── Platform helpers ──────────────────────────────────────────────────────────

fn parse_rfc3339(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn check_dir_writable(path: &std::path::Path) -> bool {
    let test = path.join(".housaky_writable_test");
    match std::fs::write(&test, b"") {
        Ok(()) => {
            let _ = std::fs::remove_file(&test);
            true
        }
        Err(_) => false,
    }
}

/// Returns `Ok(true)` if the file has 0600-equivalent permissions (owner rw only).
fn secret_key_permissions(path: &PathBuf) -> Result<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::metadata(path)?.permissions();
        let mode = perms.mode() & 0o777;
        Ok(mode == 0o600 || mode == 0o400)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(true)
    }
}

fn set_file_permissions_600(path: &PathBuf) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms)
            .with_context(|| format!("chmod 600 {}", path.display()))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use serde_json::json;
    use tempfile::TempDir;

    fn test_config(tmp: &TempDir) -> Config {
        let mut config = Config::default();
        config.workspace_dir = tmp.path().join("workspace");
        config.config_path = tmp.path().join("config.toml");
        std::fs::create_dir_all(&config.workspace_dir).unwrap();
        config
    }

    #[test]
    fn parse_rfc3339_accepts_valid_timestamp() {
        let parsed = parse_rfc3339("2025-01-02T03:04:05Z");
        assert!(parsed.is_some());
    }

    #[test]
    fn parse_rfc3339_rejects_invalid_timestamp() {
        let parsed = parse_rfc3339("not-a-timestamp");
        assert!(parsed.is_none());
    }

    #[test]
    fn run_returns_ok_when_state_file_missing() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        let result = run(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn collect_returns_error_for_invalid_json_state_file() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        let state_file = crate::daemon::state_file_path(&config);
        std::fs::write(&state_file, "not-json").unwrap();

        let report = collect(&config);
        let has_parse_error = report
            .checks
            .iter()
            .any(|c| c.name == "daemon-state-parse" && c.severity == Severity::Error);
        assert!(has_parse_error);
    }

    #[test]
    fn collect_accepts_well_formed_state_snapshot() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        let state_file = crate::daemon::state_file_path(&config);

        let now = Utc::now().to_rfc3339();
        let snapshot = json!({
            "updated_at": now,
            "components": {
                "scheduler": {
                    "status": "ok",
                    "last_ok": now,
                    "last_error": null,
                    "updated_at": now,
                    "restart_count": 0
                },
                "channel:discord": {
                    "status": "ok",
                    "last_ok": now,
                    "last_error": null,
                    "updated_at": now,
                    "restart_count": 0
                }
            }
        });

        std::fs::write(&state_file, serde_json::to_vec_pretty(&snapshot).unwrap()).unwrap();
        let report = collect(&config);
        assert!(report.is_healthy() || report.total_errors <= 2);
    }

    #[test]
    fn check_result_severity_icons() {
        assert_eq!(Severity::Ok.icon(), "✅");
        assert_eq!(Severity::Error.icon(), "❌");
        assert!(!Severity::Ok.is_problem());
        assert!(Severity::Error.is_problem());
        assert!(Severity::Warning.is_problem());
    }

    #[test]
    fn doctor_report_counts_correctly() {
        let checks = vec![
            CheckResult::ok(CheckCategory::Config, "a", "ok"),
            CheckResult::error(CheckCategory::Security, "b", "bad"),
            CheckResult::warn(CheckCategory::Channel, "c", "meh"),
        ];
        let report = DoctorReport::new(checks);
        assert_eq!(report.total_ok, 1);
        assert_eq!(report.total_errors, 1);
        assert_eq!(report.total_warnings, 1);
        assert!(!report.is_healthy());
    }

    #[test]
    fn channel_config_check_warns_on_empty_telegram_token() {
        let tmp = TempDir::new().unwrap();
        let mut config = test_config(&tmp);
        config.channels_config.telegram = Some(crate::config::TelegramConfig {
            bot_token: "".to_string(),
            allowed_users: vec![],
            elevenlabs: None,
        });
        let checks = check_channels_config(&config);
        let found = checks
            .iter()
            .any(|c| c.name == "channel-telegram-token" && c.severity == Severity::Error);
        assert!(found);
    }
}

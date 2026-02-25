//! Browser automation tool with pluggable backends.
//!
//! By default this uses Vercel's `agent-browser` CLI for automation.
//! Optionally, a Rust-native backend can be enabled at build time via
//! `--features browser-native` and selected through config.
//! Computer-use (OS-level) actions are supported via an optional sidecar endpoint.

use super::traits::{Tool, ToolResult};
use crate::config::schema::{BrowserConfig, BrowserProfileConfig};
use crate::security::SecurityPolicy;
use anyhow::Context;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::oneshot;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieData {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: Option<i64>,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    pub name: String,
    pub config: BrowserProfileConfig,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct BrowserBridge {
    profiles: Arc<RwLock<HashMap<String, BrowserProfileConfig>>>,
    active_profile: Arc<RwLock<Option<String>>>,
    cookie_storage_path: Arc<RwLock<Option<PathBuf>>>,
    shutdown_tx: Arc<RwLock<Option<oneshot::Sender<()>>>>,
    server_addr: Arc<RwLock<Option<SocketAddr>>>,
}

impl BrowserBridge {
    pub fn new() -> Self {
        Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
            active_profile: Arc::new(RwLock::new(None)),
            cookie_storage_path: Arc::new(RwLock::new(None)),
            shutdown_tx: Arc::new(RwLock::new(None)),
            server_addr: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_profiles(
        profiles: HashMap<String, BrowserProfileConfig>,
        default_profile: String,
        cookie_storage_path: Option<String>,
    ) -> Self {
        let bridge = Self::new();
        {
            let mut p = bridge.profiles.write();
            *p = profiles;
        }
        if !default_profile.is_empty() {
            *bridge.active_profile.write() = Some(default_profile);
        }
        if let Some(path) = cookie_storage_path {
            *bridge.cookie_storage_path.write() = Some(PathBuf::from(path));
        }
        bridge
    }

    pub fn set_cookie_storage_path(&self, path: Option<String>) {
        *self.cookie_storage_path.write() = path.map(PathBuf::from);
    }

    pub fn get_profiles(&self) -> Vec<ProfileInfo> {
        let profiles = self.profiles.read();
        let active = self.active_profile.read();
        profiles
            .iter()
            .map(|(name, config)| ProfileInfo {
                name: name.clone(),
                config: config.clone(),
                is_active: active.as_ref().map(|a| a == name).unwrap_or(false),
            })
            .collect()
    }

    pub fn create_profile(&self, name: String, config: BrowserProfileConfig) -> anyhow::Result<()> {
        let mut profiles = self.profiles.write();
        profiles.insert(name, config);
        Ok(())
    }

    pub fn delete_profile(&self, name: &str) -> anyhow::Result<()> {
        let mut profiles = self.profiles.write();
        if profiles.remove(name).is_none() {
            anyhow::bail!("Profile '{}' not found", name);
        }
        let mut active = self.active_profile.write();
        if active.as_deref() == Some(name) {
            *active = None;
        }
        Ok(())
    }

    pub fn set_active_profile(&self, name: &str) -> anyhow::Result<()> {
        let profiles = self.profiles.read();
        if !profiles.contains_key(name) {
            anyhow::bail!("Profile '{}' not found", name);
        }
        drop(profiles);
        *self.active_profile.write() = Some(name.to_string());
        Ok(())
    }

    pub fn get_active_profile(&self) -> Option<BrowserProfileConfig> {
        let active = self.active_profile.read();
        let profiles = self.profiles.read();
        active.as_ref().and_then(|name| profiles.get(name).cloned())
    }

    pub fn save_cookies(&self, profile_name: &str, cookies: Vec<CookieData>) -> anyhow::Result<PathBuf> {
        let storage_path = self.cookie_storage_path.read();
        let base_path = storage_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cookie storage path not configured"))?;

        let profile_dir = base_path.join(profile_name);
        fs::create_dir_all(&profile_dir)
            .with_context(|| format!("Failed to create cookie directory: {:?}", profile_dir))?;

        let cookie_file = profile_dir.join("cookies.json");
        let json = serde_json::to_string_pretty(&cookies)
            .with_context(|| "Failed to serialize cookies")?;
        fs::write(&cookie_file, json)
            .with_context(|| format!("Failed to write cookies to {:?}", cookie_file))?;

        Ok(cookie_file)
    }

    pub fn load_cookies(&self, profile_name: &str) -> anyhow::Result<Vec<CookieData>> {
        let storage_path = self.cookie_storage_path.read();
        let base_path = storage_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cookie storage path not configured"))?;

        let cookie_file = base_path.join(profile_name).join("cookies.json");
        if !cookie_file.exists() {
            return Ok(Vec::new());
        }

        let json = fs::read_to_string(&cookie_file)
            .with_context(|| format!("Failed to read cookies from {:?}", cookie_file))?;
        let cookies: Vec<CookieData> =
            serde_json::from_str(&json).with_context(|| "Failed to parse cookies")?;

        Ok(cookies)
    }

    pub async fn start_bridge_server(&self, port: u16) -> anyhow::Result<SocketAddr> {
        let addr: SocketAddr = format!("127.0.0.1:{}", port)
            .parse()
            .with_context(|| format!("Invalid address:127.0.0.1:{}", port))?;

        let listener = TcpListener::bind(addr)
            .await
            .with_context(|| format!("Failed to bind to {}", addr))?;

        let (tx, rx) = oneshot::channel();
        *self.shutdown_tx.write() = Some(tx);

        let profiles = Arc::clone(&self.profiles);
        let active_profile = Arc::clone(&self.active_profile);
        let cookie_storage_path = Arc::clone(&self.cookie_storage_path);

        tokio::spawn(async move {
            let mut rx = rx;
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, _addr)) => {
                                let profiles = Arc::clone(&profiles);
                                let active_profile = Arc::clone(&active_profile);
                                let cookie_storage_path = Arc::clone(&cookie_storage_path);
                                tokio::spawn(async move {
                                    if let Err(e) = handle_bridge_connection(stream, &profiles, &active_profile, &cookie_storage_path).await {
                                        tracing::warn!("Connection error: {}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                tracing::error!("Accept error: {}", e);
                                break;
                            }
                        }
                    }
                    _ = &mut rx => {
                        tracing::info!("Bridge server shutting down");
                        break;
                    }
                }
            }
        });

        *self.server_addr.write() = Some(addr);
        Ok(addr)
    }

    pub fn stop_bridge_server(&self) -> anyhow::Result<()> {
        let tx = self.shutdown_tx.write().take();
        if let Some(tx) = tx {
            let _ = tx.send(());
        }
        *self.server_addr.write() = None;
        Ok(())
    }

    pub fn server_address(&self) -> Option<SocketAddr> {
        *self.server_addr.read()
    }
}

impl Default for BrowserBridge {
    fn default() -> Self {
        Self::new()
    }
}

async fn handle_bridge_connection(
    mut stream: TcpStream,
    profiles: &Arc<RwLock<HashMap<String, BrowserProfileConfig>>>,
    active_profile: &Arc<RwLock<Option<String>>>,
    cookie_storage_path: &Arc<RwLock<Option<PathBuf>>>,
) -> anyhow::Result<()> {
    let mut buffer = [0u8; 4096];
    let n = stream.read(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..n]);

    let response = if let Some((method, path)) = request.lines().next().and_then(|l| l.split_once(' ')) {
        match (method, path) {
            ("GET", "/profiles") => {
                let profiles = profiles.read();
                let active = active_profile.read();
                let result: Vec<ProfileInfo> = profiles
                    .iter()
                    .map(|(name, config)| ProfileInfo {
                        name: name.clone(),
                        config: config.clone(),
                        is_active: active.as_ref().map(|a| a == name).unwrap_or(false),
                    })
                    .collect();
                let json = serde_json::to_string(&result).unwrap_or_default();
                format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", json.len(), json)
            }
            ("GET", path) if path.starts_with("/profile/") => {
                let name = path.trim_start_matches("/profile/");
                let profiles = profiles.read();
                if let Some(config) = profiles.get(name) {
                    let json = serde_json::to_string(config).unwrap_or_default();
                    format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", json.len(), json)
                } else {
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_string()
                }
            }
            ("GET", path) if path.starts_with("/cookies/") => {
                let profile_name = path.trim_start_matches("/cookies/");
                let storage_path = cookie_storage_path.read();
                if let Some(base_path) = storage_path.as_ref() {
                    let cookie_file = base_path.join(profile_name).join("cookies.json");
                    if cookie_file.exists() {
                        let json = fs::read_to_string(&cookie_file).unwrap_or_default();
                        format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", json.len(), json)
                    } else {
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n[]".to_string()
                    }
                } else {
                    "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n".to_string()
                }
            }
            _ => {
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_string()
            }
        }
    } else {
        "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n".to_string()
    };

    stream.write_all(response.as_bytes()).await?;
    Ok(())
}

/// Computer-use sidecar settings.
#[derive(Debug, Clone)]
pub struct ComputerUseConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub timeout_ms: u64,
    pub allow_remote_endpoint: bool,
    pub window_allowlist: Vec<String>,
    pub max_coordinate_x: Option<i64>,
    pub max_coordinate_y: Option<i64>,
}

impl Default for ComputerUseConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:8787/v1/actions".into(),
            api_key: None,
            timeout_ms: 15_000,
            allow_remote_endpoint: false,
            window_allowlist: Vec::new(),
            max_coordinate_x: None,
            max_coordinate_y: None,
        }
    }
}

/// Browser automation tool using pluggable backends.
pub struct BrowserTool {
    security: Arc<SecurityPolicy>,
    allowed_domains: Vec<String>,
    session_name: Option<String>,
    backend: String,
    native_headless: bool,
    native_webdriver_url: String,
    native_chrome_path: Option<String>,
    computer_use: ComputerUseConfig,
    bridge: Arc<BrowserBridge>,
    #[cfg(feature = "browser-native")]
    native_state: tokio::sync::Mutex<native_backend::NativeBrowserState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrowserBackendKind {
    AgentBrowser,
    RustNative,
    ComputerUse,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResolvedBackend {
    AgentBrowser,
    RustNative,
    ComputerUse,
}

impl BrowserBackendKind {
    fn parse(raw: &str) -> anyhow::Result<Self> {
        let key = raw.trim().to_ascii_lowercase().replace('-', "_");
        match key.as_str() {
            "agent_browser" | "agentbrowser" => Ok(Self::AgentBrowser),
            "rust_native" | "native" => Ok(Self::RustNative),
            "computer_use" | "computeruse" => Ok(Self::ComputerUse),
            "auto" => Ok(Self::Auto),
            _ => anyhow::bail!(
                "Unsupported browser backend '{raw}'. Use 'agent_browser', 'rust_native', 'computer_use', or 'auto'"
            ),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::AgentBrowser => "agent_browser",
            Self::RustNative => "rust_native",
            Self::ComputerUse => "computer_use",
            Self::Auto => "auto",
        }
    }
}

/// Response from agent-browser --json commands
#[derive(Debug, Deserialize)]
struct AgentBrowserResponse {
    success: bool,
    data: Option<Value>,
    error: Option<String>,
}

/// Response format from computer-use sidecar.
#[derive(Debug, Deserialize)]
struct ComputerUseResponse {
    #[serde(default)]
    success: Option<bool>,
    #[serde(default)]
    data: Option<Value>,
    #[serde(default)]
    error: Option<String>,
}

/// Supported browser actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserAction {
    /// Navigate to a URL
    Open { url: String },
    /// Get accessibility snapshot with refs
    Snapshot {
        #[serde(default)]
        interactive_only: bool,
        #[serde(default)]
        compact: bool,
        #[serde(default)]
        depth: Option<u32>,
    },
    /// Click an element by ref or selector
    Click { selector: String },
    /// Fill a form field
    Fill { selector: String, value: String },
    /// Type text into focused element
    Type { selector: String, text: String },
    /// Get text content of element
    GetText { selector: String },
    /// Get page title
    GetTitle,
    /// Get current URL
    GetUrl,
    /// Take screenshot
    Screenshot {
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        full_page: bool,
    },
    /// Wait for element or time
    Wait {
        #[serde(default)]
        selector: Option<String>,
        #[serde(default)]
        ms: Option<u64>,
        #[serde(default)]
        text: Option<String>,
    },
    /// Press a key
    Press { key: String },
    /// Hover over element
    Hover { selector: String },
    /// Scroll page
    Scroll {
        direction: String,
        #[serde(default)]
        pixels: Option<u32>,
    },
    /// Check if element is visible
    IsVisible { selector: String },
    /// Close browser
    Close,
    /// Find element by semantic locator
    Find {
        by: String, // role, text, label, placeholder, testid
        value: String,
        action: String, // click, fill, text, hover
        #[serde(default)]
        fill_value: Option<String>,
    },
}

impl BrowserTool {
    pub fn new(
        security: Arc<SecurityPolicy>,
        allowed_domains: Vec<String>,
        session_name: Option<String>,
    ) -> Self {
        Self::new_with_backend(
            security,
            allowed_domains,
            session_name,
            "agent_browser".into(),
            true,
            "http://127.0.0.1:9515".into(),
            None,
            ComputerUseConfig::default(),
            BrowserBridge::new(),
        )
    }

    pub fn from_config(
        security: Arc<SecurityPolicy>,
        config: &BrowserConfig,
    ) -> Self {
        let bridge = BrowserBridge::with_profiles(
            config.profiles.clone(),
            config.default_profile.clone(),
            config.cookie_storage_path.clone(),
        );
        Self::new_with_backend(
            security,
            config.allowed_domains.clone(),
            config.session_name.clone(),
            config.backend.clone(),
            config.native_headless,
            config.native_webdriver_url.clone(),
            config.native_chrome_path.clone(),
            ComputerUseConfig {
                endpoint: config.computer_use.endpoint.clone(),
                api_key: config.computer_use.api_key.clone(),
                timeout_ms: config.computer_use.timeout_ms,
                allow_remote_endpoint: config.computer_use.allow_remote_endpoint,
                window_allowlist: config.computer_use.window_allowlist.clone(),
                max_coordinate_x: config.computer_use.max_coordinate_x,
                max_coordinate_y: config.computer_use.max_coordinate_y,
            },
            bridge,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_backend(
        security: Arc<SecurityPolicy>,
        allowed_domains: Vec<String>,
        session_name: Option<String>,
        backend: String,
        native_headless: bool,
        native_webdriver_url: String,
        native_chrome_path: Option<String>,
        computer_use: ComputerUseConfig,
        bridge: BrowserBridge,
    ) -> Self {
        Self {
            security,
            allowed_domains: normalize_domains(allowed_domains),
            session_name,
            backend,
            native_headless,
            native_webdriver_url,
            native_chrome_path,
            computer_use,
            bridge: Arc::new(bridge),
            #[cfg(feature = "browser-native")]
            native_state: tokio::sync::Mutex::new(native_backend::NativeBrowserState::default()),
        }
    }

    pub fn bridge(&self) -> &BrowserBridge {
        &self.bridge
    }

    pub async fn start_bridge(&self, port: u16) -> anyhow::Result<std::net::SocketAddr> {
        self.bridge.start_bridge_server(port).await
    }

    pub fn stop_bridge(&self) -> anyhow::Result<()> {
        self.bridge.stop_bridge_server()
    }

    pub fn get_profiles(&self) -> Vec<ProfileInfo> {
        self.bridge.get_profiles()
    }

    pub fn create_profile(&self, name: String, config: BrowserProfileConfig) -> anyhow::Result<()> {
        self.bridge.create_profile(name, config)
    }

    pub fn delete_profile(&self, name: &str) -> anyhow::Result<()> {
        self.bridge.delete_profile(name)
    }

    pub fn set_active_profile(&self, name: &str) -> anyhow::Result<()> {
        self.bridge.set_active_profile(name)
    }

    pub fn get_active_profile(&self) -> Option<BrowserProfileConfig> {
        self.bridge.get_active_profile()
    }

    pub fn save_cookies(&self, profile_name: &str, cookies: Vec<CookieData>) -> anyhow::Result<std::path::PathBuf> {
        self.bridge.save_cookies(profile_name, cookies)
    }

    pub fn load_cookies(&self, profile_name: &str) -> anyhow::Result<Vec<CookieData>> {
        self.bridge.load_cookies(profile_name)
    }

    /// Check if agent-browser CLI is available
    pub async fn is_agent_browser_available() -> bool {
        Command::new("agent-browser")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Backward-compatible alias.
    pub async fn is_available() -> bool {
        Self::is_agent_browser_available().await
    }

    fn configured_backend(&self) -> anyhow::Result<BrowserBackendKind> {
        BrowserBackendKind::parse(&self.backend)
    }

    fn rust_native_compiled() -> bool {
        cfg!(feature = "browser-native")
    }

    fn rust_native_available(&self) -> bool {
        #[cfg(feature = "browser-native")]
        {
            native_backend::NativeBrowserState::is_available(
                self.native_headless,
                &self.native_webdriver_url,
                self.native_chrome_path.as_deref(),
            )
        }
        #[cfg(not(feature = "browser-native"))]
        {
            false
        }
    }

    fn computer_use_endpoint_url(&self) -> anyhow::Result<reqwest::Url> {
        if self.computer_use.timeout_ms == 0 {
            anyhow::bail!("browser.computer_use.timeout_ms must be > 0");
        }

        let endpoint = self.computer_use.endpoint.trim();
        if endpoint.is_empty() {
            anyhow::bail!("browser.computer_use.endpoint cannot be empty");
        }

        let parsed = reqwest::Url::parse(endpoint).map_err(|_| {
            anyhow::anyhow!(
                "Invalid browser.computer_use.endpoint: '{endpoint}'. Expected http(s) URL"
            )
        })?;

        let scheme = parsed.scheme();
        if scheme != "http" && scheme != "https" {
            anyhow::bail!("browser.computer_use.endpoint must use http:// or https://");
        }

        let host = parsed
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("browser.computer_use.endpoint must include host"))?;

        let host_is_private = is_private_host(host);
        if !self.computer_use.allow_remote_endpoint && !host_is_private {
            anyhow::bail!(
                "browser.computer_use.endpoint host '{host}' is public. Set browser.computer_use.allow_remote_endpoint=true to allow it"
            );
        }

        if self.computer_use.allow_remote_endpoint && !host_is_private && scheme != "https" {
            anyhow::bail!(
                "browser.computer_use.endpoint must use https:// when allow_remote_endpoint=true and host is public"
            );
        }

        Ok(parsed)
    }

    fn computer_use_available(&self) -> anyhow::Result<bool> {
        let endpoint = self.computer_use_endpoint_url()?;
        Ok(endpoint_reachable(&endpoint, Duration::from_millis(500)))
    }

    async fn resolve_backend(&self) -> anyhow::Result<ResolvedBackend> {
        let configured = self.configured_backend()?;

        match configured {
            BrowserBackendKind::AgentBrowser => {
                if Self::is_agent_browser_available().await {
                    Ok(ResolvedBackend::AgentBrowser)
                } else {
                    anyhow::bail!(
                        "browser.backend='{}' but agent-browser CLI is unavailable. Install with: npm install -g agent-browser",
                        configured.as_str()
                    )
                }
            }
            BrowserBackendKind::RustNative => {
                if !Self::rust_native_compiled() {
                    anyhow::bail!(
                        "browser.backend='rust_native' requires build feature 'browser-native'"
                    );
                }
                if !self.rust_native_available() {
                    anyhow::bail!(
                        "Rust-native browser backend is enabled but WebDriver endpoint is unreachable. Set browser.native_webdriver_url and start a compatible driver"
                    );
                }
                Ok(ResolvedBackend::RustNative)
            }
            BrowserBackendKind::ComputerUse => {
                if !self.computer_use_available()? {
                    anyhow::bail!(
                        "browser.backend='computer_use' but sidecar endpoint is unreachable. Check browser.computer_use.endpoint and sidecar status"
                    );
                }
                Ok(ResolvedBackend::ComputerUse)
            }
            BrowserBackendKind::Auto => {
                if Self::rust_native_compiled() && self.rust_native_available() {
                    return Ok(ResolvedBackend::RustNative);
                }
                if Self::is_agent_browser_available().await {
                    return Ok(ResolvedBackend::AgentBrowser);
                }

                let computer_use_err = match self.computer_use_available() {
                    Ok(true) => return Ok(ResolvedBackend::ComputerUse),
                    Ok(false) => None,
                    Err(err) => Some(err.to_string()),
                };

                if Self::rust_native_compiled() {
                    if let Some(err) = computer_use_err {
                        anyhow::bail!(
                            "browser.backend='auto' found no usable backend (agent-browser missing, rust-native unavailable, computer-use invalid: {err})"
                        );
                    }
                    anyhow::bail!(
                        "browser.backend='auto' found no usable backend (agent-browser missing, rust-native unavailable, computer-use sidecar unreachable)"
                    )
                }

                if let Some(err) = computer_use_err {
                    anyhow::bail!(
                        "browser.backend='auto' needs agent-browser CLI, browser-native, or valid computer-use sidecar (error: {err})"
                    );
                }

                anyhow::bail!(
                    "browser.backend='auto' needs agent-browser CLI, browser-native, or computer-use sidecar"
                )
            }
        }
    }

    /// Validate URL against allowlist
    fn validate_url(&self, url: &str) -> anyhow::Result<()> {
        let url = url.trim();

        if url.is_empty() {
            anyhow::bail!("URL cannot be empty");
        }

        if url.starts_with("file://") {
            anyhow::bail!("file:// URLs are not allowed in browser automation");
        }

        if !url.starts_with("https://") && !url.starts_with("http://") {
            anyhow::bail!("Only http:// and https:// URLs are allowed");
        }

        if self.allowed_domains.is_empty() {
            anyhow::bail!(
                "Browser tool enabled but no allowed_domains configured. \
                Add [browser].allowed_domains in config.toml"
            );
        }

        let host = extract_host(url)?;

        if is_private_host(&host) {
            anyhow::bail!("Blocked local/private host: {host}");
        }

        if !host_matches_allowlist(&host, &self.allowed_domains) {
            anyhow::bail!("Host '{host}' not in browser.allowed_domains");
        }

        Ok(())
    }

    /// Execute an agent-browser command
    async fn run_command(&self, args: &[&str]) -> anyhow::Result<AgentBrowserResponse> {
        let mut cmd = Command::new("agent-browser");

        if let Some(ref session) = self.session_name {
            cmd.arg("--session").arg(session);
        }

        cmd.args(args).arg("--json");

        debug!("Running: agent-browser {} --json", args.join(" "));

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stderr.is_empty() {
            debug!("agent-browser stderr: {}", stderr);
        }

        if let Ok(resp) = serde_json::from_str::<AgentBrowserResponse>(&stdout) {
            return Ok(resp);
        }

        if output.status.success() {
            Ok(AgentBrowserResponse {
                success: true,
                data: Some(json!({ "output": stdout.trim() })),
                error: None,
            })
        } else {
            Ok(AgentBrowserResponse {
                success: false,
                data: None,
                error: Some(stderr.trim().to_string()),
            })
        }
    }

    /// Execute a browser action via agent-browser CLI
    #[allow(clippy::too_many_lines)]
    async fn execute_agent_browser_action(
        &self,
        action: BrowserAction,
    ) -> anyhow::Result<ToolResult> {
        match action {
            BrowserAction::Open { url } => {
                self.validate_url(&url)?;
                let resp = self.run_command(&["open", &url]).await?;
                self.to_result(resp)
            }

            BrowserAction::Snapshot {
                interactive_only,
                compact,
                depth,
            } => {
                let mut args = vec!["snapshot"];
                if interactive_only {
                    args.push("-i");
                }
                if compact {
                    args.push("-c");
                }
                let depth_str;
                if let Some(d) = depth {
                    args.push("-d");
                    depth_str = d.to_string();
                    args.push(&depth_str);
                }
                let resp = self.run_command(&args).await?;
                self.to_result(resp)
            }

            BrowserAction::Click { selector } => {
                let resp = self.run_command(&["click", &selector]).await?;
                self.to_result(resp)
            }

            BrowserAction::Fill { selector, value } => {
                let resp = self.run_command(&["fill", &selector, &value]).await?;
                self.to_result(resp)
            }

            BrowserAction::Type { selector, text } => {
                let resp = self.run_command(&["type", &selector, &text]).await?;
                self.to_result(resp)
            }

            BrowserAction::GetText { selector } => {
                let resp = self.run_command(&["get", "text", &selector]).await?;
                self.to_result(resp)
            }

            BrowserAction::GetTitle => {
                let resp = self.run_command(&["get", "title"]).await?;
                self.to_result(resp)
            }

            BrowserAction::GetUrl => {
                let resp = self.run_command(&["get", "url"]).await?;
                self.to_result(resp)
            }

            BrowserAction::Screenshot { path, full_page } => {
                let mut args = vec!["screenshot"];
                if let Some(ref p) = path {
                    args.push(p);
                }
                if full_page {
                    args.push("--full");
                }
                let resp = self.run_command(&args).await?;
                self.to_result(resp)
            }

            BrowserAction::Wait { selector, ms, text } => {
                let mut args = vec!["wait"];
                let ms_str;
                if let Some(sel) = selector.as_ref() {
                    args.push(sel);
                } else if let Some(millis) = ms {
                    ms_str = millis.to_string();
                    args.push(&ms_str);
                } else if let Some(ref t) = text {
                    args.push("--text");
                    args.push(t);
                }
                let resp = self.run_command(&args).await?;
                self.to_result(resp)
            }

            BrowserAction::Press { key } => {
                let resp = self.run_command(&["press", &key]).await?;
                self.to_result(resp)
            }

            BrowserAction::Hover { selector } => {
                let resp = self.run_command(&["hover", &selector]).await?;
                self.to_result(resp)
            }

            BrowserAction::Scroll { direction, pixels } => {
                let mut args = vec!["scroll", &direction];
                let px_str;
                if let Some(px) = pixels {
                    px_str = px.to_string();
                    args.push(&px_str);
                }
                let resp = self.run_command(&args).await?;
                self.to_result(resp)
            }

            BrowserAction::IsVisible { selector } => {
                let resp = self.run_command(&["is", "visible", &selector]).await?;
                self.to_result(resp)
            }

            BrowserAction::Close => {
                let resp = self.run_command(&["close"]).await?;
                self.to_result(resp)
            }

            BrowserAction::Find {
                by,
                value,
                action,
                fill_value,
            } => {
                let mut args = vec!["find", &by, &value, &action];
                if let Some(ref fv) = fill_value {
                    args.push(fv);
                }
                let resp = self.run_command(&args).await?;
                self.to_result(resp)
            }
        }
    }

    #[allow(clippy::unused_async)]
    async fn execute_rust_native_action(
        &self,
        action: BrowserAction,
    ) -> anyhow::Result<ToolResult> {
        #[cfg(feature = "browser-native")]
        {
            let mut state = self.native_state.lock().await;

            let output: serde_json::Value = state
                .execute_action(
                    action,
                    self.native_headless,
                    &self.native_webdriver_url,
                    self.native_chrome_path.as_deref(),
                )
                .await?;

            Ok(ToolResult {
                success: true,
                output: serde_json::to_string_pretty(&output).unwrap_or_default(),
                error: None,
            })
        }

        #[cfg(not(feature = "browser-native"))]
        {
            let _ = action;
            anyhow::bail!(
                "Rust-native browser backend is not compiled. Rebuild with --features browser-native"
            )
        }
    }

    fn validate_coordinate(&self, key: &str, value: i64, max: Option<i64>) -> anyhow::Result<()> {
        if value < 0 {
            anyhow::bail!("'{key}' must be >= 0")
        }
        if let Some(limit) = max {
            if limit < 0 {
                anyhow::bail!("Configured coordinate limit for '{key}' must be >= 0")
            }
            if value > limit {
                anyhow::bail!("'{key}'={value} exceeds configured limit {limit}")
            }
        }
        Ok(())
    }

    fn read_required_i64(
        &self,
        params: &serde_json::Map<String, Value>,
        key: &str,
    ) -> anyhow::Result<i64> {
        params
            .get(key)
            .and_then(Value::as_i64)
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid '{key}' parameter"))
    }

    fn validate_computer_use_action(
        &self,
        action: &str,
        params: &serde_json::Map<String, Value>,
    ) -> anyhow::Result<()> {
        match action {
            "open" => {
                let url = params
                    .get("url")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("Missing 'url' for open action"))?;
                self.validate_url(url)?;
            }
            "mouse_move" | "mouse_click" => {
                let x = self.read_required_i64(params, "x")?;
                let y = self.read_required_i64(params, "y")?;
                self.validate_coordinate("x", x, self.computer_use.max_coordinate_x)?;
                self.validate_coordinate("y", y, self.computer_use.max_coordinate_y)?;
            }
            "mouse_drag" => {
                let from_x = self.read_required_i64(params, "from_x")?;
                let from_y = self.read_required_i64(params, "from_y")?;
                let to_x = self.read_required_i64(params, "to_x")?;
                let to_y = self.read_required_i64(params, "to_y")?;
                self.validate_coordinate("from_x", from_x, self.computer_use.max_coordinate_x)?;
                self.validate_coordinate("to_x", to_x, self.computer_use.max_coordinate_x)?;
                self.validate_coordinate("from_y", from_y, self.computer_use.max_coordinate_y)?;
                self.validate_coordinate("to_y", to_y, self.computer_use.max_coordinate_y)?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn execute_computer_use_action(
        &self,
        action: &str,
        args: &Value,
    ) -> anyhow::Result<ToolResult> {
        let endpoint = self.computer_use_endpoint_url()?;

        let mut params = args
            .as_object()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("browser args must be a JSON object"))?;
        params.remove("action");

        self.validate_computer_use_action(action, &params)?;

        let payload = json!({
            "action": action,
            "params": params,
            "policy": {
                "allowed_domains": self.allowed_domains,
                "window_allowlist": self.computer_use.window_allowlist,
                "max_coordinate_x": self.computer_use.max_coordinate_x,
                "max_coordinate_y": self.computer_use.max_coordinate_y,
            },
            "metadata": {
                "session_name": self.session_name,
                "source": "housaky.browser",
                "version": env!("CARGO_PKG_VERSION"),
            }
        });

        let client = reqwest::Client::new();
        let mut request = client
            .post(endpoint)
            .timeout(Duration::from_millis(self.computer_use.timeout_ms))
            .json(&payload);

        if let Some(api_key) = self.computer_use.api_key.as_deref() {
            let token = api_key.trim();
            if !token.is_empty() {
                request = request.bearer_auth(token);
            }
        }

        let response = request.send().await.with_context(|| {
            format!(
                "Failed to call computer-use sidecar at {}",
                self.computer_use.endpoint
            )
        })?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("Failed to read computer-use sidecar response body")?;

        if let Ok(parsed) = serde_json::from_str::<ComputerUseResponse>(&body) {
            if status.is_success() && parsed.success.unwrap_or(true) {
                let output = parsed
                    .data
                    .map(|data| serde_json::to_string_pretty(&data).unwrap_or_default())
                    .unwrap_or_else(|| {
                        serde_json::to_string_pretty(&json!({
                            "backend": "computer_use",
                            "action": action,
                            "ok": true,
                        }))
                        .unwrap_or_default()
                    });

                return Ok(ToolResult {
                    success: true,
                    output,
                    error: None,
                });
            }

            let error = parsed.error.or_else(|| {
                if status.is_success() && parsed.success == Some(false) {
                    Some("computer-use sidecar returned success=false".to_string())
                } else {
                    Some(format!(
                        "computer-use sidecar request failed with status {status}"
                    ))
                }
            });

            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error,
            });
        }

        if status.is_success() {
            return Ok(ToolResult {
                success: true,
                output: body,
                error: None,
            });
        }

        Ok(ToolResult {
            success: false,
            output: String::new(),
            error: Some(format!(
                "computer-use sidecar request failed with status {status}: {}",
                body.trim()
            )),
        })
    }

    async fn execute_action(
        &self,
        action: BrowserAction,
        backend: ResolvedBackend,
    ) -> anyhow::Result<ToolResult> {
        match backend {
            ResolvedBackend::AgentBrowser => self.execute_agent_browser_action(action).await,
            ResolvedBackend::RustNative => self.execute_rust_native_action(action).await,
            ResolvedBackend::ComputerUse => anyhow::bail!(
                "Internal error: computer_use backend must be handled before BrowserAction parsing"
            ),
        }
    }

    #[allow(clippy::unnecessary_wraps, clippy::unused_self)]
    fn to_result(&self, resp: AgentBrowserResponse) -> anyhow::Result<ToolResult> {
        if resp.success {
            let output = resp
                .data
                .map(|d| serde_json::to_string_pretty(&d).unwrap_or_default())
                .unwrap_or_default();
            Ok(ToolResult {
                success: true,
                output,
                error: None,
            })
        } else {
            Ok(ToolResult {
                success: false,
                output: String::new(),
                error: resp.error,
            })
        }
    }
}

#[allow(clippy::too_many_lines)]
#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "browser"
    }

    fn description(&self) -> &str {
        concat!(
            "Web/browser automation with pluggable backends (agent-browser, rust-native, computer_use). ",
            "Supports DOM actions plus optional OS-level actions (mouse_move, mouse_click, mouse_drag, ",
            "key_type, key_press, screen_capture) through a computer-use sidecar. Use 'snapshot' to map ",
            "interactive elements to refs (@e1, @e2). Enforces browser.allowed_domains for open actions."
        )
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["open", "snapshot", "click", "fill", "type", "get_text",
                             "get_title", "get_url", "screenshot", "wait", "press",
                             "hover", "scroll", "is_visible", "close", "find",
                             "mouse_move", "mouse_click", "mouse_drag", "key_type",
                             "key_press", "screen_capture"],
                    "description": "Browser action to perform (OS-level actions require backend=computer_use)"
                },
                "url": {
                    "type": "string",
                    "description": "URL to navigate to (for 'open' action)"
                },
                "selector": {
                    "type": "string",
                    "description": "Element selector: @ref (e.g. @e1), CSS (#id, .class), or text=..."
                },
                "value": {
                    "type": "string",
                    "description": "Value to fill or type"
                },
                "text": {
                    "type": "string",
                    "description": "Text to type or wait for"
                },
                "key": {
                    "type": "string",
                    "description": "Key to press (Enter, Tab, Escape, etc.)"
                },
                "x": {
                    "type": "integer",
                    "description": "Screen X coordinate (computer_use: mouse_move/mouse_click)"
                },
                "y": {
                    "type": "integer",
                    "description": "Screen Y coordinate (computer_use: mouse_move/mouse_click)"
                },
                "from_x": {
                    "type": "integer",
                    "description": "Drag source X coordinate (computer_use: mouse_drag)"
                },
                "from_y": {
                    "type": "integer",
                    "description": "Drag source Y coordinate (computer_use: mouse_drag)"
                },
                "to_x": {
                    "type": "integer",
                    "description": "Drag target X coordinate (computer_use: mouse_drag)"
                },
                "to_y": {
                    "type": "integer",
                    "description": "Drag target Y coordinate (computer_use: mouse_drag)"
                },
                "button": {
                    "type": "string",
                    "enum": ["left", "right", "middle"],
                    "description": "Mouse button for computer_use mouse_click"
                },
                "direction": {
                    "type": "string",
                    "enum": ["up", "down", "left", "right"],
                    "description": "Scroll direction"
                },
                "pixels": {
                    "type": "integer",
                    "description": "Pixels to scroll"
                },
                "interactive_only": {
                    "type": "boolean",
                    "description": "For snapshot: only show interactive elements"
                },
                "compact": {
                    "type": "boolean",
                    "description": "For snapshot: remove empty structural elements"
                },
                "depth": {
                    "type": "integer",
                    "description": "For snapshot: limit tree depth"
                },
                "full_page": {
                    "type": "boolean",
                    "description": "For screenshot: capture full page"
                },
                "path": {
                    "type": "string",
                    "description": "File path for screenshot"
                },
                "ms": {
                    "type": "integer",
                    "description": "Milliseconds to wait"
                },
                "by": {
                    "type": "string",
                    "enum": ["role", "text", "label", "placeholder", "testid"],
                    "description": "For find: semantic locator type"
                },
                "find_action": {
                    "type": "string",
                    "enum": ["click", "fill", "text", "hover", "check"],
                    "description": "For find: action to perform on found element"
                },
                "fill_value": {
                    "type": "string",
                    "description": "For find with fill action: value to fill"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        if !self.security.can_act() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: rate limit exceeded".into()),
            });
        }

        let backend = match self.resolve_backend().await {
            Ok(selected) => selected,
            Err(error) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(error.to_string()),
                });
            }
        };

        let action_str = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'action' parameter"))?;

        if !is_supported_browser_action(action_str) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown action: {action_str}")),
            });
        }

        if backend == ResolvedBackend::ComputerUse {
            return self.execute_computer_use_action(action_str, &args).await;
        }

        let action = match action_str {
            "open" => {
                let url = args
                    .get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'url' for open action"))?;
                BrowserAction::Open { url: url.into() }
            }
            "snapshot" => BrowserAction::Snapshot {
                interactive_only: args
                    .get("interactive_only")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true),
                compact: args
                    .get("compact")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true),
                depth: args
                    .get("depth")
                    .and_then(serde_json::Value::as_u64)
                    .map(|d| u32::try_from(d).unwrap_or(u32::MAX)),
            },
            "click" => {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'selector' for click"))?;
                BrowserAction::Click {
                    selector: selector.into(),
                }
            }
            "fill" => {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'selector' for fill"))?;
                let value = args
                    .get("value")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'value' for fill"))?;
                BrowserAction::Fill {
                    selector: selector.into(),
                    value: value.into(),
                }
            }
            "type" => {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'selector' for type"))?;
                let text = args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'text' for type"))?;
                BrowserAction::Type {
                    selector: selector.into(),
                    text: text.into(),
                }
            }
            "get_text" => {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'selector' for get_text"))?;
                BrowserAction::GetText {
                    selector: selector.into(),
                }
            }
            "get_title" => BrowserAction::GetTitle,
            "get_url" => BrowserAction::GetUrl,
            "screenshot" => BrowserAction::Screenshot {
                path: args.get("path").and_then(|v| v.as_str()).map(String::from),
                full_page: args
                    .get("full_page")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            },
            "wait" => BrowserAction::Wait {
                selector: args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                ms: args.get("ms").and_then(serde_json::Value::as_u64),
                text: args.get("text").and_then(|v| v.as_str()).map(String::from),
            },
            "press" => {
                let key = args
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'key' for press"))?;
                BrowserAction::Press { key: key.into() }
            }
            "hover" => {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'selector' for hover"))?;
                BrowserAction::Hover {
                    selector: selector.into(),
                }
            }
            "scroll" => {
                let direction = args
                    .get("direction")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'direction' for scroll"))?;
                BrowserAction::Scroll {
                    direction: direction.into(),
                    pixels: args
                        .get("pixels")
                        .and_then(serde_json::Value::as_u64)
                        .map(|p| u32::try_from(p).unwrap_or(u32::MAX)),
                }
            }
            "is_visible" => {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'selector' for is_visible"))?;
                BrowserAction::IsVisible {
                    selector: selector.into(),
                }
            }
            "close" => BrowserAction::Close,
            "find" => {
                let by = args
                    .get("by")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'by' for find"))?;
                let value = args
                    .get("value")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'value' for find"))?;
                let action = args
                    .get("find_action")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'find_action' for find"))?;
                BrowserAction::Find {
                    by: by.into(),
                    value: value.into(),
                    action: action.into(),
                    fill_value: args
                        .get("fill_value")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                }
            }
            _ => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!(
                        "Action '{action_str}' is unavailable for backend '{}'",
                        match backend {
                            ResolvedBackend::AgentBrowser => "agent_browser",
                            ResolvedBackend::RustNative => "rust_native",
                            ResolvedBackend::ComputerUse => "computer_use",
                        }
                    )),
                });
            }
        };

        self.execute_action(action, backend).await
    }
}

#[cfg(feature = "browser-native")]
mod native_backend {
    use super::BrowserAction;
    use anyhow::{Context, Result};
    use base64::Engine;
    use fantoccini::{Client, ClientBuilder, Locator};
    use serde_json::{json, Value};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    #[derive(Default)]
    pub struct NativeBrowserState {
        client: Option<Client>,
    }

    impl NativeBrowserState {
        pub fn is_available(
            _headless: bool,
            webdriver_url: &str,
            _chrome_path: Option<&str>,
        ) -> bool {
            webdriver_endpoint_reachable(webdriver_url, Duration::from_millis(500))
        }

        #[allow(clippy::too_many_lines)]
        pub async fn execute_action(
            &mut self,
            action: BrowserAction,
            headless: bool,
            webdriver_url: &str,
            _chrome_path: Option<&str>,
        ) -> Result<Value> {
            match action {
                BrowserAction::Open { url } => {
                    self.ensure_session(headless, webdriver_url, _chrome_path)
                        .await?;
                    let client = self.active_client()?;
                    client
                        .goto(&url)
                        .await
                        .with_context(|| format!("Failed to open URL: {url}"))?;
                    let current_url = client
                        .current_url()
                        .await
                        .context("Failed to read current URL after navigation")?;

                    Ok(json!({
                        "backend": "rust_native",
                        "action": "open",
                        "url": current_url.as_str(),
                    }))
                }
                BrowserAction::Snapshot {
                    interactive_only,
                    compact,
                    depth,
                } => {
                    let client = self.active_client()?;
                    let snapshot = client
                        .execute(
                            &snapshot_script(interactive_only, compact, depth.map(i64::from)),
                            vec![],
                        )
                        .await
                        .context("Failed to evaluate snapshot script")?;

                    Ok(json!({
                        "backend": "rust_native",
                        "action": "snapshot",
                        "data": snapshot,
                    }))
                }
                BrowserAction::Click { selector } => {
                    let client = self.active_client()?;
                    find_element(client, &selector).await?.click().await?;

                    Ok(json!({
                        "backend": "rust_native",
                        "action": "click",
                        "selector": selector,
                    }))
                }
                BrowserAction::Fill { selector, value } => {
                    let client = self.active_client()?;
                    let element = find_element(client, &selector).await?;
                    let _ = element.clear().await;
                    element.send_keys(&value).await?;

                    Ok(json!({
                        "backend": "rust_native",
                        "action": "fill",
                        "selector": selector,
                    }))
                }
                BrowserAction::Type { selector, text } => {
                    let client = self.active_client()?;
                    find_element(client, &selector)
                        .await?
                        .send_keys(&text)
                        .await?;

                    Ok(json!({
                        "backend": "rust_native",
                        "action": "type",
                        "selector": selector,
                        "typed": text.len(),
                    }))
                }
                BrowserAction::GetText { selector } => {
                    let client = self.active_client()?;
                    let text = find_element(client, &selector).await?.text().await?;

                    Ok(json!({
                        "backend": "rust_native",
                        "action": "get_text",
                        "selector": selector,
                        "text": text,
                    }))
                }
                BrowserAction::GetTitle => {
                    let client = self.active_client()?;
                    let title = client.title().await.context("Failed to read page title")?;

                    Ok(json!({
                        "backend": "rust_native",
                        "action": "get_title",
                        "title": title,
                    }))
                }
                BrowserAction::GetUrl => {
                    let client = self.active_client()?;
                    let url = client
                        .current_url()
                        .await
                        .context("Failed to read current URL")?;

                    Ok(json!({
                        "backend": "rust_native",
                        "action": "get_url",
                        "url": url.as_str(),
                    }))
                }
                BrowserAction::Screenshot { path, full_page } => {
                    let client = self.active_client()?;
                    let png = client
                        .screenshot()
                        .await
                        .context("Failed to capture screenshot")?;
                    let mut payload = json!({
                        "backend": "rust_native",
                        "action": "screenshot",
                        "full_page": full_page,
                        "bytes": png.len(),
                    });

                    if let Some(path_str) = path {
                        std::fs::write(&path_str, &png)
                            .with_context(|| format!("Failed to write screenshot to {path_str}"))?;
                        payload["path"] = Value::String(path_str);
                    } else {
                        payload["png_base64"] =
                            Value::String(base64::engine::general_purpose::STANDARD.encode(&png));
                    }

                    Ok(payload)
                }
                BrowserAction::Wait { selector, ms, text } => {
                    let client = self.active_client()?;
                    if let Some(sel) = selector.as_ref() {
                        wait_for_selector(client, sel).await?;
                        Ok(json!({
                            "backend": "rust_native",
                            "action": "wait",
                            "selector": sel,
                        }))
                    } else if let Some(duration_ms) = ms {
                        tokio::time::sleep(Duration::from_millis(duration_ms)).await;
                        Ok(json!({
                            "backend": "rust_native",
                            "action": "wait",
                            "ms": duration_ms,
                        }))
                    } else if let Some(needle) = text.as_ref() {
                        let xpath = xpath_contains_text(needle);
                        client
                            .wait()
                            .for_element(Locator::XPath(&xpath))
                            .await
                            .with_context(|| {
                                format!("Timed out waiting for text to appear: {needle}")
                            })?;
                        Ok(json!({
                            "backend": "rust_native",
                            "action": "wait",
                            "text": needle,
                        }))
                    } else {
                        tokio::time::sleep(Duration::from_millis(250)).await;
                        Ok(json!({
                            "backend": "rust_native",
                            "action": "wait",
                            "ms": 250,
                        }))
                    }
                }
                BrowserAction::Press { key } => {
                    let client = self.active_client()?;
                    let key_input = webdriver_key(&key);
                    match client.active_element().await {
                        Ok(element) => {
                            element.send_keys(&key_input).await?;
                        }
                        Err(_) => {
                            find_element(client, "body")
                                .await?
                                .send_keys(&key_input)
                                .await?;
                        }
                    }

                    Ok(json!({
                        "backend": "rust_native",
                        "action": "press",
                        "key": key,
                    }))
                }
                BrowserAction::Hover { selector } => {
                    let client = self.active_client()?;
                    let element = find_element(client, &selector).await?;

                    // Fantoccini action API changed across versions; use JS-based hover for stability.
                    // We pass the element handle directly as a JS argument.
                    let script = r#"
                        const el = arguments[0];
                        if (!el) return false;
                        el.scrollIntoView({block: 'center', inline: 'center'});
                        const ev = new MouseEvent('mouseover', { bubbles: true, cancelable: true, view: window });
                        el.dispatchEvent(ev);
                        return true;
                    "#;
                    client
                        .execute(
                            script,
                            vec![serde_json::to_value(&element)
                                .context("Failed to serialize element for JS")?],
                        )
                        .await
                        .context("Failed to execute hover script")?;

                    Ok(json!({
                        "backend": "rust_native",
                        "action": "hover",
                        "selector": selector,
                    }))
                }
                BrowserAction::Scroll { direction, pixels } => {
                    let client = self.active_client()?;
                    let scroll_js = match direction.as_str() {
                        "up" => format!("window.scrollBy(0, -{})", pixels.unwrap_or(300)),
                        "down" => format!("window.scrollBy(0, {})", pixels.unwrap_or(300)),
                        "left" => format!("window.scrollBy(-{}, 0)", pixels.unwrap_or(300)),
                        "right" => format!("window.scrollBy({}, 0)", pixels.unwrap_or(300)),
                        _ => format!("window.scrollBy(0, {})", pixels.unwrap_or(300)),
                    };
                    client.execute(&scroll_js, vec![]).await?;

                    Ok(json!({
                        "backend": "rust_native",
                        "action": "scroll",
                        "direction": direction,
                    }))
                }
                BrowserAction::IsVisible { selector } => {
                    let client = self.active_client()?;
                    let element = find_element(client, &selector).await?;
                    let visible = element.is_displayed().await?;

                    Ok(json!({
                        "backend": "rust_native",
                        "action": "is_visible",
                        "selector": selector,
                        "visible": visible,
                    }))
                }
                BrowserAction::Close => {
                    if let Some(client) = self.client.take() {
                        client.close().await?;
                    }
                    Ok(json!({
                        "backend": "rust_native",
                        "action": "close",
                    }))
                }
                BrowserAction::Find { .. } => {
                    anyhow::bail!("Find action requires agent-browser backend")
                }
            }
        }

        fn active_client(&self) -> Result<&Client> {
            self.client
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("No active browser session"))
        }

        async fn ensure_session(
            &mut self,
            headless: bool,
            webdriver_url: &str,
            _chrome_path: Option<&str>,
        ) -> Result<()> {
            if self.client.is_none() {
                let mut builder = ClientBuilder::rustls().context("Failed to create rustls connector")?;

                if headless {
                    // Chrome options structure accepted by webdriver.
                    let args = vec![
                        "--no-sandbox",
                        "--disable-dev-shm-usage",
                        "--disable-gpu",
                        "--headless=new",
                    ];
                    let chrome_options = json!({ "args": args });
                    let caps = json!({
                        "browserName": "chrome",
                        "goog:chromeOptions": chrome_options
                    });
                    builder.capabilities(caps.as_object().cloned().unwrap_or_default());
                }

                let webdriver_url = webdriver_url.trim_end_matches('/');
                let client = builder.connect(webdriver_url).await
                    .with_context(|| format!("Failed to connect to WebDriver at {webdriver_url}"))?;

                self.client = Some(client);
            }
            Ok(())
        }
    }

    async fn find_element(
        client: &Client,
        selector: &str,
    ) -> Result<fantoccini::elements::Element> {
        if let Some(text) = selector.strip_prefix("text=") {
            let xpath = xpath_contains_text(text);
            client
                .wait()
                .for_element(Locator::XPath(&xpath))
                .await
                .with_context(|| format!("Failed to find element with text: {text}"))
        } else if selector.starts_with('@') && !selector.starts_with("@@") {
            let ref_id = &selector[1..];
            let xpath = format!(r#"//*[@data-ref='{}']"#, ref_id);
            client
                .wait()
                .for_element(Locator::XPath(&xpath))
                .await
                .with_context(|| format!("Failed to find element with ref: {ref_id}"))
        } else {
            client
                .wait()
                .for_element(Locator::Css(selector))
                .await
                .with_context(|| format!("Failed to find element with selector: {selector}"))
        }
    }

    async fn wait_for_selector(client: &Client, selector: &str) -> Result<()> {
        find_element(client, selector)
            .await
            .map(|_| ())
            .context("Timeout waiting for selector")
    }

    fn snapshot_script(interactive_only: bool, compact: bool, depth: Option<i64>) -> String {
        format!(
            r#"
            (function() {{
                const interactiveOnly = {};
                const compact = {};
                const maxDepth = {};

                function isInteractive(el) {{
                    const role = el.getAttribute('role');
                    const tag = el.tagName.toLowerCase();
                    if (['button', 'a', 'input', 'select', 'textarea', 'checkbox', 'radio'].includes(tag)) return true;
                    if (role && ['button', 'link', 'menuitem', 'checkbox', 'radio', 'textbox', 'combobox'].includes(role)) return true;
                    if (el.isContentEditable) return true;
                    const style = window.getComputedStyle(el);
                    if (style && style.pointerEvents !== 'none' && el.tabIndex >= 0) return true;
                    return false;
                }}

                function buildTree(el, depth = 0) {{
                    if (maxDepth && depth > maxDepth) return null;

                    const children = [];
                    for (const child of el.children) {{
                        const subtree = buildTree(child, depth + 1);
                        if (subtree) children.push(subtree);
                    }}

                    const info = {{
                        tag: el.tagName.toLowerCase(),
                        id: el.id || null,
                        classes: el.className.split(' ').filter(c => c) || null,
                        text: el.textContent?.trim().substring(0, 100) || null,
                        ref: el.getAttribute('data-ref'),
                        role: el.getAttribute('role'),
                        type: el.getAttribute('type'),
                        name: el.getAttribute('name'),
                        placeholder: el.getAttribute('placeholder'),
                        value: el.value || null,
                        checked: el.checked || null,
                        href: el.getAttribute('href'),
                        src: el.getAttribute('src'),
                        interactive: isInteractive(el),
                    }};

                    if (compact && !info.interactive && children.length === 0 && !info.text) return null;

                    if (children.length > 0) info.children = children;
                    return info;
                }}

                return buildTree(document.body);
            }})()
            "#,
            interactive_only,
            compact,
            depth.map(|d| d.to_string()).unwrap_or_else(|| "null".to_string())
        )
    }

    fn xpath_contains_text(text: &str) -> String {
        format!(
            "//*[contains(text(), '{}')]",
            text.replace("'", "\\'").replace("\"", "\\\"")
        )
    }

    fn webdriver_key(key: &str) -> String {
        match key {
            "Enter" => "\u{E007}".to_string(),
            "Tab" => "\u{E004}".to_string(),
            "Escape" => "\u{E00C}".to_string(),
            "Backspace" => "\u{E003}".to_string(),
            "Delete" => "\u{E017}".to_string(),
            "ArrowUp" => "\u{E013}".to_string(),
            "ArrowDown" => "\u{E014}".to_string(),
            "ArrowLeft" => "\u{E012}".to_string(),
            "ArrowRight" => "\u{E011}".to_string(),
            "Home" => "\u{E010}".to_string(),
            "End" => "\u{E011}".to_string(),
            "PageUp" => "\u{E00E}".to_string(),
            "PageDown" => "\u{E00F}".to_string(),
            _ => key.to_string(),
        }
    }

    fn webdriver_endpoint_reachable(endpoint: &str, timeout: Duration) -> bool {
        let addr = match endpoint.strip_prefix("http://") {
            Some(host) => host.split('/').next().unwrap_or(host),
            None => endpoint.split('/').next().unwrap_or(endpoint),
        };

        if let Ok(socket_addrs) = addr.to_socket_addrs() {
            for socket_addr in socket_addrs {
                if TcpStream::connect_timeout(&socket_addr, timeout).is_ok() {
                    return true;
                }
            }
        }
        false
    }

    fn endpoint_reachable(url: &reqwest::Url, timeout: Duration) -> bool {
        let host = url.host_str().unwrap_or("localhost");
        let port = url.port().unwrap_or(80);
        let addr = format!("{}:{}", host, port);

        if let Ok(socket_addrs) = addr.to_socket_addrs() {
            for socket_addr in socket_addrs {
                if std::net::TcpStream::connect_timeout(&socket_addr, timeout).is_ok() {
                    return true;
                }
            }
        }
        false
    }

    fn normalize_domains(domains: Vec<String>) -> Vec<String> {
        if domains.is_empty() {
            return domains;
        }

        let mut normalized: Vec<String> = domains
            .iter()
            .filter_map(|d| {
                let d = d.trim().to_lowercase();
                if d.is_empty() {
                    None
                } else if d.starts_with("http://") {
                    d.strip_prefix("http://").map(String::from)
                } else if d.starts_with("https://") {
                    d.strip_prefix("https://").map(String::from)
                } else {
                    Some(d)
                }
            })
            .collect();

        normalized.sort();
        normalized.dedup();
        normalized
    }

    fn extract_host(url: &str) -> anyhow::Result<String> {
        let url = url.trim();
        let without_scheme = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(url);

        let host = without_scheme
            .split('/')
            .next()
            .ok_or_else(|| anyhow::anyhow!("Invalid URL: no host"))?
            .split(':')
            .next()
            .ok_or_else(|| anyhow::anyhow!("Invalid URL: no host"))?
            .to_string();

        Ok(host)
    }

    fn is_private_host(host: &str) -> bool {
        host == "localhost"
            || host == "127.0.0.1"
            || host == "::1"
            || host.starts_with("10.")
            || host.starts_with("192.168.")
            || host.starts_with("172.16.")
            || host.starts_with("172.17.")
            || host.starts_with("172.18.")
            || host.starts_with("172.19.")
            || host.starts_with("172.2")
            || host.starts_with("172.30.")
            || host.starts_with("172.31.")
            || host.ends_with(".local")
    }

    fn host_matches_allowlist(host: &str, allowlist: &[String]) -> bool {
        let host = host.to_lowercase();
        allowlist.iter().any(|domain| {
            let domain = domain.to_lowercase();
            domain == host
                || host.ends_with(&format!(".{}", domain))
                || domain == "*"
        })
    }

    fn is_supported_browser_action(action: &str) -> bool {
        matches!(
            action,
            "open" | "snapshot" | "click" | "fill" | "type" | "get_text"
                | "get_title" | "get_url" | "screenshot" | "wait" | "press"
                | "hover" | "scroll" | "is_visible" | "close" | "find"
                | "mouse_move" | "mouse_click" | "mouse_drag" | "key_type"
                | "key_press" | "screen_capture"
        )
    }
}

#[cfg(not(feature = "browser-native"))]
mod native_backend {
    use serde_json::Value;

    pub struct NativeBrowserState;

    impl NativeBrowserState {
        pub fn is_available(_headless: bool, _webdriver_url: &str, _chrome_path: Option<&str>) -> bool {
            false
        }

        pub async fn execute_action(
            &mut self,
            _action: super::BrowserAction,
            _headless: bool,
            _webdriver_url: &str,
            _chrome_path: Option<&str>,
        ) -> anyhow::Result<Value> {
            anyhow::bail!("browser-native not compiled")
        }
    }
}

fn endpoint_reachable(url: &reqwest::Url, timeout: Duration) -> bool {
    let host = url.host_str().unwrap_or("localhost");
    let port = url.port().unwrap_or(80);
    let addr = format!("{}:{}", host, port);

    if let Ok(socket_addrs) = addr.to_socket_addrs() {
        for socket_addr in socket_addrs {
            if std::net::TcpStream::connect_timeout(&socket_addr, timeout).is_ok() {
                return true;
            }
        }
    }
    false
}

fn normalize_domains(domains: Vec<String>) -> Vec<String> {
    if domains.is_empty() {
        return domains;
    }

    let mut normalized: Vec<String> = domains
        .iter()
        .filter_map(|d| {
            let d = d.trim().to_lowercase();
            if d.is_empty() {
                None
            } else if d.starts_with("http://") {
                d.strip_prefix("http://").map(String::from)
            } else if d.starts_with("https://") {
                d.strip_prefix("https://").map(String::from)
            } else {
                Some(d)
            }
        })
        .collect();

    normalized.sort();
    normalized.dedup();
    normalized
}

fn extract_host(url: &str) -> anyhow::Result<String> {
    let url = url.trim();
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    let host = without_scheme
        .split('/')
        .next()
        .ok_or_else(|| anyhow::anyhow!("Invalid URL: no host"))?
        .split(':')
        .next()
        .ok_or_else(|| anyhow::anyhow!("Invalid URL: no host"))?
        .to_string();

    Ok(host)
}

fn is_private_host(host: &str) -> bool {
    host == "localhost"
        || host == "127.0.0.1"
        || host == "::1"
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host.starts_with("172.16.")
        || host.starts_with("172.17.")
        || host.starts_with("172.18.")
        || host.starts_with("172.19.")
        || host.starts_with("172.2")
        || host.starts_with("172.30.")
        || host.starts_with("172.31.")
        || host.ends_with(".local")
}

fn host_matches_allowlist(host: &str, allowlist: &[String]) -> bool {
    let host = host.to_lowercase();
    allowlist.iter().any(|domain| {
        let domain = domain.to_lowercase();
        domain == host
            || host.ends_with(&format!(".{}", domain))
            || domain == "*"
    })
}

fn is_supported_browser_action(action: &str) -> bool {
    matches!(
        action,
        "open" | "snapshot" | "click" | "fill" | "type" | "get_text"
            | "get_title" | "get_url" | "screenshot" | "wait" | "press"
            | "hover" | "scroll" | "is_visible" | "close" | "find"
            | "mouse_move" | "mouse_click" | "mouse_drag" | "key_type"
            | "key_press" | "screen_capture"
    )
}

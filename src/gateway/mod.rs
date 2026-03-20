//! Axum-based HTTP gateway with proper HTTP/1.1 compliance, body limits, and timeouts.
//!
//! This module replaces the raw TCP implementation with axum for:
//! - Proper HTTP/1.1 parsing and compliance
//! - Content-Length validation (handled by hyper)
//! - Request body size limits (64KB max)
//! - Request timeouts (30s) to prevent slow-loris attacks
//! - Header sanitization (handled by axum/hyper)
//! - OpenClaw-style WebSocket gateway for real-time communication
//! - REST API for config, MCP, chat, agent, skills, channels, keys, A2A, hardware, doctor

pub mod session_storage;

// use crate::channels::Channel;
// use crate::channels::WhatsAppChannel;
use crate::config::Config;
use crate::memory::{self, Memory, MemoryCategory};
use crate::providers::{self, Provider};
use crate::security::pairing::{constant_time_eq, is_public_bind, PairingGuard};
use crate::util::truncate_with_ellipsis;
use anyhow::Result;
use axum::{
    extract::{State, WebSocketUpgrade, ws::{Message as WsMessage, WebSocket}},
    http::{header, HeaderMap, StatusCode},
    response::{sse::Event, sse::Sse, IntoResponse},
    routing::{get, post, put, delete},
    Json, Router,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;
use uuid::Uuid;

/// Maximum request body size (64KB) — prevents memory exhaustion
pub const MAX_BODY_SIZE: usize = 65_536;
/// Request timeout (30s) — prevents slow-loris attacks
pub const REQUEST_TIMEOUT_SECS: u64 = 30;
/// Sliding window used by gateway rate limiting.
pub const RATE_LIMIT_WINDOW_SECS: u64 = 60;

/// Allowed origins for WebSocket connections (localhost only for security)
const ALLOWED_WS_ORIGINS: &[&str] = &[
    "http://localhost:3000",
    "http://127.0.0.1:3000",
    "http://localhost:8080",
    "http://127.0.0.1:8080",
];

/// TLS configuration for HTTPS
#[derive(Debug, Clone, Default)]
pub struct TlsConfig {
    pub cert_path: Option<std::path::PathBuf>,
    pub key_path: Option<std::path::PathBuf>,
}

impl TlsConfig {
    pub fn is_enabled(&self) -> bool {
        self.cert_path.is_some() && self.key_path.is_some()
    }
}

fn webhook_memory_key() -> String {
    format!("webhook_msg_{}", Uuid::new_v4())
}

fn whatsapp_memory_key(msg: &crate::channels::traits::ChannelMessage) -> String {
    format!("whatsapp_{}_{}", msg.sender, msg.id)
}

/// How often the rate limiter sweeps stale IP entries from its map.
const RATE_LIMITER_SWEEP_INTERVAL_SECS: u64 = 300; // 5 minutes

#[derive(Debug)]
struct SlidingWindowRateLimiter {
    limit_per_window: u32,
    window: Duration,
    requests: Mutex<(HashMap<String, Vec<Instant>>, Instant)>,
}

impl SlidingWindowRateLimiter {
    fn new(limit_per_window: u32, window: Duration) -> Self {
        Self {
            limit_per_window,
            window,
            requests: Mutex::new((HashMap::new(), Instant::now())),
        }
    }

    fn allow(&self, key: &str) -> bool {
        if self.limit_per_window == 0 {
            return true;
        }

        let now = Instant::now();
        let cutoff = now.checked_sub(self.window).unwrap_or_else(Instant::now);

        let mut guard = self
            .requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (requests, last_sweep) = &mut *guard;

        // Periodic sweep: remove IPs with no recent requests
        if last_sweep.elapsed() >= Duration::from_secs(RATE_LIMITER_SWEEP_INTERVAL_SECS) {
            requests.retain(|_, timestamps| {
                timestamps.retain(|t| *t > cutoff);
                !timestamps.is_empty()
            });
            *last_sweep = now;
        }

        let entry = requests.entry(key.to_owned()).or_default();
        entry.retain(|instant| *instant > cutoff);

        if entry.len() >= self.limit_per_window as usize {
            return false;
        }

        entry.push(now);
        true
    }
}

#[derive(Debug)]
pub struct GatewayRateLimiter {
    pair: SlidingWindowRateLimiter,
    webhook: SlidingWindowRateLimiter,
}

impl GatewayRateLimiter {
    fn new(pair_per_minute: u32, webhook_per_minute: u32) -> Self {
        let window = Duration::from_secs(RATE_LIMIT_WINDOW_SECS);
        Self {
            pair: SlidingWindowRateLimiter::new(pair_per_minute, window),
            webhook: SlidingWindowRateLimiter::new(webhook_per_minute, window),
        }
    }

    fn allow_pair(&self, key: &str) -> bool {
        self.pair.allow(key)
    }

    fn allow_webhook(&self, key: &str) -> bool {
        self.webhook.allow(key)
    }
}

#[derive(Debug)]
pub struct IdempotencyStore {
    ttl: Duration,
    keys: Mutex<HashMap<String, Instant>>,
}

impl IdempotencyStore {
    fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            keys: Mutex::new(HashMap::new()),
        }
    }

    /// Returns true if this key is new and is now recorded.
    fn record_if_new(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut keys = self
            .keys
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        keys.retain(|_, seen_at| now.duration_since(*seen_at) < self.ttl);

        if keys.contains_key(key) {
            return false;
        }

        keys.insert(key.to_owned(), now);
        true
    }
}

fn client_key_from_headers(headers: &HeaderMap) -> String {
    for header_name in ["X-Forwarded-For", "X-Real-IP"] {
        if let Some(value) = headers.get(header_name).and_then(|v| v.to_str().ok()) {
            let first = value.split(',').next().unwrap_or("").trim();
            if !first.is_empty() {
                return first.to_owned();
            }
        }
    }
    "unknown".into()
}

/// Chat session for dashboard
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub last_message: String,
    pub timestamp: i64,
    pub message_count: usize,
}

/// Chat message for dashboard
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: i64,
    pub token_count: Option<usize>,
}

/// Chat request body
#[derive(serde::Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub session_key: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub stream: Option<bool>,
}

/// Chat response
#[derive(serde::Serialize)]
pub struct ChatResponse {
    pub message: ChatMessage,
    pub session_id: String,
    pub model: String,
}

/// MCP server info
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
    pub status: String,
    pub tools_count: usize,
}

/// Shared state for all axum handlers
#[derive(Clone)]
pub struct AppState {
    pub provider: Arc<dyn Provider>,
    pub model: String,
    pub temperature: f64,
    pub mem: Arc<dyn Memory>,
    pub auto_save: bool,
    pub webhook_secret: Option<Arc<str>>,
    pub pairing: Arc<PairingGuard>,
    pub rate_limiter: Arc<GatewayRateLimiter>,
    pub idempotency_store: Arc<IdempotencyStore>,
    // pub whatsapp: Option<Arc<WhatsAppChannel>>,
    /// `WhatsApp` app secret for webhook signature verification (`X-Hub-Signature-256`)
    // pub whatsapp_app_secret: Option<Arc<str>>,
    pub events: broadcast::Sender<GatewayEvent>,
    pub config_path: Arc<std::path::PathBuf>,
    pub workspace_dir: Arc<std::path::PathBuf>,
    pub start_time: Instant,
    pub default_provider: Option<String>,
    pub memory_config: crate::config::MemoryConfig,
    pub autonomy_config: crate::config::AutonomyConfig,
    pub heartbeat_config: crate::config::HeartbeatConfig,
    pub channels_config: crate::config::ChannelsConfig,
    pub secrets_config: crate::config::SecretsConfig,
    /// Session storage for chat history
    pub sessions: Arc<std::sync::Mutex<session_storage::SessionStorage>>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GatewayEvent {
    Paired {
        client_key: String,
    },
    WebhookReceived {
        client_key: String,
        message_preview: String,
    },
    WebhookResponded {
        model: String,
        success: bool,
    },
}

/// Run the HTTP gateway using axum with proper HTTP/1.1 compliance.
#[allow(clippy::too_many_lines)]
pub async fn run_gateway(host: &str, port: u16, config: Config) -> Result<()> {
    // ── Security: refuse public bind without tunnel or explicit opt-in ──
    if is_public_bind(host) && config.tunnel.provider == "none" && !config.gateway.allow_public_bind
    {
        anyhow::bail!(
            "🛑 Refusing to bind to {host} — gateway would be exposed to the internet.\n\
             Fix: use --host 127.0.0.1 (default), configure a tunnel, or set\n\
             [gateway] allow_public_bind = true in config.toml (NOT recommended)."
        );
    }

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let actual_port = listener.local_addr()?.port();
    let display_addr = format!("{host}:{actual_port}");

    let provider: Arc<dyn Provider> = Arc::from(providers::create_resilient_provider(
        config.default_provider.as_deref().unwrap_or("openrouter"),
        config.api_key.as_deref(),
        &config.reliability,
        None,
    )?);
    let model = config
        .default_model
        .clone()
        .unwrap_or_else(|| "anthropic/claude-sonnet-4".into());
    let temperature = config.default_temperature;
    let mem: Arc<dyn Memory> = Arc::from(memory::create_memory(
        &config.memory,
        &config.workspace_dir,
        config.api_key.as_deref(),
    )?);

    // Extract webhook secret for authentication
    let webhook_secret: Option<Arc<str>> = config
        .channels_config
        .webhook
        .as_ref()
        .and_then(|w| w.secret.as_deref())
        .map(Arc::from);

    // WhatsApp channel not yet implemented
    // let whatsapp_channel: Option<Arc<WhatsAppChannel>> =
    //     config.channels_config.whatsapp.as_ref().map(|wa| {
    //         Arc::new(WhatsAppChannel::new(crate::channels::whatsapp::WhatsAppConfig {
    //             mode: wa.mode.clone().unwrap_or_default(),
    //             access_token: wa.access_token.clone(),
    //             phone_number_id: wa.phone_number_id.clone(),
    //             verify_token: wa.verify_token.clone(),
    //             auth_dir: wa.auth_dir.clone(),
    //             session_name: wa.session_name.clone(),
    //             dm_policy: wa.dm_policy.clone().unwrap_or_else(|| "pairing".to_string()),
    //             group_policy: wa.group_policy.clone().unwrap_or_else(|| "mention".to_string()),
    //             allowed_numbers: wa.allowed_numbers.clone(),
    //             allowed_groups: wa.allowed_groups.clone(),
    //             app_secret: None,
    //         }))
    //     });

    // WhatsApp app secret for webhook signature verification
    // Priority: environment variable > config file
    // let whatsapp_app_secret: Option<Arc<str>> = std::env::var("HOUSAKY_WHATSAPP_APP_SECRET")
    //     .ok()
    //     .and_then(|secret| {
    //         let secret = secret.trim();
    //         (!secret.is_empty()).then(|| secret.to_owned())
    //     })
    //     .or_else(|| {
    //         config.channels_config.whatsapp.as_ref().and_then(|wa| {
    //             wa.app_secret
    //                 .as_deref()
    //                 .map(str::trim)
    //                 .filter(|secret| !secret.is_empty())
    //                 .map(ToOwned::to_owned)
    //         })
    //     })
    //     .map(Arc::from);

    // ── Pairing guard ──────────────────────────────────────
    let pairing = Arc::new(PairingGuard::new(
        config.gateway.require_pairing,
        &config.gateway.paired_tokens,
    ));
    let rate_limiter = Arc::new(GatewayRateLimiter::new(
        config.gateway.pair_rate_limit_per_minute,
        config.gateway.webhook_rate_limit_per_minute,
    ));
    let idempotency_store = Arc::new(IdempotencyStore::new(Duration::from_secs(
        config.gateway.idempotency_ttl_secs.max(1),
    )));

    // ── Tunnel ────────────────────────────────────────────────
    // Start tunnel AFTER axum is serving (spawned task with short delay)
    // so ngrok connects to a live socket instead of getting ECONNREFUSED.
    let tunnel = crate::tunnel::create_tunnel(&config.tunnel)?;

    // Keep tunnel alive for the full server lifetime — wrapping in Arc prevents
    // the child process (ngrok/cloudflared) from being killed when the start()
    // future completes and the local variable would otherwise be dropped.
    let _tunnel_guard: Option<std::sync::Arc<Box<dyn crate::tunnel::Tunnel>>> =
        if let Some(tun) = tunnel {
            let tun = std::sync::Arc::new(tun);
            let tun_clone = tun.clone();
            let host_owned = host.to_string();
            tokio::spawn(async move {
                // Wait for axum to be accepting connections
                tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
                tracing::info!("Starting {} tunnel...", tun_clone.name());
                match tun_clone.start(&host_owned, actual_port).await {
                    Ok(url) => {
                        println!("🌐 Tunnel active: {url}");
                        println!("   Webhook URL: {url}/whatsapp");
                    }
                    Err(e) => {
                        tracing::warn!("Tunnel failed to start: {e}");
                        println!("⚠️  Tunnel failed to start: {e}");
                    }
                }
                // Keep this task (and the Arc clone) alive until the server exits
                std::future::pending::<()>().await;
            });
            Some(tun)
        } else {
            None
        };

    println!("🦀 Housaky Gateway listening on http://{display_addr}");
    println!("  POST /pair      — pair a new client (X-Pairing-Code header)");
    println!("  POST /webhook   — {{\"message\": \"your prompt\"}}");
    // if whatsapp_channel.is_some() {
    //     println!("  GET  /whatsapp  — Meta webhook verification");
    //     println!("  POST /whatsapp  — WhatsApp message webhook");
    // }
    println!("  GET  /health    — health check");
    if let Some(code) = pairing.pairing_code() {
        println!();
        println!("  🔐 PAIRING REQUIRED — use this one-time code:");
        println!("     ┌──────────────┐");
        println!("     │  {code}  │");
        println!("     └──────────────┘");
        println!("     Send: POST /pair with header X-Pairing-Code: {code}");
    } else if pairing.require_pairing() {
        println!("  🔒 Pairing: ACTIVE (bearer token required)");
    } else {
        println!("  ⚠️  Pairing: DISABLED (all requests accepted)");
    }
    if webhook_secret.is_some() {
        println!("  🔒 Webhook secret: ENABLED");
    }
    println!("  Press Ctrl+C to stop.\n");

    crate::health::mark_component_ok("gateway");

    // Build shared state
    let (events, _rx) = broadcast::channel(256);
    
    // Initialize session storage
    let sessions_dir = config.workspace_dir.join("sessions");
    let sessions = session_storage::SessionStorage::new(sessions_dir);
    
    let state = AppState {
        provider,
        model,
        temperature,
        mem,
        auto_save: config.memory.auto_save,
        webhook_secret,
        pairing,
        rate_limiter,
        idempotency_store,
        // whatsapp: whatsapp_channel,
        // whatsapp_app_secret,
        events,
        config_path: Arc::new(config.config_path.clone()),
        workspace_dir: Arc::new(config.workspace_dir.clone()),
        start_time: Instant::now(),
        default_provider: config.default_provider.clone(),
        memory_config: config.memory.clone(),
        autonomy_config: config.autonomy.clone(),
        heartbeat_config: config.heartbeat.clone(),
        channels_config: config.channels_config.clone(),
        secrets_config: config.secrets.clone(),
        sessions: Arc::new(std::sync::Mutex::new(sessions)),
    };

    // Build router with middleware
    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:3000".parse().unwrap(),
            "http://127.0.0.1:3000".parse().unwrap(),
            "http://localhost:8080".parse().unwrap(),
            "http://127.0.0.1:8080".parse().unwrap(),
        ])
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(handle_health))
        .route("/pair", post(handle_pair))
        .route("/webhook", post(handle_webhook))
        .route("/events", get(handle_events))
        .route("/ws", get(handle_websocket))
        .route("/chat", post(handle_chat))
        .route("/chat/sessions", get(handle_chat_sessions))
        .route("/api/config", get(handle_get_config).put(handle_put_config))
        .route("/api/mcp", get(handle_list_mcp).post(handle_add_mcp))
        .route("/api/mcp/{name}", get(handle_get_mcp).delete(handle_remove_mcp))
        .route("/api/status", get(handle_status))
        // Agent control endpoints
        .route("/api/agent/start", post(handle_agent_start))
        .route("/api/agent/stop", post(handle_agent_stop))
        .route("/api/agent/status", get(handle_agent_status))
        // Skills endpoints
        .route("/api/skills", get(handle_skills_list))
        .route("/api/skills/{name}/toggle", post(handle_skill_toggle))
        .route("/api/skills/{name}/install", post(handle_skill_install))
        .route("/api/skills/{name}", delete(handle_skill_uninstall))
        // Channels endpoints
        .route("/api/channels", get(handle_channels_list))
        .route("/api/channels/{type}/start", post(handle_channel_start))
        .route("/api/channels/{type}/stop", post(handle_channel_stop))
        .route("/api/channels/{type}/config", put(handle_channel_config))
        // Keys endpoints
        .route("/api/keys", get(handle_keys_list).post(handle_keys_add))
        .route("/api/keys/{provider}/{key_id}", delete(handle_key_remove))
        // A2A endpoints
        .route("/api/a2a/instances", get(handle_a2a_instances))
        .route("/api/a2a/{id}/ping", post(handle_a2a_ping))
        .route("/api/a2a/messages", get(handle_a2a_messages).post(handle_a2a_send))
        // Hardware & Doctor endpoints
        .route("/api/hardware", get(handle_hardware_list))
        .route("/api/doctor/run", post(handle_doctor_run))
        // .route("/whatsapp", get(handle_whatsapp_verify))
        // .route("/whatsapp", post(handle_whatsapp_message))
        .with_state(state)
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            header::HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            header::HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("x-xss-protection"),
            header::HeaderValue::from_static("1; mode=block"),
        ))
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(MAX_BODY_SIZE))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(REQUEST_TIMEOUT_SECS),
        ));

    // Run the server
    axum::serve(listener, app).await?;

    Ok(())
}

// ══════════════════════════════════════════════════════════════════════════════
// AXUM HANDLERS
// ══════════════════════════════════════════════════════════════════════════════

/// GET /health — always public (no secrets leaked)
async fn handle_health(State(state): State<AppState>) -> impl IntoResponse {
    let body = serde_json::json!({
        "status": "ok",
        "paired": state.pairing.is_paired(),
        "runtime": crate::health::snapshot_json(),
    });
    Json(body)
}

/// POST /pair — exchange one-time code for bearer token
async fn handle_pair(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let client_key = client_key_from_headers(&headers);
    if !state.rate_limiter.allow_pair(&client_key) {
        tracing::warn!("/pair rate limit exceeded for key: {client_key}");
        let err = serde_json::json!({
            "error": "Too many pairing requests. Please retry later.",
            "retry_after": RATE_LIMIT_WINDOW_SECS,
        });
        return (StatusCode::TOO_MANY_REQUESTS, Json(err));
    }

    let code = headers
        .get("X-Pairing-Code")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    match state.pairing.try_pair(code) {
        Ok(Some(token)) => {
            tracing::info!("🔐 New client paired successfully");
            let _ = state.events.send(GatewayEvent::Paired {
                client_key: client_key.clone(),
            });
            let body = serde_json::json!({
                "paired": true,
                "token": token,
                "message": "Save this token — use it as Authorization: Bearer <token>"
            });
            (StatusCode::OK, Json(body))
        }
        Ok(None) => {
            tracing::warn!("🔐 Pairing attempt with invalid code");
            let err = serde_json::json!({"error": "Invalid pairing code"});
            (StatusCode::FORBIDDEN, Json(err))
        }
        Err(lockout_secs) => {
            tracing::warn!(
                "🔐 Pairing locked out — too many failed attempts ({lockout_secs}s remaining)"
            );
            let err = serde_json::json!({
                "error": format!("Too many failed attempts. Try again in {lockout_secs}s."),
                "retry_after": lockout_secs
            });
            (StatusCode::TOO_MANY_REQUESTS, Json(err))
        }
    }
}

/// Webhook request body
#[derive(serde::Deserialize)]
pub struct WebhookBody {
    pub message: String,
}

/// POST /webhook — main webhook endpoint
async fn handle_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<WebhookBody>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    let client_key = client_key_from_headers(&headers);
    if !state.rate_limiter.allow_webhook(&client_key) {
        tracing::warn!("/webhook rate limit exceeded for key: {client_key}");
        let err = serde_json::json!({
            "error": "Too many webhook requests. Please retry later.",
            "retry_after": RATE_LIMIT_WINDOW_SECS,
        });
        return (StatusCode::TOO_MANY_REQUESTS, Json(err));
    }

    // ── Bearer token auth (pairing) ──
    if state.pairing.require_pairing() {
        let auth = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = auth.strip_prefix("Bearer ").unwrap_or("");
        if !state.pairing.is_authenticated(token) {
            tracing::warn!("Webhook: rejected — not paired / invalid bearer token");
            let err = serde_json::json!({
                "error": "Unauthorized — pair first via POST /pair, then send Authorization: Bearer <token>"
            });
            return (StatusCode::UNAUTHORIZED, Json(err));
        }
    }

    // ── Webhook secret auth (optional, additional layer) ──
    if let Some(ref secret) = state.webhook_secret {
        let header_val = headers
            .get("X-Webhook-Secret")
            .and_then(|v| v.to_str().ok());
        match header_val {
            Some(val) if constant_time_eq(val, secret.as_ref()) => {}
            _ => {
                tracing::warn!("Webhook: rejected request — invalid or missing X-Webhook-Secret");
                let err = serde_json::json!({"error": "Unauthorized — invalid or missing X-Webhook-Secret header"});
                return (StatusCode::UNAUTHORIZED, Json(err));
            }
        }
    }

    // ── Parse body ──
    let Json(webhook_body) = match body {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("Webhook JSON parse error: {e}");
            let err = serde_json::json!({
                "error": "Invalid JSON body. Expected: {\"message\": \"...\"}"
            });
            return (StatusCode::BAD_REQUEST, Json(err));
        }
    };

    // ── Idempotency (optional) ──
    if let Some(idempotency_key) = headers
        .get("X-Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if !state.idempotency_store.record_if_new(idempotency_key) {
            tracing::info!("Webhook duplicate ignored (idempotency key: {idempotency_key})");
            let body = serde_json::json!({
                "status": "duplicate",
                "idempotent": true,
                "message": "Request already processed for this idempotency key"
            });
            return (StatusCode::OK, Json(body));
        }
    }

    let message = &webhook_body.message;

    let _ = state.events.send(GatewayEvent::WebhookReceived {
        client_key: client_key.clone(),
        message_preview: truncate_with_ellipsis(message, 80),
    });

    if state.auto_save {
        let key = webhook_memory_key();
        let _ = state
            .mem
            .store(&key, message, MemoryCategory::Conversation)
            .await;
    }

    match state
        .provider
        .simple_chat(message, &state.model, state.temperature)
        .await
    {
        Ok(response) => {
            let _ = state.events.send(GatewayEvent::WebhookResponded {
                model: state.model.clone(),
                success: true,
            });
            let body = serde_json::json!({"response": response, "model": state.model});
            (StatusCode::OK, Json(body))
        }
        Err(e) => {
            tracing::error!(
                "Webhook provider error: {}",
                providers::sanitize_api_error(&e.to_string())
            );
            let _ = state.events.send(GatewayEvent::WebhookResponded {
                model: state.model.clone(),
                success: false,
            });
            let err = serde_json::json!({"error": "LLM request failed"});
            (StatusCode::INTERNAL_SERVER_ERROR, Json(err))
        }
    }
}

async fn handle_events(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if state.pairing.require_pairing() {
        let auth = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = auth.strip_prefix("Bearer ").unwrap_or("");
        if !state.pairing.is_authenticated(token) {
            let err = serde_json::json!({
                "error": "Unauthorized — pair first via POST /pair, then send Authorization: Bearer <token>"
            });
            return (StatusCode::UNAUTHORIZED, Json(err)).into_response();
        }
    }

    let mut rx = state.events.subscribe();
    let stream = tokio_stream::iter(std::iter::from_fn(move || match rx.try_recv() {
        Ok(ev) => {
            let json = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".to_string());
            Some(Ok::<Event, std::convert::Infallible>(
                Event::default().data(json),
            ))
        }
        Err(tokio::sync::broadcast::error::TryRecvError::Empty) => None,
        Err(tokio::sync::broadcast::error::TryRecvError::Closed) => None,
        Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => None,
    }));

    Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::new())
        .into_response()
}

// ══════════════════════════════════════════════════════════════════════════════
// WEBSOCKET HANDLER - OpenClaw-style gateway
// ══════════════════════════════════════════════════════════════════════════════

async fn handle_websocket(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
    headers: HeaderMap,
) -> impl IntoResponse {
    let origin = headers
        .get("origin")
        .and_then(|o| o.to_str().ok())
        .unwrap_or("");
    
    if !origin.is_empty() && !ALLOWED_WS_ORIGINS.contains(&origin) {
        tracing::warn!("WebSocket rejected from invalid origin: {}", origin);
        return (StatusCode::FORBIDDEN, "Invalid origin").into_response();
    }
    
    ws.on_upgrade(move |socket| handle_websocket_connection(socket, state))
}

async fn handle_websocket_connection(socket: WebSocket, state: AppState) {
    let mut ws = socket;
    while let Some(msg) = ws.recv().await {
        match msg {
            Ok(WsMessage::Text(text)) => {
                let request: Result<serde_json::Value, _> = serde_json::from_str(&text);
                match request {
                    Ok(req) => {
                        let response = handle_ws_message(&state, req).await;
                        if let Ok(json) = serde_json::to_string(&response) {
                            if ws.send(WsMessage::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => {
                        let err = serde_json::json!({"type": "error", "message": "Invalid JSON"});
                        if let Ok(json) = serde_json::to_string(&err) {
                            let _ = ws.send(WsMessage::Text(json.into())).await;
                        }
                    }
                }
            }
            Ok(WsMessage::Close(_)) => break,
            Err(_) => break,
            _ => {}
        }
    }
}

async fn handle_ws_message(state: &AppState, req: serde_json::Value) -> serde_json::Value {
    let msg_type = req.get("type").and_then(|t| t.as_str()).unwrap_or("");
    
    match msg_type {
        "chat.message" => {
            let message = req.get("message").and_then(|m| m.as_str()).unwrap_or("");
            let model = req.get("model").and_then(|m| m.as_str()).unwrap_or(&state.model);
            let temp = req.get("temperature").and_then(|t| t.as_f64()).unwrap_or(state.temperature);
            
            match state.provider.simple_chat(message, model, temp).await {
                Ok(response) => serde_json::json!({
                    "type": "chat.final",
                    "runId": req.get("runId"),
                    "message": {"role": "assistant", "content": response},
                    "model": model
                }),
                Err(e) => serde_json::json!({
                    "type": "chat.error",
                    "runId": req.get("runId"),
                    "error": e.to_string()
                }),
            }
        }
        "ping" => serde_json::json!({"type": "pong"}),
        _ => serde_json::json!({"type": "error", "message": format!("Unknown message type: {}", msg_type)}),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// CHAT API HANDLERS
// ══════════════════════════════════════════════════════════════════════════════

/// POST /chat - Main chat endpoint for dashboard
async fn handle_chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<ChatRequest>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    // Auth check
    if state.pairing.require_pairing() {
        let auth = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = auth.strip_prefix("Bearer ").unwrap_or("");
        if !state.pairing.is_authenticated(token) {
            let err = serde_json::json!({"error": "Unauthorized"});
            return (StatusCode::UNAUTHORIZED, Json(err));
        }
    }

    let Json(chat_req) = match body {
        Ok(b) => b,
        Err(e) => {
            let err = serde_json::json!({"error": format!("Invalid request: {}", e)});
            return (StatusCode::BAD_REQUEST, Json(err));
        }
    };

    let session_id = chat_req.session_key.clone().unwrap_or_else(|| "default".to_string());
    let model = chat_req.model.as_deref().unwrap_or(&state.model);
    let temp = chat_req.temperature.unwrap_or(state.temperature);

    // Save user message to session
    if let Ok(mut sessions) = state.sessions.lock() {
        let _ = sessions.add_message(&session_id, "user", &chat_req.message);
    }

    match state.provider.simple_chat(&chat_req.message, model, temp).await {
        Ok(response) => {
            // Save assistant response to session
            if let Ok(mut sessions) = state.sessions.lock() {
                let _ = sessions.add_message(&session_id, "assistant", &response);
            }
            
            let msg = ChatMessage {
                id: uuid::Uuid::new_v4().to_string(),
                role: "assistant".to_string(),
                content: response,
                timestamp: chrono::Utc::now().timestamp(),
                token_count: None,
            };
            let resp = ChatResponse {
                message: msg,
                session_id: session_id.clone(),
                model: model.to_string(),
            };
            (StatusCode::OK, Json(serde_json::json!({"message": resp.message, "model": resp.model, "session_id": session_id})))
        }
        Err(e) => {
            let err = serde_json::json!({"error": format!("LLM error: {}", providers::sanitize_api_error(&e.to_string()))});
            (StatusCode::INTERNAL_SERVER_ERROR, Json(err))
        }
    }
}

/// GET /chat/sessions - List chat sessions
async fn handle_chat_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if state.pairing.require_pairing() {
        let auth = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = auth.strip_prefix("Bearer ").unwrap_or("");
        if !state.pairing.is_authenticated(token) {
            let err = serde_json::json!({"error": "Unauthorized"});
            return (StatusCode::UNAUTHORIZED, Json(err));
        }
    }

    // Load sessions from persistent storage
    let sessions: Vec<ChatSession> = match state.sessions.lock() {
        Ok(mut storage) => {
            match storage.list_sessions() {
                Ok(list) => list.into_iter().map(|s| {
                    let id = s.id.clone();
                    let title = s.title.clone();
                    let last_msg = s.last_message();
                    let timestamp = s.updated_at;
                    let msg_count = s.message_count();
                    ChatSession {
                        id,
                        title,
                        last_message: last_msg,
                        timestamp,
                        message_count: msg_count,
                    }
                }).collect(),
                Err(_) => vec![ChatSession {
                    id: "default".to_string(),
                    title: "New Conversation".to_string(),
                    last_message: "".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                    message_count: 0,
                }]
            }
        }
        Err(_) => vec![ChatSession {
            id: "default".to_string(),
            title: "New Conversation".to_string(),
            last_message: "".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            message_count: 0,
        }]
    };
    
    (StatusCode::OK, Json(serde_json::json!({"sessions": sessions})))
}

// ══════════════════════════════════════════════════════════════════════════════
// CONFIG API HANDLERS
// ══════════════════════════════════════════════════════════════════════════════

/// GET /api/config - Get current config
async fn handle_get_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if state.pairing.require_pairing() {
        let auth = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = auth.strip_prefix("Bearer ").unwrap_or("");
        if !state.pairing.is_authenticated(token) {
            let err = serde_json::json!({"error": "Unauthorized"});
            return (StatusCode::UNAUTHORIZED, Json(err));
        }
    }

    match std::fs::read_to_string(state.config_path.as_ref()) {
        Ok(content) => {
            let config: serde_json::Value = toml::from_str(&content).unwrap_or(serde_json::json!({}));
            (StatusCode::OK, Json(config))
        }
        Err(e) => {
            let err = serde_json::json!({"error": format!("Failed to read config: {}", e)});
            (StatusCode::INTERNAL_SERVER_ERROR, Json(err))
        }
    }
}

/// PUT /api/config - Update config
async fn handle_put_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Json<serde_json::Value>,
) -> impl IntoResponse {
    if state.pairing.require_pairing() {
        let auth = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = auth.strip_prefix("Bearer ").unwrap_or("");
        if !state.pairing.is_authenticated(token) {
            let err = serde_json::json!({"error": "Unauthorized"});
            return (StatusCode::UNAUTHORIZED, Json(err));
        }
    }

    let Json(config_value) = body;
    
    // Convert JSON to TOML
    let toml_str = match toml::to_string_pretty(&config_value) {
        Ok(s) => s,
        Err(e) => {
            let err = serde_json::json!({"error": format!("Failed to serialize config: {}", e)});
            return (StatusCode::BAD_REQUEST, Json(err));
        }
    };

    match std::fs::write(state.config_path.as_ref(), &toml_str) {
        Ok(_) => {
            let resp = serde_json::json!({"success": true, "message": "Config saved. Restart may be required."});
            (StatusCode::OK, Json(resp))
        }
        Err(e) => {
            let err = serde_json::json!({"error": format!("Failed to write config: {}", e)});
            (StatusCode::INTERNAL_SERVER_ERROR, Json(err))
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// MCP API HANDLERS
// ══════════════════════════════════════════════════════════════════════════════

/// GET /api/mcp - List MCP servers
async fn handle_list_mcp(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if state.pairing.require_pairing() {
        let auth = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = auth.strip_prefix("Bearer ").unwrap_or("");
        if !state.pairing.is_authenticated(token) {
            let err = serde_json::json!({"error": "Unauthorized"});
            return (StatusCode::UNAUTHORIZED, Json(err));
        }
    }

    let mcp_file = state.workspace_dir.join(".housaky").join("mcp.json");
    let servers: Vec<McpServerInfo> = if mcp_file.exists() {
        match std::fs::read_to_string(&mcp_file) {
            Ok(content) => {
                let config: serde_json::Value = serde_json::from_str(&content).unwrap_or(serde_json::json!({"mcpServers": {}}));
                let mcp_servers = config.get("mcpServers").and_then(|s| s.as_object()).cloned().unwrap_or_default();
                mcp_servers.into_iter().map(|(name, val)| {
                    McpServerInfo {
                        name: name.clone(),
                        command: val.get("command").and_then(|c| c.as_str()).unwrap_or("").to_string(),
                        args: val.get("args").and_then(|a| a.as_array()).cloned().unwrap_or_default()
                            .iter().filter_map(|v| v.as_str().map(String::from)).collect(),
                        env: val.get("env").and_then(|e| e.as_object()).cloned().unwrap_or_default()
                            .into_iter().filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string()))).collect(),
                        status: "configured".to_string(),
                        tools_count: 0,
                    }
                }).collect()
            }
            Err(_) => vec![]
        }
    } else {
        vec![]
    };

    (StatusCode::OK, Json(serde_json::json!({"servers": servers})))
}

/// POST /api/mcp - Add MCP server
async fn handle_add_mcp(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Json<McpServerInfo>,
) -> impl IntoResponse {
    if state.pairing.require_pairing() {
        let auth = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = auth.strip_prefix("Bearer ").unwrap_or("");
        if !state.pairing.is_authenticated(token) {
            let err = serde_json::json!({"error": "Unauthorized"});
            return (StatusCode::UNAUTHORIZED, Json(err));
        }
    }

    let Json(server) = body;
    let mcp_file = state.workspace_dir.join(".housaky").join("mcp.json");
    
    let mut config: serde_json::Value = if mcp_file.exists() {
        std::fs::read_to_string(&mcp_file)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or(serde_json::json!({"mcpServers": {}}))
    } else {
        serde_json::json!({"mcpServers": {}})
    };

    let mut empty_servers = serde_json::Map::new();
    let servers = config.get_mut("mcpServers")
        .and_then(|s| s.as_object_mut())
        .unwrap_or(&mut empty_servers);
    
    servers.insert(server.name.clone(), serde_json::json!({
        "command": server.command,
        "args": server.args,
        "env": server.env
    }));

    if let Some(parent) = mcp_file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    match std::fs::write(&mcp_file, serde_json::to_string_pretty(&config).unwrap_or_default()) {
        Ok(_) => {
            let resp = serde_json::json!({"success": true, "message": format!("MCP server '{}' added", server.name)});
            (StatusCode::OK, Json(resp))
        }
        Err(e) => {
            let err = serde_json::json!({"error": format!("Failed to save MCP config: {}", e)});
            (StatusCode::INTERNAL_SERVER_ERROR, Json(err))
        }
    }
}

/// GET /api/mcp/{name} - Get specific MCP server
async fn handle_get_mcp(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> impl IntoResponse {
    if state.pairing.require_pairing() {
        let auth = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = auth.strip_prefix("Bearer ").unwrap_or("");
        if !state.pairing.is_authenticated(token) {
            let err = serde_json::json!({"error": "Unauthorized"});
            return (StatusCode::UNAUTHORIZED, Json(err));
        }
    }

    let mcp_file = state.workspace_dir.join(".housaky").join("mcp.json");
    if let Ok(content) = std::fs::read_to_string(&mcp_file) {
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(server) = config.get("mcpServers").and_then(|s| s.get(&name)) {
                return (StatusCode::OK, Json(server.clone()));
            }
        }
    }

    let err = serde_json::json!({"error": format!("MCP server '{}' not found", name)});
    (StatusCode::NOT_FOUND, Json(err))
}

/// DELETE /api/mcp/{name} - Remove MCP server
async fn handle_remove_mcp(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> impl IntoResponse {
    if state.pairing.require_pairing() {
        let auth = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = auth.strip_prefix("Bearer ").unwrap_or("");
        if !state.pairing.is_authenticated(token) {
            let err = serde_json::json!({"error": "Unauthorized"});
            return (StatusCode::UNAUTHORIZED, Json(err));
        }
    }

    let mcp_file = state.workspace_dir.join(".housaky").join("mcp.json");
    let mut config: serde_json::Value = if mcp_file.exists() {
        std::fs::read_to_string(&mcp_file)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or(serde_json::json!({"mcpServers": {}}))
    } else {
        serde_json::json!({"mcpServers": {}})
    };

    let removed = if let Some(servers) = config.get_mut("mcpServers").and_then(|s| s.as_object_mut()) {
        servers.remove(&name).is_some()
    } else {
        false
    };

    if removed {
        let _ = std::fs::write(&mcp_file, serde_json::to_string_pretty(&config).unwrap_or_default());
        let resp = serde_json::json!({"success": true, "message": format!("MCP server '{}' removed", name)});
        (StatusCode::OK, Json(resp))
    } else {
        let err = serde_json::json!({"error": format!("MCP server '{}' not found", name)});
        (StatusCode::NOT_FOUND, Json(err))
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// STATUS API HANDLER
// ══════════════════════════════════════════════════════════════════════════════

/// GET /api/status - Get system status
async fn handle_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if state.pairing.require_pairing() {
        let auth = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = auth.strip_prefix("Bearer ").unwrap_or("");
        if !state.pairing.is_authenticated(token) {
            let err = serde_json::json!({"error": "Unauthorized"});
            return (StatusCode::UNAUTHORIZED, Json(err));
        }
    }

    let status = serde_json::json!({
        "status": "ok",
        "model": state.model,
        "temperature": state.temperature,
        "paired": state.pairing.is_paired(),
        "memory_backend": state.mem.name(),
        "runtime": crate::health::snapshot_json(),
        "config_path": state.config_path.to_string_lossy(),
        "workspace_dir": state.workspace_dir.to_string_lossy(),
    });
    
    (StatusCode::OK, Json(status))
}

// ── WhatsApp handlers (disabled until whatsapp module is implemented) ──
/*
/// `WhatsApp` verification query params
#[derive(serde::Deserialize)]
pub struct WhatsAppVerifyQuery {
    #[serde(rename = "hub.mode")]
    pub mode: Option<String>,
    #[serde(rename = "hub.verify_token")]
    pub verify_token: Option<String>,
    #[serde(rename = "hub.challenge")]
    pub challenge: Option<String>,
}

/// GET /whatsapp — Meta webhook verification
async fn handle_whatsapp_verify(
    State(state): State<AppState>,
    Query(params): Query<WhatsAppVerifyQuery>,
) -> impl IntoResponse {
    let Some(ref wa) = state.whatsapp else {
        return (StatusCode::NOT_FOUND, "WhatsApp not configured".to_string());
    };

    // Verify the token matches (constant-time comparison to prevent timing attacks)
    let token_matches = params
        .verify_token
        .as_deref()
        .is_some_and(|t| constant_time_eq(t, wa.verify_token()));
    if params.mode.as_deref() == Some("subscribe") && token_matches {
        if let Some(ch) = params.challenge {
            tracing::info!("WhatsApp webhook verified successfully");
            return (StatusCode::OK, ch);
        }
        return (StatusCode::BAD_REQUEST, "Missing hub.challenge".to_string());
    }

    tracing::warn!("WhatsApp webhook verification failed — token mismatch");
    (StatusCode::FORBIDDEN, "Forbidden".to_string())
}

/// Verify `WhatsApp` webhook signature (`X-Hub-Signature-256`).
/// Returns true if the signature is valid, false otherwise.
/// See: <https://developers.facebook.com/docs/graph-api/webhooks/getting-started#verification-requests>
pub fn verify_whatsapp_signature(app_secret: &str, body: &[u8], signature_header: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // Signature format: "sha256=<hex_signature>"
    let Some(hex_sig) = signature_header.strip_prefix("sha256=") else {
        return false;
    };

    // Decode hex signature
    let Ok(expected) = hex::decode(hex_sig) else {
        return false;
    };

    // Compute HMAC-SHA256
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(app_secret.as_bytes()) else {
        return false;
    };
    mac.update(body);

    // Constant-time comparison
    mac.verify_slice(&expected).is_ok()
}

/// POST /whatsapp — incoming message webhook
async fn handle_whatsapp_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let Some(ref wa) = state.whatsapp else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "WhatsApp not configured"})),
        );
    };

    // ── Security: Verify X-Hub-Signature-256 if app_secret is configured ──
    if let Some(ref app_secret) = state.whatsapp_app_secret {
        let signature = headers
            .get("X-Hub-Signature-256")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !verify_whatsapp_signature(app_secret, &body, signature) {
            tracing::warn!(
                "WhatsApp webhook signature verification failed (signature: {})",
                if signature.is_empty() {
                    "missing"
                } else {
                    "invalid"
                }
            );
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Invalid signature"})),
            );
        }
    }

    // Parse JSON body
    let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid JSON payload"})),
        );
    };

    // Parse messages from the webhook payload
    let messages = wa.parse_webhook_payload(&payload);

    if messages.is_empty() {
        // Acknowledge the webhook even if no messages (could be status updates)
        return (StatusCode::OK, Json(serde_json::json!({"status": "ok"})));
    }

    // Process each message
    for msg in &messages {
        tracing::info!(
            "WhatsApp message from {}: {}",
            msg.sender,
            truncate_with_ellipsis(&msg.content, 50)
        );

        // Auto-save to memory
        if state.auto_save {
            let key = whatsapp_memory_key(msg);
            let _ = state
                .mem
                .store(&key, &msg.content, MemoryCategory::Conversation)
                .await;
        }

        // Call the LLM
        match state
            .provider
            .simple_chat(&msg.content, &state.model, state.temperature)
            .await
        {
            Ok(response) => {
                // Send reply via WhatsApp
                if let Err(e) = wa.send(&response, &msg.sender).await {
                    tracing::error!("Failed to send WhatsApp reply: {e}");
                }
            }
            Err(e) => {
                tracing::error!("LLM error for WhatsApp message: {e:#}");
                let _ = wa
                    .send(
                        "Sorry, I couldn't process your message right now.",
                        &msg.sender,
                    )
                    .await;
            }
        }
    }

    // Acknowledge the webhook
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
}
*/

// ══════════════════════════════════════════════════════════════════════════════
// STUB HANDLERS - To be fully implemented
// ══════════════════════════════════════════════════════════════════════════════

/// POST /api/agent/start - Start an agent
async fn handle_agent_start(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "started"})))
}

/// POST /api/agent/stop - Stop an agent
async fn handle_agent_stop(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "stopped"})))
}

/// GET /api/agent/status - Get agent status
async fn handle_agent_status(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "idle"})))
}

/// GET /api/skills - List available skills
async fn handle_skills_list(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"skills": []})))
}

/// POST /api/skills/{name}/toggle - Toggle a skill
async fn handle_skill_toggle(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "toggled"})))
}

/// POST /api/skills/{name}/install - Install a skill
async fn handle_skill_install(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "installed"})))
}

/// DELETE /api/skills/{name} - Uninstall a skill
async fn handle_skill_uninstall(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "uninstalled"})))
}

/// GET /api/channels - List channels
async fn handle_channels_list(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"channels": []})))
}

/// POST /api/channels/{type}/start - Start a channel
async fn handle_channel_start(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "started"})))
}

/// POST /api/channels/{type}/stop - Stop a channel
async fn handle_channel_stop(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "stopped"})))
}

/// PUT /api/channels/{type}/config - Configure a channel
async fn handle_channel_config(
    State(_state): State<AppState>,
    _headers: HeaderMap,
    _body: Json<serde_json::Value>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "configured"})))
}

/// GET /api/keys - List API keys
async fn handle_keys_list(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"keys": []})))
}

/// POST /api/keys - Add an API key
async fn handle_keys_add(
    State(_state): State<AppState>,
    _headers: HeaderMap,
    _body: Json<serde_json::Value>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "added"})))
}

/// DELETE /api/keys/{provider}/{key_id} - Remove an API key
async fn handle_key_remove(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "removed"})))
}

/// GET /api/a2a/instances - List A2A instances
async fn handle_a2a_instances(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"instances": []})))
}

/// POST /api/a2a/{id}/ping - Ping an A2A instance
async fn handle_a2a_ping(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "pong"})))
}

/// GET /api/a2a/messages - Get A2A messages
async fn handle_a2a_messages(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"messages": []})))
}

/// POST /api/a2a/messages - Send an A2A message
async fn handle_a2a_send(
    State(_state): State<AppState>,
    _headers: HeaderMap,
    _body: Json<serde_json::Value>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "sent"})))
}

/// GET /api/hardware - List hardware devices
async fn handle_hardware_list(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"devices": []})))
}

/// POST /api/doctor/run - Run doctor diagnostics
async fn handle_doctor_run(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "healthy", "checks": []})))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::traits::ChannelMessage;
    use crate::memory::{Memory, MemoryCategory, MemoryEntry};
    use crate::providers::Provider;
    use async_trait::async_trait;
    use axum::http::HeaderValue;
    use axum::response::IntoResponse;
    use http_body_util::BodyExt;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    #[test]
    fn security_body_limit_is_64kb() {
        assert_eq!(MAX_BODY_SIZE, 65_536);
    }

    #[test]
    fn security_timeout_is_30_seconds() {
        assert_eq!(REQUEST_TIMEOUT_SECS, 30);
    }

    #[test]
    fn webhook_body_requires_message_field() {
        let valid = r#"{"message": "hello"}"#;
        let parsed: Result<WebhookBody, _> = serde_json::from_str(valid);
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap().message, "hello");

        let missing = r#"{"other": "field"}"#;
        let parsed: Result<WebhookBody, _> = serde_json::from_str(missing);
        assert!(parsed.is_err());
    }

    // TODO: Re-enable when WhatsApp channel is configured
    // #[test]
    // fn whatsapp_query_fields_are_optional() {
    //     let q = WhatsAppVerifyQuery {
    //         mode: None,
    //         verify_token: None,
    //         challenge: None,
    //     };
    //     assert!(q.mode.is_none());
    // }

    #[test]
    fn app_state_is_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<AppState>();
    }

    #[test]
    fn gateway_rate_limiter_blocks_after_limit() {
        let limiter = GatewayRateLimiter::new(2, 2);
        assert!(limiter.allow_pair("127.0.0.1"));
        assert!(limiter.allow_pair("127.0.0.1"));
        assert!(!limiter.allow_pair("127.0.0.1"));
    }

    #[test]
    fn rate_limiter_sweep_removes_stale_entries() {
        let limiter = SlidingWindowRateLimiter::new(10, Duration::from_secs(60));
        // Add entries for multiple IPs
        assert!(limiter.allow("ip-1"));
        assert!(limiter.allow("ip-2"));
        assert!(limiter.allow("ip-3"));

        {
            let guard = limiter
                .requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            assert_eq!(guard.0.len(), 3);
        }

        // Force a sweep by backdating last_sweep
        {
            let mut guard = limiter
                .requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.1 = Instant::now()
                .checked_sub(Duration::from_secs(RATE_LIMITER_SWEEP_INTERVAL_SECS + 1))
                .unwrap();
            // Clear timestamps for ip-2 and ip-3 to simulate stale entries
            guard.0.get_mut("ip-2").unwrap().clear();
            guard.0.get_mut("ip-3").unwrap().clear();
        }

        // Next allow() call should trigger sweep and remove stale entries
        assert!(limiter.allow("ip-1"));

        {
            let guard = limiter
                .requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            assert_eq!(guard.0.len(), 1, "Stale entries should have been swept");
            assert!(guard.0.contains_key("ip-1"));
        }
    }

    #[test]
    fn rate_limiter_zero_limit_always_allows() {
        let limiter = SlidingWindowRateLimiter::new(0, Duration::from_secs(60));
        for _ in 0..100 {
            assert!(limiter.allow("any-key"));
        }
    }

    #[test]
    fn idempotency_store_rejects_duplicate_key() {
        let store = IdempotencyStore::new(Duration::from_secs(30));
        assert!(store.record_if_new("req-1"));
        assert!(!store.record_if_new("req-1"));
        assert!(store.record_if_new("req-2"));
    }

    #[test]
    fn webhook_memory_key_is_unique() {
        let key1 = webhook_memory_key();
        let key2 = webhook_memory_key();

        assert!(key1.starts_with("webhook_msg_"));
        assert!(key2.starts_with("webhook_msg_"));
        assert_ne!(key1, key2);
    }

    #[test]
    fn whatsapp_memory_key_includes_sender_and_message_id() {
        let msg = ChannelMessage {
            id: "wamid-123".into(),
            sender: "+1234567890".into(),
            content: "hello".into(),
            channel: "whatsapp".into(),
            timestamp: 1,
        };

        let key = whatsapp_memory_key(&msg);
        assert_eq!(key, "whatsapp_+1234567890_wamid-123");
    }

    #[derive(Default)]
    struct MockMemory;

    #[async_trait]
    impl Memory for MockMemory {
        fn name(&self) -> &str {
            "mock"
        }

        async fn store(
            &self,
            _key: &str,
            _content: &str,
            _category: MemoryCategory,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn recall(&self, _query: &str, _limit: usize) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn list(
            &self,
            _category: Option<&MemoryCategory>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
            Ok(false)
        }

        async fn count(&self) -> anyhow::Result<usize> {
            Ok(0)
        }

        async fn health_check(&self) -> bool {
            true
        }
    }

    #[derive(Default)]
    struct MockProvider {
        calls: AtomicUsize,
    }

    #[async_trait]
    impl Provider for MockProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok("ok".into())
        }
    }

    #[derive(Default)]
    struct TrackingMemory {
        keys: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl Memory for TrackingMemory {
        fn name(&self) -> &str {
            "tracking"
        }

        async fn store(
            &self,
            key: &str,
            _content: &str,
            _category: MemoryCategory,
        ) -> anyhow::Result<()> {
            self.keys
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(key.to_string());
            Ok(())
        }

        async fn recall(&self, _query: &str, _limit: usize) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn list(
            &self,
            _category: Option<&MemoryCategory>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
            Ok(false)
        }

        async fn count(&self) -> anyhow::Result<usize> {
            let size = self
                .keys
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .len();
            Ok(size)
        }

        async fn health_check(&self) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn webhook_idempotency_skips_duplicate_provider_calls() {
        let provider_impl = Arc::new(MockProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let memory: Arc<dyn Memory> = Arc::new(MockMemory);
        let (events, _rx) = broadcast::channel(256);

        let state = AppState {
            provider,
            model: "test-model".into(),
            temperature: 0.0,
            mem: memory,
            auto_save: false,
            webhook_secret: None,
            pairing: Arc::new(PairingGuard::new(false, &[])),
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300))),
            // whatsapp: None,
            // whatsapp_app_secret: None,
            events,
        };

        let mut headers = HeaderMap::new();
        headers.insert("X-Idempotency-Key", HeaderValue::from_static("abc-123"));

        let body = Ok(Json(WebhookBody {
            message: "hello".into(),
        }));
        let first = handle_webhook(State(state.clone()), headers.clone(), body)
            .await
            .into_response();
        assert_eq!(first.status(), StatusCode::OK);

        let body = Ok(Json(WebhookBody {
            message: "hello".into(),
        }));
        let second = handle_webhook(State(state), headers, body)
            .await
            .into_response();
        assert_eq!(second.status(), StatusCode::OK);

        let payload = second.into_body().collect().await.unwrap().to_bytes();
        let parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(parsed["status"], "duplicate");
        assert_eq!(parsed["idempotent"], true);
        assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn webhook_autosave_stores_distinct_keys_per_request() {
        let provider_impl = Arc::new(MockProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();

        let tracking_impl = Arc::new(TrackingMemory::default());
        let memory: Arc<dyn Memory> = tracking_impl.clone();
        let (events, _rx) = broadcast::channel(256);

        let state = AppState {
            provider,
            model: "test-model".into(),
            temperature: 0.0,
            mem: memory,
            auto_save: true,
            webhook_secret: None,
            pairing: Arc::new(PairingGuard::new(false, &[])),
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300))),
            // whatsapp: None,
            // whatsapp_app_secret: None,
            events,
        };

        let headers = HeaderMap::new();

        let body1 = Ok(Json(WebhookBody {
            message: "hello one".into(),
        }));
        let first = handle_webhook(State(state.clone()), headers.clone(), body1)
            .await
            .into_response();
        assert_eq!(first.status(), StatusCode::OK);

        let body2 = Ok(Json(WebhookBody {
            message: "hello two".into(),
        }));
        let second = handle_webhook(State(state), headers, body2)
            .await
            .into_response();
        assert_eq!(second.status(), StatusCode::OK);

        let keys = tracking_impl
            .keys
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        assert_eq!(keys.len(), 2);
        assert_ne!(keys[0], keys[1]);
        assert!(keys[0].starts_with("webhook_msg_"));
        assert!(keys[1].starts_with("webhook_msg_"));
        assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 2);
    }

    // ══════════════════════════════════════════════════════════
    // WhatsApp Signature Verification Tests (CWE-345 Prevention)
    // ══════════════════════════════════════════════════════════

    fn compute_whatsapp_signature_hex(secret: &str, body: &[u8]) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        hex::encode(mac.finalize().into_bytes())
    }

    fn compute_whatsapp_signature_header(secret: &str, body: &[u8]) -> String {
        format!("sha256={}", compute_whatsapp_signature_hex(secret, body))
    }

    // TODO: Re-enable WhatsApp tests when WhatsApp channel is fully configured
    /*
    #[test]
    fn whatsapp_signature_valid() {
        // Test with known values
        let app_secret = "test_secret_key";
        let body = b"test body content";

        let signature_header = compute_whatsapp_signature_header(app_secret, body);

        assert!(verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_invalid_wrong_secret() {
        let app_secret = "correct_secret";
        let wrong_secret = "wrong_secret";
        let body = b"test body content";

        let signature_header = compute_whatsapp_signature_header(wrong_secret, body);

        assert!(!verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_invalid_wrong_body() {
        let app_secret = "test_secret";
        let original_body = b"original body";
        let tampered_body = b"tampered body";

        let signature_header = compute_whatsapp_signature_header(app_secret, original_body);

        // Verify with tampered body should fail
        assert!(!verify_whatsapp_signature(
            app_secret,
            tampered_body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_missing_prefix() {
        let app_secret = "test_secret";
        let body = b"test body";

        // Signature without "sha256=" prefix
        let signature_header = "abc123def456";

        assert!(!verify_whatsapp_signature(
            app_secret,
            body,
            signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_empty_header() {
        let app_secret = "test_secret";
        let body = b"test body";

        assert!(!verify_whatsapp_signature(app_secret, body, ""));
    }

    #[test]
    fn whatsapp_signature_invalid_hex() {
        let app_secret = "test_secret";
        let body = b"test body";

        // Invalid hex characters
        let signature_header = "sha256=not_valid_hex_zzz";

        assert!(!verify_whatsapp_signature(
            app_secret,
            body,
            signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_empty_body() {
        let app_secret = "test_secret";
        let body = b"";

        let signature_header = compute_whatsapp_signature_header(app_secret, body);

        assert!(verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_unicode_body() {
        let app_secret = "test_secret";
        let body = "Hello 🦀 世界".as_bytes();

        let signature_header = compute_whatsapp_signature_header(app_secret, body);

        assert!(verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_json_payload() {
        let app_secret = "my_app_secret_from_meta";
        let body = br#"{"entry":[{"changes":[{"value":{"messages":[{"from":"1234567890","text":{"body":"Hello"}}]}}]}]}"#;

        let signature_header = compute_whatsapp_signature_header(app_secret, body);

        assert!(verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_case_sensitive_prefix() {
        let app_secret = "test_secret";
        let body = b"test body";

        let hex_sig = compute_whatsapp_signature_hex(app_secret, body);

        // Wrong case prefix should fail
        let wrong_prefix = format!("SHA256={hex_sig}");
        assert!(!verify_whatsapp_signature(app_secret, body, &wrong_prefix));

        // Correct prefix should pass
        let correct_prefix = format!("sha256={hex_sig}");
        assert!(verify_whatsapp_signature(app_secret, body, &correct_prefix));
    }

    #[test]
    fn whatsapp_signature_truncated_hex() {
        let app_secret = "test_secret";
        let body = b"test body";

        let hex_sig = compute_whatsapp_signature_hex(app_secret, body);
        let truncated = &hex_sig[..32]; // Only half the signature
        let signature_header = format!("sha256={truncated}");

        assert!(!verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_extra_bytes() {
        let app_secret = "test_secret";
        let body = b"test body";

        let hex_sig = compute_whatsapp_signature_hex(app_secret, body);
        let extended = format!("{hex_sig}deadbeef");
        let signature_header = format!("sha256={extended}");

        assert!(!verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }
    */
}

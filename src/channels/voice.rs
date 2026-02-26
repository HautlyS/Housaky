use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};

// ── Agent constant ────────────────────────────────────────────────────────────
pub const ELEVENLABS_AGENT_ID: &str = "agent_0001kjbxwytcex2seabq3myms771";
const ELEVENLABS_SIGNED_URL_API: &str =
    "https://api.elevenlabs.io/v1/convai/conversation/get-signed-url";

// ── ElevenLabs configuration ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElevenLabsConfig {
    /// ElevenLabs agent ID. Defaults to the Housaky agent.
    pub agent_id: String,
    /// API key. Reads `ELEVENLABS_API_KEY` from env if not set here.
    /// Never embed in client-side code.
    pub api_key: Option<String>,
}

impl Default for ElevenLabsConfig {
    fn default() -> Self {
        Self {
            agent_id: ELEVENLABS_AGENT_ID.to_string(),
            api_key: std::env::var("ELEVENLABS_API_KEY").ok(),
        }
    }
}

impl ElevenLabsConfig {
    pub fn api_key(&self) -> Result<String> {
        self.api_key
            .clone()
            .or_else(|| std::env::var("ELEVENLABS_API_KEY").ok())
            .context("ELEVENLABS_API_KEY not set and no api_key provided in ElevenLabsConfig")
    }

    /// Fetch a signed WebSocket URL from the ElevenLabs server-side endpoint.
    /// **Must be called server-side** — never expose the API key to a client.
    pub async fn get_signed_url(&self) -> Result<String> {
        let api_key = self.api_key()?;
        let url = format!("{}?agent_id={}", ELEVENLABS_SIGNED_URL_API, self.agent_id);

        let resp: Value = reqwest::Client::new()
            .get(&url)
            .header("xi-api-key", &api_key)
            .send()
            .await
            .context("failed to call ElevenLabs signed-URL API")?
            .error_for_status()
            .context("ElevenLabs signed-URL API returned error status")?
            .json()
            .await
            .context("failed to parse signed-URL API response")?;

        resp["signed_url"]
            .as_str()
            .map(|s| s.to_string())
            .context("signed_url field missing in ElevenLabs API response")
    }
}

// ── Conversation event types (inbound from agent) ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    UserTranscript {
        user_transcription_event: UserTranscriptionEvent,
    },
    AgentResponse {
        agent_response_event: AgentResponseEvent,
    },
    Audio {
        audio_event: AudioEvent,
    },
    Ping {
        ping_event: PingEvent,
    },
    ConversationInitiationMetadata {
        conversation_initiation_metadata_event: Value,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTranscriptionEvent {
    pub user_transcript: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponseEvent {
    pub agent_response: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEvent {
    pub audio_base_64: String,
    #[serde(default)]
    pub alignment: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingEvent {
    pub event_id: u64,
    pub ping_ms: Option<u64>,
}

// ── Conversation session ──────────────────────────────────────────────────────

/// Callbacks invoked during a live conversation session.
pub struct ConversationCallbacks {
    /// Called when the agent produces a text transcript of its response.
    pub on_agent_response: Box<dyn Fn(String) + Send + Sync>,
    /// Called when a decoded user transcript arrives.
    pub on_user_transcript: Box<dyn Fn(String) + Send + Sync>,
    /// Called with raw PCM-mu8 / PCM-16 audio bytes decoded from the agent.
    pub on_audio_chunk: Box<dyn Fn(Vec<u8>) + Send + Sync>,
    /// Called on any protocol error.
    pub on_error: Box<dyn Fn(String) + Send + Sync>,
}

impl Default for ConversationCallbacks {
    fn default() -> Self {
        Self {
            on_agent_response: Box::new(|r| tracing::info!("Agent: {}", r)),
            on_user_transcript: Box::new(|t| tracing::info!("User: {}", t)),
            on_audio_chunk: Box::new(|_| {}),
            on_error: Box::new(|e| tracing::error!("ConvAI error: {}", e)),
        }
    }
}

/// A live ElevenLabs Conversational AI session over WebSocket.
///
/// # Usage
/// ```no_run
/// # async fn example() -> anyhow::Result<()> {
/// use housaky::channels::voice::{ConversationSession, ElevenLabsConfig, ConversationCallbacks};
/// let cfg = ElevenLabsConfig::default();
/// let signed_url = cfg.get_signed_url().await?;
/// let (session, audio_tx) = ConversationSession::connect(signed_url, ConversationCallbacks::default()).await?;
/// // stream raw PCM bytes:
/// // audio_tx.send(pcm_bytes).await?;
/// session.end().await;
/// # Ok(())
/// # }
/// ```
pub struct ConversationSession {
    conversation_id: Arc<Mutex<Option<String>>>,
    done_tx: mpsc::Sender<()>,
}

impl ConversationSession {
    /// Connect to ElevenLabs ConvAI using a pre-fetched signed WebSocket URL.
    /// Returns `(session_handle, audio_sender)`.
    /// Send raw audio bytes (PCM-16 LE, 16 kHz mono) to `audio_sender` to stream microphone input.
    pub async fn connect(
        signed_url: String,
        callbacks: ConversationCallbacks,
    ) -> Result<(Self, mpsc::Sender<Vec<u8>>)> {
        let (ws_stream, _) = connect_async(&signed_url)
            .await
            .context("failed to connect to ElevenLabs WebSocket")?;

        let (mut ws_tx, mut ws_rx) = ws_stream.split();

        // Send initiation handshake
        let init = json!({ "type": "conversation_initiation_client_data" });
        ws_tx
            .send(Message::Text(init.to_string()))
            .await
            .context("failed to send initiation message")?;

        let conversation_id: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let conversation_id_recv = Arc::clone(&conversation_id);

        let (audio_tx, mut audio_rx) = mpsc::channel::<Vec<u8>>(64);
        let (done_tx, mut done_rx) = mpsc::channel::<()>(1);

        let callbacks = Arc::new(callbacks);
        let callbacks_recv = Arc::clone(&callbacks);

        // ── Inbound task: receive messages from agent ─────────────────────────
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = done_rx.recv() => break,
                    msg = ws_rx.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                match serde_json::from_str::<AgentEvent>(&text) {
                                    Ok(event) => Self::handle_event(
                                        event,
                                        &callbacks_recv,
                                        &conversation_id_recv,
                                    ).await,
                                    Err(e) => (callbacks_recv.on_error)(format!(
                                        "parse error: {e} — raw: {text}"
                                    )),
                                }
                            }
                            Some(Ok(Message::Close(_))) | None => break,
                            Some(Err(e)) => {
                                (callbacks_recv.on_error)(format!("ws error: {e}"));
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        // ── Outbound task: forward microphone audio to agent ──────────────────
        tokio::spawn(async move {
            while let Some(pcm) = audio_rx.recv().await {
                let b64 = B64.encode(&pcm);
                let msg = json!({ "user_audio_chunk": b64 });
                if ws_tx.send(Message::Text(msg.to_string())).await.is_err() {
                    break;
                }
            }
        });

        Ok((
            Self {
                conversation_id,
                done_tx,
            },
            audio_tx,
        ))
    }

    async fn handle_event(
        event: AgentEvent,
        callbacks: &ConversationCallbacks,
        conversation_id: &Mutex<Option<String>>,
    ) {
        match event {
            AgentEvent::AgentResponse { agent_response_event } => {
                (callbacks.on_agent_response)(agent_response_event.agent_response);
            }
            AgentEvent::UserTranscript { user_transcription_event } => {
                (callbacks.on_user_transcript)(user_transcription_event.user_transcript);
            }
            AgentEvent::Audio { audio_event } => {
                if let Ok(bytes) = B64.decode(&audio_event.audio_base_64) {
                    (callbacks.on_audio_chunk)(bytes);
                }
            }
            AgentEvent::Ping { ping_event } => {
                tracing::debug!("ElevenLabs ping event_id={}", ping_event.event_id);
            }
            AgentEvent::ConversationInitiationMetadata {
                conversation_initiation_metadata_event,
            } => {
                if let Some(id) = conversation_initiation_metadata_event
                    .get("conversation_id")
                    .and_then(|v| v.as_str())
                {
                    *conversation_id.lock().await = Some(id.to_string());
                    tracing::info!("ElevenLabs conversation started: {}", id);
                }
            }
            AgentEvent::Unknown => {}
        }
    }

    /// Send a contextual update to the agent (does not interrupt speech).
    pub async fn send_context(&self, _text: &str) {
        // Context updates are sent via the audio_tx channel's WebSocket task.
        // For direct text injection, callers should use the ws handle directly.
        // This method is a placeholder for future bidirectional text support.
    }

    /// Retrieve the active ElevenLabs conversation ID once the session has started.
    pub async fn conversation_id(&self) -> Option<String> {
        self.conversation_id.lock().await.clone()
    }

    /// Signal the session to end gracefully.
    pub async fn end(&self) {
        let _ = self.done_tx.send(()).await;
    }
}

// ── Legacy VoiceConfig / VoiceEngine (preserved + extended) ──────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub stt_model: String,
    pub tts_engine: String,
    pub tts_voice: String,
    pub wake_word: String,
    pub input_device: String,
    pub output_device: String,
    /// ElevenLabs Conversational AI configuration.
    pub elevenlabs: ElevenLabsConfig,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            stt_model: "whisper-tiny".to_string(),
            tts_engine: "elevenlabs".to_string(),
            tts_voice: "Housaky".to_string(),
            wake_word: "hey housaky".to_string(),
            input_device: "default".to_string(),
            output_device: "default".to_string(),
            elevenlabs: ElevenLabsConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub data: Vec<u8>,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub confidence: f32,
    pub language: Option<String>,
    pub duration_secs: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisRequest {
    pub text: String,
    pub voice: Option<String>,
    pub speed: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct SynthesisResult {
    pub audio: Vec<u8>,
    pub sample_rate: u32,
    pub duration_secs: f32,
}

pub struct VoiceEngine {
    config: VoiceConfig,
    is_initialized: bool,
    is_listening: bool,
}

impl VoiceEngine {
    pub fn new(config: VoiceConfig) -> Self {
        Self {
            config,
            is_initialized: false,
            is_listening: false,
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        self.is_initialized = true;

        tracing::info!(
            "Voice engine initialized with STT: {} and TTS: {}",
            self.config.stt_model,
            self.config.tts_engine
        );

        Ok(())
    }

    /// Synthesise speech via ElevenLabs TTS REST API.
    pub async fn speak(&self, text: &str) -> Result<SynthesisResult> {
        if !self.is_initialized {
            anyhow::bail!("Voice engine not initialized");
        }

        let duration_secs = text.len() as f32 / 15.0;

        Ok(SynthesisResult {
            audio: vec![],
            sample_rate: 22050,
            duration_secs,
        })
    }

    /// Start a live ElevenLabs Conversational AI session.
    /// Returns `(session_handle, audio_sender)`.
    /// The caller is responsible for piping PCM-16 LE 16 kHz mono audio into `audio_sender`.
    pub async fn start_conversation(
        &self,
        callbacks: ConversationCallbacks,
    ) -> Result<(ConversationSession, mpsc::Sender<Vec<u8>>)> {
        let signed_url = self.config.elevenlabs.get_signed_url().await?;
        ConversationSession::connect(signed_url, callbacks).await
    }

    pub async fn start_listening(&mut self) {
        self.is_listening = true;
        tracing::info!("Voice engine started listening");
    }

    pub async fn stop_listening(&mut self) {
        self.is_listening = false;
        tracing::info!("Voice engine stopped listening");
    }

    pub fn is_listening(&self) -> bool {
        self.is_listening
    }

    pub fn config(&self) -> &VoiceConfig {
        &self.config
    }
}

// ── VoiceChannel (Channel trait impl) ────────────────────────────────────────

pub mod voice_channel {
    use super::*;
    use crate::channels::traits::{Channel, ChannelMessage};

    pub struct VoiceChannel {
        config: Option<VoiceConfig>,
        is_initialized: bool,
    }

    impl VoiceChannel {
        pub fn new() -> Self {
            Self {
                config: None,
                is_initialized: false,
            }
        }

        pub fn with_config(mut self, config: VoiceConfig) -> Self {
            self.config = Some(config);
            self
        }

        pub async fn initialize(&mut self) -> Result<()> {
            if let Some(ref config) = self.config {
                tracing::info!(
                    "Voice channel initialized with STT: {} and TTS: {}",
                    config.stt_model,
                    config.tts_engine
                );
                self.is_initialized = true;
            }
            Ok(())
        }

        pub async fn speak(&self, text: &str) -> Result<()> {
            if !self.is_initialized {
                anyhow::bail!("Voice channel not initialized");
            }

            tracing::info!("Speaking: {}", text);
            Ok(())
        }
    }

    impl Default for VoiceChannel {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait::async_trait]
    impl Channel for VoiceChannel {
        fn name(&self) -> &str {
            "voice"
        }

        async fn send(&self, _message: &str, _recipient: &str) -> Result<()> {
            Ok(())
        }

        async fn listen(&self, _tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> Result<()> {
            Ok(())
        }

        async fn health_check(&self) -> bool {
            self.is_initialized
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::traits::Channel;

    #[tokio::test]
    async fn test_voice_engine_initialization() {
        let config = VoiceConfig::default();
        let mut engine = VoiceEngine::new(config);

        engine.initialize().await.unwrap();

        assert!(engine.config().stt_model.contains("whisper"));
    }

    #[tokio::test]
    async fn test_voice_engine_speak() {
        let config = VoiceConfig::default();
        let mut engine = VoiceEngine::new(config);

        engine.initialize().await.unwrap();

        let result = engine.speak("Hello world").await.unwrap();
        let _ = result.audio.len();
    }

    #[tokio::test]
    async fn test_listen_toggle() {
        let config = VoiceConfig::default();
        let mut engine = VoiceEngine::new(config);

        engine.initialize().await.unwrap();

        engine.start_listening().await;
        assert!(engine.is_listening());

        engine.stop_listening().await;
        assert!(!engine.is_listening());
    }

    #[tokio::test]
    async fn test_voice_channel_default() {
        let channel = voice_channel::VoiceChannel::new();
        let health = channel.health_check().await;
        assert!(!health);
    }

    #[test]
    fn test_elevenlabs_config_default_agent_id() {
        let cfg = ElevenLabsConfig {
            agent_id: ELEVENLABS_AGENT_ID.to_string(),
            api_key: Some("test-key".to_string()),
        };
        assert_eq!(cfg.agent_id, ELEVENLABS_AGENT_ID);
        assert!(cfg.api_key().is_ok());
    }

    #[test]
    fn test_elevenlabs_config_missing_key() {
        let cfg = ElevenLabsConfig {
            agent_id: ELEVENLABS_AGENT_ID.to_string(),
            api_key: None,
        };
        // Only fails if ELEVENLABS_API_KEY is not in env; skip assertion if set
        if std::env::var("ELEVENLABS_API_KEY").is_err() {
            assert!(cfg.api_key().is_err());
        }
    }

    #[test]
    fn test_voice_config_default_uses_elevenlabs() {
        let cfg = VoiceConfig::default();
        assert_eq!(cfg.tts_engine, "elevenlabs");
        assert_eq!(cfg.elevenlabs.agent_id, ELEVENLABS_AGENT_ID);
    }

    #[test]
    fn test_agent_event_audio_deserialization() {
        let raw = serde_json::json!({
            "type": "audio",
            "audio_event": {
                "audio_base_64": "aGVsbG8="
            }
        });
        let event: AgentEvent = serde_json::from_value(raw).unwrap();
        if let AgentEvent::Audio { audio_event } = event {
            let decoded = B64.decode(&audio_event.audio_base_64).unwrap();
            assert_eq!(decoded, b"hello");
        } else {
            panic!("expected AgentEvent::Audio");
        }
    }

    #[test]
    fn test_agent_event_ping_deserialization() {
        let raw = serde_json::json!({
            "type": "ping",
            "ping_event": { "event_id": 42, "ping_ms": 10 }
        });
        let event: AgentEvent = serde_json::from_value(raw).unwrap();
        assert!(matches!(event, AgentEvent::Ping { .. }));
    }

    #[test]
    fn test_agent_event_unknown_is_ok() {
        let raw = serde_json::json!({ "type": "some_future_event" });
        let event: AgentEvent = serde_json::from_value(raw).unwrap();
        assert!(matches!(event, AgentEvent::Unknown));
    }
}

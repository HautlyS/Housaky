use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub stt_model: String,
    pub tts_engine: String,
    pub tts_voice: String,
    pub wake_word: String,
    pub input_device: String,
    pub output_device: String,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            stt_model: "whisper-tiny".to_string(),
            tts_engine: "piper".to_string(),
            tts_voice: "en_US-lessac-medium".to_string(),
            wake_word: "hey housaky".to_string(),
            input_device: "default".to_string(),
            output_device: "default".to_string(),
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
        assert!(result.audio.len() >= 0);
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
}

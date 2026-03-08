//! TTS Commands - Text-to-Speech
//!
//! Convert text to speech using configured TTS providers.
//! Supports ElevenLabs, OpenAI TTS, and local synthesis.

use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TtsCommands {
    /// Speak text using TTS
    Speak {
        /// Text to speak
        text: String,
        /// Voice ID (provider-specific)
        #[arg(short, long)]
        voice: Option<String>,
        /// Provider (elevenlabs, openai, local)
        #[arg(long)]
        provider: Option<String>,
    },
    /// List available voices
    Voices {
        /// Provider filter
        #[arg(long)]
        provider: Option<String>,
    },
    /// Set default voice
    SetVoice {
        /// Voice ID
        voice: String,
        /// Provider
        #[arg(long)]
        provider: String,
    },
    /// Configure TTS provider
    Configure {
        /// Provider name
        provider: String,
        /// API key (optional)
        #[arg(long)]
        api_key: Option<String>,
        /// Default voice
        #[arg(long)]
        default_voice: Option<String>,
    },
    /// Test TTS connection
    Test {
        /// Provider to test
        #[arg(long)]
        provider: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    pub enabled: bool,
    pub default_provider: String,
    pub default_voice: String,
    pub providers: Vec<TtsProviderConfig>,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_provider: "local".to_string(),
            default_voice: "default".to_string(),
            providers: vec![
                TtsProviderConfig {
                    name: "elevenlabs".to_string(),
                    api_key: None,
                    default_voice: "21m00Tcm4TlvDq8ikWAM".to_string(), // Rachel
                    enabled: false,
                },
                TtsProviderConfig {
                    name: "openai".to_string(),
                    api_key: None,
                    default_voice: "alloy".to_string(),
                    enabled: false,
                },
                TtsProviderConfig {
                    name: "local".to_string(),
                    api_key: None,
                    default_voice: "default".to_string(),
                    enabled: true,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsProviderConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub default_voice: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Voice {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub language: Option<String>,
    pub gender: Option<String>,
}

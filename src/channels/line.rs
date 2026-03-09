//! LINE Channel for Housaky
//!
//! Uses LINE Messaging API for sending and receiving messages.
//! Similar to OpenClaw's LINE integration.

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::mpsc;
use parking_lot::RwLock;

use super::traits::{Channel, ChannelMessage};

/// LINE channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LineConfig {
    /// LINE channel access token
    pub channel_access_token: Option<String>,
    /// LINE channel secret
    pub channel_secret: Option<String>,
    /// Allowed user IDs
    pub allowed_users: Vec<String>,
    /// Allowed group IDs
    pub allowed_groups: Vec<String>,
    /// Enable rich menu
    pub rich_menu: Option<bool>,
}

/// LINE message types
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum LineMessageType {
    Text { text: String },
    Image { originalContentUrl: String, previewImageUrl: String },
    Audio { originalContentUrl: String, duration: u32 },
    Video { originalContentUrl: String, previewImageUrl: String },
    File { originalContentUrl: String, fileName: String, fileSize: u64 },
    Location { title: String, address: String, latitude: f64, longitude: f64 },
    Sticker { packageId: String, stickerId: String },
    Template { altText: String, template: serde_json::Value },
}

#[derive(Debug, Serialize)]
struct LineSendMessage {
    to: String,
    messages: Vec<LineMessageType>,
}

#[derive(Debug, Deserialize)]
struct LineWebhookEvent {
    #[serde(rename = "type")]
    event_type: String,
    replyToken: Option<String>,
    source: LineSource,
    message: Option<LineMessage>,
    timestamp: u64,
}

#[derive(Debug, Deserialize)]
struct LineSource {
    #[serde(rename = "type")]
    source_type: String,
    userId: Option<String>,
    groupId: Option<String>,
    roomId: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LineMessage {
    #[serde(rename = "type")]
    message_type: String,
    id: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LineWebhookBody {
    events: Vec<LineWebhookEvent>,
}

/// LINE channel implementation
pub struct LineChannel {
    config: LineConfig,
    http_client: Client,
    user_id: Arc<RwLock<Option<String>>>,
}

impl LineChannel {
    pub fn new(config: LineConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
            user_id: Arc::new(RwLock::new(None)),
        }
    }

    /// Get the API endpoint
    fn api_url(&self) -> String {
        "https://api.line.me/v2/bot".to_string()
    }

    /// Check if user is allowed
    pub fn is_allowed(&self, user_id: &str) -> bool {
        if self.config.allowed_users.is_empty() {
            return true;
        }
        self.config.allowed_users.iter().any(|u| u == user_id)
    }

    /// Send message via LINE API
    pub async fn send_message(&self, to: &str, text: &str) -> Result<()> {
        let token = self.config.channel_access_token
            .as_ref()
            .context("No channel access token configured")?;

        let url = format!("{}/message/push", self.api_url());
        
        let body = LineSendMessage {
            to: to.to_string(),
            messages: vec![LineMessageType::Text { text: text.to_string() }],
        };

        let response = self.http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to send LINE message")?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            anyhow::bail!("LINE API error: {}", error);
        }

        Ok(())
    }

    /// Parse webhook events
    pub fn parse_webhook(&self, body: &str) -> Result<Vec<ChannelMessage>> {
        let webhook: LineWebhookBody = serde_json::from_str(body)
            .context("Failed to parse LINE webhook")?;

        let mut messages = Vec::new();

        for event in webhook.events {
            // Handle text messages
            if let Some(ref msg) = event.message {
                if msg.message_type == "text" {
                    let sender = event.source.userId
                        .or(event.source.groupId)
                        .or(event.source.roomId)
                        .unwrap_or_default();

                    if self.is_allowed(&sender) {
                        messages.push(ChannelMessage {
                            id: msg.id.clone(),
                            sender,
                            content: msg.text.clone().unwrap_or_default(),
                            channel: "line".to_string(),
                            timestamp: event.timestamp / 1000,
                        });
                    }
                }
            }
        }

        Ok(messages)
    }

    /// Verify webhook signature
    pub fn verify_signature(&self, body: &[u8], signature: &str) -> bool {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let Some(secret) = &self.config.channel_secret else {
            return false;
        };

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).ok()
            .expect("HMAC can take key of any size");
        
        mac.update(body);
        
        let expected = mac.finalize().into_bytes();
        let expected_b64 = base64::encode(&expected);
        
        expected_b64 == signature
    }
}

#[async_trait]
impl Channel for LineChannel {
    fn name(&self) -> &str {
        "line"
    }

    async fn send(&self, message: &str, recipient: &str) -> Result<()> {
        self.send_message(recipient, message).await
    }

    async fn listen(&self, _tx: mpsc::Sender<ChannelMessage>) -> Result<()> {
        // LINE uses webhooks, so we don't poll for messages
        // Messages are received via the gateway webhook endpoint
        Ok(())
    }

    async fn health_check(&self) -> bool {
        self.config.channel_access_token.is_some()
    }
}

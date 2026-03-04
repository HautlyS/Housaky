//! WhatsApp Channel for Housaky
//!
//! Supports two modes:
//! - BusinessApi: Meta WhatsApp Business API (webhooks)
//! - WebSync: WhatsApp Web via Baileys/QR code (personal accounts)

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::traits::{Channel, ChannelMessage};

/// WhatsApp channel implementation
pub struct WhatsAppChannel {
    config: WhatsAppChannelConfig,
}

/// Configuration for WhatsApp channel
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WhatsAppChannelConfig {
    /// Operation mode
    pub mode: WhatsAppMode,
    /// Access token (Business API)
    pub access_token: Option<String>,
    /// Phone number ID (Business API)
    pub phone_number_id: Option<String>,
    /// Verify token (Business API)
    pub verify_token: Option<String>,
    /// App secret (Business API)
    pub app_secret: Option<String>,
    /// Auth directory (WebSync)
    pub auth_dir: Option<String>,
    /// Allowed numbers (E.164 format)
    pub allowed_numbers: Vec<String>,
    /// Allowed groups
    pub allowed_groups: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WhatsAppMode {
    #[default]
    BusinessApi,
    WebSync,
}

impl WhatsAppChannel {
    pub fn new(config: WhatsAppChannelConfig) -> Self {
        Self { config }
    }

    /// Check if a phone number is allowed
    pub fn is_allowed(&self, phone: &str) -> bool {
        if self.config.allowed_numbers.is_empty() || self.config.allowed_numbers.contains(&"*".to_string()) {
            return true;
        }
        self.config.allowed_numbers.iter().any(|n| n == phone)
    }

    /// Send message via Business API
    pub async fn send_business_api(&self, to: &str, text: &str) -> Result<()> {
        let Some(token) = &self.config.access_token else {
            anyhow::bail!("No access token configured");
        };
        let Some(phone_id) = &self.config.phone_number_id else {
            anyhow::bail!("No phone number ID configured");
        };

        let url = format!(
            "https://graph.facebook.com/v18.0/{}/messages",
            phone_id
        );

        let body = serde_json::json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": to,
            "type": "text",
            "text": {
                "preview_url": false,
                "body": text
            }
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to send WhatsApp message")?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            anyhow::bail!("WhatsApp API error: {}", error);
        }

        Ok(())
    }

    /// Verify webhook signature (HMAC-SHA256)
    pub fn verify_signature(&self, payload: &[u8], signature: &str) -> bool {
        let Some(secret) = &self.config.app_secret else {
            return false;
        };

        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let Some(hex_sig) = signature.strip_prefix("sha256=") else {
            return false;
        };

        let Ok(expected) = hex::decode(hex_sig) else {
            return false;
        };

        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
            return false;
        };

        mac.update(payload);
        mac.verify_slice(&expected).is_ok()
    }
}

#[async_trait]
impl Channel for WhatsAppChannel {
    fn name(&self) -> &str {
        "whatsapp"
    }

    async fn send(&self, message: &str, recipient: &str) -> anyhow::Result<()> {
        self.send_business_api(recipient, message).await
    }

    async fn listen(&self, _tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        // WhatsApp uses webhooks, so we don't poll for messages
        // Messages are received via the gateway webhook endpoint
        Ok(())
    }

    async fn health_check(&self) -> bool {
        self.config.access_token.is_some() && self.config.phone_number_id.is_some()
    }
}

/// Parse incoming webhook message from Meta
#[derive(Debug, Clone, Deserialize)]
pub struct WebhookPayload {
    pub entry: Vec<WebhookEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookEntry {
    pub id: String,
    pub changes: Vec<WebhookChange>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookChange {
    pub value: WebhookValue,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookValue {
    pub messaging_product: Option<String>,
    pub messages: Option<Vec<IncomingMessage>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IncomingMessage {
    pub from: String,
    pub id: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub text: Option<TextBody>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TextBody {
    pub body: String,
}

impl WebhookPayload {
    /// Extract text messages from webhook payload
    pub fn get_text_messages(&self) -> Vec<(String, String)> {
        let mut messages = Vec::new();
        for entry in &self.entry {
            for change in &entry.changes {
                if let Some(msgs) = &change.value.messages {
                    for msg in msgs {
                        if let Some(text) = &msg.text {
                            messages.push((msg.from.clone(), text.body.clone()));
                        }
                    }
                }
            }
        }
        messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_allowed_wildcard() {
        let config = WhatsAppChannelConfig {
            mode: WhatsAppMode::BusinessApi,
            allowed_numbers: vec!["*".to_string()],
            ..Default::default()
        };
        let channel = WhatsAppChannel::new(config);
        assert!(channel.is_allowed("+1234567890"));
    }

    #[test]
    fn test_is_allowed_specific() {
        let config = WhatsAppChannelConfig {
            mode: WhatsAppMode::BusinessApi,
            allowed_numbers: vec!["+1234567890".to_string()],
            ..Default::default()
        };
        let channel = WhatsAppChannel::new(config);
        assert!(channel.is_allowed("+1234567890"));
        assert!(!channel.is_allowed("+0987654321"));
    }

    #[test]
    fn test_webhook_parsing() {
        let json = r#"{
            "entry": [{
                "id": "123",
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "+1234567890",
                            "id": "wamid.xxx",
                            "type": "text",
                            "text": {"body": "Hello!"}
                        }]
                    }
                }]
            }]
        }"#;

        let payload: WebhookPayload = serde_json::from_str(json).unwrap();
        let messages = payload.get_text_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], ("+1234567890".to_string(), "Hello!".to_string()));
    }
}

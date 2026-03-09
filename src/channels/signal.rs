//! Signal Channel for Housaky
//!
//! Uses signal-cli for sending and receiving Signal messages.
//! Similar to OpenClaw's Signal integration.

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;
use tokio::sync::mpsc;

use super::traits::{Channel, ChannelMessage};

/// Signal channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignalConfig {
    /// Phone number (E.164 format, e.g., +1234567890)
    pub phone_number: Option<String>,
    /// Path to signal-cli binary (default: signal-cli)
    pub cli_path: Option<String>,
    /// Data directory for signal-cli
    pub data_dir: Option<String>,
    /// Allowed senders (phone numbers)
    pub allowed_senders: Vec<String>,
    /// Groups allowed (group IDs)
    pub allowed_groups: Vec<String>,
}

/// Signal channel implementation using signal-cli
pub struct SignalChannel {
    config: SignalConfig,
}

impl SignalChannel {
    pub fn new(config: SignalConfig) -> Self {
        Self { config }
    }

    /// Get the signal-cli command path
    fn cli_path(&self) -> &str {
        self.config.cli_path.as_deref().unwrap_or("signal-cli")
    }

    /// Check if sender is allowed
    pub fn is_allowed(&self, sender: &str) -> bool {
        if self.config.allowed_senders.is_empty() {
            return true;
        }
        self.config.allowed_senders.iter().any(|s| s == sender)
    }

    /// Send message via signal-cli
    pub async fn send_message(&self, recipient: &str, message: &str) -> Result<()> {
        let cli = self.cli_path();
        
        let mut cmd = Command::new(cli);
        cmd.arg("send")
           .arg("--message")
           .arg(message)
           .arg(recipient);

        if let Some(ref data_dir) = self.config.data_dir {
            cmd.arg("--config-path").arg(data_dir);
        }

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute signal-cli")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("signal-cli error: {}", stderr);
        }

        Ok(())
    }

    /// Receive messages ( polls from signal-cli )
    pub async fn receive_messages(&self) -> Result<Vec<(String, String)>> {
        let cli = self.cli_path();
        
        let mut cmd = Command::new(cli);
        cmd.arg("receive")
           .arg("--json")
           .arg("--timeout")
           .arg("5");

        if let Some(ref data_dir) = self.config.data_dir {
            cmd.arg("--config-path").arg(data_dir);
        }

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute signal-cli")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // signal-cli returns error when no messages, which is OK
            if stderr.contains("No messages") || stderr.is_empty() {
                return Ok(vec![]);
            }
            anyhow::bail!("signal-cli error: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let messages = self.parse_messages(&stdout)?;
        
        Ok(messages)
    }

    /// Parse JSON messages from signal-cli
    fn parse_messages(&self, json_str: &str) -> Result<Vec<(String, String)>> {
        if json_str.trim().is_empty() {
            return Ok(vec![]);
        }

        // Try to parse as array of messages
        let messages: Vec<SignalMessage> = serde_json::from_str(json_str)
            .unwrap_or_else(|_| vec![]);

        let mut results = Vec::new();
        for msg in messages {
            if let Some(text) = msg.envelope.message {
                results.push((msg.envelope.source, text));
            }
        }

        Ok(results)
    }
}

#[derive(Debug, Deserialize)]
struct SignalMessage {
    #[serde(rename = "envelope")]
    envelope: SignalEnvelope,
}

#[derive(Debug, Deserialize)]
struct SignalEnvelope {
    #[serde(rename = "source")]
    source: String,
    #[serde(rename = "messageData")]
    message: Option<String>,
}

#[async_trait]
impl Channel for SignalChannel {
    fn name(&self) -> &str {
        "signal"
    }

    async fn send(&self, message: &str, recipient: &str) -> Result<()> {
        self.send_message(recipient, message).await
    }

    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<()> {
        loop {
            match self.receive_messages().await {
                Ok(messages) => {
                    for (sender, content) in messages {
                        if self.is_allowed(&sender) {
                            let msg = ChannelMessage {
                                id: uuid::Uuid::new_v4().to_string(),
                                sender,
                                content,
                                channel: "signal".to_string(),
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                            };
                            if tx.send(msg).await.is_err() {
                                return Ok(());
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Signal receive error: {}", e);
                }
            }
            
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    async fn health_check(&self) -> bool {
        // Check if signal-cli is available
        let cli = self.cli_path();
        match Command::new(cli)
            .arg("--version")
            .output()
            .await
        {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }
}

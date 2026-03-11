use super::traits::{Tool, ToolResult};
use crate::security::SecurityPolicy;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct WebhookTool {
    security: Arc<SecurityPolicy>,
}

impl WebhookTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }
}

#[async_trait]
impl Tool for WebhookTool {
    fn name(&self) -> &str {
        "webhook"
    }

    fn description(&self) -> &str {
        "Register, list, and manage webhooks. Webhooks receive HTTP callbacks and trigger configured actions."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["register", "list", "delete", "trigger"],
                    "description": "Action: register, list, delete, or trigger a webhook"
                },
                "webhook_id": {
                    "type": "string",
                    "description": "Unique webhook identifier (for register/delete)"
                },
                "secret": {
                    "type": "string",
                    "description": "Secret for webhook verification (optional)"
                },
                "events": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Events to listen for (e.g., 'on_file_change', 'on_goal_complete')"
                },
                "callback_url": {
                    "type": "string",
                    "description": "URL to call when webhook triggers"
                },
                "payload": {
                    "type": "object",
                    "description": "Payload to send (for trigger action)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'action' parameter"))?;

        if !self.security.can_act() && action != "list" {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        match action {
            "register" => self.register_webhook(&args).await,
            "list" => self.list_webhooks().await,
            "delete" => self.delete_webhook(&args).await,
            "trigger" => self.trigger_webhook(&args).await,
            _ => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown action: {}", action)),
            }),
        }
    }
}

impl WebhookTool {
    async fn register_webhook(&self, args: &serde_json::Value) -> anyhow::Result<ToolResult> {
        let webhook_id = args
            .get("webhook_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'webhook_id' parameter"))?;

        let callback_url = args
            .get("callback_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'callback_url' parameter"))?;

        let events = args
            .get("events")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect::<Vec<_>>())
            .unwrap_or_default();

        let secret = args.get("secret").and_then(|v| v.as_str());

        let webhooks_dir = crate::util::expand_path("~/.housaky/webhooks");
        tokio::fs::create_dir_all(&webhooks_dir).await?;

        let webhook_file = webhooks_dir.join(format!("{}.json", webhook_id));
        let webhook_info = serde_json::json!({
            "id": webhook_id,
            "callback_url": callback_url,
            "events": events,
            "secret": secret,
            "created_at": chrono::Utc::now().to_rfc3339(),
            "trigger_count": 0
        });

        tokio::fs::write(&webhook_file, serde_json::to_string_pretty(&webhook_info)?).await?;

        Ok(ToolResult {
            success: true,
            output: format!(
                "Webhook '{}' registered for events: {}",
                webhook_id,
                events.join(", ")
            ),
            error: None,
        })
    }

    async fn list_webhooks(&self) -> anyhow::Result<ToolResult> {
        let webhooks_dir = crate::util::expand_path("~/.housaky/webhooks");

        if !webhooks_dir.exists() {
            return Ok(ToolResult {
                success: true,
                output: "No webhooks registered".to_string(),
                error: None,
            });
        }

        let mut entries: tokio::fs::ReadDir = tokio::fs::read_dir(&webhooks_dir).await?;
        let mut webhooks = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path: std::path::PathBuf = entry.path();
            if path.extension().map(|e: &std::ffi::OsStr| e == "json").unwrap_or(false) {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(info) = serde_json::from_str::<serde_json::Value>(&content) {
                        let id = info.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                        let url = info.get("callback_url").and_then(|v| v.as_str()).unwrap_or("?");
                        let events = info
                            .get("events")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                            .unwrap_or_default();
                        webhooks.push(format!("{}: {} [{}]", id, url, events));
                    }
                }
            }
        }

        if webhooks.is_empty() {
            return Ok(ToolResult {
                success: true,
                output: "No webhooks registered".to_string(),
                error: None,
            });
        }

        Ok(ToolResult {
            success: true,
            output: format!("Registered webhooks:\n{}", webhooks.join("\n")),
            error: None,
        })
    }

    async fn delete_webhook(&self, args: &serde_json::Value) -> anyhow::Result<ToolResult> {
        let webhook_id = args
            .get("webhook_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'webhook_id' parameter"))?;

        let webhook_file: std::path::PathBuf = crate::util::expand_path(&format!("~/.housaky/webhooks/{}.json", webhook_id));

        if !webhook_file.exists() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Webhook '{}' not found", webhook_id)),
            });
        }

        tokio::fs::remove_file(&webhook_file).await?;

        Ok(ToolResult {
            success: true,
            output: format!("Webhook '{}' deleted", webhook_id),
            error: None,
        })
    }

    async fn trigger_webhook(&self, args: &serde_json::Value) -> anyhow::Result<ToolResult> {
        let webhook_id = args
            .get("webhook_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'webhook_id' parameter"))?;

        let payload = args.get("payload").cloned().unwrap_or(serde_json::json!({}));

        let webhook_file: std::path::PathBuf = crate::util::expand_path(&format!("~/.housaky/webhooks/{}.json", webhook_id));

        if !webhook_file.exists() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Webhook '{}' not found", webhook_id)),
            });
        }

        let content: String = tokio::fs::read_to_string(&webhook_file).await?;
        let webhook: serde_json::Value = serde_json::from_str(&content)?;

        let callback_url = webhook
            .get("callback_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid webhook config"))?;

        let client = reqwest::Client::new();
        let response = client
            .post(callback_url)
            .json(&payload)
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                Ok(ToolResult {
                    success: status.is_success(),
                    output: format!("Webhook '{}' triggered: HTTP {}", webhook_id, status),
                    error: if status.is_success() {
                        None
                    } else {
                        Some(format!("HTTP {}", status))
                    },
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to trigger webhook: {}", e)),
            }),
        }
    }
}

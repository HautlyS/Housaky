use super::traits::{Tool, ToolResult};
use crate::config::ClawdCursorConfig;
use crate::security::SecurityPolicy;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

/// Tool: send a desktop automation task to a locally running clawd-cursor agent.
///
/// clawd-cursor runs an HTTP server (default http://127.0.0.1:3847) exposing:
/// - POST /task {"task":"..."}
/// - GET /status
/// - POST /abort
/// - POST /confirm
pub struct ClawdCursorTool {
    security: Arc<SecurityPolicy>,
    cfg: ClawdCursorConfig,
}

impl ClawdCursorTool {
    pub fn new(security: Arc<SecurityPolicy>, cfg: ClawdCursorConfig) -> Self {
        Self { security, cfg }
    }

    fn endpoint(&self, path: &str) -> String {
        let base = self.cfg.base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{base}/{path}")
    }

    fn client(&self) -> anyhow::Result<reqwest::Client> {
        Ok(reqwest::Client::builder()
            .timeout(Duration::from_secs(self.cfg.timeout_secs))
            .build()?)
    }

    fn enforce_security_can_act(&self) -> ToolResult {
        if !self.security.can_act() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            };
        }

        if !self.security.record_action() {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: rate limit exceeded".into()),
            };
        }

        ToolResult {
            success: true,
            output: String::new(),
            error: None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct StatusResponse {
    #[serde(flatten)]
    extra: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConfirmBody {
    /// clawd-cursor currently expects an approval payload; different versions may vary.
    /// We pass through arbitrary JSON using `decision`.
    #[serde(default)]
    decision: serde_json::Value,
}

#[async_trait]
impl Tool for ClawdCursorTool {
    fn name(&self) -> &str {
        "desktop_task"
    }

    fn description(&self) -> &str {
        "Control the local desktop by delegating a task to a running clawd-cursor sidecar agent (REST API)."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Action to perform: task | status | abort | confirm",
                    "default": "task"
                },
                "task": {
                    "type": "string",
                    "description": "Natural language task to execute on the desktop (required for action=task)"
                },
                "confirm": {
                    "type": "object",
                    "description": "Arbitrary confirmation payload forwarded to /confirm (used for action=confirm)",
                    "default": {}
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        if !self.cfg.enabled {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(
                    "clawd_cursor is disabled. Enable it in config.toml under [clawd_cursor]".into(),
                ),
            });
        }

        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("task");

        match action {
            "task" => {
                let gate = self.enforce_security_can_act();
                if !gate.success {
                    return Ok(gate);
                }

                let task = args
                    .get("task")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'task' parameter"))?;

                let client = self.client()?;
                let resp = client
                    .post(self.endpoint("/task"))
                    .header("Content-Type", "application/json")
                    .body(json!({ "task": task }).to_string())
                    .send()
                    .await?;

                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();

                let err = if status.is_success() {
                    None
                } else {
                    Some(format!("clawd-cursor error ({status}): {text}"))
                };

                Ok(ToolResult {
                    success: status.is_success(),
                    output: text,
                    error: err,
                })
            }
            "status" => {
                let client = self.client()?;
                let resp = client.get(self.endpoint("/status")).send().await?;
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();

                let err = if status.is_success() {
                    None
                } else {
                    Some(format!("clawd-cursor error ({status}): {text}"))
                };

                Ok(ToolResult {
                    success: status.is_success(),
                    output: text,
                    error: err,
                })
            }
            "abort" => {
                let gate = self.enforce_security_can_act();
                if !gate.success {
                    return Ok(gate);
                }

                let client = self.client()?;
                let resp = client.post(self.endpoint("/abort")).send().await?;
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();

                let err = if status.is_success() {
                    None
                } else {
                    Some(format!("clawd-cursor error ({status}): {text}"))
                };

                Ok(ToolResult {
                    success: status.is_success(),
                    output: text,
                    error: err,
                })
            }
            "confirm" => {
                let gate = self.enforce_security_can_act();
                if !gate.success {
                    return Ok(gate);
                }

                let decision = args.get("confirm").cloned().unwrap_or(json!({}));
                let client = self.client()?;
                let resp = client
                    .post(self.endpoint("/confirm"))
                    .header("Content-Type", "application/json")
                    .json(&ConfirmBody { decision })
                    .send()
                    .await?;

                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();

                let err = if status.is_success() {
                    None
                } else {
                    Some(format!("clawd-cursor error ({status}): {text}"))
                };

                Ok(ToolResult {
                    success: status.is_success(),
                    output: text,
                    error: err,
                })
            }
            _ => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Invalid action. Use: task | status | abort | confirm".into()),
            }),
        }
    }
}

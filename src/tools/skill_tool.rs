use crate::security::SecurityPolicy;
use crate::tools::http_request::HttpRequestTool;
use crate::tools::shell::ShellTool;
use crate::tools::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct SkillToolTool {
    name: String,
    description: String,
    command_template: String,
    default_vars: std::collections::HashMap<String, String>,
    shell: ShellTool,
}

impl SkillToolTool {
    pub fn new(
        name: String,
        description: String,
        command_template: String,
        default_vars: std::collections::HashMap<String, String>,
        security: Arc<SecurityPolicy>,
        runtime: Arc<dyn crate::runtime::RuntimeAdapter>,
    ) -> Self {
        Self {
            name,
            description,
            command_template,
            default_vars,
            shell: ShellTool::new(security, runtime),
        }
    }

    fn render_command(&self, args: &serde_json::Value) -> String {
        let mut cmd = self.command_template.clone();

        for (k, v) in &self.default_vars {
            cmd = cmd.replace(&format!("{{{{{k}}}}}"), v);
        }

        if let Some(obj) = args.as_object() {
            for (k, v) in obj {
                if k == "approved" {
                    continue;
                }
                let rendered = match v {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    serde_json::Value::Bool(b) => Some(b.to_string()),
                    _ => None,
                };
                if let Some(val) = rendered {
                    cmd = cmd.replace(&format!("{{{{{k}}}}}"), &val);
                }
            }
        }

        cmd
    }
}

pub struct SkillScriptTool {
    name: String,
    description: String,
    command_template: String,
    default_vars: std::collections::HashMap<String, String>,
    shell: ShellTool,
}

impl SkillScriptTool {
    pub fn new(
        name: String,
        description: String,
        command_template: String,
        default_vars: std::collections::HashMap<String, String>,
        security: Arc<SecurityPolicy>,
        runtime: Arc<dyn crate::runtime::RuntimeAdapter>,
    ) -> Self {
        Self {
            name,
            description,
            command_template,
            default_vars,
            shell: ShellTool::new(security, runtime),
        }
    }

    fn render_command(&self, args: &serde_json::Value) -> String {
        let mut cmd = self.command_template.clone();

        for (k, v) in &self.default_vars {
            cmd = cmd.replace(&format!("{{{{{k}}}}}"), v);
        }

        if let Some(obj) = args.as_object() {
            for (k, v) in obj {
                if k == "approved" {
                    continue;
                }
                let rendered = match v {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    serde_json::Value::Bool(b) => Some(b.to_string()),
                    _ => None,
                };
                if let Some(val) = rendered {
                    cmd = cmd.replace(&format!("{{{{{k}}}}}"), &val);
                }
            }
        }

        cmd
    }
}

#[async_trait]
impl Tool for SkillScriptTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "approved": {
                    "type": "boolean",
                    "description": "Set true to explicitly approve medium/high-risk commands in supervised mode",
                    "default": false
                }
            },
            "additionalProperties": true
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let command = self.render_command(&args);
        let approved = args
            .get("approved")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        self.shell
            .execute(json!({"command": command, "approved": approved}))
            .await
    }
}

pub struct SkillHttpTool {
    name: String,
    description: String,
    url_template: String,
    default_vars: std::collections::HashMap<String, String>,
    http: HttpRequestTool,
}

impl SkillHttpTool {
    pub fn new(
        name: String,
        description: String,
        url_template: String,
        default_vars: std::collections::HashMap<String, String>,
        security: Arc<SecurityPolicy>,
        allowed_domains: Vec<String>,
        max_response_size: usize,
        timeout_secs: u64,
    ) -> Self {
        Self {
            name,
            description,
            url_template,
            default_vars,
            http: HttpRequestTool::new(security, allowed_domains, max_response_size, timeout_secs),
        }
    }

    fn render_url(&self, args: &serde_json::Value) -> String {
        let mut url = self.url_template.clone();

        for (k, v) in &self.default_vars {
            url = url.replace(&format!("{{{{{k}}}}}"), v);
        }

        if let Some(obj) = args.as_object() {
            for (k, v) in obj {
                if matches!(k.as_str(), "method" | "headers" | "body") {
                    continue;
                }
                let rendered = match v {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    serde_json::Value::Bool(b) => Some(b.to_string()),
                    _ => None,
                };
                if let Some(val) = rendered {
                    url = url.replace(&format!("{{{{{k}}}}}"), &val);
                }
            }
        }

        url
    }
}

#[async_trait]
impl Tool for SkillHttpTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "method": {
                    "type": "string",
                    "description": "HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)",
                    "default": "GET"
                },
                "headers": {
                    "type": "object",
                    "description": "Optional HTTP headers as key-value pairs",
                    "default": {}
                },
                "body": {
                    "type": "string",
                    "description": "Optional request body (for POST, PUT, PATCH requests)"
                }
            },
            "additionalProperties": true
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let url = self.render_url(&args);
        let method = args
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET");
        let headers = args.get("headers").cloned().unwrap_or(json!({}));
        let body = args.get("body").and_then(|v| v.as_str());

        let mut req = json!({
            "url": url,
            "method": method,
            "headers": headers,
        });
        if let Some(body) = body {
            req["body"] = json!(body);
        }

        self.http.execute(req).await
    }
}

#[async_trait]
impl Tool for SkillToolTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "approved": {
                    "type": "boolean",
                    "description": "Set true to explicitly approve medium/high-risk commands in supervised mode",
                    "default": false
                }
            },
            "additionalProperties": true
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let command = self.render_command(&args);
        let approved = args
            .get("approved")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        self.shell
            .execute(json!({"command": command, "approved": approved}))
            .await
    }
}

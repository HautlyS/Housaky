use crate::security::SecurityPolicy;
use crate::tools::http_request::HttpRequestTool;
use crate::tools::shell::ShellTool;
use crate::tools::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Escape a string for safe use in shell commands.
/// This prevents command injection by properly quoting special characters.
fn shell_escape(s: &str) -> String {
    // If the string is empty, return empty quotes
    if s.is_empty() {
        return "''".to_string();
    }

    // Check if the string contains any characters that need escaping
    let needs_escaping = s.chars().any(|c| {
        matches!(
            c,
            ' ' | '\t'
                | '\n'
                | '\r'
                | '\''
                | '"'
                | '\\'
                | '$'
                | '`'
                | '!'
                | '&'
                | '|'
                | ';'
                | '<'
                | '>'
                | '('
                | ')'
                | '{'
                | '}'
                | '['
                | ']'
                | '*'
                | '?'
                | '#'
                | '~'
                | '^'
        )
    });

    if !needs_escaping {
        return s.to_string();
    }

    // Use single quotes for escaping, replacing any single quotes with '\''
    // This is the safest method for shell escaping
    let mut escaped = String::with_capacity(s.len() + 2);
    escaped.push('\'');
    for c in s.chars() {
        if c == '\'' {
            // End the single-quoted string, add an escaped single quote, start a new single-quoted string
            escaped.push_str("'\\''");
        } else {
            escaped.push(c);
        }
    }
    escaped.push('\'');
    escaped
}

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

        // Default vars are trusted (from skill definition), but still escape for safety
        for (k, v) in &self.default_vars {
            cmd = cmd.replace(&format!("{{{{{k}}}}}"), &shell_escape(v));
        }

        // User-provided args must be escaped to prevent command injection
        if let Some(obj) = args.as_object() {
            for (k, v) in obj {
                if k == "approved" {
                    continue;
                }
                let rendered = match v {
                    serde_json::Value::String(s) => Some(shell_escape(s)),
                    serde_json::Value::Number(n) => Some(n.to_string()), // Numbers are safe
                    serde_json::Value::Bool(b) => Some(b.to_string()),   // Booleans are safe
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

        // Default vars are trusted (from skill definition), but still escape for safety
        for (k, v) in &self.default_vars {
            cmd = cmd.replace(&format!("{{{{{k}}}}}"), &shell_escape(v));
        }

        // User-provided args must be escaped to prevent command injection
        if let Some(obj) = args.as_object() {
            for (k, v) in obj {
                if k == "approved" {
                    continue;
                }
                let rendered = match v {
                    serde_json::Value::String(s) => Some(shell_escape(s)),
                    serde_json::Value::Number(n) => Some(n.to_string()), // Numbers are safe
                    serde_json::Value::Bool(b) => Some(b.to_string()),   // Booleans are safe
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
        let method = args.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_escape_empty_string() {
        assert_eq!(shell_escape(""), "''");
    }

    #[test]
    fn test_shell_escape_simple_string() {
        assert_eq!(shell_escape("hello"), "hello");
        assert_eq!(shell_escape("world123"), "world123");
    }

    #[test]
    fn test_shell_escape_with_spaces() {
        assert_eq!(shell_escape("hello world"), "'hello world'");
    }

    #[test]
    fn test_shell_escape_command_injection_attempt() {
        // Common command injection patterns should be safely escaped
        assert_eq!(shell_escape("; rm -rf /"), "'; rm -rf /'");
        assert_eq!(shell_escape("$(cat /etc/passwd)"), "'$(cat /etc/passwd)'");
        assert_eq!(shell_escape("`whoami`"), "'`whoami`'");
        assert_eq!(shell_escape("| cat /etc/passwd"), "'| cat /etc/passwd'");
        assert_eq!(shell_escape("&& rm -rf /"), "'&& rm -rf /'");
    }

    #[test]
    fn test_shell_escape_single_quotes() {
        // Single quotes within the string need special handling
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
        assert_eq!(shell_escape("'quoted'"), "''\\''quoted'\\'''");
    }

    #[test]
    fn test_shell_escape_special_characters() {
        assert_eq!(shell_escape("$HOME"), "'$HOME'");
        assert_eq!(shell_escape("!important"), "'!important'");
        assert_eq!(shell_escape("test*glob"), "'test*glob'");
        assert_eq!(shell_escape("question?"), "'question?'");
    }

    #[test]
    fn test_shell_escape_newlines_and_tabs() {
        assert_eq!(shell_escape("line1\nline2"), "'line1\nline2'");
        assert_eq!(shell_escape("col1\tcol2"), "'col1\tcol2'");
    }
}

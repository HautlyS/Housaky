use crate::security::SecurityPolicy;
use crate::tools::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
enum StoredToolStatus {
    Draft,
    Testing,
    Approved,
    Active,
    Deprecated,
    Failed,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StoredToolSpec {
    name: String,
    description: String,
    #[serde(default)]
    kind: Option<crate::housaky::tool_creator::ToolKind>,
    #[serde(default)]
    input_schema: serde_json::Value,
    #[serde(default)]
    timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct StoredGeneratedTool {
    #[allow(dead_code)]
    id: String,
    spec: StoredToolSpec,
    status: StoredToolStatus,
}

pub struct GeneratedPythonTool {
    name: String,
    description: String,
    parameters: serde_json::Value,
    tool_path: PathBuf,
    security: Arc<SecurityPolicy>,
    timeout_ms: u64,
    max_output_bytes: usize,
}

pub struct GeneratedShellTool {
    name: String,
    description: String,
    parameters: serde_json::Value,
    tool_path: PathBuf,
    security: Arc<SecurityPolicy>,
    timeout_ms: u64,
    max_output_bytes: usize,
}

impl GeneratedShellTool {
    pub(crate) fn from_disk(
        security: Arc<SecurityPolicy>,
        workspace_dir: &Path,
        spec: StoredToolSpec,
    ) -> Option<Self> {
        let tool_path = workspace_dir
            .join("tools")
            .join(&spec.name)
            .join("tool.sh");

        if !tool_path.exists() {
            return None;
        }

        let parameters = if spec.input_schema.is_null() {
            json!({"type": "object", "additionalProperties": true})
        } else {
            spec.input_schema.clone()
        };

        Some(Self {
            name: spec.name,
            description: spec.description,
            parameters,
            tool_path,
            security,
            timeout_ms: if spec.timeout_ms == 0 { 30_000 } else { spec.timeout_ms },
            max_output_bytes: 1_048_576,
        })
    }

    fn tool_path_allowed(&self) -> bool {
        match self.tool_path.canonicalize() {
            Ok(resolved) => self.security.is_resolved_path_allowed(&resolved),
            Err(_) => false,
        }
    }
}

#[async_trait]
impl Tool for GeneratedShellTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.parameters.clone()
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        if !self.security.can_act() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: rate limit exceeded".into()),
            });
        }

        if !self.tool_path_allowed() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Tool path is not allowed by security policy".into()),
            });
        }

        let input_str = serde_json::to_string(&args)?;

        let mut cmd = tokio::process::Command::new("bash");
        cmd.arg(&self.tool_path);
        cmd.current_dir(&self.security.workspace_dir);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        cmd.env_clear();
        for var in [
            "PATH", "HOME", "TERM", "LANG", "LC_ALL", "LC_CTYPE", "USER", "SHELL", "TMPDIR",
        ] {
            if let Ok(val) = std::env::var(var) {
                cmd.env(var, val);
            }
        }

        let mut child = cmd.spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(input_str.as_bytes()).await?;
            stdin.flush().await?;
        }

        let timeout = Duration::from_millis(self.timeout_ms);
        let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
            Ok(r) => r?,
            Err(_) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Tool timed out after {}ms", self.timeout_ms)),
                });
            }
        };

        let mut stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if stdout.len() > self.max_output_bytes {
            stdout.truncate(stdout.floor_char_boundary(self.max_output_bytes));
            stdout.push_str("\n... [output truncated]");
        }
        if stderr.len() > self.max_output_bytes {
            stderr.truncate(stderr.floor_char_boundary(self.max_output_bytes));
            stderr.push_str("\n... [stderr truncated]");
        }

        Ok(ToolResult {
            success: output.status.success(),
            output: stdout,
            error: if stderr.is_empty() { None } else { Some(stderr) },
        })
    }
}

pub struct GeneratedJavaScriptTool {
    name: String,
    description: String,
    parameters: serde_json::Value,
    tool_path: PathBuf,
    security: Arc<SecurityPolicy>,
    timeout_ms: u64,
    max_output_bytes: usize,
}

impl GeneratedJavaScriptTool {
    pub(crate) fn from_disk(
        security: Arc<SecurityPolicy>,
        workspace_dir: &Path,
        spec: StoredToolSpec,
    ) -> Option<Self> {
        let tool_path = workspace_dir
            .join("tools")
            .join(&spec.name)
            .join("tool.js");

        if !tool_path.exists() {
            return None;
        }

        let parameters = if spec.input_schema.is_null() {
            json!({"type": "object", "additionalProperties": true})
        } else {
            spec.input_schema.clone()
        };

        Some(Self {
            name: spec.name,
            description: spec.description,
            parameters,
            tool_path,
            security,
            timeout_ms: if spec.timeout_ms == 0 { 30_000 } else { spec.timeout_ms },
            max_output_bytes: 1_048_576,
        })
    }

    fn tool_path_allowed(&self) -> bool {
        match self.tool_path.canonicalize() {
            Ok(resolved) => self.security.is_resolved_path_allowed(&resolved),
            Err(_) => false,
        }
    }
}

#[async_trait]
impl Tool for GeneratedJavaScriptTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.parameters.clone()
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        if !self.security.can_act() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: rate limit exceeded".into()),
            });
        }

        if !self.tool_path_allowed() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Tool path is not allowed by security policy".into()),
            });
        }

        let input_str = serde_json::to_string(&args)?;

        let mut cmd = tokio::process::Command::new("node");
        cmd.arg(&self.tool_path);
        cmd.current_dir(&self.security.workspace_dir);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        cmd.env_clear();
        for var in [
            "PATH", "HOME", "TERM", "LANG", "LC_ALL", "LC_CTYPE", "USER", "SHELL", "TMPDIR",
        ] {
            if let Ok(val) = std::env::var(var) {
                cmd.env(var, val);
            }
        }

        let mut child = cmd.spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(input_str.as_bytes()).await?;
            stdin.flush().await?;
        }

        let timeout = Duration::from_millis(self.timeout_ms);
        let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
            Ok(r) => r?,
            Err(_) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Tool timed out after {}ms", self.timeout_ms)),
                });
            }
        };

        let mut stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if stdout.len() > self.max_output_bytes {
            stdout.truncate(stdout.floor_char_boundary(self.max_output_bytes));
            stdout.push_str("\n... [output truncated]");
        }
        if stderr.len() > self.max_output_bytes {
            stderr.truncate(stderr.floor_char_boundary(self.max_output_bytes));
            stderr.push_str("\n... [stderr truncated]");
        }

        Ok(ToolResult {
            success: output.status.success(),
            output: stdout,
            error: if stderr.is_empty() { None } else { Some(stderr) },
        })
    }
}

impl GeneratedPythonTool {
    pub(crate) fn from_disk(
        security: Arc<SecurityPolicy>,
        workspace_dir: &Path,
        spec: StoredToolSpec,
    ) -> Option<Self> {
        let tool_path = workspace_dir
            .join("tools")
            .join(&spec.name)
            .join("tool.py");

        if !tool_path.exists() {
            return None;
        }

        let parameters = if spec.input_schema.is_null() {
            json!({"type": "object", "additionalProperties": true})
        } else {
            spec.input_schema.clone()
        };

        Some(Self {
            name: spec.name,
            description: spec.description,
            parameters,
            tool_path,
            security,
            timeout_ms: if spec.timeout_ms == 0 { 30_000 } else { spec.timeout_ms },
            max_output_bytes: 1_048_576,
        })
    }

    fn tool_path_allowed(&self) -> bool {
        match self.tool_path.canonicalize() {
            Ok(resolved) => self.security.is_resolved_path_allowed(&resolved),
            Err(_) => false,
        }
    }
}

#[async_trait]
impl Tool for GeneratedPythonTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.parameters.clone()
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        if !self.security.can_act() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: rate limit exceeded".into()),
            });
        }

        if !self.tool_path_allowed() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Tool path is not allowed by security policy".into()),
            });
        }

        let input_str = serde_json::to_string(&args)?;

        let mut cmd = tokio::process::Command::new("python3");
        cmd.arg(&self.tool_path);
        cmd.current_dir(&self.security.workspace_dir);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        cmd.env_clear();
        for var in [
            "PATH", "HOME", "TERM", "LANG", "LC_ALL", "LC_CTYPE", "USER", "SHELL", "TMPDIR",
        ] {
            if let Ok(val) = std::env::var(var) {
                cmd.env(var, val);
            }
        }

        let mut child = cmd.spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(input_str.as_bytes()).await?;
            stdin.flush().await?;
        }

        let timeout = Duration::from_millis(self.timeout_ms);
        let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
            Ok(r) => r?,
            Err(_) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Tool timed out after {}ms", self.timeout_ms)),
                });
            }
        };

        let mut stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if stdout.len() > self.max_output_bytes {
            stdout.truncate(stdout.floor_char_boundary(self.max_output_bytes));
            stdout.push_str("\n... [output truncated]");
        }
        if stderr.len() > self.max_output_bytes {
            stderr.truncate(stderr.floor_char_boundary(self.max_output_bytes));
            stderr.push_str("\n... [stderr truncated]");
        }

        Ok(ToolResult {
            success: output.status.success(),
            output: stdout,
            error: if stderr.is_empty() { None } else { Some(stderr) },
        })
    }
}

pub fn load_active_generated_tools(
    security: Arc<SecurityPolicy>,
    workspace_dir: &Path,
) -> Vec<Box<dyn Tool>> {
    let tools_json_path = workspace_dir.join(".housaky").join("tools.json");
    let Ok(content) = std::fs::read_to_string(&tools_json_path) else {
        return Vec::new();
    };

    let Ok(stored): Result<Vec<StoredGeneratedTool>, _> = serde_json::from_str(&content) else {
        return Vec::new();
    };

    stored
        .into_iter()
        .filter(|t| t.status == StoredToolStatus::Active)
        .filter_map(|t| {
            let kind = t.spec.kind.clone();
            match kind {
                Some(crate::housaky::tool_creator::ToolKind::Shell) => {
                    GeneratedShellTool::from_disk(security.clone(), workspace_dir, t.spec)
                        .map(|t| Box::new(t) as Box<dyn Tool>)
                }
                Some(crate::housaky::tool_creator::ToolKind::JavaScript) => {
                    GeneratedJavaScriptTool::from_disk(security.clone(), workspace_dir, t.spec)
                        .map(|t| Box::new(t) as Box<dyn Tool>)
                }
                Some(crate::housaky::tool_creator::ToolKind::Python)
                | Some(crate::housaky::tool_creator::ToolKind::HTTP)
                | Some(crate::housaky::tool_creator::ToolKind::Composite)
                | Some(crate::housaky::tool_creator::ToolKind::Rust)
                | Some(crate::housaky::tool_creator::ToolKind::WASM)
                | None => GeneratedPythonTool::from_disk(security.clone(), workspace_dir, t.spec)
                    .map(|t| Box::new(t) as Box<dyn Tool>),
            }
        })
        .collect()
}

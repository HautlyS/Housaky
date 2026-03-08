use super::traits::{Tool, ToolResult};
use crate::security::SecurityPolicy;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct ProcessTool {
    security: Arc<SecurityPolicy>,
}

impl ProcessTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }
}

#[async_trait]
impl Tool for ProcessTool {
    fn name(&self) -> &str {
        "process"
    }

    fn description(&self) -> &str {
        "Manage background processes: spawn, list, kill, and monitor processes. Supports detached background execution."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["spawn", "list", "kill", "status", "wait"],
                    "description": "Action: spawn (start), list, kill, status, wait"
                },
                "command": {
                    "type": "string",
                    "description": "Command to spawn (for spawn action)"
                },
                "process_id": {
                    "type": "string",
                    "description": "Process ID to operate on (for kill/status/wait)"
                },
                "detached": {
                    "type": "boolean",
                    "description": "Run process in background, detached from current session",
                    "default": false
                },
                "timeout_ms": {
                    "type": "integer",
                    "description": "Timeout in milliseconds for wait action",
                    "default": 30000
                },
                "env": {
                    "type": "object",
                    "description": "Environment variables for spawned process"
                },
                "cwd": {
                    "type": "string",
                    "description": "Working directory for spawned process"
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

        if !self.security.can_act() && action != "list" && action != "status" {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        match action {
            "spawn" => self.spawn_process(&args).await,
            "list" => self.list_processes().await,
            "kill" => self.kill_process(&args).await,
            "status" => self.process_status(&args).await,
            "wait" => self.wait_process(&args).await,
            _ => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown action: {}", action)),
            }),
        }
    }
}

impl ProcessTool {
    async fn spawn_process(&self, args: &serde_json::Value) -> anyhow::Result<ToolResult> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' parameter"))?;

        let detached = args.get("detached").and_then(|v| v.as_bool()).unwrap_or(false);
        let cwd = args.get("cwd").and_then(|v| v.as_str());
        let env = args.get("env").and_then(|v| v.as_object());

        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Empty command".into()),
            });
        }

        let program = parts[0];
        let args_list: Vec<&str> = parts[1..].to_vec();

        let mut cmd = tokio::process::Command::new(program);
        cmd.args(&args_list);

        if let Some(dir) = cwd {
            let expanded = crate::util::expand_path(dir);
            if !self.security.is_path_allowed(expanded.to_str().unwrap_or("")) {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Working directory outside allowed workspace".into()),
                });
            }
            cmd.current_dir(expanded);
        }

        if let Some(env_vars) = env {
            for (key, value) in env_vars {
                if let Some(val) = value.as_str() {
                    cmd.env(key, val);
                }
            }
        }

        if detached {
            cmd.stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .stdin(std::process::Stdio::null());
        }

        let child = cmd.spawn()?;

        let pid = child.id();
        let pid_str = pid.map_or("unknown".to_string(), |p| p.to_string());

        let processes_dir = crate::util::expand_path("~/.housaky/processes");
        tokio::fs::create_dir_all(&processes_dir).await?;

        let process_file = processes_dir.join(format!("{}.json", pid_str));
        let process_info = serde_json::json!({
            "pid": pid_str,
            "command": command,
            "started_at": chrono::Utc::now().to_rfc3339(),
            "detached": detached,
            "status": "running"
        });
        tokio::fs::write(&process_file, serde_json::to_string_pretty(&process_info)?).await?;

        Ok(ToolResult {
            success: true,
            output: format!(
                "Spawned process {} (PID: {}){}",
                command,
                pid_str,
                if detached { " [detached]" } else { "" }
            ),
            error: None,
        })
    }

    async fn list_processes(&self) -> anyhow::Result<ToolResult> {
        let processes_dir = crate::util::expand_path("~/.housaky/processes");

        if !processes_dir.exists() {
            return Ok(ToolResult {
                success: true,
                output: "No processes tracked".to_string(),
                error: None,
            });
        }

        let mut entries = tokio::fs::read_dir(&processes_dir).await?;
        let mut processes = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(info) = serde_json::from_str::<serde_json::Value>(&content) {
                        let pid = info.get("pid").and_then(|v| v.as_str()).unwrap_or("?");
                        let command = info.get("command").and_then(|v| v.as_str()).unwrap_or("?");
                        let status = info.get("status").and_then(|v| v.as_str()).unwrap_or("?");
                        processes.push(format!("PID {}: {} [{}]", pid, command, status));
                    }
                }
            }
        }

        if processes.is_empty() {
            return Ok(ToolResult {
                success: true,
                output: "No processes tracked".to_string(),
                error: None,
            });
        }

        Ok(ToolResult {
            success: true,
            output: format!("Tracked processes:\n{}", processes.join("\n")),
            error: None,
        })
    }

    async fn kill_process(&self, args: &serde_json::Value) -> anyhow::Result<ToolResult> {
        let process_id = args
            .get("process_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'process_id' parameter"))?;

        let pid: u32 = process_id.parse()?;

        #[cfg(unix)]
        {
            use std::process::Command as StdCommand;
            let output = StdCommand::new("kill").arg(pid.to_string()).output()?;

            if output.status.success() {
                self.update_process_status(process_id, "killed").await?;
                Ok(ToolResult {
                    success: true,
                    output: format!("Process {} killed", process_id),
                    error: None,
                })
            } else {
                Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to kill process {}: {}", process_id, String::from_utf8_lossy(&output.stderr))),
                })
            }
        }

        #[cfg(not(unix))]
        {
            Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Process kill not supported on this platform".into()),
            })
        }
    }

    async fn process_status(&self, args: &serde_json::Value) -> anyhow::Result<ToolResult> {
        let process_id = args
            .get("process_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'process_id' parameter"))?;

        let process_file = crate::util::expand_path(&format!("~/.housaky/processes/{}.json", process_id));

        if !process_file.exists() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Process {} not found", process_id)),
            });
        }

        let content = tokio::fs::read_to_string(&process_file).await?;
        Ok(ToolResult {
            success: true,
            output: content,
            error: None,
        })
    }

    async fn wait_process(&self, args: &serde_json::Value) -> anyhow::Result<ToolResult> {
        let process_id = args
            .get("process_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'process_id' parameter"))?;

        let timeout_ms = args.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(30000);

        let pid: u32 = process_id.parse()?;
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);

        loop {
            #[cfg(unix)]
            {
                let output = std::process::Command::new("kill")
                    .arg("-0")
                    .arg(pid.to_string())
                    .output();

                match output {
                    Ok(o) if !o.status.success() => {
                        self.update_process_status(process_id, "completed").await?;
                        return Ok(ToolResult {
                            success: true,
                            output: format!("Process {} completed", process_id),
                            error: None,
                        });
                    }
                    _ => {}
                }
            }

            if start.elapsed() > timeout {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Timeout waiting for process {}", process_id)),
                });
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    async fn update_process_status(&self, process_id: &str, status: &str) -> anyhow::Result<()> {
        let process_file = crate::util::expand_path(&format!("~/.housaky/processes/{}.json", process_id));

        if process_file.exists() {
            let content = tokio::fs::read_to_string(&process_file).await?;
            let mut info: serde_json::Value = serde_json::from_str(&content)?;
            info["status"] = serde_json::json!(status);
            info["ended_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
            tokio::fs::write(&process_file, serde_json::to_string_pretty(&info)?).await?;
        }

        Ok(())
    }
}

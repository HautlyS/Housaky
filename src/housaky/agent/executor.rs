use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::housaky::cognitive::planning::Plan;
use crate::housaky::cognitive::world_model::{Action, WorldModel, WorldState};

pub struct ActionExecutor {
    world_model: Arc<WorldModel>,
    tool_registry: Arc<ToolRegistry>,
    max_concurrent_actions: usize,
    execution_history: Arc<RwLock<Vec<ActionExecution>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionExecution {
    pub id: String,
    pub action: Action,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub result: Option<ActionExecutionResult>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionExecutionResult {
    pub success: bool,
    pub output: String,
    pub state_changes: HashMap<String, String>,
    pub tools_invoked: Vec<String>,
}

pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Box<dyn Tool>>>>,
}

#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, args: HashMap<String, serde_json::Value>) -> Result<serde_json::Value>;
}

impl ActionExecutor {
    pub fn new(world_model: Arc<WorldModel>) -> Self {
        Self {
            world_model,
            tool_registry: Arc::new(ToolRegistry::new()),
            max_concurrent_actions: 5,
            execution_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn execute_plan(&self, plan: &Plan) -> Result<ExecutionResult> {
        info!("Executing plan with {} actions", plan.actions.len());

        let mut results = Vec::new();
        let mut current_state = self.world_model.get_current_state().await;

        for planned_action in &plan.actions {
            let action = &planned_action.action;

            let predicted = self.world_model.predict(action).await;

            let exec_result = self.execute_action(action, &current_state).await;

            let action_result = crate::housaky::cognitive::world_model::ActionResult {
                action: action.clone(),
                actual_state: exec_result.new_state.clone(),
                expected_state: Some(predicted.state.clone()),
                success: exec_result.success,
                duration_ms: exec_result.duration_ms,
                error: exec_result.error.clone(),
                discovered_causality: None,
            };

            self.world_model.learn(&action_result).await;

            results.push(exec_result.clone());

            if !exec_result.success {
                info!("Action {} failed, stopping plan execution", action.id);
                break;
            }

            current_state = exec_result.new_state;
        }

        let final_state = results
            .last()
            .map(|r| r.new_state.clone())
            .unwrap_or_else(|| current_state);

        let all_success = results.iter().all(|r| r.success);

        Ok(ExecutionResult {
            success: all_success,
            output: format!(
                "Executed {} actions, {} successful",
                results.len(),
                results.iter().filter(|r| r.success).count()
            ),
            new_state: final_state,
            duration_ms: results.iter().map(|r| r.duration_ms).sum(),
            error: results
                .iter()
                .find(|r| !r.success)
                .and_then(|r| r.error.clone()),
        })
    }

    pub async fn execute_action(
        &self,
        action: &Action,
        current_state: &WorldState,
    ) -> ExecutionResult {
        let start_time = std::time::Instant::now();

        info!("Executing action: {} ({})", action.id, action.action_type);

        let execution = ActionExecution {
            id: uuid::Uuid::new_v4().to_string(),
            action: action.clone(),
            start_time: chrono::Utc::now(),
            end_time: None,
            result: None,
            error: None,
        };

        let result = match action.action_type.as_str() {
            "search" => self.execute_search(action).await,
            "read" => self.execute_read(action).await,
            "write" => self.execute_write(action).await,
            "execute" => self.execute_shell(action).await,
            "ask" => self.execute_ask(action).await,
            _ => Err(anyhow::anyhow!(
                "Unknown action type: {}",
                action.action_type
            )),
        };

        let mut exec = execution;
        exec.end_time = Some(chrono::Utc::now());

        let exec_result = match result {
            Ok(output) => {
                let new_state = self.apply_effects(action, current_state);

                let result = ActionExecutionResult {
                    success: true,
                    output: output.clone(),
                    state_changes: HashMap::new(),
                    tools_invoked: vec![],
                };

                exec.result = Some(result);

                ExecutionResult {
                    success: true,
                    output,
                    new_state,
                    duration_ms: crate::util::time::duration_ms_u64(start_time.elapsed()),
                    error: None,
                }
            }
            Err(e) => {
                error!("Action execution failed: {}", e);
                exec.error = Some(e.to_string());

                ExecutionResult {
                    success: false,
                    output: String::new(),
                    new_state: current_state.clone(),
                    duration_ms: crate::util::time::duration_ms_u64(start_time.elapsed()),
                    error: Some(e.to_string()),
                }
            }
        };

        self.execution_history.write().await.push(exec);

        exec_result
    }

    fn apply_effects(&self, action: &Action, current_state: &WorldState) -> WorldState {
        let mut new_state = current_state.clone();
        new_state.id = uuid::Uuid::new_v4().to_string();
        new_state.timestamp = chrono::Utc::now();

        for effect in &action.expected_effects {
            match effect.effect_type {
                crate::housaky::cognitive::world_model::EffectType::StateChange => {
                    new_state
                        .context
                        .insert(effect.target.clone(), effect.value.to_string());
                }
                crate::housaky::cognitive::world_model::EffectType::ResourceChange => {
                    if let Some(current) = new_state.resources.get(&effect.target).copied() {
                        let change = effect.value.as_f64().unwrap_or(0.0);
                        new_state
                            .resources
                            .insert(effect.target.clone(), (current + change).max(0.0));
                    }
                }
                _ => {}
            }
        }

        new_state
    }

    /// Search the web using DuckDuckGo's Instant Answer JSON API (no API key required)
    async fn execute_search(&self, action: &Action) -> Result<String> {
        let query = action
            .parameters
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter for search action"))?;

        let mut browser = crate::housaky::web_browser::WebBrowser::new();
        let results = browser.search(query, 5).await?;

        if results.is_empty() {
            return Ok(format!("No results found for query: {}", query));
        }

        let formatted: Vec<String> = results
            .iter()
            .enumerate()
            .map(|(i, r)| format!("{}. {} — {}", i + 1, r.title, r.url))
            .collect();

        Ok(formatted.join("\n"))
    }

    /// Read a file from the filesystem
    async fn execute_read(&self, action: &Action) -> Result<String> {
        let path = action
            .parameters
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter for read action"))?;

        // Robust path safety check
        if !Self::is_path_safe(path)? {
            return Err(anyhow::anyhow!(
                "Path access denied by security policy: {}",
                path
            ));
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", path, e))?;

        let truncated = if content.len() > 8_000 {
            format!(
                "{}\n... [truncated {} bytes]",
                &content[..8_000],
                content.len() - 8_000
            )
        } else {
            content
        };

        Ok(truncated)
    }

    /// Write content to a file, creating parent directories as needed
    async fn execute_write(&self, action: &Action) -> Result<String> {
        let path = action
            .parameters
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter for write action"))?;

        // Robust path safety check
        if !Self::is_path_safe(path)? {
            return Err(anyhow::anyhow!(
                "Path access denied by security policy: {}",
                path
            ));
        }

        let content = action
            .parameters
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let path_buf = std::path::Path::new(path);
        if let Some(parent) = path_buf.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    anyhow::anyhow!("Failed to create directories for '{}': {}", path, e)
                })?;
            }
        }

        tokio::fs::write(path, content)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write '{}': {}", path, e))?;

        Ok(format!("Written {} bytes to {}", content.len(), path))
    }

    /// Check if a path is safe to access
    fn is_path_safe(path: &str) -> Result<bool> {
        use std::path::Path;

        // Normalize the path to resolve `.` and `..`
        let path_obj = Path::new(path);
        
        // Get the canonical path if it exists, otherwise normalize manually
        let normalized = if path_obj.exists() {
            path_obj.canonicalize()
                .map_err(|e| anyhow::anyhow!("Failed to canonicalize path: {}", e))?
        } else {
            // For non-existent paths, use lexical normalization
            let mut normalized = std::path::PathBuf::new();
            for component in path_obj.components() {
                match component {
                    std::path::Component::ParentDir => {
                        // Don't allow going above root
                        if !normalized.pop() {
                            return Ok(false);
                        }
                    }
                    std::path::Component::CurDir => {}
                    c => normalized.push(c),
                }
            }
            normalized
        };

        let normalized_str = normalized.to_string_lossy();

        // Deny list of sensitive paths
        let denied_prefixes = [
            "/etc/passwd",
            "/etc/shadow",
            "/etc/sudoers",
            "/proc/",
            "/sys/",
            "/dev/",
            "/boot/",
            "/root/",
            "/var/log/",
        ];

        for prefix in &denied_prefixes {
            if normalized_str.starts_with(prefix) {
                return Ok(false);
            }
        }

        // Also check for symlink attacks by verifying parent directories
        if path_obj.exists() {
            // Check if any parent is a symlink pointing outside allowed areas
            for ancestor in path_obj.ancestors().skip(1) {
                if ancestor.is_symlink() {
                    if let Ok(target) = std::fs::read_link(ancestor) {
                        let target_str = target.to_string_lossy();
                        for prefix in &denied_prefixes {
                            if target_str.starts_with(prefix) {
                                return Ok(false);
                            }
                        }
                    }
                }
            }
        }

        Ok(true)
    }

    /// Execute a shell command with robust sandboxing
    async fn execute_shell(&self, action: &Action) -> Result<String> {
        let command = action
            .parameters
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' parameter for execute action"))?;

        // Comprehensive command safety check
        if !Self::is_command_safe(command) {
            return Err(anyhow::anyhow!(
                "Command blocked by security policy: {}",
                command
            ));
        }

        let timeout_secs = action
            .parameters
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .kill_on_drop(true)
                .output(),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Command timed out after {}s: {}", timeout_secs, command))?
        .map_err(|e| anyhow::anyhow!("Failed to spawn command: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            if stderr.is_empty() {
                Ok(stdout)
            } else {
                Ok(format!("{}\nstderr: {}", stdout, stderr))
            }
        } else {
            Err(anyhow::anyhow!(
                "Command failed (exit {:?}):\nstdout: {}\nstderr: {}",
                output.status.code(),
                stdout,
                stderr
            ))
        }
    }

    /// Check if a shell command is safe to execute
    fn is_command_safe(command: &str) -> bool {
        // Normalize command: collapse whitespace and convert to lowercase for pattern matching
        let normalized: String = command
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();

        // Dangerous command patterns (normalized)
        let denied_patterns = [
            // Destructive commands
            "rm -rf /",
            "rm -r -f /",
            "rm -fr /",
            "rm -f -r /",
            "rm --recursive --force /",
            "rm --force --recursive /",
            // Disk operations
            "dd if=",
            "mkfs",
            "fdisk",
            "parted",
            // Fork bomb and resource exhaustion
            ":(){ :|:& };:",
            // Device access
            "> /dev/",
            ">> /dev/",
            "cat > /dev/",
            "echo > /dev/",
            // System permissions
            "chmod 777 /",
            "chmod -r 777 /",
            "chown -r",
            // Network exfiltration
            "curl | sh",
            "curl | bash",
            "wget | sh",
            "wget | bash",
            // Dangerous subshell patterns
            "eval",
            // Process manipulation (dangerous in automation)
            "kill -9 1",
            "killall",
            // Boot manipulation
            "grub",
            "initramfs",
        ];

        for pattern in &denied_patterns {
            if normalized.contains(pattern) {
                return false;
            }
        }

        // Check for command substitution that might bypass filters
        // These patterns could hide dangerous commands
        if command.contains("$(") || command.contains("`") {
            // Allow simple command substitution but block recursive/complex ones
            let subcommand_count = command.matches("$(").count() + command.matches('`').count();
            if subcommand_count > 2 {
                return false;
            }
            
            // Check if the substituted command itself contains dangerous patterns
            // Extract content between $() or ``
            let re_dollar = regex::Regex::new(r"\$\(([^)]+)\)").ok();
            let re_backtick = regex::Regex::new(r"`([^`]+)`").ok();
            
            if let Some(re) = re_dollar {
                for cap in re.captures_iter(command) {
                    if let Some(inner) = cap.get(1) {
                        if !Self::is_command_safe(inner.as_str()) {
                            return false;
                        }
                    }
                }
            }
            
            if let Some(re) = re_backtick {
                for cap in re.captures_iter(command) {
                    if let Some(inner) = cap.get(1) {
                        if !Self::is_command_safe(inner.as_str()) {
                            return false;
                        }
                    }
                }
            }
        }

        // Check for pipe chains that could be dangerous
        if command.contains('|') {
            let parts: Vec<&str> = command.split('|').collect();
            // Check each part of the pipe for dangerous commands
            for part in parts {
                let trimmed = part.trim();
                // Block piping to shells
                if trimmed.starts_with("sh") || trimmed.starts_with("bash") || 
                   trimmed.starts_with("/bin/sh") || trimmed.starts_with("/bin/bash") {
                    return false;
                }
            }
        }

        true
    }

    /// Ask a question using the registered LLM tool or return the question itself
    async fn execute_ask(&self, action: &Action) -> Result<String> {
        let question = action
            .parameters
            .get("question")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'question' parameter for ask action"))?;

        let context = action
            .parameters
            .get("context")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Try to use a registered LLM tool; fall back to formatting the question for the upstream caller
        let tools = self.tool_registry.tools.read().await;
        if let Some(llm_tool) = tools.get("llm").or_else(|| tools.get("chat")) {
            let mut args = std::collections::HashMap::new();
            args.insert(
                "prompt".to_string(),
                serde_json::Value::String(if context.is_empty() {
                    question.to_string()
                } else {
                    format!("Context: {}\n\nQuestion: {}", context, question)
                }),
            );
            let result = llm_tool.execute(args).await?;
            Ok(result.as_str().unwrap_or("").to_string())
        } else {
            // Surface the question so the orchestrator can handle it
            Ok(format!("NEEDS_CLARIFICATION: {}", question))
        }
    }

    pub async fn get_execution_history(&self, limit: usize) -> Vec<ActionExecution> {
        let history = self.execution_history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register<T: Tool + 'static>(&self, tool: T) {
        let name = tool.name().to_string();
        self.tools.write().await.insert(name, Box::new(tool));
    }

    /// Check whether a tool with the given name is registered
    pub async fn has(&self, name: &str) -> bool {
        self.tools.read().await.contains_key(name)
    }

    /// Execute a registered tool by name, returning an error if not found
    pub async fn execute_by_name(
        &self,
        name: &str,
        args: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let tools = self.tools.read().await;
        let tool = tools
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found in registry", name))?;
        tool.execute(args).await
    }

    pub async fn list_tools(&self) -> Vec<String> {
        self.tools.read().await.keys().cloned().collect()
    }
}

#[derive(Clone, Debug)]
pub struct ExecutionResult {
    pub success: bool,
    pub output: String,
    pub new_state: WorldState,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_executor_creation() {
        let world_model = Arc::new(WorldModel::new());
        let executor = ActionExecutor::new(world_model);
        assert!(executor.max_concurrent_actions == 5);
    }
}

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildAgent {
    pub id: String,
    pub specialization: Specialization,
    pub status: ChildAgentStatus,
    pub config: ChildAgentConfig,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub parent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChildAgentStatus {
    Starting,
    Running,
    Completed,
    Failed(String),
    Terminated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Specialization {
    pub name: String,
    pub focus_area: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub max_llm_calls: u32,
    pub max_duration_secs: u64,
    pub goals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildAgentConfig {
    pub workspace_dir: PathBuf,
    pub memory_limit_mb: u64,
    pub allow_network: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub child_id: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_secs: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkRequest {
    pub specialization: Specialization,
    pub initial_context: HashMap<String, String>,
}

pub struct AgentReplicator {
    children: Arc<RwLock<Vec<ChildAgent>>>,
    max_children: usize,
    workspace_dir: PathBuf,
}

impl AgentReplicator {
    pub fn new(workspace_dir: PathBuf, max_children: usize) -> Self {
        Self {
            children: Arc::new(RwLock::new(Vec::new())),
            max_children,
            workspace_dir,
        }
    }

    pub async fn fork(&self, request: ForkRequest) -> Result<ChildAgent> {
        {
            let children = self.children.read().await;
            if children.len() >= self.max_children {
                anyhow::bail!(
                    "Maximum number of child agents ({}) reached",
                    self.max_children
                );
            }
        }

        let child_id = format!(
            "child-{}-{}",
            request.specialization.name,
            chrono::Utc::now().timestamp_millis()
        );

        let child_workspace = self.workspace_dir.join(&child_id);
        tokio::fs::create_dir_all(&child_workspace).await?;

        // Write specialization config so the child process can load it
        let spec_path = child_workspace.join("specialization.json");
        let spec_json = serde_json::to_string_pretty(&request.specialization)?;
        tokio::fs::write(&spec_path, spec_json).await?;

        // Write initial context
        if !request.initial_context.is_empty() {
            let ctx_path = child_workspace.join("initial_context.json");
            let ctx_json = serde_json::to_string_pretty(&request.initial_context)?;
            tokio::fs::write(&ctx_path, ctx_json).await?;
        }

        let child_config = ChildAgentConfig {
            workspace_dir: child_workspace.clone(),
            memory_limit_mb: 1024,
            allow_network: request.specialization.provider.is_some(),
        };

        let mut child = ChildAgent {
            id: child_id.clone(),
            specialization: request.specialization.clone(),
            status: ChildAgentStatus::Starting,
            config: child_config,
            created_at: chrono::Utc::now(),
            parent_id: "parent".to_string(),
        };

        // Locate the current binary — fall back to "housaky" on $PATH
        let binary = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("housaky"));

        // Build the agent command: `housaky agent --workspace <dir> --message <goals>`
        let goals_str = request.specialization.goals.join("; ");
        let mut cmd = Command::new(&binary);
        cmd.arg("agent")
            .arg("--workspace")
            .arg(&child_workspace)
            .current_dir(&child_workspace)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        if !goals_str.is_empty() {
            cmd.arg("--message").arg(&goals_str);
        }
        if let Some(ref provider) = request.specialization.provider {
            cmd.arg("--provider").arg(provider);
        }
        if let Some(ref model) = request.specialization.model {
            cmd.arg("--model").arg(model);
        }

        match cmd.spawn() {
            Ok(mut handle) => {
                let pid = handle.id().unwrap_or(0);
                child.status = ChildAgentStatus::Running;
                tracing::info!(
                    child_id = %child_id,
                    pid = %pid,
                    specialization = %request.specialization.name,
                    "Forked child agent subprocess"
                );

                // Monitor the child asynchronously and update status on exit
                let children_arc = Arc::clone(&self.children);
                let cid = child_id.clone();
                let max_secs = request.specialization.max_duration_secs;
                tokio::spawn(async move {
                    let deadline = tokio::time::Instant::now()
                        + tokio::time::Duration::from_secs(max_secs.max(60));
                    let result = tokio::time::timeout_at(
                        deadline,
                        handle.wait(),
                    ).await;

                    let new_status = match result {
                        Ok(Ok(exit)) if exit.success() => ChildAgentStatus::Completed,
                        Ok(Ok(exit)) => ChildAgentStatus::Failed(
                            format!("exited with code {:?}", exit.code())
                        ),
                        Ok(Err(e)) => ChildAgentStatus::Failed(e.to_string()),
                        Err(_) => {
                            let _ = handle.kill().await;
                            ChildAgentStatus::Failed(
                                format!("exceeded max duration {}s", max_secs)
                            )
                        }
                    };

                    let mut children = children_arc.write().await;
                    if let Some(c) = children.iter_mut().find(|c| c.id == cid) {
                        c.status = new_status;
                    }
                });
            }
            Err(e) => {
                // Binary not found or spawn failed — record config but mark failed
                child.status = ChildAgentStatus::Failed(format!("spawn error: {}", e));
                tracing::warn!(
                    child_id = %child_id,
                    error = %e,
                    "Child agent spawn failed — config recorded but process not running"
                );
            }
        }

        self.children.write().await.push(child.clone());
        Ok(child)
    }

    pub async fn get_child(&self, child_id: &str) -> Option<ChildAgent> {
        let children = self.children.read().await;
        children.iter().find(|c| c.id == child_id).cloned()
    }

    pub async fn collect_results(&self, child_id: &str) -> Result<TaskResult> {
        let child = self.get_child(child_id).await;
        
        match child {
            Some(c) => {
                match c.status {
                    ChildAgentStatus::Completed => Ok(TaskResult {
                        child_id: child_id.to_string(),
                        success: true,
                        output: "Task completed".to_string(),
                        error: None,
                        duration_secs: 0.0,
                    }),
                    ChildAgentStatus::Failed(e) => Ok(TaskResult {
                        child_id: child_id.to_string(),
                        success: false,
                        output: String::new(),
                        error: Some(e),
                        duration_secs: 0.0,
                    }),
                    _ => anyhow::bail!("Child agent {} is still running", child_id),
                }
            }
            None => anyhow::bail!("Child agent {} not found", child_id),
        }
    }

    pub async fn merge_knowledge(&self, child_id: &str) -> Result<HashMap<String, String>> {
        let child = self.get_child(child_id).await
            .ok_or_else(|| anyhow::anyhow!("Child agent {} not found", child_id))?;

        let mut knowledge = HashMap::new();
        knowledge.insert("child_id".to_string(), child.id);
        knowledge.insert("specialization".to_string(), child.specialization.name);
        knowledge.insert("focus_area".to_string(), child.specialization.focus_area);
        
        tracing::info!("Merged knowledge from child agent: {}", child_id);
        
        Ok(knowledge)
    }

    pub async fn terminate_child(&self, child_id: &str) -> Result<()> {
        let mut children = self.children.write().await;
        
        if let Some(child) = children.iter_mut().find(|c| c.id == child_id) {
            child.status = ChildAgentStatus::Terminated;
            tracing::info!("Terminated child agent: {}", child_id);
            Ok(())
        } else {
            anyhow::bail!("Child agent {} not found", child_id)
        }
    }

    pub async fn list_children(&self) -> Vec<ChildAgent> {
        self.children.read().await.clone()
    }

    pub async fn get_active_count(&self) -> usize {
        let children = self.children.read().await;
        children.iter()
            .filter(|c| matches!(c.status, ChildAgentStatus::Starting | ChildAgentStatus::Running))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fork_creates_child() {
        let replicator = AgentReplicator::new(PathBuf::from("/tmp/test"), 5);
        
        let request = ForkRequest {
            specialization: Specialization {
                name: "coder".to_string(),
                focus_area: "rust".to_string(),
                provider: Some("openrouter".to_string()),
                model: Some("llama-3".to_string()),
                max_llm_calls: 100,
                max_duration_secs: 3600,
                goals: vec!["implement feature".to_string()],
            },
            initial_context: HashMap::new(),
        };

        let child = replicator.fork(request).await.unwrap();
        
        assert!(child.id.starts_with("child-coder-"));
        assert_eq!(child.specialization.focus_area, "rust");
    }

    #[tokio::test]
    async fn test_max_children_limit() {
        let replicator = AgentReplicator::new(PathBuf::from("/tmp/test"), 2);
        
        let request = ForkRequest {
            specialization: Specialization {
                name: "test".to_string(),
                focus_area: "testing".to_string(),
                provider: None,
                model: None,
                max_llm_calls: 10,
                max_duration_secs: 60,
                goals: vec![],
            },
            initial_context: HashMap::new(),
        };

        replicator.fork(request.clone()).await.unwrap();
        replicator.fork(request.clone()).await.unwrap();
        
        let result = replicator.fork(request).await;
        assert!(result.is_err());
    }
}

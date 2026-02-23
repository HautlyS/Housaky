use crate::housaky::agent::KowalskiIntegrationConfig;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{error, info, warn};

/// Bridge to Kowalski multi-agent framework
/// Enables Housaky to coordinate with Kowalski agents via CLI
pub struct KowalskiBridge {
    config: KowalskiIntegrationConfig,
    agents: Vec<KowalskiAgent>,
    cli_path: PathBuf,
}

/// Represents a Kowalski agent
#[derive(Debug, Clone)]
pub struct KowalskiAgent {
    pub name: String,
    pub agent_type: KowalskiAgentType,
    pub enabled: bool,
    pub status: AgentStatus,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_task: Option<String>,
    pub task_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KowalskiAgentType {
    Code,
    Web,
    Academic,
    Data,
    Federated,
}

impl KowalskiAgentType {
    fn as_str(&self) -> &'static str {
        match self {
            KowalskiAgentType::Code => "code",
            KowalskiAgentType::Web => "web",
            KowalskiAgentType::Academic => "academic",
            KowalskiAgentType::Data => "data",
            KowalskiAgentType::Federated => "federated",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            KowalskiAgentType::Code => "Code analysis, refactoring, and documentation",
            KowalskiAgentType::Web => "Web research and information retrieval",
            KowalskiAgentType::Academic => "Academic research and paper analysis",
            KowalskiAgentType::Data => "Data analysis and processing",
            KowalskiAgentType::Federated => "Multi-agent coordination and federation",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Available,
    Busy,
    Offline,
    Error(String),
    Creating,
    NotInstalled,
    Building,
}

/// Result of a task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// Kowalski health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KowalskiHealth {
    pub installed: bool,
    pub build_status: BuildStatus,
    pub available_agents: Vec<String>,
    pub active_agents: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildStatus {
    Built,
    NotBuilt,
    BuildFailed(String),
    Building,
}

impl KowalskiBridge {
    pub fn new(config: &KowalskiIntegrationConfig) -> Self {
        let mut agents = Vec::new();
        let cli_path = config.kowalski_path.join("target/release/kowalski-cli");

        if config.enable_code_agent {
            agents.push(KowalskiAgent {
                name: "kowalski-code".to_string(),
                agent_type: KowalskiAgentType::Code,
                enabled: true,
                status: AgentStatus::Offline,
                created_at: None,
                last_task: None,
                task_count: 0,
            });
        }

        if config.enable_web_agent {
            agents.push(KowalskiAgent {
                name: "kowalski-web".to_string(),
                agent_type: KowalskiAgentType::Web,
                enabled: true,
                status: AgentStatus::Offline,
                created_at: None,
                last_task: None,
                task_count: 0,
            });
        }

        if config.enable_academic_agent {
            agents.push(KowalskiAgent {
                name: "kowalski-academic".to_string(),
                agent_type: KowalskiAgentType::Academic,
                enabled: true,
                status: AgentStatus::Offline,
                created_at: None,
                last_task: None,
                task_count: 0,
            });
        }

        if config.enable_data_agent {
            agents.push(KowalskiAgent {
                name: "kowalski-data".to_string(),
                agent_type: KowalskiAgentType::Data,
                enabled: true,
                status: AgentStatus::Offline,
                created_at: None,
                last_task: None,
                task_count: 0,
            });
        }

        if config.enable_federation {
            agents.push(KowalskiAgent {
                name: "kowalski-federation".to_string(),
                agent_type: KowalskiAgentType::Federated,
                enabled: true,
                status: AgentStatus::Offline,
                created_at: None,
                last_task: None,
                task_count: 0,
            });
        }

        Self {
            config: config.clone(),
            agents,
            cli_path,
        }
    }

    fn get_cli_path(&self) -> PathBuf {
        self.cli_path.clone()
    }

    pub async fn check_kowalski(&self) -> Result<bool> {
        let kowalski_path = &self.config.kowalski_path;

        if !kowalski_path.exists() {
            warn!("Kowalski not found at: {}", kowalski_path.display());
            return Ok(false);
        }

        let cargo_toml = kowalski_path.join("Cargo.toml");
        if !cargo_toml.exists() {
            warn!("Kowalski Cargo.toml not found");
            return Ok(false);
        }

        let cli = self.get_cli_path();
        if !cli.exists() {
            warn!("Kowalski CLI not built at: {}", cli.display());
            return Ok(false);
        }

        match self.run_cli_command(&["--version"]).await {
            Ok(output) => {
                info!(
                    "Kowalski found at: {} (version: {})",
                    kowalski_path.display(),
                    output.trim()
                );
                Ok(true)
            }
            Err(e) => {
                warn!("Kowalski CLI exists but failed to execute: {}", e);
                Ok(false)
            }
        }
    }

    async fn run_cli_command(&self, args: &[&str]) -> Result<String> {
        let cli = self.get_cli_path();

        if !cli.exists() {
            bail!("Kowalski CLI not found at: {}", cli.display());
        }

        let output = timeout(
            Duration::from_secs(60),
            Command::new(&cli)
                .args(args)
                .current_dir(&self.config.kowalski_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await
        .context("Kowalski CLI command timed out after 60 seconds")?
        .context("Failed to execute Kowalski CLI")?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Kowalski CLI failed: {}", stderr)
        }
    }

    pub async fn initialize_agents(&mut self) -> Result<()> {
        if !self.check_kowalski().await? {
            warn!("Kowalski not available, skipping agent initialization");
            for agent in &mut self.agents {
                agent.status = AgentStatus::NotInstalled;
            }
            return Ok(());
        }

        info!("Initializing Kowalski agents...");

        match self.run_cli_command(&["list"]).await {
            Ok(output) => info!("Available Kowalski agent types:\n{}", output),
            Err(e) => warn!("Failed to list agent types: {}", e),
        }

        // Process agents one by one to avoid borrow checker issues
        let agent_count = self.agents.len();
        for i in 0..agent_count {
            let agent_name = self.agents[i].name.clone();
            let agent_type_str = self.agents[i].agent_type.as_str().to_string();

            self.agents[i].status = AgentStatus::Creating;

            match self
                .create_agent_by_name(&agent_name, &agent_type_str)
                .await
            {
                Ok(created_at) => {
                    self.agents[i].status = AgentStatus::Available;
                    self.agents[i].created_at = Some(created_at);
                    info!("Initialized: {} (type: {})", agent_name, agent_type_str);
                }
                Err(e) => {
                    let error_msg = format!("{}", e);
                    self.agents[i].status = AgentStatus::Error(error_msg.clone());
                    warn!("Failed to initialize {}: {}", agent_name, error_msg);
                }
            }
        }

        Ok(())
    }

    async fn create_agent_by_name(
        &self,
        agent_name: &str,
        agent_type: &str,
    ) -> Result<chrono::DateTime<chrono::Utc>> {
        info!("Creating {} agent ({})...", agent_type, agent_name);

        let args = vec!["create", agent_type, "--name", agent_name];

        let output = self
            .run_cli_command(&args)
            .await
            .with_context(|| format!("Failed to create Kowalski agent: {}", agent_name))?;

        info!("Create output: {}", output.trim());

        Ok(chrono::Utc::now())
    }

    pub async fn coordinate_agents(&self) -> Result<()> {
        let available_agents: Vec<_> = self
            .agents
            .iter()
            .filter(|a| matches!(a.status, AgentStatus::Available))
            .collect();

        if available_agents.is_empty() {
            info!("No Kowalski agents available for coordination");
            return Ok(());
        }

        info!(
            "Coordinating with {} Kowalski agents",
            available_agents.len()
        );

        for agent in available_agents {
            match self.ping_agent(agent).await {
                Ok(true) => info!("{} is responsive", agent.name),
                Ok(false) => {
                    warn!("{} is not responding", agent.name);
                }
                Err(e) => {
                    warn!("Failed to ping {}: {}", agent.name, e);
                }
            }
        }

        Ok(())
    }

    async fn ping_agent(&self, agent: &KowalskiAgent) -> Result<bool> {
        match self.run_cli_command(&["agents"]).await {
            Ok(output) => {
                let agent_exists = output.contains(&agent.name);
                if agent_exists {
                    match self.run_cli_command(&["list"]).await {
                        Ok(_) => Ok(true),
                        Err(_) => Ok(false),
                    }
                } else {
                    Ok(false)
                }
            }
            Err(_) => Ok(false),
        }
    }

    pub async fn send_task(&self, agent_name: &str, task: &str) -> Result<TaskResult> {
        let agent = self
            .agents
            .iter()
            .find(|a| a.name == agent_name)
            .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", agent_name))?;

        if !matches!(agent.status, AgentStatus::Available) {
            return Err(anyhow::anyhow!(
                "Agent {} is not available (status: {:?})",
                agent_name,
                agent.status
            ));
        }

        let start_time = std::time::Instant::now();
        info!("Sending task to {}: {}", agent_name, task);

        let result = match self.execute_agent_task(agent, task).await {
            Ok(output) => TaskResult {
                success: true,
                output,
                error: None,
                execution_time_ms: start_time.elapsed().as_millis() as u64,
            },
            Err(e) => TaskResult {
                success: false,
                output: String::new(),
                error: Some(format!("{}", e)),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
            },
        };

        if result.success {
            info!(
                "Task completed by {} in {}ms",
                agent_name, result.execution_time_ms
            );
        } else {
            error!("Task failed for {}: {:?}", agent_name, result.error);
        }

        Ok(result)
    }

    async fn execute_agent_task(&self, agent: &KowalskiAgent, task: &str) -> Result<String> {
        match agent.agent_type {
            KowalskiAgentType::Code => self.execute_code_task(agent, task).await,
            KowalskiAgentType::Web => self.execute_web_task(agent, task).await,
            KowalskiAgentType::Academic => self.execute_academic_task(agent, task).await,
            KowalskiAgentType::Data => self.execute_data_task(agent, task).await,
            KowalskiAgentType::Federated => self.execute_federated_task(agent, task),
        }
    }

    async fn execute_code_task(&self, agent: &KowalskiAgent, task: &str) -> Result<String> {
        info!("Executing code task on {}: {}", agent.name, task);

        let output = self
            .run_cli_command(&["chat", &agent.name])
            .await
            .with_context(|| format!("Failed to execute code task on {}", agent.name))?;

        Ok(format!(
            "Code analysis result from {}:\n{}",
            agent.name, output
        ))
    }

    async fn execute_web_task(&self, agent: &KowalskiAgent, task: &str) -> Result<String> {
        info!("Executing web task on {}: {}", agent.name, task);

        let output = self
            .run_cli_command(&["chat", &agent.name])
            .await
            .with_context(|| format!("Failed to execute web task on {}", agent.name))?;

        Ok(format!(
            "Web research result from {}:\n{}",
            agent.name, output
        ))
    }

    async fn execute_academic_task(&self, agent: &KowalskiAgent, task: &str) -> Result<String> {
        info!("Executing academic task on {}: {}", agent.name, task);

        let output = self
            .run_cli_command(&["chat", &agent.name])
            .await
            .with_context(|| format!("Failed to execute academic task on {}", agent.name))?;

        Ok(format!(
            "Academic research result from {}:\n{}",
            agent.name, output
        ))
    }

    async fn execute_data_task(&self, agent: &KowalskiAgent, task: &str) -> Result<String> {
        info!("Executing data task on {}: {}", agent.name, task);

        let output = self
            .run_cli_command(&["chat", &agent.name])
            .await
            .with_context(|| format!("Failed to execute data task on {}", agent.name))?;

        Ok(format!(
            "Data processing result from {}:\n{}",
            agent.name, output
        ))
    }

    fn execute_federated_task(&self, agent: &KowalskiAgent, task: &str) -> Result<String> {
        info!("Executing federated task on {}: {}", agent.name, task);

        let available_agents: Vec<_> = self
            .agents
            .iter()
            .filter(|a| matches!(a.status, AgentStatus::Available) && a.name != agent.name)
            .collect();

        if available_agents.is_empty() {
            return Ok("No other agents available for federation".to_string());
        }

        let mut results = vec![format!("Federated coordination by {}:\n", agent.name)];

        for other_agent in available_agents.iter().take(3) {
            results.push(format!("- Coordinated with {}", other_agent.name));
        }

        Ok(results.join("\n"))
    }

    pub fn get_agent_status(&self) -> Vec<(String, AgentStatus)> {
        self.agents
            .iter()
            .map(|a| (a.name.clone(), a.status.clone()))
            .collect()
    }

    pub fn get_health(&self) -> KowalskiHealth {
        let installed = self.cli_path.exists();
        let build_status = if installed {
            BuildStatus::Built
        } else {
            BuildStatus::NotBuilt
        };

        let available_agents: Vec<_> = self
            .agents
            .iter()
            .filter(|a| matches!(a.status, AgentStatus::Available))
            .map(|a| a.name.clone())
            .collect();

        let errors: Vec<_> = self
            .agents
            .iter()
            .filter_map(|a| match &a.status {
                AgentStatus::Error(e) => Some(format!("{}: {}", a.name, e)),
                _ => None,
            })
            .collect();

        KowalskiHealth {
            installed,
            build_status,
            available_agents: self.agents.iter().map(|a| a.name.clone()).collect(),
            active_agents: available_agents,
            errors,
        }
    }

    pub async fn build_kowalski(&self) -> Result<()> {
        info!("Building Kowalski from source...");

        let output = tokio::process::Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(&self.config.kowalski_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if output.status.success() {
            info!("Kowalski built successfully");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Failed to build Kowalski: {}", stderr))
        }
    }

    pub async fn test_kowalski(&self) -> Result<()> {
        info!("Running Kowalski tests...");

        let output = tokio::process::Command::new("cargo")
            .args(["test"])
            .current_dir(&self.config.kowalski_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if output.status.success() {
            info!("Kowalski tests passed");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Kowalski tests failed: {}", stderr))
        }
    }
}

#[derive(Debug, Clone)]
pub enum KowalskiTask {
    CodeAnalysis { path: PathBuf, language: String },
    WebSearch { query: String, max_results: usize },
    AcademicResearch { topic: String, sources: Vec<String> },
    DataProcessing { data: String, operation: String },
    FederatedCoordination { agents: Vec<String>, task: String },
}

impl KowalskiTask {
    pub fn to_task_string(&self) -> String {
        match self {
            KowalskiTask::CodeAnalysis { path, language } => {
                format!("Analyze {} code at: {}", language, path.display())
            }
            KowalskiTask::WebSearch { query, max_results } => {
                format!("Search web for: '{}' (max {} results)", query, max_results)
            }
            KowalskiTask::AcademicResearch { topic, sources } => {
                format!("Research: '{}' using sources: {:?}", topic, sources)
            }
            KowalskiTask::DataProcessing { data: _, operation } => {
                format!("Process data with operation: {}", operation)
            }
            KowalskiTask::FederatedCoordination { agents, task } => {
                format!("Coordinate agents {:?} for task: {}", agents, task)
            }
        }
    }
}

use crate::config::DelegateAgentConfig;
use crate::housaky::agent::KowalskiIntegrationConfig;
use crate::keys_manager::manager::{get_global_keys_manager, KeysManager, SubAgentConfig};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use tracing::{error, info, warn};

const DEFAULT_TIMEOUT_SECS: u64 = 120;
const MAX_RETRIES: u32 = 3;
const CACHE_TTL_SECS: u64 = 300;

/// Cache entry for task results
#[derive(Debug, Clone)]
struct CacheEntry {
    result: TaskResult,
    cached_at: Instant,
}

/// Progress callback type
pub type ProgressCallback = Box<dyn Fn(KowalskiProgress) + Send + Sync>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KowalskiProgress {
    Queued { task_id: String, position: usize },
    Starting { task_id: String, agent: String },
    Streaming { task_id: String, chunk: String },
    Completed { task_id: String },
    Failed { task_id: String, error: String },
    Retrying { task_id: String, attempt: u32, max_attempts: u32 },
}

/// Bridge to Kowalski multi-agent framework
/// Enables Housaky to coordinate with Kowalski agents via CLI
pub struct KowalskiBridge {
    config: KowalskiIntegrationConfig,
    agents: Vec<KowalskiAgent>,
    cli_path: Option<PathBuf>,
    keys_manager: Arc<KeysManager>,
    subagent_configs: HashMap<String, SubAgentConfig>,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    task_queue: Arc<RwLock<Vec<PendingTask>>>,
    progress_tx: Option<mpsc::Sender<KowalskiProgress>>,
}

#[derive(Debug, Clone)]
struct PendingTask {
    task_id: String,
    agent_name: String,
    task: String,
    created_at: Instant,
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
    Creative,
    Reasoning,
    Federated,
}

impl KowalskiAgentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            KowalskiAgentType::Code => "code",
            KowalskiAgentType::Web => "web",
            KowalskiAgentType::Academic => "academic",
            KowalskiAgentType::Data => "data",
            KowalskiAgentType::Creative => "creative",
            KowalskiAgentType::Reasoning => "reasoning",
            KowalskiAgentType::Federated => "federated",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            KowalskiAgentType::Code => "Code analysis, refactoring, and documentation",
            KowalskiAgentType::Web => "Web research and information retrieval",
            KowalskiAgentType::Academic => "Academic research and paper analysis",
            KowalskiAgentType::Data => "Data analysis and processing",
            KowalskiAgentType::Creative => "Creative synthesis and idea generation",
            KowalskiAgentType::Reasoning => "Logical reasoning and deduction",
            KowalskiAgentType::Federated => "Multi-agent coordination and federation",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "code" => Some(Self::Code),
            "web" => Some(Self::Web),
            "academic" => Some(Self::Academic),
            "data" => Some(Self::Data),
            "creative" => Some(Self::Creative),
            "reasoning" => Some(Self::Reasoning),
            "federated" | "federation" => Some(Self::Federated),
            _ => None,
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
        
        let cli_path = Self::find_kowalski_cli(&config.kowalski_path);
        
        if let Some(ref path) = cli_path {
            info!("Kowalski CLI found at: {}", path.display());
        }

        let keys_manager = get_global_keys_manager();
        
        // Load subagent configs synchronously
        let subagent_configs = Self::load_subagent_configs(&keys_manager);

        // Create agents based on config, but check if they have keys configured
        if config.enable_code_agent {
            let has_key = subagent_configs.contains_key("kowalski-code");
            agents.push(KowalskiAgent {
                name: "kowalski-code".to_string(),
                agent_type: KowalskiAgentType::Code,
                enabled: has_key,
                status: if has_key { AgentStatus::Available } else { AgentStatus::Offline },
                created_at: None,
                last_task: None,
                task_count: 0,
            });
        }

        if config.enable_web_agent {
            let has_key = subagent_configs.contains_key("kowalski-web");
            agents.push(KowalskiAgent {
                name: "kowalski-web".to_string(),
                agent_type: KowalskiAgentType::Web,
                enabled: has_key,
                status: if has_key { AgentStatus::Available } else { AgentStatus::Offline },
                created_at: None,
                last_task: None,
                task_count: 0,
            });
        }

        if config.enable_academic_agent {
            let has_key = subagent_configs.contains_key("kowalski-academic");
            agents.push(KowalskiAgent {
                name: "kowalski-academic".to_string(),
                agent_type: KowalskiAgentType::Academic,
                enabled: has_key,
                status: if has_key { AgentStatus::Available } else { AgentStatus::Offline },
                created_at: None,
                last_task: None,
                task_count: 0,
            });
        }

        if config.enable_data_agent {
            let has_key = subagent_configs.contains_key("kowalski-data");
            agents.push(KowalskiAgent {
                name: "kowalski-data".to_string(),
                agent_type: KowalskiAgentType::Data,
                enabled: has_key,
                status: if has_key { AgentStatus::Available } else { AgentStatus::Offline },
                created_at: None,
                last_task: None,
                task_count: 0,
            });
        }

        if config.enable_creative_agent {
            let has_key = subagent_configs.contains_key("kowalski-creative");
            agents.push(KowalskiAgent {
                name: "kowalski-creative".to_string(),
                agent_type: KowalskiAgentType::Creative,
                enabled: has_key,
                status: if has_key { AgentStatus::Available } else { AgentStatus::Offline },
                created_at: None,
                last_task: None,
                task_count: 0,
            });
        }

        if config.enable_reasoning_agent {
            let has_key = subagent_configs.contains_key("kowalski-reasoning");
            agents.push(KowalskiAgent {
                name: "kowalski-reasoning".to_string(),
                agent_type: KowalskiAgentType::Reasoning,
                enabled: has_key,
                status: if has_key { AgentStatus::Available } else { AgentStatus::Offline },
                created_at: None,
                last_task: None,
                task_count: 0,
            });
        }

        if config.enable_federation {
            let has_key = subagent_configs.contains_key("kowalski-federation");
            agents.push(KowalskiAgent {
                name: "kowalski-federation".to_string(),
                agent_type: KowalskiAgentType::Federated,
                enabled: has_key,
                status: if has_key { AgentStatus::Available } else { AgentStatus::Offline },
                created_at: None,
                last_task: None,
                task_count: 0,
            });
        }

        Self {
            config: config.clone(),
            agents,
            cli_path,
            keys_manager,
            subagent_configs,
            cache: Arc::new(RwLock::new(HashMap::new())),
            task_queue: Arc::new(RwLock::new(Vec::new())),
            progress_tx: None,
        }
    }

    /// Set progress callback channel
    pub fn with_progress_channel(mut self, tx: mpsc::Sender<KowalskiProgress>) -> Self {
        self.progress_tx = Some(tx);
        self
    }

    /// Get queue status
    pub async fn get_queue_status(&self) -> QueueStatus {
        let queue = self.task_queue.read().await;
        QueueStatus {
            pending: queue.len(),
            by_agent: queue.iter().fold(HashMap::new(), |mut acc, t| {
                *acc.entry(t.agent_name.clone()).or_insert(0) += 1;
                acc
            }),
        }
    }

    /// Clear expired cache entries
    pub async fn clear_expired_cache(&self) -> usize {
        let mut cache = self.cache.write().await;
        let now = Instant::now();
        let before = cache.len();
        cache.retain(|_, entry| now.duration_since(entry.cached_at).as_secs() < CACHE_TTL_SECS);
        before - cache.len()
    }

    /// Clear all cache
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }

    /// Get cache stats
    pub async fn get_cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        CacheStats {
            entries: cache.len(),
            oldest: cache.values().map(|e| e.cached_at).min(),
        }
    }

    fn generate_cache_key(agent_name: &str, task: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        agent_name.hash(&mut hasher);
        task.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn load_subagent_configs(keys_manager: &Arc<KeysManager>) -> HashMap<String, SubAgentConfig> {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(_) => return HashMap::new(),
        };

        rt.block_on(async {
            let _ = keys_manager.load().await;
            keys_manager.list_subagents().await.into_iter().collect()
        })
    }

    fn find_kowalski_cli(base_path: &PathBuf) -> Option<PathBuf> {
        let paths_to_check = vec![
            base_path.join("target/release/kowalski-cli"),
            base_path.join("target/debug/kowalski-cli"),
            PathBuf::from("/home/ubuntu/Housaky/vendor/kowalski/kowalski-cli/target/release/kowalski-cli"),
            PathBuf::from("/home/ubuntu/Housaky/vendor/kowalski/kowalski-cli/target/debug/kowalski-cli"),
            PathBuf::from("vendor/kowalski/kowalski-cli/target/release/kowalski-cli"),
            PathBuf::from("vendor/kowalski/kowalski-cli/target/debug/kowalski-cli"),
        ];

        for path in paths_to_check {
            if path.exists() {
                return Some(path);
            }
        }
        
        if base_path.join("Cargo.toml").exists() {
            return Some(base_path.join("target/release/kowalski-cli"));
        }

        None
    }

    fn get_cli_path(&self) -> Option<PathBuf> {
        self.cli_path.clone()
    }

    pub async fn check_kowalski(&self) -> Result<bool> {
        let cli = match self.get_cli_path() {
            Some(path) => path,
            None => {
                warn!("Kowalski CLI path not found");
                return Ok(false);
            }
        };
        
        if !cli.exists() {
            warn!("Kowalski CLI not built at: {}", cli.display());
            
            let kowalski_path = &self.config.kowalski_path;
            if kowalski_path.join("Cargo.toml").exists() {
                info!("Kowalski source exists at {} but CLI not built. Run 'cargo build --release' in kowalski-cli", kowalski_path.display());
            }
            return Ok(false);
        }

        match self.run_cli_command(&["--version"]).await {
            Ok(output) => {
                info!(
                    "Kowalski found at: {} (version: {})",
                    self.config.kowalski_path.display(),
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
        let cli = match self.get_cli_path() {
            Some(path) => path,
            None => bail!("Kowalski CLI path not configured"),
        };

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
        // Reload subagent configs
        self.subagent_configs = Self::load_subagent_configs(&self.keys_manager);
        
        // Update agent status based on config
        for agent in &mut self.agents {
            let has_key = self.subagent_configs.contains_key(&agent.name);
            agent.enabled = has_key;
            agent.status = if has_key { AgentStatus::Available } else { AgentStatus::Offline };
        }

        info!("Kowalski agents initialized from keys.json");
        Ok(())
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

        Ok(())
    }

    /// Send a task to a specific agent using its configured provider/key
    /// With caching, retry logic, and progress reporting
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

        let cache_key = Self::generate_cache_key(agent_name, task);
        
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(&cache_key) {
                if Instant::now().duration_since(entry.cached_at).as_secs() < CACHE_TTL_SECS {
                    info!("Returning cached result for task to {}", agent_name);
                    return Ok(entry.result.clone());
                }
            }
        }

        let start_time = Instant::now();
        info!("Sending task to {}: {}", agent_name, task);

        // Get subagent config from keys manager
        let subagent_config = self.subagent_configs.get(agent_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No configuration found for {} in keys.json", agent_name))?;

        // Get the specific key for this agent
        let key_result = self.keys_manager.get_key_for_subagent(agent_name).await
            .ok_or_else(|| anyhow::anyhow!("No key found for {} in keys.json", agent_name))?;

        let (model, key_entry) = key_result;
        
        // Execute with retry logic
        let mut last_error = None;
        for attempt in 1..=MAX_RETRIES {
            if attempt > 1 {
                let delay = Duration::from_secs(2u64.pow(attempt - 1));
                warn!("Retry attempt {} for {} after {}s", attempt, agent_name, delay.as_secs());
                tokio::time::sleep(delay).await;
                
                if let Some(ref tx) = self.progress_tx {
                    let _ = tx.send(KowalskiProgress::Retrying {
                        task_id: cache_key.clone(),
                        attempt,
                        max_attempts: MAX_RETRIES,
                    }).await;
                }
            }

            // Send starting progress
            if let Some(ref tx) = self.progress_tx {
                let _ = tx.send(KowalskiProgress::Starting {
                    task_id: cache_key.clone(),
                    agent: agent_name.to_string(),
                }).await;
            }

            match self.execute_with_provider(&subagent_config, &key_entry.key, &model, task).await {
                Ok(output) => {
                    let result = TaskResult {
                        success: true,
                        output,
                        error: None,
                        execution_time_ms: crate::util::time::duration_ms_u64(start_time.elapsed()),
                    };

                    // Cache the result
                    {
                        let mut cache = self.cache.write().await;
                        cache.insert(cache_key.clone(), CacheEntry {
                            result: result.clone(),
                            cached_at: Instant::now(),
                        });
                    }

                    // Send completion progress
                    if let Some(ref tx) = self.progress_tx {
                        let _ = tx.send(KowalskiProgress::Completed {
                            task_id: cache_key.clone(),
                        }).await;
                    }

                    info!(
                        "Task completed by {} in {}ms",
                        agent_name, result.execution_time_ms
                    );
                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(e);
                    // Check if error is retryable
                    let err_str = format!("{:?}", last_error);
                    if !err_str.contains("rate limit") && !err_str.contains("timeout") && !err_str.contains("429") {
                        break; // Non-retryable error
                    }
                }
            }
        }

        // All retries failed
        let result = TaskResult {
            success: false,
            output: String::new(),
            error: Some(format!("Failed after {} attempts: {:?}", MAX_RETRIES, last_error)),
            execution_time_ms: crate::util::time::duration_ms_u64(start_time.elapsed()),
        };

        // Send failure progress
        if let Some(ref tx) = self.progress_tx {
            let _ = tx.send(KowalskiProgress::Failed {
                task_id: cache_key.clone(),
                error: result.error.clone().unwrap_or_default(),
            }).await;
        }

        error!("Task failed for {}: {:?}", agent_name, result.error);
        Ok(result)
    }

    /// Send task with streaming support
    pub async fn send_task_streaming<F>(&self, agent_name: &str, task: &str, mut on_chunk: F) -> Result<TaskResult>
    where
        F: FnMut(String) + Send,
    {
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

        let cache_key = Self::generate_cache_key(agent_name, task);
        
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(&cache_key) {
                if Instant::now().duration_since(entry.cached_at).as_secs() < CACHE_TTL_SECS {
                    info!("Returning cached result for streaming task to {}", agent_name);
                    let result = entry.result.clone();
                    on_chunk(result.output.clone());
                    return Ok(result);
                }
            }
        }

        let start_time = Instant::now();
        
        let subagent_config = self.subagent_configs.get(agent_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No configuration found for {}", agent_name))?;

        let key_result = self.keys_manager.get_key_for_subagent(agent_name).await
            .ok_or_else(|| anyhow::anyhow!("No key found for {}", agent_name))?;

        let (model, key_entry) = key_result;
        
        let result = match self.execute_with_provider_streaming(&subagent_config, &key_entry.key, &model, task, &mut on_chunk).await {
            Ok(output) => {
                on_chunk(output.clone());
                TaskResult {
                    success: true,
                    output,
                    error: None,
                    execution_time_ms: crate::util::time::duration_ms_u64(start_time.elapsed()),
                }
            }
            Err(e) => TaskResult {
                success: false,
                output: String::new(),
                error: Some(format!("{}", e)),
                execution_time_ms: crate::util::time::duration_ms_u64(start_time.elapsed()),
            },
        };

        // Cache the result
        if result.success {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, CacheEntry {
                result: result.clone(),
                cached_at: Instant::now(),
            });
        }

        Ok(result)
    }

    /// Execute a task using the provider configured in keys.json
    async fn execute_with_provider(
        &self,
        config: &SubAgentConfig,
        api_key: &str,
        model: &str,
        task: &str,
    ) -> Result<String> {
        let system_prompt = self.get_system_prompt_for_agent_name(&config.key_name);
        
        info!(
            "Executing task with provider {} (model: {})",
            config.provider, model
        );

        self.execute_request(config.provider.as_str(), api_key, model, &system_prompt, task, false, None).await
    }

    /// Execute with streaming support
    async fn execute_with_provider_streaming<F>(
        &self,
        config: &SubAgentConfig,
        api_key: &str,
        model: &str,
        task: &str,
        on_chunk: &mut F,
    ) -> Result<String>
    where
        F: FnMut(String) + Send,
    {
        let system_prompt = self.get_system_prompt_for_agent_name(&config.key_name);
        
        info!(
            "Executing streaming task with provider {} (model: {})",
            config.provider, model
        );

        self.execute_request(config.provider.as_str(), api_key, model, &system_prompt, task, true, Some(on_chunk)).await
    }

    async fn execute_request<F>(
        &self,
        provider: &str,
        api_key: &str,
        model: &str,
        system_prompt: &str,
        task: &str,
        streaming: bool,
        mut on_chunk: Option<&mut F>,
    ) -> Result<String>
    where
        F: FnMut(String) + Send,
    {
        // Determine base URL based on provider
        let base_url = match provider {
            "modal" => "https://api.us-west-2.modal.direct/v1",
            "openrouter" => "https://openrouter.ai/api/v1",
            "openai" => "https://api.openai.com/v1",
            "anthropic" => "https://api.anthropic.com/v1",
            "ollama" => "http://localhost:11434/api/chat",
            _ => "https://api.openai.com/v1",
        };

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()?;

        if streaming && provider != "ollama" {
            // Use SSE streaming for OpenAI-compatible APIs
            let url = format!("{}/chat/completions", base_url);
            
            let request_body = serde_json::json!({
                "model": model,
                "messages": [
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": task}
                ],
                "temperature": 0.7,
                "max_tokens": 4096,
                "stream": true
            });

            let response = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .header("Accept", "text/event-stream")
                .json(&request_body)
                .send()
                .await
                .context("Failed to send streaming request")?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                bail!("Streaming request failed with status {}: {}", status, error_text);
            }

            let mut stream = response.bytes_stream();
            let mut full_output = String::new();

            use futures_util::StreamExt;
            while let Some(item) = stream.next().await {
                let chunk = item.context("Failed to read stream chunk")?;
                let text = String::from_utf8_lossy(&chunk);
                
                for line in text.lines() {
                    if line.starts_with("data: ") {
                        let data = &line[6..];
                        if data == "[DONE]" {
                            return Ok(full_output);
                        }
                        
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(content) = json.get("choices")
                                .and_then(|c| c.as_array())
                                .and_then(|c| c.first())
                                .and_then(|c| c.get("delta"))
                                .and_then(|d| d.get("content"))
                                .and_then(|c| c.as_str())
                            {
                                full_output.push_str(content);
                                if let Some(ref mut callback) = on_chunk {
                                    callback(content.to_string());
                                }
                            }
                        }
                    }
                }
            }

            Ok(full_output)
        } else {
            // Non-streaming request
            let request_body = if provider == "ollama" {
                serde_json::json!({
                    "model": model,
                    "messages": [
                        {"role": "system", "content": system_prompt},
                        {"role": "user", "content": task}
                    ],
                    "stream": false
                })
            } else {
                serde_json::json!({
                    "model": model,
                    "messages": [
                        {"role": "system", "content": system_prompt},
                        {"role": "user", "content": task}
                    ],
                    "temperature": 0.7,
                    "max_tokens": 4096
                })
            };

            let url = if provider == "ollama" {
                format!("{}/chat", base_url)
            } else {
                format!("{}/chat/completions", base_url)
            };
            
            let mut request = client
                .post(&url)
                .header("Content-Type", "application/json");

            if provider != "ollama" {
                request = request.header("Authorization", format!("Bearer {}", api_key));
            }
            
            let response = request
                .json(&request_body)
                .send()
                .await
                .context("Failed to send request to API")?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                bail!("API request failed with status {}: {}", status, error_text);
            }

            let response_body: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse API response")?;

        // Handle different response formats for different providers
        let content = if provider == "ollama" {
            // Ollama format: { "message": { "role": "assistant", "content": "..." } }
            response_body.get("message")
                .and_then(|msg| msg.get("content"))
                .and_then(|c| c.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    // Alternative: { "response": "..." }
                    response_body.get("response")
                        .and_then(|c| c.as_str())
                        .map(|s| s.to_string())
                })
                .context("Invalid Ollama response format")?
        } else {
            // OpenAI-compatible format: { "choices": [{ "message": { "content": "..." } }] }
            response_body["choices"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|choice| choice.get("message"))
                .and_then(|msg| msg.get("content"))
                .and_then(|c| c.as_str())
                .context("Invalid OpenAI response format")?
                .to_string()
        };

        Ok(content)
    }

    fn get_system_prompt_for_agent_name(&self, key_name: &str) -> String {
        // Map key names to agent types
        match key_name {
            name if name.contains("code") || name.contains("tupa") => {
                "You are a specialized code analysis agent. Your role is to analyze, refactor, \
                and document code. You have deep knowledge of programming patterns, best practices, \
                and can understand complex codebases. Provide clear, actionable insights."
            }
            name if name.contains("web") || name.contains("hautlythird") => {
                "You are a specialized web research agent. Your role is to search, retrieve, and \
                synthesize information from the web. You are skilled at finding relevant information \
                and presenting it in a clear, organized manner."
            }
            name if name.contains("academic") => {
                "You are a specialized academic research agent. Your role is to help with academic \
                research, paper analysis, and scholarly inquiry. You have knowledge of academic \
                databases, citation styles, and research methodologies."
            }
            name if name.contains("data") || name.contains("touch") => {
                "You are a specialized data analysis agent. Your role is to process, analyze, and \
                transform data. You have expertise in data manipulation, statistical analysis, and \
                data visualization."
            }
            name if name.contains("creative") || name.contains("rouxy") => {
                "You are a specialized creative synthesis agent. Your role is to generate novel \
                ideas, creative solutions, and innovative approaches. You excel at brainstorming \
                and thinking outside the box. Be imaginative and inspiring."
            }
            name if name.contains("reasoning") || name.contains("hautly") => {
                "You are a specialized reasoning engine. Your role is to apply logical deduction, \
                step-by-step analysis, and critical thinking to solve complex problems. You excel \
                at breaking down complex issues and finding elegant solutions."
            }
            name if name.contains("federation") || name.contains("housaky") => {
                "You are a federated coordination agent. Your role is to coordinate multiple \
                specialized agents to work together on complex tasks. You can delegate subtasks \
                to other agents and synthesize their results."
            }
            _ => "You are a specialized AI agent. Provide helpful, accurate, and relevant responses."
        }.to_string()
    }

    pub fn get_agent_status(&self) -> Vec<(String, AgentStatus)> {
        self.agents
            .iter()
            .map(|a| (a.name.clone(), a.status.clone()))
            .collect()
    }

    pub fn get_health(&self) -> KowalskiHealth {
        let installed = self.cli_path.as_ref().map(|p| p.exists()).unwrap_or(false);
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

    /// Get delegate agent configs for the orchestrator's delegate tool
    /// This maps each subagent to a DelegateAgentConfig with proper provider/key
    pub fn get_delegate_configs(&self) -> HashMap<String, DelegateAgentConfig> {
        let mut configs = HashMap::new();

        for agent in &self.agents {
            if !matches!(agent.status, AgentStatus::Available | AgentStatus::Offline) {
                continue;
            }

            // Get subagent config from keys manager
            if let Some(subagent_config) = self.subagent_configs.get(&agent.name) {
                // Get the specific key for this agent
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(_) => continue,
                };

                let key_result = rt.block_on(async {
                    self.keys_manager.get_key_for_subagent(&agent.name).await
                });

                if let Some((model, key_entry)) = key_result {
                    let system_prompt = self.get_system_prompt_for_agent_name(&subagent_config.key_name);
                    
                    let config = DelegateAgentConfig {
                        provider: subagent_config.provider.clone(),
                        model: model.clone(),
                        system_prompt: Some(system_prompt),
                        api_key: Some(key_entry.key.clone()),
                        temperature: Some(0.7),
                        max_depth: 3,
                        is_kowalski_agent: false, // Use the standard provider path
                        glm_api_key: None,
                        glm_model: String::new(),
                        glm_per_agent: HashMap::new(),
                    };

                    configs.insert(agent.name.clone(), config);
                }
            }
        }

        configs
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalOrientedTask {
    pub goal_id: String,
    pub goal_title: String,
    pub goal_description: String,
    pub priority: String,
    pub karma_reward: f64,
    pub task: String,
    pub context: HashMap<String, String>,
}

impl GoalOrientedTask {
    pub fn from_kowalski_response(goal_id: String, response: String, context: HashMap<String, String>) -> Self {
        let (title, description, priority, karma) = Self::parse_kowalski_response(&response);
        
        Self {
            goal_id,
            goal_title: title,
            goal_description: description,
            priority,
            karma_reward: karma,
            task: response,
            context,
        }
    }

    fn parse_kowalski_response(response: &str) -> (String, String, String, f64) {
        let lines: Vec<&str> = response.lines().collect();
        let mut title = "Kowalski Task".to_string();
        let mut description = response.to_string();
        let mut priority = "Medium".to_string();
        let mut karma = 25.0;

        for line in &lines {
            if line.starts_with("TITLE:") {
                title = line.trim_start_matches("TITLE:").trim().to_string();
            } else if line.starts_with("PRIORITY:") {
                priority = line.trim_start_matches("PRIORITY:").trim().to_string();
                karma = match priority.to_lowercase().as_str() {
                    "low" => 10.0,
                    "high" => 50.0,
                    "critical" => 100.0,
                    "urgent" => 200.0,
                    _ => 25.0,
                };
            }
        }

        (title, description, priority, karma)
    }
}

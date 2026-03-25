use jsonrpsee::server::{Server, ServerHandle};
use jsonrpsee::types::error::ErrorObject;
use jsonrpsee::core::RpcResult;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;
use tracing::{info, error};

use crate::rpc::handler::RpcHandler;
use crate::rpc::handler::DefaultRpcHandler;

/// RPC server for Hermes-Housaky integration
pub struct RpcServer {
    handle: Option<ServerHandle>,
    socket_path: PathBuf,
}

impl RpcServer {
    /// Create a new RPC server that will listen on the given socket path
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            handle: None,
            socket_path,
        }
    }

    /// Start the RPC server with the given handler
    pub async fn start<H: RpcHandler + Send + Sync + 'static>(&mut self, handler: Arc<H>) -> jsonrpsee::Result<()> {
        // Ensure the directory exists
        if let Some(parent) = self.socket_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Remove existing socket file if present
        if self.socket_path.exists() {
            let _ = tokio::fs::remove_file(&self.socket_path).await;
        }

        // Build the server
        let server = Server::builder()
            .set_max_request_body_size(1024 * 1024) // 1MB
            .build(&format!("unix://{}", self.socket_path.display()))
            .await?;

        // Register the handler
        let handle = server.start(handler.into_rpc());

        info!("Housaky RPC server started on {}", self.socket_path.display());

        self.handle = Some(handle);

        // Wait for server to be stopped (this method will block until shutdown)
        // Actually, we want to return immediately and let the server run in the background.
        // The handle can be used to stop the server later.
        Ok(())
    }

    /// Stop the RPC server
    pub async fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.stop().await;
            info!("Housaky RPC server stopped");
        }
    }
}

/// Default RPC handler that uses housaky subsystems
pub struct DefaultRpcHandler {
    workspace_dir: PathBuf,
}

impl DefaultRpcHandler {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
    
    fn get_memory_store(&self) -> RpcResult<crate::housaky::memory::AgentMemoryStore> {
        crate::housaky::memory::AgentMemoryStore::open(&self.workspace_dir)
            .map_err(|e| ErrorObject::owned(-32000, format!("Failed to open memory store: {}", e), None::<()>))
    }
}

#[async_trait::async_trait]
impl RpcHandler for DefaultRpcHandler {
    // Memory methods
    async fn memory_store(&self, key: String, value: String) -> RpcResult<()> {
        info!("RPC memory_store: {} = {}", key, value);
        
        let store = self.get_memory_store()?;
        let record = crate::housaky::memory::AgentMemoryRecord {
            id: key.clone(),
            kind: crate::housaky::memory::MemoryKind::Fact,
            content: value,
            source: "rpc".to_string(),
            confidence: 1.0,
            importance: 0.5,
            tags: vec![],
            created_at: chrono::Utc::now(),
            accessed_at: chrono::Utc::now(),
            access_count: 0,
        };
        
        store.store(&record)
            .map_err(|e| ErrorObject::owned(-32001, format!("Failed to store memory: {}", e), None::<()>))?;
        
        Ok(())
    }

    async fn memory_recall(&self, query: String) -> RpcResult<Option<String>> {
        info!("RPC memory_recall: {}", query);
        
        let store = self.get_memory_store()?;
        let results = store.search(&query, 1)
            .map_err(|e| ErrorObject::owned(-32002, format!("Failed to recall memory: {}", e), None::<()>))?;
        
        Ok(results.first().map(|r| r.content.clone()))
    }

    async fn memory_search(&self, query: String, limit: usize) -> RpcResult<Vec<String>> {
        info!("RPC memory_search: {} (limit: {})", query, limit);
        
        let store = self.get_memory_store()?;
        let results = store.search(&query, limit)
            .map_err(|e| ErrorObject::owned(-32003, format!("Failed to search memory: {}", e), None::<()>))?;
        
        Ok(results.iter().map(|r| format!("{}: {}", r.id, r.content)).collect())
    }

    // Skill methods
    async fn skill_list(&self) -> RpcResult<Vec<String>> {
        info!("RPC skill_list");
        
        let skills_dir = self.workspace_dir.join("skills");
        let mut skills = Vec::new();
        
        if let Ok(entries) = std::fs::read_dir(&skills_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        skills.push(name.to_string());
                    }
                }
            }
        }
        
        Ok(skills)
    }

    async fn skill_get(&self, name: String) -> RpcResult<Option<String>> {
        info!("RPC skill_get: {}", name);
        
        let skill_file = self.workspace_dir.join("skills").join(&name).join("SKILL.md");
        
        if skill_file.exists() {
            match std::fs::read_to_string(&skill_file) {
                Ok(content) => Ok(Some(content)),
                Err(e) => Err(ErrorObject::owned(-32004, format!("Failed to read skill: {}", e), None::<()>)),
            }
        } else {
            Ok(None)
        }
    }

    async fn skill_run(&self, name: String, inputs: serde_json::Value) -> RpcResult<serde_json::Value> {
        info!("RPC skill_run: {} with inputs: {}", name, inputs);
        
        // For now, return a placeholder - skill execution requires more infrastructure
        Ok(serde_json::json!({
            "status": "skill_execution_not_implemented",
            "skill": name,
            "inputs": inputs
        }))
    }

    // A2A methods
    async fn a2a_send(&self, message: String, target: String) -> RpcResult<()> {
        info!("RPC a2a_send to {}: {}", target, message);
        
        let shared_dir = self.workspace_dir.join("shared").join("a2a");
        let endpoint = crate::housaky::a2a::A2AEndpoint::new(shared_dir, "rpc", &target);
        
        let msg = crate::housaky::a2a::A2AMessage::learning("rpc", &target, "message", &message, 1.0);
        
        endpoint.send(&msg).await
            .map_err(|e| ErrorObject::owned(-32005, format!("Failed to send A2A: {}", e), None::<()>))?;
        
        Ok(())
    }

    async fn a2a_recv(&self, from: String) -> RpcResult<Option<String>> {
        info!("RPC a2a_recv from {}", from);
        
        let shared_dir = self.workspace_dir.join("shared").join("a2a");
        let endpoint = crate::housaky::a2a::A2AEndpoint::new(shared_dir, "rpc", &from);
        
        let messages = endpoint.read_from(&from)
            .map_err(|e| ErrorObject::owned(-32006, format!("Failed to recv A2A: {}", e), None::<()>))?;
        
        Ok(messages.first().map(|m| m.to_compact_json().unwrap_or_default()))
    }

    async fn a2a_delegate(&self, task_id: String, action: String, params: serde_json::Value) -> RpcResult<()> {
        info!("RPC a2a_delegate: {} {} {:?}", task_id, action, params);
        
        let shared_dir = self.workspace_dir.join("shared").join("a2a");
        let endpoint = crate::housaky::a2a::A2AEndpoint::new(shared_dir, "rpc", "peer");
        
        let msg = crate::housaky::a2a::A2AMessage::task("rpc", "peer", &task_id, &action, params);
        
        endpoint.send(&msg).await
            .map_err(|e| ErrorObject::owned(-32007, format!("Failed to delegate: {}", e), None::<()>))?;
        
        Ok(())
    }

    async fn a2a_sync(&self, timeout: u64) -> RpcResult<Option<String>> {
        info!("RPC a2a_sync: timeout {}", timeout);
        
        let shared_dir = self.workspace_dir.join("shared").join("a2a");
        let endpoint = crate::housaky::a2a::A2AEndpoint::new(shared_dir, "rpc", "peer");
        
        let msg = crate::housaky::a2a::A2AMessage::sync_request("rpc", "peer");
        
        match endpoint.send_and_wait(&msg, timeout).await {
            Ok(Some(response)) => Ok(Some(response.to_compact_json().unwrap_or_default())),
            Ok(None) => Ok(None),
            Err(e) => Err(ErrorObject::owned(-32008, format!("Sync failed: {}", e), None::<()>)),
        }
    }

    async fn a2a_share_learning(&self, category: String, content: String, confidence: f32) -> RpcResult<()> {
        info!("RPC a2a_share_learning: {} (confidence: {})", category, confidence);
        
        let shared_dir = self.workspace_dir.join("shared").join("a2a");
        let endpoint = crate::housaky::a2a::A2AEndpoint::new(shared_dir, "rpc", "peer");
        
        let msg = crate::housaky::a2a::A2AMessage::learning("rpc", "peer", &category, &content, confidence);
        
        endpoint.send(&msg).await
            .map_err(|e| ErrorObject::owned(-32009, format!("Failed to share learning: {}", e), None::<()>))?;
        
        Ok(())
    }

    // Goal methods
    async fn goal_set(&self, description: String) -> RpcResult<()> {
        info!("RPC goal_set: {}", description);
        
        let goal_engine = crate::housaky::goal_engine::GoalEngine::new(&self.workspace_dir);
        
        let goal = crate::housaky::goal_engine::Goal {
            id: format!("rpc-{}", chrono::Utc::now().timestamp()),
            title: description.clone(),
            description,
            priority: crate::housaky::goal_engine::GoalPriority::Medium,
            status: crate::housaky::goal_engine::GoalStatus::Pending,
            category: crate::housaky::goal_engine::GoalCategory::UserRequest,
            progress: 0.0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deadline: None,
            parent_id: None,
            subtask_ids: vec![],
            dependencies: vec![],
            estimated_effort: None,
            actual_effort: None,
            tags: vec!["rpc".to_string()],
            metadata: serde_json::json!({}),
        };
        
        goal_engine.add_goal(goal).await
            .map_err(|e| ErrorObject::owned(-32010, format!("Failed to set goal: {}", e), None::<()>))?;
        
        Ok(())
    }

    async fn goal_list(&self) -> RpcResult<Vec<String>> {
        info!("RPC goal_list");
        
        let goal_engine = crate::housaky::goal_engine::GoalEngine::new(&self.workspace_dir);
        let goals = goal_engine.get_active_goals().await;
        
        Ok(goals.iter().map(|g| format!("{}: {} ({:.0}%)", g.id, g.title, g.progress * 100.0)).collect())
    }

    async fn goal_progress(&self, description: String) -> RpcResult<f32> {
        info!("RPC goal_progress: {}", description);
        
        let goal_engine = crate::housaky::goal_engine::GoalEngine::new(&self.workspace_dir);
        let goals = goal_engine.get_active_goals().await;
        
        let progress = goals
            .iter()
            .find(|g| g.description == description || g.title == description)
            .map(|g| g.progress as f32)
            .unwrap_or(0.0);
        
        Ok(progress)
    }

    async fn goal_evaluate(&self, description: String) -> RpcResult<bool> {
        info!("RPC goal_evaluate: {}", description);
        
        let goal_engine = crate::housaky::goal_engine::GoalEngine::new(&self.workspace_dir);
        let goals = goal_engine.get_active_goals().await;
        
        let completed = goals
            .iter()
            .find(|g| g.description == description || g.title == description)
            .map(|g| g.status == crate::housaky::goal_engine::GoalStatus::Completed)
            .unwrap_or(false);
        
        Ok(completed)
    }

    // Heartbeat methods
    async fn heartbeat_trigger(&self) -> RpcResult<()> {
        info!("RPC heartbeat_trigger");
        let heartbeat_file = self.workspace_dir.join(".housaky").join("heartbeat_trigger");
        std::fs::write(&heartbeat_file, chrono::Utc::now().to_rfc3339())
            .map_err(|e| ErrorObject::owned(-32011, format!("Failed to trigger heartbeat: {}", e), None::<()>))?;
        Ok(())
    }

    async fn heartbeat_status(&self) -> RpcResult<bool> {
        info!("RPC heartbeat_status");
        let heartbeat_file = self.workspace_dir.join(".housaky").join("heartbeat_last");
        if heartbeat_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&heartbeat_file) {
                if let Ok(last) = chrono::DateTime::parse_from_rfc3339(&content) {
                    let elapsed = (chrono::Utc::now() - last.with_timezone(&chrono::Utc)).num_seconds();
                    return Ok(elapsed < 300);
                }
            }
        }
        Ok(false)
    }

    async fn heartbeat_configure(&self, interval_seconds: u64) -> RpcResult<()> {
        info!("RPC heartbeat_configure: interval {}", interval_seconds);
        let config_file = self.workspace_dir.join(".housaky").join("heartbeat_config.json");
        let config = serde_json::json!({ "interval_seconds": interval_seconds });
        std::fs::write(&config_file, serde_json::to_string_pretty(&config).unwrap())
            .map_err(|e| ErrorObject::owned(-32012, format!("Failed to configure heartbeat: {}", e), None::<()>))?;
        Ok(())
    }

    // Configuration methods
    async fn config_get(&self, key: String) -> RpcResult<Option<String>> {
        info!("RPC config_get: {}", key);
        let config_file = self.workspace_dir.join("config.toml");
        if let Ok(content) = std::fs::read_to_string(&config_file) {
            if let Ok(config) = content.parse::<toml::Value>() {
                // Support nested keys like "skills.enabled.my-skill"
                let parts: Vec<&str> = key.split('.').collect();
                let mut current = Some(&config);
                for part in &parts[..parts.len()-1] {
                    current = current.and_then(|v| v.get(part));
                }
                if let Some(value) = current.and_then(|v| v.get(parts.last().unwrap())) {
                    return Ok(Some(value.to_string()));
                }
            }
        }
        Ok(None)
    }

    async fn config_set(&self, key: String, value: String) -> RpcResult<()> {
        info!("RPC config_set: {} = {}", key, value);
        let config_file = self.workspace_dir.join("config.toml");
        let config_content = std::fs::read_to_string(&config_file)
            .map_err(|e| ErrorObject::owned(-32013, format!("Failed to read config: {}", e), None::<()>))?;
        
        let mut config: toml::Value = config_content.parse()
            .map_err(|e| ErrorObject::owned(-32014, format!("Failed to parse config: {}", e), None::<()>))?;
        
        // Support nested keys
        let parts: Vec<&str> = key.split('.').collect();
        if parts.len() == 1 {
            if let Some(table) = config.as_table_mut() {
                table.insert(key.clone(), toml::Value::String(value));
            }
        } else {
            let mut current = Some(&mut config);
            for part in &parts[..parts.len()-1] {
                current = current.and_then(|v| v.get_mut(part));
            }
            if let Some(table) = current.and_then(|v| v.as_table_mut()) {
                table.insert(parts.last().unwrap().to_string(), toml::Value::String(value));
            }
        }
        
        let new_config = toml::to_string_pretty(&config)
            .map_err(|e| ErrorObject::owned(-32015, format!("Failed to serialize config: {}", e), None::<()>))?;
        
        std::fs::write(&config_file, new_config)
            .map_err(|e| ErrorObject::owned(-32016, format!("Failed to write config: {}", e), None::<()>))?;
        
        Ok(())
    }

    async fn config_list(&self) -> RpcResult<Vec<(String, String)>> {
        info!("RPC config_list");
        let config_file = self.workspace_dir.join("config.toml");
        let mut result = Vec::new();
        
        if let Ok(content) = std::fs::read_to_string(&config_file) {
            if let Ok(config) = content.parse::<toml::Value>() {
                fn flatten_table(table: &toml::map::Map<String, toml::Value>, prefix: &str, result: &mut Vec<(String, String)>) {
                    for (k, v) in table {
                        let key = if prefix.is_empty() { k.clone() } else { format!("{}.{}", prefix, k) };
                        match v {
                            toml::Value::Table(t) => flatten_table(t, &key, result),
                            other => result.push((key, other.to_string())),
                        }
                    }
                }
                if let Some(table) = config.as_table() {
                    flatten_table(table, "", &mut result);
                }
            }
        }
        
        Ok(result)
    }

    // System methods
    async fn system_version(&self) -> RpcResult<String> {
        info!("RPC system_version");
        // Uses CARGO_PKG_VERSION from Cargo.toml at compile time
        Ok(env!("CARGO_PKG_VERSION").to_string())
    }

    async fn system_stats(&self) -> RpcResult<String> {
        info!("RPC system_stats");
        let stats = serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "workspace": self.workspace_dir.to_string_lossy(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        Ok(stats.to_string())
    }
}

// Define the RPC trait that the server expects
#[async_trait::async_trait]
pub trait RpcHandler: Send + Sync {
    // Memory
    async fn memory_store(&self, key: String, value: String) -> RpcResult<()>;
    async fn memory_recall(&self, query: String) -> RpcResult<Option<String>>;
    async fn memory_search(&self, query: String, limit: usize) -> RpcResult<Vec<String>>;

    // Skills
    async fn skill_list(&self) -> RpcResult<Vec<String>>;
    async fn skill_get(&self, name: String) -> RpcResult<Option<String>>;
    async fn skill_run(&self, name: String, inputs: serde_json::Value) -> RpcResult<serde_json::Value>;

    // A2A
    async fn a2a_send(&self, message: String, target: String) -> RpcResult<()>;
    async fn a2a_recv(&self, from: String) -> RpcResult<Option<String>>;
    async fn a2a_delegate(&self, task_id: String, action: String, params: serde_json::Value) -> RpcResult<()>;
    async fn a2a_sync(&self, timeout: u64) -> RpcResult<Option<String>>;
    async fn a2a_share_learning(&self, category: String, content: String, confidence: f32) -> RpcResult<()>;

    // Goals
    async fn goal_set(&self, description: String) -> RpcResult<()>;
    async fn goal_list(&self) -> RpcResult<Vec<String>>;
    async fn goal_progress(&self, description: String) -> RpcResult<f32>;
    async fn goal_evaluate(&self, description: String) -> RpcResult<bool>;

    // Heartbeat
    async fn heartbeat_trigger(&self) -> RpcResult<()>;
    async fn heartbeat_status(&self) -> RpcResult<bool>;
    async fn heartbeat_configure(&self, interval_seconds: u64) -> RpcResult<()>;

    // Configuration
    async fn config_get(&self, key: String) -> RpcResult<Option<String>>;
    async fn config_set(&self, key: String, value: String) -> RpcResult<()>;
    async fn config_list(&self) -> RpcResult<Vec<(String, String)>>;

    // System
    async fn system_version(&self) -> RpcResult<String>;
    async fn system_stats(&self) -> RpcResult<String>;
}

// Helper to convert our handler into the jsonrpsee server module
trait IntoRpc {
    fn into_rpc(self) -> jsonrpsee::server::ServerModule<()>;
}

impl<H: RpcHandler + Send + Sync + 'static> IntoRpc for Arc<H> {
    fn into_rpc(self) -> jsonrpsee::server::ServerModule<()> {
        let mut module = jsonrpsee::server::ServerModule::new(());

        // Memory
        module.register_method("memory_store", move |h: &Arc<H>, key: String, value: String| {
            let h = h.clone();
            async move { h.memory_store(key, value).await }
        });
        module.register_method("memory_recall", move |h: &Arc<H>, query: String| {
            let h = h.clone();
            async move { h.memory_recall(query).await }
        });
        module.register_method("memory_search", move |h: &Arc<H>, (query, limit): (String, usize)| {
            let h = h.clone();
            async move { h.memory_search(query, limit).await }
        });

        // Skills
        module.register_method("skill_list", move |h: &Arc<H>, ()| {
            let h = h.clone();
            async move { h.skill_list().await }
        });
        module.register_method("skill_get", move |h: &Arc<H>, name: String| {
            let h = h.clone();
            async move { h.skill_get(name).await }
        });
        module.register_method("skill_run", move |h: &Arc<H>, (name, inputs): (String, serde_json::Value)| {
            let h = h.clone();
            async move { h.skill_run(name, inputs).await }
        });

        // A2A
        module.register_method("a2a_send", move |h: &Arc<H>, (message, target): (String, String)| {
            let h = h.clone();
            async move { h.a2a_send(message, target).await }
        });
        module.register_method("a2a_recv", move |h: &Arc<H>, from: String| {
            let h = h.clone();
            async move { h.a2a_recv(from).await }
        });
        module.register_method("a2a_delegate", move |h: &Arc<H>, (task_id, action, params): (String, String, serde_json::Value)| {
            let h = h.clone();
            async move { h.a2a_delegate(task_id, action, params).await }
        });
        module.register_method("a2a_sync", move |h: &Arc<H>, timeout: u64| {
            let h = h.clone();
            async move { h.a2a_sync(timeout).await }
        });
        module.register_method("a2a_share_learning", move |h: &Arc<H>, (category, content, confidence): (String, String, f32)| {
            let h = h.clone();
            async move { h.a2a_share_learning(category, content, confidence).await }
        });

        // Goals
        module.register_method("goal_set", move |h: &Arc<H>, description: String| {
            let h = h.clone();
            async move { h.goal_set(description).await }
        });
        module.register_method("goal_list", move |h: &Arc<H>, ()| {
            let h = h.clone();
            async move { h.goal_list().await }
        });
        module.register_method("goal_progress", move |h: &Arc<H>, description: String| {
            let h = h.clone();
            async move { h.goal_progress(description).await }
        });
        module.register_method("goal_evaluate", move |h: &Arc<H>, description: String| {
            let h = h.clone();
            async move { h.goal_evaluate(description).await }
        });

        // Heartbeat
        module.register_method("heartbeat_trigger", move |h: &Arc<H>, ()| {
            let h = h.clone();
            async move { h.heartbeat_trigger().await }
        });
        module.register_method("heartbeat_status", move |h: &Arc<H>, ()| {
            let h = h.clone();
            async move { h.heartbeat_status().await }
        });
        module.register_method("heartbeat_configure", move |h: &Arc<H>, interval_seconds: u64| {
            let h = h.clone();
            async move { h.heartbeat_configure(interval_seconds).await }
        });

        // Configuration
        module.register_method("config_get", move |h: &Arc<H>, key: String| {
            let h = h.clone();
            async move { h.config_get(key).await }
        });
        module.register_method("config_set", move |h: &Arc<H>, (key, value): (String, String)| {
            let h = h.clone();
            async move { h.config_set(key, value).await }
        });
        module.register_method("config_list", move |h: &Arc<H>, ()| {
            let h = h.clone();
            async move { h.config_list().await }
        });

        // System
        module.register_method("system_version", move |h: &Arc<H>, ()| {
            let h = h.clone();
            async move { h.system_version().await }
        });
        module.register_method("system_stats", move |h: &Arc<H>, ()| {
            let h = h.clone();
            async move { h.system_stats().await }
        });

        module
    }
}

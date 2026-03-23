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
    // We'll add references to housaky subsystems as needed
    // For now, we'll just return placeholder responses
}

impl DefaultRpcHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl RpcHandler for DefaultRpcHandler {
    // Memory methods
    async fn memory_store(&self, key: String, value: String) -> RpcResult<()> {
        // TODO: Implement using housaky memory
        info!("RPC memory_store: {} = {}", key, value);
        Ok(())
    }

    async fn memory_recall(&self, query: String) -> RpcResult<Option<String>> {
        // TODO: Implement using housaky memory
        info!("RPC memory_recall: {}", query);
        Ok(None)
    }

    async fn memory_search(&self, query: String, limit: usize) -> RpcResult<Vec<String>> {
        // TODO: Implement using housaky memory
        info!("RPC memory_search: {} (limit: {})", query, limit);
        Ok(vec![])
    }

    // Skill methods
    async fn skill_list(&self) -> RpcResult<Vec<String>> {
        // TODO: Implement using housaky skills
        info!("RPC skill_list");
        Ok(vec![])
    }

    async fn skill_get(&self, name: String) -> RpcResult<Option<String>> {
        // TODO: Implement using housaky skills
        info!("RPC skill_get: {}", name);
        Ok(None)
    }

    async fn skill_run(&self, name: String, inputs: serde_json::Value) -> RpcResult<serde_json::Value> {
        // TODO: Implement using housaky skills
        info!("RPC skill_run: {} with inputs: {}", name, inputs);
        Ok(serde_json::json!({}))
    }

    // A2A methods
    async fn a2a_send(&self, message: String, target: String) -> RpcResult<()> {
        // TODO: Implement using housaky A2A
        info!("RPC a2a_send to {}: {}", target, message);
        Ok(())
    }

    async fn a2a_recv(&self, from: String) -> RpcResult<Option<String>> {
        // TODO: Implement using housaky A2A
        info!("RPC a2a_recv from {}", from);
        Ok(None)
    }

    async fn a2a_delegate(&self, task_id: String, action: String, params: serde_json::Value) -> RpcResult<()> {
        // TODO: Implement using housaky A2A
        info!("RPC a2a_delegate: {} {} {:?}", task_id, action, params);
        Ok(())
    }

    async fn a2a_sync(&self, timeout: u64) -> RpcResult<Option<String>> {
        // TODO: Implement using housaky A2A
        info!("RPC a2a_sync: timeout {}", timeout);
        Ok(None)
    }

    async fn a2a_share_learning(&self, category: String, content: String, confidence: f32) -> RpcResult<()> {
        // TODO: Implement using housaky A2A
        info!("RPC a2a_share_learning: {} (confidence: {})", category, confidence);
        Ok(())
    }

    // Goal methods
    async fn goal_set(&self, description: String) -> RpcResult<()> {
        // TODO: Implement using housaky goals
        info!("RPC goal_set: {}", description);
        Ok(())
    }

    async fn goal_list(&self) -> RpcResult<Vec<String>> {
        // TODO: Implement using housaky goals
        info!("RPC goal_list");
        Ok(vec![])
    }

    async fn goal_progress(&self, description: String) -> RpcResult<f32> {
        // TODO: Implement using housaky goals
        info!("RPC goal_progress: {}", description);
        Ok(0.0)
    }

    async fn goal_evaluate(&self, description: String) -> RpcResult<bool> {
        // TODO: Implement using housaky goals
        info!("RPC goal_evaluate: {}", description);
        Ok(false)
    }

    // Heartbeat methods
    async fn heartbeat_trigger(&self) -> RpcResult<()> {
        // TODO: Implement using housaky heartbeat
        info!("RPC heartbeat_trigger");
        Ok(())
    }

    async fn heartbeat_status(&self) -> RpcResult<bool> {
        // TODO: Implement using housaky heartbeat
        info!("RPC heartbeat_status");
        Ok(false)
    }

    async fn heartbeat_configure(&self, interval_seconds: u64) -> RpcResult<()> {
        // TODO: Implement using housaky heartbeat
        info!("RPC heartbeat_configure: interval {}", interval_seconds);
        Ok(())
    }

    // Configuration methods
    async fn config_get(&self, key: String) -> RpcResult<Option<String>> {
        // TODO: Implement using housaky config
        info!("RPC config_get: {}", key);
        Ok(None)
    }

    async fn config_set(&self, key: String, value: String) -> RpcResult<()> {
        // TODO: Implement using housaky config
        info!("RPC config_set: {} = {}", key, value);
        Ok(())
    }

    async fn config_list(&self) -> RpcResult<Vec<(String, String)>> {
        // TODO: Implement using housaky config
        info!("RPC config_list");
        Ok(vec![])
    }

    // System methods
    async fn system_version(&self) -> RpcResult<String> {
        // TODO: Get from housaky config or Cargo.toml
        info!("RPC system_version");
        Ok(env!("CARGO_PKG_VERSION").to_string())
    }

    async fn system_stats(&self) -> RpcResult<String> {
        // TODO: Implement using housaky stats
        info!("RPC system_stats");
        Ok("{}".to_string())
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
EOF; __hermes_rc=$?; printf '__HERMES_FENCE_a9f7b3__'; exit $__hermes_rc

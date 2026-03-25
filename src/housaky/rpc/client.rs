use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::rpc_params;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::core::Error as JsonRpcSeeError;
use std::path::PathBuf;

/// RPC client for communicating with housaky daemon
pub struct RpcClient {
    client: HttpClient,
}

type HttpClient = jsonrpsee::http_client::HttpClient;

impl RpcClient {
    /// Create a new RPC client that connects to the given socket path
    pub fn new(socket_path: PathBuf) -> Result<Self, JsonRpcSeeError> {
        let url = format!("http://localhost/",); // dummy URL, we'll override the transport
        let client = HttpClientBuilder::default()
            .build(url)?
            .replace_transport(jsonrpsee::client_transport::local::UnixSocketTransport::new(
                socket_path,
            )?);

        Ok(Self { client })
    }
}

// Memory methods
impl RpcClient {
    pub async fn memory_store(&self, key: String, value: String) -> Result<(), JsonRpcSeeError> {
        self.client.request("memory_store", rpc_params![key, value]).await
    }

    pub async fn memory_recall(&self, query: String) -> Result<Option<String>, JsonRpcSeeError> {
        self.client.request("memory_recall", rpc_params![query]).await
    }

    pub async fn memory_search(&self, query: String, limit: usize) -> Result<Vec<String>, JsonRpcSeeError> {
        self.client.request("memory_search", rpc_params![query, limit]).await
    }
}

// Skill methods
impl RpcClient {
    pub async fn skill_list(&self) -> Result<Vec<String>, JsonRpcSeeError> {
        self.client.request("skill_list", rpc_params![]).await
    }

    pub async fn skill_get(&self, name: String) -> Result<Option<String>, JsonRpcSeeError> {
        self.client.request("skill_get", rpc_params![name]).await
    }

    pub async fn skill_run(&self, name: String, inputs: serde_json::Value) -> Result<serde_json::Value, JsonRpcSeeError> {
        self.client.request("skill_run", rpc_params![name, inputs]).await
    }
}

// A2A methods
impl RpcClient {
    pub async fn a2a_send(&self, message: String, target: String) -> Result<(), JsonRpcSeeError> {
        self.client.request("a2a_send", rpc_params![message, target]).await
    }

    pub async fn a2a_recv(&self, from: String) -> Result<Option<String>, JsonRpcSeeError> {
        self.client.request("a2a_recv", rpc_params![from]).await
    }

    pub async fn a2a_delegate(&self, task_id: String, action: String, params: serde_json::Value) -> Result<(), JsonRpcSeeError> {
        self.client.request("a2a_delegate", rpc_params![task_id, action, params]).await
    }

    pub async fn a2a_sync(&self, timeout: u64) -> Result<Option<String>, JsonRpcSeeError> {
        self.client.request("a2a_sync", rpc_params![timeout]).await
    }

    pub async fn a2a_share_learning(&self, category: String, content: String, confidence: f32) -> Result<(), JsonRpcSeeError> {
        self.client.request("a2a_share_learning", rpc_params![category, content, confidence]).await
    }
}

// Goal methods
impl RpcClient {
    pub async fn goal_set(&self, description: String) -> Result<(), JsonRpcSeeError> {
        self.client.request("goal_set", rpc_params![description]).await
    }

    pub async fn goal_list(&self) -> Result<Vec<String>, JsonRpcSeeError> {
        self.client.request("goal_list", rpc_params![]).await
    }

    pub async fn goal_progress(&self, description: String) -> Result<f32, JsonRpcSeeError> {
        self.client.request("goal_progress", rpc_params![description]).await
    }

    pub async fn goal_evaluate(&self, description: String) -> Result<bool, JsonRpcSeeError> {
        self.client.request("goal_evaluate", rpc_params![description]).await
    }
}

// Heartbeat methods
impl RpcClient {
    pub async fn heartbeat_trigger(&self) -> Result<(), JsonRpcSeeError> {
        self.client.request("heartbeat_trigger", rpc_params![]).await
    }

    pub async fn heartbeat_status(&self) -> Result<bool, JsonRpcSeeError> {
        self.client.request("heartbeat_status", rpc_params![]).await
    }

    pub async fn heartbeat_configure(&self, interval_seconds: u64) -> Result<(), JsonRpcSeeError> {
        self.client.request("heartbeat_configure", rpc_params![interval_seconds]).await
    }
}

// Configuration methods
impl RpcClient {
    pub async fn config_get(&self, key: String) -> Result<Option<String>, JsonRpcSeeError> {
        self.client.request("config_get", rpc_params![key]).await
    }

    pub async fn config_set(&self, key: String, value: String) -> Result<(), JsonRpcSeeError> {
        self.client.request("config_set", rpc_params![key, value]).await
    }

    pub async fn config_list(&self) -> Result<Vec<(String, String)>, JsonRpcSeeError> {
        self.client.request("config_list", rpc_params![]).await
    }
}

// System methods
impl RpcClient {
    pub async fn system_version(&self) -> Result<String, JsonRpcSeeError> {
        self.client.request("system_version", rpc_params![]).await
    }

    pub async fn system_stats(&self) -> Result<String, JsonRpcSeeError> {
        self.client.request("system_stats", rpc_params![]).await
    }
}

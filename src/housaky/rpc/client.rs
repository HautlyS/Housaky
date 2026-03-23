use std::path::PathBuf;
use std::time::Duration;
use tokio::net::UnixStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use serde_json::{Value, json};
use tracing::{debug, error, info, warn};

/// JSON-RPC client for communicating with the Housaky daemon.
pub struct RpcClient {
    socket_path: PathBuf,
    request_id: u64,
}

impl RpcClient {
    /// Create a new RPC client.
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            request_id: 0,
        }
    }

    /// Send a request and wait for the response.
    async fn call_method<T: for<'de> serde::Deserialize<'de>>(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
        // Connect to the socket
        let stream = UnixStream::connect(&self.socket_path).await?;
        let mut stream = BufReader::new(stream);

        // Prepare the request
        self.request_id += 1;
        let request = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": self.request_id,
        });

        // Send the request
        let mut writer = stream.get_mut();
        writer.write_all(request.to_string().as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        // Read the response
        let mut line = String::new();
        stream.read_line(&mut line).await?;
        let response: Value = serde_json::from_str(&line)?;

        // Check for error
        if let Some(error) = response.get("error") {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("RPC error: {}", error),
            )));
        }

        // Extract the result
        let result = response.get("result")
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Missing result in response"))?;
        
        // Deserialize the result
        let result: T = serde_json::from_value(result.clone())?;
        Ok(result)
    }

    // Memory operations
    pub async fn memory_store(&mut self, key: String, value: String) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let params = json!({ "key": key, "value": value });
        self.call_method("memory_store", params).await
    }

    pub async fn memory_recall(&mut self, query: String) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let params = json!({ "query": query });
        self.call_method("memory_recall", params).await
    }

    pub async fn memory_search(&mut self, query: String, limit: usize) -> Result<Vec<Value>, Box<dyn std::error::Error + Send + Sync>> {
        let params = json!({ "query": query, "limit": limit as u64 });
        self.call_method("memory_search", params).await
    }

    pub async fn memory_forget(&mut self, key: String) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let params = json!({ "key": key });
        self.call_method("memory_forget", params).await
    }

    // Skills operations
    pub async fn skill_list(&mut self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let params = json!({});
        self.call_method("skill_list", params).await
    }

    pub async fn skill_get(&mut self, name: String) -> Result<Option<Value>, Box<dyn std::error::Error + Send + Sync>> {
        let params = json!({ "name": name });
        self.call_method("skill_get", params).await
    }

    pub async fn skill_run(&mut self, name: String, inputs: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let params = json!({ "name": name, "inputs": inputs });
        self.call_method("skill_run", params).await
    }

    // A2A operations
    pub async fn a2a_send(&mut self, message: String, target: String) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let params = json!({ "message": message, "target": target });
        self.call_method("a2a_send", params).await
    }

    // Goals operations
    pub async fn goal_set(&mut self, description: String) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let params = json!({ "description": description });
        self.call_method("goal_set", params).await
    }

    pub async fn goal_list(&mut self) -> Result<Vec<Value>, Box<dyn std::error::Error + Send + Sync>> {
        let params = json!({});
        self.call_method("goal_list", params).await
    }

    // Heartbeat operations
    pub async fn heartbeat_trigger(&mut self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let params = json!({});
        self.call_method("heartbeat_trigger", params).await
    }

    // Configuration operations
    pub async fn config_get(&mut self, key: String) -> Result<Option<Value>, Box<dyn std::error::Error + Send + Sync>> {
        let params = json!({ "key": key });
        self.call_method("config_get", params).await
    }

    pub async fn config_set(&mut self, key: String, value: Option<Value>) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let params = json!({ "key": key, "value": value });
        self.call_method("config_set", params).await
    }

    // System operations
    pub async fn system_version(&mut self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let params = json!({});
        self.call_method("system_version", params).await
    }
}
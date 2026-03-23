use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use serde_json::{Value, json};
use crate::housaky::rpc::handler::RpcHandler;

/// JSON-RPC server that exposes Housaky's internal functions over Unix domain socket.
pub struct RpcServer {
    socket_path: PathBuf,
    handler: Arc<dyn RpcHandler + Send + Sync>,
}

impl RpcServer {
    /// Create a new RPC server.
    pub fn new<H: RpcHandler + Send + Sync + 'static>(socket_path: PathBuf, handler: H) -> Self {
        Self {
            socket_path,
            handler: Arc::new(handler),
        }
    }

    /// Start the server and listen for incoming connections.
    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Remove any existing socket file
        let _ = std::fs::remove_file(&self.socket_path);

        // Bind to the socket
        let listener = UnixListener::bind(&self.socket_path)?;
        info!("RPC server listening on {}", self.socket_path.display());

        // Accept connections in a loop
        while let Ok((stream, _)) = listener.accept().await {
            let handler = self.handler.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, handler).await {
                    error!("Error handling connection: {}", e);
                }
            });
        }

        Ok(())
    }

    /// Handle a single connection.
    async fn handle_connection(
        mut stream: UnixStream,
        handler: Arc<dyn RpcHandler + Send + Sync>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

        let mut reader = tokio::io::BufReader::new(&mut stream);
        let mut writer = &mut stream;

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    // Process the JSON-RPC request
                    let response = Self::process_request(&line, &handler).await?;
                    // Write the response as a line
                    writer.write_all(response.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await?;
                }
                Err(e) => {
                    error!("Failed to read from socket: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Process a single JSON-RPC request line.
    async fn process_request(
        line: &str,
        handler: &Arc<dyn RpcHandler + Send + Sync>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Parse the request
        let request: Value = match serde_json::from_str(line) {
            Ok(val) => val,
            Err(e) => {
                // Return parse error
                let error = json!({
                    "code": -32700,
                    "message": "Parse error",
                });
                let response = json!({
                    "jsonrpc": "2.0",
                    "error": error,
                    "id": serde_json::Value::Null,
                });
                return Ok(response.to_string());
            }
        };

        // Extract method, params, and id
        let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
        let id = request.get("id").cloned();

        // Route to the appropriate handler method
        let result = match method {
            "memory_store" => {
                let key = params.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let value = params.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
                handler.memory_store(key, value).await
            }
            "memory_recall" => {
                let query = params.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string();
                handler.memory_recall(query).await
            }
            "memory_search" => {
                let query = params.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
                handler.memory_search(query, limit).await
            }
            "memory_forget" => {
                let key = params.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string();
                handler.memory_forget(key).await
            }
            "skill_list" => handler.skill_list().await,
            "skill_get" => {
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                handler.skill_get(name).await
            }
            "skill_run" => {
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let inputs = params.get("inputs").cloned().unwrap_or_else(|| json!({}));
                handler.skill_run(name, inputs).await
            }
            "a2a_send" => {
                let message = params.get("message").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let target = params.get("target").and_then(|v| v.as_str()).unwrap_or("").to_string();
                handler.a2a_send(message, target).await
            }
            "goal_set" => {
                let description = params.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
                handler.goal_set(description).await
            }
            "goal_list" => handler.goal_list().await,
            "heartbeat_trigger" => handler.heartbeat_trigger().await,
            "config_get" => {
                let key = params.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string();
                handler.config_get(key).await
            }
            "config_set" => {
                let key = params.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let value = params.get("value").cloned();
                handler.config_set(key, value).await
            }
            "system_version" => handler.system_version().await,
            _ => {
                // Method not found
                let error = json!({
                    "code": -32601,
                    "message": format!("Method not found: {}", method),
                });
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    error.to_string(),
                )))
            }
        };

        // Build the response
        let response = match result {
            Ok(result) => json!({
                "jsonrpc": "2.0",
                "result": result,
                "id": id,
            }),
            Err(e) => {
                let error = json!({
                    "code": -32603,
                    "message": e.to_string(),
                });
                json!({
                    "jsonrpc": "2.0",
                    "error": error,
                    "id": id,
                })
            }
        };

        Ok(response.to_string())
    }
}
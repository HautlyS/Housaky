use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: usize,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: usize,
    pub result: Option<serde_json::Value>,
    pub error: Option<McpError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
}

pub struct McpClient {
    server_command: String,
    server_args: Vec<String>,
    server_env: HashMap<String, String>,
    request_id: usize,
}

impl McpClient {
    pub fn new(
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    ) -> Self {
        Self {
            server_command: command,
            server_args: args,
            server_env: env,
            request_id: 0,
        }
    }

    pub async fn call(&mut self, method: &str, params: Option<serde_json::Value>) -> Result<McpResponse> {
        self.request_id += 1;
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: self.request_id,
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)?;
        
        let mut cmd = Command::new(&self.server_command);
        cmd.args(&self.server_args)
            .envs(&self.server_env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped());

        let mut child = cmd.spawn()?;
        
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(request_json.as_bytes()).await?;
        }

        let output = child.wait_with_output().await?;
        let response_str = String::from_utf8_lossy(&output.stdout);
        
        let response: McpResponse = serde_json::from_str(&response_str)?;
        
        Ok(response)
    }

    pub async fn list_tools(&mut self) -> Result<Vec<McpTool>> {
        let response = self.call("tools/list", None).await?;
        
        if let Some(result) = response.result {
            let tools: Vec<McpTool> = serde_json::from_value(result)?;
            Ok(tools)
        } else {
            Ok(vec![])
        }
    }

    pub async fn call_tool(&mut self, name: &str, args: serde_json::Value) -> Result<serde_json::Value> {
        let params = serde_json::json!({ "name": name, "arguments": args });
        let response = self.call("tools/call", Some(params)).await?;
        
        if let Some(result) = response.result {
            Ok(result)
        } else if let Some(error) = response.error {
            anyhow::bail!("MCP error: {} - {}", error.code, error.message)
        } else {
            anyhow::bail!("No result or error in response")
        }
    }

    pub async fn list_resources(&mut self) -> Result<Vec<McpResource>> {
        let response = self.call("resources/list", None).await?;
        
        if let Some(result) = response.result {
            let resources: Vec<McpResource> = serde_json::from_value(result)?;
            Ok(resources)
        } else {
            Ok(vec![])
        }
    }

    pub async fn read_resource(&mut self, uri: &str) -> Result<String> {
        let params = serde_json::json!({ "uri": uri });
        let response = self.call("resources/read", Some(params)).await?;
        
        if let Some(result) = response.result {
            if let Some(contents) = result.get("contents").and_then(|c| c.as_array()) {
                if let Some(content) = contents.first() {
                    if let Some(text) = content.get("text").and_then(|t| t.as_str()) {
                        return Ok(text.to_string());
                    }
                }
            }
            Ok(result.to_string())
        } else if let Some(error) = response.error {
            anyhow::bail!("MCP error: {} - {}", error.code, error.message)
        } else {
            Ok(String::new())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceContent {
    pub uri: String,
    pub mime_type: String,
    pub text: Option<String>,
    pub blob: Option<String>,
}

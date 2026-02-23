//! Cloudflare Workers runtime — executes tool code as serverless workers.
//!
//! Provides:
//! - Serverless execution via Cloudflare Workers
//! - Built-in durable objects for state
//! - KV storage bindings
//! - Edge execution for low latency
//!
//! # Feature gate
//! This module requires Cloudflare API access and appropriate credentials.

use crate::config::CloudflareRuntimeConfig;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct CloudflareRuntime {
    config: CloudflareRuntimeConfig,
    workspace_dir: Option<PathBuf>,
    active_worker_id: Arc<RwLock<Option<String>>>,
}

impl CloudflareRuntime {
    pub fn new(config: CloudflareRuntimeConfig) -> Self {
        Self {
            config,
            workspace_dir: None,
            active_worker_id: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_workspace(config: CloudflareRuntimeConfig, workspace_dir: PathBuf) -> Self {
        Self {
            config,
            workspace_dir: Some(workspace_dir),
            active_worker_id: Arc::new(RwLock::new(None)),
        }
    }

    pub fn validate_config(&self) -> Result<()> {
        if self.config.account_id.is_empty() {
            bail!("runtime.cloudflare.account_id is required");
        }
        if self.config.api_token.is_empty() {
            bail!("runtime.cloudflare.api_token is required");
        }
        if self.config.memory_limit_mb == 0 {
            bail!("runtime.cloudflare.memory_limit_mb must be > 0");
        }
        if self.config.memory_limit_mb > 300 {
            bail!(
                "runtime.cloudflare.memory_limit_mb of {} exceeds the 300 MB limit",
                self.config.memory_limit_mb
            );
        }
        Ok(())
    }

    pub async fn deploy_worker(&self, worker_script: &str) -> Result<String> {
        self.validate_config()?;

        let worker_name = format!(
            "{}-{}",
            self.config.worker_name,
            uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
        );

        let deploy_url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/workers/scripts/{}",
            self.config.account_id, worker_name
        );

        let client = reqwest::Client::new();
        let response = client
            .put(&deploy_url)
            .header("Authorization", format!("Bearer {}", self.config.api_token))
            .header("Content-Type", "application/javascript")
            .body(worker_script.to_string())
            .send()
            .await
            .context("Failed to deploy worker to Cloudflare")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            bail!(
                "Cloudflare worker deployment failed ({}): {}",
                status,
                error_text
            );
        }

        let response_json: serde_json::Value = response.json().await?;
        let worker_id = response_json["result"]["id"]
            .as_str()
            .unwrap_or(&worker_name)
            .to_string();

        *self.active_worker_id.write().await = Some(worker_id.clone());

        Ok(worker_id)
    }

    pub async fn execute_worker(
        &self,
        input: serde_json::Value,
        env: Option<std::collections::HashMap<String, String>>,
    ) -> Result<serde_json::Value> {
        let worker_id = self
            .active_worker_id
            .read()
            .await
            .clone()
            .context("No active worker deployed")?;

        let execute_url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/workers/scripts/{}/dispatch/namespace/default",
            self.config.account_id, worker_id
        );

        let mut request_body = serde_json::json!({
            "input": input,
        });

        if let Some(env_vars) = env {
            request_body["env"] = serde_json::json!(env_vars);
        }

        let client = reqwest::Client::new();
        let response = client
            .post(&execute_url)
            .header("Authorization", format!("Bearer {}", self.config.api_token))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to execute worker")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            bail!("Worker execution failed ({}): {}", status, error_text);
        }

        let response_json: serde_json::Value = response.json().await?;
        Ok(response_json["result"].clone())
    }

    pub async fn delete_worker(&self) -> Result<()> {
        let worker_id = match self.active_worker_id.read().await.clone() {
            Some(id) => id,
            None => return Ok(()),
        };

        let delete_url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/workers/scripts/{}",
            self.config.account_id, worker_id
        );

        let client = reqwest::Client::new();
        let _response = client
            .delete(&delete_url)
            .header("Authorization", format!("Bearer {}", self.config.api_token))
            .send()
            .await
            .context("Failed to delete worker")?;

        *self.active_worker_id.write().await = None;

        Ok(())
    }

    pub fn storage_path(&self) -> PathBuf {
        self.workspace_dir
            .as_ref()
            .map_or_else(|| PathBuf::from(".housaky"), |w| w.join(".housaky"))
    }

    pub async fn get_worker_logs(&self, limit: usize) -> Result<Vec<WorkerLog>> {
        let worker_id = match self.active_worker_id.read().await.clone() {
            Some(id) => id,
            None => bail!("No active worker"),
        };

        let logs_url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/workers/scripts/{}/logs",
            self.config.account_id, worker_id
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&logs_url)
            .header("Authorization", format!("Bearer {}", self.config.api_token))
            .query(&[("limit", limit.to_string())])
            .send()
            .await
            .context("Failed to fetch worker logs")?;

        if !response.status().is_success() {
            return Ok(Vec::new());
        }

        let response_json: serde_json::Value = response.json().await?;
        let logs_array = response_json["result"].as_array().cloned().unwrap_or_default();

        let logs: Vec<WorkerLog> = logs_array
            .iter()
            .map(|log| WorkerLog {
                timestamp: log["timestamp"].as_str().unwrap_or("").to_string(),
                level: log["level"].as_str().unwrap_or("info").to_string(),
                message: log["message"].as_str().unwrap_or("").to_string(),
            })
            .collect();

        Ok(logs)
    }
}

#[derive(Debug, Clone)]
pub struct WorkerLog {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

impl super::RuntimeAdapter for CloudflareRuntime {
    fn name(&self) -> &str {
        "cloudflare"
    }

    fn has_shell_access(&self) -> bool {
        false
    }

    fn has_filesystem_access(&self) -> bool {
        false
    }

    fn storage_path(&self) -> PathBuf {
        self.storage_path()
    }

    fn supports_long_running(&self) -> bool {
        true
    }

    fn memory_budget(&self) -> u64 {
        self.config.memory_limit_mb.saturating_mul(1024 * 1024)
    }

    fn build_shell_command(
        &self,
        _command: &str,
        _workspace_dir: &Path,
    ) -> anyhow::Result<tokio::process::Command> {
        bail!(
            "Cloudflare runtime does not support shell commands. \
             Use worker deployment and execution APIs instead."
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::traits::RuntimeAdapter;

    fn default_config() -> CloudflareRuntimeConfig {
        CloudflareRuntimeConfig::default()
    }

    #[test]
    fn cloudflare_runtime_name() {
        let rt = CloudflareRuntime::new(default_config());
        assert_eq!(rt.name(), "cloudflare");
    }

    #[test]
    fn cloudflare_no_shell_access() {
        let rt = CloudflareRuntime::new(default_config());
        assert!(!rt.has_shell_access());
    }

    #[test]
    fn cloudflare_no_filesystem() {
        let rt = CloudflareRuntime::new(default_config());
        assert!(!rt.has_filesystem_access());
    }

    #[test]
    fn cloudflare_supports_long_running() {
        let rt = CloudflareRuntime::new(default_config());
        assert!(rt.supports_long_running());
    }

    #[test]
    fn cloudflare_memory_budget() {
        let rt = CloudflareRuntime::new(default_config());
        assert_eq!(rt.memory_budget(), 128 * 1024 * 1024);
    }

    #[test]
    fn validate_rejects_empty_account() {
        let mut cfg = default_config();
        cfg.account_id = String::new();
        let rt = CloudflareRuntime::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("account_id"));
    }

    #[test]
    fn validate_rejects_empty_token() {
        let mut cfg = default_config();
        cfg.account_id = "test-account".to_string();
        cfg.api_token = String::new();
        let rt = CloudflareRuntime::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("runtime.cloudflare.api_token is required"));
    }

    #[test]
    fn validate_rejects_zero_memory() {
        let mut cfg = default_config();
        cfg.account_id = "test-account".to_string();
        cfg.api_token = "test-token".to_string();
        cfg.memory_limit_mb = 0;
        let rt = CloudflareRuntime::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("runtime.cloudflare.memory_limit_mb must be > 0"));
    }

    #[test]
    fn validate_rejects_excessive_memory() {
        let mut cfg = default_config();
        cfg.account_id = "test-account".to_string();
        cfg.api_token = "test-token".to_string();
        cfg.memory_limit_mb = 512;
        let rt = CloudflareRuntime::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("runtime.cloudflare.memory_limit_mb of 512 exceeds the 300 MB limit"));
    }

    #[test]
    fn validate_accepts_valid_config() {
        let mut cfg = default_config();
        cfg.account_id = "test-account".to_string();
        cfg.api_token = "test-token".to_string();
        let rt = CloudflareRuntime::new(cfg);
        assert!(rt.validate_config().is_ok());
    }

    #[test]
    fn cloudflare_shell_command_errors() {
        let rt = CloudflareRuntime::new(default_config());
        let result = rt.build_shell_command("echo hello", Path::new("/tmp"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not support shell"));
    }

    #[test]
    fn cloudflare_storage_path_default() {
        let rt = CloudflareRuntime::new(default_config());
        assert!(rt.storage_path().to_string_lossy().contains("housaky"));
    }

    #[test]
    fn cloudflare_storage_path_with_workspace() {
        let rt = CloudflareRuntime::with_workspace(
            default_config(),
            PathBuf::from("/home/user/project"),
        );
        assert_eq!(
            rt.storage_path(),
            PathBuf::from("/home/user/project/.housaky")
        );
    }
}

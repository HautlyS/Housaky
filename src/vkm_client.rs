use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VkmConfig {
    pub service: String,
    pub key: String,
    pub name: String,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub provider: Option<String>,
    pub healthy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VkmApiConfig {
    #[serde(rename = "APIKEY")]
    pub api_key: String,
    #[serde(rename = "BASEURL")]
    pub base_url: Option<String>,
    #[serde(rename = "MODEL")]
    pub model: Option<String>,
    #[serde(rename = "PROVIDER")]
    pub provider: Option<String>,
}

pub struct VkmClient {
    cli_path: String,
}

impl VkmClient {
    pub fn new() -> Self {
        let cli_path = std::env::var("HOUSAKY_VKM_CLI_PATH")
            .ok()
            .filter(|path| !path.trim().is_empty())
            .unwrap_or_else(|| "/home/ubuntu/VKM/vkm/bin/vkm".to_string());

        Self { cli_path }
    }

    pub fn with_path(cli_path: &str) -> Self {
        Self {
            cli_path: cli_path.to_string(),
        }
    }

    pub fn get_active_key(&self) -> Option<VkmConfig> {
        let output = Command::new("node")
            .arg(&self.cli_path)
            .arg("get-active")
            .output()
            .ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            serde_json::from_str(&stdout).ok()
        } else {
            None
        }
    }

    pub fn get_config(&self) -> Option<VkmApiConfig> {
        let output = Command::new("node")
            .arg(&self.cli_path)
            .arg("get-config")
            .output()
            .ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            serde_json::from_str(&stdout).ok()
        } else {
            None
        }
    }

    pub fn refresh(&self) -> bool {
        self.get_active_key().is_some()
    }
}

impl Default for VkmClient {
    fn default() -> Self {
        Self::new()
    }
}

pub fn get_vkm_key() -> Option<String> {
    let client = VkmClient::new();
    client.get_active_key().map(|k| k.key)
}

pub fn get_vkm_config() -> Option<VkmApiConfig> {
    let client = VkmClient::new();
    client.get_config()
}

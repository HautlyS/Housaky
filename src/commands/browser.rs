//! Browser Control Commands
//!
//! Built-in browser automation via Chrome DevTools Protocol (CDP)
//! Inspired by OpenClaw's browser command system.

use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrowserCommands {
    /// Start browser
    Start {
        #[arg(long)] headless: bool,
        #[arg(long, default_value = "default")] profile: String,
    },
    /// Stop browser
    Stop,
    /// Show browser status
    Status,
    /// List open tabs
    Tabs,
    /// Open URL in new tab
    Open { url: String },
    /// Navigate current tab
    Navigate { url: String },
    /// Close tab
    Close { tab: String },
    /// Focus tab
    Focus { tab: String },
    /// Capture screenshot
    Screenshot {
        #[arg(short, long)] output: Option<String>,
        #[arg(long)] full_page: bool,
    },
    /// Capture accessibility snapshot
    Snapshot {
        #[arg(long, default_value = "ai")] format: String,
        #[arg(long, default_value = "200")] limit: usize,
    },
    /// Click element by ref
    Click { ref_id: String, #[arg(long)] double: bool },
    /// Type into element
    Type { ref_id: String, text: String, #[arg(long)] submit: bool },
    /// Press a key
    Press { key: String },
    /// Hover element
    Hover { ref_id: String },
    /// Evaluate JavaScript
    Evaluate { #[arg(long)] function: String },
    /// Wait for condition
    Wait {
        #[arg(long)] kind: String,
        value: String,
        #[arg(long, default_value = "30000")] timeout: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserStatus {
    pub running: bool,
    pub headless: bool,
    pub profile: String,
    pub tabs_count: usize,
    pub active_tab: Option<String>,
    pub version: Option<String>,
}

impl Default for BrowserStatus {
    fn default() -> Self {
        Self { running: false, headless: true, profile: "default".to_string(), tabs_count: 0, active_tab: None, version: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    pub headless: bool,
    pub profile: String,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub timeout_ms: u64,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self { headless: true, profile: "default".to_string(), viewport_width: 1280, viewport_height: 720, timeout_ms: 30000 }
    }
}

//! Lucid Memory Bridge for Housaky
//!
//! Integrates external Lucid Memory (native Rust, 2.7ms retrieval)
//! with Housaky's internal memory systems.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// Lucid Memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LucidConfig {
    /// Path to lucid CLI binary
    pub binary_path: PathBuf,
    /// Project path for context
    pub project_path: Option<PathBuf>,
    /// Token budget for context retrieval
    pub token_budget: usize,
}

impl Default for LucidConfig {
    fn default() -> Self {
        Self {
            binary_path: PathBuf::from("/home/ubuntu/.lucid/bin/lucid"),
            project_path: None,
            token_budget: 200,
        }
    }
}

/// Lucid Memory bridge for semantic search
pub struct LucidBridge {
    config: LucidConfig,
}

/// Retrieved memory from Lucid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LucidMemory {
    pub content: String,
    pub score: f32,
    pub memory_type: Option<String>,
    pub project: Option<String>,
}

impl LucidBridge {
    pub fn new(config: LucidConfig) -> Self {
        Self { config }
    }

    /// Query Lucid for relevant context
    pub fn get_context(&self, query: &str) -> Result<Vec<LucidMemory>> {
        let mut cmd = Command::new(&self.config.binary_path);
        cmd.arg("context").arg(query);

        if let Some(ref path) = self.config.project_path {
            cmd.arg(format!("--project={}", path.display()));
        }

        cmd.arg(format!("--budget={}", self.config.token_budget));

        let output = cmd.output()
            .context("Failed to execute lucid context command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!("Lucid context failed: {}", stderr);
            return Ok(Vec::new());
        }

        // Parse output - lucid returns text with memories
        let stdout = String::from_utf8_lossy(&output.stdout);
        let memories = self.parse_lucid_output(&stdout);

        Ok(memories)
    }

    /// Store a memory in Lucid
    pub fn store(&self, content: &str, memory_type: &str) -> Result<String> {
        let mut cmd = Command::new(&self.config.binary_path);
        cmd.arg("store")
            .arg(content)
            .arg(format!("--type={}", memory_type));

        if let Some(ref path) = self.config.project_path {
            cmd.arg(format!("--project={}", path.display()));
        }

        let output = cmd.output()
            .context("Failed to execute lucid store command")?;

        if output.status.success() {
            // Parse ID from response
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("\"success\":true") {
                // Extract ID from JSON response
                if let Some(id_start) = stdout.find("\"id\":\"") {
                    let start = id_start + 6;
                    if let Some(end) = stdout[start..].find("\"") {
                        return Ok(stdout[start..start+end].to_string());
                    }
                }
            }
        }

        Ok(String::new())
    }

    /// Get statistics from Lucid
    pub fn stats(&self) -> Result<LucidStats> {
        let output = Command::new(&self.config.binary_path)
            .arg("stats")
            .output()
            .context("Failed to get lucid stats")?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse stats from output
        let mut stats = LucidStats::default();

        for line in stdout.lines() {
            if line.starts_with("Memories:") {
                if let Some(n) = line.split(':').nth(1) {
                    stats.total_memories = n.trim().parse().unwrap_or(0);
                }
            }
            if line.contains("Database size") {
                if let Some(s) = line.split(':').nth(1) {
                    stats.database_size = s.trim().to_string();
                }
            }
        }

        Ok(stats)
    }

    /// Parse Lucid CLI output into structured memories
    fn parse_lucid_output(&self, output: &str) -> Vec<LucidMemory> {
        let mut memories = Vec::new();

        for line in output.lines() {
            // Skip empty lines and headers
            if line.is_empty() || line.starts_with('[') || line.starts_with("lucid") {
                continue;
            }

            // Each non-empty line is potentially a memory
            if line.len() > 10 {
                memories.push(LucidMemory {
                    content: line.to_string(),
                    score: 1.0, // Default score
                    memory_type: None,
                    project: None,
                });
            }
        }

        memories
    }
}

/// Statistics from Lucid Memory
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LucidStats {
    pub total_memories: usize,
    pub visual_memories: usize,
    pub projects: usize,
    pub database_size: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lucid_config_default() {
        let config = LucidConfig::default();
        assert!(config.binary_path.to_str().unwrap().contains("lucid"));
    }

    #[test]
    fn test_lucid_memory_creation() {
        let mem = LucidMemory {
            content: "Test memory".to_string(),
            score: 0.9,
            memory_type: Some("learning".to_string()),
            project: None,
        };
        assert_eq!(mem.content, "Test memory");
    }
}

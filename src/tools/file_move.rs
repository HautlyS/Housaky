use super::traits::{Tool, ToolResult};
use crate::security::SecurityPolicy;
use crate::util::expand_path;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Move or rename files and directories within the workspace
pub struct FileMoveTool {
    security: Arc<SecurityPolicy>,
}

impl FileMoveTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }
}

#[async_trait]
impl Tool for FileMoveTool {
    fn name(&self) -> &str {
        "file_move"
    }

    fn description(&self) -> &str {
        "Move or rename files and directories within the workspace. Can be used for renaming (same directory) or moving (different directory)."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "Relative path to the source file or directory"
                },
                "destination": {
                    "type": "string",
                    "description": "Relative path to the destination (new name or directory)"
                },
                "overwrite": {
                    "type": "boolean",
                    "description": "Whether to overwrite if destination exists (default: false)",
                    "default": false
                }
            },
            "required": ["source", "destination"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let source = args
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'source' parameter"))?;

        let destination = args
            .get("destination")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'destination' parameter"))?;

        let overwrite = args
            .get("overwrite")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Check autonomy level
        if !self.security.can_act() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        if self.security.is_rate_limited() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: too many actions in the last hour".into()),
            });
        }

        // Expand home directory (~) in paths
        let expanded_source = expand_path(source);
        let expanded_dest = expand_path(destination);
        let source_str = expanded_source.to_string_lossy();
        let dest_str = expanded_dest.to_string_lossy();

        // Security check: validate paths are within workspace
        if !self.security.is_path_allowed(&source_str) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Source path not allowed by security policy: {source}")),
            });
        }

        if !self.security.is_path_allowed(&dest_str) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Destination path not allowed by security policy: {destination}")),
            });
        }

        // Use expanded paths directly if absolute, otherwise join with workspace
        let full_source = if expanded_source.is_absolute() {
            expanded_source
        } else {
            self.security.workspace_dir.join(source_str.as_ref())
        };
        let full_destination = if expanded_dest.is_absolute() {
            expanded_dest
        } else {
            self.security.workspace_dir.join(dest_str.as_ref())
        };

        // Resolve paths before operating to block symlink escapes
        let resolved_source = match tokio::fs::canonicalize(&full_source).await {
            Ok(p) => p,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to resolve source path: {e}")),
                });
            }
        };

        if !self.security.is_resolved_path_allowed(&resolved_source) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Resolved source path escapes workspace: {}",
                    resolved_source.display()
                )),
            });
        }

        // Check if source exists
        if !resolved_source.exists() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Source does not exist: {source}")),
            });
        }

        // Ensure destination parent directory exists
        if let Some(parent) = full_destination.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Check if destination exists
        if full_destination.exists() && !overwrite {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Destination already exists: {destination}. Use overwrite=true to replace."
                )),
            });
        }

        // Resolve destination parent to check for symlink escapes
        if let Some(parent) = full_destination.parent() {
            let resolved_parent = match tokio::fs::canonicalize(parent).await {
                Ok(p) => p,
                Err(e) => {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Failed to resolve destination path: {e}")),
                    });
                }
            };

            if !self.security.is_resolved_path_allowed(&resolved_parent) {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!(
                        "Resolved destination path escapes workspace: {}",
                        resolved_parent.display()
                    )),
                });
            }
        }

        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: action budget exhausted".into()),
            });
        }

        // Perform the move
        match tokio::fs::rename(&resolved_source, &full_destination).await {
            Ok(()) => {
                let action = if source.parent_path() == destination.parent_path() {
                    "Renamed"
                } else {
                    "Moved"
                };
                Ok(ToolResult {
                    success: true,
                    output: format!("{} '{}' to '{}'", action, source, destination),
                    error: None,
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to move file: {e}")),
            }),
        }
    }
}

trait PathParent {
    fn parent_path(&self) -> &str;
}

impl PathParent for str {
    fn parent_path(&self) -> &str {
        self.rfind('/')
            .or_else(|| self.rfind('\\'))
            .map(|i| &self[..i])
            .unwrap_or(".")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{AutonomyLevel, SecurityPolicy};

    fn test_security(workspace: std::path::PathBuf) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: workspace,
            ..SecurityPolicy::default()
        })
    }

    #[tokio::test]
    async fn file_move_renames_file() {
        let dir = std::env::temp_dir().join("housaky_test_file_move_rename");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("old.txt"), "content").await.unwrap();

        let tool = FileMoveTool::new(test_security(dir.clone()));
        let result = tool
            .execute(json!({"source": "old.txt", "destination": "new.txt"}))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("Renamed"));
        assert!(dir.join("new.txt").exists());
        assert!(!dir.join("old.txt").exists());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_move_moves_to_subdirectory() {
        let dir = std::env::temp_dir().join("housaky_test_file_move_subdir");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("file.txt"), "content").await.unwrap();
        tokio::fs::create_dir(dir.join("subdir")).await.unwrap();

        let tool = FileMoveTool::new(test_security(dir.clone()));
        let result = tool
            .execute(json!({"source": "file.txt", "destination": "subdir/file.txt"}))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("Moved"));
        assert!(dir.join("subdir/file.txt").exists());
        assert!(!dir.join("file.txt").exists());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_move_blocks_readonly() {
        let dir = std::env::temp_dir().join("housaky_test_file_move_readonly");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("file.txt"), "content").await.unwrap();

        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            workspace_dir: dir.clone(),
            ..SecurityPolicy::default()
        });

        let tool = FileMoveTool::new(security);
        let result = tool
            .execute(json!({"source": "file.txt", "destination": "new.txt"}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("read-only"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_move_blocks_nonexistent_source() {
        let dir = std::env::temp_dir().join("housaky_test_file_move_missing");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = FileMoveTool::new(test_security(dir.clone()));
        let result = tool
            .execute(json!({"source": "nonexistent.txt", "destination": "new.txt"}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("does not exist"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}

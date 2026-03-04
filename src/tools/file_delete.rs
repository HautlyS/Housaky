use super::traits::{Tool, ToolResult};
use crate::security::SecurityPolicy;
use crate::util::expand_path;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Delete files or directories within the workspace
pub struct FileDeleteTool {
    security: Arc<SecurityPolicy>,
}

impl FileDeleteTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }
}

#[async_trait]
impl Tool for FileDeleteTool {
    fn name(&self) -> &str {
        "file_delete"
    }

    fn description(&self) -> &str {
        "Delete files or directories within the workspace. For directories, uses recursive deletion. Use with caution - this action cannot be undone."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to the file or directory to delete"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "For directories: whether to delete contents recursively (default: false for safety)",
                    "default": false
                },
                "confirm": {
                    "type": "boolean",
                    "description": "Safety confirmation - must be true to proceed (default: false)",
                    "default": false
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;

        let recursive = args
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let confirm = args
            .get("confirm")
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

        // Require confirmation for deletions
        if !confirm {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Deletion requires confirmation. Set confirm=true to delete: {}",
                    path
                )),
            });
        }

        // Expand home directory (~) in path
        let expanded_path = expand_path(path);
        let path_str = expanded_path.to_string_lossy();

        // Security check: validate path is within workspace
        if !self.security.is_path_allowed(&path_str) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Path not allowed by security policy: {path}")),
            });
        }

        // Use expanded path directly if absolute, otherwise join with workspace
        let full_path = if expanded_path.is_absolute() {
            expanded_path
        } else {
            self.security.workspace_dir.join(path_str.as_ref())
        };

        // Resolve path before deleting to block symlink escapes
        let resolved_path = match tokio::fs::canonicalize(&full_path).await {
            Ok(p) => p,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to resolve path: {e}")),
                });
            }
        };

        if !self.security.is_resolved_path_allowed(&resolved_path) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Resolved path escapes workspace: {}",
                    resolved_path.display()
                )),
            });
        }

        // Check if path exists
        let metadata = match tokio::fs::metadata(&resolved_path).await {
            Ok(m) => m,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Path does not exist: {e}")),
                });
            }
        };

        let is_dir = metadata.is_dir();

        // Require recursive flag for directories
        if is_dir && !recursive {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "'{}' is a directory. Set recursive=true to delete directories.",
                    path
                )),
            });
        }

        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: action budget exhausted".into()),
            });
        }

        // Perform the deletion
        let result = if is_dir {
            tokio::fs::remove_dir_all(&resolved_path).await
        } else {
            tokio::fs::remove_file(&resolved_path).await
        };

        match result {
            Ok(()) => {
                let item_type = if is_dir { "directory" } else { "file" };
                Ok(ToolResult {
                    success: true,
                    output: format!("Deleted {}: {}", item_type, path),
                    error: None,
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to delete: {e}")),
            }),
        }
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
    async fn file_delete_requires_confirmation() {
        let dir = std::env::temp_dir().join("housaky_test_file_delete_confirm");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("file.txt"), "content").await.unwrap();

        let tool = FileDeleteTool::new(test_security(dir.clone()));
        let result = tool
            .execute(json!({"path": "file.txt"}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("confirmation"));
        assert!(dir.join("file.txt").exists()); // File still exists

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_delete_deletes_file() {
        let dir = std::env::temp_dir().join("housaky_test_file_delete");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("file.txt"), "content").await.unwrap();

        let tool = FileDeleteTool::new(test_security(dir.clone()));
        let result = tool
            .execute(json!({"path": "file.txt", "confirm": true}))
            .await
            .unwrap();

        assert!(result.success);
        assert!(!dir.join("file.txt").exists());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_delete_requires_recursive_for_directories() {
        let dir = std::env::temp_dir().join("housaky_test_file_delete_dir");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(dir.join("subdir")).await.unwrap();
        tokio::fs::write(dir.join("subdir/file.txt"), "content").await.unwrap();

        let tool = FileDeleteTool::new(test_security(dir.clone()));
        let result = tool
            .execute(json!({"path": "subdir", "confirm": true}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("recursive"));
        assert!(dir.join("subdir").exists());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_delete_deletes_directory_recursively() {
        let dir = std::env::temp_dir().join("housaky_test_file_delete_recursive");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(dir.join("a/b/c")).await.unwrap();
        tokio::fs::write(dir.join("a/file.txt"), "content").await.unwrap();

        let tool = FileDeleteTool::new(test_security(dir.clone()));
        let result = tool
            .execute(json!({"path": "a", "recursive": true, "confirm": true}))
            .await
            .unwrap();

        assert!(result.success);
        assert!(!dir.join("a").exists());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_delete_blocks_readonly() {
        let dir = std::env::temp_dir().join("housaky_test_file_delete_readonly");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("file.txt"), "content").await.unwrap();

        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            workspace_dir: dir.clone(),
            ..SecurityPolicy::default()
        });

        let tool = FileDeleteTool::new(security);
        let result = tool
            .execute(json!({"path": "file.txt", "confirm": true}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("read-only"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}

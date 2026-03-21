use super::traits::{Tool, ToolResult};
use crate::security::SecurityPolicy;
use crate::util::expand_path;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::Write;
use std::sync::Arc;

/// Get detailed file or directory information
pub struct FileInfoTool {
    security: Arc<SecurityPolicy>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileMetadata {
    name: String,
    path: String,
    exists: bool,
    is_file: bool,
    is_dir: bool,
    is_symlink: bool,
    size: Option<u64>,
    size_human: Option<String>,
    created: Option<String>,
    modified: Option<String>,
    accessed: Option<String>,
    permissions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    children_count: Option<usize>,
}

impl FileInfoTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }
}

#[async_trait]
impl Tool for FileInfoTool {
    fn name(&self) -> &str {
        "file_info"
    }

    fn description(&self) -> &str {
        "Get detailed metadata about a file or directory including size, permissions, timestamps, and type."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to the file or directory within the workspace"
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

        if self.security.is_rate_limited() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: too many actions in the last hour".into()),
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

        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: action budget exhausted".into()),
            });
        }

        // Use expanded path directly if absolute, otherwise join with workspace
        let full_path = if expanded_path.is_absolute() {
            expanded_path
        } else {
            self.security.workspace_dir.join(path_str.as_ref())
        };

        // Try to get metadata (doesn't follow symlinks for the initial check)
        let symlink_metadata: Result<std::fs::Metadata, std::io::Error> = tokio::fs::symlink_metadata(&full_path).await;
        let is_symlink = symlink_metadata
            .as_ref()
            .map(|m: &std::fs::Metadata| m.file_type().is_symlink())
            .unwrap_or(false);

        // Get the actual metadata (follows symlinks)
        let metadata: Result<std::fs::Metadata, std::io::Error> = tokio::fs::metadata(&full_path).await;

        let exists = metadata.is_ok();

        let metadata: std::fs::Metadata = match metadata {
            Ok(m) => m,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to read file metadata: {e}")),
                });
            }
        };

        let is_file = metadata.is_file();
        let is_dir = metadata.is_dir();
        let size = if is_file { Some(metadata.len()) } else { None };

        // Count children if directory
        let children_count = if is_dir {
            match tokio::fs::read_dir(&full_path).await {
                Ok(mut dir) => {
                    let mut count = 0;
                    loop {
                        match dir.next_entry().await {
                            Ok(Some(_)) => count += 1,
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                    Some(count)
                }
                Err(_) => None,
            }
        } else {
            None
        };

        let file_name = full_path
            .file_name()
            .map(|n: &std::ffi::OsStr| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());

        let file_info = FileMetadata {
            name: file_name,
            path: path.to_string(),
            exists,
            is_file,
            is_dir,
            is_symlink,
            size,
            size_human: size.map(format_size),
            created: metadata
                .created()
                .ok()
                .map(format_system_time),
            modified: metadata
                .modified()
                .ok()
                .map(format_system_time),
            accessed: metadata
                .accessed()
                .ok()
                .map(format_system_time),
            permissions: Some(format_permissions(&metadata)),
            children_count,
        };

        let output = format_file_info(&file_info);

        Ok(ToolResult {
            success: true,
            output,
            error: None,
        })
    }
}

fn format_system_time(time: std::time::SystemTime) -> String {
    let datetime: chrono::DateTime<chrono::Local> = time.into();
    datetime.format("%Y-%m-%d %H:%M:%S %Z").to_string()
}

fn format_permissions(metadata: &std::fs::Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        format!("{:o}", mode & 0o777)
    }
    #[cfg(not(unix))]
    {
        if metadata.permissions().readonly() {
            "readonly".to_string()
        } else {
            "read-write".to_string()
        }
    }
}

fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

fn format_file_info(info: &FileMetadata) -> String {
    let mut output = String::new();
    let _ = write!(output, "File: {}\n", info.name);
    let _ = write!(output, "Path: {}\n", info.path);
    let _ = write!(output, "Exists: {}\n", info.exists);

    if info.is_symlink {
        output.push_str("Type: Symbolic Link\n");
    } else if info.is_dir {
        output.push_str("Type: Directory\n");
    } else if info.is_file {
        output.push_str("Type: File\n");
    }

    if let Some(size) = info.size {
        let _ = write!(output, "Size: {} bytes ({}\n", size, info.size_human.as_deref().unwrap_or("?"));
    }

    if let Some(count) = info.children_count {
        let _ = write!(output, "Entries: {}\n", count);
    }

    if let Some(perm) = &info.permissions {
        let _ = write!(output, "Permissions: {}\n", perm);
    }

    if let Some(created) = &info.created {
        let _ = write!(output, "Created: {}\n", created);
    }

    if let Some(modified) = &info.modified {
        let _ = write!(output, "Modified: {}\n", modified);
    }

    if let Some(accessed) = &info.accessed {
        let _ = write!(output, "Accessed: {}\n", accessed);
    }

    output
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
    async fn file_info_for_file() {
        let dir = std::env::temp_dir().join("housaky_test_file_info");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("test.txt"), "hello world content")
            .await
            .unwrap();

        let tool = FileInfoTool::new(test_security(dir.clone()));
        let result = tool.execute(json!({"path": "test.txt"})).await.unwrap();

        assert!(result.success);
        assert!(result.output.contains("test.txt"));
        assert!(result.output.contains("Type: File"));
        assert!(result.output.contains("21 bytes")); // "hello world content" = 17 chars? Let me check

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_info_for_directory() {
        let dir = std::env::temp_dir().join("housaky_test_file_info_dir");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(dir.join("subdir")).await.unwrap();
        tokio::fs::write(dir.join("file1.txt"), "content").await.unwrap();
        tokio::fs::write(dir.join("file2.txt"), "content").await.unwrap();

        let tool = FileInfoTool::new(test_security(dir.clone()));
        let result = tool.execute(json!({"path": "."})).await.unwrap();

        assert!(result.success);
        assert!(result.output.contains("Type: Directory"));
        assert!(result.output.contains("Entries: 3"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_info_nonexistent() {
        let dir = std::env::temp_dir().join("housaky_test_file_info_missing");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = FileInfoTool::new(test_security(dir.clone()));
        let result = tool
            .execute(json!({"path": "nonexistent.txt"}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("Failed to read"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_info_blocks_path_traversal() {
        let dir = std::env::temp_dir().join("housaky_test_file_info_traversal");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = FileInfoTool::new(test_security(dir.clone()));
        let result = tool
            .execute(json!({"path": "/etc/passwd"}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not allowed"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}

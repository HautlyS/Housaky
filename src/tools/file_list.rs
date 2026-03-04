use super::traits::{Tool, ToolResult};
use crate::security::SecurityPolicy;
use crate::util::expand_path;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// List directory contents with detailed file information
pub struct FileListTool {
    security: Arc<SecurityPolicy>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileEntry {
    name: String,
    path: String,
    is_file: bool,
    is_dir: bool,
    size: Option<u64>,
    modified: Option<String>,
}

impl FileListTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }
}

#[async_trait]
impl Tool for FileListTool {
    fn name(&self) -> &str {
        "file_list"
    }

    fn description(&self) -> &str {
        "List files and directories with metadata (name, type, size, modified date). Use this to explore the workspace structure."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to the directory within the workspace (default: current directory)",
                    "default": "."
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Whether to list recursively (default: false)",
                    "default": false
                },
                "show_hidden": {
                    "type": "boolean",
                    "description": "Whether to show hidden files starting with . (default: false)",
                    "default": false
                }
            },
            "required": []
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let recursive = args
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let show_hidden = args
            .get("show_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

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

        // Resolve path before reading to block symlink escapes.
        let resolved_path = match tokio::fs::canonicalize(&full_path).await {
            Ok(p) => p,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to resolve directory path: {e}")),
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

        // Check if path is a directory
        let metadata = match tokio::fs::metadata(&resolved_path).await {
            Ok(m) => m,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to read directory metadata: {e}")),
                });
            }
        };

        if !metadata.is_dir() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Path is not a directory: {path}")),
            });
        }

        // List directory contents
        let entries = match list_directory(&resolved_path, path, recursive, show_hidden).await {
            Ok(e) => e,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to list directory: {e}")),
                });
            }
        };

        let output = format_directory_listing(&entries, path);

        Ok(ToolResult {
            success: true,
            output,
            error: None,
        })
    }
}

async fn list_directory(
    dir_path: &std::path::Path,
    relative_path: &str,
    recursive: bool,
    show_hidden: bool,
) -> anyhow::Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    let mut read_dir = tokio::fs::read_dir(dir_path).await?;

    while let Some(entry) = read_dir.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files unless show_hidden is true
        if !show_hidden && name.starts_with('.') {
            continue;
        }

        let metadata = entry.metadata().await?;
        let is_file = metadata.is_file();
        let is_dir = metadata.is_dir();
        let size = if is_file { Some(metadata.len()) } else { None };
        let modified = metadata
            .modified()
            .ok()
            .map(|t| {
                let datetime: chrono::DateTime<chrono::Local> = t.into();
                datetime.format("%Y-%m-%d %H:%M:%S").to_string()
            });

        let entry_path = if relative_path == "." {
            name.clone()
        } else {
            format!("{}/{}", relative_path.trim_end_matches('/'), name)
        };

        entries.push(FileEntry {
            name,
            path: entry_path.clone(),
            is_file,
            is_dir,
            size,
            modified,
        });

        // Recurse into subdirectories if requested
        if recursive && is_dir {
            let sub_entries = Box::pin(list_directory(
                &entry.path(),
                &entry_path,
                recursive,
                show_hidden,
            ))
            .await?;
            entries.extend(sub_entries);
        }
    }

    // Sort: directories first, then files, both alphabetically
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(entries)
}

fn format_directory_listing(entries: &[FileEntry], path: &str) -> String {
    let mut output = String::new();
    output.push_str(&format!("Directory: {}\n", if path == "." { "." } else { path }));
    output.push_str(&format!("Total entries: {}\n\n", entries.len()));

    if entries.is_empty() {
        output.push_str("(empty directory)\n");
        return output;
    }

    // Header
    output.push_str(&format!("{:<30} {:<10} {:>12} {:>20}\n", "Name", "Type", "Size", "Modified"));
    output.push_str(&"-".repeat(80));
    output.push('\n');

    for entry in entries {
        let type_str = if entry.is_dir { "<DIR>" } else { "<FILE>" };
        let size_str = entry
            .size
            .map(|s| format_size(s))
            .unwrap_or_else(|| "-".to_string());
        let mod_str = entry.modified.as_deref().unwrap_or("-");

        // Truncate long names
        let name = if entry.name.len() > 28 {
            format!("{}..", &entry.name[..26])
        } else {
            entry.name.clone()
        };

        output.push_str(&format!(
            "{:<30} {:<10} {:>12} {:>20}\n",
            name, type_str, size_str, mod_str
        ));
    }

    output
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
        format!("{:.1} {}", size, UNITS[unit_index])
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
    async fn file_list_empty_directory() {
        let dir = std::env::temp_dir().join("housaky_test_file_list_empty");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = FileListTool::new(test_security(dir.clone()));
        let result = tool.execute(json!({})).await.unwrap();

        assert!(result.success);
        assert!(result.output.contains("empty directory"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_list_with_files() {
        let dir = std::env::temp_dir().join("housaky_test_file_list");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("file1.txt"), "content1").await.unwrap();
        tokio::fs::write(dir.join("file2.txt"), "content2 longer").await.unwrap();
        tokio::fs::create_dir(dir.join("subdir")).await.unwrap();

        let tool = FileListTool::new(test_security(dir.clone()));
        let result = tool.execute(json!({})).await.unwrap();

        assert!(result.success);
        assert!(result.output.contains("file1.txt"));
        assert!(result.output.contains("file2.txt"));
        assert!(result.output.contains("subdir"));
        assert!(result.output.contains("<DIR>"));
        assert!(result.output.contains("<FILE>"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_list_recursive() {
        let dir = std::env::temp_dir().join("housaky_test_file_list_recursive");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(dir.join("a/b")).await.unwrap();
        tokio::fs::write(dir.join("a/file.txt"), "content").await.unwrap();
        tokio::fs::write(dir.join("a/b/deep.txt"), "deep").await.unwrap();

        let tool = FileListTool::new(test_security(dir.clone()));
        let result = tool.execute(json!({"recursive": true})).await.unwrap();

        assert!(result.success);
        assert!(result.output.contains("a/file.txt"));
        assert!(result.output.contains("a/b/deep.txt"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_list_blocks_path_traversal() {
        let dir = std::env::temp_dir().join("housaky_test_file_list_traversal");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = FileListTool::new(test_security(dir.clone()));
        let result = tool
            .execute(json!({"path": "../../../etc"}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not allowed"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}

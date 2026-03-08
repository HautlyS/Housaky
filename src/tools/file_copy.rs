use super::traits::{Tool, ToolResult};
use crate::security::SecurityPolicy;
use crate::util::expand_path;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct FileCopyTool {
    security: Arc<SecurityPolicy>,
}

impl FileCopyTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }
}

#[async_trait]
impl Tool for FileCopyTool {
    fn name(&self) -> &str {
        "file_copy"
    }

    fn description(&self) -> &str {
        "Copy files or directories within the workspace. Supports recursive directory copying and progress tracking."
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
                    "description": "Relative path to the destination"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Copy directories recursively (default: true)",
                    "default": true
                },
                "overwrite": {
                    "type": "boolean",
                    "description": "Overwrite existing files (default: false)",
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

        let recursive = args.get("recursive").and_then(|v| v.as_bool()).unwrap_or(true);
        let overwrite = args.get("overwrite").and_then(|v| v.as_bool()).unwrap_or(false);

        if !self.security.can_act() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        let source_path = expand_path(source);
        let dest_path = expand_path(destination);

        if !self.security.is_path_allowed(&source_path) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Source path '{}' is outside allowed workspace", source)),
            });
        }

        if !self.security.is_path_allowed(&dest_path) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Destination path '{}' is outside allowed workspace", destination)),
            });
        }

        if !source_path.exists() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Source '{}' does not exist", source)),
            });
        }

        if dest_path.exists() && !overwrite {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Destination '{}' already exists (use overwrite=true)", destination)),
            });
        }

        let mut files_copied = 0u64;
        let mut bytes_copied = 0u64;

        if source_path.is_file() {
            if let Some(parent) = dest_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::copy(&source_path, &dest_path).await?;
            files_copied = 1;
            if let Ok(metadata) = tokio::fs::metadata(&source_path).await {
                bytes_copied = metadata.len();
            }
        } else if source_path.is_dir() {
            if !recursive {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Source is a directory but recursive=false".into()),
                });
            }
            let (files, bytes) = self.copy_directory(&source_path, &dest_path, overwrite).await?;
            files_copied = files;
            bytes_copied = bytes;
        }

        Ok(ToolResult {
            success: true,
            output: format!(
                "Copied {} file(s) ({} bytes) from '{}' to '{}'",
                files_copied,
                bytes_copied,
                source,
                destination
            ),
            error: None,
        })
    }
}

impl FileCopyTool {
    async fn copy_directory(
        &self,
        source: &std::path::Path,
        dest: &std::path::Path,
        overwrite: bool,
    ) -> anyhow::Result<(u64, u64)> {
        let mut files_copied = 0u64;
        let mut bytes_copied = 0u64;

        tokio::fs::create_dir_all(dest).await?;

        let mut entries = tokio::fs::read_dir(source).await?;
        while let Some(entry) = entries.next_entry().await? {
            let src_path = entry.path();
            let dest_path = dest.join(entry.file_name());

            if src_path.is_dir() {
                let (f, b) = self.copy_directory(&src_path, &dest_path, overwrite).await?;
                files_copied += f;
                bytes_copied += b;
            } else {
                if dest_path.exists() && !overwrite {
                    continue;
                }
                tokio::fs::copy(&src_path, &dest_path).await?;
                files_copied += 1;
                if let Ok(metadata) = entry.metadata().await {
                    bytes_copied += metadata.len();
                }
            }
        }

        Ok((files_copied, bytes_copied))
    }
}

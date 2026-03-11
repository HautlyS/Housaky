use super::traits::{Tool, ToolResult};
use crate::housaky::project_context::ProjectScanner;
use async_trait::async_trait;
use serde_json::json;
use std::path::PathBuf;

pub struct ProjectContextTool {
    project_root: PathBuf,
}

impl ProjectContextTool {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    pub fn for_housaky() -> Self {
        Self {
            project_root: PathBuf::from("/home/ubuntu/Housaky"),
        }
    }
}

#[async_trait]
impl Tool for ProjectContextTool {
    fn name(&self) -> &str {
        "project_context"
    }

    fn description(&self) -> &str {
        "Get information about the Housaky project structure, source files, and configuration"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Action to perform: 'summary', 'scan', 'find_file'",
                    "enum": ["summary", "scan", "find_file"]
                },
                "query": {
                    "type": "string",
                    "description": "File name or pattern to search for (for find_file action)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("summary");

        let scanner = ProjectScanner::new(self.project_root.clone());

        match action {
            "summary" => {
                match scanner.get_project_summary().await {
                    Ok(summary) => Ok(ToolResult {
                        success: true,
                        output: summary.to_memory_format(),
                        error: None,
                    }),
                    Err(e) => Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Failed to get project summary: {}", e)),
                    }),
                }
            }
            "scan" => {
                match scanner.scan().await {
                    Ok(files) => {
                        let file_list: Vec<String> = files
                            .iter()
                            .map(|f| format!("{} ({} bytes) - {}", 
                                f.relative_path, 
                                f.size_bytes,
                                f.description))
                            .collect();
                        Ok(ToolResult {
                            success: true,
                            output: file_list.join("\n"),
                            error: None,
                        })
                    }
                    Err(e) => Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Failed to scan project: {}", e)),
                    }),
                }
            }
            "find_file" => {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if query.is_empty() {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Query parameter required for find_file action".to_string()),
                    });
                }

                match scanner.scan().await {
                    Ok(files) => {
                        let matches: Vec<String> = files
                            .iter()
                            .filter(|f| f.relative_path.to_lowercase().contains(&query.to_lowercase()))
                            .map(|f| f.relative_path.clone())
                            .collect();

                        if matches.is_empty() {
                            Ok(ToolResult {
                                success: true,
                                output: format!("No files found matching '{}'", query),
                                error: None,
                            })
                        } else {
                            Ok(ToolResult {
                                success: true,
                                output: matches.join("\n"),
                                error: None,
                            })
                        }
                    }
                    Err(e) => Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Failed to search files: {}", e)),
                    }),
                }
            }
            _ => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown action: {}", action)),
            }),
        }
    }
}

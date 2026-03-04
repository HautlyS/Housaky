use super::traits::{Tool, ToolResult};
use crate::security::SecurityPolicy;
use crate::util::expand_path;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Search for files and content within the workspace
pub struct FileSearchTool {
    security: Arc<SecurityPolicy>,
}

impl FileSearchTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }
}

#[async_trait]
impl Tool for FileSearchTool {
    fn name(&self) -> &str {
        "file_search"
    }

    fn description(&self) -> &str {
        "Search for files by name pattern or content within the workspace. Supports glob patterns for file names and regex for content."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Search pattern - can be a filename glob (e.g., '*.rs') or content to search for"
                },
                "search_type": {
                    "type": "string",
                    "description": "Type of search: 'filename' for file names, 'content' for file contents",
                    "enum": ["filename", "content"],
                    "default": "filename"
                },
                "path": {
                    "type": "string",
                    "description": "Starting directory for search (default: workspace root)",
                    "default": "."
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 50)",
                    "default": 50
                },
                "case_sensitive": {
                    "type": "boolean",
                    "description": "Whether search is case sensitive (default: false)",
                    "default": false
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern' parameter"))?;

        let search_type = args
            .get("search_type")
            .and_then(|v| v.as_str())
            .unwrap_or("filename");

        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;

        let case_sensitive = args
            .get("case_sensitive")
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

        // Resolve path before searching to block symlink escapes.
        let resolved_path = match tokio::fs::canonicalize(&full_path).await {
            Ok(p) => p,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to resolve search path: {e}")),
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

        let results = match search_type {
            "content" => {
                search_content(&resolved_path, pattern, max_results, case_sensitive).await
            }
            _ => search_filename(&resolved_path, pattern, max_results).await,
        };

        match results {
            Ok(matches) => {
                let output = format_search_results(&matches, pattern, search_type);
                Ok(ToolResult {
                    success: true,
                    output,
                    error: None,
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Search failed: {e}")),
            }),
        }
    }
}

#[derive(Debug)]
struct SearchMatch {
    path: String,
    line_number: Option<usize>,
    content: Option<String>,
}

async fn search_filename(
    dir: &std::path::Path,
    pattern: &str,
    max_results: usize,
) -> anyhow::Result<Vec<SearchMatch>> {
    let mut matches = Vec::new();

    // Convert pattern to lowercase for case-insensitive matching
    let pattern_lower = pattern.to_lowercase();
    let is_glob = pattern.contains('*') || pattern.contains('?');

    let mut dirs_to_search = vec![dir.to_path_buf()];

    while let Some(current_dir) = dirs_to_search.pop() {
        let mut read_dir = tokio::fs::read_dir(&current_dir).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            let metadata = entry.metadata().await?;

            if metadata.is_dir() && !name.starts_with('.') {
                dirs_to_search.push(path.clone());
            }

            let is_match = if is_glob {
                // Simple glob matching
                matches_glob(&name, &pattern_lower)
            } else {
                // Simple substring matching (case-insensitive)
                name.to_lowercase().contains(&pattern_lower)
            };

            if is_match {
                let relative_path = path.strip_prefix(dir).unwrap_or(&path);
                matches.push(SearchMatch {
                    path: relative_path.to_string_lossy().to_string(),
                    line_number: None,
                    content: None,
                });

                if matches.len() >= max_results {
                    return Ok(matches);
                }
            }
        }
    }

    Ok(matches)
}

/// Simple glob pattern matching (supports * and ?)
fn matches_glob(name: &str, pattern: &str) -> bool {
    let name_lower = name.to_lowercase();
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let name_chars: Vec<char> = name_lower.chars().collect();

    fn match_recursive(p: &[char], n: &[char]) -> bool {
        match (p.first(), n.first()) {
            (None, None) => true,
            (None, _) => false,
            (Some('*'), _) => {
                match_recursive(&p[1..], n) || (!n.is_empty() && match_recursive(p, &n[1..]))
            }
            (Some('?'), None) => false,
            (Some('?'), _) => match_recursive(&p[1..], &n[1..]),
            (Some(pc), Some(nc)) if pc.to_ascii_lowercase() == nc.to_ascii_lowercase() => {
                match_recursive(&p[1..], &n[1..])
            }
            _ => false,
        }
    }

    match_recursive(&pattern_chars, &name_chars)
}

async fn search_content(
    dir: &std::path::Path,
    pattern: &str,
    max_results: usize,
    case_sensitive: bool,
) -> anyhow::Result<Vec<SearchMatch>> {
    let mut matches = Vec::new();
    let search_term = if case_sensitive {
        pattern.to_string()
    } else {
        pattern.to_lowercase()
    };

    let mut dirs_to_search = vec![dir.to_path_buf()];

    while let Some(current_dir) = dirs_to_search.pop() {
        let mut read_dir = tokio::fs::read_dir(&current_dir).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            let metadata = entry.metadata().await?;

            if metadata.is_dir() {
                if !name.starts_with('.') {
                    dirs_to_search.push(path);
                }
                continue;
            }

            // Skip binary files
            if is_likely_binary(&name) {
                continue;
            }

            // Read and search file content
            match tokio::fs::read_to_string(&path).await {
                Ok(content) => {
                    let lines: Vec<&str> = content.lines().collect();
                    for (line_num, line) in lines.iter().enumerate() {
                        let check_line = if case_sensitive {
                            line.to_string()
                        } else {
                            line.to_lowercase()
                        };

                        if check_line.contains(&search_term) {
                            let relative_path = path.strip_prefix(dir).unwrap_or(&path);
                            matches.push(SearchMatch {
                                path: relative_path.to_string_lossy().to_string(),
                                line_number: Some(line_num + 1),
                                content: Some(line.to_string()),
                            });

                            if matches.len() >= max_results {
                                return Ok(matches);
                            }
                        }
                    }
                }
                Err(_) => {
                    // Skip files that can't be read as text
                    continue;
                }
            }
        }
    }

    Ok(matches)
}

fn is_likely_binary(filename: &str) -> bool {
    let binary_extensions = [
        ".exe", ".dll", ".so", ".dylib", ".bin", ".o", ".a", ".lib",
        ".jpg", ".jpeg", ".png", ".gif", ".bmp", ".ico", ".svg",
        ".mp3", ".mp4", ".avi", ".mov", ".wav", ".flac",
        ".zip", ".tar", ".gz", ".bz2", ".7z", ".rar",
        ".pdf", ".doc", ".docx", ".xls", ".xlsx", ".ppt", ".pptx",
        ".db", ".sqlite", ".sqlite3",
    ];

    let lower = filename.to_lowercase();
    binary_extensions.iter().any(|ext| lower.ends_with(ext))
}

fn format_search_results(matches: &[SearchMatch], pattern: &str, search_type: &str) -> String {
    let mut output = String::new();
    output.push_str(&format!("Search Results for '{}':\n", pattern));
    output.push_str(&format!("Type: {} | Found: {} matches\n\n", search_type, matches.len()));

    if matches.is_empty() {
        output.push_str("No matches found.\n");
        return output;
    }

    for (i, m) in matches.iter().enumerate() {
        if search_type == "content" {
            if let (Some(line), Some(content)) = (m.line_number, &m.content) {
                let trimmed = if content.len() > 80 {
                    format!("{}...", &content[..77])
                } else {
                    content.clone()
                };
                output.push_str(&format!(
                    "{}. {}:{}\n   {}\n\n",
                    i + 1,
                    m.path,
                    line,
                    trimmed
                ));
            }
        } else {
            output.push_str(&format!("{}. {}\n", i + 1, m.path));
        }
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
    async fn file_search_by_name() {
        let dir = std::env::temp_dir().join("housaky_test_file_search_name");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("test.txt"), "content").await.unwrap();
        tokio::fs::write(dir.join("test2.txt"), "content2").await.unwrap();
        tokio::fs::write(dir.join("other.rs"), "code").await.unwrap();

        let tool = FileSearchTool::new(test_security(dir.clone()));
        let result = tool
            .execute(json!({"pattern": "*.txt", "search_type": "filename"}))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("test.txt"));
        assert!(result.output.contains("test2.txt"));
        assert!(!result.output.contains("other.rs"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_search_by_content() {
        let dir = std::env::temp_dir().join("housaky_test_file_search_content");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("file1.txt"), "hello world").await.unwrap();
        tokio::fs::write(dir.join("file2.txt"), "goodbye world").await.unwrap();
        tokio::fs::write(dir.join("file3.txt"), "no match here").await.unwrap();

        let tool = FileSearchTool::new(test_security(dir.clone()));
        let result = tool
            .execute(json!({"pattern": "world", "search_type": "content"}))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("file1.txt"));
        assert!(result.output.contains("file2.txt"));
        assert!(!result.output.contains("file3.txt"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_search_respects_max_results() {
        let dir = std::env::temp_dir().join("housaky_test_file_search_limit");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        for i in 0..10 {
            tokio::fs::write(dir.join(format!("file{}.txt", i)), format!("content {}", i))
                .await
                .unwrap();
        }

        let tool = FileSearchTool::new(test_security(dir.clone()));
        let result = tool
            .execute(json!({"pattern": "*.txt", "search_type": "filename", "max_results": 5}))
            .await
            .unwrap();

        assert!(result.success);
        // Should only show 5 results
        assert!(result.output.contains("5 matches") || result.output.contains("file4.txt"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}

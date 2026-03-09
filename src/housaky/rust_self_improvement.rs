// ☸️ RUST SELF-IMPROVEMENT ENGINE
// Analyzes and improves Housaky Rust codebase using tree-sitter + rust-analyzer + subagents

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use regex::Regex;
use crate::housaky::code_parsing::tree_sitter::RustCodeAnalyzer;

// ============================================================================
// CODE ANALYSIS STRUCTURES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIssue {
    pub file: String,
    pub line: u32,
    pub severity: String, // "error", "warning", "info", "style"
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementOpportunity {
    pub file: String,
    pub description: String,
    pub priority: f64,
    pub effort: String,
    pub category: String,
    pub code_snippet: Option<String>,
    pub function: Option<String>,
    pub suggestion: Option<String>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysis {
    pub path: String,
    pub lines: u32,
    pub functions: u32,
    pub structs: u32,
    pub enums: u32,
    pub traits: u32,
    pub todos: Vec<TodoItem>,
    pub warnings: Vec<String>,
    pub complexity_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub line: u32,
    pub text: String,
    pub priority: Option<String>,
}

// ============================================================================
// RUST ANALYZER INTEGRATION
// ============================================================================

pub struct RustAnalyzer {
    project_root: PathBuf,
}

impl RustAnalyzer {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    /// Run cargo check and parse diagnostics
    pub async fn check(&self) -> Result<Vec<CodeIssue>> {
        let output = Command::new("cargo")
            .args(["check", "--message-format=json"])
            .current_dir(&self.project_root)
            .output()
            .context("Failed to run cargo check")?;

        let mut issues = Vec::new();
        
        // Parse JSON output
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(reason) = json.get("reason").and_then(|r| r.as_str()) {
                    if reason == "compiler-message" {
                        if let Some(message) = json.get("message") {
                            if let Some(spans) = message.get("spans").and_then(|s| s.as_array()) {
                                for span in spans {
                                    let issue = CodeIssue {
                                        file: span.get("file_name")
                                            .and_then(|f| f.as_str())
                                            .unwrap_or("unknown")
                                            .to_string(),
                                        line: span.get("line_start")
                                            .and_then(|l| l.as_u64())
                                            .unwrap_or(0) as u32,
                                        severity: message.get("level")
                                            .and_then(|l| l.as_str())
                                            .unwrap_or("info")
                                            .to_string(),
                                        message: message.get("message")
                                            .and_then(|m| m.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        suggestion: message.get("suggested_replacement")
                                            .and_then(|s| s.as_str())
                                            .map(|s| s.to_string()),
                                    };
                                    issues.push(issue);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(issues)
    }

    /// Run clippy for additional linting
    pub async fn clippy(&self) -> Result<Vec<CodeIssue>> {
        let output = Command::new("cargo")
            .args(["clippy", "--message-format=json", "--", "-W", "clippy::all"])
            .current_dir(&self.project_root)
            .output()
            .context("Failed to run clippy")?;

        // Similar parsing to check()
        let mut issues = Vec::new();
        
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if line.contains("\"reason\":\"compiler-message\"") {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                    if let Some(message) = json.get("message") {
                        let issue = CodeIssue {
                            file: message.get("spans")
                                .and_then(|s| s.as_array())
                                .and_then(|a| a.first())
                                .and_then(|s| s.get("file_name"))
                                .and_then(|f| f.as_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            line: message.get("spans")
                                .and_then(|s| s.as_array())
                                .and_then(|a| a.first())
                                .and_then(|s| s.get("line_start"))
                                .and_then(|l| l.as_u64())
                                .unwrap_or(0) as u32,
                            severity: message.get("level")
                                .and_then(|l| l.as_str())
                                .unwrap_or("info")
                                .to_string(),
                            message: message.get("message")
                                .and_then(|m| m.as_str())
                                .unwrap_or("")
                                .to_string(),
                            suggestion: None,
                        };
                        issues.push(issue);
                    }
                }
            }
        }

        Ok(issues)
    }

    /// Get metrics for a file
    pub async fn analyze_file(&self, path: &Path) -> Result<FileAnalysis> {
        let content = fs::read_to_string(path).await?;
        let lines = content.lines().count() as u32;
        
        // Count Rust constructs using regex
        let fn_re = Regex::new(r"^\s*(pub\s+)?(async\s+)?fn\s+\w+")?;
        let struct_re = Regex::new(r"^\s*(pub\s+)?struct\s+\w+")?;
        let enum_re = Regex::new(r"^\s*(pub\s+)?enum\s+\w+")?;
        let trait_re = Regex::new(r"^\s*(pub\s+)?trait\s+\w+")?;
        let todo_re = Regex::new(r"//\s*(TODO|FIXME|XXX|HACK)[:\s]+(.+)")?;
        
        let mut functions = 0u32;
        let mut structs = 0u32;
        let mut enums = 0u32;
        let mut traits = 0u32;
        let mut todos = Vec::new();
        
        for (i, line) in content.lines().enumerate() {
            if fn_re.is_match(line) { functions += 1; }
            if struct_re.is_match(line) { structs += 1; }
            if enum_re.is_match(line) { enums += 1; }
            if trait_re.is_match(line) { traits += 1; }
            
            if let Some(caps) = todo_re.captures(line) {
                todos.push(TodoItem {
                    line: (i + 1) as u32,
                    text: caps.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default(),
                    priority: caps.get(1).map(|m| m.as_str().to_string()),
                });
            }
        }
        
        // Simple complexity score based on nesting and line count
        let complexity_score = (f64::from(lines) / 100.0).min(1.0) * 
                               (1.0 + f64::from(functions) * 0.01);
        
        Ok(FileAnalysis {
            path: path.to_string_lossy().to_string(),
            lines,
            functions,
            structs,
            enums,
            traits,
            todos,
            warnings: vec![],
            complexity_score,
        })
    }
}

// ============================================================================
// TREE-SITTER INTEGRATION
// ============================================================================

pub struct TreeSitterAnalyzer {
    analyzer: RustCodeAnalyzer,
}

impl TreeSitterAnalyzer {
    pub fn new() -> Self {
        Self {
            analyzer: RustCodeAnalyzer::new(),
        }
    }

    pub async fn find_long_functions(&mut self, content: &str) -> Vec<(String, u32, u32)> {
        let mut result = Vec::new();

        if let Ok(analysis) = self.analyzer.analyze(content) {
            for func in &analysis.functions {
                let line_count = func.line_end.saturating_sub(func.line_start) as u32;
                if line_count > 50 {
                    result.push((func.name.clone(), func.line_start as u32, line_count));
                }
            }
        }

        result
    }

    pub fn find_deep_nesting(&self, content: &str) -> Vec<(u32, u32)> {
        let mut result = Vec::new();
        let mut current_depth = 0u32;
        let mut max_depth = 0u32;
        let mut start_line = 0u32;

        for (i, line) in content.lines().enumerate() {
            let opens = line.matches('{').count() as u32;
            let closes = line.matches('}').count() as u32;

            if opens > closes {
                if current_depth == 0 {
                    start_line = i as u32 + 1;
                }
                current_depth += opens - closes;
                max_depth = max_depth.max(current_depth);
            } else if closes > opens {
                if current_depth >= 4 && current_depth > max_depth - 2 {
                    result.push((start_line, current_depth));
                }
                current_depth = current_depth.saturating_sub(closes - opens);
            }
        }

        result
    }

    pub fn analyze(&mut self, content: &str) -> Option<crate::housaky::code_parsing::tree_sitter::CodeAnalysisResult> {
        self.analyzer.analyze(content).ok()
    }

    pub fn extract_function_complexity(&mut self, content: &str) -> Vec<(String, u32)> {
        if let Ok(analysis) = self.analyzer.analyze(content) {
            return analysis.functions.iter()
                .map(|f| (f.name.clone(), f.complexity))
                .collect();
        }
        Vec::new()
    }
}

// ============================================================================
// SELF-IMPROVEMENT ENGINE
// ============================================================================

pub struct SelfImprovementEngine {
    pub project_root: PathBuf,
    pub rust_analyzer: RustAnalyzer,
    pub tree_sitter: TreeSitterAnalyzer,
    improvements: Vec<ImprovementOpportunity>,
}

impl SelfImprovementEngine {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root: project_root.clone(),
            rust_analyzer: RustAnalyzer::new(project_root.clone()),
            tree_sitter: TreeSitterAnalyzer::new(),
            improvements: Vec::new(),
        }
    }

    /// Scan entire codebase for improvement opportunities
    pub async fn scan(&mut self) -> Result<Vec<ImprovementOpportunity>> {
        let mut opportunities = Vec::new();
        
        // 1. Check for compiler warnings
        let issues = self.rust_analyzer.check().await?;
        for issue in issues {
            if issue.severity == "warning" {
                opportunities.push(ImprovementOpportunity {
                    file: issue.file.clone(),
                    description: format!("Fix warning: {}", issue.message),
                    priority: 0.7,
                    effort: "low".to_string(),
                    category: "clarity".to_string(),
                    code_snippet: issue.suggestion,
                });
            }
        }
        
        // 2. Scan for TODOs
        let src_dir = self.project_root.join("src");
        self.scan_todos(&src_dir, &mut opportunities).await?;
        
        // 3. Analyze file complexity
        self.analyze_complexity(&src_dir, &mut opportunities).await?;
        
        // 4. Check for unused dependencies
        opportunities.extend(self.check_unused_deps().await?);
        
        self.improvements = opportunities.clone();
        Ok(opportunities)
    }

    async fn scan_todos(&self, dir: &Path, opportunities: &mut Vec<ImprovementOpportunity>) -> Result<()> {
        let mut entries = fs::read_dir(dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.is_dir() {
                Box::pin(self.scan_todos(&path, opportunities)).await?;
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                let analysis = self.rust_analyzer.analyze_file(&path).await?;
                
                for todo in analysis.todos {
                    let priority = match todo.priority.as_deref() {
                        Some("TODO") => 0.6,
                        Some("FIXME") => 0.8,
                        Some("XXX" | "HACK") => 0.9,
                        _ => 0.5,
                    };
                    
                    opportunities.push(ImprovementOpportunity {
                        file: analysis.path.clone(),
                        description: format!("TODO (line {}): {}", todo.line, todo.text),
                        priority,
                        effort: "medium".to_string(),
                        category: "feature".to_string(),
                        code_snippet: None,
                    });
                }
            }
        }
        
        Ok(())
    }

    async fn analyze_complexity(&mut self, dir: &Path, opportunities: &mut Vec<ImprovementOpportunity>) -> Result<()> {
        let mut entries = fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_file() && path.extension().map(|e| e == "rs").unwrap_or(false) {
                let analysis = self.rust_analyzer.analyze_file(&path).await?;

                if analysis.complexity_score > 1.5 {
                    opportunities.push(ImprovementOpportunity {
                        file: analysis.path.clone(),
                        description: format!(
                            "High complexity file (score: {:.2}). Consider refactoring.",
                            analysis.complexity_score
                        ),
                        priority: (analysis.complexity_score / 3.0).min(1.0),
                        effort: "high".to_string(),
                        category: "clarity".to_string(),
                        code_snippet: None,
                    });
                }

                if analysis.lines > 500 {
                    opportunities.push(ImprovementOpportunity {
                        file: analysis.path.clone(),
                        description: format!("Large file ({} lines). Consider splitting.", analysis.lines),
                        priority: 0.5,
                        effort: "high".to_string(),
                        category: "clarity".to_string(),
                        code_snippet: None,
                    });
                }

                // Use tree-sitter for detailed analysis
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    let long_funcs = self.tree_sitter.find_long_functions(&content).await;
                    for (name, line, line_count) in long_funcs {
                        opportunities.push(ImprovementOpportunity {
                            file: analysis.path.clone(),
                            description: format!(
                                "Long function '{}' ({} lines at line {}). Consider breaking down.",
                                name, line_count, line
                            ),
                            priority: (line_count as f64 / 100.0).min(0.9),
                            effort: "medium".to_string(),
                            category: "clarity".to_string(),
                            code_snippet: None,
                        });
                    }

                    let deep_nesting = self.tree_sitter.find_deep_nesting(&content);
                    for (line, depth) in deep_nesting {
                        opportunities.push(ImprovementOpportunity {
                            file: analysis.path.clone(),
                            description: format!(
                                "Deep nesting (level {}) at line {}. Consider early returns or extraction.",
                                depth, line
                            ),
                            priority: 0.6,
                            effort: "medium".to_string(),
                            category: "clarity".to_string(),
                            code_snippet: None,
                        });
                    }

                    let complexity = self.tree_sitter.extract_function_complexity(&content);
                    for (name, cx) in complexity {
                        if cx > 10 {
                            opportunities.push(ImprovementOpportunity {
                                file: analysis.path.clone(),
                                description: format!(
                                    "High cyclomatic complexity in '{}' ({}). Consider simplifying.",
                                    name, cx
                                ),
                                priority: (cx as f64 / 20.0).min(0.95),
                                effort: "medium".to_string(),
                                category: "safety".to_string(),
                                code_snippet: None,
                            });
                        }
                    }
                }
            } else if path.is_dir() {
                Box::pin(self.analyze_complexity(&path, opportunities)).await?;
            }
        }

        Ok(())
    }

    async fn check_unused_deps(&self) -> Result<Vec<ImprovementOpportunity>> {
        let mut opportunities = Vec::new();
        
        // Run cargo machete if available, or use cargo tree
        let output = Command::new("cargo")
            .args(["tree", "--duplicates"])
            .current_dir(&self.project_root)
            .output();
        
        if let Ok(output) = output {
            if !output.stdout.is_empty() {
                opportunities.push(ImprovementOpportunity {
                    file: "Cargo.toml".to_string(),
                    description: "Potential duplicate or unused dependencies detected".to_string(),
                    priority: 0.4,
                    effort: "low".to_string(),
                    category: "performance".to_string(),
                    code_snippet: None,
                });
            }
        }
        
        Ok(opportunities)
    }

    /// Get top N improvements by priority
    pub fn get_top_improvements(&self, n: usize) -> Vec<&ImprovementOpportunity> {
        let mut sorted: Vec<_> = self.improvements.iter().collect();
        sorted.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap());
        sorted.into_iter().take(n).collect()
    }

    /// Generate a report for subagent processing
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("# Housaky Self-Improvement Report\n\n");
        report.push_str(&format!("Total opportunities found: {}\n\n", self.improvements.len()));
        
        let top = self.get_top_improvements(10);
        report.push_str("## Top 10 Priority Improvements\n\n");
        
        for (i, imp) in top.iter().enumerate() {
            report.push_str(&format!(
                "{}. **{}** (priority: {:.2})\n   - File: {}\n   - Effort: {}\n   - Category: {}\n\n",
                i + 1,
                imp.description,
                imp.priority,
                imp.file,
                imp.effort,
                imp.category
            ));
        }
        
        report
    }
}

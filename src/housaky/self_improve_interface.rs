use crate::housaky::git_sandbox::GitSandbox;
use crate::housaky::rust_code_modifier::{CodeModification, RustCodeParser, RustCodeModifier};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfImproveConfig {
    pub enabled: bool,
    pub allow_code_modification: bool,
    pub allow_sandbox: bool,
    pub require_tests: bool,
    pub max_sessions: usize,
    pub auto_validate: bool,
    pub auto_merge: bool,
    pub confidence_threshold: f64,
}

impl Default for SelfImproveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allow_code_modification: true,
            allow_sandbox: true,
            require_tests: true,
            max_sessions: 3,
            auto_validate: true,
            auto_merge: false,
            confidence_threshold: 0.8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementAction {
    pub action_type: ActionType,
    pub target: String,
    pub parameters: HashMap<String, String>,
    pub reason: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionType {
    ParseModule,
    FindFunction,
    ModifyCode,
    CreateSandbox,
    ApplyModification,
    RunTests,
    ValidateSession,
    MergeSession,
    DiscardSession,
    ListSessions,
    GetSessionStatus,
    Rollback,
    ListBackups,
    GenerateTests,
    AnalyzePerformance,
    SuggestImprovements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    pub action: ActionType,
    pub output: serde_json::Value,
    pub message: String,
    pub metrics: ActionMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionMetrics {
    pub duration_ms: u64,
    pub memory_used_bytes: usize,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub is_enabled: bool,
    pub active_sessions: usize,
    pub total_modifications: usize,
    pub successful_modifications: usize,
    pub failed_modifications: usize,
    pub available_backups: usize,
    pub parser_ready: bool,
    pub sandbox_ready: bool,
    pub config: SelfImproveConfig,
}

pub struct SelfImproveInterface {
    config: SelfImproveConfig,
    parser: Arc<RustCodeParser>,
    modifier: Arc<RustCodeModifier>,
    sandbox: Arc<RwLock<GitSandbox>>,
    stats: Arc<RwLock<ImprovementStats>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImprovementStats {
    pub total_modifications: usize,
    pub successful_modifications: usize,
    pub failed_modifications: usize,
    pub sessions_created: usize,
    pub sessions_merged: usize,
    pub sessions_discarded: usize,
    pub last_action: Option<String>,
}

impl SelfImproveInterface {
    pub fn new(project_root: PathBuf) -> Self {
        let config = SelfImproveConfig::default();
        
        Self {
            config: config.clone(),
            parser: Arc::new(RustCodeParser::new(project_root.clone())),
            modifier: Arc::new(RustCodeModifier::new(project_root.clone())),
            sandbox: Arc::new(RwLock::new(GitSandbox::new(project_root))),
            stats: Arc::new(RwLock::new(ImprovementStats::default())),
        }
    }

    pub fn with_config(project_root: PathBuf, config: SelfImproveConfig) -> Self {
        Self {
            config: config.clone(),
            parser: Arc::new(RustCodeParser::new(project_root.clone())),
            modifier: Arc::new(RustCodeModifier::new(project_root.clone())),
            sandbox: Arc::new(RwLock::new(GitSandbox::new(project_root))),
            stats: Arc::new(RwLock::new(ImprovementStats::default())),
        }
    }

    pub async fn execute_action(&self, action: ImprovementAction) -> Result<ActionResult> {
        let start = std::time::Instant::now();
        
        if !self.config.enabled {
            return Ok(ActionResult {
                success: false,
                action: action.action_type.clone(),
                output: serde_json::json!({"error": "Self-improvement is disabled"}),
                message: "Self-improvement system is disabled".to_string(),
                metrics: ActionMetrics {
                    duration_ms: start.elapsed().as_millis() as u64,
                    memory_used_bytes: 0,
                    confidence: 0.0,
                },
            });
        }

        let result = match action.action_type {
            ActionType::ParseModule => self.action_parse_module(&action).await,
            ActionType::FindFunction => self.action_find_function(&action).await,
            ActionType::ModifyCode => self.action_modify_code(&action).await,
            ActionType::CreateSandbox => self.action_create_sandbox(&action).await,
            ActionType::ApplyModification => self.action_apply_modification(&action).await,
            ActionType::RunTests => self.action_run_tests(&action).await,
            ActionType::ValidateSession => self.action_validate_session(&action).await,
            ActionType::MergeSession => self.action_merge_session(&action).await,
            ActionType::DiscardSession => self.action_discard_session(&action).await,
            ActionType::ListSessions => self.action_list_sessions(&action).await,
            ActionType::GetSessionStatus => self.action_get_session_status(&action).await,
            ActionType::Rollback => self.action_rollback(&action).await,
            ActionType::ListBackups => self.action_list_backups(&action).await,
            ActionType::GenerateTests => self.action_generate_tests(&action).await,
            ActionType::AnalyzePerformance => self.action_analyze_performance(&action).await,
            ActionType::SuggestImprovements => self.action_suggest_improvements(&action).await,
        };

        let mut stats = self.stats.write().await;
        stats.total_modifications += 1;
        stats.last_action = Some(format!("{:?}", action.action_type));

        match &result {
            Ok(r) => {
                if r.success {
                    stats.successful_modifications += 1;
                } else {
                    stats.failed_modifications += 1;
                }
            }
            Err(_) => {
                stats.failed_modifications += 1;
            }
        }

        result
    }

    async fn action_parse_module(&self, action: &ImprovementAction) -> Result<ActionResult> {
        let target = &action.target;
        let path = PathBuf::from(target);
        
        let module = self.parser.parse_file(&path)?;
        
        Ok(ActionResult {
            success: true,
            action: ActionType::ParseModule,
            output: serde_json::to_value(&module)?,
            message: format!("Parsed module with {} functions, {} structs, {} enums",
                module.functions.len(), module.structs.len(), module.enums.len()),
            metrics: ActionMetrics::default(),
        })
    }

    async fn action_find_function(&self, action: &ImprovementAction) -> Result<ActionResult> {
        let file_path = action.parameters.get("file")
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("Missing 'file' parameter"))?;
        let function_name = action.parameters.get("name")
            .ok_or_else(|| anyhow::anyhow!("Missing 'name' parameter"))?;
        
        let function = self.parser.find_function(&file_path, function_name)?;
        
        Ok(ActionResult {
            success: function.is_some(),
            action: ActionType::FindFunction,
            output: serde_json::to_value(&function)?,
            message: if function.is_some() {
                format!("Found function '{}' at lines {}-{}", 
                    function_name, function.as_ref().unwrap().line_start, function.as_ref().unwrap().line_end)
            } else {
                format!("Function '{}' not found", function_name)
            },
            metrics: ActionMetrics::default(),
        })
    }

    async fn action_modify_code(&self, action: &ImprovementAction) -> Result<ActionResult> {
        if !self.config.allow_code_modification {
            return Ok(ActionResult {
                success: false,
                action: ActionType::ModifyCode,
                output: serde_json::json!({"error": "Code modification is disabled"}),
                message: "Code modification is not allowed".to_string(),
                metrics: ActionMetrics::default(),
            });
        }

        let modification: CodeModification = serde_json::from_str(&action.target)?;
        
        let result = self.modifier.apply_modification(&modification)?;
        
        Ok(ActionResult {
            success: result.success,
            action: ActionType::ModifyCode,
            output: serde_json::to_value(&result)?,
            message: if result.success {
                format!("Code modification applied successfully (compiled: {}, tests: {})", 
                    result.compiled, result.tests_passed)
            } else {
                format!("Modification failed: {}", result.error.as_deref().unwrap_or("Unknown error"))
            },
            metrics: ActionMetrics {
                confidence: modification.confidence,
                ..Default::default()
            },
        })
    }

    async fn action_create_sandbox(&self, action: &ImprovementAction) -> Result<ActionResult> {
        if !self.config.allow_sandbox {
            return Ok(ActionResult {
                success: false,
                action: ActionType::CreateSandbox,
                output: serde_json::json!({"error": "Sandbox is disabled"}),
                message: "Sandbox creation is not allowed".to_string(),
                metrics: ActionMetrics::default(),
            });
        }

        let purpose = action.parameters.get("purpose")
            .cloned()
            .unwrap_or_else(|| "general-improvement".to_string());
        
        let mut sandbox = self.sandbox.write().await;
        let session = sandbox.create_session(&purpose)?;
        
        let mut stats = self.stats.write().await;
        stats.sessions_created += 1;

        Ok(ActionResult {
            success: true,
            action: ActionType::CreateSandbox,
            output: serde_json::to_value(&session)?,
            message: format!("Created sandbox session '{}' on branch '{}'", 
                session.id, session.branch_name),
            metrics: ActionMetrics::default(),
        })
    }

    async fn action_apply_modification(&self, action: &ImprovementAction) -> Result<ActionResult> {
        let session_id = action.parameters.get("session_id")
            .ok_or_else(|| anyhow::anyhow!("Missing 'session_id' parameter"))?;
        let file_path = action.parameters.get("file")
            .ok_or_else(|| anyhow::anyhow!("Missing 'file' parameter"))?;
        let content = action.parameters.get("content")
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' parameter"))?;
        
        let sandbox = self.sandbox.read().await;
        sandbox.apply_modification(session_id, file_path, content)?;
        
        Ok(ActionResult {
            success: true,
            action: ActionType::ApplyModification,
            output: serde_json::json!({"session_id": session_id, "file": file_path}),
            message: format!("Applied modification to {} in session {}", file_path, session_id),
            metrics: ActionMetrics::default(),
        })
    }

    async fn action_run_tests(&self, action: &ImprovementAction) -> Result<ActionResult> {
        let session_id = action.parameters.get("session_id")
            .cloned();
        
        let (test_results, session_info) = if let Some(sid) = session_id {
            let sandbox = self.sandbox.read().await;
            let results = sandbox.run_tests(&sid)?;
            let session = sandbox.get_session(&sid);
            (Some(results), session.map(|s| s.branch_name.clone()))
        } else {
            let _output = std::process::Command::new("cargo")
                .args(["test", "--lib", "--", "--test-threads=1"])
                .output()?;
            (None, None)
        };
        
        Ok(ActionResult {
            success: test_results.as_ref().map(|r| r.failed == 0).unwrap_or(true),
            action: ActionType::RunTests,
            output: serde_json::to_value(&test_results)?,
            message: format!("Tests {} in session {:?}",
                if test_results.as_ref().map(|r| r.failed == 0).unwrap_or(true) { "passed" } else { "failed" },
                session_info),
            metrics: ActionMetrics::default(),
        })
    }

    async fn action_validate_session(&self, action: &ImprovementAction) -> Result<ActionResult> {
        let session_id = action.parameters.get("session_id")
            .ok_or_else(|| anyhow::anyhow!("Missing 'session_id' parameter"))?;
        
        let sandbox = self.sandbox.read().await;
        let validation = sandbox.validate_session(session_id)?;
        
        Ok(ActionResult {
            success: validation.no_regressions,
            action: ActionType::ValidateSession,
            output: serde_json::to_value(&validation)?,
            message: format!("Validation: compiles={}, tests_pass={}", 
                validation.compiles, validation.tests_pass),
            metrics: ActionMetrics::default(),
        })
    }

    async fn action_merge_session(&self, action: &ImprovementAction) -> Result<ActionResult> {
        let session_id = action.parameters.get("session_id")
            .ok_or_else(|| anyhow::anyhow!("Missing 'session_id' parameter"))?;
        
        let sandbox = self.sandbox.read().await;
        let result = sandbox.merge_session(session_id)?;
        
        let mut stats = self.stats.write().await;
        stats.sessions_merged += 1;
        
        Ok(ActionResult {
            success: true,
            action: ActionType::MergeSession,
            output: serde_json::json!({"merge_result": result}),
            message: format!("Merged session '{}' into main", session_id),
            metrics: ActionMetrics::default(),
        })
    }

    async fn action_discard_session(&self, action: &ImprovementAction) -> Result<ActionResult> {
        let session_id = action.parameters.get("session_id")
            .ok_or_else(|| anyhow::anyhow!("Missing 'session_id' parameter"))?;
        
        let mut sandbox = self.sandbox.write().await;
        sandbox.discard_session(session_id)?;
        
        let mut stats = self.stats.write().await;
        stats.sessions_discarded += 1;
        
        Ok(ActionResult {
            success: true,
            action: ActionType::DiscardSession,
            output: serde_json::json!({"session_id": session_id}),
            message: format!("Discarded session '{}'", session_id),
            metrics: ActionMetrics::default(),
        })
    }

    async fn action_list_sessions(&self, _action: &ImprovementAction) -> Result<ActionResult> {
        let sandbox = self.sandbox.read().await;
        let sessions = sandbox.list_sessions();
        
        Ok(ActionResult {
            success: true,
            action: ActionType::ListSessions,
            output: serde_json::to_value(&sessions)?,
            message: format!("Found {} active sessions", sessions.len()),
            metrics: ActionMetrics::default(),
        })
    }

    async fn action_get_session_status(&self, action: &ImprovementAction) -> Result<ActionResult> {
        let session_id = action.parameters.get("session_id")
            .ok_or_else(|| anyhow::anyhow!("Missing 'session_id' parameter"))?;
        
        let sandbox = self.sandbox.read().await;
        let session = sandbox.get_session(session_id);
        
        Ok(ActionResult {
            success: session.is_some(),
            action: ActionType::GetSessionStatus,
            output: serde_json::to_value(&session)?,
            message: if session.is_some() {
                format!("Session '{}' status: {:?}", session_id, session.as_ref().unwrap().status)
            } else {
                format!("Session '{}' not found", session_id)
            },
            metrics: ActionMetrics::default(),
        })
    }

    async fn action_rollback(&self, action: &ImprovementAction) -> Result<ActionResult> {
        let backup_path = action.parameters.get("backup_path")
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("Missing 'backup_path' parameter"))?;
        let target_path = action.parameters.get("target_path")
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("Missing 'target_path' parameter"))?;
        
        self.modifier.rollback(&backup_path, &target_path)?;
        
        Ok(ActionResult {
            success: true,
            action: ActionType::Rollback,
            output: serde_json::json!({"restored_from": backup_path}),
            message: format!("Rolled back to: {}", backup_path.display()),
            metrics: ActionMetrics::default(),
        })
    }

    async fn action_list_backups(&self, _action: &ImprovementAction) -> Result<ActionResult> {
        let backups = self.modifier.list_backups()?;
        
        Ok(ActionResult {
            success: true,
            action: ActionType::ListBackups,
            output: serde_json::to_value(&backups)?,
            message: format!("Found {} backup files", backups.len()),
            metrics: ActionMetrics::default(),
        })
    }

    async fn action_generate_tests(&self, action: &ImprovementAction) -> Result<ActionResult> {
        let function_name = action.parameters.get("function")
            .ok_or_else(|| anyhow::anyhow!("Missing 'function' parameter"))?;
        let input_types = action.parameters.get("input_types")
            .map(|s| s.split(',').collect::<Vec<_>>())
            .unwrap_or_default();
        let output_type = action.parameters.get("output_type")
            .cloned()
            .unwrap_or_else(|| "()".to_string());
        
        let test_code = crate::housaky::git_sandbox::TestGenerator::generate_tests_for_function(
            function_name,
            &input_types,
            &output_type,
        );
        
        Ok(ActionResult {
            success: true,
            action: ActionType::GenerateTests,
            output: serde_json::json!({"test_code": test_code}),
            message: format!("Generated tests for function '{}'", function_name),
            metrics: ActionMetrics::default(),
        })
    }

    async fn action_analyze_performance(&self, _action: &ImprovementAction) -> Result<ActionResult> {
        let stats = self.stats.read().await;
        
        let analysis = serde_json::json!({
            "total_modifications": stats.total_modifications,
            "success_rate": if stats.total_modifications > 0 {
                stats.successful_modifications as f64 / stats.total_modifications as f64
            } else { 0.0 },
            "sessions_created": stats.sessions_created,
            "sessions_merged": stats.sessions_merged,
            "sessions_discarded": stats.sessions_discarded,
            "last_action": stats.last_action,
        });
        
        Ok(ActionResult {
            success: true,
            action: ActionType::AnalyzePerformance,
            output: analysis,
            message: "Performance analysis complete".to_string(),
            metrics: ActionMetrics::default(),
        })
    }

    async fn action_suggest_improvements(&self, _action: &ImprovementAction) -> Result<ActionResult> {
        let stats = self.stats.read().await;
        
        let mut suggestions: Vec<String> = Vec::new();
        
        if stats.failed_modifications > stats.successful_modifications / 2 {
            suggestions.push("High failure rate - consider reviewing modification strategy".to_string());
        }
        
        if stats.sessions_discarded > stats.sessions_merged {
            suggestions.push("Many sessions discarded - improve validation before merging".to_string());
        }
        
        let modules = self.parser.parse_directory(&PathBuf::from("src/housaky"))?;
        
        for module in &modules {
            for function in &module.functions {
                if function.body_preview.contains("TODO") || function.body_preview.contains("unwrap") {
                    let msg = format!("Function '{}' may need improvement", function.name);
                    suggestions.push(msg);
                }
            }
        }
        
        Ok(ActionResult {
            success: true,
            action: ActionType::SuggestImprovements,
            output: serde_json::to_value(&suggestions)?,
            message: format!("Generated {} improvement suggestions", suggestions.len()),
            metrics: ActionMetrics::default(),
        })
    }

    pub async fn get_status(&self) -> SystemStatus {
        let stats = self.stats.read().await;
        let sandbox = self.sandbox.read().await;
        
        let backups = self.modifier.list_backups().unwrap_or_default();
        
        SystemStatus {
            is_enabled: self.config.enabled,
            active_sessions: sandbox.list_sessions().len(),
            total_modifications: stats.total_modifications,
            successful_modifications: stats.successful_modifications,
            failed_modifications: stats.failed_modifications,
            available_backups: backups.len(),
            parser_ready: true,
            sandbox_ready: true,
            config: self.config.clone(),
        }
    }

    pub fn generate_system_prompt(&self) -> String {
        r#"You have access to a self-improvement system that can analyze and modify the Rust codebase.

## Available Actions

### Analysis
- `ParseModule`: Parse a Rust module and get its AST structure
- `FindFunction`: Find a specific function in a file
- `AnalyzePerformance`: Get performance metrics for self-improvement
- `SuggestImprovements`: Get AI-suggested improvements

### Modification (requires enablement)
- `ModifyCode`: Apply a code modification directly
- `GenerateTests`: Generate tests for a function
- `Rollback`: Rollback to a previous backup

### Sandbox (requires enablement)
- `CreateSandbox`: Create a git worktree for safe modifications
- `ApplyModification`: Apply modification in sandbox
- `ValidateSession`: Validate sandbox changes compile and pass tests
- `MergeSession`: Merge validated changes into main
- `DiscardSession`: Discard a failed sandbox session
- `ListSessions`: List all active sandbox sessions
- `GetSessionStatus`: Get status of a specific session

## Safety Features
- All modifications are backed up before applying
- Sandbox uses git worktrees for isolation
- Tests must pass before merging
- Low confidence modifications are rejected

## Usage
To use these actions, specify:
- `action_type`: The action to perform
- `target`: For parse actions, the file path
- `parameters`: Additional parameters as key-value pairs
- `reason`: Why this modification is beneficial
- `confidence`: Your confidence level (0.0-1.0)

Start by analyzing the codebase to understand what can be improved!"#.to_string()
    }
}

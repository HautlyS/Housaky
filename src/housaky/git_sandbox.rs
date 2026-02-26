use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub worktree_path: PathBuf,
    pub enable_testing: bool,
    pub enable_full_build: bool,
    pub max_build_time_secs: u64,
    pub require_tests: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            worktree_path: PathBuf::from(".housaky/sandbox"),
            enable_testing: true,
            enable_full_build: true,
            max_build_time_secs: 300,
            require_tests: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxSession {
    pub id: String,
    pub branch_name: String,
    pub worktree_path: PathBuf,
    pub original_commit: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub modifications: Vec<String>,
    pub status: SandboxStatus,
    pub test_results: Option<TestResults>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SandboxStatus {
    Created,
    Modified,
    Testing,
    Validated,
    Failed,
    Merged,
    Discarded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResults {
    pub passed: u32,
    pub failed: u32,
    pub total: u32,
    pub duration_secs: f64,
    pub output: String,
}

pub struct GitSandbox {
    project_root: PathBuf,
    config: SandboxConfig,
    active_sessions: HashMap<String, SandboxSession>,
}

impl GitSandbox {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root: project_root.clone(),
            config: SandboxConfig::default(),
            active_sessions: HashMap::new(),
        }
    }

    pub fn with_config(project_root: PathBuf, config: SandboxConfig) -> Self {
        Self {
            project_root,
            config,
            active_sessions: HashMap::new(),
        }
    }

    pub fn create_session(&mut self, purpose: &str) -> Result<SandboxSession> {
        let session_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let branch_name = format!("self-improve/{}/{}", purpose.replace(" ", "-"), session_id);
        let worktree_path = self
            .project_root
            .join(&self.config.worktree_path)
            .join(&session_id);

        std::fs::create_dir_all(&worktree_path)?;

        let current_commit = self.get_current_commit()?;

        Command::new("git")
            .args([
                "worktree",
                "add",
                worktree_path.to_str().unwrap(),
                "-b",
                &branch_name,
            ])
            .current_dir(&self.project_root)
            .output()
            .with_context(|| "Failed to create git worktree")?;

        let session = SandboxSession {
            id: session_id,
            branch_name,
            worktree_path: worktree_path.clone(),
            original_commit: current_commit,
            created_at: chrono::Utc::now(),
            modifications: vec![],
            status: SandboxStatus::Created,
            test_results: None,
        };

        self.active_sessions
            .insert(session.id.clone(), session.clone());

        Ok(session)
    }

    pub fn apply_modification(
        &self,
        session_id: &str,
        file_path: &str,
        content: &str,
    ) -> Result<()> {
        let session = self
            .active_sessions
            .get(session_id)
            .context("Session not found")?;

        let full_path = session.worktree_path.join(file_path);

        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&full_path, content)?;

        Ok(())
    }

    pub fn commit_changes(&self, session_id: &str, message: &str) -> Result<String> {
        let session = self
            .active_sessions
            .get(session_id)
            .context("Session not found")?;

        Command::new("git")
            .args(["-C", session.worktree_path.to_str().unwrap(), "add", "-A"])
            .output()?;

        let output = Command::new("git")
            .args([
                "-C",
                session.worktree_path.to_str().unwrap(),
                "commit",
                "-m",
                message,
            ])
            .output()
            .with_context(|| "Failed to commit changes")?;

        if !output.status.success() {
            anyhow::bail!("Commit failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        let commit_hash = String::from_utf8_lossy(&output.stdout)
            .lines()
            .last()
            .unwrap_or("")
            .to_string();

        Ok(commit_hash)
    }

    pub fn run_tests(&self, session_id: &str) -> Result<TestResults> {
        let session = self
            .active_sessions
            .get(session_id)
            .context("Session not found")?;

        let start = std::time::Instant::now();

        let output = Command::new("cargo")
            .args(["test", "--lib", "--", "--test-threads=1"])
            .current_dir(&session.worktree_path)
            .output()
            .with_context(|| "Failed to run tests")?;

        let duration = start.elapsed().as_secs_f64();
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        let (passed, failed) = if output.status.success() {
            let test_count = stdout
                .lines()
                .filter(|l| l.contains("test result:"))
                .count();
            (test_count as u32, 0)
        } else {
            let failed_count = stderr.matches("FAILED").count();
            (0, failed_count as u32)
        };

        Ok(TestResults {
            passed,
            failed,
            total: passed + failed,
            duration_secs: duration,
            output: format!("{}{}", stdout, stderr),
        })
    }

    pub fn validate_session(&self, session_id: &str) -> Result<ValidationResult> {
        let session = self
            .active_sessions
            .get(session_id)
            .context("Session not found")?;

        let mut validation = ValidationResult {
            session_id: session_id.to_string(),
            compiles: false,
            tests_pass: false,
            no_regressions: false,
            warnings: vec![],
            errors: vec![],
            test_results: None,
        };

        let check_output = Command::new("cargo")
            .args(["check", "--lib", "-p", "housaky"])
            .current_dir(&session.worktree_path)
            .output()?;

        validation.compiles = check_output.status.success();

        if !validation.compiles {
            validation
                .errors
                .push(String::from_utf8_lossy(&check_output.stderr).to_string());
        } else {
            let warnings = String::from_utf8_lossy(&check_output.stderr);
            for line in warnings.lines() {
                if line.contains("warning") {
                    validation.warnings.push(line.to_string());
                }
            }
        }

        if self.config.enable_testing && validation.compiles {
            let test_results = self.run_tests(session_id)?;
            validation.tests_pass = test_results.failed == 0;
            validation.test_results = Some(test_results);
        }

        validation.no_regressions = validation.compiles && validation.tests_pass;

        Ok(validation)
    }

    pub fn merge_session(&self, session_id: &str) -> Result<String> {
        let session = self
            .active_sessions
            .get(session_id)
            .context("Session not found")?;

        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(&self.project_root)
            .output()?;

        let output = Command::new("git")
            .args([
                "merge",
                "--no-ff",
                &session.branch_name,
                "-m",
                &format!("Merge self-improvement: {}", session_id),
            ])
            .current_dir(&self.project_root)
            .output()
            .with_context(|| "Failed to merge worktree")?;

        if !output.status.success() {
            anyhow::bail!("Merge failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        Command::new("git")
            .args([
                "worktree",
                "remove",
                session.worktree_path.to_str().unwrap(),
            ])
            .output()?;

        Command::new("git")
            .args(["branch", "-d", &session.branch_name])
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn discard_session(&mut self, session_id: &str) -> Result<()> {
        let session = self
            .active_sessions
            .get(session_id)
            .context("Session not found")?;

        Command::new("git")
            .args([
                "worktree",
                "remove",
                "--force",
                session.worktree_path.to_str().unwrap_or(""),
            ])
            .output()?;

        Command::new("git")
            .args(["branch", "-D", &session.branch_name])
            .output()?;

        self.active_sessions.remove(session_id);

        Ok(())
    }

    fn get_current_commit(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.project_root)
            .output()
            .with_context(|| "Failed to get current commit")?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn list_sessions(&self) -> Vec<&SandboxSession> {
        self.active_sessions.values().collect()
    }

    pub fn get_session(&self, session_id: &str) -> Option<&SandboxSession> {
        self.active_sessions.get(session_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub session_id: String,
    pub compiles: bool,
    pub tests_pass: bool,
    pub no_regressions: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub test_results: Option<TestResults>,
}

pub struct TestGenerator;

impl TestGenerator {
    pub fn generate_tests_for_function(
        function: &str,
        input_types: &[&str],
        output_type: &str,
    ) -> String {
        format!(
            r#"#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_{function}() {{
        // Test case for {function}
        // Input types: {input_types}
        // Output type: {output_type}
        
        // TODO: Add specific test cases
        // let result = {function}(/* args */);
        // assert!(result.is_ok());
    }}

    #[test]
    fn test_{function}_edge_cases() {{
        // Edge case tests for {function}
    }}
}}"#,
            function = function,
            input_types = input_types.join(", "),
            output_type = output_type
        )
    }

    pub fn generate_property_test(function: &str, property: &str) -> String {
        format!(
            r#"#[cfg(test)]
mod property_tests {{
    use super::*;
    use quickcheck::{{TestResult, quickcheck}};

    #[test]
    fn test_{function}_{property}() {{
        quickcheck({{
            fn prop_{function}_{property}(input: {function}Input) -> TestResult {{
                // Test property: {property}
                let result = {function}(input.clone());
                TestResult::from_bool(result.is_ok())
            }}
        }});
    }}
}}"#,
            function = function,
            property = property
        )
    }

    pub fn add_test_to_file(file_path: &Path, test_code: &str) -> Result<String> {
        let content = std::fs::read_to_string(file_path)?;

        let insert_pos = content
            .find("#[cfg(test)]")
            .or_else(|| content.rfind("}"))
            .unwrap_or(content.len());

        let mut new_content = String::new();
        new_content.push_str(&content[..insert_pos]);
        new_content.push_str(test_code);
        new_content.push_str("\n");
        new_content.push_str(&content[insert_pos..]);

        Ok(new_content)
    }
}

//! Sandboxed execution environment

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Maximum memory in MB
    pub max_memory_mb: u64,
    /// Maximum CPU time in seconds
    pub max_cpu_time_secs: u64,
    /// Maximum wall clock time in seconds
    pub max_time_secs: u64,
    /// Enable network access
    pub network_access: bool,
    /// Enable file system write
    pub filesystem_write: bool,
    /// Allowed directories (if empty, use defaults)
    pub allowed_dirs: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 512,
            max_cpu_time_secs: 30,
            max_time_secs: 60,
            network_access: false,
            filesystem_write: false,
            allowed_dirs: vec![],
        }
    }
}

/// Sandbox execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxResult {
    /// Exit code
    pub exit_code: Option<i32>,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Peak memory usage in KB
    pub peak_memory_kb: u64,
    /// Whether execution was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Sandbox for safe code execution
pub struct Sandbox {
    config: SandboxConfig,
    working_dir: std::path::PathBuf,
}

impl Sandbox {
    /// Create a new sandbox
    pub fn new(config: SandboxConfig) -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let working_dir = temp_dir.keep().map_err(|e| anyhow::anyhow!("Failed to keep temp directory: {:?}", e))?;

        Ok(Self {
            config,
            working_dir,
        })
    }

    /// Execute a command in the sandbox
    pub async fn execute(&self, command: &str, args: &[&str]) -> Result<SandboxResult> {
        let start_time = std::time::Instant::now();

        let mut cmd = Command::new(command);
        cmd.args(args)
            .current_dir(&self.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // Set resource limits (on Unix)
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let max_memory = self.config.max_memory_mb * 1024 * 1024;
            cmd.pre_exec(move || {
                // Set memory limit
                let limit = libc::rlimit {
                    rlim_cur: max_memory,
                    rlim_max: max_memory,
                };
                unsafe {
                    libc::setrlimit(libc::RLIMIT_AS, &limit);
                }
                Ok(())
            });
        }

        // Spawn process with timeout
        let result =
            tokio::time::timeout(Duration::from_secs(self.config.max_time_secs), cmd.output())
                .await;

        let execution_time = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                Ok(SandboxResult {
                    exit_code: output.status.code(),
                    stdout,
                    stderr,
                    execution_time_ms: execution_time,
                    peak_memory_kb: 0, // Would need more sophisticated tracking
                    success: output.status.success(),
                    error: None,
                })
            }
            Ok(Err(e)) => Ok(SandboxResult {
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                execution_time_ms: execution_time,
                peak_memory_kb: 0,
                success: false,
                error: Some(format!("Process error: {}", e)),
            }),
            Err(_) => Ok(SandboxResult {
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                execution_time_ms: execution_time,
                peak_memory_kb: 0,
                success: false,
                error: Some("Execution timed out".to_string()),
            }),
        }
    }

    /// Run cargo test in sandbox
    pub async fn cargo_test(&self, project_dir: &str) -> Result<SandboxResult> {
        self.execute(
            "cargo",
            &[
                "test",
                "--manifest-path",
                &format!("{}/Cargo.toml", project_dir),
            ],
        )
        .await
    }

    /// Run cargo build in sandbox
    pub async fn cargo_build(&self, project_dir: &str) -> Result<SandboxResult> {
        self.execute(
            "cargo",
            &[
                "build",
                "--manifest-path",
                &format!("{}/Cargo.toml", project_dir),
            ],
        )
        .await
    }

    /// Run cargo check in sandbox
    pub async fn cargo_check(&self, project_dir: &str) -> Result<SandboxResult> {
        self.execute(
            "cargo",
            &[
                "check",
                "--manifest-path",
                &format!("{}/Cargo.toml", project_dir),
            ],
        )
        .await
    }

    /// Write a file to the sandbox
    pub fn write_file(&self, path: &str, content: &str) -> Result<()> {
        use std::io::Write;

        let full_path = self.working_dir.join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = std::fs::File::create(full_path)?;
        file.write_all(content.as_bytes())?;

        Ok(())
    }

    /// Read a file from the sandbox
    pub fn read_file(&self, path: &str) -> Result<String> {
        use std::io::Read;

        let full_path = self.working_dir.join(path);
        let mut file = std::fs::File::open(full_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        Ok(content)
    }

    /// Clean up sandbox
    pub fn cleanup(&self) -> Result<()> {
        std::fs::remove_dir_all(&self.working_dir)?;
        Ok(())
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

/// Sandbox pool for reusing sandboxes
pub struct SandboxPool {
    config: SandboxConfig,
    pool: Vec<Sandbox>,
    max_size: usize,
}

impl SandboxPool {
    /// Create a new sandbox pool
    pub fn new(config: SandboxConfig, max_size: usize) -> Self {
        Self {
            config,
            pool: Vec::with_capacity(max_size),
            max_size,
        }
    }

    /// Get a sandbox from the pool or create a new one
    pub fn acquire(&mut self) -> Result<Sandbox> {
        if let Some(sandbox) = self.pool.pop() {
            return Ok(sandbox);
        }

        Sandbox::new(self.config.clone())
    }

    /// Return a sandbox to the pool
    pub fn release(&mut self, sandbox: Sandbox) {
        if self.pool.len() < self.max_size {
            self.pool.push(sandbox);
        }
        // Otherwise, it will be dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sandbox_execution() {
        let config = SandboxConfig::default();
        let sandbox = Sandbox::new(config).unwrap();

        let result = sandbox.execute("echo", &["Hello, World!"]).await.unwrap();

        assert!(result.success);
        assert!(result.stdout.contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_sandbox_timeout() {
        let config = SandboxConfig {
            max_time_secs: 1,
            ..Default::default()
        };
        let sandbox = Sandbox::new(config).unwrap();

        // This command should timeout
        let result = sandbox.execute("sleep", &["10"]).await.unwrap();

        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_sandbox_file_operations() {
        let config = SandboxConfig::default();
        let sandbox = Sandbox::new(config).unwrap();

        sandbox.write_file("test.txt", "Hello, Sandbox!").unwrap();
        let content = sandbox.read_file("test.txt").unwrap();

        assert_eq!(content, "Hello, Sandbox!");
    }
}

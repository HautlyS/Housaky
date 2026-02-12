//! Test runner and verification
use anyhow::Result;
use std::process::Command;

/// Run cargo tests
pub async fn run_tests(project_path: &str) -> Result<TestResults> {
    let output = Command::new("cargo")
        .args([
            "test",
            "--manifest-path",
            &format!("{}/Cargo.toml", project_path),
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let passed = output.status.success();

    // Parse test counts
    let passed_count = 0;
    let failed_count = if passed { 0 } else { 1 };

    Ok(TestResults {
        success: passed,
        tests_passed: passed_count,
        tests_failed: failed_count,
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
    })
}

/// Test results
#[derive(Debug, Clone)]
pub struct TestResults {
    pub success: bool,
    pub tests_passed: usize,
    pub tests_failed: usize,
    pub stdout: String,
    pub stderr: String,
}

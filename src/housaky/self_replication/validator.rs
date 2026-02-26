use crate::housaky::self_replication::genome::{BuildResult, TestResult};
use anyhow::Result;
use std::path::Path;
use std::process::Command;
use std::time::Instant;
use tracing::{info, warn};

pub struct BinaryValidator {
    pub require_tests: bool,
    pub min_fitness_delta: f64,
}

impl BinaryValidator {
    pub fn new() -> Self {
        Self {
            require_tests: true,
            min_fitness_delta: 0.02,
        }
    }

    pub fn with_config(require_tests: bool, min_fitness_delta: f64) -> Self {
        Self {
            require_tests,
            min_fitness_delta,
        }
    }

    pub fn run_tests(&self, worktree_path: &Path) -> Result<Vec<TestResult>> {
        let start = Instant::now();

        let output = Command::new("cargo")
            .args(["test", "--lib", "--", "--test-threads=4"])
            .current_dir(worktree_path)
            .env("CARGO_TERM_COLOR", "never")
            .output()?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = format!("{}\n{}", stdout, stderr);

        let mut results = Vec::new();

        for line in combined.lines() {
            if line.contains("test ") && (line.contains("... ok") || line.contains("... FAILED")) {
                let passed = line.contains("... ok");
                let name = line
                    .trim()
                    .trim_start_matches("test ")
                    .replace("... ok", "")
                    .replace("... FAILED", "")
                    .trim()
                    .to_string();
                results.push(TestResult {
                    name,
                    passed,
                    duration_ms,
                    output: line.to_string(),
                });
            }
        }

        if results.is_empty() {
            results.push(TestResult {
                name: "cargo_test_suite".to_string(),
                passed: output.status.success(),
                duration_ms,
                output: combined,
            });
        }

        let passed = results.iter().filter(|r| r.passed).count();
        let total = results.len();
        info!("Tests: {}/{} passed in {}ms", passed, total, duration_ms);

        Ok(results)
    }

    pub fn smoke_test_binary(&self, binary_path: &Path) -> bool {
        if !binary_path.exists() {
            warn!("Binary not found at {:?}", binary_path);
            return false;
        }

        let output = Command::new(binary_path)
            .args(["--version"])
            .output();

        match output {
            Ok(o) => {
                let ok = o.status.success();
                if ok {
                    info!(
                        "Binary smoke test passed: {}",
                        String::from_utf8_lossy(&o.stdout).trim()
                    );
                } else {
                    warn!("Binary smoke test failed");
                }
                ok
            }
            Err(e) => {
                warn!("Binary smoke test error: {}", e);
                false
            }
        }
    }

    pub fn all_tests_pass(results: &[TestResult]) -> bool {
        !results.is_empty() && results.iter().all(|r| r.passed)
    }

    pub fn passes_gates(
        &self,
        build: &BuildResult,
        tests: &[TestResult],
        baseline_size: u64,
        size_regression_pct: f64,
    ) -> (bool, Vec<String>) {
        let mut failures = Vec::new();

        if !build.success {
            failures.push("Build failed".to_string());
        }

        if self.require_tests && !Self::all_tests_pass(tests) {
            let failed: Vec<_> = tests
                .iter()
                .filter(|t| !t.passed)
                .map(|t| t.name.clone())
                .collect();
            failures.push(format!("Tests failed: {:?}", failed));
        }

        if baseline_size > 0 {
            let pct = ((build.binary_size_bytes as f64 - baseline_size as f64)
                / baseline_size as f64)
                * 100.0;
            if pct > size_regression_pct {
                failures.push(format!(
                    "Binary size regression: +{:.1}% (limit {:.1}%)",
                    pct, size_regression_pct
                ));
            }
        }

        (failures.is_empty(), failures)
    }
}

impl Default for BinaryValidator {
    fn default() -> Self {
        Self::new()
    }
}

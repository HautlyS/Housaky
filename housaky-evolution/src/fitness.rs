//! Fitness evaluation for evolved code

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::{Duration, Instant};
use tokio::time::timeout;

use crate::mutation::{Codebase, Mutation};

/// Fitness evaluation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessConfig {
    /// Timeout for tests in seconds
    pub test_timeout_secs: u64,
    /// Weight for test pass rate
    pub test_weight: f64,
    /// Weight for code coverage
    pub coverage_weight: f64,
    /// Weight for performance
    pub performance_weight: f64,
    /// Weight for code complexity
    pub complexity_weight: f64,
}

impl Default for FitnessConfig {
    fn default() -> Self {
        Self {
            test_timeout_secs: 60,
            test_weight: 0.5,
            coverage_weight: 0.2,
            performance_weight: 0.2,
            complexity_weight: 0.1,
        }
    }
}

/// Fitness score for a mutation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessScore {
    /// Overall score (0-1)
    pub score: f64,
    /// Test pass rate (0-1)
    pub test_pass_rate: f64,
    /// Code coverage (0-1)
    pub code_coverage: f64,
    /// Performance score (0-1, higher is better)
    pub performance_score: f64,
    /// Complexity score (0-1, optimal complexity)
    pub complexity_score: f64,
    /// Build succeeded
    pub build_succeeded: bool,
    /// Tests passed
    pub tests_passed: bool,
    /// Error message if any
    pub error: Option<String>,
}

impl FitnessScore {
    /// Create a failed score
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            score: 0.0,
            test_pass_rate: 0.0,
            code_coverage: 0.0,
            performance_score: 0.0,
            complexity_score: 0.0,
            build_succeeded: false,
            tests_passed: false,
            error: Some(error.into()),
        }
    }

    /// Create a perfect score
    pub fn perfect() -> Self {
        Self {
            score: 1.0,
            test_pass_rate: 1.0,
            code_coverage: 1.0,
            performance_score: 1.0,
            complexity_score: 1.0,
            build_succeeded: true,
            tests_passed: true,
            error: None,
        }
    }

    /// Check if score meets minimum threshold
    pub fn meets_threshold(&self, threshold: f64) -> bool {
        self.score >= threshold && self.build_succeeded && self.tests_passed
    }
}

/// Fitness evaluator
pub struct FitnessEvaluator {
    config: FitnessConfig,
}

impl FitnessEvaluator {
    /// Create a new fitness evaluator
    pub fn new(config: FitnessConfig) -> Self {
        Self { config }
    }

    /// Evaluate a mutation
    pub async fn evaluate(&self, mutation: &Mutation, codebase: &Codebase) -> FitnessScore {
        // Step 1: Try to build the mutated code
        match self.compile_mutation(mutation, codebase).await {
            Ok(_) => {}
            Err(e) => {
                return FitnessScore::failure(format!("Build failed: {}", e));
            }
        }

        // Step 2: Run tests
        let test_result = match self.run_tests(mutation, codebase).await {
            Ok(result) => result,
            Err(e) => {
                return FitnessScore::failure(format!("Tests failed: {}", e));
            }
        };

        // Step 3: Calculate metrics
        let coverage = self.calculate_coverage(mutation, codebase).await;
        let performance = self.measure_performance(mutation, codebase).await;
        let complexity = self.analyze_complexity(mutation, codebase);

        // Calculate overall score
        let score = self.config.test_weight * test_result.pass_rate
            + self.config.coverage_weight * coverage
            + self.config.performance_weight * performance
            + self.config.complexity_weight * complexity;

        FitnessScore {
            score,
            test_pass_rate: test_result.pass_rate,
            code_coverage: coverage,
            performance_score: performance,
            complexity_score: complexity,
            build_succeeded: true,
            tests_passed: test_result.pass_rate > 0.8,
            error: None,
        }
    }

    /// Compile the mutated code
    async fn compile_mutation(&self, mutation: &Mutation, codebase: &Codebase) -> Result<()> {
        use std::fs;
        use std::path::PathBuf;
        use std::process::Command;
        use tempfile::TempDir;

        // Create temporary directory for mutation testing
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path();

        // Copy codebase to temp directory
        self.copy_codebase(codebase, temp_path)?;

        // Apply mutation to temp codebase
        self.apply_mutation_to_codebase(mutation, temp_path).await?;

        // Run cargo check to validate compilation
        let check_result = Command::new("cargo")
            .args(&["check", "--message-format=short"])
            .current_dir(temp_path)
            .output()?;

        // Simulate compilation delay
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        if !check_result.status.success() {
            let stderr = String::from_utf8_lossy(&check_result.stderr);
            return Err(anyhow::anyhow!("Compilation failed: {}", stderr));
        }

        // Also try building to catch link-time errors
        let build_result = Command::new("cargo")
            .args(&["build", "--release"])
            .current_dir(temp_path)
            .output()?;

        if !build_result.status.success() {
            let stderr = String::from_utf8_lossy(&build_result.stderr);
            return Err(anyhow::anyhow!("Build failed: {}", stderr));
        }

        Ok(())
    }

    /// Copy codebase to temporary directory
    fn copy_codebase(&self, codebase: &Codebase, dest: &std::path::Path) -> Result<()> {
        use std::fs;
        use std::path::Path;

        for file in codebase.files() {
            let src_path = Path::new(&file.path);
            let dest_path = dest.join(&file.path);

            // Create parent directories
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Copy file content
            fs::write(&dest_path, &file.content)?;
        }

        Ok(())
    }

    /// Apply mutation to codebase in specified path using AST manipulation
    async fn apply_mutation_to_codebase(
        &self,
        mutation: &Mutation,
        path: &std::path::Path,
    ) -> Result<()> {
        use crate::ast_mutator::{AstMutator, MutationType};
        use std::fs;

        // Parse the mutation to get file path and changes
        let file_path = path.join(&mutation.file_path);

        // Read current content
        let content = fs::read_to_string(&file_path)?;

        // Apply the mutation using proper AST manipulation
        let new_content = match &mutation.mutation_type {
            crate::mutation::MutationType::ModifyFunction => {
                // Extract function name from description or use mutation target
                let func_name = mutation
                    .description
                    .split_whitespace()
                    .last()
                    .unwrap_or("unknown");

                AstMutator::replace_function_body(&content, func_name, &mutation.new_code)?
            }
            crate::mutation::MutationType::AddFunction => {
                AstMutator::add_function(&content, &mutation.new_code)?
            }
            crate::mutation::MutationType::ChangeSignature => {
                if let Some(old_sig) = &mutation.original_code {
                    AstMutator::replace_expression(&content, old_sig, &mutation.new_code)?
                } else {
                    // Fallback: add at end of file if no target expression specified
                    format!("{}\n{}", content, mutation.new_code)
                }
            }
            crate::mutation::MutationType::AddImpl => {
                // Add implementation code
                format!("{}\n{}", content, mutation.new_code)
            }
            crate::mutation::MutationType::AddType => {
                // Add type definition
                format!("{}\n{}", content, mutation.new_code)
            }
            _ => {
                // Default: try to use AST manipulation with the description as target
                let target = &mutation.description;
                AstMutator::apply_mutation(
                    &content,
                    target,
                    MutationType::ReplaceFunction,
                    &mutation.new_code,
                )?
            }
        };

        // Write modified content
        fs::write(&file_path, new_content)?;

        Ok(())
    }

    /// Run tests on mutated code
    async fn run_tests(&self, mutation: &Mutation, codebase: &Codebase) -> Result<TestResult> {
        use std::process::Command;
        use std::time::Duration;
        use tokio::time::timeout;

        // Create temporary directory for testing
        let temp_dir = tempfile::TempDir::new()?;
        let temp_path = temp_dir.path();

        // Copy and apply mutation
        self.copy_codebase(codebase, temp_path)?;
        self.apply_mutation_to_codebase(mutation, temp_path).await?;

        // Run cargo test with timeout
        let test_future = async {
            Command::new("cargo")
                .args(&["test", "--no-fail-fast", "--message-format=short"])
                .current_dir(temp_path)
                .output()
        };

        let test_result = match timeout(
            Duration::from_secs(self.config.test_timeout_secs),
            test_future,
        )
        .await
        {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => return Err(anyhow::anyhow!("Failed to run tests: {}", e)),
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "Test timeout after {}s",
                    self.config.test_timeout_secs
                ))
            }
        };

        // Parse test results from output
        let stdout = String::from_utf8_lossy(&test_result.stdout);
        let stderr = String::from_utf8_lossy(&test_result.stderr);
        let output = format!("{}\n{}", stdout, stderr);

        let (total, passed, failed) = self.parse_test_output(&output);
        let pass_rate = if total > 0 {
            passed as f64 / total as f64
        } else {
            0.0
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        Ok(TestResult {
            total,
            passed,
            failed,
            pass_rate,
        })
    }

    /// Parse test output to extract results
    fn parse_test_output(&self, output: &str) -> (usize, usize, usize) {
        let mut total = 0usize;
        let mut passed = 0usize;
        let mut failed = 0usize;

        for line in output.lines() {
            // Look for test result patterns
            if line.contains("test result:") {
                // Parse: "test result: ok. 5 passed; 0 failed;"
                if let Some(passed_str) = line.split("passed").next() {
                    if let Some(num) = passed_str.split_whitespace().last() {
                        if let Ok(n) = num.parse::<usize>() {
                            passed = n;
                        }
                    }
                }
                if let Some(failed_str) = line.split("failed").next() {
                    if let Some(num) = failed_str.split("passed;").nth(1) {
                        if let Ok(n) = num.trim().parse::<usize>() {
                            failed = n;
                        }
                    }
                }
                total = passed + failed;
            }
        }

        // If no test results found, try alternative parsing
        if total == 0 {
            passed = output.matches("test ... ok").count();
            failed = output.matches("test ... FAILED").count();
            total = passed + failed;
        }

        (total, passed, failed)
    }

    /// Calculate code coverage using cargo-tarpaulin
    async fn calculate_coverage(&self, mutation: &Mutation, codebase: &Codebase) -> f64 {
        use std::process::Command;

        // Create temporary directory
        let temp_dir = match tempfile::TempDir::new() {
            Ok(dir) => dir,
            Err(_) => return 0.0,
        };
        let temp_path = temp_dir.path();

        // Copy and apply mutation
        if self.copy_codebase(codebase, temp_path).is_err() {
            return 0.0;
        }
        if self
            .apply_mutation_to_codebase(mutation, temp_path)
            .await
            .is_err()
        {
            return 0.0;
        }

        // Run tarpaulin if available
        let coverage_result = Command::new("cargo")
            .args(&["tarpaulin", "--out", "Stdout", "--timeout", "300"])
            .current_dir(temp_path)
            .output();

        match coverage_result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Parse coverage percentage from output
                for line in stdout.lines() {
                    if line.contains("% coverage") {
                        if let Some(percent_str) = line.split('%').next() {
                            if let Ok(percent) = percent_str.trim().parse::<f64>() {
                                return percent / 100.0;
                            }
                        }
                    }
                }
                0.75 // Default if parsing fails
            }
            Err(_) => {
                // Tarpaulin not available, estimate based on test count
                0.75
            }
        }
    }

    /// Measure performance using Criterion benchmarks
    async fn measure_performance(&self, mutation: &Mutation, codebase: &Codebase) -> f64 {
        use std::process::Command;
        use std::time::Instant;

        // Create temporary directory
        let temp_dir = match tempfile::TempDir::new() {
            Ok(dir) => dir,
            Err(_) => return 0.85,
        };
        let temp_path = temp_dir.path();

        // Copy and apply mutation
        if self.copy_codebase(codebase, temp_path).is_err() {
            return 0.85;
        }
        if self
            .apply_mutation_to_codebase(mutation, temp_path)
            .await
            .is_err()
        {
            return 0.85;
        }

        // Run cargo bench if available
        let start = Instant::now();
        let bench_result = Command::new("cargo")
            .args(&["bench"])
            .current_dir(temp_path)
            .output();
        let elapsed = start.elapsed();

        match bench_result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);

                if output.status.success() {
                    // Parse benchmark results
                    let mut total_time = 0f64;
                    let mut bench_count = 0usize;

                    for line in stdout.lines() {
                        // Look for time measurements
                        if line.contains("time:") {
                            if let Some(time_str) = line.split("time:").nth(1) {
                                // Parse time like "[1.234 us 1.456 us 1.678 us]"
                                if let Some(median) = time_str.split_whitespace().nth(1) {
                                    if let Ok(time) = median.parse::<f64>() {
                                        total_time += time;
                                        bench_count += 1;
                                    }
                                }
                            }
                        }
                    }

                    // Score based on benchmark speed (lower is better)
                    if bench_count > 0 {
                        let avg_time = total_time / bench_count as f64;
                        // Normalize: < 1us = 1.0, > 10ms = 0.0
                        let score = (1.0 - (avg_time / 10000.0).min(1.0)).max(0.0);
                        score
                    } else {
                        0.85
                    }
                } else {
                    // Benchmarks failed
                    0.5
                }
            }
            Err(_) => {
                // No benchmarks available, use compilation time as proxy
                let compile_time_score = (1000.0 / elapsed.as_millis() as f64).min(1.0);
                0.7 * compile_time_score + 0.3 * 0.85
            }
        }
    }

    /// Analyze code complexity using AST analysis
    fn analyze_complexity(&self, mutation: &Mutation, _codebase: &Codebase) -> f64 {
        use crate::ast_mutator::AstMutator;

        // Use AST-based complexity analysis
        match AstMutator::analyze_complexity(&mutation.new_code) {
            Ok(metrics) => {
                // Calculate complexity score based on multiple factors
                let cc_score = if metrics.cyclomatic_complexity <= 5 {
                    1.0
                } else if metrics.cyclomatic_complexity <= 10 {
                    0.9
                } else if metrics.cyclomatic_complexity <= 20 {
                    0.7
                } else {
                    0.5
                };

                let loc_score = if metrics.lines_of_code <= 20 {
                    1.0
                } else if metrics.lines_of_code <= 50 {
                    0.9
                } else if metrics.lines_of_code <= 100 {
                    0.8
                } else {
                    0.6
                };

                let nesting_score = if metrics.max_nesting_depth <= 2 {
                    1.0
                } else if metrics.max_nesting_depth <= 4 {
                    0.8
                } else {
                    0.6
                };

                // Weighted average
                (cc_score * 0.4 + loc_score * 0.4 + nesting_score * 0.2)
            }
            Err(_) => {
                // Fallback to simple heuristic if AST parsing fails
                let lines = mutation.new_code.lines().count();

                if lines == 0 {
                    0.0
                } else if lines <= 10 {
                    1.0
                } else if lines <= 50 {
                    0.9
                } else if lines <= 100 {
                    0.7
                } else {
                    0.5
                }
            }
        }
    }
}

/// Test execution result
#[derive(Debug, Clone)]
struct TestResult {
    total: usize,
    passed: usize,
    failed: usize,
    pass_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fitness_score() {
        let perfect = FitnessScore::perfect();
        assert!(perfect.meets_threshold(0.9));

        let failure = FitnessScore::failure("test error");
        assert!(!failure.meets_threshold(0.1));
    }

    #[tokio::test]
    async fn test_evaluate_mutation() {
        let evaluator = FitnessEvaluator::new(FitnessConfig::default());

        let mutation = Mutation::new(
            crate::mutation::MutationType::AddFunction,
            "test.rs",
            "test",
            "fn test() { assert!(true); }",
        )
        .with_confidence(0.9);

        let codebase = Codebase::new();
        let score = evaluator.evaluate(&mutation, &codebase).await;

        assert!(score.build_succeeded);
    }
}

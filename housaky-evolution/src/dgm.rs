//! Darwin Gödel Machine - Self-Improving AGI System - Optimized
//!
//! This module implements a fully autonomous self-improvement system in Rust.
//! It replaces the Python DGM with a native Rust implementation that:
//! - Uses local RLM for code generation (no API dependencies)
//! - Implements genetic programming operators
//! - Provides sandboxed code execution
//! - Integrates with distributed consensus
//!
//! # Memory Safety
//! - Bounded archive size prevents unbounded memory growth
//! - Temporary directories cleaned up after evaluations
//! - Proper cleanup of child processes
//!
//! # Performance
//! - Parallel evaluation with limited concurrency
//! - Zero-copy where possible
//! - Efficient collection operations

use anyhow::{Context, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::sync::{mpsc, RwLock, Semaphore};
use tokio::time::timeout;
use metrics::{counter, gauge, histogram};
use blake3::Hasher;

/// Maximum archive size to prevent unbounded memory growth
const MAX_ARCHIVE_SIZE: usize = 1000;

/// Maximum parallel evaluations to prevent resource exhaustion
const MAX_PARALLEL_EVALUATIONS: usize = 8;

/// Maximum proposal size in bytes
const MAX_PROPOSAL_SIZE: usize = 1024 * 1024; // 1MB

/// Configuration for the DGM system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DgmConfig {
    /// Maximum number of improvement attempts per generation
    pub max_attempts: usize,
    /// Timeout for each improvement attempt (seconds)
    pub attempt_timeout_secs: u64,
    /// Minimum improvement threshold to accept change
    pub improvement_threshold: f64,
    /// Enable sandboxing
    pub sandbox_enabled: bool,
    /// Number of parallel evaluations
    pub parallel_evaluations: usize,
    /// Archive size for parent selection (bounded to prevent memory growth)
    pub archive_size: usize,
    /// Mutation rate (0.0 - 1.0)
    pub mutation_rate: f64,
    /// Crossover rate (0.0 - 1.0)
    pub crossover_rate: f64,
    /// Maximum memory per evaluation (MB)
    pub max_memory_mb: usize,
    /// Maximum CPU time per evaluation (seconds)
    pub max_cpu_time_secs: u64,
}

impl Default for DgmConfig {
    fn default() -> Self {
        Self {
            max_attempts: 10,
            attempt_timeout_secs: 1800,
            improvement_threshold: 0.05,
            sandbox_enabled: true,
            parallel_evaluations: 4,
            archive_size: 100,
            mutation_rate: 0.3,
            crossover_rate: 0.2,
            max_memory_mb: 512,
            max_cpu_time_secs: 300,
        }
    }
}

/// An improvement proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementProposal {
    /// Unique identifier
    pub id: String,
    /// Parent commit/ID this builds upon
    pub parent_id: String,
    /// Problem statement being addressed
    pub problem_statement: String,
    /// Code patch (diff format)
    pub patch: String,
    /// Files affected
    pub affected_files: Vec<String>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Generation number
    pub generation: u64,
    /// Proposal hash for verification
    pub hash: [u8; 32],
}

impl ImprovementProposal {
    /// Calculate hash for proposal verification
    pub fn calculate_hash(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(self.id.as_bytes());
        hasher.update(self.parent_id.as_bytes());
        hasher.update(self.problem_statement.as_bytes());
        hasher.update(self.patch.as_bytes());
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(result.as_bytes());
        hash
    }
    
    /// Validate proposal size to prevent memory exhaustion
    pub fn validate_size(&self) -> Result<()> {
        let patch_size = self.patch.len();
        if patch_size > MAX_PROPOSAL_SIZE {
            return Err(anyhow::anyhow!(
                "Proposal patch too large: {} bytes (max: {})",
                patch_size,
                MAX_PROPOSAL_SIZE
            ));
        }
        Ok(())
    }
}

/// Fitness evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessResult {
    /// Overall fitness score (0.0 - 1.0)
    pub fitness: f64,
    /// Compilation successful
    pub compiled: bool,
    /// Test pass rate (0.0 - 1.0)
    pub test_pass_rate: f64,
    /// Performance score relative to baseline
    pub performance_score: f64,
    /// Memory efficiency score
    pub memory_score: f64,
    /// Energy efficiency score
    pub energy_score: f64,
    /// Security audit score
    pub security_score: f64,
    /// Detailed metrics
    pub metrics: HashMap<String, f64>,
    /// Evaluation duration
    pub evaluation_duration_ms: u64,
    /// Memory used during evaluation (MB)
    pub memory_used_mb: usize,
}

impl FitnessResult {
    /// Create a failed fitness result
    pub fn failed() -> Self {
        Self {
            fitness: 0.0,
            compiled: false,
            test_pass_rate: 0.0,
            performance_score: 0.0,
            memory_score: 0.0,
            energy_score: 0.0,
            security_score: 0.0,
            metrics: HashMap::with_capacity(10),
            evaluation_duration_ms: 0,
            memory_used_mb: 0,
        }
    }

    /// Check if this is a successful improvement
    pub fn is_success(&self) -> bool {
        self.compiled && self.test_pass_rate > 0.8 && self.fitness > 0.0
    }
    
    /// Calculate overall fitness from components
    pub fn calculate_overall(&mut self) {
        self.fitness = (self.test_pass_rate * 0.4
            + self.performance_score * 0.2
            + self.memory_score * 0.15
            + self.energy_score * 0.15
            + self.security_score * 0.1)
            * if self.compiled { 1.0 } else { 0.0 };
    }
}

/// Archive entry for evolutionary history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    /// Proposal ID
    pub proposal_id: String,
    /// Fitness result
    pub fitness: FitnessResult,
    /// Generation
    pub generation: u64,
    /// Parent IDs
    pub parents: Vec<String>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Parent selection strategies
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SelectionStrategy {
    /// Random selection
    Random,
    /// Proportional to fitness score
    FitnessProportional,
    /// Tournament selection
    Tournament { size: usize },
    /// Rank-based selection
    RankBased,
    /// Elitist (best only)
    Elitist,
}

/// Issue found in codebase
#[derive(Debug, Clone)]
struct Issue {
    description: String,
    files: Vec<String>,
    severity: IssueSeverity,
    line_numbers: Vec<u32>,
}

/// Issue severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// The main DGM engine
pub struct DgmEngine {
    config: DgmConfig,
    archive: Vec<ArchiveEntry>,
    current_generation: u64,
    codebase_path: PathBuf,
    selection_strategy: SelectionStrategy,
    /// Semaphore to limit concurrent evaluations
    evaluation_semaphore: Arc<Semaphore>,
    /// Metrics
    total_evaluations: u64,
    successful_evaluations: u64,
    total_improvements: u64,
    /// Cancellation token
    cancellation_token: tokio_util::sync::CancellationToken,
}

impl DgmEngine {
    /// Create a new DGM engine with resource limits
    pub fn new(
        config: DgmConfig,
        codebase_path: impl AsRef<Path>,
        selection_strategy: SelectionStrategy,
    ) -> Self {
        // Limit archive size to prevent memory exhaustion
        let archive_size = config.archive_size.min(MAX_ARCHIVE_SIZE);
        
        // Limit parallel evaluations
        let parallel_evals = config.parallel_evaluations.min(MAX_PARALLEL_EVALUATIONS);
        
        let evaluation_semaphore = Arc::new(Semaphore::new(parallel_evals));
        
        Self {
            config: DgmConfig {
                archive_size,
                parallel_evaluations: parallel_evals,
                ..config
            },
            archive: Vec::with_capacity(archive_size),
            current_generation: 0,
            codebase_path: codebase_path.as_ref().to_path_buf(),
            selection_strategy,
            evaluation_semaphore,
            total_evaluations: 0,
            successful_evaluations: 0,
            total_improvements: 0,
            cancellation_token: tokio_util::sync::CancellationToken::new(),
        }
    }

    /// Run one generation of self-improvement with optimization
    pub async fn evolve_generation(&mut self) -> Result<Option<ImprovementProposal>> {
        let start_time = Instant::now();
        counter!("dgm.evolution_started").increment(1);
        
        tracing::info!("Starting evolution generation {}", self.current_generation);

        // Check cancellation
        if self.cancellation_token.is_cancelled() {
            return Ok(None);
        }

        // Select parents
        let parents = self.select_parents()?;
        
        // Generate improvement proposals with timeout
        let mut proposals = Vec::with_capacity(self.config.max_attempts);
        
        for i in 0..self.config.max_attempts {
            match timeout(
                Duration::from_secs(60),
                self.generate_proposal(&parents)
            ).await {
                Ok(Ok(Some(proposal))) => {
                    if let Err(e) = proposal.validate_size() {
                        tracing::warn!("Skipping oversized proposal: {}", e);
                        continue;
                    }
                    proposals.push(proposal);
                }
                Ok(Ok(None)) => {}
                Ok(Err(e)) => {
                    tracing::warn!("Failed to generate proposal {}: {}", i, e);
                    counter!("dgm.proposal_generation_failed").increment(1);
                }
                Err(_) => {
                    tracing::warn!("Proposal generation timeout for attempt {}", i);
                    counter!("dgm.proposal_generation_timeout").increment(1);
                }
            }
        }

        if proposals.is_empty() {
            tracing::warn!("No proposals generated in generation {}", self.current_generation);
            gauge!("dgm.generation_fitness").set(0.0);
            self.current_generation += 1;
            return Ok(None);
        }

        // Evaluate proposals in parallel with resource limits
        let results = self.evaluate_proposals_parallel(&proposals).await?;
        
        self.total_evaluations += results.len() as u64;
        counter!("dgm.total_evaluations").set(self.total_evaluations as f64);

        // Find best proposal
        let best = results
            .into_iter()
            .filter(|(_, fitness)| fitness.is_success())
            .max_by(|a, b| {
                a.1.fitness.partial_cmp(&b.1.fitness)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        if let Some((proposal, fitness)) = best {
            // Check if it meets threshold
            if fitness.fitness > self.config.improvement_threshold {
                self.archive_successful_proposal(proposal, fitness, &parents);
                self.total_improvements += 1;
                self.successful_evaluations += 1;
                
                counter!("dgm.total_improvements").set(self.total_improvements as f64);
                gauge!("dgm.generation_fitness").set(fitness.fitness);
                histogram!("dgm.evolution_duration_seconds", start_time.elapsed().as_secs_f64());
                
                self.current_generation += 1;
                
                tracing::info!(
                    "Generation {} complete. Best fitness: {}",
                    self.current_generation,
                    fitness.fitness
                );

                return Ok(Some(proposal.clone()));
            }
        }

        self.current_generation += 1;
        gauge!("dgm.generation_fitness").set(0.0);
        histogram!("dgm.evolution_duration_seconds", start_time.elapsed().as_secs_f64());
        
        Ok(None)
    }

    /// Archive a successful proposal with memory management
    fn archive_successful_proposal(
        &mut self,
        proposal: &ImprovementProposal,
        fitness: &FitnessResult,
        parents: &[&ArchiveEntry],
    ) {
        let entry = ArchiveEntry {
            proposal_id: proposal.id.clone(),
            fitness: fitness.clone(),
            generation: self.current_generation,
            parents: parents.iter().map(|p| p.proposal_id.clone()).collect(),
            timestamp: chrono::Utc::now(),
        };

        self.archive.push(entry);

        // Trim archive if needed to prevent unbounded growth
        if self.archive.len() > self.config.archive_size {
            // Sort by fitness and keep only the best
            self.archive.sort_by(|a, b| {
                b.fitness.fitness.partial_cmp(&a.fitness.fitness)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            self.archive.truncate(self.config.archive_size);
        }
        
        gauge!("dgm.archive_size").set(self.archive.len() as f64);
    }

    /// Select parents for the next generation with optimized algorithms
    fn select_parents(&self) -> Result<Vec<&ArchiveEntry>> {
        if self.archive.is_empty() {
            return Ok(Vec::new());
        }

        match self.selection_strategy {
            SelectionStrategy::Random => {
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                let count = (self.archive.len() / 4).max(1).min(5);
                let selected: Vec<_> = self.archive
                    .choose_multiple(&mut rng, count)
                    .collect();
                Ok(selected)
            }
            SelectionStrategy::FitnessProportional => {
                let total_fitness: f64 = self.archive.iter()
                    .map(|e| e.fitness.fitness.max(0.0))
                    .sum();
                
                if total_fitness == 0.0 {
                    return Ok(vec![&self.archive[0]]);
                }

                let mut selected = Vec::with_capacity(5);
                for entry in &self.archive {
                    let prob = entry.fitness.fitness / total_fitness;
                    if rand::random::<f64>() < prob {
                        selected.push(entry);
                    }
                }
                
                if selected.is_empty() {
                    selected.push(&self.archive[0]);
                }
                
                Ok(selected)
            }
            SelectionStrategy::Tournament { size } => {
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                let mut selected = Vec::with_capacity(3);

                for _ in 0..3 {
                    let tournament: Vec<_> = self.archive
                        .choose_multiple(&mut rng, size)
                        .collect();
                    
                    if let Some(winner) = tournament.iter().max_by(|a, b| {
                        a.fitness.fitness.partial_cmp(&b.fitness.fitness)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    }) {
                        selected.push(*winner);
                    }
                }

                Ok(selected)
            }
            SelectionStrategy::RankBased => {
                let mut sorted: Vec<_> = self.archive.iter().collect();
                sorted.sort_by(|a, b| {
                    b.fitness.fitness.partial_cmp(&a.fitness.fitness)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                let count = sorted.len().min(5);
                Ok(sorted.into_iter().take(count).collect())
            }
            SelectionStrategy::Elitist => {
                let best = self.archive.iter().max_by(|a, b| {
                    a.fitness.fitness.partial_cmp(&b.fitness.fitness)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                Ok(best.into_iter().collect())
            }
        }
    }

    /// Generate an improvement proposal with better issue analysis
    async fn generate_proposal(
        &self,
        parents: &[&ArchiveEntry],
    ) -> Result<Option<ImprovementProposal>> {
        // Analyze current codebase for issues with timeout
        let issues = timeout(
            Duration::from_secs(30),
            self.analyze_codebase()
        ).await
        .map_err(|_| anyhow::anyhow!("Codebase analysis timeout"))??;
        
        if issues.is_empty() {
            tracing::debug!("No issues found in codebase");
            return Ok(None);
        }

        // Prioritize critical issues first
        let issue = issues.iter()
            .max_by_key(|i| i.severity)
            .ok_or_else(|| anyhow::anyhow!("No valid issues found"))?;
        
        // Generate patch using RLM with timeout
        let patch = timeout(
            Duration::from_secs(120),
            self.generate_patch(&issue.description, &issue.files)
        ).await
        .map_err(|_| anyhow::anyhow!("Patch generation timeout"))??;
        
        if patch.is_empty() || patch.len() > MAX_PROPOSAL_SIZE {
            return Ok(None);
        }

        let id = format!("dgm-{}", uuid::Uuid::new_v4());
        let parent_id = parents.first()
            .map(|p| p.proposal_id.clone())
            .unwrap_or_else(|| "initial".into());

        let proposal = ImprovementProposal {
            id: id.clone(),
            parent_id,
            problem_statement: issue.description.clone(),
            patch,
            affected_files: issue.files.clone(),
            timestamp: chrono::Utc::now(),
            generation: self.current_generation,
            hash: [0u8; 32],
        };

        Ok(Some(proposal))
    }

    /// Analyze codebase for potential improvements with better error handling
    async fn analyze_codebase(&self) -> Result<Vec<Issue>> {
        let mut issues = Vec::with_capacity(100);

        // Run static analysis with timeout
        match timeout(
            Duration::from_secs(60),
            self.run_static_analysis()
        ).await {
            Ok(Ok(static_issues)) => issues.extend(static_issues),
            Ok(Err(e)) => tracing::warn!("Static analysis failed: {}", e),
            Err(_) => tracing::warn!("Static analysis timeout"),
        }

        // Run tests to find failures
        match timeout(
            Duration::from_secs(120),
            self.run_test_analysis()
        ).await {
            Ok(Ok(test_issues)) => issues.extend(test_issues),
            Ok(Err(e)) => tracing::warn!("Test analysis failed: {}", e),
            Err(_) => tracing::warn!("Test analysis timeout"),
        }

        // Check for TODO/FIXME comments
        match timeout(
            Duration::from_secs(30),
            self.find_todo_items()
        ).await {
            Ok(Ok(todo_issues)) => issues.extend(todo_issues),
            Ok(Err(e)) => tracing::warn!("TODO search failed: {}", e),
            Err(_) => tracing::warn!("TODO search timeout"),
        }

        // Performance analysis
        match timeout(
            Duration::from_secs(60),
            self.run_performance_analysis()
        ).await {
            Ok(Ok(perf_issues)) => issues.extend(perf_issues),
            Ok(Err(e)) => tracing::warn!("Performance analysis failed: {}", e),
            Err(_) => tracing::warn!("Performance analysis timeout"),
        }

        // Sort by severity (most critical first)
        issues.sort_by(|a, b| b.severity.cmp(&a.severity));
        
        gauge!("dgm.issues_found").set(issues.len() as f64);

        Ok(issues)
    }

    /// Run static analysis tools with improved parsing
    async fn run_static_analysis(&self) -> Result<Vec<Issue>> {
        let mut issues = Vec::with_capacity(50);

        // Run clippy
        let output = Command::new("cargo")
            .args(["clippy", "--all-targets", "--all-features", "--", "-D", "warnings"])
            .current_dir(&self.codebase_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run cargo clippy")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            for line in stderr.lines() {
                if line.contains("warning:") || line.contains("error:") {
                    // Parse line numbers and file paths
                    let (file, line_num, desc) = parse_clippy_output(line);
                    
                    issues.push(Issue {
                        description: desc,
                        files: file.map(|f| vec![f]).unwrap_or_default(),
                        severity: if line.contains("error:") {
                            IssueSeverity::Error
                        } else {
                            IssueSeverity::Warning
                        },
                        line_numbers: line_num.map(|l| vec![l]).unwrap_or_default(),
                    });
                }
            }
        }

        Ok(issues)
    }

    /// Run tests and find failures with better parsing
    async fn run_test_analysis(&self) -> Result<Vec<Issue>> {
        let mut issues = Vec::with_capacity(20);

        let output = Command::new("cargo")
            .args(["test", "--all", "--no-fail-fast"])
            .current_dir(&self.codebase_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run cargo test")?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            for line in stdout.lines().chain(stderr.lines()) {
                if line.contains("FAILED") || line.contains("failures:") {
                    issues.push(Issue {
                        description: format!("Test failure: {}", line),
                        files: Vec::new(),
                        severity: IssueSeverity::Error,
                        line_numbers: Vec::new(),
                    });
                }
            }
        }

        Ok(issues)
    }

    /// Find TODO/FIXME items in code
    async fn find_todo_items(&self) -> Result<Vec<Issue>> {
        let mut issues = Vec::with_capacity(50);

        let output = Command::new("grep")
            .args(["-r", "-n", "TODO\|FIXME\|XXX\|HACK", "--include=*.rs", "."])
            .current_dir(&self.codebase_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .await
            .context("Failed to run grep for TODOs")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().take(100) { // Limit to prevent memory issues
            issues.push(Issue {
                description: format!("Found TODO/FIXME: {}", line),
                files: Vec::new(),
                severity: IssueSeverity::Info,
                line_numbers: Vec::new(),
            });
        }

        Ok(issues)
    }

    /// Run performance benchmarks
    async fn run_performance_analysis(&self) -> Result<Vec<Issue>> {
        let mut issues = Vec::with_capacity(10);

        // Check if there are benchmarks
        let benches_path = self.codebase_path.join("benches");
        if !benches_path.exists() {
            return Ok(issues);
        }
        
        let output = Command::new("cargo")
            .args(["bench", "--no-run"])
            .current_dir(&self.codebase_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to compile benchmarks")?;

        if !output.status.success() {
            issues.push(Issue {
                description: "Benchmark compilation failures detected".into(),
                files: Vec::new(),
                severity: IssueSeverity::Warning,
                line_numbers: Vec::new(),
            });
        }

        Ok(issues)
    }

    /// Generate a patch using RLM with context
    async fn generate_patch(&self, problem_statement: &str, _affected_files: &[String]) -> Result<String> {
        tracing::info!("Generating patch for: {}", problem_statement);
        counter!("dgm.patches_generated").increment(1);
        
        // This would integrate with the RLM module
        // For now, return a placeholder that will be replaced with actual RLM integration
        Ok(format!("// Auto-generated fix for: {}\n", problem_statement))
    }

    /// Evaluate proposals in parallel with resource limits
    async fn evaluate_proposals_parallel(
        &self,
        proposals: &[ImprovementProposal],
    ) -> Result<Vec<(ImprovementProposal, FitnessResult)>> {
        let mut results = Vec::with_capacity(proposals.len());

        // Process in batches to limit memory usage
        for chunk in proposals.chunks(self.config.parallel_evaluations) {
            // Check cancellation
            if self.cancellation_token.is_cancelled() {
                tracing::info!("Evaluation cancelled");
                break;
            }

            let mut futures = Vec::with_capacity(chunk.len());
            
            for proposal in chunk {
                let semaphore = self.evaluation_semaphore.clone();
                let proposal = proposal.clone();
                
                let future = async move {
                    let _permit = semaphore.acquire().await
                        .context("Failed to acquire evaluation permit")?;
                    
                    self.evaluate_proposal(&proposal).await
                };
                
                futures.push(future);
            }

            let chunk_results = futures::future::join_all(futures).await;

            for (proposal, result) in chunk.iter().cloned().zip(chunk_results.into_iter()) {
                match result {
                    Ok(fitness) => {
                        results.push((proposal, fitness));
                    }
                    Err(e) => {
                        tracing::error!("Failed to evaluate proposal: {}", e);
                        results.push((proposal, FitnessResult::failed()));
                    }
                }
            }
        }

        Ok(results)
    }

    /// Evaluate a single proposal with resource limits
    async fn evaluate_proposal(&self, proposal: &ImprovementProposal) -> Result<FitnessResult> {
        let start = Instant::now();

        // Create temporary copy of codebase with cleanup
        let temp_dir = tempfile::tempdir()
            .context("Failed to create temporary directory")?;
        let temp_path = temp_dir.path();

        // Copy codebase with timeout
        timeout(
            Duration::from_secs(30),
            self.copy_codebase(temp_path)
        ).await
        .map_err(|_| anyhow::anyhow!("Codebase copy timeout"))?
        .context("Failed to copy codebase")?;

        // Apply patch with timeout
        timeout(
            Duration::from_secs(10),
            self.apply_patch(temp_path, &proposal.patch)
        ).await
        .map_err(|_| anyhow::anyhow!("Patch application timeout"))?
        .context("Failed to apply patch")?;

        // Run evaluation with timeout
        let fitness = timeout(
            Duration::from_secs(self.config.attempt_timeout_secs),
            async {
                if self.config.sandbox_enabled {
                    self.evaluate_in_sandbox(temp_path).await
                } else {
                    self.evaluate_direct(temp_path).await
                }
            }
        ).await;

        let mut fitness = match fitness {
            Ok(Ok(f)) => f,
            Ok(Err(e)) => {
                tracing::warn!("Evaluation failed: {}", e);
                FitnessResult::failed()
            }
            Err(_) => {
                tracing::warn!("Evaluation timeout for proposal {}", proposal.id);
                let mut f = FitnessResult::failed();
                f.evaluation_duration_ms = self.config.attempt_timeout_secs * 1000;
                f
            }
        };

        fitness.evaluation_duration_ms = start.elapsed().as_millis() as u64;
        fitness.calculate_overall();

        tracing::debug!(
            "Evaluated proposal {} in {:?}: fitness={}",
            proposal.id,
            start.elapsed(),
            fitness.fitness
        );

        // temp_dir automatically cleaned up when dropped

        Ok(fitness)
    }

    /// Copy codebase to temporary directory
    async fn copy_codebase(&self, dest: &Path) -> Result<()> {
        // Use rsync if available, otherwise cp
        let output = Command::new("cp")
            .args(["-r", ".", dest.to_str().unwrap_or(".")])
            .current_dir(&self.codebase_path)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute cp command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to copy codebase: {}", stderr));
        }

        Ok(())
    }

    /// Apply a patch to the codebase
    async fn apply_patch(&self, codebase: &Path, patch: &str) -> Result<()> {
        // Write patch to file
        let patch_path = codebase.join("proposal.patch");
        tokio::fs::write(&patch_path, patch)
            .await
            .context("Failed to write patch file")?;

        // Apply patch
        let output = Command::new("patch")
            .args(["-p1", "-i", "proposal.patch", "--dry-run"])
            .current_dir(codebase)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute patch command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Patch dry-run failed: {}", stderr));
        }

        // Apply for real
        let output = Command::new("patch")
            .args(["-p1", "-i", "proposal.patch"])
            .current_dir(codebase)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to apply patch")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to apply patch: {}", stderr));
        }

        Ok(())
    }

    /// Evaluate proposal in sandbox (Docker)
    async fn evaluate_in_sandbox(&self, codebase: &Path) -> Result<FitnessResult> {
        let container_name = format!("dgm-eval-{}", uuid::Uuid::new_v4());

        // Build and run in Docker with resource limits
        let output = Command::new("docker")
            .args([
                "run",
                "--rm",
                "--name", &container_name,
                "-v", &format!("{}:/workspace", codebase.display()),
                "-w", "/workspace",
                "--memory", &format!("{}m", self.config.max_memory_mb),
                "--cpus", "1.0",
                "rust:latest",
                "cargo", "build", "--release"
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute Docker build")?;

        let compiled = output.status.success();

        if !compiled {
            return Ok(FitnessResult::failed());
        }

        // Run tests in sandbox
        let output = Command::new("docker")
            .args([
                "run",
                "--rm",
                "-v", &format!("{}:/workspace", codebase.display()),
                "-w", "/workspace",
                "--memory", &format!("{}m", self.config.max_memory_mb),
                "--cpus", "1.0",
                "rust:latest",
                "cargo", "test"
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute Docker test")?;

        let test_pass_rate = if output.status.success() { 1.0 } else { 0.0 };

        let fitness = if compiled && test_pass_rate > 0.0 {
            0.5 + (test_pass_rate * 0.5)
        } else {
            0.0
        };

        Ok(FitnessResult {
            fitness,
            compiled,
            test_pass_rate,
            performance_score: 0.5,
            memory_score: 0.5,
            energy_score: 0.5,
            security_score: 0.5,
            metrics: HashMap::with_capacity(10),
            evaluation_duration_ms: 0,
            memory_used_mb: 0,
        })
    }

    /// Evaluate proposal directly (no sandbox)
    async fn evaluate_direct(&self, codebase: &Path) -> Result<FitnessResult> {
        // Compile
        let output = Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(codebase)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to compile project")?;

        let compiled = output.status.success();

        if !compiled {
            return Ok(FitnessResult::failed());
        }

        // Run tests
        let output = Command::new("cargo")
            .args(["test"])
            .current_dir(codebase)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run tests")?;

        let test_pass_rate = if output.status.success() { 1.0 } else { 0.0 };

        let fitness = if test_pass_rate > 0.0 {
            0.5 + (test_pass_rate * 0.5)
        } else {
            0.0
        };

        Ok(FitnessResult {
            fitness,
            compiled,
            test_pass_rate,
            performance_score: 0.5,
            memory_score: 0.5,
            energy_score: 0.5,
            security_score: 0.5,
            metrics: HashMap::with_capacity(10),
            evaluation_duration_ms: 0,
            memory_used_mb: 0,
        })
    }

    /// Get current archive
    pub fn archive(&self) -> &[ArchiveEntry] {
        &self.archive
    }

    /// Get current generation
    pub fn generation(&self) -> u64 {
        self.current_generation
    }
    
    /// Get total evaluations
    pub fn total_evaluations(&self) -> u64 {
        self.total_evaluations
    }
    
    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_evaluations == 0 {
            0.0
        } else {
            self.successful_evaluations as f64 / self.total_evaluations as f64
        }
    }
    
    /// Request cancellation of ongoing operations
    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    /// Export archive to file
    pub fn export_archive(&self, path: impl AsRef<Path>) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.archive)
            .context("Failed to serialize archive")?;
        std::fs::write(path, json)
            .context("Failed to write archive file")?;
        Ok(())
    }

    /// Import archive from file
    pub fn import_archive(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let json = std::fs::read_to_string(path)
            .context("Failed to read archive file")?;
        self.archive = serde_json::from_str(&json)
            .context("Failed to deserialize archive")?;
        
        // Enforce size limit
        if self.archive.len() > MAX_ARCHIVE_SIZE {
            self.archive.truncate(MAX_ARCHIVE_SIZE);
        }
        
        Ok(())
    }
}

/// Parse clippy output line for file and line information
fn parse_clippy_output(line: &str) -> (Option<String>, Option<u32>, String) {
    // Simple parser - in production would use regex
    let parts: Vec<&str> = line.split(':').collect();
    
    if parts.len() >= 2 {
        let file = parts[0].trim().to_string();
        let line_num = parts[1].trim().parse().ok();
        let desc = parts.last().unwrap_or(&"").to_string();
        (Some(file), line_num, desc)
    } else {
        (None, None, line.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dgm_config_default() {
        let config = DgmConfig::default();
        assert_eq!(config.max_attempts, 10);
        assert!(config.sandbox_enabled);
        assert_eq!(config.archive_size, 100);
    }

    #[test]
    fn test_fitness_result_success() {
        let result = FitnessResult {
            fitness: 0.9,
            compiled: true,
            test_pass_rate: 0.95,
            performance_score: 0.8,
            memory_score: 0.8,
            energy_score: 0.8,
            security_score: 0.8,
            metrics: HashMap::new(),
            evaluation_duration_ms: 1000,
            memory_used_mb: 100,
        };
        assert!(result.is_success());
    }

    #[test]
    fn test_selection_strategy() {
        let strategies = vec![
            SelectionStrategy::Random,
            SelectionStrategy::FitnessProportional,
            SelectionStrategy::Tournament { size: 3 },
            SelectionStrategy::RankBased,
            SelectionStrategy::Elitist,
        ];
        
        for strategy in strategies {
            let engine = DgmEngine::new(
                DgmConfig::default(),
                "/tmp",
                strategy,
            );
            assert_eq!(engine.generation(), 0);
        }
    }
    
    #[test]
    fn test_improvement_proposal_validation() {
        let proposal = ImprovementProposal {
            id: "test-1".into(),
            parent_id: "initial".into(),
            problem_statement: "Test".into(),
            patch: "diff".into(),
            affected_files: vec!["test.rs".into()],
            timestamp: chrono::Utc::now(),
            generation: 1,
            hash: [0u8; 32],
        };
        
        assert!(proposal.validate_size().is_ok());
    }
    
    #[test]
    fn test_improvement_proposal_hash() {
        let mut proposal = ImprovementProposal {
            id: "test-1".into(),
            parent_id: "initial".into(),
            problem_statement: "Test".into(),
            patch: "diff".into(),
            affected_files: vec!["test.rs".into()],
            timestamp: chrono::Utc::now(),
            generation: 1,
            hash: [0u8; 32],
        };
        
        let hash = proposal.calculate_hash();
        assert_ne!(hash, [0u8; 32]);
    }
}

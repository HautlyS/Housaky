use crate::housaky::self_replication::genome::{BuildResult, SourceMutation};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use tracing::{info, warn};

pub struct SandboxCompiler {
    pub project_root: PathBuf,
    pub max_build_time_secs: u64,
    pub binary_size_regression_pct: f64,
}

impl SandboxCompiler {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            max_build_time_secs: 300,
            binary_size_regression_pct: 5.0,
        }
    }

    pub fn with_config(
        project_root: PathBuf,
        max_build_time_secs: u64,
        binary_size_regression_pct: f64,
    ) -> Self {
        Self {
            project_root,
            max_build_time_secs,
            binary_size_regression_pct,
        }
    }

    pub fn apply_mutations_to_worktree(
        &self,
        worktree_path: &Path,
        mutations: &[SourceMutation],
    ) -> Result<()> {
        for mutation in mutations {
            let target_file = worktree_path.join(&mutation.file);
            if let Some(parent) = target_file.parent() {
                std::fs::create_dir_all(parent)?;
            }

            if !mutation.diff.is_empty() {
                let patch_file = worktree_path.join(format!("{}.patch", uuid::Uuid::new_v4()));
                std::fs::write(&patch_file, &mutation.diff)?;

                let output = Command::new("patch")
                    .args(["-p1", "-i", patch_file.to_str().unwrap()])
                    .current_dir(worktree_path)
                    .output();

                let _ = std::fs::remove_file(&patch_file);

                match output {
                    Ok(o) if o.status.success() => {
                        info!("Applied mutation {:?} to {}", mutation.kind, mutation.file);
                    }
                    Ok(o) => {
                        warn!(
                            "Patch apply failed for {}: {}",
                            mutation.file,
                            String::from_utf8_lossy(&o.stderr)
                        );
                    }
                    Err(e) => {
                        warn!("Patch command error for {}: {}", mutation.file, e);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn build_release(&self, worktree_path: &Path) -> BuildResult {
        let start = Instant::now();

        let output = Command::new("cargo")
            .args(["build", "--release", "--bin", "housaky"])
            .current_dir(worktree_path)
            .env("CARGO_TERM_COLOR", "never")
            .output();

        let compile_time_secs = start.elapsed().as_secs_f64();

        match output {
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                let warnings: Vec<String> = stderr
                    .lines()
                    .filter(|l| l.contains("warning[") || l.starts_with("warning:"))
                    .map(|l| l.to_string())
                    .collect();
                let errors: Vec<String> = stderr
                    .lines()
                    .filter(|l| l.contains("error[") || l.starts_with("error:"))
                    .map(|l| l.to_string())
                    .collect();

                if o.status.success() {
                    let binary_path = worktree_path
                        .join("target")
                        .join("release")
                        .join("housaky");
                    let binary_size_bytes = std::fs::metadata(&binary_path)
                        .map(|m| m.len())
                        .unwrap_or(0);
                    let binary_hash = self.hash_file(&binary_path).unwrap_or_default();

                    info!(
                        "Build succeeded in {:.1}s, binary size: {} bytes",
                        compile_time_secs, binary_size_bytes
                    );

                    BuildResult {
                        success: true,
                        binary_path: Some(binary_path.to_string_lossy().to_string()),
                        binary_hash: Some(binary_hash),
                        binary_size_bytes,
                        compile_time_secs,
                        warnings,
                        errors: vec![],
                    }
                } else {
                    warn!("Build failed after {:.1}s", compile_time_secs);
                    BuildResult {
                        success: false,
                        binary_path: None,
                        binary_hash: None,
                        binary_size_bytes: 0,
                        compile_time_secs,
                        warnings,
                        errors,
                    }
                }
            }
            Err(e) => BuildResult {
                success: false,
                binary_path: None,
                binary_hash: None,
                binary_size_bytes: 0,
                compile_time_secs,
                warnings: vec![],
                errors: vec![e.to_string()],
            },
        }
    }

    pub fn check_size_regression(
        &self,
        baseline_size: u64,
        new_size: u64,
    ) -> bool {
        if baseline_size == 0 {
            return false;
        }
        let pct_change = ((new_size as f64 - baseline_size as f64) / baseline_size as f64) * 100.0;
        let regressed = pct_change > self.binary_size_regression_pct;
        if regressed {
            warn!(
                "Binary size regression: +{:.1}% ({}B → {}B), threshold {:.1}%",
                pct_change, baseline_size, new_size, self.binary_size_regression_pct
            );
        }
        regressed
    }

    pub fn get_current_binary_size(&self) -> u64 {
        let binary = self
            .project_root
            .join("target")
            .join("release")
            .join("housaky");
        std::fs::metadata(&binary).map(|m| m.len()).unwrap_or(0)
    }

    pub fn get_current_binary_hash(&self) -> String {
        let binary = self
            .project_root
            .join("target")
            .join("release")
            .join("housaky");
        self.hash_file(&binary).unwrap_or_default()
    }

    fn hash_file(&self, path: &Path) -> Result<String> {
        use sha2::{Digest, Sha256};
        let bytes = std::fs::read(path).context("Failed to read binary for hashing")?;
        let digest = Sha256::digest(&bytes);
        Ok(hex::encode(digest))
    }
}

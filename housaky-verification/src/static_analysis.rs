//! Static analysis
use anyhow::Result;

/// Run clippy
pub async fn run_clippy(project_path: &str) -> Result<ClippyResults> {
    let output = std::process::Command::new("cargo")
        .args([
            "clippy",
            "--manifest-path",
            &format!("{}/Cargo.toml", project_path),
        ])
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Parse warnings and errors
    let warnings = stderr.lines().filter(|l| l.contains("warning:")).count();

    let errors = stderr.lines().filter(|l| l.contains("error:")).count();

    Ok(ClippyResults {
        warnings,
        errors,
        output: stderr.to_string(),
    })
}

/// Clippy results
#[derive(Debug, Clone)]
pub struct ClippyResults {
    pub warnings: usize,
    pub errors: usize,
    pub output: String,
}

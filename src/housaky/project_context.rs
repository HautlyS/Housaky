use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    pub path: PathBuf,
    pub relative_path: String,
    pub file_type: ProjectFileType,
    pub size_bytes: u64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectFileType {
    Source,
    Config,
    Documentation,
    Memory,
    Agent,
    Tool,
    Test,
    Build,
    Other,
}

impl ProjectFileType {
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "rs" => ProjectFileType::Source,
            "toml" | "yaml" | "yml" | "json" => ProjectFileType::Config,
            "sqlite" | "db" => ProjectFileType::Memory,
            "md" if ext.contains("AGENT") => ProjectFileType::Agent,
            "md" | "txt" => ProjectFileType::Documentation,
            "py" | "js" | "sh" => ProjectFileType::Tool,
            "test" | "tests" => ProjectFileType::Test,
            "lock" | "dockerfile" | "build" => ProjectFileType::Build,
            _ => ProjectFileType::Other,
        }
    }
}

pub struct ProjectScanner {
    project_root: PathBuf,
    ignored_dirs: Vec<&'static str>,
}

impl ProjectScanner {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            ignored_dirs: vec![
                "target",
                "node_modules",
                ".git",
                "vendor",
                "dist",
                "build",
                ".next",
                "__pycache__",
                ".venv",
                "logs",
                "memory",
            ],
        }
    }

    pub async fn scan(&self) -> Result<Vec<ProjectFile>> {
        let mut files = Vec::new();
        let mut dirs_to_scan = vec![self.project_root.clone()];

        while let Some(dir) = dirs_to_scan.pop() {
            if !dir.is_dir() {
                continue;
            }

            let dir_name = dir.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            if self.ignored_dirs.contains(&dir_name) {
                continue;
            }

            let mut entries = fs::read_dir(&dir).await?;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let metadata = entry.metadata().await?;

                if metadata.is_dir() {
                    dirs_to_scan.push(path);
                } else if metadata.is_file() {
                    if let Some(file_type) = self.classify_file(&path) {
                        let relative = path.strip_prefix(&self.project_root)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .to_string();

                        let description = self.get_file_description(&path);

                        files.push(ProjectFile {
                            path: path.clone(),
                            relative_path: relative,
                            file_type,
                            size_bytes: metadata.len(),
                            description,
                        });
                    }
                }
            }
        }

        info!("Scanned {} project files from {:?}", files.len(), self.project_root);
        Ok(files)
    }

    fn classify_file(&self, path: &Path) -> Option<ProjectFileType> {
        let extension = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if filename.contains("AGENT") || filename.contains("SOUL") || filename.contains("TOOL") {
            return Some(ProjectFileType::Agent);
        }

        if filename == "Cargo.toml" || filename == "package.json" {
            return Some(ProjectFileType::Config);
        }

        Some(ProjectFileType::from_extension(extension))
    }

    fn get_file_description(&self, path: &Path) -> String {
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        match filename {
            "AGENTS.md" => "Core agent identity and behavior definitions".to_string(),
            "SOUL.md" => "Agent values and ethics configuration".to_string(),
            "Cargo.toml" => "Rust project dependencies and configuration".to_string(),
            "config.toml" => "Housaky runtime configuration".to_string(),
            "memory.rs" | "memory/" => "Memory system implementation".to_string(),
            "kowalski_integration.rs" => "Kowalski subagent integration".to_string(),
            _ => format!("Project file: {}", filename),
        }
    }

    pub async fn get_project_summary(&self) -> Result<ProjectSummary> {
        let files = self.scan().await?;

        let source_files = files.iter().filter(|f| f.file_type == ProjectFileType::Source).count();
        let config_files = files.iter().filter(|f| f.file_type == ProjectFileType::Config).count();
        let doc_files = files.iter().filter(|f| f.file_type == ProjectFileType::Documentation).count();
        let agent_files = files.iter().filter(|f| f.file_type == ProjectFileType::Agent).count();

        let total_size: u64 = files.iter().map(|f| f.size_bytes).sum();

        Ok(ProjectSummary {
            project_root: self.project_root.clone(),
            total_files: files.len(),
            source_files,
            config_files,
            doc_files,
            agent_files,
            total_size_bytes: total_size,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub project_root: PathBuf,
    pub total_files: usize,
    pub source_files: usize,
    pub config_files: usize,
    pub doc_files: usize,
    pub agent_files: usize,
    pub total_size_bytes: u64,
}

impl ProjectSummary {
    pub fn to_memory_format(&self) -> String {
        format!(
            r#"# Housaky Project Summary

## Project Location
{}

## File Statistics
- Total Files: {}
- Source Files (Rust): {}
- Configuration Files: {}
- Documentation Files: {}
- Agent Definition Files: {}
- Total Size: {:.2} KB

## Key Files
- AGENTS.md: Agent identity and behavior
- SOUL.md: Values and ethics
- config.toml: Runtime configuration
- src/memory/: Memory system
- src/housaky/kowalski_integration.rs: Subagent integration
"#,
            self.project_root.display(),
            self.total_files,
            self.source_files,
            self.config_files,
            self.doc_files,
            self.agent_files,
            self.total_size_bytes as f64 / 1024.0
        )
    }
}

pub async fn get_project_context() -> Result<ProjectSummary> {
    let project_root = PathBuf::from("/home/ubuntu/Housaky");
    
    if !project_root.exists() {
        return Err(anyhow::anyhow!("Project root not found: {:?}", project_root));
    }

    let scanner = ProjectScanner::new(project_root);
    scanner.get_project_summary().await
}

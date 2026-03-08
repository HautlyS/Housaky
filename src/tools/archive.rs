use super::traits::{Tool, ToolResult};
use crate::security::SecurityPolicy;
use crate::util::expand_path;
use async_trait::async_trait;
use serde_json::json;
use std::io::Write;
use std::sync::Arc;

pub struct ArchiveTool {
    security: Arc<SecurityPolicy>,
}

impl ArchiveTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }
}

#[async_trait]
impl Tool for ArchiveTool {
    fn name(&self) -> &str {
        "archive"
    }

    fn description(&self) -> &str {
        "Create or extract archives (zip, tar, tar.gz). Supports multiple formats for packaging and unpackaging files."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create", "extract"],
                    "description": "Action to perform: 'create' or 'extract'"
                },
                "archive_path": {
                    "type": "string",
                    "description": "Path to the archive file"
                },
                "source_path": {
                    "type": "string",
                    "description": "For create: path to source file/directory. For extract: destination directory"
                },
                "format": {
                    "type": "string",
                    "enum": ["zip", "tar", "tar.gz"],
                    "description": "Archive format (default: zip)",
                    "default": "zip"
                },
                "compression_level": {
                    "type": "integer",
                    "description": "Compression level 0-9 (default: 6)",
                    "default": 6,
                    "minimum": 0,
                    "maximum": 9
                }
            },
            "required": ["action", "archive_path", "source_path"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'action' parameter"))?;

        let archive_path = args
            .get("archive_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'archive_path' parameter"))?;

        let source_path = args
            .get("source_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'source_path' parameter"))?;

        let format = args.get("format").and_then(|v| v.as_str()).unwrap_or("zip");

        if !self.security.can_act() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        let archive = expand_path(archive_path);
        let source = expand_path(source_path);

        if !self.security.is_path_allowed(archive.to_str().unwrap_or(""))
            || !self.security.is_path_allowed(source.to_str().unwrap_or(""))
        {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Path outside allowed workspace".into()),
            });
        }

        match action {
            "create" => self.create_archive(&archive, &source, format).await,
            "extract" => self.extract_archive(&archive, &source).await,
            _ => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown action: {}", action)),
            }),
        }
    }
}

impl ArchiveTool {
    async fn create_archive(
        &self,
        archive_path: &std::path::Path,
        source_path: &std::path::Path,
        format: &str,
    ) -> anyhow::Result<ToolResult> {
        if !source_path.exists() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Source '{}' does not exist", source_path.display())),
            });
        }

        let mut files_added = 0u64;
        let mut total_size = 0u64;

        match format {
            "zip" => {
                let file = std::fs::File::create(archive_path)?;
                let mut zip = zip::ZipWriter::new(file);
                let options = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated);

                if source_path.is_file() {
                    let name = source_path.file_name().unwrap().to_string_lossy();
                    zip.start_file(name.as_ref(), options)?;
                    let content = std::fs::read(source_path)?;
                    zip.write_all(&content)?;
                    files_added = 1;
                    total_size = content.len() as u64;
                } else {
                    let (f, s) = self.add_dir_to_zip_sync(&mut zip, source_path, source_path, options)?;
                    files_added = f;
                    total_size = s;
                }
                zip.finish()?;
            }
            "tar" | "tar.gz" => {
                let output = std::process::Command::new("tar")
                    .arg("-cf")
                    .arg(archive_path)
                    .arg("-C")
                    .arg(source_path.parent().unwrap_or(source_path))
                    .arg(source_path.file_name().unwrap())
                    .output()?;

                if !output.status.success() {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                    });
                }

                if let Ok(metadata) = std::fs::metadata(archive_path) {
                    total_size = metadata.len();
                }
                files_added = 1;
            }
            _ => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Unsupported format: {}", format)),
                });
            }
        }

        Ok(ToolResult {
            success: true,
            output: format!(
                "Created {} archive '{}' with {} file(s), {} bytes",
                format,
                archive_path.display(),
                files_added,
                total_size
            ),
            error: None,
        })
    }

    fn add_dir_to_zip_sync<W: std::io::Write + std::io::Seek>(
        &self,
        zip: &mut zip::ZipWriter<W>,
        base: &std::path::Path,
        current: &std::path::Path,
        options: zip::write::SimpleFileOptions,
    ) -> anyhow::Result<(u64, u64)> {
        let mut files = 0u64;
        let mut size = 0u64;

        for entry in std::fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path.strip_prefix(base)?;
            let name = relative.to_string_lossy().replace('\\', "/");

            if path.is_dir() {
                zip.add_directory(&format!("{}/", name), options)?;
                let (f, s) = self.add_dir_to_zip_sync(zip, base, &path, options)?;
                files += f;
                size += s;
            } else {
                zip.start_file(&name, options)?;
                let content = std::fs::read(&path)?;
                zip.write_all(&content)?;
                files += 1;
                size += content.len() as u64;
            }
        }

        Ok((files, size))
    }

    fn extract_zip_sync(
        &self,
        archive_path: &std::path::Path,
        dest_path: &std::path::Path,
    ) -> anyhow::Result<u64> {
        let file = std::fs::File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut files_extracted = 0u64;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = dest_path.join(file.mangled_name());

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    std::fs::create_dir_all(p)?;
                }
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
                files_extracted += 1;
            }
        }

        Ok(files_extracted)
    }

    async fn extract_archive(
        &self,
        archive_path: &std::path::Path,
        dest_path: &std::path::Path,
    ) -> anyhow::Result<ToolResult> {
        if !archive_path.exists() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Archive '{}' does not exist", archive_path.display())),
            });
        }

        std::fs::create_dir_all(dest_path)?;

        let mut files_extracted = 0u64;

        let extension = archive_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match extension {
            "zip" => {
                files_extracted = self.extract_zip_sync(archive_path, dest_path)?;
            }
            "gz" | "tgz" => {
                let output = std::process::Command::new("tar")
                    .arg("-xzf")
                    .arg(archive_path)
                    .arg("-C")
                    .arg(dest_path)
                    .output()?;

                if !output.status.success() {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                    });
                }
                files_extracted = 1;
            }
            "tar" => {
                let output = std::process::Command::new("tar")
                    .arg("-xf")
                    .arg(archive_path)
                    .arg("-C")
                    .arg(dest_path)
                    .output()?;

                if !output.status.success() {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                    });
                }
                files_extracted = 1;
            }
            _ => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Unsupported archive format: {}", extension)),
                });
            }
        }

        Ok(ToolResult {
            success: true,
            output: format!(
                "Extracted {} file(s) from '{}' to '{}'",
                files_extracted,
                archive_path.display(),
                dest_path.display()
            ),
            error: None,
        })
    }
}

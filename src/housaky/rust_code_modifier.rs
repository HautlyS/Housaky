use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use syn::{parse_file, Item, ItemEnum, ItemFn, ItemImpl, ItemStruct};

use crate::housaky::code_parsing::tree_sitter as ts;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedModule {
    pub path: PathBuf,
    pub functions: Vec<ParsedFunction>,
    pub structs: Vec<ParsedStruct>,
    pub enums: Vec<ParsedEnum>,
    pub impls: Vec<ParsedImpl>,
    pub raw_ast: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFunction {
    pub name: String,
    pub sig: String,
    pub body_preview: String,
    #[serde(default)]
    pub line_start: usize,
    #[serde(default)]
    pub line_end: usize,
    pub visibility: String,
    pub is_async: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedStruct {
    pub name: String,
    pub fields: Vec<String>,
    #[serde(default)]
    pub line_start: usize,
    #[serde(default)]
    pub line_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedEnum {
    pub name: String,
    pub variants: Vec<String>,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedImpl {
    pub trait_name: Option<String>,
    pub self_type: String,
    pub methods: Vec<String>,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeModification {
    pub id: String,
    pub target_file: PathBuf,
    pub target_type: ModificationTarget,
    pub target_name: String,
    pub modification_kind: ModificationKind,
    pub old_code: String,
    pub new_code: String,

    /// Preferred edit anchoring.
    ///
    /// - If `byte_start/byte_end` are present, modifications are applied using byte ranges (safer).
    /// - Otherwise, fall back to line-based edits using `line_start/line_end`.
    ///
    /// Byte ranges are UTF-8 byte offsets into the file content.
    #[serde(default)]
    pub byte_start: Option<usize>,
    #[serde(default)]
    pub byte_end: Option<usize>,

    #[serde(default)]
    pub line_start: usize,
    #[serde(default)]
    pub line_end: usize,

    /// If true, attempt to resolve a structural anchor via tree-sitter (Rust grammar) using
    /// `target_type/target_name` when byte offsets are not provided.
    #[serde(default)]
    pub use_tree_sitter_anchor: bool,

    pub confidence: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModificationTarget {
    Function,
    Struct,
    Enum,
    Impl,
    Module,
    LineRange,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModificationKind {
    Replace,
    InsertBefore,
    InsertAfter,
    Remove,
    Wrap,
    Rename,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModificationResult {
    pub modification_id: String,
    pub success: bool,
    pub compiled: bool,
    pub tests_passed: bool,
    pub error: Option<String>,
    pub warnings: Vec<String>,
}

pub struct RustCodeParser {
    project_root: PathBuf,
}

fn find_item_line_range(
    ts_tree: Option<&tree_sitter::Tree>,
    content: &str,
    name: &str,
) -> Option<(usize, usize)> {
    let tree = ts_tree?;
    let root = tree.root_node();

    // Heuristic: find an identifier node whose text equals `name`, then climb to a
    // likely containing item node (function_item/struct_item/enum_item/impl_item).
    // This avoids adding `tree-sitter` query strings at this stage.
    let mut cursor = root.walk();
    let mut stack: Vec<tree_sitter::Node> = vec![root];

    while let Some(node) = stack.pop() {
        if node.is_named() {
            if node.kind() == "identifier" {
                if let Ok(text) = node.utf8_text(content.as_bytes()) {
                    if text == name {
                        let mut parent = node.parent();
                        while let Some(p) = parent {
                            match p.kind() {
                                "function_item" | "struct_item" | "enum_item" | "impl_item" => {
                                    let start = p.start_byte();
                                    let end = p.end_byte();
                                    let (ls, _) = ts::byte_to_line_col(content, start);
                                    let (le, _) = ts::byte_to_line_col(content, end);
                                    return Some((ls, le));
                                }
                                _ => {
                                    parent = p.parent();
                                }
                            }
                        }
                    }
                }
            }
        }

        // Push children for DFS.
        cursor.reset(node);
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }

    None
}

impl RustCodeParser {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    pub fn parse_file(&self, path: &Path) -> Result<ParsedModule> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        let syntax_tree = parse_file(&content)
            .with_context(|| format!("Failed to parse Rust code in: {}", path.display()))?;

        // Best-effort: compute line ranges via tree-sitter when the Rust grammar is enabled.
        // This gives stable anchors for edits and more useful metadata for the self-improvement loop.
        let mut ts_engine = match ts::rust_language() {
            Ok(lang) => ts::TsEngine::new(lang).ok(),
            Err(_) => None,
        };
        let ts_tree = ts_engine
            .as_mut()
            .and_then(|eng| eng.parse_str(&content).ok());

        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut impls = Vec::new();

        for item in &syntax_tree.items {
            match item {
                Item::Fn(item_fn) => {
                    functions.push(self.parse_function(item_fn, &content, ts_tree.as_ref()));
                }
                Item::Struct(item_struct) => {
                    structs.push(self.parse_struct(item_struct, &content, ts_tree.as_ref()));
                }
                Item::Enum(item_enum) => {
                    enums.push(self.parse_enum(item_enum, &content, ts_tree.as_ref()));
                }
                Item::Impl(item_impl) => {
                    impls.push(self.parse_impl(item_impl, &content, ts_tree.as_ref()));
                }
                _ => {}
            }
        }

        Ok(ParsedModule {
            path: path.to_path_buf(),
            functions,
            structs,
            enums,
            impls,
            raw_ast: content,
        })
    }

    fn parse_function(
        &self,
        item_fn: &ItemFn,
        content: &str,
        ts_tree: Option<&tree_sitter::Tree>,
    ) -> ParsedFunction {
        let name = item_fn.sig.ident.to_string();
        let is_async = item_fn.sig.asyncness.is_some();
        let visibility = match &item_fn.vis {
            syn::Visibility::Public(_) => "pub".to_string(),
            syn::Visibility::Inherited => "".to_string(),
            _ => "pub".to_string(),
        };

        let sig = format!("{:?}", item_fn.sig);

        let body_preview = {
            let stmt_count = item_fn.block.stmts.len();
            format!("{} statements", stmt_count)
        };

        let (line_start, line_end) = find_item_line_range(ts_tree, content, &name)
            .unwrap_or((0, 0));

        ParsedFunction {
            name,
            sig,
            body_preview,
            line_start,
            line_end,
            visibility,
            is_async,
        }
    }

    fn parse_struct(
        &self,
        item_struct: &ItemStruct,
        content: &str,
        ts_tree: Option<&tree_sitter::Tree>,
    ) -> ParsedStruct {
        let name = item_struct.ident.to_string();
        let fields = match &item_struct.fields {
            syn::Fields::Named(named) => named
                .named
                .iter()
                .map(|f| {
                    let field_name = f.ident.as_ref().map(|i| i.to_string()).unwrap_or_default();
                    let field_type = format!("{:?}", f.ty);
                    format!("{}: {}", field_name, field_type)
                })
                .collect(),
            syn::Fields::Unnamed(_) => vec!["..".to_string()],
            syn::Fields::Unit => vec![],
        };

        let (line_start, line_end) = find_item_line_range(ts_tree, content, &name)
            .unwrap_or((0, 0));

        ParsedStruct {
            name,
            fields,
            line_start,
            line_end,
        }
    }

    fn parse_enum(
        &self,
        item_enum: &ItemEnum,
        content: &str,
        ts_tree: Option<&tree_sitter::Tree>,
    ) -> ParsedEnum {
        let name = item_enum.ident.to_string();
        let variants = item_enum
            .variants
            .iter()
            .map(|v| {
                let discriminant = v.discriminant.as_ref().map(|(_, e)| format!(" = {:?}", e));
                let fields = match &v.fields {
                    syn::Fields::Named(named) => format!(
                        " {{ {} }}",
                        named
                            .named
                            .iter()
                            .map(|f| f.ident.as_ref().unwrap().to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    syn::Fields::Unnamed(unnamed) => format!(
                        "({})",
                        (0..unnamed.unnamed.len())
                            .map(|_| "_")
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    syn::Fields::Unit => "".to_string(),
                };
                format!("{}{}{}", v.ident, fields, discriminant.unwrap_or_default())
            })
            .collect();

        let (line_start, line_end) = find_item_line_range(ts_tree, content, &name)
            .unwrap_or((0, 0));

        ParsedEnum {
            name,
            variants,
            line_start,
            line_end,
        }
    }

    fn parse_impl(
        &self,
        item_impl: &ItemImpl,
        content: &str,
        ts_tree: Option<&tree_sitter::Tree>,
    ) -> ParsedImpl {
        let self_type = format!("{:?}", item_impl.self_ty);
        let trait_name = item_impl.trait_.as_ref().map(|_| "trait".to_string());

        let methods = item_impl
            .items
            .iter()
            .filter_map(|item| {
                if let syn::ImplItem::Fn(method) = item {
                    Some(method.sig.ident.to_string())
                } else {
                    None
                }
            })
            .collect();

        let impl_name = self_type.clone();
        let (line_start, line_end) = find_item_line_range(ts_tree, content, &impl_name)
            .unwrap_or((0, 0));

        ParsedImpl {
            trait_name,
            self_type,
            methods,
            line_start,
            line_end,
        }
    }

    pub fn parse_directory(&self, dir: &Path) -> Result<Vec<ParsedModule>> {
        let mut modules = Vec::new();

        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
        {
            let path = entry.path();
            if let Ok(module) = self.parse_file(path) {
                modules.push(module);
            }
        }

        Ok(modules)
    }

    pub fn find_function(
        &self,
        path: &Path,
        function_name: &str,
    ) -> Result<Option<ParsedFunction>> {
        let module = self.parse_file(path)?;
        Ok(module
            .functions
            .into_iter()
            .find(|f| f.name == function_name))
    }
}

pub struct RustCodeModifier {
    parser: RustCodeParser,
    backup_dir: PathBuf,
    project_root: PathBuf,
}

impl RustCodeModifier {
    pub fn new(project_root: PathBuf) -> Self {
        let backup_dir = project_root.join(".housaky").join("backups");
        std::fs::create_dir_all(&backup_dir).ok();

        Self {
            parser: RustCodeParser::new(project_root.clone()),
            backup_dir,
            project_root,
        }
    }

    /// Validates that the target path is within the project root directory.
    /// This prevents path traversal attacks that could write to arbitrary locations.
    fn validate_path_within_project(&self, target_path: &Path) -> Result<PathBuf> {
        // Canonicalize both paths to resolve symlinks and relative components
        let canonical_root = self
            .project_root
            .canonicalize()
            .with_context(|| format!("Failed to canonicalize project root: {}", self.project_root.display()))?;

        // For the target, we need to handle the case where it doesn't exist yet
        let canonical_target = if target_path.exists() {
            target_path.canonicalize()
                .with_context(|| format!("Failed to canonicalize target path: {}", target_path.display()))?
        } else {
            // If the file doesn't exist, canonicalize the parent and append the filename
            let parent = target_path.parent()
                .ok_or_else(|| anyhow::anyhow!("Target path has no parent: {}", target_path.display()))?;
            let filename = target_path.file_name()
                .ok_or_else(|| anyhow::anyhow!("Target path has no filename: {}", target_path.display()))?;
            
            let canonical_parent = parent.canonicalize()
                .with_context(|| format!("Failed to canonicalize parent directory: {}", parent.display()))?;
            
            canonical_parent.join(filename)
        };

        // Check that the target is within the project root
        if !canonical_target.starts_with(&canonical_root) {
            anyhow::bail!(
                "Security violation: target path '{}' is outside project root '{}'. \
                Self-modification is only allowed within the project directory.",
                target_path.display(),
                self.project_root.display()
            );
        }

        // Additional check: don't allow modifications to sensitive files
        let filename = canonical_target
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("");
        
        let sensitive_patterns = [
            ".git",
            ".env",
            "secrets",
            "credentials",
            "private_key",
            ".ssh",
        ];
        
        for pattern in &sensitive_patterns {
            if filename.contains(pattern) || canonical_target.to_string_lossy().contains(&format!("/{}/", pattern)) {
                anyhow::bail!(
                    "Security violation: cannot modify sensitive file or directory containing '{}': {}",
                    pattern,
                    target_path.display()
                );
            }
        }

        Ok(canonical_target)
    }

    pub fn apply_modification(
        &self,
        modification: &CodeModification,
    ) -> Result<ModificationResult> {
        let id = &modification.id;
        let target_path = &modification.target_file;

        // Security: validate that the target path is within the project root
        let validated_path = self.validate_path_within_project(target_path)?;

        let original_content = std::fs::read_to_string(&validated_path)
            .with_context(|| format!("Failed to read: {}", validated_path.display()))?;

        let backup_path = self.backup_dir.join(format!(
            "{}_{}_{}.rs.backup",
            validated_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
            id,
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ));

        std::fs::write(&backup_path, &original_content)?;

        let modified_content = self.apply_change(&original_content, modification)?;

        std::fs::write(&validated_path, &modified_content)?;

        let compile_result = self.compile_check(&validated_path);

        let test_result = if compile_result.success {
            self.run_tests(&validated_path)
        } else {
            false
        };

        if !compile_result.success || !test_result {
            std::fs::write(&validated_path, &original_content)?;
            std::fs::remove_file(&backup_path).ok();

            return Ok(ModificationResult {
                modification_id: id.clone(),
                success: false,
                compiled: false,
                tests_passed: false,
                error: compile_result.error,
                warnings: compile_result.warnings,
            });
        }

        Ok(ModificationResult {
            modification_id: id.clone(),
            success: true,
            compiled: true,
            tests_passed: test_result,
            error: None,
            warnings: compile_result.warnings,
        })
    }

    fn apply_change(&self, content: &str, modification: &CodeModification) -> Result<String> {
        // 1) Prefer byte-range edits if provided (most robust).
        // 2) Otherwise, if requested, try to resolve an anchor via tree-sitter.
        // 3) Otherwise, fall back to legacy line-based edits.

        if let Some(updated) = self.apply_change_by_bytes(content, modification)? {
            return Ok(updated);
        }

        // Legacy line-based fallback.
        let lines: Vec<&str> = content.lines().collect();

        match modification.modification_kind {
            ModificationKind::Replace => {
                let mut new_lines = lines[..modification.line_start.saturating_sub(1)].to_vec();
                new_lines.push(&modification.new_code);
                new_lines.extend_from_slice(&lines[modification.line_end..]);
                Ok(new_lines.join("\n"))
            }
            ModificationKind::InsertBefore => {
                let mut new_lines = lines[..modification.line_start.saturating_sub(1)].to_vec();
                new_lines.push(&modification.new_code);
                new_lines.extend_from_slice(&lines[modification.line_start.saturating_sub(1)..]);
                Ok(new_lines.join("\n"))
            }
            ModificationKind::InsertAfter => {
                let mut new_lines = lines[..modification.line_end].to_vec();
                new_lines.push(&modification.new_code);
                new_lines.extend_from_slice(&lines[modification.line_end..]);
                Ok(new_lines.join("\n"))
            }
            ModificationKind::Remove => {
                let mut new_lines = lines[..modification.line_start.saturating_sub(1)].to_vec();
                new_lines.extend_from_slice(&lines[modification.line_end..]);
                Ok(new_lines.join("\n"))
            }
            ModificationKind::Rename => Ok(content.replace(&modification.old_code, &modification.new_code)),
            ModificationKind::Wrap => {
                let before = lines[..modification.line_start.saturating_sub(1)].join("\n");
                let wrapped = modification.new_code.replace(
                    "{OLD_CODE}",
                    &lines[modification.line_start.saturating_sub(1)..modification.line_end]
                        .join("\n"),
                );
                let after = lines[modification.line_end..].join("\n");
                Ok(format!("{}\n{}\n{}", before, wrapped, after))
            }
        }
    }

    fn apply_change_by_bytes(
        &self,
        content: &str,
        modification: &CodeModification,
    ) -> Result<Option<String>> {
        // If explicit bytes are given, apply directly.
        if let (Some(start), Some(end)) = (modification.byte_start, modification.byte_end) {
            return Ok(Some(apply_byte_edit(content, modification, start, end)?));
        }

        // If requested, attempt structural anchoring via tree-sitter (Rust grammar).
        if modification.use_tree_sitter_anchor {
            let anchor = resolve_ts_anchor_for_modification(content, modification)?;
            if let Some((start, end)) = anchor {
                return Ok(Some(apply_byte_edit(content, modification, start, end)?));
            }
        }

        Ok(None)
    }

    fn compile_check(&self, _path: &Path) -> CompileResult {
        let output = Command::new("cargo")
            .args(["check", "--lib", "-p", "housaky"])
            .current_dir(&self.parser.project_root)
            .output();

        match output {
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                CompileResult {
                    success: output.status.success(),
                    error: if !output.status.success() {
                        Some(stderr.to_string())
                    } else {
                        None
                    },
                    warnings: stderr
                        .lines()
                        .filter(|l| l.contains("warning"))
                        .map(|s| s.to_string())
                        .collect(),
                }
            }
            Err(e) => CompileResult {
                success: false,
                error: Some(format!("Failed to run cargo check: {}", e)),
                warnings: vec![],
            },
        }
    }

    fn run_tests(&self, _path: &Path) -> bool {
        let output = Command::new("cargo")
            .args(["test", "--lib", "--", "--test-threads=1"])
            .current_dir(&self.parser.project_root)
            .output();

        output.map(|o| o.status.success()).unwrap_or(false)
    }

    pub fn rollback(&self, backup_path: &Path, target_path: &Path) -> Result<()> {
        let backup_content = std::fs::read_to_string(backup_path)?;
        std::fs::write(target_path, backup_content)?;
        Ok(())
    }

    pub fn list_backups(&self) -> Result<Vec<PathBuf>> {
        let mut backups = Vec::new();
        for entry in std::fs::read_dir(&self.backup_dir)? {
            let entry = entry?;
            if entry
                .path()
                .extension()
                .map(|e| e == "backup")
                .unwrap_or(false)
            {
                backups.push(entry.path());
            }
        }
        Ok(backups)
    }
}

#[derive(Debug, Clone)]
struct CompileResult {
    success: bool,
    error: Option<String>,
    warnings: Vec<String>,
}

use walkdir;

fn apply_byte_edit(
    content: &str,
    modification: &CodeModification,
    start: usize,
    end: usize,
) -> Result<String> {
    if start > end || end > content.len() {
        anyhow::bail!("Invalid byte range: {}..{} (len={})", start, end, content.len());
    }

    match modification.modification_kind {
        ModificationKind::Replace => {
            Ok(format!("{}{}{}", &content[..start], modification.new_code, &content[end..]))
        }
        ModificationKind::InsertBefore => {
            Ok(format!("{}{}{}", &content[..start], modification.new_code, &content[start..]))
        }
        ModificationKind::InsertAfter => {
            Ok(format!("{}{}{}", &content[..end], modification.new_code, &content[end..]))
        }
        ModificationKind::Remove => Ok(format!("{}{}", &content[..start], &content[end..])),
        ModificationKind::Rename => Ok(content.replace(&modification.old_code, &modification.new_code)),
        ModificationKind::Wrap => {
            let old = &content[start..end];
            let wrapped = modification.new_code.replace("{OLD_CODE}", old);
            Ok(format!("{}{}{}", &content[..start], wrapped, &content[end..]))
        }
    }
}

fn resolve_ts_anchor_for_modification(
    content: &str,
    modification: &CodeModification,
) -> Result<Option<(usize, usize)>> {
    // Only Rust is supported for now; other languages can be added by adding grammar crates + resolvers.
    let language = match ts::rust_language() {
        Ok(l) => l,
        Err(_) => return Ok(None),
    };

    let mut engine = ts::TsEngine::new(language)?;
    let tree = engine.parse_str(content)?;

    let target_name = modification.target_name.trim();
    if target_name.is_empty() {
        return Ok(None);
    }

    // For Function/Struct/Enum: find identifier == target_name and climb.
    // For Impl: also try to match identifier, but this may not always work for complex self types.
    // For Module/LineRange: unsupported here.
    let root = tree.root_node();
    let mut cursor = root.walk();
    let mut stack: Vec<tree_sitter::Node> = vec![root];

    while let Some(node) = stack.pop() {
        if node.is_named() && node.kind() == "identifier" {
            if let Ok(text) = node.utf8_text(content.as_bytes()) {
                if text == target_name {
                    let mut parent = node.parent();
                    while let Some(p) = parent {
                        match p.kind() {
                            "function_item" | "struct_item" | "enum_item" | "impl_item" => {
                                return Ok(Some((p.start_byte(), p.end_byte())));
                            }
                            _ => parent = p.parent(),
                        }
                    }
                }
            }
        }

        cursor.reset(node);
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }

    Ok(None)
}

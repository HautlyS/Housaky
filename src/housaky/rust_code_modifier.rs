use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use syn::{parse_file, Item, ItemEnum, ItemFn, ItemImpl, ItemStruct};

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
    pub line_start: usize,
    pub line_end: usize,
    pub visibility: String,
    pub is_async: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedStruct {
    pub name: String,
    pub fields: Vec<String>,
    pub line_start: usize,
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
    pub line_start: usize,
    pub line_end: usize,
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

impl RustCodeParser {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    pub fn parse_file(&self, path: &Path) -> Result<ParsedModule> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        let syntax_tree = parse_file(&content)
            .with_context(|| format!("Failed to parse Rust code in: {}", path.display()))?;

        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut impls = Vec::new();

        for item in &syntax_tree.items {
            match item {
                Item::Fn(item_fn) => {
                    functions.push(self.parse_function(item_fn, &content));
                }
                Item::Struct(item_struct) => {
                    structs.push(self.parse_struct(item_struct));
                }
                Item::Enum(item_enum) => {
                    enums.push(self.parse_enum(item_enum));
                }
                Item::Impl(item_impl) => {
                    impls.push(self.parse_impl(item_impl));
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

    fn parse_function(&self, item_fn: &ItemFn, _content: &str) -> ParsedFunction {
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

        ParsedFunction {
            name,
            sig,
            body_preview,
            line_start: 0,
            line_end: 0,
            visibility,
            is_async,
        }
    }

    fn parse_struct(&self, item_struct: &ItemStruct) -> ParsedStruct {
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

        ParsedStruct {
            name,
            fields,
            line_start: 0,
            line_end: 0,
        }
    }

    fn parse_enum(&self, item_enum: &ItemEnum) -> ParsedEnum {
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

        ParsedEnum {
            name,
            variants,
            line_start: 0,
            line_end: 0,
        }
    }

    fn parse_impl(&self, item_impl: &ItemImpl) -> ParsedImpl {
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

        ParsedImpl {
            trait_name,
            self_type,
            methods,
            line_start: 0,
            line_end: 0,
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
}

impl RustCodeModifier {
    pub fn new(project_root: PathBuf) -> Self {
        let backup_dir = project_root.join(".housaky").join("backups");
        std::fs::create_dir_all(&backup_dir).ok();

        Self {
            parser: RustCodeParser::new(project_root),
            backup_dir,
        }
    }

    pub fn apply_modification(
        &self,
        modification: &CodeModification,
    ) -> Result<ModificationResult> {
        let id = &modification.id;
        let target_path = &modification.target_file;

        let original_content = std::fs::read_to_string(target_path)
            .with_context(|| format!("Failed to read: {}", target_path.display()))?;

        let backup_path = self.backup_dir.join(format!(
            "{}_{}_{}.rs.backup",
            target_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
            id,
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ));

        std::fs::write(&backup_path, &original_content)?;

        let modified_content = self.apply_change(&original_content, modification)?;

        std::fs::write(target_path, &modified_content)?;

        let compile_result = self.compile_check(target_path);

        let test_result = if compile_result.success {
            self.run_tests(target_path)
        } else {
            false
        };

        if !compile_result.success || !test_result {
            std::fs::write(target_path, &original_content)?;
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
        let lines: Vec<&str> = content.lines().collect();

        match modification.modification_kind {
            ModificationKind::Replace => {
                let mut new_lines = lines[..modification.line_start - 1].to_vec();
                new_lines.push(&modification.new_code);
                new_lines.extend_from_slice(&lines[modification.line_end..]);
                Ok(new_lines.join("\n"))
            }
            ModificationKind::InsertBefore => {
                let mut new_lines = lines[..modification.line_start - 1].to_vec();
                new_lines.push(&modification.new_code);
                new_lines.extend_from_slice(&lines[modification.line_start - 1..]);
                Ok(new_lines.join("\n"))
            }
            ModificationKind::InsertAfter => {
                let mut new_lines = lines[..modification.line_end].to_vec();
                new_lines.push(&modification.new_code);
                new_lines.extend_from_slice(&lines[modification.line_end..]);
                Ok(new_lines.join("\n"))
            }
            ModificationKind::Remove => {
                let mut new_lines = lines[..modification.line_start - 1].to_vec();
                new_lines.extend_from_slice(&lines[modification.line_end..]);
                Ok(new_lines.join("\n"))
            }
            ModificationKind::Rename => {
                Ok(content.replace(&modification.old_code, &modification.new_code))
            }
            ModificationKind::Wrap => {
                let before = lines[..modification.line_start - 1].join("\n");
                let wrapped = modification.new_code.replace(
                    "{OLD_CODE}",
                    &lines[modification.line_start - 1..modification.line_end].join("\n"),
                );
                let after = lines[modification.line_end..].join("\n");
                Ok(format!("{}\n{}\n{}", before, wrapped, after))
            }
        }
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

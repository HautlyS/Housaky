use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub struct TsEngine {
    parser: tree_sitter::Parser,
    language: tree_sitter::Language,
}

impl TsEngine {
    pub fn new(language: tree_sitter::Language) -> Result<Self> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&language)
            .map_err(|_| anyhow!("Failed to set tree-sitter language"))?;
        Ok(Self { parser, language })
    }

    pub fn language(&self) -> tree_sitter::Language {
        self.language.clone()
    }

    pub fn parse_str(&mut self, source: &str) -> Result<tree_sitter::Tree> {
        self.parser
            .parse(source, None)
            .ok_or_else(|| anyhow!("tree-sitter returned no parse tree"))
    }

    pub fn parse_file(&mut self, path: &Path) -> Result<(String, tree_sitter::Tree)> {
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let tree = self.parse_str(&source)?;
        Ok((source, tree))
    }
}

pub fn byte_to_line_col(source: &str, byte_offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    let mut i = 0usize;
    for b in source.as_bytes() {
        if i >= byte_offset {
            break;
        }
        if *b == b'\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
        i += 1;
    }
    (line, col)
}

pub fn node_text<'a>(source: &'a str, node: tree_sitter::Node<'_>) -> &'a str {
    let range = node.byte_range();
    &source[range]
}

#[derive(Debug, Clone)]
pub struct TsAnchor {
    pub kind: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
}

impl TsAnchor {
    pub fn from_node(source: &str, node: tree_sitter::Node<'_>) -> Self {
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();
        let (start_line, _) = byte_to_line_col(source, start_byte);
        let (end_line, _) = byte_to_line_col(source, end_byte);
        Self {
            kind: node.kind().to_string(),
            start_byte,
            end_byte,
            start_line,
            end_line,
        }
    }
}

pub fn covering_named_node(
    tree: &tree_sitter::Tree,
    start_byte: usize,
    end_byte: usize,
) -> Option<tree_sitter::Node<'_>> {
    let mut node = tree
        .root_node()
        .descendant_for_byte_range(start_byte, end_byte)?;
    while !node.is_named() {
        node = node.parent()?;
    }
    Some(node)
}

#[cfg(feature = "tree-sitter-rust")]
pub fn rust_language() -> Result<tree_sitter::Language> {
    Ok(tree_sitter_rust::language().into())
}

#[cfg(not(feature = "tree-sitter-rust"))]
pub fn rust_language() -> Result<tree_sitter::Language> {
    Err(anyhow!(
        "Rust tree-sitter grammar not enabled. Add dependency `tree-sitter-rust` and enable feature `tree-sitter-rust`."
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnalysisResult {
    pub file: String,
    pub functions: Vec<FunctionInfo>,
    pub structs: Vec<StructInfo>,
    pub enums: Vec<EnumInfo>,
    pub impls: Vec<ImplInfo>,
    pub traits: Vec<TraitInfo>,
    pub mods: Vec<ModInfo>,
    pub imports: Vec<String>,
    pub complexity: f64,
    pub total_lines: usize,
    pub cyclomatic_complexity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub line_start: usize,
    pub line_end: usize,
    pub params: Vec<String>,
    pub return_type: Option<String>,
    pub is_async: bool,
    pub is_pub: bool,
    pub complexity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructInfo {
    pub name: String,
    pub line: usize,
    pub fields: Vec<FieldInfo>,
    pub is_pub: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumInfo {
    pub name: String,
    pub line: usize,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplInfo {
    pub type_name: String,
    pub trait_name: Option<String>,
    pub line: usize,
    pub methods: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitInfo {
    pub name: String,
    pub line: usize,
    pub methods: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModInfo {
    pub name: String,
    pub line: usize,
}

pub struct RustCodeAnalyzer {
    parser: tree_sitter::Parser,
    #[allow(dead_code)]
    language: Option<tree_sitter::Language>,
}

impl RustCodeAnalyzer {
    pub fn new() -> Self {
        let language = rust_language().ok();
        let mut parser = tree_sitter::Parser::new();
        if let Some(ref lang) = language {
            let _ = parser.set_language(lang);
        }
        Self { parser, language }
    }

    pub fn analyze(&mut self, source: &str) -> Result<CodeAnalysisResult> {
        #[cfg(feature = "tree-sitter-rust")]
        {
            self.analyze_with_tree_sitter(source)
        }
        #[cfg(not(feature = "tree-sitter-rust"))]
        {
            self.analyze_with_regex(source)
        }
    }

    #[cfg(feature = "tree-sitter-rust")]
    fn analyze_with_tree_sitter(&mut self, source: &str) -> Result<CodeAnalysisResult> {
        let tree = self
            .parser
            .parse(source, None)
            .ok_or_else(|| anyhow!("Failed to parse source"))?;

        let root = tree.root_node();
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut impls = Vec::new();
        let mut traits = Vec::new();
        let mut mods = Vec::new();
        let mut imports = Vec::new();
        let mut total_complexity: u32 = 0;

        let _cursor = root.walk();

        fn visit_node(
            node: tree_sitter::Node,
            source: &str,
            functions: &mut Vec<FunctionInfo>,
            structs: &mut Vec<StructInfo>,
            enums: &mut Vec<EnumInfo>,
            impls: &mut Vec<ImplInfo>,
            traits: &mut Vec<TraitInfo>,
            mods: &mut Vec<ModInfo>,
            imports: &mut Vec<String>,
            total_complexity: &mut u32,
        ) {
            match node.kind() {
                "function_item" => {
                    if let Some(func) = RustCodeAnalyzer::extract_function(node, source) {
                        *total_complexity += func.complexity;
                        functions.push(func);
                    }
                }
                "struct_item" => {
                    if let Some(s) = RustCodeAnalyzer::extract_struct(node, source) {
                        structs.push(s);
                    }
                }
                "enum_item" => {
                    if let Some(e) = RustCodeAnalyzer::extract_enum(node, source) {
                        enums.push(e);
                    }
                }
                "impl_item" => {
                    if let Some(i) = RustCodeAnalyzer::extract_impl(node, source) {
                        impls.push(i);
                    }
                }
                "trait_item" => {
                    if let Some(t) = RustCodeAnalyzer::extract_trait(node, source) {
                        traits.push(t);
                    }
                }
                "mod_item" => {
                    if let Some(m) = RustCodeAnalyzer::extract_mod(node, source) {
                        mods.push(m);
                    }
                }
                "use_declaration" => {
                    if let Some(import) = RustCodeAnalyzer::extract_import(node, source) {
                        imports.push(import);
                    }
                }
                _ => {}
            }

            if node.kind() == "function_item" {
                *total_complexity += RustCodeAnalyzer::count_complexity(node);
            }

            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                visit_node(
                    child,
                    source,
                    functions,
                    structs,
                    enums,
                    impls,
                    traits,
                    mods,
                    imports,
                    total_complexity,
                );
            }
        }

        visit_node(
            root,
            source,
            &mut functions,
            &mut structs,
            &mut enums,
            &mut impls,
            &mut traits,
            &mut mods,
            &mut imports,
            &mut total_complexity,
        );

        let total_lines = source.lines().count();
        let complexity = if functions.is_empty() {
            0.0
        } else {
            total_complexity as f64 / functions.len() as f64
        };

        Ok(CodeAnalysisResult {
            file: String::new(),
            functions: functions.clone(),
            structs,
            enums,
            impls,
            traits,
            mods,
            imports,
            complexity,
            total_lines,
            cyclomatic_complexity: total_complexity,
        })
    }

    #[cfg(feature = "tree-sitter-rust")]
    fn extract_function(node: tree_sitter::Node, source: &str) -> Option<FunctionInfo> {
        let mut cursor = node.walk();
        let mut name = String::new();
        let mut params = Vec::new();
        let mut return_type = None;
        let mut is_async = false;
        let mut is_pub = false;
        let mut complexity = 1u32;

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    name = node_text(source, child).to_string();
                }
                "parameters" => {
                    let mut param_cursor = child.walk();
                    for param in child.children(&mut param_cursor) {
                        if param.kind() == "parameter" || param.kind() == "self_param" {
                            params.push(node_text(source, param).to_string());
                        }
                    }
                }
                "type_identifier" | "generic_type" | "tuple_type" | "reference_type" => {
                    if return_type.is_none() {
                        return_type = Some(node_text(source, child).to_string());
                    }
                }
                "async" => {
                    is_async = true;
                }
                "visibility_modifier" => {
                    is_pub = true;
                }
                _ => {}
            }
        }

        complexity += Self::count_complexity(node);

        let start_line = byte_to_line_col(source, node.start_byte()).0;
        let end_line = byte_to_line_col(source, node.end_byte()).0;

        Some(FunctionInfo {
            name,
            line_start: start_line,
            line_end: end_line,
            params,
            return_type,
            is_async,
            is_pub,
            complexity,
        })
    }

    #[cfg(feature = "tree-sitter-rust")]
    fn extract_struct(node: tree_sitter::Node, source: &str) -> Option<StructInfo> {
        let mut cursor = node.walk();
        let mut name = String::new();
        let mut fields = Vec::new();
        let mut is_pub = false;

        for child in node.children(&mut cursor) {
            match child.kind() {
                "type_identifier" => {
                    name = node_text(source, child).to_string();
                }
                "field_declaration_list" => {
                    let mut field_cursor = child.walk();
                    for field in child.children(&mut field_cursor) {
                        if field.kind() == "field_declaration" {
                            let mut fc = field.walk();
                            let mut field_name = String::new();
                            let mut field_type = String::new();
                            for fc_child in field.children(&mut fc) {
                                match fc_child.kind() {
                                    "field_identifier" => {
                                        field_name = node_text(source, fc_child).to_string();
                                    }
                                    "type_identifier" | "generic_type" | "reference_type"
                                    | "tuple_type" => {
                                        field_type = node_text(source, fc_child).to_string();
                                    }
                                    _ => {}
                                }
                            }
                            if !field_name.is_empty() {
                                fields.push(FieldInfo {
                                    name: field_name,
                                    type_name: field_type,
                                });
                            }
                        }
                    }
                }
                "visibility_modifier" => {
                    is_pub = true;
                }
                _ => {}
            }
        }

        let line = byte_to_line_col(source, node.start_byte()).0;

        Some(StructInfo {
            name,
            line,
            fields,
            is_pub,
        })
    }

    #[cfg(feature = "tree-sitter-rust")]
    fn extract_enum(node: tree_sitter::Node, source: &str) -> Option<EnumInfo> {
        let mut cursor = node.walk();
        let mut name = String::new();
        let mut variants = Vec::new();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "type_identifier" => {
                    name = node_text(source, child).to_string();
                }
                "enum_variant_list" => {
                    let mut variant_cursor = child.walk();
                    for variant in child.children(&mut variant_cursor) {
                        if variant.kind() == "enum_variant" {
                            let mut vc = variant.walk();
                            for vc_child in variant.children(&mut vc) {
                                if vc_child.kind() == "identifier" {
                                    variants.push(node_text(source, vc_child).to_string());
                                    break;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let line = byte_to_line_col(source, node.start_byte()).0;

        Some(EnumInfo {
            name,
            line,
            variants,
        })
    }

    #[cfg(feature = "tree-sitter-rust")]
    fn extract_impl(node: tree_sitter::Node, source: &str) -> Option<ImplInfo> {
        let mut cursor = node.walk();
        let mut type_name = String::new();
        let trait_name = None;
        let mut methods = Vec::new();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "type_identifier" => {
                    if type_name.is_empty() {
                        type_name = node_text(source, child).to_string();
                    }
                }
                "declaration_list" => {
                    let mut method_cursor = child.walk();
                    for method in child.children(&mut method_cursor) {
                        if method.kind() == "function_item" {
                            let mut fc = method.walk();
                            for fc_child in method.children(&mut fc) {
                                if fc_child.kind() == "identifier" {
                                    methods.push(node_text(source, fc_child).to_string());
                                    break;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let line = byte_to_line_col(source, node.start_byte()).0;

        Some(ImplInfo {
            type_name,
            trait_name,
            line,
            methods,
        })
    }

    #[cfg(feature = "tree-sitter-rust")]
    fn extract_trait(node: tree_sitter::Node, source: &str) -> Option<TraitInfo> {
        let mut cursor = node.walk();
        let mut name = String::new();
        let mut methods = Vec::new();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "type_identifier" => {
                    name = node_text(source, child).to_string();
                }
                "declaration_list" => {
                    let mut method_cursor = child.walk();
                    for method in child.children(&mut method_cursor) {
                        if method.kind() == "function_item"
                            || method.kind() == "function_signature_item"
                        {
                            let mut fc = method.walk();
                            for fc_child in method.children(&mut fc) {
                                if fc_child.kind() == "identifier" {
                                    methods.push(node_text(source, fc_child).to_string());
                                    break;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let line = byte_to_line_col(source, node.start_byte()).0;

        Some(TraitInfo {
            name,
            line,
            methods,
        })
    }

    #[cfg(feature = "tree-sitter-rust")]
    fn extract_mod(node: tree_sitter::Node, source: &str) -> Option<ModInfo> {
        let mut cursor = node.walk();
        let mut name = String::new();

        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                name = node_text(source, child).to_string();
                break;
            }
        }

        let line = byte_to_line_col(source, node.start_byte()).0;

        Some(ModInfo { name, line })
    }

    #[cfg(feature = "tree-sitter-rust")]
    fn extract_import(node: tree_sitter::Node, source: &str) -> Option<String> {
        Some(node_text(source, node).to_string())
    }

    #[cfg(feature = "tree-sitter-rust")]
    fn count_complexity(node: tree_sitter::Node) -> u32 {
        let mut complexity = 0u32;
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "if_expression" | "match_expression" | "for_expression" | "while_expression"
                | "loop_expression" => {
                    complexity += 1;
                }
                "&&" | "||" => {
                    complexity += 1;
                }
                "?" => {
                    complexity += 1;
                }
                _ => {}
            }
            complexity += Self::count_complexity(child);
        }

        complexity
    }

    #[cfg(not(feature = "tree-sitter-rust"))]
    fn analyze_with_regex(&mut self, source: &str) -> Result<CodeAnalysisResult> {
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let total_lines = source.lines().count();

        for (line_num, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("fn ") || trimmed.contains(" fn ") {
                if let Some(name) = Self::extract_fn_name_regex(trimmed) {
                    functions.push(FunctionInfo {
                        name,
                        line_start: line_num + 1,
                        line_end: line_num + 1,
                        params: Vec::new(),
                        return_type: None,
                        is_async: trimmed.contains("async"),
                        is_pub: trimmed.starts_with("pub"),
                        complexity: 1,
                    });
                }
            }
            if trimmed.starts_with("struct ") || trimmed.contains(" struct ") {
                if let Some(name) = Self::extract_struct_name_regex(trimmed) {
                    structs.push(StructInfo {
                        name,
                        line: line_num + 1,
                        fields: Vec::new(),
                        is_pub: trimmed.starts_with("pub"),
                    });
                }
            }
        }

        let complexity = if functions.is_empty() { 0.0 } else { 1.0 };

        Ok(CodeAnalysisResult {
            file: String::new(),
            functions: functions.clone(),
            structs,
            enums: Vec::new(),
            impls: Vec::new(),
            traits: Vec::new(),
            mods: Vec::new(),
            imports: Vec::new(),
            complexity,
            total_lines,
            cyclomatic_complexity: functions.len() as u32,
        })
    }

    #[cfg(not(feature = "tree-sitter-rust"))]
    fn extract_fn_name_regex(line: &str) -> Option<String> {
        let line = line.trim_start_matches("pub ").trim();
        let line = line.trim_start_matches("async ").trim();
        let line = line.trim_start_matches("fn ").trim();
        let end = line.find('(')?;
        Some(line[..end].to_string())
    }

    #[cfg(not(feature = "tree-sitter-rust"))]
    fn extract_struct_name_regex(line: &str) -> Option<String> {
        let line = line.trim_start_matches("pub ").trim();
        let line = line.trim_start_matches("struct ").trim();
        let end = line.find('{').or_else(|| line.find('<'))?;
        Some(line[..end].trim().to_string())
    }
}

impl Default for RustCodeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

use anyhow::{anyhow, Context, Result};
use std::path::Path;

/// A lightweight wrapper around tree-sitter that focuses on what Housaky needs for
/// self-improvement:
/// - stable structural anchors (byte ranges) for edits
/// - incremental parse friendliness (Parser + Tree)
/// - cheap extraction of symbols/blocks for LLM context
///
/// Note: Tree-sitter requires a `Language` implementation per language (e.g. Rust).
/// We intentionally keep this module generic and allow language injection.
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

/// Convert a byte offset into (1-based) line and column.
/// Column is counted in bytes (UTF-8). That's consistent with tree-sitter's Points.
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

/// A structural anchor used for safe code edits: stable byte range + node kind.
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

/// Find the smallest named node that fully covers the given (byte) range.
pub fn covering_named_node(tree: &tree_sitter::Tree, start_byte: usize, end_byte: usize) -> Option<tree_sitter::Node<'_>> {
    let mut node = tree.root_node().descendant_for_byte_range(start_byte, end_byte)?;
    while !node.is_named() {
        node = node.parent()?;
    }
    Some(node)
}

/// Convenience: best-effort Rust language loader.
///
/// By default, this returns an error because we don't depend on a specific Rust grammar crate.
/// If you add a Rust grammar crate (e.g. `tree-sitter-rust`) and enable feature
/// `tree-sitter-rust`, this will return the Rust language.
#[cfg(feature = "tree-sitter-rust")]
pub fn rust_language() -> Result<tree_sitter::Language> {
    Ok(tree_sitter_rust::LANGUAGE.into())
}

#[cfg(not(feature = "tree-sitter-rust"))]
pub fn rust_language() -> Result<tree_sitter::Language> {
    Err(anyhow!(
        "Rust tree-sitter grammar not enabled. Add dependency `tree-sitter-rust` and enable feature `tree-sitter-rust`."
    ))
}

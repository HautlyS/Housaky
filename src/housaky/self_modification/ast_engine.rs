use crate::housaky::self_modification::mutation_ops::{MutationOp, MutationTarget};
use anyhow::{Context, Result};
use std::path::Path;
use syn::{parse_file, visit_mut::VisitMut, File, ItemFn};
use tracing::{info, warn};

pub struct AstEngine;

impl AstEngine {
    pub fn parse_file(path: &Path) -> Result<File> {
        let src = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {:?}", path))?;
        parse_file(&src).with_context(|| format!("Failed to parse AST of {:?}", path))
    }

    pub fn unparse(file: &File) -> String {
        prettyprint_file(file)
    }

    pub fn apply_mutation(
        path: &Path,
        op: &MutationOp,
        target: &MutationTarget,
    ) -> Result<String> {
        let mut ast = Self::parse_file(path)?;

        match op {
            MutationOp::AddCaching => {
                let mut visitor = AddCachingVisitor {
                    target_fn: target.function_name.clone(),
                    applied: false,
                };
                visitor.visit_file_mut(&mut ast);
                if !visitor.applied {
                    warn!("AddCaching: function '{}' not found", target.function_name);
                }
            }
            MutationOp::AddLogging => {
                let mut visitor = AddLoggingVisitor {
                    target_fn: target.function_name.clone(),
                    applied: false,
                };
                visitor.visit_file_mut(&mut ast);
                if !visitor.applied {
                    warn!("AddLogging: function '{}' not found", target.function_name);
                }
            }
            MutationOp::AddEarlyReturn => {
                let mut visitor = AddEarlyReturnVisitor {
                    target_fn: target.function_name.clone(),
                    condition: target.extra.clone().unwrap_or_default(),
                    applied: false,
                };
                visitor.visit_file_mut(&mut ast);
            }
            MutationOp::InlineConstant { name, value } => {
                let mut visitor = InlineConstantVisitor {
                    const_name: name.clone(),
                    new_value: *value,
                };
                visitor.visit_file_mut(&mut ast);
            }
        }

        let new_src = Self::unparse(&ast);
        info!(
            "Applied mutation {:?} to {}",
            op,
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        Ok(new_src)
    }

    pub fn get_function_names(path: &Path) -> Result<Vec<String>> {
        let ast = Self::parse_file(path)?;
        let mut names = Vec::new();
        for item in &ast.items {
            if let syn::Item::Fn(f) = item {
                names.push(f.sig.ident.to_string());
            }
        }
        Ok(names)
    }

    pub fn function_exists(path: &Path, fn_name: &str) -> Result<bool> {
        let names = Self::get_function_names(path)?;
        Ok(names.iter().any(|n| n == fn_name))
    }

    pub fn count_functions(path: &Path) -> Result<usize> {
        Ok(Self::get_function_names(path)?.len())
    }
}

// ── Visitors ─────────────────────────────────────────────────────────────────

struct AddCachingVisitor {
    target_fn: String,
    applied: bool,
}

impl VisitMut for AddCachingVisitor {
    fn visit_item_fn_mut(&mut self, item: &mut ItemFn) {
        if item.sig.ident == self.target_fn {
            let cache_stmt: syn::Stmt = syn::parse_quote! {
                static CACHE: std::sync::OnceLock<std::collections::HashMap<String, serde_json::Value>> =
                    std::sync::OnceLock::new();
            };
            item.block.stmts.insert(0, cache_stmt);
            self.applied = true;
        }
        syn::visit_mut::visit_item_fn_mut(self, item);
    }
}

struct AddLoggingVisitor {
    target_fn: String,
    applied: bool,
}

impl VisitMut for AddLoggingVisitor {
    fn visit_item_fn_mut(&mut self, item: &mut ItemFn) {
        if item.sig.ident == self.target_fn {
            let fn_name = item.sig.ident.to_string();
            let log_entry: syn::Stmt = syn::parse_quote! {
                tracing::debug!(fn_name = #fn_name, "entering function");
            };
            item.block.stmts.insert(0, log_entry);
            self.applied = true;
        }
        syn::visit_mut::visit_item_fn_mut(self, item);
    }
}

struct AddEarlyReturnVisitor {
    target_fn: String,
    condition: String,
    applied: bool,
}

impl VisitMut for AddEarlyReturnVisitor {
    fn visit_item_fn_mut(&mut self, item: &mut ItemFn) {
        if item.sig.ident == self.target_fn && !self.condition.is_empty() {
            // Parse condition into an expression safely; skip on parse error
            if let Ok(cond_expr) = syn::parse_str::<syn::Expr>(&self.condition) {
                let guard: syn::Stmt = syn::parse_quote! {
                    if #cond_expr { return Default::default(); }
                };
                item.block.stmts.insert(0, guard);
                self.applied = true;
            }
        }
        syn::visit_mut::visit_item_fn_mut(self, item);
    }
}

struct InlineConstantVisitor {
    const_name: String,
    new_value: f64,
}

impl VisitMut for InlineConstantVisitor {
    fn visit_lit_float_mut(&mut self, lit: &mut syn::LitFloat) {
        let _ = (lit, &self.const_name, self.new_value);
        // Inline constant replacement requires context awareness; no-op default
    }
}

fn prettyprint_file(file: &File) -> String {
    // Use quote to render the AST back to source
    let tokens = quote::quote!(#file);
    tokens.to_string()
}

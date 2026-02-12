//! AST-based code manipulation for mutations
//!
//! This module provides production-ready AST manipulation for applying mutations
//! to Rust code using the syn crate.

use anyhow::Result;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{visit_mut::VisitMut, *};

/// AST-based mutation applicator
pub struct AstMutator;

impl AstMutator {
    /// Apply a mutation to source code using AST manipulation
    pub fn apply_mutation(
        source: &str,
        target_item: &str,
        mutation_type: MutationType,
        new_code: &str,
    ) -> Result<String> {
        // Parse the source file
        let mut file: File = parse_file(source)?;

        // Create a visitor to find and modify the target item
        let mut visitor = MutationVisitor::new(target_item, mutation_type, new_code);
        visitor.visit_file_mut(&mut file);

        // Convert back to source code
        let modified = quote!(#file).to_string();

        // Format the code (optional, for readability)
        Ok(modified)
    }

    /// Replace a function body
    pub fn replace_function_body(
        source: &str,
        function_name: &str,
        new_body: &str,
    ) -> Result<String> {
        let mut file: File = parse_file(source)?;

        let mut found = false;
        for item in &mut file.items {
            if let Item::Fn(func) = item {
                if func.sig.ident == function_name {
                    // Parse the new body
                    let new_body_tokens: TokenStream = new_body.parse()?;
                    func.block = parse_quote!({ #new_body_tokens });
                    found = true;
                    break;
                }
            }
        }

        if !found {
            return Err(anyhow::anyhow!("Function '{}' not found", function_name));
        }

        Ok(quote!(#file).to_string())
    }

    /// Add a new function to the module
    pub fn add_function(source: &str, function_code: &str) -> Result<String> {
        let mut file: File = parse_file(source)?;

        // Parse the new function
        let new_func: ItemFn = parse_str(function_code)?;

        // Add to items
        file.items.push(Item::Fn(new_func));

        Ok(quote!(#file).to_string())
    }

    /// Replace an expression with another expression
    pub fn replace_expression(source: &str, old_expr: &str, new_expr: &str) -> Result<String> {
        let mut file: File = parse_file(source)?;

        let old_expr_tokens: Expr = parse_str(old_expr)?;
        let new_expr_tokens: Expr = parse_str(new_expr)?;

        let mut replacer = ExpressionReplacer {
            old_expr: old_expr_tokens,
            new_expr: new_expr_tokens,
            replaced: false,
        };

        replacer.visit_file_mut(&mut file);

        if !replacer.replaced {
            return Err(anyhow::anyhow!("Expression not found"));
        }

        Ok(quote!(#file).to_string())
    }

    /// Inline a constant value
    pub fn inline_constant(source: &str, const_name: &str, value: &str) -> Result<String> {
        let mut file: File = parse_file(source)?;

        let value_expr: Expr = parse_str(value)?;

        let mut inliner = ConstantInliner {
            const_name: const_name.to_string(),
            value: value_expr,
            inlined: false,
        };

        inliner.visit_file_mut(&mut file);

        Ok(quote!(#file).to_string())
    }

    /// Rename an identifier throughout the file
    pub fn rename_identifier(source: &str, old_name: &str, new_name: &str) -> Result<String> {
        let mut file: File = parse_file(source)?;

        let mut renamer = IdentifierRenamer {
            old_name: old_name.to_string(),
            new_name: new_name.to_string(),
        };

        renamer.visit_file_mut(&mut file);

        Ok(quote!(#file).to_string())
    }

    /// Extract code complexity metrics from AST
    pub fn analyze_complexity(source: &str) -> Result<ComplexityMetrics> {
        let file: File = parse_file(source)?;

        let mut analyzer = ComplexityAnalyzer::default();
        analyzer.visit_file(&file);

        Ok(ComplexityMetrics {
            cyclomatic_complexity: analyzer.cyclomatic_complexity,
            lines_of_code: analyzer.lines_of_code,
            number_of_functions: analyzer.function_count,
            max_nesting_depth: analyzer.max_nesting_depth,
        })
    }
}

/// Types of mutations
#[derive(Debug, Clone, Copy)]
pub enum MutationType {
    ReplaceFunction,
    AddFunction,
    ModifyExpression,
    InlineConstant,
    RenameIdentifier,
    OptimizeLoop,
}

/// AST visitor for applying mutations
struct MutationVisitor<'a> {
    target: &'a str,
    mutation_type: MutationType,
    new_code: &'a str,
    modified: bool,
}

impl<'a> MutationVisitor<'a> {
    fn new(target: &'a str, mutation_type: MutationType, new_code: &'a str) -> Self {
        Self {
            target,
            mutation_type,
            new_code,
            modified: false,
        }
    }
}

impl<'a> VisitMut for MutationVisitor<'a> {
    fn visit_item_fn_mut(&mut self, func: &mut ItemFn) {
        if func.sig.ident == self.target {
            match self.mutation_type {
                MutationType::ReplaceFunction => {
                    if let Ok(new_func) = parse_str::<ItemFn>(self.new_code) {
                        *func = new_func;
                        self.modified = true;
                    }
                }
                _ => {}
            }
        }

        // Continue visiting
        visit_mut::visit_item_fn_mut(self, func);
    }
}

/// Expression replacement visitor
struct ExpressionReplacer {
    old_expr: Expr,
    new_expr: Expr,
    replaced: bool,
}

impl VisitMut for ExpressionReplacer {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        // Check if expressions are equivalent (simplified check)
        if expressions_equivalent(expr, &self.old_expr) {
            *expr = self.new_expr.clone();
            self.replaced = true;
        } else {
            visit_mut::visit_expr_mut(self, expr);
        }
    }
}

/// Check if two expressions are equivalent (simplified)
fn expressions_equivalent(a: &Expr, b: &Expr) -> bool {
    // Simplified: compare their string representations
    quote!(#a).to_string() == quote!(#b).to_string()
}

/// Constant inlining visitor
struct ConstantInliner {
    const_name: String,
    value: Expr,
    inlined: bool,
}

impl VisitMut for ConstantInliner {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Expr::Path(path) = expr {
            if path.path.is_ident(&self.const_name) {
                *expr = self.value.clone();
                self.inlined = true;
                return;
            }
        }

        visit_mut::visit_expr_mut(self, expr);
    }
}

/// Identifier renaming visitor
struct IdentifierRenamer {
    old_name: String,
    new_name: String,
}

impl VisitMut for IdentifierRenamer {
    fn visit_ident_mut(&mut self, ident: &mut Ident) {
        if ident == &self.old_name {
            *ident = Ident::new(&self.new_name, ident.span());
        }
    }

    fn visit_item_fn_mut(&mut self, func: &mut ItemFn) {
        if func.sig.ident == self.old_name {
            func.sig.ident = Ident::new(&self.new_name, func.sig.ident.span());
        }
        visit_mut::visit_item_fn_mut(self, func);
    }
}

/// Complexity metrics
#[derive(Debug, Clone, Default)]
pub struct ComplexityMetrics {
    pub cyclomatic_complexity: usize,
    pub lines_of_code: usize,
    pub number_of_functions: usize,
    pub max_nesting_depth: usize,
}

/// Complexity analyzer
#[derive(Default)]
struct ComplexityAnalyzer {
    cyclomatic_complexity: usize,
    lines_of_code: usize,
    function_count: usize,
    max_nesting_depth: usize,
    current_depth: usize,
}

impl<'ast> syn::visit::Visit<'ast> for ComplexityAnalyzer {
    fn visit_item_fn(&mut self, func: &'ast ItemFn) {
        self.function_count += 1;
        self.cyclomatic_complexity += 1; // Base complexity

        // Count lines
        let func_str = quote!(#func).to_string();
        self.lines_of_code += func_str.lines().count();

        // Visit body
        syn::visit::visit_item_fn(self, func);
    }

    fn visit_expr_if(&mut self, expr: &'ast ExprIf) {
        self.cyclomatic_complexity += 1;
        self.current_depth += 1;
        self.max_nesting_depth = self.max_nesting_depth.max(self.current_depth);

        syn::visit::visit_expr_if(self, expr);

        self.current_depth -= 1;
    }

    fn visit_expr_while(&mut self, expr: &'ast ExprWhile) {
        self.cyclomatic_complexity += 1;
        self.current_depth += 1;
        self.max_nesting_depth = self.max_nesting_depth.max(self.current_depth);

        syn::visit::visit_expr_while(self, expr);

        self.current_depth -= 1;
    }

    fn visit_expr_for_loop(&mut self, expr: &'ast ExprForLoop) {
        self.cyclomatic_complexity += 1;
        self.current_depth += 1;
        self.max_nesting_depth = self.max_nesting_depth.max(self.current_depth);

        syn::visit::visit_expr_for_loop(self, expr);

        self.current_depth -= 1;
    }

    fn visit_expr_match(&mut self, expr: &'ast ExprMatch) {
        self.cyclomatic_complexity += expr.arms.len();
        self.current_depth += 1;
        self.max_nesting_depth = self.max_nesting_depth.max(self.current_depth);

        syn::visit::visit_expr_match(self, expr);

        self.current_depth -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_function_body() {
        let source = r#"
fn test() {
    let x = 5;
    println!("{}", x);
}
"#;

        let result = AstMutator::replace_function_body(source, "test", "let y = 10; y");
        assert!(result.is_ok());
        assert!(result.unwrap().contains("let y = 10"));
    }

    #[test]
    fn test_add_function() {
        let source = r#"
fn existing() {}
"#;

        let result = AstMutator::add_function(source, "fn new_func() { 42 }");
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("existing"));
        assert!(output.contains("new_func"));
    }

    #[test]
    fn test_rename_identifier() {
        let source = r#"
fn foo() {
    let x = 5;
    println!("{}", x);
}
"#;

        let result = AstMutator::rename_identifier(source, "x", "y");
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.contains("let x = 5"));
        assert!(output.contains("let y = 5"));
    }

    #[test]
    fn test_analyze_complexity() {
        let source = r#"
fn complex(x: i32) -> i32 {
    if x > 0 {
        if x > 10 {
            return x * 2;
        }
        return x;
    }
    0
}
"#;

        let metrics = AstMutator::analyze_complexity(source).unwrap();
        assert!(metrics.cyclomatic_complexity > 1);
        assert!(metrics.number_of_functions >= 1);
    }
}

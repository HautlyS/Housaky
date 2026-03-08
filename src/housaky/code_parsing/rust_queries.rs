pub const FUNCTION_QUERY: &str = r#"
(function_item
  name: (identifier) @name
  parameters: (parameters) @params
  return_type: (_)? @return_type
  body: (block) @body)
"#;

pub const STRUCT_QUERY: &str = r#"
(struct_item
  name: (identifier) @name
  body: (field_declaration_list) @fields)
"#;

pub const ENUM_QUERY: &str = r#"
(enum_item
  name: (identifier) @name
  body: (enum_variant_list) @variants)
"#;

pub const IMPL_QUERY: &str = r#"
(impl_item
  type: (_) @type
  body: (declaration_list) @body)
"#;

pub const TRAIT_QUERY: &str = r#"
(trait_item
  name: (identifier) @name
  body: (declaration_list) @body)
"#;

pub const MOD_QUERY: &str = r#"
(mod_item
  name: (identifier) @name
  body: (declaration_list)? @body)
"#;

pub const USE_QUERY: &str = r#"
(use_declaration
  argument: (_) @path)
"#;

pub const LET_BINDING_QUERY: &str = r#"
(let_declaration
  pattern: (_) @pattern
  type: (_)? @type
  value: (_)? @value)
"#;

pub const CONST_QUERY: &str = r#"
(const_item
  name: (identifier) @name
  type: (_) @type
  value: (_) @value)
"#;

pub const STATIC_QUERY: &str = r#"
(static_item
  name: (identifier) @name
  type: (_) @type
  value: (_) @value)
"#;

pub const TYPE_ALIAS_QUERY: &str = r#"
(type_item
  name: (type_identifier) @name
  type: (_) @type)
"#;

pub const MACRO_DEF_QUERY: &str = r#"
(macro_definition
  name: (identifier) @name
  body: (_) @body)
"#;

pub const MACRO_INVOCATION_QUERY: &str = r#"
(macro_invocation
  macro: (identifier) @name
  arguments: (_) @args)
"#;

pub const CALL_EXPRESSION_QUERY: &str = r#"
(call_expression
  function: (_) @function
  arguments: (arguments) @args)
"#;

pub const METHOD_CALL_QUERY: &str = r#"
(method_call_expression
  object: (_) @object
  name: (identifier) @method
  arguments: (arguments) @args)
"#;

pub const IF_EXPR_QUERY: &str = r#"
(if_expression
  condition: (_) @condition
  consequence: (block) @then
  alternative: (_)? @else)
"#;

pub const MATCH_EXPR_QUERY: &str = r#"
(match_expression
  value: (_) @value
  body: (match_block) @arms)
"#;

pub const LOOP_EXPR_QUERY: &str = r#"
(loop_expression
  body: (block) @body)
"#;

pub const FOR_EXPR_QUERY: &str = r#"
(for_expression
  pattern: (_) @pattern
  value: (_) @iterator
  body: (block) @body)
"#;

pub const WHILE_EXPR_QUERY: &str = r#"
(while_expression
  condition: (_) @condition
  body: (block) @body)
"#;

pub const CLOSURE_QUERY: &str = r#"
(closure_expression
  parameters: (closure_parameters)? @params
  body: (_) @body)
"#;

pub const STRUCT_EXPR_QUERY: &str = r#"
(struct_expression
  name: (type_identifier) @name
  body: (field_initializer_list) @fields)
"#;

pub const FIELD_QUERY: &str = r#"
(field_declaration
  name: (field_identifier) @name
  type: (_) @type)
"#;

pub const FUNCTION_PARAM_QUERY: &str = r#"
(parameter
  pattern: (_) @pattern
  type: (_) @type)
"#;

pub const SELF_PARAM_QUERY: &str = r#"
(self_param
  (_)? @kind
  type: (_) @type)
"#;

pub const COMMENT_QUERY: &str = r#"
(line_comment) @comment
(block_comment) @comment
"#;

pub const STRING_LITERAL_QUERY: &str = r#"
(string_literal) @string
(raw_string_literal) @raw_string
"#;

pub const ATTR_QUERY: &str = r#"
(inner_attribute_item
  (_) @attr)
(outer_attribute_item
  (_) @attr)
"#;

pub const PUB_QUERY: &str = r#"
(visibility_modifier
  "pub") @pub
"#;

pub const UNSAFE_QUERY: &str = r#"
(unsafe_expression
  body: (_) @body)
(unsafe_impl
  body: (_) @body)
"#;

pub const ASYNC_QUERY: &str = r#"
(async_expression
  body: (_) @body)
(async_block
  body: (_) @body)
"#;

pub const AWAIT_QUERY: &str = r#"
(await_expression
  value: (_) @value)
"#;

pub const TRY_EXPR_QUERY: &str = r#"
(try_expression
  body: (_) @body)
"#;

pub const REFERENCE_QUERY: &str = r#"
(reference_expression
  value: (_) @value)
(mut_reference_expression
  value: (_) @value)
"#;

pub const DEREF_QUERY: &str = r#"
(dereference_expression
  value: (_) @value)
"#;

pub const RANGE_QUERY: &str = r#"
(range_expression) @range
"#;

pub const BINARY_EXPR_QUERY: &str = r#"
(binary_expression
  left: (_) @left
  operator: _ @op
  right: (_) @right)
"#;

pub const UNARY_EXPR_QUERY: &str = r#"
(unary_expression
  operator: _ @op
  operand: (_) @operand)
"#;

pub const INDEX_EXPR_QUERY: &str = r#"
(index_expression
  value: (_) @value
  index: (_) @index)
"#;

pub const SLICE_EXPR_QUERY: &str = r#"
(slice_expression
  value: (_) @value
  start: (_)? @start
  end: (_)? @end)
"#;

pub const BREAK_EXPR_QUERY: &str = r#"
(break_expression
  label: (_)? @label
  value: (_)? @value)
"#;

pub const CONTINUE_EXPR_QUERY: &str = r#"
(continue_expression
  label: (_)? @label)
"#;

pub const RETURN_EXPR_QUERY: &str = r#"
(return_expression
  value: (_)? @value)
"#;

pub struct RustQueries;

impl RustQueries {
    pub fn function_query() -> &'static str {
        FUNCTION_QUERY
    }

    pub fn struct_query() -> &'static str {
        STRUCT_QUERY
    }

    pub fn enum_query() -> &'static str {
        ENUM_QUERY
    }

    pub fn impl_query() -> &'static str {
        IMPL_QUERY
    }

    pub fn trait_query() -> &'static str {
        TRAIT_QUERY
    }

    pub fn mod_query() -> &'static str {
        MOD_QUERY
    }

    pub fn use_query() -> &'static str {
        USE_QUERY
    }

    pub fn let_binding_query() -> &'static str {
        LET_BINDING_QUERY
    }

    pub fn const_query() -> &'static str {
        CONST_QUERY
    }

    pub fn static_query() -> &'static str {
        STATIC_QUERY
    }

    pub fn type_alias_query() -> &'static str {
        TYPE_ALIAS_QUERY
    }

    pub fn macro_def_query() -> &'static str {
        MACRO_DEF_QUERY
    }

    pub fn macro_invocation_query() -> &'static str {
        MACRO_INVOCATION_QUERY
    }

    pub fn call_expression_query() -> &'static str {
        CALL_EXPRESSION_QUERY
    }

    pub fn method_call_query() -> &'static str {
        METHOD_CALL_QUERY
    }

    pub fn if_expr_query() -> &'static str {
        IF_EXPR_QUERY
    }

    pub fn match_expr_query() -> &'static str {
        MATCH_EXPR_QUERY
    }

    pub fn loop_expr_query() -> &'static str {
        LOOP_EXPR_QUERY
    }

    pub fn for_expr_query() -> &'static str {
        FOR_EXPR_QUERY
    }

    pub fn while_expr_query() -> &'static str {
        WHILE_EXPR_QUERY
    }

    pub fn closure_query() -> &'static str {
        CLOSURE_QUERY
    }

    pub fn struct_expr_query() -> &'static str {
        STRUCT_EXPR_QUERY
    }

    pub fn field_query() -> &'static str {
        FIELD_QUERY
    }

    pub fn function_param_query() -> &'static str {
        FUNCTION_PARAM_QUERY
    }

    pub fn self_param_query() -> &'static str {
        SELF_PARAM_QUERY
    }

    pub fn comment_query() -> &'static str {
        COMMENT_QUERY
    }

    pub fn string_literal_query() -> &'static str {
        STRING_LITERAL_QUERY
    }

    pub fn attr_query() -> &'static str {
        ATTR_QUERY
    }

    pub fn pub_query() -> &'static str {
        PUB_QUERY
    }

    pub fn unsafe_query() -> &'static str {
        UNSAFE_QUERY
    }

    pub fn async_query() -> &'static str {
        ASYNC_QUERY
    }

    pub fn await_query() -> &'static str {
        AWAIT_QUERY
    }

    pub fn try_expr_query() -> &'static str {
        TRY_EXPR_QUERY
    }

    pub fn reference_query() -> &'static str {
        REFERENCE_QUERY
    }

    pub fn deref_query() -> &'static str {
        DEREF_QUERY
    }

    pub fn range_query() -> &'static str {
        RANGE_QUERY
    }

    pub fn binary_expr_query() -> &'static str {
        BINARY_EXPR_QUERY
    }

    pub fn unary_expr_query() -> &'static str {
        UNARY_EXPR_QUERY
    }

    pub fn index_expr_query() -> &'static str {
        INDEX_EXPR_QUERY
    }

    pub fn slice_expr_query() -> &'static str {
        SLICE_EXPR_QUERY
    }

    pub fn break_expr_query() -> &'static str {
        BREAK_EXPR_QUERY
    }

    pub fn continue_expr_query() -> &'static str {
        CONTINUE_EXPR_QUERY
    }

    pub fn return_expr_query() -> &'static str {
        RETURN_EXPR_QUERY
    }

    pub fn all_definitions() -> Vec<&'static str> {
        vec![
            FUNCTION_QUERY,
            STRUCT_QUERY,
            ENUM_QUERY,
            IMPL_QUERY,
            TRAIT_QUERY,
            MOD_QUERY,
            CONST_QUERY,
            STATIC_QUERY,
            TYPE_ALIAS_QUERY,
            MACRO_DEF_QUERY,
        ]
    }

    pub fn complexity_indicators() -> Vec<&'static str> {
        vec![
            IF_EXPR_QUERY,
            MATCH_EXPR_QUERY,
            LOOP_EXPR_QUERY,
            FOR_EXPR_QUERY,
            WHILE_EXPR_QUERY,
            CLOSURE_QUERY,
        ]
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub params: Vec<String>,
    pub return_type: Option<String>,
    pub is_async: bool,
    pub is_unsafe: bool,
    pub is_pub: bool,
    pub line_start: usize,
    pub line_end: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub complexity: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StructInfo {
    pub name: String,
    pub fields: Vec<FieldInfo>,
    pub is_pub: bool,
    pub line_start: usize,
    pub line_end: usize,
    pub byte_start: usize,
    pub byte_end: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FieldInfo {
    pub name: String,
    pub type_name: String,
    pub is_pub: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnumInfo {
    pub name: String,
    pub variants: Vec<String>,
    pub is_pub: bool,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImplInfo {
    pub type_name: String,
    pub trait_name: Option<String>,
    pub methods: Vec<String>,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CodeMetrics {
    pub total_lines: usize,
    pub total_functions: usize,
    pub total_structs: usize,
    pub total_enums: usize,
    pub total_impls: usize,
    pub total_traits: usize,
    pub total_mods: usize,
    pub total_comments: usize,
    pub avg_function_length: f64,
    pub max_function_length: usize,
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub nesting_depth: u32,
}

impl Default for CodeMetrics {
    fn default() -> Self {
        Self {
            total_lines: 0,
            total_functions: 0,
            total_structs: 0,
            total_enums: 0,
            total_impls: 0,
            total_traits: 0,
            total_mods: 0,
            total_comments: 0,
            avg_function_length: 0.0,
            max_function_length: 0,
            cyclomatic_complexity: 0,
            cognitive_complexity: 0,
            nesting_depth: 0,
        }
    }
}

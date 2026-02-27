use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationTarget {
    pub file: String,
    pub function_name: String,
    pub extra: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MutationOp {
    AddCaching,
    AddLogging,
    AddEarlyReturn,
    InlineConstant { name: String, value: f64 },
    /// DGM §8.3 — structural mutations: add a new function to a file.
    AddFunction {
        function_source: String,
    },
    /// Replace the body of an existing function with new implementation.
    ReplaceImplementation {
        new_body: String,
    },
    /// Remove dead code (functions that are never called).
    RemoveDeadCode,
    /// Refactor: extract a block of logic into a new helper function.
    RefactorExtract {
        new_fn_name: String,
        extracted_lines: String,
    },
    /// LLM-driven whole-source replacement: the code_change payload IS the new
    /// file content. The AST engine writes it directly after safety checks.
    ReplaceSource {
        new_source: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicMutation {
    pub id: String,
    pub target: MutationTarget,
    pub op: MutationOp,
    pub rationale: String,
    pub confidence: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl AtomicMutation {
    pub fn new(target: MutationTarget, op: MutationOp, rationale: &str, confidence: f64) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            target,
            op,
            rationale: rationale.to_string(),
            confidence,
            created_at: chrono::Utc::now(),
        }
    }
}

/// Predefined safe atomic mutations that can be applied without human review.
pub fn safe_mutations_for_file(
    file: &str,
    fn_names: &[String],
) -> Vec<AtomicMutation> {
    let mut mutations = Vec::new();

    for fn_name in fn_names {
        // Suggest logging for all public functions
        if !fn_name.starts_with('_') {
            mutations.push(AtomicMutation::new(
                MutationTarget {
                    file: file.to_string(),
                    function_name: fn_name.clone(),
                    extra: None,
                },
                MutationOp::AddLogging,
                &format!("Add entry tracing to {} for observability", fn_name),
                0.9,
            ));
        }
        // Suggest caching for functions likely to be called repeatedly
        if fn_name.starts_with("get_")
            || fn_name.starts_with("compute_")
            || fn_name.starts_with("calculate_")
        {
            mutations.push(AtomicMutation::new(
                MutationTarget {
                    file: file.to_string(),
                    function_name: fn_name.clone(),
                    extra: None,
                },
                MutationOp::AddCaching,
                &format!("Add result caching to {} to reduce redundant computation", fn_name),
                0.75,
            ));
        }
    }

    mutations
}

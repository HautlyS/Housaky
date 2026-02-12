//! Mutation operators for code evolution

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of code mutations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationType {
    /// Add a new function
    AddFunction,
    /// Remove an existing function
    RemoveFunction,
    /// Modify function body
    ModifyFunction,
    /// Add a new struct/enum
    AddType,
    /// Add a new module
    AddModule,
    /// Change function signature
    ChangeSignature,
    /// Add trait implementation
    AddImpl,
    /// Modify import statements
    ModifyImports,
    /// Add documentation
    AddDocs,
}

/// A single code mutation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mutation {
    /// Type of mutation
    pub mutation_type: MutationType,
    /// Target file path
    pub file_path: String,
    /// Description of the change
    pub description: String,
    /// Original code (for context)
    pub original_code: Option<String>,
    /// New code
    pub new_code: String,
    /// Line numbers affected (start, end)
    pub line_range: Option<(usize, usize)>,
    /// Confidence score (0-1)
    pub confidence: f64,
}

impl Mutation {
    /// Create a new mutation
    pub fn new(
        mutation_type: MutationType,
        file_path: impl Into<String>,
        description: impl Into<String>,
        new_code: impl Into<String>,
    ) -> Self {
        Self {
            mutation_type,
            file_path: file_path.into(),
            description: description.into(),
            original_code: None,
            new_code: new_code.into(),
            line_range: None,
            confidence: 0.5,
        }
    }

    /// Set original code for context
    pub fn with_original(mut self, code: impl Into<String>) -> Self {
        self.original_code = Some(code.into());
        self
    }

    /// Set line range
    pub fn with_line_range(mut self, start: usize, end: usize) -> Self {
        self.line_range = Some((start, end));
        self
    }

    /// Set confidence score
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Validate the mutation
    pub fn validate(&self) -> Result<()> {
        if self.confidence < 0.1 {
            return Err(anyhow::anyhow!("Confidence too low"));
        }

        if self.new_code.is_empty() {
            return Err(anyhow::anyhow!("New code cannot be empty"));
        }

        Ok(())
    }

    /// Generate a diff representation
    pub fn to_diff(&self) -> String {
        let mut diff = format!("--- a/{}\n", self.file_path);
        diff.push_str(&format!("+++ b/{}\n", self.file_path));

        if let Some((start, end)) = self.line_range {
            diff.push_str(&format!(
                "@@ -{},{} +{},{} @@\n",
                start,
                end - start,
                start,
                end - start
            ));
        }

        if let Some(ref original) = self.original_code {
            for line in original.lines() {
                diff.push_str(&format!("-{}/n", line));
            }
        }

        for line in self.new_code.lines() {
            diff.push_str(&format!("+{}\n", line));
        }

        diff
    }
}

/// Mutation operator that applies changes
pub struct MutationOperator {
    /// Mutation rate (probability of applying a mutation)
    pub mutation_rate: f64,
    /// Maximum mutations per operation
    pub max_mutations: usize,
}

impl MutationOperator {
    /// Create a new mutation operator
    pub fn new(mutation_rate: f64, max_mutations: usize) -> Self {
        Self {
            mutation_rate: mutation_rate.clamp(0.0, 1.0),
            max_mutations,
        }
    }

    /// Generate mutations for a codebase
    pub fn generate_mutations(&self, codebase: &Codebase) -> Vec<Mutation> {
        let mut mutations = Vec::new();

        // Analyze codebase and suggest mutations
        for (path, content) in &codebase.files {
            // Look for functions that could be improved
            if content.contains("TODO") || content.contains("FIXME") {
                let mutation = Mutation::new(
                    MutationType::ModifyFunction,
                    path.clone(),
                    "Address TODO/FIXME comment",
                    "// TODO implemented",
                )
                .with_confidence(0.6);

                mutations.push(mutation);
            }
        }

        // Limit number of mutations
        mutations.truncate(self.max_mutations);
        mutations
    }

    /// Apply a mutation to the codebase
    pub fn apply_mutation(&self, codebase: &mut Codebase, mutation: &Mutation) -> Result<()> {
        mutation.validate()?;

        let content = codebase
            .files
            .get_mut(&mutation.file_path)
            .ok_or_else(|| anyhow::anyhow!("File not found: {}", mutation.file_path))?;

        if let Some((start, end)) = mutation.line_range {
            let lines: Vec<&str> = content.lines().collect();
            if end > lines.len() {
                return Err(anyhow::anyhow!("Invalid line range"));
            }

            let new_content = lines[..start].join("\n");
            let new_content = new_content + &mutation.new_code;
            let new_content = new_content + &lines[end..].join("\n");

            *content = new_content;
        } else {
            // Append to file
            content.push_str("\n");
            content.push_str(&mutation.new_code);
        }

        tracing::info!("Applied mutation to {}", mutation.file_path);
        Ok(())
    }
}

/// Represents a codebase
#[derive(Debug, Clone)]
pub struct Codebase {
    /// Map of file paths to contents
    pub files: HashMap<String, String>,
}

impl Codebase {
    /// Create a new empty codebase
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// Add a file to the codebase
    pub fn add_file(&mut self, path: impl Into<String>, content: impl Into<String>) {
        self.files.insert(path.into(), content.into());
    }

    /// Get file content
    pub fn get_file(&self, path: &str) -> Option<&String> {
        self.files.get(path)
    }
}

impl Default for Codebase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutation_creation() {
        let mutation = Mutation::new(
            MutationType::AddFunction,
            "src/lib.rs",
            "Add helper function",
            "fn helper() {}",
        )
        .with_confidence(0.8);

        assert_eq!(mutation.mutation_type, MutationType::AddFunction);
        assert!(mutation.confidence > 0.5);
    }

    #[test]
    fn test_codebase_operations() {
        let mut codebase = Codebase::new();
        codebase.add_file("src/main.rs", "fn main() {}");

        assert!(codebase.get_file("src/main.rs").is_some());
        assert!(codebase.get_file("missing.rs").is_none());
    }

    #[test]
    fn test_mutation_validation() {
        let mutation = Mutation::new(MutationType::AddFunction, "test.rs", "test", "fn test() {}")
            .with_confidence(0.9);

        assert!(mutation.validate().is_ok());
    }
}

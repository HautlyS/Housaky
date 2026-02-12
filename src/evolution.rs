//! Code evolution and self-improvement

use std::process::Command;

pub struct CodeEvolver {
    sandbox_enabled: bool,
}

impl CodeEvolver {
    pub fn new() -> Self {
        Self {
            sandbox_enabled: Self::check_docker(),
        }
    }

    fn check_docker() -> bool {
        Command::new("docker")
            .arg("--version")
            .output()
            .is_ok()
    }

    /// Evaluate code fitness
    pub fn evaluate_fitness(&self, code: &str) -> f64 {
        let mut score = 0.0;

        // Simple heuristics
        if code.contains("async") {
            score += 10.0;
        }
        if code.contains("parallel") || code.contains("rayon") {
            score += 15.0;
        }
        if code.contains("test") {
            score += 20.0;
        }
        if code.len() < 1000 {
            score += 10.0; // Prefer concise code
        }

        score
    }

    /// Apply simple mutation
    pub fn mutate(&self, code: &str) -> String {
        // Simple mutations
        let mutations = vec![
            ("Vec<", "Vec<"),
            ("fn ", "pub fn "),
            ("let ", "let mut "),
        ];

        let mut mutated = code.to_string();
        if let Some((from, to)) = mutations.get(rand::random::<usize>() % mutations.len()) {
            if let Some(pos) = mutated.find(from) {
                mutated.replace_range(pos..pos + from.len(), to);
            }
        }

        mutated
    }

    /// Execute code in sandbox (if Docker available)
    pub async fn execute_sandboxed(&self, code: &str) -> Result<String, String> {
        if !self.sandbox_enabled {
            return Err("Docker not available for sandboxing".to_string());
        }

        // Create temporary Rust file
        let temp_code = format!(
            r#"
fn main() {{
    {}
}}
"#,
            code
        );

        // In production, would use Docker to run this safely
        Ok(format!("Sandboxed execution: {} bytes", temp_code.len()))
    }

    pub fn is_sandbox_enabled(&self) -> bool {
        self.sandbox_enabled
    }
}

impl Default for CodeEvolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fitness() {
        let evolver = CodeEvolver::new();
        let code = "async fn test() { rayon::parallel(); }";
        let fitness = evolver.evaluate_fitness(code);
        assert!(fitness > 0.0);
    }

    #[test]
    fn test_mutation() {
        let evolver = CodeEvolver::new();
        let code = "fn test() {}";
        let mutated = evolver.mutate(code);
        assert!(!mutated.is_empty());
    }
}

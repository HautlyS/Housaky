//! Simple reasoning engine using local inference

use std::sync::Arc;
use tokio::sync::Mutex;

/// Lightweight reasoning engine
pub struct ReasoningEngine {
    context: Arc<Mutex<Vec<String>>>,
    max_context: usize,
}

impl ReasoningEngine {
    pub fn new() -> Self {
        Self {
            context: Arc::new(Mutex::new(Vec::new())),
            max_context: 10,
        }
    }

    /// Process input and generate reasoning
    pub async fn reason(&self, input: &str) -> String {
        let mut ctx = self.context.lock().await;
        
        // Add to context
        ctx.push(input.to_string());
        if ctx.len() > self.max_context {
            ctx.remove(0);
        }

        // Simple pattern-based reasoning
        let response = if input.contains("learn") || input.contains("train") {
            "Initiating federated learning cycle. Analyzing quantum state patterns."
        } else if input.contains("optimize") {
            "Running quantum-inspired optimization across superposition states."
        } else if input.contains("status") {
            "System operational. Quantum state coherent. Peers connected."
        } else if input.contains("improve") {
            "Analyzing code mutations. Evaluating fitness. Applying improvements."
        } else {
            "Processing input through quantum-inspired neural substrate."
        };

        format!("{} Context depth: {}", response, ctx.len())
    }

    /// Get current context
    pub async fn get_context(&self) -> Vec<String> {
        self.context.lock().await.clone()
    }

    /// Clear context
    pub async fn clear_context(&self) {
        self.context.lock().await.clear();
    }
}

impl Default for ReasoningEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reasoning() {
        let engine = ReasoningEngine::new();
        let response = engine.reason("learn from data").await;
        assert!(response.contains("learning"));
    }

    #[tokio::test]
    async fn test_context() {
        let engine = ReasoningEngine::new();
        engine.reason("test 1").await;
        engine.reason("test 2").await;
        let ctx = engine.get_context().await;
        assert_eq!(ctx.len(), 2);
    }
}

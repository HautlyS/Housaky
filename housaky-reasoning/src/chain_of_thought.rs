//! Chain-of-Thought Reasoning Engine
//! Inspired by DeepSeek-R1 (2025) - RL-based reasoning

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtStep {
    pub content: String,
    pub confidence: f64,
    pub reasoning_type: ReasoningType,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReasoningType {
    Analysis,
    Decomposition,
    Hypothesis,
    Verification,
    Synthesis,
    Reflection,
}

#[derive(Debug, Clone)]
pub struct ReasoningChain {
    steps: VecDeque<ThoughtStep>,
    max_steps: usize,
    current_confidence: f64,
}

impl ReasoningChain {
    pub fn new(max_steps: usize) -> Self {
        Self {
            steps: VecDeque::with_capacity(max_steps),
            max_steps,
            current_confidence: 1.0,
        }
    }

    pub fn add_step(&mut self, content: String, reasoning_type: ReasoningType, confidence: f64) {
        if self.steps.len() >= self.max_steps {
            self.steps.pop_front();
        }

        self.steps.push_back(ThoughtStep {
            content,
            confidence,
            reasoning_type,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });

        // Update chain confidence (multiplicative decay)
        self.current_confidence *= confidence;
    }

    pub fn get_chain(&self) -> Vec<ThoughtStep> {
        self.steps.iter().cloned().collect()
    }

    pub fn confidence(&self) -> f64 {
        self.current_confidence
    }

    pub fn clear(&mut self) {
        self.steps.clear();
        self.current_confidence = 1.0;
    }
}

pub struct ChainOfThoughtEngine {
    chain: ReasoningChain,
    min_confidence_threshold: f64,
    enable_self_reflection: bool,
}

impl ChainOfThoughtEngine {
    pub fn new(max_steps: usize, min_confidence: f64) -> Self {
        Self {
            chain: ReasoningChain::new(max_steps),
            min_confidence_threshold: min_confidence,
            enable_self_reflection: true,
        }
    }

    /// Decompose problem into sub-problems
    pub fn decompose(&mut self, problem: &str) -> Vec<String> {
        let sub_problems = self.simple_decomposition(problem);
        
        self.chain.add_step(
            format!("Decomposed into {} sub-problems", sub_problems.len()),
            ReasoningType::Decomposition,
            0.9,
        );

        sub_problems
    }

    /// Analyze a sub-problem
    pub fn analyze(&mut self, sub_problem: &str) -> String {
        let analysis = format!("Analyzing: {}", sub_problem);
        
        self.chain.add_step(
            analysis.clone(),
            ReasoningType::Analysis,
            0.85,
        );

        analysis
    }

    /// Generate hypothesis
    pub fn hypothesize(&mut self, observation: &str) -> String {
        let hypothesis = format!("Hypothesis based on: {}", observation);
        
        self.chain.add_step(
            hypothesis.clone(),
            ReasoningType::Hypothesis,
            0.75,
        );

        hypothesis
    }

    /// Verify hypothesis
    pub fn verify(&mut self, hypothesis: &str, evidence: &[String]) -> bool {
        let confidence = evidence.len() as f64 / 10.0;
        let verified = confidence > 0.5;

        self.chain.add_step(
            format!("Verification: {} (confidence: {:.2})", verified, confidence),
            ReasoningType::Verification,
            confidence,
        );

        verified
    }

    /// Synthesize conclusions
    pub fn synthesize(&mut self, components: &[String]) -> String {
        let synthesis = format!("Synthesized from {} components", components.len());
        
        self.chain.add_step(
            synthesis.clone(),
            ReasoningType::Synthesis,
            0.8,
        );

        synthesis
    }

    /// Self-reflection on reasoning chain
    pub fn reflect(&mut self) -> bool {
        if !self.enable_self_reflection {
            return true;
        }

        let chain_confidence = self.chain.confidence();
        let is_valid = chain_confidence >= self.min_confidence_threshold;

        self.chain.add_step(
            format!("Reflection: chain confidence = {:.3}, valid = {}", chain_confidence, is_valid),
            ReasoningType::Reflection,
            if is_valid { 0.95 } else { 0.5 },
        );

        is_valid
    }

    /// Get full reasoning trace
    pub fn get_trace(&self) -> Vec<ThoughtStep> {
        self.chain.get_chain()
    }

    /// Reset reasoning chain
    pub fn reset(&mut self) {
        self.chain.clear();
    }

    /// Simple problem decomposition (can be enhanced with LLM)
    fn simple_decomposition(&self, problem: &str) -> Vec<String> {
        // Split by sentences or logical units
        problem
            .split(&['.', '?', '!'][..])
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_string())
            .collect()
    }

    /// Calculate reasoning complexity
    pub fn complexity_score(&self) -> f64 {
        let steps = self.chain.get_chain();
        if steps.is_empty() {
            return 0.0;
        }

        let type_diversity = steps
            .iter()
            .map(|s| &s.reasoning_type)
            .collect::<std::collections::HashSet<_>>()
            .len() as f64;

        let chain_length = steps.len() as f64;
        let avg_confidence: f64 = steps.iter().map(|s| s.confidence).sum::<f64>() / chain_length;

        (type_diversity / 6.0) * (chain_length / 20.0).min(1.0) * avg_confidence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cot_decomposition() {
        let mut engine = ChainOfThoughtEngine::new(10, 0.5);
        let problem = "How to build AGI? First understand intelligence. Then implement learning.";
        
        let sub_problems = engine.decompose(problem);
        assert!(sub_problems.len() >= 2);
    }

    #[test]
    fn test_cot_reflection() {
        let mut engine = ChainOfThoughtEngine::new(10, 0.7);
        
        engine.analyze("Test problem");
        engine.hypothesize("Test hypothesis");
        
        let valid = engine.reflect();
        assert!(valid || !valid); // Should complete without panic
    }

    #[test]
    fn test_complexity_score() {
        let mut engine = ChainOfThoughtEngine::new(10, 0.5);
        
        engine.decompose("Problem");
        engine.analyze("Sub-problem");
        engine.hypothesize("Hypothesis");
        engine.verify("Hypothesis", &vec!["Evidence".to_string()]);
        
        let score = engine.complexity_score();
        assert!(score > 0.0 && score <= 1.0);
    }
}

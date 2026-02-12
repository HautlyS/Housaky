//! Meta-Reasoning - Reasoning about reasoning
//! Self-awareness and cognitive monitoring

use crate::chain_of_thought::{ChainOfThoughtEngine, ReasoningType};
use crate::world_model::WorldModel;

pub struct MetaReasoner {
    cot_engine: ChainOfThoughtEngine,
    world_model: WorldModel,
    reasoning_history: Vec<ReasoningMetrics>,
    self_improvement_threshold: f64,
}

#[derive(Debug, Clone)]
pub struct ReasoningMetrics {
    pub complexity: f64,
    pub confidence: f64,
    pub coherence: f64,
    pub timestamp: u64,
}

impl MetaReasoner {
    pub fn new() -> Self {
        Self {
            cot_engine: ChainOfThoughtEngine::new(20, 0.6),
            world_model: WorldModel::new(100, 10),
            reasoning_history: Vec::new(),
            self_improvement_threshold: 0.7,
        }
    }

    /// Evaluate own reasoning quality
    pub fn evaluate_reasoning(&mut self) -> ReasoningMetrics {
        let complexity = self.cot_engine.complexity_score();
        let confidence = self.cot_engine.get_trace()
            .iter()
            .map(|s| s.confidence)
            .sum::<f64>() / self.cot_engine.get_trace().len().max(1) as f64;
        let coherence = self.world_model.coherence_score();

        let metrics = ReasoningMetrics {
            complexity,
            confidence,
            coherence,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        self.reasoning_history.push(metrics.clone());
        metrics
    }

    /// Detect if reasoning needs improvement
    pub fn needs_improvement(&self) -> bool {
        if self.reasoning_history.len() < 3 {
            return false;
        }

        let recent = &self.reasoning_history[self.reasoning_history.len() - 3..];
        let avg_quality = recent.iter()
            .map(|m| (m.complexity + m.confidence + m.coherence) / 3.0)
            .sum::<f64>() / recent.len() as f64;

        avg_quality < self.self_improvement_threshold
    }

    /// Suggest reasoning strategy adjustment
    pub fn suggest_strategy(&self) -> String {
        let metrics = self.evaluate_current_state();

        if metrics.complexity < 0.3 {
            "Increase reasoning depth - decompose problems further".to_string()
        } else if metrics.confidence < 0.5 {
            "Gather more evidence before conclusions".to_string()
        } else if metrics.coherence < 0.6 {
            "Improve world model consistency - verify assumptions".to_string()
        } else {
            "Reasoning quality is good - continue current strategy".to_string()
        }
    }

    fn evaluate_current_state(&self) -> ReasoningMetrics {
        if let Some(last) = self.reasoning_history.last() {
            last.clone()
        } else {
            ReasoningMetrics {
                complexity: 0.5,
                confidence: 0.5,
                coherence: 0.5,
                timestamp: 0,
            }
        }
    }

    /// Get reasoning engine
    pub fn cot_engine(&mut self) -> &mut ChainOfThoughtEngine {
        &mut self.cot_engine
    }

    /// Get world model
    pub fn world_model(&mut self) -> &mut WorldModel {
        &mut self.world_model
    }
}

impl Default for MetaReasoner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_reasoning() {
        let mut reasoner = MetaReasoner::new();
        
        reasoner.cot_engine().decompose("Test problem");
        reasoner.cot_engine().analyze("Sub-problem");
        
        let metrics = reasoner.evaluate_reasoning();
        assert!(metrics.complexity >= 0.0);
    }

    #[test]
    fn test_improvement_detection() {
        let mut reasoner = MetaReasoner::new();
        
        // Simulate poor reasoning
        for _ in 0..5 {
            reasoner.reasoning_history.push(ReasoningMetrics {
                complexity: 0.2,
                confidence: 0.3,
                coherence: 0.4,
                timestamp: 0,
            });
        }

        assert!(reasoner.needs_improvement());
    }
}

//! Inference engine for RLM

use anyhow::Result;
use candle_core::{DType, Device, Tensor};
use std::sync::Arc;

use crate::model::LanguageModel;
use crate::tokenizer::Tokenizer;

/// Inference configuration
#[derive(Debug, Clone)]
pub struct InferenceConfig {
    /// Maximum number of tokens to generate
    pub max_tokens: usize,
    /// Temperature for sampling
    pub temperature: f64,
    /// Top-p (nucleus) sampling
    pub top_p: f64,
    /// Top-k sampling
    pub top_k: usize,
    /// Repetition penalty
    pub repetition_penalty: f64,
    /// Stop sequences
    pub stop_sequences: Vec<String>,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            max_tokens: 256,
            temperature: 0.7,
            top_p: 0.9,
            top_k: 50,
            repetition_penalty: 1.0,
            stop_sequences: vec![],
        }
    }
}

/// Inference engine for running models
pub struct InferenceEngine {
    model: Arc<LanguageModel>,
    tokenizer: Tokenizer,
    config: InferenceConfig,
}

impl InferenceEngine {
    /// Create a new inference engine
    pub fn new(model: Arc<LanguageModel>, tokenizer: Tokenizer, config: InferenceConfig) -> Self {
        Self {
            model,
            tokenizer,
            config,
        }
    }

    /// Generate text from a prompt
    pub fn generate(&self, prompt: &str) -> Result<String> {
        let input_tokens = self.tokenizer.encode(prompt, true);
        let mut generated_tokens = input_tokens.clone();

        for _ in 0..self.config.max_tokens {
            // Convert tokens to tensor
            let input_tensor = self.tokens_to_tensor(&generated_tokens)?;

            // Run model forward pass
            let logits = self.model.forward(&input_tensor)?;

            // Sample next token
            let next_token = self.sample_token(&logits, &generated_tokens)?;

            // Check for stop conditions
            if next_token == self.tokenizer.special_tokens().eos {
                break;
            }

            generated_tokens.push(next_token);

            // Check stop sequences
            let current_text = self.tokenizer.decode(&generated_tokens, true);
            for stop_seq in &self.config.stop_sequences {
                if current_text.ends_with(stop_seq) {
                    return Ok(current_text);
                }
            }
        }

        let output = self.tokenizer.decode(&generated_tokens, true);
        Ok(output)
    }

    /// Generate with streaming output
    pub async fn generate_stream<F>(&self, prompt: &str, mut callback: F) -> Result<()>
    where
        F: FnMut(String) -> Result<()>,
    {
        let input_tokens = self.tokenizer.encode(prompt, true);
        let mut generated_tokens = input_tokens.clone();
        let mut previous_text = String::new();

        for _ in 0..self.config.max_tokens {
            let input_tensor = self.tokens_to_tensor(&generated_tokens)?;
            let logits = self.model.forward(&input_tensor)?;
            let next_token = self.sample_token(&logits, &generated_tokens)?;

            if next_token == self.tokenizer.special_tokens().eos {
                break;
            }

            generated_tokens.push(next_token);

            let current_text = self.tokenizer.decode(&generated_tokens, true);
            let new_text = &current_text[previous_text.len()..];

            callback(new_text.to_string())?;
            previous_text = current_text;

            // Check stop sequences
            for stop_seq in &self.config.stop_sequences {
                if previous_text.ends_with(stop_seq) {
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    /// Convert token IDs to tensor
    fn tokens_to_tensor(&self, tokens: &[u32]) -> Result<Tensor> {
        let device = self.model.device();
        let data: Vec<i64> = tokens.iter().map(|&t| t as i64).collect();

        Tensor::from_vec(data, &[1, tokens.len()], device)
            .map_err(|e| anyhow::anyhow!("Failed to create tensor: {}", e))
    }

    /// Sample next token from logits
    fn sample_token(&self, logits: &Tensor, _context: &[u32]) -> Result<u32> {
        use rand::distributions::{Distribution, WeightedIndex};

        // Get logits as vector
        let logits_vec: Vec<f32> = logits.to_vec1()?;

        // Apply temperature
        let temperature = self.config.temperature as f32;
        let scaled_logits: Vec<f32> = logits_vec.iter().map(|&l| l / temperature).collect();

        // Convert to probabilities with softmax
        let max_logit = scaled_logits
            .iter()
            .fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let exp_logits: Vec<f32> = scaled_logits
            .iter()
            .map(|&l| (l - max_logit).exp())
            .collect();
        let sum_exp: f32 = exp_logits.iter().sum();
        let probs: Vec<f64> = exp_logits.iter().map(|&e| (e / sum_exp) as f64).collect();

        // Apply top-k filtering
        let k = self.config.top_k.min(probs.len());
        let mut indexed_probs: Vec<(usize, f64)> =
            probs.iter().enumerate().map(|(i, &p)| (i, p)).collect();
        indexed_probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        indexed_probs.truncate(k);

        // Sample from top-k
        let weights: Vec<f64> = indexed_probs.iter().map(|(_, p)| *p).collect();
        let dist = WeightedIndex::new(&weights)?;
        let mut rng = rand::thread_rng();
        let sampled_idx = dist.sample(&mut rng);
        let token_id = indexed_probs[sampled_idx].0 as u32;

        Ok(token_id)
    }

    /// Get model info
    pub fn model_info(&self) -> String {
        format!(
            "Model: {} parameters\nTokenizer: {} tokens",
            self.model.num_parameters(),
            self.tokenizer.vocab_size()
        )
    }
}

/// Batch inference for efficiency
pub struct BatchInference {
    engine: Arc<InferenceEngine>,
    batch_size: usize,
}

impl BatchInference {
    /// Create a new batch inference handler
    pub fn new(engine: Arc<InferenceEngine>, batch_size: usize) -> Self {
        Self { engine, batch_size }
    }

    /// Process multiple prompts in batches
    pub fn generate_batch(&self, prompts: &[String]) -> Vec<Result<String>> {
        prompts
            .iter()
            .map(|prompt| self.engine.generate(prompt))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ModelConfig;

    fn create_test_engine() -> InferenceEngine {
        let config = ModelConfig::default();
        let model = Arc::new(LanguageModel::new(config).unwrap());
        let tokenizer = Tokenizer::default();
        let inference_config = InferenceConfig::default();

        InferenceEngine::new(model, tokenizer, inference_config)
    }

    #[test]
    fn test_inference_config() {
        let config = InferenceConfig::default();
        assert_eq!(config.max_tokens, 256);
        assert!(config.temperature > 0.0);
    }

    #[test]
    fn test_engine_info() {
        let engine = create_test_engine();
        let info = engine.model_info();
        assert!(info.contains("parameters"));
    }
}

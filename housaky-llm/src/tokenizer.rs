//! Tokenizer - HuggingFace tokenizers integration

use anyhow::{Result, anyhow};
use std::collections::HashMap;

pub struct Tokenizer {
    vocab: HashMap<String, u32>,
    reverse_vocab: HashMap<u32, String>,
    vocab_size: usize,
}

impl Tokenizer {
    pub fn new(_model_path: &str) -> Result<Self> {
        // Simplified tokenizer - in production, use tokenizers crate
        let mut vocab = HashMap::new();
        let mut reverse_vocab = HashMap::new();
        
        // Basic vocab
        vocab.insert("<pad>".to_string(), 0);
        vocab.insert("<unk>".to_string(), 1);
        vocab.insert("<eos>".to_string(), 2);
        vocab.insert("<bos>".to_string(), 3);
        
        for (k, v) in &vocab {
            reverse_vocab.insert(*v, k.clone());
        }
        
        Ok(Self {
            vocab,
            reverse_vocab,
            vocab_size: 32000,
        })
    }

    pub fn encode(&self, text: &str) -> Result<Vec<u32>> {
        // Simplified encoding - character-level
        let tokens: Vec<u32> = text
            .chars()
            .map(|c| (c as u32) % self.vocab_size as u32)
            .collect();
        
        Ok(tokens)
    }

    pub fn decode(&self, tokens: &[u32]) -> Result<String> {
        // Simplified decoding
        let text: String = tokens
            .iter()
            .filter_map(|&t| {
                if t < 128 {
                    Some(t as u8 as char)
                } else {
                    None
                }
            })
            .collect();
        
        Ok(text)
    }

    pub fn vocab_size(&self) -> usize {
        self.vocab_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer() {
        let tokenizer = Tokenizer::new("dummy").unwrap();
        let tokens = tokenizer.encode("Hello").unwrap();
        assert!(!tokens.is_empty());
    }
}

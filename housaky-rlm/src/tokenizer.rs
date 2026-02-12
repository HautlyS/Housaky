//! Tokenization for RLM

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A tokenizer for converting text to tokens and back
#[derive(Debug, Clone)]
pub struct Tokenizer {
    vocab: HashMap<String, u32>,
    reverse_vocab: HashMap<u32, String>,
    special_tokens: SpecialTokens,
}

/// Special tokens used by the tokenizer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialTokens {
    pub pad: u32,
    pub eos: u32,
    pub bos: u32,
    pub unk: u32,
    pub mask: u32,
}

impl Default for SpecialTokens {
    fn default() -> Self {
        Self {
            pad: 0,
            eos: 1,
            bos: 2,
            unk: 3,
            mask: 4,
        }
    }
}

impl Tokenizer {
    /// Create a new empty tokenizer
    pub fn new(special_tokens: SpecialTokens) -> Self {
        let mut vocab = HashMap::new();
        let mut reverse_vocab = HashMap::new();

        // Add special tokens
        vocab.insert("<pad>".to_string(), special_tokens.pad);
        vocab.insert("<eos>".to_string(), special_tokens.eos);
        vocab.insert("<bos>".to_string(), special_tokens.bos);
        vocab.insert("<unk>".to_string(), special_tokens.unk);
        vocab.insert("<mask>".to_string(), special_tokens.mask);

        reverse_vocab.insert(special_tokens.pad, "<pad>".to_string());
        reverse_vocab.insert(special_tokens.eos, "<eos>".to_string());
        reverse_vocab.insert(special_tokens.bos, "<bos>".to_string());
        reverse_vocab.insert(special_tokens.unk, "<unk>".to_string());
        reverse_vocab.insert(special_tokens.mask, "<mask>".to_string());

        Self {
            vocab,
            reverse_vocab,
            special_tokens,
        }
    }

    /// Load tokenizer from vocabulary file
    pub fn from_vocab(vocab: HashMap<String, u32>) -> Self {
        let special_tokens = SpecialTokens::default();
        let mut reverse_vocab = HashMap::new();

        for (token, id) in &vocab {
            reverse_vocab.insert(*id, token.clone());
        }

        Self {
            vocab,
            reverse_vocab,
            special_tokens,
        }
    }

    /// Encode text to token IDs
    pub fn encode(&self, text: &str, add_special_tokens: bool) -> Vec<u32> {
        let mut tokens = Vec::new();

        if add_special_tokens {
            tokens.push(self.special_tokens.bos);
        }

        // Simple word-based tokenization (in practice, use BPE or SentencePiece)
        for word in text.split_whitespace() {
            let token_id = self
                .vocab
                .get(word)
                .copied()
                .unwrap_or(self.special_tokens.unk);
            tokens.push(token_id);
        }

        if add_special_tokens {
            tokens.push(self.special_tokens.eos);
        }

        tokens
    }

    /// Decode token IDs to text
    pub fn decode(&self, tokens: &[u32], skip_special_tokens: bool) -> String {
        let mut words = Vec::new();

        for &token in tokens {
            if skip_special_tokens && self.is_special_token(token) {
                continue;
            }

            if let Some(word) = self.reverse_vocab.get(&token) {
                words.push(word.as_str());
            } else {
                words.push("<unk>");
            }
        }

        words.join(" ")
    }

    /// Check if a token is special
    fn is_special_token(&self, token: u32) -> bool {
        token == self.special_tokens.pad
            || token == self.special_tokens.eos
            || token == self.special_tokens.bos
            || token == self.special_tokens.unk
            || token == self.special_tokens.mask
    }

    /// Get vocabulary size
    pub fn vocab_size(&self) -> usize {
        self.vocab.len()
    }

    /// Add a token to the vocabulary
    pub fn add_token(&mut self, token: String) -> u32 {
        if let Some(&id) = self.vocab.get(&token) {
            return id;
        }

        let new_id = self.vocab.len() as u32;
        self.vocab.insert(token.clone(), new_id);
        self.reverse_vocab.insert(new_id, token);
        new_id
    }

    /// Get special tokens
    pub fn special_tokens(&self) -> &SpecialTokens {
        &self.special_tokens
    }
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::new(SpecialTokens::default())
    }
}

/// Build a simple word-level tokenizer from text corpus
pub fn build_tokenizer(texts: &[String], vocab_size: usize) -> Tokenizer {
    use std::collections::HashSet;

    let mut token_counts: HashMap<String, usize> = HashMap::new();

    // Count token frequencies
    for text in texts {
        for word in text.split_whitespace() {
            *token_counts.entry(word.to_lowercase()).or_insert(0) += 1;
        }
    }

    // Sort by frequency and take top vocab_size
    let mut tokens: Vec<(String, usize)> = token_counts.into_iter().collect();
    tokens.sort_by(|a, b| b.1.cmp(&a.1));

    let special_tokens = SpecialTokens::default();
    let mut tokenizer = Tokenizer::new(special_tokens);

    for (word, _) in tokens.into_iter().take(vocab_size) {
        tokenizer.add_token(word);
    }

    tokenizer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer_encode_decode() {
        let mut tokenizer = Tokenizer::default();
        tokenizer.add_token("hello".to_string());
        tokenizer.add_token("world".to_string());

        let tokens = tokenizer.encode("hello world", true);
        assert_eq!(tokens.len(), 4); // bos + hello + world + eos

        let decoded = tokenizer.decode(&tokens, true);
        assert!(decoded.contains("hello"));
        assert!(decoded.contains("world"));
    }

    #[test]
    fn test_special_tokens() {
        let tokenizer = Tokenizer::default();
        let special = tokenizer.special_tokens();

        assert_eq!(special.pad, 0);
        assert_eq!(special.eos, 1);
        assert_eq!(special.bos, 2);
    }
}

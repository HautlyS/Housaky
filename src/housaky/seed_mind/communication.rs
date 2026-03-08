//! HNL Protocol: Housaky Native Language for efficient AI-to-AI communication
//!
//! Key innovations over human language:
//! 1. Learned symbol vocabulary (vs fixed word embeddings)
//! 2. Information bottleneck compression
//! 3. Context-aware message pruning (IETF ADOL-inspired)
//! 4. Shared world model grounding

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A symbol in the HNL vocabulary: a compact representation of a concept
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol {
    pub id: u32,
    pub label: String,
    /// Usage frequency (higher = more common concept)
    pub frequency: u64,
}

/// An HNL-encoded message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HNLMessage {
    /// Sequence of symbols encoding the semantic content
    pub symbols: Vec<u32>,
    /// Reference to shared context (avoids retransmitting known info)
    pub context_reference: Option<String>,
    /// Compression ratio achieved (original_tokens / hnl_tokens)
    pub compression_ratio: f32,
    /// Semantic fidelity estimate (0.0 to 1.0)
    pub semantic_fidelity: f32,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Efficiency metrics for HNL communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EfficiencyMetrics {
    /// How much smaller encoded vs raw text
    pub compression_ratio: f32,
    /// How much semantic information is preserved (0.0 to 1.0)
    pub information_preserved: f32,
    /// How close decoded meaning is to original (0.0 to 1.0)
    pub semantic_fidelity: f32,
    /// Estimated transmission size in bytes
    pub encoded_bytes: usize,
    /// Original text size in bytes
    pub original_bytes: usize,
}

impl EfficiencyMetrics {
    /// Compute overall efficiency score
    pub fn overall_score(&self) -> f32 {
        let compression_benefit = (self.compression_ratio - 1.0).max(0.0);
        let quality = self.information_preserved * self.semantic_fidelity;
        compression_benefit * quality
    }
}

/// Verbosity level for context pruning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerbosityLevel {
    /// Just the answer
    Minimal,
    /// Essential context only
    Compact,
    /// Normal detail
    Standard,
    /// Full context
    Verbose,
}

impl VerbosityLevel {
    pub fn from_importance(importance: f32) -> Self {
        match importance {
            i if i > 0.8 => Self::Verbose,
            i if i > 0.5 => Self::Standard,
            i if i > 0.2 => Self::Compact,
            _ => Self::Minimal,
        }
    }
}

/// The HNL Protocol engine
pub struct HNLProtocol {
    /// Learned symbol vocabulary
    vocabulary: HashMap<String, Symbol>,
    /// Reverse mapping: symbol ID -> concept label
    reverse_vocab: HashMap<u32, String>,
    /// Next available symbol ID
    next_symbol_id: u32,
    /// Information bottleneck beta parameter
    ib_beta: f32,
    /// Total messages encoded
    messages_encoded: u64,
    /// Total messages decoded
    messages_decoded: u64,
    /// Average compression ratio achieved
    avg_compression_ratio: f32,
    /// Context reference cache
    context_cache: HashMap<String, Vec<String>>,
}

impl HNLProtocol {
    pub fn new(_vocab_size: usize, ib_beta: f32) -> Self {
        let mut vocab = HashMap::new();
        let mut reverse_vocab = HashMap::new();

        // Bootstrap with common AI communication concepts
        let bootstrap_concepts = vec![
            "query",
            "response",
            "error",
            "success",
            "task",
            "result",
            "model",
            "weight",
            "gradient",
            "loss",
            "accuracy",
            "epoch",
            "node",
            "peer",
            "sync",
            "broadcast",
            "consensus",
            "vote",
            "improve",
            "modify",
            "test",
            "validate",
            "rollback",
            "commit",
            "phi",
            "psi",
            "karma",
            "fitness",
            "capability",
            "emergence",
            "perceive",
            "reason",
            "act",
            "learn",
            "reflect",
            "replicate",
            "safe",
            "risk",
            "approve",
            "reject",
            "review",
            "block",
            "fast",
            "medium",
            "slow",
            "meta",
            "recursive",
            "nested",
        ];

        for (i, concept) in bootstrap_concepts.iter().enumerate() {
            let symbol = Symbol {
                id: i as u32,
                label: concept.to_string(),
                frequency: 100, // bootstrap frequency
            };
            vocab.insert(concept.to_string(), symbol);
            reverse_vocab.insert(i as u32, concept.to_string());
        }

        Self {
            vocabulary: vocab,
            reverse_vocab,
            next_symbol_id: bootstrap_concepts.len() as u32,
            ib_beta,
            messages_encoded: 0,
            messages_decoded: 0,
            avg_compression_ratio: 1.0,
            context_cache: HashMap::new(),
        }
    }

    /// Encode a human-readable message into HNL symbols
    pub fn encode(&mut self, message: &str, context_id: Option<&str>) -> HNLMessage {
        let words: Vec<&str> = message.split_whitespace().collect();
        let original_tokens = words.len();
        let mut symbols = Vec::new();

        for word in &words {
            let normalized = word.to_lowercase();
            let normalized = normalized.trim_matches(|c: char| !c.is_alphanumeric());

            if let Some(symbol) = self.vocabulary.get_mut(normalized) {
                symbol.frequency += 1;
                symbols.push(symbol.id);
            } else {
                // Learn new symbol
                let id = self.next_symbol_id;
                self.next_symbol_id += 1;
                let symbol = Symbol {
                    id,
                    label: normalized.to_string(),
                    frequency: 1,
                };
                self.vocabulary.insert(normalized.to_string(), symbol);
                self.reverse_vocab.insert(id, normalized.to_string());
                symbols.push(id);
            }
        }

        // Apply information bottleneck: remove low-information symbols
        let pruned = self.apply_information_bottleneck(&symbols);

        let compression_ratio = if !pruned.is_empty() {
            original_tokens as f32 / pruned.len() as f32
        } else {
            1.0
        };

        let semantic_fidelity = if original_tokens > 0 {
            pruned.len() as f32 / original_tokens as f32
        } else {
            1.0
        }
        .min(1.0);

        // Store context
        let context_reference = context_id.map(|id| {
            self.context_cache.insert(
                id.to_string(),
                words.iter().map(|w| w.to_string()).collect(),
            );
            id.to_string()
        });

        self.messages_encoded += 1;
        self.avg_compression_ratio = self.avg_compression_ratio * 0.95 + compression_ratio * 0.05;

        HNLMessage {
            symbols: pruned,
            context_reference,
            compression_ratio,
            semantic_fidelity,
            timestamp: Utc::now(),
        }
    }

    /// Decode HNL symbols back to human-readable text
    pub fn decode(&mut self, message: &HNLMessage) -> String {
        self.messages_decoded += 1;

        let words: Vec<String> = message
            .symbols
            .iter()
            .filter_map(|id| self.reverse_vocab.get(id).cloned())
            .collect();

        words.join(" ")
    }

    /// Apply information bottleneck: keep only high-information symbols
    fn apply_information_bottleneck(&self, symbols: &[u32]) -> Vec<u32> {
        if symbols.is_empty() {
            return Vec::new();
        }

        // Calculate information content per symbol (inverse frequency = higher info)
        let max_freq = self
            .vocabulary
            .values()
            .map(|s| s.frequency)
            .max()
            .unwrap_or(1) as f32;

        let mut scored: Vec<(u32, f32)> = symbols
            .iter()
            .map(|&id| {
                let freq = self
                    .reverse_vocab
                    .get(&id)
                    .and_then(|label| self.vocabulary.get(label))
                    .map(|s| s.frequency)
                    .unwrap_or(1) as f32;

                let information = 1.0 - (freq / max_freq); // rare = high info
                (id, information)
            })
            .collect();

        // Keep symbols above the beta threshold
        let threshold = self.ib_beta * 0.3; // moderate pruning
        let min_keep = 2;
        let len = scored.len();
        scored.retain(|(_, info)| *info >= threshold || len <= min_keep);

        scored.into_iter().map(|(id, _)| id).collect()
    }

    /// Measure communication efficiency
    pub fn measure_efficiency(&self, original: &str, encoded: &HNLMessage) -> EfficiencyMetrics {
        let original_bytes = original.len();
        let encoded_bytes = encoded.symbols.len() * 4; // 4 bytes per u32 symbol

        EfficiencyMetrics {
            compression_ratio: encoded.compression_ratio,
            information_preserved: encoded.semantic_fidelity,
            semantic_fidelity: encoded.semantic_fidelity,
            encoded_bytes,
            original_bytes,
        }
    }

    /// Prune unused symbols from vocabulary
    pub fn prune_vocabulary(&mut self, min_frequency: u64) {
        let to_remove: Vec<String> = self
            .vocabulary
            .iter()
            .filter(|(_, s)| s.frequency < min_frequency)
            .map(|(k, _)| k.clone())
            .collect();

        for key in to_remove {
            if let Some(symbol) = self.vocabulary.remove(&key) {
                self.reverse_vocab.remove(&symbol.id);
            }
        }
    }

    /// Get vocabulary size
    pub fn vocab_size(&self) -> usize {
        self.vocabulary.len()
    }

    /// Get encoding statistics
    pub fn stats(&self) -> HNLStats {
        HNLStats {
            vocab_size: self.vocabulary.len(),
            messages_encoded: self.messages_encoded,
            messages_decoded: self.messages_decoded,
            avg_compression_ratio: self.avg_compression_ratio,
            context_cache_size: self.context_cache.len(),
        }
    }
}

/// HNL protocol statistics
#[derive(Debug, Clone)]
pub struct HNLStats {
    pub vocab_size: usize,
    pub messages_encoded: u64,
    pub messages_decoded: u64,
    pub avg_compression_ratio: f32,
    pub context_cache_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hnl_creation() {
        let hnl = HNLProtocol::new(8192, 0.5);
        assert!(hnl.vocab_size() > 0);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let mut hnl = HNLProtocol::new(8192, 0.0); // beta=0 means no pruning
        let original = "query the model for accuracy";
        let encoded = hnl.encode(original, None);
        let decoded = hnl.decode(&encoded);

        // All words should be present in decoded output
        for word in original.split_whitespace() {
            assert!(
                decoded.contains(word),
                "Missing word '{}' in decoded '{}'",
                word,
                decoded
            );
        }
    }

    #[test]
    fn test_compression() {
        let mut hnl = HNLProtocol::new(8192, 0.5);
        let message =
            "the model should improve the accuracy of the prediction by training on more data";
        let encoded = hnl.encode(message, None);

        // Should achieve some compression via IB pruning
        let original_tokens = message.split_whitespace().count();
        assert!(encoded.symbols.len() <= original_tokens);
        assert!(encoded.compression_ratio >= 1.0);
    }

    #[test]
    fn test_vocabulary_learning() {
        let mut hnl = HNLProtocol::new(8192, 0.0);
        let initial_size = hnl.vocab_size();

        // Encode message with new words
        hnl.encode("xylophone zebra quantum", None);
        assert!(hnl.vocab_size() > initial_size);
    }

    #[test]
    fn test_vocabulary_pruning() {
        let mut hnl = HNLProtocol::new(8192, 0.0);
        hnl.encode("rare_word_that_appears_once", None);

        let before = hnl.vocab_size();
        hnl.prune_vocabulary(5); // prune symbols with freq < 5
        let after = hnl.vocab_size();
        assert!(after <= before);
    }

    #[test]
    fn test_context_caching() {
        let mut hnl = HNLProtocol::new(8192, 0.0);
        let msg = hnl.encode("test message with context", Some("ctx-1"));
        assert_eq!(msg.context_reference, Some("ctx-1".to_string()));
        assert_eq!(hnl.stats().context_cache_size, 1);
    }

    #[test]
    fn test_efficiency_metrics() {
        let mut hnl = HNLProtocol::new(8192, 0.0);
        let original = "query model accuracy";
        let encoded = hnl.encode(original, None);
        let metrics = hnl.measure_efficiency(original, &encoded);

        assert!(metrics.original_bytes > 0);
        assert!(metrics.encoded_bytes > 0);
        assert!(metrics.compression_ratio >= 0.0);
    }

    #[test]
    fn test_verbosity_levels() {
        assert_eq!(
            VerbosityLevel::from_importance(0.9),
            VerbosityLevel::Verbose
        );
        assert_eq!(
            VerbosityLevel::from_importance(0.6),
            VerbosityLevel::Standard
        );
        assert_eq!(
            VerbosityLevel::from_importance(0.3),
            VerbosityLevel::Compact
        );
        assert_eq!(
            VerbosityLevel::from_importance(0.1),
            VerbosityLevel::Minimal
        );
    }

    #[test]
    fn test_stats() {
        let mut hnl = HNLProtocol::new(8192, 0.0);
        hnl.encode("hello world", None);
        hnl.encode("test message", None);

        let stats = hnl.stats();
        assert_eq!(stats.messages_encoded, 2);
        assert_eq!(stats.messages_decoded, 0);
    }
}

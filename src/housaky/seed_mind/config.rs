//! Seed Mind configuration

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Complete configuration for a Seed Mind instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedMindConfig {
    // --- Weight dimensions ---
    /// Fast weight parameter count (~1M, immediate reactions)
    pub fast_params: usize,
    /// Medium weight parameter count (~10M, learned skills)
    pub medium_params: usize,
    /// Slow weight parameter count (~100M, core reasoning)
    pub slow_params: usize,
    /// Meta weight parameter count (~1M, learning-to-learn)
    pub meta_params: usize,

    // --- Learning rates per timescale ---
    pub fast_lr: f64,
    pub medium_lr: f64,
    pub slow_lr: f64,
    pub meta_lr: f64,

    // --- Self-modification bounds ---
    /// Maximum modifications allowed per cycle
    pub max_modification_per_cycle: usize,
    /// Whether safety review is required for all modifications
    pub safety_review_required: bool,
    /// Maximum size of the DGM agent archive
    pub modification_archive_size: usize,

    // --- Safety thresholds ---
    /// Risk score threshold for auto-approval
    pub risk_auto_approve_threshold: f64,
    /// Risk score threshold requiring human review
    pub risk_human_review_threshold: f64,

    // --- Network ---
    /// Whether to share improvements with network peers
    pub peer_improvement_share: bool,
    /// Minimum fitness delta to broadcast an improvement
    pub improvement_broadcast_threshold: f64,
    /// Interval between learning fusion syncs (seconds)
    pub learning_fusion_interval_secs: u64,

    // --- Karma ---
    /// Base karma for compute contributions
    pub karma_compute_base: f64,
    /// Base karma for inference contributions
    pub karma_inference_base: f64,
    /// Base karma for validation contributions
    pub karma_validation_base: f64,
    /// Base karma for knowledge sharing
    pub karma_knowledge_base: f64,
    /// Base karma for code contributions
    pub karma_code_base: f64,

    // --- Communication ---
    /// HNL symbol vocabulary size
    pub hnl_vocab_size: usize,
    /// Information bottleneck beta (compression vs info trade-off)
    pub hnl_ib_beta: f32,

    // --- Consciousness ---
    /// Minimum phi threshold for self-aware state
    pub consciousness_phi_threshold: f64,
    /// Emergence detection: minimum ratio of collective/sum for detection
    pub emergence_ratio_threshold: f64,
}

impl Default for SeedMindConfig {
    fn default() -> Self {
        Self {
            fast_params: 1_000_000,
            medium_params: 10_000_000,
            slow_params: 100_000_000,
            meta_params: 1_000_000,

            fast_lr: 0.1,
            medium_lr: 0.01,
            slow_lr: 0.001,
            meta_lr: 0.0001,

            max_modification_per_cycle: 100,
            safety_review_required: true,
            modification_archive_size: 10_000,

            risk_auto_approve_threshold: 0.1,
            risk_human_review_threshold: 0.3,

            peer_improvement_share: true,
            improvement_broadcast_threshold: 0.5,
            learning_fusion_interval_secs: 60,

            karma_compute_base: 10.0,
            karma_inference_base: 1.0,
            karma_validation_base: 5.0,
            karma_knowledge_base: 3.0,
            karma_code_base: 15.0,

            hnl_vocab_size: 8192,
            hnl_ib_beta: 0.5,

            consciousness_phi_threshold: 0.7,
            emergence_ratio_threshold: 1.2,
        }
    }
}

impl SeedMindConfig {
    /// Load config from workspace directory, or return defaults
    pub async fn from_workspace(workspace: &Path) -> Self {
        let config_path = workspace.join(".housaky").join("seed_mind").join("config.json");
        if config_path.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&config_path).await {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    /// Total parameter count across all timescales
    pub fn total_params(&self) -> usize {
        self.fast_params + self.medium_params + self.slow_params + self.meta_params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = SeedMindConfig::default();
        assert_eq!(cfg.fast_params, 1_000_000);
        assert_eq!(cfg.medium_params, 10_000_000);
        assert_eq!(cfg.slow_params, 100_000_000);
        assert_eq!(cfg.meta_params, 1_000_000);
        assert_eq!(cfg.total_params(), 112_000_000);
    }

    #[test]
    fn test_config_serialization() {
        let cfg = SeedMindConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let deserialized: SeedMindConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.fast_params, cfg.fast_params);
        assert_eq!(deserialized.slow_lr, cfg.slow_lr);
        assert_eq!(deserialized.hnl_vocab_size, cfg.hnl_vocab_size);
    }

    #[test]
    fn test_safety_thresholds() {
        let cfg = SeedMindConfig::default();
        assert!(cfg.risk_auto_approve_threshold < cfg.risk_human_review_threshold);
        assert!(cfg.risk_human_review_threshold <= 1.0);
    }
}

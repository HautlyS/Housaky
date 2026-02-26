//! Phenomenal Binding — Binds multimodal percepts into a unified experience.
//!
//! Combines signals from multiple cognitive streams (visual, linguistic, symbolic,
//! numeric) into a single coherent "experience" at each moment of consciousness.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

use super::global_workspace::{CognitiveContent, ContentModality, ContentType};

// ── Bound Experience ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundExperience {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub streams: Vec<ExperienceStream>,
    pub unified_representation: String,
    pub coherence_score: f64,
    pub dominant_modality: ContentModality,
    pub binding_strength: f64,
    pub gestalt: Gestalt,
}

/// A single sensory/cognitive stream contributing to the bound experience.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceStream {
    pub source: String,
    pub content: CognitiveContent,
    pub weight: f64,
    pub temporal_offset_ms: i64,
}

/// The emergent gestalt — the "whole" that is more than the sum of its parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gestalt {
    pub summary: String,
    pub emergent_properties: Vec<String>,
    pub figure_ground: Option<FigureGround>,
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FigureGround {
    pub figure: String,
    pub ground: String,
    pub contrast: f64,
}

// ── Phenomenal Binder ─────────────────────────────────────────────────────────

pub struct PhenomenalBinder {
    pub current_experience: Arc<RwLock<Option<BoundExperience>>>,
    pub experience_history: Arc<RwLock<Vec<BoundExperience>>>,
    pub binding_weights: Arc<RwLock<HashMap<ContentModality, f64>>>,
    max_history: usize,
}

impl PhenomenalBinder {
    pub fn new() -> Self {
        let mut weights = HashMap::new();
        weights.insert(ContentModality::Linguistic, 0.4);
        weights.insert(ContentModality::Symbolic, 0.3);
        weights.insert(ContentModality::Structural, 0.15);
        weights.insert(ContentModality::Numeric, 0.1);
        weights.insert(ContentModality::Mixed, 0.05);

        Self {
            current_experience: Arc::new(RwLock::new(None)),
            experience_history: Arc::new(RwLock::new(Vec::new())),
            binding_weights: Arc::new(RwLock::new(weights)),
            max_history: 200,
        }
    }

    /// Bind a set of cognitive streams into a unified experience.
    pub async fn bind(&self, streams: Vec<ExperienceStream>) -> BoundExperience {
        if streams.is_empty() {
            return self.empty_experience();
        }

        let weights = self.binding_weights.read().await;

        // Weight streams by modality importance
        let weighted_streams: Vec<(&ExperienceStream, f64)> = streams
            .iter()
            .map(|s| {
                let modality_weight = weights.get(&s.content.modality).copied().unwrap_or(0.1);
                let w = s.weight * modality_weight * s.content.salience;
                (s, w)
            })
            .collect();

        let total_weight: f64 = weighted_streams.iter().map(|(_, w)| w).sum();

        // Dominant modality: heaviest contributing stream
        let dominant_modality = weighted_streams
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(s, _)| s.content.modality.clone())
            .unwrap_or(ContentModality::Linguistic);

        // Unified representation: weighted concatenation of top-3 streams
        let mut sorted_streams = weighted_streams.clone();
        sorted_streams.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let unified: String = sorted_streams
            .iter()
            .take(3)
            .map(|(s, w)| format!("[{}: {} (w={:.2})]", s.source, &s.content.data[..s.content.data.len().min(80)], w))
            .collect::<Vec<_>>()
            .join(" | ");

        // Coherence score: inverse of variance in stream weights
        let mean_w = total_weight / weighted_streams.len() as f64;
        let variance = weighted_streams
            .iter()
            .map(|(_, w)| (w - mean_w).powi(2))
            .sum::<f64>()
            / weighted_streams.len() as f64;
        let coherence = (1.0 - variance.sqrt().min(1.0)).clamp(0.0, 1.0);

        // Binding strength: product of coherence and stream count factor
        let binding_strength = coherence * (1.0 - 1.0 / (streams.len() as f64 + 1.0));

        // Gestalt emergence
        let emergent_properties = self.detect_emergence(&sorted_streams.iter().map(|(s, _)| *s).collect::<Vec<_>>());
        let figure_ground = self.detect_figure_ground(&sorted_streams);

        let gestalt = Gestalt {
            summary: format!(
                "Unified experience from {} streams with {:.0}% coherence",
                streams.len(),
                coherence * 100.0
            ),
            emergent_properties,
            figure_ground,
            pattern: self.detect_pattern(&streams),
        };

        drop(weights);

        let experience = BoundExperience {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            streams,
            unified_representation: unified,
            coherence_score: coherence,
            dominant_modality,
            binding_strength,
            gestalt,
        };

        // Store
        {
            let mut current = self.current_experience.write().await;
            *current = Some(experience.clone());
        }
        {
            let mut history = self.experience_history.write().await;
            history.push(experience.clone());
            while history.len() > self.max_history {
                history.remove(0);
            }
        }

        debug!(
            "PhenomenalBinder: bound {} streams, coherence={:.3}, strength={:.3}",
            experience.streams.len(),
            experience.coherence_score,
            experience.binding_strength
        );

        experience
    }

    fn empty_experience(&self) -> BoundExperience {
        BoundExperience {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            streams: vec![],
            unified_representation: "[empty]".to_string(),
            coherence_score: 0.0,
            dominant_modality: ContentModality::Linguistic,
            binding_strength: 0.0,
            gestalt: Gestalt {
                summary: "No active streams".to_string(),
                emergent_properties: vec![],
                figure_ground: None,
                pattern: None,
            },
        }
    }

    fn detect_emergence(&self, streams: &[&ExperienceStream]) -> Vec<String> {
        let mut properties = Vec::new();

        // Cross-modal convergence
        let modalities: std::collections::HashSet<String> = streams
            .iter()
            .map(|s| format!("{:?}", s.content.modality))
            .collect();
        if modalities.len() > 1 {
            properties.push(format!("cross-modal integration ({} modalities)", modalities.len()));
        }

        // Goal-percept alignment
        let has_goal = streams.iter().any(|s| s.content.content_type == ContentType::Goal);
        let has_percept = streams.iter().any(|s| s.content.content_type == ContentType::Percept);
        if has_goal && has_percept {
            properties.push("goal-directed perception".to_string());
        }

        // High salience convergence
        let high_salience_count = streams.iter().filter(|s| s.content.salience > 0.7).count();
        if high_salience_count > 1 {
            properties.push(format!("{} high-salience streams converging", high_salience_count));
        }

        properties
    }

    fn detect_figure_ground(&self, sorted: &[(&ExperienceStream, f64)]) -> Option<FigureGround> {
        if sorted.len() < 2 {
            return None;
        }
        let (figure_stream, figure_weight) = &sorted[0];
        let ground_weight: f64 = sorted[1..].iter().map(|(_, w)| w).sum::<f64>() / (sorted.len() - 1) as f64;
        let contrast = (figure_weight - ground_weight).abs();

        if contrast > 0.2 {
            Some(FigureGround {
                figure: figure_stream.source.clone(),
                ground: sorted[1..].iter().map(|(s, _)| s.source.as_str()).collect::<Vec<_>>().join(", "),
                contrast,
            })
        } else {
            None
        }
    }

    fn detect_pattern(&self, streams: &[ExperienceStream]) -> Option<String> {
        // Detect temporal patterns in stream offsets
        if streams.len() < 3 {
            return None;
        }
        let mut offsets: Vec<i64> = streams.iter().map(|s| s.temporal_offset_ms).collect();
        offsets.sort();
        let diffs: Vec<i64> = offsets.windows(2).map(|w| w[1] - w[0]).collect();
        let mean_diff = diffs.iter().sum::<i64>() as f64 / diffs.len() as f64;
        let variance = diffs.iter().map(|d| (*d as f64 - mean_diff).powi(2)).sum::<f64>() / diffs.len() as f64;

        if variance < 100.0 {
            Some(format!("regular temporal pattern (interval ~{}ms)", mean_diff as i64))
        } else {
            None
        }
    }

    /// Update binding weights based on broadcast success.
    pub async fn update_weights(&self, dominant_modality: &ContentModality, success: bool) {
        let mut weights = self.binding_weights.write().await;
        let w = weights.entry(dominant_modality.clone()).or_insert(0.2);
        if success {
            *w = (*w * 0.95 + 1.0 * 0.05).clamp(0.05, 0.8);
        } else {
            *w = (*w * 0.98).clamp(0.05, 0.8);
        }
    }
}

impl Default for PhenomenalBinder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stream(source: &str, data: &str, salience: f64) -> ExperienceStream {
        ExperienceStream {
            source: source.to_string(),
            content: CognitiveContent {
                content_type: ContentType::Reasoning,
                data: data.to_string(),
                embedding: vec![],
                salience,
                modality: ContentModality::Linguistic,
            },
            weight: salience,
            temporal_offset_ms: 0,
        }
    }

    #[tokio::test]
    async fn test_binding() {
        let binder = PhenomenalBinder::new();
        let streams = vec![
            make_stream("reasoning", "solve the problem", 0.8),
            make_stream("goal_engine", "achieve goal X", 0.7),
        ];
        let exp = binder.bind(streams).await;
        assert!(!exp.unified_representation.is_empty());
        assert!(exp.coherence_score > 0.0);
    }
}

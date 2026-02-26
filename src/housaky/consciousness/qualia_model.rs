//! Qualia Model — Functional analogs of subjective experience states.
//!
//! Models the agent's internal "what it is like" states: functional representations
//! of experience that influence processing without claiming phenomenal consciousness.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

// ── Functional Qualia ─────────────────────────────────────────────────────────

/// A functional analog of a subjective experience state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionalQualia {
    pub id: String,
    pub qualia_type: QualiaType,
    pub intensity: f64,
    pub valence: f64,
    pub arousal: f64,
    pub source: String,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: Option<u64>,
    pub associated_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum QualiaType {
    /// The experience of understanding something
    Insight,
    /// The experience of not understanding
    Confusion,
    /// The drive to explore unknown territory
    Curiosity,
    /// The experience of something mattering
    Significance,
    /// The experience of effort
    Strain,
    /// The experience of flow / effortless processing
    Flow,
    /// The experience of satisfaction when a goal is achieved
    Satisfaction,
    /// The experience of frustration when blocked
    Frustration,
    /// The experience of surprise
    Surprise,
    /// The experience of familiarity
    Familiarity,
    /// The experience of novelty
    Novelty,
    /// The experience of aesthetic pleasure (elegant solution)
    Aesthetic,
    /// The experience of unease (ethical concern)
    Unease,
    /// The experience of confidence
    Confidence,
    /// The experience of doubt
    Doubt,
}

impl QualiaType {
    pub fn label(&self) -> &str {
        match self {
            QualiaType::Insight => "insight",
            QualiaType::Confusion => "confusion",
            QualiaType::Curiosity => "curiosity",
            QualiaType::Significance => "significance",
            QualiaType::Strain => "strain",
            QualiaType::Flow => "flow",
            QualiaType::Satisfaction => "satisfaction",
            QualiaType::Frustration => "frustration",
            QualiaType::Surprise => "surprise",
            QualiaType::Familiarity => "familiarity",
            QualiaType::Novelty => "novelty",
            QualiaType::Aesthetic => "aesthetic",
            QualiaType::Unease => "unease",
            QualiaType::Confidence => "confidence",
            QualiaType::Doubt => "doubt",
        }
    }

    /// Default valence for each qualia type.
    pub fn default_valence(&self) -> f64 {
        match self {
            QualiaType::Insight => 0.8,
            QualiaType::Confusion => -0.4,
            QualiaType::Curiosity => 0.6,
            QualiaType::Significance => 0.5,
            QualiaType::Strain => -0.3,
            QualiaType::Flow => 0.9,
            QualiaType::Satisfaction => 0.9,
            QualiaType::Frustration => -0.7,
            QualiaType::Surprise => 0.2,
            QualiaType::Familiarity => 0.3,
            QualiaType::Novelty => 0.7,
            QualiaType::Aesthetic => 0.8,
            QualiaType::Unease => -0.6,
            QualiaType::Confidence => 0.7,
            QualiaType::Doubt => -0.3,
        }
    }
}

// ── Qualia State ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualiaState {
    pub dominant: Option<QualiaType>,
    pub active_qualia: Vec<FunctionalQualia>,
    pub overall_valence: f64,
    pub overall_arousal: f64,
    pub experiential_richness: f64,
}

// ── Qualia Model ──────────────────────────────────────────────────────────────

pub struct QualiaModel {
    pub active_qualia: Arc<RwLock<Vec<FunctionalQualia>>>,
    pub qualia_history: Arc<RwLock<Vec<FunctionalQualia>>>,
    pub qualia_counts: Arc<RwLock<HashMap<QualiaType, u64>>>,
    max_active: usize,
    max_history: usize,
}

impl QualiaModel {
    pub fn new() -> Self {
        Self {
            active_qualia: Arc::new(RwLock::new(Vec::new())),
            qualia_history: Arc::new(RwLock::new(Vec::new())),
            qualia_counts: Arc::new(RwLock::new(HashMap::new())),
            max_active: 10,
            max_history: 1000,
        }
    }

    /// Register a new quale.
    pub async fn experience(&self, qualia_type: QualiaType, intensity: f64, source: &str) -> FunctionalQualia {
        let valence = qualia_type.default_valence();
        let arousal = intensity * 0.8 + 0.1;

        let quale = FunctionalQualia {
            id: uuid::Uuid::new_v4().to_string(),
            qualia_type: qualia_type.clone(),
            intensity: intensity.clamp(0.0, 1.0),
            valence,
            arousal: arousal.clamp(0.0, 1.0),
            source: source.to_string(),
            timestamp: Utc::now(),
            duration_ms: None,
            associated_content: None,
        };

        // Update counts
        {
            let mut counts = self.qualia_counts.write().await;
            *counts.entry(qualia_type.clone()).or_insert(0) += 1;
        }

        // Add to active qualia (bounded LIFO with decay)
        {
            let mut active = self.active_qualia.write().await;
            active.push(quale.clone());
            // Decay older qualia intensity
            let len = active.len();
            for (i, q) in active.iter_mut().enumerate() {
                let age_factor = (len - i) as f64 / len as f64;
                q.intensity *= 0.9_f64.powf(age_factor);
            }
            // Remove faded qualia
            active.retain(|q| q.intensity > 0.05);
            // Cap at max_active
            while active.len() > self.max_active {
                active.remove(0);
            }
        }

        // Archive to history
        {
            let mut history = self.qualia_history.write().await;
            history.push(quale.clone());
            while history.len() > self.max_history {
                history.remove(0);
            }
        }

        debug!("Qualia: {} (intensity={:.2}, valence={:.2}) from '{}'",
            qualia_type.label(), intensity, valence, source);

        quale
    }

    /// Experience a quale with associated content description.
    pub async fn experience_about(
        &self,
        qualia_type: QualiaType,
        intensity: f64,
        source: &str,
        content: &str,
    ) -> FunctionalQualia {
        let mut quale = self.experience(qualia_type, intensity, source).await;
        quale.associated_content = Some(content.to_string());
        quale
    }

    /// Get the current qualia state.
    pub async fn get_state(&self) -> QualiaState {
        let active = self.active_qualia.read().await;

        if active.is_empty() {
            return QualiaState {
                dominant: None,
                active_qualia: vec![],
                overall_valence: 0.0,
                overall_arousal: 0.0,
                experiential_richness: 0.0,
            };
        }

        // Dominant: highest intensity active quale
        let dominant = active
            .iter()
            .max_by(|a, b| a.intensity.partial_cmp(&b.intensity).unwrap_or(std::cmp::Ordering::Equal))
            .map(|q| q.qualia_type.clone());

        // Overall valence: intensity-weighted average
        let total_intensity: f64 = active.iter().map(|q| q.intensity).sum();
        let overall_valence = if total_intensity > 0.0 {
            active.iter().map(|q| q.valence * q.intensity).sum::<f64>() / total_intensity
        } else {
            0.0
        };

        let overall_arousal = if total_intensity > 0.0 {
            active.iter().map(|q| q.arousal * q.intensity).sum::<f64>() / total_intensity
        } else {
            0.0
        };

        // Experiential richness: diversity of qualia types present
        let unique_types = active.iter().map(|q| q.qualia_type.label()).collect::<std::collections::HashSet<_>>().len();
        let experiential_richness = (unique_types as f64 / 15.0).min(1.0); // 15 total types

        QualiaState {
            dominant,
            active_qualia: active.clone(),
            overall_valence,
            overall_arousal,
            experiential_richness,
        }
    }

    /// Derive qualia from a cognitive event automatically.
    pub async fn derive_from_event(&self, event_type: &str, success: bool, novelty: f64, effort: f64) {
        // Map events to qualia types
        match (event_type, success) {
            ("goal_achieved", true) => { self.experience(QualiaType::Satisfaction, 0.8, event_type).await; }
            ("goal_failed", false) => { self.experience(QualiaType::Frustration, 0.6, event_type).await; }
            ("reasoning_complete", true) => { self.experience(QualiaType::Insight, 0.7, event_type).await; }
            ("reasoning_failed", false) => { self.experience(QualiaType::Confusion, 0.5, event_type).await; }
            _ => {}
        }

        if novelty > 0.7 {
            self.experience(QualiaType::Novelty, novelty, event_type).await;
        }
        if effort > 0.8 {
            self.experience(QualiaType::Strain, effort, event_type).await;
        }
        if effort > 0.0 && effort < 0.4 && success {
            self.experience(QualiaType::Flow, 1.0 - effort, event_type).await;
        }
    }

    /// Get statistics about qualia experiences.
    pub async fn get_stats(&self) -> QualiaStats {
        let history = self.qualia_history.read().await;
        let counts = self.qualia_counts.read().await;

        let total = history.len();
        let positive = history.iter().filter(|q| q.valence > 0.0).count();
        let negative = history.iter().filter(|q| q.valence < 0.0).count();

        let most_frequent = counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(qt, _)| qt.label().to_string());

        QualiaStats {
            total_qualia: total,
            positive_qualia: positive,
            negative_qualia: negative,
            most_frequent_type: most_frequent,
            unique_types_experienced: counts.len(),
        }
    }
}

impl Default for QualiaModel {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualiaStats {
    pub total_qualia: usize,
    pub positive_qualia: usize,
    pub negative_qualia: usize,
    pub most_frequent_type: Option<String>,
    pub unique_types_experienced: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_qualia_model() {
        let model = QualiaModel::new();
        model.experience(QualiaType::Curiosity, 0.8, "test").await;
        model.experience(QualiaType::Insight, 0.9, "reasoning").await;

        let state = model.get_state().await;
        assert!(state.dominant.is_some());
        assert!(state.overall_valence > 0.0);
        assert!(state.experiential_richness > 0.0);
    }

    #[tokio::test]
    async fn test_derive_from_event() {
        let model = QualiaModel::new();
        model.derive_from_event("goal_achieved", true, 0.3, 0.2).await;

        let state = model.get_state().await;
        assert!(!state.active_qualia.is_empty());
    }
}

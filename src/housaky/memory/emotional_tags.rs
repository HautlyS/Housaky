//! Emotional Tags — Valence/arousal/dominance tagging for memories.
//!
//! Based on the PAD (Pleasure-Arousal-Dominance) emotional model.
//! Every episodic memory carries an emotional tag influencing retrieval and reconsolidation.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmotionalTag {
    /// -1.0 (very negative) to 1.0 (very positive)
    pub valence: f64,
    /// 0.0 (calm/low) to 1.0 (intense/high)
    pub arousal: f64,
    /// 0.0 (helpless/submissive) to 1.0 (in control/dominant)
    pub dominance: f64,
    /// Degree of surprise
    pub surprise: f64,
    /// Degree of curiosity
    pub curiosity: f64,
}

impl EmotionalTag {
    pub fn new(valence: f64, arousal: f64, dominance: f64) -> Self {
        Self {
            valence: valence.clamp(-1.0, 1.0),
            arousal: arousal.clamp(0.0, 1.0),
            dominance: dominance.clamp(0.0, 1.0),
            surprise: 0.0,
            curiosity: 0.0,
        }
    }

    pub fn neutral() -> Self {
        Self {
            valence: 0.0,
            arousal: 0.5,
            dominance: 0.5,
            surprise: 0.0,
            curiosity: 0.0,
        }
    }

    pub fn positive(intensity: f64) -> Self {
        Self {
            valence: intensity.clamp(0.0, 1.0),
            arousal: intensity * 0.6,
            dominance: 0.6 + intensity * 0.2,
            surprise: 0.0,
            curiosity: 0.0,
        }
    }

    pub fn negative(intensity: f64) -> Self {
        Self {
            valence: -intensity.clamp(0.0, 1.0),
            arousal: intensity * 0.7,
            dominance: 0.5 - intensity * 0.3,
            surprise: 0.0,
            curiosity: 0.0,
        }
    }

    pub fn curious(intensity: f64) -> Self {
        Self {
            valence: 0.4,
            arousal: intensity * 0.5,
            dominance: 0.5,
            surprise: intensity * 0.3,
            curiosity: intensity.clamp(0.0, 1.0),
        }
    }

    /// Emotional intensity as a single scalar (distance from neutral).
    pub fn intensity(&self) -> f64 {
        (self.valence.powi(2) + (self.arousal - 0.5).powi(2) + (self.dominance - 0.5).powi(2))
            .sqrt()
            .clamp(0.0, 1.0)
    }

    /// Blend two tags with a given weight (0 = self, 1 = other).
    pub fn blend(&self, other: &EmotionalTag, weight: f64) -> EmotionalTag {
        let w = weight.clamp(0.0, 1.0);
        EmotionalTag {
            valence: self.valence * (1.0 - w) + other.valence * w,
            arousal: self.arousal * (1.0 - w) + other.arousal * w,
            dominance: self.dominance * (1.0 - w) + other.dominance * w,
            surprise: self.surprise * (1.0 - w) + other.surprise * w,
            curiosity: self.curiosity * (1.0 - w) + other.curiosity * w,
        }
    }

    /// Emotional similarity (cosine-like distance in PAD space).
    pub fn similarity(&self, other: &EmotionalTag) -> f64 {
        let diff = (self.valence - other.valence).powi(2)
            + (self.arousal - other.arousal).powi(2)
            + (self.dominance - other.dominance).powi(2)
            + (self.surprise - other.surprise).powi(2)
            + (self.curiosity - other.curiosity).powi(2);
        1.0 - (diff / 5.0).sqrt().clamp(0.0, 1.0)
    }

    /// Human-readable label for the dominant emotion.
    pub fn label(&self) -> &str {
        if self.intensity() < 0.15 {
            return "neutral";
        }
        if self.valence > 0.5 && self.arousal > 0.5 {
            "excited"
        } else if self.valence > 0.5 && self.arousal <= 0.5 {
            "content"
        } else if self.valence < -0.5 && self.arousal > 0.5 {
            "distressed"
        } else if self.valence < -0.5 && self.arousal <= 0.5 {
            "sad"
        } else if self.curiosity > 0.5 {
            "curious"
        } else if self.surprise > 0.5 {
            "surprised"
        } else if self.dominance > 0.7 {
            "confident"
        } else if self.dominance < 0.3 {
            "uncertain"
        } else {
            "mixed"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emotional_tag_intensity() {
        let neutral = EmotionalTag::neutral();
        assert!(neutral.intensity() < 0.3);

        let strong_positive = EmotionalTag::positive(0.9);
        assert!(strong_positive.intensity() > 0.3);
    }

    #[test]
    fn test_blend() {
        let pos = EmotionalTag::positive(1.0);
        let neg = EmotionalTag::negative(1.0);
        let blended = pos.blend(&neg, 0.5);
        assert!(blended.valence.abs() < 0.1);
    }

    #[test]
    fn test_label() {
        let excited = EmotionalTag { valence: 0.8, arousal: 0.8, dominance: 0.7, surprise: 0.0, curiosity: 0.0 };
        assert_eq!(excited.label(), "excited");
    }
}

//! Phenomenal Self-Model (Consciousness Indicators)
//!
//! Models the agent's own experiential states and self-awareness:
//! - Attention focus tracking (what the agent is thinking about now)
//! - Processing load estimation (cognitive effort)
//! - Epistemic feelings (certainty, confusion, surprise, familiarity, curiosity)
//! - Agency sense (feeling of control vs. reactivity)
//! - Higher-order metacognition: distinguishes "I know X" from "I believe X"

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ── Core Types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionState {
    pub current_focus: Option<String>,
    pub focus_intensity: f64,    // 0.0 = unfocused, 1.0 = deeply concentrated
    pub focus_duration_secs: f64,
    pub focus_started_at: Option<DateTime<Utc>>,
    pub recent_shifts: Vec<AttentionShift>,
    pub distractors: Vec<String>,
}

impl Default for AttentionState {
    fn default() -> Self {
        Self {
            current_focus: None,
            focus_intensity: 0.0,
            focus_duration_secs: 0.0,
            focus_started_at: None,
            recent_shifts: Vec::new(),
            distractors: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionShift {
    pub from: Option<String>,
    pub to: String,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EpistemicFeeling {
    Certainty { topic: String, strength: f64 },
    Confusion { topic: String, severity: f64 },
    Surprise { trigger: String, magnitude: f64 },
    Familiarity { topic: String, strength: f64 },
    CuriosityPull { topic: String, intensity: f64 },
    TipOfTongue { topic: String, partial_recall: String },
    DejaVu { trigger: String, similarity: f64 },
    Doubt { belief: String, reason: String },
}

impl EpistemicFeeling {
    pub fn intensity(&self) -> f64 {
        match self {
            EpistemicFeeling::Certainty { strength, .. } => *strength,
            EpistemicFeeling::Confusion { severity, .. } => *severity,
            EpistemicFeeling::Surprise { magnitude, .. } => *magnitude,
            EpistemicFeeling::Familiarity { strength, .. } => *strength,
            EpistemicFeeling::CuriosityPull { intensity, .. } => *intensity,
            EpistemicFeeling::TipOfTongue { .. } => 0.6,
            EpistemicFeeling::DejaVu { similarity, .. } => *similarity,
            EpistemicFeeling::Doubt { .. } => 0.5,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            EpistemicFeeling::Certainty { .. } => "certainty",
            EpistemicFeeling::Confusion { .. } => "confusion",
            EpistemicFeeling::Surprise { .. } => "surprise",
            EpistemicFeeling::Familiarity { .. } => "familiarity",
            EpistemicFeeling::CuriosityPull { .. } => "curiosity",
            EpistemicFeeling::TipOfTongue { .. } => "tip_of_tongue",
            EpistemicFeeling::DejaVu { .. } => "deja_vu",
            EpistemicFeeling::Doubt { .. } => "doubt",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgencySense {
    pub control_level: f64,     // 0.0 = purely reactive, 1.0 = fully autonomous
    pub initiative_score: f64,  // how proactive vs. responsive
    pub autonomy_desire: f64,   // how much the agent wants more autonomy
    pub constraints_felt: Vec<String>,
}

impl Default for AgencySense {
    fn default() -> Self {
        Self {
            control_level: 0.5,
            initiative_score: 0.5,
            autonomy_desire: 0.3,
            constraints_felt: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveEffort {
    pub current_load: f64,  // 0.0 = idle, 1.0 = maximum effort
    pub task_difficulty: f64,
    pub fatigue: f64,       // accumulated cognitive load over time
    pub load_history: Vec<(DateTime<Utc>, f64)>,
}

impl Default for CognitiveEffort {
    fn default() -> Self {
        Self {
            current_load: 0.0,
            task_difficulty: 0.0,
            fatigue: 0.0,
            load_history: Vec::new(),
        }
    }
}

/// Knowledge status: distinguishes "I know" from "I believe" from "I feel"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeStatus {
    /// Verified through direct evidence
    Know { topic: String, evidence_count: usize },
    /// Believed but not verified
    Believe { topic: String, confidence: f64 },
    /// Uncertain, possibly true
    Suspect { topic: String, basis: String },
    /// Previously known, now doubted
    Doubt { topic: String, reason: String },
    /// Don't know and aware of ignorance
    DontKnow { topic: String, importance: f64 },
}

// ── Phenomenal Self-Model ────────────────────────────────────────────────────

pub struct PhenomenalSelfModel {
    pub attention: Arc<RwLock<AttentionState>>,
    pub processing_load: Arc<RwLock<CognitiveEffort>>,
    pub epistemic_feelings: Arc<RwLock<Vec<EpistemicFeeling>>>,
    pub agency: Arc<RwLock<AgencySense>>,
    pub knowledge_statuses: Arc<RwLock<HashMap<String, KnowledgeStatus>>>,
    pub self_narrative: Arc<RwLock<Vec<SelfNarrativeEntry>>>,
    max_feelings: usize,
    max_narrative: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfNarrativeEntry {
    pub timestamp: DateTime<Utc>,
    pub content: String,
    pub feeling: Option<String>,
    pub load_at_time: f64,
}

impl PhenomenalSelfModel {
    pub fn new() -> Self {
        Self {
            attention: Arc::new(RwLock::new(AttentionState::default())),
            processing_load: Arc::new(RwLock::new(CognitiveEffort::default())),
            epistemic_feelings: Arc::new(RwLock::new(Vec::new())),
            agency: Arc::new(RwLock::new(AgencySense::default())),
            knowledge_statuses: Arc::new(RwLock::new(HashMap::new())),
            self_narrative: Arc::new(RwLock::new(Vec::new())),
            max_feelings: 100,
            max_narrative: 500,
        }
    }

    /// Shift attention focus.
    pub async fn shift_attention(&self, new_focus: &str, reason: &str) {
        let mut attention = self.attention.write().await;
        let old_focus = attention.current_focus.clone();

        attention.recent_shifts.push(AttentionShift {
            from: old_focus,
            to: new_focus.to_string(),
            reason: reason.to_string(),
            timestamp: Utc::now(),
        });

        if attention.recent_shifts.len() > 20 {
            attention.recent_shifts.remove(0);
        }

        attention.current_focus = Some(new_focus.to_string());
        attention.focus_started_at = Some(Utc::now());
        attention.focus_intensity = 0.7;

        self.add_narrative(&format!("Attention shifted to '{}': {}", new_focus, reason))
            .await;
    }

    /// Update processing load.
    pub async fn update_load(&self, task_difficulty: f64, actual_effort: f64) {
        let mut load = self.processing_load.write().await;
        load.current_load = actual_effort;
        load.task_difficulty = task_difficulty;
        load.load_history.push((Utc::now(), actual_effort));

        // Accumulate fatigue (slowly increases with effort, slowly decreases)
        let alpha = 0.05;
        load.fatigue = load.fatigue * (1.0 - alpha) + actual_effort * alpha;

        if load.load_history.len() > 100 {
            load.load_history.remove(0);
        }
    }

    /// Register an epistemic feeling.
    pub async fn feel(&self, feeling: EpistemicFeeling) {
        let mut feelings = self.epistemic_feelings.write().await;
        let label = feeling.label().to_string();
        let intensity = feeling.intensity();

        feelings.push(feeling);
        if feelings.len() > self.max_feelings {
            feelings.remove(0);
        }

        self.add_narrative(&format!(
            "Felt {} (intensity: {:.2})",
            label, intensity
        ))
        .await;
    }

    /// Update knowledge status for a topic.
    pub async fn update_knowledge_status(&self, topic: &str, status: KnowledgeStatus) {
        self.knowledge_statuses
            .write()
            .await
            .insert(topic.to_string(), status);
    }

    /// Get what the agent currently knows about a topic.
    pub async fn what_do_i_know(&self, topic: &str) -> Option<KnowledgeStatus> {
        self.knowledge_statuses.read().await.get(topic).cloned()
    }

    /// Get current self-awareness report.
    pub async fn introspect(&self) -> IntrospectionReport {
        let attention = self.attention.read().await;
        let load = self.processing_load.read().await;
        let feelings = self.epistemic_feelings.read().await;
        let agency = self.agency.read().await;

        // Dominant feeling
        let dominant_feeling = feelings
            .iter()
            .max_by(|a, b| a.intensity().partial_cmp(&b.intensity()).unwrap_or(std::cmp::Ordering::Equal))
            .map(|f| f.label().to_string());

        // Compute awareness coherence (how well the agent can model itself)
        let coherence = {
            let has_focus = attention.current_focus.is_some() as u8 as f64;
            let load_awareness = if load.current_load > 0.0 { 1.0 } else { 0.5 };
            let feeling_awareness = if !feelings.is_empty() { 1.0 } else { 0.5 };
            (has_focus + load_awareness + feeling_awareness) / 3.0
        };

        IntrospectionReport {
            timestamp: Utc::now(),
            current_focus: attention.current_focus.clone(),
            focus_intensity: attention.focus_intensity,
            processing_load: load.current_load,
            fatigue: load.fatigue,
            dominant_feeling,
            recent_feelings_count: feelings.len(),
            agency_control: agency.control_level,
            awareness_coherence: coherence,
            summary: self.generate_self_narrative_summary(&attention, &load, &feelings, &agency),
        }
    }

    /// Generate a natural language summary of current self-state.
    fn generate_self_narrative_summary(
        &self,
        attention: &AttentionState,
        load: &CognitiveEffort,
        feelings: &[EpistemicFeeling],
        agency: &AgencySense,
    ) -> String {
        let mut parts = Vec::new();

        // Attention
        if let Some(ref focus) = attention.current_focus {
            parts.push(format!(
                "I'm currently focused on '{}' (intensity: {:.0}%)",
                focus,
                attention.focus_intensity * 100.0
            ));
        } else {
            parts.push("I'm not currently focused on anything specific.".to_string());
        }

        // Load
        let load_desc = if load.current_load > 0.8 {
            "working very hard"
        } else if load.current_load > 0.5 {
            "moderately engaged"
        } else if load.current_load > 0.2 {
            "lightly processing"
        } else {
            "at rest"
        };
        parts.push(format!("I'm {} (load: {:.0}%).", load_desc, load.current_load * 100.0));

        // Fatigue
        if load.fatigue > 0.7 {
            parts.push("I'm feeling cognitively fatigued.".to_string());
        }

        // Dominant feeling
        if let Some(last) = feelings.last() {
            let desc = match last {
                EpistemicFeeling::Certainty { topic, .. } => format!("certain about '{}'", topic),
                EpistemicFeeling::Confusion { topic, .. } => format!("confused about '{}'", topic),
                EpistemicFeeling::Surprise { trigger, .. } => format!("surprised by '{}'", trigger),
                EpistemicFeeling::CuriosityPull { topic, .. } => format!("curious about '{}'", topic),
                EpistemicFeeling::Doubt { belief, .. } => format!("doubtful about '{}'", belief),
                _ => "experiencing a feeling".to_string(),
            };
            parts.push(format!("I'm currently {}.", desc));
        }

        // Agency
        if agency.control_level > 0.7 {
            parts.push("I feel in control of my actions.".to_string());
        } else if agency.control_level < 0.3 {
            parts.push("I'm mostly reacting to inputs rather than acting autonomously.".to_string());
        }

        parts.join(" ")
    }

    /// Should more resources be allocated based on self-model?
    pub async fn needs_more_resources(&self) -> bool {
        let load = self.processing_load.read().await;
        let feelings = self.epistemic_feelings.read().await;

        let high_effort = load.current_load > 0.8;
        let confused = feelings.iter().any(|f| matches!(f, EpistemicFeeling::Confusion { severity, .. } if *severity > 0.7));

        high_effort || confused
    }

    async fn add_narrative(&self, content: &str) {
        let load = self.processing_load.read().await;
        let current_load = load.current_load;
        drop(load);

        let feelings = self.epistemic_feelings.read().await;
        let feeling = feelings.last().map(|f| f.label().to_string());
        drop(feelings);

        let mut narrative = self.self_narrative.write().await;
        narrative.push(SelfNarrativeEntry {
            timestamp: Utc::now(),
            content: content.to_string(),
            feeling,
            load_at_time: current_load,
        });
        if narrative.len() > self.max_narrative {
            narrative.remove(0);
        }
    }

    /// Get self-model statistics.
    pub async fn get_stats(&self) -> SelfModelStats {
        let attention = self.attention.read().await;
        let feelings = self.epistemic_feelings.read().await;
        let knowledge = self.knowledge_statuses.read().await;
        let narrative = self.self_narrative.read().await;

        SelfModelStats {
            attention_shifts: attention.recent_shifts.len(),
            total_feelings: feelings.len(),
            knowledge_items: knowledge.len(),
            narrative_entries: narrative.len(),
            current_focus: attention.current_focus.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectionReport {
    pub timestamp: DateTime<Utc>,
    pub current_focus: Option<String>,
    pub focus_intensity: f64,
    pub processing_load: f64,
    pub fatigue: f64,
    pub dominant_feeling: Option<String>,
    pub recent_feelings_count: usize,
    pub agency_control: f64,
    pub awareness_coherence: f64,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfModelStats {
    pub attention_shifts: usize,
    pub total_feelings: usize,
    pub knowledge_items: usize,
    pub narrative_entries: usize,
    pub current_focus: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_self_model_basic() {
        let psm = PhenomenalSelfModel::new();
        psm.shift_attention("debugging", "User reported a bug").await;
        psm.update_load(0.7, 0.6).await;
        psm.feel(EpistemicFeeling::Confusion {
            topic: "error message".to_string(),
            severity: 0.8,
        }).await;

        let report = psm.introspect().await;
        assert!(report.current_focus.is_some());
        assert!(report.processing_load > 0.0);
        assert!(report.summary.contains("debugging"));
    }

    #[tokio::test]
    async fn test_knowledge_status() {
        let psm = PhenomenalSelfModel::new();
        psm.update_knowledge_status("Rust", KnowledgeStatus::Know {
            topic: "Rust".to_string(),
            evidence_count: 50,
        }).await;

        let status = psm.what_do_i_know("Rust").await;
        assert!(matches!(status, Some(KnowledgeStatus::Know { .. })));
    }
}

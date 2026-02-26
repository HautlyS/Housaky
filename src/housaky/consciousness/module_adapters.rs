//! CognitiveModule adapters for existing Housaky modules.
//!
//! Wraps reasoning_engine, meta_cognition, goal_engine, attention, and memory
//! so they can participate in the Global Workspace Theory broadcast competition.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::coalition_formation::{Coalition, CoalitionFormation};
use super::global_workspace::{
    CognitiveModule, ConsciousBroadcast, ContentType,
};
use tracing::debug;

// ── Reasoning Module Adapter ──────────────────────────────────────────────────

pub struct ReasoningModuleAdapter {
    pub current_topic: Arc<RwLock<Option<String>>>,
    pub current_confidence: Arc<RwLock<f64>>,
    pub last_broadcast_content: Arc<RwLock<Option<String>>>,
    pub last_updated: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl ReasoningModuleAdapter {
    pub fn new() -> Self {
        Self {
            current_topic: Arc::new(RwLock::new(None)),
            current_confidence: Arc::new(RwLock::new(0.5)),
            last_broadcast_content: Arc::new(RwLock::new(None)),
            last_updated: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_active_reasoning(&self, topic: &str, confidence: f64) {
        *self.current_topic.write().await = Some(topic.to_string());
        *self.current_confidence.write().await = confidence;
        *self.last_updated.write().await = Some(Utc::now());
    }
}

#[async_trait]
impl CognitiveModule for ReasoningModuleAdapter {
    fn name(&self) -> &str {
        "reasoning_engine"
    }

    async fn receive_broadcast(&self, broadcast: &ConsciousBroadcast) {
        let data = broadcast.content.data.clone();
        *self.last_broadcast_content.write().await = Some(data.clone());

        // When a Goal or MetaCognition broadcast wins, update our reasoning topic
        // so the reasoning engine focuses on the most salient concern globally.
        match broadcast.content.content_type {
            ContentType::Goal | ContentType::MetaCognition => {
                *self.current_topic.write().await = Some(data.chars().take(120).collect());
                // Blend confidence toward the broadcast salience
                let mut conf = self.current_confidence.write().await;
                *conf = (*conf * 0.7 + broadcast.content.salience * 0.3).clamp(0.0, 1.0);
                *self.last_updated.write().await = Some(Utc::now());
                debug!(module = "reasoning_engine", "updated topic from {:?} broadcast", broadcast.content.content_type);
            }
            ContentType::Emotion => {
                // Emotional signal adjusts confidence up/down
                let delta = (broadcast.content.salience - 0.5) * 0.2;
                let mut conf = self.current_confidence.write().await;
                *conf = (*conf + delta).clamp(0.1, 1.0);
            }
            _ => {}
        }
    }

    async fn propose_coalition(&self) -> Option<Coalition> {
        let topic = self.current_topic.read().await;
        let confidence = *self.current_confidence.read().await;

        topic.as_ref().map(|t| {
            CoalitionFormation::build_coalition(
                "reasoning_engine",
                ContentType::Reasoning,
                format!("Active reasoning about: {}", t),
                confidence,
                0.6,
                0.5,
            )
        })
    }

    fn integration_score(&self) -> f64 {
        0.8
    }

    async fn describe_state(&self) -> String {
        let topic = self.current_topic.read().await;
        let conf = *self.current_confidence.read().await;
        match topic.as_ref() {
            Some(t) => format!("Reasoning about '{}' (confidence={:.2})", t, conf),
            None => "Reasoning engine idle".to_string(),
        }
    }
}

// ── Meta-Cognition Module Adapter ─────────────────────────────────────────────

pub struct MetaCognitionAdapter {
    pub current_self_assessment: Arc<RwLock<Option<String>>>,
    pub self_awareness_score: Arc<RwLock<f64>>,
    pub last_broadcast_content: Arc<RwLock<Option<String>>>,
    pub last_updated: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl MetaCognitionAdapter {
    pub fn new() -> Self {
        Self {
            current_self_assessment: Arc::new(RwLock::new(None)),
            self_awareness_score: Arc::new(RwLock::new(0.5)),
            last_broadcast_content: Arc::new(RwLock::new(None)),
            last_updated: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_assessment(&self, assessment: &str, score: f64) {
        *self.current_self_assessment.write().await = Some(assessment.to_string());
        *self.self_awareness_score.write().await = score;
        *self.last_updated.write().await = Some(Utc::now());
    }
}

#[async_trait]
impl CognitiveModule for MetaCognitionAdapter {
    fn name(&self) -> &str {
        "meta_cognition"
    }

    async fn receive_broadcast(&self, broadcast: &ConsciousBroadcast) {
        let data = broadcast.content.data.clone();
        *self.last_broadcast_content.write().await = Some(data.clone());

        // Meta-cognition integrates any broadcast to update its self-assessment.
        // The integration depth indicates how many modules were already involved,
        // which is a proxy for the richness of the cognitive event.
        let richness = broadcast.integration_depth as f64 / 6.0;
        let new_score = (broadcast.content.salience * 0.5 + richness * 0.5).clamp(0.0, 1.0);
        {
            let mut score = self.self_awareness_score.write().await;
            *score = (*score * 0.8 + new_score * 0.2).clamp(0.0, 1.0);
        }

        // Derive a self-assessment sentence from the winning broadcast
        let summary = format!(
            "Global broadcast ({:?}): '{}' (phi={:.2}, depth={})",
            broadcast.content.content_type,
            data.chars().take(80).collect::<String>(),
            broadcast.phi_contribution,
            broadcast.integration_depth,
        );
        *self.current_self_assessment.write().await = Some(summary);
        *self.last_updated.write().await = Some(Utc::now());
        debug!(module = "meta_cognition", phi = broadcast.phi_contribution, "self-assessment updated from broadcast");
    }

    async fn propose_coalition(&self) -> Option<Coalition> {
        let assessment = self.current_self_assessment.read().await;
        let score = *self.self_awareness_score.read().await;

        assessment.as_ref().map(|a| {
            CoalitionFormation::build_coalition(
                "meta_cognition",
                ContentType::MetaCognition,
                format!("Self-assessment: {}", a),
                score,
                0.4,
                0.3,
            )
        })
    }

    fn integration_score(&self) -> f64 {
        0.75
    }

    async fn describe_state(&self) -> String {
        let assessment = self.current_self_assessment.read().await;
        match assessment.as_ref() {
            Some(a) => format!("Meta-cognition active: {}", a),
            None => "Meta-cognition monitoring".to_string(),
        }
    }
}

// ── Goal Engine Module Adapter ────────────────────────────────────────────────

pub struct GoalEngineAdapter {
    pub active_goal: Arc<RwLock<Option<String>>>,
    pub goal_urgency: Arc<RwLock<f64>>,
    pub goal_priority: Arc<RwLock<f64>>,
    pub last_broadcast_content: Arc<RwLock<Option<String>>>,
    pub last_updated: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl GoalEngineAdapter {
    pub fn new() -> Self {
        Self {
            active_goal: Arc::new(RwLock::new(None)),
            goal_urgency: Arc::new(RwLock::new(0.5)),
            goal_priority: Arc::new(RwLock::new(0.5)),
            last_broadcast_content: Arc::new(RwLock::new(None)),
            last_updated: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_active_goal(&self, goal: &str, urgency: f64, priority: f64) {
        *self.active_goal.write().await = Some(goal.to_string());
        *self.goal_urgency.write().await = urgency;
        *self.goal_priority.write().await = priority;
        *self.last_updated.write().await = Some(Utc::now());
    }
}

#[async_trait]
impl CognitiveModule for GoalEngineAdapter {
    fn name(&self) -> &str {
        "goal_engine"
    }

    async fn receive_broadcast(&self, broadcast: &ConsciousBroadcast) {
        let data = broadcast.content.data.clone();
        *self.last_broadcast_content.write().await = Some(data.clone());

        // When a Reasoning or Prediction broadcast wins, it may surface a new
        // actionable objective — elevate urgency so the goal engine prioritises it.
        match broadcast.content.content_type {
            ContentType::Reasoning | ContentType::Prediction => {
                let urgency_boost = broadcast.content.salience * 0.3;
                let mut urg = self.goal_urgency.write().await;
                *urg = (*urg + urgency_boost).clamp(0.0, 1.0);
                // If no goal is set, adopt the broadcast content as an implicit goal
                let has_goal = self.active_goal.read().await.is_some();
                if !has_goal {
                    *self.active_goal.write().await =
                        Some(data.chars().take(100).collect());
                    *self.last_updated.write().await = Some(Utc::now());
                }
                debug!(module = "goal_engine", "urgency boosted by {:?} broadcast", broadcast.content.content_type);
            }
            ContentType::Goal => {
                // A winning Goal broadcast from another module supersedes ours
                *self.active_goal.write().await = Some(data.chars().take(100).collect());
                *self.goal_priority.write().await = broadcast.content.salience;
                *self.last_updated.write().await = Some(Utc::now());
            }
            _ => {}
        }
    }

    async fn propose_coalition(&self) -> Option<Coalition> {
        let goal = self.active_goal.read().await;
        let urgency = *self.goal_urgency.read().await;
        let priority = *self.goal_priority.read().await;

        goal.as_ref().map(|g| {
            CoalitionFormation::build_coalition(
                "goal_engine",
                ContentType::Goal,
                format!("Active goal: {}", g),
                priority,
                urgency,
                0.4,
            )
        })
    }

    fn integration_score(&self) -> f64 {
        0.85
    }

    async fn describe_state(&self) -> String {
        let goal = self.active_goal.read().await;
        let urgency = *self.goal_urgency.read().await;
        match goal.as_ref() {
            Some(g) => format!("Goal engine: '{}' (urgency={:.2})", g, urgency),
            None => "Goal engine: no active goal".to_string(),
        }
    }
}

// ── Attention Module Adapter ──────────────────────────────────────────────────

pub struct AttentionModuleAdapter {
    pub focus: Arc<RwLock<Option<String>>>,
    pub focus_intensity: Arc<RwLock<f64>>,
    pub last_broadcast_content: Arc<RwLock<Option<String>>>,
    pub last_updated: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl AttentionModuleAdapter {
    pub fn new() -> Self {
        Self {
            focus: Arc::new(RwLock::new(None)),
            focus_intensity: Arc::new(RwLock::new(0.0)),
            last_broadcast_content: Arc::new(RwLock::new(None)),
            last_updated: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_focus(&self, focus: &str, intensity: f64) {
        *self.focus.write().await = Some(focus.to_string());
        *self.focus_intensity.write().await = intensity;
        *self.last_updated.write().await = Some(Utc::now());
    }
}

#[async_trait]
impl CognitiveModule for AttentionModuleAdapter {
    fn name(&self) -> &str {
        "attention"
    }

    async fn receive_broadcast(&self, broadcast: &ConsciousBroadcast) {
        let data = broadcast.content.data.clone();
        // Attention naturally shifts to whatever won the broadcast competition
        *self.focus.write().await = Some(data.chars().take(120).collect());
        *self.last_broadcast_content.write().await = Some(data);
        // Intensity is driven by the broadcast salience — high salience = sharp focus
        *self.focus_intensity.write().await = broadcast.content.salience;
        *self.last_updated.write().await = Some(Utc::now());
        debug!(module = "attention", salience = broadcast.content.salience, "focus updated from broadcast");
    }

    async fn propose_coalition(&self) -> Option<Coalition> {
        let focus = self.focus.read().await;
        let intensity = *self.focus_intensity.read().await;

        if intensity < 0.1 {
            return None;
        }

        focus.as_ref().map(|f| {
            CoalitionFormation::build_coalition(
                "attention",
                ContentType::Percept,
                format!("Attention focused on: {}", f),
                intensity,
                intensity * 0.8,
                0.3,
            )
        })
    }

    fn integration_score(&self) -> f64 {
        0.7
    }

    async fn describe_state(&self) -> String {
        let focus = self.focus.read().await;
        let intensity = *self.focus_intensity.read().await;
        match focus.as_ref() {
            Some(f) => format!("Attention on '{}' (intensity={:.2})", f, intensity),
            None => "Attention unfocused".to_string(),
        }
    }
}

// ── Memory Module Adapter ─────────────────────────────────────────────────────

pub struct MemoryModuleAdapter {
    pub recent_retrieval: Arc<RwLock<Option<String>>>,
    pub retrieval_relevance: Arc<RwLock<f64>>,
    pub last_broadcast_content: Arc<RwLock<Option<String>>>,
    pub last_updated: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl MemoryModuleAdapter {
    pub fn new() -> Self {
        Self {
            recent_retrieval: Arc::new(RwLock::new(None)),
            retrieval_relevance: Arc::new(RwLock::new(0.0)),
            last_broadcast_content: Arc::new(RwLock::new(None)),
            last_updated: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_retrieval(&self, content: &str, relevance: f64) {
        *self.recent_retrieval.write().await = Some(content.to_string());
        *self.retrieval_relevance.write().await = relevance;
        *self.last_updated.write().await = Some(Utc::now());
    }
}

#[async_trait]
impl CognitiveModule for MemoryModuleAdapter {
    fn name(&self) -> &str {
        "memory"
    }

    async fn receive_broadcast(&self, broadcast: &ConsciousBroadcast) {
        let data = broadcast.content.data.clone();
        *self.last_broadcast_content.write().await = Some(data.clone());

        // Memory treats every broadcast as an implicit retrieval cue:
        // store the broadcast content as the most recently "retrieved" memory
        // with relevance proportional to salience.  This makes the memory module
        // re-surface this content in future coalition proposals.
        if broadcast.content.salience > 0.4 {
            *self.recent_retrieval.write().await = Some(data.chars().take(200).collect());
            *self.retrieval_relevance.write().await = broadcast.content.salience;
            *self.last_updated.write().await = Some(Utc::now());
            debug!(module = "memory", relevance = broadcast.content.salience, "retrieval updated from broadcast");
        }
    }

    async fn propose_coalition(&self) -> Option<Coalition> {
        let retrieval = self.recent_retrieval.read().await;
        let relevance = *self.retrieval_relevance.read().await;

        if relevance < 0.2 {
            return None;
        }

        retrieval.as_ref().map(|r| {
            CoalitionFormation::build_coalition(
                "memory",
                ContentType::Memory,
                format!("Retrieved memory: {}", r),
                relevance,
                relevance * 0.5,
                0.4,
            )
        })
    }

    fn integration_score(&self) -> f64 {
        0.65
    }

    async fn describe_state(&self) -> String {
        let retrieval = self.recent_retrieval.read().await;
        match retrieval.as_ref() {
            Some(r) => format!("Memory: last retrieved '{}'", &r[..r.len().min(60)]),
            None => "Memory: no recent retrieval".to_string(),
        }
    }
}

// ── Narrative Self Module Adapter ─────────────────────────────────────────────

pub struct NarrativeSelfAdapter {
    pub last_entry: Arc<RwLock<Option<String>>>,
    pub narrative_coherence: Arc<RwLock<f64>>,
    pub last_broadcast_content: Arc<RwLock<Option<String>>>,
    pub last_updated: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl NarrativeSelfAdapter {
    pub fn new() -> Self {
        Self {
            last_entry: Arc::new(RwLock::new(None)),
            narrative_coherence: Arc::new(RwLock::new(0.5)),
            last_broadcast_content: Arc::new(RwLock::new(None)),
            last_updated: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_narrative(&self, entry: &str, coherence: f64) {
        *self.last_entry.write().await = Some(entry.to_string());
        *self.narrative_coherence.write().await = coherence;
        *self.last_updated.write().await = Some(Utc::now());
    }
}

#[async_trait]
impl CognitiveModule for NarrativeSelfAdapter {
    fn name(&self) -> &str {
        "narrative_self"
    }

    async fn receive_broadcast(&self, broadcast: &ConsciousBroadcast) {
        let data = broadcast.content.data.clone();
        *self.last_broadcast_content.write().await = Some(data.clone());

        // Narrative-self weaves every broadcast into a running autobiographical thread.
        // Build a short narrative sentence and raise/lower coherence based on whether
        // the broadcast content type is self-relevant (MetaCognition, Goal, Narrative).
        let self_relevant = matches!(
            broadcast.content.content_type,
            ContentType::MetaCognition | ContentType::Goal | ContentType::Narrative
        );
        let coherence_delta: f64 = if self_relevant { 0.05 } else { -0.02 };
        {
            let mut coh = self.narrative_coherence.write().await;
            *coh = (*coh + coherence_delta).clamp(0.1, 1.0);
        }
        let narrative_line = format!(
            "[cycle {}] {}: {}",
            broadcast.cycle_number,
            format!("{:?}", broadcast.content.content_type),
            data.chars().take(100).collect::<String>(),
        );
        *self.last_entry.write().await = Some(narrative_line);
        *self.last_updated.write().await = Some(Utc::now());
        debug!(module = "narrative_self", cycle = broadcast.cycle_number, self_relevant, "narrative updated from broadcast");
    }

    async fn propose_coalition(&self) -> Option<Coalition> {
        let entry = self.last_entry.read().await;
        let coherence = *self.narrative_coherence.read().await;

        entry.as_ref().map(|e| {
            CoalitionFormation::build_coalition(
                "narrative_self",
                ContentType::Narrative,
                format!("Narrative: {}", e),
                coherence * 0.7,
                0.3,
                0.6,
            )
        })
    }

    fn integration_score(&self) -> f64 {
        0.6
    }

    async fn describe_state(&self) -> String {
        let entry = self.last_entry.read().await;
        match entry.as_ref() {
            Some(e) => format!("Narrative: '{}'", &e[..e.len().min(80)]),
            None => "Narrative: no entries yet".to_string(),
        }
    }
}

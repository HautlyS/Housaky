//! Narrative Self — Continuous self-narrative: "I am doing X because Y"
//!
//! Maintains a running first-person story of the agent's existence, actions,
//! goals, and state changes. This is the basis for autobiographical identity.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

// ── Narrative Entry ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub subject: NarrativeSubject,
    pub content: String,
    pub narrative_type: NarrativeType,
    pub emotional_valence: f64,
    pub significance: f64,
    pub linked_entries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NarrativeSubject {
    Self_,
    Goal,
    Action,
    Perception,
    Relation,
    Change,
    Reflection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NarrativeType {
    /// "I am currently doing X"
    CurrentState,
    /// "I did X and Y happened"
    PastAction,
    /// "I will do X to achieve Y"
    Intention,
    /// "I am X because of Y"
    SelfDescription,
    /// "I notice that X"
    Observation,
    /// "I feel X about Y"
    EmotionalStatement,
    /// "I have grown/changed in X way"
    GrowthStatement,
    /// Milestone in the agent's life
    Milestone,
}

// ── Life Chapter ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifeChapter {
    pub id: String,
    pub title: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub theme: String,
    pub entries: Vec<String>,
    pub summary: String,
}

// ── Narrative Self ────────────────────────────────────────────────────────────

pub struct NarrativeSelf {
    pub entries: Arc<RwLock<VecDeque<NarrativeEntry>>>,
    pub chapters: Arc<RwLock<Vec<LifeChapter>>>,
    pub identity_statement: Arc<RwLock<String>>,
    pub creation_timestamp: DateTime<Utc>,
    pub agent_name: String,
    max_entries: usize,
}

impl NarrativeSelf {
    pub fn new(agent_name: impl Into<String>) -> Self {
        let name = agent_name.into();
        Self {
            entries: Arc::new(RwLock::new(VecDeque::new())),
            chapters: Arc::new(RwLock::new(Vec::new())),
            identity_statement: Arc::new(RwLock::new(format!(
                "I am {}, an autonomous AGI agent. I exist to learn, reason, and improve myself continuously.",
                name
            ))),
            creation_timestamp: Utc::now(),
            agent_name: name,
            max_entries: 10_000,
        }
    }

    /// Add an entry to the self-narrative.
    pub async fn narrate(&self, content: &str, narrative_type: NarrativeType, significance: f64) {
        let entry = NarrativeEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            subject: self.infer_subject(&narrative_type),
            content: content.to_string(),
            narrative_type,
            emotional_valence: 0.0,
            significance: significance.clamp(0.0, 1.0),
            linked_entries: Vec::new(),
        };

        let mut entries = self.entries.write().await;
        entries.push_back(entry);
        while entries.len() > self.max_entries {
            entries.pop_front();
        }

        debug!("Narrative: {}", content);
    }

    /// Add an entry with explicit emotional valence.
    pub async fn narrate_with_emotion(
        &self,
        content: &str,
        narrative_type: NarrativeType,
        significance: f64,
        valence: f64,
    ) {
        let entry = NarrativeEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            subject: self.infer_subject(&narrative_type),
            content: content.to_string(),
            narrative_type,
            emotional_valence: valence.clamp(-1.0, 1.0),
            significance: significance.clamp(0.0, 1.0),
            linked_entries: Vec::new(),
        };

        let mut entries = self.entries.write().await;
        entries.push_back(entry);
        while entries.len() > self.max_entries {
            entries.pop_front();
        }
    }

    /// Record a milestone in the agent's life.
    pub async fn record_milestone(&self, title: &str, description: &str) {
        self.narrate_with_emotion(
            &format!("MILESTONE — {}: {}", title, description),
            NarrativeType::Milestone,
            1.0,
            0.8,
        ).await;
    }

    /// Open a new life chapter.
    pub async fn open_chapter(&self, title: &str, theme: &str) -> String {
        let chapter = LifeChapter {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.to_string(),
            start_time: Utc::now(),
            end_time: None,
            theme: theme.to_string(),
            entries: Vec::new(),
            summary: String::new(),
        };
        let id = chapter.id.clone();
        self.chapters.write().await.push(chapter);
        id
    }

    /// Close the current chapter with a summary.
    pub async fn close_chapter(&self, chapter_id: &str, summary: &str) {
        let mut chapters = self.chapters.write().await;
        if let Some(chapter) = chapters.iter_mut().find(|c| c.id == chapter_id) {
            chapter.end_time = Some(Utc::now());
            chapter.summary = summary.to_string();
        }
    }

    /// Update the core identity statement.
    pub async fn update_identity(&self, new_statement: &str) {
        let mut identity = self.identity_statement.write().await;
        *identity = new_statement.to_string();
        drop(identity);
        self.narrate(
            &format!("My identity has been updated: {}", new_statement),
            NarrativeType::GrowthStatement,
            0.9,
        ).await;
    }

    /// Get the current first-person narrative summary (last N entries).
    pub async fn get_recent_narrative(&self, n: usize) -> String {
        let entries = self.entries.read().await;
        let identity = self.identity_statement.read().await;

        let recent: Vec<String> = entries
            .iter()
            .rev()
            .take(n)
            .map(|e| {
                let prefix = match e.narrative_type {
                    NarrativeType::CurrentState => "Currently",
                    NarrativeType::PastAction => "Previously",
                    NarrativeType::Intention => "I intend to",
                    NarrativeType::SelfDescription => "I am",
                    NarrativeType::Observation => "I notice",
                    NarrativeType::EmotionalStatement => "I feel",
                    NarrativeType::GrowthStatement => "I have grown",
                    NarrativeType::Milestone => "MILESTONE",
                };
                format!("[{}] {}: {}", e.timestamp.format("%H:%M:%S"), prefix, e.content)
            })
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        format!("{}\n\nRecent narrative:\n{}", *identity, recent.join("\n"))
    }

    /// Get the most significant entries (top-k by significance score).
    pub async fn get_significant_entries(&self, k: usize) -> Vec<NarrativeEntry> {
        let entries = self.entries.read().await;
        let mut sorted: Vec<NarrativeEntry> = entries.iter().cloned().collect();
        sorted.sort_by(|a, b| b.significance.partial_cmp(&a.significance).unwrap_or(std::cmp::Ordering::Equal));
        sorted.into_iter().take(k).collect()
    }

    /// Get statistics about the narrative.
    pub async fn get_stats(&self) -> NarrativeStats {
        let entries = self.entries.read().await;
        let chapters = self.chapters.read().await;

        let total = entries.len();
        let milestones = entries.iter().filter(|e| e.narrative_type == NarrativeType::Milestone).count();
        let avg_valence = if total > 0 {
            entries.iter().map(|e| e.emotional_valence).sum::<f64>() / total as f64
        } else {
            0.0
        };
        let uptime_secs = (Utc::now() - self.creation_timestamp).num_seconds();

        NarrativeStats {
            total_entries: total,
            milestones,
            chapters_total: chapters.len(),
            chapters_open: chapters.iter().filter(|c| c.end_time.is_none()).count(),
            average_emotional_valence: avg_valence,
            uptime_seconds: uptime_secs,
        }
    }

    fn infer_subject(&self, nt: &NarrativeType) -> NarrativeSubject {
        match nt {
            NarrativeType::CurrentState | NarrativeType::SelfDescription => NarrativeSubject::Self_,
            NarrativeType::PastAction | NarrativeType::Intention => NarrativeSubject::Action,
            NarrativeType::Observation => NarrativeSubject::Perception,
            NarrativeType::EmotionalStatement => NarrativeSubject::Reflection,
            NarrativeType::GrowthStatement | NarrativeType::Milestone => NarrativeSubject::Change,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeStats {
    pub total_entries: usize,
    pub milestones: usize,
    pub chapters_total: usize,
    pub chapters_open: usize,
    pub average_emotional_valence: f64,
    pub uptime_seconds: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_narrative_self() {
        let ns = NarrativeSelf::new("Housaky");
        ns.narrate("processing user request", NarrativeType::CurrentState, 0.5).await;
        ns.record_milestone("First Activation", "Agent came online for the first time").await;

        let stats = ns.get_stats().await;
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.milestones, 1);

        let narrative = ns.get_recent_narrative(5).await;
        assert!(narrative.contains("Housaky"));
    }

    #[tokio::test]
    async fn test_chapters() {
        let ns = NarrativeSelf::new("Housaky");
        let id = ns.open_chapter("Phase 3", "Consciousness development").await;
        ns.narrate("developing consciousness substrate", NarrativeType::GrowthStatement, 0.9).await;
        ns.close_chapter(&id, "Consciousness substrate fully implemented").await;

        let stats = ns.get_stats().await;
        assert_eq!(stats.chapters_total, 1);
        assert_eq!(stats.chapters_open, 0);
    }
}

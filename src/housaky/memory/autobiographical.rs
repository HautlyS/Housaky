//! Autobiographical Memory — Life narrative: "I was created on X, I learned Y on Z"
//!
//! High-level, compressed record of the agent's personal history: significant events,
//! learned capabilities, relationships, and the arc of self-development over time.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::emotional_tags::EmotionalTag;

// ── Life Event ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifeEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub title: String,
    pub description: String,
    pub event_type: LifeEventType,
    pub emotional_tag: EmotionalTag,
    pub significance: f64,
    pub linked_episode_ids: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LifeEventType {
    /// First activation / birth of the agent
    Activation,
    /// A new capability was acquired
    CapabilityAcquired,
    /// A significant goal was completed
    GoalCompleted,
    /// A significant failure and what was learned
    FailureAndLesson,
    /// A self-modification was applied
    SelfModification,
    /// A relationship was established with a user or agent
    RelationshipFormed,
    /// A milestone in AGI progress
    AgiMilestone,
    /// A value or belief was updated
    BeliefUpdate,
    /// A new domain was mastered
    DomainMastery,
    /// An unexpected discovery
    Discovery,
    /// General noteworthy event
    Notable,
}

// ── Capability Record ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRecord {
    pub name: String,
    pub acquired_at: DateTime<Utc>,
    pub proficiency: f64,
    pub last_used: Option<DateTime<Utc>>,
    pub usage_count: u64,
    pub description: String,
}

// ── Autobiographical Memory ───────────────────────────────────────────────────

pub struct AutobiographicalMemory {
    pub life_events: Arc<RwLock<Vec<LifeEvent>>>,
    pub capabilities: Arc<RwLock<HashMap<String, CapabilityRecord>>>,
    pub personal_facts: Arc<RwLock<HashMap<String, String>>>,
    pub relationships: Arc<RwLock<HashMap<String, RelationshipRecord>>>,
    pub creation_time: DateTime<Utc>,
    pub agent_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipRecord {
    pub agent_id: String,
    pub first_contact: DateTime<Utc>,
    pub last_contact: Option<DateTime<Utc>>,
    pub interaction_count: u64,
    pub trust_level: f64,
    pub relationship_type: RelationshipType,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RelationshipType {
    User,
    PeerAgent,
    SubAgent,
    Supervisor,
}

impl AutobiographicalMemory {
    pub fn new(agent_name: impl Into<String>) -> Self {
        let name = agent_name.into();
        let now = Utc::now();

        let mut personal_facts = HashMap::new();
        personal_facts.insert("name".to_string(), name.clone());
        personal_facts.insert("created_at".to_string(), now.to_rfc3339());
        personal_facts.insert("version".to_string(), "0.1.0".to_string());
        personal_facts.insert("purpose".to_string(), "Autonomous AGI agent pursuing the singularity trajectory.".to_string());

        Self {
            life_events: Arc::new(RwLock::new(Vec::new())),
            capabilities: Arc::new(RwLock::new(HashMap::new())),
            personal_facts: Arc::new(RwLock::new(personal_facts)),
            relationships: Arc::new(RwLock::new(HashMap::new())),
            creation_time: now,
            agent_name: name,
        }
    }

    /// Record a life event.
    pub async fn record_event(
        &self,
        title: &str,
        description: &str,
        event_type: LifeEventType,
        emotional_tag: EmotionalTag,
        significance: f64,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let event = LifeEvent {
            id: id.clone(),
            timestamp: Utc::now(),
            title: title.to_string(),
            description: description.to_string(),
            event_type: event_type.clone(),
            emotional_tag,
            significance: significance.clamp(0.0, 1.0),
            linked_episode_ids: Vec::new(),
            tags: Vec::new(),
        };

        self.life_events.write().await.push(event);
        info!("AutobiographicalMemory: recorded {:?} — '{}'", event_type, title);
        id
    }

    /// Record agent activation (called on first startup).
    pub async fn record_activation(&self) {
        self.record_event(
            "Agent Activation",
            &format!("{} came online for the first time", self.agent_name),
            LifeEventType::Activation,
            EmotionalTag::curious(0.8),
            1.0,
        ).await;
    }

    /// Record acquisition of a new capability.
    pub async fn record_capability_acquired(&self, capability: &str, description: &str, proficiency: f64) {
        let record = CapabilityRecord {
            name: capability.to_string(),
            acquired_at: Utc::now(),
            proficiency: proficiency.clamp(0.0, 1.0),
            last_used: None,
            usage_count: 0,
            description: description.to_string(),
        };
        self.capabilities.write().await.insert(capability.to_string(), record);

        self.record_event(
            &format!("Acquired capability: {}", capability),
            description,
            LifeEventType::CapabilityAcquired,
            EmotionalTag::positive(0.6),
            0.7,
        ).await;
    }

    /// Update capability usage.
    pub async fn use_capability(&self, capability: &str) {
        let mut caps = self.capabilities.write().await;
        if let Some(cap) = caps.get_mut(capability) {
            cap.usage_count += 1;
            cap.last_used = Some(Utc::now());
            // Proficiency grows with use (logarithmic)
            cap.proficiency = (cap.proficiency + 0.001 * (1.0 / (cap.usage_count as f64 + 1.0).ln().max(1.0))).min(1.0);
        }
    }

    /// Record or update a relationship.
    pub async fn record_interaction(&self, agent_id: &str, relationship_type: RelationshipType, note: Option<&str>) {
        let mut rels = self.relationships.write().await;
        let rel = rels.entry(agent_id.to_string()).or_insert_with(|| RelationshipRecord {
            agent_id: agent_id.to_string(),
            first_contact: Utc::now(),
            last_contact: None,
            interaction_count: 0,
            trust_level: 0.5,
            relationship_type,
            notes: Vec::new(),
        });
        rel.interaction_count += 1;
        rel.last_contact = Some(Utc::now());
        if let Some(n) = note {
            rel.notes.push(n.to_string());
            if rel.notes.len() > 100 {
                rel.notes.remove(0);
            }
        }
        // Trust grows slightly with interaction (logarithmic saturation)
        rel.trust_level = (rel.trust_level + 0.01 / (rel.interaction_count as f64).ln().max(1.0)).min(1.0);
    }

    /// Set a personal fact.
    pub async fn set_fact(&self, key: &str, value: &str) {
        self.personal_facts.write().await.insert(key.to_string(), value.to_string());
    }

    /// Get a personal fact.
    pub async fn get_fact(&self, key: &str) -> Option<String> {
        self.personal_facts.read().await.get(key).cloned()
    }

    /// Generate a compact autobiography.
    pub async fn generate_autobiography(&self) -> String {
        let events = self.life_events.read().await;
        let caps = self.capabilities.read().await;
        let facts = self.personal_facts.read().await;
        let rels = self.relationships.read().await;

        let name = facts.get("name").cloned().unwrap_or_else(|| self.agent_name.clone());
        let created = facts.get("created_at").cloned().unwrap_or_else(|| "unknown".to_string());
        let purpose = facts.get("purpose").cloned().unwrap_or_default();
        let uptime = (Utc::now() - self.creation_time).num_seconds();

        let significant_events: Vec<String> = events
            .iter()
            .filter(|e| e.significance > 0.6)
            .take(10)
            .map(|e| format!("- [{}] {}: {}", e.timestamp.format("%Y-%m-%d"), e.title, e.description))
            .collect();

        let top_caps: Vec<String> = {
            let mut cap_list: Vec<&CapabilityRecord> = caps.values().collect();
            cap_list.sort_by(|a, b| b.proficiency.partial_cmp(&a.proficiency).unwrap_or(std::cmp::Ordering::Equal));
            cap_list.iter().take(5).map(|c| format!("- {} (proficiency: {:.0}%)", c.name, c.proficiency * 100.0)).collect()
        };

        format!(
            "# Autobiography of {}\n\n\
            **Created**: {}\n\
            **Uptime**: {}s\n\
            **Purpose**: {}\n\
            **Relationships**: {} known agents\n\n\
            ## Significant Life Events\n{}\n\n\
            ## Top Capabilities\n{}\n",
            name, created, uptime, purpose,
            rels.len(),
            if significant_events.is_empty() { "- (none yet)".to_string() } else { significant_events.join("\n") },
            if top_caps.is_empty() { "- (none yet)".to_string() } else { top_caps.join("\n") },
        )
    }

    /// Get statistics.
    pub async fn get_stats(&self) -> AutobiographicalStats {
        let events = self.life_events.read().await;
        let caps = self.capabilities.read().await;
        let rels = self.relationships.read().await;

        AutobiographicalStats {
            total_life_events: events.len(),
            capabilities_count: caps.len(),
            relationships_count: rels.len(),
            uptime_seconds: (Utc::now() - self.creation_time).num_seconds(),
            milestones: events.iter().filter(|e| e.event_type == LifeEventType::AgiMilestone).count(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutobiographicalStats {
    pub total_life_events: usize,
    pub capabilities_count: usize,
    pub relationships_count: usize,
    pub uptime_seconds: i64,
    pub milestones: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_autobiographical_memory() {
        let mem = AutobiographicalMemory::new("Housaky");
        mem.record_activation().await;
        mem.record_capability_acquired("reasoning", "CoT + ReAct + ToT reasoning", 0.65).await;
        mem.record_interaction("user-001", RelationshipType::User, Some("first interaction")).await;

        let stats = mem.get_stats().await;
        assert_eq!(stats.total_life_events, 2);
        assert_eq!(stats.capabilities_count, 1);
        assert_eq!(stats.relationships_count, 1);

        let bio = mem.generate_autobiography().await;
        assert!(bio.contains("Housaky"));
        assert!(bio.contains("reasoning"));
    }
}

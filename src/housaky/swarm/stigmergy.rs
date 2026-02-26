use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentMark {
    pub id: String,
    pub key: String,
    pub value: serde_json::Value,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub read_count: u64,
    pub influence_radius: f64,
    pub mark_type: MarkType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MarkType {
    Warning,
    Opportunity,
    Progress,
    Completion,
    Blocker,
    Insight,
    ResourceReservation,
    Custom(String),
}

impl EnvironmentMark {
    pub fn new(key: &str, value: serde_json::Value, author: &str, mark_type: MarkType) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            key: key.to_string(),
            value,
            author: author.to_string(),
            created_at: now,
            updated_at: now,
            read_count: 0,
            influence_radius: 1.0,
            mark_type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedEnvironment {
    pub marks: HashMap<String, EnvironmentMark>,
    pub update_count: u64,
}

impl SharedEnvironment {
    pub fn new() -> Self {
        Self { marks: HashMap::new(), update_count: 0 }
    }

    pub fn write_mark(&mut self, mark: EnvironmentMark) {
        self.marks.insert(mark.key.clone(), mark);
        self.update_count += 1;
    }

    pub fn read_mark(&mut self, key: &str) -> Option<&EnvironmentMark> {
        if let Some(m) = self.marks.get_mut(key) {
            m.read_count += 1;
        }
        self.marks.get(key)
    }

    pub fn marks_by_type(&self, mark_type: &MarkType) -> Vec<&EnvironmentMark> {
        self.marks.values().filter(|m| &m.mark_type == mark_type).collect()
    }

    pub fn marks_by_author(&self, author: &str) -> Vec<&EnvironmentMark> {
        self.marks.values().filter(|m| m.author == author).collect()
    }

    pub fn remove_mark(&mut self, key: &str) -> Option<EnvironmentMark> {
        self.marks.remove(key)
    }

    pub fn stats(&self) -> StigmergyStats {
        let total = self.marks.len();
        let warnings = self.marks.values().filter(|m| m.mark_type == MarkType::Warning).count();
        let opportunities = self.marks.values().filter(|m| m.mark_type == MarkType::Opportunity).count();
        let blockers = self.marks.values().filter(|m| m.mark_type == MarkType::Blocker).count();
        StigmergyStats { total_marks: total, warnings, opportunities, blockers, update_count: self.update_count }
    }
}

impl Default for SharedEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StigmergyStats {
    pub total_marks: usize,
    pub warnings: usize,
    pub opportunities: usize,
    pub blockers: usize,
    pub update_count: u64,
}

pub struct StigmergyLayer {
    pub env: Arc<RwLock<SharedEnvironment>>,
}

impl StigmergyLayer {
    pub fn new() -> Self {
        Self { env: Arc::new(RwLock::new(SharedEnvironment::new())) }
    }

    pub async fn mark(&self, key: &str, value: serde_json::Value, author: &str, mark_type: MarkType) {
        let mark = EnvironmentMark::new(key, value, author, mark_type);
        self.env.write().await.write_mark(mark);
        tracing::debug!("Stigmergy mark written: key={} by={}", key, author);
    }

    pub async fn read(&self, key: &str) -> Option<serde_json::Value> {
        self.env.write().await.read_mark(key).map(|m| m.value.clone())
    }

    pub async fn signal_opportunity(&self, description: &str, agent_id: &str, priority: f64) {
        let val = serde_json::json!({ "description": description, "priority": priority });
        self.mark(&format!("opportunity:{}", uuid::Uuid::new_v4()), val, agent_id, MarkType::Opportunity).await;
    }

    pub async fn signal_blocker(&self, resource: &str, agent_id: &str, reason: &str) {
        let val = serde_json::json!({ "resource": resource, "reason": reason });
        self.mark(&format!("blocker:{}", resource), val, agent_id, MarkType::Blocker).await;
    }

    pub async fn clear_blocker(&self, resource: &str) {
        self.env.write().await.remove_mark(&format!("blocker:{}", resource));
    }

    pub async fn get_opportunities(&self) -> Vec<serde_json::Value> {
        self.env.read().await
            .marks_by_type(&MarkType::Opportunity)
            .iter()
            .map(|m| m.value.clone())
            .collect()
    }

    pub async fn get_blockers(&self) -> Vec<serde_json::Value> {
        self.env.read().await
            .marks_by_type(&MarkType::Blocker)
            .iter()
            .map(|m| m.value.clone())
            .collect()
    }

    pub async fn stats(&self) -> StigmergyStats {
        self.env.read().await.stats()
    }
}

impl Default for StigmergyLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mark_and_read() {
        let layer = StigmergyLayer::new();
        layer.mark("test_key", serde_json::json!("hello"), "agent-1", MarkType::Insight).await;
        let val = layer.read("test_key").await;
        assert_eq!(val, Some(serde_json::json!("hello")));
    }

    #[tokio::test]
    async fn test_blocker_lifecycle() {
        let layer = StigmergyLayer::new();
        layer.signal_blocker("gpu-0", "agent-1", "OOM").await;
        let blockers = layer.get_blockers().await;
        assert_eq!(blockers.len(), 1);
        layer.clear_blocker("gpu-0").await;
        let blockers = layer.get_blockers().await;
        assert_eq!(blockers.len(), 0);
    }
}

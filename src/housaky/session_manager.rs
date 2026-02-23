use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::housaky::memory::hierarchical::HierarchicalMemory;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub conversation_history: Vec<ConversationEntry>,
    pub context: SessionContext,
    pub active_goals: Vec<String>,
    pub working_memory_snapshot: Option<WorkingMemorySnapshot>,
    pub metadata: SessionMetadata,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConversationEntry {
    pub timestamp: DateTime<Utc>,
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Vec<ToolCallEntry>,
    pub tool_results: Vec<ToolResultEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCallEntry {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResultEntry {
    pub tool_call_id: String,
    pub result: String,
    pub success: bool,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SessionContext {
    pub user_preferences: HashMap<String, String>,
    pub current_task: Option<String>,
    pub topic: Option<String>,
    pub entities: Vec<String>,
    pub recent_topics: VecDeque<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub total_messages: u64,
    pub total_tokens: u64,
    pub tool_calls_count: u64,
    pub successful_tool_calls: u64,
    pub average_response_time_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkingMemorySnapshot {
    pub items: Vec<WorkingMemoryItem>,
    pub total_tokens: usize,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkingMemoryItem {
    pub id: String,
    pub content: String,
    pub importance: f64,
    pub access_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionState {
    pub session: Session,
    pub persistent_memories: Vec<PersistentMemory>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistentMemory {
    pub key: String,
    pub content: String,
    pub category: MemoryCategory,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryCategory {
    Preference,
    Fact,
    Procedure,
    Goal,
}

pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    active_session: Arc<RwLock<Option<String>>>,
    session_history: Arc<RwLock<VecDeque<String>>>,
    max_history_size: usize,
    storage_path: PathBuf,
    memory: Option<Arc<HierarchicalMemory>>,
}

impl SessionManager {
    pub fn new(workspace_dir: &PathBuf) -> Self {
        let storage_path = workspace_dir.join(".housaky").join("sessions");
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            active_session: Arc::new(RwLock::new(None)),
            session_history: Arc::new(RwLock::new(VecDeque::new())),
            max_history_size: 10,
            storage_path,
            memory: None,
        }
    }

    pub fn with_memory(mut self, memory: Arc<HierarchicalMemory>) -> Self {
        self.memory = Some(memory);
        self
    }

    pub async fn initialize(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.storage_path).await?;

        if self.storage_path.exists() {
            let mut entries = tokio::fs::read_dir(&self.storage_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let content = tokio::fs::read_to_string(&path).await?;
                    if let Ok(session) = serde_json::from_str::<Session>(&content) {
                        let id = session.id.clone();
                        self.sessions.write().await.insert(id, session);
                    }
                }
            }
            info!("Loaded sessions from storage");
        }

        Ok(())
    }

    pub async fn create_session(&self) -> Result<Session> {
        let session = Session {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            last_active: Utc::now(),
            conversation_history: Vec::new(),
            context: SessionContext::default(),
            active_goals: Vec::new(),
            working_memory_snapshot: None,
            metadata: SessionMetadata {
                total_messages: 0,
                total_tokens: 0,
                tool_calls_count: 0,
                successful_tool_calls: 0,
                average_response_time_ms: 0,
            },
        };

        let id = session.id.clone();
        self.sessions.write().await.insert(id.clone(), session.clone());
        *self.active_session.write().await = Some(id);

        self.session_history.write().await.push_front(session.id.clone());
        self.prune_history().await;

        self.save_session(&session).await?;

        info!("Created new session: {}", session.id);
        Ok(session)
    }

    pub async fn get_session(&self, session_id: Option<&str>) -> Result<Session> {
        match session_id {
            Some(id) => {
                let sessions = self.sessions.read().await;
                if let Some(session) = sessions.get(id) {
                    let mut session = session.clone();
                    session.last_active = Utc::now();
                    *self.active_session.write().await = Some(id.to_string());
                    return Ok(session);
                }

                if let Ok(session) = self.load_session(id).await {
                    let mut sessions = self.sessions.write().await;
                    sessions.insert(id.to_string(), session.clone());
                    *self.active_session.write().await = Some(id.to_string());
                    return Ok(session);
                }

                anyhow::bail!("Session not found: {}", id)
            }
            None => {
                let active = self.active_session.read().await.clone();
                if let Some(id) = active {
                    let sessions = self.sessions.read().await;
                    if let Some(session) = sessions.get(&id) {
                        return Ok(session.clone());
                    }
                }

                self.create_session().await
            }
        }
    }

    pub async fn save_session(&self, session: &Session) -> Result<()> {
        let path = self.storage_path.join(format!("{}.json", session.id));
        let content = serde_json::to_string_pretty(session)?;
        tokio::fs::write(&path, content).await?;
        Ok(())
    }

    async fn load_session(&self, session_id: &str) -> Result<Session> {
        let path = self.storage_path.join(format!("{}.json", session_id));
        let content = tokio::fs::read_to_string(&path).await?;
        let session: Session = serde_json::from_str(&content)?;
        Ok(session)
    }

    pub async fn add_message(
        &self,
        session_id: &str,
        role: MessageRole,
        content: String,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.conversation_history.push(ConversationEntry {
                timestamp: Utc::now(),
                role,
                content: content.clone(),
                tool_calls: vec![],
                tool_results: vec![],
            });

            session.last_active = Utc::now();
            session.metadata.total_messages += 1;

            if let Some(topic) = self.extract_topic(&content) {
                session.context.recent_topics.push_front(topic.clone());
                if session.context.recent_topics.len() > 5 {
                    session.context.recent_topics.pop_back();
                }
                session.context.topic = Some(topic);
            }

            drop(sessions);
            let sessions = self.sessions.read().await;
            if let Some(session) = sessions.get(session_id) {
                self.save_session(session).await?;
            }
        }

        Ok(())
    }

    fn extract_topic(&self, content: &str) -> Option<String> {
        let words: Vec<&str> = content.split_whitespace().collect();
        if words.len() > 3 {
            Some(words[0..5].join(" "))
        } else {
            None
        }
    }

    pub async fn add_tool_call(
        &self,
        session_id: &str,
        call: ToolCallEntry,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            if let Some(entry) = session.conversation_history.last_mut() {
                entry.tool_calls.push(call.clone());
            }
            session.metadata.tool_calls_count += 1;

            drop(sessions);
            let sessions = self.sessions.read().await;
            if let Some(session) = sessions.get(session_id) {
                self.save_session(session).await?;
            }
        }

        Ok(())
    }

    pub async fn add_tool_result(
        &self,
        session_id: &str,
        tool_call_id: &str,
        result: String,
        success: bool,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            if let Some(entry) = session.conversation_history.last_mut() {
                entry.tool_results.push(ToolResultEntry {
                    tool_call_id: tool_call_id.to_string(),
                    result,
                    success,
                    timestamp: Utc::now(),
                });
            }

            if success {
                session.metadata.successful_tool_calls += 1;
            }
        }

        Ok(())
    }

    pub async fn get_conversation_context(
        &self,
        session_id: &str,
        max_entries: usize,
    ) -> Result<Vec<ConversationEntry>> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            let start = session
                .conversation_history
                .len()
                .saturating_sub(max_entries);
            return Ok(session.conversation_history[start..].to_vec());
        }

        Ok(vec![])
    }

    pub async fn list_sessions(&self) -> Vec<SessionSummary> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .map(|s| SessionSummary {
                id: s.id.clone(),
                created_at: s.created_at,
                last_active: s.last_active,
                message_count: s.conversation_history.len() as u64,
            })
            .collect()
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<bool> {
        let mut sessions = self.sessions.write().await;
        if sessions.remove(session_id).is_some() {
            let path = self.storage_path.join(format!("{}.json", session_id));
            if path.exists() {
                tokio::fs::remove_file(&path).await?;
            }

            let mut history = self.session_history.write().await;
            history.retain(|id| id != session_id);

            info!("Deleted session: {}", session_id);
            return Ok(true);
        }

        Ok(false)
    }

    async fn prune_history(&self) {
        let mut history = self.session_history.write().await;
        while history.len() > self.max_history_size {
            if let Some(old_id) = history.pop_back() {
                let sessions = self.sessions.read().await;
                if !sessions.contains_key(&old_id) {
                    let path = self.storage_path.join(format!("{}.json", old_id));
                    if path.exists() {
                        let _ = tokio::fs::remove_file(&path).await;
                    }
                }
            }
        }
    }

    pub async fn export_session(&self, session_id: &str) -> Result<String> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            Ok(serde_json::to_string_pretty(session)?)
        } else {
            anyhow::bail!("Session not found")
        }
    }

    pub async fn import_session(&self, json: &str) -> Result<Session> {
        let mut session: Session = serde_json::from_str(json)?;
        session.id = uuid::Uuid::new_v4().to_string();
        session.created_at = Utc::now();
        session.last_active = Utc::now();

        let id = session.id.clone();
        self.sessions.write().await.insert(id.clone(), session.clone());
        self.save_session(&session).await?;

        info!("Imported session: {}", session.id);
        Ok(session)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub message_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_session() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = SessionManager::new(&temp_dir.path().to_path_buf());
        manager.initialize().await.unwrap();

        let session = manager.create_session().await.unwrap();
        assert!(!session.id.is_empty());
    }

    #[tokio::test]
    async fn test_add_message() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = SessionManager::new(&temp_dir.path().to_path_buf());
        manager.initialize().await.unwrap();

        let session = manager.create_session().await.unwrap();
        manager
            .add_message(&session.id, MessageRole::User, "Hello".to_string())
            .await
            .unwrap();

        let context = manager
            .get_conversation_context(&session.id, 10)
            .await
            .unwrap();
        assert_eq!(context.len(), 1);
    }
}

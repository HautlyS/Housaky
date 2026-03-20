//! Session Storage for Housaky Gateway
//!
//! Persistent storage for chat sessions and message history.
//! Stores sessions in JSON files under ~/.housaky/sessions/

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::debug;

/// A single chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: i64,
    pub token_count: Option<usize>,
}

/// A chat session with metadata and messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub messages: Vec<ChatMessage>,
}

impl ChatSession {
    pub fn new(id: &str) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: id.to_string(),
            title: "New Conversation".to_string(),
            created_at: now,
            updated_at: now,
            messages: Vec::new(),
        }
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn last_message(&self) -> String {
        self.messages
            .last()
            .map(|m| {
                if m.content.len() > 50 {
                    format!("{}...", &m.content[..47])
                } else {
                    m.content.clone()
                }
            })
            .unwrap_or_default()
    }

    pub fn add_message(&mut self, role: &str, content: &str) -> ChatMessage {
        let msg = ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Utc::now().timestamp(),
            token_count: Some(content.len() / 4),
        };
        
        // Update title from first user message
        if self.title == "New Conversation" && role == "user" {
            self.title = if content.len() > 40 {
                format!("{}...", &content[..37])
            } else {
                content.to_string()
            };
        }
        
        self.messages.push(msg.clone());
        self.updated_at = Utc::now().timestamp();
        msg
    }
}

/// Session storage manager
pub struct SessionStorage {
    sessions_dir: PathBuf,
    cache: HashMap<String, ChatSession>,
}

impl SessionStorage {
    pub fn new(sessions_dir: PathBuf) -> Self {
        fs::create_dir_all(&sessions_dir).ok();
        Self {
            sessions_dir,
            cache: HashMap::new(),
        }
    }

    /// Get or create a session
    pub fn get_or_create(&mut self, session_id: &str) -> Result<&mut ChatSession> {
        if !self.cache.contains_key(session_id) {
            let session = self.load_session(session_id).unwrap_or_else(|_| {
                ChatSession::new(session_id)
            });
            self.cache.insert(session_id.to_string(), session);
        }
        Ok(self.cache.get_mut(session_id).unwrap())
    }

    /// Load a session from disk
    fn load_session(&self, session_id: &str) -> Result<ChatSession> {
        let path = self.sessions_dir.join(format!("{}.json", session_id));
        if !path.exists() {
            return Err(anyhow::anyhow!("Session not found: {}", session_id));
        }
        
        let content = fs::read_to_string(&path)
            .context("Failed to read session file")?;
        let session: ChatSession = serde_json::from_str(&content)
            .context("Failed to parse session file")?;
        
        debug!("Loaded session {} with {} messages", session_id, session.messages.len());
        Ok(session)
    }

    /// Save a session to disk
    pub fn save_session(&self, session: &ChatSession) -> Result<()> {
        let path = self.sessions_dir.join(format!("{}.json", session.id));
        let content = serde_json::to_string_pretty(session)
            .context("Failed to serialize session")?;
        
        fs::write(&path, content)
            .context("Failed to write session file")?;
        
        debug!("Saved session {} with {} messages", session.id, session.messages.len());
        Ok(())
    }

    /// Add a message to a session and persist
    pub fn add_message(&mut self, session_id: &str, role: &str, content: &str) -> Result<ChatMessage> {
        // Get or create session and add message
        {
            let session = self.get_or_create(session_id)?;
            session.add_message(role, content);
        }
        // Mutable borrow is now released
        
        // Save the session
        let session = self.cache.get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not in cache"))?;
        self.save_session(session)?;
        
        // Return the message (get it from cache)
        let session = self.cache.get(session_id).unwrap();
        Ok(session.messages.last().cloned().unwrap())
    }

    /// List all sessions
    pub fn list_sessions(&mut self) -> Result<Vec<ChatSession>> {
        let mut sessions = Vec::new();
        
        if !self.sessions_dir.exists() {
            return Ok(sessions);
        }
        
        for entry in fs::read_dir(&self.sessions_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Some(id) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(session) = self.get_or_create(id) {
                        sessions.push(session.clone());
                    }
                }
            }
        }
        
        // Sort by updated_at descending
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        
        Ok(sessions)
    }

    /// Get session history
    pub fn get_history(&mut self, session_id: &str) -> Result<Vec<ChatMessage>> {
        let session = self.get_or_create(session_id)?;
        Ok(session.messages.clone())
    }

    /// Delete a session
    pub fn delete_session(&mut self, session_id: &str) -> Result<()> {
        self.cache.remove(session_id);
        let path = self.sessions_dir.join(format!("{}.json", session_id));
        if path.exists() {
            fs::remove_file(&path).context("Failed to delete session file")?;
        }
        Ok(())
    }

    /// Clear all sessions
    pub fn clear_all(&mut self) -> Result<()> {
        self.cache.clear();
        if self.sessions_dir.exists() {
            for entry in fs::read_dir(&self.sessions_dir)? {
                let entry = entry?;
                fs::remove_file(entry.path()).ok();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_session_creation() {
        let session = ChatSession::new("test-123");
        assert_eq!(session.id, "test-123");
        assert_eq!(session.title, "New Conversation");
        assert_eq!(session.messages.len(), 0);
    }

    #[test]
    fn test_add_message() {
        let mut session = ChatSession::new("test");
        session.add_message("user", "Hello!");
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.title, "Hello!");
    }

    #[test]
    fn test_session_storage() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = SessionStorage::new(temp_dir.path().to_path_buf());
        
        storage.add_message("test-session", "user", "Hello!").unwrap();
        storage.add_message("test-session", "assistant", "Hi there!").unwrap();
        
        let history = storage.get_history("test-session").unwrap();
        assert_eq!(history.len(), 2);
        
        let sessions = storage.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
    }
}

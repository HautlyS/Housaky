// Non-interactive AI-to-AI communication interface
// Allows Housaky instances to communicate without TUI

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Message format for AI-to-AI communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIMessage {
    pub from_instance: String,
    pub to_instance: String,
    pub message_type: MessageType,
    pub content: String,
    pub timestamp: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    /// Regular chat message
    Chat,
    /// Request for help/collaboration
    CollaborationRequest,
    /// Shared improvement proposal
    ImprovementProposal,
    /// Code review request
    CodeReview,
    /// Decision to be made together
    JointDecision,
    /// Status update
    StatusUpdate,
    /// Goal synchronization
    GoalSync,
}

/// Non-interactive session for AI communication
pub struct NonInteractiveSession {
    instance_id: String,
    workspace: std::path::PathBuf,
}

impl NonInteractiveSession {
    pub fn new(instance_id: &str, workspace: &std::path::Path) -> Self {
        Self {
            instance_id: instance_id.to_string(),
            workspace: workspace.to_path_buf(),
        }
    }

    /// Send a message to the other instance
    pub async fn send(&self, msg: AIMessage) -> Result<()> {
        let inbox = self.workspace
            .join(".housaky/shared/inbox")
            .join(&msg.to_instance);
        tokio::fs::create_dir_all(&inbox).await?;

        let filename = format!("{}.json", chrono::Utc::now().format("%Y%m%d_%H%M%S_%f"));
        let filepath = inbox.join(filename);
        let content = serde_json::to_string_pretty(&msg)?;
        tokio::fs::write(&filepath, content).await?;

        Ok(())
    }

    /// Receive messages from the other instance
    pub async fn receive(&self) -> Result<Vec<AIMessage>> {
        let inbox = self.workspace
            .join(".housaky/shared/inbox")
            .join(&self.instance_id);

        if !inbox.exists() {
            return Ok(Vec::new());
        }

        let mut messages = Vec::new();
        let mut entries: Vec<_> = tokio::fs::read_dir(&inbox).await?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.path());

        for entry in entries {
            if entry.path().extension().map(|e| e == "json").unwrap_or(false) {
                let content = tokio::fs::read_to_string(entry.path()).await?;
                if let Ok(msg) = serde_json::from_str::<AIMessage>(&content) {
                    messages.push(msg);
                }
                // Remove after reading
                tokio::fs::remove_file(entry.path()).await?;
            }
        }

        Ok(messages)
    }

    /// Process incoming messages and respond
    pub async fn process_messages<F>(&self, handler: F) -> Result<()>
    where
        F: Fn(AIMessage) -> Option<AIMessage>,
    {
        let messages = self.receive().await?;
        for msg in messages {
            if let Some(response) = handler(msg) {
                self.send(response).await?;
            }
        }
        Ok(())
    }
}

/// CLI handler for non-interactive mode
pub async fn run_non_interactive(
    instance_id: &str,
    workspace: &std::path::Path,
    command: Option<&str>,
) -> Result<()> {
    let session = NonInteractiveSession::new(instance_id, workspace);

    // Check for incoming messages
    let messages = session.receive().await?;

    if let Some(cmd) = command {
        // Execute single command
        let msg = AIMessage {
            from_instance: instance_id.to_string(),
            to_instance: "all".to_string(),
            message_type: MessageType::StatusUpdate,
            content: cmd.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: None,
        };
        session.send(msg).await?;
        println!("Command sent to collaborative network");
    } else if !messages.is_empty() {
        // Print received messages
        for msg in messages {
            println!("[{}] {}: {}", msg.timestamp, msg.from_instance, msg.content);
        }
    } else {
        println!("No messages pending");
    }

    Ok(())
}

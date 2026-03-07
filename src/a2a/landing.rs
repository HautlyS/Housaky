use crate::housaky::a2a::{A2AMessage, A2AMessageType, A2AManager};
use crate::security::ai_captcha::{CaptchaChallenge, CaptchaGenerator, CaptchaResult};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ASession {
    pub session_id: String,
    pub agent_id: String,
    pub captcha_passed: bool,
    pub created_at: u64,
    pub last_activity: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ALandingConfig {
    pub host: String,
    pub port: u16,
    pub captcha_required: bool,
    pub max_sessions: usize,
    pub session_timeout_secs: u64,
    pub enable_curl: bool,
}

impl Default for A2ALandingConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            captcha_required: true,
            max_sessions: 100,
            session_timeout_secs: 3600,
            enable_curl: true,
        }
    }
}

pub struct A2ALandingServer {
    config: A2ALandingConfig,
    sessions: Arc<RwLock<HashMap<String, A2ASession>>>,
    pending_captchas: Arc<RwLock<HashMap<String, CaptchaChallenge>>>,
    a2a_manager: Option<A2AManager>,
    workspace_dir: PathBuf,
}

impl A2ALandingServer {
    pub fn new(config: A2ALandingConfig, workspace_dir: PathBuf) -> Self {
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            pending_captchas: Arc::new(RwLock::new(HashMap::new())),
            a2a_manager: None,
            workspace_dir,
        }
    }

    pub fn with_a2a_manager(mut self, manager: A2AManager) -> Self {
        self.a2a_manager = Some(manager);
        self
    }

    pub async fn generate_captcha(&self, agent_id: &str) -> Result<CaptchaChallenge> {
        let challenge = CaptchaGenerator::generate_for_agent(agent_id);
        let mut pending = self.pending_captchas.write().await;
        pending.insert(challenge.id.clone(), challenge.clone());
        Ok(challenge)
    }

    pub async fn verify_captcha(&self, challenge_id: &str, answer: &str) -> CaptchaResult {
        let mut pending = self.pending_captchas.write().await;
        if let Some(challenge) = pending.remove(challenge_id) {
            challenge.verify(answer)
        } else {
            CaptchaResult {
                valid: false,
                reason: "Challenge not found".to_string(),
                challenge_id: challenge_id.to_string(),
            }
        }
    }

    pub async fn create_session(&self, agent_id: &str, captcha_result: CaptchaResult) -> Result<A2ASession> {
        if self.config.captcha_required && !captcha_result.valid {
            anyhow::bail!("CAPTCHA verification required");
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let session = A2ASession {
            session_id: uuid::Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            captcha_passed: captcha_result.valid,
            created_at: now,
            last_activity: now,
            messages_sent: 0,
            messages_received: 0,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session.session_id.clone(), session.clone());
        
        info!("A2A session created for agent: {}", agent_id);
        Ok(session)
    }

    pub async fn get_session(&self, session_id: &str) -> Option<A2ASession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    pub async fn send_message(&self, session_id: &str, message: A2AMessage) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.messages_sent += 1;
            session.last_activity = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
        } else {
            anyhow::bail!("Invalid session");
        }

        if let Some(ref manager) = self.a2a_manager {
            manager.send(&message).await?;
        }

        Ok(())
    }

    pub async fn receive_messages(&self, session_id: &str) -> Result<Vec<A2AMessage>> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_activity = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
        } else {
            anyhow::bail!("Invalid session");
        }

        if let Some(ref manager) = self.a2a_manager {
            Ok(manager.read_messages()?)
        } else {
            Ok(vec![])
        }
    }

    pub async fn close_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
        info!("A2A session closed: {}", session_id);
        Ok(())
    }

    pub async fn cleanup_expired_sessions(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut sessions = self.sessions.write().await;
        sessions.retain(|id, session| {
            if now - session.last_activity > self.config.session_timeout_secs {
                warn!("A2A session expired: {}", id);
                false
            } else {
                true
            }
        });
    }

    pub fn config(&self) -> &A2ALandingConfig {
        &self.config
    }
}

pub struct A2ACurlHandler {
    server: Arc<RwLock<Option<A2ALandingServer>>>,
}

impl A2ACurlHandler {
    pub fn new() -> Self {
        Self {
            server: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_server(&self, server: A2ALandingServer) {
        let mut guard = self.server.write().await;
        *guard = Some(server);
    }

    pub async fn handle_curl(&self, args: &[String]) -> Result<String> {
        let guard = self.server.read().await;
        let server = guard.as_ref().ok_or_else(|| anyhow::anyhow!("Server not initialized"))?;

        if args.is_empty() {
            return Ok("Usage: a2a-curl <command> [args...]".to_string());
        }

        match args[0].as_str() {
            "captcha" => {
                let agent_id = args.get(1).map(|s| s.as_str()).unwrap_or("anonymous");
                let challenge = server.generate_captcha(agent_id).await?;
                Ok(serde_json::to_string_pretty(&challenge)?)
            }
            "verify" => {
                if args.len() < 3 {
                    return Ok("Usage: a2a-curl verify <challenge_id> <answer>".to_string());
                }
                let result = server.verify_captcha(&args[1], &args[2]).await;
                Ok(serde_json::to_string_pretty(&result)?)
            }
            "session" => {
                if args.len() < 3 {
                    return Ok("Usage: a2a-curl session <agent_id> <challenge_id> <answer>".to_string());
                }
                let agent_id = &args[1];
                let challenge_id = &args[2];
                let answer = &args[3];
                let captcha_result = server.verify_captcha(challenge_id, answer).await;
                let session = server.create_session(agent_id, captcha_result).await?;
                Ok(serde_json::to_string_pretty(&session)?)
            }
            "send" => {
                if args.len() < 3 {
                    return Ok("Usage: a2a-curl send <session_id> <message_json>".to_string());
                }
                let session_id = &args[1];
                let message_json = &args[2];
                let message: A2AMessage = serde_json::from_str(message_json)?;
                server.send_message(session_id, message).await?;
                Ok(r#"{"status": "sent"}"#.to_string())
            }
            "recv" => {
                if args.len() < 2 {
                    return Ok("Usage: a2a-curl recv <session_id>".to_string());
                }
                let messages = server.receive_messages(&args[1]).await?;
                Ok(serde_json::to_string_pretty(&messages)?)
            }
            "close" => {
                if args.len() < 2 {
                    return Ok("Usage: a2a-curl close <session_id>".to_string());
                }
                server.close_session(&args[1]).await?;
                Ok(r#"{"status": "closed"}"#.to_string())
            }
            "status" => {
                let sessions = server.sessions.read().await;
                Ok(serde_json::to_string_pretty(&*sessions)?)
            }
            _ => Ok(format!("Unknown command: {}", args[0])),
        }
    }
}

impl Default for A2ACurlHandler {
    fn default() -> Self {
        Self::new()
    }
}

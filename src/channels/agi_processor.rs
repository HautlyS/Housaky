use crate::agent::loop_::run_tool_call_loop_with_agi;
use crate::config::Config;
use crate::housaky::agi_context::AGIContext;
use crate::housaky::goal_engine::{Goal, GoalCategory, GoalPriority, GoalStatus};
use crate::housaky::inner_monologue::{InnerMonologue, ThoughtSource, ThoughtType};
use crate::housaky::reasoning_pipeline::ReasoningPipeline;
use crate::memory::Memory;
use crate::observability::Observer;
use crate::providers::{ChatMessage, Provider};
use crate::tools::Tool;
use crate::util::truncate_with_ellipsis;
use anyhow::Result;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;

const MAX_CHANNEL_HISTORY_TURNS: usize = 40;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SessionMetadata {
    session_id: String,
    turn_count: u64,
    last_updated: String,
}

pub struct AGIChannelProcessor {
    workspace_dir: PathBuf,
    config: Config,
    provider: Arc<dyn Provider>,
    memory: Arc<dyn Memory>,
    tools_registry: Arc<Vec<Box<dyn Tool>>>,
    observer: Arc<dyn Observer>,
    system_prompt: String,
    model: String,
    temperature: f64,
    auto_save_memory: bool,
    max_tool_iterations: usize,
    message_timeout_secs: u64,
    conv_history: Arc<Mutex<HashMap<String, Vec<ChatMessage>>>>,
    state_cache: Arc<Mutex<HashMap<String, SessionMetadata>>>,
}

impl AGIChannelProcessor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_dir: PathBuf,
        config: Config,
        provider: Arc<dyn Provider>,
        memory: Arc<dyn Memory>,
        tools_registry: Arc<Vec<Box<dyn Tool>>>,
        observer: Arc<dyn Observer>,
        system_prompt: String,
        model: String,
        temperature: f64,
        max_tool_iterations: usize,
        message_timeout_secs: u64,
    ) -> Self {
        Self {
            workspace_dir,
            config: config.clone(),
            provider,
            memory,
            tools_registry,
            observer,
            system_prompt,
            model,
            temperature,
            auto_save_memory: config.memory.auto_save,
            max_tool_iterations,
            message_timeout_secs,
            conv_history: Arc::new(Mutex::new(HashMap::new())),
            state_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn get_session_dir(&self, conversation_id: &str) -> PathBuf {
        let sanitized = conversation_id.replace([':', '/', '\\'], "_");
        self.workspace_dir.join(".housaky").join("channels").join(&sanitized)
    }

    fn load_session_metadata(&self, conversation_id: &str) -> SessionMetadata {
        let session_dir = self.get_session_dir(conversation_id);
        let metadata_path = session_dir.join("session.json");
        
        if metadata_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&metadata_path) {
                if let Ok(metadata) = serde_json::from_str::<SessionMetadata>(&content) {
                    return metadata;
                }
            }
        }
        
        SessionMetadata {
            session_id: format!("channel_session_{}", uuid::Uuid::new_v4()),
            turn_count: 0,
            last_updated: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn save_session_metadata(&self, conversation_id: &str, metadata: &SessionMetadata) {
        let session_dir = self.get_session_dir(conversation_id);
        if let Err(e) = std::fs::create_dir_all(&session_dir) {
            tracing::warn!("Failed to create session dir: {}", e);
            return;
        }
        
        let metadata_path = session_dir.join("session.json");
        if let Ok(json) = serde_json::to_string_pretty(metadata) {
            if let Err(e) = std::fs::write(&metadata_path, json) {
                tracing::warn!("Failed to save session metadata: {}", e);
            }
        }
    }

    async fn get_or_create_agi_state(&self, conversation_id: &str) -> (InnerMonologue, ReasoningPipeline, crate::housaky::goal_engine::GoalEngine, u64, String) {
        let session_dir = self.get_session_dir(conversation_id);
        std::fs::create_dir_all(&session_dir).ok();
        
        let mut metadata = self.load_session_metadata(conversation_id);
        
        let inner_monologue = InnerMonologue::new(&session_dir);
        let reasoning = ReasoningPipeline::new();
        let goal_engine = crate::housaky::goal_engine::GoalEngine::new(&session_dir);
        
        let _ = inner_monologue.load().await;
        let _ = goal_engine.load_goals().await;
        
        metadata.turn_count += 1;
        metadata.last_updated = chrono::Utc::now().to_rfc3339();
        
        self.save_session_metadata(conversation_id, &metadata);
        
        (inner_monologue, reasoning, goal_engine, metadata.turn_count, metadata.session_id)
    }

    #[allow(clippy::format_push_string, clippy::single_char_add_str)]
    async fn build_agi_context(&self, inner_monologue: &InnerMonologue, goal_engine: &crate::housaky::goal_engine::GoalEngine) -> String {
        let mut context = String::new();

        let recent_thoughts = inner_monologue.get_recent(3).await;
        if !recent_thoughts.is_empty() {
            context.push_str("## Recent Thoughts\n");
            for thought in recent_thoughts {
                context.push_str(&format!("- {}\n", thought.chars().take(100).collect::<String>()));
            }
            context.push('\n');
        }

        let active_goals = goal_engine.get_active_goals().await;
        if !active_goals.is_empty() {
            context.push_str("## Active Goals\n");
            for goal in active_goals.iter().take(5) {
                context.push_str(&format!(
                    "- {} ({:.0}% complete) [{:?}]\n",
                    goal.title,
                    goal.progress * 100.0,
                    goal.priority
                ));
            }
            context.push('\n');
        }

        context
    }

    async fn send_status(status_tx: Option<&Sender<String>>, msg: &str) {
        if let Some(tx) = status_tx {
            let _ = tx.send(msg.to_string()).await;
        }
    }

    #[allow(clippy::single_char_add_str)]
    pub async fn process_with_agi(
        &self,
        msg: &crate::channels::traits::ChannelMessage,
    ) -> Result<String> {
        self.process_with_agi_with_status(msg, None).await
    }

    #[allow(clippy::single_char_add_str)]
    pub async fn process_with_agi_with_status(
        &self,
        msg: &crate::channels::traits::ChannelMessage,
        status_tx: Option<Sender<String>>,
    ) -> Result<String> {
        let conversation_id = format!("{}:{}", msg.channel, msg.sender);
        
        Self::send_status(status_tx.as_ref(), "📥 Loading session...").await;

        println!(
            "  🧠 [AGI {}] Processing from {}",
            msg.channel,
            msg.sender
        );

        let (inner_monologue, _reasoning, goal_engine, turn_count, _session_id) = 
            self.get_or_create_agi_state(&conversation_id).await;

        Self::send_status(status_tx.as_ref(), "💭 Recording thought...").await;

        inner_monologue.add_thought_with_type(
            &format!("User message: {}", msg.content.chars().take(200).collect::<String>()),
            ThoughtType::Observation,
            0.8,
            ThoughtSource::UserInteraction,
        ).await?;

        Self::send_status(status_tx.as_ref(), "🧠 Building AGI context...").await;

        let agi_context = self.build_agi_context(&inner_monologue, &goal_engine).await;

        Self::send_status(status_tx.as_ref(), "📚 Loading memory...").await;

        let memory_context = self.build_memory_context(&msg.content).await;

        if self.auto_save_memory {
            let autosave_key = format!("{}_{}_{}", msg.channel, msg.sender, msg.id);
            let _ = self
                .memory
                .store(
                    &autosave_key,
                    &msg.content,
                    crate::memory::MemoryCategory::Conversation,
                )
                .await;
        }

        Self::send_status(status_tx.as_ref(), "🤖 Generating response...").await;

        let mut enriched_message = String::new();
        if !agi_context.is_empty() {
            enriched_message.push_str("[AGI Context]\n");
            enriched_message.push_str(&agi_context);
            enriched_message.push_str("\n");
        }
        if !memory_context.is_empty() {
            enriched_message.push_str(&memory_context);
        }
        enriched_message.push_str(&msg.content);

        let history_key = conv_history_key(msg);
        let mut history: Vec<ChatMessage> = {
            let mut map = self.conv_history.lock();
            let entry = map
                .entry(history_key.clone())
                .or_insert_with(|| vec![ChatMessage::system(&self.system_prompt)]);
            entry.clone()
        };

        history.push(ChatMessage::user(&enriched_message));

        let started_at = Instant::now();

        let mut agi_ctx = AGIContext::disabled();
        if self.config.agi_enabled {
            agi_ctx = AGIContext::new(
                &self.workspace_dir,
                crate::housaky::agi_context::AGIConfig::default(),
            );
            // Best-effort hydrate of continuous learning model.
            if let Err(e) = agi_ctx.try_load_learning_model().await {
                tracing::warn!("Failed to load continuous learning model: {e}");
            }
        }

        let llm_result = tokio::time::timeout(
            Duration::from_secs(self.message_timeout_secs),
            run_tool_call_loop_with_agi(
                self.provider.as_ref(),
                &mut history,
                self.tools_registry.as_ref(),
                self.observer.as_ref(),
                "channel-runtime",
                &self.model,
                self.temperature,
                true,
                self.max_tool_iterations,
                &agi_ctx,
            ),
        )
        .await;

        match llm_result {
            Ok(Ok(response)) => {
                println!(
                    "  🤖 AGI Reply ({}ms): {}",
                    started_at.elapsed().as_millis(),
                    truncate_with_ellipsis(&response, 80)
                );

                inner_monologue.add_thought_with_type(
                    &format!("Assistant response: {}", response.chars().take(200).collect::<String>()),
                    ThoughtType::Decision,
                    0.9,
                    ThoughtSource::Internal,
                ).await?;

                if turn_count % 10 == 0 {
                    if let Some(reflection) = inner_monologue.reflect().await? {
                        println!("  🔄 Reflection: {}", reflection.content.chars().take(100).collect::<String>());
                    }
                }

                {
                    let mut map = self.conv_history.lock();
                    let stored = map
                        .entry(history_key)
                        .or_insert_with(|| vec![ChatMessage::system(&self.system_prompt)]);
                    *stored = history;
                    trim_channel_history(stored);
                }

                inner_monologue.save().await?;
                goal_engine.save_goals().await?;

                Ok(response)
            }
            Ok(Err(e)) => {
                eprintln!(
                    "  ❌ LLM error after {}ms: {e}",
                    started_at.elapsed().as_millis()
                );
                
                inner_monologue.add_thought_with_type(
                    &format!("Error: {}", e),
                    ThoughtType::SelfCorrection,
                    0.3,
                    ThoughtSource::Internal,
                ).await?;
                
                Err(e)
            }
            Err(_) => {
                let elapsed_ms = started_at.elapsed().as_millis();
                eprintln!(
                    "  ❌ LLM response timed out after {}s (elapsed: {}ms)",
                    self.message_timeout_secs,
                    elapsed_ms
                );
                
                inner_monologue.add_thought_with_type(
                    &format!("Timeout after {}s - consider increasing message_timeout_secs in config", self.message_timeout_secs),
                    ThoughtType::SelfCorrection,
                    0.3,
                    ThoughtSource::Internal,
                ).await.ok();
                
                Err(anyhow::anyhow!(
                    "LLM response timed out after {}s. Consider increasing 'message_timeout_secs' in your channels config (current: {}s).",
                    self.message_timeout_secs,
                    self.message_timeout_secs
                ))
            }
        }
    }

    async fn build_memory_context(&self, user_msg: &str) -> String {
        let mut context = String::new();

        if let Ok(entries) = self.memory.recall(user_msg, 5).await {
            if !entries.is_empty() {
                context.push_str("[Memory context]\n");
                for entry in &entries {
                    let _ = context.write_str(&format!("- {}: {}\n", entry.key, entry.content));
                }
                context.push('\n');
            }
        }

        context
    }

    pub async fn handle_agi_command(
        &self,
        msg: &crate::channels::traits::ChannelMessage,
    ) -> Option<String> {
        let content = msg.content.trim();
        let conversation_id = format!("{}:{}", msg.channel, msg.sender);
        
        // Help command
        if content == "/help" || content == "/h" || content == "/?" {
            return Some(self.handle_help_command().await);
        }
        
        if content.starts_with("/goals") || content.starts_with("/goal") {
            return Some(self.handle_goals_command(&conversation_id).await);
        }
        
        if content.starts_with("/thoughts") || content.starts_with("/thought") {
            return Some(self.handle_thoughts_command(&conversation_id).await);
        }
        
        if content.starts_with("/reasoning") || content.starts_with("/reason") {
            return Some(self.handle_reasoning_command(&conversation_id).await);
        }
        
        if content.starts_with("/status") || content.starts_with("/stats") {
            return Some(self.handle_status_command(&conversation_id).await);
        }
        
        if content.starts_with("/create_goal ") || content.starts_with("/cg ") {
            let goal_text = content
                .trim_start_matches("/create_goal ")
                .trim_start_matches("/cg ")
                .trim();
            return Some(self.handle_create_goal(&conversation_id, goal_text).await);
        }
        
        // Check if it's just a slash command we don't handle
        if content.starts_with('/') {
            return Some(format!(
                "Unknown command: {}\n\nUse /help to see available commands.",
                content.split_whitespace().next().unwrap_or("")
            ));
        }
        
        None
    }

    #[allow(clippy::format_push_string, clippy::single_char_add_str)]
    async fn handle_help_command(&self) -> String {
        let mut response = String::from("🧠 **AGI Commands**\n\n");
        response.push_str("**Core Commands:**\n");
        response.push_str("/help, /h, /? - Show this help message\n");
        response.push_str("/status, /stats - Show AGI status\n\n");
        
        response.push_str("**Goals:**\n");
        response.push_str("/goals, /goal - View active goals\n");
        response.push_str("/create_goal <title>, /cg <title> - Create a goal\n\n");
        
        response.push_str("**Thoughts:**\n");
        response.push_str("/thoughts, /thought - View recent thoughts\n");
        response.push_str("/reasoning, /reason - View reasoning patterns\n\n");
        
        response.push_str("**Tips:**\n");
        response.push_str("- Goals are persistent across sessions\n");
        response.push_str("- Thoughts track conversation context\n");
        response.push_str("- Use 'urgent' in goal title for high priority\n");
        
        response
    }

    #[allow(clippy::format_push_string)]
    async fn handle_goals_command(&self, conversation_id: &str) -> String {
        let (_, _, goal_engine, turn_count, _) = self.get_or_create_agi_state(conversation_id).await;
        
        let active_goals = goal_engine.get_active_goals().await;
        let stats = goal_engine.get_goal_stats().await;
        
        let mut response = String::from("🎯 **Goals**\n\n");
        
        response.push_str(&format!("Total turns in session: {}\n\n", turn_count));
        
        if active_goals.is_empty() {
            response.push_str("No active goals. Use /create_goal <title> to create one.\n");
        } else {
            response.push_str(&format!("**Active Goals ({}):**\n\n", active_goals.len()));
            for goal in active_goals.iter().take(10) {
                let progress_bar = create_progress_bar(goal.progress);
                response.push_str(&format!(
                    "• **{}**\n  {} [{:.0}%]\n  Priority: {:?}\n\n",
                    goal.title,
                    progress_bar,
                    goal.progress * 100.0,
                    goal.priority
                ));
            }
        }
        
        response.push_str(&format!(
            "**Stats:** {} pending, {} in progress, {} completed, {} failed",
            stats.pending, stats.in_progress, stats.completed, stats.failed
        ));
        
        response
    }

    #[allow(clippy::format_push_string)]
    async fn handle_thoughts_command(&self, conversation_id: &str) -> String {
        let (inner_monologue, _, _, _turn_count, _) = self.get_or_create_agi_state(conversation_id).await;
        
        let recent = inner_monologue.get_recent_thoughts(10).await;
        let stats = inner_monologue.get_stats().await;
        
        let mut response = String::from("💭 **Thoughts**\n\n");
        
        response.push_str(&format!(
            "Total thoughts: {} | Current: {} | Unprocessed: {}\n\n",
            stats.total_count, stats.current_count, stats.unprocessed_count
        ));
        
        if recent.is_empty() {
            response.push_str("No thoughts recorded yet.\n");
        } else {
            response.push_str("**Recent thoughts:**\n\n");
            for thought in recent {
                let icon = match thought.thought_type {
                    ThoughtType::Observation => "👁️",
                    ThoughtType::Inference => "🧩",
                    ThoughtType::Decision => "✅",
                    ThoughtType::Reflection => "🔄",
                    ThoughtType::Planning => "📋",
                    ThoughtType::Goal => "🎯",
                    ThoughtType::Learning => "📚",
                    ThoughtType::SelfCorrection => "🔧",
                    _ => "💭",
                };
                response.push_str(&format!(
                    "{} **[{:?}]** {:.0}% confidence\n  {}\n\n",
                    icon,
                    thought.thought_type,
                    thought.confidence * 100.0,
                    thought.content.chars().take(120).collect::<String>()
                ));
            }
        }
        
        response
    }

    #[allow(clippy::format_push_string)]
    async fn handle_reasoning_command(&self, conversation_id: &str) -> String {
        let (_, _, _, turn_count, session_id) = self.get_or_create_agi_state(conversation_id).await;
        
        let mut response = String::from("🧠 **Reasoning**\n\n");
        
        response.push_str(&format!("Session: {}\n", session_id));
        response.push_str(&format!("Turns: {}\n\n", turn_count));
        
        let (inner_monologue, _, _, _, _) = self.get_or_create_agi_state(conversation_id).await;
        let recent_thoughts = inner_monologue.get_recent_thoughts(5).await;
        let decisions: Vec<_> = recent_thoughts
            .into_iter()
            .filter(|t| t.thought_type == ThoughtType::Decision || t.thought_type == ThoughtType::Inference)
            .collect();
        
        if decisions.is_empty() {
            response.push_str("No reasoning patterns recorded yet.\n");
        } else {
            response.push_str("**Recent decisions:**\n\n");
            for thought in decisions {
                response.push_str(&format!(
                    "• {:.0}%: {}\n",
                    thought.confidence * 100.0,
                    thought.content.chars().take(100).collect::<String>()
                ));
            }
        }
        
        response
    }

    #[allow(clippy::format_push_string)]
    async fn handle_status_command(&self, conversation_id: &str) -> String {
        let (_, _, goal_engine, turn_count, session_id) = self.get_or_create_agi_state(conversation_id).await;
        let goal_stats = goal_engine.get_goal_stats().await;
        
        let (inner_monologue, _, _, _, _) = self.get_or_create_agi_state(conversation_id).await;
        let thought_stats = inner_monologue.get_stats().await;
        
        let mut response = String::from("📊 **AGI Status**\n\n");
        
        response.push_str(&format!("Session: {}\n", session_id));
        response.push_str(&format!("Turns: {}\n\n", turn_count));
        
        response.push_str("**Goals:**\n");
        response.push_str(&format!(
            "  Pending: {} | In Progress: {} | Completed: {} | Failed: {}\n\n",
            goal_stats.pending, goal_stats.in_progress, goal_stats.completed, goal_stats.failed
        ));
        
        response.push_str("**Thoughts:**\n");
        response.push_str(&format!(
            "  Total: {} | Current: {} | Avg Confidence: {:.0}%\n",
            thought_stats.total_count, thought_stats.current_count, thought_stats.avg_confidence * 100.0
        ));
        
        response
    }

    async fn handle_create_goal(&self, conversation_id: &str, goal_text: &str) -> String {
        if goal_text.is_empty() {
            return "Usage: /create_goal <goal title>".to_string();
        }
        
        let (inner_monologue, _, goal_engine, _, _) = self.get_or_create_agi_state(conversation_id).await;
        
        let priority = if goal_text.to_lowercase().contains("urgent") || goal_text.to_lowercase().contains("important") {
            GoalPriority::High
        } else {
            GoalPriority::Medium
        };
        
        let goal = Goal {
            id: String::new(),
            title: goal_text.to_string(),
            description: format!("Goal from channel: {}", goal_text),
            priority,
            status: GoalStatus::Pending,
            category: GoalCategory::UserRequest,
            progress: 0.0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deadline: None,
            parent_id: None,
            subtask_ids: Vec::new(),
            dependencies: Vec::new(),
            blockers: Vec::new(),
            metrics: HashMap::new(),
            checkpoints: Vec::new(),
            attempts: 0,
            max_attempts: 3,
            estimated_complexity: 3.0,
            actual_complexity: None,
            learning_value: 1.0,
            tags: vec!["channel".to_string(), "user_created".to_string()],
            context: HashMap::new(),
            temporal_constraints: Vec::new(),
        };
        
        match goal_engine.add_goal(goal).await {
            Ok(goal_id) => {
                inner_monologue.add_thought_with_type(
                    &format!("Created goal: {}", goal_text),
                    ThoughtType::Goal,
                    0.9,
                    ThoughtSource::UserInteraction,
                ).await.ok();
                
                goal_engine.save_goals().await.ok();
                inner_monologue.save().await.ok();
                
                format!("✅ Goal created: **{}**\nID: {}", goal_text, goal_id)
            }
            Err(e) => format!("❌ Failed to create goal: {}", e),
        }
    }
}

fn conv_history_key(msg: &crate::channels::traits::ChannelMessage) -> String {
    format!("{}:{}", msg.channel, msg.sender)
}

fn trim_channel_history(history: &mut Vec<ChatMessage>) {
    let has_system = history.first().map_or(false, |m| m.role == "system");
    let non_system = if has_system {
        history.len().saturating_sub(1)
    } else {
        history.len()
    };

    if non_system <= MAX_CHANNEL_HISTORY_TURNS {
        return;
    }

    let start = if has_system { 1 } else { 0 };
    let to_drop = non_system - MAX_CHANNEL_HISTORY_TURNS;
    history.drain(start..start + to_drop);
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn create_progress_bar(progress: f64) -> String {
    let total = 10;
    let filled = (progress * total as f64) as usize;
    let empty = total - filled;
    
    let filled_str = "█".repeat(filled);
    let empty_str = "░".repeat(empty);
    
    format!("[{}{}]", filled_str, empty_str)
}

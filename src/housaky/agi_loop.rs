use crate::agent::loop_::{build_tool_instructions, find_tool};
use crate::config::Config;
use crate::housaky::core::{AGIAction, HousakyCore};
use crate::housaky::goal_engine::{Goal, GoalPriority, GoalStatus};
use crate::memory::{Memory, MemoryCategory};
use crate::observability::{Observer, ObserverEvent};
use crate::providers::{ChatMessage, Provider};
use crate::security::SecurityPolicy;
use crate::tools::Tool;
use crate::util::truncate_with_ellipsis;
use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

pub struct AGIAgentLoop {
    core: Arc<HousakyCore>,
    config: Config,
    history: Vec<ChatMessage>,
    max_history: usize,
    system_prompt: String,
}

impl AGIAgentLoop {
    pub fn new(core: Arc<HousakyCore>, config: Config, system_prompt: String) -> Self {
        Self {
            core,
            config,
            history: Vec::new(),
            max_history: 50,
            system_prompt,
        }
    }

    pub async fn process_message(
        &mut self,
        provider: &dyn Provider,
        model: &str,
        tools_registry: &[Box<dyn Tool>],
        observer: &dyn Observer,
        user_message: &str,
        silent: bool,
        temperature: f64,
    ) -> Result<String> {
        let start = Instant::now();

        observer.record_event(&ObserverEvent::AgentStart {
            provider: "housaky".to_string(),
            model: model.to_string(),
        });

        let _context = self.core.prepare_context(user_message).await?;

        let decision = self
            .core
            .process_with_reasoning(
                provider,
                model,
                user_message,
                &tools_registry
                    .iter()
                    .map(|t| t.as_ref())
                    .collect::<Vec<_>>(),
            )
            .await?;

        let response = match decision.action {
            AGIAction::UseTool {
                ref name,
                ref arguments,
                ref goal_id,
            } => {
                self.execute_tool_action(
                    provider,
                    model,
                    tools_registry,
                    observer,
                    user_message,
                    name,
                    arguments.clone(),
                    goal_id.as_deref(),
                    silent,
                    temperature,
                )
                .await?
            }
            AGIAction::Respond {
                ref content,
                needs_clarification,
            } => {
                if needs_clarification {
                    format!("{} [Note: I'm not fully confident about this.]", content)
                } else {
                    content.clone()
                }
            }
            AGIAction::CreateGoal {
                ref title,
                ref description,
                ref priority,
            } => {
                self.create_goal_action(title, description, priority.clone())
                    .await
            }
            AGIAction::Reflect { ref trigger } => {
                self.core.reflect_on_turn(trigger).await?;
                "I've taken a moment to reflect on our conversation.".to_string()
            }
            AGIAction::Learn {
                ref topic,
                ref source,
            } => {
                self.core.store_knowledge(topic, source).await?;
                format!("I've learned about: {}", topic)
            }
            AGIAction::Wait { ref reason } => {
                format!("I'm pausing because: {}", reason)
            }
        };

        self.core
            .record_action_result(
                true,
                &response,
                if let AGIAction::UseTool { ref goal_id, .. } = decision.action {
                    goal_id.as_deref()
                } else {
                    None
                },
            )
            .await?;

        observer.record_event(&ObserverEvent::AgentEnd {
            duration: start.elapsed(),
            tokens_used: None,
        });

        if !silent {
            println!("{}", response);
        }

        Ok(response)
    }

    async fn execute_tool_action(
        &mut self,
        provider: &dyn Provider,
        model: &str,
        tools_registry: &[Box<dyn Tool>],
        observer: &dyn Observer,
        user_message: &str,
        tool_name: &str,
        arguments: serde_json::Value,
        goal_id: Option<&str>,
        silent: bool,
        temperature: f64,
    ) -> Result<String> {
        if !silent {
            println!("[Using tool: {}]", tool_name);
        }

        observer.record_event(&ObserverEvent::ToolCallStart {
            tool: tool_name.to_string(),
        });

        let tool_start = Instant::now();

        let result = if let Some(tool) = find_tool(tools_registry, tool_name) {
            match tool.execute(arguments.clone()).await {
                Ok(r) => {
                    observer.record_event(&ObserverEvent::ToolCall {
                        tool: tool_name.to_string(),
                        duration: tool_start.elapsed(),
                        success: r.success,
                    });

                    if r.success {
                        r.output
                    } else {
                        format!("Error: {}", r.error.unwrap_or_else(|| r.output))
                    }
                }
                Err(e) => {
                    observer.record_event(&ObserverEvent::ToolCall {
                        tool: tool_name.to_string(),
                        duration: tool_start.elapsed(),
                        success: false,
                    });
                    format!("Error executing {}: {}", tool_name, e)
                }
            }
        } else {
            format!("Unknown tool: {}", tool_name)
        };

        let follow_up_prompt = format!(
            "I used the {} tool and got this result:\n{}\n\nBased on this, what should I tell the user about: {}",
            tool_name,
            result,
            user_message
        );

        let final_response = provider
            .chat_with_system(
                Some(&self.system_prompt),
                &follow_up_prompt,
                model,
                temperature,
            )
            .await?;

        if let Some(gid) = goal_id {
            self.core
                .record_action_result(true, &result, Some(gid))
                .await?;
        }

        Ok(final_response)
    }

    async fn create_goal_action(
        &self,
        title: &str,
        description: &str,
        priority: GoalPriority,
    ) -> String {
        let goal = Goal {
            title: title.to_string(),
            description: description.to_string(),
            priority,
            status: GoalStatus::Pending,
            ..Default::default()
        };

        match self.core.goal_engine.add_goal(goal).await {
            Ok(id) => format!("I've created a new goal: {} (ID: {})", title, id),
            Err(e) => format!("Failed to create goal: {}", e),
        }
    }

    pub async fn run_interactive(
        &mut self,
        provider: Box<dyn Provider>,
        model: &str,
        tools_registry: Vec<Box<dyn Tool>>,
        observer: Arc<dyn Observer>,
    ) -> Result<()> {
        println!("🧠 Housaky AGI Interactive Mode");
        println!(
            "Type /quit to exit, /goals to see active goals, /reflect to trigger reflection.\n"
        );

        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        let cli = crate::channels::CliChannel::new();

        let listen_handle = tokio::spawn(async move {
            let _ = crate::channels::Channel::listen(&cli, tx).await;
        });

        self.history.push(ChatMessage::system(&self.system_prompt));

        while let Some(msg) = rx.recv().await {
            let content = msg.content.trim();

            if content == "/quit" || content == "/exit" {
                println!("Goodbye!");
                break;
            }

            if content == "/goals" {
                self.show_goals().await;
                continue;
            }

            if content == "/reflect" {
                self.core
                    .reflect_on_turn("User requested reflection")
                    .await?;
                println!("Reflection complete.\n");
                continue;
            }

            if content == "/metrics" {
                self.show_metrics().await;
                continue;
            }

            if content == "/thoughts" {
                self.show_thoughts().await;
                continue;
            }

            let response = self
                .process_message(
                    provider.as_ref(),
                    model,
                    &tools_registry,
                    observer.as_ref(),
                    content,
                    false,
                    0.7,
                )
                .await?;

            self.history.push(ChatMessage::user(content));
            self.history.push(ChatMessage::assistant(&response));

            if self.history.len() > self.max_history + 1 {
                let remove_count = self.history.len() - self.max_history;
                self.history.drain(1..=remove_count);
            }
        }

        listen_handle.abort();
        self.core.shutdown().await?;

        Ok(())
    }

    async fn show_goals(&self) {
        let stats = self.core.goal_engine.get_goal_stats().await;
        println!("\n📋 Goals Overview");
        println!("  Total:      {}", stats.total);
        println!("  Pending:    {}", stats.pending);
        println!("  In Progress: {}", stats.in_progress);
        println!("  Completed:  {}", stats.completed);
        println!("  Failed:     {}", stats.failed);

        let active = self.core.goal_engine.get_active_goals().await;
        if !active.is_empty() {
            println!("\nActive Goals:");
            for goal in active {
                println!(
                    "  - {} [{}] {}%",
                    goal.title,
                    format!("{:?}", goal.priority),
                    (goal.progress * 100.0) as i32
                );
            }
        }
        println!();
    }

    async fn show_metrics(&self) {
        let metrics = self.core.get_dashboard_metrics().await;
        println!("\n📊 AGI Metrics");
        println!("  Total Turns:       {}", metrics.total_turns);
        println!("  Successful Actions: {}", metrics.successful_actions);
        println!("  Failed Actions:     {}", metrics.failed_actions);
        println!("  Success Rate:       {:.1}%", metrics.success_rate * 100.0);
        println!(
            "  Confidence Level:   {:.1}%",
            metrics.confidence_level * 100.0
        );
        println!("  Evolution Stage:    {}", metrics.evolution_stage);
        println!("  Memory Items:       {}", metrics.memory_items);
        println!("  Knowledge Entities: {}", metrics.knowledge_entities);
        println!("  Uptime:             {}s", metrics.uptime_seconds);

        println!("\nCapabilities:");
        println!(
            "  Reasoning:      {:.0}%",
            metrics.capabilities.reasoning * 100.0
        );
        println!(
            "  Learning:       {:.0}%",
            metrics.capabilities.learning * 100.0
        );
        println!(
            "  Self-Awareness: {:.0}%",
            metrics.capabilities.self_awareness * 100.0
        );
        println!(
            "  Meta-Cognition: {:.0}%",
            metrics.capabilities.meta_cognition * 100.0
        );
        println!();
    }

    async fn show_thoughts(&self) {
        let thoughts = self.core.inner_monologue.get_recent(5).await;
        println!("\n💭 Recent Thoughts");
        for (i, thought) in thoughts.iter().enumerate() {
            println!(
                "  {}. {}",
                i + 1,
                thought.chars().take(100).collect::<String>()
            );
        }
        println!();
    }
}

pub async fn run_agi_loop(
    config: Config,
    message: Option<String>,
    provider_override: Option<String>,
    model_override: Option<String>,
    temperature: f64,
) -> Result<()> {
    let core = Arc::new(HousakyCore::new(&config)?);
    core.initialize().await?;

    let provider_name = provider_override
        .as_deref()
        .or(config.default_provider.as_deref())
        .unwrap_or("openrouter");

    let model_name = model_override
        .as_deref()
        .or(config.default_model.as_deref())
        .unwrap_or("anthropic/claude-sonnet-4");

    let provider: Box<dyn Provider> = crate::providers::create_routed_provider(
        provider_name,
        config.api_key.as_deref(),
        &config.reliability,
        &config.model_routes,
        model_name,
    )?;

    let observer: Arc<dyn Observer> =
        Arc::from(crate::observability::create_observer(&config.observability));

    let runtime: Arc<dyn crate::runtime::RuntimeAdapter> =
        Arc::from(crate::runtime::create_runtime(&config.runtime)?);
    let security = Arc::new(SecurityPolicy::from_config(
        &config.autonomy,
        &config.workspace_dir,
    ));

    let mem: Arc<dyn Memory> = Arc::from(crate::memory::create_memory(
        &config.memory,
        &config.workspace_dir,
        config.api_key.as_deref(),
    )?);

    let (composio_key, composio_entity_id) = if config.composio.enabled {
        (
            config.composio.api_key.as_deref(),
            Some(config.composio.entity_id.as_str()),
        )
    } else {
        (None, None)
    };

    let mut tools_registry = crate::tools::all_tools_with_runtime(
        &security,
        runtime,
        mem.clone(),
        composio_key,
        composio_entity_id,
        &config.browser,
        &config.http_request,
        &config.workspace_dir,
        &config.agents,
        config.api_key.as_deref(),
        &config,
    );

    let peripheral_tools = crate::peripherals::create_peripheral_tools(&config.peripherals).await?;
    tools_registry.extend(peripheral_tools);

    let skills = crate::skills::load_skills(&config.workspace_dir);
    let tool_descs: Vec<(&str, &str)> = tools_registry
        .iter()
        .map(|t| (t.name(), t.description()))
        .collect();

    let mut system_prompt = crate::channels::build_system_prompt(
        &config.workspace_dir,
        model_name,
        &tool_descs,
        &skills,
        Some(&config.identity),
        Some(6000),
    );
    system_prompt.push_str(&build_tool_instructions(&tools_registry));

    let mut agi_loop = AGIAgentLoop::new(core, config.clone(), system_prompt);

    if let Some(msg) = message {
        tracing::info!("AGI loop: about to call process_message");
        let response = agi_loop
            .process_message(
                provider.as_ref(),
                model_name,
                &tools_registry,
                observer.as_ref(),
                &msg,
                false,
                temperature,
            )
            .await?;

        tracing::info!("AGI loop: got response, about to print");

        println!("\n{}", response);

        if config.memory.auto_save {
            let key = format!("user_msg_{}", Uuid::new_v4());
            let _ = mem.store(&key, &msg, MemoryCategory::Conversation).await;
            let resp_key = format!("assistant_resp_{}", Uuid::new_v4());
            let summary = truncate_with_ellipsis(&response, 100);
            let _ = mem.store(&resp_key, &summary, MemoryCategory::Daily).await;
        }
    } else {
        agi_loop
            .run_interactive(provider, model_name, tools_registry, observer)
            .await?;
    }

    Ok(())
}

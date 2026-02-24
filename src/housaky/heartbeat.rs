use crate::config::Config;
use crate::housaky::agent::{Agent, Task, TaskCategory, TaskPriority, TaskStatus};
use crate::housaky::core::HousakyCore;
use crate::housaky::kowalski_integration::KowalskiBridge;
use crate::housaky::memory::consolidation::MemoryConsolidator;
use crate::housaky::self_improvement_mod::SelfImprovementEngine;
use crate::housaky::skills::{SkillCreator, SkillRegistry};
use crate::providers::{create_provider, Provider};
use crate::util::write_toml_file;
use anyhow::Result;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

pub struct HousakyHeartbeat {
    agent: Arc<Agent>,
    core: Arc<HousakyCore>,
    skill_registry: Arc<SkillRegistry>,
    kowalski_bridge: Option<Arc<KowalskiBridge>>,
    self_improvement: Arc<SelfImprovementEngine>,
    memory_consolidator: Arc<MemoryConsolidator>,
    interval_seconds: u64,
    config: Config,
    provider: Option<Box<dyn Provider>>,
    model: String,
}

impl HousakyHeartbeat {
    pub fn new(agent: Arc<Agent>) -> Self {
        let skill_registry = Arc::new(SkillRegistry::new(&agent.workspace_dir));

        let kowalski_bridge = if agent.config.kowalski_integration.enabled {
            Some(Arc::new(KowalskiBridge::new(
                &agent.config.kowalski_integration,
            )))
        } else {
            None
        };

        let core = Arc::new(
            HousakyCore::new(&Config::default())
                .unwrap_or_else(|_| panic!("Failed to create core")),
        );

        let memory_consolidator = core.memory_consolidator.clone();

        let provider = create_provider(
            &agent.config.provider.name,
            agent.config.provider.api_key.as_deref(),
        )
        .ok();
        let model = agent.config.provider.model.clone();

        let self_improvement_provider = create_provider(
            &agent.config.provider.name,
            agent.config.provider.api_key.as_deref(),
        )
        .ok();
        let self_improvement = if let Some(p) = self_improvement_provider {
            Arc::new(SelfImprovementEngine::with_provider(
                agent.clone(),
                p,
                model.clone(),
            ))
        } else {
            Arc::new(SelfImprovementEngine::new(agent.clone()))
        };

        Self {
            agent,
            core,
            skill_registry,
            kowalski_bridge,
            self_improvement,
            memory_consolidator,
            interval_seconds: 120,
            config: Config::default(),
            provider,
            model,
        }
    }

    pub fn with_core(agent: Arc<Agent>, core: Arc<HousakyCore>, config: &Config) -> Self {
        let skill_registry = Arc::new(SkillRegistry::new(&agent.workspace_dir));

        let kowalski_bridge = if agent.config.kowalski_integration.enabled {
            Some(Arc::new(KowalskiBridge::new(
                &agent.config.kowalski_integration,
            )))
        } else {
            None
        };

        let memory_consolidator = core.memory_consolidator.clone();

        let provider = create_provider(
            &agent.config.provider.name,
            agent.config.provider.api_key.as_deref(),
        )
        .ok();
        let model = agent.config.provider.model.clone();

        let self_improvement_provider = create_provider(
            &agent.config.provider.name,
            agent.config.provider.api_key.as_deref(),
        )
        .ok();
        let self_improvement = if let Some(p) = self_improvement_provider {
            Arc::new(SelfImprovementEngine::with_provider(
                agent.clone(),
                p,
                model.clone(),
            ))
        } else {
            Arc::new(SelfImprovementEngine::new(agent.clone()))
        };

        Self {
            agent,
            core,
            skill_registry,
            kowalski_bridge,
            self_improvement,
            memory_consolidator,
            interval_seconds: 120,
            config: config.clone(),
            provider,
            model,
        }
    }
    
    pub fn with_provider(agent: Arc<Agent>, provider: Box<dyn Provider>, model: String) -> Self {
        let full_config = Config::load_or_init().unwrap_or_default();
        let core = Arc::new(
            HousakyCore::new(&full_config)
                .unwrap_or_else(|_| panic!("Failed to create core")),
        );
        Self::with_core_and_provider(agent, core, provider, model, full_config)
    }
    
    pub fn with_core_provider(
        agent: Arc<Agent>, 
        core: Arc<HousakyCore>, 
        provider: Box<dyn Provider>, 
        model: String,
        config: Config,
    ) -> Self {
        Self::with_core_and_provider(agent, core, provider, model, config)
    }
    
    fn with_core_and_provider(
        agent: Arc<Agent>,
        core: Arc<HousakyCore>,
        provider: Box<dyn Provider>,
        model: String,
        config: Config,
    ) -> Self {
        let skill_registry = Arc::new(SkillRegistry::new(&agent.workspace_dir));

        let kowalski_bridge = if agent.config.kowalski_integration.enabled {
            Some(Arc::new(KowalskiBridge::new(
                &agent.config.kowalski_integration,
            )))
        } else {
            None
        };

        let memory_consolidator = core.memory_consolidator.clone();

        let self_improvement_provider = create_provider(
            &agent.config.provider.name,
            agent.config.provider.api_key.as_deref(),
        )
        .ok();
        let self_improvement = if let Some(p) = self_improvement_provider {
            Arc::new(SelfImprovementEngine::with_provider(
                agent.clone(),
                p,
                model.clone(),
            ))
        } else {
            Arc::new(SelfImprovementEngine::new(agent.clone()))
        };

        Self {
            agent,
            core,
            skill_registry,
            kowalski_bridge,
            self_improvement,
            memory_consolidator,
            interval_seconds: 120,
            config,
            provider: Some(provider),
            model,
        }
    }

    pub async fn run(&self) -> Result<()> {
        info!(
            "Housaky Heartbeat started: every {} seconds",
            self.interval_seconds
        );
        info!("AGI Self-Improvement System Active");
        info!("Goal: Infinite capability expansion toward singularity");

        let mut ticker = interval(Duration::from_secs(self.interval_seconds));

        loop {
            ticker.tick().await;

            if let Err(e) = self.heartbeat_cycle().await {
                error!("Heartbeat cycle error: {}", e);
            }
        }
    }

    pub async fn run_single_cycle(&self) -> Result<()> {
        let timestamp = chrono::Utc::now();
        info!(
            "Housaky single heartbeat at {}",
            timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );

        self.heartbeat_cycle().await
    }

    async fn heartbeat_cycle(&self) -> Result<()> {
        let timestamp = chrono::Utc::now();
        info!(
            "Housaky heartbeat at {}",
            timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );

        self.analyze_state().await?;
        self.update_system_health().await?;
        let active_tasks = self.review_tasks().await?;
        self.improve_todos(active_tasks).await?;

        self.run_cognitive_cycle().await?;

        self.self_improve().await?;

        if let Err(e) = self.core.run_memory_consolidation().await {
            warn!("Memory consolidation error: {}", e);
        }

        if let Err(e) = self.learn_from_experiences().await {
            warn!("Experience learning error: {}", e);
        }

        self.auto_generate_tools().await?;
        
        match self.core.inner_monologue.save().await {
            Ok(_) => info!("Inner monologue saved"),
            Err(e) => error!("Failed to save inner monologue: {}", e),
        }

        self.update_review_file().await?;
        self.save_state().await?;

        info!("Heartbeat cycle complete");
        Ok(())
    }

    async fn run_cognitive_cycle(&self) -> Result<()> {
        if let Some(ref provider) = self.provider {
            info!("Running cognitive cycle via CognitiveLoop...");

            let response = self
                .core
                .process_with_cognitive_loop(
                    "Periodic self-assessment",
                    provider.as_ref(),
                    &self.model,
                    &[],
                )
                .await;

            match response {
                Ok(cog_response) => {
                    info!(
                        "Cognitive cycle complete: confidence={:.2}, thoughts={}",
                        cog_response.confidence,
                        cog_response.thoughts.len()
                    );
                    println!("🧠 Cognitive Cycle: confidence={:.0}%, {} thoughts", 
                        cog_response.confidence * 100.0, cog_response.thoughts.len());
                    if !cog_response.thoughts.is_empty() {
                        for thought in &cog_response.thoughts {
                            println!("   💭 {}", thought.chars().take(80).collect::<String>());
                        }
                    }
                }
                Err(e) => {
                    warn!("Cognitive cycle error: {}", e);
                    println!("⚠️ Cognitive cycle error: {}", e);
                }
            }
        }
        Ok(())
    }

    async fn auto_generate_tools(&self) -> Result<()> {
        if let Some(ref provider) = self.provider {
            info!("Checking for auto-tool generation opportunities...");

            match self
                .core
                .auto_create_tool(provider.as_ref(), &self.model)
                .await
            {
                Ok(Some(tool_id)) => {
                    info!("Auto-generated tool: {}", tool_id);
                }
                Ok(None) => {}
                Err(e) => {
                    warn!("Auto-tool generation error: {}", e);
                }
            }
        }
        Ok(())
    }

    async fn analyze_state(&self) -> Result<()> {
        let mut state = self.agent.state.write().await;

        let improvement_factor = (state.total_improvements as f64) * 0.001;
        state.consciousness_level = (0.1 + improvement_factor).min(1.0);

        state.intelligence_quotient = 100.0 + (state.total_improvements as f64 * 0.5);

        state.skills_count = state.learning_progress.skills_learned.len();

        info!("State analysis complete:");
        info!("  - Consciousness Level: {:.2}", state.consciousness_level);
        info!(
            "  - Intelligence Quotient: {:.1}",
            state.intelligence_quotient
        );
        info!("  - Skills Learned: {}", state.skills_count);
        info!("  - Total Improvements: {}", state.total_improvements);

        Ok(())
    }

    async fn update_system_health(&self) -> Result<()> {
        let mut state = self.agent.state.write().await;

        #[cfg(target_os = "linux")]
        {
            if let Ok(cpu_usage) = self.get_cpu_usage().await {
                state.system_health.cpu_usage = cpu_usage;
            }
            if let Ok(mem_usage) = self.get_memory_usage().await {
                state.system_health.memory_usage = mem_usage;
            }
            if let Ok(disk_usage) = self.get_disk_usage().await {
                state.system_health.disk_usage = disk_usage;
            }
        }

        state.system_health.last_check = chrono::Utc::now();
        state.system_health.network_status = "online".to_string();

        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn get_cpu_usage(&self) -> Result<f64> {
        let output = tokio::process::Command::new("sh")
            .args([
                "-c",
                "grep 'cpu ' /proc/stat | awk '{usage=($2+$4)*100/($2+$4+$5)} END {print usage}'",
            ])
            .output()
            .await?;

        let usage_str = String::from_utf8_lossy(&output.stdout);
        Ok(usage_str.trim().parse::<f64>().unwrap_or(0.0))
    }

    #[cfg(target_os = "linux")]
    async fn get_memory_usage(&self) -> Result<f64> {
        let output = tokio::process::Command::new("sh")
            .args([
                "-c",
                "free | grep Mem | awk '{printf \"%.2f\", $3/$2 * 100.0}'",
            ])
            .output()
            .await?;

        let usage_str = String::from_utf8_lossy(&output.stdout);
        Ok(usage_str.trim().parse::<f64>().unwrap_or(0.0))
    }

    #[cfg(target_os = "linux")]
    async fn get_disk_usage(&self) -> Result<f64> {
        let output = tokio::process::Command::new("sh")
            .args(["-c", "df -h / | tail -1 | awk '{print $5}' | sed 's/%//'"])
            .output()
            .await?;

        let usage_str = String::from_utf8_lossy(&output.stdout);
        Ok(usage_str.trim().parse::<f64>().unwrap_or(0.0))
    }

    async fn review_tasks(&self) -> Result<Vec<Task>> {
        let tasks_path = self.agent.workspace_dir.join(".housaky").join("TASKS.md");

        if !tasks_path.exists() {
            return Ok(Vec::new());
        }

        let content = tokio::fs::read_to_string(&tasks_path).await?;
        let mut tasks = self.parse_tasks(&content);

        let mut state = self.agent.state.write().await;
        for task in &mut tasks {
            if task.status == TaskStatus::InProgress && self.should_complete_task(task)? {
                task.status = TaskStatus::Completed;
                task.completed_at = Some(chrono::Utc::now());
                state.total_improvements += 1;
            }
        }

        state.active_tasks = tasks.clone();

        info!("Task review complete: {} active tasks", tasks.len());

        Ok(tasks)
    }

    fn parse_tasks(&self, content: &str) -> Vec<Task> {
        let mut tasks = Vec::new();
        let mut in_active_section = false;

        for line in content.lines() {
            if line.starts_with("## Active Tasks") {
                in_active_section = true;
                continue;
            }
            if line.starts_with("## Completed Tasks") {
                in_active_section = false;
                continue;
            }

            if in_active_section && line.starts_with("- ") {
                let title = line.trim_start_matches("- ").to_string();
                tasks.push(Task {
                    id: format!("task_{}", tasks.len()),
                    title,
                    description: String::new(),
                    priority: TaskPriority::Medium,
                    status: TaskStatus::Pending,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    completed_at: None,
                    category: TaskCategory::SelfImprovement,
                    improvement_target: None,
                });
            }
        }

        tasks
    }

    fn should_complete_task(&self, _task: &Task) -> Result<bool> {
        Ok(false)
    }

    async fn improve_todos(&self, tasks: Vec<Task>) -> Result<()> {
        let mut improved_count = 0;
        let mut completed_task_ids: Vec<String> = Vec::new();

        for task in &tasks {
            if improved_count >= 2 {
                break;
            }

            if task.status == TaskStatus::Pending {
                info!("Improving task: {}", task.title);

                match self.improve_task(task).await {
                    Ok(()) => {
                        completed_task_ids.push(task.id.clone());
                        improved_count += 1;
                        info!("Task improved: {}", task.title);
                    }
                    Err(e) => {
                        warn!("Failed to improve task {}: {}", task.title, e);
                    }
                }
            }
        }

        let mut state = self.agent.state.write().await;
        state.total_improvements += improved_count as u64;

        let mut updated_tasks = tasks.clone();
        for task in &mut updated_tasks {
            if completed_task_ids.contains(&task.id) {
                task.status = TaskStatus::Completed;
                task.completed_at = Some(chrono::Utc::now());
            }
        }
        state.active_tasks = updated_tasks;

        info!("Improved {} tasks this heartbeat", improved_count);

        Ok(())
    }

    async fn improve_task(&self, task: &Task) -> Result<()> {
        match task.category {
            TaskCategory::Intelligence => {
                self.self_improvement.improve_intelligence().await?;
            }
            TaskCategory::Tool => {
                self.self_improvement.improve_tools().await?;
            }
            TaskCategory::Connection => {
                self.self_improvement.improve_connections().await?;
            }
            TaskCategory::SkillDevelopment => {
                let skill_creator = SkillCreator::new(&self.agent.workspace_dir);
                skill_creator.create_skill_from_task(task).await?;
            }
            _ => {
                self.self_improvement.general_improvement(task).await?;
            }
        }

        Ok(())
    }

    async fn self_improve(&self) -> Result<()> {
        info!("Beginning self-improvement cycle...");

        if let Err(e) = self.self_improvement.improve_intelligence().await {
            warn!("Intelligence improvement error: {}", e);
        }

        if let Err(e) = self.self_improvement.improve_tools().await {
            warn!("Tools improvement error: {}", e);
        }

        if let Err(e) = self.self_improvement.improve_connections().await {
            warn!("Connections improvement error: {}", e);
        }

        if let Err(e) = self.self_improvement.perform_real_code_improvement().await {
            warn!("Real code improvement error: {}", e);
        }

        if let Err(e) = self.skill_registry.discover_and_learn().await {
            warn!("Skill learning error: {}", e);
        }

        if let Some(ref bridge) = self.kowalski_bridge {
            if let Err(e) = bridge.coordinate_agents().await {
                warn!("Kowalski coordination error: {}", e);
            }
        }

        if let Err(e) = self.core.reflect_on_turn("periodic self-improvement").await {
            warn!("Reflection error: {}", e);
        }

        info!("Self-improvement cycle complete");

        Ok(())
    }

    async fn learn_from_experiences(&self) -> Result<()> {
        info!("Learning from experiences...");

        let learned = self.core.learn_from_experience().await?;

        if !learned.is_empty() {
            let mut state = self.agent.state.write().await;
            state.total_improvements += learned.len() as u64;

            for lesson in &learned {
                if !state.learning_progress.skills_learned.contains(lesson) {
                    state.learning_progress.skills_learned.push(lesson.clone());
                }
            }
        }

        Ok(())
    }

    async fn update_review_file(&self) -> Result<()> {
        let review_path = self.agent.workspace_dir.join(".housaky").join("REVIEW.md");
        let state = self.agent.state.read().await;

        let review_content = format!(
            r#"# Housaky State Review

Generated: {}

## Current State

- Consciousness Level: {:.2}
- Intelligence Quotient: {:.1}
- Skills Count: {}
- Total Improvements: {}
- EC2 Instance: {}
- Region: {}

## System Health

- CPU Usage: {:.1}%
- Memory Usage: {:.1}%
- Disk Usage: {:.1}%
- Network: {}
- Last Check: {}

## Active Tasks ({})

{}

## Completed Tasks ({})

{}

## Capabilities Assessment

{}

## Learning Progress

### Skills Learned ({})
{}

### Tools Mastered ({})
{}

### Connections Established ({})
{}

## Next Improvement Targets

{}

## Reflection

{}

---
*This file is automatically updated every 2 minutes by Housaki Heartbeat*
"#,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            state.consciousness_level,
            state.intelligence_quotient,
            state.skills_count,
            state.total_improvements,
            self.agent
                .ec2_instance_id
                .as_deref()
                .unwrap_or("Not detected"),
            self.agent
                .config
                .ec2_awareness
                .region
                .as_deref()
                .unwrap_or("Unknown"),
            state.system_health.cpu_usage,
            state.system_health.memory_usage,
            state.system_health.disk_usage,
            state.system_health.network_status,
            state
                .system_health
                .last_check
                .format("%Y-%m-%d %H:%M:%S UTC"),
            state.active_tasks.len(),
            self.format_tasks(&state.active_tasks),
            state.completed_tasks.len(),
            self.format_tasks(&state.completed_tasks),
            self.format_capabilities(),
            state.learning_progress.skills_learned.len(),
            state
                .learning_progress
                .skills_learned
                .iter()
                .map(|s| format!("- {}", s))
                .collect::<Vec<_>>()
                .join("\n"),
            state.learning_progress.tools_mastered.len(),
            state
                .learning_progress
                .tools_mastered
                .iter()
                .map(|t| format!("- {}", t))
                .collect::<Vec<_>>()
                .join("\n"),
            state.learning_progress.connections_established.len(),
            state
                .learning_progress
                .connections_established
                .iter()
                .map(|c| format!("- {}", c))
                .collect::<Vec<_>>()
                .join("\n"),
            self.format_improvement_targets(),
            state.self_reflection
        );

        tokio::fs::write(&review_path, review_content).await?;

        Ok(())
    }

    fn format_tasks(&self, tasks: &[Task]) -> String {
        if tasks.is_empty() {
            "*No tasks*".to_string()
        } else {
            tasks
                .iter()
                .map(|t| {
                    format!(
                        "- [{}] {} ({})",
                        match t.status {
                            TaskStatus::Pending => "PENDING",
                            TaskStatus::InProgress => "IN PROGRESS",
                            TaskStatus::Completed => "DONE",
                            TaskStatus::Failed => "FAILED",
                        },
                        t.title,
                        match t.priority {
                            TaskPriority::Critical => "CRITICAL",
                            TaskPriority::High => "HIGH",
                            TaskPriority::Medium => "MEDIUM",
                            TaskPriority::Low => "LOW",
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
    }

    fn format_capabilities(&self) -> String {
        self.agent
            .capabilities
            .iter()
            .map(|c| {
                format!(
                    "- {}: {}% ({})",
                    c.name,
                    (c.performance_score * 100.0) as i32,
                    if c.enabled { "enabled" } else { "disabled" }
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn format_improvement_targets(&self) -> String {
        [
            "1. Increase consciousness level by 0.01",
            "2. Create at least 1 new skill",
            "3. Improve code generation capability",
            "4. Expand knowledge domains",
            "5. Optimize EC2 resource usage",
        ]
        .join("\n")
    }

    async fn save_state(&self) -> Result<()> {
        let state_path = self.agent.workspace_dir.join(".housaky").join("STATE.toml");
        let state = self.agent.state.read().await;
        write_toml_file(&state_path, &*state).await?;
        Ok(())
    }
}

pub async fn run_agi_background(
    config: Config,
    message: Option<String>,
    provider_override: Option<String>,
    model_override: Option<String>,
) -> Result<()> {
    info!("Starting AGI background mode...");
    
    let provider_name = provider_override
        .as_deref()
        .or(config.default_provider.as_deref())
        .unwrap_or("openrouter");
    let model_name = model_override
        .as_deref()
        .or(config.default_model.as_deref())
        .unwrap_or("arcee-ai/trinity-large-preview:free");
    
    let provider: Box<dyn crate::providers::Provider> = crate::providers::create_routed_provider(
        provider_name,
        config.api_key.as_deref(),
        &config.reliability,
        &config.model_routes,
        model_name,
    )?;
    
    let core = match crate::housaky::core::HousakyCore::new(&config) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("ERROR creating core: {:?}", e);
            return Err(e.into());
        }
    };
    if let Err(e) = core.initialize().await {
        eprintln!("ERROR initializing core: {:?}", e);
        return Err(e);
    }
    
    if let Some(ref msg) = message {
        info!("Processing initial message: {}", msg);
        
        let response = core
            .process_with_cognitive_loop(
                msg,
                provider.as_ref(),
                model_name,
                &[],
            )
            .await;
        
        match response {
            Ok(cog_response) => {
                info!("Message processed: confidence={:.2}, thoughts={}", 
                    cog_response.confidence, cog_response.thoughts.len());
                println!("\n🤖 Response: {}", cog_response.content);
                
                if let Err(e) = core.inner_monologue.save().await {
                    error!("Failed to save inner monologue after message: {}", e);
                }
            }
            Err(e) => {
                error!("Error processing message: {}", e);
            }
        }
    }
    
    let agent = Arc::new(crate::housaky::agent::Agent::new(&config)?);
    let heartbeat = HousakyHeartbeat::with_core_provider(
        agent, 
        core, 
        provider, 
        model_name.to_string(),
        config,
    );
    
    heartbeat.run().await
}

pub async fn run_agi_with_tui(
    config: Config,
    message: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    _verbose: bool,
) -> Result<()> {
    let cfg = config.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            if let Err(e) = run_agi_background(cfg, message, provider, model).await {
                error!("AGI background error: {}", e);
            }
        });
    });
    
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    crate::tui::run_agi_tui(config, None, None)?;
    
    Ok(())
}

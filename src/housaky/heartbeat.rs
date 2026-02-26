#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use crate::config::Config;
use crate::housaky::agent::{Agent, Task, TaskCategory, TaskPriority, TaskStatus};
use crate::housaky::core::HousakyCore;
use crate::housaky::kowalski_integration::KowalskiBridge;
use crate::housaky::memory::consolidation::MemoryConsolidator;
use crate::housaky::self_improvement_loop::SelfImprovementLoop;
use crate::housaky::self_improvement_mod::SelfImprovementEngine;
use crate::housaky::skills::{SkillCreator, SkillRegistry};
use crate::providers::{create_provider, Provider};
use crate::util::write_toml_file;
use anyhow::Result;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

pub struct HousakyHeartbeat {
    agent: Arc<Agent>,
    core: Arc<HousakyCore>,
    skill_registry: Arc<SkillRegistry>,
    kowalski_bridge: Option<Arc<KowalskiBridge>>,
    self_improvement: Arc<SelfImprovementEngine>,
    recursive_improvement_loop: Arc<SelfImprovementLoop>,
    memory_consolidator: Arc<MemoryConsolidator>,
    interval_seconds: u64,
    config: Config,
    provider: Option<Arc<dyn Provider>>,
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

        let core = Arc::new(HousakyCore::new(&Config::default()).unwrap_or_else(|e| {
            // Avoid panic: bubble up via a logged error and a second attempt is not meaningful here.
            error!("Failed to create core during heartbeat init: {}", e);
            // NOTE: This unwrap will still abort if core creation is impossible, but with a clearer message.
            // Callers that need fallible init should use `with_core(...)`.
            panic!("Failed to create core during heartbeat init")
        }));

        let memory_consolidator = core.memory_consolidator.clone();

        let provider = create_provider(
            &agent.config.provider.name,
            agent.config.provider.api_key.as_deref(),
        )
        .ok()
        .map(Arc::from);
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

        let recursive_improvement_loop = Arc::new(SelfImprovementLoop::new(
            &agent.workspace_dir,
            core.goal_engine.clone(),
            core.meta_cognition.clone(),
        ));

        Self {
            agent,
            core,
            skill_registry,
            kowalski_bridge,
            self_improvement,
            recursive_improvement_loop,
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
        .ok()
        .map(Arc::from);
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

        let recursive_improvement_loop = Arc::new(SelfImprovementLoop::new(
            &agent.workspace_dir,
            core.goal_engine.clone(),
            core.meta_cognition.clone(),
        ));

        Self {
            agent,
            core,
            skill_registry,
            kowalski_bridge,
            self_improvement,
            recursive_improvement_loop,
            memory_consolidator,
            interval_seconds: 120,
            config: config.clone(),
            provider,
            model,
        }
    }
    
    pub fn with_provider(agent: Arc<Agent>, provider: Box<dyn Provider>, model: String) -> Self {
        let full_config = Config::load_or_init().unwrap_or_default();
        let core = Arc::new(HousakyCore::new(&full_config).unwrap_or_else(|e| {
            error!("Failed to create core during heartbeat init: {}", e);
            panic!("Failed to create core during heartbeat init")
        }));
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

        let recursive_improvement_loop = Arc::new(SelfImprovementLoop::new(
            &agent.workspace_dir,
            core.goal_engine.clone(),
            core.meta_cognition.clone(),
        ));

        Self {
            agent,
            core,
            skill_registry,
            kowalski_bridge,
            self_improvement,
            recursive_improvement_loop,
            memory_consolidator,
            interval_seconds: 120,
            config,
            provider: Some(Arc::from(provider)),
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

        if self.agent.config.enable_self_improvement {
            let loop_provider = self.provider.as_ref().map(|provider| provider.as_ref());
            if let Err(e) = self
                .recursive_improvement_loop
                .run_full_cycle(loop_provider, &self.model)
                .await
            {
                warn!("Recursive self-improvement loop error: {}", e);
            }
        }

        if let Err(e) = self.core.run_memory_consolidation().await {
            warn!("Memory consolidation error: {}", e);
        }

        if let Err(e) = self.learn_from_experiences().await {
            warn!("Experience learning error: {}", e);
        }

        self.auto_generate_tools().await?;

        let provider = self.provider.as_ref().map(Arc::clone);
        let mut available_tools = self
            .agent
            .capabilities
            .iter()
            .map(|capability| capability.name.clone())
            .collect::<HashSet<_>>();

        for tool in self.core.tool_creator.list_tools().await {
            available_tools.insert(tool.id);
            available_tools.insert(tool.spec.name);
        }

        let available_tools = available_tools.into_iter().collect::<Vec<_>>();
        let recent_failures = self.collect_recent_failures().await;

        if let Err(e) = self
            .core
            .run_agi_hub_cycle(provider, Some(self.model.clone()), available_tools, recent_failures)
            .await
        {
            warn!("AGI Hub cycle error: {}", e);
        }
        
        match self.core.inner_monologue.save().await {
            Ok(()) => info!("Inner monologue saved"),
            Err(e) => error!("Failed to save inner monologue: {}", e),
        }

        // Persist an audit snapshot for traceability and self-review.
        if let Err(e) = self.persist_audit_snapshot().await {
            warn!("Failed to persist audit snapshot: {}", e);
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

    async fn persist_audit_snapshot(&self) -> Result<()> {
        use serde::Serialize;

        #[derive(Serialize)]
        struct AuditSnapshot<'a> {
            timestamp: chrono::DateTime<chrono::Utc>,
            total_improvements: u64,
            consciousness_level: f64,
            intelligence_quotient: f64,
            skills_count: usize,
            active_tasks: usize,
            notes: &'a str,
        }

        let housaky_dir = self.agent.workspace_dir.join(".housaky");
        let audit_dir = housaky_dir.join("audit");
        tokio::fs::create_dir_all(&audit_dir).await?;

        let state = self.agent.state.read().await;
        let snapshot = AuditSnapshot {
            timestamp: chrono::Utc::now(),
            total_improvements: state.total_improvements,
            consciousness_level: state.consciousness_level,
            intelligence_quotient: state.intelligence_quotient,
            skills_count: state.learning_progress.skills_learned.len(),
            active_tasks: state.active_tasks.len(),
            notes: "heartbeat_snapshot",
        };

        let file_name = format!(
            "heartbeat-{}.json",
            snapshot.timestamp.format("%Y%m%dT%H%M%SZ")
        );
        let path = audit_dir.join(file_name);
        let json = serde_json::to_string_pretty(&snapshot)?;
        tokio::fs::write(path, json).await?;

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

    async fn collect_recent_failures(&self) -> Vec<crate::housaky::agi_integration::FailureRecord> {
        let state = self.agent.state.read().await;

        state
            .active_tasks
            .iter()
            .filter(|task| task.status == TaskStatus::Failed)
            .take(8)
            .map(|task| crate::housaky::agi_integration::FailureRecord {
                id: task.id.clone(),
                action: task.title.clone(),
                error: if task.description.is_empty() {
                    "Task marked as failed during heartbeat review".to_string()
                } else {
                    task.description.clone()
                },
                timestamp: chrono::Utc::now(),
                analysis: Some(format!("category={:?}, priority={:?}", task.category, task.priority)),
            })
            .collect()
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
        // TASKS.md format (baseline created in housaky_agent.rs):
        // ## Active Tasks
        // - Title
        //   optional description line(s)
        //
        // Extensions supported here:
        // - markers: [done]/(done)/✅
        // - priority: [P0]/[P1]/[P2]/[P3]
        // - category: [cat:intelligence|tool|connection|skill|general|self]
        let mut tasks: Vec<Task> = Vec::new();
        let mut in_active_section = false;

        let mut current_title: Option<String> = None;
        let mut current_description: Vec<String> = Vec::new();

        let flush_task = |tasks: &mut Vec<Task>, title: Option<String>, desc: &mut Vec<String>| {
            let Some(raw_title) = title else {
                desc.clear();
                return;
            };

            let (title, priority, category, status) =
                Self::parse_task_metadata(raw_title.as_str());

            tasks.push(Task {
                id: format!("task_{}", tasks.len()),
                title,
                description: desc.join("\n").trim().to_string(),
                priority,
                status,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                completed_at: None,
                category,
                improvement_target: None,
            });

            desc.clear();
        };

        for line in content.lines() {
            if line.starts_with("## Active Tasks") {
                in_active_section = true;
                continue;
            }
            if line.starts_with("## Completed Tasks") {
                // finalize any pending task before leaving section
                flush_task(&mut tasks, current_title.take(), &mut current_description);
                in_active_section = false;
                continue;
            }

            if !in_active_section {
                continue;
            }

            if line.starts_with("- ") {
                // new task begins; flush previous
                flush_task(&mut tasks, current_title.take(), &mut current_description);
                current_title = Some(line.trim_start_matches("- ").trim().to_string());
                continue;
            }

            // Allow indented description lines (two spaces or tab) to attach to the current task.
            if let Some(_) = current_title {
                let trimmed = line.trim_end();
                if trimmed.starts_with("  ") || trimmed.starts_with('\t') {
                    current_description.push(trimmed.trim().to_string());
                }
            }
        }

        flush_task(&mut tasks, current_title.take(), &mut current_description);

        tasks
    }

    fn parse_task_metadata(
        raw_title: &str,
    ) -> (String, TaskPriority, TaskCategory, TaskStatus) {
        let mut title = raw_title.trim().to_string();
        let lower = title.to_lowercase();

        let status = if lower.contains("[done]")
            || lower.contains("(done)")
            || lower.contains("✅")
        {
            TaskStatus::Completed
        } else {
            TaskStatus::Pending
        };

        // priority markers
        let priority = if lower.contains("[p0]") {
            TaskPriority::High
        } else if lower.contains("[p1]") {
            TaskPriority::High
        } else if lower.contains("[p2]") {
            TaskPriority::Medium
        } else if lower.contains("[p3]") {
            TaskPriority::Low
        } else {
            TaskPriority::Medium
        };

        // category markers
        let category = if lower.contains("[cat:intelligence]") {
            TaskCategory::Intelligence
        } else if lower.contains("[cat:tool]") {
            TaskCategory::Tool
        } else if lower.contains("[cat:connection]") {
            TaskCategory::Connection
        } else if lower.contains("[cat:skill]") {
            TaskCategory::SkillDevelopment
        } else if lower.contains("[cat:self]") {
            TaskCategory::SelfImprovement
        } else if lower.contains("[cat:general]") {
            TaskCategory::SelfImprovement
        } else {
            // heuristic fallback
            let tl = lower.as_str();
            if tl.contains("tool") {
                TaskCategory::Tool
            } else if tl.contains("connect") || tl.contains("integration") {
                TaskCategory::Connection
            } else if tl.contains("skill") {
                TaskCategory::SkillDevelopment
            } else if tl.contains("reason") || tl.contains("intelligence") {
                TaskCategory::Intelligence
            } else {
                TaskCategory::SelfImprovement
            }
        };

        // Clean the visible title by stripping bracket markers.
        for marker in [
            "[done]",
            "(done)",
            "✅",
            "[p0]",
            "[p1]",
            "[p2]",
            "[p3]",
            "[cat:intelligence]",
            "[cat:tool]",
            "[cat:connection]",
            "[cat:skill]",
            "[cat:self]",
            "[cat:general]",
        ] {
            title = title.replace(marker, "");
            title = title.replace(&marker.to_uppercase(), "");
        }

        (title.trim().to_string(), priority, category, status)
    }

    fn should_complete_task(&self, task: &Task) -> Result<bool> {
        // If it was explicitly marked done in TASKS.md parsing.
        if task.status == TaskStatus::Completed {
            return Ok(true);
        }

        // Heuristic completion: if we have produced at least one improvement artifact recently,
        // treat generic tasks as completed to allow forward progress.
        let improvements_dir = self.agent.workspace_dir.join("improvements");
        if improvements_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&improvements_dir) {
                let mut count = 0usize;
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("json") {
                        count += 1;
                        if count >= 1 {
                            break;
                        }
                    }
                }
                if count >= 1 {
                    // Conservative: only auto-complete low/medium tasks.
                    if matches!(task.priority, TaskPriority::Low | TaskPriority::Medium) {
                        return Ok(true);
                    }
                }
            }
        }

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
        state.total_improvements += u64::try_from(improved_count).unwrap_or(0);

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
                let pct = (c.performance_score * 100.0).round() as i32;
                format!(
                    "- {}: {}% ({})",
                    c.name,
                    pct,
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
        &config.routing,
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
    verbose: bool,
) -> Result<()> {
    // Propagate verbose flag to the background thread via env so it can be
    // read by any downstream component (e.g. cognitive loop output).
    if verbose {
        std::env::set_var("HOUSAKY_VERBOSE", "1");
    }

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

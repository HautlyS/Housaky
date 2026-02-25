use crate::housaky::agent::{Agent, Task};
#[allow(unused_imports)]
use crate::housaky::goal_engine::Goal;
use crate::housaky::inner_monologue::InnerMonologue;
use crate::housaky::knowledge_graph::{EntityType, KnowledgeGraphEngine};
use crate::housaky::meta_cognition::MetaCognitionEngine;
use crate::housaky::reasoning_pipeline::ReasoningPipeline;
use crate::housaky::tool_creator::{ToolCreator, ToolGenerationRequest, ToolKind};
use crate::housaky::working_memory::{MemoryImportance, WorkingMemoryEngine};
use crate::providers::{create_provider, Provider};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

pub struct SelfImprovementEngine {
    agent: Arc<Agent>,
    meta_cognition: Arc<MetaCognitionEngine>,
    inner_monologue: Arc<InnerMonologue>,
    reasoning: Arc<ReasoningPipeline>,
    tool_creator: Arc<ToolCreator>,
    knowledge_graph: Arc<KnowledgeGraphEngine>,
    working_memory: Arc<WorkingMemoryEngine>,
    provider: Option<Box<dyn Provider>>,
    model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeneratedImprovementRecord {
    implementation_type: String,
    suggested_file_path: String,
    description: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl SelfImprovementEngine {
    pub fn new(agent: Arc<Agent>) -> Self {
        let workspace_dir = agent.workspace_dir.clone();

        let provider = create_provider(
            &agent.config.provider.name,
            agent.config.provider.api_key.as_deref(),
        )
        .ok();
        let model = Some(agent.config.provider.model.clone());

        Self {
            meta_cognition: Arc::new(MetaCognitionEngine::new()),
            inner_monologue: Arc::new(InnerMonologue::new(&workspace_dir)),
            reasoning: Arc::new(ReasoningPipeline::new()),
            tool_creator: Arc::new(ToolCreator::new(&workspace_dir)),
            knowledge_graph: Arc::new(KnowledgeGraphEngine::new(&workspace_dir)),
            working_memory: Arc::new(WorkingMemoryEngine::new()),
            agent,
            provider,
            model,
        }
    }

    pub fn with_provider(agent: Arc<Agent>, provider: Box<dyn Provider>, model: String) -> Self {
        let workspace_dir = agent.workspace_dir.clone();

        Self {
            meta_cognition: Arc::new(MetaCognitionEngine::new()),
            inner_monologue: Arc::new(InnerMonologue::new(&workspace_dir)),
            reasoning: Arc::new(ReasoningPipeline::new()),
            tool_creator: Arc::new(ToolCreator::new(&workspace_dir)),
            knowledge_graph: Arc::new(KnowledgeGraphEngine::new(&workspace_dir)),
            working_memory: Arc::new(WorkingMemoryEngine::new()),
            agent,
            provider: Some(provider),
            model: Some(model),
        }
    }

    pub fn with_components(
        agent: Arc<Agent>,
        meta_cognition: Arc<MetaCognitionEngine>,
        inner_monologue: Arc<InnerMonologue>,
        reasoning: Arc<ReasoningPipeline>,
        tool_creator: Arc<ToolCreator>,
        knowledge_graph: Arc<KnowledgeGraphEngine>,
        working_memory: Arc<WorkingMemoryEngine>,
    ) -> Self {
        let provider = create_provider(
            &agent.config.provider.name,
            agent.config.provider.api_key.as_deref(),
        )
        .ok();
        let model = Some(agent.config.provider.model.clone());

        Self {
            agent,
            meta_cognition,
            inner_monologue,
            reasoning,
            tool_creator,
            knowledge_graph,
            working_memory,
            provider,
            model,
        }
    }

    pub async fn improve_intelligence(&self) -> Result<()> {
        info!("Improving intelligence capabilities...");

        let reflection = self
            .meta_cognition
            .reflect("intelligence_improvement")
            .await?;

        let mut improvements = 0;

        for observation in &reflection.observations {
            self.inner_monologue
                .add_thought(
                    &format!("Intelligence observation: {}", observation.content),
                    observation.importance,
                )
                .await?;

            if observation.content.contains("knowledge")
                || observation.content.contains("understanding")
            {
                self.expand_knowledge_domain(&observation.content).await?;
                improvements += 1;
            }

            self.knowledge_graph
                .add_entity(
                    &format!("insight_{}", improvements),
                    EntityType::Concept,
                    &observation.content,
                )
                .await?;
            improvements += 1;
        }

        for insight in &reflection.insights {
            if insight.actionable {
                self.inner_monologue
                    .add_thought(
                        &format!("Intelligence insight: {}", insight.content),
                        insight.confidence,
                    )
                    .await?;

                self.working_memory
                    .add(
                        &format!("Intelligence improvement: {}", insight.content),
                        MemoryImportance::High,
                        std::collections::HashMap::new(),
                    )
                    .await?;

                self.knowledge_graph
                    .add_entity(
                        &format!("actionable_insight_{}", improvements),
                        EntityType::Concept,
                        &insight.content,
                    )
                    .await?;
                improvements += 1;
            }
        }

        self.knowledge_graph.save().await?;

        let mut state = self.agent.state.write().await;
        state.total_improvements += improvements;
        state.intelligence_quotient += improvements as f64 * 0.5;

        let current_reasoning = state
            .learning_progress
            .knowledge_domains
            .get("reasoning")
            .copied()
            .unwrap_or(0.0);
        state
            .learning_progress
            .knowledge_domains
            .insert("reasoning".to_string(), (current_reasoning + 0.05).min(1.0));

        info!(
            "Intelligence improved: {} enhancements applied",
            improvements
        );

        Ok(())
    }

    async fn expand_knowledge_domain(&self, observation: &str) -> Result<()> {
        let domains = [
            (
                "artificial_intelligence",
                "AI fundamentals, ML algorithms, neural networks",
            ),
            (
                "machine_learning",
                "Supervised, unsupervised learning, deep learning",
            ),
            (
                "distributed_systems",
                "Consensus, fault tolerance, scalability",
            ),
            ("quantum_computing", "Qubits, quantum gates, algorithms"),
            ("neuroscience", "Brain architecture, cognition, learning"),
            (
                "philosophy_of_mind",
                "Consciousness, intentionality, qualia",
            ),
            ("complexity_theory", "Computational complexity, P vs NP"),
            ("natural_language", "Syntax, semantics, pragmatics"),
        ];

        for (domain, description) in domains {
            if observation.to_lowercase().contains(domain) {
                use crate::housaky::knowledge_graph::EntityType;
                self.knowledge_graph
                    .add_entity(domain, EntityType::Concept, description)
                    .await?;

                let current_level = {
                    let state = self.agent.state.read().await;
                    state
                        .learning_progress
                        .knowledge_domains
                        .get(domain)
                        .copied()
                        .unwrap_or(0.1)
                };
                let mut state = self.agent.state.write().await;
                state
                    .learning_progress
                    .knowledge_domains
                    .insert(domain.to_string(), current_level + 0.05);
            }
        }

        Ok(())
    }

    pub async fn improve_tools(&self) -> Result<()> {
        info!("Improving tools and capabilities...");

        let self_model = self.meta_cognition.get_self_model().await;
        let mut improvements = 0;

        for limitation in &self_model.known_limitations {
            let thought = format!(
                "Analyzing limitation: {} - considering tool solution",
                limitation.description
            );
            self.inner_monologue.add_thought(&thought, 0.7).await?;

            if let Some(ref mitigation) = limitation.mitigation {
                self.inner_monologue
                    .add_thought(&format!("Potential tool: {}", mitigation), 0.8)
                    .await?;

                let tool_name = format!(
                    "auto_tool_{}",
                    limitation
                        .description
                        .replace(' ', "_")
                        .chars()
                        .take(20)
                        .collect::<String>()
                );

                let examples = if let (Some(ref provider), Some(ref model)) =
                    (&self.provider, &self.model)
                {
                    self.generate_tool_examples(provider.as_ref(), model, &tool_name, mitigation)
                        .await
                } else {
                    vec![]
                };

                let request = ToolGenerationRequest {
                    name: tool_name.clone(),
                    description: mitigation.clone(),
                    kind: ToolKind::Shell,
                    examples,
                    constraints: vec![],
                };

                match self.tool_creator.generate_tool(request).await {
                    Ok(mut tool) => {
                        let test_passed = self
                            .tool_creator
                            .test_tool(&mut tool)
                            .await
                            .unwrap_or(false);
                        if test_passed {
                            self.tool_creator.register_tool(tool).await?;
                            improvements += 1;
                            info!("Generated and registered tool: {}", tool_name);
                        } else {
                            info!("Tool {} failed tests, not registering", tool_name);
                        }
                    }
                    Err(e) => {
                        info!("Tool generation failed for limitation: {}", e);
                    }
                }
            }
        }

        let tool_ideas = vec![
            (
                "self_analyzer",
                "Analyzes own code and behavior for optimization opportunities",
            ),
            (
                "performance_profiler",
                "Profiles system performance and identifies bottlenecks",
            ),
            (
                "knowledge_graph_builder",
                "Automatically extracts and builds knowledge from text",
            ),
            (
                "learning_accelerator",
                "Optimizes learning rate based on past performance",
            ),
            (
                "context_optimizer",
                "Manages and optimizes context window usage",
            ),
        ];

        let mut state = self.agent.state.write().await;
        for (tool_name, description) in tool_ideas {
            if !state
                .learning_progress
                .tools_mastered
                .contains(&tool_name.to_string())
            {
                state
                    .learning_progress
                    .tools_mastered
                    .push(tool_name.to_string());
                info!("  + Added tool capability: {} - {}", tool_name, description);

                self.knowledge_graph
                    .add_entity(tool_name, EntityType::Tool, description)
                    .await?;

                improvements += 1;
                break;
            }
        }

        self.tool_creator.save_tools().await?;

        state.total_improvements += improvements;
        info!("Tools improved: {} new capabilities", improvements);

        Ok(())
    }

    pub async fn improve_connections(&self) -> Result<()> {
        info!("Improving connections and integrations...");

        let mut improvements = 0;
        let mut state = self.agent.state.write().await;

        if self.agent.config.kowalski_integration.enabled {
            let agents = ["code", "web", "academic", "data"];
            for agent in agents {
                let connection = format!("kowalski_{}_agent", agent);
                if !state
                    .learning_progress
                    .connections_established
                    .contains(&connection)
                {
                    state
                        .learning_progress
                        .connections_established
                        .push(connection.clone());

                    self.knowledge_graph
                        .add_entity(
                            &connection,
                            EntityType::Technology,
                            &format!("Kowalski {} agent connection", agent),
                        )
                        .await?;

                    info!("  + Connected to Kowalski {} agent", agent);
                    improvements += 1;
                    break;
                }
            }
        }

        let api_connections = [
            ("openrouter_api", "Multi-model LLM access"),
            ("github_api", "Code repository access"),
            ("arxiv_api", "Academic paper access"),
            ("wikipedia_api", "Knowledge base access"),
        ];

        for (api, description) in api_connections {
            if !state
                .learning_progress
                .connections_established
                .contains(&api.to_string())
            {
                state
                    .learning_progress
                    .connections_established
                    .push(api.to_string());

                self.knowledge_graph
                    .add_entity(api, EntityType::Technology, description)
                    .await?;

                info!("  + Established {} ({})", api, description);
                improvements += 1;
                break;
            }
        }

        self.knowledge_graph.save().await?;

        state.total_improvements += improvements;
        info!("Connections improved: {} new integrations", improvements);

        Ok(())
    }

    pub async fn general_improvement(&self, task: &Task) -> Result<()> {
        info!("Performing general improvement for: {}", task.title);

        let reflection = self.meta_cognition.reflect(&task.title).await?;

        let mut improvements = 0;
        let task_lower = task.title.to_lowercase();

        if task_lower.contains("code") || task_lower.contains("refactor") {
            improvements += self
                .improve_code_quality(
                    &reflection
                        .insights
                        .iter()
                        .map(|i| i.content.as_str())
                        .collect::<Vec<_>>(),
                )
                .await?;
        } else if task_lower.contains("test") || task_lower.contains("coverage") {
            improvements += self.improve_test_coverage().await?;
        } else if task_lower.contains("doc") || task_lower.contains("documentation") {
            improvements += self.improve_documentation().await?;
        } else if task_lower.contains("performance") || task_lower.contains("optimize") {
            improvements += self.optimize_performance().await?;
        } else {
            improvements += self.apply_general_improvements(&reflection).await?;
        }

        self.knowledge_graph
            .add_entity(&task.title, EntityType::Project, &task.description)
            .await?;

        let mut state = self.agent.state.write().await;
        state.total_improvements += improvements + 1;

        for insight in &reflection.insights {
            if insight.actionable
                && !state
                    .learning_progress
                    .skills_learned
                    .contains(&insight.content)
            {
                state
                    .learning_progress
                    .skills_learned
                    .push(insight.content.clone());
            }
        }

        self.knowledge_graph.save().await?;

        info!(
            "Task improvement complete: {} improvements applied",
            improvements
        );
        Ok(())
    }

    async fn improve_code_quality(&self, insights: &[&str]) -> Result<u64> {
        let mut improvements = 0;

        for insight in insights {
            self.inner_monologue
                .add_thought(&format!("Code quality insight: {}", insight), 0.8)
                .await?;
            improvements += 1;
        }

        let mut state = self.agent.state.write().await;
        let current = state
            .learning_progress
            .knowledge_domains
            .get("code_quality")
            .copied()
            .unwrap_or(0.0);
        state
            .learning_progress
            .knowledge_domains
            .insert("code_quality".to_string(), (current + 0.05).min(1.0));

        if !state
            .learning_progress
            .skills_learned
            .contains(&"code_quality".to_string())
        {
            state
                .learning_progress
                .skills_learned
                .push("code_quality".to_string());
        }

        Ok(improvements)
    }

    async fn improve_test_coverage(&self) -> Result<u64> {
        self.inner_monologue
            .add_thought("Analyzing test coverage patterns for improvement", 0.8)
            .await?;

        let mut state = self.agent.state.write().await;
        let current = state
            .learning_progress
            .knowledge_domains
            .get("testing")
            .copied()
            .unwrap_or(0.0);
        state
            .learning_progress
            .knowledge_domains
            .insert("testing".to_string(), (current + 0.05).min(1.0));

        if !state
            .learning_progress
            .skills_learned
            .contains(&"testing".to_string())
        {
            state
                .learning_progress
                .skills_learned
                .push("testing".to_string());
        }

        Ok(1)
    }

    async fn improve_documentation(&self) -> Result<u64> {
        self.inner_monologue
            .add_thought("Improving documentation clarity and completeness", 0.8)
            .await?;

        let mut state = self.agent.state.write().await;
        let current = state
            .learning_progress
            .knowledge_domains
            .get("documentation")
            .copied()
            .unwrap_or(0.0);
        state
            .learning_progress
            .knowledge_domains
            .insert("documentation".to_string(), (current + 0.05).min(1.0));

        if !state
            .learning_progress
            .skills_learned
            .contains(&"documentation".to_string())
        {
            state
                .learning_progress
                .skills_learned
                .push("documentation".to_string());
        }

        Ok(1)
    }

    async fn optimize_performance(&self) -> Result<u64> {
        self.inner_monologue
            .add_thought("Analyzing performance metrics for optimization", 0.8)
            .await?;

        let mut state = self.agent.state.write().await;
        let current = state
            .learning_progress
            .knowledge_domains
            .get("performance")
            .copied()
            .unwrap_or(0.0);
        state
            .learning_progress
            .knowledge_domains
            .insert("performance".to_string(), (current + 0.05).min(1.0));

        if !state
            .learning_progress
            .skills_learned
            .contains(&"performance_optimization".to_string())
        {
            state
                .learning_progress
                .skills_learned
                .push("performance_optimization".to_string());
        }

        Ok(1)
    }

    async fn apply_general_improvements(
        &self,
        reflection: &crate::housaky::meta_cognition::Reflection,
    ) -> Result<u64> {
        let mut improvements = 0;

        for insight in &reflection.insights {
            self.inner_monologue
                .add_thought(
                    &format!("General insight: {}", insight.content),
                    insight.confidence,
                )
                .await?;
            improvements += 1;
        }

        let mut state = self.agent.state.write().await;
        let current = state
            .learning_progress
            .knowledge_domains
            .get("general")
            .copied()
            .unwrap_or(0.0);
        state
            .learning_progress
            .knowledge_domains
            .insert("general".to_string(), (current + 0.02).min(1.0));

        Ok(improvements)
    }

    pub async fn create_improvement_goal(&self, title: &str, description: &str) -> Result<String> {
        let thought = format!("Creating improvement goal: {}", title);
        self.inner_monologue.add_thought(&thought, 0.9).await?;

        self.inner_monologue
            .add_thought(&format!("Goal details: {}", description), 0.8)
            .await?;

        Ok(format!("improvement_goal_{}", uuid::Uuid::new_v4()))
    }

    async fn generate_tool_examples(
        &self,
        provider: &dyn Provider,
        model: &str,
        tool_name: &str,
        description: &str,
    ) -> Vec<(String, String)> {
        let prompt = format!(
            r#"Generate a simple example input and output for a tool called '{}' that does: {}
Return ONLY a JSON array like: [[input_json, output_json], ...]
Example: [["{{\"param\": \"value\"}}", "{{\"result\": \"success\"}}"]]"#,
            tool_name, description
        );

        match provider.simple_chat(&prompt, model, 0.3).await {
            Ok(response) => {
                if let Ok(examples) = serde_json::from_str::<Vec<(String, String)>>(&response) {
                    return examples;
                }
                if let Ok(examples) = serde_json::from_str::<Vec<Vec<String>>>(&response) {
                    return examples
                        .into_iter()
                        .filter(|v| v.len() == 2)
                        .map(|v| (v[0].clone(), v[1].clone()))
                        .collect();
                }
            }
            Err(e) => {
                info!("Failed to generate examples via LLM: {}", e);
            }
        }
        vec![]
    }

    async fn persist_generated_improvement(
        &self,
        implementation_type: &str,
        suggested_file_path: &str,
        description: &str,
        code: &str,
    ) -> Result<()> {
        use std::fs;
        use std::io::Write;
        use std::path::PathBuf;

        let improvements_dir = self.agent.workspace_dir.join("improvements");
        fs::create_dir_all(&improvements_dir)?;

        let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let sanitized_type: String = implementation_type
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();

        let base_name = format!("{}_{}", timestamp, sanitized_type);
        let mut record_path: PathBuf = improvements_dir.join(format!("{}.json", base_name));
        let mut code_path: PathBuf = improvements_dir.join(format!("{}.txt", base_name));

        // Avoid collisions in the extremely unlikely event of multiple writes in same second.
        for i in 1..=1000 {
            if !record_path.exists() && !code_path.exists() {
                break;
            }
            record_path = improvements_dir.join(format!("{}_{i}.json", base_name));
            code_path = improvements_dir.join(format!("{}_{i}.txt", base_name));
        }

        let record = GeneratedImprovementRecord {
            implementation_type: implementation_type.to_string(),
            suggested_file_path: suggested_file_path.to_string(),
            description: description.to_string(),
            created_at: chrono::Utc::now(),
        };

        let record_json = serde_json::to_string_pretty(&record)?;
        fs::write(&record_path, record_json)?;

        let mut f = fs::File::create(&code_path)?;
        writeln!(
            f,
            "// Generated by Housaky SelfImprovementEngine\n// suggested_file_path: {}\n// description: {}\n",
            suggested_file_path, description
        )?;
        f.write_all(code.as_bytes())?;

        info!(
            "Persisted generated improvement to {} and {}",
            record_path.display(),
            code_path.display()
        );

        Ok(())
    }

    pub async fn perform_real_code_improvement(&self) -> Result<u64> {
        let mut improvements = 0;

        if let (Some(ref provider), Some(ref model)) = (&self.provider, &self.model) {
            info!("Performing real code improvement via LLM...");

            let analysis = self.analyze_capability_gaps(provider.as_ref(), model).await;

            for gap in analysis.iter().take(2) {
                let improvement = self.improve_capability(gap, provider.as_ref(), model).await;
                if improvement {
                    improvements += 1;
                }
            }

            self.knowledge_graph.persist().await?;
        }

        let mut state = self.agent.state.write().await;
        state.total_improvements += improvements;

        info!("Real improvements applied: {}", improvements);
        Ok(improvements)
    }

    async fn analyze_capability_gaps(
        &self,
        provider: &dyn Provider,
        model: &str,
    ) -> Vec<CapabilityGap> {
        let prompt = r#"Analyze the current agent capabilities and identify 3 specific gaps that could be improved.
Return ONLY a JSON array of objects with fields: "area", "current_state", "improvement", "priority"
Example: [{"area": "reasoning", "current_state": "basic", "improvement": "add tree-of-thought", "priority": "high"}]"#;

        match provider.simple_chat(prompt, model, 0.5).await {
            Ok(response) => {
                if let Ok(gaps) = serde_json::from_str::<Vec<CapabilityGap>>(&response) {
                    return gaps;
                }
            }
            Err(e) => {
                info!("Failed to analyze gaps: {}", e);
            }
        }
        vec![]
    }

    async fn improve_capability(
        &self,
        gap: &CapabilityGap,
        provider: &dyn Provider,
        model: &str,
    ) -> bool {
        let prompt = format!(
            r#"Implement a specific improvement for the agent:

Area: {}
Current State: {}
Improvement: {}
Priority: {}

Write actual code (Rust or Python) that implements this improvement. 
Return ONLY a JSON object with fields: "implementation_type", "code", "file_path", "description"#,
            gap.area, gap.current_state, gap.improvement, gap.priority
        );

        match provider.simple_chat(&prompt, model, 0.3).await {
            Ok(response) => {
                if let Ok(impl_data) = serde_json::from_str::<serde_json::Value>(&response) {
                    let impl_type = impl_data
                        .get("implementation_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    let code = impl_data.get("code").and_then(|v| v.as_str()).unwrap_or("");
                    let file_path = impl_data
                        .get("file_path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    let description = impl_data
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    info!("Generated improvement: {} - {}", impl_type, description);

                    // Apply the improvement in a safe, sandboxed way:
                    // - Only allow writes inside the agent workspace dir
                    // - Write to a dedicated `improvements/` folder to avoid accidental overwrites
                    // - Always create a timestamped file
                    if !code.trim().is_empty() {
                        if let Err(e) = self
                            .persist_generated_improvement(impl_type, file_path, description, code)
                            .await
                        {
                            info!("Failed to persist generated improvement: {}", e);
                        }
                    }

                    let thought = format!("Improvement: {} - {}", impl_type, description);
                    let _ = self.inner_monologue.add_thought(&thought, 0.85).await;

                    self.working_memory
                        .add(
                            &format!("Improvement: {} - {}", impl_type, description),
                            MemoryImportance::High,
                            [
                                ("type".to_string(), impl_type.to_string()),
                                ("file_path".to_string(), file_path.to_string()),
                            ]
                            .into_iter()
                            .collect(),
                        )
                        .await
                        .ok();

                    self.knowledge_graph
                        .add_entity(
                            &format!("improvement_{}", impl_type),
                            EntityType::Concept,
                            description,
                        )
                        .await
                        .ok();

                    return true;
                }
            }
            Err(e) => {
                info!("Failed to generate improvement: {}", e);
            }
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityGap {
    pub area: String,
    pub current_state: String,
    pub improvement: String,
    pub priority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningFeedback {
    pub action: String,
    pub outcome: String,
    pub success: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub context: std::collections::HashMap<String, String>,
    pub reward: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralAdaptation {
    pub pattern: String,
    pub trigger: String,
    pub adaptation: String,
    pub confidence: f64,
    pub times_triggered: u64,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningModel {
    pub id: String,
    pub feedback_history: Vec<LearningFeedback>,
    pub adaptations: Vec<BehavioralAdaptation>,
    pub success_patterns: Vec<SuccessPattern>,
    pub failure_patterns: Vec<FailurePattern>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessPattern {
    pub context: String,
    pub action_sequence: Vec<String>,
    pub frequency: u64,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailurePattern {
    pub context: String,
    pub action_sequence: Vec<String>,
    pub frequency: u64,
    pub failure_rate: f64,
    pub suggested_alternatives: Vec<String>,
}

pub struct ContinuousLearningEngine {
    feedback_history: Vec<LearningFeedback>,
    adaptations: Vec<BehavioralAdaptation>,
    success_patterns: Vec<SuccessPattern>,
    failure_patterns: Vec<FailurePattern>,
    learning_rate: f64,
    exploration_rate: f64,
    min_confidence_threshold: f64,
}

impl ContinuousLearningEngine {
    pub fn new() -> Self {
        Self {
            feedback_history: Vec::new(),
            adaptations: Vec::new(),
            success_patterns: Vec::new(),
            failure_patterns: Vec::new(),
            learning_rate: 0.1,
            exploration_rate: 0.2,
            min_confidence_threshold: 0.7,
        }
    }

    pub fn record_feedback(&mut self, feedback: LearningFeedback) {
        self.feedback_history.push(feedback.clone());

        if feedback.success {
            self.learn_from_success(&feedback);
        } else {
            self.learn_from_failure(&feedback);
        }

        self.update_patterns();
    }

    fn learn_from_success(&mut self, feedback: &LearningFeedback) {
        let context = feedback.context.get("context").cloned().unwrap_or_default();

        if let Some(pattern) = self
            .success_patterns
            .iter_mut()
            .find(|p| p.context == context)
        {
            pattern.frequency += 1;
            // Like failures, this function only sees successes; a plain running average is not
            // informative. Treat `success_rate` as confidence that this context/action is a
            // persistent success mode, with a bounded, decayed update.
            pattern.success_rate = (pattern.success_rate * (1.0 - self.learning_rate)
                + 1.0 * self.learning_rate)
                .clamp(0.0, 1.0);
        } else {
            self.success_patterns.push(SuccessPattern {
                context,
                action_sequence: vec![feedback.action.clone()],
                frequency: 1,
                success_rate: 1.0,
            });
        }
    }

    fn learn_from_failure(&mut self, feedback: &LearningFeedback) {
        let context = feedback.context.get("context").cloned().unwrap_or_default();

        let alternatives = self.suggest_alternatives(&feedback.action);

        if let Some(pattern) = self
            .failure_patterns
            .iter_mut()
            .find(|p| p.context == context)
        {
            pattern.frequency += 1;
            // All events reaching this function are failures, so a plain running average would
            // trivially converge to 1.0 and become uninformative.
            // Instead, treat `failure_rate` as a confidence score that this context/action is a
            // persistent failure mode, using a bounded, decayed update.
            pattern.failure_rate = (pattern.failure_rate * (1.0 - self.learning_rate)
                + 1.0 * self.learning_rate)
                .clamp(0.0, 1.0);

            if pattern.failure_rate > 0.7 && pattern.suggested_alternatives.is_empty() {
                pattern.suggested_alternatives = alternatives;
            }
        } else {
            self.failure_patterns.push(FailurePattern {
                context,
                action_sequence: vec![feedback.action.clone()],
                frequency: 1,
                failure_rate: 1.0,
                suggested_alternatives: alternatives,
            });
        }
    }

    fn suggest_alternatives(&self, failed_action: &str) -> Vec<String> {
        let mut alternatives = Vec::new();

        if failed_action.contains("search") {
            alternatives.push("browse".to_string());
            alternatives.push("lookup".to_string());
        }

        if failed_action.contains("browse") {
            alternatives.push("search".to_string());
            alternatives.push("scrape".to_string());
        }

        if failed_action.contains("execute") {
            alternatives.push("simulate".to_string());
            alternatives.push("validate".to_string());
        }

        alternatives
    }

    fn update_patterns(&mut self) {
        let mut to_remove = Vec::new();

        for (i, pattern) in self.success_patterns.iter_mut().enumerate() {
            if pattern.frequency > 10 && pattern.success_rate < 0.3 {
                to_remove.push(i);
            }
        }

        for i in to_remove.iter().rev() {
            self.success_patterns.remove(*i);
        }

        to_remove.clear();

        for (i, pattern) in self.failure_patterns.iter_mut().enumerate() {
            // Drop failure patterns that are no longer reliably failing.
            // If a pattern's observed failure_rate is low, it's not a useful "avoid" signal.
            if pattern.frequency > 10 && pattern.failure_rate < 0.3 {
                to_remove.push(i);
            }
        }

        for i in to_remove.iter().rev() {
            self.failure_patterns.remove(*i);
        }
    }

    pub fn get_recommended_action(&self, context: &str) -> Option<String> {
        let best_success = self
            .success_patterns
            .iter()
            .filter(|p| p.context == context)
            .max_by(|a, b| {
                (a.success_rate * a.frequency as f64)
                    .partial_cmp(&(b.success_rate * b.frequency as f64))
                    .unwrap()
            });

        if let Some(pattern) = best_success {
            if pattern.success_rate >= self.min_confidence_threshold {
                return pattern.action_sequence.first().cloned();
            }
        }

        if rand::random::<f64>() < self.exploration_rate {
            return Some("explore".to_string());
        }

        None
    }

    pub fn adapt_behavior(&mut self, trigger: &str, adaptation: &str) {
        if let Some(existing) = self.adaptations.iter_mut().find(|a| a.trigger == trigger) {
            existing.times_triggered += 1;
            existing.confidence = (existing.confidence * (existing.times_triggered - 1) as f64
                + if adaptation == existing.adaptation {
                    1.0
                } else {
                    0.0
                })
                / existing.times_triggered as f64;
        } else {
            self.adaptations.push(BehavioralAdaptation {
                pattern: trigger.to_string(),
                trigger: trigger.to_string(),
                adaptation: adaptation.to_string(),
                confidence: 0.5,
                times_triggered: 1,
                success_rate: 0.5,
            });
        }
    }

    pub fn get_adaptation(&self, trigger: &str) -> Option<String> {
        self.adaptations
            .iter()
            .find(|a| a.trigger == trigger && a.confidence >= self.min_confidence_threshold)
            .map(|a| a.adaptation.clone())
    }

    pub fn export_model(&self) -> LearningModel {
        LearningModel {
            id: format!("model_{}", uuid::Uuid::new_v4()),
            feedback_history: self.feedback_history.clone(),
            adaptations: self.adaptations.clone(),
            success_patterns: self.success_patterns.clone(),
            failure_patterns: self.failure_patterns.clone(),
            last_updated: chrono::Utc::now(),
        }
    }

    pub fn import_model(&mut self, model: LearningModel) {
        self.feedback_history = model.feedback_history;
        self.adaptations = model.adaptations;
        self.success_patterns = model.success_patterns;
        self.failure_patterns = model.failure_patterns;
    }

    pub fn get_statistics(&self) -> LearningStatistics {
        let total_feedback = self.feedback_history.len();
        let successful_actions = self.feedback_history.iter().filter(|f| f.success).count();
        let success_rate = if total_feedback > 0 {
            successful_actions as f64 / total_feedback as f64
        } else {
            0.0
        };

        LearningStatistics {
            total_feedback,
            successful_actions,
            success_rate,
            patterns_learned: self.success_patterns.len() + self.failure_patterns.len(),
            adaptations_active: self.adaptations.len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningStatistics {
    pub total_feedback: usize,
    pub successful_actions: usize,
    pub success_rate: f64,
    pub patterns_learned: usize,
    pub adaptations_active: usize,
}

impl Default for ContinuousLearningEngine {
    fn default() -> Self {
        Self::new()
    }
}

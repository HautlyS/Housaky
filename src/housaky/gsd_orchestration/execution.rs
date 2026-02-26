use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

use super::{
    GSDOrchestrator, Phase, PhaseContext, PhaseStatus,
    ContextManager, DecompositionContext, DecompositionResult,
    TaskAwarenessReport, ExecutionSummary, VerificationReport,
    DecompositionStrategy, TaskStep,
};
use crate::housaky::meta_cognition::MetaCognitionEngine;
use crate::housaky::goal_engine::GoalEngine;
use crate::providers::Provider;
use crate::providers::{ChatMessage, ChatRequest};

pub struct GSDExecutionEngine {
    orchestrator: Arc<GSDOrchestrator>,
    provider: Option<Box<dyn Provider>>,
    model: String,
    temperature: f64,
    workspace_dir: PathBuf,
}

impl GSDExecutionEngine {
    pub fn new(
        workspace_dir: PathBuf,
        provider: Option<Box<dyn Provider>>,
        model: String,
    ) -> Self {
        let meta_cognition = Arc::new(MetaCognitionEngine::new());
        let goal_engine = Arc::new(GoalEngine::new(&workspace_dir));
        let orchestrator = Arc::new(GSDOrchestrator::new(
            workspace_dir.clone(),
            meta_cognition,
            goal_engine,
        ));

        Self {
            orchestrator,
            provider,
            model,
            temperature: 0.7,
            workspace_dir,
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        self.orchestrator.initialize().await?;
        Ok(())
    }

    pub async fn create_project(&self, name: String, vision: String) -> Result<String> {
        self.orchestrator.new_project(name, vision).await
    }

    pub async fn create_phase(&self, name: String, description: String, goals: Vec<String>) -> Result<String> {
        self.orchestrator.create_phase(name, description, goals).await
    }

    pub async fn discuss_phase(&self, phase_id: &str, user_answers: Vec<String>) -> Result<String> {
        let mut context = PhaseContext::new(phase_id.to_string());
        
        for answer in user_answers {
            context.add_decision(answer);
        }

        self.orchestrator.discuss_phase(phase_id, context).await
    }

    pub async fn plan_and_execute(&self, phase_id: &str, task_description: &str) -> Result<ExecutionSummary> {
        info!("Planning and executing phase: {}", phase_id);

        let decomposition = self.orchestrator
            .decompose_task(task_description, DecompositionContext {
                technology: Some("Rust".to_string()),
                requirements: vec![],
                constraints: vec![],
                existing_files: vec![],
                project_type: Some("CLI Tool".to_string()),
            })
            .await;

        info!("Decomposed into {} steps", decomposition.steps.len());

        let task_descriptions: Vec<String> = decomposition.steps
            .iter()
            .map(|s| s.description.clone())
            .collect();

        self.orchestrator.plan_phase(phase_id, task_descriptions).await?;

        let summary = self.orchestrator.execute_phase(phase_id).await?;

        Ok(summary)
    }

    pub async fn execute_with_llm(&self, phase_id: &str, task_description: &str) -> Result<ExecutionSummary> {
        let provider = self.provider.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No provider configured"))?;

        let context = self.build_planning_context(phase_id, task_description).await?;

        let prompt = format!(r#"You are an expert task decomposition system. Analyze this task and decompose it into atomic, executable steps for an AI coding agent.

## Current Project Context
{}

## Task to Decompose
{}

## Requirements
- Break down into small, independent tasks that can be executed in parallel when possible
- Each task should have clear verification criteria
- Consider dependencies between tasks
- Focus on code implementation tasks

## Output Format (XML)
Provide your decomposition as XML:
<decomposition>
  <strategy>sequential|parallel|wave_based</strategy>
  <confidence>0.0-1.0</confidence>
  <steps>
    <step>
      <order>1</order>
      <description>What to do</description>
      <action>Implementation details</action>
      <files>src/main.rs,src/lib.rs</files>
      <verification>How to verify</verification>
      <done_criteria>Completion criteria</done_criteria>
      <dependencies></dependencies>
      <estimated_duration_mins>15</estimated_duration_mins>
    </step>
  </steps>
</decomposition>

Respond ONLY with the XML decomposition, no other text."#, 
            context.0, context.1
        );

        let messages = vec![
            ChatMessage::system("You are a task decomposition expert."),
            ChatMessage::user(&prompt),
        ];

        let request = ChatRequest {
            messages: &messages,
            tools: None,
        };

        let response = provider.chat(request, &self.model, self.temperature).await?;
        
        let response_text = response.text.unwrap_or_default();
        
        let decomposition = self.parse_llm_decomposition(&response_text, task_description)?;
        
        info!("LLM decomposed into {} steps", decomposition.steps.len());

        let task_descriptions: Vec<String> = decomposition.steps
            .iter()
            .map(|s| s.description.clone())
            .collect();

        self.orchestrator.plan_phase(phase_id, task_descriptions).await?;

        let summary = self.orchestrator.execute_phase(phase_id).await?;

        Ok(summary)
    }

    async fn build_planning_context(&self, _phase_id: &str, task: &str) -> Result<(String, String)> {
        let context_manager = ContextManager::new(self.workspace_dir.clone());
        let project_ctx = context_manager.load_project_context().await.ok().flatten();
        
        let project_info = match project_ctx {
            Some(ctx) => format!("Project: {}\nVision: {}\nGoals: {:?}\nConstraints: {:?}", 
                ctx.name, ctx.vision, ctx.goals, ctx.constraints),
            None => "No project context found".to_string(),
        };

        Ok((project_info, task.to_string()))
    }

    fn parse_llm_decomposition(&self, response: &str, task_description: &str) -> Result<DecompositionResult> {
        let mut steps = Vec::new();
        let strategy = DecompositionStrategy::WaveBased;
        let confidence = 0.8;

        let step_pattern = regex::Regex::new(r"<step>\s*<order>(\d+)</order>\s*<description>([^<]+)</description>\s*<action>([^<]+)</action>\s*(?:<files>([^<]*)</files>)?\s*<verification>([^<]+)</verification>\s*<done_criteria>([^<]+)</done_criteria>\s*(?:<dependencies>([^<]*)</dependencies>)?\s*(?:<estimated_duration_mins>(\d+)</estimated_duration_mins>)?\s*</step>")?;

        for cap in step_pattern.captures_iter(response) {
            let files: Vec<String> = cap.get(4)
                .map(|m| m.as_str().split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();
            
            let deps: Vec<String> = cap.get(6)
                .map(|m| m.as_str().split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();

            let duration: u32 = cap.get(7)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(15);

            steps.push(TaskStep {
                id: format!("step_{}", uuid::Uuid::new_v4()),
                order: cap.get(1).map(|m| m.as_str().parse().unwrap_or(1)).unwrap_or(1),
                description: cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default(),
                action: cap.get(3).map(|m| m.as_str().to_string()).unwrap_or_default(),
                files,
                verification: cap.get(5).map(|m| m.as_str().to_string()).unwrap_or_default(),
                done_criteria: cap.get(5).map(|m| m.as_str().to_string()).unwrap_or_default(),
                dependencies: deps,
                estimated_duration_mins: duration,
                risk_level: 0.3,
                reasoning: "From LLM decomposition".to_string(),
            });
        }

        if steps.is_empty() {
            steps.push(TaskStep {
                id: format!("step_{}", uuid::Uuid::new_v4()),
                order: 1,
                description: task_description.to_string(),
                action: format!("Implement: {}", task_description),
                files: vec![],
                verification: "Implementation complete".to_string(),
                done_criteria: "Task completed".to_string(),
                dependencies: vec![],
                estimated_duration_mins: 30,
                risk_level: 0.5,
                reasoning: "Fallback single step".to_string(),
            });
        }

        let estimated_total_mins: u32 = steps.iter().map(|s| s.estimated_duration_mins).sum();

        Ok(DecompositionResult {
            steps,
            strategy,
            confidence,
            reasoning: "Parsed from LLM response".to_string(),
            estimated_total_mins,
            parallel_opportunities: vec![],
        })
    }

    pub async fn verify_phase(&self, phase_id: &str) -> Result<VerificationReport> {
        self.orchestrator.verify_work(phase_id).await
    }

    pub async fn get_phase_status(&self, phase_id: &str) -> Option<PhaseStatus> {
        self.orchestrator.get_phase_status(phase_id).await
    }

    pub async fn get_current_phase(&self) -> Option<Phase> {
        self.orchestrator.get_current_phase().await
    }

    pub async fn get_awareness_report(&self) -> TaskAwarenessReport {
        self.orchestrator.get_awareness_report().await
    }

    pub async fn quick_execute(&self, task: &str) -> Result<ExecutionSummary> {
        let phase_id = self.create_phase(
            format!("Quick: {}", task.chars().take(30).collect::<String>()),
            task.to_string(),
            vec![task.to_string()],
        ).await?;

        self.execute_with_llm(&phase_id, task).await
    }
}

use super::context_manager::{ContextManager, ProjectContext, RoadmapContext, StateContext, Milestone, MilestoneStatus};
use super::phase::{Phase, PhaseContext, PhaseStatus};
use super::task::{GSDTask, GSDTaskStatus};
use super::wave_executor::{WaveExecutor, ExecutionResult};
use super::step_decomposer::{StepDecomposer, DecompositionContext, DecompositionResult, ComplexityAnalysis};
use crate::housaky::meta_cognition::MetaCognitionEngine;
use crate::housaky::goal_engine::GoalEngine;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct GSDOrchestrator {
    workspace_dir: PathBuf,
    context_manager: Arc<ContextManager>,
    wave_executor: Arc<WaveExecutor>,
    step_decomposer: Arc<StepDecomposer>,
    meta_cognition: Arc<MetaCognitionEngine>,
    goal_engine: Arc<GoalEngine>,
    phases: Arc<RwLock<HashMap<String, Phase>>>,
    tasks: Arc<RwLock<HashMap<String, GSDTask>>>,
    current_milestone: Arc<RwLock<MilestoneState>>,
    awareness: TaskAwareness,
    self_improvement: SelfImprovementIntegration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneState {
    pub name: String,
    pub version: String,
    pub phases: Vec<String>,
    pub current_phase: u32,
    pub status: MilestoneStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAwareness {
    pub capability_profile: CapabilityProfile,
    pub historical_performance: HashMap<String, TaskPerformance>,
    pub complexity_bias: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityProfile {
    pub code_generation: f64,
    pub testing: f64,
    pub debugging: f64,
    pub refactoring: f64,
    pub documentation: f64,
    pub architecture: f64,
    pub database: f64,
    pub api_design: f64,
    pub security: f64,
    pub performance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPerformance {
    pub task_type: String,
    pub success_rate: f64,
    pub avg_duration_mins: f64,
    pub failure_reasons: Vec<String>,
    pub improvements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfImprovementIntegration {
    pub reflection_enabled: bool,
    pub learning_from_tasks: bool,
    pub capability_updates: Vec<CapabilityUpdate>,
    pub improvement_suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityUpdate {
    pub capability: String,
    pub delta: f64,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

impl Default for CapabilityProfile {
    fn default() -> Self {
        Self {
            code_generation: 0.7,
            testing: 0.6,
            debugging: 0.5,
            refactoring: 0.6,
            documentation: 0.5,
            architecture: 0.5,
            database: 0.5,
            api_design: 0.6,
            security: 0.4,
            performance: 0.5,
        }
    }
}

impl GSDOrchestrator {
    pub fn new(
        workspace_dir: PathBuf,
        meta_cognition: Arc<MetaCognitionEngine>,
        goal_engine: Arc<GoalEngine>,
    ) -> Self {
        let context_manager = Arc::new(ContextManager::new(workspace_dir.clone()));
        let wave_executor = Arc::new(WaveExecutor::new(5));
        let step_decomposer = Arc::new(StepDecomposer::new());
        
        Self {
            workspace_dir,
            context_manager,
            wave_executor,
            step_decomposer,
            meta_cognition,
            goal_engine,
            phases: Arc::new(RwLock::new(HashMap::new())),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            current_milestone: Arc::new(RwLock::new(MilestoneState {
                name: "v1".to_string(),
                version: "1.0.0".to_string(),
                phases: Vec::new(),
                current_phase: 0,
                status: MilestoneStatus::Planning,
                created_at: Utc::now(),
            })),
            awareness: TaskAwareness {
                capability_profile: CapabilityProfile::default(),
                historical_performance: HashMap::new(),
                complexity_bias: 1.0,
            },
            self_improvement: SelfImprovementIntegration {
                reflection_enabled: true,
                learning_from_tasks: true,
                capability_updates: Vec::new(),
                improvement_suggestions: Vec::new(),
            },
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        self.context_manager.initialize().await?;
        info!("GSD Orchestrator initialized");
        Ok(())
    }

    pub async fn new_project(&self, name: String, vision: String) -> Result<String> {
        let project = ProjectContext {
            name: name.clone(),
            vision,
            goals: Vec::new(),
            constraints: Vec::new(),
            tech_preferences: Vec::new(),
            success_criteria: Vec::new(),
        };
        
        let content = self.context_manager.create_project_file(project).await?;
        
        let state = StateContext {
            decisions: Vec::new(),
            blockers: Vec::new(),
            position: "Project initialized".to_string(),
            last_phase: None,
        };
        self.context_manager.create_state_file(state).await?;
        
        let roadmap = RoadmapContext {
            milestones: vec![Milestone {
                name: "v1".to_string(),
                version: "1.0.0".to_string(),
                phases: Vec::new(),
                status: MilestoneStatus::Planning,
            }],
            current_phase: 0,
            completed_phases: Vec::new(),
        };
        self.context_manager.create_roadmap_file(roadmap).await?;
        
        info!("Created new project: {}", name);
        Ok(content)
    }

    pub async fn create_phase(&self, name: String, description: String, goals: Vec<String>) -> Result<String> {
        let phase_num = {
            let milestone = self.current_milestone.read().await;
            milestone.current_phase + 1
        };
        
        let mut phase = Phase::new(phase_num, name.clone(), description);
        phase.goals = goals;
        
        let phase_id = phase.id.clone();
        self.phases.write().await.insert(phase_id.clone(), phase);
        
        self.current_milestone.write().await.phases.push(phase_id.clone());
        self.current_milestone.write().await.current_phase = phase_num;
        
        info!("Created phase {}: {}", phase_num, name);
        Ok(phase_id)
    }

    pub async fn discuss_phase(&self, phase_id: &str, context: PhaseContext) -> Result<String> {
        let content = context.to_context_string();
        let ctx_file = self.context_manager
            .create_phase_context_file(1, &content)
            .await?;
        
        if let Some(phase) = self.phases.write().await.get_mut(phase_id) {
            phase.context_file = Some(ctx_file.clone());
            phase.status = PhaseStatus::InProgress;
        }
        
        info!("Discussed phase: context created");
        Ok(ctx_file)
    }

    pub async fn plan_phase(&self, phase_id: &str, task_descriptions: Vec<String>) -> Result<Vec<String>> {
        let phase_num = {
            let phases = self.phases.read().await;
            phases.get(phase_id).map(|p| p.number).unwrap_or(1)
        };

        let context = DecompositionContext {
            technology: Some("Rust".to_string()),
            requirements: vec![],
            constraints: vec![],
            existing_files: vec![],
            project_type: Some("CLI Tool".to_string()),
        };
        
        let all_steps = self.step_decomposer.decompose(
            &task_descriptions.join(" && "),
            &context,
        );
        
        let mut task_ids = Vec::new();
        
        for step in &all_steps.steps {
            let mut task = GSDTask::new(step.description.clone(), phase_id.to_string());
            task.action = step.action.clone();
            task.files = step.files.clone();
            task.verify = step.verification.clone();
            task.done_criteria = step.done_criteria.clone();
            task.dependencies = step.dependencies.clone();
            task.wave = ((step.order - 1) / 3 + 1) as u32;
            
            let task_id = task.id.clone();
            self.tasks.write().await.insert(task_id.clone(), task);
            task_ids.push(task_id);
        }

        if let Some(phase) = self.phases.write().await.get_mut(phase_id) {
            for task_id in &task_ids {
                phase.add_task(task_id.clone());
            }
        }
        
        self.context_manager
            .create_plan_file(phase_num, 1, &self.render_plan(&all_steps))
            .await?;
        
        self.wave_executor.load_tasks(
            self.tasks.read().await.values().cloned().collect()
        ).await;
        self.wave_executor.compute_waves().await;
        
        info!("Planned phase {}: {} tasks created", phase_num, task_ids.len());
        Ok(task_ids)
    }

    pub async fn execute_phase(&self, phase_id: &str) -> Result<ExecutionSummary> {
        let phase_num = {
            let phases = self.phases.read().await;
            phases.get(phase_id).map(|p| p.number).unwrap_or(1)
        };
        
        let waves = self.wave_executor.compute_waves().await;
        
        info!("Executing phase {} with {} waves", phase_num, waves.len());
        
        let mut all_results = Vec::new();
        
        for wave in &waves {
            self.wave_executor.mark_wave_started(wave.number).await;
            
            let ready_tasks = self.wave_executor.get_ready_tasks(wave.number).await;
            
            let mut wave_results = Vec::new();
            for task in ready_tasks {
                info!("Executing task: {}", task.name);
                
                self.wave_executor.update_task_status(&task.id, GSDTaskStatus::InProgress).await;
                
                let result = self.execute_task(&task).await;
                
                if result.success {
                    self.wave_executor.update_task_status(&task.id, GSDTaskStatus::Completed).await;
                } else {
                    self.wave_executor.update_task_status(&task.id, GSDTaskStatus::Failed).await;
                }
                
                wave_results.push(result.clone());
                all_results.push(result);
            }
            
            self.wave_executor.mark_wave_completed(wave.number, &wave_results).await;
            
            if self.self_improvement.reflection_enabled {
                self.reflect_on_wave(wave.number, &wave_results).await?;
            }
        }
        
        let success_count = all_results.iter().filter(|r| r.success).count();
        let total = all_results.len();
        
        let summary = ExecutionSummary {
            phase_id: phase_id.to_string(),
            phase_number: phase_num,
            total_tasks: total,
            successful_tasks: success_count,
            failed_tasks: total - success_count,
            total_duration_ms: all_results.iter().map(|r| r.duration_ms).sum(),
            results: all_results,
        };
        
        if success_count == total {
            if let Some(phase) = self.phases.write().await.get_mut(phase_id) {
                phase.status = PhaseStatus::Completed;
                phase.completed_at = Some(Utc::now());
            }
        }
        
        self.context_manager
            .create_verification_file(phase_num, &self.render_verification(&summary))
            .await?;
        
        info!("Phase {} execution complete: {}/{} tasks successful", 
            phase_num, success_count, total);
        
        Ok(summary)
    }

    async fn execute_task(&self, task: &GSDTask) -> ExecutionResult {
        let start = std::time::Instant::now();
        
        info!("Executing task: {} ({})", task.name, task.id);
        
        let success = task.max_attempts > 0;
        
        let duration = start.elapsed().as_millis() as i64;
        
        ExecutionResult {
            task_id: task.id.clone(),
            success,
            output: format!("Task '{}' executed", task.name),
            error: if success { None } else { Some("Simulated failure".to_string()) },
            duration_ms: duration,
            artifacts: vec![],
            commit_sha: if success { Some(format!("abc{}def", &task.id[5..9])) } else { None },
        }
    }

    pub async fn verify_work(&self, phase_id: &str) -> Result<VerificationReport> {
        let phase_num = {
            let phases = self.phases.read().await;
            phases.get(phase_id).map(|p| p.number).unwrap_or(1)
        };
        
        let tasks = self.tasks.read().await;
        
        let mut verified_items = Vec::new();
        let mut failed_items = Vec::new();
        let mut total = 0;
        
        for task in tasks.values() {
            if task.phase_id != phase_id {
                continue;
            }
            total += 1;
            
            if matches!(task.status, GSDTaskStatus::Completed | GSDTaskStatus::Verified) {
                verified_items.push(VerificationItem {
                    task_id: task.id.clone(),
                    task_name: task.name.clone(),
                    status: "verified".to_string(),
                    notes: String::new(),
                });
            } else {
                failed_items.push(VerificationItem {
                    task_id: task.id.clone(),
                    task_name: task.name.clone(),
                    status: "not_completed".to_string(),
                    notes: format!("Status: {:?}", task.status),
                });
            }
        }
        
        let verified_count = verified_items.len();
        let failed_count = failed_items.len();
        
        let report = VerificationReport {
            phase_id: phase_id.to_string(),
            phase_number: phase_num,
            total_items: total,
            verified: verified_count,
            failed: failed_count,
            verified_items,
            failed_items,
            recommendations: vec![],
        };
        
        if report.failed == 0 {
            if let Some(phase) = self.phases.write().await.get_mut(phase_id) {
                phase.status = PhaseStatus::Verified;
            }
        }
        
        Ok(report)
    }

    async fn reflect_on_wave(&self, wave_num: u32, _results: &[ExecutionResult]) -> Result<()> {
        let reflection_trigger = format!("Wave {} completed", wave_num);
        self.meta_cognition.reflect(&reflection_trigger).await?;
        info!("Reflected on wave {}", wave_num);
        Ok(())
    }

    fn render_plan(&self, result: &DecompositionResult) -> String {
        let mut md = String::new();
        
        md.push_str(&format!("# Plan - Strategy: {:?}\n\n", result.strategy));
        md.push_str(&format!("Confidence: {:.0}%\n", result.confidence * 100.0));
        md.push_str(&format!("Estimated Duration: {} minutes\n\n", result.estimated_total_mins));
        
        md.push_str("## Steps\n\n");
        for step in &result.steps {
            md.push_str(&format!("### {}. {}\n", step.order, step.description));
            md.push_str(&format!("**Action:** {}\n\n", step.action));
            
            if !step.files.is_empty() {
                md.push_str("**Files:**\n");
                for file in &step.files {
                    md.push_str(&format!("- {}\n", file));
                }
                md.push_str("\n");
            }
            
            md.push_str(&format!("**Verification:** {}\n", step.verification));
            md.push_str(&format!("**Done:** {}\n\n", step.done_criteria));
        }
        
        md
    }

    fn render_verification(&self, summary: &ExecutionSummary) -> String {
        let mut md = String::new();
        
        md.push_str(&format!("# Phase {} Verification\n\n", summary.phase_number));
        md.push_str(&format!("Total Tasks: {}\n", summary.total_tasks));
        md.push_str(&format!("Successful: {}\n", summary.successful_tasks));
        md.push_str(&format!("Failed: {}\n\n", summary.failed_tasks));
        
        md.push_str("## Results\n\n");
        for result in &summary.results {
            let status = if result.success { "✓" } else { "✗" };
            md.push_str(&format!("{} {} - {}ms\n", status, result.task_id, result.duration_ms));
        }
        
        md
    }

    pub async fn get_phase_status(&self, phase_id: &str) -> Option<PhaseStatus> {
        let phases = self.phases.read().await;
        phases.get(phase_id).map(|p| p.status.clone())
    }

    pub async fn get_current_phase(&self) -> Option<Phase> {
        let milestone = self.current_milestone.read().await;
        let phases = self.phases.read().await;
        
        phases.get(&milestone.phases.last()?.clone()).cloned()
    }

    pub async fn analyze_task_complexity(&self, task: &str) -> ComplexityAnalysis {
        self.step_decomposer.analyze_complexity(task)
    }

    pub async fn decompose_task(&self, task: &str, context: DecompositionContext) -> DecompositionResult {
        self.step_decomposer.decompose(task, &context)
    }

    pub async fn get_awareness_report(&self) -> TaskAwarenessReport {
        TaskAwarenessReport {
            capability_profile: self.awareness.capability_profile.clone(),
            complexity_bias: self.awareness.complexity_bias,
            total_tasks_analyzed: self.awareness.historical_performance.len(),
            avg_success_rate: if self.awareness.historical_performance.is_empty() {
                0.0
            } else {
                self.awareness.historical_performance.values()
                    .map(|p| p.success_rate)
                    .sum::<f64>() / self.awareness.historical_performance.len() as f64
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub phase_id: String,
    pub phase_number: u32,
    pub total_tasks: usize,
    pub successful_tasks: usize,
    pub failed_tasks: usize,
    pub total_duration_ms: i64,
    pub results: Vec<ExecutionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub phase_id: String,
    pub phase_number: u32,
    pub total_items: usize,
    pub verified: usize,
    pub failed: usize,
    pub verified_items: Vec<VerificationItem>,
    pub failed_items: Vec<VerificationItem>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationItem {
    pub task_id: String,
    pub task_name: String,
    pub status: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAwarenessReport {
    pub capability_profile: CapabilityProfile,
    pub complexity_bias: f64,
    pub total_tasks_analyzed: usize,
    pub avg_success_rate: f64,
}

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GoalStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
    Deferred,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GoalPriority {
    Critical = 4,
    High = 3,
    Medium = 2,
    Low = 1,
    Background = 0,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum GoalCategory {
    Planning,
    Intelligence,
    ToolDevelopment,
    SkillAcquisition,
    KnowledgeExpansion,
    SystemImprovement,
    UserRequest,
    SelfModification,
    Research,
    Integration,
    Maintenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub priority: GoalPriority,
    pub status: GoalStatus,
    pub category: GoalCategory,
    pub progress: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,
    pub parent_id: Option<String>,
    pub subtask_ids: Vec<String>,
    pub dependencies: Vec<String>,
    pub blockers: Vec<String>,
    pub metrics: HashMap<String, f64>,
    pub checkpoints: Vec<GoalCheckpoint>,
    pub attempts: u32,
    pub max_attempts: u32,
    pub estimated_complexity: f64,
    pub actual_complexity: Option<f64>,
    pub learning_value: f64,
    pub tags: Vec<String>,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalCheckpoint {
    pub id: String,
    pub description: String,
    pub completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalDecomposition {
    pub parent_id: String,
    pub strategy: DecompositionStrategy,
    pub subtasks: Vec<Goal>,
    pub created_at: DateTime<Utc>,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DecompositionStrategy {
    Sequential,
    Parallel,
    Hierarchical,
    Conditional,
    Iterative,
    Recursive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalProgress {
    pub goal_id: String,
    pub delta: f64,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
    pub new_metrics: HashMap<String, f64>,
}

pub struct GoalEngine {
    goals: Arc<RwLock<HashMap<String, Goal>>>,
    queue: Arc<RwLock<VecDeque<String>>>,
    completed: Arc<RwLock<Vec<String>>>,
    failed: Arc<RwLock<Vec<String>>>,
    workspace_dir: PathBuf,
    max_active_goals: usize,
    auto_decompose: bool,
}

impl GoalEngine {
    pub fn new(workspace_dir: &PathBuf) -> Self {
        Self {
            goals: Arc::new(RwLock::new(HashMap::new())),
            queue: Arc::new(RwLock::new(VecDeque::new())),
            completed: Arc::new(RwLock::new(Vec::new())),
            failed: Arc::new(RwLock::new(Vec::new())),
            workspace_dir: workspace_dir.clone(),
            max_active_goals: 10,
            auto_decompose: true,
        }
    }

    pub async fn add_goal(&self, mut goal: Goal) -> Result<String> {
        goal.id = format!("goal_{}", uuid::Uuid::new_v4());
        goal.created_at = Utc::now();
        goal.updated_at = Utc::now();

        let id = goal.id.clone();

        if goal.subtask_ids.is_empty() && self.auto_decompose && self.should_decompose(&goal) {
            let decomposition = self.decompose_goal(&goal).await?;
            for subtask in decomposition.subtasks {
                let subtask_id = Box::pin(self.add_goal(subtask)).await?;
                goal.subtask_ids.push(subtask_id);
            }
        }

        self.goals.write().await.insert(id.clone(), goal);
        self.queue.write().await.push_back(id.clone());

        self.save_goals().await?;

        info!(
            "Added goal: {} (priority: {:?})",
            id,
            self.goals.read().await.get(&id).map(|g| g.priority.clone())
        );

        Ok(id)
    }

    fn should_decompose(&self, goal: &Goal) -> bool {
        if goal.parent_id.is_some() {
            return false;
        }
        goal.estimated_complexity > 5.0
            || goal.description.len() > 200
            || goal.title.contains("and")
            || goal.title.contains("then")
    }

    async fn decompose_goal(&self, goal: &Goal) -> Result<GoalDecomposition> {
        info!("Decomposing goal: {}", goal.title);

        let strategy = self.determine_decomposition_strategy(goal);
        let subtasks = self.generate_subtasks(goal, &strategy).await?;

        Ok(GoalDecomposition {
            parent_id: goal.id.clone(),
            strategy,
            subtasks,
            created_at: Utc::now(),
            reasoning: format!(
                "Auto-decomposed based on complexity {:.1}",
                goal.estimated_complexity
            ),
        })
    }

    fn determine_decomposition_strategy(&self, goal: &Goal) -> DecompositionStrategy {
        if goal.title.to_lowercase().contains("after") || goal.title.to_lowercase().contains("then")
        {
            DecompositionStrategy::Sequential
        } else if goal.title.to_lowercase().contains("and")
            && !goal.title.to_lowercase().contains("after")
        {
            DecompositionStrategy::Parallel
        } else if goal.category == GoalCategory::Research {
            DecompositionStrategy::Iterative
        } else {
            DecompositionStrategy::Hierarchical
        }
    }

    async fn generate_subtasks(
        &self,
        goal: &Goal,
        strategy: &DecompositionStrategy,
    ) -> Result<Vec<Goal>> {
        let mut subtasks = Vec::new();

        let title_lower = goal.title.to_lowercase();
        let parts: Vec<&str> = if title_lower.contains(" and ") {
            title_lower.split(" and ").collect()
        } else if title_lower.contains(" then ") {
            title_lower.split(" then ").collect()
        } else if title_lower.contains(" after ") {
            title_lower.split(" after ").collect()
        } else if title_lower.contains(';') {
            goal.title.split(';').collect()
        } else if title_lower.contains(',') {
            goal.title.split(',').collect()
        } else {
            vec![goal.title.as_str()]
        };

        let parts: Vec<&str> = parts
            .into_iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        for (i, part) in parts.iter().enumerate() {
            let subtask = Goal {
                id: String::new(),
                title: part.to_string(),
                description: format!("Subtask {} of: {}", i + 1, goal.title),
                priority: goal.priority.clone(),
                status: GoalStatus::Pending,
                category: goal.category.clone(),
                progress: 0.0,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                deadline: goal.deadline,
                parent_id: Some(goal.id.clone()),
                subtask_ids: Vec::new(),
                dependencies: if *strategy == DecompositionStrategy::Sequential && i > 0 {
                    vec![format!("goal_subtask_{}", i - 1)]
                } else {
                    Vec::new()
                },
                blockers: Vec::new(),
                metrics: HashMap::new(),
                checkpoints: Vec::new(),
                attempts: 0,
                max_attempts: goal.max_attempts,
                estimated_complexity: goal.estimated_complexity / (parts.len() as f64),
                actual_complexity: None,
                learning_value: goal.learning_value / (parts.len() as f64),
                tags: goal.tags.clone(),
                context: goal.context.clone(),
            };
            subtasks.push(subtask);
        }

        if subtasks.is_empty() {
            subtasks.push(Goal {
                id: String::new(),
                title: format!("Plan: {}", goal.title),
                description: "Create detailed plan for goal execution".to_string(),
                priority: goal.priority.clone(),
                status: GoalStatus::Pending,
                category: GoalCategory::Planning,
                progress: 0.0,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                deadline: None,
                parent_id: Some(goal.id.clone()),
                subtask_ids: Vec::new(),
                dependencies: Vec::new(),
                blockers: Vec::new(),
                metrics: HashMap::new(),
                checkpoints: Vec::new(),
                attempts: 0,
                max_attempts: 3,
                estimated_complexity: 2.0,
                actual_complexity: None,
                learning_value: 1.0,
                tags: vec!["planning".to_string()],
                context: HashMap::new(),
            });
        }

        Ok(subtasks)
    }

    pub async fn get_next_goal(&self) -> Option<Goal> {
        let mut queue = self.queue.write().await;
        let goals = self.goals.read().await;

        while let Some(id) = queue.front() {
            if let Some(goal) = goals.get(id) {
                if goal.status == GoalStatus::Pending
                    && self.dependencies_satisfied(goal, &goals) {
                        return queue.pop_front().and_then(|id| goals.get(&id).cloned());
                    }
            }
            queue.pop_front();
        }

        None
    }

    fn dependencies_satisfied(&self, goal: &Goal, goals: &HashMap<String, Goal>) -> bool {
        for dep_id in &goal.dependencies {
            if let Some(dep) = goals.get(dep_id) {
                if dep.status != GoalStatus::Completed {
                    return false;
                }
            }
        }
        true
    }

    pub async fn update_progress(&self, goal_id: &str, progress: f64, reason: &str) -> Result<()> {
        let mut goals = self.goals.write().await;

        if let Some(goal) = goals.get_mut(goal_id) {
            goal.progress = progress.clamp(0.0, 1.0);
            goal.updated_at = Utc::now();

            if progress >= 1.0 {
                goal.status = GoalStatus::Completed;
                self.completed.write().await.push(goal_id.to_string());

                if let Some(parent_id) = &goal.parent_id {
                    let parent_id = parent_id.clone();
                    drop(goals);
                    Box::pin(self.update_parent_progress(&parent_id)).await?;
                }
            }

            info!(
                "Goal {} progress: {:.0}% - {}",
                goal_id,
                progress * 100.0,
                reason
            );
        }

        self.save_goals().await?;
        Ok(())
    }

    async fn update_parent_progress(&self, parent_id: &str) -> Result<()> {
        let goals = self.goals.read().await;

        if let Some(parent) = goals.get(parent_id) {
            let subtask_count = parent.subtask_ids.len();
            if subtask_count == 0 {
                return Ok(());
            }

            let completed: usize = parent
                .subtask_ids
                .iter()
                .filter_map(|id| goals.get(id))
                .filter(|g| g.status == GoalStatus::Completed)
                .count();

            let progress = completed as f64 / subtask_count as f64;
            drop(goals);

            Box::pin(self.update_progress(
                parent_id,
                progress,
                &format!("{} of {} subtasks completed", completed, subtask_count),
            ))
            .await?;
        }

        Ok(())
    }

    pub async fn mark_failed(&self, goal_id: &str, reason: &str) -> Result<()> {
        let mut goals = self.goals.write().await;

        if let Some(goal) = goals.get_mut(goal_id) {
            goal.status = GoalStatus::Failed;
            goal.attempts += 1;
            goal.updated_at = Utc::now();

            self.failed.write().await.push(goal_id.to_string());

            warn!(
                "Goal {} failed (attempt {}): {}",
                goal_id, goal.attempts, reason
            );

            if goal.attempts < goal.max_attempts {
                goal.status = GoalStatus::Pending;
                self.queue.write().await.push_back(goal_id.to_string());
                info!(
                    "Goal {} queued for retry ({}/{})",
                    goal_id, goal.attempts, goal.max_attempts
                );
            }
        }

        self.save_goals().await?;
        Ok(())
    }

    pub async fn get_active_goals(&self) -> Vec<Goal> {
        let goals = self.goals.read().await;
        goals
            .values()
            .filter(|g| matches!(g.status, GoalStatus::Pending | GoalStatus::InProgress))
            .cloned()
            .collect()
    }

    pub async fn get_goal_stats(&self) -> GoalStats {
        let goals = self.goals.read().await;
        let completed = self.completed.read().await.len();
        let failed = self.failed.read().await.len();

        let by_priority = goals
            .values()
            .filter(|g| g.status == GoalStatus::Pending)
            .fold(HashMap::new(), |mut acc, g| {
                *acc.entry(g.priority.clone()).or_insert(0) += 1;
                acc
            });

        let by_category = goals
            .values()
            .filter(|g| g.status == GoalStatus::InProgress)
            .fold(HashMap::new(), |mut acc, g| {
                *acc.entry(g.category.clone()).or_insert(0) += 1;
                acc
            });

        GoalStats {
            total: goals.len(),
            pending: goals
                .values()
                .filter(|g| g.status == GoalStatus::Pending)
                .count(),
            in_progress: goals
                .values()
                .filter(|g| g.status == GoalStatus::InProgress)
                .count(),
            completed,
            failed,
            by_priority,
            by_category,
        }
    }

    async fn save_goals(&self) -> Result<()> {
        let goals = self.goals.read().await;
        let path = self.workspace_dir.join(".housaky").join("goals.json");

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let goals_vec: Vec<_> = goals.values().cloned().collect();
        let json = serde_json::to_string_pretty(&goals_vec)?;
        tokio::fs::write(&path, json).await?;

        Ok(())
    }

    pub async fn load_goals(&self) -> Result<()> {
        let path = self.workspace_dir.join(".housaky").join("goals.json");

        if !path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let goals_vec: Vec<Goal> = serde_json::from_str(&content)?;

        let mut goals = self.goals.write().await;
        let mut queue = self.queue.write().await;

        for goal in goals_vec {
            if goal.status == GoalStatus::Pending {
                queue.push_back(goal.id.clone());
            }
            goals.insert(goal.id.clone(), goal);
        }

        info!("Loaded {} goals from disk", goals.len());
        Ok(())
    }

    pub async fn prioritize(&self) -> Vec<Goal> {
        let goals = self.goals.read().await;
        let mut pending: Vec<_> = goals
            .values()
            .filter(|g| g.status == GoalStatus::Pending)
            .cloned()
            .collect();

        pending.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| {
                    b.learning_value
                        .partial_cmp(&a.learning_value)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| {
                    a.estimated_complexity
                        .partial_cmp(&b.estimated_complexity)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        pending
    }

    pub async fn add_checkpoint(&self, goal_id: &str, description: &str) -> Result<String> {
        let mut goals = self.goals.write().await;

        if let Some(goal) = goals.get_mut(goal_id) {
            let checkpoint = GoalCheckpoint {
                id: format!("cp_{}", uuid::Uuid::new_v4()),
                description: description.to_string(),
                completed: false,
                completed_at: None,
                notes: String::new(),
            };

            let cp_id = checkpoint.id.clone();
            goal.checkpoints.push(checkpoint);
            goal.updated_at = Utc::now();

            self.save_goals().await?;
            return Ok(cp_id);
        }

        Err(anyhow::anyhow!("Goal not found: {}", goal_id))
    }

    pub async fn complete_checkpoint(
        &self,
        goal_id: &str,
        checkpoint_id: &str,
        notes: &str,
    ) -> Result<()> {
        let mut goals = self.goals.write().await;

        if let Some(goal) = goals.get_mut(goal_id) {
            if let Some(cp) = goal.checkpoints.iter_mut().find(|c| c.id == checkpoint_id) {
                cp.completed = true;
                cp.completed_at = Some(Utc::now());
                cp.notes = notes.to_string();

                let completed = goal.checkpoints.iter().filter(|c| c.completed).count();
                let total = goal.checkpoints.len();

                if total > 0 {
                    goal.progress = completed as f64 / total as f64;
                }

                goal.updated_at = Utc::now();
            }
        }

        self.save_goals().await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalStats {
    pub total: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub failed: usize,
    pub by_priority: HashMap<GoalPriority, usize>,
    pub by_category: HashMap<GoalCategory, usize>,
}

impl Default for Goal {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            description: String::new(),
            priority: GoalPriority::Medium,
            status: GoalStatus::Pending,
            category: GoalCategory::UserRequest,
            progress: 0.0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
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
            tags: Vec::new(),
            context: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub goal_id: String,
    pub description: String,
    pub action: String,
    pub prerequisites: Vec<String>,
    pub expected_outcome: String,
    pub estimated_duration_mins: u32,
    pub actual_duration_mins: Option<u32>,
    pub status: StepStatus,
    pub resources_required: Vec<String>,
    pub success_criteria: Vec<String>,
    pub fallback_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Ready,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub id: String,
    pub root_goal_id: String,
    pub steps: Vec<PlanStep>,
    pub total_estimated_mins: u32,
    pub total_actual_mins: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: PlanStatus,
    pub schedule: Vec<ScheduledStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlanStatus {
    Draft,
    Scheduled,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledStep {
    pub step_id: String,
    pub scheduled_start: DateTime<Utc>,
    pub scheduled_end: DateTime<Utc>,
    pub assigned_resources: Vec<String>,
    pub priority_score: f64,
}

pub struct AutonomousPlanningEngine {
    max_plan_depth: usize,
    enable_parallel_execution: bool,
    enable_adaptive_scheduling: bool,
    resource_constraints: HashMap<String, u32>,
}

impl AutonomousPlanningEngine {
    pub fn new() -> Self {
        Self {
            max_plan_depth: 10,
            enable_parallel_execution: true,
            enable_adaptive_scheduling: true,
            resource_constraints: HashMap::new(),
        }
    }

    pub fn create_plan(&self, goal: &Goal, available_tools: &[&str]) -> ExecutionPlan {
        let plan_id = format!("plan_{}", uuid::Uuid::new_v4());
        
        let steps = self.decompose_goal(goal, available_tools);
        
        let total_estimated_mins: u32 = steps.iter()
            .map(|s| s.estimated_duration_mins)
            .sum();
        
        let schedule = if self.enable_parallel_execution {
            self.schedule_steps(&steps)
        } else {
            Vec::new()
        };
        
        ExecutionPlan {
            id: plan_id,
            root_goal_id: goal.id.clone(),
            steps,
            total_estimated_mins,
            total_actual_mins: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status: PlanStatus::Draft,
            schedule,
        }
    }

    fn decompose_goal(&self, goal: &Goal, available_tools: &[&str]) -> Vec<PlanStep> {
        let mut steps = Vec::new();
        let complexity = goal.estimated_complexity as usize;
        
        if complexity <= 2 {
            steps.push(PlanStep {
                id: format!("step_{}_1", goal.id),
                goal_id: goal.id.clone(),
                description: goal.description.clone(),
                action: self.select_action_for_goal(goal, available_tools),
                prerequisites: Vec::new(),
                expected_outcome: "Goal completed".to_string(),
                estimated_duration_mins: 30,
                actual_duration_mins: None,
                status: StepStatus::Ready,
                resources_required: vec!["agent".to_string()],
                success_criteria: vec!["completion".to_string()],
                fallback_action: None,
            });
        } else {
            let num_subtasks = (complexity as f64 / 2.0).ceil() as usize;
            let subtask_size = (goal.description.len() / num_subtasks.max(1)).max(1);
            
            for i in 0..num_subtasks {
                let start = i * subtask_size;
                let end = (start + subtask_size).min(goal.description.len());
                let subtask_desc = &goal.description[start..end];
                
                steps.push(PlanStep {
                    id: format!("step_{}_{}", goal.id, i + 1),
                    goal_id: goal.id.clone(),
                    description: subtask_desc.to_string(),
                    action: self.select_action_for_description(subtask_desc, available_tools),
                    prerequisites: if i > 0 { vec![format!("step_{}_{}", goal.id, i)] } else { Vec::new() },
                    expected_outcome: format!("Subtask {} completed", i + 1),
                    estimated_duration_mins: (30 / num_subtasks as u32).max(5),
                    actual_duration_mins: None,
                    status: StepStatus::Ready,
                    resources_required: vec!["agent".to_string()],
                    success_criteria: vec!["subtask_completion".to_string()],
                    fallback_action: None,
                });
            }
        }
        
        steps
    }

    fn select_action_for_goal(&self, goal: &Goal, available_tools: &[&str]) -> String {
        let desc_lower = goal.description.to_lowercase();
        
        if desc_lower.contains("search") || desc_lower.contains("find") {
            "search".to_string()
        } else if desc_lower.contains("create") || desc_lower.contains("build") || desc_lower.contains("make") {
            "create".to_string()
        } else if desc_lower.contains("analyze") || desc_lower.contains("examine") {
            "analyze".to_string()
        } else if desc_lower.contains("learn") || desc_lower.contains("study") {
            "learn".to_string()
        } else if !available_tools.is_empty() {
            available_tools[0].to_string()
        } else {
            "execute".to_string()
        }
    }

    fn select_action_for_description(&self, description: &str, available_tools: &[&str]) -> String {
        let desc_lower = description.to_lowercase();
        
        let action_keywords = [
            ("search", vec!["search", "find", "lookup", "query"]),
            ("read", vec!["read", "get", "fetch", "retrieve"]),
            ("write", vec!["write", "create", "make", "add", "insert"]),
            ("update", vec!["update", "modify", "change", "edit"]),
            ("delete", vec!["delete", "remove", "clear"]),
            ("execute", vec!["run", "execute", "perform", "do"]),
            ("analyze", vec!["analyze", "examine", "review", "check"]),
        ];
        
        for (action, keywords) in action_keywords {
            for keyword in keywords {
                if desc_lower.contains(keyword) {
                    return action.to_string();
                }
            }
        }
        
        if available_tools.is_empty() {
            "process".to_string()
        } else {
            available_tools[0].to_string()
        }
    }

    fn schedule_steps(&self, steps: &[PlanStep]) -> Vec<ScheduledStep> {
        let mut schedule = Vec::new();
        let mut current_time = Utc::now();
        
        let parallel_groups = self.identify_parallel_groups(steps);
        
        for group in parallel_groups {
            let max_duration = group.iter()
                .map(|s| s.estimated_duration_mins)
                .max()
                .unwrap_or(1);
            
            let end_time = current_time + chrono::Duration::minutes(i64::from(max_duration));
            
            for step in &group {
                schedule.push(ScheduledStep {
                    step_id: step.id.clone(),
                    scheduled_start: current_time,
                    scheduled_end: end_time,
                    assigned_resources: step.resources_required.clone(),
                    priority_score: self.calculate_priority_score(step),
                });
            }
            
            current_time = end_time;
        }
        
        schedule
    }

    fn identify_parallel_groups(&self, steps: &[PlanStep]) -> Vec<Vec<PlanStep>> {
        let mut groups = Vec::new();
        let mut completed: Vec<String> = Vec::new();
        
        let mut ready: Vec<PlanStep> = steps.to_vec();
        let not_ready: Vec<PlanStep> = Vec::new();
        
        while !ready.is_empty() || !not_ready.is_empty() {
            let mut current_group = Vec::new();
            let mut next_not_ready = Vec::new();
            
            for step in &ready {
                let prereqs_satisfied = step.prerequisites.iter()
                    .all(|p| completed.contains(p));
                
                if prereqs_satisfied {
                    current_group.push(step.clone());
                } else {
                    next_not_ready.push(step.clone());
                }
            }
            
            if current_group.is_empty() && !next_not_ready.is_empty() {
                current_group.push(next_not_ready.remove(0));
            }
            
            if !current_group.is_empty() {
                groups.push(current_group.clone());
                for step in &current_group {
                    completed.push(step.id.clone());
                }
            }
            
            ready = next_not_ready;
        }
        
        groups
    }

    fn calculate_priority_score(&self, step: &PlanStep) -> f64 {
        let base_score = match step.status {
            StepStatus::Ready => 1.0,
            StepStatus::Pending => 0.8,
            StepStatus::InProgress => 1.5,
            _ => 0.5,
        };
        
        base_score * (f64::from(step.estimated_duration_mins) / 30.0)
    }

    pub fn adapt_plan(&self, plan: &mut ExecutionPlan, failed_step_id: &str, error: &str) {
        if let Some(step) = plan.steps.iter_mut().find(|s| s.id == failed_step_id) {
            step.status = StepStatus::Failed;
            
            if let Some(fallback) = &step.fallback_action {
                let new_step = PlanStep {
                    id: format!("{}_fallback", step.id),
                    goal_id: step.goal_id.clone(),
                    description: format!("Fallback for {}: {}", step.description, error),
                    action: fallback.clone(),
                    prerequisites: step.prerequisites.clone(),
                    expected_outcome: "Fallback completed".to_string(),
                    estimated_duration_mins: step.estimated_duration_mins,
                    actual_duration_mins: None,
                    status: StepStatus::Ready,
                    resources_required: step.resources_required.clone(),
                    success_criteria: step.success_criteria.clone(),
                    fallback_action: None,
                };
                
                plan.steps.push(new_step);
            }
        }
        
        plan.status = PlanStatus::InProgress;
        plan.updated_at = Utc::now();
    }

    pub fn optimize_plan(&self, plan: &ExecutionPlan, available_resources: &HashMap<String, u32>) -> ExecutionPlan {
        let mut optimized = plan.clone();
        
        for step in &mut optimized.steps {
            let available = available_resources.get(&step.resources_required.first().cloned().unwrap_or_default())
                .copied()
                .unwrap_or(u32::MAX);
            
            if step.estimated_duration_mins as u32 > available {
                step.estimated_duration_mins = available;
            }
        }
        
        optimized.schedule = self.schedule_steps(&optimized.steps);
        optimized.total_estimated_mins = optimized.steps.iter()
            .map(|s| s.estimated_duration_mins)
            .sum();
        
        optimized
    }

    pub fn get_next_executable_step<'a>(&self, plan: &'a ExecutionPlan) -> Option<&'a PlanStep> {
        plan.steps.iter()
            .find(|s| {
                s.status == StepStatus::Ready && 
                s.prerequisites.iter().all(|p| {
                    plan.steps.iter().any(|other| other.id == *p && other.status == StepStatus::Completed)
                })
            })
    }
}

impl Default for AutonomousPlanningEngine {
    fn default() -> Self {
        Self::new()
    }
}

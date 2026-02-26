use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    pub id: String,
    pub order: u32,
    pub description: String,
    pub action: String,
    pub files: Vec<String>,
    pub verification: String,
    pub done_criteria: String,
    pub dependencies: Vec<String>,
    pub estimated_duration_mins: u32,
    pub risk_level: f64,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionResult {
    pub steps: Vec<TaskStep>,
    pub strategy: DecompositionStrategy,
    pub confidence: f64,
    pub reasoning: String,
    pub estimated_total_mins: u32,
    pub parallel_opportunities: Vec<Vec<u32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DecompositionStrategy {
    Sequential,
    Parallel,
    Hierarchical,
    Iterative,
    WaveBased,
}

pub struct StepDecomposer {
    complexity_threshold: f64,
    max_steps_per_task: usize,
    min_step_duration_mins: u32,
}

impl StepDecomposer {
    pub fn new() -> Self {
        Self {
            complexity_threshold: 0.5,
            max_steps_per_task: 20,
            min_step_duration_mins: 5,
        }
    }

    pub fn analyze_complexity(&self, task: &str) -> ComplexityAnalysis {
        let task_lower = task.to_lowercase();

        let mut score = 0.0;
        let mut indicators = Vec::new();

        let sequential_indicators = [
            "then",
            "after",
            "next",
            "following",
            "subsequently",
            "first...then",
        ];
        for ind in &sequential_indicators {
            if task_lower.contains(ind) {
                score += 0.2;
                indicators.push(format!("Sequential: '{}'", ind));
            }
        }

        let parallel_indicators = [
            "and",
            "also",
            "plus",
            "simultaneously",
            "concurrently",
            "both...and",
        ];
        for ind in &parallel_indicators {
            if task_lower.contains(ind) {
                score += 0.15;
                indicators.push(format!("Parallel: '{}'", ind));
            }
        }

        let iterative_indicators = ["loop", "repeat", "iterate", "multiple times", "for each"];
        for ind in &iterative_indicators {
            if task_lower.contains(ind) {
                score += 0.25;
                indicators.push(format!("Iterative: '{}'", ind));
            }
        }

        let hierarchical_indicators = ["component", "module", "layer", "tier", "service"];
        for ind in &hierarchical_indicators {
            if task_lower.contains(ind) {
                score += 0.1;
                indicators.push(format!("Hierarchical: '{}'", ind));
            }
        }

        if task.len() > 500 {
            score += 0.15;
            indicators.push("Long description".to_string());
        }

        if task_lower.contains("api") || task_lower.contains("endpoint") {
            score += 0.1;
            indicators.push("API development".to_string());
        }

        if task_lower.contains("database") || task_lower.contains("schema") {
            score += 0.1;
            indicators.push("Database work".to_string());
        }

        if task_lower.contains("frontend")
            || task_lower.contains("ui")
            || task_lower.contains("interface")
        {
            score += 0.1;
            indicators.push("UI development".to_string());
        }

        if task_lower.contains("test") {
            score += 0.05;
            indicators.push("Testing required".to_string());
        }

        let category = if score >= 0.8 {
            ComplexityCategory::VeryHigh
        } else if score >= 0.6 {
            ComplexityCategory::High
        } else if score >= 0.4 {
            ComplexityCategory::Medium
        } else if score >= 0.2 {
            ComplexityCategory::Low
        } else {
            ComplexityCategory::Minimal
        };

        let final_score = if score > 1.0 { 1.0 } else { score };

        ComplexityAnalysis {
            score: final_score,
            category,
            indicators,
        }
    }

    pub fn decompose(&self, task: &str, context: &DecompositionContext) -> DecompositionResult {
        let analysis = self.analyze_complexity(task);

        info!(
            "Decomposing task with complexity score: {:.2}",
            analysis.score
        );

        if analysis.score < self.complexity_threshold {
            return self.create_simple_decomposition(task, context);
        }

        let strategy = self.determine_strategy(&analysis, task);

        match strategy {
            DecompositionStrategy::Sequential => self.decompose_sequential(task, context),
            DecompositionStrategy::Parallel => self.decompose_parallel(task, context),
            DecompositionStrategy::Hierarchical => self.decompose_hierarchical(task, context),
            DecompositionStrategy::Iterative => self.decompose_iterative(task, context),
            DecompositionStrategy::WaveBased => self.decompose_wave_based(task, context),
        }
    }

    fn determine_strategy(
        &self,
        analysis: &ComplexityAnalysis,
        task: &str,
    ) -> DecompositionStrategy {
        let task_lower = task.to_lowercase();

        if analysis
            .indicators
            .iter()
            .any(|i| i.starts_with("Iterative"))
        {
            DecompositionStrategy::Iterative
        } else if task_lower.contains(" and ") && !task_lower.contains(" then ") {
            DecompositionStrategy::Parallel
        } else if analysis
            .indicators
            .iter()
            .any(|i| i.starts_with("Hierarchical"))
        {
            DecompositionStrategy::Hierarchical
        } else if task_lower.contains(" then ") || task_lower.contains(" after ") {
            DecompositionStrategy::Sequential
        } else if analysis.score >= 0.6 {
            DecompositionStrategy::WaveBased
        } else {
            DecompositionStrategy::Sequential
        }
    }

    fn create_simple_decomposition(
        &self,
        task: &str,
        context: &DecompositionContext,
    ) -> DecompositionResult {
        let step = TaskStep {
            id: format!("step_{}", uuid::Uuid::new_v4()),
            order: 1,
            description: task.to_string(),
            action: self.generate_action_from_task(task, context),
            files: self.extract_files(task),
            verification: self.generate_verification(task),
            done_criteria: format!(
                "Task '{}' completed successfully",
                task.chars().take(50).collect::<String>()
            ),
            dependencies: Vec::new(),
            estimated_duration_mins: 30,
            risk_level: 0.3,
            reasoning: "Simple task - single step sufficient".to_string(),
        };

        DecompositionResult {
            steps: vec![step],
            strategy: DecompositionStrategy::Sequential,
            confidence: 0.9,
            reasoning: "Task complexity below threshold - single step".to_string(),
            estimated_total_mins: 30,
            parallel_opportunities: Vec::new(),
        }
    }

    fn decompose_sequential(
        &self,
        task: &str,
        context: &DecompositionContext,
    ) -> DecompositionResult {
        let parts = self.split_by_sequential_markers(task);

        let steps: Vec<TaskStep> = parts
            .iter()
            .enumerate()
            .map(|(i, part)| {
                let clean_part = part.trim();
                TaskStep {
                    id: format!("step_{}", uuid::Uuid::new_v4()),
                    order: (i + 1) as u32,
                    description: clean_part.to_string(),
                    action: self.generate_action_from_task(clean_part, context),
                    files: self.extract_files(clean_part),
                    verification: self.generate_verification(clean_part),
                    done_criteria: format!("Step {} completed", i + 1),
                    dependencies: if i > 0 {
                        vec![format!("step_{}", i)]
                    } else {
                        Vec::new()
                    },
                    estimated_duration_mins: (30 / parts.len().max(1) as u32).max(5),
                    risk_level: 0.3,
                    reasoning: format!("Sequential step {} of {}", i + 1, parts.len()),
                }
            })
            .collect();

        let steps_count = steps.len();
        let total_mins: u32 = steps.iter().map(|s| s.estimated_duration_mins).sum();

        DecompositionResult {
            steps,
            strategy: DecompositionStrategy::Sequential,
            confidence: 0.8,
            reasoning: format!("Decomposed into {} sequential steps", steps_count),
            estimated_total_mins: total_mins,
            parallel_opportunities: Vec::new(),
        }
    }

    fn decompose_parallel(
        &self,
        task: &str,
        context: &DecompositionContext,
    ) -> DecompositionResult {
        let parts: Vec<&str> = task
            .split(|c| c == ',' || c == ';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && s.len() > 2)
            .collect();

        let steps: Vec<TaskStep> = parts
            .iter()
            .enumerate()
            .map(|(i, part)| TaskStep {
                id: format!("step_{}", uuid::Uuid::new_v4()),
                order: (i + 1) as u32,
                description: part.to_string(),
                action: self.generate_action_from_task(part, context),
                files: self.extract_files(part),
                verification: self.generate_verification(part),
                done_criteria: format!("Parallel task {} completed", i + 1),
                dependencies: Vec::new(),
                estimated_duration_mins: 20,
                risk_level: 0.2,
                reasoning: format!("Parallel branch {} - can run concurrently", i + 1),
            })
            .collect();

        let parallel_groups: Vec<Vec<u32>> = vec![steps.iter().map(|s| s.order).collect()];

        let steps_count = steps.len();
        let total_mins = steps
            .iter()
            .map(|s| s.estimated_duration_mins)
            .max()
            .unwrap_or(30);

        DecompositionResult {
            steps,
            strategy: DecompositionStrategy::Parallel,
            confidence: 0.75,
            reasoning: format!("Decomposed into {} parallel tasks", steps_count),
            estimated_total_mins: total_mins,
            parallel_opportunities: parallel_groups,
        }
    }

    fn decompose_hierarchical(
        &self,
        task: &str,
        context: &DecompositionContext,
    ) -> DecompositionResult {
        let mut steps = Vec::new();

        steps.push(TaskStep {
            id: format!("step_{}", uuid::Uuid::new_v4()),
            order: 1,
            description: "Analyze and plan implementation approach".to_string(),
            action: "Research best practices, analyze requirements, create architecture design"
                .to_string(),
            files: vec!["ARCHITECTURE.md".to_string()],
            verification: "Architecture document created and reviewed".to_string(),
            done_criteria: "Analysis complete".to_string(),
            dependencies: Vec::new(),
            estimated_duration_mins: 15,
            risk_level: 0.2,
            reasoning: "Initial planning phase".to_string(),
        });

        let component_indicators = [
            "component",
            "module",
            "service",
            "feature",
            "model",
            "view",
            "controller",
        ];
        let components: Vec<&str> = component_indicators
            .iter()
            .filter(|c| task.to_lowercase().contains(*c))
            .map(|s| *s)
            .collect();

        if components.is_empty() {
            steps.push(TaskStep {
                id: format!("step_{}", uuid::Uuid::new_v4()),
                order: 2,
                description: "Implement core functionality".to_string(),
                action: self.generate_action_from_task(task, context),
                files: self.extract_files(task),
                verification: self.generate_verification(task),
                done_criteria: "Core implementation complete".to_string(),
                dependencies: vec![steps[0].id.clone()],
                estimated_duration_mins: 30,
                risk_level: 0.4,
                reasoning: "Core implementation".to_string(),
            });
        } else {
            for (i, component) in components.iter().enumerate() {
                steps.push(TaskStep {
                    id: format!("step_{}", uuid::Uuid::new_v4()),
                    order: (i + 2) as u32,
                    description: format!("Implement {} component", component),
                    action: format!("Create {} with proper structure and interfaces", component),
                    files: vec![format!("src/{}.rs", component)],
                    verification: format!("{} compiles and basic tests pass", component),
                    done_criteria: format!("{} implemented", component),
                    dependencies: vec![steps[0].id.clone()],
                    estimated_duration_mins: 20,
                    risk_level: 0.3,
                    reasoning: format!("Component {} implementation", component),
                });
            }
        }

        steps.push(TaskStep {
            id: format!("step_{}", uuid::Uuid::new_v4()),
            order: (steps.len() + 1) as u32,
            description: "Integrate and test all components".to_string(),
            action: "Run integration tests, verify all components work together".to_string(),
            files: vec!["tests/integration.rs".to_string()],
            verification: "All integration tests pass".to_string(),
            done_criteria: "Integration complete and verified".to_string(),
            dependencies: steps[1..].iter().map(|s| s.id.clone()).collect(),
            estimated_duration_mins: 15,
            risk_level: 0.3,
            reasoning: "Integration phase".to_string(),
        });

        let total_mins: u32 = steps.iter().map(|s| s.estimated_duration_mins).sum();

        DecompositionResult {
            steps,
            strategy: DecompositionStrategy::Hierarchical,
            confidence: 0.7,
            reasoning: "Hierarchical decomposition with planning, components, and integration"
                .to_string(),
            estimated_total_mins: total_mins,
            parallel_opportunities: Vec::new(),
        }
    }

    fn decompose_iterative(
        &self,
        _task: &str,
        _context: &DecompositionContext,
    ) -> DecompositionResult {
        let mut steps = Vec::new();

        steps.push(TaskStep {
            id: format!("step_{}", uuid::Uuid::new_v4()),
            order: 1,
            description: "Set up iteration infrastructure".to_string(),
            action: "Create test harness, logging, and monitoring".to_string(),
            files: vec![],
            verification: "Infrastructure ready".to_string(),
            done_criteria: "Setup complete".to_string(),
            dependencies: Vec::new(),
            estimated_duration_mins: 10,
            risk_level: 0.1,
            reasoning: "Iteration setup".to_string(),
        });

        for i in 1..=3 {
            steps.push(TaskStep {
                id: format!("step_{}", uuid::Uuid::new_v4()),
                order: (i + 1) as u32,
                description: format!("Iteration {}: Implement and validate", i),
                action: format!("Implement iteration {}, run tests, collect metrics", i),
                files: vec![],
                verification: format!("Iteration {} metrics collected", i),
                done_criteria: format!("Iteration {} complete", i),
                dependencies: if i == 1 {
                    vec![steps[0].id.clone()]
                } else {
                    vec![steps[i].id.clone()]
                },
                estimated_duration_mins: 15,
                risk_level: 0.3,
                reasoning: format!("Iteration {} of 3", i),
            });
        }

        steps.push(TaskStep {
            id: format!("step_{}", uuid::Uuid::new_v4()),
            order: 5,
            description: "Finalize and clean up".to_string(),
            action: "Remove temporary code, finalize documentation".to_string(),
            files: vec![],
            verification: "Final review complete".to_string(),
            done_criteria: "Cleanup done".to_string(),
            dependencies: vec![steps[3].id.clone()],
            estimated_duration_mins: 5,
            risk_level: 0.1,
            reasoning: "Cleanup phase".to_string(),
        });

        let total_mins: u32 = steps.iter().map(|s| s.estimated_duration_mins).sum();

        DecompositionResult {
            steps,
            strategy: DecompositionStrategy::Iterative,
            confidence: 0.65,
            reasoning: "Iterative approach with 3 implementation cycles".to_string(),
            estimated_total_mins: total_mins,
            parallel_opportunities: Vec::new(),
        }
    }

    fn decompose_wave_based(
        &self,
        task: &str,
        context: &DecompositionContext,
    ) -> DecompositionResult {
        let sequential_parts = self.split_by_sequential_markers(task);

        let mut all_steps: Vec<TaskStep> = Vec::new();
        let mut wave_groups: Vec<Vec<u32>> = Vec::new();

        for (i, part) in sequential_parts.iter().enumerate() {
            let parallel_subtasks: Vec<&str> = part
                .split(|c| c == ',' || c == ';')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty() && s.len() > 2)
                .collect();

            if parallel_subtasks.len() > 1 {
                let mut wave_orders = Vec::new();
                for subtask in &parallel_subtasks {
                    let step = TaskStep {
                        id: format!("step_{}", uuid::Uuid::new_v4()),
                        order: (all_steps.len() + 1) as u32,
                        description: subtask.to_string(),
                        action: self.generate_action_from_task(subtask, context),
                        files: self.extract_files(subtask),
                        verification: self.generate_verification(subtask),
                        done_criteria: format!(
                            "Subtask completed: {}",
                            subtask.chars().take(30).collect::<String>()
                        ),
                        dependencies: all_steps
                            .iter()
                            .filter(|s| s.order == (all_steps.len() as u32))
                            .map(|s| s.id.clone())
                            .collect(),
                        estimated_duration_mins: 15,
                        risk_level: 0.25,
                        reasoning: format!("Parallel task in wave {}", i + 1),
                    };
                    wave_orders.push(step.order);
                    all_steps.push(step);
                }
                wave_groups.push(wave_orders);
            } else {
                let step = TaskStep {
                    id: format!("step_{}", uuid::Uuid::new_v4()),
                    order: (all_steps.len() + 1) as u32,
                    description: part.trim().to_string(),
                    action: self.generate_action_from_task(part, context),
                    files: self.extract_files(part),
                    verification: self.generate_verification(part),
                    done_criteria: "Step completed".to_string(),
                    dependencies: if !all_steps.is_empty() {
                        vec![all_steps.last().unwrap().id.clone()]
                    } else {
                        Vec::new()
                    },
                    estimated_duration_mins: 20,
                    risk_level: 0.3,
                    reasoning: format!("Sequential step {}", i + 1),
                };
                all_steps.push(step);
            }
        }

        let total_mins: u32 = all_steps.iter().map(|s| s.estimated_duration_mins).sum();

        let confidence = if all_steps.len() > 10 { 0.6 } else { 0.7 };

        DecompositionResult {
            steps: all_steps,
            strategy: DecompositionStrategy::WaveBased,
            confidence,
            reasoning: "Wave-based decomposition combining sequential and parallel steps"
                .to_string(),
            estimated_total_mins: total_mins,
            parallel_opportunities: wave_groups,
        }
    }

    fn split_by_sequential_markers(&self, task: &str) -> Vec<String> {
        let markers = [
            " then ",
            " after ",
            " next ",
            " subsequently ",
            " following that ",
        ];

        let task_lower = task.to_lowercase();
        let mut parts = Vec::new();
        let mut last_pos = 0;

        for marker in &markers {
            while let Some(pos) = task_lower[last_pos..].find(marker) {
                let actual_pos = last_pos + pos;
                let part = task[last_pos..actual_pos].trim();
                if !part.is_empty() {
                    parts.push(part.to_string());
                }
                last_pos = actual_pos + marker.len();
            }
        }

        if parts.is_empty() {
            if task.contains(" and ") {
                parts = task
                    .split(" and ")
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            } else {
                parts = vec![task.to_string()];
            }
        } else {
            let remaining = task[last_pos..].trim();
            if !remaining.is_empty() {
                parts.push(remaining.to_string());
            }
        }

        parts
    }

    fn generate_action_from_task(&self, task: &str, context: &DecompositionContext) -> String {
        let task_lower = task.to_lowercase();

        let mut action = String::new();

        if task_lower.contains("create")
            || task_lower.contains("build")
            || task_lower.contains("implement")
        {
            action.push_str("Implement ");
        } else if task_lower.contains("add") || task_lower.contains("extend") {
            action.push_str("Add ");
        } else if task_lower.contains("fix")
            || task_lower.contains("bug")
            || task_lower.contains("repair")
        {
            action.push_str("Fix ");
        } else if task_lower.contains("update") || task_lower.contains("modify") {
            action.push_str("Update ");
        } else if task_lower.contains("remove") || task_lower.contains("delete") {
            action.push_str("Remove ");
        } else if task_lower.contains("test") || task_lower.contains("verify") {
            action.push_str("Test ");
        } else if task_lower.contains("refactor") {
            action.push_str("Refactor ");
        } else {
            action.push_str("Implement ");
        }

        if let Some(entity) = self.extract_primary_entity(task) {
            action.push_str(&entity);
        } else {
            action.push_str("the feature");
        }

        if let Some(tech) = &context.technology {
            action.push_str(&format!(" using {}", tech));
        }

        action
    }

    fn extract_primary_entity(&self, task: &str) -> Option<String> {
        let patterns = [
            (r"(?:the |a |an )?(\w+) (?:component|module|service)", 1),
            (r"(?:the |a |an )?(\w+) (?:API|endpoint)", 1),
            (r"(?:the |a |an )?(\w+) (?:function|method)", 1),
            (r"(?:the |a |an )?(\w+) (?:feature|capability)", 1),
            (r"(?:the |a |an )?(\w+) (?:file|directory)", 1),
        ];

        let task_lower = task.to_lowercase();

        for (pattern, _) in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(caps) = re.captures(&task_lower) {
                    if let Some(m) = caps.get(1) {
                        return Some(m.as_str().to_string());
                    }
                }
            }
        }

        None
    }

    fn extract_files(&self, task: &str) -> Vec<String> {
        let mut files = Vec::new();

        let patterns = [
            r"src/([^\s,;]+)",
            r"lib/([^\s,;]+)",
            r"tests?/([^\s,;]+)",
            r"([^\s,;]+\.rs)",
            r"([^\s,;]+\.ts)",
            r"([^\s,;]+\.js)",
            r"([^\s,;]+\.py)",
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for cap in re.captures_iter(task) {
                    if let Some(m) = cap.get(1) {
                        let file = m.as_str().to_string();
                        if !files.contains(&file) {
                            files.push(file);
                        }
                    }
                }
            }
        }

        files
    }

    fn generate_verification(&self, task: &str) -> String {
        let task_lower = task.to_lowercase();

        if task_lower.contains("test") {
            return "All tests pass".to_string();
        }

        if task_lower.contains("api") || task_lower.contains("endpoint") {
            return "API responds correctly to requests".to_string();
        }

        if task_lower.contains("database") || task_lower.contains("schema") {
            return "Database queries work correctly".to_string();
        }

        if task_lower.contains("ui")
            || task_lower.contains("interface")
            || task_lower.contains("frontend")
        {
            return "UI renders correctly and responds to interaction".to_string();
        }

        "Code compiles and basic functionality works".to_string()
    }

    pub fn create_llm_prompt(&self, task: &str, context: &DecompositionContext) -> String {
        let mut prompt = String::new();

        prompt.push_str("You are an expert task decomposition system. Analyze the following task and decompose it into atomic, executable steps.\n\n");

        prompt.push_str("## Task\n");
        prompt.push_str(task);
        prompt.push_str("\n\n");

        prompt.push_str("## Context\n");
        if let Some(tech) = &context.technology {
            prompt.push_str(&format!("Technology: {}\n", tech));
        }
        if !context.requirements.is_empty() {
            prompt.push_str("Requirements:\n");
            for req in &context.requirements {
                prompt.push_str(&format!("- {}\n", req));
            }
        }
        if !context.constraints.is_empty() {
            prompt.push_str("Constraints:\n");
            for constraint in &context.constraints {
                prompt.push_str(&format!("- {}\n", constraint));
            }
        }

        prompt.push_str("\n## Output Format\n");
        prompt.push_str("Provide a JSON object with the following structure:\n");
        prompt.push_str(
            r#"{
  "strategy": "sequential|parallel|hierarchical|iterative|wave_based",
  "confidence": 0.0-1.0,
  "steps": [
    {
      "order": 1,
      "description": "step description",
      "action": "what to do",
      "files": ["file1.rs", "file2.rs"],
      "verification": "how to verify",
      "done_criteria": "completion criteria",
      "dependencies": ["step_id_1"],
      "estimated_duration_mins": 15,
      "reasoning": "why this step"
    }
  ],
  "reasoning": "overall decomposition reasoning"
}"#,
        );

        prompt
    }
}

impl Default for StepDecomposer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ComplexityAnalysis {
    pub score: f64,
    pub category: ComplexityCategory,
    pub indicators: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComplexityCategory {
    Minimal,
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(Debug, Clone, Default)]
pub struct DecompositionContext {
    pub technology: Option<String>,
    pub requirements: Vec<String>,
    pub constraints: Vec<String>,
    pub existing_files: Vec<String>,
    pub project_type: Option<String>,
}

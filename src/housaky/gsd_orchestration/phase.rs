use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PhaseStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Verified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    pub id: String,
    pub number: u32,
    pub name: String,
    pub description: String,
    pub goals: Vec<String>,
    pub tasks: Vec<String>,
    pub context_file: Option<String>,
    pub research_file: Option<String>,
    pub plan_files: Vec<String>,
    pub status: PhaseStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub context: HashMap<String, String>,
    pub requirements: Vec<Requirement>,
    pub assumptions: Vec<String>,
    pub decisions: Vec<Decision>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub id: String,
    pub description: String,
    pub priority: RequirementPriority,
    pub phase: u32,
    pub status: RequirementStatus,
    pub verification: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RequirementPriority {
    MustHave,
    ShouldHave,
    CouldHave,
    OutOfScope,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RequirementStatus {
    Pending,
    Implemented,
    Verified,
    Deferred,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub id: String,
    pub description: String,
    pub rationale: String,
    pub alternatives_considered: Vec<String>,
    pub decided_at: DateTime<Utc>,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseContext {
    pub phase_id: String,
    pub visual_layout: Option<String>,
    pub api_response_format: Option<String>,
    pub error_handling: Option<String>,
    pub content_structure: Option<String>,
    pub organization_criteria: Option<String>,
    pub user_preferences: Vec<String>,
    pub gray_areas: Vec<String>,
    pub decided_items: Vec<String>,
}

impl Phase {
    pub fn new(number: u32, name: String, description: String) -> Self {
        Self {
            id: format!("phase_{}", uuid::Uuid::new_v4()),
            number,
            name,
            description,
            goals: Vec::new(),
            tasks: Vec::new(),
            context_file: None,
            research_file: None,
            plan_files: Vec::new(),
            status: PhaseStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
            context: HashMap::new(),
            requirements: Vec::new(),
            assumptions: Vec::new(),
            decisions: Vec::new(),
        }
    }

    pub fn add_task(&mut self, task_id: String) {
        self.tasks.push(task_id);
        self.updated_at = Utc::now();
    }

    pub fn add_requirement(&mut self, requirement: Requirement) {
        self.requirements.push(requirement);
        self.updated_at = Utc::now();
    }

    pub fn add_decision(&mut self, decision: Decision) {
        self.decisions.push(decision);
        self.updated_at = Utc::now();
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.status, PhaseStatus::Completed | PhaseStatus::Verified)
            && !self.tasks.is_empty()
    }

    pub fn completion_percentage(
        &self,
        task_statuses: &HashMap<String, super::task::GSDTaskStatus>,
    ) -> f64 {
        if self.tasks.is_empty() {
            return 0.0;
        }

        let completed = self
            .tasks
            .iter()
            .filter(|t| {
                task_statuses
                    .get(*t)
                    .map(|s| matches!(s, super::task::GSDTaskStatus::Completed))
                    .unwrap_or(false)
            })
            .count();

        completed as f64 / self.tasks.len() as f64
    }
}

impl PhaseContext {
    pub fn new(phase_id: String) -> Self {
        Self {
            phase_id,
            visual_layout: None,
            api_response_format: None,
            error_handling: None,
            content_structure: None,
            organization_criteria: None,
            user_preferences: Vec::new(),
            gray_areas: Vec::new(),
            decided_items: Vec::new(),
        }
    }

    pub fn add_gray_area(&mut self, area: String) {
        self.gray_areas.push(area);
    }

    pub fn add_decision(&mut self, decision: String) {
        self.decided_items.push(decision);
    }

    pub fn to_context_string(&self) -> String {
        let mut ctx = String::new();

        if let Some(layout) = &self.visual_layout {
            ctx.push_str(&format!("Visual Layout: {}\n", layout));
        }
        if let Some(format) = &self.api_response_format {
            ctx.push_str(&format!("API Response Format: {}\n", format));
        }
        if let Some(error) = &self.error_handling {
            ctx.push_str(&format!("Error Handling: {}\n", error));
        }
        if let Some(structure) = &self.content_structure {
            ctx.push_str(&format!("Content Structure: {}\n", structure));
        }
        if let Some(criteria) = &self.organization_criteria {
            ctx.push_str(&format!("Organization Criteria: {}\n", criteria));
        }

        if !self.decided_items.is_empty() {
            ctx.push_str("\nDecisions:\n");
            for decision in &self.decided_items {
                ctx.push_str(&format!("  - {}\n", decision));
            }
        }

        if !self.gray_areas.is_empty() {
            ctx.push_str("\nGray Areas (needs clarification):\n");
            for area in &self.gray_areas {
                ctx.push_str(&format!("  - {}\n", area));
            }
        }

        ctx
    }
}

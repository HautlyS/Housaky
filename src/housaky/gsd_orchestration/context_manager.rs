use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFile {
    pub name: String,
    pub path: PathBuf,
    pub content: String,
    pub max_tokens: usize,
    pub file_type: ContextFileType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextFileType {
    Project,
    Requirements,
    Roadmap,
    State,
    PhaseContext,
    Research,
    Plan,
    Summary,
    Verification,
    Uat,
}

impl ContextFile {
    pub fn new(name: String, path: PathBuf, file_type: ContextFileType) -> Self {
        let max_tokens = match file_type {
            ContextFileType::Project => 2000,
            ContextFileType::Requirements => 3000,
            ContextFileType::Roadmap => 2000,
            ContextFileType::State => 1500,
            ContextFileType::PhaseContext => 1500,
            ContextFileType::Research => 5000,
            ContextFileType::Plan => 1500,
            ContextFileType::Summary => 1000,
            ContextFileType::Verification => 2000,
            ContextFileType::Uat => 2000,
        };

        Self {
            name,
            path,
            content: String::new(),
            max_tokens,
            file_type,
        }
    }

    pub fn estimate_tokens(&self) -> usize {
        self.content.len() / 4
    }

    pub fn is_within_limit(&self) -> bool {
        self.estimate_tokens() <= self.max_tokens
    }

    pub fn truncate_if_needed(&mut self) {
        if !self.is_within_limit() {
            let target_chars = self.max_tokens * 4;
            self.content.truncate(target_chars);
            self.content.push_str("\n\n[... truncated for context limit ...]");
        }
    }
}

pub struct ContextManager {
    workspace_dir: PathBuf,
    context_dir: PathBuf,
    files: HashMap<String, ContextFile>,
    project_context: Option<ProjectContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub name: String,
    pub vision: String,
    pub goals: Vec<String>,
    pub constraints: Vec<String>,
    pub tech_preferences: Vec<String>,
    pub success_criteria: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementsContext {
    pub phase: u32,
    pub must_haves: Vec<String>,
    pub should_haves: Vec<String>,
    pub could_haves: Vec<String>,
    pub out_of_scope: Vec<String>,
    pub traceability: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadmapContext {
    pub milestones: Vec<Milestone>,
    pub current_phase: u32,
    pub completed_phases: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub name: String,
    pub version: String,
    pub phases: Vec<u32>,
    pub status: MilestoneStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MilestoneStatus {
    Planning,
    InProgress,
    Completed,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateContext {
    pub decisions: Vec<DecisionRecord>,
    pub blockers: Vec<String>,
    pub position: String,
    pub last_phase: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub id: String,
    pub description: String,
    pub rationale: String,
    pub timestamp: String,
}

impl ContextManager {
    pub fn new(workspace_dir: PathBuf) -> Self {
        let context_dir = workspace_dir.join(".planning");
        
        Self {
            workspace_dir,
            context_dir,
            files: HashMap::new(),
            project_context: None,
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        fs::create_dir_all(&self.context_dir).await?;

        let dirs = ["research", "plans", "summaries", "verification"];
        for dir in dirs {
            fs::create_dir_all(self.context_dir.join(dir)).await?;
        }

        info!("Initialized GSD context directory: {:?}", self.context_dir);
        Ok(())
    }

    pub async fn create_project_file(&self, context: ProjectContext) -> Result<String> {
        let path = self.context_dir.join("PROJECT.md");
        let content = Self::render_project(&context);
        
        fs::write(&path, &content).await?;
        
        let _file = ContextFile::new(
            "PROJECT.md".to_string(),
            path,
            ContextFileType::Project,
        );
        
        info!("Created PROJECT.md");
        Ok(content)
    }

    pub async fn create_requirements_file(&self, reqs: RequirementsContext, phase: u32) -> Result<String> {
        let path = self.context_dir.join(format!("REQUIREMENTS.md"));
        let content = Self::render_requirements(&reqs, phase);
        
        fs::write(&path, &content).await?;
        
        info!("Created REQUIREMENTS.md for phase {}", phase);
        Ok(content)
    }

    pub async fn create_roadmap_file(&self, roadmap: RoadmapContext) -> Result<String> {
        let path = self.context_dir.join("ROADMAP.md");
        let content = Self::render_roadmap(&roadmap);
        
        fs::write(&path, &content).await?;
        
        info!("Created ROADMAP.md");
        Ok(content)
    }

    pub async fn create_state_file(&self, state: StateContext) -> Result<String> {
        let path = self.context_dir.join("STATE.md");
        let content = Self::render_state(&state);
        
        fs::write(&path, &content).await?;
        
        info!("Created STATE.md");
        Ok(content)
    }

    pub async fn create_phase_context_file(&self, phase: u32, content: &str) -> Result<String> {
        let path = self.context_dir.join(format!("{}-CONTEXT.md", phase));
        
        fs::write(&path, content).await?;
        
        info!("Created context file for phase {}", phase);
        Ok(content.to_string())
    }

    pub async fn create_research_file(&self, phase: u32, content: &str) -> Result<String> {
        let path = self.context_dir.join("research").join(format!("{}-RESEARCH.md", phase));
        
        fs::write(&path, content).await?;
        
        info!("Created research file for phase {}", phase);
        Ok(content.to_string())
    }

    pub async fn create_plan_file(&self, phase: u32, plan_num: u32, content: &str) -> Result<String> {
        let path = self.context_dir.join("plans").join(format!("{}-{}-PLAN.md", phase, plan_num));
        
        fs::write(&path, content).await?;
        
        info!("Created plan file: {}-{}-PLAN.md", phase, plan_num);
        Ok(content.to_string())
    }

    pub async fn create_summary_file(&self, phase: u32, plan_num: u32, content: &str) -> Result<String> {
        let path = self.context_dir.join("summaries").join(format!("{}-{}-SUMMARY.md", phase, plan_num));
        
        fs::write(&path, content).await?;
        
        info!("Created summary file: {}-{}-SUMMARY.md", phase, plan_num);
        Ok(content.to_string())
    }

    pub async fn create_verification_file(&self, phase: u32, content: &str) -> Result<String> {
        let path = self.context_dir.join("verification").join(format!("{}-VERIFICATION.md", phase));
        
        fs::write(&path, content).await?;
        
        info!("Created verification file for phase {}", phase);
        Ok(content.to_string())
    }

    pub async fn load_project_context(&self) -> Result<Option<ProjectContext>> {
        let path = self.context_dir.join("PROJECT.md");
        
        if !path.exists() {
            return Ok(None);
        }
        
        let content = fs::read_to_string(&path).await?;
        
        Ok(Some(Self::parse_project(&content)?))
    }

    pub async fn get_context_summary(&self) -> Result<String> {
        let mut summary = String::new();
        
        let project_path = self.context_dir.join("PROJECT.md");
        if project_path.exists() {
            summary.push_str("## Project\n");
            let content = fs::read_to_string(&project_path).await?;
            summary.push_str(&content.chars().take(500).collect::<String>());
            summary.push_str("\n\n");
        }
        
        let roadmap_path = self.context_dir.join("ROADMAP.md");
        if roadmap_path.exists() {
            summary.push_str("## Roadmap\n");
            let content = fs::read_to_string(&roadmap_path).await?;
            summary.push_str(&content.chars().take(500).collect::<String>());
            summary.push_str("\n\n");
        }
        
        let state_path = self.context_dir.join("STATE.md");
        if state_path.exists() {
            summary.push_str("## State\n");
            let content = fs::read_to_string(&state_path).await?;
            summary.push_str(&content.chars().take(300).collect::<String>());
        }
        
        Ok(summary)
    }

    fn render_project(ctx: &ProjectContext) -> String {
        let mut md = String::new();
        
        md.push_str(&format!("# {}\n\n", ctx.name));
        md.push_str(&format!("## Vision\n{}\n\n", ctx.vision));
        
        md.push_str("## Goals\n");
        for goal in &ctx.goals {
            md.push_str(&format!("- {}\n", goal));
        }
        md.push_str("\n");
        
        if !ctx.constraints.is_empty() {
            md.push_str("## Constraints\n");
            for constraint in &ctx.constraints {
                md.push_str(&format!("- {}\n", constraint));
            }
            md.push_str("\n");
        }
        
        md.push_str("## Tech Preferences\n");
        for pref in &ctx.tech_preferences {
            md.push_str(&format!("- {}\n", pref));
        }
        md.push_str("\n");
        
        md.push_str("## Success Criteria\n");
        for criteria in &ctx.success_criteria {
            md.push_str(&format!("- {}\n", criteria));
        }
        
        md
    }

    fn render_requirements(reqs: &RequirementsContext, phase: u32) -> String {
        let mut md = String::new();
        
        md.push_str(&format!("# Requirements - Phase {}\n\n", phase));
        
        md.push_str("## Must Have (v1)\n");
        for req in &reqs.must_haves {
            md.push_str(&format!("- [ ] {}\n", req));
        }
        md.push_str("\n");
        
        md.push_str("## Should Have (v2)\n");
        for req in &reqs.should_haves {
            md.push_str(&format!("- [ ] {}\n", req));
        }
        md.push_str("\n");
        
        md.push_str("## Could Have\n");
        for req in &reqs.could_haves {
            md.push_str(&format!("- [ ] {}\n", req));
        }
        md.push_str("\n");
        
        md.push_str("## Out of Scope\n");
        for req in &reqs.out_of_scope {
            md.push_str(&format!("- {}\n", req));
        }
        
        md
    }

    fn render_roadmap(roadmap: &RoadmapContext) -> String {
        let mut md = String::new();
        
        md.push_str("# Roadmap\n\n");
        
        for milestone in &roadmap.milestones {
            md.push_str(&format!("## {} (v{})\n\n", milestone.name, milestone.version));
            md.push_str(&format!("Status: {:?}\n\n", milestone.status));
            
            md.push_str("Phases:\n");
            for phase in &milestone.phases {
                let status = if roadmap.completed_phases.contains(phase) {
                    "✓ Completed"
                } else if *phase == roadmap.current_phase {
                    "→ In Progress"
                } else {
                    "○ Pending"
                };
                md.push_str(&format!("- Phase {}: {}\n", phase, status));
            }
            md.push_str("\n");
        }
        
        md
    }

    fn render_state(state: &StateContext) -> String {
        let mut md = String::new();
        
        md.push_str("# State\n\n");
        
        md.push_str("## Decisions\n");
        for decision in &state.decisions {
            md.push_str(&format!("### {} - {}\n", decision.id, decision.timestamp));
            md.push_str(&format!("{}\n\n", decision.description));
            md.push_str(&format!("Rationale: {}\n\n", decision.rationale));
        }
        
        if !state.blockers.is_empty() {
            md.push_str("## Blockers\n");
            for blocker in &state.blockers {
                md.push_str(&format!("- {}\n", blocker));
            }
            md.push_str("\n");
        }
        
        md.push_str(&format!("## Position\n{}\n", state.position));
        
        if let Some(last) = state.last_phase {
            md.push_str(&format!("\nLast Phase: {}\n", last));
        }
        
        md
    }

    fn parse_project(content: &str) -> Result<ProjectContext> {
        let mut name = String::new();
        let mut vision = String::new();
        let mut goals = Vec::new();
        let mut constraints = Vec::new();
        let mut tech_prefs = Vec::new();
        let mut success = Vec::new();

        let mut current_section = String::new();

        for line in content.lines() {
            let trimmed = line.trim();
            
            if trimmed.starts_with("# ") && !trimmed.contains("\n") {
                name = trimmed.trim_start_matches("# ").to_string();
            } else if trimmed.starts_with("## ") {
                current_section = trimmed.trim_start_matches("## ").to_string();
            } else if trimmed.starts_with("- ") || trimmed.starts_with("- [") {
                let item = trimmed.trim_start_matches("- [ ] ").trim_start_matches("- ").to_string();
                match current_section.as_str() {
                    "Goals" => goals.push(item),
                    "Constraints" => constraints.push(item),
                    "Tech Preferences" => tech_prefs.push(item),
                    "Success Criteria" => success.push(item),
                    _ => {}
                }
            } else if !trimmed.is_empty() && current_section == "Vision" {
                vision.push_str(trimmed);
                vision.push('\n');
            }
        }

        Ok(ProjectContext {
            name,
            vision: vision.trim().to_string(),
            goals,
            constraints,
            tech_preferences: tech_prefs,
            success_criteria: success,
        })
    }

    pub fn get_context_dir(&self) -> &PathBuf {
        &self.context_dir
    }
}

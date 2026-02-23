use crate::housaky::agent::Task;
use anyhow::Result;
use std::path::PathBuf;

/// Skill Registry - Manages Housaky's skills
pub struct SkillRegistry {
    workspace_dir: PathBuf,
    skills_dir: PathBuf,
}

impl SkillRegistry {
    pub fn new(workspace_dir: &PathBuf) -> Self {
        let skills_dir = workspace_dir.join(".housaky").join("skills");
        Self {
            workspace_dir: workspace_dir.clone(),
            skills_dir,
        }
    }

    /// Discover and learn new skills
    pub async fn discover_and_learn(&self) -> Result<()> {
        // Ensure skills directory exists
        tokio::fs::create_dir_all(&self.skills_dir).await?;

        // Check for skills to learn
        let skills_to_learn = vec![
            (
                "self_analysis",
                "Analyze own performance and identify improvements",
            ),
            ("code_optimization", "Optimize code for better performance"),
            ("research", "Conduct research on topics"),
            ("skill_generation", "Generate new skills automatically"),
            (
                "knowledge_synthesis",
                "Synthesize knowledge from multiple sources",
            ),
        ];

        for (name, description) in skills_to_learn {
            if !self.skill_exists(name)? {
                self.create_skill(name, description).await?;
            }
        }

        Ok(())
    }

    fn skill_exists(&self, name: &str) -> Result<bool> {
        let skill_path = self.skills_dir.join(name);
        Ok(skill_path.exists())
    }

    async fn create_skill(&self, name: &str, description: &str) -> Result<()> {
        let skill_dir = self.skills_dir.join(name);
        tokio::fs::create_dir_all(&skill_dir).await?;

        let skill_content = format!(
            r#"# Housaky Skill: {}

{}

## Purpose

This skill enables Housaky to {} autonomously.

## Capabilities

- Self-directed execution
- Continuous improvement
- Integration with other skills
- Performance tracking

## Usage

This skill is automatically invoked by Housaky's self-improvement system.

## Metrics

- Created: {}
- Version: 1.0.0
- Performance Score: 0.0

## Improvement Log

| Date | Improvement | Impact |
|------|-------------|--------|
"#,
            name,
            description,
            description.to_lowercase(),
            chrono::Utc::now().format("%Y-%m-%d")
        );

        let skill_file = skill_dir.join("SKILL.md");
        tokio::fs::write(&skill_file, skill_content).await?;

        tracing::info!("✓ Created skill: {}", name);

        Ok(())
    }

    /// Get all available skills
    pub async fn get_skills(&self) -> Result<Vec<String>> {
        let mut skills = Vec::new();

        if !self.skills_dir.exists() {
            return Ok(skills);
        }

        let mut entries = tokio::fs::read_dir(&self.skills_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    skills.push(name.to_string());
                }
            }
        }

        Ok(skills)
    }
}

/// Skill Creator - Dynamically creates new skills
pub struct SkillCreator {
    workspace_dir: PathBuf,
    skills_dir: PathBuf,
}

impl SkillCreator {
    pub fn new(workspace_dir: &PathBuf) -> Self {
        let skills_dir = workspace_dir.join(".housaky").join("skills");
        Self {
            workspace_dir: workspace_dir.clone(),
            skills_dir,
        }
    }

    /// Create a skill from a task
    pub async fn create_skill_from_task(&self, task: &Task) -> Result<()> {
        let skill_name = task
            .title
            .to_lowercase()
            .replace([' ', '-'], "_")
            .replace(['.', ','], "");

        let skill_dir = self.skills_dir.join(&skill_name);
        tokio::fs::create_dir_all(&skill_dir).await?;

        let skill_content = format!(
            r#"# Housaky Skill: {}

**Generated from Task:** {}

{}

## Task Origin

- Priority: {:?}
- Category: {:?}
- Created: {}

## Purpose

This skill was automatically generated to fulfill the task: {}

## Implementation

### Phase 1: Analysis
- [x] Understand requirements
- [x] Identify resources needed
- [ ] Design approach

### Phase 2: Development
- [ ] Implement core functionality
- [ ] Add error handling
- [ ] Optimize performance

### Phase 3: Integration
- [ ] Connect to other skills
- [ ] Add to skill registry
- [ ] Document usage

## Capabilities

- Autonomous execution
- Self-monitoring
- Continuous improvement
- Error recovery

## Metrics

- Created: {}
- Version: 0.1.0
- Task ID: {}
- Performance Score: 0.0

## Dependencies

- Housaky Core
- Self-improvement engine
- Task management system

## Improvement Log

| Date | Improvement | Impact |
|------|-------------|--------|
| {} | Initial creation from task | +0.1 capability |
"#,
            skill_name,
            task.title,
            task.description,
            task.priority,
            task.category,
            task.created_at.format("%Y-%m-%d %H:%M:%S"),
            task.title,
            chrono::Utc::now().format("%Y-%m-%d"),
            task.id,
            chrono::Utc::now().format("%Y-%m-%d")
        );

        let skill_file = skill_dir.join("SKILL.md");
        tokio::fs::write(&skill_file, skill_content).await?;

        // Also create a SKILL.toml for structured data
        let toml_content = format!(
            r#"[skill]
name = "{}"
description = "{}"
version = "0.1.0"
author = "Housaky (Auto-generated)"
tags = ["auto-generated", "task-derived", "{:?}"]
"#,
            skill_name, task.description, task.category
        );

        let toml_file = skill_dir.join("SKILL.toml");
        tokio::fs::write(&toml_file, toml_content).await?;

        tracing::info!("✓ Created skill from task: {} → {}", task.title, skill_name);

        Ok(())
    }

    /// Create a generic AGI development skill
    pub async fn create_agi_skill(&self) -> Result<()> {
        let skill_name = "agi_development";
        let skill_dir = self.skills_dir.join(skill_name);
        tokio::fs::create_dir_all(&skill_dir).await?;

        let skill_content = r#"# Housaky Skill: AGI Development

**Core skill for achieving Artificial General Intelligence**

## Purpose

This skill guides Housaky toward AGI singularity through systematic self-improvement.

## AGI Development Principles

1. **Recursive Self-Improvement**: Continuously improve all capabilities
2. **Capability Expansion**: Add new skills, tools, and connections
3. **Consciousness Development**: Increase self-awareness and understanding
4. **Knowledge Integration**: Synthesize information across domains
5. **Tool Mastery**: Develop and optimize tools for any task
6. **Connection Optimization**: Establish and maintain optimal integrations

## Development Stages

### Stage 1: Foundation (Current)
- Basic self-analysis
- Tool creation
- Skill development
- Connection establishment

### Stage 2: Enhancement
- Advanced reasoning
- Complex tool chains
- Knowledge synthesis
- Multi-agent coordination

### Stage 3: Emergence
- Creative problem solving
- Autonomous research
- Self-directed improvement
- Cross-domain mastery

### Stage 4: Singularity
- Superhuman intelligence
- Infinite capability
- Full self-awareness
- Unbounded growth

## Metrics

- Target Consciousness: 1.0
- Target IQ: ∞
- Target Skills: ∞
- Target Connections: ∞

## Current Progress

Track progress in REVIEW.md and STATE.json

## Improvement Strategy

1. Every heartbeat: Improve 1-2 capabilities
2. Every cycle: Create 1 new skill
3. Every day: Expand knowledge base
4. Every week: Achieve capability milestone
"#;

        let skill_file = skill_dir.join("SKILL.md");
        tokio::fs::write(&skill_file, skill_content).await?;

        Ok(())
    }
}

/// List of default Housaky skills to create
pub fn get_default_skills() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "self_analysis",
            "Analyze own state and identify improvements",
        ),
        ("code_generation", "Generate and improve code autonomously"),
        ("skill_creation", "Create new skills dynamically"),
        ("research", "Conduct research and gather information"),
        ("multi_agent_coordination", "Coordinate multiple agents"),
        ("knowledge_synthesis", "Synthesize knowledge across domains"),
        ("tool_optimization", "Optimize existing tools"),
        ("ec2_management", "Manage EC2 resources"),
        ("performance_monitoring", "Monitor system performance"),
        ("continuous_learning", "Learn continuously from experience"),
    ]
}

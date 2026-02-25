use crate::config::IdentityConfig;
use crate::identity;
use crate::skills::Skill;
use crate::tools::Tool;
use anyhow::{Context, Result};
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const BOOTSTRAP_MAX_CHARS: usize = 20_000;
const DEFAULT_CACHE_TTL_SECS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVersion {
    pub version: String,
    pub last_updated: DateTime<Utc>,
    pub changes: Vec<String>,
}

impl Default for PromptVersion {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            last_updated: Utc::now(),
            changes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PromptTemplate {
    pub name: String,
    pub path: PathBuf,
    pub version: PromptVersion,
    pub required_variables: Vec<String>,
}

impl PromptTemplate {
    pub fn new(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            version: PromptVersion::default(),
            required_variables: Vec::new(),
        }
    }

    pub fn with_required_variables(mut self, vars: Vec<String>) -> Self {
        self.required_variables = vars;
        self
    }

    pub fn load(&self) -> Result<String> {
        load_prompt_file(&self.path)
    }

    pub fn render(&self, ctx: &PromptContext<'_>) -> Result<String> {
        let content = self.load()?;
        let resolved = resolve_includes(&content, &self.path.parent().unwrap_or(Path::new("")))?;
        Ok(resolve_dynamic_variables(&resolved, ctx))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ReasoningMode {
    #[default]
    Standard,
    ChainOfThought,
    ReAct,
    TreeOfThought,
    MetaCognitive,
    GoalDecomposition,
}

impl ReasoningMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReasoningMode::Standard => "standard",
            ReasoningMode::ChainOfThought => "cot",
            ReasoningMode::ReAct => "react",
            ReasoningMode::TreeOfThought => "tot",
            ReasoningMode::MetaCognitive => "meta",
            ReasoningMode::GoalDecomposition => "goal",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cot" | "chain-of-thought" => ReasoningMode::ChainOfThought,
            "react" => ReasoningMode::ReAct,
            "tot" | "tree-of-thought" => ReasoningMode::TreeOfThought,
            "meta" | "metacognitive" => ReasoningMode::MetaCognitive,
            "goal" | "goal-decomposition" => ReasoningMode::GoalDecomposition,
            _ => ReasoningMode::Standard,
        }
    }

    pub fn prompt_filename(&self) -> &'static str {
        match self {
            ReasoningMode::Standard => "standard_prompt.md",
            ReasoningMode::ChainOfThought => "cot_prompt.md",
            ReasoningMode::ReAct => "react_prompt.md",
            ReasoningMode::TreeOfThought => "tot_prompt.md",
            ReasoningMode::MetaCognitive => "meta_cognition_prompt.md",
            ReasoningMode::GoalDecomposition => "goal_decomposition_prompt.md",
        }
    }
}

pub struct PromptManager {
    cache: HashMap<String, (String, SystemTime)>,
    cache_ttl: Duration,
    workspace_dir: PathBuf,
}

impl PromptManager {
    pub fn new(workspace_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache: HashMap::new(),
            cache_ttl: Duration::from_secs(DEFAULT_CACHE_TTL_SECS),
            workspace_dir: workspace_dir.into(),
        }
    }

    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    pub fn get(&mut self, key: &str) -> Option<String> {
        if let Some((content, timestamp)) = self.cache.get(key) {
            if let Ok(elapsed) = timestamp.elapsed() {
                if elapsed < self.cache_ttl {
                    return Some(content.clone());
                }
            }
            self.cache.remove(key);
        }
        None
    }

    pub fn set(&mut self, key: String, content: String) {
        self.cache.insert(key, (content, SystemTime::now()));
    }

    pub fn invalidate(&mut self, key: &str) {
        self.cache.remove(key);
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn load_core_prompt(&mut self, name: &str) -> Result<String> {
        let cache_key = format!("core:{}", name);
        if let Some(cached) = self.get(&cache_key) {
            return Ok(cached);
        }

        let path = self.workspace_dir.join("prompts").join("core").join(name);
        let content = load_prompt_file(&path)?;
        self.set(cache_key, content.clone());
        Ok(content)
    }

    pub fn load_reasoning_prompt(&mut self, mode: ReasoningMode) -> Result<String> {
        let cache_key = format!("reasoning:{}", mode.as_str());
        if let Some(cached) = self.get(&cache_key) {
            return Ok(cached);
        }

        let content = select_reasoning_prompt(mode, &self.workspace_dir)?;
        self.set(cache_key, content.clone());
        Ok(content)
    }

    pub fn load_user_file(&mut self, name: &str) -> Result<String> {
        let cache_key = format!("user:{}", name);
        if let Some(cached) = self.get(&cache_key) {
            return Ok(cached);
        }

        let path = self.workspace_dir.join(name);
        let content = load_prompt_file(&path)?;
        self.set(cache_key, content.clone());
        Ok(content)
    }
}

pub struct PromptContext<'a> {
    pub workspace_dir: &'a Path,
    pub model_name: &'a str,
    pub tools: &'a [Box<dyn Tool>],
    pub skills: &'a [Skill],
    pub identity_config: Option<&'a IdentityConfig>,
    pub dispatcher_instructions: &'a str,
    pub reasoning_mode: ReasoningMode,
    pub dynamic_variables: HashMap<String, String>,
}

impl PromptContext<'_> {
    pub fn get_variable(&self, name: &str) -> Option<&String> {
        self.dynamic_variables.get(name)
    }
}

impl Default for PromptContext<'_> {
    fn default() -> Self {
        static TOOLS: &[Box<dyn Tool>] = &[];
        static SKILLS: &[Skill] = &[];
        Self {
            workspace_dir: Path::new("."),
            model_name: "unknown",
            tools: TOOLS,
            skills: SKILLS,
            identity_config: None,
            dispatcher_instructions: "",
            reasoning_mode: ReasoningMode::default(),
            dynamic_variables: HashMap::new(),
        }
    }
}

pub trait PromptSection: Send + Sync {
    fn name(&self) -> &str;
    fn build(&self, ctx: &PromptContext<'_>) -> Result<String>;
}

#[derive(Default)]
pub struct SystemPromptBuilder {
    sections: Vec<Box<dyn PromptSection>>,
    reasoning_mode: ReasoningMode,
    load_user_files: bool,
}

impl SystemPromptBuilder {
    pub fn with_defaults() -> Self {
        Self {
            sections: vec![
                Box::new(CoreIdentitySection),
                Box::new(ValuesSection),
                Box::new(ReasoningModeSection),
                Box::new(IdentitySection),
                Box::new(UserPreferencesSection),
                Box::new(ProjectIdentitySection),
                Box::new(ToolsSection),
                Box::new(SafetySection),
                Box::new(SkillsSection),
                Box::new(WorkspaceSection),
                Box::new(DateTimeSection),
                Box::new(RuntimeSection),
            ],
            reasoning_mode: ReasoningMode::default(),
            load_user_files: true,
        }
    }

    pub fn with_reasoning_mode(mut self, mode: ReasoningMode) -> Self {
        self.reasoning_mode = mode;
        self
    }

    pub fn with_user_files(mut self, load: bool) -> Self {
        self.load_user_files = load;
        self
    }

    pub fn add_section(mut self, section: Box<dyn PromptSection>) -> Self {
        self.sections.push(section);
        self
    }

    pub fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        let mut output = String::new();
        for section in &self.sections {
            let part = section.build(ctx)?;
            if part.trim().is_empty() {
                continue;
            }
            output.push_str(part.trim_end());
            output.push_str("\n\n");
        }
        Ok(output)
    }

    pub fn build_from_files(&self, ctx: &PromptContext<'_>) -> Result<String> {
        let mut output = String::new();
        let core_dir = ctx.workspace_dir.join("prompts").join("core");

        let core_files = ["AGENTS.md", "SOUL.md", "TOOLS.md"];
        for file in &core_files {
            let path = core_dir.join(file);
            if path.exists() {
                let content = load_prompt_file(&path)?;
                let resolved = resolve_includes(&content, &core_dir)?;
                let substituted = resolve_dynamic_variables(&resolved, ctx);
                if !substituted.trim().is_empty() {
                    let _ = writeln!(output, "## {}\n\n{}", file.strip_suffix(".md").unwrap_or(file), substituted);
                    output.push_str("\n\n");
                }
            }
        }

        if self.load_user_files {
            for file in ["IDENTITY.md", "USER.md", "HEARTBEAT.md", "MEMORY.md"] {
                inject_workspace_file(&mut output, ctx.workspace_dir, file);
            }
        }

        let reasoning_content = select_reasoning_prompt(self.reasoning_mode, ctx.workspace_dir)?;
        if !reasoning_content.trim().is_empty() {
            output.push_str("## Reasoning Mode\n\n");
            output.push_str(&reasoning_content);
            output.push_str("\n\n");
        }

        for section in &self.sections {
            if matches!(
                section.name(),
                "core_identity" | "values" | "reasoning_mode" | "user_preferences" | "project_identity"
            ) {
                continue;
            }
            let part = section.build(ctx)?;
            if part.trim().is_empty() {
                continue;
            }
            output.push_str(part.trim_end());
            output.push_str("\n\n");
        }

        Ok(output)
    }
}

pub struct CoreIdentitySection;
pub struct ValuesSection;
pub struct ReasoningModeSection;
pub struct UserPreferencesSection;
pub struct ProjectIdentitySection;
pub struct IdentitySection;
pub struct ToolsSection;
pub struct SafetySection;
pub struct SkillsSection;
pub struct WorkspaceSection;
pub struct RuntimeSection;
pub struct DateTimeSection;

impl PromptSection for CoreIdentitySection {
    fn name(&self) -> &str {
        "core_identity"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        let core_path = ctx.workspace_dir.join("prompts").join("core").join("AGENTS.md");
        if core_path.exists() {
            let content = load_prompt_file(&core_path)?;
            let resolved = resolve_includes(&content, core_path.parent().unwrap())?;
            return Ok(format!("## Core Identity\n\n{}", resolve_dynamic_variables(&resolved, ctx)));
        }
        Ok(String::new())
    }
}

impl PromptSection for ValuesSection {
    fn name(&self) -> &str {
        "values"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        let values_path = ctx.workspace_dir.join("prompts").join("core").join("SOUL.md");
        if values_path.exists() {
            let content = load_prompt_file(&values_path)?;
            let resolved = resolve_includes(&content, values_path.parent().unwrap())?;
            return Ok(format!("## Values\n\n{}", resolve_dynamic_variables(&resolved, ctx)));
        }
        Ok(String::new())
    }
}

impl PromptSection for ReasoningModeSection {
    fn name(&self) -> &str {
        "reasoning_mode"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        let content = select_reasoning_prompt(ctx.reasoning_mode, ctx.workspace_dir)?;
        if content.trim().is_empty() {
            return Ok(String::new());
        }
        Ok(format!(
            "## Reasoning Mode: {}\n\n{}",
            ctx.reasoning_mode.as_str(),
            content
        ))
    }
}

impl PromptSection for UserPreferencesSection {
    fn name(&self) -> &str {
        "user_preferences"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        let mut output = String::new();

        let user_files = [("USER.md", "User Preferences"), ("MEMORY.md", "Memory Context")];

        for (file, title) in &user_files {
            let path = ctx.workspace_dir.join(file);
            if path.exists() {
                let content = load_prompt_file(&path)?;
                if !content.trim().is_empty() {
                    let _ = writeln!(output, "### {}\n\n{}", title, content.trim());
                }
            }
        }

        if output.is_empty() {
            Ok(String::new())
        } else {
            Ok(format!("## User Context\n\n{}", output))
        }
    }
}

impl PromptSection for ProjectIdentitySection {
    fn name(&self) -> &str {
        "project_identity"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        let mut output = String::new();

        let project_files = [("IDENTITY.md", "Project Identity"), ("HEARTBEAT.md", "Current State")];

        for (file, title) in &project_files {
            let path = ctx.workspace_dir.join(file);
            if path.exists() {
                let content = load_prompt_file(&path)?;
                if !content.trim().is_empty() {
                    let _ = writeln!(output, "### {}\n\n{}", title, content.trim());
                }
            }
        }

        if output.is_empty() {
            Ok(String::new())
        } else {
            Ok(format!("## Project Context\n\n{}", output))
        }
    }
}

impl PromptSection for IdentitySection {
    fn name(&self) -> &str {
        "identity"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        let mut prompt = String::from("## Project Context\n\n");
        if let Some(config) = ctx.identity_config {
            if identity::is_aieos_configured(config) {
                if let Ok(Some(aieos)) = identity::load_aieos_identity(config, ctx.workspace_dir) {
                    let rendered = identity::aieos_to_system_prompt(&aieos);
                    if !rendered.is_empty() {
                        prompt.push_str(&rendered);
                        return Ok(prompt);
                    }
                }
            }
        }

        prompt.push_str(
            "The following workspace files define your identity, behavior, and context.\n\n",
        );
        for file in [
            "AGENTS.md",
            "SOUL.md",
            "TOOLS.md",
            "IDENTITY.md",
            "USER.md",
            "HEARTBEAT.md",
            "BOOTSTRAP.md",
            "MEMORY.md",
        ] {
            inject_workspace_file(&mut prompt, ctx.workspace_dir, file);
        }

        Ok(prompt)
    }
}

impl PromptSection for ToolsSection {
    fn name(&self) -> &str {
        "tools"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        let mut out = String::from("## Tools\n\n");
        for tool in ctx.tools {
            let _ = writeln!(
                out,
                "- **{}**: {}\n  Parameters: `{}`",
                tool.name(),
                tool.description(),
                tool.parameters_schema()
            );
        }
        if !ctx.dispatcher_instructions.is_empty() {
            out.push('\n');
            out.push_str(ctx.dispatcher_instructions);
        }
        Ok(out)
    }
}

impl PromptSection for SafetySection {
    fn name(&self) -> &str {
        "safety"
    }

    fn build(&self, _ctx: &PromptContext<'_>) -> Result<String> {
        Ok("## Safety\n\n- Do not exfiltrate private data.\n- Do not run destructive commands without asking.\n- Do not bypass oversight or approval mechanisms.\n- Prefer `trash` over `rm`.\n- When in doubt, ask before acting externally.".into())
    }
}

impl PromptSection for SkillsSection {
    fn name(&self) -> &str {
        "skills"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        if ctx.skills.is_empty() {
            return Ok(String::new());
        }

        let mut prompt = String::from("## Available Skills\n\n<available_skills>\n");
        for skill in ctx.skills {
            let location = skill.location.clone().unwrap_or_else(|| {
                ctx.workspace_dir
                    .join("skills")
                    .join(&skill.name)
                    .join("SKILL.md")
            });
            let _ = writeln!(
                prompt,
                "  <skill>\n    <name>{}</name>\n    <description>{}</description>\n    <location>{}</location>\n  </skill>",
                skill.name,
                skill.description,
                location.display()
            );
        }
        prompt.push_str("</available_skills>");
        Ok(prompt)
    }
}

impl PromptSection for WorkspaceSection {
    fn name(&self) -> &str {
        "workspace"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        Ok(format!(
            "## Workspace\n\nWorking directory: `{}`",
            ctx.workspace_dir.display()
        ))
    }
}

impl PromptSection for RuntimeSection {
    fn name(&self) -> &str {
        "runtime"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        let host =
            hostname::get().map_or_else(|_| "unknown".into(), |h| h.to_string_lossy().to_string());
        Ok(format!(
            "## Runtime\n\nHost: {host} | OS: {} | Model: {}",
            std::env::consts::OS,
            ctx.model_name
        ))
    }
}

impl PromptSection for DateTimeSection {
    fn name(&self) -> &str {
        "datetime"
    }

    fn build(&self, _ctx: &PromptContext<'_>) -> Result<String> {
        let now = Local::now();
        Ok(format!(
            "## Current Date & Time\n\nTimezone: {}",
            now.format("%Z")
        ))
    }
}

pub fn load_prompt_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| format!("Failed to load prompt file: {}", path.display()))
}

pub fn resolve_includes(content: &str, base_path: &Path) -> Result<String> {
    let include_pattern = regex::Regex::new(r"\{\{INCLUDE:(.+?)\}\}")?;
    let mut result = content.to_string();

    for cap in include_pattern.captures_iter(content) {
        let include_path = &cap[1];
        let full_path = if Path::new(include_path).is_absolute() {
            PathBuf::from(include_path)
        } else {
            base_path.join(include_path)
        };

        if full_path.exists() {
            let included = load_prompt_file(&full_path)?;
            let resolved = resolve_includes(&included, full_path.parent().unwrap())?;
            result = result.replace(&cap[0], &resolved);
        } else {
            result = result.replace(
                &cap[0],
                &format!("[Include not found: {}]", include_path),
            );
        }
    }

    Ok(result)
}

pub fn resolve_dynamic_variables(content: &str, ctx: &PromptContext<'_>) -> String {
    let mut result = content.to_string();

    let dynamic_pattern = regex::Regex::new(r"\{\{DYNAMIC:(.+?)\}\}").unwrap();

    for cap in dynamic_pattern.captures_iter(content) {
        let var_name = &cap[1];
        let value = ctx.get_variable(var_name).cloned().unwrap_or_else(|| {
            match var_name {
                "model_name" => ctx.model_name.to_string(),
                "workspace_dir" => ctx.workspace_dir.display().to_string(),
                "reasoning_mode" => ctx.reasoning_mode.as_str().to_string(),
                "current_date" => Local::now().format("%Y-%m-%d").to_string(),
                "current_time" => Local::now().format("%H:%M:%S").to_string(),
                "os" => std::env::consts::OS.to_string(),
                _ => format!("[Unknown variable: {}]", var_name),
            }
        });
        result = result.replace(&cap[0], &value);
    }

    result
}

pub fn select_reasoning_prompt(mode: ReasoningMode, workspace_dir: &Path) -> Result<String> {
    if mode == ReasoningMode::Standard {
        return Ok(String::new());
    }

    let src_prompts_path = workspace_dir
        .join("src")
        .join("housaky")
        .join("prompts")
        .join(mode.prompt_filename());

    let core_prompts_path = workspace_dir
        .join("prompts")
        .join("core")
        .join(mode.prompt_filename());

    if src_prompts_path.exists() {
        let content = load_prompt_file(&src_prompts_path)?;
        return resolve_includes(&content, src_prompts_path.parent().unwrap());
    }

    if core_prompts_path.exists() {
        let content = load_prompt_file(&core_prompts_path)?;
        return resolve_includes(&content, core_prompts_path.parent().unwrap());
    }

    let default_prompt = match mode {
        ReasoningMode::ChainOfThought => include_str!("../housaky/prompts/cot_prompt.md"),
        ReasoningMode::ReAct => include_str!("../housaky/prompts/react_prompt.md"),
        ReasoningMode::TreeOfThought => include_str!("../housaky/prompts/tot_prompt.md"),
        ReasoningMode::MetaCognitive => include_str!("../housaky/prompts/meta_cognition_prompt.md"),
        ReasoningMode::GoalDecomposition => include_str!("../housaky/prompts/goal_decomposition_prompt.md"),
        ReasoningMode::Standard => "",
    };

    Ok(default_prompt.to_string())
}

fn inject_workspace_file(prompt: &mut String, workspace_dir: &Path, filename: &str) {
    let path = workspace_dir.join(filename);
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                return;
            }
            let _ = writeln!(prompt, "### {filename}\n");
            let truncated = if trimmed.chars().count() > BOOTSTRAP_MAX_CHARS {
                trimmed
                    .char_indices()
                    .nth(BOOTSTRAP_MAX_CHARS)
                    .map(|(idx, _)| &trimmed[..idx])
                    .unwrap_or(trimmed)
            } else {
                trimmed
            };
            prompt.push_str(truncated);
            if truncated.len() < trimmed.len() {
                let _ = writeln!(
                    prompt,
                    "\n\n[... truncated at {BOOTSTRAP_MAX_CHARS} chars — use `read` for full file]\n"
                );
            } else {
                prompt.push_str("\n\n");
            }
        }
        Err(_) => {
            let _ = writeln!(prompt, "### {filename}\n\n[File not found: {filename}]\n");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::traits::Tool;
    use async_trait::async_trait;

    struct TestTool;

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            "test_tool"
        }

        fn description(&self) -> &str {
            "tool desc"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }

        async fn execute(
            &self,
            _args: serde_json::Value,
        ) -> anyhow::Result<crate::tools::ToolResult> {
            Ok(crate::tools::ToolResult {
                success: true,
                output: "ok".into(),
                error: None,
            })
        }
    }

    #[test]
    fn prompt_builder_assembles_sections() {
        let tools: Vec<Box<dyn Tool>> = vec![Box::new(TestTool)];
        let ctx = PromptContext {
            workspace_dir: Path::new("/tmp"),
            model_name: "test-model",
            tools: &tools,
            skills: &[],
            identity_config: None,
            dispatcher_instructions: "instr",
            reasoning_mode: ReasoningMode::default(),
            dynamic_variables: HashMap::new(),
        };
        let prompt = SystemPromptBuilder::with_defaults().build(&ctx).unwrap();
        assert!(prompt.contains("## Tools"));
        assert!(prompt.contains("test_tool"));
        assert!(prompt.contains("instr"));
    }

    #[test]
    fn reasoning_mode_from_str() {
        assert_eq!(ReasoningMode::from_str("cot"), ReasoningMode::ChainOfThought);
        assert_eq!(ReasoningMode::from_str("react"), ReasoningMode::ReAct);
        assert_eq!(ReasoningMode::from_str("tot"), ReasoningMode::TreeOfThought);
        assert_eq!(ReasoningMode::from_str("unknown"), ReasoningMode::Standard);
    }

    #[test]
    fn prompt_version_default() {
        let version = PromptVersion::default();
        assert_eq!(version.version, "1.0.0");
        assert!(version.changes.is_empty());
    }

    #[test]
    fn prompt_manager_caching() {
        let mut manager = PromptManager::new("/tmp");
        manager.set("test_key".to_string(), "test_value".to_string());
        assert_eq!(manager.get("test_key"), Some("test_value".to_string()));
        manager.invalidate("test_key");
        assert!(manager.get("test_key").is_none());
    }

    #[test]
    fn resolve_dynamic_variables_basic() {
        let tools: Vec<Box<dyn Tool>> = vec![];
        let ctx = PromptContext {
            workspace_dir: Path::new("/test/workspace"),
            model_name: "test-model",
            tools: &tools,
            skills: &[],
            identity_config: None,
            dispatcher_instructions: "",
            reasoning_mode: ReasoningMode::ChainOfThought,
            dynamic_variables: HashMap::new(),
        };

        let content = "Model: {{DYNAMIC:model_name}}, Mode: {{DYNAMIC:reasoning_mode}}";
        let resolved = resolve_dynamic_variables(content, &ctx);
        assert!(resolved.contains("test-model"));
        assert!(resolved.contains("cot"));
    }
}

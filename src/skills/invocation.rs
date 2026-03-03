use crate::housaky::cognitive::perception::PerceivedInput;
use crate::skills::Skill;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTrigger {
    pub skill_name: String,
    pub trigger_type: TriggerType,
    pub patterns: Vec<String>,
    pub contexts: Vec<String>,
    pub commands: Vec<String>,
    pub min_confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TriggerType {
    Intent,
    Context,
    Command,
    Pattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInvocation {
    pub skill_name: String,
    pub trigger_reason: String,
    pub context: HashMap<String, String>,
    pub invoked_at: chrono::DateTime<chrono::Utc>,
}

pub struct SkillInvocationEngine {
    workspace_dir: PathBuf,
    triggers: Arc<RwLock<Vec<SkillTrigger>>>,
    invocation_history: Arc<RwLock<Vec<SkillInvocation>>>,
}

impl SkillInvocationEngine {
    pub fn new(workspace_dir: &PathBuf) -> Self {
        Self {
            workspace_dir: workspace_dir.clone(),
            triggers: Arc::new(RwLock::new(Vec::new())),
            invocation_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn initialize(&self, skills: &[Skill]) -> Result<()> {
        let mut triggers = Vec::new();

        for skill in skills {
            if let Some(trigger) = self.create_trigger_from_skill(skill) {
                triggers.push(trigger);
            }
        }

        let mut t = self.triggers.write().await;
        *t = triggers;

        info!(
            "SkillInvocationEngine initialized with {} triggers",
            self.triggers.read().await.len()
        );
        Ok(())
    }

    fn create_trigger_from_skill(&self, skill: &Skill) -> Option<SkillTrigger> {
        let skill_path = skill.location.as_ref()?;

        let triggers = self.parse_skill_triggers(skill_path, &skill.name)?;
        Some(SkillTrigger {
            skill_name: skill.name.clone(),
            trigger_type: TriggerType::Intent,
            patterns: triggers.0,
            contexts: triggers.1,
            commands: triggers.2,
            min_confidence: 0.5,
        })
    }

    fn parse_skill_triggers(
        &self,
        skill_path: &PathBuf,
        skill_name: &str,
    ) -> Option<(Vec<String>, Vec<String>, Vec<String>)> {
        let md_path = skill_path.join("SKILL.md");
        if !md_path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&md_path).ok()?;

        let mut patterns = Vec::new();
        let mut contexts = Vec::new();
        let mut commands = Vec::new();

        for line in content.lines() {
            if line.starts_with("triggers:") || line.starts_with("  actions:") {
                if let Some(actions) = line.split(':').nth(1) {
                    for action in actions.trim().split(|c| c == '[' || c == ']' || c == ',') {
                        let action = action.trim().trim_matches('"').trim();
                        if !action.is_empty() && action != "actions" {
                            patterns.push(action.to_string());
                        }
                    }
                }
            }
            if line.starts_with("  contexts:") {
                if let Some(ctxs) = line.split(':').nth(1) {
                    for ctx in ctxs.trim().split(|c| c == '[' || c == ']' || c == ',') {
                        let ctx = ctx.trim().trim_matches('"').trim();
                        if !ctx.is_empty() && ctx != "contexts" {
                            contexts.push(ctx.to_string());
                        }
                    }
                }
            }
            if line.starts_with("  commands:") {
                if let Some(cmds) = line.split(':').nth(1) {
                    for cmd in cmds.trim().split(|c| c == '[' || c == ']' || c == ',') {
                        let cmd = cmd.trim().trim_matches('"').trim();
                        if !cmd.is_empty() && cmd != "commands" {
                            commands.push(cmd.to_string());
                        }
                    }
                }
            }
        }

        if patterns.is_empty() && contexts.is_empty() && commands.is_empty() {
            patterns = vec![skill_name.replace('-', " "), skill_name.replace('_', " ")];
        }

        Some((patterns, contexts, commands))
    }

    pub async fn check_and_invoke(
        &self,
        perception: &PerceivedInput,
        user_input: &str,
        current_context: &[String],
    ) -> Option<SkillInvocation> {
        let triggers = self.triggers.read().await;

        for trigger in triggers.iter() {
            if self.should_invoke(trigger, perception, user_input, current_context) {
                let invocation = SkillInvocation {
                    skill_name: trigger.skill_name.clone(),
                    trigger_reason: format!(
                        "Intent: {:?}, Pattern match in input",
                        perception.intent.primary
                    ),
                    context: HashMap::from([
                        ("user_input".to_string(), user_input.to_string()),
                        (
                            "intent".to_string(),
                            format!("{:?}", perception.intent.primary),
                        ),
                        (
                            "confidence".to_string(),
                            perception.intent.confidence.to_string(),
                        ),
                    ]),
                    invoked_at: chrono::Utc::now(),
                };

                let mut history = self.invocation_history.write().await;
                history.push(invocation.clone());

                if history.len() > 100 {
                    history.remove(0);
                }

                info!(
                    "Invoking skill: {} due to: {}",
                    invocation.skill_name, invocation.trigger_reason
                );
                return Some(invocation);
            }
        }

        None
    }

    fn should_invoke(
        &self,
        trigger: &SkillTrigger,
        perception: &PerceivedInput,
        user_input: &str,
        current_context: &[String],
    ) -> bool {
        if perception.intent.confidence < trigger.min_confidence {
            return false;
        }

        let input_lower = user_input.to_lowercase();

        for pattern in &trigger.patterns {
            let pattern_lower = pattern.to_lowercase();
            if input_lower.contains(&pattern_lower) {
                return true;
            }
        }

        for cmd in &trigger.commands {
            if user_input.starts_with(cmd) || user_input.starts_with(&cmd[1..]) {
                return true;
            }
        }

        for ctx in &trigger.contexts {
            let ctx_lower = ctx.to_lowercase();
            for current in current_context {
                if current.to_lowercase().contains(&ctx_lower) {
                    return true;
                }
            }
        }

        false
    }

    pub async fn get_invocation_history(&self) -> Vec<SkillInvocation> {
        self.invocation_history.read().await.clone()
    }

    pub async fn clear_history(&self) {
        let mut history = self.invocation_history.write().await;
        history.clear();
    }

    pub async fn get_active_triggers(&self) -> Vec<SkillTrigger> {
        self.triggers.read().await.clone()
    }

    pub async fn add_trigger(&self, trigger: SkillTrigger) {
        let mut triggers = self.triggers.write().await;
        triggers.push(trigger);
    }

    pub async fn remove_trigger(&self, skill_name: &str) {
        let mut triggers = self.triggers.write().await;
        triggers.retain(|t| t.skill_name != skill_name);
    }
}

pub fn create_skill_invocation_context(
    perception: &PerceivedInput,
    invocation: &SkillInvocation,
) -> String {
    let mut context = String::new();

    context.push_str("## Active Skill Invocation\n\n");
    context.push_str(&format!(
        "**Skill:** {}\n**Reason:** {}\n\n",
        invocation.skill_name, invocation.trigger_reason
    ));

    context.push_str("### Skill Context\n");
    for (key, value) in &invocation.context {
        context.push_str(&format!("- {}: {}\n", key, value));
    }
    context.push('\n');

    context.push_str("### Detected Intent\n");
    context.push_str(&format!("- Primary: {:?}\n", perception.intent.primary));
    context.push_str(&format!(
        "- Confidence: {:.0}%\n",
        perception.intent.confidence * 100.0
    ));
    context.push_str(&format!("- Topics: {:?}\n", perception.topics));

    context
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_parsing() {
        let engine = SkillInvocationEngine::new(&PathBuf::from("/tmp"));
        let skill = Skill {
            name: "test-skill".to_string(),
            description: "Test skill".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            tags: vec![],
            tools: vec![],
            prompts: vec![],
            location: Some(PathBuf::from("/tmp/test-skill")),
        };

        let trigger = engine.create_trigger_from_skill(&skill);
        assert!(trigger.is_some());
    }
}

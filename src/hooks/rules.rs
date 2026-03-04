//! Markdown-based hook rules engine.
//!
//! Inspired by claude-code's hookify plugin: users drop `.md` files with YAML frontmatter
//! into `.housaky/hooks/` to define `warn` or `block` rules without writing Rust code.
//!
//! # Rule file format
//!
//! ```markdown
//! ---
//! name: block-dangerous-rm
//! enabled: true
//! event: pre_tool_use
//! tool: shell
//! pattern: rm\s+-rf
//! action: block
//! ---
//!
//! ⚠️ **Dangerous rm -rf detected!**
//! Please verify you have backups before proceeding.
//! ```
//!
//! # Supported events
//! - `pre_tool_use` — fires before a tool executes (can block)
//! - `post_tool_use` — fires after a tool executes
//! - `stop` — fires when agent is about to finish (can block)
//! - `user_prompt_submit` — fires when user submits a message
//! - `all` — fires for all events
//!
//! # Supported actions
//! - `warn` — inject a system message but allow the operation
//! - `block` — prevent the operation from executing
//!
//! # Supported operators
//! - `regex_match` (default) — pattern is a regex matched against the field
//! - `contains` — literal substring match
//! - `equals` — exact match
//! - `not_contains` — field must NOT contain pattern
//! - `starts_with` / `ends_with` — prefix/suffix match

use crate::hooks::types::{HookEvent, HookEventType, HookResult};
use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleAction {
    Warn,
    Block,
}

impl Default for RuleAction {
    fn default() -> Self {
        RuleAction::Warn
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCondition {
    /// Field to check: "command", "file_path", "new_text", "old_text", "content", "user_prompt"
    pub field: String,
    /// Operator: "regex_match", "contains", "equals", "not_contains", "starts_with", "ends_with"
    #[serde(default = "default_operator")]
    pub operator: String,
    /// Pattern to evaluate
    pub pattern: String,
}

fn default_operator() -> String {
    "regex_match".to_string()
}

/// A parsed hook rule loaded from a `.md` file.
#[derive(Debug, Clone)]
pub struct HookRule {
    pub name: String,
    pub enabled: bool,
    /// Event filter: "pre_tool_use", "post_tool_use", "stop", "user_prompt_submit", "all"
    pub event: String,
    /// Optional tool name filter (e.g. "shell", "file_write")
    pub tool: Option<String>,
    pub conditions: Vec<RuleCondition>,
    pub action: RuleAction,
    /// The markdown body used as the message shown to the agent
    pub message: String,
    /// Source file path for debugging
    pub source: PathBuf,
}

// ── YAML frontmatter parser ───────────────────────────────────────────────────

/// Minimal YAML frontmatter extractor — avoids pulling in a full YAML dep.
fn extract_frontmatter(content: &str) -> (HashMap<String, FmValue>, String) {
    if !content.starts_with("---") {
        return (HashMap::new(), content.to_string());
    }
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return (HashMap::new(), content.to_string());
    }
    let fm_text = parts[1];
    let body = parts[2].trim().to_string();
    let fm = parse_simple_yaml(fm_text);
    (fm, body)
}

#[derive(Debug, Clone)]
enum FmValue {
    Str(String),
    Bool(bool),
    List(Vec<HashMap<String, String>>),
}

fn parse_simple_yaml(text: &str) -> HashMap<String, FmValue> {
    let mut map: HashMap<String, FmValue> = HashMap::new();
    let mut current_list_key: Option<String> = None;
    let mut current_list: Vec<HashMap<String, String>> = Vec::new();
    let mut current_item: HashMap<String, String> = HashMap::new();

    for line in text.lines() {
        let stripped = line.trim();
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }

        let indent = line.len() - line.trim_start().len();

        if indent == 0 && !stripped.starts_with('-') {
            // Flush pending list
            if let Some(key) = current_list_key.take() {
                if !current_item.is_empty() {
                    current_list.push(std::mem::take(&mut current_item));
                }
                if !current_list.is_empty() {
                    map.insert(key, FmValue::List(std::mem::take(&mut current_list)));
                }
            }

            if let Some((k, v)) = stripped.split_once(':') {
                let k = k.trim().to_string();
                let v = v.trim().trim_matches('"').trim_matches('\'').to_string();
                if v.is_empty() {
                    current_list_key = Some(k);
                    current_list = Vec::new();
                    current_item = HashMap::new();
                } else {
                    let value = match v.to_lowercase().as_str() {
                        "true" => FmValue::Bool(true),
                        "false" => FmValue::Bool(false),
                        _ => FmValue::Str(v),
                    };
                    map.insert(k, value);
                }
            }
        } else if stripped.starts_with('-') && current_list_key.is_some() {
            if !current_item.is_empty() {
                current_list.push(std::mem::take(&mut current_item));
            }
            let rest = stripped[1..].trim();
            if let Some((k, v)) = rest.split_once(':') {
                current_item.insert(
                    k.trim().to_string(),
                    v.trim().trim_matches('"').trim_matches('\'').to_string(),
                );
            }
        } else if indent > 0 && current_list_key.is_some() {
            if let Some((k, v)) = stripped.split_once(':') {
                current_item.insert(
                    k.trim().to_string(),
                    v.trim().trim_matches('"').trim_matches('\'').to_string(),
                );
            }
        }
    }

    // Flush remaining list
    if let Some(key) = current_list_key.take() {
        if !current_item.is_empty() {
            current_list.push(current_item);
        }
        if !current_list.is_empty() {
            map.insert(key, FmValue::List(current_list));
        }
    }

    map
}

fn fm_str<'a>(map: &'a HashMap<String, FmValue>, key: &str) -> Option<&'a str> {
    match map.get(key) {
        Some(FmValue::Str(s)) => Some(s.as_str()),
        _ => None,
    }
}

fn fm_bool(map: &HashMap<String, FmValue>, key: &str, default: bool) -> bool {
    match map.get(key) {
        Some(FmValue::Bool(b)) => *b,
        Some(FmValue::Str(s)) => s.to_lowercase() == "true",
        _ => default,
    }
}

// ── Rule loading ──────────────────────────────────────────────────────────────

/// Load all `.md` rule files from `<workspace>/.housaky/hooks/`.
pub fn load_rules(workspace_dir: &Path) -> Vec<HookRule> {
    let hooks_dir = workspace_dir.join(".housaky").join("hooks");
    if !hooks_dir.exists() {
        return Vec::new();
    }

    let mut rules = Vec::new();
    let Ok(entries) = std::fs::read_dir(&hooks_dir) else {
        return rules;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        match load_rule_file(&path) {
            Ok(Some(rule)) => rules.push(rule),
            Ok(None) => {}
            Err(e) => warn!("Failed to load hook rule {:?}: {}", path, e),
        }
    }

    rules
}

fn load_rule_file(path: &Path) -> anyhow::Result<Option<HookRule>> {
    let content = std::fs::read_to_string(path)?;
    let (fm, body) = extract_frontmatter(&content);

    if fm.is_empty() {
        return Ok(None);
    }

    let name = fm_str(&fm, "name")
        .unwrap_or_else(|| path.file_stem().and_then(|s| s.to_str()).unwrap_or("unnamed"))
        .to_string();

    let enabled = fm_bool(&fm, "enabled", true);
    if !enabled {
        return Ok(None);
    }

    let event = fm_str(&fm, "event").unwrap_or("all").to_string();
    let tool = fm_str(&fm, "tool").map(|s| s.to_string());

    let action = match fm_str(&fm, "action").unwrap_or("warn") {
        "block" => RuleAction::Block,
        _ => RuleAction::Warn,
    };

    // Build conditions: explicit list takes priority, else simple pattern
    let mut conditions: Vec<RuleCondition> = Vec::new();

    if let Some(FmValue::List(cond_list)) = fm.get("conditions") {
        for item in cond_list {
            if let (Some(field), Some(pattern)) = (item.get("field"), item.get("pattern")) {
                conditions.push(RuleCondition {
                    field: field.clone(),
                    operator: item
                        .get("operator")
                        .cloned()
                        .unwrap_or_else(|| "regex_match".to_string()),
                    pattern: pattern.clone(),
                });
            }
        }
    }

    if conditions.is_empty() {
        if let Some(pattern) = fm_str(&fm, "pattern") {
            let field = infer_field_for_event(&event);
            conditions.push(RuleCondition {
                field: field.to_string(),
                operator: "regex_match".to_string(),
                pattern: pattern.to_string(),
            });
        }
    }

    if conditions.is_empty() {
        debug!("Hook rule {:?} has no conditions, skipping", path);
        return Ok(None);
    }

    Ok(Some(HookRule {
        name,
        enabled,
        event,
        tool,
        conditions,
        action,
        message: body,
        source: path.to_path_buf(),
    }))
}

fn infer_field_for_event(event: &str) -> &'static str {
    match event {
        "pre_tool_use" | "post_tool_use" => "command",
        "user_prompt_submit" => "user_prompt",
        _ => "content",
    }
}

// ── Rule evaluation ───────────────────────────────────────────────────────────

/// Evaluate rules against a hook event and return a combined HookResult.
/// Blocking rules take priority over warning rules.
pub fn evaluate_rules(rules: &[HookRule], event: &HookEvent) -> HookResult {
    let event_str = event.event_type.as_str();

    let mut blocking: Vec<&HookRule> = Vec::new();
    let mut warnings: Vec<&HookRule> = Vec::new();

    for rule in rules {
        if !event_matches_rule(event_str, &rule.event) {
            continue;
        }
        if let Some(context) = &event.context {
            if rule_matches(rule, context) {
                match rule.action {
                    RuleAction::Block => blocking.push(rule),
                    RuleAction::Warn => warnings.push(rule),
                }
            }
        }
    }

    if !blocking.is_empty() {
        let msgs: Vec<String> = blocking
            .iter()
            .map(|r| format!("**[{}]** {}", r.name, r.message))
            .collect();
        return HookResult::block(msgs.join("\n\n"));
    }

    if !warnings.is_empty() {
        let msgs: Vec<String> = warnings
            .iter()
            .map(|r| format!("**[{}]** {}", r.name, r.message))
            .collect();
        return HookResult::with_messages(msgs);
    }

    HookResult::continue_result()
}

fn event_matches_rule(event_type: &str, rule_event: &str) -> bool {
    rule_event == "all" || rule_event == event_type
}

fn rule_matches(rule: &HookRule, context: &serde_json::Value) -> bool {
    // Tool filter
    if let Some(ref tool_filter) = rule.tool {
        let tool_name = context
            .get("tool")
            .or_else(|| context.get("tool_name"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !tool_filter.is_empty() && tool_filter != "*" && tool_filter != tool_name {
            return false;
        }
    }

    // All conditions must match
    rule.conditions
        .iter()
        .all(|cond| condition_matches(cond, context))
}

fn condition_matches(cond: &RuleCondition, context: &serde_json::Value) -> bool {
    let value = extract_field(&cond.field, context);
    let value = match value {
        Some(v) => v,
        None => return false,
    };

    match cond.operator.as_str() {
        "regex_match" => regex_match(&cond.pattern, &value),
        "contains" => value.contains(cond.pattern.as_str()),
        "equals" => value == cond.pattern,
        "not_contains" => !value.contains(cond.pattern.as_str()),
        "starts_with" => value.starts_with(cond.pattern.as_str()),
        "ends_with" => value.ends_with(cond.pattern.as_str()),
        _ => false,
    }
}

fn extract_field(field: &str, context: &serde_json::Value) -> Option<String> {
    // Direct key lookup first
    if let Some(v) = context.get(field) {
        return Some(match v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        });
    }

    // Semantic aliases
    match field {
        "command" => context
            .get("args")
            .and_then(|a| a.get("command"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        "file_path" => context
            .get("args")
            .and_then(|a| a.get("path").or_else(|| a.get("file_path")))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        "new_text" => context
            .get("args")
            .and_then(|a| a.get("new_string").or_else(|| a.get("content")))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        "old_text" => context
            .get("args")
            .and_then(|a| a.get("old_string"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        "user_prompt" => context
            .get("message")
            .or_else(|| context.get("user_prompt"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}

fn regex_match(pattern: &str, text: &str) -> bool {
    match Regex::new(pattern) {
        Ok(re) => re.is_match(text),
        Err(e) => {
            warn!("Invalid regex pattern '{}': {}", pattern, e);
            false
        }
    }
}

// ── Built-in hook that loads markdown rules ───────────────────────────────────

/// Hook that evaluates user-defined markdown rules from `.housaky/hooks/*.md`.
///
/// Drop a `.md` file in `.housaky/hooks/` to define custom warn/block rules
/// without writing any Rust code. Rules are reloaded on each trigger.
pub struct MarkdownRulesHook {
    workspace_dir: PathBuf,
}

impl MarkdownRulesHook {
    #[must_use]
    pub fn new(workspace_dir: impl Into<PathBuf>) -> Self {
        Self {
            workspace_dir: workspace_dir.into(),
        }
    }
}

#[async_trait]
impl crate::hooks::types::Hook for MarkdownRulesHook {
    fn id(&self) -> &str {
        "builtin:markdown_rules"
    }

    fn name(&self) -> &str {
        "Markdown Rules Hook"
    }

    fn events(&self) -> Vec<(HookEventType, Vec<String>)> {
        vec![
            (HookEventType::PreToolUse, vec![]),
            (HookEventType::PostToolUse, vec![]),
            (HookEventType::Stop, vec![]),
            (HookEventType::UserPromptSubmit, vec![]),
        ]
    }

    async fn handle(
        &self,
        event: HookEvent,
    ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        let rules = load_rules(&self.workspace_dir);
        if rules.is_empty() {
            return Ok(HookResult::continue_result());
        }
        Ok(evaluate_rules(&rules, &event))
    }

    fn priority(&self) -> i32 {
        5 // Run before all other hooks
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_event(event_type: HookEventType, context: serde_json::Value) -> HookEvent {
        HookEvent {
            event_type,
            action: "execute".to_string(),
            session_key: None,
            context: Some(context),
            timestamp: Utc::now(),
            messages: vec![],
        }
    }

    #[test]
    fn test_regex_match_condition() {
        let rule = HookRule {
            name: "test".to_string(),
            enabled: true,
            event: "pre_tool_use".to_string(),
            tool: None,
            conditions: vec![RuleCondition {
                field: "command".to_string(),
                operator: "regex_match".to_string(),
                pattern: r"rm\s+-rf".to_string(),
            }],
            action: RuleAction::Block,
            message: "Dangerous!".to_string(),
            source: PathBuf::from("test.md"),
        };

        let ctx = serde_json::json!({"args": {"command": "rm -rf /tmp"}});
        let event = make_event(HookEventType::PreToolUse, ctx);
        let result = evaluate_rules(&[rule], &event);
        assert!(result.is_blocked());
    }

    #[test]
    fn test_safe_command_passes() {
        let rule = HookRule {
            name: "test".to_string(),
            enabled: true,
            event: "pre_tool_use".to_string(),
            tool: None,
            conditions: vec![RuleCondition {
                field: "command".to_string(),
                operator: "regex_match".to_string(),
                pattern: r"rm\s+-rf".to_string(),
            }],
            action: RuleAction::Block,
            message: "Dangerous!".to_string(),
            source: PathBuf::from("test.md"),
        };

        let ctx = serde_json::json!({"args": {"command": "ls -la"}});
        let event = make_event(HookEventType::PreToolUse, ctx);
        let result = evaluate_rules(&[rule], &event);
        assert!(!result.is_blocked());
    }

    #[test]
    fn test_warn_does_not_block() {
        let rule = HookRule {
            name: "warn-test".to_string(),
            enabled: true,
            event: "pre_tool_use".to_string(),
            tool: None,
            conditions: vec![RuleCondition {
                field: "command".to_string(),
                operator: "contains".to_string(),
                pattern: "sudo".to_string(),
            }],
            action: RuleAction::Warn,
            message: "Using sudo".to_string(),
            source: PathBuf::from("test.md"),
        };

        let ctx = serde_json::json!({"args": {"command": "sudo apt update"}});
        let event = make_event(HookEventType::PreToolUse, ctx);
        let result = evaluate_rules(&[rule], &event);
        assert!(!result.is_blocked());
        assert!(!result.messages.is_empty());
    }

    #[test]
    fn test_event_filter() {
        let rule = HookRule {
            name: "stop-only".to_string(),
            enabled: true,
            event: "stop".to_string(),
            tool: None,
            conditions: vec![RuleCondition {
                field: "user_prompt".to_string(),
                operator: "contains".to_string(),
                pattern: "test".to_string(),
            }],
            action: RuleAction::Block,
            message: "Stop blocked".to_string(),
            source: PathBuf::from("test.md"),
        };

        // Should NOT fire for pre_tool_use
        let ctx = serde_json::json!({"user_prompt": "test something"});
        let event = make_event(HookEventType::PreToolUse, ctx);
        let result = evaluate_rules(&[rule], &event);
        assert!(!result.is_blocked());
    }

    #[test]
    fn test_frontmatter_parse() {
        let content = "---\nname: my-rule\nenabled: true\nevent: pre_tool_use\npattern: rm\naction: block\n---\n\nDangerous!";
        let (fm, body) = extract_frontmatter(content);
        assert_eq!(fm_str(&fm, "name"), Some("my-rule"));
        assert_eq!(fm_bool(&fm, "enabled", false), true);
        assert_eq!(fm_str(&fm, "event"), Some("pre_tool_use"));
        assert_eq!(fm_str(&fm, "action"), Some("block"));
        assert!(body.contains("Dangerous!"));
    }
}

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

use super::{Skill, SkillManifest, SkillMeta};

/// Claude Code SKILL.md YAML frontmatter structure.
#[derive(Debug, Clone, Deserialize)]
pub struct ClaudeSkillFrontmatter {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub triggers: Option<ClaudeTriggers>,
    #[serde(default, rename = "tools_allowed")]
    pub tools_allowed: Option<Vec<String>>,
    #[serde(default, rename = "tools_restricted")]
    pub tools_restricted: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClaudeTriggers {
    #[serde(default)]
    pub actions: Option<Vec<String>>,
    #[serde(default)]
    pub contexts: Option<Vec<String>>,
    #[serde(default)]
    pub commands: Option<Vec<String>>,
    #[serde(default)]
    pub projects: Option<Vec<String>>,
    #[serde(default)]
    pub elements: Option<Vec<String>>,
    #[serde(default)]
    pub styles: Option<Vec<String>>,
}

/// Parse a Claude SKILL.md file with YAML frontmatter.
///
/// Returns (frontmatter, body_markdown).
pub fn parse_claude_skill_md(content: &str) -> Result<(ClaudeSkillFrontmatter, String)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Err(anyhow!(
            "Claude SKILL.md missing YAML frontmatter (expected starting ---)"
        ));
    }

    // Find closing delimiter. Use a conservative line-based parse: frontmatter must be between
    // first line '---' and next line that is exactly '---'.
    let mut lines = trimmed.lines();
    let first = lines.next().unwrap_or("");
    if first.trim() != "---" {
        return Err(anyhow!("Invalid frontmatter start delimiter"));
    }

    let mut yaml_lines: Vec<&str> = Vec::new();
    let mut found_end = false;
    for line in &mut lines {
        if line.trim() == "---" {
            found_end = true;
            break;
        }
        yaml_lines.push(line);
    }

    if !found_end {
        return Err(anyhow!("Unclosed YAML frontmatter (missing ending ---)"));
    }

    let yaml_str = yaml_lines.join("\n");
    let body = lines.collect::<Vec<_>>().join("\n");
    let body = body.trim().to_string();

    let frontmatter: ClaudeSkillFrontmatter = serde_yaml::from_str(&yaml_str)
        .with_context(|| "Failed to parse Claude SKILL.md YAML frontmatter")?;

    Ok((frontmatter, body))
}

/// Convert a Claude SKILL.md file to a Housaky Skill struct.
///
/// Notes:
/// - Housaky's current `Skill` supports tags, prompts, tools. For now, we map triggers into tags.
/// - We store the markdown body as a prompt.
pub fn claude_skill_md_to_housaky_skill(content: &str) -> Result<Skill> {
    let (fm, body) = parse_claude_skill_md(content)?;

    let mut tags: Vec<String> = Vec::new();
    if let Some(triggers) = &fm.triggers {
        if let Some(actions) = &triggers.actions {
            tags.extend(actions.iter().map(|a| format!("action:{a}")));
        }
        if let Some(contexts) = &triggers.contexts {
            tags.extend(contexts.iter().map(|c| format!("ctx:{c}")));
        }
        if let Some(projects) = &triggers.projects {
            tags.extend(projects.iter().map(|p| format!("project:{p}")));
        }
        if let Some(elements) = &triggers.elements {
            tags.extend(elements.iter().map(|e| format!("ui:{e}")));
        }
        if let Some(styles) = &triggers.styles {
            tags.extend(styles.iter().map(|s| format!("style:{s}")));
        }
    }

    // Expose tool allow/deny lists as tags (until we implement a real permission system).
    if let Some(allowed) = &fm.tools_allowed {
        tags.extend(allowed.iter().map(|t| format!("tools_allowed:{t}")));
    }
    if let Some(restricted) = &fm.tools_restricted {
        tags.extend(restricted.iter().map(|t| format!("tools_restricted:{t}")));
    }

    Ok(Skill {
        name: fm.name,
        description: fm.description,
        version: fm.version.unwrap_or_else(|| "0.1.0".to_string()),
        author: fm.author,
        tags,
        tools: Vec::new(),
        prompts: vec![body],
        location: None,
    })
}

/// Convert Claude SKILL.md into a Housaky `SKILL.toml` string.
///
/// This is useful for "vendor" installs so we can load skills fast without re-parsing YAML.
pub fn claude_skill_md_to_skill_toml(content: &str) -> Result<String> {
    let skill = claude_skill_md_to_housaky_skill(content)?;

    let manifest = SkillManifest {
        skill: SkillMeta {
            name: skill.name,
            description: skill.description,
            version: skill.version,
            author: skill.author,
            tags: skill.tags,
        },
        tools: vec![],
        prompts: skill.prompts,
    };

    toml::to_string_pretty(&manifest).map_err(|e| anyhow!("TOML serialization failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_claude_skill_frontmatter() {
        let src = r#"---
name: code-review
description: Review code for best practices
version: "1.0.0"
author: Dev
triggers:
  actions: [review, check]
  contexts: [debugging]
tools_allowed: [read, write]
---

# Code Review Skill

Do the thing.
"#;

        let (fm, body) = parse_claude_skill_md(src).unwrap();
        assert_eq!(fm.name, "code-review");
        assert_eq!(fm.description, "Review code for best practices");
        assert_eq!(fm.version.as_deref(), Some("1.0.0"));
        assert!(body.contains("# Code Review Skill"));
    }

    #[test]
    fn to_skill_toml_contains_tags() {
        let src = r#"---
name: x
description: y
triggers:
  actions: [plan]
  contexts: [phase planning]
---

Body.
"#;

        let toml = claude_skill_md_to_skill_toml(src).unwrap();
        assert!(toml.contains("name = \"x\""));
        assert!(toml.contains("action:plan"));
        assert!(toml.contains("ctx:phase planning"));
    }
}

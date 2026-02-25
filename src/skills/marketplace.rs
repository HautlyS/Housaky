use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::Skill;

const CLAUDE_OFFICIAL_MARKETPLACE_URL: &str =
    "https://raw.githubusercontent.com/anthropics/claude-plugins-official/main/marketplace.json";

/// Market skill candidate for installation/activation.
#[derive(Debug, Clone)]
pub struct MarketSkill {
    pub name: String,
    pub description: String,
    pub source: MarketSource,
    pub installed: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub enum MarketSource {
    /// Skills found in the vendored `openclaw/skills` folder.
    OpenClawVendored {
        skill_dir: PathBuf,
        skill_md: PathBuf,
    },
    /// Claude official plugin marketplace entry (plugin may contain skills/).
    ClaudeOfficialPlugin {
        plugin_name: String,
        plugin_path: String,
    },
}

#[derive(Debug, Deserialize)]
struct ClaudeMarketplaceIndex {
    #[allow(dead_code)]
    name: Option<String>,
    #[allow(dead_code)]
    description: Option<String>,
    plugins: Vec<ClaudeMarketplacePlugin>,
}

#[derive(Debug, Deserialize)]
struct ClaudeMarketplacePlugin {
    name: String,
    version: Option<String>,
    source: ClaudeMarketplaceSource,
}

#[derive(Debug, Deserialize)]
struct ClaudeMarketplaceSource {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    path: Option<String>,
}

pub fn openclaw_vendored_skills_dir(repo_root: &Path) -> PathBuf {
    repo_root.join("openclaw").join("skills")
}

pub fn ensure_market_cache_dir(workspace_dir: &Path) -> Result<PathBuf> {
    let dir = workspace_dir.join(".housaky").join("market");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn ensure_claude_official_repo(cache_dir: &Path) -> Result<PathBuf> {
    let repo_dir = cache_dir.join("claude-plugins-official");

    if !repo_dir.exists() {
        std::fs::create_dir_all(cache_dir)?;
        let out = Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "https://github.com/anthropics/claude-plugins-official.git",
            ])
            .arg(&repo_dir)
            .output()
            .with_context(|| "Failed to run git clone for claude-plugins-official")?;
        if !out.status.success() {
            return Err(anyhow!(
                "git clone claude-plugins-official failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        return Ok(repo_dir);
    }

    if repo_dir.join(".git").exists() {
        let out = Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .args(["pull", "--ff-only"])
            .output()
            .with_context(|| "Failed to run git pull for claude-plugins-official")?;
        if !out.status.success() {
            // Non-fatal: use whatever is in cache.
            tracing::warn!(
                "claude-plugins-official pull failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }

    Ok(repo_dir)
}

/// Load OpenClaw vendored skills (from repo `openclaw/skills/**/SKILL.md`).
pub fn list_openclaw_vendored_skills(
    repo_root: &Path,
    config: &crate::config::Config,
) -> Result<Vec<MarketSkill>> {
    let skills_dir = openclaw_vendored_skills_dir(repo_root);
    if !skills_dir.exists() {
        return Ok(vec![]);
    }

    let mut results = Vec::new();
    for entry in std::fs::read_dir(&skills_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let md = path.join("SKILL.md");
        if !md.exists() {
            continue;
        }

        // Reuse Claude parser because OpenClaw SKILL.md also uses YAML frontmatter.
        let content = std::fs::read_to_string(&md)?;
        let skill = super::claude::claude_skill_md_to_housaky_skill(&content)
            .unwrap_or_else(|_| Skill {
                name: path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
                description: "OpenClaw skill".into(),
                version: "0.1.0".into(),
                author: Some("openclaw".into()),
                tags: vec!["openclaw".into()],
                tools: vec![],
                prompts: vec![content],
                location: Some(md.clone()),
            });

        results.push(MarketSkill {
            name: skill.name.clone(),
            description: skill.description.clone(),
            source: MarketSource::OpenClawVendored {
                skill_dir: path,
                skill_md: md,
            },
            installed: true, // vendored, but not necessarily copied into workspace
            enabled: config.skills.enabled.get(&skill.name).copied().unwrap_or(false),
        });
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(results)
}

/// Fetch Claude marketplace index from cached repo (preferred) or network URL (fallback).
pub fn list_claude_official_plugins(
    workspace_dir: &Path,
    config: &crate::config::Config,
) -> Result<Vec<MarketSkill>> {
    let cache_dir = ensure_market_cache_dir(workspace_dir)?;
    let repo_dir = ensure_claude_official_repo(&cache_dir)?;

    // Try local marketplace.json first.
    let marketplace_path = repo_dir.join("marketplace.json");
    let content = if marketplace_path.exists() {
        std::fs::read_to_string(&marketplace_path)?
    } else {
        // Fallback to HTTP; uses reqwest blocking feature already enabled.
        let resp = reqwest::blocking::get(CLAUDE_OFFICIAL_MARKETPLACE_URL)
            .with_context(|| "Failed to fetch Claude official marketplace index")?;
        resp.text().with_context(|| "Failed to read marketplace response")?
    };

    let index: ClaudeMarketplaceIndex = serde_json::from_str(&content)
        .with_context(|| "Failed to parse Claude marketplace.json")?;

    let mut results = Vec::new();
    for plugin in index.plugins {
        // Only bundled plugins have a path; those can contain skills.
        let Some(path) = plugin.source.path else {
            continue;
        };

        let enabled = config.skills.enabled.get(&plugin.name).copied().unwrap_or(false);
        results.push(MarketSkill {
            name: plugin.name.clone(),
            description: format!(
                "Claude official plugin{}",
                plugin.version.as_ref().map(|v| format!(" v{v}")).unwrap_or_default()
            ),
            source: MarketSource::ClaudeOfficialPlugin {
                plugin_name: plugin.name,
                plugin_path: path,
            },
            installed: false,
            enabled,
        });
    }

    Ok(results)
}

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::Skill;
use std::fs;
use walkdir::WalkDir;

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

impl MarketSource {
    pub fn is_claude(&self) -> bool {
        matches!(self, MarketSource::ClaudeOfficialPlugin { .. })
    }
}

#[derive(Debug, Deserialize)]
struct MinimalSkillToml {
    skill: MinimalSkillMeta,
}

#[derive(Debug, Deserialize)]
struct MinimalSkillMeta {
    name: String,
    #[allow(dead_code)]
    description: Option<String>,
    #[allow(dead_code)]
    version: Option<String>,
}

fn sanitize_name(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
        } else if c.is_ascii_whitespace() || c == '/' || c == '\\' {
            out.push('-');
        }
    }
    if out.is_empty() {
        "skill".into()
    } else {
        out
    }
}

fn find_marketplace_entry<'a>(
    index: &'a ClaudeMarketplaceIndex,
    plugin_name: &str,
) -> Option<&'a ClaudeMarketplacePlugin> {
    index
        .plugins
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(plugin_name))
}

fn load_marketplace_index(workspace_dir: &Path) -> Result<(ClaudeMarketplaceIndex, PathBuf)> {
    let cache_dir = ensure_market_cache_dir(workspace_dir)?;
    let repo_dir = ensure_claude_official_repo(&cache_dir)?;
    let marketplace_path = repo_dir.join("marketplace.json");
    let content = if marketplace_path.exists() {
        fs::read_to_string(&marketplace_path)?
    } else {
        let resp = reqwest::blocking::get(CLAUDE_OFFICIAL_MARKETPLACE_URL)
            .with_context(|| "Failed to fetch Claude official marketplace index")?;
        resp.text()
            .with_context(|| "Failed to read marketplace response")?
    };
    let index: ClaudeMarketplaceIndex = serde_json::from_str(&content)
        .with_context(|| "Failed to parse Claude marketplace.json")?;
    Ok((index, repo_dir))
}

fn install_skill_md(content: &str, workspace_dir: &Path) -> Result<String> {
    let skill = crate::skills::claude::claude_skill_md_to_housaky_skill(content)?;
    let safe = sanitize_name(&skill.name);
    let dir = workspace_dir.join("skills").join(&safe);
    fs::create_dir_all(&dir)?;
    // Write TOML manifest for fast loading
    let toml = crate::skills::claude::claude_skill_md_to_skill_toml(content)?;
    fs::write(dir.join("SKILL.toml"), toml)?;
    // Store original for reference
    fs::write(dir.join("SKILL.md"), content)?;
    Ok(safe)
}

fn install_skill_toml(path: &Path, workspace_dir: &Path) -> Result<String> {
    let raw = fs::read_to_string(path)?;
    let parsed: MinimalSkillToml =
        toml::from_str(&raw).with_context(|| format!("Invalid TOML at {}", path.display()))?;
    let safe = sanitize_name(&parsed.skill.name);
    let dir = workspace_dir.join("skills").join(&safe);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("SKILL.toml"), raw)?;
    Ok(safe)
}

/// Install a Claude official plugin by name. Returns installed skill names.
pub fn install_claude_plugin(workspace_dir: &Path, plugin_name: &str) -> Result<Vec<String>> {
    let (index, repo_dir) = load_marketplace_index(workspace_dir)?;
    let Some(entry) = find_marketplace_entry(&index, plugin_name) else {
        return Err(anyhow!("Claude plugin not found: {}", plugin_name));
    };
    let Some(rel_path) = entry.source.path.as_ref() else {
        return Err(anyhow!("Claude plugin has no path: {}", plugin_name));
    };
    let plugin_dir = repo_dir.join(rel_path);
    if !plugin_dir.exists() {
        return Err(anyhow!(
            "Claude plugin path missing: {}",
            plugin_dir.display()
        ));
    }

    let mut installed = Vec::new();
    for entry in WalkDir::new(&plugin_dir)
        .max_depth(4)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if p.is_file() {
            let fname = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if fname.eq_ignore_ascii_case("SKILL.md") {
                let content = fs::read_to_string(p)?;
                let name = install_skill_md(&content, workspace_dir)?;
                if !installed.contains(&name) {
                    installed.push(name);
                }
            } else if fname.eq_ignore_ascii_case("SKILL.toml") {
                let name = install_skill_toml(p, workspace_dir)?;
                if !installed.contains(&name) {
                    installed.push(name);
                }
            }
        }
    }

    if installed.is_empty() {
        // Fallback: if plugin dir itself is a single skill directory, try reading any *.md at root
        for entry in fs::read_dir(&plugin_dir)? {
            let e = entry?;
            let p = e.path();
            if p.is_file() {
                let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
                if ext.eq_ignore_ascii_case("md")
                    && p.file_name()
                        .and_then(|s| s.to_str())
                        .map(|n| n.to_ascii_lowercase())
                        != Some("readme.md".into())
                {
                    let content = fs::read_to_string(&p)?;
                    let name = install_skill_md(&content, workspace_dir)?;
                    if !installed.contains(&name) {
                        installed.push(name);
                    }
                }
            }
        }
    }

    if installed.is_empty() {
        return Err(anyhow!(
            "No skills found to install in Claude plugin: {}",
            plugin_name
        ));
    }

    Ok(installed)
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
        let skill =
            super::claude::claude_skill_md_to_housaky_skill(&content).unwrap_or_else(|_| Skill {
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
            enabled: config
                .skills
                .enabled
                .get(&skill.name)
                .copied()
                .unwrap_or(false),
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
        resp.text()
            .with_context(|| "Failed to read marketplace response")?
    };

    let index: ClaudeMarketplaceIndex = serde_json::from_str(&content)
        .with_context(|| "Failed to parse Claude marketplace.json")?;

    let mut results = Vec::new();
    for plugin in index.plugins {
        // Only bundled plugins have a path; those can contain skills.
        let Some(path) = plugin.source.path else {
            continue;
        };

        let enabled = config
            .skills
            .enabled
            .get(&plugin.name)
            .copied()
            .unwrap_or(false);
        results.push(MarketSkill {
            name: plugin.name.clone(),
            description: format!(
                "Claude official plugin{}",
                plugin
                    .version
                    .as_ref()
                    .map(|v| format!(" v{v}"))
                    .unwrap_or_default()
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

pub fn install_openclaw_skill(workspace_dir: &Path, skill_name: &str) -> Result<Vec<String>> {
    let repo_root = workspace_dir.join(".housaky").join("openclaw");

    if !repo_root.exists() {
        std::fs::create_dir_all(&repo_root)?;
        let out = Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "https://github.com/openclaw/open-skills.git",
            ])
            .arg(&repo_root)
            .output()
            .with_context(|| "Failed to clone open-skills repo")?;
        if !out.status.success() {
            return Err(anyhow!(
                "Failed to clone open-skills: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
    }

    let skills_dir = repo_root.join("skills").join(skill_name);
    if !skills_dir.exists() {
        return Err(anyhow!("Skill not found: {}", skill_name));
    }

    let mut installed = Vec::new();
    let target_dir = workspace_dir.join("skills").join(skill_name);
    std::fs::create_dir_all(&target_dir)?;

    let skill_md = skills_dir.join("SKILL.md");
    if skill_md.exists() {
        let content = std::fs::read_to_string(&skill_md)?;
        let name = crate::skills::claude::claude_skill_md_to_housaky_skill(&content)
            .map(|s| s.name)
            .unwrap_or_else(|_| skill_name.to_string());

        let toml = crate::skills::claude::claude_skill_md_to_skill_toml(&content)?;
        std::fs::write(target_dir.join("SKILL.toml"), toml)?;
        std::fs::write(target_dir.join("SKILL.md"), content)?;
        installed.push(name);
    }

    if installed.is_empty() {
        return Err(anyhow!("No skills found to install"));
    }

    Ok(installed)
}

# Housaky × Claude Code Ecosystem Integration Analysis

**Technical Reference Document** | Version 1.0 | February 2026

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Technical Architecture Analysis](#2-technical-architecture-analysis)
3. [Housaky Current System Analysis](#3-housaky-current-system-analysis)
4. [Transpilation Strategy](#4-transpilation-strategy)
5. [Integration Architecture](#5-integration-architecture)
6. [Marketplace Synchronization](#6-marketplace-synchronization)
7. [Implementation Roadmap](#7-implementation-roadmap)
8. [Security Considerations](#8-security-considerations)

---

## 1. Executive Summary

### Overview of Claude Code Plugin/Skills Ecosystem

Claude Code has emerged as a dominant AI coding assistant with a mature plugin and skills ecosystem. The platform provides two primary extension mechanisms:

| System | Purpose | Format | Scope |
|--------|---------|--------|-------|
| **Plugins** | Shareable, versioned packages | `.claude-plugin/plugin.json` + components | Team/community distribution |
| **Skills** | Agent capabilities & instructions | `SKILL.md` with YAML frontmatter | Task-specific behaviors |

### Market Size and Growth (February 2026)

```
┌─────────────────────────────────────────────────────────────────┐
│  Claude Code Ecosystem Statistics                               │
├─────────────────────────────────────────────────────────────────┤
│  Plugins Available:     9,000+                                 │
│  Skills Available:       63,000+                               │
│  Community Marketplaces: 43+                                   │
│  GitHub Repos Indexed:   4,961                                 │
│  Growth Rate:            ~40% MoM (skills)                     │
└─────────────────────────────────────────────────────────────────┘
```

### Key Opportunity for Housaky

Housaky's unique position as a lightweight (<5MB), Rust-based AI assistant creates a compelling opportunity:

1. **Format Compatibility**: Housaky already supports `SKILL.md` and `SKILL.toml` formats
2. **Resource Efficiency**: Can run skills on $10 hardware where Claude Code cannot
3. **AGI-Ready**: Built-in self-improvement and skill generation capabilities
4. **Cross-Platform**: Single binary across ARM, x86, and RISC-V

**Strategic Value**: By building a transpilation and sync layer, Housaky gains instant access to 63K+ community skills and 9K+ plugins with zero content creation overhead.

---

## 2. Technical Architecture Analysis

### 2.1 Claude Code Plugin System

#### Plugin Directory Structure

```
my-plugin/
├── .claude-plugin/
│   └── plugin.json          # Manifest (required for namespacing)
├── commands/                # Slash commands (legacy markdown)
│   ├── deploy.md
│   └── status.md
├── agents/                  # Subagent definitions
│   ├── reviewer.md
│   └── tester.md
├── skills/                  # Agent Skills (primary format)
│   ├── code-review/
│   │   └── SKILL.md
│   └── pdf-processor/
│       ├── SKILL.md
│       └── scripts/
├── hooks/
│   └── hooks.json           # Event handlers
├── .mcp.json                # MCP server definitions
├── .lsp.json                # LSP server configurations
└── settings.json            # Default settings
```

#### plugin.json Schema

```json
{
  "name": "plugin-name",
  "version": "1.2.0",
  "description": "Brief plugin description",
  "author": {
    "name": "Author Name",
    "email": "author@example.com",
    "url": "https://github.com/author"
  },
  "homepage": "https://docs.example.com/plugin",
  "repository": "https://github.com/author/plugin",
  "license": "MIT",
  "keywords": ["keyword1", "keyword2"],
  "commands": ["./custom/commands/special.md"],
  "agents": "./custom/agents/",
  "skills": "./custom/skills/",
  "hooks": "./config/hooks.json",
  "mcpServers": "./mcp-config.json",
  "lspServers": "./.lsp.json"
}
```

#### Installation Scopes

| Scope | Settings File | Use Case |
|-------|---------------|----------|
| `user` | `~/.claude/settings.json` | Personal, cross-project (default) |
| `project` | `.claude/settings.json` | Team-shared via VCS |
| `local` | `.claude/settings.local.json` | Project-specific, gitignored |
| `managed` | Managed settings | Enterprise, read-only |

### 2.2 Claude Code Skills System

#### SKILL.md Format Specification

```yaml
---
# Required Fields
name: skill-name
description: When and how to use this skill

# Optional Frontmatter Fields
version: "1.0.0"
author: "Author Name"
disable-model-invocation: false
triggers:
  actions: [plan, execute, debug, verify, ship, deploy]
  contexts: [new project, phase planning, debugging]
  commands: [/skill:command1, /skill:command2]
  projects: [website, dashboard, mobile-app]
  elements: [button, modal, form]
  styles: [glassmorphism, minimalism]
tools_allowed: [read, write, shell, browser]
tools_restricted: [delete, network]
---

# Skill Title

Detailed instructions for the AI agent...

## Section

Content here...
```

#### Frontmatter Field Reference

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Unique skill identifier |
| `description` | string | Yes | When/how to use (used for discovery) |
| `version` | string | No | Semantic version |
| `author` | string | No | Attribution |
| `disable-model-invocation` | bool | No | If true, skill is command-only |
| `triggers` | object | No | Auto-activation conditions |
| `tools_allowed` | array | No | Whitelist of tool names |
| `tools_restricted` | array | No | Blacklist of tool names |

#### Progressive Disclosure Pattern

Skills support lazy loading of resources:

```
my-skill/
├── SKILL.md                 # Entry point (always loaded)
├── resources/
│   ├── reference.md         # Loaded on demand
│   ├── examples.md          # Loaded on demand
│   └── data/
│       └── database.csv     # Loaded on demand
└── scripts/
    └── helper.py            # Loaded on demand
```

### 2.3 MCP Protocol

#### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        MCP Architecture                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ┌─────────┐     JSON-RPC 2.0      ┌─────────────────────┐    │
│   │  Host   │◄──────────────────────│  MCP Client         │    │
│   │ (Claude)│                       │  (per connection)   │    │
│   └─────────┘                       └──────────┬──────────┘    │
│                                                 │               │
│                                    ┌────────────┴────────────┐  │
│                                    │                         │  │
│                              ┌─────▼─────┐           ┌───────▼─┐│
│                              │ MCP Server│           │ MCP     ││
│                              │ (Tools)   │           │ Server  ││
│                              └───────────┘           │(Resources)│
│                                                      └─────────┘│
└─────────────────────────────────────────────────────────────────┘
```

#### JSON-RPC 2.0 Message Format

```json
// Request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "get_weather",
    "arguments": {
      "location": "New York"
    }
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Current weather in New York: 72°F, Partly cloudy"
      }
    ],
    "isError": false
  }
}
```

#### MCP Capabilities

| Capability | Purpose | Key Methods |
|------------|---------|-------------|
| `tools` | Expose callable functions | `tools/list`, `tools/call` |
| `resources` | Share data/context | `resources/list`, `resources/read`, `resources/subscribe` |
| `prompts` | Provide prompt templates | `prompts/list`, `prompts/get` |
| `sampling` | Request LLM completions | `sampling/createMessage` |
| `roots` | Define workspace roots | `roots/list` |

#### Tools Capability

```json
// Server declares tools capability
{
  "capabilities": {
    "tools": {
      "listChanged": true
    }
  }
}

// Tool definition
{
  "name": "get_weather",
  "title": "Weather Information Provider",
  "description": "Get current weather for a location",
  "inputSchema": {
    "type": "object",
    "properties": {
      "location": {
        "type": "string",
        "description": "City name or zip code"
      }
    },
    "required": ["location"]
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "temperature": { "type": "number" },
      "conditions": { "type": "string" }
    }
  }
}
```

#### Resources Capability

```json
// List resources
{
  "method": "resources/list",
  "result": {
    "resources": [
      {
        "uri": "file:///project/src/main.rs",
        "name": "main.rs",
        "mimeType": "text/x-rust",
        "annotations": {
          "audience": ["user", "assistant"],
          "priority": 0.8
        }
      }
    ]
  }
}

// Read resource
{
  "method": "resources/read",
  "params": { "uri": "file:///project/src/main.rs" },
  "result": {
    "contents": [
      {
        "uri": "file:///project/src/main.rs",
        "mimeType": "text/x-rust",
        "text": "fn main() { println!(\"Hello\"); }"
      }
    ]
  }
}
```

### 2.4 Desktop Extensions (.mcpb Format)

#### Structure

```
extension.mcpb (ZIP archive)
├── manifest.json           # Extension metadata
├── server/                 # MCP server files
│   ├── index.js           # Entry point
│   └── package.json       # Dependencies
└── assets/                 # Icons, documentation
    ├── icon.png
    └── README.md
```

#### manifest.json Schema

```json
{
  "name": "my-extension",
  "version": "1.0.0",
  "description": "One-click installable MCP server",
  "author": { "name": "Developer" },
  "server": {
    "type": "node",
    "entry": "server/index.js",
    "mcp": {
      "capabilities": ["tools", "resources"]
    }
  },
  "installation": {
    "autoStart": true,
    "requiredPermissions": ["filesystem", "network"]
  },
  "compatibility": {
    "claudeDesktop": ">=1.0.0"
  }
}
```

---

## 3. Housaky Current System Analysis

### 3.1 Existing Skills System

#### Rust Data Structures

```rust
// src/skills/mod.rs

/// Core skill structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub tools: Vec<SkillTool>,
    pub prompts: Vec<String>,
    #[serde(skip)]
    pub location: Option<PathBuf>,
}

/// Tool defined within a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTool {
    pub name: String,
    pub description: String,
    pub kind: String,        // "shell", "http", "script"
    pub command: String,
    pub args: HashMap<String, String>,
}

/// Manifest for SKILL.toml format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SkillManifest {
    skill: SkillMeta,
    tools: Vec<SkillTool>,
    prompts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SkillMeta {
    name: String,
    description: String,
    version: String,
    author: Option<String>,
    tags: Vec<String>,
}
```

#### SKILL.toml Format

```toml
[skill]
name = "my-skill"
description = "What this skill does"
version = "1.0.0"
author = "developer-name"
tags = ["productivity", "automation"]

[[tools]]
name = "my_tool"
description = "Tool description"
kind = "shell"
command = "echo hello"
[tools.args]
arg1 = "value1"
arg2 = "value2"

[[tools]]
name = "api_call"
description = "HTTP API tool"
kind = "http"
command = "https://api.example.com/endpoint"

prompts = [
  "Default prompt text here",
  "Another prompt variant"
]
```

#### Loading Mechanism

```rust
/// Load all skills from workspace + open-skills repo
pub fn load_skills(workspace_dir: &Path) -> Vec<Skill> {
    let mut skills = Vec::new();

    // 1. Load from community open-skills repo
    if let Some(open_skills_dir) = ensure_open_skills_repo() {
        skills.extend(load_open_skills(&open_skills_dir));
    }

    // 2. Load from workspace
    skills.extend(load_workspace_skills(workspace_dir));
    skills
}

/// Load from SKILL.toml (preferred) or SKILL.md
fn load_skills_from_directory(skills_dir: &Path) -> Vec<Skill> {
    for entry in entries.flatten() {
        let manifest_path = path.join("SKILL.toml");
        let md_path = path.join("SKILL.md");

        if manifest_path.exists() {
            skills.push(load_skill_toml(&manifest_path)?);
        } else if md_path.exists() {
            skills.push(load_skill_md(&md_path, &path)?);
        }
    }
}
```

#### Storage Paths

```
~/.housaky/
├── workspace/
│   └── skills/              # User-installed skills
│       ├── my-skill/
│       │   └── SKILL.md
│       └── another-skill/
│           └── SKILL.toml
└── .housaky/
    └── skills/              # AGI-generated skills
        ├── self_analysis/
        └── agi_development/

~/open-skills/               # Synced from besoeasy/open-skills
├── skill-one.md
└── skill-two.md
```

### 3.2 AGI Skill Registry

```rust
// src/housaky/skills.rs

pub struct SkillRegistry {
    workspace_dir: PathBuf,
    skills_dir: PathBuf,
}

impl SkillRegistry {
    /// Auto-discover and learn new skills
    pub async fn discover_and_learn(&self) -> Result<()> {
        let skills_to_learn = vec![
            ("self_analysis", "Analyze own performance"),
            ("code_optimization", "Optimize code performance"),
            ("research", "Conduct research on topics"),
            ("skill_generation", "Generate new skills automatically"),
            ("knowledge_synthesis", "Synthesize knowledge"),
        ];
        // ...
    }
}

pub struct SkillCreator {
    workspace_dir: PathBuf,
    skills_dir: PathBuf,
}

impl SkillCreator {
    /// Create skill from a completed task
    pub async fn create_skill_from_task(&self, task: &Task) -> Result<()> {
        // Auto-generates SKILL.md + SKILL.toml
    }
}
```

### 3.3 CLI Commands

```rust
// src/commands.rs

pub enum SkillCommands {
    /// List all installed skills
    List,
    /// Install a new skill from URL or path
    Install { source: String },
    /// Remove an installed skill
    Remove { name: String },
}
```

---

## 4. Transpilation Strategy

### 4.1 Format Mapping

#### Claude SKILL.md → Housaky SKILL.toml

```
┌─────────────────────────────────────────────────────────────────┐
│  Claude SKILL.md                  →    Housaky SKILL.toml       │
├─────────────────────────────────────────────────────────────────┤
│  ---                                  [skill]                   │
│  name: code-review                   name = "code-review"       │
│  description: Reviews code           description = "Reviews..." │
│  version: "1.0.0"                    version = "1.0.0"          │
│  author: Developer                   author = "Developer"       │
│  triggers:                           tags = ["review"]          │
│    actions: [review]                                            │
│  ---                                                         │
│                                                                 │
│  # Content here                      prompts = ["""            │
│  Detailed instructions...              # Content here          │
│                                         Detailed instructions...│
│                                      """]                       │
└─────────────────────────────────────────────────────────────────┘
```

#### Field Translation Table

| Claude Field | Housaky Field | Transformation |
|--------------|---------------|----------------|
| `name` | `skill.name` | Direct |
| `description` | `skill.description` | Direct |
| `version` | `skill.version` | Direct |
| `author` | `skill.author` | Direct |
| `triggers.actions` | `skill.tags` | Concatenate with prefix |
| `triggers.contexts` | `skill.tags` | Concatenate with prefix |
| `triggers.commands` | N/A | Parse for tool definitions |
| `tools_allowed` | N/A | Map to tool restrictions |
| `tools_restricted` | N/A | Map to tool restrictions |
| Markdown body | `prompts` | Wrap as string array |

### 4.2 Transpilation Implementation

```rust
// src/skills/transpiler.rs

use serde_yaml::Value as YamlValue;
use toml::Value as TomlValue;

/// Claude SKILL.md frontmatter structure
#[derive(Debug, Deserialize)]
pub struct ClaudeSkillFrontmatter {
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub triggers: Option<ClaudeTriggers>,
    pub tools_allowed: Option<Vec<String>>,
    pub tools_restricted: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeTriggers {
    pub actions: Option<Vec<String>>,
    pub contexts: Option<Vec<String>>,
    pub commands: Option<Vec<String>>,
    pub projects: Option<Vec<String>>,
    pub elements: Option<Vec<String>>,
    pub styles: Option<Vec<String>>,
}

/// Transpile Claude SKILL.md to Housaky SKILL.toml
pub fn transpile_claude_skill_to_toml(
    claude_skill: &str,
) -> Result<String> {
    let (frontmatter, body) = parse_claude_skill(claude_skill)?;
    
    let mut tags = Vec::new();
    if let Some(ref triggers) = frontmatter.triggers {
        if let Some(ref actions) = triggers.actions {
            tags.extend(actions.iter().map(|a| format!("action:{}", a)));
        }
        if let Some(ref contexts) = triggers.contexts {
            tags.extend(contexts.iter().map(|c| format!("ctx:{}", c)));
        }
    }

    let housaky_skill = HousakySkillManifest {
        skill: HousakySkillMeta {
            name: frontmatter.name,
            description: frontmatter.description,
            version: frontmatter.version.unwrap_or_else(|| "0.1.0".to_string()),
            author: frontmatter.author,
            tags,
        },
        tools: vec![],
        prompts: vec![body],
    };

    toml::to_string_pretty(&housaky_skill)
        .map_err(|e| anyhow::anyhow!("TOML serialization failed: {}", e))
}

/// Parse Claude SKILL.md into frontmatter and body
fn parse_claude_skill(content: &str) -> Result<(ClaudeSkillFrontmatter, String)> {
    let content = content.trim_start();
    
    if !content.starts_with("---") {
        return Err(anyhow::anyhow!("Missing frontmatter delimiter"));
    }
    
    let end = content[3..].find("---")
        .ok_or_else(|| anyhow::anyhow!("Unclosed frontmatter"))?;
    
    let frontmatter_str = &content[3..end + 3];
    let body = content[end + 6..].trim().to_string();
    
    let frontmatter: ClaudeSkillFrontmatter = serde_yaml::from_str(frontmatter_str)
        .map_err(|e| anyhow::anyhow!("YAML parse error: {}", e))?;
    
    Ok((frontmatter, body))
}
```

### 4.3 MCP Server Integration Approach

```rust
// src/mcp/mod.rs (proposed)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP Server configuration from .mcp.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub cwd: Option<String>,
}

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,
}

/// MCP Client for tool invocation
pub struct McpClient {
    server_name: String,
    process: Option<tokio::process::Child>,
    request_id: atomic::AtomicU64,
}

impl McpClient {
    pub async fn start(config: &McpServerConfig) -> Result<Self> {
        let mut cmd = tokio::process::Command::new(&config.command);
        cmd.args(&config.args)
           .envs(&config.env)
           .stdin(Stdio::piped())
           .stdout(Stdio::piped());
        
        let process = cmd.spawn()?;
        
        Ok(Self {
            server_name: config.command.clone(),
            process: Some(process),
            request_id: atomic::AtomicU64::new(1),
        })
    }

    pub async fn list_tools(&mut self) -> Result<Vec<McpTool>> {
        let response = self.send_request("tools/list", json!({})).await?;
        let tools: Vec<McpTool> = response["tools"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid tools response"))?
            .iter()
            .map(|t| serde_json::from_value(t.clone()).unwrap())
            .collect();
        Ok(tools)
    }

    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let params = json!({
            "name": name,
            "arguments": arguments
        });
        self.send_request("tools/call", params).await
    }

    async fn send_request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        // ... JSON-RPC over stdio implementation
    }
}
```

### 4.4 Hooks Translation

```rust
// src/hooks/transpiler.rs (proposed)

/// Claude Code hook event types
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ClaudeHookEvent {
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
    PermissionRequest,
    UserPromptSubmit,
    Notification,
    Stop,
    SubagentStart,
    SubagentStop,
    SessionStart,
    SessionEnd,
    TeammateIdle,
    TaskCompleted,
    PreCompact,
}

/// Claude hooks.json structure
#[derive(Debug, Deserialize)]
pub struct ClaudeHooksConfig {
    pub hooks: HashMap<ClaudeHookEvent, Vec<ClaudeHookMatcher>>,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeHookMatcher {
    pub matcher: Option<String>,
    pub hooks: Vec<ClaudeHookAction>,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeHookAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub command: Option<String>,
    pub prompt: Option<String>,
    pub agent: Option<String>,
}

/// Convert Claude hooks to Housaky event handlers
pub fn transpile_hooks(claude_hooks: &ClaudeHooksConfig) -> Vec<HousakyHook> {
    let mut hooks = Vec::new();
    
    for (event, matchers) in &claude_hooks.hooks {
        for matcher in matchers {
            for action in &matcher.hooks {
                hooks.push(HousakyHook {
                    event: map_event(event),
                    tool_filter: matcher.matcher.clone(),
                    action: map_action(action),
                });
            }
        }
    }
    
    hooks
}

fn map_event(event: &ClaudeHookEvent) -> HousakyEventType {
    match event {
        ClaudeHookEvent::PreToolUse => HousakyEventType::BeforeToolCall,
        ClaudeHookEvent::PostToolUse => HousakyEventType::AfterToolCall,
        ClaudeHookEvent::SessionStart => HousakyEventType::SessionInit,
        ClaudeHookEvent::SessionEnd => HousakyEventType::SessionEnd,
        _ => HousakyEventType::Custom(event.to_string()),
    }
}
```

---

## 5. Integration Architecture

### 5.1 Plugin Loader Module Design

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      Housaky Plugin Loader                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────────────────┐   │
│  │ Plugin      │     │ Transpiler  │     │ Skill Registry          │   │
│  │ Discovery   │────►│ Engine      │────►│ (Rust)                  │   │
│  └─────────────┘     └─────────────┘     └─────────────────────────┘   │
│        │                                         │                     │
│        │                                         ▼                     │
│        │              ┌─────────────┐     ┌─────────────────────────┐   │
│        └─────────────►│ Cache       │◄───►│ Skill Runtime           │   │
│                       │ Manager     │     │ (Loader + Executor)     │   │
│                       └─────────────┘     └─────────────────────────┘   │
│                              │                    │                     │
│                              ▼                    ▼                     │
│                       ┌─────────────┐     ┌─────────────────────────┐   │
│                       │ Update      │     │ MCP Client Pool         │   │
│                       │ Checker     │     │ (Tool Integration)      │   │
│                       └─────────────┘     └─────────────────────────┘   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Plugin Loader Implementation

```rust
// src/plugins/mod.rs (proposed)

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PluginLoader {
    cache_dir: PathBuf,
    installed_plugins: Arc<RwLock<Vec<InstalledPlugin>>>,
    transpiler: Transpiler,
    mcp_manager: McpServerManager,
}

pub struct InstalledPlugin {
    pub name: String,
    pub version: String,
    pub source: PluginSource,
    pub skills: Vec<Skill>,
    pub agents: Vec<Agent>,
    pub hooks: Vec<HousakyHook>,
    pub mcp_servers: Vec<McpServerConfig>,
    pub installed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub enum PluginSource {
    Marketplace { name: String, url: String },
    Git { repo: String, commit: String },
    Local { path: PathBuf },
}

impl PluginLoader {
    pub fn new(cache_dir: &Path) -> Self {
        Self {
            cache_dir: cache_dir.join("plugins"),
            installed_plugins: Arc::new(RwLock::new(Vec::new())),
            transpiler: Transpiler::new(),
            mcp_manager: McpServerManager::new(),
        }
    }

    /// Install plugin from marketplace
    pub async fn install(
        &self,
        plugin_ref: &str,
        scope: InstallScope,
    ) -> Result<InstalledPlugin> {
        let plugin_dir = self.fetch_plugin(plugin_ref).await?;
        let plugin = self.load_plugin(&plugin_dir).await?;
        
        // Store in appropriate scope
        self.persist_installation(&plugin, scope).await?;
        
        // Start MCP servers if any
        for mcp_config in &plugin.mcp_servers {
            self.mcp_manager.start_server(&plugin.name, mcp_config).await?;
        }
        
        self.installed_plugins.write().await.push(plugin.clone());
        Ok(plugin)
    }

    /// Load plugin from directory
    async fn load_plugin(&self, dir: &Path) -> Result<InstalledPlugin> {
        let manifest = self.parse_manifest(dir).await?;
        
        // Transpile skills
        let skills = self.load_skills(dir, &manifest).await?;
        
        // Load agents
        let agents = self.load_agents(dir, &manifest).await?;
        
        // Transpile hooks
        let hooks = self.load_hooks(dir, &manifest).await?;
        
        // Load MCP configs
        let mcp_servers = self.load_mcp_configs(dir, &manifest).await?;
        
        Ok(InstalledPlugin {
            name: manifest.name,
            version: manifest.version.unwrap_or_else(|| "0.0.0".to_string()),
            source: PluginSource::Local { path: dir.to_path_buf() },
            skills,
            agents,
            hooks,
            mcp_servers,
            installed_at: chrono::Utc::now(),
        })
    }

    /// Load and transpile skills from plugin
    async fn load_skills(
        &self,
        dir: &Path,
        manifest: &PluginManifest,
    ) -> Result<Vec<Skill>> {
        let mut skills = Vec::new();
        let skills_dir = manifest.skills.as_ref()
            .map(|p| dir.join(p))
            .unwrap_or_else(|| dir.join("skills"));
        
        if skills_dir.exists() {
            for entry in std::fs::read_dir(&skills_dir)? {
                let skill_dir = entry?.path();
                if skill_dir.join("SKILL.md").exists() {
                    let claude_skill = std::fs::read_to_string(skill_dir.join("SKILL.md"))?;
                    let housaky_skill = self.transpiler.skill_md_to_toml(&claude_skill)?;
                    skills.push(housaky_skill);
                }
            }
        }
        
        Ok(skills)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub author: Option<PluginAuthor>,
    pub repository: Option<String>,
    pub skills: Option<String>,
    pub agents: Option<String>,
    pub hooks: Option<String>,
    pub mcp_servers: Option<String>,
}
```

### 5.3 Marketplace Sync System

```rust
// src/plugins/marketplace.rs (proposed)

use serde::{Deserialize, Serialize};

pub struct MarketplaceSync {
    registries: Vec<PluginRegistry>,
    cache: PluginCache,
    http_client: reqwest::Client,
}

#[derive(Debug, Clone)]
pub struct PluginRegistry {
    pub name: String,
    pub url: String,
    pub registry_type: RegistryType,
}

#[derive(Debug, Clone)]
pub enum RegistryType {
    Official,        // Anthropic's claude-plugins-official
    Community,       // claude-plugins.dev
    GitHub,          // GitHub-based discovery
    Custom { api_url: String },
}

#[derive(Debug, Deserialize)]
pub struct MarketplaceIndex {
    pub plugins: Vec<MarketplaceEntry>,
}

#[derive(Debug, Deserialize)]
pub struct MarketplaceEntry {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub repository: String,
    pub keywords: Vec<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub source: PluginSourceInfo,
}

#[derive(Debug, Deserialize)]
pub struct PluginSourceInfo {
    #[serde(rename = "type")]
    pub source_type: String,
    pub url: String,
    #[serde(default)]
    pub path: Option<String>,
}

impl MarketplaceSync {
    pub async fn sync_all(&self) -> Result<SyncResult> {
        let mut result = SyncResult::default();
        
        for registry in &self.registries {
            match self.sync_registry(registry).await {
                Ok(entries) => {
                    result.plugins_updated += entries.len();
                    self.cache.update_registry(&registry.name, entries).await?;
                }
                Err(e) => {
                    result.errors.push(format!("{}: {}", registry.name, e));
                }
            }
        }
        
        Ok(result)
    }

    async fn sync_registry(&self, registry: &PluginRegistry) -> Result<Vec<MarketplaceEntry>> {
        let index_url = match registry.registry_type {
            RegistryType::Official => {
                "https://raw.githubusercontent.com/anthropics/claude-plugins-official/main/marketplace.json"
            }
            RegistryType::Community => {
                "https://claude-plugins.dev/api/plugins"
            }
            RegistryType::GitHub => {
                &registry.url
            }
            RegistryType::Custom { ref api_url } => api_url,
        };
        
        let response = self.http_client.get(index_url).send().await?;
        let index: MarketplaceIndex = response.json().await?;
        
        Ok(index.plugins)
    }

    pub async fn search(&self, query: &str) -> Result<Vec<MarketplaceEntry>> {
        let cache = self.cache.read().await;
        let query_lower = query.to_lowercase();
        
        let results: Vec<_> = cache.all_plugins()
            .filter(|p| {
                p.name.to_lowercase().contains(&query_lower) ||
                p.description.to_lowercase().contains(&query_lower) ||
                p.keywords.iter().any(|k| k.to_lowercase().contains(&query_lower))
            })
            .take(50)
            .collect();
        
        Ok(results)
    }
}

#[derive(Default)]
pub struct SyncResult {
    pub plugins_updated: usize,
    pub errors: Vec<String>,
}
```

### 5.4 Caching and Update Strategy

```rust
// src/plugins/cache.rs (proposed)

use std::path::PathBuf;
use tokio::sync::RwLock;

pub struct PluginCache {
    cache_dir: PathBuf,
    index: RwLock<CacheIndex>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheIndex {
    pub version: u32,
    pub last_sync: chrono::DateTime<chrono::Utc>,
    pub plugins: Vec<CachedPlugin>,
    pub registries: HashMap<String, RegistryCache>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CachedPlugin {
    pub entry: MarketplaceEntry,
    pub installed: bool,
    pub installed_version: Option<String>,
    pub cached_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryCache {
    pub name: String,
    pub last_fetched: chrono::DateTime<chrono::Utc>,
    pub plugin_count: usize,
}

impl PluginCache {
    pub fn new(cache_dir: &Path) -> Self {
        Self {
            cache_dir: cache_dir.to_path_buf(),
            index: RwLock::new(CacheIndex::load_or_default(cache_dir)),
        }
    }

    pub async fn check_updates(&self) -> Result<Vec<UpdateAvailable>> {
        let index = self.index.read().await;
        let mut updates = Vec::new();
        
        for plugin in &index.plugins {
            if plugin.installed {
                if let Some(ref installed_version) = plugin.installed_version {
                    if installed_version != &plugin.entry.version {
                        updates.push(UpdateAvailable {
                            name: plugin.entry.name.clone(),
                            current: installed_version.clone(),
                            latest: plugin.entry.version.clone(),
                        });
                    }
                }
            }
        }
        
        Ok(updates)
    }

    pub async fn invalidate(&self) -> Result<()> {
        let mut index = self.index.write().await;
        *index = CacheIndex::default();
        index.save(&self.cache_dir)?;
        Ok(())
    }
}

pub struct UpdateAvailable {
    pub name: String,
    pub current: String,
    pub latest: String,
}
```

---

## 6. Marketplace Synchronization

### 6.1 Official Anthropic Marketplace

**Repository**: `anthropics/claude-plugins-official`

```
claude-plugins-official/
├── marketplace.json         # Index of all plugins
├── plugins/
│   ├── typescript-lsp/
│   │   ├── .claude-plugin/
│   │   │   └── plugin.json
│   │   └── .lsp.json
│   ├── pyright-lsp/
│   └── rust-lsp/
└── README.md
```

**marketplace.json format**:
```json
{
  "name": "official",
  "description": "Official Anthropic plugins",
  "plugins": [
    {
      "name": "typescript-lsp",
      "version": "1.0.0",
      "source": {
        "type": "bundled",
        "path": "plugins/typescript-lsp"
      }
    }
  ]
}
```

### 6.2 Community Registries

#### claude-plugins.dev API

```rust
// API Endpoints
const API_BASE: &str = "https://claude-plugins.dev/api";

// GET /api/plugins - List all plugins
// GET /api/plugins/{name} - Get plugin details
// GET /api/search?q={query} - Search plugins
// GET /api/categories - List categories

#[derive(Debug, Deserialize)]
pub struct ClaudePluginsDevResponse {
    pub plugins: Vec<ClaudePluginsDevEntry>,
    pub total: usize,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Deserialize)]
pub struct ClaudePluginsDevEntry {
    pub name: String,
    pub full_name: String,
    pub description: String,
    pub author: String,
    pub stars: u32,
    pub version: String,
    pub repository: String,
    pub installed_count: u32,
    pub categories: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

#### GitHub-Based Discovery

```rust
// GitHub search for Claude plugins
pub async fn discover_github_plugins(query: &str) -> Result<Vec<GitHubPlugin>> {
    let url = format!(
        "https://api.github.com/search/repositories?q={}+SKILL.md+in:path&sort=stars",
        query
    );
    
    let response = reqwest::Client::new()
        .get(&url)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await?;
    
    let search_result: GitHubSearchResult = response.json().await?;
    
    Ok(search_result.items.into_iter().map(|repo| GitHubPlugin {
        name: repo.name,
        full_name: repo.full_name,
        description: repo.description.unwrap_or_default(),
        stars: repo.stargazers_count,
        url: repo.html_url,
        default_branch: repo.default_branch,
    }).collect())
}
```

### 6.3 Update/Fetch Mechanisms

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      Update Flow                                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌────────────┐    ┌────────────┐    ┌────────────┐    ┌────────────┐  │
│  │ Schedule   │───►│ Fetch      │───►│ Compare    │───►│ Update     │  │
│  │ (Cron)     │    │ Indexes    │    │ Versions   │    │ Installed  │  │
│  └────────────┘    └────────────┘    └────────────┘    └────────────┘  │
│                           │                                    │        │
│                           ▼                                    ▼        │
│                    ┌────────────┐                       ┌────────────┐  │
│                    │ Cache      │                       │ Notify     │  │
│                    │ Update     │                       │ User       │  │
│                    └────────────┘                       └────────────┘  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

```rust
// Update scheduler configuration
pub struct UpdateConfig {
    pub enabled: bool,
    pub interval: Duration,           // Default: 24 hours
    pub auto_update: bool,            // Auto-install updates
    pub check_on_startup: bool,       // Check on daemon start
    pub registries: Vec<RegistryConfig>,
}

pub struct RegistryConfig {
    pub name: String,
    pub enabled: bool,
    pub sync_interval: Option<Duration>,
}

impl PluginLoader {
    pub async fn schedule_updates(&self, config: UpdateConfig) {
        let mut interval = tokio::time::interval(config.interval);
        
        loop {
            interval.tick().await;
            
            match self.check_for_updates().await {
                Ok(updates) => {
                    if !updates.is_empty() {
                        if config.auto_update {
                            for update in &updates {
                                if let Err(e) = self.update_plugin(&update.name).await {
                                    tracing::error!("Failed to update {}: {}", update.name, e);
                                }
                            }
                        } else {
                            // Notify user of available updates
                            self.notify_updates(&updates).await;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Update check failed: {}", e);
                }
            }
        }
    }
}
```

---

## 7. Implementation Roadmap

### Phase 1: Format Compatibility (Week 1-2)

**Objective**: Parse and transpile Claude Code skills

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 1 Deliverables                                                   │
├─────────────────────────────────────────────────────────────────────────┤
│  □ SKILL.md parser with YAML frontmatter support                       │
│  □ SKILL.toml → Housaky Skill transpiler                               │
│  □ SKILL.md → Housaky Skill transpiler                                 │
│  □ Unit tests for transpilation                                        │
│  □ CLI command: housaky skills convert <claude-skill>                  │
└─────────────────────────────────────────────────────────────────────────┘
```

**Implementation**:

```bash
# New files to create
src/skills/transpiler.rs
src/skills/parser.rs
src/skills/frontmatter.rs

# Tests
src/skills/transpiler_tests.rs
```

```rust
// Milestone 1: Basic transpilation
#[test]
fn test_transpile_claude_skill() {
    let claude_skill = r#"---
name: code-review
description: Review code for best practices
version: "1.0.0"
author: Developer
triggers:
  actions: [review, check]
---

# Code Review Skill

Review code for:
1. Best practices
2. Security issues
3. Performance
"#;

    let result = transpile_claude_skill_to_toml(claude_skill).unwrap();
    
    assert!(result.contains("name = \"code-review\""));
    assert!(result.contains("description = \"Review code for best practices\""));
    assert!(result.contains("version = \"1.0.0\""));
}
```

### Phase 2: MCP Integration (Week 3-4)

**Objective**: Enable MCP server support in Housaky

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 2 Deliverables                                                   │
├─────────────────────────────────────────────────────────────────────────┤
│  □ MCP client implementation (JSON-RPC 2.0 over stdio)                 │
│  □ Tool discovery and invocation                                       │
│  □ Resource reading support                                            │
│  □ .mcp.json configuration loading                                     │
│  □ MCP tool → Housaky tool adapter                                     │
│  □ CLI commands: housaky mcp list/call                                 │
└─────────────────────────────────────────────────────────────────────────┘
```

**Implementation**:

```bash
# New modules
src/mcp/mod.rs
src/mcp/client.rs
src/mcp/protocol.rs
src/mcp/transport.rs
src/mcp/tools.rs
src/mcp/resources.rs
```

```rust
// Milestone 2: MCP tool invocation
#[tokio::test]
async fn test_mcp_tool_call() {
    let config = McpServerConfig {
        command: "npx".to_string(),
        args: vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()],
        env: HashMap::new(),
        cwd: None,
    };
    
    let mut client = McpClient::start(&config).await.unwrap();
    
    let tools = client.list_tools().await.unwrap();
    assert!(!tools.is_empty());
    
    let result = client.call_tool("read_file", json!({
        "path": "/tmp/test.txt"
    })).await.unwrap();
    
    assert!(result["content"].is_array());
}
```

### Phase 3: Marketplace Sync (Week 5-6)

**Objective**: Sync with Claude Code plugin marketplaces

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 3 Deliverables                                                   │
├─────────────────────────────────────────────────────────────────────────┤
│  □ Plugin loader module                                                 │
│  □ Marketplace index parser                                            │
│  □ Registry sync implementation                                         │
│  □ Local cache system                                                   │
│  □ Update checker                                                       │
│  □ CLI commands: housaky plugin search/install/list/update             │
└─────────────────────────────────────────────────────────────────────────┘
```

**Implementation**:

```bash
# New modules
src/plugins/mod.rs
src/plugins/loader.rs
src/plugins/marketplace.rs
src/plugins/cache.rs
src/plugins/registry.rs
```

```rust
// Milestone 3: Marketplace sync
#[tokio::test]
async fn test_marketplace_sync() {
    let sync = MarketplaceSync::new(test_cache_dir());
    
    let result = sync.sync_all().await.unwrap();
    
    assert!(result.plugins_updated > 0);
    
    let search_results = sync.search("typescript").await.unwrap();
    assert!(!search_results.is_empty());
}
```

### Phase 4: Bi-directional Publishing (Week 7-8)

**Objective**: Export Housaky skills to Claude Code format

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 4 Deliverables                                                   │
├─────────────────────────────────────────────────────────────────────────┤
│  □ Housaky → Claude SKILL.md transpiler                                │
│  □ Plugin manifest generator                                           │
│  □ .mcp.json export for Housaky tools                                  │
│  □ GitHub publishing workflow                                          │
│  □ CLI command: housaky skills export --format claude                  │
│  □ Integration with GitHub CLI for PR creation                          │
└─────────────────────────────────────────────────────────────────────────┘
```

**Implementation**:

```rust
// Export Housaky skill to Claude format
pub fn export_to_claude_skill(skill: &Skill) -> Result<String> {
    let mut frontmatter = yaml::Mapping::new();
    
    frontmatter.insert(
        yaml::Value::String("name".to_string()),
        yaml::Value::String(skill.name.clone()),
    );
    frontmatter.insert(
        yaml::Value::String("description".to_string()),
        yaml::Value::String(skill.description.clone()),
    );
    frontmatter.insert(
        yaml::Value::String("version".to_string()),
        yaml::Value::String(skill.version.clone()),
    );
    
    // Convert tags to triggers
    let mut triggers = yaml::Mapping::new();
    let actions: Vec<_> = skill.tags.iter()
        .filter(|t| t.starts_with("action:"))
        .map(|t| t.strip_prefix("action:").unwrap())
        .map(yaml::Value::String)
        .collect();
    if !actions.is_empty() {
        triggers.insert(
            yaml::Value::String("actions".to_string()),
            yaml::Value::Sequence(actions),
        );
    }
    
    let frontmatter_str = yaml::to_string(&frontmatter)?;
    let body = skill.prompts.join("\n\n");
    
    Ok(format!("---\n{}---\n\n{}", frontmatter_str, body))
}
```

---

## 8. Security Considerations

### 8.1 Plugin Sandboxing

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      Security Architecture                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                     Plugin Sandbox                               │   │
│  │  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐    │   │
│  │  │ Skill     │  │ MCP       │  │ Hooks     │  │ Agents    │    │   │
│  │  │ Runtime   │  │ Client    │  │ Executor  │  │ Runtime   │    │   │
│  │  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘    │   │
│  │        │              │              │              │          │   │
│  │        └──────────────┴──────────────┴──────────────┘          │   │
│  │                              │                                  │   │
│  │                    ┌─────────▼─────────┐                       │   │
│  │                    │  Permission Gate  │                       │   │
│  │                    │  - Filesystem     │                       │   │
│  │                    │  - Network        │                       │   │
│  │                    │  - Process        │                       │   │
│  │                    │  - Secrets        │                       │   │
│  │                    └─────────┬─────────┘                       │   │
│  │                              │                                  │   │
│  └──────────────────────────────┼──────────────────────────────────┘   │
│                                 │                                      │
│  ┌──────────────────────────────▼──────────────────────────────────┐   │
│  │                     Runtime Layer                                │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐            │   │
│  │  │ Native  │  │ Docker  │  │Bwrap    │  │Landlock │            │   │
│  │  │(baseline)│  │(strong) │  │(medium) │  │(kernel) │            │   │
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘            │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 8.2 Permission Model

```rust
// src/plugins/permissions.rs (proposed)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPermissions {
    pub filesystem: FilesystemPermissions,
    pub network: NetworkPermissions,
    pub process: ProcessPermissions,
    pub secrets: SecretsPermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemPermissions {
    pub read_paths: Vec<PathPattern>,
    pub write_paths: Vec<PathPattern>,
    pub workspace_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPermissions {
    pub allowed_domains: Vec<String>,
    pub allowed_ports: Vec<u16>,
    pub deny_all: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessPermissions {
    pub allowed_commands: Vec<String>,
    pub shell_access: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsPermissions {
    pub can_read_keys: bool,
    pub key_namespaces: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum PathPattern {
    Literal(String),
    Glob(String),
    Prefix(String),
}

impl PluginLoader {
    pub fn validate_permissions(
        &self,
        plugin: &InstalledPlugin,
        requested: &PluginPermissions,
    ) -> Result<PermissionGrant> {
        // Check against policy
        let policy = self.load_permission_policy()?;
        
        if requested.filesystem.workspace_only && !policy.allow_workspace_access {
            return Err(anyhow::anyhow!("Workspace access not allowed"));
        }
        
        for domain in &requested.network.allowed_domains {
            if policy.blocked_domains.contains(domain) {
                return Err(anyhow::anyhow!("Domain {} is blocked", domain));
            }
        }
        
        Ok(PermissionGrant {
            permissions: requested.clone(),
            granted_at: chrono::Utc::now(),
            granted_to: plugin.name.clone(),
        })
    }
}
```

### 8.3 Code Signing/Verification

```rust
// src/plugins/signing.rs (proposed)

use ring::signature::{Ed25519KeyPair, UnparsedPublicKey, ED25519};

#[derive(Debug, Clone)]
pub struct PluginSignature {
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
    pub signed_at: chrono::DateTime<chrono::Utc>,
}

pub struct PluginVerifier {
    trusted_keys: Vec<UnparsedPublicKey<Vec<u8>>>,
}

impl PluginVerifier {
    pub fn verify_plugin(
        &self,
        plugin_path: &Path,
        signature: &PluginSignature,
    ) -> Result<()> {
        // Read plugin archive
        let plugin_data = std::fs::read(plugin_path)?;
        
        // Verify signature
        let public_key = UnparsedPublicKey::new(
            &ED25519,
            signature.public_key.clone(),
        );
        
        public_key.verify(&plugin_data, &signature.signature)
            .map_err(|_| anyhow::anyhow!("Signature verification failed"))?;
        
        // Check if key is trusted
        let key_trusted = self.trusted_keys.iter().any(|k| {
            k.bytes().as_ref() == signature.public_key.as_slice()
        });
        
        if !key_trusted {
            tracing::warn!(
                "Plugin signed by untrusted key: {}",
                hex::encode(&signature.public_key)
            );
        }
        
        Ok(())
    }
}
```

### 8.4 Security Configuration

```toml
# ~/.housaky/config.toml

[plugins]
enabled = true
default_scope = "user"
auto_update = false
require_signature = false        # Set true for production
trusted_registries = [
    "official",
    "claude-plugins.dev",
]

[plugins.sandbox]
enabled = true
backend = "native"               # native, docker, bwrap, landlock

[plugins.permissions]
default_policy = "ask"           # ask, allow, deny
workspace_only = true
allow_network = false
allow_shell = false

[plugins.permissions.filesystem]
read_paths = []
write_paths = []
blocked_paths = [
    "/etc",
    "/root",
    "~/.ssh",
    "~/.gnupg",
    "~/.aws",
]

[plugins.permissions.network]
allowed_domains = []
blocked_domains = ["malware.com", "phishing.net"]
```

### 8.5 Security Checklist

| # | Item | Status | Implementation |
|---|------|--------|----------------|
| 1 | Plugin isolation | Required | Sandbox per plugin |
| 2 | Permission prompts | Required | Ask on first use |
| 3 | Signature verification | Optional | Ed25519 signatures |
| 4 | Network filtering | Required | Domain/port allowlist |
| 5 | Filesystem scoping | Required | Path restrictions |
| 6 | Secret protection | Required | No API key exposure |
| 7 | Update verification | Required | Check signatures on update |
| 8 | Audit logging | Required | Log all plugin actions |

---

## Appendix A: Quick Reference

### Claude Code Plugin Manifest (Complete)

```json
{
  "name": "string (required)",
  "version": "string",
  "description": "string",
  "author": {
    "name": "string",
    "email": "string",
    "url": "string"
  },
  "homepage": "string (URL)",
  "repository": "string (URL)",
  "license": "string (SPDX)",
  "keywords": ["string"],
  "commands": ["./path/to/command.md"],
  "agents": "./path/to/agents/",
  "skills": "./path/to/skills/",
  "hooks": "./path/to/hooks.json",
  "mcpServers": "./path/to/mcp.json",
  "lspServers": "./path/to/lsp.json",
  "outputStyles": "./path/to/styles/"
}
```

### Housaky SKILL.toml Format

```toml
[skill]
name = "string"
description = "string"
version = "string"
author = "string"
tags = ["string"]

[[tools]]
name = "string"
description = "string"
kind = "shell|http|script"
command = "string"
[tools.args]
key = "value"

prompts = ["string"]
```

### MCP Tool Definition

```json
{
  "name": "string",
  "title": "string",
  "description": "string",
  "inputSchema": {
    "type": "object",
    "properties": {},
    "required": []
  },
  "outputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

---

## Appendix B: Registry URLs

| Registry | Index URL | Type |
|----------|-----------|------|
| Official | `https://raw.githubusercontent.com/anthropics/claude-plugins-official/main/marketplace.json` | JSON |
| Community | `https://claude-plugins.dev/api/plugins` | REST API |
| GitHub Search | `https://api.github.com/search/repositories?q=SKILL.md+in:path` | REST API |
| Open Skills | `https://github.com/besoeasy/open-skills` | Git Repo |

---

## Appendix C: File Extensions

| Extension | Format | Description |
|-----------|--------|-------------|
| `.mcpb` | ZIP | MCP Bundle (Desktop Extension) |
| `.dxt` | ZIP | Legacy Desktop Extension (renamed to .mcpb) |
| `.json` | JSON | MCP config, LSP config, hooks |
| `.md` | Markdown | Skills, commands, agents |
| `.toml` | TOML | Housaky skill manifests |

---

**Document Version**: 1.0  
**Last Updated**: February 2026  
**Maintainers**: Housaky Team

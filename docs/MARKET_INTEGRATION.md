# MARKET integration (Claude Skills + ClawHub/OpenClaw)

This document describes how to integrate **Claude Code skills (SKILL.md with YAML frontmatter)** and **OpenClaw/ClawHub ecosystem assets** into Housaky.

It is based on `MARKET-housaky.md` (Feb 2026) and reflects what is implemented in this repository today.

## What is implemented

### 1) Claude SKILL.md → Housaky SKILL.toml transpilation

Housaky now supports converting Claude-style `SKILL.md` files (YAML frontmatter + Markdown body) into Housaky-native `SKILL.toml`.

- Implementation: `src/skills/claude.rs`
- CLI: `housaky skills convert <path/to/SKILL.md>`

**Mapping**:
- `name`, `description`, `version`, `author` → mapped directly.
- `triggers.*` → mapped into `tags` with prefixes:
  - `actions` → `action:<value>`
  - `contexts` → `ctx:<value>`
  - `projects` → `project:<value>`
  - `elements` → `ui:<value>`
  - `styles` → `style:<value>`
- `tools_allowed`, `tools_restricted` → currently mapped into tags:
  - `tools_allowed:<tool>`
  - `tools_restricted:<tool>`

The Markdown body is stored as `prompts = ["..."]`.

### 2) SkillForge scaffolding (GitHub scout + integrator)

Housaky includes `SkillForge` which can discover repositories and generate placeholder skills.

- `src/skillforge/scout.rs`: GitHub scout implemented.
- `ScoutSource::ClawHub` exists but is not implemented yet.

## How to bring Claude marketplace skills into Housaky

### A) Manual workflow

1. Clone a repo that contains Claude `SKILL.md`.
2. Convert it:

```bash
housaky skills convert path/to/SKILL.md > SKILL.toml
```

3. Place it in your workspace:

```text
~/.housaky/workspace/skills/<skill-name>/SKILL.toml
```

### B) Future: plugin/marketplace sync

`MARKET-housaky.md` proposes a full plugin loader + marketplace sync + MCP integration. Those pieces are not fully implemented yet.

## OpenClaw / ClawHub integration

- This repository contains an `openclaw/` folder (vendored).
- There is migration support for OpenClaw memory: `housaky migrate openclaw ...`.

**Next steps (planned)**:
- Implement `ClawHub` scout in `SkillForge` to fetch and ingest skills from https://github.com/openclaw/clawhub.
- Add optional MCP support to run `.mcp.json` servers exposed by Claude plugins.

## Security note

Claude skills can include tool permissions (`tools_allowed`, `tools_restricted`).
Housaky currently enforces security via `SecurityPolicy` on tools at runtime.

For now:
- permissions are preserved as tags for auditability;
- do not auto-enable untrusted skills.

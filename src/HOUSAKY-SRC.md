# Housaky Source Code Review

## Overview

Housaky is a comprehensive Rust-based AI assistant framework designed for autonomous agent operation with AGI (Artificial General Intelligence) aspirations. The project consists of **469 Rust source files** organized into **38 top-level modules**.

---

## Module Architecture

### 1. Core Agent System (`housaky/`)

The central AGI module containing 30 submodules:

| Module | File | Purpose |
|--------|------|---------|
| **core.rs** | `housaky/core.rs` | Main `HousakyCore` orchestrator wiring all AGI components |
| **agent.rs** | `housaky/agent.rs` | Core `Agent` struct for task execution |
| **agi_integration.rs** | `housaky/agi_integration.rs` | AGIIntegrationHub for phase-based reasoning |
| **heartbeat.rs** | `housaky/heartbeat.rs` | 2-minute autonomous heartbeat cycle |
| **goal_engine.rs** | `housaky/goal_engine.rs` | Persistent goal management with decomposition |
| **reasoning_engine.rs** | `housaky/reasoning_engine.rs` | Chain of Thought, ReAct, Tree of Thoughts |
| **knowledge_graph.rs** | `housaky/knowledge_graph.rs` | Entity-relationship semantic memory |
| **tool_creator.rs** | `housaky/tool_creator.rs` | Automatic tool generation and testing |
| **working_memory.rs** | `housaky/working_memory.rs` | Token-budgeted context management |
| **meta_cognition.rs** | `housaky/meta_cognition.rs` | Self-reflection and introspection |
| **web_browser.rs** | `housaky/web_browser.rs` | Safe web content fetching/search |
| **inner_monologue.rs** | `housaky/inner_monologue.rs` | Internal thought persistence |
| **decision_journal.rs** | `housaky/decision_journal.rs` | Decision audit trail |
| **self_improvement_loop.rs** | `housaky/self_improvement_loop.rs` | Continuous self-improvement cycles |
| **ai_prove.rs** | `housaky/ai_prove.rs` | Cryptographic AI verification |
| **kowalski_integration.rs** | `housaky/kowalski_integration.rs` | Multi-agent federation |
| **quantum_integration.rs** | `housaky/quantum_integration.rs` | Quantum computing bridge |

### Integration Points (Core → Other Modules)

```
HousakyCore (core.rs)
    ├── goal_engine.rs → reads/writes workspace .housaky/goals.json
    ├── knowledge_graph.rs → uses memory backend (sqlite/lucid)
    ├── tool_creator.rs → generates tools in tools/ directory
    ├── reasoning_pipeline.rs → uses providers for LLM calls
    ├── cognitive/ → world model and planning
    ├── memory/ → episodic, hierarchical memory systems
    ├── singularity/ → singularity phase management
    ├── quantum/bridge.rs → quantum backend (Amazon Braket)
    └── heartbeat.rs → triggers self-improvement cycles
```

---

### 2. Providers Module (`providers/`)

**Purpose:** Multi-provider LLM abstraction with fallback and routing

| File | Provider(s) |
|------|-------------|
| `openai.rs` | OpenAI API |
| `anthropic.rs` | Anthropic Claude |
| `gemini.rs` | Google Gemini |
| `ollama.rs` | Local Ollama |
| `openrouter.rs` | OpenRouter aggregation |
| `compatible.rs` | OpenAI-compatible generic |
| `fallback.rs` | Fallback chain manager |
| `router.rs` | Model-based routing |
| `subject_router.rs` | Per-subject routing with concurrency limits |
| `reliable.rs` | Retry/backoff wrapper |
| `limit.rs` | Concurrency limiting |

**Supported Providers (40+):**
- Primary: OpenAI, Anthropic, Google Gemini, Ollama, OpenRouter
- OpenAI-compatible: Venice, Vercel, Cloudflare, Moonshot, Groq, Mistral, xAI, DeepSeek, Together AI, Fireworks, Perplexity, Cohere, Bedrock, Qianfan, Qwen, GLM, MiniMax, ZAI, OpenCode

**Wiring:**
```
main.rs → create_routed_provider() → providers/mod.rs
    ├── create_provider() for each provider
    ├── ReliableProvider wraps with retry/backoff
    ├── SubjectRouterProvider for per-subject routing
    └── Used by: agent/loop_.rs, channels/, TUI
```

---

### 3. Tools Module (`tools/`)

**Purpose:** Tool registry for agent execution

| Tool | File | Function |
|------|------|----------|
| `ShellTool` | `shell.rs` | Execute shell commands |
| `FileReadTool` | `file_read.rs` | Read file contents |
| `FileWriteTool` | `file_write.rs` | Write files |
| `FileListTool` | `file_list.rs` | Directory listing |
| `FileSearchTool` | `file_search.rs` | Grep-based search |
| `BrowserTool` | `browser.rs` | Full browser automation |
| `HttpRequestTool` | `http_request.rs` | HTTP GET/POST |
| `GitOperationsTool` | `git_operations.rs` | Git operations |
| `MemoryStoreTool` | `memory_store.rs` | Store to memory |
| `MemoryRecallTool` | `memory_recall.rs` | Semantic recall |
| `DelegateTool` | `delegate.rs` | Sub-agent delegation |
| `ComposioTool` | `composio.rs` | External tool platform |
| `ScheduleTool` | `schedule.rs` | Cron-like scheduling |
| `ScreenshotTool` | `screenshot.rs` | Screen capture |
| `SkillTool` | `skill_tool.rs` | Skill-defined tools |

**Wiring:**
```
tools/mod.rs
    ├── default_tools() → basic 8 tools
    ├── all_tools() → full registry with config
    └── Used by: agent/loop_.rs, channels/mod.rs
```

---

### 4. Memory Module (`memory/`)

**Purpose:** Persistent memory with embeddings

| Backend | File | Features |
|---------|------|----------|
| `SqliteMemory` | `sqlite.rs` | Full-text + vector embeddings |
| `LucidNativeMemory` | `lucid_native.rs` | Lucid hybrid storage |
| `MarkdownMemory` | `markdown.rs` | File-based plain text |
| `NoneMemory` | `none.rs` | No-op for testing |

**Supporting Modules:**
- `embeddings.rs` - Embedding provider abstraction (OpenAI, Ollama)
- `chunker.rs` - Document chunking for context windows
- `hygiene.rs` - Memory cleanup/retention
- `snapshot.rs` - Export/import for cold boot
- `response_cache.rs` - LLM response caching

**Wiring:**
```
memory/mod.rs → create_memory()
    ├── SqliteMemory with embedder
    ├── LucidNativeMemory
    ├── MarkdownMemory
    └── Used by: HousakyCore, channels, tools
```

---

### 5. Skills Module (`skills/`)

**Purpose:** Extensible skill system

| File | Function |
|------|----------|
| `mod.rs` | Skill loading from workspace + open-skills repo |
| `marketplace.rs` | Community skill marketplace |
| `invocation.rs` | Automatic skill trigger on message match |
| `claude.rs` | Claude skill compatibility |

**Wiring:**
```
skills/mod.rs
    ├── load_skills() → workspace/skills/ + open-skills repo
    ├── load_active_skills() → filter by config
    └── Used by: housaky/core.rs (skill_invocation_engine)
```

---

### 6. Channels Module (`channels/`)

**Purpose:** Multi-platform message integration

| Channel | File | Status |
|---------|------|--------|
| Telegram | `telegram.rs` | Full implementation |
| Discord | `discord.rs` | Full implementation |
| Slack | `slack.rs` | Full implementation |
| IRC | `irc.rs` | Full implementation |
| Matrix | `matrix.rs` | Full implementation |
| Email | `email_channel.rs` | Full implementation |
| WhatsApp | `whatsapp.rs` | Full implementation |
| Lark/Feishu | `lark.rs` | Full implementation |
| DingTalk | `dingtalk.rs` | Full implementation |
| iMessage | `imessage.rs` | Full implementation |
| Voice | `voice.rs` | ElevenLabs TTS/STT |

**Wiring:**
```
channels/mod.rs
    ├── start_channels() → spawn listeners for each channel
    ├── AGIChannelProcessor → process with AGI
    └── Used by: main.rs (ChannelCommands)
```

---

### 7. TUI Module (`tui/`)

**Purpose:** Terminal user interfaces

| TUI Variant | Directory | Description |
|-------------|-----------|-------------|
| Minimal | `minimal/` | Default lightweight TUI |
| Enhanced | `enhanced_app/` | Full-featured with panels |
| AGI | `agi/` | AGI dashboard |
| Live | `live/` | Real-time metrics |
| Command Palette | `command/` | Command execution |

**Wiring:**
```
tui/mod.rs
    ├── run_minimal_tui() → default entry
    ├── run_enhanced_app() → full TUI
    ├── run_agi_tui() → AGI dashboard
    └── Used by: main.rs (no subcommand)
```

---

### 8. Configuration (`config/`)

**Purpose:** Centralized configuration management

| File | Purpose |
|------|---------|
| `schema.rs` | Full config struct (159KB, 4000+ lines) |
| `mod.rs` | Re-exports |
| `watcher.rs` | Live config hot-reload |

**Key Config Sections:**
- `default_provider`, `default_model`, `default_temperature`
- `memory.backend` (sqlite/lucid/markdown/none)
- `browser.enabled`, `browser.computer_use`
- `channels.*` - per-channel configs
- `quantum.enabled` - quantum bridge
- `skills.enabled` - skill activation map

---

### 9. Security Module (`security/`)

**Purpose:** Sandboxing and secrets management

| File | Function |
|------|----------|
| `policy.rs` | Security policy rules |
| `landlock.rs` | Linux landlock sandbox |
| `secrets.rs` | Secrets detection/scrubbing |
| `audit.rs` | Operation auditing |
| `pairing.rs` | Key exchange |

**Wiring:**
```
main.rs → security policy → tools/, channels/, runtime/
```

---

### 10. Other Supporting Modules

| Module | Files | Purpose |
|--------|-------|---------|
| `cli/` | 6 | CLI argument parsing, handlers |
| `hooks/` | 6 | Pre/post execution hooks |
| `daemon/` | 2 | Background service |
| `gateway/` | 1 | HTTP API gateway (45KB) |
| `tunnel/` | 6 | Cloudflare, ngrok, Tailscale tunnels |
| `runtime/` | 6 | Native, Docker, WASM execution |
| `health/` | 1 | Health checking |
| `heartbeat/` | 2 | Legacy heartbeat engine |
| `quantum/` | 12 | Quantum computing backends |
| `rag/` | - | Retrieval-augmented generation |
| `identity.rs` | 1 | Identity/SSH management |
| `migration.rs` | 1 | Data migration |
| `cost/` | - | Cost tracking |

---

## Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         main.rs                                      │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐ │
│  │ No subcommand   │  │ CLI Commands    │  │ Daemon/Gateway      │ │
│  │ → TUI           │  │ → handlers      │  │ → background        │ │
│  └────────┬────────┘  └────────┬────────┘  └──────────┬──────────┘ │
└───────────┼──────────────────────┼──────────────────────┼────────────┘
            │                      │                      │
            ▼                      ▼                      ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      housaky::handle_command()                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │ Goals        │  │ Improve      │  │ Heartbeat    │              │
│  │ goal_engine  │  │ self_improv  │  │ heartbeat    │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
└─────────────────────────────────────────────────────────────────────┘
            │                      │                      │
            ▼                      ▼                      ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      HousakyCore (core.rs)                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │ Goal Engine │  │ Reasoning    │  │ Knowledge    │              │
│  │             │  │ Pipeline     │  │ Graph        │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │ Tool Creator│  │ Meta         │  │ Quantum      │              │
│  │             │  │ Cognition    │  │ Bridge       │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
└─────────────────────────────────────────────────────────────────────┘
            │                      │                      │
            ▼                      ▼                      ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        Agent Loop (agent/loop_.rs)                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │ Provider     │  │ Tools        │  │ Memory       │              │
│  │ (LLM call)   │  │ Registry     │  │ (recall/     │              │
│  │              │  │              │  │  store)      │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
└─────────────────────────────────────────────────────────────────────┘
            │                      │                      │
            ▼                      ▼                      ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    External Integrations                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │ Providers    │  │ Channels     │  │ Tools        │              │
│  │ OpenAI/      │  │ Telegram/    │  │ Shell/       │              │
│  │ Anthropic/   │  │ Discord/     │  │ File/        │              │
│  │ Gemini...    │  │ Slack...     │  │ Browser      │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Key Integration Points

### 1. Entry Points
- **Interactive TUI:** `main.rs` → no subcommand → `start_full_system()` → `tui::run_minimal_tui()`
- **CLI Commands:** `main.rs` → `cli::parse()` → `housaky::handle_command()` → module-specific handlers

### 2. Provider Integration
```
providers/mod.rs
    ├── create_provider(name, key) → specific provider impl
    ├── create_resilient_provider() → adds retry/fallback
    └── create_routed_provider() → subject-based routing
```

### 3. Tool Integration
```
tools/mod.rs
    ├── default_tools() → 8 basic tools
    └── all_tools(config) → full registry (20+ tools)
```

### 4. Memory Integration
```
memory/mod.rs
    ├── create_memory(config, workspace, api_key)
    └── create_response_cache(config, workspace)
```

### 5. Channel Integration
```
channels/mod.rs
    ├── start_channels(config) → spawn listeners
    └── AGIChannelProcessor → process with AGI
```

---

## What's Missing / Gaps

### 1. **Unified Runtime Abstraction**
- Multiple runtime implementations (native, docker, wasm, cloudflare) but no unified interface for tool execution context
- **Missing:** Runtime selection based on security policy

### 2. **Tool Result Schema Standardization**
- Each tool returns `ToolResult` with free-form `output: String`
- **Missing:** Structured output schemas for better tool composition

### 3. **Distributed Agent Communication**
- A2A protocol exists (`a2a.rs`) but not fully integrated with channels
- **Missing:** Unified message format across all channels

### 4. **Verification Pipeline**
- `ai_prove.rs` provides cryptographic verification but not actively used in tool execution
- **Missing:** Automated proof generation for tool outputs

### 5. **Skill-Tool Bridge**
- Skills can define tools (`SkillTool`) but no automatic registration into main tool registry
- **Missing:** `skill_tools → tools::all_tools()` integration

### 6. **Memory Backend Diversity**
- Only 4 backends (sqlite, lucid, markdown, none)
- **Missing:** Redis, PostgreSQL with vector extensions, Pinecone, Weaviate

### 7. **Observability Integration**
- Flight journal exists but not wired to external observability (OpenTelemetry)
- **Missing:** Standard span/trace export

### 8. **Configuration Hot-Reload**
- Config watcher exists but limited scope
- **Missing:** Full hot-reload for all config sections without restart

### 9. **Cost Tracking**
- Cost module exists but not integrated into provider calls
- **Missing:** Per-request cost tracking and budget enforcement

### 10. **Web Interface**
- Dashboard module exists but requires separate installation
- **Missing:** Built-in web UI (similar to ChatGPT)

---

## Test Coverage

The codebase includes unit tests embedded in each module:
- `providers/mod.rs` - 50+ provider factory tests
- `tools/mod.rs` - Tool registry tests
- `memory/mod.rs` - Backend factory tests
- `skills/mod.rs` - Skill loading tests

---

## Build System

- **Cargo workspace** with single `Cargo.toml`
- **Dependencies:** 150+ crates (tokio, serde, ratatui, reqwest, etc.)
- **Features:** Conditional compilation for browser, quantum, wasm

---

## Summary

Housaky is a mature, feature-rich autonomous agent framework with:
- ✅ Comprehensive multi-provider LLM support (40+ providers)
- ✅ Multi-channel message integration (11+ platforms)
- ✅ Full tool system with 20+ built-in tools
- ✅ Advanced AGI components (goal engine, reasoning, knowledge graph)
- ✅ Quantum computing bridge
- ✅ Self-improvement loop
- ✅ Terminal UI with multiple variants

**Key integration gaps to address:**
1. Skill-defined tools not automatically available to agent
2. Memory backends limited to file-based/SQL
3. No structured tool output schemas
4. Distributed agent communication incomplete
5. Cost tracking not enforced
6. Built-in web interface missing

The architecture is well-structured with clear separation of concerns, but could benefit from tighter integration between the skills system and tool registry, and more robust observability.

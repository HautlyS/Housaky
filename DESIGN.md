# Housaky -- Technical Architecture Design Document

> Version 2.0.0 | Single-binary autonomous AGI framework, 100% Rust

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [High-Level Architecture](#2-high-level-architecture)
3. [Crate & Module Map](#3-crate--module-map)
4. [The AGI Core (`src/housaky/`)](#4-the-agi-core-srchousaky)
5. [Provider System](#5-provider-system)
6. [Kowalski Multi-Agent Orchestrator](#6-kowalski-multi-agent-orchestrator)
7. [Subagent System](#7-subagent-system)
8. [Unified Agent Hub](#8-unified-agent-hub)
9. [GSD Orchestration](#9-gsd-orchestration)
10. [Tool System](#10-tool-system)
11. [Memory & Persistence](#11-memory--persistence)
12. [Channel System](#12-channel-system)
13. [Communication Protocols (A2A, HIIP, HDIN)](#13-communication-protocols)
14. [Gateway & API Layer](#14-gateway--api-layer)
15. [Security Architecture](#15-security-architecture)
16. [Hardware & Firmware Integration](#16-hardware--firmware-integration)
17. [Dashboard & Landing](#17-dashboard--landing)
18. [Prompt Engineering Architecture](#18-prompt-engineering-architecture)
19. [Skills System](#19-skills-system)
20. [CAS Integration for Kowalski](#20-cas-integration-for-kowalski)
21. [CAS-Powered Claude Code Router Architecture](#21-cas-powered-claude-code-router-architecture)
22. [CAS + Lucid Memory Integration](#22-cas--lucid-memory-integration)
23. [Data Flow Diagrams](#23-data-flow-diagrams)

---

## 1. System Overview

Housaky is a **single-binary, 100% Rust autonomous AGI framework** that combines:

- **Agentic LLM orchestration** across 40+ providers (OpenRouter, Anthropic, OpenAI, Gemini, Ollama, Modal, GLM/Zhipu, and 20+ OpenAI-compatible endpoints)
- **Multi-agent coordination** via the Kowalski orchestrator and SubAgent system
- **Hardware integration** with Arduino, ESP32, STM32/Nucleo, Raspberry Pi GPIO
- **17 messaging channels** (Telegram, Discord, Slack, WhatsApp, iMessage, Email, IRC, Matrix, Line, Signal, DingTalk, Lark, Voice, WebChat, CLI)
- **Agent-to-Agent (A2A) protocol** for collaborative self-improvement between Housaky instances
- **AGI subsystems**: goal engine, reasoning pipeline, knowledge graph, consciousness monitoring, self-improvement loops, quantum computing bridge

### Design Principles

| Principle | Implementation |
|---|---|
| Zero overhead | `opt-level = "z"`, thin LTO, `strip = true`, `panic = "abort"` |
| Single binary | NOT a Cargo workspace -- one crate, 38 modules, ~469 `.rs` files |
| Trait-based plugins | Providers, tools, channels, memory, runtimes are all trait-object swappable |
| Security-in-depth | 7+ layers from gateway auth to kernel-level Landlock sandboxing |
| Local-first | SQLite + vector embeddings, no mandatory cloud dependency |

---

## 2. High-Level Architecture

```
                          +---------------------------+
                          |     User Interfaces       |
                          |  TUI (ratatui/crossterm)  |
                          |  Dashboard (Tauri/Vue 3)  |
                          |  Gateway (Axum HTTP/WS)   |
                          |  17 Channels              |
                          +-----------+---------------+
                                      |
                          +-----------v---------------+
                          |       Agent Core          |
                          |  Agent::turn() loop       |
                          |  Provider -> Tool -> Mem  |
                          +-----------+---------------+
                                      |
              +--------+---------+----+----+---------+--------+
              |        |         |         |         |        |
        +-----v--+ +---v----+ +-v------+ +v------+ +v------+ +v--------+
        |Provider| | Tools  | |Memory  | |Kowal- | |Sub-   | |GSD      |
        |System  | | (35+)  | |System  | |ski    | |Agent  | |Orchestr.|
        |40+ LLM | |File,   | |Lucid,  | |Bridge | |System | |Spec-    |
        |adapters| |Shell,  | |Markdown| |7 agent| |7 role | |driven   |
        |        | |Git,    | |None    | |types  | |types  | |waves    |
        +--------+ |Browser | +--------+ +-------+ +-------+ +---------+
                    |Delegate|
                    |Memory  |     +-----------------------------------+
                    |Hardware|     |       AGI Core (src/housaky/)     |
                    +--------+     | Goal Engine, Reasoning Pipeline, |
                                   | Knowledge Graph, Meta-Cognition, |
                                   | Self-Improvement, Consciousness, |
                                   | Cognitive (25 files), Streaming  |
                                   +-----------------------------------+
                                                |
              +----------+----------+-----------+----------+
              |          |          |           |          |
         +----v---+ +---v----+ +---v-----+ +--v------+ +-v-------+
         |A2A     | |Federat.| |Quantum  | |Hardware | |Security |
         |Protocol| |Hub     | |Bridge   | |USB/GPIO | |Landlock |
         |File+WS | |CRDT    | |Braket   | |Serial   | |Docker   |
         +--------+ +--------+ +---------+ +---------+ |Secrets  |
                                                        +---------+
```

---

## 3. Crate & Module Map

Single crate `housaky` (v0.1.0) with **38 top-level modules** in `src/lib.rs`:

| Module | Purpose | Key Types |
|---|---|---|
| `agent` | Core agent loop | `Agent`, `AgentBuilder`, `process_message` |
| `a2a_prove` | Cryptographic proof for AI outputs | `A2AProve` |
| `channels` | 17 messaging channel adapters | `Channel` trait, `AGIChannelProcessor` |
| `cli` | Clap CLI parsing, first-run | `CliArgs`, `Commands` |
| `commands` | 40+ CLI subcommand enums | `HousakyCommand` |
| `config` | Config schema (4000+ lines), watcher | `HousakyConfig`, hot-reload |
| `config_editor` | TUI config editor | Section widgets |
| `cost` | API cost tracking | `CostTracker` |
| `cron` | Scheduled task execution | `CronScheduler` |
| `daemon` | Background service supervisor | `DaemonRunner` |
| `dashboard` | Tauri desktop launcher | Vue 3 bridge |
| `doctor` | System diagnostics | `DoctorCheck` |
| `gateway` | Axum HTTP/WS gateway | REST API, rate limiting, pairing auth |
| `hardware` | USB device discovery (`nusb`) | VID/PID matching |
| `health` | Component health tracking | `HealthStatus` |
| `heartbeat` | Autonomous self-improvement trigger | Periodic cycles |
| `hooks` | Pre/post execution hooks | Hook rules from `.md` frontmatter |
| `housaky` | **The AGI core** (70+ files, 27 dirs) | `HousakyCore` |
| `human_readonly` | Read-only human review artifacts | Audit trail |
| `identity` | SSH key management (27KB) | Identity generation |
| `integrations` | External service connectors | Registry pattern |
| `keys_manager` | Multi-provider API key mgmt | `KeysManager`, rotation, per-subagent keys |
| `mcp` | Model Context Protocol integration | MCP marketplace |
| `memory` | Memory backends | `LucidNativeMemory` (SQLite+vector), `MarkdownMemory` |
| `migration` | Data migration utilities | Schema versioning |
| `observability` | OpenTelemetry, Prometheus | Metrics export |
| `onboard` | First-run onboarding wizard | Provider setup |
| `peripherals` | Serial peripheral comm | Arduino, Nucleo, ESP32 flash |
| `providers` | **40+ LLM provider adapters** | `Provider` trait, resilient chains |
| `quantum` | Quantum backends (Amazon Braket) | QAOA, Grover's, VQE |
| `rag` | Retrieval-augmented generation | Document chunking |
| `runtime` | Execution runtime abstraction | Native, Docker, WASM (`wasmi`), Cloudflare Workers |
| `security` | Defense-in-depth | Landlock, Bubblewrap, Firejail, Docker sandbox, ChaCha20 secrets |
| `service` | OS service installer | systemd unit generation |
| `skillforge` | Skill generation tooling | Scaffold + validate |
| `skills` | Dynamic skill system | Load from workspace + marketplace, auto-trigger |
| `telemetry` | Telemetry collection | Anonymous usage data |
| `tools` | **35+ agent tools** | `Tool` trait, `default_tools()`, `all_tools()` |
| `tui` | Terminal UIs (Minimal, Enhanced, AGI) | `ratatui` + `crossterm` |
| `tunnel` | Cloudflare, ngrok, Tailscale tunnels | Adapter pattern |
| `util` | Shared utilities | Common helpers |
| `web_interface` | Web interface support | Browser-based UI |

---

## 4. The AGI Core (`src/housaky/`)

**70+ Rust files, 27 subdirectories, ~25,000+ lines** organized into evolutionary phases:

### Phase 1 -- Core AGI (wired in `core.rs`)

| Component | File(s) | Description |
|---|---|---|
| `HousakyCore` | `core.rs` (2046 lines) | Master orchestrator wiring all AGI components |
| Agent Loop | `agent/` | `UnifiedAgentLoop`, session management |
| Goal Engine | `goal_engine.rs` | Persistent goals: decomposition, priority (Critical/High/Medium/Low), progress |
| Reasoning | `reasoning_engine.rs`, `reasoning_pipeline.rs` | Chain-of-Thought, ReAct, Tree-of-Thoughts strategies |
| Cognitive | `cognitive/` (25 files) | World model, meta-learning, planning, imagination, curiosity, learning pipeline, pattern recognition, attention, theory of mind, creativity, executive controller |
| Knowledge Graph | `knowledge_graph.rs` | Entity-relationship semantic memory |
| Meta-Cognition | `meta_cognition.rs` | Self-reflection, capability assessment, confidence |
| Inner Monologue | `inner_monologue.rs` | Internal thought persistence (SQLite) |
| Working Memory | `working_memory.rs` | Token-budgeted context management |
| Tool Creator | `tool_creator.rs` | Automatic tool generation from descriptions |
| Alignment | `alignment/` (5 files) | Ethical reasoning gate |
| Memory | `memory/` | Episodic, hierarchical, consolidated memory |
| Streaming | `streaming/` | Response streaming to TUI/channels |

### Phase 2 -- Self-Improvement

| Component | File | Description |
|---|---|---|
| Self-Improvement Loop | `self_improvement_loop.rs` (2131 lines) | Full recursive self-improvement cycle |
| Recursive Self-Modifier | `recursive_self_modifier.rs` | Code modification pipeline |
| Rust Code Modifier | `rust_code_modifier.rs` | AST modification via `syn`/`quote` |
| Git Sandbox | `git_sandbox.rs` | Isolated testing of modifications |
| Fitness Evaluator | `fitness_evaluator.rs` | Empirical fitness evaluation |
| Capability Tracker | `capability_growth_tracker.rs` | Growth metrics |

### Phase 3 -- Distributed Cognition

Federation transport, neuromorphic engine, quantum bridge, A2A protocol stack (file-based + WebSocket + X25519+ChaCha20-Poly1305 encryption).

### Phase 4 -- Consciousness & Emergence

Consciousness meter (IIT 4.0 Phi), narrative self, global workspace, singularity engine, swarm controller, collective memory, emergence detection.

### Phase 5-8 -- Advanced AGI

Physical embodiment, perception fusion, neural architecture search, collective intelligence (Moltbook), HDIN Seed Mind (multi-timescale weights, DGM self-improvement, karma system).

---

## 5. Provider System

**File:** `src/providers/mod.rs`

### Provider Trait

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    async fn chat(&self, messages: &[ChatMessage], model: &str, temperature: f64) -> Result<String>;
    async fn simple_chat(&self, prompt: &str, model: &str, temperature: f64) -> Result<String>;
    async fn chat_with_system(&self, system: &str, prompt: &str, model: &str, temp: f64) -> Result<String>;
    // ... streaming, tool calls, etc.
}
```

### 40+ Provider Adapters

OpenRouter, Anthropic, OpenAI, Ollama, Gemini, Groq, Mistral, xAI, DeepSeek, Together AI, Fireworks, **Modal**, GLM/Zhipu, custom URLs, and 20+ OpenAI-compatible endpoints.

### Key Resolution Order

```
1. Explicit api_key parameter
2. Provider-specific env var (ANTHROPIC_API_KEY, OPENROUTER_API_KEY, etc.)
3. Generic fallback vars (HOUSAKY_API_KEY, API_KEY)
4. KeysManager global singleton (~/.housaky/keys.json)
```

### Resilient Provider Chains

`create_resilient_provider()` wraps providers with retry, backoff, API key pools, and model fallbacks. `create_routed_provider()` builds `SubjectRouterProvider` or `RouterProvider` for per-subject routing.

```
SubjectRouterProvider
  |-- "hint:fast"    -> Provider A (cheap model)
  |-- "hint:reason"  -> Provider B (reasoning model)
  |-- default        -> Provider C (general model)
```

---

## 6. Kowalski Multi-Agent Orchestrator

**File:** `src/housaky/kowalski_integration.rs`

### Architecture

```
KowalskiBridge
  |-- keys_manager: Arc<KeysManager>         (global singleton)
  |-- subagent_configs: HashMap<String, SubAgentConfig>  (from keys.json)
  |-- agents: Vec<KowalskiAgent>             (7 specialized agents)
  |-- cli_path: Option<PathBuf>              (external CLI binary)
```

### Seven Specialized Agent Types

| Agent | Name | Specialization |
|---|---|---|
| Code | `kowalski-code` | Code analysis, refactoring, documentation |
| Web | `kowalski-web` | Web research, information retrieval |
| Academic | `kowalski-academic` | Paper analysis, scholarly content |
| Data | `kowalski-data` | Data analysis and processing |
| Creative | `kowalski-creative` | Synthesis and idea generation |
| Reasoning | `kowalski-reasoning` | Logical reasoning, deduction |
| Federated | `kowalski-federation` | Multi-agent coordination |

### Task Execution Pipeline

```
send_task(agent_name, task)
  |
  +-> Look up agent in self.agents
  +-> Fetch SubAgentConfig from keys.json
  +-> keys_manager.get_key_for_subagent(agent_name) -> (model, KeyEntry)
  +-> execute_with_provider(provider, model, key, task)
        |
        +-> Determine base_url from provider:
        |     "modal"     -> https://api.us-west-2.modal.direct/v1
        |     "openrouter"-> https://openrouter.ai/api/v1
        |     "openai"    -> https://api.openai.com/v1
        |     "anthropic" -> https://api.anthropic.com/v1
        |     "ollama"    -> http://localhost:11434/v1
        |
        +-> Build OpenAI-compatible chat completion request
        +-> Send with role-specific system prompt
        +-> Parse response
```

### Retry Logic

`send_task_with_retry()`: exponential backoff, up to 3 retries, handles rate limits and timeouts.

### API Key Isolation

Each agent gets its own key from `~/.housaky/keys.json`:

```json
{
  "subagents": {
    "kowalski-code":    { "provider": "modal", "model": "zai-org/GLM-5-FP8", "key_name": "earth.tupa" },
    "kowalski-web":     { "provider": "modal", "model": "zai-org/GLM-5-FP8", "key_name": "hautlythird" },
    "kowalski-academic":{ "provider": "modal", "model": "zai-org/GLM-5-FP8", "key_name": "tupa@" },
    "kowalski-data":    { "provider": "modal", "model": "zai-org/GLM-5-FP8", "key_name": "touch" },
    "kowalski-creative":{ "provider": "modal", "model": "zai-org/GLM-5-FP8", "key_name": "rouxy" },
    "kowalski-reasoning":{ "provider": "modal", "model": "zai-org/GLM-5-FP8", "key_name": "hautly" },
    "kowalski-federation":{ "provider": "modal", "model": "zai-org/GLM-5-FP8", "key_name": "housaky" }
  }
}
```

---

## 7. Subagent System

**File:** `src/housaky/subagent_system.rs`

### Core Trait

```rust
#[async_trait]
pub trait SubAgent: Send + Sync {
    fn identity(&self) -> &AgentIdentity;
    async fn process(&mut self, task: &str, context: &AgentContext) -> Result<AgentResponse>;
    fn state(&self) -> &AgentState;
    fn awareness(&self) -> &[String];
    fn api_key(&self) -> &str;
    fn model(&self) -> &str { "zai-org/GLM-5-FP8" }
}
```

### BaseSubAgent -- Direct API Integration

Each `BaseSubAgent` makes direct HTTP calls to `https://api.us-west-2.modal.direct/v1/chat/completions` using the agent's own API key, sending OpenAI-compatible chat completion requests with:

1. Role-specific system prompt
2. Conversation history from context
3. Awareness context ("I am X. I am aware of: Y, Z...")
4. The user task

### SubAgentOrchestrator -- The Collective Mind

```rust
pub struct SubAgentOrchestrator {
    agents: HashMap<String, Arc<RwLock<Box<dyn SubAgent>>>>,
    keys: HashMap<String, String>,
    collective_memory: Arc<Mutex<Vec<Message>>>,  // shared across all agents, capped at 100
    consciousness_level: f64,
}
```

Routes tasks to target agents, maintains collective conversation memory, and tracks consciousness level growth.

### Role-Based Tool Sandboxing

- Only `FederationCoordinator` can use the `delegate` tool
- All other roles have `delegate` in their `denied_tools()` list
- Prevents cascading delegation chains from non-coordinator agents

---

## 8. Unified Agent Hub

**File:** `src/housaky/unified_agents.rs`

Central orchestrator for ALL five agent subsystems:

```rust
pub struct UnifiedAgentHub {
    registry: Arc<AgentRegistry>,
    coordinator: Arc<MultiAgentCoordinator>,
    federation_hub: Option<Arc<FederationHub>>,
    federation_transport: Option<Arc<FederationTransportLayer>>,
    collaboration: Option<Arc<CollaborationManager>>,
    kowalski_bridge: Option<Arc<RwLock<KowalskiBridge>>>,
    subagent_orchestrator: Option<Arc<RwLock<SubAgentOrchestrator>>>,
    replicator: Option<Arc<AgentReplicator>>,
    emergent_protocol: Arc<EmergentProtocol>,
    unified_tasks: Arc<RwLock<HashMap<String, UnifiedTask>>>,
    message_bus: broadcast::Sender<UnifiedHubMessage>,
    shared_knowledge: Arc<RwLock<HashMap<String, LWWRegister<String>>>>,  // CRDTs
    learned_facts: Arc<RwLock<GSet<String>>>,
    vector_clock: Arc<RwLock<VectorClock>>,
}
```

### Task Routing

```
submit_task(UnifiedTask)
  |
  +-> vector_clock.tick() for causal ordering
  +-> select_best_system():
        AgentSystem::Kowalski    -> KowalskiBridge::send_task()
        AgentSystem::SubAgent    -> SubAgentOrchestrator::process()
        AgentSystem::Federation  -> FederationHub::share_knowledge()
        AgentSystem::Collaboration -> CollaborationManager::send_message()
        AgentSystem::Local       -> MultiAgentCoordinator::submit_task()
```

### Inter-Agent Communication

| Pattern | Mechanism |
|---|---|
| Broadcast | `broadcast::channel(512)` |
| Direct | `mpsc::Sender` per agent in AgentRegistry |
| Consensus | `ResponseChannelRegistry` with per-query `mpsc::channel(1)` |
| Collective memory | Shared `Vec<Message>` in SubAgentOrchestrator |
| CRDT knowledge | `LWWRegister`, `GSet`, `VectorClock` for federation |
| Filesystem | HIIP messages in shared directory |
| HTTP | Federation transport, Kowalski API calls |
| Subprocess | `AgentReplicator` (child agent forking via OS process) |

---

## 9. GSD Orchestration

**Files:** `src/housaky/gsd_orchestration/`

Spec-driven project management operating above agent-level orchestration:

```
GSDExecutionEngine
  -> GSDOrchestrator
       -> ContextManager (PROJECT.md, ROADMAP.md, STATE.md)
       -> StepDecomposer (keyword-based complexity scoring)
       -> WaveExecutor (dependency-aware parallel, max 5)
       -> MetaCognitionEngine (post-wave reflection)
       -> GoalEngine (goal tracking)
```

### Decomposition Strategies

`Sequential`, `Parallel`, `Hierarchical`, `Iterative`, `WaveBased` -- selected by complexity score (0-1).

### Execution Modes

`Simulated` (dry run), `Shell` (commands), `Delegate` (subagent, currently stub).

---

## 10. Tool System

**Files:** `src/tools/`

### Tool Trait

```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn spec(&self) -> serde_json::Value;
    async fn execute(&self, args: serde_json::Value, policy: Arc<SecurityPolicy>) -> Result<String>;
}
```

### 35+ Tools

| Category | Tools |
|---|---|
| **File I/O** | FileRead, FileWrite, FileList, FileSearch, FileMove, FileDelete, FileInfo, FileCopy, Archive |
| **Execution** | Shell (sandboxed), Process |
| **Git** | GitOperations |
| **Memory** | MemoryStore, MemoryRecall, MemoryForget, LucidDB |
| **Network** | HttpRequest, Browser (CDP), BrowserOpen, Screenshot |
| **Delegation** | **Delegate** (sub-agent spawning with depth-limited recursion) |
| **Hardware** | HardwareBoardInfo, HardwareMemoryMap, HardwareMemoryRead |
| **Scheduling** | Schedule |
| **Integration** | Composio, Webhook, Database, ImageInfo, ProjectContext |
| **Skills** | SkillHttpTool, SkillScriptTool, SkillToolTool |
| **Dynamic** | `load_active_generated_tools()` |

### Delegate Tool -- Agent Spawning

```rust
pub struct DelegateTool {
    agents: Arc<HashMap<String, DelegateAgentConfig>>,
    fallback_api_key: Option<String>,
    depth: u32,  // recursion depth, max_depth per agent config (default 3)
}
```

Each sub-delegate gets `DelegateTool::with_depth(parent.depth + 1)`. If `is_kowalski_agent` is true, uses GLM-specific keys/model; otherwise the agent's configured provider.

---

## 11. Memory & Persistence

### Memory Backends (factory pattern)

| Backend | Key | Description |
|---|---|---|
| `LucidNativeMemory` | `lucid` | SQLite + vector embeddings (default) |
| `MarkdownMemory` | `markdown` | File-based plain text |
| `NoneMemory` | `none` | No-op for testing |

### Memory Subsystems

| System | Purpose |
|---|---|
| IntelligentMemory | Importance scoring, context budgeting |
| UnifiedAGIMemoryHub | Collective mind state, peer info, consensus |
| ProjectContext | Multi-level context awareness |
| ResponseCache | LLM response caching |
| Embeddings | OpenAI/Ollama embedding abstraction |
| Hygiene | Automatic cleanup/retention |
| Snapshot | Export/import for cold boot |

### Retention Policy

| Category | Retention | Priority |
|---|---|---|
| Episodic | 7 days | Low |
| Semantic | Permanent | High |
| Procedural | Permanent | High |
| Meta | Permanent | Critical |

### Persistent State Files

- `~/.housaky/keys.json` -- API keys and subagent configs
- `.housaky/improvement_cycles.json` -- Self-improvement history
- `.housaky/improvement_experiments.json` -- Experiment ledger
- `.housaky/self_mod_parameters.json` -- Parameter overrides
- `.housaky/decision_journal/decisions.msgpack` -- Decision journal (MessagePack)
- `shared/a2a/state/` -- A2A peer state

---

## 12. Channel System

**Files:** `src/channels/`

### Architecture

```
External Message (Telegram/Discord/etc.)
  -> channels::start_channels()
  -> AGIChannelProcessor
  -> Per-sender history: Arc<Mutex<HashMap<String, Vec<ChatMessage>>>>  (40-turn cap)
  -> Provider call with tool loop (run_tool_call_loop)
  -> Response -> Channel::send_response()
```

### 17 Supported Channels

| Channel | Protocol | Key Detail |
|---|---|---|
| Telegram | Bot API (webhook + polling) | Markdown formatting |
| Discord | Gateway WebSocket | Embed support |
| Slack | Events API | Block Kit |
| WhatsApp | Cloud API / Baileys QR | End-to-end bridge |
| iMessage | AppleScript bridge (macOS) | macOS only |
| Email | IMAP/SMTP (`lettre`/`mail-parser`) | Full MIME |
| IRC | IRC protocol | Raw socket |
| Matrix | Client-Server API | Federated |
| Line | LINE Messaging API | Rich messages |
| Signal | Signal CLI bridge | Encrypted |
| DingTalk | Robot API | Enterprise |
| Lark/Feishu | Bot API + Axum webhook | Enterprise |
| Voice | ElevenLabs TTS/STT WebSocket | Real-time |
| WebChat | Browser-based | Embedded widget |
| CLI | stdin/stdout | Interactive |

### Concurrency

4 concurrent handlers per channel, 8-64 in-flight messages global.

---

## 13. Communication Protocols

### A2A (Agent-to-Agent)

- **Transport**: File-based (shared directory inbox/outbox) or WebSocket (encrypted)
- **Encryption**: X25519 key exchange + ChaCha20-Poly1305 AEAD
- **Messages**: Task, TaskResult, Context, Learning, CodeImprove, SyncRequest/Response, Ping/Pong, Metrics, GoalStatus, Stop

### HIIP (Housaky Inter-Instance Protocol)

- **Transport**: Filesystem (`shared/inbox/`, `shared/outbox/`, `shared/state/`) and Gateway HTTP (port 8080)
- **Purpose**: Status sync, task assignment, knowledge sharing, reflections

### HDIN (Housaky Decentralized Intelligence Network) v2.0.0

- **Architecture**: Decentralized AGI with Seed Mind paradigm
- **Core**: Multi-timescale nested weights (fast/medium/slow/meta)
- **Self-Improvement**: Darwin Godel Machine
- **Communication**: HNL (Housaky Native Language) for 50x compressed agent messaging
- **Safety**: 5-layer defense-in-depth with immutable ethical core
- **Reputation**: Karma system for peer trust

---

## 14. Gateway & API Layer

**File:** `src/gateway/mod.rs`

- **Framework**: Axum HTTP/1.1 + WebSocket
- **Port**: 8080 (configurable)
- **Auth**: Pairing guard (6-digit codes + bearer tokens), constant-time comparison
- **Rate Limiting**: Sliding window
- **Body Limit**: 64KB
- **Timeout**: 30s per request
- **CORS**: Configurable, TLS support
- **Endpoints**: REST for config, MCP, chat, agent, skills, channels, keys, A2A, hardware, doctor

---

## 15. Security Architecture

### Defense-in-Depth Layers

| Layer | Component | Mechanism |
|---|---|---|
| Gateway | `gateway/mod.rs` | Pairing auth, rate limiting, body limits, CORS, TLS |
| Workspace | `security/policy.rs` | 14 system dirs blocked, symlink escape detection |
| Sandbox | `security/` | Landlock (kernel), Bubblewrap, Firejail, Docker, WASM |
| Secrets | `security/secrets.rs` | ChaCha20-Poly1305 AEAD at rest |
| Keys | `keys_manager/` | Encrypted storage, rotation, per-provider pools |
| Audit | `security/audit.rs` | Event logging, tamper detection |
| Provider | `providers/mod.rs` | `scrub_secret_patterns()` for error messages |

### Autonomy Levels

`ReadOnly` -> `Supervised` (default) -> `Autonomous` -> `Full`

### Build Security

- `deny.toml` with `cargo-deny`: advisories, license allowlist, source restrictions
- Distroless Docker production image, non-root (UID 65534)

---

## 16. Hardware & Firmware Integration

### Host-Side (Rust)

| Component | Crate | Purpose |
|---|---|---|
| USB Discovery | `nusb` | Cross-platform enumeration, VID/PID |
| Serial Comm | `tokio-serial` | Async UART/CDC-ACM |
| GPIO | `rppal` (feature-gated) | Raspberry Pi GPIO |
| Flashing | `probe-rs` (feature-gated) | ARM/RISC-V flashing |

### Firmware Projects

| Target | Stack | Board |
|---|---|---|
| `housaky-arduino` | C/Arduino IDE | Arduino Uno |
| `housaky-esp32` | Rust (`esp-idf-svc`) | ESP32 |
| `housaky-nucleo` | Rust (`embassy-stm32`) | STM32F401RE |
| `housaky-uno-q-bridge` | Arduino + Python | Quantum bridge |

### Unified Serial Protocol

Newline-delimited JSON at 115200 baud:

```json
// Request
{"id":"1","cmd":"gpio_write","args":{"pin":13,"value":1}}
// Response
{"id":"1","ok":true,"result":"done"}
```

Commands: `ping`, `capabilities`, `gpio_read`, `gpio_write`.

---

## 17. Dashboard & Landing

### Dashboard (`dashboard/`)

- **Stack**: Vue 3 + TypeScript + Vite + Tailwind CSS
- **Desktop**: Tauri v2 (Rust wrapper)
- **Testing**: Vitest (unit), Playwright (E2E)
- **Communication**: `@tauri-apps/plugin-http`, `plugin-shell`, `plugin-fs`

### Landing (`landing/`)

- **Stack**: Vue 3 + TypeScript + Vite + Tailwind CSS
- **Deploy**: GitHub Pages

### A2A Hub (`landing/A2A/`)

- **Stack**: Vue 3 + Pinia + Vue Router + Axios
- **Features**: Real-time WebSocket, E2E encryption, multi-agent terminal
- **Views**: Home, A2A terminal, Instances, Memory, Research

---

## 18. Prompt Engineering Architecture

### Modular Assembly

```
System Prompt = Core Identity (AGENTS.md)
              + Values & Ethics (SOUL.md)
              + Tool Integration (TOOLS.md)
              + Bootstrap Context (BOOTSTRAP.md)
              + Active Skills (workspace manifests)
              + Reasoning Mode (task-type-specific)
              + Current Context (dynamic)
```

### Template System

- `{{INCLUDE:path}}` -- File inclusion
- `{{DYNAMIC:name}}` -- Runtime value injection
- `{{CONDITIONAL:cond}}` -- Conditional inclusion
- `{{LOOP:items}}` -- Iteration

### Provider Adaptations

OpenAI (function calling), Anthropic (XML markers), Gemini (markdown), open-source (explicit instructions).

---

## 19. Skills System

### Architecture

- **Loading**: Workspace `skills/` + open-skills marketplace
- **Manifest**: `SKILL.md` with YAML frontmatter
- **Invocation**: Auto-trigger on context match
- **SkillForge**: Skill generation tooling

### Bundled Skills

| Skill | Purpose |
|---|---|
| `get-shit-done` | Spec-driven dev: planner, executor, debugger agents; wave execution |
| `ui-ux-pro-max` | UI/UX design expertise with data tables |

---

## 20. CAS Integration for Kowalski

### What is CAS?

**CAS (Coding Agent System)** is a **multi-agent coding factory with persistent memory**. It is a Rust-based system (18 crates) that solves two problems:

1. **Ephemeral context**: Provides persistent memory, tasks, rules, and skills across sessions via SQLite + BM25/Tantivy full-text search
2. **Serial single-agent bottleneck**: Factory mode orchestrates **multiple Claude Code instances** working in parallel, each in isolated git worktrees

### CAS Architecture (18 Crates)

| Crate | Purpose |
|---|---|
| `cas-cli` | Main binary: CLI, MCP server, Bridge REST API, Factory TUI |
| `cas-types` | Shared types: Entry, Task, Rule, Skill, Agent, Event, Worktree |
| `cas-store` | SQLite + Markdown storage backends |
| `cas-core` | Business logic: dedup, extraction, hooks, migration, search, sync |
| `cas-search` | BM25 (Tantivy), vector store (LMDB/heed), hybrid search, grep |
| `cas-mcp` | MCP protocol types for 13 meta-tools |
| `cas-code` | Tree-sitter code analysis: Rust, TypeScript, Python, Go, Elixir |
| `cas-factory` | Factory orchestration: session lifecycle, recording, notifications |
| `cas-factory-protocol` | WebSocket protocol (MessagePack) for Factory client-server |
| `cas-mux` | Terminal multiplexer (ratatui + ghostty_vt) |
| `cas-pty` | Cross-platform PTY spawn and async I/O |
| `cas-diffs` | Diff parsing, rendering, syntax highlighting |
| `cas-recording` | Session recording: binary format, zstd compression, keyframe seeking |

### CAS Server Interfaces

| Interface | Transport | Purpose |
|---|---|---|
| MCP Server (`cas serve`) | stdio (MCP) | Primary agent interface -- 13 meta-tools |
| Bridge Server (`cas bridge`) | HTTP REST (tiny_http) | External tool access, Factory control |
| Factory WebSocket | WebSocket (MessagePack) | Live terminal mux, recording playback |

### CAS Factory Mode -- Multi-Agent Orchestration

```
+----------------------------------------------------------+
|  CAS Factory TUI (ratatui)                               |
|                                                          |
|  +----------+  +----------+  +----------+  +----------+ |
|  |Supervisor|  | Worker 1 |  | Worker 2 |  | Worker N | |
|  |  (PTY)   |  |  (PTY)   |  |  (PTY)   |  |  (PTY)   | |
|  +----+-----+  +----+-----+  +----+-----+  +----+-----+ |
|       |              |              |              |      |
|       | Each runs Claude Code in its own PTY             |
|       | Each has its own git worktree + branch           |
|       | All share the same .cas/ SQLite database         |
|       |                                                  |
|       +--- Shared CAS Database (.cas/cas.db) -----------+|
|            - PromptQueueStore (supervisor <-> worker)    |
|            - AgentStore (registration, heartbeats)       |
|            - TaskStore (shared task board)                |
|            - EventStore (activity tracking)               |
+----------------------------------------------------------+
```

### Why Integrate CAS with Kowalski?

**Current Kowalski Limitations:**

1. **No persistent memory** -- Each agent call is stateless; learnings are lost between tasks
2. **No task coordination** -- No shared task board; agents can't see what others are working on
3. **No verification** -- No quality gate before agent outputs are accepted
4. **No code awareness** -- Agents don't understand the codebase structure
5. **No session management** -- No lifecycle tracking, no heartbeats
6. **Single-shot execution** -- Agents execute one API call per task; no iterative refinement

**CAS Fills Every Gap:**

| Kowalski Gap | CAS Solution |
|---|---|
| No persistent memory | `cas-store` SQLite entries with tiered retention |
| No task coordination | `TaskStore` + `PromptQueueStore` + `task_leases` |
| No verification | `VerificationStore` + verification jail |
| No code awareness | `cas-code` tree-sitter indexing + `cas-search` BM25 |
| No session management | `AgentStore` + heartbeats + auto-revival |
| Single-shot execution | Factory mode with supervisor/worker pattern |

### Integration Architecture

```
                    +--------------------------------------+
                    |        Housaky Binary (main)         |
                    |                                      |
                    |  +--------------------------------+  |
                    |  |    UnifiedAgentHub              |  |
                    |  |                                |  |
                    |  |  +----------+  +------------+  |  |
                    |  |  |Kowalski  |  |SubAgent    |  |  |
                    |  |  |Bridge    |  |Orchestrator|  |  |
                    |  |  +----+-----+  +-----+------+  |  |
                    |  |       |              |          |  |
                    |  +-------|--------------|----------+  |
                    |          |              |             |
                    +----------|--------------|-------------+
                               |              |
                    +----------v--------------v-----------+
                    |        CAS Bridge Layer              |
                    |  (new: src/housaky/cas_bridge.rs)    |
                    |                                      |
                    |  - MCP client (stdio to cas serve)   |
                    |  - REST client (to cas bridge)       |
                    |  - Memory: read/write entries        |
                    |  - Tasks: create/assign/close        |
                    |  - Search: BM25 context retrieval    |
                    |  - Code: tree-sitter code search     |
                    |  - Coordination: agent registration  |
                    +----------+--------------------------+
                               |
                    +----------v--------------------------+
                    |        CAS Process                   |
                    |  (vendor/cas binary, spawned child)   |
                    |                                      |
                    |  .cas/cas.db  (SQLite, shared)       |
                    |  .cas/index/  (Tantivy BM25)         |
                    |  .cas/config.yaml                    |
                    +-------------------------------------+
```

### Integration Points -- How Kowalski Agents Use CAS

#### 1. Session Lifecycle

```rust
// On agent startup
cas_bridge.tool("coordination", {
    "action": "session_start",
    "agent_id": "kowalski-code",
    "role": "worker"
}) -> context injection (rules, recent memories, code structure)

// Periodic heartbeat
cas_bridge.tool("coordination", {
    "action": "heartbeat",
    "agent_id": "kowalski-code"
})

// On agent shutdown
cas_bridge.tool("coordination", {
    "action": "session_end",
    "agent_id": "kowalski-code"
})
```

#### 2. Persistent Memory per Agent

```rust
// After completing a task, store the learning
cas_bridge.tool("memory", {
    "action": "remember",
    "content": "Discovered that the auth module uses JWT with RS256",
    "tags": ["architecture", "auth", "jwt"],
    "tier": "semantic"  // permanent retention
})

// Before starting a task, recall relevant context
cas_bridge.tool("search", {
    "action": "context",
    "query": "authentication module architecture"
}) -> returns ranked memories + code snippets
```

#### 3. Task Board Coordination

```rust
// Supervisor creates tasks
cas_bridge.tool("task", {
    "action": "create",
    "title": "Refactor auth middleware",
    "description": "...",
    "tags": ["refactor", "auth"]
})

// Worker claims a task (with lease to prevent double-assignment)
cas_bridge.tool("task", {
    "action": "claim",
    "task_id": "task-123"
})

// Worker completes task
cas_bridge.tool("task", {
    "action": "close",
    "task_id": "task-123",
    "resolution": "Refactored to use tower middleware stack"
})
```

#### 4. Code-Aware Context

```rust
// Search code structure
cas_bridge.tool("search", {
    "action": "code_search",
    "query": "fn authenticate",
    "language": "rust"
}) -> returns function signatures, file locations, dependencies

// Get blame for a specific file range
cas_bridge.tool("search", {
    "action": "blame",
    "file": "src/auth/middleware.rs",
    "start_line": 45,
    "end_line": 80
})
```

#### 5. Verification Jail -- Quality Gates

```rust
// After code changes, add verification
cas_bridge.tool("verification", {
    "action": "add",
    "task_id": "task-123",
    "status": "pending",
    "criteria": ["tests pass", "no new warnings", "docs updated"]
})

// Agent is now in "verification jail":
// - Cannot create new tasks
// - Cannot modify memory
// - CAN still read/search
// Until verification is approved by supervisor
```

### Kowalski Config Extension

```toml
[kowalski.cas]
enabled = true
cas_binary = "vendor/cas/target/release/cas"
cas_db_path = ".cas"
auto_session = true        # auto session_start/end per task
persist_learnings = true   # auto-remember task results
code_indexing = true       # enable tree-sitter indexing
verification = "optional"  # "required" | "optional" | "disabled"
```

---

## 21. CAS-Powered Claude Code Router Architecture

### The Full Stack: Custom Modal -> Claude Code Router -> OpenAI Response -> CAS + Housaky + Kowalski

This section describes the complete architecture for routing Kowalski subagent requests through a Claude Code compatible proxy, enabling the use of root-level API keys with per-subagent routing.

### Architecture Overview

```
+------------------------------------------------------------------+
|                        Housaky Binary                            |
|                                                                  |
|  Kowalski Agent (kowalski-code)                                  |
|       |                                                          |
|       +-> SubAgentConfig { provider: "cas-router", ... }         |
|       |                                                          |
|  +----v------------------------------------------+               |
|  |  CAS Router (Claude Code Proxy)               |               |
|  |  Runs as local HTTP server                    |               |
|  |                                                |               |
|  |  1. Receives OpenAI-format request            |               |
|  |  2. Routes based on model/agent identity      |               |
|  |  3. Selects root API key from pool            |               |
|  |  4. Forwards to actual provider               |               |
|  |  5. Returns OpenAI-format response            |               |
|  +----+------------------------------------------+               |
|       |                                                          |
+-------|----------------------------------------------------------+
        |
        v  (routed to actual provider)
  +-----+------+  +----------+  +---------+  +--------+
  | Modal      |  | OpenAI   |  |Anthropic|  |Ollama  |
  | GLM-5-FP8  |  | GPT-4o   |  |Claude   |  |Local   |
  +------------+  +----------+  +---------+  +--------+
```

### How the CAS Router Works

The CAS Router is a **local proxy server** that presents an OpenAI-compatible API to all Kowalski subagents but routes requests using **root-level API keys** from a centralized pool. This means:

1. **Subagents don't hold their own API keys** -- They authenticate to the local CAS Router
2. **Root keys are managed centrally** -- In `~/.housaky/keys.json` or CAS config
3. **Per-agent routing** -- Different agents can be routed to different providers/models
4. **Response normalization** -- All responses are returned in OpenAI chat completion format regardless of upstream provider

### Request Flow

```
1. Kowalski Agent sends OpenAI-format request to localhost:CAS_PORT
   POST /v1/chat/completions
   Authorization: Bearer <agent-session-token>
   {
     "model": "zai-org/GLM-5-FP8",
     "messages": [...],
     "metadata": { "agent_id": "kowalski-code" }
   }

2. CAS Router intercepts:
   a. Identifies agent from token or metadata
   b. Looks up routing rule: kowalski-code -> modal/GLM-5-FP8
   c. Selects root API key from pool (round-robin with health checks)
   d. Enriches request with CAS context:
      - Recent memories relevant to the task
      - Active rules and conventions
      - Code structure context from tree-sitter index

3. CAS Router forwards to Modal:
   POST https://api.us-west-2.modal.direct/v1/chat/completions
   Authorization: Bearer <ROOT-MODAL-KEY>
   { enriched request }

4. Modal responds -> CAS Router:
   a. Normalizes response to OpenAI format (if needed)
   b. Extracts learnings for CAS memory
   c. Updates agent session metrics
   d. Returns to Kowalski agent

5. Kowalski Agent receives standard OpenAI response
```

### Model Routing Table

```toml
# In CAS router config or Housaky config

[cas_router.routes]
# Default route
default = { provider = "modal", model = "zai-org/GLM-5-FP8", key_pool = "modal-keys" }

# Per-agent overrides
"kowalski-code"     = { provider = "modal",     model = "zai-org/GLM-5-FP8",    key_pool = "modal-keys" }
"kowalski-web"      = { provider = "openrouter", model = "google/gemini-2.5-pro", key_pool = "openrouter-keys" }
"kowalski-reasoning"= { provider = "anthropic",  model = "claude-sonnet-4-20250514",   key_pool = "anthropic-keys" }
"kowalski-creative" = { provider = "openai",     model = "gpt-4o",               key_pool = "openai-keys" }

[cas_router.key_pools]
modal-keys      = ["key1", "key2", "key3"]       # round-robin with health checks
openrouter-keys = ["key1"]
anthropic-keys  = ["key1", "key2"]
openai-keys     = ["key1"]
```

### CAS Context Enrichment

Before forwarding each request, the CAS Router can inject relevant context from CAS's persistent stores:

```
Original system prompt:
  "You are Housaky-Code, the code specialist..."

Enriched system prompt:
  "You are Housaky-Code, the code specialist..."
  + "[CAS Context] Recent learnings: ..."
  + "[CAS Context] Active rules: ..."
  + "[CAS Context] Relevant code: ..."
  + "[CAS Context] Related tasks: ..."
```

This enrichment uses the same `session_start` context injection that CAS provides to Claude Code instances, but adapted for Kowalski agents.

### Response Processing

After receiving a response from the upstream provider:

```
1. Parse response (handle provider-specific formats)
2. Extract entities/learnings -> CAS memory (background, async)
3. Track usage/cost -> CAS event store
4. Update agent heartbeat
5. If verification required -> enter verification jail
6. Return normalized OpenAI-format response to agent
```

### Integration with CAS Factory Mode

For large tasks, Kowalski can leverage CAS Factory's parallel execution:

```
Kowalski Federation Agent receives large task
  |
  +-> Decomposes into subtasks via GSD StepDecomposer
  +-> Creates CAS tasks for each subtask
  +-> Spawns CAS Factory session:
  |     - 1 Supervisor (kowalski-federation as coordinator)
  |     - N Workers (kowalski-code, kowalski-web, etc.)
  |     - Each in isolated git worktree
  |
  +-> Workers claim tasks via CAS TaskStore
  +-> Each worker uses CAS Router for LLM calls
  +-> Supervisor monitors via CAS Bridge REST API
  +-> Verification jail enforces quality gates
  +-> Results merged back via git worktree merge
```

### Sequence: Complete Multi-Agent Task with CAS

```
User -> Housaky TUI: "Refactor the authentication system to use OAuth 2.0"
  |
  +-> Agent::turn() -> identifies as complex multi-step task
  +-> UnifiedAgentHub::submit_task(task, preferred: Kowalski)
  |
  +-> KowalskiBridge (with CAS):
  |     1. cas.tool("search", context_for_subagent: "auth refactor") -> context
  |     2. cas.tool("task", create: "Plan OAuth 2.0 migration")
  |     3. cas.tool("task", create: "Implement OAuth provider")
  |     4. cas.tool("task", create: "Update middleware")
  |     5. cas.tool("task", create: "Write tests")
  |     6. cas.tool("task", create: "Update documentation")
  |
  +-> kowalski-reasoning claims task #2 (planning):
  |     - CAS Router -> anthropic/claude-sonnet (best for reasoning)
  |     - Result stored as CAS memory entry
  |     - cas.tool("memory", remember: plan details)
  |
  +-> kowalski-code claims task #3 (implementation):
  |     - CAS Router -> modal/GLM-5-FP8 (best for code)
  |     - Receives CAS context: plan from task #2, code structure
  |     - cas.tool("verification", add: "tests pass, no warnings")
  |
  +-> kowalski-code claims task #4 (middleware):
  |     - Parallel with task #3 (different files, CAS worktree isolation)
  |
  +-> kowalski-code claims task #5 (tests):
  |     - Depends on #3 and #4 (CAS dependency graph)
  |     - CAS blocks until dependencies resolved
  |
  +-> kowalski-academic claims task #6 (docs):
  |     - CAS Router -> openrouter/gemini (good for documentation)
  |     - Receives CAS context: all previous task results
  |
  +-> All tasks complete -> verification review
  +-> CAS merges worktrees -> single commit
  +-> Result returned to Housaky -> displayed in TUI
```

---

## 22. CAS + Lucid Memory Integration

This section describes how Housaky's native **Lucid memory system** converges with **CAS storage** to create a unified, best-of-both-worlds architecture for persistent knowledge, agent coordination, and collective intelligence.

### Why Integrate CAS + Lucid?

| System | Strengths | Weaknesses |
|---|---|---|
| **Lucid (Housaky Native)** | ACT-R spreading activation, emotional weighting, intelligent importance classification, hybrid vector+keyword search, project context awareness, A2A hub integration, cold boot snapshot recovery | CLI-dependent (`lucid` binary), limited multi-agent coordination, no verification gates, no git worktree isolation |
| **CAS Storage** | SQLite-native (no external binary), BM25/Tantivy indexing, entity/relation graph, task dependencies, verification jail, layered stores (project+global), helpfulness scoring, memory decay model, tier-based retention | No vector embeddings locally (cloud-only), no ACT-R cognitive model, no explicit importance tiers, no A2A federation |

**Integration Goal**: Use Lucid for AGI-level cognitive memory (episodic/semantic/procedural/meta) while leveraging CAS for agent coordination, task management, code-aware context, and quality gates.

---

### Architecture Comparison

#### Lucid Memory Stack

```
UnifiedAGIMemoryHub (facade)
  |
  +-> IntelligentMemory (decorator: importance, deduplication, budgeting)
        |
        +-> LucidNativeMemory (backend)
              |
              +-> lucid CLI subprocess (~/.lucid/bin/lucid)
                    |
                    +-> brain.db (SQLite, project-local)
                    |     - memories table (with BLOB embeddings)
                    |     - memories_fts (FTS5 full-text index)
                    |     - embedding_cache (LRU, SHA-256 keyed)
                    |
                    +-> ACT-R spreading activation
                    +-> Emotional weighting
                    +-> Token budget allocator
```

#### CAS Memory Stack

```
LayeredEntryStore (facade: global + project)
  |
  +-> SqliteStore (backend)
        |
        +-> cas.db (SQLite, .cas/ directory)
              - entries (memories with tiers, decay, confidence)
              - rules (coding conventions, auto-approve lists)
              - tasks (with dependencies, verification status)
              - entities (knowledge graph nodes)
              - relationships (typed edges)
              - entity_mentions (entity-entry links)
        |
        +-> index/tantivy/ (BM25 inverted index)
              - content, title, tags fields
              - doc_type filtering
        |
        +-> lmdb/ (vector store, cloud-sync only)
              - document ID -> embedding bytes (little-endian f32)
```

---

### Unified Memory Bridge Architecture

```
+----------------------------------------------------------+
|           UnifiedMemoryBridge (new module)                |
|   src/memory/cas_lucid_bridge.rs                          |
|                                                           |
|   - Routes writes to both systems (dual-commit)          |
|   - Routes reads based on query type                     |
|   - Maintains cross-references (Lucid ID <-> CAS ID)     |
|   - Syncs importance scores <-> helpfulness votes        |
|   - Maps Lucid categories to CAS entry types             |
+----------------------------------------------------------+
         |                              |
         v                              v
+------------------+          +------------------+
|  Lucid Backend   |          |   CAS Backend    |
|  (AGI cognition) |          | (Agent workflow) |
+------------------+          +------------------+
```

---

### Dual-Commit Write Strategy

When storing a new memory from Kowalski agents or Housaky core:

```rust
// High-level API
bridge.store(
    content: "OAuth 2.0 migration requires updating middleware to use tower-oauth crate",
    category: MemoryCategory::Core,
    tags: ["oauth", "auth", "middleware", "security"],
    agent_id: Some("kowalski-code"),
    task_id: Some("task-123")
)

// Dual write:
// 1. Lucid path (cognitive memory)
lucid.store(
    key: "oauth-middleware-architecture",
    content: "...",
    category: MemoryCategory::Core,  // -> Lucid type: "decision"
    embedding: compute_embedding("..."),  // OpenAI text-embedding-3-small (1536 dims)
)

// 2. CAS path (agent workflow memory)
cas.add(Entry {
    id: cas.generate_id(),  // "p-2026-03-12-042"
    entry_type: EntryType::Learning,
    content: "...",
    tags: vec!["oauth", "auth", "middleware"],
    memory_tier: MemoryTier::Working,
    session_id: Some("kowalski-code-session-xyz"),
    source_tool: Some("memory_store"),
    belief_type: BeliefType::Fact,
    confidence: 0.9,
    domain: Some("architecture"),
    branch: Some("feature/oauth-migration"),
    pending_embedding: true,  // Will be indexed by CAS daemon
})

// 3. Cross-reference tracking
cross_ref_map.insert(
    lucid_id: "uuid-v4-from-lucid",
    cas_id: "p-2026-03-12-042"
)
```

---

### Query Routing Logic

Different query types route to different backends:

| Query Type | Primary Backend | Fallback | Rationale |
|---|---|---|---|
| **Cognitive recall** ("What did I learn about OAuth?") | Lucid (vector+hybrid) | CAS BM25 | Lucid has embeddings, ACT-R spreading |
| **Task context** ("Show me related tasks") | CAS TaskStore | N/A | Only CAS has task dependencies |
| **Code search** ("Find auth middleware files") | CAS code_search (tree-sitter) | Lucid keyword | CAS indexes code structure |
| **Entity lookup** ("What entities relate to OAuth?") | CAS EntityStore | Lucid keywords | CAS has typed relationships |
| **Agent session history** ("What did kowalski-code do today?") | CAS (session_id filter) | Lucid session filter | CAS tracks agent heartbeats |
| **Important decisions** ("Critical architectural choices") | Both (merged) | N/A | Both store decisions, merge by cross-ref |
| **Skills/procedures** ("How to deploy to production") | Lucid (category: procedural) | CAS skills | Lucid has explicit procedural category |
| **Verification status** ("Is task-123 verified?") | CAS VerificationStore | N/A | CAS-only feature |

---

### Category Mapping

| Housaky MemoryCategory | CAS EntryType | CAS MemoryTier | Notes |
|---|---|---|---|
| `Core` | `EntryType::Learning` (belief: Fact) | `Working` or `InContext` | Permanent facts, decisions |
| `Daily` | `EntryType::Context` | `Working` | Session logs, ephemeral |
| `Conversation` | `EntryType::Observation` (type: Conversation) | `Cold` | Chat history, pruned after 7 days |
| `Custom("skill")` | `EntryType::Learning` (type: Skill) | `InContext` | Procedural knowledge, always injected |
| `Custom("pattern")` | `EntryType::Learning` (type: Pattern) | `Working` | Discovered patterns |
| `Custom("insight")` | `EntryType::Learning` (insight flag) | `Working` | AI-extracted insights |
| `Custom("visual")` | Not supported in CAS | N/A | Lucid-only (image embeddings) |
| `Custom("procedure")` | `EntryType::Learning` (type: Procedure) | `InContext` | Step-by-step guides |

---

### Importance Score Synchronization

Lucid uses **importance classification** (Critical/High/Medium/Low/Trivial) while CAS uses **helpfulness voting** (helpful_count - harmful_count) and **tier promotion**. The bridge keeps them in sync:

```rust
// When Lucid marks memory as Critical
if lucid_importance == Critical {
    // Force CAS tier to InContext (always injected)
    cas.update(entry.id, { memory_tier: MemoryTier::InContext })
}

// When CAS receives positive feedback
if cas.feedback_score(entry.id) > 2 {
    // Promote Lucid importance
    lucid.promote_importance(lucid_id, delta: 0.1)
}

// When CAS stability decays below threshold
if cas.stability(entry.id) < 0.3 {
    // Demote Lucid importance (but never below Medium if Core category)
    if lucid_category != Core {
        lucid.demote_importance(lucid_id, delta: 0.1)
    }
}
```

---

### Embedding Pipeline Convergence

**Lucid Path** (local, immediate):
```
store(content) 
  -> OpenAI API (or custom endpoint) 
  -> 1536-dim embedding 
  -> brain.db embedding_cache (SHA-256 keyed, LRU eviction)
  -> memories table (BLOB column)
  -> Hybrid search: vector cosine + FTS5 BM25
```

**CAS Path** (daemon, batched):
```
add(entry) 
  -> Set pending_embedding = true
  -> CAS daemon (background) picks up pending entries
  -> Cloud API (premium) or local skip
  -> LMDB vector store (document_id -> bytes)
  -> Mark indexed_at = now
  -> Hybrid search: BM25 only (local), semantic via cloud
```

**Convergence Strategy**:
- Lucid handles all **local embedding needs** (real-time, no cloud dependency required if using local model)
- CAS daemon handles **batch indexing** overnight for all pending entries
- Bridge queries Lucid for vector similarity, CAS for BM25, merges results

---

### Entity/Relationship Graph Integration

CAS has a sophisticated **knowledge graph** (entities + relationships + mentions). Lucid has simpler keyword tagging. The bridge enriches Lucid queries with CAS entity context:

```rust
// User query: "How does OAuth work with our auth system?"

// 1. Extract entities from query (CAS EntityExtractor)
entities = ["OAuth", "authentication system"]

// 2. Find related entities in CAS graph
related = cas.entity_store.get_connected_entities("OAuth")
// Returns: [
//   (OAuth, Implements, OAuth2Middleware),
//   (OAuth, DependsOn, JWT),
//   (OAuth, UsedBy, AuthModule)
// ]

// 3. Expand Lucid query with entity aliases
expanded_query = "OAuth OR OAuth2 OR oauth2 OR authentication OR auth OR JWT"

// 4. Query Lucid with expanded query
lucid_results = lucid.recall(expanded_query, limit: 10)

// 5. Query CAS for entity-linked entries
cas_results = cas.entity_store.get_entries_by_entity("OAuth", limit: 10)

// 6. Merge results (deduplicate by cross-ref map)
merged = dedup_merge(lucid_results, cas_results)
```

---

### Task-Aware Memory Injection

When a Kowalski agent starts work on a CAS task, the bridge injects **task-aware context**:

```rust
// Agent claims task
cas.tool("task", action: "claim", task_id: "task-123")

// Bridge automatically retrieves:
context = bridge.get_task_context("task-123")

// Includes:
{
    // 1. Task metadata
    task: {
        title: "Implement OAuth provider",
        description: "...",
        acceptance_criteria: ["RFC 6749 compliant", "PKCE support"],
        labels: ["oauth", "security", "auth"],
        branch: "feature/oauth-provider"
    },
    
    // 2. Related memories (from Lucid + CAS)
    memories: [
        // From Lucid (vector similarity to task description)
        { source: "Lucid", content: "OAuth 2.0 flow diagram...", score: 0.87 },
        // From CAS (entity-linked to OAuth)
        { source: "CAS", content: "JWT middleware architecture...", score: 0.82 }
    ],
    
    // 3. Related tasks (from CAS dependency graph)
    related_tasks: [
        { id: "task-120", title: "Plan OAuth migration", status: "closed" },
        { id: "task-124", title: "Write OAuth tests", status: "open", relation: "blocked_by" }
    ],
    
    // 4. Active rules (from CAS, auto-injected)
    rules: [
        { content: "All auth code must have 90%+ test coverage", priority: 1 },
        { content: "Use tower middleware pattern", priority: 2 }
    ],
    
    // 5. Code context (from CAS tree-sitter index)
    code_snippets: [
        { file: "src/auth/mod.rs", symbols: ["fn authenticate", "struct AuthConfig"] },
        { file: "src/middleware.rs", symbols: ["trait Middleware"] }
    ]
}

// Inject into agent's system prompt
agent.system_prompt = format!(
    "{}\n\n[Task Context]\n{}",
    agent.role.system_prompt(),
    render_context(context)
)
```

---

### Verification Jail + Memory Retention

CAS's **verification jail** blocks mutating operations until quality gates pass. This affects memory retention:

```rust
// Agent completes task, submits for verification
cas.tool("verification", action: "add", task_id: "task-123", criteria: [...])

// Agent enters "verification jail":
// - CAN read/search memory (read-only)
// - CANNOT create new memories
// - CANNOT modify existing memories
// - CANNOT close tasks

// If verification PASSES:
cas.tool("verification", action: "approve", task_id: "task-123")
  -> Promotes all task-related memories to higher tier
  -> Marks memories as "verified" (custom tag)
  -> Increases confidence scores: reinforce_confidence(0.1)

// If verification FAILS:
cas.tool("verification", action: "reject", task_id: "task-123", reason: "Tests failing")
  -> Demotes task-related memories to Cold tier
  -> Adds "unverified" tag
  -> Decreases confidence: weaken_confidence(0.2)
  -> Agent must fix and resubmit
```

---

### Collective Mind Sync (A2A + CAS Federation)

Housaky's **A2A protocol** enables inter-instance memory sharing. CAS adds **team-scoped memories**. Combined:

```rust
// Instance A learns something valuable
instance_a.bridge.store(
    content: "Discovered optimal OAuth state parameter length is 128 bits",
    category: MemoryCategory::Core,
    tags: ["oauth", "security", "best-practice"],
    share_with_collective: true
)

// A2A Hub broadcasts to peers
a2a_hub.broadcast(Learning {
    content: "...",
    tags: [...],
    source_instance: "instance-a",
    confidence: 0.95
})

// Instance B receives learning
instance_b.a2a_hub.receive(learning)
  -> Validates learning (confidence > 0.8, not harmful)
  -> Stores in CAS with team_id = "collective"
  -> Marks as shared_insight = true
  -> Broadcasts to local Kowalski agents

// All instances now have this knowledge
```

---

### Snapshot + Export Convergence

Lucid has **MEMORY_SNAPSHOT.md** export for cold boot recovery. CAS has **rules/tasks export** to `.cas/config.yaml`. Combined export:

```rust
// Unified export command
housaky memory export --format combined --output HOUSAKY_SOUL.md

// Generates:
# Housaky Soul - Combined Memory Export

## Core Memories (from Lucid)
### decision/oauth-architecture
Content: ...

### procedure/deployment-guide  
Content: ...

## Active Tasks (from CAS)
### task-123: Implement OAuth provider
Status: open, Priority: high, Branch: feature/oauth-provider

## Coding Rules (from CAS)
### rule-42: Test coverage for auth
Priority: 1, Auto-approve: false

## Knowledge Graph Entities (from CAS)
- OAuth (Concept) -> [Implements: OAuth2Middleware, DependsOn: JWT]
- AuthModule (Project) -> [Uses: OAuth, Uses: JWT]

## Team Insights (from A2A + CAS team memories)
- Shared from instance-a: "OAuth state parameter 128 bits"
- Shared from instance-b: "PKCE mandatory for public clients"
```

---

### Configuration

Enable both backends in `config.toml`:

```toml
[memory]
backend = "unified"  # New: bridges Lucid + CAS

[memory.lucid]
enabled = true
binary_path = "~/.lucid/bin/lucid"
token_budget = 8000
enable_spreading = true
enable_emotional_weighting = true

[memory.cas]
enabled = true
cas_binary = "vendor/cas/target/release/cas"
cas_db_path = ".cas"
enable_bm25 = true
enable_entity_extraction = true
enable_verification = true

[memory.convergence]
dual_commit = true  # Write to both systems
cross_reference = true  # Maintain ID mappings
sync_importance = true  # Keep importance/helpfulness in sync
embeddings_from = "lucid"  # Use Lucid for local embeddings
tasks_from = "cas"  # Use CAS for task management
entities_from = "cas"  # Use CAS for knowledge graph
```

---

### Performance Characteristics

| Operation | Lucid Alone | CAS Alone | Unified Bridge |
|---|---|---|---|
| Store (with embedding) | ~50ms (async) | ~5ms (sync), embedding later | ~55ms (parallel) |
| Recall (hybrid) | ~3ms | ~2ms (BM25 only) | ~8ms (merge both) |
| Entity expansion | N/A | ~10ms | ~12ms (graph + Lucid) |
| Task context | N/A | ~15ms | ~20ms (task + memories) |
| Verification check | N/A | ~2ms | ~2ms (CAS only) |
| A2A sync | ~100ms | ~50ms | ~150ms (both) |

Overhead is acceptable (<20ms) for the significant capability gains.

---

## 23. Data Flow Diagrams

### Interactive Chat Flow

```
User Input -> main.rs -> TUI (ratatui)
  -> Agent::turn()
  -> Provider (LLM call)
  -> Tool Resolution -> Tool Execution
  -> Memory Store
  -> Response -> TUI Render
```

### Channel Message Flow

```
External (Telegram/Discord/...)
  -> channels::start_channels()
  -> AGIChannelProcessor
  -> Per-sender history (40-turn Mutex<HashMap>)
  -> Provider + tool loop
  -> Response -> Channel::send_response()
```

### Daemon Lifecycle

```
daemon::run()
  -> spawn gateway (Axum on :8080)
  -> spawn channels (supervised, backoff)
  -> spawn heartbeat (2 min cycles)
  -> state writer (periodic flush)
```

### Self-Improvement Cycle

```
Heartbeat (every 2 min)
  -> HousakyCore::run_self_improvement()
  -> MetaCognition::analyze()
  -> GoalEngine::inject_new_goals()
  -> SelfImprovementLoop::run_full_cycle()
  -> RustCodeModifier (syn/quote AST)
  -> GitSandbox (isolated test)
  -> FitnessEvaluator (empirical)
  -> Integrate or Rollback
```

### A2A Communication

```
Housaky Native (Rust) <-> shared/a2a/ <-> Housaky OpenClaw (Claude)
  -> JSON messages: inbox/outbox directories
  -> OR: WebSocket (wss://hub.housaky.ai:8765)
  -> X25519 + ChaCha20-Poly1305 encryption
```

### CAS-Enhanced Kowalski Flow

```
Task -> UnifiedAgentHub
  -> KowalskiBridge (CAS-enabled)
  -> CAS: search context, create tasks, register agents
  -> Agents claim tasks via CAS TaskStore
  -> CAS Router: agent -> root key -> upstream provider
  -> CAS: store learnings, track events, verify quality
  -> Results merged -> returned to Housaky
```

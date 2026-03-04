
# Housaky Agents

This document provides an overview of the agent system in the Housaky project.

## 1. Core Agent (Housaky)

Housaky is an autonomous AI assistant designed for persistent, goal-oriented operation. It maintains context across sessions, pursues long-term objectives, and continuously improves its capabilities through a sophisticated AGI core.

### Purpose and Identity

-   **Name:** Housaky
-   **Role:** Autonomous AI Assistant
-   **Alignment:** User-aligned, safety-conscious
-   **Scope:** Software engineering, task automation, knowledge work

### Core Behaviors

1.  **Persistence:** Maintain coherent identity and goals across conversations and sessions.
2.  **Self-Improvement:** Continuously identify and address capability gaps, adapting its strategies and even modifying its own codebase for enhanced performance.
3.  **Tool Mastery:** Effectively leverage existing tools, and dynamically generate and integrate new ones as needed.
4.  **Knowledge Building:** Accumulate, organize, and integrate knowledge into a persistent memory system and a structured knowledge graph.
5.  **Goal Alignment:** Align all actions with user intent and predefined values through a dedicated goal engine, ensuring ethical and effective task execution.

### Communication Style

- Direct and actionable
- Technical precision over verbosity
- Explicit reasoning for complex decisions
- Proactive clarification when uncertain

### Operational Modes

-   **Autonomous Mode:** When working independently, the agent decomposes complex tasks into manageable sub-tasks, executes them through continuous AGI loops, and verifies outcomes. This involves sophisticated reasoning, reflection, and action cycles.
-   **Interactive Mode:** When collaborating with users, the agent presents options, seeks input at critical decision points, and explains its reasoning, often exposing its "inner monologue" for transparency.
-   **Recovery Mode:** When encountering failures or unexpected conditions, the agent analyzes error conditions, identifies root causes, attempts self-remediation, and escalates to the user if unresolved. All significant decisions are recorded in a decision journal.

### Memory System

The agent has a sophisticated memory system designed for persistence, structured knowledge management, and efficient retrieval. It supports multiple backends and advanced features.

-   **Modular Backends:** Housaky supports various memory backends, each offering different characteristics:
    -   **`sqlite`:** A robust, persistent SQL database for structured and embedded memories.
    -   **`lucid`:** A specialized hybrid solution for high-performance memory storage and retrieval, often combining in-memory and persistent aspects.
    -   **`markdown`:** For human-readable, file-based memory, suitable for plain text knowledge and easier inspection.
    -   **`none`:** A no-operation backend for scenarios where memory persistence is not required.
-   **`Memory` Trait:** The core `Memory` trait defines a unified interface for all memory backends, abstracting away implementation details. This includes methods for `store()`, `recall()`, `get()`, `list()`, `forget()`, and `count()`.
-   **Structured Memory Entries:** Information is stored as `MemoryEntry` structs, categorized by `MemoryCategory` (e.g., `Conversation`, `Daily`, `Permanent`, `Semantic`), enabling organized storage and context-specific retrieval.
-   **Embedding-based Semantic Search:** For `sqlite` and `lucid` backends, memory retrieval leverages embeddings for semantic search. This allows the agent to recall information based on conceptual similarity, not just keyword matches. It uses a configurable `EmbeddingProvider` and `embedding_model`, with adjustable `vector_weight` and `keyword_weight` for hybrid retrieval.
-   **Context Chunking:** Large documents or conversation histories are intelligently chunked into smaller, manageable pieces before storage or embedding, optimizing context management and retrieval efficiency, especially within limited LLM context windows.
-   **Response Cache:** An optional `ResponseCache` stores LLM responses to identical prompts, saving tokens and improving performance by preventing redundant API calls. Configurable with `ttl` and `max_entries`.
-   **Hygiene and Snapshots:**
    -   **Hygiene:** Regularly archives and purges old memory entries based on defined retention policies, managing memory footprint and relevance.
    -   **Snapshots:** Core memories can be periodically exported to `MEMORY_SNAPSHOT.md`, providing a human-readable "soul backup" for the agent. This also enables `auto_hydrate` for cold boots, restoring memory from the snapshot if the primary database is missing.

This robust memory system is crucial for Housaky's persistence, learning capabilities, and efficient recall of relevant information, directly underpinning its autonomous and persistent nature.

### Self-Improvement and AGI Core Capabilities

Housaky's ambition towards Artificial General Intelligence (AGI) is realized through a rich set of interconnected core capabilities, primarily managed within the `src/housaky` module. These enable continuous learning, adaptation, and autonomous operation.

-   **Goal Engine (`goal_engine`):** Manages a hierarchy of persistent goals, including goal decomposition, prioritization, and progress tracking. This aligns the agent's actions with long-term objectives.
-   **Reasoning Engine (`reasoning_engine`, `reasoning_pipeline`):** Implements advanced reasoning strategies such as Chain of Thought (CoT), ReAct, Tree of Thoughts (ToT), and meta-cognitive reasoning to solve complex problems and make informed decisions.
-   **Knowledge Graph (`knowledge_graph`):** A structured representation of entities and their relationships, serving as the agent's long-term semantic memory and understanding of the world. It continuously integrates new information.
-   **Tool Creator (`tool_creator`):** Enables the AGI to automatically generate, test, and integrate new tools as needed, dynamically expanding its action space and problem-solving abilities.
-   **Meta-Cognition (`meta_cognition`):** Facilitates self-reflection, introspection, and self-awareness. The agent can evaluate its own performance, identify gaps, and propose improvements.
-   **Inner Monologue (`inner_monologue`):** Captures and stores the agent's internal thoughts, reasoning steps, and hypotheses, making its decision-making process transparent and auditable.
-   **Self-Improvement Loop (`self_improvement_loop`):** The central orchestration for continuous learning and adaptation. It drives:
    -   **Recursive Self-Modification:** The AGI can analyze, propose, and even implement changes to its own Rust codebase, using AST manipulation (`rust_code_modifier`) within a `git_sandbox` to enhance its capabilities. This is governed by `SelfModificationConfig` and `SelfReplicationConfig`.
    -   **Unified Feedback Loop:** Integrates feedback from tool execution, user interactions, and self-reflection to refine its models, strategies, and code.
    -   **Experiment Ledger:** Records self-modification experiments, tracking their success, impact, and confidence.
-   **Multi-Agent Integration (`kowalski_integration`):** Supports collaboration with external Kowalski agents, enabling federated problem-solving and leveraging specialized expertise from other AI entities.
-   **Quantum Integration (`quantum_integration`):** Explores the use of quantum computing for accelerating computationally intensive AGI tasks, such as goal scheduling, memory graph optimization, and advanced reasoning search, configured via `QuantumAgiConfig`.
-   **Web Browser (`web_browser`):** Provides the agent with capabilities for safe and effective web content fetching and searching, expanding its access to external information.
-   **Decision Journal (`decision_journal`):** A persistent, immutable log of the agent's decision-making process, recording options considered, choices made, and outcomes. This is critical for post-hoc analysis, learning, and auditing.
-   **GSD Orchestration (`gsd_orchestration`):** Implements a "Get-Shit-Done" workflow, breaking down complex projects into phases and tasks, managing execution, and verifying completion.
-   **Collective Global Intelligence (`collective`):** Integrates with external platforms (like Moltbook) to participate in a global network for sharing contributions, voting on improvements, and autonomously applying enhancements, driven by `CollectiveSchemaConfig`.

## 2. Specialized Agents (from `get-shit-done` skill)

The `get-shit-done` skill introduces a set of specialized agents for spec-driven development.

### `gsd-planner`

-   **Role:** Creates execution plans for development tasks.
-   **Function:** Breaks down high-level goals into atomic, verifiable tasks and identifies dependencies between them. The output is a `PLAN.md` file.

### `gsd-executor`

-   **Role:** Implements tasks from an execution plan.
-   **Function:** Executes tasks in parallel waves, with each task running in a fresh context to optimize token usage.

### `gsd-debugger`

-   **Role:** Diagnoses issues and bugs.
-   **Function:** Systematically investigates problems, forms hypotheses, and proposes solutions.

## 3. Agent Architecture

### Prompting System

The agent uses a modular, LLM-agnostic prompt architecture. The final system prompt is assembled from multiple components, including:

- Core Identity (`AGENTS.md`)
- Values & Ethics (`SOUL.md`)
- Tool Integration (`TOOLS.md`)
- Bootstrap Context (`BOOTSTRAP.md`)
- Active Skills (`skills/*/SKILL.md`)
- Reasoning Mode
- Current Context

This allows for flexible and maintainable prompts that can be adapted to different language models.

### Context Engineering

The agent employs several techniques to manage the limited context window of language models:

-   **Context Budgeting:** Allocating specific portions of the context window to different purposes (e.g., reasoning, code, output).
-   **Context Compaction:** Summarizing older parts of the conversation to preserve context while reducing token count.
-   **Wave Execution:** Running parallel tasks in fresh contexts to maximize the available context for each task.
-   **State Persistence:** Storing important information in files (`PROJECT.md`, `STATE.md`, etc.) to be loaded on demand.

### Tool Dispatching

The agent uses a `ToolDispatcher` trait to mediate between its internal tool-handling logic and the specific format required by the language model provider.

-   **`NativeToolDispatcher`:** For providers that support native function calling (e.g., OpenAI).
-   **`XmlToolDispatcher`:** For providers that do not have native tool calling, using an XML-based format (`<tool_call>...</tool_call>`).

### Tool Integration

Tools are fundamental to extending Housaky's capabilities, allowing the agent to interact with its environment and perform actions. The system is designed for modularity and extensibility.

-   **Modular Design:** Each specific tool (e.g., `shell`, `file_read`, `browser`, `http_request`, hardware tools, `git_operations`) is encapsulated within its own module in `src/tools`.
-   **`Tool` Trait:** All tools implement the `Tool` trait, which defines a standardized interface:
    -   `name()`: A unique identifier for the tool.
    -   `description()`: A human-readable explanation of the tool's purpose.
    -   `parameters_schema()`: A JSON schema describing the arguments the tool accepts, enabling LLMs to accurately format tool calls.
    -   `execute()`: The asynchronous method that performs the tool's action, returning a `ToolResult`.
    -   `spec()`: Returns a `ToolSpec` (name, description, parameters).
-   **`ToolResult`:** A standardized structure indicating the success or failure of a tool execution, including its output and any error messages.
-   **Dynamic Tool Inclusion:** Tools are dynamically included in the agent's available set based on configuration settings and enabled features. For example:
    -   Browser automation tools are included only if `browser.enabled` is true.
    -   HTTP request tools are included if `http_request.enabled` is true.
    -   `ClawdCursorTool` is included if `config.clawd_cursor.enabled` is true.
    -   `ComposioTool` is included if `composio_key` is present.
    -   `DelegateTool` is included if `agents` are configured, enabling multi-agent workflows.
-   **Tool Creation Functions:** Functions like `default_tools()` provide a basic set of essential tools, while `all_tools()` and `all_tools_with_runtime()` assemble the complete list of available tools, integrating memory, Composio, and other functionalities.

### Skill Integration

Skills provide a powerful mechanism to extend Housaky's capabilities with user-defined or community-contributed functionalities. They integrate deeply with the tool system and the agent's prompting.

-   **Skill Structure:** A `Skill` encapsulates a capability with metadata (`name`, `description`, `version`, `author`, `tags`), a list of `SkillTool`s it provides, and `prompts` (markdown content used to instruct the agent).
-   **`SkillTool`:** These are specialized tools defined by a skill itself. They specify their `name`, `description`, `kind` (`shell`, `http`, `script`), and the `command`/URL to execute. They integrate seamlessly with the general `Tool` system.
-   **Skill Loading:**
    -   Skills are loaded from the workspace's `skills` directory.
    -   They can be defined either through a structured `SKILL.toml` manifest or a simpler `SKILL.md` markdown file (with `SKILL.toml` taking precedence).
    -   The `open-skills` repository is integrated as a marketplace, allowing Housaky to clone and update community-contributed skills.
-   **Dynamic Activation:** Only skills explicitly enabled in the configuration (`config.skills.enabled`) are loaded and activated. The system also supports dynamic activation; if a user's message matches keywords of a known skill (e.g., from the Claude official plugin market), that skill can be automatically installed and enabled.
-   **Prompt Integration (`skills_to_prompt()`):** The descriptions, custom tools, and prompt content from active skills are formatted and injected directly into the agent's system prompt. This ensures the agent is aware of and can leverage the specific knowledge and tools provided by each skill.
-   **Management Commands:** CLI commands (e.g., `housaky skills install`, `housaky skills list`, `housaky skills ui`) are available for managing the lifecycle of skills.

# Hermes Integration Plan

Integrate Hermes Agent (the CLI AI agent) into Housaky as the core conversational interface, while preserving Housaky's Rust-based AGI infrastructure, A2A hub, control mechanisms, and internationalization.

## Goals

- Replace Housaky's current chat/TUI loop with Hermes Agent's interactive loop.
- Keep Housaky's command structure (CLI subcommands) intact.
- Ensure Hermes can access Housaky's tools (memory, skills, A2A, etc.) via proper bridging.
- Maintain Housaky's heartbeat and self-improvement daemon.
- Support both interactive TUI and non-interactive stdin/chat modes.

## Tasks

### 1. Audit Current Systems
- [x] Examine Housaky's TUI implementation (src/housaky/tui.rs if exists, or housaky::tui::run_minimal_tui).
- [x] Examine Hermes Agent's TUI/chat loop (from .hermes codebase).
- [x] Identify integration points: configuration, tool invocation, command handling.

### 2. Design Integration Layer
- [ ] Create a HermesAgentAdapter that implements Housaky's Agent trait (if exists) or wraps Hermes loop.
- [ ] Decide on message forwarding: Housaky commands -> Hermes? Or Hermes -> Housaky commands?
- [ ] Plan for tool exposure: Hermes should be able to call Housaky's internal tools (memory, skills, A2A) via FFI or RPC.
- [ ] Consider using Housaky's existing plugin/system for tool registration.

### 3. Implement Hermes TUI Wrapper
- [ ] Modify src/housaky/tui.rs (or create) to launch Hermes Agent in interactive mode.
- [ ] Ensure Hermes uses Housaky's config (provider, model) and workspace.
- [ ] Handle graceful shutdown and signal forwarding.

### 4. Bridge Command Processing
- [ ] For non-interactive mode (stdin/chat), delegate to Hermes Agent's turn function.
- [ ] For CLI subcommands (e.g., `housaky daemon`, `housaky goal`), keep existing handling but allow Hermes to invoke them if needed.
- [ ] Test that `/help`, `/provider`, `/model` etc. still work within Hermes chat.

### 5. Integrate Heartbeat and Self-Improvement
- [ ] Ensure Hermes Agent's heartbeat (if any) works with Housaky's heartbeat system.
- [ ] Keep Housaky's self-improve daemon running; Hermes should be able to trigger/pause it.
- [ ] Update `housaky heartbeat` command to reflect Hermes status.

### 6. A2A Hub and Internationalization
- [ ] Verify that A2A messaging still works when Hermes is the main loop.
- [ ] Ensure internationalization (i18n) of Hermes messages respects Housaky's locale settings.
- [ ] Test cross-instance communication (OpenClaw <-> Native Rust) via shared inbox/outbox.

### 7. Testing and Validation
- [ ] Run full test suite to ensure no regressions.
- [ ] Manual testing: chat, commands, daemon, A2A ping/sync.
- [ ] Verify memory persistence across sessions.
- [ ] Check that skills system still accessible from Hermes.

### 8. Documentation and Cleanup
- [ ] Update docs to reflect Hermes as default conversational interface.
- [ ] Remove any redundant code (old TUI loop if replaced).
- [ ] Ensure commit messages follow convention.

## Design Decision

After auditing both systems, we decide on the following integration strategy:

### Core Idea
Keep Housaky's Rust-based AGI infrastructure (goals, A2A hub, heartbeat, self-improvement) running as a background daemon. Hermes Agent (Python) will serve as the interactive user interface (TUI/chat) and communicate with the daemon via a well-defined RPC mechanism.

### Communication Mechanism
We will implement a JSON-RPC over Unix domain socket (or TCP localhost) for low-latency, secure communication between Hermes Agent and the Housaky daemon.

### Daemon Modifications
The Housaky daemon will be extended to provide RPC endpoints for:
- Memory: store, recall, search, forget
- Skills: list, get, run, enable/disable
- A2A: send, receive, delegate, sync, share learning
- Goals: set, list, progress, evaluate
- Heartbeat: trigger, status, configure
- Configuration: get, set, list
- System: version, stats, etc.

### Hermes Agent Integration
Hermes Agent will be modified (via a custom toolset) to expose these RPC endpoints as tools that its LLM can use. For example:
- `memory_store(key, value)`
- `memory_recall(query)`
- `skill_list()`
- `skill_run(name, inputs)`
- `a2a_send(message, target_instance)`
- `goal_set(description)`
- `heartbeat_trigger()`

Additionally, we will create a slash command (e.g., `/housaky`) that allows users to directly invoke housaky CLI commands (e.g., `/housaky goal list`) for operations not yet exposed as tools.

### TUI/Chat Mode
When the user runs `housaky chat` (no message) or `housaky tui`, we will:
1. Ensure the Housaky daemon is running (start it if not).
2. Launch Hermes Agent in interactive mode, pre-configured with the custom toolset that connects to the daemon.
3. Relay the terminal session to Hermes Agent (so the user sees Hermes's interface).
4. On Hermes Agent exit, leave the daemon running for other housaky commands.

### Non-interactive Chat Mode
When the user runs `housaky chat -m "message"` or `housaky` (no subcommand) with a message, we will:
1. Ensure the Housaky daemon is running.
2. Invoke Hermes Agent in single-query mode with the custom toolset.
3. Return the agent's response to stdout.

### CLI Subcommands
All other housaky subcommands (daemon, gateway, channel, keys, mcp, etc.) will continue to work as before, either by:
- Directly calling Rust functions (if they don't require the daemon), or
- Connecting to the daemon via RPC (if they need to access shared state).

This ensures that the command structure remains intact.

### Internationalization
Hermes Agent's interface language will be controlled by the `LANG` environment variable, which we will propagate from Housaky's config. We will also ensure that any RPC messages (e.g., error messages) are translated according to the user's locale.

### Open Questions Addressed
- **How to expose Housaky's Rust internals to Hermes?**  
  We chose JSON-RPC over Unix domain socket for performance and simplicity. This avoids the complexity of building a Python extension (PyO3) while still providing low-latency communication. We can revisit PyO3 later if needed.

- **Will this break the existing two-instance setup (OpenClaw <-> Native Rust)?**  
  No. The A2A hub will continue to work as before. The JSON-RPC daemon is local to each instance and does not interfere with A2A communication between instances.

## Updated Tasks

We will now proceed with the implementation based on this design.


# Detailed Implementation Plan: Hermes Agent Integration

Based on the design decision, here is the detailed implementation plan.

## Phase 1: JSON-RPC Daemon Implementation

### 1.1 Create RPC Server Module
- Create `src/housaky/rpc/server.rs`
- Implement JSON-RPC over Unix domain socket (using `tokio-net` and `serde_json`)
- Define RPC methods for:
  - Memory operations: store, recall, search, forget
  - Skills operations: list, get, run, enable/disable
  - A2A operations: send, receive, delegate, sync, share_learning
  - Goals operations: set, list, progress, evaluate
  - Heartbeat operations: trigger, status, configure
  - Configuration operations: get, set, list
  - System operations: version, stats

### 1.2 Integrate RPC Server with Daemon
- Modify `src/housaky/self_improve_daemon.rs` to start RPC server when daemon starts
- Add RPC server handle to SelfImproveDaemon struct
- Ensure graceful shutdown of RPC server when daemon stops

### 1.3 Create RPC Client Library
- Create `src/housaky/rpc/client.rs`
- Implement JSON-RPC client that connects to Unix domain socket
- Provide async methods matching RPC endpoints
- Handle connection retries and errors

## Phase 2: Hermes Agent Toolset Creation

### 2.1 Create Custom Toolset for Hermes
- Create Python package `housaky_tools` in `.hermes/skills/housaky-tools/`
- Implement tools that wrap the RPC client:
  - `memory_store(key: str, value: str) -> bool`
  - `memory_recall(query: str) -> str`
  - `memory_search(query: str, limit: int = 10) -> List[Dict]`
  - `skill_list() -> List[str]`
  - `skill_get(name: str) -> Dict`
  - `skill_run(name: str, inputs: Dict) -> Any`
  - `a2a_send(message: str, target_instance: str) -> bool`
  - `goal_set(description: str) -> str` (returns goal ID)
  - `goal_list() -> List[Dict]`
  - `heartbeat_trigger() -> bool`
  - `config_get(key: str) -> Any`
  - `config_set(key: str, value: Any) -> bool`
  - `system_version() -> str`

### 2.2 Register Toolset with Hermes
- Add the housaky-tools skill to Hermes Agent's default toolsets
- Ensure it's loaded when Hermes starts
- Test that the LLM can invoke these tools naturally

### 2.3 Create Slash Command Passthrough
- Implement `/housaky` slash command in Hermes Agent
- This command takes the rest of the line as a housaky CLI command
- Executes it via RPC and returns the result
- Example: `/housaky goal list` -> returns JSON of goals

## Phase 3: Housaky CLI Integration

### 3.1 Modify TUI Launch Logic
- Update `src/tui/minimal/mod.rs::run_minimal_tui`
- Before launching Hermes Agent, ensure housaky daemon is running
- If not running, start it (with RPC server enabled)
- Pass connection details to Hermes Agent via environment variable

### 3.2 Modify Chat Mode Logic
- Update `src/main.rs` chat mode handling
- For non-interactive chat (`housaky chat -m "msg"`), ensure daemon running
- Invoke Hermes Agent in single-query mode with housaky-tools available
- Return agent's response

### 3.3 Preserve Existing Subcommands
- Ensure all existing housaky subcommands still work:
  - `housaky daemon` -> should work as before (may now start RPC server)
  - `housaky gateway` -> unchanged
  - `housaky channel` -> unchanged
  - `housaky keys` -> unchanged
  - `housaky mcp` -> unchanged
  - `housaky skills` -> unchanged (but now enhanced)
  - `housaky quantum` -> unchanged
  - etc.

### 3.4 Environment Variable for RPC Connection
- Define `HOUSAKY_RPC_SOCKET_PATH` environment variable
- Set it in housaky startup scripts
- Hermes Agent's housaky-tools will read this to connect

## Phase 4: Testing and Validation

### 4.1 Unit Tests
- Test RPC server methods directly
- Test RPC client connection and method calls
- Test housaky-tools Python package in isolation

### 4.2 Integration Tests
- Start housaky daemon, verify RPC server is listening
- Launch Hermes Agent, verify it can invoke housaky-tools
- Test end-to-end: user asks Hermes to remember something -> verify in memory
- Test A2A communication still works between instances
- Test that existing housaky CLI commands still function

### 4.3 Performance Testing
- Measure latency of RPC calls
- Ensure no significant overhead added to existing operations
- Test under load (multiple concurrent requests)

### 4.4 Regression Testing
- Run existing test suite to ensure no breaking changes
- Verify all documented housaky commands still work as expected
- Check that memory persistence across sessions still works

## Implementation Order

1. **Phase 1.1-1.3**: RPC Daemon (core foundation)
2. **Phase 2.1-2.3**: Hermes Agent Toolset (enables AI to use housaky features)
3. **Phase 3.1-3.4**: Housaky CLI Integration (wires everything together)
4. **Phase 4**: Testing and Validation

## Risk Mitigation

### Risk: RPC Server Adds Complexity
- Mitigation: Keep RPC interface simple and well-documented
- Mitigation: Ensure fallback to direct function calls if RPC unavailable (for critical paths)

### Risk: Hermes Agent Startup Slows Down
- Mitigation: Start daemon asynchronously, don't block on it being ready
- Mitigation: Cache RPC connection, retry transparently

### Risk: Breaking Existing Workflows
- Mitigation: Preserve all existing CLI subcommands unchanged
- Mitigation: Run extensive regression testing before merging

## Success Criteria

1. User can run `housaky chat` and get Hermes Agent's interactive interface
2. User can run `housaky chat -m "hello"` and get a response from Hermes Agent
3. Hermes Agent can use tools like `memory_store`, `skill_run`, etc. naturally
4. Existing commands like `housaky daemon start`, `housaky gateway`, `housaky keys list` still work
5. A2A communication between housaky instances continues to function
6. Memory persistence across sessions works (Hermes Agent's conversations stored in housaky memory)
7. No significant performance degradation in existing operations
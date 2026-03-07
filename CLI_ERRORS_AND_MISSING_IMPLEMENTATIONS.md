# Housaky CLI - Error & Missing Implementation Report

## Compilation Status: ✅ Compiles with 0 warnings

## Runtime Issues & Missing Implementations

### 1. Kowalski Integration (HIGH PRIORITY)
**Status**: Uses external binary instead of built-in library
**Issue**: The `KowalskiBridge` in `src/housaky/kowalski_integration.rs` relies on finding an external CLI binary at:
- `vendor/kowalski/kowalski-cli/target/release/kowalski-cli`
- Or custom path from config

**Problem**: 
- The CLI binary is not built by default
- Even when built, it's an external process (slow, IPC overhead)
- No in-process API available

**Solution Options**:
1. **Quick Fix**: Build the kowalski-cli automatically in the build process
2. **Proper Fix**: Integrate kowalski-core as a path dependency and use the Agent trait directly

**Kowalski-core issues**:
- Uses Rust edition 2024 (unstable) - needs to be changed to 2021
- Multiple circular dependencies between agent crates

---

### 2. MCP Marketplace (MEDIUM PRIORITY)
**Status**: Registry fetch fails
**Command**: `housaky mcp list`
**Error**: `Failed to parse MCP registry - expected value at line 9 column 1`

**Cause**: Network fetch of registry.json fails or returns invalid JSON
**Solution**: 
- Add offline fallback
- Cache registry locally
- Add better error messages

---

### 3. Dashboard Desktop App (MEDIUM PRIORITY)
**Status**: Tauri desktop app not built
**Command**: `housaky dashboard --desktop`
**Error**: `Dashboard desktop app not found - Build it with: cd dashboard && pnpm tauri build`

**Solution**: Build the Tauri app:
```bash
cd dashboard
pnpm tauri build
```

---

### 4. Config TUI (LOW PRIORITY)
**Status**: TUI not available in non-TTY
**Command**: `housaky config -s agent`
**Error**: `No such device or address (os error 6)`

**Cause**: Config editing uses TUI which requires a terminal
**Solution**: Add non-interactive mode or use default editor

---

### 5. Quantum Computing (EXPECTED - NEEDS CREDENTIALS)
**Status**: Requires AWS credentials
**Command**: `housaky quantum device-info`
**Error**: `No credentials provider`

**Cause**: Amazon Braket requires AWS credentials
**Solution**: Document AWS setup requirements

---

### 6. Chat/Agent (EXPECTED - NEEDS API KEY)
**Status**: API key not configured
**Command**: `housaky chat -m "hello"`
**Error**: `Custom API key not set`

**Cause**: User hasn't configured API key
**Solution**: Run `housaky onboard` or set API key

---

## Command Coverage Analysis

### Fully Working Commands (27)
| Command | Status |
|---------|--------|
| `housaky --help` | ✅ |
| `housaky --version` | ✅ |
| `housaky status` | ✅ |
| `housaky keys list` | ✅ |
| `housaky models refresh` | ✅ (expected: no live discovery) |
| `housaky skill list` | ✅ |
| `housaky mcp installed` | ✅ |
| `housaky daemon status` | ✅ |
| `housaky goal list` | ✅ |
| `housaky channel list` | ✅ |
| `housaky cron list` | ✅ |
| `housaky doctor` | ✅ |
| `housaky gsd status` | ✅ |
| `housaky thoughts -c 3` | ✅ |
| `housaky self-mod status` | ✅ |
| `housaky self-mod experiments` | ✅ |
| `housaky collective status` | ✅ |
| `housaky collective pending` | ✅ |
| `housaky hw discover` | ✅ |
| `housaky hw list` | ✅ |
| `housaky quantum devices` | ✅ |
| `housaky a2a ping` | ✅ |
| `housaky heartbeat` | ✅ |
| `housaky service status` | ✅ |
| `housaky dashboard --start` | ✅ |
| `housaky gsd awareness` | ✅ |
| `housaky migrate --help` | ✅ |

### Commands with Issues (6)
| Command | Status | Issue |
|---------|--------|-------|
| `housaky mcp list` | ⚠️ | Registry parse error |
| `housaky dashboard --desktop` | ⚠️ | Tauri not built |
| `housaky config -s agent` | ⚠️ | TUI not available |
| `housaky quantum device-info` | ⚠️ | No AWS credentials |
| `housaky kowalski` | ⚠️ | CLI not built |
| `housaky chat -m "hello"` | ⚠️ | No API key (expected) |

---

## Missing Features to Implement

### 1. Secure Native Channel to Housaky
**Description**: Native encrypted channel for communication with Housaky
**Status**: Not implemented
**Required for**:
- Secure remote access
- End-to-end encryption
- Authentication

**Implementation ideas**:
- WebSocket-based protocol
- ChaCha20-Poly1305 encryption (already in deps)
- Token-based authentication

### 2. Kowalski Built-in Integration
**Required changes**:
1. Fix edition 2024 -> 2021 in kowalski-core/Cargo.toml
2. Add path dependencies to Housaky's Cargo.toml
3. Rewrite KowalskiBridge to use Agent trait directly
4. Remove CLI subprocess calls

### 3. MCP Offline Support
**Required changes**:
1. Cache registry.json locally
2. Add fallback for network failures
3. Add retry logic with exponential backoff

### 4. Dashboard Tauri Build
**Required changes**:
1. Build Tauri app: `cd dashboard && pnpm tauri build`
2. Or disable --desktop flag if not supported

---

## Architecture Review

### Current Issues

1. **Modular but fragmented**: Multiple agent systems (Kowalski, SubAgent, Federation) but no unified API
2. **External dependencies**: Too many external processes (kowalski-cli)
3. **Missing tests**: No comprehensive test suite
4. **Configuration complexity**: Many config options not documented

### Recommendations

1. **Unified Agent API**: Create trait `AgentBackend` with implementations for:
   - Kowalski (when integrated)
   - SubAgentOrchestrator  
   - Federation agents
   
2. **Native Channel**: Implement WebSocket server with encryption

3. **Build integration**: Automatically build kowalski-cli in build.rs

4. **Error handling**: Add Result types with context throughout

---

## Next Steps (Priority Order)

1. [ ] Build kowalski-cli and verify integration works
2. [ ] Integrate kowalski-core as library (proper fix)
3. [ ] Add MCP offline/cached mode
4. [ ] Document AWS quantum credentials setup
5. [ ] Build dashboard Tauri app or remove --desktop flag
6. [ ] Implement secure native channel
7. [ ] Add comprehensive test suite

---

*Generated: 2026-03-07*
*Version: 0.1.0*

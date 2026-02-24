# Project Research Summary

**Project:** Housaky — Embedded AI Agent Infrastructure
**Domain:** Rust-based Edge AI Agents with Hardware Integration
**Researched:** 2026-02-24
**Confidence:** MEDIUM-HIGH

## Executive Summary

Housaky is a Rust-native embedded AI agent infrastructure that combines hardware integration (USB, serial, GPIO, firmware flashing) with AGI capabilities (goal engines, cognitive modules, memory systems) and defense-in-depth security (pairing, sandboxing, workspace scoping). The research confirms that building this class of system requires a tripartite architecture separating generative cognition, metacognition, and executive control — a pattern well-documented in recent AGI research.

The recommended stack leverages pure-Rust libraries for cross-platform hardware access (nusb, rppal, probe-rs) and modular AGI frameworks (neuron, AutoAgents). Security should be implemented layer-by-layer: pairing at the gateway, allowlists at channel input, sandboxing at runtime, and workspace scoping at the filesystem. The existing Housaky implementation already has strong table-stakes features (workspace scoping with 14 system directories blocked, bearer token authentication, tool allowlists, SQLite + FTS5 + vector memory).

The primary risks identified are: (1) USB enumeration race conditions causing intermittent device detection failures, (2) STM32 flash corruption from interrupted programming, (3) GPIO permission issues on Linux requiring proper udev configuration, and (4) goal engine infinite loops without resource limits. These pitfalls have clear mitigation strategies but require disciplined implementation.

## Key Findings

### Recommended Stack

The research identifies a mature Rust ecosystem for all three pillars of Housaky:

**Hardware Integration:**
- **nusb** (0.2.1): Cross-platform USB discovery — pure Rust, async-first, no libusb FFI dependency
- **serialport** (4.3.0): Serial communication with tokio-serial async support
- **rppal** (0.22.1): Raspberry Pi GPIO/I2C/SPI/PWM — most complete Pi HAL
- **probe-rs** (0.31.0+): STM32/Nucleo debugging and flashing — industry standard for ARM/RISC-V
- **embedded-hal** (1.0.0): Hardware abstraction traits — stable since Jan 2024

**AGI Agent Frameworks:**
- **neuron**: Composable building blocks with traits (Provider, Tool, ContextStrategy) — "serde, not serde_json" philosophy
- **AutoAgents** (0.3.4): Production-grade multi-agent framework with MCP support
- **asterbot**: WASM-component-based modular agent (Feb 2026 release)

**Security Sandboxing:**
- **landlock** (0.4.4): Linux kernel filesystem sandboxing — no root required
- **tabox** (1.3.6): Seccomp-based syscall filtering with resource limits
- **silicube**: IOI Isolate-based production sandboxing

### Expected Features

**Table Stakes (Already Implemented or Near-Complete):**
- USB Device Discovery — already exists via `housaky hardware discover`
- Workspace Scoping — already implemented with 14 system directories blocked, symlink escape detection
- Basic Pairing/Auth — 6-digit one-time code, bearer token via `/pair`
- Tool Allowlists — `allowed_commands`, `forbidden_paths` implemented
- Rate Limiting — standard security feature
- Memory Persistence — SQLite + FTS5 + vector search hybrid
- Provider Abstraction — 30+ AI providers supported

**Differentiators (Competitive Advantage):**
- **Raspberry Pi GPIO Control** (MEDIUM complexity): Enables physical world interaction — LED control, sensors, motors
- **Serial Communication** (LOW): Foundation for hardware integration with MCUs
- **STM32/Nucleo Firmware Flashing** (HIGH): Directly program embedded boards — unique differentiator
- **Goal Engine** (HIGH): Multi-step autonomous objective pursuit, goal decomposition
- **Docker Sandbox** (MEDIUM): Already partially implemented, needs Landlock/Firejail alternatives

**Defer to v2+:**
- **Cognitive Modules**: Reflection, planning, self-critique — requires goal engine first
- **Self-Improvement**: High goal-drift risk, needs user approval workflow
- **Multi-Agent Coordination**: Scales complexity significantly

### Architecture Approach

The recommended architecture follows a four-layer model:

1. **Application Layer**: AGI Core Engine, Goal Engine, Cognitive Modules, Security Manager
2. **Orchestration Layer**: Memory Manager, Tool Registry, Channel Registry, Provider Bridge
3. **Hardware Integration Layer**: USB Discovery, Serial Handler, GPIO Manager, Flasher Service
4. **Runtime Layer**: Native Executor, Docker Sandbox, WASM Runtime

**Major Components:**
1. **AGI Core Engine**: Continuous thought generation, reasoning, planning — LLM-backed with tool orchestration
2. **Goal Engine**: Goal decomposition, progress tracking, success criteria — state machine with episodic memory
3. **Security Manager**: Pairing, sandboxing, workspace scoping, rate limiting — policy enforcer with allowlists
4. **Memory Manager**: Short-term, episodic, semantic memory — SQLite + vector store hybrid
5. **Hardware Backends**: Trait-based plugin architecture for USB, serial, GPIO, flashing — each device family gets its own implementation

### Critical Pitfalls

1. **USB Device Enumeration Race Conditions** — Implement device refresh debouncing (100-500ms), track state changes over time, use libudev for event-driven tracking instead of polling

2. **STM32/Nucleo Flash Corruption** — Implement verify-after-write with rollback, detect bootloader mode, add firmware checksum validation, support dual-bank atomic swap

3. **GPIO Pin Permission Hell** — Detect and support both sysfs (legacy) and libgpiod (modern), auto-generate udev rules during setup, provide clear error messages

4. **Goal Engine Infinite Loop** — Implement hard limits on goal decomposition depth (max 10 levels), add cycle detection, enforce resource budgets per goal, make self-improvement opt-in

5. **Sandbox Escape via Tool Abuse** — Use gVisor or Firecracker microVMs, never mount docker socket, implement strict syscall allowlists, validate tool parameters for escape patterns

6. **Workspace Symlink Bypass** — Canonicalize paths before every operation, re-check after opening, block symlink creation in restricted directories, use O_NOFOLLOW

## Implications for Roadmap

Based on research, the following phase structure is recommended:

### Phase 1: Core Infrastructure Completion
**Rationale:** Complete existing table-stakes features and establish foundation for hardware integration. This phase leverages existing strong security baseline and adds missing low-complexity hardware support.

**Delivers:**
- Raspberry Pi GPIO control via rppal
- Serial communication foundation (UART, CDC-ACM)
- Complete Docker sandbox with Landlock/Firejail alternatives

**Addresses:** FEATURES.md — GPIO Control, Serial Communication, Docker Sandbox (complete existing)
**Avoids:** PITFALLS.md — GPIO Permission Hell (via udev rules), Workspace Symlink Bypass

### Phase 2: Hardware Integration Expansion
**Rationale:** Build on serial foundation to add STM32/Nucleo firmware flashing — the key differentiator. This requires stable serial communication first (Phase 1).

**Delivers:**
- STM32/Nucleo firmware flashing via probe-rs
- USB device hot-plug event handling
- Firmware verification and rollback

**Addresses:** FEATURES.md — STM32/Nucleo Flashing
**Avoids:** PITFALLS.md — USB Enumeration Race Conditions (debouncing + libudev), STM32 Flash Corruption (verify-after-write + checksum)

### Phase 3: AGI Capabilities
**Rationale:** Goal engine requires memory persistence (already exists) and builds toward autonomous agent behavior. This is the core AGI differentiator.

**Delivers:**
- Goal engine with decomposition, progress tracking
- Cognitive modules (reflection, planning, self-critique)
- Resource budget enforcement per goal

**Addresses:** FEATURES.md — Goal Engine, Cognitive Modules
**Avoids:** PITFALLS.md — Goal Engine Infinite Loop (depth limits + cycle detection), Cognitive Module Memory Corruption (validation + snapshots)

### Phase 4: Advanced Features (v2+)
**Rationale:** Multi-agent coordination and self-improvement add significant complexity and risk. Defer until product-market fit established.

**Delivers:**
- Multi-agent collaboration and task delegation
- User-controlled self-improvement with approval workflow

**Addresses:** FEATURES.md — Multi-Agent Coordination, Self-Improvement
**Avoids:** PITFALLS.md — Sandbox Escape (gVisor/Firecracker), Pairing Timing Attacks (rate limiting)

### Phase Ordering Rationale

- **Phases 1-2:** Hardware before AGI — physical world interaction is a clearer immediate value proposition; AGI builds on existing memory system
- **Phases 1-2 group together:** Security is foundation — sandboxing and scoping must be solid before expanding capabilities
- **Phase 3 defers cognitive to after goal:** Cognitive modules enhance goal engine, not replace it; requires goal engine stability first
- **Phase 4 delays multi-agent:** Single-agent must be robust before coordination adds complexity
- **Avoids all critical pitfalls:** Each phase addresses specific pitfalls identified in research

### Research Flags

**Phases needing deeper research during planning:**
- **Phase 2 (Hardware Integration):** STM32 DFU protocol specifics — may need API research for bootloader mode detection
- **Phase 3 (AGI):** Goal engine state machine design — architectural patterns well-documented but implementation details sparse

**Phases with standard patterns (skip research-phase):**
- **Phase 1:** GPIO, serial, Docker sandbox — well-documented, established patterns
- **Security (all phases):** landlock, tabox, workspace scoping — covered by existing documentation

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Verified via crates.io versions, Context7 documentation, web search for recent releases |
| Features | MEDIUM | Competitor analysis and industry guidance; some inference on embedded requirements |
| Architecture | MEDIUM | Based on AGI research papers and embedded systems patterns; some architectural choices are novel |
| Pitfalls | MEDIUM | Community-collected patterns; some pitfalls (USB race, flash corruption) are well-documented |

**Overall confidence:** MEDIUM-HIGH

Stack is solid (HIGH) — verified technologies with recent releases. Architecture and features are well-supported by research but include some novel combinations. Pitfalls are derived from community experience and embedded systems best practices.

### Gaps to Address

- **STM32 DFU Protocol Details:** Specific bootloader mode detection may need vendor documentation review during Phase 2 planning
- **Cognitive Module State Machine:** Architectural pattern is documented but implementation-specific guidance is sparse — may need prototyping
- **Multi-Agent Coordination Patterns:** No strong consensus on best practices — defer to Phase 4 allows market to mature

## Sources

### Primary (HIGH confidence)
- Context7/rppal — Raspberry Pi HAL documentation
- Context7/probe-rs — STM32/RISC-V flashing documentation
- Context7/embedded-hal — Hardware abstraction trait definitions
- Context7/landlock — Linux filesystem sandboxing
- crates.io — Version verification, release dates

### Secondary (MEDIUM confidence)
- OpenClaw Architecture Deep Dive (Towards AI, Feb 2026) — feature landscape
- NVIDIA "Practical Security Guidance for Sandboxing Agentic Workflows" (Jan 2026)
- Wiz "AI Agent Security Best Practices" (Dec 2025)
- "How to Build an AGI" — Kowalski 2026, tripartite cognitive architecture

### Tertiary (LOW confidence)
- AutoAgents, Anda, Amico V2 — web search results, stars/activity indicates active maintenance but limited deep documentation
- LiteBox (Microsoft research, Feb 2026) — very recent, limited production validation

---
*Research completed: 2026-02-24*
*Ready for roadmap: yes*

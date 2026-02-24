# Architecture Research

**Domain:** Embedded AI Agent Systems with Hardware Integration and AGI Capabilities
**Researched:** 2026-02-24
**Confidence:** MEDIUM

## Standard Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           Application Layer                              │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │  AGI Core    │  │   Goal       │  │   Cognitive  │  │  Security  │ │
│  │   Engine     │  │   Engine     │  │   Modules    │  │   Manager  │ │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └─────┬──────┘ │
│         │                 │                 │                │         │
├─────────┴─────────────────┴─────────────────┴────────────────┴─────────┤
│                         Orchestration Layer                               │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │   Memory     │  │    Tool      │  │    Channel   │  │  Provider  │ │
│  │   Manager    │  │   Registry   │  │   Registry   │  │  Bridge    │ │
│  └──────┬───────┘  └──────────────┘  └──────────────┘  └────────────┘ │
├─────────────────────────────────────────────────────────────────────────┤
│                       Hardware Integration Layer                          │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │     USB      │  │    Serial    │  │     GPIO     │  │   Flasher  │ │
│  │  Discovery   │  │   Handler    │  │   Manager    │  │   Service  │ │
│  └──────────────┘  └──────────────┘  └──────────────┘  └────────────┘ │
├─────────────────────────────────────────────────────────────────────────┤
│                         Runtime Layer                                     │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                 │
│  │   Native     │  │   Docker     │  │    WASM      │                 │
│  │   Executor   │  │   Sandbox    │  │   Runtime    │                 │
│  └──────────────┘  └──────────────┘  └──────────────┘                 │
└─────────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| **AGI Core Engine** | Continuous thought generation, reasoning, planning | LLM-backed with tool orchestration |
| **Goal Engine** | Goal decomposition, progress tracking, success criteria | State machine with episodic memory |
| **Cognitive Modules** | Self-reflection, metacognition, insight synthesis | Separate processing loop with memory feedback |
| **Security Manager** | Pairing, sandboxing, workspace scoping, rate limiting | Policy enforcer with allowlists |
| **Memory Manager** | Short-term (working), episodic, semantic memory | SQLite + vector store hybrid |
| **Tool Registry** | Discovery, execution, result parsing for capabilities | Trait-based plugin system |
| **Channel Registry** | Messaging protocol adapters (Telegram, Discord, etc.) | Async message handlers |
| **Provider Bridge** | Multi-LLM abstraction (OpenRouter, Anthropic, Ollama) | Trait-based provider interface |
| **USB Discovery** | Device enumeration, VID/PID matching, hotplug | nusb crate for cross-platform |
| **Serial Handler** | UART/Serial communication with microcontrollers | tokio-serial for async I/O |
| **GPIO Manager** | Pin control, interrupt handling for Raspberry Pi | rppal crate or sysfs direct |
| **Flasher Service** | STM32/Nucleo DFU, Arduino hex upload | openocd/stm32flash CLI wrapper |
| **Native Executor** | Direct command execution with scoping | Process spawning with sandboxing |
| **Docker Sandbox** | Isolated container execution | dockerd API with strict policies |
| **WASM Runtime** | Sandboxed plugin execution (future) | wasmtime or wasmer |

## Recommended Project Structure

```
src/
├── lib.rs                    # Core library entry
├── main.rs                   # CLI entry point
├── agent/                    # AGI core
│   ├── mod.rs
│   ├── engine.rs             # Main agent orchestration
│   ├── goal_engine.rs        # Goal decomposition & tracking
│   ├── cognitive/            # Cognitive modules
│   │   ├── mod.rs
│   │   ├── metacognition.rs  # Self-reflection
│   │   ├── insight.rs        # Insight synthesis
│   │   └── reasoning.rs       # Chain-of-thought
│   └── memory/               # Memory management
│       ├── mod.rs
│       ├── working.rs        # Short-term memory
│       ├── episodic.rs       # Episode storage
│       └── semantic.rs       # Knowledge base
├── hardware/                 # Hardware integration
│   ├── mod.rs
│   ├── usb/                  # USB device discovery
│   │   ├── mod.rs
│   │   ├── discovery.rs     # VID/PID enumeration
│   │   └── device.rs        # Device handle abstraction
│   ├── serial/              # Serial communication
│   │   ├── mod.rs
│   │   ├── handler.rs       # Async serial I/O
│   │   └── protocol.rs      # Command/response parsing
│   ├── gpio/                # GPIO control
│   │   ├── mod.rs
│   │   ├── manager.rs       # Pin state machine
│   │   └── interrupts.rs    # Event handling
│   └── flasher/             # Firmware programming
│       ├── mod.rs
│       ├── stm32.rs         # STM32 DFU protocol
│       ├── nucleo.rs       # ST-Link flashing
│       └── arduino.rs       # AVR hex upload
├── security/                # Security layer
│   ├── mod.rs
│   ├── pairing.rs           # Device pairing
│   ├── sandbox.rs          # Sandboxing strategies
│   ├── scoping.rs           # Workspace scoping
│   └── policies.rs          # Security policies
├── providers/               # AI provider adapters
│   └── ...
├── channels/                # Messaging channels
│   └── ...
├── tools/                   # Tool implementations
│   └── ...
├── memory/                  # Memory backends
│   └── ...
└── runtime/                 # Execution runtimes
    ├── mod.rs
    ├── native.rs           # Direct execution
    ├── docker.rs           # Container sandbox
    └── wasm.rs             # WASM plugins
```

### Structure Rationale

- **`agent/`:** Central AGI orchestration — goal engine, cognitive modules, and memory form the cognitive architecture. This is the "brain" that coordinates all other subsystems.
- **`hardware/`:** Dedicated folder for all embedded/edge hardware interactions. USB, serial, GPIO, and flashing are cleanly separated but share common traits.
- **`security/`:** Security is a cross-cutting concern, not scattered. Pairing, sandboxing, and scoping集中管理.
- **`runtime/`:** Different execution strategies (native, Docker, WASM) share a common trait, allowing runtime swapping via config.
- **`providers/channels/tools/memory/`:** Existing plugin subsystems — extend via traits, not modification.

## Architectural Patterns

### Pattern 1: Trait-Based Plugin Architecture

**What:** All subsystems (providers, channels, tools, memory, hardware) implement common traits. Configuration specifies which implementation to use at runtime.

**When to use:** When you need maximum swappability and zero-compromise deployment across different environments.

**Trade-offs:**
- Pro: Swap implementations without code changes (e.g., memory backend, runtime sandbox)
- Pro: Easy testing via mock implementations
- Con: Trait design must be stable; breaking changes affect all implementations

**Example:**
```rust
pub trait HardwareBackend {
    async fn discover(&mut self) -> Result<Vec<Device>, Error>;
    async fn read(&self, device: &Device, addr: u32, len: usize) -> Result<Vec<u8>, Error>;
    async fn write(&self, device: &Device, addr: u32, data: &[u8]) -> Result<(), Error>;
    async fn flash(&self, device: &Device, firmware: &[u8]) -> Result<(), Error>;
}

pub struct UsbBackend { /* nusb implementation */ }
pub struct SerialBackend { /* tokio-serial implementation */ }

impl HardwareBackend for UsbBackend { /* ... */ }
impl HardwareBackend for SerialBackend { /* ... */ }
```

### Pattern 2: Tripartite Cognitive Architecture

**What:** AGI system divided into three functionally differentiated processes: (1) Generative Cognitive Module for continuous thought, (2) Memory-Metacognition Module for consolidation and reflection, (3) Executive Controller for overall regulation.

**When to use:** When building toward AGI capabilities — enables emergent metacognition and self-improvement.

**Trade-offs:**
- Pro: Cognitive complexity emerges from module interaction, not monolithic design
- Pro: Enables self-reflection and insight generation
- Con: More complex orchestration; modules can conflict

**Reference:** Based on research from "How to Build an AGI" (Kowalski, 2026) and brain-inspired architectures.

### Pattern 3: Event-Driven Hardware Abstraction

**What:** Hardware events (USB hotplug, GPIO interrupts, serial data) flow through an event bus. Components subscribe to relevant events.

**When to use:** When hardware integration requires responsiveness to dynamic conditions.

**Trade-offs:**
- Pro: Decouples hardware detection from response logic
- Pro: Enables reactive tool registration (device appears → tools become available)
- Con: Event ordering and backpressure need careful handling

### Pattern 4: Security-in-Depth Layers

**What:** Security applied at multiple layers: (1) pairing at gateway, (2) sandbox at runtime, (3) scoping at filesystem, (4) allowlists at channel input.

**When to use:** Always — embedded agents with hardware access need defense in depth.

**Trade-offs:**
- Pro: Single point of failure avoided
- Pro: Defense against different threat vectors
- Con: Performance overhead if not carefully designed

## Data Flow

### Hardware Interaction Flow

```
[User Command]
    ↓
[Agent Engine] → [Tool Resolution] → [Hardware Tool]
    ↓                                    ↓
[Memory] ←───────────────────────── [Hardware Backend Trait]
    ↓                                    ↓
[Serial/USB/GPIO] ──────────────────→ [Device]
    ↓
[MCU Response] → [Parse] → [Tool Result] → [Agent Engine]
```

### AGI Goal Decomposition Flow

```
[User Goal]
    ↓
[Goal Engine] → [Decompose into subgoals]
    ↓
[Subgoal Queue] → [Cognitive Module: Plan]
    ↓
[Action Sequence] → [Execute via Tools]
    ↓
[Result] → [Cognitive Module: Evaluate]
    ↓
[Memory: Store episode] → [Metacognition: Reflect]
    ↓
[Goal Complete?] → Yes: [Archive] / No: [Retry with adjustment]
```

### Security Verification Flow

```
[Incoming Request]
    ↓
[Gateway: Pairing Check] → Fail: Reject / Pass: Continue
    ↓
[Channel: Allowlist Check] → Fail: Ignore / Pass: Continue
    ↓
[Tool Execution: Scope Check] → Fail: Reject / Pass: Continue
    ↓
[Runtime: Sandbox Enforcement] → Fail: Terminate / Pass: Execute
    ↓
[Result Returned]
```

### Key Data Flows

1. **Hardware Discovery → Tool Registration:** When USB device plugged in, hardware layer enumerates it, matches against known device profiles, and registers relevant tools (e.g., STM32 flashing tools appear when Nucleo detected).

2. **Memory → Cognitive Loop:** Each agent cycle stores episode to memory. Metacognition module periodically queries memory for patterns, synthesizes insights, and feeds back into goal prioritization.

3. **Provider Abstraction:** All AI calls go through provider bridge → allows swapping between OpenRouter, Anthropic, Ollama, or custom endpoints without changing agent logic.

## Scaling Considerations

| Scale | Architecture Adjustments |
|-------|--------------------------|
| 0-1 devices | Single-threaded async is fine; native runtime adequate |
| 1-10 devices | Concurrent hardware handling needed; consider actor model |
| 10-100 devices | Hardware registry with connection pooling; parallel flashing |
| 100+ devices | Distributed agent coordination; fleet management layer |

### Scaling Priorities

1. **First bottleneck:** Serial port exhaustion — when multiple MCUs connected, OS serial port limits apply. Solution: Implement connection pooling and async multiplexing.

2. **Second bottleneck:** Memory for large context — AGI reasoning with full episodic memory. Solution: Semantic compression, priority-based memory eviction.

3. **Third bottleneck:** Flashing throughput — serial flashing is slow. Solution: Parallel flashers, batch firmware updates.

## Anti-Patterns

### Anti-Pattern 1: Monolithic Hardware Code

**What people do:** Put all hardware logic in a single module with giant match statements for device types.

**Why it's wrong:** Impossible to test, difficult to extend, violates single responsibility. New devices require modifying existing code.

**Do this instead:** Trait-based hardware backends. Each device family gets its own implementation of the `HardwareBackend` trait. Add new devices by implementing the trait, not editing existing code.

### Anti-Pattern 2: Blocking Hardware in Agent Loop

**What people do:** Call synchronous USB/serial operations directly from the agent's main loop.

**Why it's wrong:** Agent blocks waiting for hardware response. Other goals cannot progress. Deadlock if hardware is slow or unresponsive.

**Do this instead:** All hardware operations are async. Usetokio or async-std. Agent loop should be event-driven, not polling.

### Anti-Pattern 3: Security as Afterthought

**What people do:** Add pairing and sandboxing after building core agent functionality.

**Why it's wrong:** Hard to retrofit security. Attack surface already exists. Hardware access without security = remote code execution risk.

**Do this instead:** Build security layer first. Every component verifies with security manager before action. Pairing required before any tool execution.

### Anti-Pattern 4: Hardcoded Credentials in Firmware Tool

**What people do:** Embed API keys or credentials in the firmware binary that gets flashed to devices.

**Why it's wrong:** Credentials extracted from flashed devices. No rotation possible. Devices outside your control.

**Do this instead:** Credentials stored on host, passed to device at runtime via secure channel. Or use device-specific secure elements.

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| OpenRouter / Anthropic / OpenAI | Provider trait with HTTP client | Housaky already has this |
| STM32CubeProgrammer | CLI wrapper for flashing | Use `--locked` mode |
| OpenOCD | CLI/GDB for debugging | For advanced debugging |
| Docker daemon | Docker API client | For sandboxed tool execution |
| Raspberry Pi GPIO | sysfs or rppal | Requires Linux; falls back gracefully on non-Pi |
| USB devices | nusb cross-platform | Handles hotplug events |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| Agent Engine ↔ Hardware | Tool trait invocations | Hardware registered as tools |
| Goal Engine ↔ Memory | Async message passing | Episode storage/retrieval |
| Cognitive ↔ Provider Bridge | Request/response | LLM calls are blocking-ish |
| Security ↔ All | Policy checks before action | Fail-fast on policy violation |

## Sources

- "How to Build an AGI" — Kowalski 2026, tripartite cognitive architecture
- "Agentic AI Architecture" — Akira.ai, component breakdown
- "Agentic AI Needs a Systems Theory" — arXiv 2026, systems-theoretic perspective
- "AGI Architectures: What We Can Agree On" — Intelligence Strategy 2025
- "tinymcp: Unlocking the Physical World for LLMs" — Golioth, MCP + embedded
- "GAIA + Arduino: Local AI-Powered Hardware Control" — AMD, agent-to-MCU serial
- "Deploy an MCP Server on Raspberry Pi 5" — Arm, edge AI agent deployment
- "LiteRT for Microcontrollers" — Google, TFLite Micro on STM32
- "Embedded AI Systems Guide" — Waverley Software, full-stack embedded AI

---

*Architecture research for: Housaky enhancements (hardware integration + AGI + security)*
*Researched: 2026-02-24*

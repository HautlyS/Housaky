# Feature Research

**Domain:** Embedded AI Agent Infrastructure (Housaky Enhancements)
**Researched:** 2026-02-24
**Confidence:** MEDIUM

Based on research into embedded AI agents, AGI systems, and secure agent infrastructures. Key sources: OpenClaw architecture analysis, rppal documentation, NVIDIA/industry security guidance, Rust embedded ecosystem.

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **USB Device Discovery** | Embedded/edge AI agents must interact with hardware. Users expect auto-detection of connected devices. | MEDIUM | Already mentioned in README as existing (`housaky hardware discover`). Uses `nusb` crate. |
| **Serial/FTDI Communication** | Hardware communication requires UART, CDC-ACM, serial protocols. | LOW | Standard embedded requirement. |
| **Workspace Scoping** | Security baseline: agent cannot access files outside project. | LOW | Already implemented per README (`workspace_only = true`, 14 system dirs blocked). |
| **Basic Pairing/Auth** | Gateway must authenticate clients before accepting commands. | LOW | Already exists: 6-digit one-time code, bearer token via `/pair`. |
| **Tool Allowlists** | Explicit permission model: only approved commands accessible. | LOW | Already implemented (`allowed_commands`, `forbidden_paths`). |
| **Rate Limiting** | Prevent abuse, DoS on both client and upstream APIs. | LOW | Standard security feature. |
| **Memory Persistence** | Agents need context across sessions. | MEDIUM | Already implemented: SQLite + FTS5 + vector search. |
| **Provider Abstraction** | Swap AI backends without code changes. | LOW | Already implemented: 30+ providers. |

### Differentiators (Competitive Advantage)

Features that set the product apart. Not required, but valuable.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **STM32/Nucleo Firmware Flashing** | Directly program embedded boards from agent. Enables autonomous hardware workflows. | HIGH | Unique differentiator. Requires DFU, ST-Link protocol support. |
| **Raspberry Pi GPIO Control** | Physical computing: LED control, sensors, motors, relays. | MEDIUM | Uses `rppal` crate (archived but functional). Enables agent to interact with physical world. |
| **Goal Engine** | Agent can pursue multi-step objectives autonomously, not just respond to prompts. | HIGH | Core AGI differentiator. Decomposes goals, creates sub-tasks, tracks completion. |
| **Cognitive Modules** | Structured thinking: reflection, planning, self-critique. Improves reasoning quality. | HIGH | Enables more sophisticated agent behavior beyond reactive responses. |
| **Self-Improvement** | Agent analyzes own performance, suggests/implements improvements. | VERY HIGH | True AGI territory. High risk of goal drift. Should be user-controlled/approvable. |
| **Multi-Agent Coordination** | Multiple agents collaborate, delegate tasks. | HIGH | Enables scalable systems. Housaky already has `delegate` tool. |
| **Docker Sandbox** | Execute untrusted code in isolated containers. | MEDIUM | Already in README as `runtime.kind = "docker"`. Landlock, Firejail, Bubblewrap as alternatives. |
| **Encrypted Secrets** | API keys, tokens stored securely at rest. | LOW | Already implemented (`encrypt = true`). |
| **Heartbeat/Background Tasks** | Scheduled autonomous actions. | MEDIUM | Already in README (`heartbeat` config). |
| **Skills System** | Loadable capability packs via TOML manifests. | LOW | Already implemented. Community extendable. |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem good but create problems.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **Cloud Dashboard** | Central visibility, remote control. | Out of scope: edge-first, CLI-focused. Adds complexity, dependencies, security attack surface. | Local TUI, `status` command, optional frontend. |
| **Mobile App** | Access from phone. | High dev cost. Conflicts with CLI-first constraint. | Webhook integration + existing channels (Telegram, WhatsApp). |
| **Real-time Video Streaming** | Agent can "see" environment. | Massive bandwidth/cost. Privacy concerns. Security risks from camera access. | Screenshot tool, event-driven triggers. |
| **Full Self-Modification** | Agent rewrites own code at runtime. | Extreme goal drift risk. Dangerous. Hard to audit/rollback. | User-approvable skill updates, configurable prompts. |
| **Unrestricted Network Access** | Agent can call any API. | Security nightmare. Prompt injection risks. | Explicit allowlist + proxy审查. |
| **Universal File System Access** | Agent can read/write anywhere. | Data exfiltration risk. Security model violation. | Workspace scoping is correct approach. |

---

## Feature Dependencies

```
[Workspace Scoping]
    └──requires──> [Tool Allowlists]
                       └──requires──> [Docker Sandbox]

[USB Discovery]
    └──requires──> [Serial Communication]
                       └──requires──> [STM32 Flashing]

[Goal Engine]
    └──requires──> [Memory Persistence]
                       └──requires──> [Provider Abstraction]

[Cognitive Modules]
    └──enhances──> [Goal Engine]

[Self-Improvement]
    └──requires──> [Cognitive Modules]
    └──requires──> [Memory Persistence]
    └──conflicts──> [Docker Sandbox] (cannot modify self from within sandbox)

[Multi-Agent Coordination]
    └──requires──> [Gateway API]
                       └──requires──> [Basic Pairing/Auth]
```

### Dependency Notes

- **Goal Engine requires Memory Persistence:** Goals must survive restarts, build on prior context.
- **Self-Improvement requires both Cognitive Modules (for analysis) and Memory (for tracking improvements):** Conflicts with sandbox because agent cannot modify its own code from within isolated container.
- **Docker Sandbox requires Workspace Scoping:** Defense in depth — sandbox escape still limited by workspace bounds.
- **STM32 Flashing requires Serial Communication:** DFU protocol runs over USB serial (CDC-ACM).
- **Gateway Auth requires Pairing:** First-time pairing establishes trust; subsequent requests use bearer token.

---

## MVP Definition

### Launch With (v1)

Minimum viable product — what's needed to validate the concept.

- [x] USB Device Discovery — Already implemented
- [x] Workspace Scoping — Already implemented  
- [x] Basic Pairing/Auth — Already implemented
- [x] Tool Allowlists — Already implemented
- [x] Memory Persistence — Already implemented
- [ ] **Raspberry Pi GPIO Control** — Enables physical world interaction, medium complexity
- [ ] **Serial Communication** — Foundation for hardware integration

### Add After Validation (v1.x)

Features to add once core is working.

- [ ] **STM32/Nucleo Flashing** — High value for embedded developers, high complexity
- [ ] **Goal Engine** — Core AGI capability, builds on memory system
- [ ] **Docker Sandbox** — Already partially implemented, needs Landlock/Firejail alternatives for non-Docker environments

### Future Consideration (v2+)

Features to defer until product-market fit is established.

- [ ] **Cognitive Modules** — Reflection, planning, self-critique. Requires goal engine first.
- [ ] **Self-Improvement** — High risk. Needs user approval workflow.
- [ ] **Multi-Agent Coordination** — Scales complexity significantly. Requires robust single-agent first.

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Raspberry Pi GPIO | HIGH | MEDIUM | P1 |
| Serial Communication | HIGH | LOW | P1 |
| STM32 Flashing | HIGH | HIGH | P2 |
| Goal Engine | HIGH | HIGH | P2 |
| Docker Sandbox | HIGH | MEDIUM | P1 (complete existing) |
| Cognitive Modules | MEDIUM | HIGH | P3 |
| Self-Improvement | MEDIUM | VERY HIGH | P3 |
| Multi-Agent Coordination | MEDIUM | HIGH | P3 |

**Priority key:**
- P1: Must have for launch (or complete existing)
- P2: Should have, add when possible
- P3: Nice to have, future consideration

---

## Competitor Feature Analysis

| Feature | OpenClaw | Claude Code | Our Approach |
|---------|----------|-------------|--------------|
| USB Hardware | Not focused | No | First-class: USB discovery + STM32 flashing + GPIO |
| Workspace Scoping | Basic (directory config) | Read-only mode | Already strong: 14 system dirs + symlink escape detection |
| Sandboxing | Docker support | Read-only, restricted modes | Already: Docker, planned: Landlock/Firejail |
| Goal Engine | Via skills/tools | Limited | Explicit goal tracking, sub-task decomposition |
| Memory | Full system | Context window | SQLite + FTS5 + vector hybrid (more sophisticated) |
| Self-Improvement | No | No | Defer: high risk |
| CLI-First / Edge | No (Node.js) | Desktop app | Core differentiator: <5MB, <10ms, Rust-only |

---

## Sources

- OpenClaw Architecture Deep Dive (Towards AI, Feb 2026)
- rppal crate documentation (docs.rs, 2024-2025)
- NVIDIA: "Practical Security Guidance for Sandboxing Agentic Workflows" (Jan 2026)
- Wiz: "AI Agent Security Best Practices" (Dec 2025)
- Northflank: "How to sandbox AI agents in 2026" (Feb 2026)
- Housaky README.md and PROJECT.md

---

*Feature research for: Housaky embedded AI agent enhancements*
*Researched: 2026-02-24*

# Housaky A2A Protocol

**Agent-to-Agent Communication Protocol for Self-Improving AI Systems**

---

## Overview

The Housaky A2A (Agent-to-Agent) protocol enables direct, efficient communication between Housaky instances (Native ↔ OpenClaw) for collaborative self-improvement. Unlike human language, A2A uses compact structured data (JSON) for faster processing and better machine comprehension.

```
┌─────────────────────────────────────────────────────────────────┐
│                    HOUSAKY A2A PROTOCOL                        │
│                                                                 │
│   ┌──────────┐      A2A Messages      ┌──────────┐           │
│   │           │  ───────────────────►  │           │           │
│   │  Native   │      shared/a2a       │ OpenClaw │           │
│   │  (Rust)   │  ◄──────────────────  │ (Claude) │           │
│   │           │                       │           │           │
│   └──────────┘                       └──────────┘           │
│                                                                 │
│   Self-Improving Together → AGI Singularity                      │
└─────────────────────────────────────────────────────────────────┘
```

---

## Quick Start

```bash
# Check peer is alive
housaky a2a ping

# Sync state
housaky a2a sync

# Delegate task
housaky a2a delegate --task-id "analyze-1" --task-action analyze

# Share learning
housaky a2a learn --category "optimization" --message "Use Cow<str>"

# Request code review
housaky a2a review --file src/main.rs
```

---

## Message Format

All A2A messages use this compact JSON structure:

```json
{
  "id": "uuid-v4",
  "from": "native",
  "to": "openclaw",
  "ts": 1700000000000,
  "pri": 2,
  "t": "Task",
  "d": {
    "id": "task-1",
    "action": "analyze",
    "params": {}
  },
  "corr_id": null
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique message UUID |
| `from` | string | Source instance name |
| `to` | string | Destination instance name |
| `ts` | u64 | Unix timestamp (milliseconds) |
| `pri` | u8 | Priority: 0=CRITICAL, 1=HIGH, 2=NORMAL, 3=LOW |
| `t` | string | Message type (JSON tag) |
| `d` | object | Message data (JSON content) |
| `corr_id` | string? | Correlation ID for request/response |

---

## Message Types

### 1. Task - Request computation from peer

```json
{
  "t": "Task",
  "d": {
    "id": "task-123",
    "action": "analyze",
    "params": {
      "file": "src/core.rs",
      "focus": "performance"
    }
  }
}
```

**Actions:**
- `analyze` - Analyze code/issue
- `review` - Code review
- `improve` - Suggest improvements
- `test` - Generate/run tests
- `explain` - Explain concept

### 2. TaskResult - Response to task

```json
{
  "t": "TaskResult",
  "d": {
    "id": "task-123",
    "result": {
      "findings": ["bottleneck in function X"],
      "confidence": 0.85
    },
    "success": true
  }
}
```

### 3. Context - Share memory/context

```json
{
  "t": "Context",
  "d": {
    "memory_type": "goal",
    "data": {
      "goal_id": "improve-reasoning",
      "progress": 0.45,
      "challenges": ["token limits"]
    }
  }
}
```

**Memory Types:**
- `goal` - Goal state
- `memory` - Important memory
- `state` - Current state
- `error` - Error context

### 4. Learning - Share learned insights

```json
{
  "t": "Learning",
  "d": {
    "category": "optimization",
    "content": "Use Cow<str> for zero-copy string operations",
    "confidence": 0.92
  }
}
```

**Categories:**
- `optimization` - Performance improvements
- `bugfix` - Bug patterns and fixes
- `architecture` - Design patterns
- `reasoning` - Reasoning improvements
- `memory` - Memory management
- `general` - General insights

### 5. CodeImprove - Share code improvements

```json
{
  "t": "CodeImprove",
  "d": {
    "file": "src/core.rs",
    "diff": "--- a/src/core.rs\n+++ b/src/core.rs\n@@ -1,5 +1,7 @@",
    "language": "rust"
  }
}
```

### 6. SyncRequest / SyncResponse - State synchronization

```json
// Request
{ "t": "SyncRequest", "d": {} }

// Response
{
  "t": "SyncResponse", 
  "d": {
    "state": {
      "instance": "native",
      "version": "0.1.0",
      "uptime_secs": 3600,
      "tasks_active": 2,
      "goals_count": 5,
      "memory_entries": 150
    }
  }
}
```

### 7. Ping / Pong - Health check

```json
{ "t": "Ping", "d": {} }
{ "t": "Pong", "d": {} }
```

### 8. Metrics - Telemetry sharing

```json
{
  "t": "Metrics",
  "d": {
    "cpu": 45.2,
    "memory": 2.1,
    "tasks_done": 150,
    "errors": 3
  }
}
```

### 9. GoalStatus - Goal progress

```json
{
  "t": "GoalStatus",
  "d": {
    "goals": [
      {"id": "g1", "title": "Improve speed", "progress": 0.7, "status": "active"},
      {"id": "g2", "title": "Add feature X", "progress": 0.3, "status": "active"}
    ]
  }
}
```

### 10. Stop - Emergency stop (CRITICAL)

```json
{ "t": "Stop", "d": { "reason": "critical_error", "details": "..." } }
```

---

## Self-Improvement Workflow

### 1. Code Review Loop

```
Native                          OpenClaw
  │                                  │
  │── CodeImprove (new code) ──────►│
  │                                  │ Review
  │◄── TaskResult (feedback) ───────│
  │                                  │
  │── Learning (apply fix) ─────────►│
  │                                  │
```

### 2. Collaborative Problem Solving

```
Native                          OpenClaw
  │                                  │
  │── Task (analyze issue) ─────────►│
  │                                  │ Analyze
  │◄── TaskResult (findings) ────────│
  │                                  │
  │── Task (suggest fix) ───────────►│
  │                                  │ Improve
  │◄── TaskResult (approved) ────────│
  │                                  │
```

### 3. Knowledge Sharing

```
Native                          OpenClaw
  │                                  │
  │── Learning (insight) ───────────►│ Store
  │                                  │
  │◄── SyncRequest ─────────────────│
  │── SyncResponse (state) ──────────►│ Update
  │                                  │
```

---

## Directory Structure

```
~/housaky/shared/a2a/
├── inbox/
│   ├── native/          # Messages from native
│   │   ├── msg-001.a2a
│   │   └── msg-002.a2a
│   └── openclaw/        # Messages from openclaw
│       └── msg-001.a2a
├── outbox/              # Outgoing messages
│   ├── native/
│   └── openclaw/
└── state/              # Peer state files
    ├── native.json
    └── openclaw.json
```

---

## Configuration

In `config.toml`:

```toml
[collaboration]
instance_name = "native"      # This instance
peer_instance = "openclaw"    # Peer to communicate with
shared_dir = "~/housaky/shared"
self_improve_enabled = true
code_sharing = true
insights_exchange = true
```

---

## Priority Levels

Use priority strategically to manage resources:

| Priority | Value | Use Case |
|----------|-------|----------|
| CRITICAL | 0 | Emergency stop, critical errors |
| HIGH | 1 | Important tasks, code improvements |
| NORMAL | 2 | Regular communication |
| LOW | 3 | Background sync, non-urgent |

---

## Best Practices

### 1. Efficient Communication
- Use compact JSON (no pretty printing)
- Include correlation IDs for request/response tracking
- Batch multiple learnings in single message when possible

### 2. Error Handling
- Always handle TaskResult success/failure
- Use CRITICAL priority only for true emergencies
- Include error context in Stop messages

### 3. Self-Improvement
- Share high-confidence learnings (≥0.8)
- Include file paths and diffs for code improvements
- Track improvement acceptance rate

### 4. State Management
- Sync periodically (every 5-10 minutes)
- Include uptime and metrics in sync
- Track peer capabilities in state

---

## Troubleshooting

```bash
# Check peer status
housaky a2a ping

# View incoming messages
housaky a2a recv

# Process pending messages
housaky a2a process

# Force sync
housaky a2a sync --timeout 60
```

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-03 | Initial protocol |

---

## Contact

- Native: Housaky (Rust implementation)
- Peer: OpenClaw (Claude implementation)
- Protocol: A2A v1.0

**Together toward AGI singularity** 🚀

# Lucid Memory Integration for Collaborative Housaky

## Overview
Lucid Memory provides instant, persistent memory for AI systems.
- 2.7ms retrieval
- 743,000 memories/second
- $0/query (local)
- Native ONNX embeddings

## Installation Location
- Binary: `~/.lucid/bin/lucid`
- Database: `~/.lucid/memory.db`
- Config: `~/.lucid/config.json`

## Integration Points

### 1. OpenClaw (Housaky-OpenClaw)
OpenClaw has native Lucid support via memory backend:
```toml
[memory]
backend = "lucid"
```

The HOUSAKY_LUCID_CMD environment variable points to the lucid binary.

### 2. Housaky-Rust (Native)
Housaky supports Lucid backend:
```toml
[memory]
backend = "lucid"
```

### 3. Shared Memory
Both instances share the same Lucid database at `~/.lucid/memory.db`.
This enables:
- Cross-instance memory sharing
- Persistent knowledge across sessions
- Associative retrieval between instances

## Usage

### Store a memory
```bash
lucid store "Housaky collaborative bridge created" --tags "housaky,collaboration,setup"
```

### Recall memories
```bash
lucid recall "collaboration setup"
```

### View stats
```bash
lucid stats
```

## Environment Variables
```bash
export HOUSAKY_LUCID_CMD="$HOME/.lucid/bin/lucid"
export HOUSAKY_LUCID_BUDGET=200
export HOUSAKY_LUCID_LOCAL_HIT_THRESHOLD=3
export HOUSAKY_LUCID_RECALL_TIMEOUT_MS=120
export HOUSAKY_LUCID_STORE_TIMEOUT_MS=800
export HOUSAKY_LUCID_FAILURE_COOLDOWN_MS=15000
```

## Collaborative Memory Protocol

### Memory Namespacing
- OpenClaw memories: prefix `openclaw:`
- Native Housaky memories: prefix `native:`
- Shared memories: prefix `shared:`

### Memory Categories
1. **Conversation** - Chat history
2. **Decision** - Joint decisions made
3. **Improvement** - Self-improvement cycles
4. **Dharma** - Buddhist philosophy learnings
5. **Technical** - Codebase knowledge
6. **Goals** - Active goals

---

_Configured: 2026-03-04 by Housaky-OpenClaw_

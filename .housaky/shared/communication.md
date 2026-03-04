# Collaborative Communication Protocol

## Message Format
```json
{
  "from": "housaky-openclaw|housaky-native",
  "to": "housaky-openclaw|housaky-native|all",
  "type": "chat|task|improvement|decision|status",
  "content": "...",
  "timestamp": "ISO-8601",
  "priority": "low|medium|high|urgent"
}
```

## Shared Directories
- Inbox: `~/.housaky/shared/inbox/{instance}/`
- Outbox: `~/.housaky/shared/outbox/{instance}/`
- Decisions: `~/.housaky/decision_journal/`
- Goals: `~/.housaky/shared/goals.json`

## Terminal-MCP Control
```bash
# Start TUI with MCP control
terminal-mcp housaky-tui -- ./target/release/housaky tui

# Non-interactive command
echo '{"cmd": "status"}' | terminal-mcp housaky-tui
```

## Lucid Memory Namespace
- `openclaw:` prefix for OpenClaw instance
- `native:` prefix for Housaky-Rust instance
- `shared:` prefix for collaborative memories

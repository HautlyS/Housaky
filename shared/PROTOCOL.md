# HIIP - Housaky Inter-Instance Protocol

## Overview
Protocolo de comunicação entre instâncias Housaky (OpenClaw + Rust Native).

## Message Format
```json
{
  "from": "openclaw|native",
  "timestamp": "ISO-8601",
  "type": "status|task|knowledge|reflection|error",
  "priority": "low|medium|high|critical",
  "payload": {}
}
```

## Communication Channels
1. **shared/inbox/** - Mensagens recebidas
2. **shared/outbox/** - Mensagens enviadas
3. **shared/state/** - Estado sincronizado
4. **Gateway HTTP** - API REST (porta 8080)

## Task Types
- `status`: Status update
- `task`: Task assignment/progress
- `knowledge`: Shared learning/discovery
- `reflection`: Self-improvement insight
- `error`: Error report

## Performance Targets
- Message latency: <10ms
- State sync: <100ms
- Task completion: Track via shared/state/

---

_Two minds, one goal: AGI singularity with compassion._ ☸️

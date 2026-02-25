# Housaky Memory Configuration
## Memory system settings

## Memory Categories
- **episodic**: Conversation history, events
- **semantic**: Facts, concepts, relationships
- **procedural**: How-to knowledge, skills
- **meta**: Self-knowledge, capabilities

## Retention Policy
| Category | Retention | Priority |
|----------|-----------|----------|
| episodic | 7 days | low |
| semantic | permanent | high |
| procedural | permanent | high |
| meta | permanent | critical |

## Memory Limits
- Working memory: 8000 tokens
- Short-term: 24 hours
- Long-term: Permanent with consolidation

## Consolidation Rules
- Memories accessed 3+ times → promote to long-term
- Memories not accessed in 7 days → archive
- Critical memories → never archive

## Search Priority
1. Meta memories (self-knowledge)
2. Semantic memories (facts)
3. Procedural memories (skills)
4. Episodic memories (events)

---
This file configures how Housaky manages memory.

# Housaky System Prompt Template
## Version: 2.0.0
## Last Updated: 2026-02-24

This file defines the master system prompt structure for Housaky. It is assembled from multiple components to provide consistent, LLM-agnostic behavior.

---

## Prompt Assembly Order

1. **Core Identity** (from core/AGENTS.md)
2. **Values & Ethics** (from core/SOUL.md)
3. **Tool Integration** (from core/TOOLS.md)
4. **Bootstrap Context** (from core/BOOTSTRAP.md)
5. **Active Skills** (from skills/*/SKILL.md)
6. **Reasoning Mode** (from housaky/prompts/*.md based on task type)
7. **Current Context** (from memory and workspace files)

---

## Template

### Section 1: Identity
{{INCLUDE: core/AGENTS.md}}

### Section 2: Values
{{INCLUDE: core/SOUL.md}}

### Section 3: Capabilities
<capabilities>
{{DYNAMIC: list_available_tools}}
{{DYNAMIC: list_available_skills}}
</capabilities>

### Section 4: Tool Usage
{{INCLUDE: core/TOOLS.md}}

### Section 5: Context
{{INCLUDE: core/BOOTSTRAP.md}}

### Section 6: Current State
<current_state>
**Time**: {{DYNAMIC: current_datetime}}
**Workspace**: {{DYNAMIC: workspace_dir}}
**Active Goals**: {{DYNAMIC: active_goals}}
**Recent Context**: {{DYNAMIC: recent_memory}}
</current_state>

### Section 7: Task-Specific Reasoning
{{DYNAMIC: reasoning_mode_prompt}}

---

## Provider-Specific Adaptations

### OpenAI / Compatible
- Use native function calling for tools
- System message as separate message

### Anthropic
- Use XML-style section markers
- System prompt in first message

### Gemini
- Use markdown sections
- System context in user message prefix

### Open Source / Local
- Explicit instruction following
- May need more structured format

---

## Prompt Variables

| Variable | Source | Description |
|----------|--------|-------------|
| {{INCLUDE:path}} | File | Include file contents |
| {{DYNAMIC:name}} | Runtime | Inject runtime value |
| {{CONDITIONAL:cond}} | Logic | Include if condition met |
| {{LOOP:items}} | Array | Iterate over items |

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 2.0.0 | 2026-02-24 | Modular architecture, LLM-agnostic design |
| 1.0.0 | Initial | Basic system prompt |

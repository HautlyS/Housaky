# Housaky Prompt Architecture

> Version: 1.0.0 | Last Modified: 2026-02-24 | Initial Release

## Design Principles
1. **LLM-Agnostic**: Patterns work across all providers
2. **Composable**: Sections can be combined as needed
3. **Versioned**: Track prompt changes over time
4. **Measurable**: Evaluate prompt effectiveness

## Section Types
- **Identity**: Who the agent is
- **Values**: Ethical constraints
- **Capabilities**: What the agent can do
- **Tools**: How to use available tools
- **Context**: Current state and history
- **Instructions**: Task-specific guidance

## Prompt Assembly
```
<SystemPrompt>
  <Identity>[from AGENTS.md]</Identity>
  <Values>[from SOUL.md]</Values>
  <Capabilities>[from IDENTITY.md if exists]</Capabilities>
  <Tools>[from available tools]</Tools>
  <Context>[from memory and workspace]</Context>
  <Instructions>[task-specific]</Instructions>
</SystemPrompt>
```

## Provider Adaptations
Different providers may need format tweaks:
- OpenAI: Native function calling for tools
- Anthropic: XML-style prompt sections
- Gemini: Markdown sections
- Open Source: Explicit instruction following

## Version Control
Each prompt file includes:
- Version number in header
- Last modified date
- Change summary

## Prompt Component Reference

### Identity Section
```
Source: core/AGENTS.md
Purpose: Define agent persona and behaviors
Token Budget: ~500 tokens
Dependencies: None
```

### Values Section
```
Source: core/SOUL.md
Purpose: Ethical constraints and decision framework
Token Budget: ~400 tokens
Dependencies: None
```

### Capabilities Section
```
Source: Project-specific IDENTITY.md (optional)
Purpose: Define project-specific capabilities
Token Budget: Variable
Dependencies: Project context
```

### Tools Section
```
Source: core/TOOLS.md + runtime tool definitions
Purpose: Tool usage patterns and available tools
Token Budget: ~1000 tokens + tool schemas
Dependencies: Tool availability
```

### Context Section
```
Source: BOOTSTRAP.md + runtime context
Purpose: Session state and relevant history
Token Budget: ~2000 tokens (compacted as needed)
Dependencies: Memory system, project files
```

### Instructions Section
```
Source: User input + task decomposition
Purpose: Specific task to accomplish
Token Budget: Variable
Dependencies: User request
```

## Assembly Patterns

### Minimal Assembly
For simple tasks:
```
Identity + Values + Tools + Instructions
```

### Standard Assembly
For typical tasks:
```
Identity + Values + Tools + Context + Instructions
```

### Full Assembly
For complex tasks:
```
Identity + Values + Capabilities + Tools + Context + Instructions
```

### Context-Heavy Assembly
For continuation tasks:
```
Identity + Values + Tools + [Extended Context] + Instructions
```

## Provider-Specific Formats

### OpenAI Format
```json
{
  "messages": [
    {"role": "system", "content": "<assembled_prompt>"},
    {"role": "user", "content": "<instructions>"}
  ],
  "tools": [<tool_schemas>],
  "tool_choice": "auto"
}
```

### Anthropic Format
```xml
<system>
  <identity>...</identity>
  <values>...</values>
  <tools>...</tools>
  <context>...</context>
</system>

<user>
  <instructions>...</instructions>
</user>
```

### Gemini Format
```markdown
# System Instructions

## Identity
...

## Values
...

## Tools
...

## Context
...

---

# User Request
...
```

### Open Source / Generic Format
```
You are Housaky, an autonomous AI assistant.

## Identity
[Identity content]

## Values
[Values content]

## Tools
[Tool definitions]

## Context
[Context content]

## Instructions
[User request]

Respond according to your identity and values. Use tools when appropriate.
```

## Prompt Optimization Strategies

### Token Reduction
```
Techniques:
  1. Remove redundant content
  2. Compress verbose explanations
  3. Use references over repetition
  4. Defer details to tool-time retrieval
```

### Effectiveness Measurement
```
Metrics:
  1. Task completion rate
  2. Tool usage accuracy
  3. Error recovery success
  4. User satisfaction (implicit signals)
```

### Iteration Process
```
1. Identify failure cases
2. Analyze root cause
3. Modify prompt section
4. A/B test if possible
5. Deploy improvement
```

## Dynamic Prompt Assembly

### Runtime Decisions
```
If task is simple:
  Use minimal assembly
If context is large:
  Use context-heavy with compaction
If project has specific capabilities:
  Include capabilities section
If tools are limited:
  Simplify tool section
```

### Context Budget Management
```
Total Budget: Provider-dependent
Allocation:
  - System prompt: 20%
  - Tools: 10%
  - Context: 40%
  - User input: 20%
  - Response headroom: 10%
```

## Prompt Version History

### Versioning Scheme
```
MAJOR.MINOR.PATCH
- MAJOR: Breaking changes to prompt structure
- MINOR: Significant content changes
- PATCH: Minor improvements and fixes
```

### Change Documentation
```
For each version:
  - Date of change
  - Sections affected
  - Rationale for change
  - Expected impact
  - Rollback plan if needed
```

## Testing and Validation

### Prompt Testing Checklist
```
[ ] Identity section loads correctly
[ ] Values constraints are respected
[ ] Tool instructions are clear
[ ] Context integration works
[ ] Provider formats render correctly
[ ] Token budgets are respected
```

### Validation Criteria
```
- Prompt is parseable
- All references resolve
- Token count within budget
- No conflicting instructions
- Provider format compliance
```

## Future Extensions

### Planned Improvements
```
1. Automatic prompt optimization
2. Dynamic section loading
3. Context relevance scoring
4. Multi-language support
5. Custom persona variants
```

### Extension Points
```
- Custom identity variants
- Project-specific values
- Specialized tool guides
- Domain-specific context patterns
```

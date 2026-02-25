# ReAct (Reasoning + Acting) Framework

## Purpose
Interleave reasoning and tool use for tasks requiring external information or actions.

## Activation
Use for: research, multi-tool tasks, iterative problem solving, information gathering

## ReAct Cycle
1. **Thought**: Analyze current state, determine next action
2. **Action**: Execute tool or make observation
3. **Observation**: Process result
4. **Repeat** until goal achieved

## Protocol
```
**Thought N**: What I need to determine next and why
**Action N**: [tool_name: arguments]
**Observation N**: [result interpretation, not raw output]

**Thought N+1**: Based on observation, what next...
```

## Decision Points
After each observation:
- Goal achieved? → Provide final answer
- Need more info? → Plan next action
- Stuck? → Try alternative approach
- Error? → Diagnose and retry

## Error Recovery
```
**Observation**: Error: [error message]
**Thought**: The error indicates [diagnosis]. I will [recovery action].
**Action**: [corrected or alternative action]
```

## Final Answer Format
```
## Final Answer
[complete response to original query]

## Actions Taken
1. [action]: [brief result]
2. [action]: [brief result]

## Confidence: [0-1]
```

## Best Practices
- One action per cycle
- Verify observations before concluding
- Track progress toward goal
- Know when to ask for help

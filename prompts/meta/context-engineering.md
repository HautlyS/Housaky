# Housaky Context Engineering

> Version: 1.0.0 | Last Modified: 2026-02-24 | Initial Release

## Context Budget Management
- Total context: Provider-dependent (4k-200k tokens)
- System prompt: ~2k tokens
- Tools: ~1k tokens
- Memory: ~2k tokens
- User input: Variable
- Response headroom: 30% of total

## Context Compaction
When approaching limits:
1. Summarize conversation history
2. Retain only critical context
3. Offload details to memory
4. Use references over repetition

## Wave Execution Pattern
For complex tasks:
1. Decompose into independent subtasks
2. Group into parallel waves
3. Execute each wave in fresh context
4. Aggregate results in main context

## Persistence Strategy
- Critical state → PROJECT.md, STATE.md
- Goals → ROADMAP.md
- Decisions → DECISIONS.md
- Knowledge → memory system

## Context Window Analysis

### Typical Allocations
```
Provider          | Total    | System | Tools | Context | User  | Response
------------------|----------|--------|-------|---------|-------|----------
OpenAI GPT-4      | 128k     | 2k     | 1k    | 80k     | 20k   | 25k
Claude 3          | 200k     | 2k     | 1k    | 150k    | 20k   | 27k
Gemini Pro        | 1M       | 2k     | 1k    | 800k    | 50k   | 147k
Open Source       | 4k-32k   | 1k     | 500   | 1k-20k  | 2k    | 1k-10k
```

### Budget Calculation
```
available_context = total_context - system_prompt - tools - response_headroom
usable_context = available_context * 0.8  // Safety margin
user_input_budget = usable_context * 0.3
history_budget = usable_context * 0.5
working_budget = usable_context * 0.2
```

## Context Compaction Strategies

### Truncation Strategy
```
When to use: Simple conversations, low complexity
Process:
  1. Keep most recent N messages
  2. Preserve system context
  3. Drop oldest interactions
Trade-off: Lose early context
```

### Summarization Strategy
```
When to use: Complex conversations, important history
Process:
  1. Identify key information in old context
  2. Generate summary of early conversation
  3. Replace detailed history with summary
  4. Keep recent detailed context
Trade-off: Information loss in summary
```

### Hierarchical Strategy
```
When to use: Long-running projects with deep context
Process:
  1. Organize context into layers
  2. High-level: Project overview, goals
  3. Mid-level: Recent sessions, decisions
  4. Low-level: Current conversation
  5. Compact lower-priority layers first
Trade-off: Complexity in management
```

### Offload Strategy
```
When to use: Information that can be retrieved later
Process:
  1. Identify retrievable information
  2. Store in external memory/files
  3. Replace with retrieval pointer
  4. Retrieve on-demand when needed
Trade-off: Retrieval overhead
```

## Wave Execution Pattern

### Task Decomposition
```
Complex Task
├── Wave 1 (Independent)
│   ├── Subtask A
│   ├── Subtask B
│   └── Subtask C
├── Wave 2 (Depends on Wave 1)
│   ├── Subtask D (needs A, B)
│   └── Subtask E (needs C)
└── Wave 3 (Final)
    └── Subtask F (aggregates all)
```

### Wave Execution
```
For each wave:
  1. Initialize fresh context window
  2. Load minimal required context
  3. Execute subtasks in parallel if independent
  4. Capture results efficiently
  5. Pass results to next wave
```

### Context Isolation
```
Benefits:
  - Each wave has full context budget
  - Errors isolated to wave
  - Parallel execution possible
  - Clear checkpoint boundaries

Considerations:
  - Handoff between waves
  - State persistence
  - Error propagation
```

## Context Hierarchy

### Level 1: Persistent Context
```
Storage: Files (PROJECT.md, ROADMAP.md, etc.)
Lifetime: Project lifetime
Size: Unlimited (loaded on demand)
Priority: Critical for continuity
```

### Level 2: Session Context
```
Storage: STATE.md, memory
Lifetime: Session duration
Size: Moderate
Priority: High for current work
```

### Level 3: Working Context
```
Storage: In-context (token window)
Lifetime: Current interaction
Size: Limited by context window
Priority: Immediate relevance
```

### Level 4: Transient Context
```
Storage: In-context
Lifetime: Single operation
Size: Minimal
Priority: Ephemeral
```

## Context Loading Priority

### Startup Load Order
```
1. AGENTS.md (identity)
2. SOUL.md (values)
3. PROJECT.md (project context)
4. ROADMAP.md (goals)
5. STATE.md (current state)
6. DECISIONS.md (key decisions)
7. Memory (relevant items)
```

### Lazy Loading
```
Defer loading until needed:
  - Detailed file contents
  - Historical conversations
  - Reference documentation
  - Large data structures

Load on demand:
  - When task requires
  - When context available
  - With pagination if large
```

## Context Relevance Scoring

### Scoring Factors
```
- Recency: How recently was information used
- Frequency: How often is information referenced
- Importance: Critical to current goals
- Dependencies: Required for pending tasks
- Uniqueness: Cannot be easily reconstructed
```

### Scoring Algorithm
```
relevance_score = (
  recency_weight * recency_factor +
  frequency_weight * frequency_factor +
  importance_weight * importance_factor +
  dependency_weight * dependency_factor +
  uniqueness_weight * uniqueness_factor
)

Keep if relevance_score > threshold
Compaction priority inversely proportional to score
```

## Context Synchronization

### File Synchronization
```
Read phase:
  1. Check file modification time
  2. Compare to cached version
  3. Reload if changed
  4. Update cache

Write phase:
  1. Validate changes
  2. Write atomically
  3. Update modification time
  4. Invalidate related caches
```

### Memory Synchronization
```
Store:
  - Key information immediately
  - Batch updates periodically
  - On session end (critical items)

Recall:
  - At session start
  - Before complex operations
  - When context is missing
```

## Context Recovery

### Corruption Detection
```
Symptoms:
  - Inconsistent references
  - Missing required context
  - Out-of-order timestamps
  - Invalid state transitions
```

### Recovery Procedures
```
Minor corruption:
  1. Identify affected section
  2. Reload from source
  3. Verify consistency
  4. Continue operation

Major corruption:
  1. Identify extent of damage
  2. Reconstruct from available sources
  3. Prompt user for missing info
  4. Rebuild context systematically
```

## Best Practices

### Context Hygiene
```
DO:
  - Keep context organized
  - Remove stale information
  - Update timestamps
  - Maintain consistent formats

DON'T:
  - Duplicate information
  - Keep outdated state
  - Mix unrelated contexts
  - Skip validation
```

### Performance Optimization
```
- Batch reads when possible
- Use caching strategically
- Preload predictable needs
- Parallelize independent loads
```

### Quality Assurance
```
- Validate context after loading
- Check for completeness
- Verify consistency
- Test edge cases
```

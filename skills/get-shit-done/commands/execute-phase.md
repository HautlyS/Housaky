# /gsd:execute-phase

Execute all plans in parallel waves with fresh agent contexts.

## Usage
```
/gsd:execute-phase [N]
/gsd:execute-phase        # Executes current phase
/gsd:execute-phase 2      # Executes phase 2
```

## Wave Execution Model

### Concept
Execute independent tasks in parallel "waves" where each task gets a fresh agent with ~200k tokens of context. This maximizes parallelism while keeping context budgets clean.

### Wave Structure
```
Wave 0: [Task A] [Task B] [Task C]     ← All independent, parallel execution
         ↓       ↓       ↓
         ✓       ✓       ✓             ← Results collected
         └───────┴───────┘
                 ↓
Wave 1: [Task D] [Task E]              ← Depends on Wave 0, parallel
         ↓       ↓
         ✓       ✓
         └───────┘
             ↓
Wave 2: [Task F]                        ← Depends on Wave 1
         ↓
         ✓
```

### Per-Task Context
Each spawned agent receives:
- Task specification (XML from PLAN.md)
- Required file context (minimal)
- Fresh ~200k token budget
- Isolated execution environment

## Execution Workflow

### 1. Pre-Execution
```
Load PLAN.md
Parse wave structure
Verify task dependencies satisfied
Prepare execution order
```

### 2. Wave Execution
For each wave:
```
Foreach task in wave (parallel):
  Spawn fresh agent
  Load task spec
  Load minimal file context
  Execute implementation
  Run verification
  Commit if verified
  Return result

Collect all results
Verify wave complete
Proceed to next wave
```

### 3. Post-Execution
```
Aggregate all results
Update STATE.md
Run full test suite
Generate execution report
```

## Task Execution Flow

### Agent Spawn
```yaml
context:
  task: phase-N-task-M
  spec: <task>...</task>
  files: [minimal set]
  budget: 200000 tokens

instructions: |
  1. Read task specification
  2. Implement according to <action>
  3. Run verification command
  4. If verified, commit changes
  5. Return completion status
```

### Result Collection
```yaml
task_result:
  id: phase-N-task-M
  status: success | failure | partial
  files_modified: [...]
  verification: passed | failed
  commit: sha | null
  notes: ...
  errors: [...]
```

## Parallel Execution Rules

### Maximum Parallelism
- Execute all Wave 0 tasks simultaneously
- Each task in separate agent instance
- No shared state between tasks
- Collect results before next wave

### Context Isolation
- Never share agent memory between tasks
- Each task loads only its needed files
- Output goes to designated files
- State persisted to filesystem

### Failure Handling
```
If task fails:
  1. Log error details
  2. Mark task as failed
  3. Continue other wave tasks
  4. Report all failures at wave end
  5. Allow retry or manual fix
```

## Execution Report

Generated after phase execution:
```markdown
# Phase N Execution Report

## Summary
- Total tasks: X
- Successful: Y
- Failed: Z
- Duration: [time]

## Wave Results

### Wave 0
| Task | Status | Files | Commit |
|------|--------|-------|--------|
| 0.1  | ✓      | 3     | abc123 |
| 0.2  | ✓      | 2     | def456 |
| 0.3  | ✗      | 0     | -      |

### Wave 1
...

## Failures
### Task 0.3
**Error**: [description]
**Resolution**: [manual steps needed]

## Next Steps
1. Fix Task 0.3
2. Retry Wave 1
3. Continue to Phase N+1
```

## Output
- Execution report in `.planning/execution-report.md`
- Updated STATE.md with progress
- Commits for completed tasks

## Example
```
/gsd:execute-phase 1
```

Spawns parallel agents for each Wave 0 task, collects results, proceeds through waves until phase complete.

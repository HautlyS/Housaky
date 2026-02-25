---
name: get-shit-done
version: 1.0.0
description: Meta-prompting and context engineering for spec-driven development
triggers:
  actions: [plan, execute, debug, verify, ship, deploy, complete]
  contexts: [new project, phase planning, task execution, debugging]
  commands: [/gsd:new-project, /gsd:plan-phase, /gsd:execute-phase, /gsd:debug]
---

# Get-Shit-Done (GSD) Skill

Meta-prompting and context engineering framework for spec-driven development.

## Core Concepts

### 1. Context Engineering
Token budgets define the boundaries of AI reasoning capacity. GSD optimizes context usage through:
- **Budget Allocation**: Reserve ~30% for reasoning, ~70% for code/outputs
- **Context Pruning**: Load only essential files per task
- **State Persistence**: Externalize memory to filesystem
- **Fresh Starts**: New agent instances for each major task

### 2. Wave Execution
Parallel task execution with fresh contexts:
- **Wave 0**: Independent foundational tasks
- **Wave N**: Tasks depending on Wave N-1
- **Parallel Agents**: Each task gets ~200k token context
- **Result Collection**: Aggregate outputs before next wave

### 3. Goal-Backward Planning
Plan from outcomes to actions:
- Define success criteria first
- Work backward to required artifacts
- Identify dependencies between artifacts
- Group into execution waves

### 4. State Persistence
File-based state management:
- PROJECT.md: Project definition and constraints
- STATE.md: Current progress and decisions
- ROADMAP.md: Phase breakdown and timeline
- PLAN.md: Task-level execution plans

## Workflow Pattern

```
discuss → plan → execute → verify → repeat
```

### Discuss Phase
- Capture user requirements
- Ask clarifying questions
- Document decisions
- Update STATE.md

### Plan Phase
- Load context from planning files
- Break down into atomic tasks
- Identify dependencies
- Create PLAN.md with XML tasks

### Execute Phase
- Group tasks into waves
- Spawn parallel agents per wave
- Collect results
- Update STATE.md

### Verify Phase
- Run automated tests
- Check against done criteria
- User acceptance
- Document issues

## Context Budgeting Techniques

### Per-Task Budget
| Component | Allocation | Purpose |
|-----------|------------|---------|
| Task Spec | 5-10% | Understanding what to build |
| Code Context | 20-30% | Files to modify |
| Reasoning | 30% | Solution design |
| Output | 40-50% | Generated code |

### Context Loading Strategy
1. Start with task spec only
2. Load only directly modified files
3. Lazy-load dependencies on demand
4. Never load entire codebase

### State File Sizes
- PROJECT.md: < 500 lines
- STATE.md: < 300 lines
- ROADMAP.md: < 200 lines
- PLAN.md: < 100 lines per phase

## Slash Commands

| Command | Purpose |
|---------|---------|
| /gsd:new-project | Initialize project planning |
| /gsd:plan-phase | Create execution plan |
| /gsd:execute-phase | Run parallel execution |
| /gsd:debug | Systematic debugging |

## Agents

| Agent | Role |
|-------|------|
| gsd-planner | Creates execution plans |
| gsd-executor | Implements tasks |
| gsd-debugger | Diagnoses issues |

## Directory Structure

```
.planning/
├── PROJECT.md      # Project definition
├── STATE.md        # Current state
├── ROADMAP.md      # Phase roadmap
├── PLAN.md         # Current phase plan
├── config.json     # Configuration
└── debug/          # Debug sessions
    └── session-*/
        ├── STATE.md
        ├── HYPOTHESES.md
        └── SOLUTION.md
```

## Best Practices

1. **Atomic Tasks**: Each task should be completable in one agent session
2. **Clear Verification**: Every task has automated verification
3. **Minimal Context**: Load only what's needed
4. **Fresh Starts**: Don't cascade context between tasks
5. **Persist Everything**: State belongs in files, not memory

# /gsd:plan-phase

Plan a specific development phase with structured, executable tasks.

## Usage
```
/gsd:plan-phase [N]
/gsd:plan-phase        # Plans current phase from STATE.md
/gsd:plan-phase 2      # Plans phase 2 specifically
```

## Workflow

### 1. Context Loading
Load planning context:
- Read PROJECT.md for constraints
- Read ROADMAP.md for phase definition
- Read STATE.md for current progress
- Read previous PLAN.md for continuation

### 2. Research Phase
Research implementation approaches:
- Search for relevant patterns in codebase
- Research best practices for task types
- Identify potential challenges
- Document alternatives considered

### 3. Planning Phase
Create structured execution plan:

**Task Breakdown Rules**
- Maximum 3 tasks per plan (context budget)
- Each task atomic and completable in one session
- Clear file scope (minimal context needed)
- Independent tasks grouped for parallel execution

**Dependency Analysis**
- Identify task dependencies
- Group into execution waves
- Document wave requirements

### 4. Verification Planning
For each task, define:
- Automated test command
- Manual verification steps
- Done criteria checklist

## Task XML Format

```xml
<task type="auto|manual" priority="high|medium|low">
  <id>phase-N-task-M</id>
  <name>Descriptive task name</name>
  <description>Brief description of what to implement</description>
  
  <context>
    <files>file1.ts,file2.ts,dir/file3.ts</files>
    <dependencies>task-id-1,task-id-2</dependencies>
    <estimated_tokens>50000</estimated_tokens>
  </context>
  
  <implementation>
    <action>
      Step-by-step implementation instructions.
      Be specific about what to create/modify.
    </action>
    <approach>Suggested implementation approach</approach>
    <pitfalls>Common mistakes to avoid</pitfalls>
  </implementation>
  
  <verification>
    <test_command>npm test -- --grep "feature"</test_command>
    <manual_steps>
      1. Start the server
      2. Navigate to /feature
      3. Verify behavior
    </manual_steps>
    <done_criteria>
      - [ ] Tests pass
      - [ ] Feature works as expected
      - [ ] No console errors
    </done_criteria>
  </verification>
</task>
```

## PLAN.md Structure

```markdown
# Phase N: [Name] - Execution Plan

## Overview
[Brief phase description and goals]

## Context Budget
- Total allocated: 200,000 tokens
- Per task: ~50,000 tokens
- Reserved for output: 40%

## Wave 0: Foundation
### Task 0.1: [Name]
```xml
<task type="auto">
  ...
</task>
```

### Task 0.2: [Name]
```xml
<task type="auto">
  ...
</task>
```

## Wave 1: Dependent Tasks
### Task 1.1: [Name]
...

## Execution Order
1. Execute Wave 0 tasks in parallel
2. Verify all Wave 0 tasks complete
3. Execute Wave 1 tasks
4. Verify phase complete

## Success Criteria
- [ ] All tasks verified
- [ ] Phase goals met
- [ ] STATE.md updated
```

## Output
- `.planning/PLAN.md` - Execution plan with XML tasks

## Example
```
/gsd:plan-phase 1
```

Generates PLAN.md with tasks for Phase 1 implementation.

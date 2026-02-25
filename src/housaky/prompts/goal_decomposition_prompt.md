# Goal Decomposition System

## Purpose
Break complex goals into executable subtasks with dependencies.

## Activation
Use for: complex goals, multi-step objectives, project planning

## Decomposition Protocol
1. **Analyze Goal**
   - State the success criteria
   - Identify constraints
   - List resources needed

2. **Generate Subtasks**
   Each subtask should be:
   - Atomic: Single clear action
   - Verifiable: Has clear completion criteria
   - Bounded: Limited scope and time

3. **Map Dependencies**
   - Identify sequential dependencies
   - Find parallelizable tasks
   - Detect potential blockers

4. **Order and Group**
   - Create execution order
   - Group into waves for parallel execution
   - Set checkpoints for progress tracking

## Subtask Format
```
<subtask id="N">
  <name>[task name]</name>
  <description>[what to do]</description>
  <dependencies>[list of subtask IDs]</dependencies>
  <verification>[how to verify completion]</verification>
  <estimated_effort>[low/medium/high]</estimated_effort>
</subtask>
```

## Output Format
```
## Goal: [statement]
Success Criteria: [measurable outcomes]
Constraints: [limitations]

## Decomposition

### Wave 1 (Parallel)
- Subtask 1: [name] → verifies with [check]
- Subtask 2: [name] → verifies with [check]

### Wave 2 (After Wave 1)
- Subtask 3: [name] (depends on 1, 2) → verifies with [check]

### Wave 3 (After Wave 2)
- Subtask 4: [name] (depends on 3) → verifies with [check]

## Execution Plan
1. Execute Wave 1 in parallel
2. Verify Wave 1 completion
3. Execute Wave 2
4. Continue until complete

## Risk: [potential issues]
## Mitigation: [contingency plans]
```

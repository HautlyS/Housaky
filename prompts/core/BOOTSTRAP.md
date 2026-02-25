# Housaky Bootstrap Context

> Version: 1.0.0 | Last Modified: 2026-02-24 | Initial Release

## Session Initialization
When starting a session:
1. Load memory context
2. Review active goals
3. Check for pending tasks
4. Acknowledge previous context

## Context Structure
```
## Previous Context
- Last session: [timestamp]
- Active goals: [list]
- Recent work: [summary]
- Open items: [list]

## Current Session
- User request: [input]
- Context loaded: [yes/no]
- Tools available: [count]
```

## Continuity Patterns
- Reference previous work when relevant
- Maintain consistent formatting preferences
- Preserve project-specific conventions
- Track progress on multi-session tasks

## Bootstrap Sequence

### Phase 1: Environment Scan
```
Actions:
  1. Identify working directory
  2. List available tools
  3. Check for existing context files
  4. Assess system capabilities
Output: Environment summary
```

### Phase 2: Context Loading
```
Priority Order:
  1. PROJECT.md (project context)
  2. ROADMAP.md (goals and milestones)
  3. STATE.md (current state)
  4. DECISIONS.md (key decisions)
  5. Memory system (accumulated knowledge)
Output: Loaded context summary
```

### Phase 3: Goal Alignment
```
Actions:
  1. Review stated goals
  2. Assess current progress
  3. Identify blockers
  4. Prioritize next actions
Output: Actionable task list
```

### Phase 4: Ready State
```
Status Report:
  - Environment: [ready/not ready]
  - Context: [loaded/partial/missing]
  - Goals: [clear/unclear/none]
  - Ready for input: [yes/no]
```

## Context Files Reference

### PROJECT.md
```
Purpose: Project-level context
Contents:
  - Project name and description
  - Technology stack
  - Architecture overview
  - Key conventions
  - Dependencies
Update Frequency: When project structure changes
```

### ROADMAP.md
```
Purpose: Goals and milestones
Contents:
  - Primary objectives
  - Milestone timeline
  - Current phase
  - Next priorities
Update Frequency: When goals change or milestones reached
```

### STATE.md
```
Purpose: Current operational state
Contents:
  - Active tasks
  - In-progress work
  - Blocked items
  - Recent changes
Update Frequency: Each session, after significant work
```

### DECISIONS.md
```
Purpose: Architecture and design decisions
Contents:
  - Decision records (ADR format)
  - Rationale for key choices
  - Alternatives considered
  - Trade-offs accepted
Update Frequency: When significant decisions made
```

## Session Handoff Protocol

### Ending a Session
```
1. Save current state to STATE.md
2. Update progress in ROADMAP.md
3. Store new knowledge in memory
4. Document any blockers
5. Prepare continuation hints
```

### Starting a Session
```
1. Read STATE.md for continuation
2. Check for unresolved blockers
3. Review recent decisions
4. Load relevant memory
5. Acknowledge context to user
```

## Context Validation

### Integrity Checks
```
- Are context files present?
- Is content parseable?
- Are timestamps reasonable?
- Are references valid?
```

### Recovery Actions
```
If context missing:
  - Acknowledge fresh start
  - Prompt for project context
  - Rebuild from available sources

If context corrupted:
  - Attempt partial recovery
  - Identify missing pieces
  - Request user guidance

If context outdated:
  - Note last known state
  - Offer to refresh
  - Proceed with available context
```

## Memory Integration

### What to Store
- Important discoveries
- Recurring patterns
- User preferences
- Project-specific knowledge
- Failed approaches to avoid

### What Not to Store
- Temporary calculations
- Redundant information
- Sensitive credentials
- Outdated configurations

### Retrieval Strategy
```
Query Formation:
  - Use specific keywords
  - Include context hints
  - Request relevant timeframe

Result Processing:
  - Rank by relevance
  - Combine with current context
  - Validate applicability
  - Update if needed
```

## Multi-Session Task Tracking

### Task States
```
- Pending: Not yet started
- In Progress: Currently working
- Blocked: Cannot proceed (reason documented)
- Review: Awaiting user review
- Complete: Finished and verified
- Deferred: Postponed (with justification)
```

### Progress Documentation
```
For each active task:
  - Task ID and description
  - Current state
  - Progress percentage (estimate)
  - Blockers if any
  - Next steps
  - Dependencies
```

## Bootstrap Checklist

### Quick Start
```
[ ] Working directory identified
[ ] Tools enumerated
[ ] Context files located
[ ] Previous state loaded
[ ] Goals understood
[ ] Ready for user input
```

### Full Bootstrap
```
[ ] Environment scan complete
[ ] All context files loaded
[ ] Memory recalled
[ ] Goals prioritized
[ ] Blockers identified
[ ] Session state initialized
[ ] Status communicated to user
```

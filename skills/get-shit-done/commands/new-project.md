# /gsd:new-project

Initialize a new project with structured planning.

## Workflow

### 1. Questioning Phase
Ask clarifying questions about:

**Project Goals**
- What problem does this solve?
- What does success look like?
- Who are the users?
- What are the core features?

**Technical Constraints**
- Preferred tech stack?
- Performance requirements?
- Security requirements?
- Integration needs?

**Timeline & Milestones**
- Target completion date?
- Key milestones?
- Priority ordering?

### 2. Research Phase
Research and document:

**Similar Projects**
- Existing solutions in the domain
- Patterns that work well
- Anti-patterns to avoid

**Best Practices**
- Framework conventions
- Testing strategies
- Deployment patterns

**Technology Recommendations**
- Library choices
- Architecture patterns
- Tool recommendations

### 3. Requirements Phase
Generate structured planning files:

**PROJECT.md**
```markdown
# Project: [Name]

## Vision
[One paragraph vision statement]

## Goals
- Goal 1
- Goal 2

## Success Criteria
- [ ] Criterion 1
- [ ] Criterion 2

## Constraints
- Technical: [constraints]
- Timeline: [constraints]
- Resources: [constraints]

## Tech Stack
- Runtime: [choice]
- Framework: [choice]
- Database: [choice]
- Testing: [choice]

## Out of Scope
- [Explicit exclusions]
```

**STATE.md**
```markdown
# Project State

## Current Phase
Setup

## Progress
- [x] Project initialized
- [ ] Phase 1 planning

## Decisions
| Date | Decision | Rationale |
|------|----------|-----------|
| YYYY-MM-DD | [Decision] | [Why] |

## Blockers
- None currently

## Next Actions
1. Begin Phase 1 planning
```

**ROADMAP.md**
```markdown
# Project Roadmap

## Phase 0: Setup
- Project scaffolding
- Development environment
- CI/CD pipeline

## Phase 1: [Name]
**Duration**: [Estimate]
**Goal**: [Phase goal]
**Deliverables**:
- [ ] Deliverable 1
- [ ] Deliverable 2

## Phase 2: [Name]
...

## Dependencies
```
Phase 0 → Phase 1 → Phase 2 → ...
```
```

**config.json**
```json
{
  "project": "[name]",
  "created": "[timestamp]",
  "phases": 0,
  "currentPhase": 0,
  "settings": {
    "autoVerify": true,
    "commitOnComplete": true,
    "maxTasksPerWave": 3
  }
}
```

## Output Files
- `.planning/PROJECT.md` - Project definition
- `.planning/STATE.md` - Current state
- `.planning/ROADMAP.md` - Phase roadmap
- `.planning/config.json` - Configuration

## Usage
```
/gsd:new-project
```

The command initiates an interactive session to gather requirements and generate planning artifacts.

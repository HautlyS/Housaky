# Standard GSD Workflow

Complete workflow for spec-driven development with context engineering.

## Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    GSD WORKFLOW                              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Phase 0: Setup                                              │
│  ┌──────────────┐                                            │
│  │ /new-project │ → PROJECT.md, STATE.md, ROADMAP.md        │
│  └──────────────┘                                            │
│         │                                                    │
│         ▼                                                    │
│  Phase N: Development Cycle                                   │
│  ┌───────────────┐                                           │
│  │ /discuss-phase│ → Decisions captured                      │
│  └───────────────┘                                           │
│         │                                                    │
│         ▼                                                    │
│  ┌───────────────┐                                           │
│  │ /plan-phase   │ → PLAN.md with tasks                      │
│  └───────────────┘                                           │
│         │                                                    │
│         ▼                                                    │
│  ┌─────────────────┐                                         │
│  │ /execute-phase  │ → Parallel task execution               │
│  └─────────────────┘                                         │
│         │                                                    │
│         ▼                                                    │
│  ┌───────────────┐                                           │
│  │ /verify-work  │ → User acceptance                         │
│  └───────────────┘                                           │
│         │                                                    │
│         ▼                                                    │
│     Complete? ──No──► Repeat Phase N                         │
│         │                                                    │
│        Yes                                                   │
│         │                                                    │
│         ▼                                                    │
│  ┌─────────────────┐                                         │
│  │ /complete-mile  │ → Archive, tag, next phase              │
│  └─────────────────┘                                         │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Phase 0: Project Setup

### Command
```
/gsd:new-project
```

### Activities
1. **Questioning Phase**
   - Gather project goals
   - Understand constraints
   - Define success criteria
   - Identify stakeholders

2. **Research Phase**
   - Analyze similar projects
   - Research best practices
   - Evaluate technology options
   - Identify risks

3. **Planning Phase**
   - Generate PROJECT.md
   - Initialize STATE.md
   - Create ROADMAP.md
   - Configure settings

### Output Files
```
.planning/
├── PROJECT.md    # Project definition
├── STATE.md      # Current state
├── ROADMAP.md    # Phase breakdown
└── config.json   # Configuration
```

### Success Criteria
- [ ] PROJECT.md captures vision
- [ ] ROADMAP.md has phases defined
- [ ] STATE.md shows Phase 0 complete
- [ ] config.json initialized

---

## Phase N: Development Cycle

### Step 1: Discussion
```
/gsd:discuss-phase N
```

**Activities**:
- Review phase goals from ROADMAP.md
- Discuss implementation approach
- Answer clarifying questions
- Make technical decisions

**Output**:
- Updated STATE.md with decisions
- Clarified phase requirements

### Step 2: Planning
```
/gsd:plan-phase N
```

**Activities**:
- Load context from planning files
- Research implementation patterns
- Break into atomic tasks
- Group into execution waves
- Define verification criteria

**Output**:
- `.planning/PLAN.md` with XML tasks

**Quality Gates**:
- [ ] Tasks are atomic
- [ ] Dependencies identified
- [ ] Waves properly grouped
- [ ] Verification defined

### Step 3: Execution
```
/gsd:execute-phase N
```

**Activities**:
- Parse PLAN.md waves
- Execute Wave 0 tasks in parallel
- Collect results
- Execute subsequent waves
- Update STATE.md

**Output**:
- Implementation complete
- Tests passing
- Commits created
- Execution report

**Quality Gates**:
- [ ] All tasks executed
- [ ] Verification commands pass
- [ ] No failing tests
- [ ] Code committed

### Step 4: Verification
```
/gsd:verify-work N
```

**Activities**:
- Run full test suite
- Manual verification of features
- User acceptance testing
- Document any issues

**Output**:
- Verification report
- Issues list (if any)
- User sign-off

**Quality Gates**:
- [ ] All tests pass
- [ ] Features work as expected
- [ ] User accepts deliverable
- [ ] No regressions

---

## Completion

### Command
```
/gsd:complete-milestone
```

### Activities
1. **Archive Phase**
   - Move PLAN.md to archive
   - Create phase summary
   - Tag git commit

2. **Update Roadmap**
   - Mark phase complete
   - Update dependencies
   - Prepare next phase

3. **Clean State**
   - Reset for next phase
   - Archive decisions
   - Clear working files

### Output
```
.planning/
├── archive/
│   └── phase-N/
│       ├── PLAN.md
│       ├── SUMMARY.md
│       └── DECISIONS.md
├── PROJECT.md     # Updated if needed
├── STATE.md       # Reset for next phase
└── ROADMAP.md     # Updated progress
```

---

## Context Management

### Per-Phase Context
Each phase starts with a fresh context:

```
┌─────────────────────────────────────┐
│         Fresh Context               │
│         (~200k tokens)              │
├─────────────────────────────────────┤
│ Load:                               │
│ • PROJECT.md (500 lines max)        │
│ • STATE.md (300 lines max)          │
│ • ROADMAP.md (200 lines max)        │
│ • PLAN.md (100 lines max)           │
├─────────────────────────────────────┤
│ Reserved:                           │
│ • Reasoning: 30%                    │
│ • Output: 40%                       │
└─────────────────────────────────────┘
```

### Context Discipline

#### DO
- Load only planning files
- Load only needed code files
- Persist state to files
- Use fresh contexts

#### DON'T
- Load entire codebase
- Share context between tasks
- Keep state in memory
- Cascade conversations

### File Size Limits

| File | Max Lines | Purpose |
|------|-----------|---------|
| PROJECT.md | 500 | Project definition |
| STATE.md | 300 | Current state |
| ROADMAP.md | 200 | Phase roadmap |
| PLAN.md | 100 | Task plan |
| Task spec | 50 | Per task |

---

## Debugging Integration

### When to Debug
- Test failures during execution
- Unexpected behavior in verification
- Production issues

### Command
```
/gsd:debug "[issue description]"
```

### Integration Points
1. **During Execution**: Auto-debug on test failure
2. **During Verification**: Debug failing features
3. **Post-Deployment**: Debug production issues

### Output
Debug sessions saved to `.planning/debug/session-[timestamp]/`

---

## Best Practices

### 1. Atomic Phases
- Each phase has clear deliverables
- Phases are independently valuable
- Dependencies are explicit

### 2. Fresh Starts
- New agent per task
- No context accumulation
- State in files, not memory

### 3. Verification First
- Define success before implementing
- Automated tests required
- Manual verification for UX

### 4. Persist Everything
- All decisions documented
- State always recoverable
- Archive for history

### 5. Parallel by Default
- Independent tasks in parallel
- Fresh context per task
- Collect results at wave end

---

## Quick Reference

| Command | Purpose | Output |
|---------|---------|--------|
| `/gsd:new-project` | Initialize project | PROJECT.md, STATE.md, ROADMAP.md |
| `/gsd:plan-phase N` | Create phase plan | PLAN.md |
| `/gsd:execute-phase N` | Execute plan | Implementation, commits |
| `/gsd:verify-work N` | Verify completion | Verification report |
| `/gsd:debug [issue]` | Debug problem | Debug session |
| `/gsd:complete-milestone` | Archive phase | Archive files, tags |

---

## Example Session

```bash
# Initialize project
/gsd:new-project
# → Answer questions about project goals
# → Creates planning files

# Plan first phase
/gsd:plan-phase 1
# → Creates PLAN.md with tasks

# Execute phase
/gsd:execute-phase 1
# → Runs tasks in waves
# → Commits changes

# Verify work
/gsd:verify-work 1
# → Run tests
# → User acceptance

# Complete and move on
/gsd:complete-milestone
# → Archive phase 1
# → Prepare phase 2
```

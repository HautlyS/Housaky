# /gsd:debug

Systematic debugging with state tracking and hypothesis testing.

## Usage
```
/gsd:debug [issue]
/gsd:debug "test failures in auth module"
/gsd:debug "TypeError in user service"
```

## Debug Workflow

### 1. Capture Current State
Document the problem state:

```markdown
# Debug Session: [timestamp]

## Problem Statement
[Clear description of the issue]

## Current State
- Branch: [git branch]
- Last working commit: [sha if known]
- Error message: [full error]
- Reproduction steps: [steps]

## System State
- Environment: [dev/staging/prod]
- Dependencies: [relevant versions]
- Configuration: [relevant config]
```

### 2. Isolate the Problem
Narrow down the issue scope:

```markdown
## Isolation

### What works
- [Working functionality]

### What doesn't work
- [Broken functionality]

### Recent changes
- Commit [sha]: [change description]

### Affected components
- Files: [list]
- Modules: [list]
- Services: [list]
```

### 3. Hypothesize Causes
Generate and document hypotheses:

```markdown
## Hypotheses

### Hypothesis 1: [Name]
**Likelihood**: High/Medium/Low
**Cause**: [Proposed cause]
**Test**: [How to verify]
**Fix**: [Proposed solution]

### Hypothesis 2: [Name]
**Likelihood**: Medium
**Cause**: [Proposed cause]
**Test**: [How to verify]
**Fix**: [Proposed solution]
```

### 4. Test Hypotheses
Execute tests in order of likelihood:

```markdown
## Hypothesis Testing

### Test 1: [Hypothesis name]
**Action**: [What was done]
**Result**: PASS/FAIL
**Evidence**: [Observations]
**Conclusion**: [What this means]

### Test 2: [Hypothesis name]
**Action**: [What was done]
**Result**: PASS/FAIL
**Evidence**: [Observations]
**Conclusion**: [What this means]
```

### 5. Document Solution
Record the fix and prevention:

```markdown
## Solution

### Root Cause
[The actual cause identified]

### Fix Applied
[What was changed to fix it]
```bash
[Commands run or code changes]
```

### Verification
- [ ] Test suite passes
- [ ] Manual verification complete
- [ ] Edge cases tested

## Prevention

### How to prevent recurrence
1. [Prevention measure 1]
2. [Prevention measure 2]

### Recommended improvements
- [Improvement 1]
- [Improvement 2]

### Test coverage
- Add test: [description of test to add]

## Lessons Learned
[Key takeaways from this debugging session]
```

## Debug Session Files

### Directory Structure
```
.planning/debug/
└── session-[timestamp]/
    ├── STATE.md          # Problem state
    ├── HYPOTHESES.md     # Hypotheses and tests
    ├── SOLUTION.md       # Fix documentation
    └── DIFF.md           # Changes made
```

### STATE.md Template
```markdown
# Debug State: [issue-name]

## Status
INVESTIGATING | TESTING | RESOLVED | ESCALATED

## Problem
[Description]

## Context
[Relevant context]

## Timeline
| Time | Action | Result |
|------|--------|--------|
| ... | ... | ... |
```

### HYPOTHESES.md Template
```markdown
# Debug Hypotheses

## Active Hypotheses
| # | Hypothesis | Likelihood | Status |
|---|------------|------------|--------|
| 1 | [Name] | High | TESTING |
| 2 | [Name] | Medium | PENDING |

## Tested Hypotheses
| # | Hypothesis | Result | Evidence |
|---|------------|--------|----------|
| 3 | [Name] | REJECTED | [Why] |
```

### SOLUTION.md Template
```markdown
# Debug Solution

## Root Cause
[Technical explanation]

## Fix
```diff
[file changes]
```

## Commands
```bash
[commands to apply fix]
```

## Verification
[How to verify the fix works]

## Prevention
[How to prevent this issue in future]
```

## Debug Strategies

### Binary Search
1. Identify midpoint in code execution
2. Verify state at midpoint
3. Narrow search based on result
4. Repeat until isolated

### State Comparison
1. Capture working state
2. Capture broken state
3. Diff the states
4. Investigate differences

### Minimal Reproduction
1. Create smallest possible reproduction
2. Remove all non-essential code
3. Verify issue persists
4. Add back components one by one

### Time Travel
1. Use git bisect for regression
2. Identify breaking commit
3. Analyze the change
4. Understand root cause

## Output
- Debug session directory in `.planning/debug/session-[timestamp]/`
- Updated STATE.md with resolution
- Commit with fix and test

## Example
```
/gsd:debug "auth tests failing after refactor"
```

Creates debug session, guides through hypothesis testing, documents solution.

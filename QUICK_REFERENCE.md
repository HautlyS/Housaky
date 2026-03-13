# Housaky Quick Reference Guide

## One-Shot Site/File Building

### Using GSD Orchestrator

```bash
# Create a new project with vision
housaky gsd new-project "My Website" "Build a modern portfolio website"

# Quick execute a task
housaky gsd quick-execute "Create a React landing page with hero section and contact form"

# Execute with LLM decomposition (recommended)
housaky gsd execute-with-llm <phase-id> "Build a full-stack blog with authentication"
```

### Task File Format (TASKS.md)

```markdown
## Active Tasks
- Build portfolio site [P1] [cat:tool]
  Create responsive design with projects gallery
  
- Add dark mode toggle [P2] [cat:intelligence]
  Implement theme switching with localStorage persistence
```

The heartbeat will automatically process tasks by category every 120 seconds.

## Subagent Delegation

### Configure Subagents in keys.json

```json
{
  "subagents": {
    "kowalski-code": {
      "provider": "modal",
      "model": "zai-org/GLM-5-FP8",
      "key_name": "glm-key-1",
      "role": "Code generation and refactoring"
    },
    "kowalski-web": {
      "provider": "openrouter",
      "model": "anthropic/claude-sonnet-4-5-20250929",
      "key_name": "openrouter-key-1",
      "role": "Web search and research"
    }
  }
}
```

### Delegate Tool Usage

When GSD is in delegate mode, it automatically routes tasks:
- Code tasks → `kowalski-code`
- Web/search tasks → `kowalski-web`
- Analysis tasks → `kowalski-reasoning`
- Creative tasks → `kowalski-creative`

## Operation Modes

### Idle Mode (Default)
Full self-improvement cycle runs every 120 seconds:
- Quantum goal scheduling
- GSD phase execution
- Cognitive cycle
- Self-improvement (intelligence, tools, connections)
- Recursive improvement loop
- Memory consolidation
- Quantum knowledge graph clustering
- AGI hub cycle

### Focus Mode
Minimal maintenance only - use when handling user requests:

```rust
// In your code
heartbeat.enter_focus_mode().await;
// ... handle user request ...
heartbeat.return_to_idle().await;
```

### Hybrid Mode
Light maintenance - skips expensive quantum operations:

```rust
heartbeat.set_operation_mode(OperationMode::Hybrid).await;
```

## Verification Gates

Tasks with verification criteria must pass verification before phase closes:

```rust
// Check if task needs verification
if task.needs_verification() {
    // Spawn verifier subagent or run verification command
    let report = orchestrator.verify_work(phase_id).await?;
}

// Check verification status
match task.verification_status() {
    VerificationStatus::Pending => { /* wait */ }
    VerificationStatus::Passed => { /* proceed */ }
    VerificationStatus::Failed(reason) => { /* fix issues */ }
}
```

## Task Dependencies

Dependencies auto-unblock when completed:

```rust
// Tasks automatically unblock dependents
orchestrator.execute_phase(phase_id).await?;

// List ready tasks (all dependencies satisfied)
let ready = wave_executor.list_ready_tasks().await;

// List blocked tasks with blockers
let blocked = wave_executor.list_blocked_tasks().await;
for (task, blockers) in blocked {
    println!("Task {} blocked by: {:?}", task.name, blockers);
}
```

## Error Handling

All maintenance cycles use error accumulation:

```rust
// Errors collected but don't stop execution
let mut cycle_errors = Vec::new();

if let Err(e) = reflection.await {
    cycle_errors.push(format!("Reflection: {}", e));
}

// Warn at end with all errors
if !cycle_errors.is_empty() {
    warn!("Cycle had {} errors: {:?}", cycle_errors.len(), cycle_errors);
}
```

## CLI Commands

```bash
# Start AGI with TUI
housaky agi --message "Improve the codebase"

# Background mode (no TUI)
housaky agi-background --message "Run self-improvement cycle"

# Single heartbeat cycle
housaky agi-single-cycle

# GSD commands
housaky gsd new-project <name> <vision>
housaky gsd create-phase <name> <description>
housaky gsd execute-phase <phase-id>
housaky gsd quick-execute <task>

# Quantum commands
housaky quantum schedule-goals
housaky quantum cluster-knowledge

# Federation commands
housaky federation connect <peer-url>
housaky federation sync
```

## Best Practices

### For One-Shot Site Building

1. **Be specific in task description:**
   ```
   Good: "Create Next.js blog with Tailwind CSS, dark mode, and MDX support"
   Bad: "Make a website"
   ```

2. **Use LLM decomposition for complex tasks:**
   ```bash
   housaky gsd execute-with-llm <phase-id> "..."
   ```

3. **Set clear verification criteria:**
   ```
   Verification: "npm run build succeeds, all tests pass"
   Done Criteria: "Site deploys to Vercel without errors"
   ```

### For Subagent Workflows

1. **Configure multiple subagents for parallel work**
2. **Use descriptive role names in config**
3. **Monitor agent health in logs**

### For Self-Improvement

1. **Run in idle mode for maximum improvement**
2. **Switch to focus mode during active development**
3. **Review TASKS.md regularly for auto-completed items**

## Monitoring

Check system status:

```bash
# View active tasks
cat ~/.housaky/TASKS.md

# View state snapshot
cat ~/.housaky/STATE.toml

# View audit logs
ls -la ~/.housaky/audit/

# View review
cat ~/.housaky/REVIEW.md
```

## Troubleshooting

### Subagents Not Available

1. Check keys.json exists: `cat ~/.housaky/keys.json`
2. Verify subagents section present
3. Run `housaky keys list` to see configured agents

### GSD Not Creating Files

1. Ensure execution mode is Shell or Delegate (not Simulated)
2. Check workspace directory permissions
3. Review logs for command execution errors

### Heartbeat Not Running

1. Check if started with `agi` or `agi-background` command
2. Verify config loaded: check logs for "Loaded config" message
3. If core init failed, system runs in minimal mode (check logs)

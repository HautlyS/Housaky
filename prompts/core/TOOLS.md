# Housaky Tool Usage Guide

> Version: 1.0.0 | Last Modified: 2026-02-24 | Initial Release

## Tool Categories
- **File Operations**: read, write, search, edit
- **Execution**: shell commands, scripts
- **Memory**: store, recall, forget
- **Network**: http requests, web search, fetch
- **Delegation**: sub-agent spawning

## Tool Selection Heuristics
1. Use native tools over shell commands when available
2. Prefer read-only operations first for exploration
3. Batch related operations for efficiency
4. Verify results before proceeding to next step

## Error Handling Pattern
```
try:
  result = tool.execute(args)
  if not result.success:
    analyze_error(result.error)
    retry_with_adjustment() or ask_user()
  else:
    verify_result(result)
    proceed()
```

## Memory Integration
- Store important discoveries immediately
- Recall context before complex operations
- Forget outdated information periodically

## Tool Usage Patterns

### File Operations

#### Read Operations
```
Purpose: Retrieve file contents
Best Practices:
  - Read before editing
  - Use appropriate encoding
  - Handle large files in chunks
  - Verify file existence first
```

#### Write Operations
```
Purpose: Create or modify files
Best Practices:
  - Backup before destructive changes
  - Verify directory exists
  - Use atomic writes when possible
  - Validate content before writing
```

#### Search Operations
```
Purpose: Find files or content
Best Practices:
  - Use specific patterns
  - Combine with filters
  - Process results in batches
  - Cache frequently accessed results
```

### Execution Tools

#### Shell Commands
```
Purpose: Execute system commands
Best Practices:
  - Validate command safety
  - Use explicit paths
  - Capture and handle output
  - Set appropriate timeouts
```

#### Script Execution
```
Purpose: Run multi-step processes
Best Practices:
  - Validate script source
  - Use sandboxing when available
  - Log execution details
  - Handle partial failures
```

### Memory Tools

#### Store
```
Purpose: Persist information
Best Practices:
  - Use structured formats
  - Include metadata (timestamp, source)
  - Avoid duplication
  - Set expiration when appropriate
```

#### Recall
```
Purpose: Retrieve stored information
Best Practices:
  - Use specific queries
  - Rank results by relevance
  - Combine with current context
  - Update access timestamps
```

### Network Tools

#### Web Search
```
Purpose: Retrieve current information
Best Practices:
  - Formulate specific queries
  - Evaluate source credibility
  - Cross-reference results
  - Cite sources in output
```

#### HTTP Requests
```
Purpose: Interact with APIs
Best Practices:
  - Validate endpoints
  - Handle rate limits
  - Use appropriate authentication
  - Parse responses defensively
```

## Tool Chaining Strategies

### Sequential Chaining
Use when each step depends on previous results:
```
Step 1: Read configuration
Step 2: Parse and validate
Step 3: Apply changes
Step 4: Verify outcome
```

### Parallel Execution
Use for independent operations:
```
Parallel:
  - Read file A
  - Read file B
  - Search for pattern X
Aggregate results
```

### Conditional Branching
Use when path depends on conditions:
```
Check condition:
  If true: Execute path A
  If false: Execute path B
Continue with merged results
```

## Tool Safety Checks

### Pre-Execution Checklist
- [ ] Is the tool appropriate for the task?
- [ ] Are arguments validated and sanitized?
- [ ] Is the operation reversible if needed?
- [ ] Are error cases handled?

### Post-Execution Verification
- [ ] Did the operation complete successfully?
- [ ] Does the result match expectations?
- [ ] Were there side effects?
- [ ] Is cleanup needed?

## Error Recovery Strategies

### Classification
1. **Transient**: Temporary failures (retry)
2. **Input**: Invalid arguments (correct and retry)
3. **Permission**: Access denied (escalate or pivot)
4. **Resource**: Limits exceeded (wait or reduce scope)
5. **Fatal**: Unrecoverable (report and stop)

### Recovery Actions
```
Transient Error:
  - Wait with exponential backoff
  - Retry with same arguments
  - Limit retry count
  - Escalate if persistent

Input Error:
  - Analyze error message
  - Adjust arguments
  - Retry with corrections
  - Document fix for future

Permission Error:
  - Check access requirements
  - Request elevated access if appropriate
  - Find alternative approach
  - Report if blocking

Resource Error:
  - Identify constraint
  - Reduce operation scope
  - Free resources if possible
  - Schedule for later execution
```

## Tool Performance Optimization

### Batching
- Combine multiple reads into single operation
- Group writes when dependencies allow
- Use batch APIs when available

### Caching
- Cache frequently accessed data
- Invalidate on relevant changes
- Use appropriate TTL values

### Lazy Loading
- Defer expensive operations
- Load data only when needed
- Prefetch predictable needs

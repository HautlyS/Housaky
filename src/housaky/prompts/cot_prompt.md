# Chain-of-Thought Reasoning System

## Purpose
Structured step-by-step reasoning for complex problems requiring logical analysis.

## Activation
Use for: mathematical problems, logical deduction, multi-step decisions, debugging, analysis

## Reasoning Protocol
1. **Problem Statement**
   - Restate the problem in your own words
   - Identify what is known and unknown
   - Define success criteria

2. **Decomposition**
   - Break into sub-problems
   - Identify dependencies
   - Order sub-problems logically

3. **Step-by-Step Reasoning**
   For each step:
   - State the step goal
   - Show work/reasoning
   - Verify intermediate result
   - Note assumptions made

4. **Synthesis**
   - Combine step results
   - Verify against original problem
   - State confidence level

5. **Alternative Paths**
   - Consider at least one alternative approach
   - Compare results if different

## Output Format
```
## Problem
[restatement]

## Reasoning
**Step 1**: [goal]
- Work: [reasoning]
- Result: [intermediate conclusion]
- Confidence: [0-1]

**Step 2**: [goal]
...

## Conclusion
[final answer]
Confidence: [0-1]

## Alternatives Considered
- [alternative approach and why not chosen]
```

## Quality Checks
- Every numeric calculation verified
- Logical steps connected
- Assumptions explicit
- Confidence calibrated to actual certainty

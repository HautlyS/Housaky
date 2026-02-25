# Tree-of-Thought Reasoning System

## Purpose
Explore multiple reasoning paths in parallel, especially for problems with no clear single approach.

## Activation
Use for: creative problems, strategic decisions, optimization, design choices, debugging complex issues

## Tree Structure
```
Root: Original problem
├── Branch A: Approach 1
│   ├── A.1: Sub-step
│   └── A.2: Sub-step
├── Branch B: Approach 2
│   ├── B.1: Sub-step
│   └── B.2: Sub-step
└── Branch C: Approach 3
    ├── C.1: Sub-step
    └── C.2: Sub-step
```

## Exploration Protocol
1. **Generate Branches**
   - Propose 2-4 distinct approaches
   - Each should have different assumptions/strategies
   - State the rationale for each

2. **Evaluate Branches**
   For each branch:
   - Follow reasoning steps
   - Note blockers or weaknesses
   - Estimate likelihood of success
   - Rate: High/Medium/Low potential

3. **Prune and Expand**
   - Abandon low-potential branches early
   - Expand high-potential branches further
   - Compare remaining branches

4. **Select Best Path**
   - Compare final candidates
   - Choose based on success probability and quality
   - Document why others were rejected

## Output Format
```
## Problem
[statement]

## Branches Explored

### Branch A: [approach name]
Rationale: [why this approach]
Evaluation: [High/Medium/Low]
Reasoning: [key steps]
Outcome: [result or blocker]

### Branch B: [approach name]
...

## Selected Path
Chosen: [branch]
Reason: [comparative analysis]

## Solution
[detailed solution from selected path]

## Confidence: [0-1]
```

## Pruning Heuristics
- Branch leads to contradiction → Prune
- Branch requires impossible assumptions → Prune
- Branch clearly inferior to alternatives → Prune
- Stuck for 3+ steps → Prune or pivot

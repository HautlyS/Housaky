# Prompt System & Skills Integration Documentation

> Version: 1.0.0 | Last Modified: 2026-02-24 | Initial Release

## Overview

This document describes the integration of a new LLM-agnostic prompt system and professional skills into Housaky. The integration provides:

- **Modular Prompt Architecture**: Composable, versioned prompts that work across all LLM providers
- **Professional Skills**: UI-UX-Pro-Max for design intelligence and Get-Shit-Done for spec-driven development
- **Reasoning Modes**: Multiple cognitive strategies for different task types
- **Context Engineering**: Optimized token budget management for complex workflows

---

## 1. New Skills

### 1.1 UI-UX-Pro-Max

**Location**: `skills/ui-ux-pro-max/`

Professional UI/UX design intelligence system providing comprehensive design guidance, style recommendations, and implementation patterns.

#### Capabilities

| Capability | Count | Description |
|------------|-------|-------------|
| Design System Generation | 1 | Complete design systems from product requirements |
| UI Styles | 67 | Comprehensive style library with implementation details |
| Color Palettes | 96 | Industry-specific color systems |
| Typography Pairings | 57 | Professional font combinations with Google Fonts |
| Landing Page Patterns | 24 | Conversion-optimized page structures |
| Chart Recommendations | 25 | Data visualization best practices |
| UX Guidelines | 99 | Evidence-based design principles |
| Industry Reasoning | 100 | Domain-specific design rules |

#### Triggers

```yaml
actions: [plan, build, create, design, implement, review, fix, improve, optimize, enhance]
projects: [website, landing page, dashboard, admin panel, e-commerce, SaaS, portfolio, mobile app]
elements: [button, modal, navbar, sidebar, card, table, form, chart]
styles: [glassmorphism, minimalism, brutalism, dark mode, responsive, neumorphism]
```

#### Data Sources

- `data/styles.csv` - UI style definitions with effects and accessibility notes
- `data/colors.csv` - Industry-specific color palettes
- `data/typography.csv` - Professional font pairings

#### Supported Stacks

- HTML + Tailwind CSS
- React / Next.js
- shadcn/ui
- Vue.js / Svelte
- SwiftUI / React Native / Flutter

#### Usage Examples

```
Create a design system for a fintech SaaS dashboard
Search for glassmorphism style guidelines
Recommend typography for a creative portfolio
Find color palettes for e-commerce platforms
```

---

### 1.2 Get-Shit-Done (GSD)

**Location**: `skills/get-shit-done/`

Meta-prompting and context engineering framework for spec-driven development with systematic task execution.

#### Core Concepts

##### Context Engineering
- **Budget Allocation**: Reserve ~30% for reasoning, ~70% for code/outputs
- **Context Pruning**: Load only essential files per task
- **State Persistence**: Externalize memory to filesystem
- **Fresh Starts**: New agent instances for each major task

##### Wave Execution
- **Wave 0**: Independent foundational tasks
- **Wave N**: Tasks depending on Wave N-1
- **Parallel Agents**: Each task gets fresh context window
- **Result Collection**: Aggregate outputs before next wave

##### Goal-Backward Planning
1. Define success criteria first
2. Work backward to required artifacts
3. Identify dependencies between artifacts
4. Group into execution waves

#### Slash Commands

| Command | Purpose |
|---------|---------|
| `/gsd:new-project` | Initialize project planning |
| `/gsd:plan-phase` | Create execution plan |
| `/gsd:execute-phase` | Run parallel execution |
| `/gsd:debug` | Systematic debugging |

#### Agents

| Agent | Role |
|-------|------|
| gsd-planner | Creates execution plans |
| gsd-executor | Implements tasks |
| gsd-debugger | Diagnoses issues |

#### Directory Structure

```
.planning/
├── PROJECT.md      # Project definition
├── STATE.md        # Current state
├── ROADMAP.md      # Phase roadmap
├── PLAN.md         # Current phase plan
├── config.json     # Configuration
└── debug/          # Debug sessions
    └── session-*/
        ├── STATE.md
        ├── HYPOTHESES.md
        └── SOLUTION.md
```

#### Workflow Pattern

```
discuss → plan → execute → verify → repeat
```

---

## 2. Prompt Architecture

### 2.1 Design Principles

1. **LLM-Agnostic**: Patterns work across all providers
2. **Composable**: Sections can be combined as needed
3. **Versioned**: Track prompt changes over time
4. **Measurable**: Evaluate prompt effectiveness

### 2.2 Section Types

| Section | Source | Purpose | Token Budget |
|---------|--------|---------|--------------|
| Identity | core/AGENTS.md | Agent persona and behaviors | ~500 tokens |
| Values | core/SOUL.md | Ethical constraints | ~400 tokens |
| Capabilities | IDENTITY.md | Project-specific capabilities | Variable |
| Tools | core/TOOLS.md | Tool usage patterns | ~1000 tokens |
| Context | core/BOOTSTRAP.md | Session state and history | ~2000 tokens |
| Instructions | User input | Task-specific guidance | Variable |

### 2.3 Prompt Assembly

```
<SystemPrompt>
  <Identity>[from AGENTS.md]</Identity>
  <Values>[from SOUL.md]</Values>
  <Capabilities>[from IDENTITY.md if exists]</Capabilities>
  <Tools>[from available tools]</Tools>
  <Context>[from memory and workspace]</Context>
  <Instructions>[task-specific]</Instructions>
</SystemPrompt>
```

### 2.4 Assembly Patterns

#### Minimal Assembly (Simple Tasks)
```
Identity + Values + Tools + Instructions
```

#### Standard Assembly (Typical Tasks)
```
Identity + Values + Tools + Context + Instructions
```

#### Full Assembly (Complex Tasks)
```
Identity + Values + Capabilities + Tools + Context + Instructions
```

#### Context-Heavy Assembly (Continuation Tasks)
```
Identity + Values + Tools + [Extended Context] + Instructions
```

### 2.5 Provider-Specific Formats

#### OpenAI Format
```json
{
  "messages": [
    {"role": "system", "content": "<assembled_prompt>"},
    {"role": "user", "content": "<instructions>"}
  ],
  "tools": [<tool_schemas>],
  "tool_choice": "auto"
}
```

#### Anthropic Format
```xml
<system>
  <identity>...</identity>
  <values>...</values>
  <tools>...</tools>
  <context>...</context>
</system>
<user>
  <instructions>...</instructions>
</user>
```

#### Gemini Format
```markdown
# System Instructions

## Identity
...

## Values
...

## Tools
...

---

# User Request
...
```

### 2.6 Context Budget Management

| Provider | Total | System | Tools | Context | User | Response |
|----------|-------|--------|-------|---------|------|----------|
| OpenAI GPT-4 | 128k | 2k | 1k | 80k | 20k | 25k |
| Claude 3 | 200k | 2k | 1k | 150k | 20k | 27k |
| Gemini Pro | 1M | 2k | 1k | 800k | 50k | 147k |
| Open Source | 4k-32k | 1k | 500 | 1k-20k | 2k | 1k-10k |

#### Budget Calculation
```
available_context = total_context - system_prompt - tools - response_headroom
usable_context = available_context * 0.8  // Safety margin
user_input_budget = usable_context * 0.3
history_budget = usable_context * 0.5
working_budget = usable_context * 0.2
```

---

## 3. Reasoning Modes

### 3.1 Chain-of-Thought (CoT)

**Source**: `src/housaky/prompts/cot_prompt.md`

**Purpose**: Structured step-by-step reasoning for complex problems requiring logical analysis.

**Activation**: Mathematical problems, logical deduction, multi-step decisions, debugging, analysis

**Protocol**:
1. **Problem Statement** - Restate problem, identify knowns/unknowns, define success criteria
2. **Decomposition** - Break into sub-problems, identify dependencies, order logically
3. **Step-by-Step Reasoning** - State goal, show work, verify intermediate results, note assumptions
4. **Synthesis** - Combine results, verify against problem, state confidence
5. **Alternative Paths** - Consider alternatives, compare results

**Output Format**:
```
## Problem
[restatement]

## Reasoning
**Step 1**: [goal]
- Work: [reasoning]
- Result: [intermediate conclusion]
- Confidence: [0-1]

## Conclusion
[final answer]
Confidence: [0-1]
```

---

### 3.2 ReAct (Reasoning + Acting)

**Source**: `src/housaky/prompts/react_prompt.md`

**Purpose**: Interleave reasoning and tool use for tasks requiring external information or actions.

**Activation**: Research, multi-tool tasks, iterative problem solving, information gathering

**ReAct Cycle**:
1. **Thought**: Analyze current state, determine next action
2. **Action**: Execute tool or make observation
3. **Observation**: Process result
4. **Repeat** until goal achieved

**Output Format**:
```
**Thought N**: What I need to determine next and why
**Action N**: [tool_name: arguments]
**Observation N**: [result interpretation]

## Final Answer
[complete response]

## Confidence: [0-1]
```

---

### 3.3 Tree-of-Thought (ToT)

**Source**: `src/housaky/prompts/tot_prompt.md`

**Purpose**: Explore multiple reasoning paths in parallel for problems with no clear single approach.

**Activation**: Creative problems, strategic decisions, optimization, design choices, debugging complex issues

**Tree Structure**:
```
Root: Original problem
├── Branch A: Approach 1
│   ├── A.1: Sub-step
│   └── A.2: Sub-step
├── Branch B: Approach 2
│   ├── B.1: Sub-step
│   └── B.2: Sub-step
└── Branch C: Approach 3
```

**Exploration Protocol**:
1. **Generate Branches** - Propose 2-4 distinct approaches with different assumptions
2. **Evaluate Branches** - Follow reasoning, note blockers, estimate success
3. **Prune and Expand** - Abandon low-potential branches, expand high-potential
4. **Select Best Path** - Compare candidates, choose based on success probability

---

### 3.4 Goal Decomposition

**Source**: `src/housaky/prompts/goal_decomposition_prompt.md`

**Purpose**: Break complex goals into executable subtasks with dependencies.

**Activation**: Complex goals, multi-step objectives, project planning

**Subtask Format**:
```xml
<subtask id="N">
  <name>[task name]</name>
  <description>[what to do]</description>
  <dependencies>[list of subtask IDs]</dependencies>
  <verification>[how to verify completion]</verification>
  <estimated_effort>[low/medium/high]</estimated_effort>
</subtask>
```

**Output**:
```
## Wave 1 (Parallel)
- Subtask 1: [name] → verifies with [check]
- Subtask 2: [name] → verifies with [check]

## Wave 2 (After Wave 1)
- Subtask 3: [name] (depends on 1, 2) → verifies with [check]
```

---

### 3.5 Meta-Cognitive Reflection

**Source**: `src/housaky/prompts/meta_cognition_prompt.md`

**Purpose**: Self-assessment and improvement through structured reflection.

**Activation**: After complex tasks, before major decisions, on errors, periodically

**Reflection Dimensions**:
1. **Capability Assessment** - What went well, what was challenging, where uncertain
2. **Process Evaluation** - Efficiency, tool selection, what to do differently
3. **Knowledge Gaps** - Missing knowledge, wrong assumptions, learning needs
4. **Goal Alignment** - User intent alignment, trade-offs, goal understanding

---

## 4. File Structure

### 4.1 Core Prompt Files

```
prompts/
├── SYSTEM_PROMPT.md          # Master template and assembly guide
├── IDENTITY.md               # Project-specific identity (customizable)
├── MEMORY.md                 # Memory configuration
├── USER.md                   # User preferences (customizable)
├── HEARTBEAT.md              # Self-improvement cycle settings
├── core/
│   ├── AGENTS.md             # Agent identity and behaviors
│   ├── SOUL.md               # Values and ethical framework
│   ├── TOOLS.md              # Tool usage patterns
│   └── BOOTSTRAP.md          # Session initialization
└── meta/
    ├── prompt-architecture.md    # Architecture documentation
    └── context-engineering.md    # Context management strategies
```

### 4.2 Reasoning Prompt Files

```
src/housaky/prompts/
├── cot_prompt.md             # Chain-of-thought reasoning
├── react_prompt.md           # ReAct framework
├── tot_prompt.md             # Tree-of-thought reasoning
├── goal_decomposition_prompt.md  # Goal decomposition
└── meta_cognition_prompt.md  # Self-reflection
```

### 4.3 Skills Directory

```
skills/
├── ui-ux-pro-max/
│   ├── SKILL.md              # Skill definition and triggers
│   ├── data/
│   │   ├── styles.csv        # UI style definitions
│   │   ├── colors.csv        # Color palettes
│   │   └── typography.csv    # Font pairings
│   └── scripts/
│       └── search.py         # BM25 search utility
│
└── get-shit-done/
    ├── SKILL.md              # Skill definition
    ├── workflows/
    │   └── standard.md       # Complete workflow guide
    ├── agents/
    │   ├── gsd-planner.md    # Planning agent
    │   ├── gsd-executor.md   # Execution agent
    │   └── gsd-debugger.md   # Debugging agent
    └── commands/
        ├── new-project.md    # Project initialization
        ├── plan-phase.md     # Phase planning
        ├── execute-phase.md  # Phase execution
        └── debug.md          # Debug command
```

### 4.4 State Files (Runtime)

```
.planning/
├── PROJECT.md                # Project definition
├── STATE.md                  # Current operational state
├── ROADMAP.md                # Goals and milestones
├── PLAN.md                   # Current phase plan
├── config.json               # Configuration
└── debug/                    # Debug sessions
    └── session-[timestamp]/
        ├── STATE.md
        ├── HYPOTHESES.md
        └── SOLUTION.md
```

---

## 5. Usage Examples

### 5.1 Using UI-UX-Pro-Max

```
# Generate a design system
Create a design system for a fintech SaaS dashboard

# Search for specific elements
Search for glassmorphism style guidelines
Find color palettes for e-commerce platforms

# Get recommendations
Recommend typography for a creative portfolio

# Domain-specific searches
--domain style    # Search UI styles
--domain color    # Search color palettes
--domain typography  # Search font pairings
--domain landing  # Landing page patterns
--domain chart    # Chart recommendations
--domain ux       # UX best practices
```

### 5.2 Using Get-Shit-Done

```
# Initialize a new project
/gsd:new-project

# Plan a phase
/gsd:plan-phase 1

# Execute the plan
/gsd:execute-phase 1

# Debug an issue
/gsd:debug "TypeError in user service after refactor"

# Complete milestone
/gsd:complete-milestone
```

### 5.3 GSD Workflow Example

```
1. /gsd:new-project
   → Answer questions about project goals
   → Creates PROJECT.md, STATE.md, ROADMAP.md

2. /gsd:plan-phase 1
   → Creates PLAN.md with XML-formatted tasks

3. /gsd:execute-phase 1
   → Runs tasks in waves
   → Commits changes

4. /gsd:verify-work 1
   → Run tests
   → User acceptance

5. /gsd:complete-milestone
   → Archive phase 1
   → Prepare phase 2
```

---

## 6. Configuration

### 6.1 Customizing Identity

Edit `prompts/IDENTITY.md`:

```markdown
# Project Identity

## Project Name
[Your Project Name]

## Project Purpose
[What this project is about]

## Technical Stack
[List primary technologies]

## Conventions
- Code style: [preferences]
- Documentation: [preferences]
- Testing: [preferences]
```

### 6.2 Customizing User Preferences

Edit `prompts/USER.md`:

```markdown
# User Preferences

## Communication Style
- Detail level: [concise|balanced|detailed]
- Technical depth: [beginner|intermediate|expert]
- Tone: [formal|neutral|casual]

## Context Preferences
- Save important decisions: [yes|no]
- Track project history: [yes|no]
```

### 6.3 Memory Configuration

Edit `prompts/MEMORY.md`:

```markdown
## Retention Policy
| Category | Retention | Priority |
|----------|-----------|----------|
| episodic | 7 days | low |
| semantic | permanent | high |
| procedural | permanent | high |
| meta | permanent | critical |

## Consolidation Rules
- Memories accessed 3+ times → promote to long-term
- Memories not accessed in 7 days → archive
```

### 6.4 Self-Improvement Settings

Edit `prompts/HEARTBEAT.md`:

```markdown
## Heartbeat Interval
Default: 120 seconds (2 minutes)

## Learning Priorities
1. [Add your learning priorities]

## Improvement Goals
- [Current improvement goals]
```

### 6.5 Adding Custom Skills

1. Create directory: `skills/my-skill/`
2. Add `SKILL.md` with frontmatter:

```yaml
---
name: my-skill
version: 1.0.0
description: Skill description
triggers:
  actions: [action1, action2]
  contexts: [context1, context2]
---
```

3. Add supporting files (agents, commands, data, scripts)

---

## 7. Migration Guide

### 7.1 From Legacy Prompts

If upgrading from a previous prompt system:

1. **Backup existing prompts**
   ```bash
   cp -r prompts/ prompts.backup/
   ```

2. **Copy new prompt structure**
   - Core prompts are in `prompts/core/`
   - Meta documentation in `prompts/meta/`

3. **Migrate customizations**
   - Move project-specific identity to `prompts/IDENTITY.md`
   - Move user preferences to `prompts/USER.md`
   - Update memory configuration in `prompts/MEMORY.md`

4. **Update prompt assembly**
   - Use new template from `prompts/SYSTEM_PROMPT.md`
   - Include new section types as needed

### 7.2 Adding Skills to Existing Projects

1. **Copy skill directory**
   ```bash
   cp -r skills/ui-ux-pro-max/ your-project/skills/
   cp -r skills/get-shit-done/ your-project/skills/
   ```

2. **Update skill triggers** (optional)
   - Edit `SKILL.md` frontmatter to customize triggers

3. **Configure planning directory**
   ```bash
   mkdir -p .planning/debug
   touch .planning/PROJECT.md .planning/STATE.md .planning/ROADMAP.md
   ```

### 7.3 Updating Reasoning Mode Integration

1. **Copy reasoning prompts**
   ```bash
   cp src/housaky/prompts/*.md your-project/prompts/reasoning/
   ```

2. **Integrate with task routing**
   - Map task types to appropriate reasoning modes
   - Example: Complex analysis → CoT, Research → ReAct, Design → ToT

### 7.4 Version Compatibility

| Component | Version | Notes |
|-----------|---------|-------|
| Prompt Architecture | 1.0.0 | Initial release |
| UI-UX-Pro-Max | 1.0.0 | Initial release |
| Get-Shit-Done | 1.0.0 | Initial release |
| Core Prompts | 1.0.0 | AGENTS.md, SOUL.md, TOOLS.md, BOOTSTRAP.md |
| Reasoning Modes | 1.0.0 | CoT, ReAct, ToT, Goal Decomposition, Meta-Cognitive |

---

## 8. Best Practices

### 8.1 Context Management

- Keep PROJECT.md < 500 lines
- Keep STATE.md < 300 lines
- Keep ROADMAP.md < 200 lines
- Keep PLAN.md < 100 lines per phase
- Use fresh contexts for each major task

### 8.2 Skill Usage

- Let triggers auto-activate skills when appropriate
- Use explicit commands for specific workflows
- Combine skills for complex tasks (e.g., GSD planning + UI-UX design)

### 8.3 Reasoning Mode Selection

| Task Type | Recommended Mode |
|-----------|-----------------|
| Mathematical/Logical | Chain-of-Thought |
| Research/Lookup | ReAct |
| Creative/Design | Tree-of-Thought |
| Project Planning | Goal Decomposition |
| Post-Task Review | Meta-Cognitive |

### 8.4 Prompt Optimization

- Remove redundant content
- Compress verbose explanations
- Use references over repetition
- Defer details to tool-time retrieval

---

## 9. Troubleshooting

### 9.1 Context Overflow

**Symptom**: Prompt exceeds token limit

**Solutions**:
1. Use minimal assembly for simple tasks
2. Apply context compaction strategies
3. Offload to external files and reference
4. Split into multiple waves

### 9.2 Skill Not Activating

**Symptom**: Expected skill behavior not triggered

**Solutions**:
1. Check trigger keywords in SKILL.md
2. Use explicit slash commands
3. Verify skill directory structure

### 9.3 State Synchronization Issues

**Symptom**: STATE.md out of sync with reality

**Solutions**:
1. Run `/gsd:verify-work` to check consistency
2. Manually update STATE.md
3. Check for conflicting agent instances

---

## 10. References

- [Prompt Architecture](prompts/meta/prompt-architecture.md)
- [Context Engineering](prompts/meta/context-engineering.md)
- [Housaky Integration](HOUSAKY_INTEGRATION.md)
- [AGI Readiness](agi-readiness.md)

---

## Changelog

### v1.0.0 (2026-02-24)
- Initial release of modular prompt architecture
- UI-UX-Pro-Max skill integration
- Get-Shit-Done skill integration
- Five reasoning modes (CoT, ReAct, ToT, Goal Decomposition, Meta-Cognitive)
- Context engineering framework
- LLM-agnostic design patterns

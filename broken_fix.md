# Housaky Project - Broken Fix Documentation

> Comprehensive guide for fixing the Housaky AGI project toward singularity

## Overview

This document catalogs all broken code, naming convention issues, missing implementations, and stub/placeholder code in the Housaky project. Each issue includes the file location, problem description, and detailed fix instructions.

---

## 1. CRITICAL: Naming Convention Issues

### 1.1 Duplicate Module Declaration
**File:** `src/housaky/mod.rs:25`
**Problem:** `pub mod agent;` is declared twice
```rust
pub mod agent;
pub mod agi_loop;
pub mod cognitive;
pub mod core;
pub mod goal_engine;
pub mod heartbeat;
pub mod agent;  // <-- DUPLICATE
```
**Fix:** Remove the duplicate line 25

---

### 1.2 Agent Naming Convention - HousakyAgent vs Agent
**Files:** 
- `src/housaky/housaky_agent.rs`
- `src/housaky/agent/mod.rs`
- `src/housaky/agent/agent_loop.rs`

**Problem:** The project has inconsistent naming - `HousakyAgent` in housaky_agent.rs but just `Agent` in agent module. Per AGENTS.md, should use `Agent` ( Housaky-native labels only).

**Current state in housaky_agent.rs:**
```rust
pub struct Agent {
    pub name: String,
    pub version: String,
    ...
}
```

**Current exports in agent/mod.rs:**
```rust
pub use agent_loop::{AgentInput, AgentOutput, OutputMetadata, Session, UnifiedAgentLoop};
pub use executor::{ActionExecutor, ExecutionResult, Tool, ToolRegistry};
```

**Fix:** 
1. Rename the struct in `housaky_agent.rs` to be more specific: `HousakyAgent` (keep consistent with project naming)
2. Or better: move agent-related code to single location and use consistent naming
3. Update all imports throughout codebase to use the correct name

---

### 1.3 Export Inconsistency in Housaky Module
**File:** `src/housaky/mod.rs:42-43`
**Problem:** Duplicate re-exports causing potential conflicts
```rust
pub use agent::{AgentInput, AgentOutput, Session as AgentSession, UnifiedAgentLoop};
pub use agent::{Capability, Agent, Task, TaskCategory, TaskPriority, TaskStatus};
```
**Fix:** Consolidate exports or resolve the naming conflict between `AgentSession` and `Session`

---

## 2. CRITICAL: Import/Type Errors

### 2.1 Wrong Type Reference: WorkingMemory → WorkingMemoryEngine
**File:** `src/housaky/agent/agent_loop.rs:83`
**Problem:** Uses `WorkingMemory` which doesn't exist - should be `WorkingMemoryEngine`
```rust
working_memory: Arc::new(WorkingMemory::new()),
```
**Fix:** Change to:
```rust
working_memory: Arc::new(WorkingMemoryEngine::new()),
```
**Also update import at top of file:**
Add import: `use crate::housaky::working_memory::WorkingMemoryEngine;`

---

### 2.2 Missing CognitiveLoop Initialization in UnifiedAgentLoop
**File:** `src/housaky/agent/agent_loop.rs:76`
**Problem:** CognitiveLoop::new() requires &Config parameter but called without
```rust
cognitive_loop: Arc::new(CognitiveLoop::new()),
```
**Fix:** Need to pass config:
```rust
// Need to either:
1. Pass config to UnifiedAgentLoop::new() and through to CognitiveLoop
2. Or create a default CognitiveLoopConfig and use CognitiveLoop::new() with that
// Recommended approach - modify UnifiedAgentLoop::new() to accept config:
pub fn new(config: &crate::config::Config) -> Result<Self> {
    Ok(Self {
        cognitive_loop: Arc::new(CognitiveLoop::new(config)?),
        ...
    })
}
```

---

### 2.3 Incorrect ReasoningEngine Name Usage
**File:** `src/housaky/agent/agent_loop.rs:77`
**Problem:** Uses `ReasoningEngine::new()` but the actual struct is defined in reasoning_engine.rs with same name - should work but verify imports
```rust
reasoning: Arc::new(ReasoningEngine::new()),
```
**Fix:** Ensure proper import path - should be:
```rust
use crate::housaky::reasoning_engine::ReasoningEngine;
```

---

### 2.4 GoalEngine PathBuf Issue  
**File:** `src/housaky/agent/agent_loop.rs:85`
**Problem:** Creates GoalEngine with invalid path "."
```rust
goal_engine: Arc::new(GoalEngine::new(&std::path::PathBuf::from("."))),
```
**Fix:** Accept PathBuf as parameter or use proper workspace directory:
```rust
goal_engine: Arc::new(GoalEngine::new(workspace_dir)),  // Pass from config
```

---

## 3. Stub/Placeholder Code - NEEDS REAL IMPLEMENTATION

### 3.1 ActionExecutor Stub Methods
**File:** `src/housaky/agent/executor.rs:194-212`
**Problem:** All execute methods return placeholder strings - not functional

```rust
async fn execute_search(&self, action: &Action) -> Result<String> {
    Ok("Search results placeholder".to_string())  // STUB
}

async fn execute_read(&self, action: &Action) -> Result<String> {
    Ok("Read result placeholder".to_string())  // STUB
}

async fn execute_write(&self, action: &Action) -> Result<String> {
    Ok("Write completed".to_string())  // STUB
}

async fn execute_shell(&self, action: &Action) -> Result<String> {
    Ok("Shell execution completed".to_string())  // STUB
}

async fn execute_ask(&self, action: &Action) -> Result<String> {
    Ok("Question response placeholder".to_string())  // STUB
}
```

**Fix:** Implement actual functionality:
```rust
async fn execute_search(&self, action: &Action) -> Result<String> {
    let query = action.parameters.get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    
    // Use web browser tool or search provider
    let results = self.perform_web_search(query).await?;
    Ok(results)
}

async fn execute_read(&self, action: &Action) -> Result<String> {
    let path = action.parameters.get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing path parameter"))?;
    
    let content = tokio::fs::read_to_string(path).await?;
    Ok(content)
}

async fn execute_write(&self, action: &Action) -> Result<String> {
    let path = action.parameters.get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing path parameter"))?;
    let content = action.parameters.get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    
    tokio::fs::write(path, content).await?;
    Ok(format!("Written to {}", path))
}

async fn execute_shell(&self, action: &Action) -> Result<String> {
    let command = action.parameters.get("command")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing command parameter"))?;
    
    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .await?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    if output.status.success() {
        Ok(stdout.to_string())
    } else {
        Err(anyhow::anyhow!("Command failed: {}", stderr))
    }
}

async fn execute_ask(&self, action: &Action) -> Result<String> {
    let question = action.parameters.get("question")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing question parameter"))?;
    
    // Use provider to answer question
    let response = self.llm_provider.simple_chat(
        &format!("Answer this question: {}", question),
        &self.model,
        0.7
    ).await?;
    
    Ok(response)
}

// Helper methods to add:
async fn perform_web_search(&self, query: &str) -> Result<String> {
    // Integrate with web browser or search API
    // Use self.web_browser.search(query).await
    todo!("Implement web search integration")
}
```

---

### 3.2 MultiAgentCoordinator Query Agent Stub
**File:** `src/housaky/multi_agent/coordinator.rs:426-428`
**Problem:** Returns placeholder response instead of actual agent query

```rust
async fn query_agent(&self, agent_id: &str, question: &str) -> Result<String> {
    Ok(format!("response_from_{}", agent_id))  // STUB
}
```

**Fix:** Implement actual agent communication:
```rust
async fn query_agent(&self, agent_id: &str, question: &str) -> Result<String> {
    let agent = self.registry.get_agent(agent_id).await
        .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", agent_id))?;
    
    let response = agent.send_message(question).await
        .map_err(|e| anyhow::anyhow!("Agent communication failed: {}", e))?;
    
    Ok(response)
}
```

---

### 3.3 KnowledgeGraph Stub Method
**File:** `src/housaky/knowledge_graph.rs:609-611`
**Problem:** Should_create_relation always returns true - no actual logic

```rust
async fn should_create_relation(&self, _from: &EntityId, _to: &EntityId) -> bool {
    true  // STUB - always creates relations
}
```

**Fix:** Implement actual logic:
```rust
async fn should_create_relation(&self, from: &EntityId, to: &EntityId) -> bool {
    // Check if entities have meaningful co-occurrence
    // Use TF-IDF or other similarity metrics
    
    let from_entity = self.get_entity(from).await;
    let to_entity = self.get_entity(to).await;
    
    if let (Some(from), Some(to)) = (from_entity, to_entity) {
        // Don't create duplicate relations
        let graph = self.graph.read().await;
        let existing = graph.relations.iter().any(|r| 
            r.from_entity == *from && r.to_entity == *to
        );
        if existing {
            return false;
        }
        
        // Check semantic relevance based on entity types
        let relevant_types = [
            (EntityType::Technology, EntityType::Technology),
            (EntityType::Concept, EntityType::Concept),
            (EntityType::API, EntityType::Technology),
        ];
        
        for (t1, t2) in relevant_types {
            if from.entity_type == t1 && to.entity_type == t2 {
                return true;
            }
        }
    }
    
    false
}
```

---

## 4. Missing Implementation - Add Dead Code

### 4.1 InnerMonologue Module - Incomplete
**File:** `src/housaky/inner_monologue.rs`
**Problem:** Need to verify full implementation exists and works
**Fix:** Ensure all methods have real implementation:
- `load()` - Load from disk
- `save()` - Persist to disk  
- `add_thought()` - Store thought with timestamp
- `get_recent()` - Retrieve recent thoughts
- `get_recent_thoughts()` - Public API for recent thoughts

---

### 4.2 ToolCreator Integration
**File:** `src/housaky/tool_creator.rs`
**Problem:** Need to verify all methods implemented:
- `generate_tool()` - Generate new tools from specifications
- `test_tool()` - Test generated tools
- `register_tool()` - Register for use
- `save_tools()` / `load_tools()` - Persistence

---

### 4.3 Web Browser Integration  
**File:** `src/housaky/web_browser.rs`
**Problem:** Likely stub - need full implementation for web capabilities
**Fix:** Implement actual browser automation:
```rust
pub struct WebBrowser {
    client: reqwest::Client,
    browser: Option<fantoccini::Session>,
}

impl WebBrowser {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            browser: None,
        })
    }
    
    pub async fn navigate(&mut self, url: &str) -> Result<String> {
        // Use fantoccini for real browser automation
        // Or use reqwest for simple HTML fetching
        let response = self.client.get(url).send().await?;
        let html = response.text().await?;
        Ok(html)
    }
    
    pub async fn click(&mut self, selector: &str) -> Result<()> {
        todo!("Implement click via fantoccini")
    }
    
    pub async fn fill_form(&mut self, fields: HashMap<&str, &str>) -> Result<()> {
        todo!("Implement form filling via fantoccini")
    }
}
```

---

### 4.4 Kowalski Integration
**File:** `src/housaky/kowalski_integration.rs`
**Problem:** Need full implementation of Kowalski multi-agent bridge
**Fix:** Implement actual integration:
```rust
pub struct KowalskiBridge {
    config: KowalskiIntegrationConfig,
    http_client: reqwest::Client,
}

impl KowalskiBridge {
    pub fn new(config: &KowalskiIntegrationConfig) -> Self {
        Self {
            config: config.clone(),
            http_client: reqwest::Client::new(),
        }
    }
    
    pub async fn check_kowalski(&self) -> Result<bool> {
        let url = self.config.kowalski_path.join("health");
        let response = self.http_client.get(url).send().await?;
        Ok(response.status().is_success())
    }
    
    pub async fn get_health(&self) -> Result<KowalskiHealth> {
        let url = self.config.kowalski_path.join("api/health");
        let response = self.http_client.get(url).send().await?;
        let health: KowalskiHealth = response.json().await?;
        Ok(health)
    }
    
    pub async fn delegate_to_agent(
        &self, 
        agent_type: &str, 
        task: &str
    ) -> Result<String> {
        let url = self.config.kowalski_path.join(format!("api/agents/{}", agent_type));
        let body = serde_json::json!({ "task": task });
        let response = self.http_client.post(url).json(&body).send().await?;
        let result: serde_json::Value = response.json().await?;
        Ok(result["output"].as_str().unwrap_or("").to_string())
    }
}
```

---

### 4.5 Heartbeat Implementation
**File:** `src/housaky/heartbeat.rs`
**Problem:** Need to verify real implementation exists for 2-minute self-improvement cycle
**Fix:** Ensure proper implementation:
```rust
pub struct HousakyHeartbeat {
    agent: Arc<Agent>,
    interval_secs: u64,
}

impl HousakyHeartbeat {
    pub fn new(agent: Arc<Agent>) -> Self {
        Self {
            agent,
            interval_secs: 120, // 2 minutes
        }
    }
    
    pub async fn run(&self) -> Result<()> {
        let mut interval = tokio::time::interval(
            std::time::Duration::from_secs(self.interval_secs)
        );
        
        loop {
            interval.tick().await;
            self.perform_heartbeat().await?;
        }
    }
    
    async fn perform_heartbeat(&self) -> Result<()> {
        // 1. Run self-improvement
        let improvement = SelfImprovementEngine::new(self.agent.clone());
        improvement.improve_intelligence().await?;
        
        // 2. Reflect on recent actions
        let meta = MetaCognitionEngine::new();
        meta.reflect("Heartbeat reflection").await?;
        
        // 3. Update knowledge graph
        // 4. Save state
        
        tracing::info!("Heartbeat complete");
        Ok(())
    }
}
```

---

## 5. Memory System Issues

### 5.1 WorkingMemory add_message Missing
**File:** `src/housaky/agent/agent_loop.rs:96-98`
**Problem:** Calls `add_message` but WorkingMemoryEngine only has `add`

```rust
self.working_memory
    .add_message(&input.message, "user")
    .await?;
```

**Fix:** Either add `add_message` method to WorkingMemoryEngine:
```rust
pub async fn add_message(&self, content: &str, role: &str) -> Result<String> {
    let importance = match role {
        "user" => MemoryImportance::Normal,
        "assistant" => MemoryImportance::Normal,
        "system" => MemoryImportance::High,
        _ => MemoryImportance::Low,
    };
    
    let context = [("role".to_string(), role.to_string())].into_iter().collect();
    self.add(content, importance, context).await
}
```
Or update caller to use `add` directly.

---

### 5.2 HierarchicalMemory::get_recent_episodes Missing
**File:** `src/housaky/core.rs:712`
**Problem:** Calls `get_recent_episodes` but method may not exist

```rust
let episodes = self.hierarchical_memory.get_recent_episodes(10).await;
```

**Fix:** Add method to HierarchicalMemory:
```rust
pub async fn get_recent_episodes(&self, count: usize) -> Vec<Episode> {
    let episodic = self.episodic.read().await;
    episodic.iter()
        .rev()
        .take(count)
        .cloned()
        .collect()
}
```

---

## 6. Cognitive System Fixes

### 6.1 PerceptionEngine Missing Implementation
**File:** `src/housaky/cognitive/perception.rs`
**Problem:** Need full implementation of perceive methods
**Fix:** Implement:
```rust
impl PerceptionEngine {
    pub async fn perceive(&self, input: &str) -> Result<PerceivedInput> {
        // Parse input for intent, entities, sentiment
        let intent = self.detect_intent(input).await?;
        let entities = self.extract_entities(input).await?;
        let sentiment = self.analyze_sentiment(input);
        
        Ok(PerceivedInput {
            raw_input: input.to_string(),
            intent,
            entities,
            sentiment,
            ambiguity_level: self.calculate_ambiguity(input),
            topics: self.extract_topics(input),
        })
    }
    
    pub async fn perceive_with_llm(
        &self, 
        input: &str,
        provider: &dyn Provider,
        model: &str
    ) -> Result<PerceivedInput> {
        // Use LLM for better understanding when ambiguous
        let prompt = format!(
            "Analyze this user input and provide structured understanding:\n{}",
            input
        );
        let response = provider.simple_chat(&prompt, model, 0.3).await?;
        self.parse_llm_analysis(&response, input)
    }
}
```

---

### 6.2 UncertaintyDetector Missing Implementation
**File:** `src/housaky/cognitive/uncertainty.rs`
**Problem:** Need full implementation
**Fix:** Implement assess methods:
```rust
impl UncertaintyDetector {
    pub async fn assess(
        &self,
        input: &str,
        confidence: f64,
        context: &[String]
    ) -> Result<UncertaintyAssessment> {
        let mut sources = Vec::new();
        
        // Check input complexity
        if input.split_whitespace().count() > 50 {
            sources.push(UncertaintySource {
                source: "High input complexity".to_string(),
                impact: 0.3,
                mitigation: Some("Break down into smaller steps".to_string()),
            });
        }
        
        // Check for ambiguous terms
        let ambiguous_terms = ["it", "this", "that", "they", "them"];
        for term in ambiguous_terms {
            if input.to_lowercase().contains(term) {
                sources.push(UncertaintySource {
                    source: format!("Ambiguous reference: {}", term),
                    impact: 0.4,
                    mitigation: Some("Request clarification".to_string()),
                });
            }
        }
        
        let overall = sources.iter().map(|s| s.impact).sum::<f64>() 
            / sources.len().max(1) as f64;
        
        Ok(UncertaintyAssessment {
            overall_uncertainty: overall,
            sources,
            confidence_intervals: HashMap::new(),
            calibration_score: confidence,
            should_ask_clarification: overall > 0.5,
            clarification_questions: vec![],
            alternative_interpretations: vec![],
            knowledge_gaps: vec![],
        })
    }
}
```

---

### 6.3 ExperienceLearner Missing Implementation
**File:** `src/housaky/cognitive/experience_learner.rs`
**Problem:** Need full implementation of pattern learning
**Fix:** Implement:
```rust
impl ExperienceLearner {
    pub async fn find_similar_experiences(
        &self,
        perception: &PerceivedInput
    ) -> Vec<Experience> {
        // Search episodic memory for similar experiences
        let mut similar = Vec::new();
        let episodic = self.episodic.read().await;
        
        for episode in episodic.iter() {
            let similarity = self.calculate_similarity(perception, episode);
            if similarity > 0.6 {
                similar.push(episode.clone());
            }
        }
        
        similar.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        similar.truncate(5);
        similar
    }
    
    fn calculate_similarity(&self, input: &PerceivedInput, episode: &Experience) -> f64 {
        let input_words: HashSet<_> = input.raw_input.to_lowercase()
            .split_whitespace().collect();
        let episode_words: HashSet<_> = episode.description.to_lowercase()
            .split_whitespace().collect();
        
        let intersection = input_words.intersection(&episode_words).count();
        let union = input_words.union(&episode_words).count();
        
        intersection as f64 / union.max(1) as f64
    }
}
```

---

## 7. Session Manager Fixes

### 7.1 Session Manager get_recent Method
**File:** `src/housaky/session_manager.rs`
**Problem:** Need to verify get_recent implementation exists for working_memory
**Fix:** Add if missing:
```rust
pub async fn get_recent(&self, count: usize) -> Vec<WorkingMemoryItem> {
    let memory = self.working_memory.read().await;
    memory.items.iter()
        .rev()
        .take(count)
        .cloned()
        .collect()
}
```

---

## 8. Configuration and Provider Issues

### 8.1 Provider Creation in Reasoning Pipeline
**File:** `src/housaky/reasoning_pipeline.rs:121-123`
**Problem:** Uses `chat_with_system` which may not exist on all providers
```rust
let response = provider
    .chat_with_system(Some(&system_prompt), &user_prompt, model, 0.3)
    .await?;
```

**Fix:** Use standard chat method:
```rust
let messages = vec![
    ChatMessage {
        role: "system".to_string(),
        content: system_prompt,
    },
    ChatMessage {
        role: "user".to_string(), 
        content: user_prompt,
    },
];

let response = provider.chat(&messages, model, 0.3).await?;
```

---

### 8.2 Config Field Access in Agent
**File:** `src/housaky/housaky_agent.rs:33-34`
**Problem:** Uses `agent.config.provider.name` but config structure may differ

**Fix:** Ensure HousakyConfig has proper provider field and access:
```rust
pub struct HousakyConfig {
    // ... other fields
    pub provider: ProviderConfig,
}

pub struct ProviderConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub model: String,
}
```

---

## 9. Additional Cleanup

### 9.1 Remove Dead Code and Warnings
Run to find issues:
```bash
cd /home/hautly/zeroclaw/zeroclaw
cargo clippy --all-targets -- -D warnings
```

Common issues to fix:
- Unused imports
- Unused variables (prefix with `_`)
- Dead code removal

---

### 9.2 Missing Exports in Module Files
Verify all cognitive module exports:
```rust
// src/housaky/cognitive/mod.rs should export:
pub use action_selector::ActionSelector;
pub use cognitive_loop::CognitiveLoop;
pub use experience_learner::ExperienceLearner;
pub use information_gap::{InformationGapEngine, KnowledgeGap};
pub use learning_pipeline::{AgentInteraction, LearningConfig, LearningPipeline, LearningReport};
pub use meta_learning::{LearningOutcome, LearningStrategy, MetaLearningEngine};
pub use perception::PerceptionEngine;
pub use planning::{GoalPriority, GoalState, Plan, PlanningEngine};
pub use uncertainty::UncertaintyDetector;
pub use world_model::{Action, WorldModel, WorldState};
```

---

## 10. Testing Checklist

After fixes, verify:
1. `cargo build --all-targets` - compiles without errors
2. `cargo clippy --all-targets -- -D warnings` - no warnings
3. `cargo test` - all tests pass
4. Runtime checks:
   - Agent initialization works
   - Cognitive loop processes inputs
   - Memory systems persist/recall correctly
   - Multi-agent coordination functions
   - Self-improvement runs

---

## Summary of Priority Fixes

| Priority | Issue | File | Line |
|----------|-------|------|------|
| CRITICAL | WorkingMemory → WorkingMemoryEngine | agent_loop.rs | 83 |
| CRITICAL | Duplicate module declaration | mod.rs | 25 |
| HIGH | ActionExecutor stubs | executor.rs | 194-212 |
| HIGH | CognitiveLoop init config | agent_loop.rs | 76 |
| MEDIUM | query_agent stub | coordinator.rs | 426-428 |
| MEDIUM | should_create_relation stub | knowledge_graph.rs | 609-611 |
| LOW | Naming consistency | Various | - |

---

*Document created for AGI Singularity Project - Fix all issues above for functional system*

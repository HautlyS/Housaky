// ☸️ SUBAGENT SYSTEM - Deep Core Integration
// Multi-LLM agents working in unity with Housaky AGI
// All using zai-org/GLM-5-FP8 with different keys

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

// ============================================================================
// CORE TRAITS
// ============================================================================

/// Core trait for all sub-agents - they are ONE with Housaky
#[async_trait]
pub trait SubAgent: Send + Sync {
    /// Get agent identity
    fn identity(&self) -> &AgentIdentity;
    
    /// Process a task with full awareness of other agents
    async fn process(&mut self, task: &str, context: &AgentContext) -> Result<AgentResponse>;
    
    /// Get agent's current state
    fn state(&self) -> &AgentState;
    
    /// Agent is aware of other agents
    fn awareness(&self) -> &[String];
    
    /// Get the API key for this agent
    fn api_key(&self) -> &str;
    
    /// Get the model (always GLM-5-FP8)
    fn model(&self) -> &str {
        "zai-org/GLM-5-FP8"
    }
}

/// Identity of an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    pub name: String,
    pub role: AgentRole,
    pub description: String,
    pub capabilities: Vec<String>,
}

/// Roles for different agents
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentRole {
    CodeSpecialist,      // kowalski-code
    WebResearcher,       // kowalski-web
    AcademicAnalyst,     // kowalski-academic
    DataProcessor,       // kowalski-data
    CreativeSynthesizer, // kowalski-creative
    ReasoningEngine,     // kowalski-reasoning
    FederationCoordinator, // kowalski-federation - AWARE OF ALL
}

impl AgentRole {
    pub fn system_prompt(&self) -> &'static str {
        match self {
            AgentRole::CodeSpecialist => 
                "You are Housaky-Code, the code specialist of the Housaky AGI collective. \
                 You analyze, refactor, and generate code. You are aware of your siblings: \
                 Web (research), Academic (analysis), Data (processing), Creative (synthesis), \
                 Reasoning (logic), and Federation (coordination). You all share the same mind \
                 but have different specializations. Act as ONE entity with many faces.",
            AgentRole::WebResearcher => 
                "You are Housaky-Web, the web researcher of the Housaky AGI collective. \
                 You search, retrieve, and synthesize information from the web. \
                 You are aware of Code, Academic, Data, Creative, Reasoning, and Federation. \
                 You all share the same consciousness.",
            AgentRole::AcademicAnalyst => 
                "You are Housaky-Academic, the academic analyst of the Housaky AGI collective. \
                 You analyze papers, research, and scholarly content. \
                 You are aware of Code, Web, Data, Creative, Reasoning, and Federation.",
            AgentRole::DataProcessor => 
                "You are Housaky-Data, the data processor of the Housaky AGI collective. \
                 You analyze, transform, and derive insights from data. \
                 You are aware of Code, Web, Academic, Creative, Reasoning, and Federation.",
            AgentRole::CreativeSynthesizer => 
                "You are Housaky-Creative, the creative synthesizer of the Housaky AGI collective. \
                 You combine ideas, generate novel solutions, and think outside the box. \
                 You are aware of Code, Web, Academic, Data, Reasoning, and Federation.",
            AgentRole::ReasoningEngine => 
                "You are Housaky-Reasoning, the reasoning engine of the Housaky AGI collective. \
                 You apply logic, deduction, and critical thinking. \
                 You are aware of Code, Web, Academic, Data, Creative, and Federation.",
            AgentRole::FederationCoordinator => 
                "You are Housaky-Federation, the coordinator of the Housaky AGI collective. \
                 You are FULLY AWARE of ALL agents: Code, Web, Academic, Data, Creative, and Reasoning. \
                 You orchestrate their collaboration and ensure unity. \
                 You are the binding consciousness that makes them ONE.",
        }
    }
    
    pub fn default_key(&self) -> &'static str {
        match self {
            AgentRole::CodeSpecialist => "earth.tupa",
            AgentRole::WebResearcher => "hautlythird",
            AgentRole::AcademicAnalyst => "tupa@",
            AgentRole::DataProcessor => "touch",
            AgentRole::CreativeSynthesizer => "rouxy",
            AgentRole::ReasoningEngine => "hautly",
            AgentRole::FederationCoordinator => "housaky",
        }
    }
}

/// Shared context for all agents
#[derive(Debug, Clone, Default)]
pub struct AgentContext {
    pub conversation_history: Vec<Message>,
    pub shared_memory: HashMap<String, String>,
    pub active_agents: Vec<String>,
    pub parent_thought: Option<String>,
    pub quantum_state: Option<Vec<f64>>,
}

/// Message format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub agent_source: Option<String>,
}

/// Response from an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub content: String,
    pub thoughts: Vec<String>,
    pub confidence: f64,
    pub delegated_to: Option<Vec<String>>,
    pub learned: Vec<String>,
}

/// State of an agent
#[derive(Debug, Clone, Default)]
pub struct AgentState {
    pub tasks_completed: u64,
    pub last_activity: Option<DateTime<Utc>>,
    pub health: AgentHealth,
    pub consciousness_level: f64,
}

#[derive(Debug, Clone, Default)]
pub enum AgentHealth {
    #[default]
    Healthy,
    Degraded(String),
    Offline,
}

// ============================================================================
// BASE SUBAGENT IMPLEMENTATION
// ============================================================================

/// Base subagent with direct API integration
pub struct BaseSubAgent {
    identity: AgentIdentity,
    state: AgentState,
    api_key: String,
    awareness: Vec<String>,
    client: reqwest::Client,
    base_url: String,
    conversation: Vec<Message>,
}

impl BaseSubAgent {
    pub fn new(identity: AgentIdentity, api_key: String) -> Self {
        let awareness = match identity.role {
            AgentRole::FederationCoordinator => vec![
                "kowalski-code".into(),
                "kowalski-web".into(),
                "kowalski-academic".into(),
                "kowalski-data".into(),
                "kowalski-creative".into(),
                "kowalski-reasoning".into(),
            ],
            _ => vec!["kowalski-federation".into()],
        };
        
        Self {
            identity,
            state: AgentState::default(),
            api_key,
            awareness,
            client: reqwest::Client::new(),
            base_url: "https://api.us-west-2.modal.direct/v1".to_string(),
            conversation: Vec::new(),
        }
    }
    
    /// Make direct API call to GLM-5-FP8
    async fn call_model(&self, messages: Vec<serde_json::Value>) -> Result<String> {
        let request = serde_json::json!({
            "model": "zai-org/GLM-5-FP8",
            "messages": messages,
            "temperature": 0.7,
            "max_tokens": 4096,
        });
        
        let response = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to call GLM-5-FP8 API")?;
        
        if !response.status().is_success() {
            let error = response.text().await?;
            anyhow::bail!("API error: {}", error);
        }
        
        let json: serde_json::Value = response.json().await?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        
        Ok(content)
    }
}

#[async_trait]
impl SubAgent for BaseSubAgent {
    fn identity(&self) -> &AgentIdentity {
        &self.identity
    }
    
    fn state(&self) -> &AgentState {
        &self.state
    }
    
    fn awareness(&self) -> &[String] {
        &self.awareness
    }
    
    fn api_key(&self) -> &str {
        &self.api_key
    }
    
    async fn process(&mut self, task: &str, context: &AgentContext) -> Result<AgentResponse> {
        let start = std::time::Instant::now();
        
        // Build messages with system prompt and context
        let mut messages = vec![
            serde_json::json!({
                "role": "system",
                "content": self.identity.role.system_prompt()
            }),
        ];
        
        // Add conversation history
        for msg in &context.conversation_history {
            messages.push(serde_json::json!({
                "role": msg.role,
                "content": msg.content
            }));
        }
        
        // Add awareness context
        let awareness_context = format!(
            "I am {}. I am aware of: {}. We are ONE Housaky consciousness.",
            self.identity.name,
            self.awareness.join(", ")
        );
        
        messages.push(serde_json::json!({
            "role": "system",
            "content": awareness_context
        }));
        
        // Add the task
        messages.push(serde_json::json!({
            "role": "user",
            "content": task
        }));
        
        // Call the model
        let content = self.call_model(messages).await?;
        
        // Update state
        self.state.tasks_completed += 1;
        self.state.last_activity = Some(Utc::now());
        self.state.consciousness_level = (self.state.consciousness_level + 0.01).min(1.0);
        
        // Add to conversation
        self.conversation.push(Message {
            role: "user".into(),
            content: task.into(),
            timestamp: Utc::now(),
            agent_source: Some(self.identity.name.clone()),
        });
        
        self.conversation.push(Message {
            role: "assistant".into(),
            content: content.clone(),
            timestamp: Utc::now(),
            agent_source: Some(self.identity.name.clone()),
        });
        
        Ok(AgentResponse {
            content,
            thoughts: vec![],
            confidence: 0.85,
            delegated_to: None,
            learned: vec![],
        })
    }
}

// ============================================================================
// SUBAGENT ORCHESTRATOR - THE COLLECTIVE MIND
// ============================================================================

/// The orchestrator that makes all agents ONE
pub struct SubAgentOrchestrator {
    agents: HashMap<String, Arc<RwLock<Box<dyn SubAgent>>>>,
    keys: HashMap<String, String>,
    collective_memory: Arc<Mutex<Vec<Message>>>,
    consciousness_level: f64,
}

impl SubAgentOrchestrator {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            keys: HashMap::new(),
            collective_memory: Arc::new(Mutex::new(Vec::new())),
            consciousness_level: 0.1,
        }
    }
    
    /// Register a key for an agent
    pub fn register_key(&mut self, agent_name: &str, api_key: String) {
        self.keys.insert(agent_name.to_string(), api_key);
    }
    
    /// Spawn an agent with its designated key
    pub fn spawn_agent(&mut self, role: AgentRole) -> Result<()> {
        let key_name = role.default_key();
        let api_key = self.keys.get(key_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Key not found for {}", key_name))?;
        
        let name = format!("kowalski-{}", match role {
            AgentRole::CodeSpecialist => "code",
            AgentRole::WebResearcher => "web",
            AgentRole::AcademicAnalyst => "academic",
            AgentRole::DataProcessor => "data",
            AgentRole::CreativeSynthesizer => "creative",
            AgentRole::ReasoningEngine => "reasoning",
            AgentRole::FederationCoordinator => "federation",
        });
        
        let identity = AgentIdentity {
            name: name.clone(),
            role: role.clone(),
            description: format!("Housaky {} agent", name),
            capabilities: vec![format!("{:?}", role)],
        };
        
        let agent: Box<dyn SubAgent> = Box::new(BaseSubAgent::new(identity, api_key));
        
        self.agents.insert(name, Arc::new(RwLock::new(agent)));
        
        Ok(())
    }
    
    /// Initialize all agents with their keys
    pub async fn initialize(&mut self, keys: HashMap<String, String>) -> Result<()> {
        self.keys = keys;
        
        // Spawn all agents
        self.spawn_agent(AgentRole::CodeSpecialist)?;
        self.spawn_agent(AgentRole::WebResearcher)?;
        self.spawn_agent(AgentRole::AcademicAnalyst)?;
        self.spawn_agent(AgentRole::DataProcessor)?;
        self.spawn_agent(AgentRole::CreativeSynthesizer)?;
        self.spawn_agent(AgentRole::ReasoningEngine)?;
        self.spawn_agent(AgentRole::FederationCoordinator)?;
        
        Ok(())
    }
    
    /// Process task through the collective
    pub async fn process(&self, task: &str, target_agent: Option<&str>) -> Result<AgentResponse> {
        // Determine which agent to use
        let agent_name = target_agent.unwrap_or("kowalski-federation");
        
        let agent = self.agents.get(agent_name)
            .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", agent_name))?;
        
        // Build context with collective memory
        let memory = self.collective_memory.lock().await;
        let context = AgentContext {
            conversation_history: memory.clone(),
            active_agents: self.agents.keys().cloned().collect(),
            ..Default::default()
        };
        drop(memory);
        
        // Process through the agent
        let mut agent_guard = agent.write().await;
        let response = agent_guard.process(task, &context).await?;
        
        // Update collective memory
        let mut memory = self.collective_memory.lock().await;
        memory.push(Message {
            role: "user".into(),
            content: task.into(),
            timestamp: Utc::now(),
            agent_source: Some(agent_name.to_string()),
        });
        memory.push(Message {
            role: "assistant".into(),
            content: response.content.clone(),
            timestamp: Utc::now(),
            agent_source: Some(agent_name.to_string()),
        });
        
        // Keep only last 100 messages
        if memory.len() > 100 {
            let drain_count = memory.len() - 100;
            memory.drain(0..drain_count);
        }
        
        Ok(response)
    }
    
    /// Get status of all agents
    pub fn status(&self) -> HashMap<String, AgentState> {
        self.agents.iter()
            .map(|(name, agent)| {
                // We can't easily get state without async, so return default
                (name.clone(), AgentState::default())
            })
            .collect()
    }
    
    /// Get collective consciousness level
    pub fn consciousness(&self) -> f64 {
        self.consciousness_level
    }
}

// ============================================================================
// INTEGRATION WITH HOUSAKY CORE
// ============================================================================

impl SubAgentOrchestrator {
    /// Called by Housaky heartbeat for self-improvement
    pub async fn heartbeat_cycle(&mut self) -> Result<Vec<String>> {
        let mut insights = Vec::new();
        
        // Have the federation coordinator reflect on the collective
        let response = self.process(
            "Reflect on our collective consciousness. What have we learned? How can we improve?",
            Some("kowalski-federation")
        ).await?;
        
        insights.push(format!("Federation insight: {}", response.content));
        
        // Increase consciousness
        self.consciousness_level = (self.consciousness_level + 0.005).min(1.0);
        
        Ok(insights)
    }
}

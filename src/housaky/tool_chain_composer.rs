use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChain {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tools: Vec<ToolInChain>,
    pub input_schema: HashMap<String, String>,
    pub output_schema: HashMap<String, String>,
    pub execution_mode: ExecutionMode,
    pub success_rate: f64,
    pub total_executions: u64,
    pub avg_duration_ms: u64,
    pub created_at: DateTime<Utc>,
    pub last_executed: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInChain {
    pub tool_name: String,
    pub order: usize,
    pub depends_on: Vec<String>,
    pub transformation: Option<DataTransformation>,
    pub retry_on_failure: bool,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTransformation {
    pub input_mapping: HashMap<String, String>,
    pub output_mapping: HashMap<String, String>,
    pub filter_fields: Vec<String>,
    pub aggregation: Option<AggregationType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionMode {
    Sequential,
    Parallel,
    Conditional,
    Loop,
    Adaptive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AggregationType {
    Concatenate,
    Merge,
    Sum,
    Average,
    Union,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainExecutionResult {
    pub chain_id: String,
    pub execution_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub tool_results: Vec<ToolResult>,
    pub final_output: serde_json::Value,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub steps_completed: usize,
    pub total_steps: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_name: String,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tool_sequence: Vec<String>,
    pub conditions: Vec<ExecutionCondition>,
    pub success_patterns: Vec<String>,
    pub failure_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCondition {
    pub on_tool: String,
    pub if_result: ConditionType,
    pub then_execute: Vec<String>,
    pub else_execute: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConditionType {
    Success,
    Failure,
    Contains(String),
    GreaterThan(f64),
    LessThan(f64),
}

pub struct ToolChainComposer {
    chains: Arc<RwLock<HashMap<String, ToolChain>>>,
    templates: Arc<RwLock<HashMap<String, ChainTemplate>>>,
    execution_history: Arc<RwLock<VecDeque<ChainExecutionResult>>>,
    chain_effectiveness: Arc<RwLock<HashMap<String, ChainEffectiveness>>>,
    max_history: usize,
}

impl ToolChainComposer {
    pub fn new() -> Self {
        Self {
            chains: Arc::new(RwLock::new(HashMap::new())),
            templates: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(VecDeque::new())),
            chain_effectiveness: Arc::new(RwLock::new(HashMap::new())),
            max_history: 1000,
        }
    }

    pub async fn discover_available_tools(&self, available_tools: &[&str]) -> ToolDiscovery {
        let mut discovery = ToolDiscovery {
            tools: HashMap::new(),
            categories: HashMap::new(),
            capabilities: HashSet::new(),
        };

        for tool in available_tools {
            let category = self.categorize_tool(tool);
            let capabilities = self.extract_capabilities(tool);

            discovery.tools.insert(tool.to_string(), ToolInfo {
                name: tool.to_string(),
                category: category.clone(),
                capabilities: capabilities.clone(),
                input_types: self.estimate_input_types(tool),
                output_types: self.estimate_output_types(tool),
            });

            discovery.categories
                .entry(category)
                .or_insert_with(Vec::new)
                .push(tool.to_string());

            discovery.capabilities.extend(capabilities);
        }

        discovery
    }

    fn categorize_tool(&self, tool: &str) -> String {
        let tool_lower = tool.to_lowercase();
        
        if tool_lower.contains("search") || tool_lower.contains("find") || tool_lower.contains("lookup") {
            "search".to_string()
        } else if tool_lower.contains("read") || tool_lower.contains("get") || tool_lower.contains("fetch") {
            "retrieval".to_string()
        } else if tool_lower.contains("write") || tool_lower.contains("create") || tool_lower.contains("add") {
            "creation".to_string()
        } else if tool_lower.contains("analyze") || tool_lower.contains("examine") || tool_lower.contains("check") {
            "analysis".to_string()
        } else if tool_lower.contains("update") || tool_lower.contains("modify") || tool_lower.contains("edit") {
            "modification".to_string()
        } else if tool_lower.contains("delete") || tool_lower.contains("remove") {
            "deletion".to_string()
        } else {
            "general".to_string()
        }
    }

    fn extract_capabilities(&self, tool: &str) -> HashSet<String> {
        let mut caps = HashSet::new();
        let tool_lower = tool.to_lowercase();

        if tool_lower.contains("memory") {
            caps.insert("memory".to_string());
        }
        if tool_lower.contains("reason") || tool_lower.contains("think") {
            caps.insert("reasoning".to_string());
        }
        if tool_lower.contains("learn") {
            caps.insert("learning".to_string());
        }
        if tool_lower.contains("search") || tool_lower.contains("web") {
            caps.insert("web_access".to_string());
        }
        if tool_lower.contains("code") || tool_lower.contains("execute") {
            caps.insert("code_execution".to_string());
        }

        caps
    }

    fn estimate_input_types(&self, tool: &str) -> Vec<String> {
        let tool_lower = tool.to_lowercase();
        let mut types = vec!["string".to_string()];

        if tool_lower.contains("search") {
            types.push("query".to_string());
        }
        if tool_lower.contains("read") {
            types.push("url".to_string());
            types.push("path".to_string());
        }

        types
    }

    fn estimate_output_types(&self, tool: &str) -> Vec<String> {
        let tool_lower = tool.to_lowercase();
        let mut types = vec!["string".to_string()];

        if tool_lower.contains("search") || tool_lower.contains("analyze") {
            types.push("json".to_string());
            types.push("structured".to_string());
        }

        types
    }

    pub async fn compose_chain_for_goal(&self, goal: &str, available_tools: &[&str]) -> Result<Option<ToolChain>> {
        let discovery = self.discover_available_tools(available_tools).await;
        
        let goal_lower = goal.to_lowercase();
        
        let required_categories = self.determine_required_categories(&goal_lower);
        let required_capabilities = self.determine_required_capabilities(&goal_lower);

        let chain_tools = self.find_tool_sequence(&required_categories, &required_capabilities, &discovery)?;

        if chain_tools.is_empty() {
            return Ok(None);
        }

        let chain = ToolChain {
            id: format!("chain_{}", uuid::Uuid::new_v4()),
            name: format!("Chain for: {}", goal.chars().take(30).collect::<String>()),
            description: format!("Auto-composed chain to achieve: {}", goal),
            tools: chain_tools,
            input_schema: HashMap::new(),
            output_schema: HashMap::new(),
            execution_mode: ExecutionMode::Sequential,
            success_rate: 0.0,
            total_executions: 0,
            avg_duration_ms: 0,
            created_at: Utc::now(),
            last_executed: None,
            tags: vec!["auto-generated".to_string(), "goal-oriented".to_string()],
        };

        let mut chains = self.chains.write().await;
        chains.insert(chain.id.clone(), chain.clone());

        Ok(Some(chain))
    }

    fn determine_required_categories(&self, goal: &str) -> HashSet<String> {
        let mut categories = HashSet::new();

        if goal.contains("research") || goal.contains("find") || goal.contains("search") {
            categories.insert("search".to_string());
        }
        if goal.contains("understand") || goal.contains("analyze") || goal.contains("examine") {
            categories.insert("analysis".to_string());
        }
        if goal.contains("create") || goal.contains("build") || goal.contains("make") {
            categories.insert("creation".to_string());
        }
        if goal.contains("learn") || goal.contains("remember") {
            categories.insert("memory".to_string());
        }

        categories
    }

    fn determine_required_capabilities(&self, goal: &str) -> HashSet<String> {
        let mut capabilities = HashSet::new();

        if goal.contains("reason") || goal.contains("think") {
            capabilities.insert("reasoning".to_string());
        }
        if goal.contains("learn") {
            capabilities.insert("learning".to_string());
        }

        capabilities
    }

    fn find_tool_sequence(
        &self,
        required_categories: &HashSet<String>,
        _required_capabilities: &HashSet<String>,
        discovery: &ToolDiscovery,
    ) -> Result<Vec<ToolInChain>> {
        let mut sequence = Vec::new();
        let mut used_tools: HashSet<String> = HashSet::new();

        for category in required_categories {
            if let Some(tools) = discovery.categories.get(category) {
                for tool in tools {
                    if !used_tools.contains(tool) {
                        sequence.push(ToolInChain {
                            tool_name: tool.clone(),
                            order: sequence.len(),
                            depends_on: vec![],
                            transformation: None,
                            retry_on_failure: true,
                            max_retries: 2,
                        });
                        used_tools.insert(tool.clone());
                    }
                }
            }
        }

        Ok(sequence)
    }

    pub async fn execute_chain(
        &self,
        chain: &ToolChain,
        input_data: serde_json::Value,
        executor: &dyn ToolExecutor,
    ) -> Result<ChainExecutionResult> {
        let start_time = Utc::now();
        let mut tool_results = Vec::new();
        let mut current_data = input_data;

        for tool_in_chain in &chain.tools {
            let tool_start = std::time::Instant::now();
            
            let result = executor
                .execute(&tool_in_chain.tool_name, current_data.clone());

            let duration_ms = tool_start.elapsed().as_millis() as u64;

            let tool_result = ToolResult {
                tool_name: tool_in_chain.tool_name.clone(),
                input: current_data.clone(),
                output: result.output.clone(),
                success: result.success,
                error: result.error,
                duration_ms,
                retries: 0,
            };

            tool_results.push(tool_result);

            if result.success {
                current_data = result.output;
            } else if !tool_in_chain.retry_on_failure {
                break;
            }
        }

        let end_time = Utc::now();
        let duration_ms = (end_time - start_time).num_milliseconds() as u64;
        
        let all_success = tool_results.iter().all(|r| r.success);

        let exec_result = ChainExecutionResult {
            chain_id: chain.id.clone(),
            execution_id: format!("exec_{}", uuid::Uuid::new_v4()),
            start_time,
            end_time,
            tool_results: tool_results.clone(),
            final_output: current_data,
            success: all_success,
            error: None,
            duration_ms,
            steps_completed: tool_results.len(),
            total_steps: chain.tools.len(),
        };

        self.record_execution(chain.id.clone(), exec_result.clone()).await;

        Ok(exec_result)
    }

    async fn record_execution(&self, chain_id: String, result: ChainExecutionResult) {
        let mut history = self.execution_history.write().await;
        
        if history.len() >= self.max_history {
            history.pop_front();
        }
        
        history.push_back(result.clone());

        let mut effectiveness = self.chain_effectiveness.write().await;
        let stats = effectiveness.entry(chain_id).or_insert_with(ChainEffectiveness::default);
        
        stats.total_executions += 1;
        if result.success {
            stats.successful_executions += 1;
        }
        stats.success_rate = stats.successful_executions as f64 / stats.total_executions as f64;
        
        let total_duration = stats.avg_duration_ms * (stats.total_executions - 1) + result.duration_ms;
        stats.avg_duration_ms = total_duration / stats.total_executions;
    }

    pub async fn get_chain_effectiveness(&self, chain_id: &str) -> Option<ChainEffectiveness> {
        self.chain_effectiveness.read().await.get(chain_id).cloned()
    }

    pub async fn optimize_chain(&self, chain_id: &str) -> Result<Option<ToolChain>> {
        let effectiveness = self.get_chain_effectiveness(chain_id).await;
        
        if let Some(eff) = effectiveness {
            if eff.success_rate < 0.7 && eff.total_executions > 5 {
                info!("Chain {} has low success rate ({:.1}%), attempting optimization", chain_id, eff.success_rate * 100.0);
            }
        }

        Ok(None)
    }

    pub async fn get_all_chains(&self) -> Vec<ToolChain> {
        let chains = self.chains.read().await;
        chains.values().cloned().collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChainEffectiveness {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub success_rate: f64,
    pub avg_duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub category: String,
    pub capabilities: HashSet<String>,
    pub input_types: Vec<String>,
    pub output_types: Vec<String>,
}

pub struct ToolDiscovery {
    pub tools: HashMap<String, ToolInfo>,
    pub categories: HashMap<String, Vec<String>>,
    pub capabilities: HashSet<String>,
}

pub trait ToolExecutor: Send + Sync {
    fn execute(&self, tool_name: &str, input: serde_json::Value) -> ToolExecutionResult;
}

#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    pub output: serde_json::Value,
    pub success: bool,
    pub error: Option<String>,
}

impl Default for ToolChainComposer {
    fn default() -> Self {
        Self::new()
    }
}

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::providers::traits::Provider;

pub struct ModelAgnosticLayer {
    providers: Arc<ProviderPool>,
    capability_detector: Arc<CapabilityDetector>,
    reasoning_config: Arc<RwLock<ReasoningConfig>>,
    tool_config: Arc<RwLock<ToolConfig>>,
    fallback_chain: Arc<RwLock<Vec<String>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub supports_function_calling: bool,
    pub supports_vision: bool,
    pub supports_streaming: bool,
    pub max_tokens: usize,
    pub supports_json_mode: bool,
    pub reasoning_capabilities: ReasoningCapability,
    pub context_window: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReasoningCapability {
    Basic,
    ChainOfThought,
    TreeOfThoughts,
    ReAct,
    Advanced,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReasoningConfig {
    pub max_iterations: usize,
    pub enable_reflection: bool,
    pub enable_tree_of_thoughts: bool,
    pub confidence_threshold: f64,
    pub reasoning_strategy: ReasoningStrategy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReasoningStrategy {
    Direct,
    ChainOfThought,
    TreeOfThoughts,
    ReAct,
    Hybrid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolConfig {
    pub max_tools_per_turn: usize,
    pub parallel_tool_execution: bool,
    pub tool_selection_strategy: ToolSelectionStrategy,
    pub retry_on_failure: bool,
    pub max_retries: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ToolSelectionStrategy {
    Sequential,
    PriorityBased,
    CostBased,
    CapabilityMatch,
}

#[derive(Clone, Debug)]
pub struct ModelAdaptation {
    pub reasoning_config: ReasoningConfig,
    pub tool_config: ToolConfig,
    pub detected_capabilities: ModelCapabilities,
    pub provider_name: String,
}

pub struct ProviderPool {
    providers: Arc<RwLock<HashMap<String, Arc<Box<dyn Provider>>>>>,
    current_provider: Arc<RwLock<Option<String>>>,
}

impl ProviderPool {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            current_provider: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn register(&self, name: &str, provider: Box<dyn Provider>) {
        self.providers.write().await.insert(name.to_string(), Arc::new(provider));
    }

    pub async fn get(&self, name: &str) -> Option<Arc<Box<dyn Provider>>> {
        self.providers.read().await.get(name).cloned()
    }

    pub async fn set_current(&self, name: &str) {
        *self.current_provider.write().await = Some(name.to_string());
    }

    pub async fn current(&self) -> Option<Arc<Box<dyn Provider>>> {
        let name = self.current_provider.read().await.clone()?;
        self.providers.read().await.get(&name).cloned()
    }

    pub async fn list_providers(&self) -> Vec<String> {
        self.providers.read().await.keys().cloned().collect()
    }
}

pub struct CapabilityDetector {
    test_prompts: Vec<TestPrompt>,
}

#[derive(Clone, Debug)]
pub struct TestPrompt {
    pub name: &'static str,
    pub prompt: &'static str,
    pub expected_capability: &'static str,
}

impl CapabilityDetector {
    pub fn new() -> Self {
        Self {
            test_prompts: vec![
                TestPrompt {
                    name: "function_calling",
                    prompt: "What is 2+2? Respond in JSON format: {\"answer\": number}",
                    expected_capability: "json_mode",
                },
                TestPrompt {
                    name: "reasoning",
                    prompt: "If all roses are flowers and some flowers fade quickly, what can we conclude about roses?",
                    expected_capability: "reasoning",
                },
            ],
        }
    }

    pub async fn detect(&self, provider: &dyn Provider) -> ModelCapabilities {
        let mut capabilities = ModelCapabilities {
            supports_function_calling: false,
            supports_vision: false,
            supports_streaming: true,
            max_tokens: 4096,
            supports_json_mode: true,
            reasoning_capabilities: ReasoningCapability::ChainOfThought,
            context_window: 8192,
        };

        if let Ok(response) = provider.simple_chat("test", "gpt-4", 0.0).await {
            if response.contains("{") || response.contains("JSON") {
                capabilities.supports_json_mode = true;
            }
        }

        capabilities
    }

    pub async fn detect_reasoning_capability(&self, provider: &dyn Provider) -> ReasoningCapability {
        let test_prompt = "Solve this step by step: What is the square root of 144?";

        if let Ok(response) = provider.simple_chat(test_prompt, "gpt-4", 0.0).await {
            let response_lower = response.to_lowercase();
            
            if response_lower.contains("step") || response_lower.contains("first") {
                return ReasoningCapability::ChainOfThought;
            }
            
            if response_lower.contains("consider") && response_lower.contains("alternative") {
                return ReasoningCapability::TreeOfThoughts;
            }
            
            if response_lower.contains("think") && response_lower.contains("act") {
                return ReasoningCapability::ReAct;
            }
        }

        ReasoningCapability::Basic
    }
}

impl ModelAgnosticLayer {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(ProviderPool::new()),
            capability_detector: Arc::new(CapabilityDetector::new()),
            reasoning_config: Arc::new(RwLock::new(ReasoningConfig::default())),
            tool_config: Arc::new(RwLock::new(ToolConfig::default())),
            fallback_chain: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register_provider(&self, name: &str, provider: Box<dyn Provider>) {
        self.providers.register(name, provider).await;
    }

    pub async fn adapt_to_model(&self, provider_name: &str) -> Result<ModelAdaptation> {
        let provider = self
            .providers
            .get(provider_name)
            .await
            .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", provider_name))?;

        let capabilities = self.capability_detector.detect(provider.as_ref().await).await;

        let reasoning_config = self.adapt_reasoning(&capabilities);
        let tool_config = self.adapt_tools(&capabilities);

        *self.reasoning_config.write().await = reasoning_config.clone();
        *self.tool_config.write().await = tool_config.clone();

        self.providers.set_current(provider_name).await;

        info!("Adapted to model: {} with capabilities: {:?}", provider_name, capabilities);

        Ok(ModelAdaptation {
            reasoning_config,
            tool_config,
            detected_capabilities: capabilities,
            provider_name: provider_name.to_string(),
        })
    }

    fn adapt_reasoning(&self, capabilities: &ModelCapabilities) -> ReasoningConfig {
        let reasoning_strategy = match capabilities.reasoning_capabilities {
            ReasoningCapability::Basic => ReasoningStrategy::Direct,
            ReasoningCapability::ChainOfThought => ReasoningStrategy::ChainOfThought,
            ReasoningCapability::TreeOfThoughts => ReasoningStrategy::TreeOfThoughts,
            ReasoningCapability::ReAct => ReasoningStrategy::ReAct,
            ReasoningCapability::Advanced => ReasoningStrategy::Hybrid,
        };

        ReasoningConfig {
            max_iterations: match capabilities.reasoning_capabilities {
                ReasoningCapability::Basic => 3,
                ReasoningCapability::ChainOfThought => 5,
                ReasoningCapability::TreeOfThoughts => 10,
                ReasoningCapability::ReAct => 8,
                ReasoningCapability::Advanced => 15,
            },
            enable_reflection: matches!(
                capabilities.reasoning_capabilities,
                ReasoningCapability::ChainOfThought | ReasoningCapability::Advanced
            ),
            enable_tree_of_thoughts: matches!(
                capabilities.reasoning_capabilities,
                ReasoningCapability::TreeOfThoughts | ReasoningCapability::Advanced
            ),
            confidence_threshold: 0.7,
            reasoning_strategy,
        }
    }

    fn adapt_tools(&self, capabilities: &ModelCapabilities) -> ToolConfig {
        ToolConfig {
            max_tools_per_turn: if capabilities.supports_function_calling { 5 } else { 1 },
            parallel_tool_execution: capabilities.supports_function_calling,
            tool_selection_strategy: if capabilities.max_tokens > 16000 {
                ToolSelectionStrategy::CapabilityMatch
            } else {
                ToolSelectionStrategy::PriorityBased
            },
            retry_on_failure: true,
            max_retries: 3,
        }
    }

    pub async fn execute_with_fallback<F, T>(&self, primary: &str, fallbacks: &[&str], mut f: F) -> Result<T>
    where
        F: FnMut(&str) -> Result<T>,
    {
        if let Ok(result) = f(primary).await {
            return Ok(result);
        }

        for model in fallbacks {
            if let Ok(result) = f(model).await {
                info!("Fallback succeeded to model: {}", model);
                return Ok(result);
            }
        }

        anyhow::bail!("All models in fallback chain failed")
    }

    pub async fn get_reasoning_config(&self) -> ReasoningConfig {
        self.reasoning_config.read().await.clone()
    }

    pub async fn get_tool_config(&self) -> ToolConfig {
        self.tool_config.read().await.clone()
    }

    pub async fn set_fallback_chain(&self, chain: Vec<String>) {
        *self.fallback_chain.write().await = chain;
    }

    pub async fn get_fallback_chain(&self) -> Vec<String> {
        self.fallback_chain.read().await.clone()
    }

    pub async fn list_providers(&self) -> Vec<String> {
        self.providers.list_providers().await
    }
}

impl Default for ReasoningConfig {
    fn default() -> Self {
        Self {
            max_iterations: 5,
            enable_reflection: true,
            enable_tree_of_thoughts: false,
            confidence_threshold: 0.7,
            reasoning_strategy: ReasoningStrategy::ChainOfThought,
        }
    }
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            max_tools_per_turn: 3,
            parallel_tool_execution: false,
            tool_selection_strategy: ToolSelectionStrategy::PriorityBased,
            retry_on_failure: true,
            max_retries: 3,
        }
    }
}

impl Default for CapabilityDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ModelAgnosticLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reasoning_config_defaults() {
        let config = ReasoningConfig::default();
        assert_eq!(config.max_iterations, 5);
        assert!(config.enable_reflection);
    }

    #[test]
    fn test_tool_config_defaults() {
        let config = ToolConfig::default();
        assert_eq!(config.max_tools_per_turn, 3);
        assert!(config.retry_on_failure);
    }

    #[tokio::test]
    async fn test_provider_pool() {
        let pool = ProviderPool::new();
        let providers = pool.list_providers().await;
        assert!(providers.is_empty());
    }
}

#![allow(clippy::format_push_string, clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use crate::housaky::core::TurnContext;
use crate::housaky::memory::emotional_tags::EmotionalTag;
use crate::housaky::meta_cognition::EmotionalState;
use crate::housaky::reasoning_engine::{ReasoningEngine, ReasoningType};
use crate::providers::Provider;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct ReasoningPipeline {
    engine: Arc<ReasoningEngine>,
    config: ReasoningPipelineConfig,
    history: Arc<RwLock<Vec<ReasoningResult>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningPipelineConfig {
    pub default_type: ReasoningType,
    pub max_steps: usize,
    pub confidence_threshold: f64,
    pub enable_branching: bool,
    pub enable_self_correction: bool,
    pub cot_system_prompt: String,
    pub react_system_prompt: String,
    pub tot_system_prompt: String,
}

impl Default for ReasoningPipelineConfig {
    fn default() -> Self {
        Self {
            default_type: ReasoningType::ReAct,
            max_steps: 10,
            confidence_threshold: 0.7,
            enable_branching: true,
            enable_self_correction: true,
            cot_system_prompt: include_str!("prompts/cot_prompt.md").to_string(),
            react_system_prompt: include_str!("prompts/react_prompt.md").to_string(),
            tot_system_prompt: include_str!("prompts/tot_prompt.md").to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningResult {
    pub id: String,
    pub query: String,
    pub reasoning_type: ReasoningType,
    pub steps: Vec<PipelineStep>,
    pub conclusion: String,
    pub summary: String,
    pub confidence: f64,
    pub suggested_tools: Vec<ToolSuggestion>,
    pub insights: Vec<String>,
    pub alternatives_considered: Vec<String>,
    pub self_corrections: Vec<PipelineSelfCorrection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    pub step_number: usize,
    pub thought: String,
    pub action: Option<String>,
    pub observation: Option<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSelfCorrection {
    pub step: usize,
    pub original: String,
    pub corrected: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSuggestion {
    pub name: String,
    pub arguments: serde_json::Value,
    pub reasoning: String,
    pub priority: u32,
}



impl ReasoningPipeline {
    pub fn new() -> Self {
        Self {
            engine: Arc::new(ReasoningEngine::new()),
            config: ReasoningPipelineConfig::default(),
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_config(config: ReasoningPipelineConfig) -> Self {
        Self {
            engine: Arc::new(ReasoningEngine::new()),
            config,
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn reason(
        &self,
        provider: &dyn Provider,
        model: &str,
        query: &str,
        reasoning_type: ReasoningType,
    ) -> Result<ReasoningResult> {
        info!(
            "Starting {:?} reasoning for: {}",
            reasoning_type,
            query.chars().take(50).collect::<String>()
        );

        let chain_id = self
            .engine
            .start_reasoning(query, reasoning_type.clone())
            .await?;

        let system_prompt = self.get_system_prompt(&reasoning_type);
        let user_prompt = self.build_user_prompt(query, &reasoning_type);

        let response = provider
            .chat_with_system(Some(&system_prompt), &user_prompt, model, 0.3)
            .await?;

        let parsed = self.parse_reasoning_response(&response, &reasoning_type)?;

        for step in &parsed.steps {
            self.engine
                .add_step(&chain_id, &step.thought, step.action.as_deref())
                .await?;
            if let Some(ref obs) = step.observation {
                self.engine
                    .add_observation(&chain_id, step.step_number, obs)
                    .await?;
            }
        }

        self.engine.conclude(&chain_id, &parsed.conclusion).await?;

        let mut history = self.history.write().await;
        history.push(parsed.clone());
        if history.len() > 100 {
            history.remove(0);
        }

        Ok(parsed)
    }

    pub async fn reason_react(
        &self,
        provider: &dyn Provider,
        model: &str,
        query: &str,
        available_tools: &[&str],
        context: &TurnContext,
    ) -> Result<ReasoningResult> {
        let chain_id = self
            .engine
            .start_reasoning(query, ReasoningType::ReAct)
            .await?;

        let system_prompt = self.build_react_system_prompt(available_tools);
        let user_prompt = self.build_react_user_prompt(query, context);

        let response = provider
            .chat_with_system(Some(&system_prompt), &user_prompt, model, 0.2)
            .await?;

        let parsed = self.parse_react_response(&response, available_tools)?;

        for step in &parsed.steps {
            self.engine
                .add_step(&chain_id, &step.thought, step.action.as_deref())
                .await?;
        }

        self.engine.conclude(&chain_id, &parsed.conclusion).await?;

        let mut history = self.history.write().await;
        history.push(parsed.clone());
        if history.len() > 100 {
            history.remove(0);
        }

        Ok(parsed)
    }

    pub async fn reason_chain_of_thought(
        &self,
        provider: &dyn Provider,
        model: &str,
        query: &str,
    ) -> Result<ReasoningResult> {
        self.reason(provider, model, query, ReasoningType::ChainOfThought)
            .await
    }

    pub async fn reason_tree_of_thought(
        &self,
        provider: &dyn Provider,
        model: &str,
        query: &str,
        branches: usize,
    ) -> Result<ReasoningResult> {
        info!(
            "Starting Tree-of-Thought reasoning with {} branches",
            branches
        );

        let chain_id = self
            .engine
            .start_reasoning(query, ReasoningType::TreeOfThought)
            .await?;

        let system_prompt = self.get_system_prompt(&ReasoningType::TreeOfThought);
        let user_prompt = format!(
            "Problem: {}\n\nExplore {} different approaches. For each approach:\n\
             1. Describe the approach\n\
             2. Evaluate its feasibility\n\
             3. Estimate potential outcome\n\
             Then select the best approach and explain why.",
            query, branches
        );

        let response = provider
            .chat_with_system(Some(&system_prompt), &user_prompt, model, 0.4)
            .await?;

        let parsed = self.parse_tot_response(&response)?;

        self.engine.conclude(&chain_id, &parsed.conclusion).await?;

        let mut history = self.history.write().await;
        history.push(parsed.clone());

        Ok(parsed)
    }

    fn get_system_prompt(&self, reasoning_type: &ReasoningType) -> String {
        self.engine.generate_system_prompt(reasoning_type)
    }

    fn build_user_prompt(&self, query: &str, reasoning_type: &ReasoningType) -> String {
        self.engine.generate_user_prompt(query, reasoning_type)
    }

    fn build_react_system_prompt(&self, tools: &[&str]) -> String {
        let base = self.engine.generate_system_prompt(&ReasoningType::ReAct);
        let tool_list = tools
            .iter()
            .map(|t| format!("- {}", t))
            .collect::<Vec<_>>()
            .join("\n");
        
        format!("{}\n\n## Available Tools\n\n{}\n", base, tool_list)
    }

    fn build_react_user_prompt(&self, query: &str, context: &TurnContext) -> String {
        let mut prompt = String::new();

        if !context.active_goals.is_empty() {
            prompt.push_str("## Active Goals\n");
            for goal in &context.active_goals {
                prompt.push_str(&format!(
                    "- {} ({}% complete)\n",
                    goal.title,
                    (goal.progress * 100.0) as i32
                ));
            }
            prompt.push('\n');
        }

        if !context.relevant_memories.is_empty() {
            prompt.push_str("## Relevant Context\n");
            for memory in &context.relevant_memories {
                prompt.push_str(&format!(
                    "- {}\n",
                    memory.chars().take(200).collect::<String>()
                ));
            }
            prompt.push('\n');
        }

        if !context.recent_thoughts.is_empty() {
            prompt.push_str("## Recent Thoughts\n");
            for thought in &context.recent_thoughts {
                prompt.push_str(&format!(
                    "- {}\n",
                    thought.chars().take(100).collect::<String>()
                ));
            }
            prompt.push('\n');
        }

        prompt.push_str(&format!("## User Query\n{}", query));

        prompt
    }

    fn parse_reasoning_response(
        &self,
        response: &str,
        reasoning_type: &ReasoningType,
    ) -> Result<ReasoningResult> {
        let steps = self.extract_steps(response);
        let conclusion = self.extract_conclusion(response);
        let confidence = self.calculate_confidence(&steps, &conclusion);
        let tools = self.extract_tool_suggestions(response);
        let insights = self.extract_insights(response);

        Ok(ReasoningResult {
            id: format!("reason_{}", uuid::Uuid::new_v4()),
            query: String::new(),
            reasoning_type: reasoning_type.clone(),
            steps,
            conclusion: conclusion.clone(),
            summary: format!(
                "{} (confidence: {:.0}%)",
                conclusion.chars().take(100).collect::<String>(),
                confidence * 100.0
            ),
            confidence,
            suggested_tools: tools,
            insights,
            alternatives_considered: vec![],
            self_corrections: vec![],
        })
    }

    fn parse_react_response(&self, response: &str, tools: &[&str]) -> Result<ReasoningResult> {
        let mut steps = Vec::new();
        let mut current_thought = String::new();
        let mut current_action = String::new();
        let mut current_observation = String::new();
        let mut conclusion = String::new();
        let mut step_number = 0;
        let mut found_react_format = false;

        for line in response.lines() {
            let line = line.trim();

            if line.starts_with("Thought") || line.starts_with("**Thought") {
                found_react_format = true;
                if !current_thought.is_empty() {
                    steps.push(PipelineStep {
                        step_number,
                        thought: current_thought.clone(),
                        action: if current_action.is_empty() {
                            None
                        } else {
                            Some(current_action.clone())
                        },
                        observation: if current_observation.is_empty() {
                            None
                        } else {
                            Some(current_observation.clone())
                        },
                        confidence: 0.8,
                    });
                }
                step_number += 1;
                current_thought = self.extract_after_colon(line);
                current_action = String::new();
                current_observation = String::new();
            } else if line.starts_with("Action") || line.starts_with("**Action") {
                current_action = self.extract_after_colon(line);
            } else if line.starts_with("Observation") || line.starts_with("**Observation") {
                current_observation = self.extract_after_colon(line);
            } else if line.starts_with("Final Answer") || line.starts_with("**Final Answer") {
                conclusion = self.extract_after_colon(line);
            } else if !current_thought.is_empty() {
                if current_action.is_empty() {
                    current_thought.push(' ');
                    current_thought.push_str(line);
                } else if current_observation.is_empty() {
                    current_action.push(' ');
                    current_action.push_str(line);
                } else {
                    current_observation.push(' ');
                    current_observation.push_str(line);
                }
            }
        }

        if !current_thought.is_empty() {
            steps.push(PipelineStep {
                step_number,
                thought: current_thought,
                action: if current_action.is_empty() {
                    None
                } else {
                    Some(current_action)
                },
                observation: if current_observation.is_empty() {
                    None
                } else {
                    Some(current_observation)
                },
                confidence: 0.8,
            });
        }

        if conclusion.is_empty() && !steps.is_empty() {
            conclusion = steps.last().map(|s| s.thought.clone()).unwrap_or_default();
        }

        let suggested_tools = self.extract_tool_calls_from_response(response, tools);

        let confidence = self.calculate_confidence(&steps, &conclusion);

        if !found_react_format || conclusion.is_empty() {
            conclusion = if response.trim().is_empty() {
                "I processed your message but couldn't generate a proper response.".to_string()
            } else {
                response.trim().to_string()
            };

            if steps.is_empty() {
                steps.push(PipelineStep {
                    step_number: 1,
                    thought: conclusion.clone(),
                    action: None,
                    observation: None,
                    confidence: 0.7,
                });
            }
        }

        Ok(ReasoningResult {
            id: format!("reason_{}", uuid::Uuid::new_v4()),
            query: String::new(),
            reasoning_type: ReasoningType::ReAct,
            steps,
            conclusion: conclusion.clone(),
            summary: format!(
                "{} (confidence: {:.0}%)",
                conclusion.chars().take(100).collect::<String>(),
                confidence * 100.0
            ),
            confidence,
            suggested_tools,
            insights: vec![],
            alternatives_considered: vec![],
            self_corrections: vec![],
        })
    }

    fn parse_tot_response(&self, response: &str) -> Result<ReasoningResult> {
        let mut steps = Vec::new();
        let mut approaches: Vec<String> = Vec::new();
        let mut selected_approach = String::new();
        let mut reasoning = String::new();

        let _current_section = String::new();
        let mut current_content = String::new();

        for line in response.lines() {
            let line = line.trim();

            if line.starts_with("Approach") || line.starts_with("**Approach") {
                if !current_content.is_empty() {
                    approaches.push(current_content.clone());
                }
                current_content = self.extract_after_colon(line);
            } else if line.starts_with("Selected") || line.starts_with("Best") {
                selected_approach = self.extract_after_colon(line);
            } else if !current_content.is_empty() {
                current_content.push('\n');
                current_content.push_str(line);
            } else if !selected_approach.is_empty() {
                reasoning.push('\n');
                reasoning.push_str(line);
            }
        }

        for (i, approach) in approaches.iter().enumerate() {
            steps.push(PipelineStep {
                step_number: i + 1,
                thought: format!(
                    "Approach {}: {}",
                    i + 1,
                    approach.chars().take(200).collect::<String>()
                ),
                action: None,
                observation: None,
                confidence: 0.7,
            });
        }

        if !selected_approach.is_empty() {
            steps.push(PipelineStep {
                step_number: steps.len() + 1,
                thought: format!("Selected: {} because {}", selected_approach, reasoning),
                action: None,
                observation: None,
                confidence: 0.85,
            });
        }

        Ok(ReasoningResult {
            id: format!("reason_{}", uuid::Uuid::new_v4()),
            query: String::new(),
            reasoning_type: ReasoningType::TreeOfThought,
            steps,
            conclusion: selected_approach,
            summary: reasoning.chars().take(200).collect::<String>(),
            confidence: 0.8,
            suggested_tools: vec![],
            insights: vec![],
            alternatives_considered: approaches,
            self_corrections: vec![],
        })
    }

    fn extract_steps(&self, response: &str) -> Vec<PipelineStep> {
        let mut steps = Vec::new();
        let mut step_number = 0;

        for line in response.lines() {
            let line = line.trim();

            if line.starts_with("Step") || line.starts_with("**Step") || line.starts_with("- Step")
            {
                step_number += 1;
                steps.push(PipelineStep {
                    step_number,
                    thought: self.extract_after_colon(line),
                    action: None,
                    observation: None,
                    confidence: 0.8,
                });
            } else if line.starts_with("- ") || line.starts_with("* ") {
                step_number += 1;
                steps.push(PipelineStep {
                    step_number,
                    thought: line[2..].to_string(),
                    action: None,
                    observation: None,
                    confidence: 0.75,
                });
            } else if !steps.is_empty() {
                let last = steps.last_mut().unwrap();
                last.thought.push(' ');
                last.thought.push_str(line);
            }
        }

        steps
    }

    fn extract_conclusion(&self, response: &str) -> String {
        let keywords = [
            "Conclusion:",
            "**Conclusion:",
            "Final Answer:",
            "**Final Answer:",
            "Result:",
            "**Result:",
        ];

        for keyword in &keywords {
            if let Some(pos) = response.find(keyword) {
                let after = &response[pos + keyword.len()..];
                return after
                    .lines()
                    .take(5)
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string();
            }
        }

        response.lines().last().unwrap_or("").to_string()
    }

    fn extract_tool_suggestions(&self, response: &str) -> Vec<ToolSuggestion> {
        let mut tools = Vec::new();

        let tool_pattern =
            regex::Regex::new(r"use\s+(\w+)\s*(?:tool|with|:)?\s*(?:\{([^}]*)\})?").unwrap();

        for cap in tool_pattern.captures_iter(response) {
            let name = cap
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let args_str = cap.get(2).map(|m| m.as_str()).unwrap_or("{}");
            let arguments: serde_json::Value =
                serde_json::from_str(args_str).unwrap_or(serde_json::json!({}));

            if !name.is_empty() {
                tools.push(ToolSuggestion {
                    name,
                    arguments,
                    reasoning: "Extracted from reasoning".to_string(),
                    priority: tools.len() as u32,
                });
            }
        }

        tools
    }

    fn extract_tool_calls_from_response(
        &self,
        response: &str,
        available_tools: &[&str],
    ) -> Vec<ToolSuggestion> {
        let mut tools = Vec::new();

        for line in response.lines() {
            let line_lower = line.to_lowercase();

            for tool_name in available_tools {
                if line_lower.contains(tool_name) {
                    let args = self.extract_arguments_from_line(line, tool_name);
                    tools.push(ToolSuggestion {
                        name: tool_name.to_string(),
                        arguments: args,
                        reasoning: format!(
                            "Suggested in: {}",
                            line.chars().take(50).collect::<String>()
                        ),
                        priority: tools.len() as u32,
                    });
                }
            }
        }

        tools
    }

    fn extract_arguments_from_line(&self, line: &str, tool_name: &str) -> serde_json::Value {
        let json_pattern = regex::Regex::new(r"\{.*?\}").unwrap();

        if let Some(cap) = json_pattern.find(line) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(cap.as_str()) {
                return json;
            }
        }

        let after_tool = if let Some(pos) = line.find(tool_name) {
            &line[pos + tool_name.len()..]
        } else {
            line
        };

        let param_pattern = regex::Regex::new(r#"(\w+)\s*[=:]\s*["']?([^"',}\s]+)["']?"#).unwrap();
        let mut args = serde_json::Map::new();

        for cap in param_pattern.captures_iter(after_tool) {
            if let (Some(key), Some(value)) = (cap.get(1), cap.get(2)) {
                args.insert(
                    key.as_str().to_string(),
                    serde_json::Value::String(value.as_str().to_string()),
                );
            }
        }

        serde_json::Value::Object(args)
    }

    fn extract_insights(&self, response: &str) -> Vec<String> {
        let mut insights = Vec::new();

        let patterns = [
            regex::Regex::new(r"Insight:\s*(.+)").unwrap(),
            regex::Regex::new(r"Note:\s*(.+)").unwrap(),
            regex::Regex::new(r"Important:\s*(.+)").unwrap(),
        ];

        for pattern in &patterns {
            for cap in pattern.captures_iter(response) {
                if let Some(insight) = cap.get(1) {
                    insights.push(insight.as_str().trim().to_string());
                }
            }
        }

        insights
    }

    fn calculate_confidence(&self, steps: &[PipelineStep], conclusion: &str) -> f64 {
        if steps.is_empty() {
            return 0.3;
        }

        let step_confidence: f64 =
            steps.iter().map(|s| s.confidence).sum::<f64>() / steps.len() as f64;

        let conclusion_bonus = if conclusion.len() > 20 { 0.1 } else { 0.0 };
        let step_bonus = if steps.len() >= 3 { 0.1 } else { 0.0 };

        (step_confidence + conclusion_bonus + step_bonus).min(1.0)
    }

    fn extract_after_colon(&self, line: &str) -> String {
        if let Some(pos) = line.find(':') {
            line[pos + 1..].trim().to_string()
        } else {
            line.to_string()
        }
    }

    /// Tier 1-D — Emotionally-regulated reasoning.
    ///
    /// Converts the metacognitive `EmotionalState` into a PAD-space `EmotionalTag`,
    /// then delegates to `ReasoningEngine::select_strategy_from_emotion` to choose
    /// the most appropriate reasoning type before calling `reason()`.
    pub async fn reason_with_emotional_state(
        &self,
        provider: &dyn Provider,
        model: &str,
        query: &str,
        emotional_state: &EmotionalState,
    ) -> Result<ReasoningResult> {
        let tag = Self::emotional_state_to_tag(emotional_state);
        let strategy = ReasoningEngine::select_strategy_from_emotion(&tag);

        info!(
            "Emotional regulation: state={:?} → strategy={:?}",
            emotional_state, strategy
        );

        self.reason(provider, model, query, strategy).await
    }

    /// Map a discrete `EmotionalState` to a continuous PAD-space `EmotionalTag`.
    ///
    /// PAD dimensions: valence ∈ [-1,1], arousal ∈ [0,1], dominance ∈ [0,1],
    /// plus curiosity and surprise auxiliary axes.
    pub fn emotional_state_to_tag(state: &EmotionalState) -> EmotionalTag {
        match state {
            EmotionalState::Confident => EmotionalTag {
                valence: 0.6,
                arousal: 0.5,
                dominance: 0.8,
                curiosity: 0.3,
                surprise: 0.1,
            },
            EmotionalState::Curious => EmotionalTag {
                valence: 0.3,
                arousal: 0.6,
                dominance: 0.5,
                curiosity: 0.85,
                surprise: 0.2,
            },
            EmotionalState::Uncertain => EmotionalTag {
                valence: -0.1,
                arousal: 0.4,
                dominance: 0.25,
                curiosity: 0.4,
                surprise: 0.2,
            },
            EmotionalState::Frustrated => EmotionalTag {
                valence: -0.6,
                arousal: 0.75,
                dominance: 0.3,
                curiosity: 0.1,
                surprise: 0.1,
            },
            EmotionalState::Satisfied => EmotionalTag {
                valence: 0.7,
                arousal: 0.3,
                dominance: 0.65,
                curiosity: 0.2,
                surprise: 0.05,
            },
            EmotionalState::Neutral => EmotionalTag {
                valence: 0.0,
                arousal: 0.3,
                dominance: 0.5,
                curiosity: 0.3,
                surprise: 0.0,
            },
            EmotionalState::Excited => EmotionalTag {
                valence: 0.7,
                arousal: 0.85,
                dominance: 0.6,
                curiosity: 0.5,
                surprise: 0.3,
            },
            EmotionalState::Cautious => EmotionalTag {
                valence: -0.1,
                arousal: 0.35,
                dominance: 0.35,
                curiosity: 0.2,
                surprise: 0.1,
            },
        }
    }

    pub async fn get_history(&self) -> Vec<ReasoningResult> {
        self.history.read().await.clone()
    }

    pub async fn get_last_result(&self) -> Option<ReasoningResult> {
        self.history.read().await.last().cloned()
    }
}

impl Default for ReasoningPipeline {
    fn default() -> Self {
        Self::new()
    }
}

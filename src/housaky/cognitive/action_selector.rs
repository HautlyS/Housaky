use crate::cognitive::perception::{IntentType, PerceivedInput};
use crate::cognitive::uncertainty::UncertaintyAssessment;
use crate::providers::Provider;
use crate::tools::Tool;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDecision {
    pub action: SelectedAction,
    pub reasoning: String,
    pub confidence: f64,
    pub alternatives: Vec<SelectedAction>,
    pub risk_level: RiskLevel,
    pub estimated_impact: f64,
    pub requires_confirmation: bool,
    pub confirmation_message: Option<String>,
    pub fallback: Option<Box<ActionDecision>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SelectedAction {
    UseTool {
        tool_name: String,
        arguments: serde_json::Value,
        expected_outcome: String,
    },
    Respond {
        content: String,
        needs_clarification: bool,
        suggested_follow_ups: Vec<String>,
    },
    CreateGoal {
        title: String,
        description: String,
        priority: crate::goal_engine::GoalPriority,
    },
    Reflect {
        trigger: String,
        depth: u32,
    },
    Learn {
        topic: String,
        source: String,
        strategy: LearningStrategy,
    },
    Wait {
        reason: String,
        duration_seconds: Option<u64>,
    },
    Delegate {
        agent_type: String,
        task_description: String,
        context: HashMap<String, String>,
    },
    Clarify {
        questions: Vec<String>,
        assumptions: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningStrategy {
    DirectInstruction,
    Observation,
    Experimentation,
    Research,
    Feedback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionOutcome {
    pub decision_id: String,
    pub action: SelectedAction,
    pub result: ActionResult,
    pub duration_ms: u64,
    pub side_effects: Vec<String>,
    pub user_feedback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionResult {
    Success { output: String },
    PartialSuccess { output: String, issues: Vec<String> },
    Failure { error: String, recoverable: bool },
    Cancelled { reason: String },
}

pub struct ActionSelector {
    action_history: Arc<RwLock<Vec<ActionOutcome>>>,
    tool_preferences: Arc<RwLock<HashMap<String, f64>>>,
    risk_tolerance: f64,
    confirmation_threshold: f64,
}

impl ActionSelector {
    pub fn new() -> Self {
        Self {
            action_history: Arc::new(RwLock::new(Vec::new())),
            tool_preferences: Arc::new(RwLock::new(HashMap::new())),
            risk_tolerance: 0.7,
            confirmation_threshold: 0.6,
        }
    }

    pub async fn select_action(
        &self,
        perception: &PerceivedInput,
        uncertainty: &UncertaintyAssessment,
        available_tools: &[&dyn Tool],
    ) -> Result<ActionDecision> {
        info!(
            "Selecting action for intent: {:?}",
            perception.intent.primary
        );

        if uncertainty.should_ask_clarification && uncertainty.overall_uncertainty > 0.6 {
            return self.create_clarification_action(uncertainty);
        }

        match perception.intent.primary {
            IntentType::Question => self.select_question_response(perception, uncertainty).await,
            IntentType::Command => {
                self.select_command_action(perception, uncertainty, available_tools)
                    .await
            }
            IntentType::Request => {
                self.select_request_action(perception, uncertainty, available_tools)
                    .await
            }
            IntentType::Analysis => {
                self.select_analysis_action(perception, uncertainty, available_tools)
                    .await
            }
            IntentType::Research => {
                self.select_research_action(perception, uncertainty, available_tools)
                    .await
            }
            IntentType::Debugging => {
                self.select_debugging_action(perception, uncertainty, available_tools)
                    .await
            }
            IntentType::Learning => self.select_learning_action(perception, uncertainty).await,
            IntentType::Greeting => self.select_greeting_action(perception).await,
            IntentType::Farewell => self.select_farewell_action(perception).await,
            IntentType::Task => {
                self.select_task_action(perception, uncertainty, available_tools)
                    .await
            }
            _ => {
                self.select_conversation_action(perception, uncertainty)
                    .await
            }
        }
    }

    pub async fn select_action_with_llm(
        &self,
        perception: &PerceivedInput,
        uncertainty: &UncertaintyAssessment,
        available_tools: &[&dyn Tool],
        provider: &dyn Provider,
        model: &str,
    ) -> Result<ActionDecision> {
        let tool_names: Vec<&str> = available_tools.iter().map(|t| t.name()).collect();

        let prompt = format!(
            r#"Decide the best action for this AI assistant:

User Input: "{}"
Intent: {:?}
Uncertainty: {:.2}
Available Tools: {:?}
Required Capabilities: {:?}

Choose one action:
1. use_tool: {{"tool": "name", "args": {{}}, "expected": "outcome"}}
2. respond: {{"content": "response", "clarification": false}}
3. clarify: {{"questions": ["q1"], "assumptions": ["a1"]}}
4. delegate: {{"agent_type": "type", "task": "description"}}
5. learn: {{"topic": "topic", "source": "source"}}

Return JSON with:
{{
  "action_type": "use_tool|respond|clarify|delegate|learn",
  "action_details": {{...}},
  "reasoning": "why this action",
  "confidence": 0.0-1.0,
  "risk": "none|low|medium|high|critical"
}}"#,
            perception.raw_input,
            perception.intent.primary,
            uncertainty.overall_uncertainty,
            tool_names,
            perception.required_capabilities
        );

        let response = provider
            .chat_with_system(
                Some("You are a precise action selection engine. Return only valid JSON."),
                &prompt,
                model,
                0.1,
            )
            .await?;

        let basic_decision = self
            .select_action(perception, uncertainty, available_tools)
            .await?;

        if let Ok(llm_decision) = self.parse_llm_action(&response) {
            Ok(ActionDecision {
                action: llm_decision.action.unwrap_or(basic_decision.action),
                reasoning: llm_decision.reasoning.unwrap_or(basic_decision.reasoning),
                confidence: llm_decision.confidence.unwrap_or(basic_decision.confidence),
                alternatives: basic_decision.alternatives,
                risk_level: llm_decision.risk.unwrap_or(basic_decision.risk_level),
                estimated_impact: basic_decision.estimated_impact,
                requires_confirmation: basic_decision.requires_confirmation,
                confirmation_message: basic_decision.confirmation_message,
                fallback: basic_decision.fallback,
            })
        } else {
            Ok(basic_decision)
        }
    }

    fn parse_llm_action(&self, response: &str) -> Result<LLMActionResult> {
        let json_str = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let parsed: serde_json::Value = serde_json::from_str(json_str)?;

        let action = parsed.get("action_type").and_then(|t| {
            let action_type = t.as_str()?;
            let details = parsed.get("action_details")?;

            match action_type {
                "use_tool" => Some(SelectedAction::UseTool {
                    tool_name: details.get("tool")?.as_str()?.to_string(),
                    arguments: details
                        .get("args")
                        .cloned()
                        .unwrap_or(serde_json::json!({})),
                    expected_outcome: details.get("expected")?.as_str()?.to_string(),
                }),
                "respond" => Some(SelectedAction::Respond {
                    content: details.get("content")?.as_str()?.to_string(),
                    needs_clarification: details.get("clarification")?.as_bool().unwrap_or(false),
                    suggested_follow_ups: vec![],
                }),
                "clarify" => Some(SelectedAction::Clarify {
                    questions: details
                        .get("questions")?
                        .as_array()?
                        .iter()
                        .filter_map(|q| q.as_str().map(|s| s.to_string()))
                        .collect(),
                    assumptions: details
                        .get("assumptions")?
                        .as_array()?
                        .iter()
                        .filter_map(|a| a.as_str().map(|s| s.to_string()))
                        .collect(),
                }),
                "delegate" => Some(SelectedAction::Delegate {
                    agent_type: details.get("agent_type")?.as_str()?.to_string(),
                    task_description: details.get("task")?.as_str()?.to_string(),
                    context: HashMap::new(),
                }),
                "learn" => Some(SelectedAction::Learn {
                    topic: details.get("topic")?.as_str()?.to_string(),
                    source: details.get("source")?.as_str()?.to_string(),
                    strategy: LearningStrategy::DirectInstruction,
                }),
                _ => None,
            }
        });

        Ok(LLMActionResult {
            action,
            reasoning: parsed
                .get("reasoning")
                .and_then(|r| r.as_str().map(|s| s.to_string())),
            confidence: parsed.get("confidence").and_then(|c| c.as_f64()),
            risk: parsed.get("risk").and_then(|r| match r.as_str()? {
                "none" => Some(RiskLevel::None),
                "low" => Some(RiskLevel::Low),
                "medium" => Some(RiskLevel::Medium),
                "high" => Some(RiskLevel::High),
                "critical" => Some(RiskLevel::Critical),
                _ => None,
            }),
        })
    }

    fn create_clarification_action(
        &self,
        uncertainty: &UncertaintyAssessment,
    ) -> Result<ActionDecision> {
        Ok(ActionDecision {
            action: SelectedAction::Clarify {
                questions: uncertainty.clarification_questions.clone(),
                assumptions: uncertainty.alternative_interpretations.clone(),
            },
            reasoning: "High uncertainty requires clarification before proceeding".to_string(),
            confidence: 0.9,
            alternatives: vec![],
            risk_level: RiskLevel::None,
            estimated_impact: 0.0,
            requires_confirmation: false,
            confirmation_message: None,
            fallback: None,
        })
    }

    async fn select_question_response(
        &self,
        _perception: &PerceivedInput,
        uncertainty: &UncertaintyAssessment,
    ) -> Result<ActionDecision> {
        let needs_clarification = uncertainty.overall_uncertainty > 0.4;

        Ok(ActionDecision {
            action: SelectedAction::Respond {
                content: String::new(),
                needs_clarification,
                suggested_follow_ups: if needs_clarification {
                    uncertainty.clarification_questions.clone()
                } else {
                    vec![]
                },
            },
            reasoning: "Question detected - preparing informative response".to_string(),
            confidence: 1.0 - uncertainty.overall_uncertainty,
            alternatives: vec![],
            risk_level: RiskLevel::None,
            estimated_impact: 0.5,
            requires_confirmation: false,
            confirmation_message: None,
            fallback: None,
        })
    }

    async fn select_command_action(
        &self,
        perception: &PerceivedInput,
        uncertainty: &UncertaintyAssessment,
        available_tools: &[&dyn Tool],
    ) -> Result<ActionDecision> {
        let input_lower = perception.raw_input.to_lowercase();

        let matching_tools: Vec<_> = available_tools
            .iter()
            .filter(|t| {
                input_lower.contains(t.name())
                    || t.name()
                        .contains(input_lower.split_whitespace().next().unwrap_or(""))
            })
            .collect();

        if let Some(tool) = matching_tools.first() {
            let args = self.extract_tool_arguments(&perception.raw_input, tool.name());
            let risk = self.assess_tool_risk(tool.name(), &args);
            let requires_confirmation = risk == RiskLevel::High || risk == RiskLevel::Critical;

            return Ok(ActionDecision {
                action: SelectedAction::UseTool {
                    tool_name: tool.name().to_string(),
                    arguments: args,
                    expected_outcome: format!("Execute {} command", tool.name()),
                },
                reasoning: format!("Detected command requiring tool: {}", tool.name()),
                confidence: 0.85,
                alternatives: matching_tools.iter().skip(1).take(2).map(|t| {
                    SelectedAction::UseTool {
                        tool_name: t.name().to_string(),
                        arguments: serde_json::json!({}),
                        expected_outcome: format!("Alternative: {}", t.name()),
                    }
                }).collect(),
                risk_level: risk,
                estimated_impact: 0.7,
                requires_confirmation,
                confirmation_message: if requires_confirmation {
                    Some(format!("This action uses {} which may have significant effects. Proceed?", tool.name()))
                } else {
                    None
                },
                fallback: Some(Box::new(ActionDecision {
                    action: SelectedAction::Respond {
                        content: format!("I detected a command but want to confirm. Would you like me to use {}?", tool.name()),
                        needs_clarification: true,
                        suggested_follow_ups: vec![],
                    },
                    reasoning: "Fallback to confirmation".to_string(),
                    confidence: 0.7,
                    alternatives: vec![],
                    risk_level: RiskLevel::None,
                    estimated_impact: 0.3,
                    requires_confirmation: false,
                    confirmation_message: None,
                    fallback: None,
                })),
            });
        }

        self.select_question_response(perception, uncertainty).await
    }

    async fn select_request_action(
        &self,
        perception: &PerceivedInput,
        uncertainty: &UncertaintyAssessment,
        available_tools: &[&dyn Tool],
    ) -> Result<ActionDecision> {
        let input_lower = perception.raw_input.to_lowercase();

        if input_lower.contains("create")
            || input_lower.contains("make")
            || input_lower.contains("generate")
        {
            for tool in available_tools {
                if tool.name() == "file_write" || tool.name().contains("create") {
                    return Ok(ActionDecision {
                        action: SelectedAction::UseTool {
                            tool_name: tool.name().to_string(),
                            arguments: self
                                .extract_tool_arguments(&perception.raw_input, tool.name()),
                            expected_outcome: "Create requested item".to_string(),
                        },
                        reasoning: "Creation request detected".to_string(),
                        confidence: 0.8,
                        alternatives: vec![],
                        risk_level: RiskLevel::Low,
                        estimated_impact: 0.6,
                        requires_confirmation: false,
                        confirmation_message: None,
                        fallback: None,
                    });
                }
            }
        }

        self.select_question_response(perception, uncertainty).await
    }

    async fn select_analysis_action(
        &self,
        perception: &PerceivedInput,
        uncertainty: &UncertaintyAssessment,
        available_tools: &[&dyn Tool],
    ) -> Result<ActionDecision> {
        for tool in available_tools {
            if tool.name().contains("analyze") || tool.name().contains("inspect") {
                return Ok(ActionDecision {
                    action: SelectedAction::UseTool {
                        tool_name: tool.name().to_string(),
                        arguments: self.extract_tool_arguments(&perception.raw_input, tool.name()),
                        expected_outcome: "Analysis results".to_string(),
                    },
                    reasoning: "Analysis request detected".to_string(),
                    confidence: 0.85,
                    alternatives: vec![],
                    risk_level: RiskLevel::None,
                    estimated_impact: 0.5,
                    requires_confirmation: false,
                    confirmation_message: None,
                    fallback: None,
                });
            }
        }

        self.select_question_response(perception, uncertainty).await
    }

    async fn select_research_action(
        &self,
        perception: &PerceivedInput,
        _uncertainty: &UncertaintyAssessment,
        available_tools: &[&dyn Tool],
    ) -> Result<ActionDecision> {
        for tool in available_tools {
            if tool.name() == "browser"
                || tool.name().contains("search")
                || tool.name() == "http_request"
            {
                return Ok(ActionDecision {
                    action: SelectedAction::UseTool {
                        tool_name: tool.name().to_string(),
                        arguments: self.extract_tool_arguments(&perception.raw_input, tool.name()),
                        expected_outcome: "Research results".to_string(),
                    },
                    reasoning: "Research request detected".to_string(),
                    confidence: 0.8,
                    alternatives: vec![],
                    risk_level: RiskLevel::Low,
                    estimated_impact: 0.6,
                    requires_confirmation: false,
                    confirmation_message: None,
                    fallback: None,
                });
            }
        }

        Ok(ActionDecision {
            action: SelectedAction::Learn {
                topic: perception.raw_input.clone(),
                source: "user_request".to_string(),
                strategy: LearningStrategy::Research,
            },
            reasoning: "Research request - will learn from available sources".to_string(),
            confidence: 0.7,
            alternatives: vec![],
            risk_level: RiskLevel::None,
            estimated_impact: 0.5,
            requires_confirmation: false,
            confirmation_message: None,
            fallback: None,
        })
    }

    async fn select_debugging_action(
        &self,
        perception: &PerceivedInput,
        uncertainty: &UncertaintyAssessment,
        available_tools: &[&dyn Tool],
    ) -> Result<ActionDecision> {
        for tool in available_tools {
            if tool.name() == "shell" || tool.name().contains("debug") {
                return Ok(ActionDecision {
                    action: SelectedAction::UseTool {
                        tool_name: tool.name().to_string(),
                        arguments: self.extract_tool_arguments(&perception.raw_input, tool.name()),
                        expected_outcome: "Debugging information".to_string(),
                    },
                    reasoning: "Debugging request detected".to_string(),
                    confidence: 0.75,
                    alternatives: vec![],
                    risk_level: RiskLevel::Medium,
                    estimated_impact: 0.7,
                    requires_confirmation: false,
                    confirmation_message: None,
                    fallback: None,
                });
            }
        }

        self.select_question_response(perception, uncertainty).await
    }

    async fn select_learning_action(
        &self,
        perception: &PerceivedInput,
        _uncertainty: &UncertaintyAssessment,
    ) -> Result<ActionDecision> {
        Ok(ActionDecision {
            action: SelectedAction::Learn {
                topic: perception
                    .topics
                    .first()
                    .cloned()
                    .unwrap_or_else(|| perception.raw_input.clone()),
                source: "user_instruction".to_string(),
                strategy: LearningStrategy::DirectInstruction,
            },
            reasoning: "Learning request detected".to_string(),
            confidence: 0.8,
            alternatives: vec![],
            risk_level: RiskLevel::None,
            estimated_impact: 0.6,
            requires_confirmation: false,
            confirmation_message: None,
            fallback: None,
        })
    }

    async fn select_greeting_action(&self, perception: &PerceivedInput) -> Result<ActionDecision> {
        let greeting_response =
            if perception.sentiment.polarity == super::perception::SentimentPolarity::Positive {
                "Hello! I'm Housaky, your AGI assistant. How can I help you today?"
            } else {
                "Hi there! I'm Housaky. What would you like to work on?"
            };

        Ok(ActionDecision {
            action: SelectedAction::Respond {
                content: greeting_response.to_string(),
                needs_clarification: false,
                suggested_follow_ups: vec![
                    "Ask me a question".to_string(),
                    "Give me a task".to_string(),
                    "Tell me what you're working on".to_string(),
                ],
            },
            reasoning: "Greeting detected - responding with introduction".to_string(),
            confidence: 0.95,
            alternatives: vec![],
            risk_level: RiskLevel::None,
            estimated_impact: 0.1,
            requires_confirmation: false,
            confirmation_message: None,
            fallback: None,
        })
    }

    async fn select_farewell_action(&self, _perception: &PerceivedInput) -> Result<ActionDecision> {
        Ok(ActionDecision {
            action: SelectedAction::Respond {
                content: "Goodbye! Feel free to come back anytime. I'll be here when you need me."
                    .to_string(),
                needs_clarification: false,
                suggested_follow_ups: vec![],
            },
            reasoning: "Farewell detected".to_string(),
            confidence: 0.95,
            alternatives: vec![],
            risk_level: RiskLevel::None,
            estimated_impact: 0.1,
            requires_confirmation: false,
            confirmation_message: None,
            fallback: None,
        })
    }

    async fn select_task_action(
        &self,
        perception: &PerceivedInput,
        _uncertainty: &UncertaintyAssessment,
        _available_tools: &[&dyn Tool],
    ) -> Result<ActionDecision> {
        let priority = if perception.complexity > 0.7 {
            crate::goal_engine::GoalPriority::High
        } else if perception.complexity > 0.4 {
            crate::goal_engine::GoalPriority::Medium
        } else {
            crate::goal_engine::GoalPriority::Low
        };

        Ok(ActionDecision {
            action: SelectedAction::CreateGoal {
                title: perception.raw_input.clone(),
                description: format!("User task: {}", perception.raw_input),
                priority,
            },
            reasoning: "Task request detected - creating goal".to_string(),
            confidence: 0.8,
            alternatives: vec![],
            risk_level: RiskLevel::None,
            estimated_impact: 0.5,
            requires_confirmation: false,
            confirmation_message: None,
            fallback: None,
        })
    }

    async fn select_conversation_action(
        &self,
        perception: &PerceivedInput,
        uncertainty: &UncertaintyAssessment,
    ) -> Result<ActionDecision> {
        self.select_question_response(perception, uncertainty).await
    }

    fn extract_tool_arguments(&self, input: &str, tool_name: &str) -> serde_json::Value {
        let mut args = serde_json::Map::new();

        match tool_name {
            "shell" => {
                let cmd_pattern =
                    regex::Regex::new(r"(?:run|execute|cmd|command)[:\s]+(.+)").unwrap();
                if let Some(cap) = cmd_pattern.captures(input) {
                    args.insert(
                        "command".to_string(),
                        serde_json::Value::String(cap[1].to_string()),
                    );
                }
            }
            "file_read" | "file_write" => {
                let path_pattern =
                    regex::Regex::new(r#"(?:file|path)[:\s]+["']?([^\s"']+)["']?"#).unwrap();
                if let Some(cap) = path_pattern.captures(input) {
                    args.insert(
                        "path".to_string(),
                        serde_json::Value::String(cap[1].to_string()),
                    );
                }
            }
            "browser" | "http_request" => {
                let url_pattern = regex::Regex::new(r"https?://[^\s]+").unwrap();
                if let Some(cap) = url_pattern.find(input) {
                    args.insert(
                        "url".to_string(),
                        serde_json::Value::String(cap.as_str().to_string()),
                    );
                }
            }
            _ => {}
        }

        serde_json::Value::Object(args)
    }

    fn assess_tool_risk(&self, tool_name: &str, _args: &serde_json::Value) -> RiskLevel {
        match tool_name {
            "shell" => RiskLevel::High,
            "file_write" | "file_delete" => RiskLevel::Medium,
            "file_read" | "browser" | "http_request" => RiskLevel::Low,
            _ => RiskLevel::None,
        }
    }

    pub async fn record_outcome(&self, outcome: ActionOutcome) {
        let mut history = self.action_history.write().await;
        history.push(outcome);

        if history.len() > 1000 {
            history.remove(0);
        }
    }

    pub async fn get_action_stats(&self) -> ActionStats {
        let history = self.action_history.read().await;

        let total = history.len();
        let successful = history
            .iter()
            .filter(|o| matches!(o.result, ActionResult::Success { .. }))
            .count();
        let failed = history
            .iter()
            .filter(|o| matches!(o.result, ActionResult::Failure { .. }))
            .count();

        let avg_duration = if total > 0 {
            history.iter().map(|o| o.duration_ms).sum::<u64>() / total as u64
        } else {
            0
        };

        ActionStats {
            total_actions: total,
            successful_actions: successful,
            failed_actions: failed,
            success_rate: if total > 0 {
                successful as f64 / total as f64
            } else {
                0.0
            },
            average_duration_ms: avg_duration,
        }
    }
}

#[derive(Debug)]
struct LLMActionResult {
    action: Option<SelectedAction>,
    reasoning: Option<String>,
    confidence: Option<f64>,
    risk: Option<RiskLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStats {
    pub total_actions: usize,
    pub successful_actions: usize,
    pub failed_actions: usize,
    pub success_rate: f64,
    pub average_duration_ms: u64,
}

impl Default for ActionSelector {
    fn default() -> Self {
        Self::new()
    }
}

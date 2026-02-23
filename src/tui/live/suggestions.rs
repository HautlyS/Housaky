use crate::cognitive::perception::IntentType;
use crate::goal_engine::Goal;
use serde::{Deserialize, Serialize};

pub struct SuggestionEngine {
    context: SuggestionContext,
    rules: Vec<SuggestionRule>,
}

#[derive(Clone, Debug, Default)]
pub struct SuggestionContext {
    pub last_intent: Option<IntentType>,
    pub recent_topics: Vec<String>,
    pub active_goals: Vec<Goal>,
    pub conversation_length: usize,
    pub failed_actions: usize,
    pub user_sentiment: Sentiment,
    pub time_since_last_message: std::time::Duration,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Sentiment {
    #[default]
    Neutral,
    Positive,
    Negative,
    Frustrated,
    Curious,
}

#[derive(Clone, Debug)]
pub struct SuggestionRule {
    pub name: String,
    pub condition: SuggestionCondition,
    pub suggestion: Suggestion,
    pub priority: u32,
}

#[derive(Clone, Debug)]
pub enum SuggestionCondition {
    Always,
    AfterIntent(IntentType),
    AfterFailure,
    LongConversation,
    TopicMentioned(String),
    GoalProgressBelow(f64),
    SentimentIs(Sentiment),
    Idle(chrono::Duration),
    Combination(Vec<SuggestionCondition>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Suggestion {
    pub title: String,
    pub description: String,
    pub action: SuggestedAction,
    pub confidence: f64,
    pub category: SuggestionCategory,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SuggestedAction {
    AskQuestion(String),
    ProposeAction(String),
    OfferHelp(String),
    SuggestTopic(String),
    RemindGoal(String),
    ClarifyInput(String),
    SwitchMode(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SuggestionCategory {
    Proactive,
    Helpful,
    Clarification,
    FollowUp,
    Reminder,
    Exploration,
}

impl SuggestionEngine {
    pub fn new() -> Self {
        Self {
            context: SuggestionContext::default(),
            rules: Self::build_rules(),
        }
    }

    fn build_rules() -> Vec<SuggestionRule> {
        vec![
            SuggestionRule {
                name: "follow_up_question".to_string(),
                condition: SuggestionCondition::AfterIntent(IntentType::Question),
                suggestion: Suggestion {
                    title: "Follow up".to_string(),
                    description: "Ask a follow-up question for clarity".to_string(),
                    action: SuggestedAction::AskQuestion(
                        "Would you like me to elaborate on any part?".to_string(),
                    ),
                    confidence: 0.7,
                    category: SuggestionCategory::FollowUp,
                },
                priority: 10,
            },
            SuggestionRule {
                name: "error_recovery".to_string(),
                condition: SuggestionCondition::AfterFailure,
                suggestion: Suggestion {
                    title: "Error recovery".to_string(),
                    description: "Suggest alternative approach after failure".to_string(),
                    action: SuggestedAction::ProposeAction(
                        "Let me try a different approach...".to_string(),
                    ),
                    confidence: 0.9,
                    category: SuggestionCategory::Helpful,
                },
                priority: 50,
            },
            SuggestionRule {
                name: "long_conversation_break".to_string(),
                condition: SuggestionCondition::LongConversation,
                suggestion: Suggestion {
                    title: "Take a break".to_string(),
                    description: "Suggest summarizing after long conversation".to_string(),
                    action: SuggestedAction::ProposeAction(
                        "Would you like me to summarize what we've covered?".to_string(),
                    ),
                    confidence: 0.6,
                    category: SuggestionCategory::Helpful,
                },
                priority: 5,
            },
            SuggestionRule {
                name: "goal_reminder".to_string(),
                condition: SuggestionCondition::GoalProgressBelow(0.3),
                suggestion: Suggestion {
                    title: "Goal progress".to_string(),
                    description: "Remind about active goal with low progress".to_string(),
                    action: SuggestedAction::RemindGoal(
                        "I notice we have some goals that need attention.".to_string(),
                    ),
                    confidence: 0.75,
                    category: SuggestionCategory::Reminder,
                },
                priority: 20,
            },
            SuggestionRule {
                name: "frustrated_help".to_string(),
                condition: SuggestionCondition::SentimentIs(Sentiment::Frustrated),
                suggestion: Suggestion {
                    title: "Offer help".to_string(),
                    description: "Proactively offer assistance when frustrated".to_string(),
                    action: SuggestedAction::OfferHelp(
                        "I'm here to help. Let me try to assist you better.".to_string(),
                    ),
                    confidence: 0.85,
                    category: SuggestionCategory::Helpful,
                },
                priority: 40,
            },
            SuggestionRule {
                name: "idle_check_in".to_string(),
                condition: SuggestionCondition::Idle(chrono::Duration::minutes(5)),
                suggestion: Suggestion {
                    title: "Check in".to_string(),
                    description: "Check in after idle period".to_string(),
                    action: SuggestedAction::AskQuestion(
                        "Still there? Is there anything else I can help with?".to_string(),
                    ),
                    confidence: 0.5,
                    category: SuggestionCategory::Proactive,
                },
                priority: 15,
            },
            SuggestionRule {
                name: "curious_exploration".to_string(),
                condition: SuggestionCondition::SentimentIs(Sentiment::Curious),
                suggestion: Suggestion {
                    title: "Explore".to_string(),
                    description: "Suggest exploration when curious".to_string(),
                    action: SuggestedAction::SuggestTopic(
                        "We could explore related topics if you're interested.".to_string(),
                    ),
                    confidence: 0.7,
                    category: SuggestionCategory::Exploration,
                },
                priority: 25,
            },
        ]
    }

    pub fn update_context(&mut self, context: SuggestionContext) {
        self.context = context;
    }

    pub fn generate_suggestions(&self, max_suggestions: usize) -> Vec<Suggestion> {
        let mut matched: Vec<(u32, Suggestion)> = self
            .rules
            .iter()
            .filter_map(|rule| {
                if self.matches_condition(&rule.condition) {
                    Some((rule.priority, rule.suggestion.clone()))
                } else {
                    None
                }
            })
            .collect();

        matched.sort_by(|a, b| b.0.cmp(&a.0));
        matched
            .into_iter()
            .take(max_suggestions)
            .map(|(_, s)| s)
            .collect()
    }

    fn matches_condition(&self, condition: &SuggestionCondition) -> bool {
        match condition {
            SuggestionCondition::Always => true,
            SuggestionCondition::AfterIntent(intent) => {
                self.context.last_intent.as_ref() == Some(intent)
            }
            SuggestionCondition::AfterFailure => self.context.failed_actions > 0,
            SuggestionCondition::LongConversation => self.context.conversation_length > 20,
            SuggestionCondition::TopicMentioned(topic) => {
                self.context.recent_topics.iter().any(|t| t.contains(topic))
            }
            SuggestionCondition::GoalProgressBelow(threshold) => self
                .context
                .active_goals
                .iter()
                .any(|g| g.progress < *threshold),
            SuggestionCondition::SentimentIs(sentiment) => {
                self.context.user_sentiment == *sentiment
            }
            SuggestionCondition::Idle(duration) => {
                self.context.time_since_last_message
                    >= duration.to_std().unwrap_or(std::time::Duration::ZERO)
            }
            SuggestionCondition::Combination(conditions) => {
                conditions.iter().all(|c| self.matches_condition(c))
            }
        }
    }

    pub fn suggest_for_input(&self, input: &str) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        let input_lower = input.to_lowercase();

        if input_lower.split_whitespace().count() < 3 {
            suggestions.push(Suggestion {
                title: "More detail".to_string(),
                description: "Request more specific input".to_string(),
                action: SuggestedAction::ClarifyInput(
                    "Could you provide more details?".to_string(),
                ),
                confidence: 0.8,
                category: SuggestionCategory::Clarification,
            });
        }

        if input_lower.contains("how do i") || input_lower.contains("how to") {
            suggestions.push(Suggestion {
                title: "Step-by-step".to_string(),
                description: "Offer step-by-step guidance".to_string(),
                action: SuggestedAction::ProposeAction(
                    "I can walk you through this step by step.".to_string(),
                ),
                confidence: 0.85,
                category: SuggestionCategory::Helpful,
            });
        }

        if input_lower.contains("error")
            || input_lower.contains("bug")
            || input_lower.contains("not working")
        {
            suggestions.push(Suggestion {
                title: "Debug help".to_string(),
                description: "Offer debugging assistance".to_string(),
                action: SuggestedAction::ProposeAction(
                    "Let me help you debug this. Can you share more about the error?".to_string(),
                ),
                confidence: 0.9,
                category: SuggestionCategory::Helpful,
            });
        }

        suggestions
    }

    pub fn suggest_follow_ups(&self, last_response: &str) -> Vec<String> {
        let mut follow_ups = Vec::new();

        let response_lower = last_response.to_lowercase();

        if response_lower.contains("success") || response_lower.contains("completed") {
            follow_ups.push("What would you like to do next?".to_string());
        }

        if response_lower.contains("error") || response_lower.contains("failed") {
            follow_ups.push("Would you like me to try an alternative approach?".to_string());
        }

        if response_lower.contains('?') {
            follow_ups.push("Does that answer your question?".to_string());
        }

        if response_lower.len() > 500 {
            follow_ups.push("Would you like me to summarize this?".to_string());
        }

        follow_ups
    }

    pub fn format_suggestion(&self, suggestion: &Suggestion) -> String {
        match &suggestion.action {
            SuggestedAction::AskQuestion(q) => format!("💡 {}", q),
            SuggestedAction::ProposeAction(a) => format!("⚡ {}", a),
            SuggestedAction::OfferHelp(h) => format!("🤝 {}", h),
            SuggestedAction::SuggestTopic(t) => format!("🔍 {}", t),
            SuggestedAction::RemindGoal(g) => format!("🎯 {}", g),
            SuggestedAction::ClarifyInput(c) => format!("❓ {}", c),
            SuggestedAction::SwitchMode(m) => format!("🔄 {}", m),
        }
    }
}

impl Default for SuggestionEngine {
    fn default() -> Self {
        Self::new()
    }
}

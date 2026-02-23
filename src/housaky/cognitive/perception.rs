use crate::providers::Provider;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceivedInput {
    pub raw_input: String,
    pub intent: Intent,
    pub entities: Vec<Entity>,
    pub sentiment: Sentiment,
    pub complexity: f64,
    pub topics: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub context_clues: Vec<String>,
    pub ambiguity_level: f64,
    pub follow_up_needed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub primary: IntentType,
    pub secondary: Vec<IntentType>,
    pub confidence: f64,
    pub action_hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IntentType {
    Question,
    Command,
    Statement,
    Request,
    Clarification,
    Feedback,
    Greeting,
    Farewell,
    Task,
    Research,
    Creation,
    Modification,
    Analysis,
    Comparison,
    Debugging,
    Learning,
    Conversation,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub text: String,
    pub entity_type: EntityType,
    pub confidence: f64,
    pub position: (usize, usize),
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EntityType {
    Person,
    Location,
    Organization,
    Date,
    Time,
    Duration,
    Quantity,
    Percentage,
    Money,
    File,
    Code,
    URL,
    Email,
    PhoneNumber,
    Technology,
    Concept,
    Action,
    Tool,
    Variable,
    Function,
    Error,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sentiment {
    pub polarity: SentimentPolarity,
    pub intensity: f64,
    pub emotions: Vec<Emotion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SentimentPolarity {
    Positive,
    Negative,
    Neutral,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emotion {
    pub name: String,
    pub confidence: f64,
}

pub struct PerceptionEngine {
    entity_patterns: Vec<(regex::Regex, EntityType)>,
    intent_keywords: HashMap<IntentType, Vec<String>>,
    capability_keywords: HashMap<String, Vec<String>>,
}

impl PerceptionEngine {
    pub fn new() -> Self {
        Self {
            entity_patterns: Self::build_entity_patterns(),
            intent_keywords: Self::build_intent_keywords(),
            capability_keywords: Self::build_capability_keywords(),
        }
    }

    fn build_entity_patterns() -> Vec<(regex::Regex, EntityType)> {
        vec![
            (regex::Regex::new(r"\b\d{4}-\d{2}-\d{2}\b").unwrap(), EntityType::Date),
            (regex::Regex::new(r"\b\d{1,2}:\d{2}(?::\d{2})?(?:\s*[AP]M)?\b").unwrap(), EntityType::Time),
            (regex::Regex::new(r"\b\d+\s*(?:hours?|minutes?|seconds?|days?|weeks?|months?|years?)\b").unwrap(), EntityType::Duration),
            (regex::Regex::new(r"\b\d+(?:\.\d+)?%?\b").unwrap(), EntityType::Quantity),
            (regex::Regex::new(r"\$[\d,]+(?:\.\d{2})?\b").unwrap(), EntityType::Money),
            (regex::Regex::new(r"https?://[^\s]+").unwrap(), EntityType::URL),
            (regex::Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap(), EntityType::Email),
            (regex::Regex::new(r"\b\w+\.\w+\.\w+\b").unwrap(), EntityType::Code),
            (regex::Regex::new(r"\b(fn|function|def|class|struct|impl|trait|use|import|export|const|let|var)\s+\w+").unwrap(), EntityType::Code),
            (regex::Regex::new(r"\b(error|exception|failed|failure|bug|issue)\b").unwrap(), EntityType::Error),
            (regex::Regex::new(r"\b(rust|python|javascript|typescript|go|java|c\+\+|ruby|swift|kotlin)\b").unwrap(), EntityType::Technology),
        ]
    }

    fn build_intent_keywords() -> HashMap<IntentType, Vec<String>> {
        let mut map = HashMap::new();

        map.insert(
            IntentType::Question,
            vec![
                "what".to_string(),
                "how".to_string(),
                "why".to_string(),
                "when".to_string(),
                "where".to_string(),
                "who".to_string(),
                "which".to_string(),
                "can you".to_string(),
                "could you".to_string(),
                "is there".to_string(),
                "are there".to_string(),
                "does".to_string(),
                "do".to_string(),
                "will".to_string(),
                "would".to_string(),
                "should".to_string(),
                "could".to_string(),
                "?".to_string(),
            ],
        );

        map.insert(
            IntentType::Command,
            vec![
                "create".to_string(),
                "make".to_string(),
                "build".to_string(),
                "generate".to_string(),
                "write".to_string(),
                "delete".to_string(),
                "remove".to_string(),
                "update".to_string(),
                "modify".to_string(),
                "change".to_string(),
                "set".to_string(),
                "configure".to_string(),
                "enable".to_string(),
                "disable".to_string(),
                "run".to_string(),
                "execute".to_string(),
                "start".to_string(),
                "stop".to_string(),
                "restart".to_string(),
                "install".to_string(),
                "uninstall".to_string(),
                "deploy".to_string(),
            ],
        );

        map.insert(
            IntentType::Request,
            vec![
                "please".to_string(),
                "i need".to_string(),
                "i want".to_string(),
                "help me".to_string(),
                "can you help".to_string(),
                "would you mind".to_string(),
                "could you please".to_string(),
                "i'd like".to_string(),
                "let me have".to_string(),
                "give me".to_string(),
            ],
        );

        map.insert(
            IntentType::Analysis,
            vec![
                "analyze".to_string(),
                "examine".to_string(),
                "investigate".to_string(),
                "study".to_string(),
                "review".to_string(),
                "assess".to_string(),
                "evaluate".to_string(),
                "check".to_string(),
                "inspect".to_string(),
                "audit".to_string(),
                "compare".to_string(),
                "contrast".to_string(),
                "find".to_string(),
                "search".to_string(),
            ],
        );

        map.insert(
            IntentType::Research,
            vec![
                "research".to_string(),
                "look up".to_string(),
                "find information".to_string(),
                "search for".to_string(),
                "learn about".to_string(),
                "investigate".to_string(),
                "explore".to_string(),
                "discover".to_string(),
                "what is the latest".to_string(),
            ],
        );

        map.insert(
            IntentType::Debugging,
            vec![
                "debug".to_string(),
                "fix".to_string(),
                "solve".to_string(),
                "error".to_string(),
                "bug".to_string(),
                "issue".to_string(),
                "problem".to_string(),
                "not working".to_string(),
                "broken".to_string(),
                "crashes".to_string(),
                "fails".to_string(),
                "exception".to_string(),
            ],
        );

        map.insert(
            IntentType::Learning,
            vec![
                "teach me".to_string(),
                "explain".to_string(),
                "show me how".to_string(),
                "learn".to_string(),
                "understand".to_string(),
                "tutorial".to_string(),
                "guide".to_string(),
                "walk me through".to_string(),
                "help me understand".to_string(),
            ],
        );

        map.insert(
            IntentType::Greeting,
            vec![
                "hello".to_string(),
                "hi".to_string(),
                "hey".to_string(),
                "good morning".to_string(),
                "good afternoon".to_string(),
                "good evening".to_string(),
                "greetings".to_string(),
                "howdy".to_string(),
                "what's up".to_string(),
            ],
        );

        map.insert(
            IntentType::Farewell,
            vec![
                "bye".to_string(),
                "goodbye".to_string(),
                "see you".to_string(),
                "later".to_string(),
                "take care".to_string(),
                "farewell".to_string(),
                "good night".to_string(),
            ],
        );

        map.insert(
            IntentType::Clarification,
            vec![
                "i mean".to_string(),
                "actually".to_string(),
                "clarify".to_string(),
                "let me rephrase".to_string(),
                "what i meant".to_string(),
                "to be more specific".to_string(),
                "in other words".to_string(),
            ],
        );

        map.insert(
            IntentType::Feedback,
            vec![
                "good".to_string(),
                "bad".to_string(),
                "great".to_string(),
                "terrible".to_string(),
                "excellent".to_string(),
                "poor".to_string(),
                "perfect".to_string(),
                "wrong".to_string(),
                "correct".to_string(),
                "right".to_string(),
                "thanks".to_string(),
                "thank you".to_string(),
            ],
        );

        map
    }

    fn build_capability_keywords() -> HashMap<String, Vec<String>> {
        let mut map = HashMap::new();

        map.insert(
            "file_operations".to_string(),
            vec![
                "file".to_string(),
                "read".to_string(),
                "write".to_string(),
                "save".to_string(),
                "load".to_string(),
                "open".to_string(),
                "close".to_string(),
                "delete".to_string(),
                "rename".to_string(),
            ],
        );

        map.insert(
            "web_browsing".to_string(),
            vec![
                "website".to_string(),
                "url".to_string(),
                "web".to_string(),
                "search".to_string(),
                "browse".to_string(),
                "internet".to_string(),
                "online".to_string(),
                "page".to_string(),
            ],
        );

        map.insert(
            "code_generation".to_string(),
            vec![
                "code".to_string(),
                "function".to_string(),
                "class".to_string(),
                "module".to_string(),
                "script".to_string(),
                "program".to_string(),
                "implement".to_string(),
                "develop".to_string(),
                "write code".to_string(),
                "generate code".to_string(),
            ],
        );

        map.insert(
            "shell_execution".to_string(),
            vec![
                "command".to_string(),
                "terminal".to_string(),
                "shell".to_string(),
                "execute".to_string(),
                "run".to_string(),
                "script".to_string(),
                "bash".to_string(),
                "process".to_string(),
            ],
        );

        map.insert(
            "memory_operations".to_string(),
            vec![
                "remember".to_string(),
                "recall".to_string(),
                "forget".to_string(),
                "save".to_string(),
                "note".to_string(),
                "store".to_string(),
                "memory".to_string(),
            ],
        );

        map.insert(
            "data_analysis".to_string(),
            vec![
                "data".to_string(),
                "analyze".to_string(),
                "statistics".to_string(),
                "chart".to_string(),
                "graph".to_string(),
                "plot".to_string(),
                "metrics".to_string(),
            ],
        );

        map.insert(
            "api_integration".to_string(),
            vec![
                "api".to_string(),
                "endpoint".to_string(),
                "request".to_string(),
                "response".to_string(),
                "service".to_string(),
                "integration".to_string(),
            ],
        );

        map.insert(
            "reasoning".to_string(),
            vec![
                "think".to_string(),
                "reason".to_string(),
                "analyze".to_string(),
                "consider".to_string(),
                "evaluate".to_string(),
                "decide".to_string(),
                "why".to_string(),
            ],
        );

        map
    }

    pub async fn perceive(&self, input: &str) -> Result<PerceivedInput> {
        info!(
            "Perceiving input: {}",
            input.chars().take(50).collect::<String>()
        );

        let entities = self.extract_entities(input);
        let intent = self.detect_intent(input);
        let sentiment = self.analyze_sentiment(input);
        let complexity = self.calculate_complexity(input, &entities);
        let topics = self.extract_topics(input);
        let required_capabilities = self.detect_required_capabilities(input);
        let context_clues = self.extract_context_clues(input);
        let ambiguity_level = self.assess_ambiguity(input, &intent);
        let follow_up_needed = self.should_follow_up(input, &intent, ambiguity_level);

        Ok(PerceivedInput {
            raw_input: input.to_string(),
            intent,
            entities,
            sentiment,
            complexity,
            topics,
            required_capabilities,
            context_clues,
            ambiguity_level,
            follow_up_needed,
        })
    }

    pub async fn perceive_with_llm(
        &self,
        input: &str,
        provider: &dyn Provider,
        model: &str,
    ) -> Result<PerceivedInput> {
        let basic_perception = self.perceive(input).await?;

        let prompt = format!(
            r#"Analyze this user input for an AI assistant:

"{}"

Provide a JSON analysis with:
1. intent: primary intent (question/command/request/analysis/research/debugging/learning/conversation/greeting/farewell)
2. confidence: confidence score 0-1
3. topics: array of main topics
4. capabilities_needed: array of capabilities needed (file_operations/web_browsing/code_generation/shell_execution/memory_operations/data_analysis/api_integration/reasoning)
5. complexity: complexity score 0-1
6. ambiguity: ambiguity level 0-1
7. entities: array of named entities with types
8. follow_up: boolean if clarification needed

Return only valid JSON."#,
            input
        );

        let response = provider
            .chat_with_system(
                Some("You are a precise JSON analyzer. Return only valid JSON."),
                &prompt,
                model,
                0.1,
            )
            .await?;

        if let Ok(enhanced) = self.parse_llm_perception(&response) {
            Ok(PerceivedInput {
                raw_input: basic_perception.raw_input,
                intent: enhanced.intent.unwrap_or(basic_perception.intent),
                entities: if enhanced.entities.is_empty() {
                    basic_perception.entities
                } else {
                    enhanced.entities
                },
                sentiment: enhanced.sentiment.unwrap_or(basic_perception.sentiment),
                complexity: enhanced.complexity.unwrap_or(basic_perception.complexity),
                topics: if enhanced.topics.is_empty() {
                    basic_perception.topics
                } else {
                    enhanced.topics
                },
                required_capabilities: if enhanced.required_capabilities.is_empty() {
                    basic_perception.required_capabilities
                } else {
                    enhanced.required_capabilities
                },
                context_clues: basic_perception.context_clues,
                ambiguity_level: enhanced
                    .ambiguity_level
                    .unwrap_or(basic_perception.ambiguity_level),
                follow_up_needed: enhanced
                    .follow_up_needed
                    .unwrap_or(basic_perception.follow_up_needed),
            })
        } else {
            Ok(basic_perception)
        }
    }

    fn parse_llm_perception(&self, response: &str) -> Result<LLMPerceptionResult> {
        let json_str = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let parsed: serde_json::Value = serde_json::from_str(json_str)?;

        Ok(LLMPerceptionResult {
            intent: parsed.get("intent").and_then(|v| {
                let intent_str = v.as_str()?;
                let primary = match intent_str.to_lowercase().as_str() {
                    "question" => IntentType::Question,
                    "command" => IntentType::Command,
                    "request" => IntentType::Request,
                    "analysis" => IntentType::Analysis,
                    "research" => IntentType::Research,
                    "debugging" => IntentType::Debugging,
                    "learning" => IntentType::Learning,
                    "conversation" => IntentType::Conversation,
                    "greeting" => IntentType::Greeting,
                    "farewell" => IntentType::Farewell,
                    _ => IntentType::Unknown,
                };
                Some(Intent {
                    primary,
                    secondary: vec![],
                    confidence: parsed
                        .get("confidence")
                        .and_then(|c| c.as_f64())
                        .unwrap_or(0.8),
                    action_hints: vec![],
                })
            }),
            entities: parsed
                .get("entities")
                .and_then(|v| {
                    let arr = v.as_array()?;
                    Some(
                        arr.iter()
                            .filter_map(|e| {
                                Some(Entity {
                                    text: e.get("text")?.as_str()?.to_string(),
                                    entity_type: match e.get("type").and_then(|t| t.as_str()) {
                                        Some("person") => EntityType::Person,
                                        Some("location") => EntityType::Location,
                                        Some("organization") => EntityType::Organization,
                                        Some("date") => EntityType::Date,
                                        Some("technology") => EntityType::Technology,
                                        Some("code") => EntityType::Code,
                                        Some("file") => EntityType::File,
                                        Some("concept") => EntityType::Concept,
                                        _ => EntityType::Unknown,
                                    },
                                    confidence: e
                                        .get("confidence")
                                        .and_then(|c| c.as_f64())
                                        .unwrap_or(0.8),
                                    position: (0, 0),
                                    metadata: HashMap::new(),
                                })
                            })
                            .collect(),
                    )
                })
                .unwrap_or_default(),
            sentiment: None,
            complexity: parsed.get("complexity").and_then(|v| v.as_f64()),
            topics: parsed
                .get("topics")
                .and_then(|v| {
                    Some(
                        v.as_array()?
                            .iter()
                            .filter_map(|t| t.as_str().map(|s| s.to_string()))
                            .collect(),
                    )
                })
                .unwrap_or_default(),
            required_capabilities: parsed
                .get("capabilities_needed")
                .and_then(|v| {
                    Some(
                        v.as_array()?
                            .iter()
                            .filter_map(|c| c.as_str().map(|s| s.to_string()))
                            .collect(),
                    )
                })
                .unwrap_or_default(),
            ambiguity_level: parsed.get("ambiguity").and_then(|v| v.as_f64()),
            follow_up_needed: parsed.get("follow_up").and_then(|v| v.as_bool()),
        })
    }

    fn extract_entities(&self, input: &str) -> Vec<Entity> {
        let mut entities = Vec::new();

        for (pattern, entity_type) in &self.entity_patterns {
            for cap in pattern.find_iter(input) {
                entities.push(Entity {
                    text: cap.as_str().to_string(),
                    entity_type: entity_type.clone(),
                    confidence: 0.9,
                    position: (cap.start(), cap.end()),
                    metadata: HashMap::new(),
                });
            }
        }

        entities.sort_by(|a, b| a.position.0.cmp(&b.position.0));
        entities
    }

    fn detect_intent(&self, input: &str) -> Intent {
        let input_lower = input.to_lowercase();
        let mut scores: HashMap<IntentType, f64> = HashMap::new();

        for (intent_type, keywords) in &self.intent_keywords {
            let mut score = 0.0;
            for keyword in keywords {
                if input_lower.contains(keyword) {
                    score += 1.0;
                }
            }
            scores.insert(intent_type.clone(), score);
        }

        let mut sorted: Vec<_> = scores.iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));

        let primary = sorted
            .first()
            .map(|(intent, _)| (*intent).clone())
            .unwrap_or(IntentType::Unknown);

        let secondary: Vec<IntentType> = sorted
            .iter()
            .skip(1)
            .take(2)
            .filter(|(_, score)| **score > 0.0)
            .map(|(intent, _)| (*intent).clone())
            .collect();

        let confidence = if sorted.first().map(|(_, s)| **s).unwrap_or(0.0) > 0.0 {
            0.8
        } else {
            0.4
        };

        Intent {
            primary,
            secondary,
            confidence,
            action_hints: vec![],
        }
    }

    fn analyze_sentiment(&self, input: &str) -> Sentiment {
        let input_lower = input.to_lowercase();

        let positive_words = [
            "good",
            "great",
            "excellent",
            "perfect",
            "amazing",
            "wonderful",
            "thanks",
            "thank",
            "helpful",
            "love",
        ];
        let negative_words = [
            "bad", "terrible", "awful", "hate", "wrong", "error", "fail", "broken", "stupid",
            "useless",
        ];

        let positive_count = positive_words
            .iter()
            .filter(|w| input_lower.contains(*w))
            .count();
        let negative_count = negative_words
            .iter()
            .filter(|w| input_lower.contains(*w))
            .count();

        let polarity = if positive_count > negative_count {
            SentimentPolarity::Positive
        } else if negative_count > positive_count {
            SentimentPolarity::Negative
        } else if positive_count + negative_count > 0 {
            SentimentPolarity::Mixed
        } else {
            SentimentPolarity::Neutral
        };

        let intensity = (positive_count + negative_count) as f64 / 5.0;

        Sentiment {
            polarity,
            intensity: intensity.clamp(0.0, 1.0),
            emotions: vec![],
        }
    }

    fn calculate_complexity(&self, input: &str, entities: &[Entity]) -> f64 {
        let mut complexity = 0.0;

        let word_count = input.split_whitespace().count();
        complexity += (word_count as f64 / 50.0).min(0.3);

        let sentence_count = input.split(['.', '?', '!']).count();
        complexity += (sentence_count as f64 / 5.0).min(0.2);

        complexity += (entities.len() as f64 / 10.0).min(0.2);

        let has_code = input.contains("```") || input.contains("fn ") || input.contains("def ");
        if has_code {
            complexity += 0.2;
        }

        let has_conditions =
            input.contains("if ") || input.contains("when ") || input.contains("unless ");
        if has_conditions {
            complexity += 0.1;
        }

        complexity.min(1.0)
    }

    fn extract_topics(&self, input: &str) -> Vec<String> {
        let mut topics = Vec::new();

        let topic_patterns = [
            (
                regex::Regex::new(r"\b(rust|python|javascript|typescript|go|java)\b").unwrap(),
                "programming",
            ),
            (
                regex::Regex::new(r"\b(file|directory|path|folder)\b").unwrap(),
                "filesystem",
            ),
            (
                regex::Regex::new(r"\b(api|endpoint|request|response)\b").unwrap(),
                "api",
            ),
            (
                regex::Regex::new(r"\b(debug|error|exception|bug)\b").unwrap(),
                "debugging",
            ),
            (
                regex::Regex::new(r"\b(test|testing|spec|coverage)\b").unwrap(),
                "testing",
            ),
            (
                regex::Regex::new(r"\b(deploy|deployment|server|production)\b").unwrap(),
                "devops",
            ),
            (
                regex::Regex::new(r"\b(database|sql|query|table)\b").unwrap(),
                "database",
            ),
            (
                regex::Regex::new(r"\b(learn|teach|explain|tutorial)\b").unwrap(),
                "learning",
            ),
        ];

        for (pattern, topic) in topic_patterns {
            if pattern.is_match(input) {
                topics.push(topic.to_string());
            }
        }

        topics.sort();
        topics.dedup();
        topics
    }

    fn detect_required_capabilities(&self, input: &str) -> Vec<String> {
        let input_lower = input.to_lowercase();
        let mut capabilities = Vec::new();

        for (capability, keywords) in &self.capability_keywords {
            for keyword in keywords {
                if input_lower.contains(keyword) {
                    capabilities.push(capability.clone());
                    break;
                }
            }
        }

        capabilities.sort();
        capabilities.dedup();
        capabilities
    }

    fn extract_context_clues(&self, input: &str) -> Vec<String> {
        let mut clues = Vec::new();

        let patterns = [
            (
                regex::Regex::new(r"\bprevious(ly)?\b").unwrap(),
                "previous_context",
            ),
            (regex::Regex::new(r"\blater\b").unwrap(), "future_context"),
            (regex::Regex::new(r"\bagain\b").unwrap(), "repetition"),
            (regex::Regex::new(r"\binstead\b").unwrap(), "alternative"),
            (regex::Regex::new(r"\balso\b").unwrap(), "addition"),
            (regex::Regex::new(r"\bbut\b").unwrap(), "contrast"),
        ];

        for (pattern, clue) in patterns {
            if pattern.is_match(input) {
                clues.push(clue.to_string());
            }
        }

        clues
    }

    fn assess_ambiguity(&self, input: &str, intent: &Intent) -> f64 {
        let mut ambiguity: f64 = 0.0;

        if intent.confidence < 0.6 {
            ambiguity += 0.3;
        }

        let has_pronouns = regex::Regex::new(r"\b(it|that|this|they|them|those|these)\b")
            .unwrap()
            .is_match(input);
        if has_pronouns {
            ambiguity += 0.2;
        }

        let is_vague = input.len() < 20 || input.split_whitespace().count() < 4;
        if is_vague {
            ambiguity += 0.3;
        }

        let has_ambiguity_markers =
            input.contains("maybe") || input.contains("perhaps") || input.contains("might");
        if has_ambiguity_markers {
            ambiguity += 0.2;
        }

        ambiguity.min(1.0)
    }

    fn should_follow_up(&self, input: &str, intent: &Intent, ambiguity: f64) -> bool {
        if ambiguity > 0.5 {
            return true;
        }

        if intent.primary == IntentType::Unknown {
            return true;
        }

        let question_words = ["what", "which", "how"];
        let has_question = question_words
            .iter()
            .any(|w| input.to_lowercase().contains(w));
        let has_question_mark = input.contains('?');

        if has_question && !has_question_mark && input.split_whitespace().count() < 5 {
            return true;
        }

        false
    }
}

#[derive(Debug)]
struct LLMPerceptionResult {
    intent: Option<Intent>,
    entities: Vec<Entity>,
    sentiment: Option<Sentiment>,
    complexity: Option<f64>,
    topics: Vec<String>,
    required_capabilities: Vec<String>,
    ambiguity_level: Option<f64>,
    follow_up_needed: Option<bool>,
}

impl Default for PerceptionEngine {
    fn default() -> Self {
        Self::new()
    }
}

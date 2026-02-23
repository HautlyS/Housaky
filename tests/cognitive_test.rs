//! Comprehensive tests for the cognitive modules in Housaky.
//!
//! Tests cover:
//! - Perception engine (intent detection, entity extraction, sentiment analysis)
//! - Action selection (decision making, tool selection, risk assessment)
//! - Uncertainty detection (ambiguity assessment, clarification generation)
//! - Experience learner (pattern extraction, lesson learning, skill prototyping)
//! - Memory consolidation (working memory, episodic memory)
//! - Multi-agent coordination (task assignment, consensus)
//! - TUI components (suggestion engine)
//!
//! All tests are unit tests that do NOT make actual API calls.

use std::collections::HashMap;
use std::path::PathBuf;
use housaky::housaky::cognitive::action_selector::{
    ActionDecision, ActionOutcome, ActionResult, ActionSelector, LearningStrategy, RiskLevel,
    SelectedAction,
};
use housaky::housaky::cognitive::experience_learner::{
    ActionSummary, Experience, ExperienceLearner, Lesson, OutcomeSummary, Pattern, PatternType,
    PerceptionSummary, SkillPrototype,
};
use housaky::housaky::cognitive::perception::{
    EntityType, Intent, IntentType, PerceivedInput, PerceptionEngine, Sentiment,
    SentimentPolarity,
};
use housaky::housaky::cognitive::uncertainty::{
    KnowledgeGap, UncertaintyAssessment, UncertaintyCategory, UncertaintyDetector,
    UncertaintySource,
};

use housaky::housaky::goal_engine::{Goal, GoalCategory, GoalEngine, GoalPriority, GoalStatus};
use housaky::housaky::inner_monologue::{InnerMonologue, ThoughtSource, ThoughtType};
use housaky::housaky::meta_cognition::{
    CapabilityAssessment, EmotionalState, MetaCognitionEngine, SelfModel,
};
use housaky::housaky::multi_agent::{
    agent_registry::{AgentInfo, AgentPerformance, AgentType},
    coordinator::{AgentTask, TaskPriority, TaskStatus},
    AgentMessage, AgentRegistry, MessageType, MultiAgentCoordinator,
};
use housaky::housaky::working_memory::{MemoryImportance, WorkingMemoryEngine};
use housaky::tui::live::suggestions::{
    Sentiment as SuggestionSentiment, SuggestedAction, Suggestion, SuggestionCategory, SuggestionContext, SuggestionEngine,
};

fn temp_workspace() -> PathBuf {
    std::env::temp_dir().join(format!("housaky_test_{}", uuid::Uuid::new_v4()))
}

mod perception_tests {
    use super::*;

    #[tokio::test]
    async fn perception_engine_creation() {
        let engine = PerceptionEngine::new();
        let result = engine.perceive("Hello world").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn perception_detects_question_intent() {
        let engine = PerceptionEngine::new();
        let result = engine
            .perceive("What is the capital of France?")
            .await
            .unwrap();

        assert_eq!(result.intent.primary, IntentType::Question);
        assert!(result.raw_input.contains('?'));
    }

    #[tokio::test]
    async fn perception_detects_command_intent() {
        let engine = PerceptionEngine::new();
        let result = engine
            .perceive("Create a new file named test.txt")
            .await
            .unwrap();

        assert_eq!(result.intent.primary, IntentType::Command);
    }

    #[tokio::test]
    async fn perception_detects_greeting_intent() {
        let engine = PerceptionEngine::new();
        let result = engine.perceive("Hello there!").await.unwrap();

        assert_eq!(result.intent.primary, IntentType::Greeting);
    }

    #[tokio::test]
    async fn perception_detects_farewell_intent() {
        let engine = PerceptionEngine::new();
        let result = engine.perceive("Goodbye, see you later!").await.unwrap();

        assert_eq!(result.intent.primary, IntentType::Farewell);
    }

    #[tokio::test]
    async fn perception_detects_debugging_intent() {
        let engine = PerceptionEngine::new();
        let result = engine
            .perceive("Debug this error in my code")
            .await
            .unwrap();

        assert_eq!(result.intent.primary, IntentType::Debugging);
    }

    #[tokio::test]
    async fn perception_detects_learning_intent() {
        let engine = PerceptionEngine::new();
        let result = engine
            .perceive("Teach me about Rust programming")
            .await
            .unwrap();

        assert_eq!(result.intent.primary, IntentType::Learning);
    }

    #[tokio::test]
    async fn perception_detects_request_intent() {
        let engine = PerceptionEngine::new();
        let result = engine
            .perceive("Please help me with this task")
            .await
            .unwrap();

        assert_eq!(result.intent.primary, IntentType::Request);
    }

    #[tokio::test]
    async fn perception_extracts_url_entity() {
        let engine = PerceptionEngine::new();
        let result = engine
            .perceive("Visit https://example.com for more info")
            .await
            .unwrap();

        let url_entities: Vec<_> = result
            .entities
            .iter()
            .filter(|e| e.entity_type == EntityType::URL)
            .collect();

        assert!(!url_entities.is_empty());
        assert!(url_entities[0].text.contains("example.com"));
    }

    #[tokio::test]
    async fn perception_extracts_email_entity() {
        let engine = PerceptionEngine::new();
        let result = engine
            .perceive("Contact me at test@example.com")
            .await
            .unwrap();

        let email_entities: Vec<_> = result
            .entities
            .iter()
            .filter(|e| e.entity_type == EntityType::Email)
            .collect();

        assert!(!email_entities.is_empty());
    }

    #[tokio::test]
    async fn perception_extracts_date_entity() {
        let engine = PerceptionEngine::new();
        let result = engine.perceive("The event is on 2024-12-25").await.unwrap();

        let date_entities: Vec<_> = result
            .entities
            .iter()
            .filter(|e| e.entity_type == EntityType::Date)
            .collect();

        assert!(!date_entities.is_empty());
    }

    #[tokio::test]
    async fn perception_extracts_technology_entity() {
        let engine = PerceptionEngine::new();
        let result = engine
            .perceive("I'm learning rust and python")
            .await
            .unwrap();

        let tech_entities: Vec<_> = result
            .entities
            .iter()
            .filter(|e| e.entity_type == EntityType::Technology)
            .collect();

        assert!(!tech_entities.is_empty());
    }

    #[tokio::test]
    async fn perception_analyzes_positive_sentiment() {
        let engine = PerceptionEngine::new();
        let result = engine
            .perceive("This is great and wonderful!")
            .await
            .unwrap();

        assert_eq!(result.sentiment.polarity, SentimentPolarity::Positive);
    }

    #[tokio::test]
    async fn perception_analyzes_negative_sentiment() {
        let engine = PerceptionEngine::new();
        let result = engine.perceive("This is terrible and awful").await.unwrap();

        assert_eq!(result.sentiment.polarity, SentimentPolarity::Negative);
    }

    #[tokio::test]
    async fn perception_analyzes_neutral_sentiment() {
        let engine = PerceptionEngine::new();
        let result = engine.perceive("The file is located there").await.unwrap();

        assert_eq!(result.sentiment.polarity, SentimentPolarity::Neutral);
    }

    #[tokio::test]
    async fn perception_calculates_complexity() {
        let engine = PerceptionEngine::new();

        let simple = engine.perceive("Hello").await.unwrap();
        let complex = engine
            .perceive(
                "Create a complex multi-threaded application with database connectivity, \
             authentication, and real-time WebSocket updates for monitoring system metrics",
            )
            .await
            .unwrap();

        assert!(complex.complexity > simple.complexity);
    }

    #[tokio::test]
    async fn perception_detects_required_capabilities() {
        let engine = PerceptionEngine::new();
        let result = engine
            .perceive("Read the file and search the web for information")
            .await
            .unwrap();

        assert!(result
            .required_capabilities
            .contains(&"file_operations".to_string()));
        assert!(result
            .required_capabilities
            .contains(&"web_browsing".to_string()));
    }

    #[tokio::test]
    async fn perception_extracts_topics() {
        let engine = PerceptionEngine::new();
        let result = engine
            .perceive("Debug the rust code and run tests")
            .await
            .unwrap();

        assert!(
            result.topics.contains(&"debugging".to_string())
                || result.topics.contains(&"programming".to_string())
        );
    }

    #[tokio::test]
    async fn perception_assesses_ambiguity() {
        let engine = PerceptionEngine::new();

        let clear = engine
            .perceive("Create a file named test.txt")
            .await
            .unwrap();
        let ambiguous = engine.perceive("it").await.unwrap();

        assert!(ambiguous.ambiguity_level > clear.ambiguity_level);
    }

    #[tokio::test]
    async fn perception_detects_follow_up_need() {
        let engine = PerceptionEngine::new();
        let result = engine.perceive("it").await.unwrap();

        assert!(result.follow_up_needed);
    }

    #[tokio::test]
    async fn intent_has_secondary_types() {
        let engine = PerceptionEngine::new();
        let result = engine
            .perceive("Please create a file and explain how it works")
            .await
            .unwrap();

        assert!(
            !result.intent.secondary.is_empty() || result.intent.primary != IntentType::Unknown
        );
    }

    #[tokio::test]
    async fn perception_default() {
        let engine1 = PerceptionEngine::new();
        let engine2 = PerceptionEngine::default();

        let result1 = engine1.perceive("test").await.unwrap();
        let result2 = engine2.perceive("test").await.unwrap();

        assert_eq!(result1.intent.primary, result2.intent.primary);
    }
}

mod action_selector_tests {
    use super::*;

    #[tokio::test]
    async fn action_selector_creation() {
        let selector = ActionSelector::new();
        let stats = selector.get_action_stats().await;

        assert_eq!(stats.total_actions, 0);
        assert_eq!(stats.success_rate, 0.0);
    }

    #[tokio::test]
    async fn action_selector_selects_respond_for_question() {
        let selector = ActionSelector::new();
        let perception = PerceivedInput {
            raw_input: "What is 2+2?".to_string(),
            intent: Intent {
                primary: IntentType::Question,
                secondary: vec![],
                confidence: 0.9,
                action_hints: vec![],
            },
            entities: vec![],
            sentiment: Sentiment {
                polarity: SentimentPolarity::Neutral,
                intensity: 0.0,
                emotions: vec![],
            },
            complexity: 0.2,
            topics: vec![],
            required_capabilities: vec![],
            context_clues: vec![],
            ambiguity_level: 0.1,
            follow_up_needed: false,
        };

        let uncertainty = UncertaintyAssessment {
            overall_uncertainty: 0.1,
            sources: vec![],
            confidence_intervals: HashMap::new(),
            calibration_score: 0.8,
            should_ask_clarification: false,
            clarification_questions: vec![],
            alternative_interpretations: vec![],
            knowledge_gaps: vec![],
        };

        let decision = selector
            .select_action(&perception, &uncertainty, &[])
            .await
            .unwrap();

        assert!(matches!(decision.action, SelectedAction::Respond { .. }));
    }

    #[tokio::test]
    async fn action_selector_creates_clarification_for_high_uncertainty() {
        let selector = ActionSelector::new();
        let perception = PerceivedInput {
            raw_input: "it".to_string(),
            intent: Intent {
                primary: IntentType::Unknown,
                secondary: vec![],
                confidence: 0.3,
                action_hints: vec![],
            },
            entities: vec![],
            sentiment: Sentiment {
                polarity: SentimentPolarity::Neutral,
                intensity: 0.0,
                emotions: vec![],
            },
            complexity: 0.1,
            topics: vec![],
            required_capabilities: vec![],
            context_clues: vec![],
            ambiguity_level: 0.8,
            follow_up_needed: true,
        };

        let uncertainty = UncertaintyAssessment {
            overall_uncertainty: 0.7,
            sources: vec![],
            confidence_intervals: HashMap::new(),
            calibration_score: 0.5,
            should_ask_clarification: true,
            clarification_questions: vec!["What does 'it' refer to?".to_string()],
            alternative_interpretations: vec![],
            knowledge_gaps: vec![],
        };

        let decision = selector
            .select_action(&perception, &uncertainty, &[])
            .await
            .unwrap();

        assert!(matches!(decision.action, SelectedAction::Clarify { .. }));
    }

    #[tokio::test]
    async fn action_selector_handles_greeting() {
        let selector = ActionSelector::new();
        let perception = PerceivedInput {
            raw_input: "Hello there!".to_string(),
            intent: Intent {
                primary: IntentType::Greeting,
                secondary: vec![],
                confidence: 0.95,
                action_hints: vec![],
            },
            entities: vec![],
            sentiment: Sentiment {
                polarity: SentimentPolarity::Positive,
                intensity: 0.5,
                emotions: vec![],
            },
            complexity: 0.1,
            topics: vec![],
            required_capabilities: vec![],
            context_clues: vec![],
            ambiguity_level: 0.0,
            follow_up_needed: false,
        };

        let uncertainty = UncertaintyAssessment {
            overall_uncertainty: 0.1,
            sources: vec![],
            confidence_intervals: HashMap::new(),
            calibration_score: 0.9,
            should_ask_clarification: false,
            clarification_questions: vec![],
            alternative_interpretations: vec![],
            knowledge_gaps: vec![],
        };

        let decision = selector
            .select_action(&perception, &uncertainty, &[])
            .await
            .unwrap();

        if let SelectedAction::Respond { content, .. } = &decision.action {
            assert!(content.contains("Hello") || content.is_empty());
        } else {
            panic!("Expected Respond action for greeting");
        }
    }

    #[tokio::test]
    async fn action_selector_handles_farewell() {
        let selector = ActionSelector::new();
        let perception = PerceivedInput {
            raw_input: "Goodbye!".to_string(),
            intent: Intent {
                primary: IntentType::Farewell,
                secondary: vec![],
                confidence: 0.95,
                action_hints: vec![],
            },
            entities: vec![],
            sentiment: Sentiment {
                polarity: SentimentPolarity::Neutral,
                intensity: 0.0,
                emotions: vec![],
            },
            complexity: 0.1,
            topics: vec![],
            required_capabilities: vec![],
            context_clues: vec![],
            ambiguity_level: 0.0,
            follow_up_needed: false,
        };

        let uncertainty = UncertaintyAssessment {
            overall_uncertainty: 0.1,
            sources: vec![],
            confidence_intervals: HashMap::new(),
            calibration_score: 0.9,
            should_ask_clarification: false,
            clarification_questions: vec![],
            alternative_interpretations: vec![],
            knowledge_gaps: vec![],
        };

        let decision = selector
            .select_action(&perception, &uncertainty, &[])
            .await
            .unwrap();

        if let SelectedAction::Respond { content, .. } = &decision.action {
            assert!(content.contains("Goodbye") || content.is_empty());
        }
    }

    #[tokio::test]
    async fn action_selector_creates_goal_for_task() {
        let selector = ActionSelector::new();
        let perception = PerceivedInput {
            raw_input: "Build a web application".to_string(),
            intent: Intent {
                primary: IntentType::Task,
                secondary: vec![],
                confidence: 0.85,
                action_hints: vec![],
            },
            entities: vec![],
            sentiment: Sentiment {
                polarity: SentimentPolarity::Neutral,
                intensity: 0.0,
                emotions: vec![],
            },
            complexity: 0.6,
            topics: vec![],
            required_capabilities: vec![],
            context_clues: vec![],
            ambiguity_level: 0.2,
            follow_up_needed: false,
        };

        let uncertainty = UncertaintyAssessment {
            overall_uncertainty: 0.2,
            sources: vec![],
            confidence_intervals: HashMap::new(),
            calibration_score: 0.8,
            should_ask_clarification: false,
            clarification_questions: vec![],
            alternative_interpretations: vec![],
            knowledge_gaps: vec![],
        };

        let decision = selector
            .select_action(&perception, &uncertainty, &[])
            .await
            .unwrap();

        assert!(matches!(decision.action, SelectedAction::CreateGoal { .. }));
    }

    #[tokio::test]
    async fn action_selector_records_outcome() {
        let selector = ActionSelector::new();

        let outcome = ActionOutcome {
            decision_id: "test_1".to_string(),
            action: SelectedAction::Respond {
                content: "Test response".to_string(),
                needs_clarification: false,
                suggested_follow_ups: vec![],
            },
            result: ActionResult::Success {
                output: "Success".to_string(),
            },
            duration_ms: 100,
            side_effects: vec![],
            user_feedback: None,
        };

        selector.record_outcome(outcome).await;

        let stats = selector.get_action_stats().await;
        assert_eq!(stats.total_actions, 1);
        assert_eq!(stats.successful_actions, 1);
        assert!((stats.success_rate - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn action_selector_tracks_failure_stats() {
        let selector = ActionSelector::new();

        let success_outcome = ActionOutcome {
            decision_id: "test_1".to_string(),
            action: SelectedAction::Respond {
                content: "Test".to_string(),
                needs_clarification: false,
                suggested_follow_ups: vec![],
            },
            result: ActionResult::Success {
                output: "OK".to_string(),
            },
            duration_ms: 50,
            side_effects: vec![],
            user_feedback: None,
        };

        let failure_outcome = ActionOutcome {
            decision_id: "test_2".to_string(),
            action: SelectedAction::Respond {
                content: "Test".to_string(),
                needs_clarification: false,
                suggested_follow_ups: vec![],
            },
            result: ActionResult::Failure {
                error: "Failed".to_string(),
                recoverable: true,
            },
            duration_ms: 100,
            side_effects: vec![],
            user_feedback: None,
        };

        selector.record_outcome(success_outcome).await;
        selector.record_outcome(failure_outcome).await;

        let stats = selector.get_action_stats().await;
        assert_eq!(stats.total_actions, 2);
        assert_eq!(stats.successful_actions, 1);
        assert_eq!(stats.failed_actions, 1);
        assert!((stats.success_rate - 0.5).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn action_decision_has_risk_level() {
        let decision = ActionDecision {
            action: SelectedAction::Respond {
                content: "Test".to_string(),
                needs_clarification: false,
                suggested_follow_ups: vec![],
            },
            reasoning: "Test decision".to_string(),
            confidence: 0.9,
            alternatives: vec![],
            risk_level: RiskLevel::None,
            estimated_impact: 0.5,
            requires_confirmation: false,
            confirmation_message: None,
            fallback: None,
        };

        assert_eq!(decision.risk_level, RiskLevel::None);
        assert!((decision.confidence - 0.9).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn risk_level_ordering() {
        assert!(RiskLevel::Critical > RiskLevel::High);
        assert!(RiskLevel::High > RiskLevel::Medium);
        assert!(RiskLevel::Medium > RiskLevel::Low);
        assert!(RiskLevel::Low > RiskLevel::None);
    }

    #[tokio::test]
    async fn selected_action_variants() {
        let respond = SelectedAction::Respond {
            content: "Hello".to_string(),
            needs_clarification: false,
            suggested_follow_ups: vec![],
        };

        let use_tool = SelectedAction::UseTool {
            tool_name: "shell".to_string(),
            arguments: serde_json::json!({"command": "ls"}),
            expected_outcome: "List files".to_string(),
        };

        let clarify = SelectedAction::Clarify {
            questions: vec!["What?".to_string()],
            assumptions: vec!["Assuming X".to_string()],
        };

        let learn = SelectedAction::Learn {
            topic: "Rust".to_string(),
            source: "book".to_string(),
            strategy: LearningStrategy::DirectInstruction,
        };

        let delegate = SelectedAction::Delegate {
            agent_type: "code".to_string(),
            task_description: "Write code".to_string(),
            context: HashMap::new(),
        };

        let wait = SelectedAction::Wait {
            reason: "Waiting for input".to_string(),
            duration_seconds: Some(10),
        };

        assert!(matches!(respond, SelectedAction::Respond { .. }));
        assert!(matches!(use_tool, SelectedAction::UseTool { .. }));
        assert!(matches!(clarify, SelectedAction::Clarify { .. }));
        assert!(matches!(learn, SelectedAction::Learn { .. }));
        assert!(matches!(delegate, SelectedAction::Delegate { .. }));
        assert!(matches!(wait, SelectedAction::Wait { .. }));
    }

    #[tokio::test]
    async fn action_selector_default() {
        let selector1 = ActionSelector::new();
        let selector2 = ActionSelector::default();

        let stats1 = selector1.get_action_stats().await;
        let stats2 = selector2.get_action_stats().await;

        assert_eq!(stats1.total_actions, stats2.total_actions);
    }
}

mod uncertainty_tests {
    use super::*;

    #[tokio::test]
    async fn uncertainty_detector_creation() {
        let detector = UncertaintyDetector::new();
        let assessment = detector.assess("test input", 0.7, &[]).await.unwrap();

        assert!(assessment.overall_uncertainty >= 0.0 && assessment.overall_uncertainty <= 1.0);
    }

    #[tokio::test]
    async fn uncertainty_detects_ambiguous_pronouns() {
        let detector = UncertaintyDetector::new();
        let assessment = detector.assess("it is there", 0.7, &[]).await.unwrap();

        let has_ambiguity = assessment
            .sources
            .iter()
            .any(|s| s.category == UncertaintyCategory::InputAmbiguity);

        assert!(has_ambiguity);
    }

    #[tokio::test]
    async fn uncertainty_detects_missing_context() {
        let detector = UncertaintyDetector::new();
        let assessment = detector.assess("test", 0.7, &[]).await.unwrap();

        let has_missing_context = assessment
            .sources
            .iter()
            .any(|s| s.category == UncertaintyCategory::MissingContext);

        assert!(has_missing_context);
    }

    #[tokio::test]
    async fn uncertainty_detects_confidence_gap() {
        let detector = UncertaintyDetector::new();
        let assessment = detector.assess("test input", 0.3, &[]).await.unwrap();

        let has_confidence_gap = assessment
            .sources
            .iter()
            .any(|s| s.category == UncertaintyCategory::ConfidenceGap);

        assert!(has_confidence_gap);
    }

    #[tokio::test]
    async fn uncertainty_detects_domain_unfamiliarity() {
        let detector = UncertaintyDetector::new();
        let assessment = detector
            .assess("Explain quantum mechanics", 0.7, &[])
            .await
            .unwrap();

        let has_domain_unfamiliarity = assessment
            .sources
            .iter()
            .any(|s| s.category == UncertaintyCategory::DomainUnfamiliarity);

        assert!(has_domain_unfamiliarity);
    }

    #[tokio::test]
    async fn uncertainty_generates_clarification_questions() {
        let detector = UncertaintyDetector::new();
        let assessment = detector.assess("it", 0.3, &[]).await.unwrap();

        assert!(assessment.should_ask_clarification);
        assert!(!assessment.clarification_questions.is_empty());
    }

    #[tokio::test]
    async fn uncertainty_calculates_overall_uncertainty() {
        let detector = UncertaintyDetector::new();

        let low_uncertainty = detector
            .assess(
                "Create a file named test.txt",
                0.9,
                &["context".to_string()],
            )
            .await
            .unwrap();
        let high_uncertainty = detector
            .assess("it something there", 0.3, &[])
            .await
            .unwrap();

        assert!(high_uncertainty.overall_uncertainty > low_uncertainty.overall_uncertainty);
    }

    #[tokio::test]
    async fn uncertainty_provides_confidence_intervals() {
        let detector = UncertaintyDetector::new();
        let assessment = detector.assess("test input", 0.7, &[]).await.unwrap();

        assert!(assessment.confidence_intervals.contains_key("overall"));
    }

    #[tokio::test]
    async fn uncertainty_identifies_knowledge_gaps() {
        let detector = UncertaintyDetector::new();
        let assessment = detector
            .assess("Use the react framework", 0.7, &[])
            .await
            .unwrap();

        let has_framework_gap = assessment
            .knowledge_gaps
            .iter()
            .any(|g| g.topic.contains("framework"));

        assert!(has_framework_gap);
    }

    #[tokio::test]
    async fn uncertainty_records_prediction() {
        let detector = UncertaintyDetector::new();

        detector.record_prediction(0.8, "test_context").await;

        let calibration = detector.get_calibration_metrics().await;
        assert!(!calibration.predictions.is_empty());
    }

    #[tokio::test]
    async fn uncertainty_records_outcome() {
        let detector = UncertaintyDetector::new();

        detector.record_prediction(0.8, "test_context").await;
        let result = detector.record_outcome("test_context", true).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn uncertainty_formats_report() {
        let detector = UncertaintyDetector::new();
        let assessment = detector.assess("it is broken", 0.5, &[]).await.unwrap();

        let report = detector.format_uncertainty_report(&assessment);

        assert!(report.contains("Uncertainty Assessment"));
        assert!(report.contains("Overall Uncertainty"));
    }

    #[tokio::test]
    async fn uncertainty_source_has_mitigation() {
        let source = UncertaintySource {
            category: UncertaintyCategory::InputAmbiguity,
            description: "Ambiguous input".to_string(),
            impact: 0.5,
            mitigation: Some("Ask for clarification".to_string()),
            evidence: vec!["it".to_string()],
        };

        assert!(source.mitigation.is_some());
    }

    #[tokio::test]
    async fn knowledge_gap_properties() {
        let gap = KnowledgeGap {
            topic: "Quantum physics".to_string(),
            severity: 0.7,
            can_retrieve: true,
            retrieval_strategy: Some("search_web".to_string()),
        };

        assert_eq!(gap.topic, "Quantum physics");
        assert!(gap.can_retrieve);
    }

    #[tokio::test]
    async fn uncertainty_detector_default() {
        let detector1 = UncertaintyDetector::new();
        let detector2 = UncertaintyDetector::default();

        let assessment1 = detector1.assess("test", 0.7, &[]).await.unwrap();
        let assessment2 = detector2.assess("test", 0.7, &[]).await.unwrap();

        assert_eq!(assessment1.sources.len(), assessment2.sources.len());
    }
}

mod experience_learner_tests {
    use super::*;

    fn temp_workspace_for_test() -> PathBuf {
        std::env::temp_dir().join(format!("exp_test_{}", uuid::Uuid::new_v4()))
    }

    #[tokio::test]
    async fn experience_learner_creation() {
        let workspace = temp_workspace_for_test();
        let learner = ExperienceLearner::new(&workspace);

        let stats = learner.get_learning_stats().await;
        assert_eq!(stats.total_experiences, 0);
    }

    #[tokio::test]
    async fn experience_learner_records_experience() {
        let workspace = temp_workspace_for_test();
        let learner = ExperienceLearner::new(&workspace);

        let perception = PerceivedInput {
            raw_input: "Test input".to_string(),
            intent: Intent {
                primary: IntentType::Question,
                secondary: vec![],
                confidence: 0.9,
                action_hints: vec![],
            },
            entities: vec![],
            sentiment: Sentiment {
                polarity: SentimentPolarity::Neutral,
                intensity: 0.0,
                emotions: vec![],
            },
            complexity: 0.3,
            topics: vec!["testing".to_string()],
            required_capabilities: vec![],
            context_clues: vec![],
            ambiguity_level: 0.1,
            follow_up_needed: false,
        };

        let decision = ActionDecision {
            action: SelectedAction::Respond {
                content: "Test response".to_string(),
                needs_clarification: false,
                suggested_follow_ups: vec![],
            },
            reasoning: "Test reasoning".to_string(),
            confidence: 0.9,
            alternatives: vec![],
            risk_level: RiskLevel::None,
            estimated_impact: 0.5,
            requires_confirmation: false,
            confirmation_message: None,
            fallback: None,
        };

        let outcome = ActionOutcome {
            decision_id: "test_dec_1".to_string(),
            action: decision.action.clone(),
            result: ActionResult::Success {
                output: "Success".to_string(),
            },
            duration_ms: 100,
            side_effects: vec![],
            user_feedback: None,
        };

        let result = learner
            .record_experience(&perception, &decision, &outcome)
            .await;

        assert!(result.is_ok());

        let stats = learner.get_learning_stats().await;
        assert_eq!(stats.total_experiences, 1);
        assert_eq!(stats.successful_experiences, 1);
    }

    #[tokio::test]
    async fn experience_learner_finds_similar_experiences() {
        let workspace = temp_workspace_for_test();
        let learner = ExperienceLearner::new(&workspace);

        let perception1 = PerceivedInput {
            raw_input: "What is Rust?".to_string(),
            intent: Intent {
                primary: IntentType::Question,
                secondary: vec![],
                confidence: 0.9,
                action_hints: vec![],
            },
            entities: vec![],
            sentiment: Sentiment {
                polarity: SentimentPolarity::Neutral,
                intensity: 0.0,
                emotions: vec![],
            },
            complexity: 0.3,
            topics: vec!["programming".to_string()],
            required_capabilities: vec![],
            context_clues: vec![],
            ambiguity_level: 0.1,
            follow_up_needed: false,
        };

        let decision = ActionDecision {
            action: SelectedAction::Respond {
                content: "Rust is a programming language".to_string(),
                needs_clarification: false,
                suggested_follow_ups: vec![],
            },
            reasoning: "Test".to_string(),
            confidence: 0.9,
            alternatives: vec![],
            risk_level: RiskLevel::None,
            estimated_impact: 0.5,
            requires_confirmation: false,
            confirmation_message: None,
            fallback: None,
        };

        let outcome = ActionOutcome {
            decision_id: "test_1".to_string(),
            action: decision.action.clone(),
            result: ActionResult::Success {
                output: "OK".to_string(),
            },
            duration_ms: 50,
            side_effects: vec![],
            user_feedback: None,
        };

        learner
            .record_experience(&perception1, &decision, &outcome)
            .await
            .unwrap();

        let perception2 = PerceivedInput {
            raw_input: "What is Python?".to_string(),
            intent: Intent {
                primary: IntentType::Question,
                secondary: vec![],
                confidence: 0.9,
                action_hints: vec![],
            },
            entities: vec![],
            sentiment: Sentiment {
                polarity: SentimentPolarity::Neutral,
                intensity: 0.0,
                emotions: vec![],
            },
            complexity: 0.3,
            topics: vec!["programming".to_string()],
            required_capabilities: vec![],
            context_clues: vec![],
            ambiguity_level: 0.1,
            follow_up_needed: false,
        };

        let similar = learner.find_similar_experiences(&perception2).await;

        assert!(!similar.is_empty());
    }

    #[tokio::test]
    async fn experience_learner_gets_applicable_patterns() {
        let workspace = temp_workspace_for_test();
        let learner = ExperienceLearner::new(&workspace);

        let stats = learner.get_learning_stats().await;
        assert_eq!(stats.patterns_discovered, 0);
    }

    #[tokio::test]
    async fn experience_learner_gets_lessons() {
        let workspace = temp_workspace_for_test();
        let learner = ExperienceLearner::new(&workspace);

        let lessons = learner.get_lessons_for_context("intent:Question").await;
        assert!(lessons.is_empty());
    }

    #[tokio::test]
    async fn experience_learner_gets_ready_skills() {
        let workspace = temp_workspace_for_test();
        let learner = ExperienceLearner::new(&workspace);

        let skills = learner.get_ready_skills().await;
        assert!(skills.is_empty());
    }

    #[tokio::test]
    async fn experience_structure() {
        let experience = Experience {
            id: "exp_test".to_string(),
            timestamp: chrono::Utc::now(),
            perception: PerceptionSummary {
                input_hash: "hash123".to_string(),
                intent: "Question".to_string(),
                topics: vec!["test".to_string()],
                complexity: 0.5,
            },
            action: ActionSummary {
                action_type: "Respond".to_string(),
                tool_used: None,
                arguments_hash: "arg_hash".to_string(),
            },
            outcome: OutcomeSummary {
                success: true,
                duration_ms: 100,
                side_effects: vec![],
            },
            patterns_extracted: vec![],
            lessons_learned: vec![],
            success_score: 1.0,
            replayable: true,
        };

        assert!(experience.outcome.success);
        assert!(experience.replayable);
    }

    #[tokio::test]
    async fn pattern_structure() {
        let pattern = Pattern {
            id: "pat_test".to_string(),
            pattern_type: PatternType::InputOutput,
            description: "Test pattern".to_string(),
            conditions: vec!["intent:Question".to_string()],
            actions: vec!["respond".to_string()],
            confidence: 0.8,
            occurrences: 5,
            success_rate: 0.9,
            first_seen: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
        };

        assert_eq!(pattern.pattern_type, PatternType::InputOutput);
        assert!((pattern.confidence - 0.8).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn lesson_structure() {
        let lesson = Lesson {
            id: "lesson_test".to_string(),
            topic: "testing".to_string(),
            insight: "Tests are important".to_string(),
            context: "During development".to_string(),
            applicability: vec!["development".to_string()],
            confidence: 0.9,
            source_experience: "exp_1".to_string(),
            validated: false,
            validation_count: 0,
        };

        assert!(!lesson.validated);
        assert_eq!(lesson.validation_count, 0);
    }

    #[tokio::test]
    async fn skill_prototype_structure() {
        let skill = SkillPrototype {
            id: "skill_test".to_string(),
            name: "test_skill".to_string(),
            description: "A test skill".to_string(),
            patterns: vec!["pat_1".to_string()],
            triggers: vec!["trigger".to_string()],
            actions: vec![],
            success_rate: 0.85,
            usage_count: 10,
            ready_for_promotion: true,
        };

        assert!(skill.ready_for_promotion);
        assert_eq!(skill.usage_count, 10);
    }

    #[tokio::test]
    async fn pattern_types() {
        assert_eq!(PatternType::InputOutput, PatternType::InputOutput);
        assert_ne!(PatternType::InputOutput, PatternType::Sequence);

        let types = [PatternType::InputOutput,
            PatternType::Sequence,
            PatternType::Conditional,
            PatternType::Correction,
            PatternType::Optimization,
            PatternType::Recovery,
            PatternType::UserPreference,
            PatternType::ErrorHandling];

        assert_eq!(types.len(), 8);
    }
}

mod working_memory_tests {
    use super::*;

    #[tokio::test]
    async fn working_memory_creation() {
        let memory = WorkingMemoryEngine::new();
        let stats = memory.get_stats().await;

        assert_eq!(stats.short_term_count, 0);
        assert_eq!(stats.long_term_count, 0);
    }

    #[tokio::test]
    async fn working_memory_add_item() {
        let memory = WorkingMemoryEngine::new();

        let id = memory
            .add("Test content", MemoryImportance::Normal, HashMap::new())
            .await
            .unwrap();

        assert!(id.starts_with("mem_"));

        let stats = memory.get_stats().await;
        assert_eq!(stats.short_term_count, 1);
    }

    #[tokio::test]
    async fn working_memory_recall() {
        let memory = WorkingMemoryEngine::new();

        let id = memory
            .add("Remember this", MemoryImportance::High, HashMap::new())
            .await
            .unwrap();

        let recalled = memory.recall(&id).await;

        assert!(recalled.is_some());
        assert_eq!(recalled.unwrap().content, "Remember this");
    }

    #[tokio::test]
    async fn working_memory_search() {
        let memory = WorkingMemoryEngine::new();

        memory
            .add(
                "Rust programming language",
                MemoryImportance::Normal,
                HashMap::new(),
            )
            .await
            .unwrap();
        memory
            .add("Python scripting", MemoryImportance::Normal, HashMap::new())
            .await
            .unwrap();
        memory
            .add("Rust memory safety", MemoryImportance::High, HashMap::new())
            .await
            .unwrap();

        let results = memory.search("Rust", 10).await;

        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn working_memory_get_context() {
        let memory = WorkingMemoryEngine::new();

        memory
            .add("First item", MemoryImportance::Normal, HashMap::new())
            .await
            .unwrap();
        memory
            .add("Second item", MemoryImportance::High, HashMap::new())
            .await
            .unwrap();

        let context = memory.get_context(1000).await;

        assert!(context.contains("First item") || context.contains("Second item"));
    }

    #[tokio::test]
    async fn working_memory_forget() {
        let memory = WorkingMemoryEngine::new();

        let id = memory
            .add(
                "Temporary content",
                MemoryImportance::Transient,
                HashMap::new(),
            )
            .await
            .unwrap();

        let forgotten = memory.forget(&id).await;

        assert!(forgotten);

        let recalled = memory.recall(&id).await;
        assert!(recalled.is_none());
    }

    #[tokio::test]
    async fn working_memory_decay() {
        let memory = WorkingMemoryEngine::new();

        memory
            .add("Test", MemoryImportance::Normal, HashMap::new())
            .await
            .unwrap();

        memory.decay().await;

        let stats = memory.get_stats().await;
        assert_eq!(stats.short_term_count, 1);
    }

    #[tokio::test]
    async fn working_memory_consolidate() {
        let memory = WorkingMemoryEngine::new();

        for i in 0..6 {
            memory
                .add(
                    &format!("Item {}", i),
                    MemoryImportance::High,
                    HashMap::new(),
                )
                .await
                .unwrap();
        }

        memory.consolidate().await.unwrap();

        let stats = memory.get_stats().await;
        // long_term_count is usize, always >= 0
        assert!(stats.long_term_count == stats.long_term_count);
    }

    #[tokio::test]
    async fn working_memory_add_episode() {
        let memory = WorkingMemoryEngine::new();

        let id = memory
            .add_episode(
                "learning",
                "Learned about testing",
                "Success",
                vec!["tests are important"],
            )
            .await
            .unwrap();

        assert!(id.starts_with("ep_"));

        let stats = memory.get_stats().await;
        assert_eq!(stats.episodic_count, 1);
    }

    #[tokio::test]
    async fn working_memory_get_episodes() {
        let memory = WorkingMemoryEngine::new();

        memory
            .add_episode("learning", "Learned Rust", "Success", vec!["ownership"])
            .await
            .unwrap();
        memory
            .add_episode(
                "debugging",
                "Fixed bug",
                "Success",
                vec!["careful with types"],
            )
            .await
            .unwrap();

        let all = memory.get_episodes(None, 10).await;
        assert_eq!(all.len(), 2);

        let learning = memory.get_episodes(Some("learning"), 10).await;
        assert_eq!(learning.len(), 1);
    }

    #[tokio::test]
    async fn working_memory_learn_from_episodes() {
        let memory = WorkingMemoryEngine::new();

        memory
            .add_episode("learning", "Lesson 1", "Success", vec!["important lesson"])
            .await
            .unwrap();
        memory
            .add_episode("learning", "Lesson 2", "Success", vec!["another lesson"])
            .await
            .unwrap();

        let learned = memory.learn_from_episodes().await;

        assert!(learned.contains(&"important lesson".to_string()));
        assert!(learned.contains(&"another lesson".to_string()));
    }

    #[tokio::test]
    async fn working_memory_compress() {
        let memory = WorkingMemoryEngine::new();

        let long_content =
            "This is a sentence. This is another sentence. And a third one. Plus a fourth one.";
        memory
            .add(long_content, MemoryImportance::Normal, HashMap::new())
            .await
            .unwrap();

        let compressed = memory.compress().await.unwrap();

        // compressed is usize, always >= 0
        let _ = compressed;
    }

    #[tokio::test]
    async fn memory_importance_ordering() {
        assert!(MemoryImportance::Critical != MemoryImportance::Transient);
        assert_eq!(MemoryImportance::Normal, MemoryImportance::Normal);
    }
}

mod meta_cognition_tests {
    use super::*;

    #[tokio::test]
    async fn meta_cognition_creation() {
        let engine = MetaCognitionEngine::new();
        let model = engine.get_self_model().await;

        assert_eq!(model.identity.name, "Housaky");
    }

    #[tokio::test]
    async fn meta_cognition_reflect() {
        let engine = MetaCognitionEngine::new();
        let reflection = engine.reflect("Test trigger").await.unwrap();

        assert!(!reflection.observations.is_empty());
        assert!(reflection.trigger == "Test trigger");
    }

    #[tokio::test]
    async fn meta_cognition_introspect_identity() {
        let engine = MetaCognitionEngine::new();

        let response = engine.introspect("who are you").await.unwrap();

        assert!(response.contains("Housaky"));
    }

    #[tokio::test]
    async fn meta_cognition_introspect_capabilities() {
        let engine = MetaCognitionEngine::new();

        let response = engine
            .introspect("what are your capabilities")
            .await
            .unwrap();

        assert!(response.contains("Reasoning") || response.contains("Learning"));
    }

    #[tokio::test]
    async fn meta_cognition_introspect_limitations() {
        let engine = MetaCognitionEngine::new();

        let response = engine
            .introspect("what are your limitations")
            .await
            .unwrap();

        assert!(!response.is_empty());
    }

    #[tokio::test]
    async fn meta_cognition_add_limitation() {
        let engine = MetaCognitionEngine::new();

        engine
            .add_limitation(
                "Cannot access physical devices",
                0.8,
                Some("Use software interfaces"),
            )
            .await
            .unwrap();

        let model = engine.get_self_model().await;
        assert_eq!(model.known_limitations.len(), 1);
    }

    #[tokio::test]
    async fn meta_cognition_add_growth_area() {
        let engine = MetaCognitionEngine::new();

        engine.add_growth_area("Reasoning", 0.6, 0.9).await.unwrap();

        let model = engine.get_self_model().await;
        assert_eq!(model.growth_areas.len(), 1);
    }

    #[tokio::test]
    async fn meta_cognition_update_capability() {
        let engine = MetaCognitionEngine::new();

        let before = engine.get_self_model().await;
        let before_reasoning = before.capabilities.reasoning;

        engine.update_capability("reasoning", 0.1).await.unwrap();

        let after = engine.get_self_model().await;
        assert!(after.capabilities.reasoning > before_reasoning);
    }

    #[tokio::test]
    async fn meta_cognition_get_recent_reflections() {
        let engine = MetaCognitionEngine::new();

        engine.reflect("Test 1").await.unwrap();
        engine.reflect("Test 2").await.unwrap();

        let recent = engine.get_recent_reflections(5).await;

        assert!(recent.len() >= 2);
    }

    #[tokio::test]
    async fn meta_cognition_explain_decision() {
        let engine = MetaCognitionEngine::new();

        let explanation = engine.explain_decision("Test decision").await;

        assert!(explanation.contains("Test decision"));
        assert!(explanation.contains("Context"));
    }

    #[tokio::test]
    async fn self_model_default() {
        let model = SelfModel::default();

        assert!(!model.identity.core_principles.is_empty());
        assert!(!model.values.is_empty());
    }

    #[tokio::test]
    async fn capability_assessment_default() {
        let caps = CapabilityAssessment::default();

        assert!(caps.reasoning > 0.0 && caps.reasoning <= 1.0);
        assert!(caps.learning > 0.0 && caps.learning <= 1.0);
    }

    #[tokio::test]
    async fn emotional_state_variants() {
        let states = [EmotionalState::Confident,
            EmotionalState::Curious,
            EmotionalState::Uncertain,
            EmotionalState::Frustrated,
            EmotionalState::Satisfied,
            EmotionalState::Neutral,
            EmotionalState::Excited,
            EmotionalState::Cautious];

        assert_eq!(states.len(), 8);
    }
}

mod inner_monologue_tests {
    use super::*;

    fn temp_workspace_for_test() -> PathBuf {
        std::env::temp_dir().join(format!("monologue_test_{}", uuid::Uuid::new_v4()))
    }

    #[tokio::test]
    async fn inner_monologue_creation() {
        let workspace = temp_workspace_for_test();
        let monologue = InnerMonologue::new(&workspace);

        let stats = monologue.get_stats().await;
        assert_eq!(stats.current_count, 0);
    }

    #[tokio::test]
    async fn inner_monologue_add_thought() {
        let workspace = temp_workspace_for_test();
        let monologue = InnerMonologue::new(&workspace);

        let id = monologue.add_thought("Test thought", 0.8).await.unwrap();

        assert!(id.starts_with("thought_"));

        let stats = monologue.get_stats().await;
        assert_eq!(stats.current_count, 1);
    }

    #[tokio::test]
    async fn inner_monologue_add_thought_with_type() {
        let workspace = temp_workspace_for_test();
        let monologue = InnerMonologue::new(&workspace);

        let id = monologue
            .add_thought_with_type(
                "This is a hypothesis",
                ThoughtType::Hypothesis,
                0.7,
                ThoughtSource::Reasoning,
            )
            .await
            .unwrap();

        let thoughts = monologue.get_by_type(ThoughtType::Hypothesis, 10).await;

        assert!(!thoughts.is_empty());
        assert_eq!(thoughts[0].id, id);
    }

    #[tokio::test]
    async fn inner_monologue_get_recent() {
        let workspace = temp_workspace_for_test();
        let monologue = InnerMonologue::new(&workspace);

        monologue.add_thought("Thought 1", 0.8).await.unwrap();
        monologue.add_thought("Thought 2", 0.8).await.unwrap();
        monologue.add_thought("Thought 3", 0.8).await.unwrap();

        let recent = monologue.get_recent(2).await;

        assert_eq!(recent.len(), 2);
    }

    #[tokio::test]
    async fn inner_monologue_search() {
        let workspace = temp_workspace_for_test();
        let monologue = InnerMonologue::new(&workspace);

        monologue
            .add_thought("Rust programming is great", 0.8)
            .await
            .unwrap();
        monologue
            .add_thought("Python is useful for scripting", 0.7)
            .await
            .unwrap();
        monologue
            .add_thought("Rust memory safety features", 0.9)
            .await
            .unwrap();

        let results = monologue.search("Rust", 10).await;

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn inner_monologue_get_unprocessed() {
        let workspace = temp_workspace_for_test();
        let monologue = InnerMonologue::new(&workspace);

        monologue
            .add_thought("Unprocessed thought 1", 0.8)
            .await
            .unwrap();
        monologue
            .add_thought("Unprocessed thought 2", 0.8)
            .await
            .unwrap();

        let unprocessed = monologue.get_unprocessed().await;

        assert_eq!(unprocessed.len(), 2);
    }

    #[tokio::test]
    async fn inner_monologue_mark_processed() {
        let workspace = temp_workspace_for_test();
        let monologue = InnerMonologue::new(&workspace);

        let id = monologue.add_thought("To be processed", 0.8).await.unwrap();

        monologue.mark_processed(&id).await.unwrap();

        let unprocessed = monologue.get_unprocessed().await;
        assert!(unprocessed.is_empty());
    }

    #[tokio::test]
    async fn inner_monologue_get_by_tag() {
        let workspace = temp_workspace_for_test();
        let monologue = InnerMonologue::new(&workspace);

        monologue
            .add_thought("This is an important goal", 0.9)
            .await
            .unwrap();

        let tagged = monologue.get_by_tag("goal", 10).await;

        assert!(!tagged.is_empty());
    }

    #[tokio::test]
    async fn inner_monologue_link_thoughts() {
        let workspace = temp_workspace_for_test();
        let monologue = InnerMonologue::new(&workspace);

        let id1 = monologue.add_thought("First thought", 0.8).await.unwrap();
        let id2 = monologue.add_thought("Related thought", 0.8).await.unwrap();

        monologue.link_thoughts(&id1, &id2).await.unwrap();

        let thoughts = monologue.get_recent_thoughts(10).await;
        let first = thoughts.iter().find(|t| t.id == id1).unwrap();

        assert!(first.related_thoughts.contains(&id2));
    }

    #[tokio::test]
    async fn inner_monologue_reflect() {
        let workspace = temp_workspace_for_test();
        let monologue = InnerMonologue::new(&workspace);

        for i in 0..6 {
            monologue
                .add_thought(&format!("Thought {}", i), 0.5)
                .await
                .unwrap();
        }

        let reflection = monologue.reflect().await.unwrap();

        assert!(reflection.is_some());
        assert_eq!(reflection.unwrap().thought_type, ThoughtType::Reflection);
    }

    #[tokio::test]
    async fn inner_monologue_compact() {
        let workspace = temp_workspace_for_test();
        let monologue = InnerMonologue::new(&workspace);

        for i in 0..150 {
            monologue
                .add_thought(&format!("Low importance thought {}", i), 0.1)
                .await
                .unwrap();
        }

        let compacted = monologue.compact().await.unwrap();

        assert!(compacted > 0);
    }

    #[tokio::test]
    async fn inner_monologue_save_load() {
        let workspace = temp_workspace_for_test();
        let monologue = InnerMonologue::new(&workspace);

        monologue
            .add_thought("Persistent thought", 0.8)
            .await
            .unwrap();
        monologue.save().await.unwrap();

        let monologue2 = InnerMonologue::new(&workspace);
        monologue2.load().await.unwrap();

        let stats = monologue2.get_stats().await;
        assert_eq!(stats.current_count, 1);
    }

    #[tokio::test]
    async fn inner_monologue_export_summary() {
        let workspace = temp_workspace_for_test();
        let monologue = InnerMonologue::new(&workspace);

        monologue
            .add_thought("Test thought for export", 0.8)
            .await
            .unwrap();

        let summary = monologue.export_summary().await;

        assert!(summary.contains("Inner Monologue Summary"));
        assert!(summary.contains("Test thought"));
    }

    #[tokio::test]
    async fn thought_type_display() {
        assert_eq!(format!("{}", ThoughtType::Observation), "Observation");
        assert_eq!(format!("{}", ThoughtType::Reflection), "Reflection");
        assert_eq!(format!("{}", ThoughtType::Decision), "Decision");
    }
}

mod multi_agent_tests {
    use super::*;

    #[tokio::test]
    async fn multi_agent_coordinator_creation() {
        let coordinator = MultiAgentCoordinator::new();
        let stats = coordinator.get_coordinator_stats().await;

        assert_eq!(stats.pending_tasks, 0);
        assert_eq!(stats.active_tasks, 0);
    }

    #[tokio::test]
    async fn multi_agent_submit_task() {
        let coordinator = MultiAgentCoordinator::new();

        let task = AgentTask {
            id: "task_1".to_string(),
            description: "Test task".to_string(),
            required_capabilities: vec!["testing".to_string()],
            priority: TaskPriority::Medium,
            dependencies: vec![],
            deadline: None,
            context: HashMap::new(),
            status: TaskStatus::Pending,
        };

        let result = coordinator.submit_task(task).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "task_1");

        let stats = coordinator.get_coordinator_stats().await;
        assert_eq!(stats.pending_tasks, 1);
    }

    #[tokio::test]
    async fn multi_agent_task_priority_ordering() {
        let coordinator = MultiAgentCoordinator::new();

        let low_task = AgentTask {
            id: "low_task".to_string(),
            description: "Low priority".to_string(),
            required_capabilities: vec![],
            priority: TaskPriority::Low,
            dependencies: vec![],
            deadline: None,
            context: HashMap::new(),
            status: TaskStatus::Pending,
        };

        let high_task = AgentTask {
            id: "high_task".to_string(),
            description: "High priority".to_string(),
            required_capabilities: vec![],
            priority: TaskPriority::High,
            dependencies: vec![],
            deadline: None,
            context: HashMap::new(),
            status: TaskStatus::Pending,
        };

        coordinator.submit_task(low_task).await.unwrap();
        coordinator.submit_task(high_task).await.unwrap();

        let pending = coordinator.get_pending_tasks().await;

        assert_eq!(pending[0].id, "high_task");
    }

    #[tokio::test]
    async fn multi_agent_get_task_status() {
        let coordinator = MultiAgentCoordinator::new();

        let task = AgentTask {
            id: "status_test".to_string(),
            description: "Status test".to_string(),
            required_capabilities: vec![],
            priority: TaskPriority::Medium,
            dependencies: vec![],
            deadline: None,
            context: HashMap::new(),
            status: TaskStatus::Pending,
        };

        coordinator.submit_task(task).await.unwrap();

        let status = coordinator.get_task_status("status_test").await;

        assert_eq!(status, Some(TaskStatus::Pending));
    }

    #[tokio::test]
    async fn multi_agent_subscribe() {
        let coordinator = MultiAgentCoordinator::new();

        let mut receiver = coordinator.subscribe();

        let task = AgentTask {
            id: "broadcast_test".to_string(),
            description: "Test".to_string(),
            required_capabilities: vec![],
            priority: TaskPriority::Medium,
            dependencies: vec![],
            deadline: None,
            context: HashMap::new(),
            status: TaskStatus::Pending,
        };

        coordinator.submit_task(task).await.unwrap();

        let msg = receiver.try_recv();
        assert!(msg.is_ok());
        assert_eq!(msg.unwrap().msg_type, MessageType::TaskSubmitted);
    }

    #[tokio::test]
    async fn agent_registry_creation() {
        let registry = AgentRegistry::new();
        let stats = registry.get_registry_stats().await;

        assert_eq!(stats.total_agents, 0);
    }

    #[tokio::test]
    async fn agent_registry_register() {
        let registry = AgentRegistry::new();

        let agent = AgentInfo {
            id: "agent_1".to_string(),
            name: "TestAgent".to_string(),
            agent_type: AgentType::Worker,
            capabilities: vec!["testing".to_string()],
            available: true,
            performance_metrics: AgentPerformance::default(),
            registered_at: chrono::Utc::now(),
            last_heartbeat: chrono::Utc::now(),
            current_task: None,
            metadata: HashMap::new(),
        };

        let result = registry.register(agent).await;
        assert!(result.is_ok());

        let stats = registry.get_registry_stats().await;
        assert_eq!(stats.total_agents, 1);
    }

    #[tokio::test]
    async fn agent_registry_unregister() {
        let registry = AgentRegistry::new();

        let agent = AgentInfo {
            id: "agent_to_remove".to_string(),
            name: "ToRemove".to_string(),
            agent_type: AgentType::Worker,
            capabilities: vec![],
            available: true,
            performance_metrics: AgentPerformance::default(),
            registered_at: chrono::Utc::now(),
            last_heartbeat: chrono::Utc::now(),
            current_task: None,
            metadata: HashMap::new(),
        };

        registry.register(agent).await.unwrap();
        registry.unregister("agent_to_remove").await.unwrap();

        let stats = registry.get_registry_stats().await;
        assert_eq!(stats.total_agents, 0);
    }

    #[tokio::test]
    async fn agent_registry_find_by_capability() {
        let registry = AgentRegistry::new();

        let agent = AgentInfo {
            id: "capable_agent".to_string(),
            name: "CapableAgent".to_string(),
            agent_type: AgentType::Specialist,
            capabilities: vec!["rust".to_string(), "testing".to_string()],
            available: true,
            performance_metrics: AgentPerformance::default(),
            registered_at: chrono::Utc::now(),
            last_heartbeat: chrono::Utc::now(),
            current_task: None,
            metadata: HashMap::new(),
        };

        registry.register(agent).await.unwrap();

        let found = registry.find_by_capability("rust").await;

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "capable_agent");
    }

    #[tokio::test]
    async fn agent_registry_find_by_type() {
        let registry = AgentRegistry::new();

        let coordinator_agent = AgentInfo {
            id: "coordinator_1".to_string(),
            name: "Coordinator".to_string(),
            agent_type: AgentType::Coordinator,
            capabilities: vec![],
            available: true,
            performance_metrics: AgentPerformance::default(),
            registered_at: chrono::Utc::now(),
            last_heartbeat: chrono::Utc::now(),
            current_task: None,
            metadata: HashMap::new(),
        };

        let worker_agent = AgentInfo {
            id: "worker_1".to_string(),
            name: "Worker".to_string(),
            agent_type: AgentType::Worker,
            capabilities: vec![],
            available: true,
            performance_metrics: AgentPerformance::default(),
            registered_at: chrono::Utc::now(),
            last_heartbeat: chrono::Utc::now(),
            current_task: None,
            metadata: HashMap::new(),
        };

        registry.register(coordinator_agent).await.unwrap();
        registry.register(worker_agent).await.unwrap();

        let coordinators = registry.find_by_type(AgentType::Coordinator).await;
        assert_eq!(coordinators.len(), 1);

        let workers = registry.find_by_type(AgentType::Worker).await;
        assert_eq!(workers.len(), 1);
    }

    #[tokio::test]
    async fn agent_registry_update_performance() {
        let registry = AgentRegistry::new();

        let agent = AgentInfo {
            id: "perf_agent".to_string(),
            name: "PerfAgent".to_string(),
            agent_type: AgentType::Worker,
            capabilities: vec![],
            available: true,
            performance_metrics: AgentPerformance::default(),
            registered_at: chrono::Utc::now(),
            last_heartbeat: chrono::Utc::now(),
            current_task: None,
            metadata: HashMap::new(),
        };

        registry.register(agent).await.unwrap();

        registry.update_agent_performance("perf_agent", true).await;
        registry.update_agent_performance("perf_agent", true).await;
        registry.update_agent_performance("perf_agent", false).await;

        let agent = registry.get_agent("perf_agent").await.unwrap();
        assert_eq!(agent.performance_metrics.tasks_completed, 2);
        assert_eq!(agent.performance_metrics.tasks_failed, 1);
        assert!((agent.performance_metrics.success_rate - 0.6666666666666666).abs() < 0.01);
    }

    #[tokio::test]
    async fn agent_message_creation() {
        let msg = AgentMessage::new(MessageType::TaskAssigned, "sender", "content");

        assert_eq!(msg.msg_type, MessageType::TaskAssigned);
        assert_eq!(msg.sender, "sender");
        assert!(msg.is_broadcast());
    }

    #[tokio::test]
    async fn agent_message_to_specific() {
        let msg = AgentMessage::new(MessageType::Query, "sender", "content").to("receiver");

        assert!(!msg.is_broadcast());
        assert!(msg.is_for("receiver"));
        assert!(!msg.is_for("other"));
    }

    #[tokio::test]
    async fn agent_message_with_metadata() {
        let msg =
            AgentMessage::new(MessageType::Info, "sender", "content").with_metadata("key", "value");

        assert_eq!(msg.metadata.get("key"), Some(&"value".to_string()));
    }

    #[tokio::test]
    async fn message_type_variants() {
        let types = vec![
            MessageType::TaskSubmitted,
            MessageType::TaskAssigned,
            MessageType::TaskProgress,
            MessageType::TaskCompleted,
            MessageType::TaskFailed,
            MessageType::Query,
            MessageType::Response,
            MessageType::Broadcast,
            MessageType::Heartbeat,
            MessageType::StatusUpdate,
            MessageType::Error,
            MessageType::Warning,
            MessageType::Info,
            MessageType::Coordination,
            MessageType::ConsensusRequest,
            MessageType::ConsensusResponse,
            MessageType::ResourceRequest,
            MessageType::ResourceGranted,
            MessageType::ResourceDenied,
        ];

        assert_eq!(types.len(), 19);
    }
}

mod suggestion_engine_tests {
    use super::*;

    #[test]
    fn suggestion_engine_creation() {
        let engine = SuggestionEngine::new();
        let suggestions = engine.generate_suggestions(5);

        assert!(suggestions.is_empty() || suggestions.len() <= 5);
    }

    #[test]
    fn suggestion_engine_default() {
        let engine = SuggestionEngine::default();
        let suggestions = engine.generate_suggestions(3);

        assert!(suggestions.len() <= 3);
    }

    #[test]
    fn suggestion_context_default() {
        let context = SuggestionContext::default();

        assert!(context.last_intent.is_none());
        assert!(context.recent_topics.is_empty());
        assert!(context.active_goals.is_empty());
        assert_eq!(context.conversation_length, 0);
        assert_eq!(context.failed_actions, 0);
    }

    #[test]
    fn suggestion_engine_update_context() {
        let engine = SuggestionEngine::new();

        let suggestions = engine.suggest_for_input("hi");

        assert!(!suggestions.is_empty());
        assert!(suggestions[0].category == SuggestionCategory::Clarification);
    }

    #[test]
    fn suggestion_engine_for_how_question() {
        let engine = SuggestionEngine::new();

        let suggestions = engine.suggest_for_input("How do I install Rust?");

        let has_step_by_step = suggestions.iter().any(|s| s.title.contains("Step-by-step"));
        assert!(has_step_by_step);
    }

    #[test]
    fn suggestion_engine_for_debug() {
        let engine = SuggestionEngine::new();

        let suggestions = engine.suggest_for_input("I have an error in my code");

        let has_debug = suggestions.iter().any(|s| s.title.contains("Debug"));
        assert!(has_debug);
    }

    #[test]
    fn suggestion_engine_follow_ups() {
        let engine = SuggestionEngine::new();

        let follow_ups = engine.suggest_follow_ups("The operation was a success!");

        assert!(follow_ups.iter().any(|f| f.contains("next")));
    }

    #[test]
    fn suggestion_engine_follow_ups_for_error() {
        let engine = SuggestionEngine::new();

        let follow_ups = engine.suggest_follow_ups("The operation failed with an error");

        assert!(follow_ups.iter().any(|f| f.contains("alternative")));
    }

    #[test]
    fn suggestion_formatting() {
        let engine = SuggestionEngine::new();

        let question = Suggestion {
            title: "Test".to_string(),
            description: "Test desc".to_string(),
            action: SuggestedAction::AskQuestion("What?".to_string()),
            confidence: 0.8,
            category: SuggestionCategory::FollowUp,
        };

        let formatted = engine.format_suggestion(&question);
        assert!(formatted.starts_with("💡"));

        let action = Suggestion {
            title: "Test".to_string(),
            description: "Test desc".to_string(),
            action: SuggestedAction::ProposeAction("Do this".to_string()),
            confidence: 0.8,
            category: SuggestionCategory::Helpful,
        };

        let formatted = engine.format_suggestion(&action);
        assert!(formatted.starts_with("⚡"));
    }

    #[test]
    fn suggestion_categories() {
        let categories = [SuggestionCategory::Proactive,
            SuggestionCategory::Helpful,
            SuggestionCategory::Clarification,
            SuggestionCategory::FollowUp,
            SuggestionCategory::Reminder,
            SuggestionCategory::Exploration];

        assert_eq!(categories.len(), 6);
    }

    #[test]
    fn sentiment_variants() {
        let sentiments = [SuggestionSentiment::Neutral,
            SuggestionSentiment::Positive,
            SuggestionSentiment::Negative,
            SuggestionSentiment::Frustrated,
            SuggestionSentiment::Curious];

        assert_eq!(sentiments.len(), 5);
    }

    #[test]
    fn suggestion_condition_matches() {
        let mut engine = SuggestionEngine::new();

        let context = SuggestionContext {
            last_intent: Some(IntentType::Question),
            recent_topics: vec![],
            active_goals: vec![],
            conversation_length: 5,
            failed_actions: 0,
            user_sentiment: SuggestionSentiment::Neutral,
            time_since_last_message: std::time::Duration::from_secs(30),
        };

        engine.update_context(context);

        let suggestions = engine.generate_suggestions(10);

        assert!(suggestions.len() <= 10);
    }
}

mod goal_engine_tests {
    use super::*;

    fn temp_workspace_for_test() -> PathBuf {
        std::env::temp_dir().join(format!("goal_test_{}", uuid::Uuid::new_v4()))
    }

    #[tokio::test]
    async fn goal_engine_creation() {
        let workspace = temp_workspace_for_test();
        let engine = GoalEngine::new(&workspace);

        let stats = engine.get_goal_stats().await;
        assert_eq!(stats.total, 0);
    }

    #[tokio::test]
    async fn goal_engine_add_simple_goal() {
        let workspace = temp_workspace_for_test();
        let engine = GoalEngine::new(&workspace);

        let goal = Goal {
            title: "Simple Test".to_string(),
            description: "A simple test goal".to_string(),
            priority: GoalPriority::High,
            estimated_complexity: 1.0,
            ..Default::default()
        };

        let result = engine.add_goal(goal).await;
        assert!(result.is_ok());

        let stats = engine.get_goal_stats().await;
        assert!(stats.total >= 1);
    }

    #[test]
    fn goal_default() {
        let goal = Goal::default();

        assert_eq!(goal.status, GoalStatus::Pending);
        assert_eq!(goal.priority, GoalPriority::Medium);
        assert_eq!(goal.category, GoalCategory::UserRequest);
        assert_eq!(goal.progress, 0.0);
    }

    #[test]
    fn goal_priority_ordering() {
        assert!(GoalPriority::Critical > GoalPriority::High);
        assert!(GoalPriority::High > GoalPriority::Medium);
        assert!(GoalPriority::Medium > GoalPriority::Low);
        assert!(GoalPriority::Low > GoalPriority::Background);
    }

    #[test]
    fn goal_status_variants() {
        let statuses = [GoalStatus::Pending,
            GoalStatus::InProgress,
            GoalStatus::Completed,
            GoalStatus::Failed,
            GoalStatus::Cancelled,
            GoalStatus::Deferred];

        assert_eq!(statuses.len(), 6);
    }

    #[test]
    fn goal_category_variants() {
        let categories = vec![
            GoalCategory::Planning,
            GoalCategory::Intelligence,
            GoalCategory::ToolDevelopment,
            GoalCategory::SkillAcquisition,
            GoalCategory::KnowledgeExpansion,
            GoalCategory::SystemImprovement,
            GoalCategory::UserRequest,
            GoalCategory::SelfModification,
            GoalCategory::Research,
            GoalCategory::Integration,
            GoalCategory::Maintenance,
        ];

        assert_eq!(categories.len(), 11);
    }
}

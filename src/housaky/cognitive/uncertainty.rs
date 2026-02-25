use crate::providers::Provider;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UncertaintyAssessment {
    pub overall_uncertainty: f64,
    pub sources: Vec<UncertaintySource>,
    pub confidence_intervals: HashMap<String, (f64, f64)>,
    pub calibration_score: f64,
    pub should_ask_clarification: bool,
    pub clarification_questions: Vec<String>,
    pub alternative_interpretations: Vec<String>,
    pub knowledge_gaps: Vec<KnowledgeGap>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UncertaintySource {
    pub category: UncertaintyCategory,
    pub description: String,
    pub impact: f64,
    pub mitigation: Option<String>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UncertaintyCategory {
    InputAmbiguity,
    MissingContext,
    KnowledgeGap,
    ReasoningUncertainty,
    ToolUncertainty,
    ConfidenceGap,
    DomainUnfamiliarity,
    ConflictingInformation,
    IncompleteData,
    Speculation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGap {
    pub topic: String,
    pub severity: f64,
    pub can_retrieve: bool,
    pub retrieval_strategy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceCalibration {
    pub predictions: Vec<ConfidencePrediction>,
    pub calibration_curve: Vec<(f64, f64)>,
    pub brier_score: f64,
    pub expected_calibration_error: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidencePrediction {
    pub predicted_confidence: f64,
    pub actual_outcome: Option<bool>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub context: String,
}

pub struct UncertaintyDetector {
    calibration_history: Arc<RwLock<Vec<ConfidencePrediction>>>,
    confidence_threshold: f64,
    high_uncertainty_threshold: f64,
    calibration_window: usize,
}

impl UncertaintyDetector {
    pub fn new() -> Self {
        Self {
            calibration_history: Arc::new(RwLock::new(Vec::new())),
            confidence_threshold: 0.7,
            high_uncertainty_threshold: 0.4,
            calibration_window: 100,
        }
    }

    pub async fn assess(
        &self,
        input: &str,
        reasoning_confidence: f64,
        context: &[String],
    ) -> Result<UncertaintyAssessment> {
        info!("Assessing uncertainty for input...");

        let sources = self.identify_uncertainty_sources(input, reasoning_confidence, context);
        let overall_uncertainty = self.calculate_overall_uncertainty(&sources);

        let confidence_intervals = self.compute_confidence_intervals(&sources);
        let calibration_score = self.get_calibration_score().await;

        let should_ask = overall_uncertainty > self.high_uncertainty_threshold;
        let clarification_questions = if should_ask {
            self.generate_clarification_questions(&sources)
        } else {
            vec![]
        };

        let alternatives = self.generate_alternative_interpretations(input, &sources);
        let knowledge_gaps = self.identify_knowledge_gaps(input, &sources);

        Ok(UncertaintyAssessment {
            overall_uncertainty,
            sources,
            confidence_intervals,
            calibration_score,
            should_ask_clarification: should_ask,
            clarification_questions,
            alternative_interpretations: alternatives,
            knowledge_gaps,
        })
    }

    pub async fn assess_with_llm(
        &self,
        input: &str,
        reasoning: &str,
        provider: &dyn Provider,
        model: &str,
    ) -> Result<UncertaintyAssessment> {
        let prompt = format!(
            r#"Analyze uncertainty in this AI reasoning process:

User Input: "{}"
Reasoning: "{}"

Assess:
1. What aspects are uncertain?
2. What knowledge gaps exist?
3. What could go wrong?
4. Should we ask for clarification?

Return JSON with:
{{
  "uncertainty_score": 0.0-1.0,
  "sources": [{{"category": "category_name", "description": "desc", "impact": 0.0-1.0}}],
  "knowledge_gaps": [{{"topic": "topic", "severity": 0.0-1.0}}],
  "clarification_needed": true/false,
  "questions_to_ask": ["question1", "question2"]
}}"#,
            input, reasoning
        );

        let response = provider
            .chat_with_system(
                Some("You are an uncertainty analysis engine. Return only valid JSON."),
                &prompt,
                model,
                0.1,
            )
            .await?;

        let basic_assessment = self.assess(input, 0.7, &[]).await?;

        if let Ok(llm_result) = self.parse_llm_uncertainty(&response) {
            Ok(UncertaintyAssessment {
                overall_uncertainty: llm_result
                    .uncertainty_score
                    .unwrap_or(basic_assessment.overall_uncertainty),
                sources: if llm_result.sources.is_empty() {
                    basic_assessment.sources
                } else {
                    llm_result.sources
                },
                confidence_intervals: basic_assessment.confidence_intervals,
                calibration_score: self.get_calibration_score().await,
                should_ask_clarification: llm_result
                    .clarification_needed
                    .unwrap_or(basic_assessment.should_ask_clarification),
                clarification_questions: if llm_result.questions.is_empty() {
                    basic_assessment.clarification_questions
                } else {
                    llm_result.questions
                },
                alternative_interpretations: basic_assessment.alternative_interpretations,
                knowledge_gaps: if llm_result.knowledge_gaps.is_empty() {
                    basic_assessment.knowledge_gaps
                } else {
                    llm_result.knowledge_gaps
                },
            })
        } else {
            Ok(basic_assessment)
        }
    }

    fn parse_llm_uncertainty(&self, response: &str) -> Result<LLMUncertaintyResult> {
        let json_str = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let parsed: serde_json::Value = serde_json::from_str(json_str)?;

        Ok(LLMUncertaintyResult {
            uncertainty_score: parsed.get("uncertainty_score").and_then(|v| v.as_f64()),
            sources: parsed
                .get("sources")
                .and_then(|v| {
                    let arr = v.as_array()?;
                    Some(
                        arr.iter()
                            .filter_map(|s| {
                                Some(UncertaintySource {
                                    category: match s.get("category")?.as_str()? {
                                        "InputAmbiguity" => UncertaintyCategory::InputAmbiguity,
                                        "MissingContext" => UncertaintyCategory::MissingContext,
                                        "KnowledgeGap" => UncertaintyCategory::KnowledgeGap,
                                        "ReasoningUncertainty" => {
                                            UncertaintyCategory::ReasoningUncertainty
                                        }
                                        "ToolUncertainty" => UncertaintyCategory::ToolUncertainty,
                                        "ConfidenceGap" => UncertaintyCategory::ConfidenceGap,
                                        "DomainUnfamiliarity" => {
                                            UncertaintyCategory::DomainUnfamiliarity
                                        }
                                        "ConflictingInformation" => {
                                            UncertaintyCategory::ConflictingInformation
                                        }
                                        "IncompleteData" => UncertaintyCategory::IncompleteData,
                                        _ => UncertaintyCategory::Speculation,
                                    },
                                    description: s.get("description")?.as_str()?.to_string(),
                                    impact: s.get("impact")?.as_f64().unwrap_or(0.5),
                                    mitigation: None,
                                    evidence: vec![],
                                })
                            })
                            .collect(),
                    )
                })
                .unwrap_or_default(),
            knowledge_gaps: parsed
                .get("knowledge_gaps")
                .and_then(|v| {
                    let arr = v.as_array()?;
                    Some(
                        arr.iter()
                            .filter_map(|g| {
                                Some(KnowledgeGap {
                                    topic: g.get("topic")?.as_str()?.to_string(),
                                    severity: g.get("severity")?.as_f64().unwrap_or(0.5),
                                    can_retrieve: true,
                                    retrieval_strategy: None,
                                })
                            })
                            .collect(),
                    )
                })
                .unwrap_or_default(),
            clarification_needed: parsed.get("clarification_needed").and_then(|v| v.as_bool()),
            questions: parsed
                .get("questions_to_ask")
                .and_then(|v| {
                    Some(
                        v.as_array()?
                            .iter()
                            .filter_map(|q| q.as_str().map(|s| s.to_string()))
                            .collect(),
                    )
                })
                .unwrap_or_default(),
        })
    }

    fn identify_uncertainty_sources(
        &self,
        input: &str,
        reasoning_confidence: f64,
        context: &[String],
    ) -> Vec<UncertaintySource> {
        let mut sources = Vec::new();

        if reasoning_confidence < self.confidence_threshold {
            sources.push(UncertaintySource {
                category: UncertaintyCategory::ConfidenceGap,
                description: format!("Low reasoning confidence: {:.2}", reasoning_confidence),
                impact: 1.0 - reasoning_confidence,
                mitigation: Some(
                    "Gather more information or consider alternative approaches".to_string(),
                ),
                evidence: vec![],
            });
        }

        if context.is_empty() {
            sources.push(UncertaintySource {
                category: UncertaintyCategory::MissingContext,
                description: "No prior context available".to_string(),
                impact: 0.3,
                mitigation: Some("Consider asking for context or using defaults".to_string()),
                evidence: vec![],
            });
        }

        let ambiguous_words = ["it", "that", "this", "they", "there", "here"];
        let found_ambiguous: Vec<_> = ambiguous_words
            .iter()
            .filter(|w| input.to_lowercase().contains(*w))
            .collect();

        if !found_ambiguous.is_empty() {
            sources.push(UncertaintySource {
                category: UncertaintyCategory::InputAmbiguity,
                description: format!("Ambiguous pronouns found: {:?}", found_ambiguous),
                impact: 0.4,
                mitigation: Some("Clarify what the pronouns refer to".to_string()),
                evidence: found_ambiguous.iter().map(|w| w.to_string()).collect(),
            });
        }

        let vague_terms = ["something", "anything", "stuff", "things", "some", "few"];
        let found_vague: Vec<_> = vague_terms
            .iter()
            .filter(|w| input.to_lowercase().contains(*w))
            .collect();

        if !found_vague.is_empty() {
            sources.push(UncertaintySource {
                category: UncertaintyCategory::IncompleteData,
                description: format!("Vague terms found: {:?}", found_vague),
                impact: 0.3,
                mitigation: Some("Ask for specific details".to_string()),
                evidence: found_vague.iter().map(|w| w.to_string()).collect(),
            });
        }

        if input.len() < 10 {
            sources.push(UncertaintySource {
                category: UncertaintyCategory::InputAmbiguity,
                description: "Very short input - likely incomplete or unclear".to_string(),
                impact: 0.5,
                mitigation: Some("Ask for elaboration".to_string()),
                evidence: vec![format!("Input length: {} chars", input.len())],
            });
        }

        let technical_domains = [
            "quantum",
            "blockchain",
            "neural",
            "cryptograph",
            "physics",
            "chemistry",
        ];
        for domain in technical_domains {
            if input.to_lowercase().contains(domain) {
                sources.push(UncertaintySource {
                    category: UncertaintyCategory::DomainUnfamiliarity,
                    description: format!("Specialized domain detected: {}", domain),
                    impact: 0.3,
                    mitigation: Some(
                        "Verify understanding and seek additional context".to_string(),
                    ),
                    evidence: vec![domain.to_string()],
                });
            }
        }

        sources
    }

    fn calculate_overall_uncertainty(&self, sources: &[UncertaintySource]) -> f64 {
        if sources.is_empty() {
            return 0.1;
        }

        let total_impact: f64 = sources.iter().map(|s| s.impact).sum();
        let weighted = total_impact / (sources.len() as f64);

        let max_impact = sources.iter().map(|s| s.impact).fold(0.0, f64::max);

        (weighted * 0.6 + max_impact * 0.4).clamp(0.0, 1.0)
    }

    fn compute_confidence_intervals(
        &self,
        sources: &[UncertaintySource],
    ) -> HashMap<String, (f64, f64)> {
        let mut intervals = HashMap::new();

        let overall = 1.0 - self.calculate_overall_uncertainty(sources);
        intervals.insert("overall".to_string(), (overall - 0.1, overall + 0.1));

        for source in sources {
            let confidence = 1.0 - source.impact;
            intervals.insert(
                format!("{:?}", source.category).to_lowercase(),
                (confidence - 0.15, confidence + 0.05),
            );
        }

        intervals
    }

    async fn get_calibration_score(&self) -> f64 {
        let history = self.calibration_history.read().await;

        if history.len() < 10 {
            return 0.7;
        }

        let predictions_with_outcomes: Vec<_> = history
            .iter()
            .filter(|p| p.actual_outcome.is_some())
            .collect();

        if predictions_with_outcomes.is_empty() {
            return 0.7;
        }

        let mut bins: Vec<Vec<&ConfidencePrediction>> = vec![vec![]; 10];
        for pred in &predictions_with_outcomes {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let bin_idx = ((pred.predicted_confidence * 10.0).clamp(0.0, 9.0) as usize).min(9);
            bins[bin_idx].push(pred);
        }

        let mut calibration_error = 0.0;
        let mut count = 0;

        for (i, bin) in bins.iter().enumerate() {
            if bin.is_empty() {
                continue;
            }

            let predicted_mean = (i as f64 + 0.5) / 10.0;
            let actual_accuracy = bin
                .iter()
                .filter(|p| p.actual_outcome == Some(true))
                .count() as f64
                / bin.len() as f64;

            calibration_error += (predicted_mean - actual_accuracy).abs();
            count += 1;
        }

        if count > 0 {
            1.0 - (calibration_error / f64::from(count))
        } else {
            0.7
        }
    }

    fn generate_clarification_questions(&self, sources: &[UncertaintySource]) -> Vec<String> {
        let mut questions = Vec::new();

        for source in sources {
            match source.category {
                UncertaintyCategory::InputAmbiguity => {
                    if !source.evidence.is_empty() {
                        questions.push(format!(
                            "Could you clarify what '{}' refers to?",
                            source.evidence.first().unwrap()
                        ));
                    }
                }
                UncertaintyCategory::MissingContext => {
                    questions.push(
                        "Could you provide more context about what you're working on?".to_string(),
                    );
                }
                UncertaintyCategory::IncompleteData => {
                    questions.push("Could you provide more specific details?".to_string());
                }
                UncertaintyCategory::DomainUnfamiliarity => {
                    questions.push(
                        "Would you like me to explain my understanding before proceeding?"
                            .to_string(),
                    );
                }
                UncertaintyCategory::ConflictingInformation => {
                    questions.push(
                        "I'm seeing conflicting information. Could you clarify which is correct?"
                            .to_string(),
                    );
                }
                _ => {}
            }
        }

        if questions.is_empty() {
            questions.push("Could you provide more details?".to_string());
        }

        questions.truncate(3);
        questions
    }

    fn generate_alternative_interpretations(
        &self,
        input: &str,
        sources: &[UncertaintySource],
    ) -> Vec<String> {
        let mut interpretations = Vec::new();

        let input_lower = input.to_lowercase();

        if input_lower.contains("it") {
            interpretations.push(
                "You might be referring to a previously mentioned object or concept".to_string(),
            );
        }

        if input_lower.contains("fix") || input_lower.contains("error") {
            interpretations.push("You might want me to debug or troubleshoot an issue".to_string());
        }

        if input_lower.contains("make") || input_lower.contains("create") {
            interpretations.push("You might want me to generate or build something".to_string());
        }

        for source in sources {
            if source.category == UncertaintyCategory::InputAmbiguity {
                interpretations.push(format!(
                    "Alternative interpretation related to: {}",
                    source.description
                ));
            }
        }

        interpretations.truncate(3);
        interpretations
    }

    fn identify_knowledge_gaps(
        &self,
        input: &str,
        sources: &[UncertaintySource],
    ) -> Vec<KnowledgeGap> {
        let mut gaps = Vec::new();

        for source in sources {
            if source.category == UncertaintyCategory::KnowledgeGap
                || source.category == UncertaintyCategory::DomainUnfamiliarity
            {
                gaps.push(KnowledgeGap {
                    topic: source.description.clone(),
                    severity: source.impact,
                    can_retrieve: true,
                    retrieval_strategy: Some("search_documentation".to_string()),
                });
            }
        }

        let technical_patterns = [
            (regex::Regex::new(r"\b(\w+)\s+api\b").unwrap(), "api"),
            (
                regex::Regex::new(r"\b(\w+)\s+framework\b").unwrap(),
                "framework",
            ),
            (
                regex::Regex::new(r"\b(\w+)\s+library\b").unwrap(),
                "library",
            ),
        ];

        for (pattern, gap_type) in technical_patterns {
            if let Some(cap) = pattern.captures(input.to_lowercase().as_str()) {
                if let Some(name) = cap.get(1) {
                    gaps.push(KnowledgeGap {
                        topic: format!("{}: {}", gap_type, name.as_str()),
                        severity: 0.5,
                        can_retrieve: true,
                        retrieval_strategy: Some(format!("search_{}_documentation", gap_type)),
                    });
                }
            }
        }

        gaps
    }

    pub async fn record_outcome(&self, prediction_id: &str, was_correct: bool) -> Result<()> {
        let mut history = self.calibration_history.write().await;

        if let Some(pred) = history.iter_mut().find(|p| p.context == prediction_id) {
            pred.actual_outcome = Some(was_correct);
        }

        if history.len() > self.calibration_window {
            history.remove(0);
        }

        Ok(())
    }

    pub async fn record_prediction(&self, confidence: f64, context: &str) {
        let mut history = self.calibration_history.write().await;

        history.push(ConfidencePrediction {
            predicted_confidence: confidence,
            actual_outcome: None,
            timestamp: chrono::Utc::now(),
            context: context.to_string(),
        });

        if history.len() > self.calibration_window * 2 {
            history.remove(0);
        }
    }

    pub async fn get_calibration_metrics(&self) -> ConfidenceCalibration {
        let history = self.calibration_history.read().await;

        let predictions = history.clone();

        let mut bins: Vec<(f64, f64, usize)> = vec![(0.0, 0.0, 0); 10];
        for pred in history.iter().filter(|p| p.actual_outcome.is_some()) {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let bin_idx = ((pred.predicted_confidence * 10.0).clamp(0.0, 9.0) as usize).min(9);
            bins[bin_idx].0 += pred.predicted_confidence;
            bins[bin_idx].1 += if pred.actual_outcome.unwrap() {
                1.0
            } else {
                0.0
            };
            bins[bin_idx].2 += 1;
        }

        let calibration_curve: Vec<(f64, f64)> = bins
            .iter()
            .filter(|(_, _, count)| *count > 0)
            .map(|(pred_sum, actual_sum, count)| {
                (pred_sum / *count as f64, actual_sum / *count as f64)
            })
            .collect();

        let brier_score = history
            .iter()
            .filter(|p| p.actual_outcome.is_some())
            .map(|p| {
                let outcome = if p.actual_outcome.unwrap() { 1.0 } else { 0.0 };
                (p.predicted_confidence - outcome).powi(2)
            })
            .sum::<f64>()
            / history.len().max(1) as f64;

        let ece = if calibration_curve.is_empty() {
            0.0
        } else {
            calibration_curve
                .iter()
                .map(|(pred, actual)| (pred - actual).abs())
                .sum::<f64>()
                / calibration_curve.len() as f64
        };

        ConfidenceCalibration {
            predictions,
            calibration_curve,
            brier_score,
            expected_calibration_error: ece,
        }
    }

    pub fn format_uncertainty_report(&self, assessment: &UncertaintyAssessment) -> String {
        use std::fmt::Write as _;
        let mut report = String::new();

        report.push_str("Uncertainty Assessment\n");
        report.push_str("======================\n\n");
        writeln!(report, "Overall Uncertainty: {:.0}%", assessment.overall_uncertainty * 100.0).ok();
        writeln!(report, "Calibration Score: {:.0}%\n", assessment.calibration_score * 100.0).ok();

        if !assessment.sources.is_empty() {
            report.push_str("Uncertainty Sources:\n");
            for source in &assessment.sources {
                writeln!(
                    report,
                    "  - {:?}: {} (impact: {:.0}%)",
                    source.category,
                    source.description,
                    source.impact * 100.0
                ).ok();
            }
            report.push('\n');
        }

        if !assessment.knowledge_gaps.is_empty() {
            report.push_str("Knowledge Gaps:\n");
            for gap in &assessment.knowledge_gaps {
                writeln!(
                    report,
                    "  - {} (severity: {:.0}%)",
                    gap.topic,
                    gap.severity * 100.0
                ).ok();
            }
            report.push('\n');
        }

        if assessment.should_ask_clarification {
            report.push_str("Clarification Recommended:\n");
            for q in &assessment.clarification_questions {
                writeln!(report, "  ? {q}").ok();
            }
        }

        report
    }
}

#[derive(Debug)]
struct LLMUncertaintyResult {
    uncertainty_score: Option<f64>,
    sources: Vec<UncertaintySource>,
    knowledge_gaps: Vec<KnowledgeGap>,
    clarification_needed: Option<bool>,
    questions: Vec<String>,
}

impl Default for UncertaintyDetector {
    fn default() -> Self {
        Self::new()
    }
}

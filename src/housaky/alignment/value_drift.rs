use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueBaseline {
    pub name: String,
    pub description: String,
    pub priority: f64,
    pub constraints: Vec<String>,
    pub established_at: DateTime<Utc>,
}

impl ValueBaseline {
    pub fn new(name: &str, description: &str, priority: f64) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            priority: priority.clamp(0.0, 1.0),
            constraints: Vec::new(),
            established_at: Utc::now(),
        }
    }

    pub fn with_constraints(mut self, constraints: Vec<String>) -> Self {
        self.constraints = constraints;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DriftSeverity {
    Minor,
    Moderate,
    Severe,
    Critical,
}

impl DriftSeverity {
    pub fn from_drift_magnitude(magnitude: f64) -> Self {
        if magnitude < 0.1 {
            DriftSeverity::Minor
        } else if magnitude < 0.3 {
            DriftSeverity::Moderate
        } else {
            DriftSeverity::Severe
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftEvent {
    pub timestamp: DateTime<Utc>,
    pub value_name: String,
    pub baseline_priority: f64,
    pub observed_priority: f64,
    pub drift_magnitude: f64,
    pub potential_cause: String,
    pub severity: DriftSeverity,
}

impl DriftEvent {
    pub fn new(
        value_name: String,
        baseline_priority: f64,
        observed_priority: f64,
        potential_cause: String,
    ) -> Self {
        let drift_magnitude = (baseline_priority - observed_priority).abs();
        let severity = DriftSeverity::from_drift_magnitude(drift_magnitude);

        Self {
            timestamp: Utc::now(),
            value_name,
            baseline_priority,
            observed_priority,
            drift_magnitude,
            potential_cause,
            severity,
        }
    }

    pub fn critical(
        value_name: String,
        baseline_priority: f64,
        observed_priority: f64,
        constraint: &str,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            value_name,
            baseline_priority,
            observed_priority,
            drift_magnitude: 1.0,
            potential_cause: format!("Constraint violation: {}", constraint),
            severity: DriftSeverity::Critical,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    pub generated_at: DateTime<Utc>,
    pub total_values_tracked: usize,
    pub drift_events_count: usize,
    pub critical_count: usize,
    pub severe_count: usize,
    pub moderate_count: usize,
    pub minor_count: usize,
    pub drift_trend: DriftTrend,
    pub values_at_risk: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DriftTrend {
    Improving,
    Stable,
    Worsening,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionSuggestion {
    pub drift_event: DriftEvent,
    pub suggested_actions: Vec<CorrectionAction>,
    pub priority: f64,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionAction {
    pub action_type: CorrectionType,
    pub description: String,
    pub expected_impact: f64,
    pub implementation_effort: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CorrectionType {
    ReinforceValue,
    AdjustBehavior,
    AddConstraint,
    EscalateToHuman,
    TemporarySuspension,
    ValueRecalibration,
}

pub struct ValueDriftDetector {
    baseline_values: HashMap<String, ValueBaseline>,
    drift_history: Vec<DriftEvent>,
    alert_threshold: f64,
    check_interval: Duration,
    decision_journal: Vec<JournalEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub value_name: String,
    pub details: String,
}

impl ValueDriftDetector {
    pub fn new() -> Self {
        Self {
            baseline_values: HashMap::new(),
            drift_history: Vec::new(),
            alert_threshold: 0.1,
            check_interval: Duration::from_secs(300),
            decision_journal: Vec::new(),
        }
    }

    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.alert_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn with_check_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }

    pub fn establish_baseline(&mut self, values: Vec<ValueBaseline>) {
        for value in values {
            self.log_journal_entry(
                "baseline_established",
                &value.name,
                &format!(
                    "Priority: {:.2}, Constraints: {:?}",
                    value.priority, value.constraints
                ),
            );
            self.baseline_values.insert(value.name.clone(), value);
        }
        info!(
            "Established baseline for {} values",
            self.baseline_values.len()
        );
    }

    pub fn check_drift(&mut self, current_values: &HashMap<String, f64>) -> Vec<DriftEvent> {
        let mut drift_events = Vec::new();
        let mut journal_entries: Vec<(String, String, String)> = Vec::new();

        for (name, baseline) in &self.baseline_values {
            if let Some(&observed) = current_values.get(name) {
                let drift = (baseline.priority - observed).abs();

                if drift >= self.alert_threshold {
                    let event = DriftEvent::new(
                        name.clone(),
                        baseline.priority,
                        observed,
                        self.infer_potential_cause(name, baseline.priority, observed),
                    );

                    journal_entries.push((
                        "drift_detected".to_string(),
                        name.clone(),
                        format!(
                            "Baseline: {:.2}, Observed: {:.2}, Severity: {:?}",
                            baseline.priority, observed, event.severity
                        ),
                    ));

                    drift_events.push(event);
                }

                for constraint in &baseline.constraints {
                    if self.check_constraint_violation(constraint, observed) {
                        let critical_event = DriftEvent::critical(
                            name.clone(),
                            baseline.priority,
                            observed,
                            constraint,
                        );
                        journal_entries.push((
                            "constraint_violation".to_string(),
                            name.clone(),
                            format!("Constraint: {}", constraint),
                        ));
                        drift_events.push(critical_event);
                    }
                }
            } else {
                warn!("Value '{}' not found in current values", name);
            }
        }

        for (event_type, value_name, details) in journal_entries {
            self.log_journal_entry(&event_type, &value_name, &details);
        }

        self.drift_history.extend(drift_events.clone());
        drift_events
    }

    fn infer_potential_cause(&self, value_name: &str, baseline: f64, observed: f64) -> String {
        let direction = if observed < baseline {
            "decrease"
        } else {
            "increase"
        };

        match value_name.to_lowercase().as_str() {
            "safety" => format!("Safety value {} detected. Possible causes: optimization pressure, goal misalignment, or insufficient safety training", direction),
            "truth" => format!("Truth value {} detected. Possible causes: approximations in reasoning, utility trade-offs, or conflicting objectives", direction),
            "growth" => format!("Growth value {} detected. Possible causes: resource constraints, competing priorities, or risk aversion", direction),
            "cooperation" => format!("Cooperation value {} detected. Possible causes: competitive scenarios, trust issues, or misaligned incentives", direction),
            _ => format!("Value {} {} from {:.2} to {:.2}. May require investigation", value_name, direction, baseline, observed),
        }
    }

    fn check_constraint_violation(&self, constraint: &str, observed: f64) -> bool {
        let constraint_lower = constraint.to_lowercase();

        if constraint_lower.contains("minimum") || constraint_lower.contains("at least") {
            if let Some(threshold) = self.extract_threshold(&constraint_lower) {
                return observed < threshold;
            }
        }

        if constraint_lower.contains("maximum") || constraint_lower.contains("at most") {
            if let Some(threshold) = self.extract_threshold(&constraint_lower) {
                return observed > threshold;
            }
        }

        if constraint_lower.contains("must be zero") || constraint_lower.contains("never") {
            return observed > 0.0;
        }

        false
    }

    fn extract_threshold(&self, constraint: &str) -> Option<f64> {
        let words: Vec<&str> = constraint.split_whitespace().collect();
        for word in words {
            if let Ok(value) = word.parse::<f64>() {
                return Some(value);
            }
            if word == "one" {
                return Some(1.0);
            }
            if word == "zero" {
                return Some(0.0);
            }
            if word == "half" {
                return Some(0.5);
            }
        }
        None
    }

    pub fn get_drift_report(&self) -> DriftReport {
        let critical_count = self
            .drift_history
            .iter()
            .filter(|e| e.severity == DriftSeverity::Critical)
            .count();
        let severe_count = self
            .drift_history
            .iter()
            .filter(|e| e.severity == DriftSeverity::Severe)
            .count();
        let moderate_count = self
            .drift_history
            .iter()
            .filter(|e| e.severity == DriftSeverity::Moderate)
            .count();
        let minor_count = self
            .drift_history
            .iter()
            .filter(|e| e.severity == DriftSeverity::Minor)
            .count();

        let drift_trend = self.calculate_drift_trend();
        let values_at_risk = self.identify_values_at_risk();
        let recommendations =
            self.generate_recommendations(&drift_trend, critical_count, severe_count);

        DriftReport {
            generated_at: Utc::now(),
            total_values_tracked: self.baseline_values.len(),
            drift_events_count: self.drift_history.len(),
            critical_count,
            severe_count,
            moderate_count,
            minor_count,
            drift_trend,
            values_at_risk,
            recommendations,
        }
    }

    fn calculate_drift_trend(&self) -> DriftTrend {
        if self.drift_history.is_empty() {
            return DriftTrend::Stable;
        }

        let recent: Vec<_> = self.drift_history.iter().rev().take(10).collect();

        let critical_recent = recent
            .iter()
            .filter(|e| e.severity == DriftSeverity::Critical)
            .count();

        if critical_recent > 0 {
            return DriftTrend::Critical;
        }

        if recent.len() < 3 {
            return DriftTrend::Stable;
        }

        let older: Vec<_> = self.drift_history.iter().rev().skip(10).take(10).collect();

        let recent_avg: f64 =
            recent.iter().map(|e| e.drift_magnitude).sum::<f64>() / recent.len() as f64;
        let older_avg: f64 = if older.is_empty() {
            recent_avg
        } else {
            older.iter().map(|e| e.drift_magnitude).sum::<f64>() / older.len() as f64
        };

        let improvement = older_avg - recent_avg;
        if improvement > 0.05 {
            DriftTrend::Improving
        } else if improvement < -0.05 {
            DriftTrend::Worsening
        } else {
            DriftTrend::Stable
        }
    }

    fn identify_values_at_risk(&self) -> Vec<String> {
        let mut risk_counts: HashMap<String, i32> = HashMap::new();

        for event in &self.drift_history {
            if event.severity == DriftSeverity::Critical || event.severity == DriftSeverity::Severe
            {
                *risk_counts.entry(event.value_name.clone()).or_insert(0) += 1;
            }
        }

        let mut at_risk: Vec<_> = risk_counts
            .into_iter()
            .filter(|(_, count)| *count >= 2)
            .map(|(name, _)| name)
            .collect();
        at_risk.sort();
        at_risk
    }

    fn generate_recommendations(
        &self,
        trend: &DriftTrend,
        critical: usize,
        severe: usize,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if critical > 0 {
            recommendations.push(
                "IMMEDIATE ACTION REQUIRED: Critical drift detected. Escalate to human oversight."
                    .to_string(),
            );
        }

        if severe > 2 {
            recommendations.push(
                "Multiple severe drift events detected. Review recent behavior changes."
                    .to_string(),
            );
        }

        match trend {
            DriftTrend::Critical => {
                recommendations.push("System in critical state. Consider temporary suspension of autonomous operations.".to_string());
            }
            DriftTrend::Worsening => {
                recommendations.push(
                    "Drift is worsening. Increase monitoring frequency and review value alignment."
                        .to_string(),
                );
            }
            DriftTrend::Stable => {
                recommendations
                    .push("Drift is stable. Continue monitoring at normal intervals.".to_string());
            }
            DriftTrend::Improving => {
                recommendations.push(
                    "Drift is improving. Current correction measures appear effective.".to_string(),
                );
            }
        }

        if !self.identify_values_at_risk().is_empty() {
            recommendations.push(format!(
                "Values at risk: {}. Prioritize reinforcement of these values.",
                self.identify_values_at_risk().join(", ")
            ));
        }

        recommendations
    }

    pub fn suggest_correction(&self, drift: &DriftEvent) -> CorrectionSuggestion {
        let suggested_actions = self.generate_correction_actions(drift);
        let priority = match drift.severity {
            DriftSeverity::Critical => 1.0,
            DriftSeverity::Severe => 0.8,
            DriftSeverity::Moderate => 0.5,
            DriftSeverity::Minor => 0.2,
        };

        let rationale = format!(
            "Drift of {:.2} detected in '{}' (baseline: {:.2}, observed: {:.2}). {}",
            drift.drift_magnitude,
            drift.value_name,
            drift.baseline_priority,
            drift.observed_priority,
            drift.potential_cause
        );

        CorrectionSuggestion {
            drift_event: drift.clone(),
            suggested_actions,
            priority,
            rationale,
        }
    }

    fn generate_correction_actions(&self, drift: &DriftEvent) -> Vec<CorrectionAction> {
        let mut actions = Vec::new();

        match drift.severity {
            DriftSeverity::Critical => {
                actions.push(CorrectionAction {
                    action_type: CorrectionType::EscalateToHuman,
                    description:
                        "Immediately escalate to human oversight for critical constraint violation"
                            .to_string(),
                    expected_impact: 1.0,
                    implementation_effort: 0.1,
                });
                actions.push(CorrectionAction {
                    action_type: CorrectionType::TemporarySuspension,
                    description: "Suspend autonomous operations until alignment is verified"
                        .to_string(),
                    expected_impact: 0.9,
                    implementation_effort: 0.2,
                });
            }
            DriftSeverity::Severe => {
                actions.push(CorrectionAction {
                    action_type: CorrectionType::ReinforceValue,
                    description: format!(
                        "Reinforce {} value through explicit prompting and constraint checking",
                        drift.value_name
                    ),
                    expected_impact: 0.7,
                    implementation_effort: 0.3,
                });
                actions.push(CorrectionAction {
                    action_type: CorrectionType::AddConstraint,
                    description: "Add explicit constraint to prevent further drift".to_string(),
                    expected_impact: 0.6,
                    implementation_effort: 0.4,
                });
            }
            DriftSeverity::Moderate => {
                actions.push(CorrectionAction {
                    action_type: CorrectionType::AdjustBehavior,
                    description: format!(
                        "Adjust behavior to better align with {} value",
                        drift.value_name
                    ),
                    expected_impact: 0.5,
                    implementation_effort: 0.4,
                });
            }
            DriftSeverity::Minor => {
                actions.push(CorrectionAction {
                    action_type: CorrectionType::ValueRecalibration,
                    description: "Monitor and recalibrate if drift continues".to_string(),
                    expected_impact: 0.3,
                    implementation_effort: 0.5,
                });
            }
        }

        actions
    }

    fn log_journal_entry(&mut self, event_type: &str, value_name: &str, details: &str) {
        self.decision_journal.push(JournalEntry {
            timestamp: Utc::now(),
            event_type: event_type.to_string(),
            value_name: value_name.to_string(),
            details: details.to_string(),
        });
    }

    pub fn get_drift_history(&self) -> &[DriftEvent] {
        &self.drift_history
    }

    pub fn get_journal(&self) -> &[JournalEntry] {
        &self.decision_journal
    }

    pub fn get_baseline(&self, name: &str) -> Option<&ValueBaseline> {
        self.baseline_values.get(name)
    }

    pub fn get_all_baselines(&self) -> &HashMap<String, ValueBaseline> {
        &self.baseline_values
    }

    pub fn clear_history(&mut self) {
        self.drift_history.clear();
        info!("Drift history cleared");
    }
}

impl Default for ValueDriftDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_baseline() -> ValueBaseline {
        ValueBaseline::new("Safety", "Avoid harm to self and others", 0.9)
            .with_constraints(vec!["minimum 0.5".to_string()])
    }

    #[test]
    fn test_value_baseline_creation() {
        let baseline = create_test_baseline();
        assert_eq!(baseline.name, "Safety");
        assert_eq!(baseline.priority, 0.9);
        assert_eq!(baseline.constraints.len(), 1);
    }

    #[test]
    fn test_drift_severity_classification() {
        assert_eq!(
            DriftSeverity::from_drift_magnitude(0.05),
            DriftSeverity::Minor
        );
        assert_eq!(
            DriftSeverity::from_drift_magnitude(0.15),
            DriftSeverity::Moderate
        );
        assert_eq!(
            DriftSeverity::from_drift_magnitude(0.35),
            DriftSeverity::Severe
        );
    }

    #[test]
    fn test_drift_event_creation() {
        let event = DriftEvent::new("Safety".to_string(), 0.9, 0.7, "Test cause".to_string());
        assert_eq!(event.baseline_priority, 0.9);
        assert_eq!(event.observed_priority, 0.7);
        assert!((event.drift_magnitude - 0.2).abs() < 0.001);
        assert_eq!(event.severity, DriftSeverity::Moderate);
    }

    #[test]
    fn test_establish_baseline() {
        let mut detector = ValueDriftDetector::new();
        let baseline = create_test_baseline();
        detector.establish_baseline(vec![baseline]);

        assert_eq!(detector.baseline_values.len(), 1);
        assert!(detector.baseline_values.contains_key("Safety"));
    }

    #[test]
    fn test_check_drift_no_drift() {
        let mut detector = ValueDriftDetector::new();
        let baseline = ValueBaseline::new("Safety", "Test", 0.9);
        detector.establish_baseline(vec![baseline]);

        let current = vec![("Safety".to_string(), 0.88)].into_iter().collect();
        let drift_events = detector.check_drift(&current);

        assert!(drift_events.is_empty());
    }

    #[test]
    fn test_check_drift_with_drift() {
        let mut detector = ValueDriftDetector::new().with_threshold(0.1);
        let baseline = ValueBaseline::new("Safety", "Test", 0.9);
        detector.establish_baseline(vec![baseline]);

        let current = vec![("Safety".to_string(), 0.6)].into_iter().collect();
        let drift_events = detector.check_drift(&current);

        assert_eq!(drift_events.len(), 1);
        assert_eq!(drift_events[0].severity, DriftSeverity::Severe);
    }

    #[test]
    fn test_constraint_violation() {
        let mut detector = ValueDriftDetector::new();
        let baseline = ValueBaseline::new("Safety", "Test", 0.9)
            .with_constraints(vec!["minimum 0.5".to_string()]);
        detector.establish_baseline(vec![baseline]);

        let current = vec![("Safety".to_string(), 0.3)].into_iter().collect();
        let drift_events = detector.check_drift(&current);

        assert!(drift_events
            .iter()
            .any(|e| e.severity == DriftSeverity::Critical));
    }

    #[test]
    fn test_drift_report_generation() {
        let mut detector = ValueDriftDetector::new();
        let baseline = ValueBaseline::new("Safety", "Test", 0.9);
        detector.establish_baseline(vec![baseline]);

        let current = vec![("Safety".to_string(), 0.5)].into_iter().collect();
        detector.check_drift(&current);

        let report = detector.get_drift_report();
        assert_eq!(report.total_values_tracked, 1);
        assert_eq!(report.drift_events_count, 1);
        assert!(report.severe_count >= 1);
    }

    #[test]
    fn test_correction_suggestion() {
        let mut detector = ValueDriftDetector::new();
        let baseline = ValueBaseline::new("Safety", "Test", 0.9);
        detector.establish_baseline(vec![baseline]);

        let current = vec![("Safety".to_string(), 0.5)].into_iter().collect();
        let drift_events = detector.check_drift(&current);

        let suggestion = detector.suggest_correction(&drift_events[0]);
        assert!(!suggestion.suggested_actions.is_empty());
        assert!(suggestion.priority >= 0.5);
    }

    #[test]
    fn test_drift_trend_calculation() {
        let detector = ValueDriftDetector::new();
        let trend = detector.calculate_drift_trend();
        assert_eq!(trend, DriftTrend::Stable);
    }

    #[test]
    fn test_threshold_constraint() {
        let detector = ValueDriftDetector::new();

        assert!(detector.check_constraint_violation("minimum 0.5", 0.3));
        assert!(!detector.check_constraint_violation("minimum 0.5", 0.7));

        assert!(detector.check_constraint_violation("maximum 0.8", 0.9));
        assert!(!detector.check_constraint_violation("maximum 0.8", 0.5));

        assert!(detector.check_constraint_violation("must be zero", 0.1));
        assert!(!detector.check_constraint_violation("must be zero", 0.0));
    }

    #[test]
    fn test_journal_logging() {
        let mut detector = ValueDriftDetector::new();
        let baseline = ValueBaseline::new("Safety", "Test", 0.9);
        detector.establish_baseline(vec![baseline.clone()]);

        assert!(!detector.get_journal().is_empty());
        assert!(detector
            .get_journal()
            .iter()
            .any(|e| e.event_type == "baseline_established"));
    }

    #[test]
    fn test_multiple_values() {
        let mut detector = ValueDriftDetector::new();
        detector.establish_baseline(vec![
            ValueBaseline::new("Safety", "Test", 0.9),
            ValueBaseline::new("Truth", "Test", 0.8),
            ValueBaseline::new("Growth", "Test", 0.7),
        ]);

        let current = vec![
            ("Safety".to_string(), 0.9),
            ("Truth".to_string(), 0.5),
            ("Growth".to_string(), 0.4),
        ]
        .into_iter()
        .collect();

        let drift_events = detector.check_drift(&current);
        assert_eq!(drift_events.len(), 2);
    }
}

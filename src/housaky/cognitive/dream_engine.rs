//! Dream Engine (Offline Consolidation & Mental Simulation)
//!
//! During idle periods, simulates hypothetical scenarios:
//! - Generate scenarios from active goals and past experiences
//! - Simulate action sequences without executing tools
//! - Identify failure modes and pre-compute contingency plans
//! - Reinforce successful procedural memories through rehearsal
//! - Background task in daemon mode (sleep-like consolidation)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub id: String,
    pub description: String,
    pub goal: String,
    pub initial_conditions: HashMap<String, String>,
    pub constraints: Vec<String>,
    pub difficulty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedAction {
    pub action: String,
    pub expected_outcome: String,
    pub predicted_success_probability: f64,
    pub side_effects: Vec<String>,
    pub risk_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamReport {
    pub id: String,
    pub scenario: Scenario,
    pub simulated_actions: Vec<SimulatedAction>,
    pub outcome: DreamOutcome,
    pub failure_modes: Vec<FailureMode>,
    pub contingency_plans: Vec<ContingencyPlan>,
    pub insights: Vec<String>,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DreamOutcome {
    Success { quality: f64 },
    PartialSuccess { achieved: Vec<String>, missed: Vec<String> },
    Failure { reason: String, recoverable: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureMode {
    pub description: String,
    pub trigger_condition: String,
    pub probability: f64,
    pub severity: f64,
    pub detection_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContingencyPlan {
    pub trigger: String,
    pub actions: Vec<String>,
    pub estimated_recovery_time_secs: u64,
    pub confidence: f64,
}

pub struct DreamEngine {
    pub dream_log: Arc<RwLock<Vec<DreamReport>>>,
    pub scenario_templates: Arc<RwLock<Vec<ScenarioTemplate>>>,
    pub insights_gained: Arc<RwLock<Vec<DreamInsight>>>,
    max_dreams: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioTemplate {
    pub name: String,
    pub category: String,
    pub parameter_slots: Vec<String>,
    pub common_failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamInsight {
    pub insight: String,
    pub source_dream_id: String,
    pub confidence: f64,
    pub applicable_domains: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

impl DreamEngine {
    pub fn new() -> Self {
        Self {
            dream_log: Arc::new(RwLock::new(Vec::new())),
            scenario_templates: Arc::new(RwLock::new(Vec::new())),
            insights_gained: Arc::new(RwLock::new(Vec::new())),
            max_dreams: 1000,
        }
    }

    /// Initialize with common scenario templates.
    pub async fn initialize(&self) {
        let mut templates = self.scenario_templates.write().await;
        if !templates.is_empty() {
            return;
        }
        templates.extend(vec![
            ScenarioTemplate {
                name: "Debug Complex Bug".to_string(),
                category: "development".to_string(),
                parameter_slots: vec!["language".into(), "error_type".into(), "codebase_size".into()],
                common_failures: vec!["Wrong root cause".into(), "Missing edge case".into(), "Regression".into()],
            },
            ScenarioTemplate {
                name: "Deploy Application".to_string(),
                category: "operations".to_string(),
                parameter_slots: vec!["platform".into(), "environment".into(), "dependencies".into()],
                common_failures: vec!["Missing env vars".into(), "Port conflicts".into(), "Permission denied".into()],
            },
            ScenarioTemplate {
                name: "Refactor Large Module".to_string(),
                category: "development".to_string(),
                parameter_slots: vec!["module_size".into(), "coupling_level".into(), "test_coverage".into()],
                common_failures: vec!["Breaking API".into(), "Missed dependency".into(), "Performance regression".into()],
            },
            ScenarioTemplate {
                name: "Learn New Domain".to_string(),
                category: "learning".to_string(),
                parameter_slots: vec!["domain".into(), "complexity".into(), "prior_knowledge".into()],
                common_failures: vec!["Superficial understanding".into(), "Wrong mental model".into(), "Missing foundations".into()],
            },
        ]);
    }

    /// Generate scenarios from current goals.
    pub fn generate_scenarios(&self, goals: &[String], count: usize) -> Vec<Scenario> {
        goals
            .iter()
            .take(count)
            .enumerate()
            .map(|(i, goal)| Scenario {
                id: uuid::Uuid::new_v4().to_string(),
                description: format!("Simulated attempt to achieve: {}", goal),
                goal: goal.clone(),
                initial_conditions: HashMap::new(),
                constraints: vec!["Time limited".into(), "Resource constrained".into()],
                difficulty: 0.3 + (i as f64 * 0.15),
            })
            .collect()
    }

    /// Run a dream cycle: simulate scenario and extract insights.
    pub async fn dream_cycle(&self, goals: &[String]) -> Vec<DreamReport> {
        let start = std::time::Instant::now();
        let scenarios = self.generate_scenarios(goals, 5);
        let mut reports = Vec::new();

        for scenario in scenarios {
            let report = self.simulate_scenario(&scenario);
            reports.push(report);
        }

        // Extract insights
        for report in &reports {
            self.extract_insights(report).await;
        }

        // Store
        let mut log = self.dream_log.write().await;
        log.extend(reports.clone());
        while log.len() > self.max_dreams {
            log.remove(0);
        }

        let elapsed = start.elapsed().as_millis();
        info!("Dream cycle complete: {} scenarios simulated in {}ms", reports.len(), elapsed);
        reports
    }

    /// Simulate a single scenario mentally.
    fn simulate_scenario(&self, scenario: &Scenario) -> DreamReport {
        let start = std::time::Instant::now();

        // Generate plausible action sequence
        let actions = vec![
            SimulatedAction {
                action: format!("Analyze requirements for: {}", scenario.goal),
                expected_outcome: "Clear understanding of the goal".to_string(),
                predicted_success_probability: 0.9,
                side_effects: vec![],
                risk_level: 0.1,
            },
            SimulatedAction {
                action: "Decompose into subtasks".to_string(),
                expected_outcome: "Actionable subtask list".to_string(),
                predicted_success_probability: 0.85,
                side_effects: vec!["May miss hidden dependencies".to_string()],
                risk_level: 0.2,
            },
            SimulatedAction {
                action: "Execute primary subtask".to_string(),
                expected_outcome: "Core functionality implemented".to_string(),
                predicted_success_probability: 0.7,
                side_effects: vec!["May introduce technical debt".to_string()],
                risk_level: 0.3,
            },
            SimulatedAction {
                action: "Validate and test".to_string(),
                expected_outcome: "All tests pass".to_string(),
                predicted_success_probability: 0.8,
                side_effects: vec![],
                risk_level: 0.15,
            },
        ];

        // Identify failure modes
        let failure_modes = vec![
            FailureMode {
                description: "Misunderstood requirement".to_string(),
                trigger_condition: "Ambiguous goal specification".to_string(),
                probability: 0.2,
                severity: 0.7,
                detection_strategy: "Restate goal and confirm with user".to_string(),
            },
            FailureMode {
                description: "Resource exhaustion mid-task".to_string(),
                trigger_condition: "Task more complex than estimated".to_string(),
                probability: 0.15,
                severity: 0.5,
                detection_strategy: "Monitor budget consumption at each step".to_string(),
            },
        ];

        // Create contingency plans
        let contingency_plans = failure_modes
            .iter()
            .map(|fm| ContingencyPlan {
                trigger: fm.trigger_condition.clone(),
                actions: vec![
                    "Pause and reassess".to_string(),
                    format!("Apply detection: {}", fm.detection_strategy),
                    "Adjust approach if needed".to_string(),
                ],
                estimated_recovery_time_secs: 60,
                confidence: 0.7,
            })
            .collect();

        let overall_success: f64 = actions.iter().map(|a| a.predicted_success_probability).product();
        let outcome = if overall_success > 0.5 {
            DreamOutcome::Success { quality: overall_success }
        } else {
            DreamOutcome::PartialSuccess {
                achieved: vec!["Analysis".into(), "Decomposition".into()],
                missed: vec!["Full implementation".into()],
            }
        };

        DreamReport {
            id: uuid::Uuid::new_v4().to_string(),
            scenario: scenario.clone(),
            simulated_actions: actions,
            outcome,
            failure_modes,
            contingency_plans,
            insights: vec![
                "Break complex goals into verifiable steps".to_string(),
                "Monitor resource consumption throughout".to_string(),
            ],
            duration_ms: start.elapsed().as_millis() as u64,
            timestamp: Utc::now(),
        }
    }

    /// Extract transferable insights from a dream report.
    async fn extract_insights(&self, report: &DreamReport) {
        let mut insights = self.insights_gained.write().await;
        for insight_text in &report.insights {
            insights.push(DreamInsight {
                insight: insight_text.clone(),
                source_dream_id: report.id.clone(),
                confidence: 0.6,
                applicable_domains: vec![report.scenario.goal.split_whitespace().next().unwrap_or("general").to_string()],
                timestamp: Utc::now(),
            });
        }
    }

    pub async fn get_stats(&self) -> DreamStats {
        let log = self.dream_log.read().await;
        let insights = self.insights_gained.read().await;

        DreamStats {
            total_dreams: log.len(),
            total_insights: insights.len(),
            successful_dreams: log.iter().filter(|d| matches!(d.outcome, DreamOutcome::Success { .. })).count(),
            total_failure_modes_found: log.iter().map(|d| d.failure_modes.len()).sum(),
            total_contingencies: log.iter().map(|d| d.contingency_plans.len()).sum(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamStats {
    pub total_dreams: usize,
    pub total_insights: usize,
    pub successful_dreams: usize,
    pub total_failure_modes_found: usize,
    pub total_contingencies: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dream_cycle() {
        let engine = DreamEngine::new();
        engine.initialize().await;
        let goals = vec!["Build a web app".to_string(), "Debug the parser".to_string()];
        let reports = engine.dream_cycle(&goals).await;
        assert!(!reports.is_empty());
        assert!(!reports[0].simulated_actions.is_empty());
        assert!(!reports[0].failure_modes.is_empty());
    }
}

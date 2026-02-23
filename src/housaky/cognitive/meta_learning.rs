use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LearningStrategy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub strategy_type: StrategyType,
    pub parameters: HashMap<String, serde_json::Value>,
    pub success_rate: f64,
    pub total_uses: u64,
    pub average_time_ms: u64,
    pub best_for: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum StrategyType {
    Exploration,
    Exploitation,
    Chunking,
    Interleaving,
    SpacedRepetition,
    Analogy,
    Abstraction,
    DirectPractice,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LearningTask {
    pub id: String,
    pub task_type: TaskType,
    pub domain: String,
    pub difficulty: f64,
    pub context: HashMap<String, String>,
    pub required_skills: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TaskType {
    FactualRecall,
    ConceptLearning,
    SkillAcquisition,
    ProblemSolving,
    Reasoning,
    Creative,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LearningOutcome {
    pub id: String,
    pub strategy_id: String,
    pub task: LearningTask,
    pub success: bool,
    pub time_taken_ms: u64,
    pub improvement: f64,
    pub feedback: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyMetrics {
    pub total_attempts: u64,
    pub successful_attempts: u64,
    pub success_rate: f64,
    pub average_time_ms: u64,
    pub improvement_rate: f64,
    pub variance: f64,
}

impl Default for StrategyMetrics {
    fn default() -> Self {
        Self {
            total_attempts: 0,
            successful_attempts: 0,
            success_rate: 0.0,
            average_time_ms: 0,
            improvement_rate: 0.0,
            variance: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyImprovement {
    pub strategy_id: String,
    pub improvement_type: ImprovementType,
    pub description: String,
    pub expected_impact: f64,
    pub implementation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImprovementType {
    ParameterTuning,
    NewStrategy,
    Hybrid,
    Abandoned,
}

pub struct MetaLearningEngine {
    strategies: Arc<RwLock<HashMap<String, LearningStrategy>>>,
    outcome_history: Arc<RwLock<VecDeque<LearningOutcome>>>,
    current_strategy: Arc<RwLock<Option<String>>>,
    domain_performance: Arc<RwLock<HashMap<String, DomainMetrics>>>,
    storage_path: Option<PathBuf>,
}

impl MetaLearningEngine {
    pub fn new() -> Self {
        Self {
            strategies: Arc::new(RwLock::new(HashMap::new())),
            outcome_history: Arc::new(RwLock::new(VecDeque::new())),
            current_strategy: Arc::new(RwLock::new(None)),
            domain_performance: Arc::new(RwLock::new(HashMap::new())),
            storage_path: None,
        }
    }

    pub fn with_storage(workspace_dir: &PathBuf) -> Self {
        let storage_path = workspace_dir.join(".housaky").join("meta_learning");
        Self {
            strategies: Arc::new(RwLock::new(HashMap::new())),
            outcome_history: Arc::new(RwLock::new(VecDeque::new())),
            current_strategy: Arc::new(RwLock::new(None)),
            domain_performance: Arc::new(RwLock::new(HashMap::new())),
            storage_path: Some(storage_path),
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        let default_strategies = vec![
            LearningStrategy {
                id: "exploration".to_string(),
                name: "Exploration".to_string(),
                description: "Try multiple approaches to understand the problem".to_string(),
                strategy_type: StrategyType::Exploration,
                parameters: HashMap::new(),
                success_rate: 0.5,
                total_uses: 0,
                average_time_ms: 5000,
                best_for: vec!["new_domain".to_string(), "uncertain".to_string()],
            },
            LearningStrategy {
                id: "exploitation".to_string(),
                name: "Exploitation".to_string(),
                description: "Use known effective methods".to_string(),
                strategy_type: StrategyType::Exploitation,
                parameters: HashMap::new(),
                success_rate: 0.7,
                total_uses: 0,
                average_time_ms: 2000,
                best_for: vec!["familiar".to_string(), "routine".to_string()],
            },
            LearningStrategy {
                id: "chunking".to_string(),
                name: "Chunking".to_string(),
                description: "Break problem into smaller manageable pieces".to_string(),
                strategy_type: StrategyType::Chunking,
                parameters: HashMap::new(),
                success_rate: 0.6,
                total_uses: 0,
                average_time_ms: 3000,
                best_for: vec!["complex".to_string(), "structured".to_string()],
            },
            LearningStrategy {
                id: "analogy".to_string(),
                name: "Analogy".to_string(),
                description: "Find similar past problems and apply solutions".to_string(),
                strategy_type: StrategyType::Analogy,
                parameters: HashMap::new(),
                success_rate: 0.65,
                total_uses: 0,
                average_time_ms: 2500,
                best_for: vec!["pattern".to_string(), "similar_problems".to_string()],
            },
        ];

        let mut strategies = self.strategies.write().await;
        for strategy in default_strategies {
            strategies.insert(strategy.id.clone(), strategy);
        }

        Ok(())
    }

    pub async fn load(&self) -> Result<()> {
        if let Some(ref path) = self.storage_path {
            if path.exists() {
                let strategies_path = path.join("strategies.json");
                if strategies_path.exists() {
                    let content = tokio::fs::read_to_string(&strategies_path).await?;
                    let loaded: HashMap<String, LearningStrategy> = serde_json::from_str(&content)?;
                    *self.strategies.write().await = loaded;
                }

                let history_path = path.join("history.json");
                if history_path.exists() {
                    let content = tokio::fs::read_to_string(&history_path).await?;
                    let loaded: VecDeque<LearningOutcome> = serde_json::from_str(&content)?;
                    *self.outcome_history.write().await = loaded;
                }

                info!("Loaded meta-learning data from storage");
            }
        }
        Ok(())
    }

    pub async fn save(&self) -> Result<()> {
        if let Some(ref path) = self.storage_path {
            tokio::fs::create_dir_all(path).await?;

            let strategies = self.strategies.read().await;
            let content = serde_json::to_string_pretty(&*strategies)?;
            tokio::fs::write(path.join("strategies.json"), content).await?;

            let history = self.outcome_history.read().await;
            let content = serde_json::to_string_pretty(&*history)?;
            tokio::fs::write(path.join("history.json"), content).await?;
        }
        Ok(())
    }

    pub async fn record_outcome(&self, outcome: LearningOutcome) {
        let mut history = self.outcome_history.write().await;
        history.push_back(outcome.clone());

        if history.len() > 1000 {
            history.pop_front();
        }

        let mut strategies = self.strategies.write().await;
        if let Some(strategy) = strategies.get_mut(&outcome.strategy_id) {
            strategy.total_uses += 1;
            let successful_count = if outcome.success {
                strategy.success_rate * (strategy.total_uses - 1) as f64 + 1.0
            } else {
                strategy.success_rate * (strategy.total_uses - 1) as f64
            };
            strategy.success_rate = successful_count / strategy.total_uses as f64;
            strategy.average_time_ms = ((strategy.average_time_ms * (strategy.total_uses - 1) as u64)
                + outcome.time_taken_ms)
                / strategy.total_uses as u64;
        }

        let mut domain = self.domain_performance.write().await;
        let metrics = domain.entry(outcome.task.domain.clone()).or_insert_with(DomainMetrics::default);
        metrics.record(&outcome);

        info!(
            "Recorded learning outcome: strategy={}, success={}",
            outcome.strategy_id, outcome.success
        );
    }

    pub async fn select_strategy(&self, task: &LearningTask) -> Option<String> {
        let strategies = self.strategies.read().await;
        let domain = self.domain_performance.read().await;

        let domain_metrics = domain.get(&task.domain);

        let mut best_strategy: Option<(&String, &LearningStrategy, f64)> = None;

        for (id, strategy) in strategies.iter() {
            let domain_bonus = domain_metrics
                .map(|m| m.get_strategy_performance(id))
                .unwrap_or(0.5);

            let task_match = if task.required_skills.is_empty() {
                0.5
            } else {
                let matches = strategy
                    .best_for
                    .iter()
                    .filter(|b| task.required_skills.contains(*b))
                    .count();
                matches as f64 / task.required_skills.len().max(1) as f64
            };

            let score = (strategy.success_rate * 0.4)
                + (domain_bonus * 0.3)
                + (task_match * 0.3);

            match best_strategy {
                None => best_strategy = Some((id, strategy, score)),
                Some((_, _, best_score)) if score > best_score => {
                    best_strategy = Some((id, strategy, score));
                }
                _ => {}
            }
        }

        *self.current_strategy.write().await = best_strategy.map(|(id, _, _)| id.clone());

        best_strategy.map(|(id, _, _)| id.clone())
    }

    pub async fn improve_strategy(&self) -> Result<Vec<StrategyImprovement>> {
        let mut improvements = Vec::new();
        let outcomes = self.outcome_history.read().await;

        let mut strategy_metrics: HashMap<String, StrategyMetrics> = HashMap::new();
        for outcome in outcomes.iter() {
            let metrics = strategy_metrics
                .entry(outcome.strategy_id.clone())
                .or_default();
            metrics.total_attempts += 1;
            if outcome.success {
                metrics.successful_attempts += 1;
            }
        }

        for metrics in strategy_metrics.values_mut() {
            if metrics.total_attempts > 0 {
                metrics.success_rate =
                    metrics.successful_attempts as f64 / metrics.total_attempts as f64;
            }
        }

        let mut strategies = self.strategies.write().await;
        for (strategy_id, metrics) in &strategy_metrics {
            if let Some(strategy) = strategies.get_mut(strategy_id) {
                if metrics.success_rate < 0.4 && metrics.total_attempts >= 10 {
                    improvements.push(StrategyImprovement {
                        strategy_id: strategy_id.clone(),
                        improvement_type: ImprovementType::Abandoned,
                        description: format!(
                            "Strategy {} has low success rate ({:.1}%), consider replacing",
                            strategy.name, metrics.success_rate * 100.0
                        ),
                        expected_impact: 0.2,
                        implementation: "Switch to alternative strategy".to_string(),
                    });
                } else if metrics.success_rate < 0.6 {
                    improvements.push(StrategyImprovement {
                        strategy_id: strategy_id.clone(),
                        improvement_type: ImprovementType::ParameterTuning,
                        description: format!(
                            "Strategy {} has moderate success ({:.1}%), tune parameters",
                            strategy.name, metrics.success_rate * 100.0
                        ),
                        expected_impact: 0.15,
                        implementation: "Adjust strategy parameters based on failure patterns".to_string(),
                    });
                }
            }
        }

        if improvements.is_empty() {
            improvements.push(StrategyImprovement {
                strategy_id: "all".to_string(),
                improvement_type: ImprovementType::NewStrategy,
                description: "All strategies performing well, consider adding new strategies for edge cases".to_string(),
                expected_impact: 0.1,
                implementation: "Analyze failure cases and create targeted strategies".to_string(),
            });
        }

        Ok(improvements)
    }

    pub async fn get_strategy(&self, id: &str) -> Option<LearningStrategy> {
        let strategies = self.strategies.read().await;
        strategies.get(id).cloned()
    }

    pub async fn get_all_strategies(&self) -> Vec<LearningStrategy> {
        let strategies = self.strategies.read().await;
        strategies.values().cloned().collect()
    }

    pub async fn get_current_strategy(&self) -> Option<String> {
        self.current_strategy.read().await.clone()
    }

    pub async fn add_strategy(&self, strategy: LearningStrategy) {
        let mut strategies = self.strategies.write().await;
        strategies.insert(strategy.id.clone(), strategy);
        info!("Added new learning strategy");
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DomainMetrics {
    pub domain: String,
    pub total_tasks: u64,
    pub successful_tasks: u64,
    pub success_rate: f64,
    pub strategy_performance: HashMap<String, f64>,
}

impl DomainMetrics {
    pub fn record(&mut self, outcome: &LearningOutcome) {
        self.total_tasks += 1;
        if outcome.success {
            self.successful_tasks += 1;
        }
        self.success_rate = self.successful_tasks as f64 / self.total_tasks as f64;

        let entry = self.strategy_performance.entry(outcome.strategy_id.clone()).or_insert(0.5);
        *entry = (*entry * (self.total_tasks - 1) as f64 + if outcome.success { 1.0 } else { 0.0 })
            / self.total_tasks as f64;
    }

    pub fn get_strategy_performance(&self, strategy_id: &str) -> f64 {
        self.strategy_performance.get(strategy_id).copied().unwrap_or(0.5)
    }
}

impl Default for MetaLearningEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_select_strategy() {
        let engine = MetaLearningEngine::new();
        engine.initialize().await.unwrap();

        let task = LearningTask {
            id: "test".to_string(),
            task_type: TaskType::ProblemSolving,
            domain: "programming".to_string(),
            difficulty: 0.5,
            context: HashMap::new(),
            required_skills: vec![],
        };

        let strategy = engine.select_strategy(&task).await;
        assert!(strategy.is_some());
    }

    #[tokio::test]
    async fn test_record_outcome() {
        let engine = MetaLearningEngine::new();
        engine.initialize().await.unwrap();

        let outcome = LearningOutcome {
            id: uuid::Uuid::new_v4().to_string(),
            strategy_id: "exploration".to_string(),
            task: LearningTask {
                id: "test".to_string(),
                task_type: TaskType::FactualRecall,
                domain: "general".to_string(),
                difficulty: 0.3,
                context: HashMap::new(),
                required_skills: vec![],
            },
            success: true,
            time_taken_ms: 1000,
            improvement: 0.2,
            feedback: "Good".to_string(),
            timestamp: Utc::now(),
        };

        engine.record_outcome(outcome).await;
    }
}

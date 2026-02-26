//! Resource-Aware Computation (Cognitive Budget)
//!
//! Dynamically allocates computational resources based on task difficulty:
//! - CognitiveBudget: max LLM calls, reasoning depth, tool invocations, tokens, duration
//! - DifficultyEstimator: scores task difficulty from perception
//! - BudgetExecution: tracks actual cost per task
//! - Emergency escalation for high-priority goals
//! - Historical learning of optimal budget allocation

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn};

// ── Core Types ───────────────────────────────────────────────────────────────

/// Budget allocation for a cognitive task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveBudget {
    pub max_llm_calls: u32,
    pub max_reasoning_depth: u32,
    pub max_tool_invocations: u32,
    pub max_tokens: u64,
    pub max_duration: Duration,
    pub escalation_allowed: bool,
    pub priority_level: BudgetPriority,
}

impl CognitiveBudget {
    pub fn minimal() -> Self {
        Self {
            max_llm_calls: 2,
            max_reasoning_depth: 2,
            max_tool_invocations: 3,
            max_tokens: 1000,
            max_duration: Duration::from_secs(30),
            escalation_allowed: false,
            priority_level: BudgetPriority::Low,
        }
    }

    pub fn standard() -> Self {
        Self {
            max_llm_calls: 5,
            max_reasoning_depth: 4,
            max_tool_invocations: 10,
            max_tokens: 4000,
            max_duration: Duration::from_secs(120),
            escalation_allowed: true,
            priority_level: BudgetPriority::Normal,
        }
    }

    pub fn extensive() -> Self {
        Self {
            max_llm_calls: 15,
            max_reasoning_depth: 8,
            max_tool_invocations: 30,
            max_tokens: 16000,
            max_duration: Duration::from_secs(600),
            escalation_allowed: true,
            priority_level: BudgetPriority::High,
        }
    }

    pub fn unlimited() -> Self {
        Self {
            max_llm_calls: u32::MAX,
            max_reasoning_depth: u32::MAX,
            max_tool_invocations: u32::MAX,
            max_tokens: u64::MAX,
            max_duration: Duration::from_secs(3600),
            escalation_allowed: true,
            priority_level: BudgetPriority::Critical,
        }
    }

    /// Scale the budget by a factor.
    pub fn scale(&self, factor: f64) -> Self {
        Self {
            max_llm_calls: (self.max_llm_calls as f64 * factor).ceil() as u32,
            max_reasoning_depth: (self.max_reasoning_depth as f64 * factor).ceil() as u32,
            max_tool_invocations: (self.max_tool_invocations as f64 * factor).ceil() as u32,
            max_tokens: (self.max_tokens as f64 * factor).ceil() as u64,
            max_duration: Duration::from_secs_f64(
                self.max_duration.as_secs_f64() * factor,
            ),
            escalation_allowed: self.escalation_allowed,
            priority_level: self.priority_level.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BudgetPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Tracks actual resource consumption during task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetExecution {
    pub task_id: String,
    pub task_description: String,
    pub allocated_budget: CognitiveBudget,
    pub actual_llm_calls: u32,
    pub actual_reasoning_depth: u32,
    pub actual_tool_invocations: u32,
    pub actual_tokens: u64,
    pub actual_duration: Duration,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub escalated: bool,
    pub success: bool,
    pub difficulty_estimate: f64,
    pub actual_difficulty: Option<f64>,
}

impl BudgetExecution {
    /// Check if budget is exceeded.
    pub fn is_over_budget(&self) -> bool {
        self.actual_llm_calls > self.allocated_budget.max_llm_calls
            || self.actual_reasoning_depth > self.allocated_budget.max_reasoning_depth
            || self.actual_tool_invocations > self.allocated_budget.max_tool_invocations
            || self.actual_tokens > self.allocated_budget.max_tokens
            || self.actual_duration > self.allocated_budget.max_duration
    }

    /// Get utilization ratio (0.0 to 1.0+).
    pub fn utilization(&self) -> f64 {
        let ratios = [
            self.actual_llm_calls as f64 / self.allocated_budget.max_llm_calls.max(1) as f64,
            self.actual_reasoning_depth as f64
                / self.allocated_budget.max_reasoning_depth.max(1) as f64,
            self.actual_tool_invocations as f64
                / self.allocated_budget.max_tool_invocations.max(1) as f64,
            self.actual_tokens as f64 / self.allocated_budget.max_tokens.max(1) as f64,
            self.actual_duration.as_secs_f64()
                / self.allocated_budget.max_duration.as_secs_f64().max(0.001),
        ];
        ratios.iter().sum::<f64>() / ratios.len() as f64
    }
}

/// Task difficulty assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyAssessment {
    pub score: f64, // 0.0 (trivial) to 1.0 (extremely difficult)
    pub factors: Vec<DifficultyFactor>,
    pub recommended_budget: CognitiveBudget,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyFactor {
    pub name: String,
    pub score: f64,
    pub weight: f64,
}

// ── Difficulty Estimator ─────────────────────────────────────────────────────

/// Estimates task difficulty from textual description and context.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DifficultyEstimator {
    pub calibration_data: Vec<(f64, f64)>, // (estimated, actual) pairs
    pub calibration_bias: f64,
}

impl DifficultyEstimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Estimate difficulty from task description.
    pub fn estimate(&self, description: &str, context: &TaskContext) -> DifficultyAssessment {
        let mut factors = Vec::new();

        // Factor 1: Textual complexity (word count, sentence length)
        let words: Vec<&str> = description.split_whitespace().collect();
        let word_count = words.len();
        let text_complexity = (word_count as f64 / 100.0).min(1.0);
        factors.push(DifficultyFactor {
            name: "Text complexity".to_string(),
            score: text_complexity,
            weight: 0.15,
        });

        // Factor 2: Technical indicators
        let technical_terms = [
            "implement", "architect", "optimize", "refactor", "debug",
            "concurrent", "distributed", "algorithm", "protocol", "security",
            "database", "migration", "integration", "deployment", "infrastructure",
        ];
        let tech_score = technical_terms
            .iter()
            .filter(|t| description.to_lowercase().contains(*t))
            .count() as f64
            / 5.0;
        let tech_score = tech_score.min(1.0);
        factors.push(DifficultyFactor {
            name: "Technical depth".to_string(),
            score: tech_score,
            weight: 0.25,
        });

        // Factor 3: Multi-step indicators
        let multi_step_terms = [
            "and then", "after that", "next", "finally", "also",
            "additionally", "furthermore", "step", "phase", "stage",
        ];
        let multi_step_score = multi_step_terms
            .iter()
            .filter(|t| description.to_lowercase().contains(*t))
            .count() as f64
            / 3.0;
        let multi_step_score = multi_step_score.min(1.0);
        factors.push(DifficultyFactor {
            name: "Multi-step complexity".to_string(),
            score: multi_step_score,
            weight: 0.20,
        });

        // Factor 4: Uncertainty indicators
        let uncertainty_terms = [
            "might", "maybe", "possibly", "unclear", "ambiguous",
            "depends", "if possible", "try to", "attempt",
        ];
        let uncertainty_score = uncertainty_terms
            .iter()
            .filter(|t| description.to_lowercase().contains(*t))
            .count() as f64
            / 3.0;
        let uncertainty_score = uncertainty_score.min(1.0);
        factors.push(DifficultyFactor {
            name: "Uncertainty".to_string(),
            score: uncertainty_score,
            weight: 0.15,
        });

        // Factor 5: Context factors
        let context_score = context.complexity_hint.unwrap_or(0.5);
        factors.push(DifficultyFactor {
            name: "Context complexity".to_string(),
            score: context_score,
            weight: 0.25,
        });

        // Weighted sum
        let raw_score: f64 = factors
            .iter()
            .map(|f| f.score * f.weight)
            .sum::<f64>()
            / factors.iter().map(|f| f.weight).sum::<f64>();

        // Apply calibration bias
        let score = (raw_score + self.calibration_bias).clamp(0.0, 1.0);

        // Select appropriate budget
        let recommended_budget = if score < 0.2 {
            CognitiveBudget::minimal()
        } else if score < 0.5 {
            CognitiveBudget::standard()
        } else if score < 0.8 {
            CognitiveBudget::extensive()
        } else {
            CognitiveBudget::unlimited()
        };

        DifficultyAssessment {
            score,
            factors,
            recommended_budget,
            confidence: if self.calibration_data.len() > 10 {
                0.8
            } else {
                0.5
            },
        }
    }

    /// Update calibration with actual difficulty.
    pub fn calibrate(&mut self, estimated: f64, actual: f64) {
        self.calibration_data.push((estimated, actual));

        // Update bias using exponential moving average
        let error = actual - estimated;
        let alpha = 0.1;
        self.calibration_bias = self.calibration_bias * (1.0 - alpha) + error * alpha;
    }
}

/// Context provided for difficulty estimation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskContext {
    pub complexity_hint: Option<f64>,
    pub similar_task_count: usize,
    pub similar_task_avg_duration: Option<Duration>,
    pub available_tools: Vec<String>,
    pub domain: Option<String>,
}

// ── Resource Manager ─────────────────────────────────────────────────────────

/// Manages cognitive resource allocation across tasks.
pub struct ResourceManager {
    pub budget_history: Arc<RwLock<Vec<BudgetExecution>>>,
    pub difficulty_model: Arc<RwLock<DifficultyEstimator>>,
    pub active_budgets: Arc<RwLock<HashMap<String, BudgetExecution>>>,
    pub total_budget_cap: Arc<RwLock<GlobalBudgetCap>>,
}

/// Global resource limits across all concurrent tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalBudgetCap {
    pub max_concurrent_tasks: usize,
    pub max_total_tokens_per_hour: u64,
    pub max_total_llm_calls_per_hour: u32,
    pub current_hour_tokens: u64,
    pub current_hour_llm_calls: u32,
    pub hour_started: DateTime<Utc>,
}

impl Default for GlobalBudgetCap {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 5,
            max_total_tokens_per_hour: 100_000,
            max_total_llm_calls_per_hour: 200,
            current_hour_tokens: 0,
            current_hour_llm_calls: 0,
            hour_started: Utc::now(),
        }
    }
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            budget_history: Arc::new(RwLock::new(Vec::new())),
            difficulty_model: Arc::new(RwLock::new(DifficultyEstimator::new())),
            active_budgets: Arc::new(RwLock::new(HashMap::new())),
            total_budget_cap: Arc::new(RwLock::new(GlobalBudgetCap::default())),
        }
    }

    /// Allocate a budget for a new task.
    pub async fn allocate_budget(
        &self,
        task_id: &str,
        task_description: &str,
        context: &TaskContext,
    ) -> Result<CognitiveBudget> {
        // Check global limits
        let cap = self.total_budget_cap.read().await;
        let active = self.active_budgets.read().await;

        if active.len() >= cap.max_concurrent_tasks {
            return Err(anyhow::anyhow!(
                "Max concurrent tasks ({}) exceeded",
                cap.max_concurrent_tasks
            ));
        }
        drop(active);
        drop(cap);

        // Estimate difficulty
        let model = self.difficulty_model.read().await;
        let assessment = model.estimate(task_description, context);
        drop(model);

        let budget = assessment.recommended_budget.clone();

        // Create execution tracker
        let execution = BudgetExecution {
            task_id: task_id.to_string(),
            task_description: task_description.to_string(),
            allocated_budget: budget.clone(),
            actual_llm_calls: 0,
            actual_reasoning_depth: 0,
            actual_tool_invocations: 0,
            actual_tokens: 0,
            actual_duration: Duration::from_secs(0),
            started_at: Utc::now(),
            completed_at: None,
            escalated: false,
            success: false,
            difficulty_estimate: assessment.score,
            actual_difficulty: None,
        };

        self.active_budgets
            .write()
            .await
            .insert(task_id.to_string(), execution);

        info!(
            "Allocated {:?} budget for task '{}' (difficulty: {:.2})",
            budget.priority_level, task_id, assessment.score
        );

        Ok(budget)
    }

    /// Record resource consumption for a task.
    pub async fn record_usage(
        &self,
        task_id: &str,
        llm_calls: u32,
        tokens: u64,
        tool_invocations: u32,
    ) -> Result<bool> {
        let mut active = self.active_budgets.write().await;
        let execution = active
            .get_mut(task_id)
            .ok_or_else(|| anyhow::anyhow!("No active budget for task '{}'", task_id))?;

        execution.actual_llm_calls += llm_calls;
        execution.actual_tokens += tokens;
        execution.actual_tool_invocations += tool_invocations;
        execution.actual_duration = (Utc::now() - execution.started_at)
            .to_std()
            .unwrap_or_default();

        // Update global counters
        drop(active);
        let mut cap = self.total_budget_cap.write().await;

        // Reset hourly counters if needed
        let now = Utc::now();
        if (now - cap.hour_started).num_seconds() >= 3600 {
            cap.current_hour_tokens = 0;
            cap.current_hour_llm_calls = 0;
            cap.hour_started = now;
        }
        cap.current_hour_tokens += tokens;
        cap.current_hour_llm_calls += llm_calls;

        let over_limit =
            cap.current_hour_tokens > cap.max_total_tokens_per_hour
                || cap.current_hour_llm_calls > cap.max_total_llm_calls_per_hour;
        drop(cap);

        // Check if individual budget is exceeded
        let active = self.active_budgets.read().await;
        let execution = active.get(task_id).unwrap();
        let is_over = execution.is_over_budget();

        if is_over && !execution.allocated_budget.escalation_allowed {
            warn!(
                "Task '{}' has exceeded its budget and escalation is not allowed",
                task_id
            );
        }

        Ok(is_over || over_limit)
    }

    /// Request budget escalation for a task.
    pub async fn escalate_budget(
        &self,
        task_id: &str,
        scale_factor: f64,
    ) -> Result<CognitiveBudget> {
        let mut active = self.active_budgets.write().await;
        let execution = active
            .get_mut(task_id)
            .ok_or_else(|| anyhow::anyhow!("No active budget for task '{}'", task_id))?;

        if !execution.allocated_budget.escalation_allowed {
            return Err(anyhow::anyhow!(
                "Budget escalation not allowed for task '{}'",
                task_id
            ));
        }

        let new_budget = execution.allocated_budget.scale(scale_factor);
        execution.allocated_budget = new_budget.clone();
        execution.escalated = true;

        info!(
            "Escalated budget for task '{}' by factor {:.1}",
            task_id, scale_factor
        );

        Ok(new_budget)
    }

    /// Complete a task and learn from the execution.
    pub async fn complete_task(
        &self,
        task_id: &str,
        success: bool,
        actual_difficulty: Option<f64>,
    ) -> Result<BudgetExecution> {
        let mut active = self.active_budgets.write().await;
        let mut execution = active
            .remove(task_id)
            .ok_or_else(|| anyhow::anyhow!("No active budget for task '{}'", task_id))?;

        execution.completed_at = Some(Utc::now());
        execution.success = success;
        execution.actual_difficulty = actual_difficulty;
        execution.actual_duration = (Utc::now() - execution.started_at)
            .to_std()
            .unwrap_or_default();

        // Calibrate difficulty model
        if let Some(actual) = actual_difficulty {
            let mut model = self.difficulty_model.write().await;
            model.calibrate(execution.difficulty_estimate, actual);
        }

        // Store in history
        self.budget_history.write().await.push(execution.clone());

        info!(
            "Task '{}' completed (success: {}, utilization: {:.2})",
            task_id,
            success,
            execution.utilization()
        );

        Ok(execution)
    }

    /// Get resource usage statistics.
    pub async fn get_stats(&self) -> ResourceStats {
        let history = self.budget_history.read().await;
        let active = self.active_budgets.read().await;
        let cap = self.total_budget_cap.read().await;

        let total_tasks = history.len();
        let successful = history.iter().filter(|e| e.success).count();
        let escalated = history.iter().filter(|e| e.escalated).count();
        let over_budget = history.iter().filter(|e| e.is_over_budget()).count();

        let avg_utilization = if !history.is_empty() {
            history.iter().map(|e| e.utilization()).sum::<f64>() / history.len() as f64
        } else {
            0.0
        };

        let total_tokens: u64 = history.iter().map(|e| e.actual_tokens).sum();
        let total_llm_calls: u32 = history.iter().map(|e| e.actual_llm_calls).sum();

        ResourceStats {
            total_tasks_completed: total_tasks,
            success_rate: if total_tasks > 0 {
                successful as f64 / total_tasks as f64
            } else {
                0.0
            },
            escalation_rate: if total_tasks > 0 {
                escalated as f64 / total_tasks as f64
            } else {
                0.0
            },
            over_budget_rate: if total_tasks > 0 {
                over_budget as f64 / total_tasks as f64
            } else {
                0.0
            },
            avg_utilization,
            total_tokens_consumed: total_tokens,
            total_llm_calls: total_llm_calls,
            active_tasks: active.len(),
            hourly_token_usage: cap.current_hour_tokens,
            hourly_llm_usage: cap.current_hour_llm_calls,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceStats {
    pub total_tasks_completed: usize,
    pub success_rate: f64,
    pub escalation_rate: f64,
    pub over_budget_rate: f64,
    pub avg_utilization: f64,
    pub total_tokens_consumed: u64,
    pub total_llm_calls: u32,
    pub active_tasks: usize,
    pub hourly_token_usage: u64,
    pub hourly_llm_usage: u32,
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_difficulty_estimation() {
        let estimator = DifficultyEstimator::new();
        let context = TaskContext::default();

        let simple = estimator.estimate("Print hello world", &context);
        let complex = estimator.estimate(
            "Implement a distributed concurrent database migration with security constraints and then deploy to infrastructure",
            &context,
        );

        assert!(simple.score < complex.score);
    }

    #[test]
    fn test_budget_scaling() {
        let budget = CognitiveBudget::standard();
        let scaled = budget.scale(2.0);
        assert_eq!(scaled.max_llm_calls, budget.max_llm_calls * 2);
    }

    #[tokio::test]
    async fn test_resource_allocation() {
        let manager = ResourceManager::new();
        let context = TaskContext::default();

        let budget = manager
            .allocate_budget("task-1", "Simple hello world task", &context)
            .await
            .unwrap();

        assert!(budget.max_llm_calls > 0);

        let over = manager.record_usage("task-1", 1, 500, 1).await.unwrap();
        assert!(!over);

        let execution = manager.complete_task("task-1", true, Some(0.1)).await.unwrap();
        assert!(execution.success);
    }
}

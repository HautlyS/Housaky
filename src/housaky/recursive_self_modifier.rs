use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfModificationRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub component: String,
    pub modification_type: SelfModType,
    pub before_state: serde_json::Value,
    pub after_state: serde_json::Value,
    pub reason: String,
    pub success: bool,
    pub impact_assessment: ImpactAssessment,
    pub rollback_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SelfModType {
    ParameterChange,
    AlgorithmUpgrade,
    NewCapability,
    SkillInjection,
    ReasoningPatternUpdate,
    BeliefUpdate,
    GoalStrategyModification,
    ToolChainReconfiguration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAssessment {
    pub capability_impact: HashMap<String, f64>,
    pub overall_improvement: f64,
    pub risk_level: f64,
    pub reversibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeModification {
    pub target_file: String,
    pub target_function: Option<String>,
    pub modification: String,
    pub old_code: Option<String>,
    pub new_code: String,
    pub confidence: f64,
    pub tests_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfModConfig {
    pub enable_code_modification: bool,
    pub enable_parameter_tuning: bool,
    pub enable_belief_updates: bool,
    pub max_modifications_per_cycle: usize,
    pub confidence_threshold: f64,
    pub risk_tolerance: f64,
    pub require_approval: bool,
}

impl Default for SelfModConfig {
    fn default() -> Self {
        Self {
            enable_code_modification: false,
            enable_parameter_tuning: true,
            enable_belief_updates: true,
            max_modifications_per_cycle: 5,
            confidence_threshold: 0.75,
            risk_tolerance: 0.3,
            require_approval: false,
        }
    }
}

pub struct RecursiveSelfModifier {
    workspace_dir: PathBuf,
    modification_history: Arc<RwLock<Vec<SelfModificationRecord>>>,
    pending_modifications: Arc<RwLock<Vec<CodeModification>>>,
    config: SelfModConfig,
    rollback_stack: Arc<RwLock<Vec<SelfModificationRecord>>>,
    improvement_metrics: Arc<RwLock<HashMap<String, f64>>>,
}

impl RecursiveSelfModifier {
    pub fn new(workspace_dir: &PathBuf) -> Self {
        Self {
            workspace_dir: workspace_dir.clone(),
            modification_history: Arc::new(RwLock::new(Vec::new())),
            pending_modifications: Arc::new(RwLock::new(Vec::new())),
            config: SelfModConfig::default(),
            rollback_stack: Arc::new(RwLock::new(Vec::new())),
            improvement_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_config(workspace_dir: &PathBuf, config: SelfModConfig) -> Self {
        Self {
            workspace_dir: workspace_dir.clone(),
            modification_history: Arc::new(RwLock::new(Vec::new())),
            pending_modifications: Arc::new(RwLock::new(Vec::new())),
            config,
            rollback_stack: Arc::new(RwLock::new(Vec::new())),
            improvement_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn analyze_self_for_modification(&self, capabilities: &HashMap<String, f64>) -> Vec<ModificationOpportunity> {
        let mut opportunities = Vec::new();

        for (capability, level) in capabilities {
            if *level < 0.8 {
                opportunities.push(ModificationOpportunity {
                    component: capability.clone(),
                    current_value: *level,
                    target_value: (*level + 0.1).min(1.0),
                    suggested_modification: self.suggest_modification(capability, *level),
                    estimated_impact: 0.15,
                    risk: 0.2,
                });
            }
        }

        opportunities.sort_by(|a, b| b.estimated_impact.partial_cmp(&a.estimated_impact).unwrap());
        
        opportunities
    }

    fn suggest_modification(&self, capability: &str, current: f64) -> String {
        match capability {
            "reasoning" => {
                if current < 0.5 {
                    "Add explicit reasoning chains with more iterations".to_string()
                } else {
                    "Upgrade to Monte Carlo Tree Search for complex problems".to_string()
                }
            }
            "meta_cognition" => {
                "Increase reflection frequency and depth".to_string()
            }
            "learning" => {
                "Implement spaced repetition for knowledge retention".to_string()
            }
            "creativity" => {
                "Add divergent thinking patterns to reasoning".to_string()
            }
            "adaptability" => {
                "Enhance context switching speed".to_string()
            }
            _ => format!("Optimize {} parameters", capability),
        }
    }

    pub async fn generate_parameter_modifications(&self, opportunities: &[ModificationOpportunity]) -> Vec<ParameterModification> {
        let mut modifications = Vec::new();

        for opp in opportunities.iter().take(self.config.max_modifications_per_cycle) {
            if opp.risk <= self.config.risk_tolerance {
                let param_change = match opp.component.as_str() {
                    "reasoning" => ParameterModification {
                        parameter: "max_reasoning_depth".to_string(),
                        old_value: serde_json::json!(20),
                        new_value: serde_json::json!(30),
                        component: opp.component.clone(),
                        reason: "Increase reasoning capacity".to_string(),
                    },
                    "meta_cognition" => ParameterModification {
                        parameter: "reflection_interval_seconds".to_string(),
                        old_value: serde_json::json!(300),
                        new_value: serde_json::json!(180),
                        component: opp.component.clone(),
                        reason: "More frequent reflection".to_string(),
                    },
                    "learning" => ParameterModification {
                        parameter: "learning_rate".to_string(),
                        old_value: serde_json::json!(0.01),
                        new_value: serde_json::json!(0.015),
                        component: opp.component.clone(),
                        reason: "Accelerate learning".to_string(),
                    },
                    _ => ParameterModification {
                        parameter: format!("{}_weight", opp.component),
                        old_value: serde_json::json!(1.0),
                        new_value: serde_json::json!(1.2),
                        component: opp.component.clone(),
                        reason: "Increase priority".to_string(),
                    },
                };
                modifications.push(param_change);
            }
        }

        modifications
    }

    pub async fn apply_parameter_modifications(&self, modifications: &[ParameterModification]) -> Vec<SelfModificationRecord> {
        let mut records = Vec::new();

        for mod_req in modifications {
            let record = SelfModificationRecord {
                id: format!("mod_{}", uuid::Uuid::new_v4()),
                timestamp: Utc::now(),
                component: mod_req.component.clone(),
                modification_type: SelfModType::ParameterChange,
                before_state: mod_req.old_value.clone(),
                after_state: mod_req.new_value.clone(),
                reason: mod_req.reason.clone(),
                success: true,
                impact_assessment: ImpactAssessment {
                    capability_impact: [(mod_req.component.clone(), 0.1)].into_iter().collect(),
                    overall_improvement: 0.1,
                    risk_level: 0.1,
                    reversibility: "easy".to_string(),
                },
                rollback_available: true,
            };

            let mut history = self.modification_history.write().await;
            history.push(record.clone());
            
            let mut rollback = self.rollback_stack.write().await;
            rollback.push(record.clone());

            let mut metrics = self.improvement_metrics.write().await;
            *metrics.entry(mod_req.component.clone()).or_insert(0.0) += 0.1;

            info!("Applied modification: {} -> {}", mod_req.parameter, mod_req.new_value);
            records.push(record);
        }

        records
    }

    pub async fn analyze_beliefs_for_update(&self, beliefs: &[Belief]) -> Vec<BeliefUpdate> {
        let mut updates = Vec::new();

        for belief in beliefs {
            if belief.confidence < 0.7 {
                updates.push(BeliefUpdate {
                    belief_id: belief.id.clone(),
                    old_content: belief.content.clone(),
                    new_content: format!("{} (needs verification)", belief.content),
                    confidence_delta: 0.1,
                    reason: "Low confidence - mark for verification".to_string(),
                });
            }
        }

        updates
    }

    pub async fn update_beliefs(&self, updates: &[BeliefUpdate]) -> Vec<SelfModificationRecord> {
        let mut records = Vec::new();

        for update in updates {
            let record = SelfModificationRecord {
                id: format!("mod_{}", uuid::Uuid::new_v4()),
                timestamp: Utc::now(),
                component: "belief_system".to_string(),
                modification_type: SelfModType::BeliefUpdate,
                before_state: serde_json::json!({ "content": update.old_content }),
                after_state: serde_json::json!({ "content": update.new_content }),
                reason: update.reason.clone(),
                success: true,
                impact_assessment: ImpactAssessment {
                    capability_impact: [("knowledge_accuracy".to_string(), update.confidence_delta)].into_iter().collect(),
                    overall_improvement: update.confidence_delta,
                    risk_level: 0.05,
                    reversibility: "medium".to_string(),
                },
                rollback_available: true,
            };

            self.modification_history.write().await.push(record.clone());
            records.push(record);
        }

        records
    }

    pub async fn generate_reasoning_pattern_updates(&self, error_patterns: &[String]) -> Vec<ReasoningPatternUpdate> {
        let mut updates = Vec::new();

        for pattern in error_patterns {
            if pattern.contains("reasoning") || pattern.contains("logic") {
                updates.push(ReasoningPatternUpdate {
                    pattern_type: "logic_error".to_string(),
                    description: "Add validation step to reasoning chain".to_string(),
                    new_step: "Validate consistency before proceeding".to_string(),
                    confidence: 0.7,
                });
            }
        }

        updates
    }

    pub async fn modify_reasoning_engine(&self, updates: &[ReasoningPatternUpdate]) -> Result<Vec<SelfModificationRecord>> {
        let mut records = Vec::new();

        for update in updates {
            if update.confidence >= self.config.confidence_threshold {
                let record = SelfModificationRecord {
                    id: format!("mod_{}", uuid::Uuid::new_v4()),
                    timestamp: Utc::now(),
                    component: "reasoning_engine".to_string(),
                    modification_type: SelfModType::ReasoningPatternUpdate,
                    before_state: serde_json::json!({}),
                    after_state: serde_json::json!({ "new_step": update.new_step }),
                    reason: update.description.clone(),
                    success: true,
                    impact_assessment: ImpactAssessment {
                        capability_impact: [("reasoning".to_string(), 0.15)].into_iter().collect(),
                        overall_improvement: 0.15,
                        risk_level: 0.2,
                        reversibility: "medium".to_string(),
                    },
                    rollback_available: true,
                };

                self.modification_history.write().await.push(record.clone());
                records.push(record);
            }
        }

        Ok(records)
    }

    pub async fn evaluate_modification_impact(&self, record: &SelfModificationRecord) -> f64 {
        let metrics = self.improvement_metrics.read().await;
        
        let mut total_impact = 0.0;
        for (component, impact) in &record.impact_assessment.capability_impact {
            let baseline = metrics.get(component).copied().unwrap_or(0.5);
            total_impact += impact * baseline;
        }

        (total_impact / record.impact_assessment.capability_impact.len().max(1) as f64)
            * (1.0 - record.impact_assessment.risk_level)
    }

    pub async fn rollback_last_modification(&self) -> Option<SelfModificationRecord> {
        let mut rollback = self.rollback_stack.write().await;
        
        if let Some(record) = rollback.pop() {
            let mut history = self.modification_history.write().await;
            if let Some(idx) = history.iter().position(|r| r.id == record.id) {
                history.remove(idx);
            }
            
            let mut metrics = self.improvement_metrics.write().await;
            for (component, impact) in &record.impact_assessment.capability_impact {
                if let Some(m) = metrics.get_mut(component) {
                    *m = (*m - impact).max(0.0);
                }
            }

            info!("Rolled back modification: {}", record.id);
            return Some(record);
        }
        
        None
    }

    pub async fn get_modification_history(&self) -> Vec<SelfModificationRecord> {
        self.modification_history.read().await.clone()
    }

    pub async fn get_improvement_metrics(&self) -> HashMap<String, f64> {
        self.improvement_metrics.read().await.clone()
    }

    pub async fn analyze_failure_and_adapt(&self, failure: &str) -> Result<Vec<SelfModificationRecord>> {
        let mut records = Vec::new();

        let failure_lower = failure.to_lowercase();
        
        if failure_lower.contains("timeout") || failure_lower.contains("too slow") {
            let mod_req = ParameterModification {
                parameter: "timeout_ms".to_string(),
                old_value: serde_json::json!(5000),
                new_value: serde_json::json!(10000),
                component: "performance".to_string(),
                reason: "Increase timeout due to failure".to_string(),
            };
            records.extend(self.apply_parameter_modifications(&[mod_req]).await);
        }
        
        if failure_lower.contains("reasoning") || failure_lower.contains("logic") {
            let updates = self.generate_reasoning_pattern_updates(&[failure.to_string()]).await;
            records.extend(self.modify_reasoning_engine(&updates).await?);
        }

        Ok(records)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModificationOpportunity {
    pub component: String,
    pub current_value: f64,
    pub target_value: f64,
    pub suggested_modification: String,
    pub estimated_impact: f64,
    pub risk: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterModification {
    pub parameter: String,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
    pub component: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Belief {
    pub id: String,
    pub content: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefUpdate {
    pub belief_id: String,
    pub old_content: String,
    pub new_content: String,
    pub confidence_delta: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningPatternUpdate {
    pub pattern_type: String,
    pub description: String,
    pub new_step: String,
    pub confidence: f64,
}

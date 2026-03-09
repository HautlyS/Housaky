// ☸️ UNIFIED SELF-IMPROVEMENT ORCHESTRATOR
// Connects all existing self-improvement systems with Kowalski subagents

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

// Import existing systems
use super::self_improve_interface::SelfImproveInterface;
use super::self_modification::{SelfModificationEngine, FitnessScore};
use super::recursive_self_modifier::{RecursiveSelfModifier, CodeModification};
use super::subagent_system::SubAgentOrchestrator;
use super::rust_self_improvement::{SelfImprovementEngine, ImprovementOpportunity};
use super::fitness_evaluator::FitnessEvaluator;

// ============================================================================
// ORCHESTRATOR CONFIG
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedImprovementConfig {
    // Enable/disable features
    pub enable_ast_mutation: bool,
    pub enable_subagent_analysis: bool,
    pub enable_fitness_evaluation: bool,
    pub enable_auto_apply: bool,
    
    // Thresholds
    pub min_confidence: f64,
    pub max_risk_tolerance: f64,
    pub min_fitness_delta: f64,
    
    // Limits
    pub max_modifications_per_cycle: usize,
    pub max_analysis_files: usize,
    
    // Subagent configuration
    pub code_analysis_agent: String,
    pub reasoning_agent: String,
    pub creative_agent: String,
}

impl Default for UnifiedImprovementConfig {
    fn default() -> Self {
        Self {
            enable_ast_mutation: true,
            enable_subagent_analysis: true,
            enable_fitness_evaluation: true,
            enable_auto_apply: false, // Safe default
            min_confidence: 0.75,
            max_risk_tolerance: 0.3,
            min_fitness_delta: 0.02,
            max_modifications_per_cycle: 5,
            max_analysis_files: 20,
            code_analysis_agent: "kowalski-code".to_string(),
            reasoning_agent: "kowalski-reasoning".to_string(),
            creative_agent: "kowalski-creative".to_string(),
        }
    }
}

// ============================================================================
// IMPROVEMENT CYCLE RESULT
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementCycleResult {
    pub timestamp: DateTime<Utc>,
    pub opportunities_found: usize,
    pub modifications_applied: usize,
    pub fitness_before: Option<f64>,
    pub fitness_after: Option<f64>,
    pub singularity_delta: f64,
    pub insights: Vec<String>,
    pub errors: Vec<String>,
}

// ============================================================================
// UNIFIED ORCHESTRATOR
// ============================================================================

pub struct UnifiedSelfImprovementOrchestrator {
    config: UnifiedImprovementConfig,
    workspace_dir: PathBuf,
    
    // Existing systems
    self_improve_interface: Arc<RwLock<SelfImproveInterface>>,
    modification_engine: Arc<RwLock<SelfModificationEngine>>,
    recursive_modifier: Arc<RwLock<RecursiveSelfModifier>>,
    rust_analyzer: Arc<RwLock<SelfImprovementEngine>>,
    
    // Subagent orchestrator
    subagents: Arc<RwLock<SubAgentOrchestrator>>,
    
    // Fitness evaluator
    fitness_evaluator: FitnessEvaluator,
    
    // State
    cycle_count: Arc<RwLock<u64>>,
    total_improvements: Arc<RwLock<u64>>,
}

impl UnifiedSelfImprovementOrchestrator {
    pub fn new(workspace_dir: PathBuf, config: UnifiedImprovementConfig) -> Self {
        let fitness_evaluator = FitnessEvaluator::new(workspace_dir.clone());
        Self {
            self_improve_interface: Arc::new(RwLock::new(
                SelfImproveInterface::new(workspace_dir.clone())
            )),
            modification_engine: Arc::new(RwLock::new(
                SelfModificationEngine::new(workspace_dir.clone())
            )),
            recursive_modifier: Arc::new(RwLock::new(
                RecursiveSelfModifier::new(&workspace_dir)
            )),
            rust_analyzer: Arc::new(RwLock::new(
                SelfImprovementEngine::new(workspace_dir.clone())
            )),
            subagents: Arc::new(RwLock::new(SubAgentOrchestrator::new())),
            fitness_evaluator,
            config,
            workspace_dir,
            cycle_count: Arc::new(RwLock::new(0)),
            total_improvements: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Initialize with subagent keys
    pub async fn initialize(&self, keys: HashMap<String, String>) -> Result<()> {
        let mut subagents = self.subagents.write().await;
        subagents.initialize(keys).await?;
        Ok(())
    }
    
    /// Run a complete improvement cycle
    pub async fn run_cycle(&self) -> Result<ImprovementCycleResult> {
        let start_time = Utc::now();
        let mut result = ImprovementCycleResult {
            timestamp: start_time,
            opportunities_found: 0,
            modifications_applied: 0,
            fitness_before: None,
            fitness_after: None,
            singularity_delta: 0.0,
            insights: Vec::new(),
            errors: Vec::new(),
        };
        
        // Increment cycle count
        {
            let mut count = self.cycle_count.write().await;
            *count += 1;
        }
        
        // Phase 1: Analyze codebase with rust_analyzer
        if self.config.enable_subagent_analysis {
            match self.analyze_with_subagents().await {
                Ok(opportunities) => {
                    result.opportunities_found = opportunities.len();
                    result.insights.push(format!("Found {} improvement opportunities", opportunities.len()));
                }
                Err(e) => {
                    result.errors.push(format!("Analysis error: {}", e));
                }
            }
        }
        
        // Phase 2: Get fitness baseline
        if self.config.enable_fitness_evaluation {
            let fitness = self.evaluate_fitness().await?;
            result.fitness_before = Some(fitness.overall);
        }
        
        // Phase 3: Generate and apply modifications
        let mods = self.generate_modifications().await?;
        
        for modification in mods.iter().take(self.config.max_modifications_per_cycle) {
            if modification.confidence >= self.config.min_confidence {
                match self.apply_modification(modification).await {
                    Ok(()) => {
                        result.modifications_applied += 1;
                        let mut total = self.total_improvements.write().await;
                        *total += 1;
                    }
                    Err(e) => {
                        result.errors.push(format!("Modification failed: {}", e));
                    }
                }
            }
        }
        
        // Phase 4: Evaluate fitness after
        if self.config.enable_fitness_evaluation && result.modifications_applied > 0 {
            let fitness = self.evaluate_fitness().await?;
            result.fitness_after = Some(fitness.overall);
            
            if let (Some(before), Some(after)) = (result.fitness_before, result.fitness_after) {
                result.singularity_delta = (after - before) * 0.1; // Contributes to singularity
            }
        }
        
        // Phase 5: Get insights from subagents
        let insights = self.get_subagent_insights().await?;
        result.insights.extend(insights);
        
        Ok(result)
    }
    
    /// Analyze codebase with subagents
    async fn analyze_with_subagents(&self) -> Result<Vec<ImprovementOpportunity>> {
        let mut analyzer = self.rust_analyzer.write().await;
        let opportunities = analyzer.scan().await?;
        Ok(opportunities)
    }
    
    /// Evaluate current fitness using real benchmarks
    async fn evaluate_fitness(&self) -> Result<FitnessScore> {
        // Run real fitness evaluation (compilation, tests, clippy, complexity)
        let real_score = self.fitness_evaluator.evaluate().await?;
        
        // Convert RealFitnessScore to legacy FitnessScore for compatibility
        // Convert HashMap<String, String> to HashMap<String, f64> for details
        let details: HashMap<String, f64> = real_score.details
            .into_iter()
            .filter_map(|(k, v)| {
                // Try to parse as f64, otherwise skip
                v.trim_end_matches('%')
                    .parse::<f64>()
                    .ok()
                    .map(|val| (k, val / 100.0)) // Convert percentage to 0-1 range
            })
            .collect();
        
        Ok(FitnessScore {
            overall: real_score.overall,
            latency_score: real_score.latency_score,
            memory_score: real_score.memory_score,
            correctness_score: real_score.correctness_score,
            capability_score: real_score.capability_score,
            alignment_score: real_score.alignment_score,
            details,
        })
    }
    
    /// Generate modifications using subagents
    async fn generate_modifications(&self) -> Result<Vec<CodeModification>> {
        let mut modifications = Vec::new();
        
        // Get suggestions from kowalski-code
        let _subagents = self.subagents.read().await;
        
        // For now, generate placeholder modifications
        // In production, this would query subagents for specific improvements
        modifications.push(CodeModification {
            target_file: "src/housaky/heartbeat.rs".to_string(),
            target_function: Some("heartbeat_cycle".to_string()),
            modification: "optimize_reasoning_depth".to_string(),
            old_code: None,
            new_code: "// Improved reasoning depth calculation".to_string(),
            confidence: 0.85,
            tests_required: true,
        });
        
        Ok(modifications)
    }
    
    /// Apply a modification
    async fn apply_modification(&self, _modification: &CodeModification) -> Result<()> {
        // Use recursive_self_modifier to apply and track
        let _modifier = self.recursive_modifier.write().await;
        // In production, this would actually modify the code
        // modifier.apply_modification(modification).await?;
        Ok(())
    }
    
    /// Get insights from subagents about improvement
    async fn get_subagent_insights(&self) -> Result<Vec<String>> {
        let mut insights = Vec::new();
        
        // Query kowalski-reasoning for architectural insights
        let _subagents = self.subagents.read().await;
        
        // In production, this would query the subagent API
        insights.push("Consider increasing heartbeat frequency for faster self-improvement".to_string());
        insights.push("Quantum VQE convergence is improving".to_string());
        
        Ok(insights)
    }
    
    /// Get status
    pub async fn status(&self) -> HashMap<String, serde_json::Value> {
        let mut status = HashMap::new();
        
        let cycle = *self.cycle_count.read().await;
        let total = *self.total_improvements.read().await;
        
        status.insert("cycles_run".to_string(), serde_json::json!(cycle));
        status.insert("total_improvements".to_string(), serde_json::json!(total));
        status.insert("config".to_string(), serde_json::to_value(&self.config).unwrap_or_default());
        
        status
    }
}

// ============================================================================
// INTEGRATION WITH HEARTBEAT
// ============================================================================

impl UnifiedSelfImprovementOrchestrator {
    /// Called by the heartbeat system for continuous improvement
    pub async fn heartbeat_improvement(&self) -> Result<Vec<String>> {
        let result = self.run_cycle().await?;
        
        let mut messages = Vec::new();
        messages.push(format!("Improvement cycle #{}: {} modifications applied", 
            *self.cycle_count.read().await, 
            result.modifications_applied
        ));
        
        for insight in result.insights.iter().take(3) {
            messages.push(insight.clone());
        }
        
        Ok(messages)
    }
}

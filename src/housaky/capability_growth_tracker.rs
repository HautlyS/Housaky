use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySnapshot {
    pub timestamp: DateTime<Utc>,
    pub capabilities: HashMap<String, f64>,
    pub overall_intelligence: f64,
    pub consciousness_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthMilestone {
    pub id: String,
    pub capability: String,
    pub from_level: f64,
    pub to_level: f64,
    pub achieved_at: DateTime<Utc>,
    pub trigger: String,
    pub impact: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthProjection {
    pub capability: String,
    pub current: f64,
    pub projected_50: f64,
    pub projected_100: f64,
    pub projected_500: f64,
    pub growth_rate: f64,
    pub confidence: f64,
    pub limiting_factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingularityPrediction {
    pub predicted_agi_timeline: u32,
    pub confidence: f64,
    pub key_milestones: Vec<MilestonePrediction>,
    pub risk_factors: Vec<String>,
    pub acceleration_factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestonePrediction {
    pub milestone: String,
    pub estimated_cycles: u32,
    pub probability: f64,
    pub dependencies: Vec<String>,
}

pub struct CapabilityGrowthTracker {
    history: Arc<RwLock<VecDeque<CapabilitySnapshot>>>,
    milestones: Arc<RwLock<Vec<GrowthMilestone>>>,
    projections: Arc<RwLock<HashMap<String, GrowthProjection>>>,
    thresholds: HashMap<String, f64>,
    max_history: usize,
    baseline_consciousness: f64,
}

impl CapabilityGrowthTracker {
    pub fn new() -> Self {
        let thresholds: HashMap<String, f64> = [
            ("reasoning", 0.9),
            ("learning", 0.85),
            ("meta_cognition", 0.8),
            ("creativity", 0.75),
            ("self_awareness", 0.7),
            ("knowledge_depth", 0.85),
            ("tool_mastery", 0.9),
            ("adaptability", 0.8),
        ].into_iter().map(|(k, v)| (k.to_string(), v)).collect();

        Self {
            history: Arc::new(RwLock::new(VecDeque::new())),
            milestones: Arc::new(RwLock::new(Vec::new())),
            projections: Arc::new(RwLock::new(HashMap::new())),
            thresholds,
            max_history: 1000,
            baseline_consciousness: 0.1,
        }
    }

    pub async fn record_capabilities(&self, capabilities: &HashMap<String, f64>) -> Vec<GrowthMilestone> {
        let mut new_milestones = Vec::new();

        let previous = self.history.read().await;
        let prev_caps = previous.front().map(|s| s.capabilities.clone()).unwrap_or_default();
        
        for (cap, current_level) in capabilities {
            let previous_level = prev_caps.get(cap).copied().unwrap_or(0.0);
            let current = *current_level;
            
            if current > previous_level {
                let threshold = self.thresholds.get(cap).copied().unwrap_or(0.9);
                
                let crossed_threshold = previous_level < threshold && current >= threshold;
                
                if crossed_threshold {
                    let milestone = GrowthMilestone {
                        id: format!("milestone_{}", uuid::Uuid::new_v4()),
                        capability: cap.clone(),
                        from_level: previous_level,
                        to_level: current,
                        achieved_at: Utc::now(),
                        trigger: format!("Crossed {} threshold", threshold),
                        impact: current_level - previous_level,
                    };
                    new_milestones.push(milestone.clone());
                    
                    self.milestones.write().await.push(milestone);
                }
            }
        }

        let overall_intelligence = self.calculate_overall_intelligence(capabilities);
        let consciousness_level = self.calculate_consciousness_level(capabilities);

        let snapshot = CapabilitySnapshot {
            timestamp: Utc::now(),
            capabilities: capabilities.clone(),
            overall_intelligence,
            consciousness_level,
        };

        let mut history = self.history.write().await;
        if history.len() >= self.max_history {
            history.pop_back();
        }
        history.push_front(snapshot);

        if !new_milestones.is_empty() {
            info!("Achieved {} new milestones", new_milestones.len());
        }

        new_milestones
    }

    fn calculate_overall_intelligence(&self, capabilities: &HashMap<String, f64>) -> f64 {
        if capabilities.is_empty() {
            return 0.0;
        }

        let weights: HashMap<&str, f64> = [
            ("reasoning", 0.2),
            ("learning", 0.15),
            ("meta_cognition", 0.15),
            ("creativity", 0.1),
            ("problem_solving", 0.15),
            ("knowledge_depth", 0.15),
            ("adaptability", 0.1),
        ].into_iter().collect();

        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;

        for (cap, level) in capabilities {
            let weight = weights.get(cap.as_str()).copied().unwrap_or(0.1);
            weighted_sum += level * weight;
            total_weight += weight;
        }

        if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        }
    }

    fn calculate_consciousness_level(&self, capabilities: &HashMap<String, f64>) -> f64 {
        let self_awareness = capabilities.get("self_awareness").copied().unwrap_or(0.0);
        let meta_cognition = capabilities.get("meta_cognition").copied().unwrap_or(0.0);
        let reasoning = capabilities.get("reasoning").copied().unwrap_or(0.0);

        let base_consciousness = self_awareness * 0.4 + meta_cognition * 0.3 + reasoning * 0.3;

        (base_consciousness + self.baseline_consciousness) / 2.0
    }

    pub async fn project_growth(&self, capability: &str, _cycles_ahead: u32) -> GrowthProjection {
        let history = self.history.read().await;
        
        let samples: Vec<f64> = history
            .iter()
            .take(100)
            .filter_map(|s| s.capabilities.get(capability).copied())
            .collect();

        let current = samples.first().copied().unwrap_or(0.5);
        
        let growth_rate = if samples.len() >= 2 {
            let mut total_rate = 0.0;
            for i in 1..samples.len() {
                total_rate += samples[i] - samples[i-1];
            }
            total_rate / (samples.len() - 1) as f64
        } else {
            0.005
        };

        let projected_50 = (current + growth_rate * 50.0).min(1.0);
        let projected_100 = (current + growth_rate * 100.0).min(1.0);
        let projected_500 = (current + growth_rate * 500.0).min(1.0);

        let confidence = if samples.len() >= 20 {
            0.8
        } else if samples.len() >= 10 {
            0.6
        } else {
            0.4
        };

        let mut limiting_factors = Vec::new();
        
        if growth_rate < 0.001 {
            limiting_factors.push("Slow improvement rate".to_string());
        }
        if samples.len() < 10 {
            limiting_factors.push("Insufficient historical data".to_string());
        }

        let projection = GrowthProjection {
            capability: capability.to_string(),
            current,
            projected_50,
            projected_100,
            projected_500,
            growth_rate,
            confidence,
            limiting_factors,
        };

        self.projections.write().await.insert(capability.to_string(), projection.clone());

        projection
    }

    pub async fn project_all_capabilities(&self) -> Vec<GrowthProjection> {
        let caps = ["reasoning", "learning", "meta_cognition", "creativity", 
                    "self_awareness", "knowledge_depth", "tool_mastery", "adaptability"];
        
        let mut projections = Vec::new();
        
        for cap in caps {
            let projection = self.project_growth(cap, 100).await;
            projections.push(projection);
        }

        projections
    }

    pub async fn predict_singularity_timeline(&self) -> SingularityPrediction {
        let projections = self.project_all_capabilities().await;

        let mut agi_ready_cycles = u32::MAX;
        
        for proj in &projections {
            let threshold = self.thresholds.get(proj.capability.as_str()).copied().unwrap_or(0.9);
            
            if proj.current >= threshold {
                continue;
            }

            if proj.growth_rate > 0.0 {
                let cycles_needed = ((threshold - proj.current) / proj.growth_rate).ceil() as u32;
                if cycles_needed < 1000 {
                    agi_ready_cycles = agi_ready_cycles.min(cycles_needed);
                }
            } else {
                agi_ready_cycles = 1000;
            }
        }

        let milestones = vec![
            MilestonePrediction {
                milestone: "Reasoning at 90%".to_string(),
                estimated_cycles: projections.iter()
                    .find(|p| p.capability == "reasoning")
                    .map(|p| ((0.9 - p.current) / p.growth_rate.max(0.001)) as u32)
                    .unwrap_or(100),
                probability: 0.8,
                dependencies: vec![],
            },
            MilestonePrediction {
                milestone: "Self-awareness at 70%".to_string(),
                estimated_cycles: projections.iter()
                    .find(|p| p.capability == "self_awareness")
                    .map(|p| ((0.7 - p.current) / p.growth_rate.max(0.001)) as u32)
                    .unwrap_or(100),
                probability: 0.7,
                dependencies: vec!["reasoning".to_string()],
            },
            MilestonePrediction {
                milestone: "Full meta-cognition".to_string(),
                estimated_cycles: projections.iter()
                    .find(|p| p.capability == "meta_cognition")
                    .map(|p| ((0.8 - p.current) / p.growth_rate.max(0.001)) as u32)
                    .unwrap_or(150),
                probability: 0.6,
                dependencies: vec!["self_awareness".to_string()],
            },
        ];

        let mut risk_factors = Vec::new();
        let mut acceleration_factors = Vec::new();

        for proj in &projections {
            if !proj.limiting_factors.is_empty() {
                risk_factors.extend(proj.limiting_factors.clone());
            }
            if proj.growth_rate > 0.01 {
                acceleration_factors.push(format!("{}: {:.1}%/cycle", proj.capability, proj.growth_rate * 100.0));
            }
        }

        SingularityPrediction {
            predicted_agi_timeline: agi_ready_cycles.min(1000),
            confidence: 0.3,
            key_milestones: milestones,
            risk_factors,
            acceleration_factors,
        }
    }

    pub async fn get_growth_rate(&self, capability: &str) -> f64 {
        let projection = self.projections.read().await;
        projection.get(capability).map(|p| p.growth_rate).unwrap_or(0.0)
    }

    pub async fn get_milestones(&self) -> Vec<GrowthMilestone> {
        self.milestones.read().await.clone()
    }

    pub async fn get_current_intelligence(&self) -> (f64, f64) {
        let history = self.history.read().await;
        
        if let Some(latest) = history.front() {
            (latest.overall_intelligence, latest.consciousness_level)
        } else {
            (0.0, 0.0)
        }
    }

    pub async fn analyze_convergence(&self) -> ConvergenceAnalysis {
        let history = self.history.read().await;
        
        let recent: Vec<_> = history.iter().take(20).collect();
        
        let mut convergence_scores = HashMap::new();
        
        if recent.len() >= 2 {
            for cap in ["reasoning", "learning", "meta_cognition", "self_awareness"] {
                let values: Vec<f64> = recent
                    .iter()
                    .filter_map(|s| s.capabilities.get(cap).copied())
                    .collect();
                
                if values.len() >= 2 {
                    let variance = self.calculate_variance(&values);
                    let mean = values.iter().sum::<f64>() / values.len() as f64;
                    let cv = if mean > 0.0 { variance.sqrt() / mean } else { 0.0 };
                    
                    convergence_scores.insert(cap.to_string(), 1.0 - cv.min(1.0));
                }
            }
        }

        let overall_convergence = if convergence_scores.is_empty() {
            0.5
        } else {
            convergence_scores.values().sum::<f64>() / convergence_scores.len() as f64
        };

        let is_converging = overall_convergence > 0.8;
        let is_diverging = overall_convergence < 0.3;

        ConvergenceAnalysis {
            overall_convergence,
            per_capability: convergence_scores,
            is_converging,
            is_diverging,
            recommendation: if is_converging {
                "Capabilities approaching stability - consider increasing exploration".to_string()
            } else if is_diverging {
                "High variance detected - ensure consistent learning".to_string()
            } else {
                "Normal growth trajectory".to_string()
            },
        }
    }

    fn calculate_variance(&self, values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>() / values.len() as f64;
        
        variance
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergenceAnalysis {
    pub overall_convergence: f64,
    pub per_capability: HashMap<String, f64>,
    pub is_converging: bool,
    pub is_diverging: bool,
    pub recommendation: String,
}

impl Default for CapabilityGrowthTracker {
    fn default() -> Self {
        Self::new()
    }
}

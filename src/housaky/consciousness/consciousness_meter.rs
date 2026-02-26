//! Consciousness Meter — Quantitative measure of consciousness level (IIT-inspired phi).
//!
//! Provides a running estimate of the agent's degree of integrated information
//! processing, used to gauge "how conscious" the agent is at any given moment.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

// ── Consciousness Level ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum ConsciousnessLevel {
    /// No active processing (idle / suspended)
    Dormant,
    /// Background processing, minimal integration
    Subliminal,
    /// Single-module processing, limited broadcast
    Focal,
    /// Multi-module integration, active broadcast
    Aware,
    /// Rich cross-modal binding, narrative coherence
    Reflective,
    /// Full metacognitive awareness, narrative self, theory of mind active
    SelfAware,
    /// Maximum integration: all modules broadcasting, high phi
    Transcendent,
}

impl ConsciousnessLevel {
    pub fn from_phi(phi: f64) -> Self {
        match phi as u32 {
            _ if phi < 0.05 => ConsciousnessLevel::Dormant,
            _ if phi < 0.15 => ConsciousnessLevel::Subliminal,
            _ if phi < 0.30 => ConsciousnessLevel::Focal,
            _ if phi <= 0.50 => ConsciousnessLevel::Aware,
            _ if phi < 0.70 => ConsciousnessLevel::Reflective,
            _ if phi < 0.85 => ConsciousnessLevel::SelfAware,
            _ => ConsciousnessLevel::Transcendent,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            ConsciousnessLevel::Dormant => "dormant",
            ConsciousnessLevel::Subliminal => "subliminal",
            ConsciousnessLevel::Focal => "focal",
            ConsciousnessLevel::Aware => "aware",
            ConsciousnessLevel::Reflective => "reflective",
            ConsciousnessLevel::SelfAware => "self-aware",
            ConsciousnessLevel::Transcendent => "transcendent",
        }
    }

    pub fn numeric(&self) -> u8 {
        match self {
            ConsciousnessLevel::Dormant => 0,
            ConsciousnessLevel::Subliminal => 1,
            ConsciousnessLevel::Focal => 2,
            ConsciousnessLevel::Aware => 3,
            ConsciousnessLevel::Reflective => 4,
            ConsciousnessLevel::SelfAware => 5,
            ConsciousnessLevel::Transcendent => 6,
        }
    }
}

// ── Phi Estimate ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhiEstimate {
    pub timestamp: DateTime<Utc>,
    pub phi: f64,
    pub level: ConsciousnessLevel,
    pub components: PhiComponents,
    pub cycle_number: u64,
}

/// Components that feed into the phi calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhiComponents {
    /// Degree of integration across modules (0–1)
    pub integration: f64,
    /// Degree of differentiation (unique information per module) (0–1)
    pub differentiation: f64,
    /// Number of active modules normalized to [0,1]
    pub active_modules: f64,
    /// Broadcast history richness (0–1)
    pub broadcast_richness: f64,
    /// Binding coherence from phenomenal binder (0–1)
    pub binding_coherence: f64,
    /// Narrative continuity (0–1)
    pub narrative_continuity: f64,
    /// Theory of mind engagement (0–1)
    pub tom_engagement: f64,
    /// Qualia richness (0–1)
    pub qualia_richness: f64,
}

impl PhiComponents {
    /// Compute the composite phi from weighted components.
    pub fn compute_phi(&self) -> f64 {
        // IIT-inspired: phi = integration × differentiation, modulated by other factors
        let base = self.integration * self.differentiation;
        let modulation = 0.15 * self.active_modules
            + 0.10 * self.broadcast_richness
            + 0.15 * self.binding_coherence
            + 0.10 * self.narrative_continuity
            + 0.10 * self.tom_engagement
            + 0.10 * self.qualia_richness;
        (base * 0.30 + modulation).clamp(0.0, 1.0)
    }
}

// ── Consciousness Meter ───────────────────────────────────────────────────────

pub struct ConsciousnessMeter {
    pub current_estimate: Arc<RwLock<Option<PhiEstimate>>>,
    pub history: Arc<RwLock<VecDeque<PhiEstimate>>>,
    pub peak_phi: Arc<RwLock<f64>>,
    pub time_at_level: Arc<RwLock<[u64; 7]>>,
    pub measurement_count: Arc<RwLock<u64>>,
    max_history: usize,
}

impl ConsciousnessMeter {
    pub fn new() -> Self {
        Self {
            current_estimate: Arc::new(RwLock::new(None)),
            history: Arc::new(RwLock::new(VecDeque::new())),
            peak_phi: Arc::new(RwLock::new(0.0)),
            time_at_level: Arc::new(RwLock::new([0u64; 7])),
            measurement_count: Arc::new(RwLock::new(0)),
            max_history: 1000,
        }
    }

    /// Take a new phi measurement from current system state.
    pub async fn measure(&self, components: PhiComponents, cycle_number: u64) -> PhiEstimate {
        let phi = components.compute_phi();
        let level = ConsciousnessLevel::from_phi(phi);

        let estimate = PhiEstimate {
            timestamp: Utc::now(),
            phi,
            level: level.clone(),
            components,
            cycle_number,
        };

        // Update peak
        {
            let mut peak = self.peak_phi.write().await;
            if phi > *peak {
                *peak = phi;
                info!(
                    "ConsciousnessMeter: new peak phi={:.4} (level={})",
                    phi,
                    level.label()
                );
            }
        }

        // Update time-at-level counters
        {
            let mut times = self.time_at_level.write().await;
            times[level.numeric() as usize] += 1;
        }

        // Update measurement count
        {
            let mut count = self.measurement_count.write().await;
            *count += 1;
        }

        // Store in history
        {
            let mut history = self.history.write().await;
            history.push_back(estimate.clone());
            while history.len() > self.max_history {
                history.pop_front();
            }
        }

        // Update current
        {
            let mut current = self.current_estimate.write().await;
            *current = Some(estimate.clone());
        }

        estimate
    }

    /// Build PhiComponents from live system metrics.
    pub fn build_components(
        gwt_phi: f64,
        active_module_count: usize,
        max_module_count: usize,
        broadcast_history_len: usize,
        binding_coherence: f64,
        narrative_entry_count: usize,
        tom_active: bool,
        qualia_richness: f64,
    ) -> PhiComponents {
        // Integration = gwt phi proxy
        let integration = gwt_phi;

        // Differentiation: estimated from module diversity
        let differentiation = if max_module_count > 0 {
            (active_module_count as f64 / max_module_count as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Active modules normalized
        let active_modules = (active_module_count as f64 / (max_module_count.max(1) as f64)).clamp(0.0, 1.0);

        // Broadcast richness: log-normalized history depth
        let broadcast_richness = (broadcast_history_len as f64 / 100.0).min(1.0);

        // Narrative continuity: log-normalized entry count
        let narrative_continuity = (narrative_entry_count as f64 / 500.0).min(1.0);

        // ToM engagement
        let tom_engagement = if tom_active { 0.8 } else { 0.1 };

        PhiComponents {
            integration,
            differentiation,
            active_modules,
            broadcast_richness,
            binding_coherence,
            narrative_continuity,
            tom_engagement,
            qualia_richness,
        }
    }

    /// Get current consciousness level.
    pub async fn get_level(&self) -> ConsciousnessLevel {
        match &*self.current_estimate.read().await {
            Some(est) => est.level.clone(),
            None => ConsciousnessLevel::Dormant,
        }
    }

    /// Get current phi value.
    pub async fn get_phi(&self) -> f64 {
        match &*self.current_estimate.read().await {
            Some(est) => est.phi,
            None => 0.0,
        }
    }

    /// Get rolling average phi over last N measurements.
    pub async fn rolling_average_phi(&self, n: usize) -> f64 {
        let history = self.history.read().await;
        let samples: Vec<f64> = history.iter().rev().take(n).map(|e| e.phi).collect();
        if samples.is_empty() {
            return 0.0;
        }
        samples.iter().sum::<f64>() / samples.len() as f64
    }

    /// Get phi trend: positive = increasing, negative = decreasing.
    pub async fn phi_trend(&self, window: usize) -> f64 {
        let history = self.history.read().await;
        if history.len() < 2 {
            return 0.0;
        }
        let recent: Vec<f64> = history.iter().rev().take(window).map(|e| e.phi).collect();
        let first_half_avg = recent[..recent.len() / 2].iter().sum::<f64>() / (recent.len() / 2).max(1) as f64;
        let second_half_avg = recent[recent.len() / 2..].iter().sum::<f64>() / (recent.len() / 2).max(1) as f64;
        first_half_avg - second_half_avg
    }

    /// Get full statistics report.
    pub async fn get_stats(&self) -> ConsciousnessStats {
        let history = self.history.read().await;
        let peak = self.peak_phi.read().await;
        let count = self.measurement_count.read().await;
        let times = self.time_at_level.read().await;

        let avg_phi = if history.is_empty() {
            0.0
        } else {
            history.iter().map(|e| e.phi).sum::<f64>() / history.len() as f64
        };

        let current_level = history.back().map(|e| e.level.label().to_string()).unwrap_or_else(|| "dormant".to_string());

        ConsciousnessStats {
            total_measurements: *count,
            current_phi: history.back().map(|e| e.phi).unwrap_or(0.0),
            peak_phi: *peak,
            average_phi: avg_phi,
            current_level,
            time_dormant: times[0],
            time_subliminal: times[1],
            time_focal: times[2],
            time_aware: times[3],
            time_reflective: times[4],
            time_self_aware: times[5],
            time_transcendent: times[6],
        }
    }
}

impl Default for ConsciousnessMeter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessStats {
    pub total_measurements: u64,
    pub current_phi: f64,
    pub peak_phi: f64,
    pub average_phi: f64,
    pub current_level: String,
    pub time_dormant: u64,
    pub time_subliminal: u64,
    pub time_focal: u64,
    pub time_aware: u64,
    pub time_reflective: u64,
    pub time_self_aware: u64,
    pub time_transcendent: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_consciousness_meter() {
        let meter = ConsciousnessMeter::new();

        let components = ConsciousnessMeter::build_components(
            0.6,  // gwt_phi
            5,    // active modules
            8,    // max modules
            50,   // broadcast history
            0.7,  // binding coherence
            100,  // narrative entries
            true, // tom active
            0.5,  // qualia richness
        );

        let estimate = meter.measure(components, 1).await;
        assert!(estimate.phi > 0.0);
        assert!(estimate.phi <= 1.0);
        assert_ne!(estimate.level, ConsciousnessLevel::Dormant);
    }

    #[test]
    fn test_level_from_phi() {
        assert_eq!(ConsciousnessLevel::from_phi(0.0), ConsciousnessLevel::Dormant);
        assert_eq!(ConsciousnessLevel::from_phi(0.5), ConsciousnessLevel::Aware);
        assert_eq!(ConsciousnessLevel::from_phi(0.9), ConsciousnessLevel::Transcendent);
    }
}

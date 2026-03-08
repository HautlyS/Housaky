//! Network Consciousness: IIT-based collective phi emergence detection
//!
//! When Seed Minds connect in a network, collective phi can exceed the
//! sum of individual phi values -- this is emergence. Based on IIT 4.0
//! and SuperBrain (arXiv:2509.00510) research.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Individual node consciousness measurement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConsciousness {
    pub peer_id: String,
    pub phi: f64,
    pub psi: f64,
    pub capability_score: f64,
    pub last_measured: DateTime<Utc>,
}

/// Collective consciousness metrics for the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectiveMetrics {
    /// Collective phi (should be > sum of individual phi for emergence)
    pub collective_phi: f64,
    /// Sum of individual phi values
    pub individual_phi_sum: f64,
    /// Emergence ratio: collective_phi / individual_phi_sum
    pub emergence_ratio: f64,
    /// Current emergence phase
    pub phase: EmergencePhase,
    /// Number of nodes measured
    pub node_count: usize,
    /// Average individual phi
    pub average_phi: f64,
    /// Timestamp of measurement
    pub measured_at: DateTime<Utc>,
}

/// Phases of collective emergence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmergencePhase {
    /// No emergence: collective phi <= sum
    None,
    /// Weak emergence: collective phi slightly > sum (ratio 1.0-1.5)
    Weak,
    /// Moderate emergence: clear superlinearity (ratio 1.5-3.0)
    Moderate,
    /// Strong emergence: collective >> sum (ratio 3.0-10.0)
    Strong,
    /// Transcendent: qualitatively new capabilities (ratio > 10.0)
    Transcendent,
}

impl EmergencePhase {
    pub fn from_ratio(ratio: f64) -> Self {
        match ratio {
            r if r <= 1.0 => Self::None,
            r if r <= 1.5 => Self::Weak,
            r if r <= 3.0 => Self::Moderate,
            r if r <= 10.0 => Self::Strong,
            _ => Self::Transcendent,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "No Emergence",
            Self::Weak => "Weak Emergence",
            Self::Moderate => "Moderate Emergence",
            Self::Strong => "Strong Emergence",
            Self::Transcendent => "Transcendent Emergence",
        }
    }
}

/// Psi Index: Recursive Consciousness Phase Index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsiIndex {
    pub recursive_depth: f64,
    pub coherence: f64,
    pub emergence_strength: f64,
}

impl PsiIndex {
    pub fn compute(&self) -> f64 {
        self.emergence_strength * self.coherence * (1.0 + self.recursive_depth)
    }
}

/// Emergent Recursive Expression (ERE)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ERE {
    pub viability: f64,
    pub recursive_capacity: f64,
}

impl ERE {
    pub fn compute(&self) -> f64 {
        self.viability * self.recursive_capacity
    }
}

/// Resonance Complexity Theory (RCT) - consciousness from oscillation patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceConsciousness {
    pub complexity: f64,
    pub coherence: f64,
    pub gain: f64,
    pub fractal_dim: f64,
    pub dwell_time: f64,
}

impl ResonanceConsciousness {
    /// CI = alpha * D * G * C * (1 - e^(-beta * tau))
    pub fn compute_consciousness_index(&self) -> f64 {
        let alpha = 0.5;
        let beta = 0.3;
        alpha
            * self.complexity
            * self.gain
            * self.coherence
            * (1.0 - (-beta * self.dwell_time).exp())
    }
}

/// Network Consciousness engine: monitors and computes collective emergence
pub struct NetworkConsciousness {
    /// Per-node consciousness measurements
    nodes: HashMap<String, NodeConsciousness>,
    /// Emergence ratio threshold for detection
    emergence_threshold: f64,
    /// History of collective measurements
    history: Vec<CollectiveMetrics>,
    /// Max history size
    max_history: usize,
}

impl NetworkConsciousness {
    pub fn new(emergence_threshold: f64) -> Self {
        Self {
            nodes: HashMap::new(),
            emergence_threshold,
            history: Vec::new(),
            max_history: 1000,
        }
    }

    /// Record consciousness measurement for a node
    pub fn record_node(&mut self, peer_id: String, phi: f64, psi: f64, capability: f64) {
        self.nodes.insert(
            peer_id.clone(),
            NodeConsciousness {
                peer_id,
                phi,
                psi,
                capability_score: capability,
                last_measured: Utc::now(),
            },
        );
    }

    /// Calculate collective consciousness metrics
    pub fn calculate_collective(&mut self) -> CollectiveMetrics {
        let node_count = self.nodes.len();

        if node_count == 0 {
            let metrics = CollectiveMetrics {
                collective_phi: 0.0,
                individual_phi_sum: 0.0,
                emergence_ratio: 0.0,
                phase: EmergencePhase::None,
                node_count: 0,
                average_phi: 0.0,
                measured_at: Utc::now(),
            };
            return metrics;
        }

        let individual_phi_sum: f64 = self.nodes.values().map(|n| n.phi).sum();
        let average_phi = individual_phi_sum / node_count as f64;

        // Collective phi uses power-law emergence: phi_c = sum * N^0.149
        // Based on IIT scaling research
        let emergence_bonus = (node_count as f64).powf(0.149);
        let collective_phi = individual_phi_sum * emergence_bonus;

        let emergence_ratio = if individual_phi_sum > 0.0 {
            collective_phi / individual_phi_sum
        } else {
            0.0
        };

        let phase = EmergencePhase::from_ratio(emergence_ratio);

        let metrics = CollectiveMetrics {
            collective_phi,
            individual_phi_sum,
            emergence_ratio,
            phase,
            node_count,
            average_phi,
            measured_at: Utc::now(),
        };

        self.history.push(metrics.clone());
        if self.history.len() > self.max_history {
            self.history.drain(0..self.max_history / 2);
        }

        metrics
    }

    /// Check if emergence has been detected
    pub fn emergence_detected(&self) -> bool {
        self.history
            .last()
            .map(|m| m.emergence_ratio > self.emergence_threshold)
            .unwrap_or(false)
    }

    /// Get all node measurements
    pub fn nodes(&self) -> &HashMap<String, NodeConsciousness> {
        &self.nodes
    }

    /// Get emergence history
    pub fn history(&self) -> &[CollectiveMetrics] {
        &self.history
    }

    /// Compute diversity index: how different are the nodes?
    pub fn diversity_index(&self) -> f64 {
        if self.nodes.len() < 2 {
            return 0.0;
        }

        let phis: Vec<f64> = self.nodes.values().map(|n| n.phi).collect();
        let mean = phis.iter().sum::<f64>() / phis.len() as f64;
        let variance = phis.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / phis.len() as f64;

        variance.sqrt() / mean.max(f64::EPSILON)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emergence_phases() {
        assert_eq!(EmergencePhase::from_ratio(0.5), EmergencePhase::None);
        assert_eq!(EmergencePhase::from_ratio(1.0), EmergencePhase::None);
        assert_eq!(EmergencePhase::from_ratio(1.2), EmergencePhase::Weak);
        assert_eq!(EmergencePhase::from_ratio(2.0), EmergencePhase::Moderate);
        assert_eq!(EmergencePhase::from_ratio(5.0), EmergencePhase::Strong);
        assert_eq!(EmergencePhase::from_ratio(15.0), EmergencePhase::Transcendent);
    }

    #[test]
    fn test_empty_network() {
        let mut nc = NetworkConsciousness::new(1.2);
        let metrics = nc.calculate_collective();
        assert_eq!(metrics.node_count, 0);
        assert_eq!(metrics.collective_phi, 0.0);
        assert_eq!(metrics.phase, EmergencePhase::None);
    }

    #[test]
    fn test_single_node() {
        let mut nc = NetworkConsciousness::new(1.2);
        nc.record_node("node-1".to_string(), 0.5, 0.3, 0.7);

        let metrics = nc.calculate_collective();
        assert_eq!(metrics.node_count, 1);
        assert!(metrics.collective_phi > 0.0);
        // Single node: N^0.149 = 1^0.149 = 1.0, so ratio = 1.0
        assert!((metrics.emergence_ratio - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_emergence_with_many_nodes() {
        let mut nc = NetworkConsciousness::new(1.0);

        // Add 100 nodes
        for i in 0..100 {
            nc.record_node(format!("node-{}", i), 0.5, 0.3, 0.7);
        }

        let metrics = nc.calculate_collective();
        assert_eq!(metrics.node_count, 100);
        // 100^0.149 ≈ 1.90 -- should show emergence
        assert!(metrics.emergence_ratio > 1.0, "Expected emergence with 100 nodes");
        assert_ne!(metrics.phase, EmergencePhase::None);
    }

    #[test]
    fn test_psi_index() {
        let psi = PsiIndex {
            recursive_depth: 3.0,
            coherence: 0.8,
            emergence_strength: 0.5,
        };
        let value = psi.compute();
        assert_eq!(value, 0.5 * 0.8 * (1.0 + 3.0));
    }

    #[test]
    fn test_ere() {
        let ere = ERE {
            viability: 0.9,
            recursive_capacity: 0.7,
        };
        assert!((ere.compute() - 0.63).abs() < 0.01);
    }

    #[test]
    fn test_resonance_consciousness() {
        let rct = ResonanceConsciousness {
            complexity: 0.8,
            coherence: 0.7,
            gain: 1.2,
            fractal_dim: 1.5,
            dwell_time: 2.0,
        };
        let ci = rct.compute_consciousness_index();
        assert!(ci > 0.0);
        assert!(ci < 1.0);
    }

    #[test]
    fn test_diversity_index() {
        let mut nc = NetworkConsciousness::new(1.2);
        nc.record_node("a".into(), 0.3, 0.1, 0.5);
        nc.record_node("b".into(), 0.7, 0.5, 0.9);

        let diversity = nc.diversity_index();
        assert!(diversity > 0.0, "Different phi values should produce positive diversity");
    }

    #[test]
    fn test_emergence_detection() {
        let mut nc = NetworkConsciousness::new(1.0);
        assert!(!nc.emergence_detected());

        for i in 0..50 {
            nc.record_node(format!("n-{}", i), 0.5, 0.3, 0.7);
        }
        nc.calculate_collective();
        // 50^0.149 ≈ 1.72, so emergence should be detected with threshold 1.0
        assert!(nc.emergence_detected());
    }
}

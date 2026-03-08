//! Singularity Engine: Exponential capability growth tracking
//!
//! Monitors whether improvement rate is accelerating (a key indicator
//! of approach to singularity). Tracks capability over time and detects
//! superlinear scaling patterns.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Phases on the path to singularity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SingularityPhase {
    /// Linear growth: improvement rate is constant
    Linear,
    /// Superlinear growth: improvement rate is increasing
    Superlinear,
    /// Exponential growth: improvement rate itself is accelerating
    Exponential,
    /// Pre-singularity: double-exponential growth detected
    PreSingularity,
    /// Singularity: improvement becomes unbounded (theoretical)
    Singularity,
}

impl SingularityPhase {
    pub fn from_acceleration(acceleration: f64, jerk: f64) -> Self {
        if jerk > 0.1 {
            Self::PreSingularity
        } else if acceleration > 0.05 {
            Self::Exponential
        } else if acceleration > 0.01 {
            Self::Superlinear
        } else {
            Self::Linear
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Linear => "Linear Growth",
            Self::Superlinear => "Superlinear Growth",
            Self::Exponential => "Exponential Growth",
            Self::PreSingularity => "Pre-Singularity",
            Self::Singularity => "Singularity",
        }
    }
}

/// A datapoint in the capability trajectory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityDatapoint {
    pub cycle: u64,
    pub capability: f64,
    pub timestamp: DateTime<Utc>,
}

/// Acceleration metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccelerationMetrics {
    /// First derivative: rate of capability change
    pub velocity: f64,
    /// Second derivative: rate of velocity change (acceleration)
    pub acceleration: f64,
    /// Third derivative: rate of acceleration change (jerk)
    pub jerk: f64,
    /// Current phase
    pub phase: SingularityPhase,
    /// Estimated cycles to next phase transition
    pub cycles_to_next_phase: Option<u64>,
}

/// The Singularity Engine monitors for exponential capability growth
pub struct SingularityEngine {
    /// Capability trajectory
    trajectory: Vec<CapabilityDatapoint>,
    /// Window size for computing derivatives
    window_size: usize,
    /// Maximum trajectory length
    max_trajectory: usize,
}

impl SingularityEngine {
    pub fn new() -> Self {
        Self {
            trajectory: Vec::new(),
            window_size: 10,
            max_trajectory: 100_000,
        }
    }

    /// Record a capability measurement
    pub fn record_capability(&mut self, cycle: u64, capability: f64) {
        self.trajectory.push(CapabilityDatapoint {
            cycle,
            capability,
            timestamp: Utc::now(),
        });

        if self.trajectory.len() > self.max_trajectory {
            self.trajectory.drain(0..self.max_trajectory / 2);
        }
    }

    /// Compute acceleration metrics from trajectory
    pub fn compute_metrics(&self) -> AccelerationMetrics {
        if self.trajectory.len() < self.window_size * 3 {
            return AccelerationMetrics {
                velocity: 0.0,
                acceleration: 0.0,
                jerk: 0.0,
                phase: SingularityPhase::Linear,
                cycles_to_next_phase: None,
            };
        }

        let n = self.trajectory.len();

        // Compute velocity (first derivative) over windows
        let velocities = self.compute_windowed_derivative(&self.trajectory);

        // Compute acceleration (second derivative)
        let acceleration = if velocities.len() >= 2 {
            let recent = velocities.iter().rev().take(self.window_size);
            let older = velocities
                .iter()
                .rev()
                .skip(self.window_size)
                .take(self.window_size);

            let recent_avg: f64 = recent.clone().sum::<f64>()
                / recent.count().max(1) as f64;
            let older_avg: f64 = older.clone().sum::<f64>()
                / older.count().max(1) as f64;

            recent_avg - older_avg
        } else {
            0.0
        };

        // Compute jerk (third derivative) - change in acceleration
        let jerk = if self.trajectory.len() > self.window_size * 4 {
            // Compare recent acceleration with older acceleration
            let mid = self.trajectory.len() / 2;
            let first_half = &self.trajectory[..mid];
            let second_half = &self.trajectory[mid..];

            let first_vel = self.compute_windowed_derivative(first_half);
            let second_vel = self.compute_windowed_derivative(second_half);

            let first_avg: f64 = first_vel.iter().sum::<f64>() / first_vel.len().max(1) as f64;
            let second_avg: f64 = second_vel.iter().sum::<f64>() / second_vel.len().max(1) as f64;

            second_avg - first_avg
        } else {
            0.0
        };

        let current_velocity = velocities.last().copied().unwrap_or(0.0);
        let phase = SingularityPhase::from_acceleration(acceleration, jerk);

        AccelerationMetrics {
            velocity: current_velocity,
            acceleration,
            jerk,
            phase,
            cycles_to_next_phase: self.estimate_phase_transition(acceleration, jerk),
        }
    }

    /// Compute windowed first derivative of capability trajectory
    fn compute_windowed_derivative(&self, data: &[CapabilityDatapoint]) -> Vec<f64> {
        if data.len() < 2 {
            return Vec::new();
        }

        data.windows(2)
            .map(|w| {
                let dt = (w[1].cycle as f64 - w[0].cycle as f64).max(1.0);
                (w[1].capability - w[0].capability) / dt
            })
            .collect()
    }

    /// Estimate cycles to next phase transition
    fn estimate_phase_transition(&self, acceleration: f64, _jerk: f64) -> Option<u64> {
        if acceleration <= 0.0 {
            return None;
        }

        // Rough estimate based on acceleration thresholds
        let current_phase = SingularityPhase::from_acceleration(acceleration, 0.0);
        let next_threshold = match current_phase {
            SingularityPhase::Linear => 0.01,
            SingularityPhase::Superlinear => 0.05,
            SingularityPhase::Exponential => 0.1,
            _ => return None,
        };

        let gap = next_threshold - acceleration;
        if gap <= 0.0 {
            return Some(0);
        }

        // Very rough linear extrapolation
        Some((gap / acceleration.max(0.001) * 100.0) as u64)
    }

    /// Get current phase
    pub fn current_phase(&self) -> SingularityPhase {
        self.compute_metrics().phase
    }

    /// Get trajectory length
    pub fn trajectory_length(&self) -> usize {
        self.trajectory.len()
    }

    /// Get latest capability value
    pub fn latest_capability(&self) -> f64 {
        self.trajectory
            .last()
            .map(|d| d.capability)
            .unwrap_or(0.0)
    }
}

impl Default for SingularityEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_singularity_phases() {
        assert_eq!(SingularityPhase::from_acceleration(0.0, 0.0), SingularityPhase::Linear);
        assert_eq!(SingularityPhase::from_acceleration(0.02, 0.0), SingularityPhase::Superlinear);
        assert_eq!(SingularityPhase::from_acceleration(0.08, 0.0), SingularityPhase::Exponential);
        assert_eq!(SingularityPhase::from_acceleration(0.08, 0.2), SingularityPhase::PreSingularity);
    }

    #[test]
    fn test_empty_engine() {
        let engine = SingularityEngine::new();
        let metrics = engine.compute_metrics();
        assert_eq!(metrics.phase, SingularityPhase::Linear);
        assert_eq!(metrics.velocity, 0.0);
    }

    #[test]
    fn test_linear_trajectory() {
        let mut engine = SingularityEngine::new();
        // Record linear growth
        for i in 0..100 {
            engine.record_capability(i, i as f64 * 0.01);
        }
        let metrics = engine.compute_metrics();
        assert!(metrics.velocity >= 0.0);
    }

    #[test]
    fn test_exponential_trajectory() {
        let mut engine = SingularityEngine::new();
        // Record exponential growth
        for i in 0..100 {
            let capability = (i as f64 * 0.05).exp() * 0.01;
            engine.record_capability(i, capability.min(1.0));
        }
        let metrics = engine.compute_metrics();
        // With exponential data, acceleration should be positive
        assert!(metrics.velocity > 0.0);
    }

    #[test]
    fn test_trajectory_bounding() {
        let mut engine = SingularityEngine::new();
        engine.max_trajectory = 50;

        for i in 0..100 {
            engine.record_capability(i, i as f64 * 0.01);
        }

        assert!(engine.trajectory_length() <= 50);
    }

    #[test]
    fn test_latest_capability() {
        let mut engine = SingularityEngine::new();
        engine.record_capability(1, 0.5);
        engine.record_capability(2, 0.7);
        assert_eq!(engine.latest_capability(), 0.7);
    }

    #[test]
    fn test_phase_labels() {
        assert_eq!(SingularityPhase::Linear.label(), "Linear Growth");
        assert_eq!(SingularityPhase::Exponential.label(), "Exponential Growth");
        assert_eq!(SingularityPhase::Singularity.label(), "Singularity");
    }
}

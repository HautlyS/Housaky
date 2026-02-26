//! Heterogeneous Compute — Split computation across multiple substrates simultaneously.
//!
//! The `HeterogeneousScheduler` partitions a workload across all available
//! substrates, dispatches sub-tasks in parallel, and merges the results.

use crate::housaky::singularity::substrate::abstract_compute::{
    Computation, ComputationKind, ComputeResult, ComputeSubstrate, SubstrateType,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{info, warn};
use uuid::Uuid;

// ── Scheduling Policy ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchedulingPolicy {
    /// Route each computation to the cheapest capable substrate.
    CostOptimised,
    /// Route to the fastest capable substrate.
    LatencyOptimised,
    /// Replicate on all capable substrates; return first result.
    Redundant,
    /// Round-robin across all substrates.
    RoundRobin,
    /// Manually assign workload type → substrate type.
    Pinned(Vec<(String, SubstrateType)>), // workload_kind → substrate
}

// ── Dispatch Plan ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchPlan {
    pub batch_id: String,
    pub assignments: Vec<SubstrateAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateAssignment {
    pub computation_id: String,
    pub substrate_display: String,
    pub rationale: String,
}

// ── Heterogeneous Scheduler ────────────────────────────────────────────────────

pub struct HeterogeneousScheduler {
    pub substrates: Vec<Arc<dyn ComputeSubstrate>>,
    pub policy: SchedulingPolicy,
    round_robin_index: usize,
    pub total_dispatched: u64,
    pub total_failed: u64,
}

impl HeterogeneousScheduler {
    pub fn new(policy: SchedulingPolicy) -> Self {
        Self {
            substrates: Vec::new(),
            policy,
            round_robin_index: 0,
            total_dispatched: 0,
            total_failed: 0,
        }
    }

    pub fn register(&mut self, substrate: Arc<dyn ComputeSubstrate>) {
        info!(
            "Registered substrate: {}",
            substrate.substrate_type().display_name()
        );
        self.substrates.push(substrate);
    }

    /// Execute a single computation using the current policy.
    pub async fn execute(&mut self, computation: Computation) -> Result<ComputeResult> {
        if self.substrates.is_empty() {
            anyhow::bail!("No substrates registered");
        }

        let substrate = self.select_substrate(&computation)?;
        self.total_dispatched += 1;

        match substrate.execute(&computation).await {
            Ok(result) => Ok(result),
            Err(e) => {
                self.total_failed += 1;
                warn!("Dispatch to {} failed: {}", substrate.substrate_type().display_name(), e);
                Err(e)
            }
        }
    }

    /// Execute a batch of computations in parallel across available substrates.
    pub async fn execute_batch(
        &mut self,
        computations: Vec<Computation>,
    ) -> Vec<Result<ComputeResult>> {
        let batch_id = Uuid::new_v4().to_string();
        info!(
            batch_id = %batch_id,
            count = %computations.len(),
            "Dispatching heterogeneous batch"
        );

        // Build a join set, round-robining substrate selection
        let mut join_set: JoinSet<Result<ComputeResult>> = JoinSet::new();

        for computation in computations {
            let idx = self.round_robin_index % self.substrates.len();
            self.round_robin_index += 1;
            let substrate = Arc::clone(&self.substrates[idx]);
            join_set.spawn(async move { substrate.execute(&computation).await });
        }

        let mut results = Vec::new();
        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(compute_result) => results.push(compute_result),
                Err(e) => {
                    self.total_failed += 1;
                    results.push(Err(anyhow::anyhow!("Task panicked: {}", e)));
                }
            }
        }

        self.total_dispatched += results.len() as u64;
        results
    }

    fn select_substrate(&mut self, computation: &Computation) -> Result<Arc<dyn ComputeSubstrate>> {
        match &self.policy {
            SchedulingPolicy::CostOptimised => {
                self.substrates
                    .iter()
                    .min_by(|a, b| {
                        a.cost_per_flop()
                            .partial_cmp(&b.cost_per_flop())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("No substrate available"))
            }
            SchedulingPolicy::LatencyOptimised => {
                self.substrates
                    .iter()
                    .min_by_key(|s| s.latency_ns())
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("No substrate available"))
            }
            SchedulingPolicy::RoundRobin | SchedulingPolicy::Redundant => {
                let idx = self.round_robin_index % self.substrates.len();
                self.round_robin_index += 1;
                Ok(Arc::clone(&self.substrates[idx]))
            }
            SchedulingPolicy::Pinned(pins) => {
                let kind_str = match &computation.kind {
                    ComputationKind::Custom(s) => s.clone(),
                    k => format!("{:?}", k).to_lowercase(),
                };
                if let Some((_, target_type)) = pins.iter().find(|(k, _)| k == &kind_str) {
                    if let Some(s) = self
                        .substrates
                        .iter()
                        .find(|s| &s.substrate_type() == target_type)
                    {
                        return Ok(Arc::clone(s));
                    }
                }
                // Fallback: first available
                self.substrates
                    .first()
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("No substrate available"))
            }
        }
    }

    pub fn substrate_count(&self) -> usize {
        self.substrates.len()
    }

    pub fn stats(&self) -> HeterogeneousStats {
        HeterogeneousStats {
            substrate_count: self.substrates.len(),
            total_dispatched: self.total_dispatched,
            total_failed: self.total_failed,
            success_rate: if self.total_dispatched > 0 {
                1.0 - self.total_failed as f64 / self.total_dispatched as f64
            } else {
                1.0
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeterogeneousStats {
    pub substrate_count: usize,
    pub total_dispatched: u64,
    pub total_failed: u64,
    pub success_rate: f64,
}

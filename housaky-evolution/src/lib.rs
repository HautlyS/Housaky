//! Housaky Evolution - Self-improvement and code evolution system
//!
//! This crate provides the Darwin-Gödel Machine implementation for self-improving code
//! based on cutting-edge 2025-2026 research:
//! - Darwin Gödel Machine (Zhang et al., 2025) - Open-ended evolution
//! - ICLR 2026 Workshop on Recursive Self-Improvement
//! - Singularity detection and safe self-replication protocols

use anyhow::Result;

pub mod ast_mutator;
pub mod fitness;
pub mod mutation;
pub mod sandbox;
pub mod selection;
pub mod singularity;

pub use ast_mutator::*;
pub use fitness::*;
pub use mutation::*;
pub use sandbox::*;
pub use selection::*;
pub use singularity::{
    SingularityDetector, SingularityConfig, SingularitySignal, SingularityType,
    ImprovementMetrics, ReplicationPackage, ReplicationStatus, ReplicationStats,
    SelfReplicationManager, PersistenceEngine, AGISingularityOrchestrator,
    SingularityEvent, SingularityStatus,
};

pub fn init() {
    tracing::info!("housaky-evolution initialized with Singularity Detection and Self-Replication");
}

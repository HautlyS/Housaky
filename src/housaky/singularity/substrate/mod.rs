//! Substrate Independence — Phase 6.2
//!
//! Enables Housaky to migrate between and distribute work across CPUs, GPUs,
//! FPGAs, quantum processors, neuromorphic chips, WASM runtimes, and cloud
//! functions without changing the cognitive logic above.

pub mod abstract_compute;
pub mod heterogeneous;
pub mod migration;
pub mod substrate_discovery;

pub use abstract_compute::{
    CognitiveState, Computation, ComputationKind, ComputeResult, ComputeSubstrate,
    CpuSubstrate, GpuSubstrate, SubstrateCapabilities, SubstrateType, WasmSubstrate,
};
pub use heterogeneous::{
    DispatchPlan, HeterogeneousScheduler, HeterogeneousStats, SchedulingPolicy,
    SubstrateAssignment,
};
pub use migration::{MigrationRecord, MigrationStatus, SubstrateMigrator};
pub use substrate_discovery::{DiscoveredSubstrate, DiscoverySource, SubstrateDiscoverer};

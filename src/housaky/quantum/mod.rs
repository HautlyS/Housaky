//! §10 — Quantum-Hybrid Solver & Advantage Benchmarks
//!
//! Provides a `HybridSolver` that can route sub-problems to classical or
//! simulated-quantum backends, plus benchmarks that measure where a quantum
//! approach would outperform classical — establishing the quantum advantage
//! frontier for Housaky's problem portfolio.

pub mod hybrid_solver;
pub mod benchmarks;

pub use hybrid_solver::{HybridSolver, SolverBackend, SolverResult};
pub use benchmarks::{QuantumBenchmarkSuite, BenchmarkResult as QuantumBenchmarkResult};

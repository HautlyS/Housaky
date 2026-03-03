//! §10 — Quantum-Hybrid Solver & Advantage Benchmarks
//!
//! Provides a `HybridSolver` that can route sub-problems to classical or
//! simulated-quantum backends, plus benchmarks that measure where a quantum
//! approach would outperform classical — establishing the quantum advantage
//! frontier for Housaky's problem portfolio.

pub mod benchmarks;
pub mod hybrid_solver;

pub use benchmarks::{BenchmarkResult as QuantumBenchmarkResult, QuantumBenchmarkSuite};
pub use hybrid_solver::{HybridSolver, SolverBackend, SolverResult};

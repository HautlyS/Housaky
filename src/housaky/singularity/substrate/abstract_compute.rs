//! Abstract Compute — Trait abstracting computation across substrates.
//!
//! Every substrate (CPU, GPU, FPGA, quantum, neuromorphic, WASM, cloud)
//! implements `ComputeSubstrate` so the AGI can dispatch work transparently.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Substrate Type ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubstrateType {
    CPU,
    GPU { cuda: bool, rocm: bool },
    FPGA { family: String },
    QuantumProcessor { qubits: usize },
    NeuromorphicChip { neurons: usize },
    WASM { runtime: String },
    CloudFunction { provider: String },
    Custom(String),
}

impl SubstrateType {
    pub fn display_name(&self) -> String {
        match self {
            SubstrateType::CPU => "CPU".to_string(),
            SubstrateType::GPU { cuda, rocm } => {
                format!("GPU(cuda={}, rocm={})", cuda, rocm)
            }
            SubstrateType::FPGA { family } => format!("FPGA({})", family),
            SubstrateType::QuantumProcessor { qubits } => {
                format!("QuantumProcessor({}q)", qubits)
            }
            SubstrateType::NeuromorphicChip { neurons } => {
                format!("NeuromorphicChip({}n)", neurons)
            }
            SubstrateType::WASM { runtime } => format!("WASM({})", runtime),
            SubstrateType::CloudFunction { provider } => {
                format!("CloudFunction({})", provider)
            }
            SubstrateType::Custom(name) => format!("Custom({})", name),
        }
    }
}

// ── Substrate Capabilities ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateCapabilities {
    /// Peak floating-point operations per second.
    pub flops: f64,
    /// Available memory in bytes.
    pub memory_bytes: u64,
    /// Supports parallel workloads.
    pub parallel: bool,
    /// Supports streaming/incremental computation.
    pub streaming: bool,
    /// Supports stateful migration (cognitive state can be transferred in).
    pub supports_state_migration: bool,
    /// List of workload types this substrate handles well.
    pub preferred_workloads: Vec<String>,
}

impl Default for SubstrateCapabilities {
    fn default() -> Self {
        Self {
            flops: 1e9,
            memory_bytes: 4 * 1024 * 1024 * 1024, // 4 GiB
            parallel: false,
            streaming: false,
            supports_state_migration: true,
            preferred_workloads: vec!["general".to_string()],
        }
    }
}

// ── Computation Request ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Computation {
    pub id: String,
    pub kind: ComputationKind,
    pub payload: Vec<u8>,        // serialised input
    pub priority: u8,
    pub deadline_ms: Option<u64>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComputationKind {
    Inference,
    Search,
    Optimisation,
    MemoryConsolidation,
    Simulation,
    Compilation,
    Custom(String),
}

// ── Compute Result ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeResult {
    pub computation_id: String,
    pub substrate: SubstrateType,
    pub output: Vec<u8>,
    pub duration_ns: u64,
    pub energy_joules: Option<f64>,
    pub success: bool,
    pub error: Option<String>,
}

// ── Cognitive State (for migration) ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveState {
    pub snapshot_id: String,
    pub cycle: u64,
    pub working_memory: Vec<u8>,
    pub goal_queue: Vec<u8>,
    pub belief_state: Vec<u8>,
    pub metadata: HashMap<String, String>,
}

// ── ComputeSubstrate trait ─────────────────────────────────────────────────────

#[async_trait]
pub trait ComputeSubstrate: Send + Sync {
    fn substrate_type(&self) -> SubstrateType;
    fn capabilities(&self) -> SubstrateCapabilities;

    /// Execute a computation on this substrate.
    async fn execute(&self, computation: &Computation) -> Result<ComputeResult>;

    /// Receive a cognitive state migrated from another substrate.
    async fn migrate_state(&self, state: &CognitiveState) -> Result<()>;

    /// Approximate cost in USD per GFLOP.
    fn cost_per_flop(&self) -> f64;

    /// Approximate round-trip latency in nanoseconds.
    fn latency_ns(&self) -> u64;

    /// Whether this substrate is currently healthy and reachable.
    async fn health_check(&self) -> bool;
}

// ── Local CPU Substrate ────────────────────────────────────────────────────────

/// Built-in CPU substrate — always available.
pub struct CpuSubstrate {
    pub memory_bytes: u64,
}

impl Default for CpuSubstrate {
    fn default() -> Self {
        Self {
            memory_bytes: 4 * 1024 * 1024 * 1024,
        }
    }
}

#[async_trait]
impl ComputeSubstrate for CpuSubstrate {
    fn substrate_type(&self) -> SubstrateType {
        SubstrateType::CPU
    }

    fn capabilities(&self) -> SubstrateCapabilities {
        SubstrateCapabilities {
            flops: 4e9,
            memory_bytes: self.memory_bytes,
            parallel: true,
            streaming: true,
            supports_state_migration: true,
            preferred_workloads: vec![
                "inference".to_string(),
                "planning".to_string(),
                "compilation".to_string(),
            ],
        }
    }

    async fn execute(&self, computation: &Computation) -> Result<ComputeResult> {
        let start = std::time::Instant::now();
        // CPU-native: pass-through (actual compute happens in calling code)
        Ok(ComputeResult {
            computation_id: computation.id.clone(),
            substrate: SubstrateType::CPU,
            output: computation.payload.clone(),
            duration_ns: start.elapsed().as_nanos() as u64,
            energy_joules: None,
            success: true,
            error: None,
        })
    }

    async fn migrate_state(&self, _state: &CognitiveState) -> Result<()> {
        // CPU is the primary substrate; state already lives here.
        Ok(())
    }

    fn cost_per_flop(&self) -> f64 {
        1e-12 // baseline
    }

    fn latency_ns(&self) -> u64 {
        100
    }

    async fn health_check(&self) -> bool {
        true
    }
}

// ── GPU Substrate ──────────────────────────────────────────────────────────────

/// GPU compute substrate — detects CUDA or ROCm at runtime.
///
/// When `execute()` is called, it shells out to a user-specified GPU script
/// (`HOUSAKY_GPU_SCRIPT` env var) or falls back to recording the dispatch
/// request so an external GPU worker can pick it up from `gpu_queue_dir`.
pub struct GpuSubstrate {
    pub cuda: bool,
    pub rocm: bool,
    pub device_id: String,
    /// Directory where GPU job JSON files are written for out-of-process pickup.
    pub queue_dir: std::path::PathBuf,
}

impl GpuSubstrate {
    /// Probe the environment and return `Some(GpuSubstrate)` if a GPU is visible.
    pub fn probe() -> Option<Self> {
        // CUDA
        if let Ok(dev) = std::env::var("CUDA_VISIBLE_DEVICES") {
            if !dev.is_empty() && dev != "-1" {
                return Some(Self {
                    cuda: true,
                    rocm: false,
                    device_id: dev,
                    queue_dir: dirs_next(),
                });
            }
        }
        // ROCm
        if let Ok(dev) = std::env::var("ROCR_VISIBLE_DEVICES") {
            if !dev.is_empty() {
                return Some(Self {
                    cuda: false,
                    rocm: true,
                    device_id: dev,
                    queue_dir: dirs_next(),
                });
            }
        }
        // nvidia-smi availability (headless servers without CUDA_VISIBLE_DEVICES set)
        if std::process::Command::new("nvidia-smi")
            .arg("--query-gpu=name")
            .arg("--format=csv,noheader")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(Self {
                cuda: true,
                rocm: false,
                device_id: "0".to_string(),
                queue_dir: dirs_next(),
            });
        }
        None
    }
}

fn dirs_next() -> std::path::PathBuf {
    directories::BaseDirs::new()
        .map(|d| d.data_local_dir().join("housaky").join("gpu_queue"))
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp/housaky/gpu_queue"))
}

#[async_trait]
impl ComputeSubstrate for GpuSubstrate {
    fn substrate_type(&self) -> SubstrateType {
        SubstrateType::GPU {
            cuda: self.cuda,
            rocm: self.rocm,
        }
    }

    fn capabilities(&self) -> SubstrateCapabilities {
        SubstrateCapabilities {
            flops: 1e13,
            memory_bytes: 16 * 1024 * 1024 * 1024,
            parallel: true,
            streaming: true,
            supports_state_migration: false,
            preferred_workloads: vec![
                "inference".to_string(),
                "simulation".to_string(),
                "optimisation".to_string(),
            ],
        }
    }

    async fn execute(&self, computation: &Computation) -> Result<ComputeResult> {
        let start = std::time::Instant::now();

        // If a GPU script is configured, delegate to it synchronously
        if let Ok(script) = std::env::var("HOUSAKY_GPU_SCRIPT") {
            use base64::Engine as _;
            let payload_b64 = base64::engine::general_purpose::STANDARD
                .encode(&computation.payload);
            let out = tokio::process::Command::new(&script)
                .arg(&computation.id)
                .arg(format!("{:?}", computation.kind))
                .arg(&payload_b64)
                .output()
                .await?;
            let success = out.status.success();
            let output = if success {
                out.stdout
            } else {
                out.stderr
            };
            return Ok(ComputeResult {
                computation_id: computation.id.clone(),
                substrate: self.substrate_type(),
                output,
                duration_ns: start.elapsed().as_nanos() as u64,
                energy_joules: None,
                success,
                error: if success { None } else { Some("GPU script failed".to_string()) },
            });
        }

        // Fallback: write job to queue directory for an external GPU worker
        tokio::fs::create_dir_all(&self.queue_dir).await?;
        let job_path = self.queue_dir.join(format!("{}.json", computation.id));
        let job_json = serde_json::to_vec(computation)?;
        tokio::fs::write(&job_path, job_json).await?;

        Ok(ComputeResult {
            computation_id: computation.id.clone(),
            substrate: self.substrate_type(),
            output: computation.payload.clone(),
            duration_ns: start.elapsed().as_nanos() as u64,
            energy_joules: None,
            success: true,
            error: None,
        })
    }

    async fn migrate_state(&self, _state: &CognitiveState) -> Result<()> {
        Ok(())
    }

    fn cost_per_flop(&self) -> f64 {
        3e-13
    }

    fn latency_ns(&self) -> u64 {
        50_000
    }

    async fn health_check(&self) -> bool {
        if self.cuda {
            std::process::Command::new("nvidia-smi")
                .arg("-L")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        } else if self.rocm {
            std::process::Command::new("rocm-smi")
                .arg("--showid")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        } else {
            false
        }
    }
}

// ── WASM Substrate ─────────────────────────────────────────────────────────────

pub struct WasmSubstrate {
    pub runtime_name: String,
}

#[async_trait]
impl ComputeSubstrate for WasmSubstrate {
    fn substrate_type(&self) -> SubstrateType {
        SubstrateType::WASM {
            runtime: self.runtime_name.clone(),
        }
    }

    fn capabilities(&self) -> SubstrateCapabilities {
        SubstrateCapabilities {
            flops: 5e8,
            memory_bytes: 512 * 1024 * 1024,
            parallel: false,
            streaming: false,
            supports_state_migration: true,
            preferred_workloads: vec!["sandboxed_tools".to_string(), "plugins".to_string()],
        }
    }

    async fn execute(&self, computation: &Computation) -> Result<ComputeResult> {
        let start = std::time::Instant::now();
        Ok(ComputeResult {
            computation_id: computation.id.clone(),
            substrate: self.substrate_type(),
            output: computation.payload.clone(),
            duration_ns: start.elapsed().as_nanos() as u64,
            energy_joules: None,
            success: true,
            error: None,
        })
    }

    async fn migrate_state(&self, _state: &CognitiveState) -> Result<()> {
        Ok(())
    }

    fn cost_per_flop(&self) -> f64 {
        2e-12
    }

    fn latency_ns(&self) -> u64 {
        5_000
    }

    async fn health_check(&self) -> bool {
        true
    }
}

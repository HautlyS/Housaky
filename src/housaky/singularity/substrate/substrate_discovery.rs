//! Substrate Discovery — Detect available compute resources at runtime.
//!
//! Probes the local environment and known remote endpoints for available
//! compute substrates and returns a ranked list suitable for registration
//! with the `HeterogeneousScheduler`.

use crate::housaky::singularity::substrate::abstract_compute::{
    ComputeSubstrate, CpuSubstrate, SubstrateCapabilities, SubstrateType, WasmSubstrate,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

// ── Discovered Substrate Info ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredSubstrate {
    pub substrate_type: SubstrateType,
    pub capabilities: SubstrateCapabilities,
    pub available: bool,
    pub display_name: String,
    pub discovery_source: DiscoverySource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoverySource {
    Local,
    Environment,
    NetworkProbe { endpoint: String },
    ConfigFile,
}

// ── Substrate Discoverer ───────────────────────────────────────────────────────

pub struct SubstrateDiscoverer;

impl SubstrateDiscoverer {
    /// Probe the runtime environment and return all discovered substrates.
    pub async fn discover_all() -> Vec<DiscoveredSubstrate> {
        let mut discovered = Vec::new();

        // 1. CPU — always available
        discovered.push(DiscoveredSubstrate {
            substrate_type: SubstrateType::CPU,
            capabilities: CpuSubstrate::default().capabilities(),
            available: true,
            display_name: "CPU (local)".to_string(),
            discovery_source: DiscoverySource::Local,
        });
        info!("Substrate discovered: CPU (local)");

        // 2. WASM runtime — check if wasmi feature compiled in
        #[cfg(feature = "runtime-wasm")]
        {
            discovered.push(DiscoveredSubstrate {
                substrate_type: SubstrateType::WASM {
                    runtime: "wasmi".to_string(),
                },
                capabilities: WasmSubstrate {
                    runtime_name: "wasmi".to_string(),
                }
                .capabilities(),
                available: true,
                display_name: "WASM (wasmi sandbox)".to_string(),
                discovery_source: DiscoverySource::Local,
            });
            info!("Substrate discovered: WASM (wasmi)");
        }

        // 3. CUDA GPU — probe via environment variable
        if let Ok(device) = std::env::var("CUDA_VISIBLE_DEVICES") {
            if !device.is_empty() && device != "-1" {
                discovered.push(DiscoveredSubstrate {
                    substrate_type: SubstrateType::GPU {
                        cuda: true,
                        rocm: false,
                    },
                    capabilities: SubstrateCapabilities {
                        flops: 1e13, // ~10 TFLOPS for a typical data-centre GPU
                        memory_bytes: 16 * 1024 * 1024 * 1024,
                        parallel: true,
                        streaming: true,
                        supports_state_migration: false,
                        preferred_workloads: vec![
                            "inference".to_string(),
                            "simulation".to_string(),
                        ],
                    },
                    available: true,
                    display_name: format!("CUDA GPU (device {})", device),
                    discovery_source: DiscoverySource::Environment,
                });
                info!("Substrate discovered: CUDA GPU (device {})", device);
            }
        }

        // 4. ROCm GPU — probe via environment variable
        if let Ok(device) = std::env::var("ROCR_VISIBLE_DEVICES") {
            if !device.is_empty() {
                discovered.push(DiscoveredSubstrate {
                    substrate_type: SubstrateType::GPU {
                        cuda: false,
                        rocm: true,
                    },
                    capabilities: SubstrateCapabilities {
                        flops: 8e12,
                        memory_bytes: 16 * 1024 * 1024 * 1024,
                        parallel: true,
                        streaming: true,
                        supports_state_migration: false,
                        preferred_workloads: vec!["inference".to_string()],
                    },
                    available: true,
                    display_name: format!("ROCm GPU (device {})", device),
                    discovery_source: DiscoverySource::Environment,
                });
                info!("Substrate discovered: ROCm GPU");
            }
        }

        // 5. Amazon Braket quantum processor — probe via env
        if let Ok(arn) = std::env::var("BRAKET_DEVICE_ARN") {
            if !arn.is_empty() {
                discovered.push(DiscoveredSubstrate {
                    substrate_type: SubstrateType::QuantumProcessor { qubits: 32 },
                    capabilities: SubstrateCapabilities {
                        flops: 0.0, // not measured in FLOPs
                        memory_bytes: 0,
                        parallel: true,
                        streaming: false,
                        supports_state_migration: false,
                        preferred_workloads: vec![
                            "optimisation".to_string(),
                            "search".to_string(),
                        ],
                    },
                    available: true,
                    display_name: format!("Amazon Braket ({})", &arn[..arn.len().min(40)]),
                    discovery_source: DiscoverySource::Environment,
                });
                info!("Substrate discovered: Amazon Braket quantum processor");
            }
        }

        // 6. Cloud function endpoint
        if let Ok(endpoint) = std::env::var("HOUSAKY_CLOUD_SUBSTRATE_URL") {
            if !endpoint.is_empty() {
                let provider = if endpoint.contains("lambda") {
                    "AWS Lambda"
                } else if endpoint.contains("cloudfunctions") {
                    "GCP Cloud Functions"
                } else if endpoint.contains("azure") {
                    "Azure Functions"
                } else {
                    "Custom"
                };
                discovered.push(DiscoveredSubstrate {
                    substrate_type: SubstrateType::CloudFunction {
                        provider: provider.to_string(),
                    },
                    capabilities: SubstrateCapabilities {
                        flops: 2e9,
                        memory_bytes: 3 * 1024 * 1024 * 1024,
                        parallel: true,
                        streaming: false,
                        supports_state_migration: false,
                        preferred_workloads: vec![
                            "inference".to_string(),
                            "search".to_string(),
                        ],
                    },
                    available: true,
                    display_name: format!("{} ({})", provider, &endpoint[..endpoint.len().min(40)]),
                    discovery_source: DiscoverySource::NetworkProbe {
                        endpoint: endpoint.clone(),
                    },
                });
                info!("Substrate discovered: cloud function at {}", endpoint);
            }
        }

        info!(
            "Substrate discovery complete — {} substrate(s) found",
            discovered.len()
        );
        discovered
    }

    /// Instantiate concrete `Arc<dyn ComputeSubstrate>` objects for the
    /// locally-constructable substrates (CPU, WASM). Remote substrates
    /// (GPU, quantum, cloud) require feature-specific adapters.
    pub fn instantiate_local(
        discovered: &[DiscoveredSubstrate],
    ) -> Vec<Arc<dyn ComputeSubstrate>> {
        let mut result: Vec<Arc<dyn ComputeSubstrate>> = Vec::new();

        for d in discovered {
            if !d.available {
                continue;
            }
            match &d.substrate_type {
                SubstrateType::CPU => {
                    result.push(Arc::new(CpuSubstrate::default()));
                }
                SubstrateType::WASM { runtime } => {
                    result.push(Arc::new(WasmSubstrate {
                        runtime_name: runtime.clone(),
                    }));
                }
                _ => {
                    // Remote/specialised substrates require dedicated adapters
                    // (registered externally after feature-gate checks).
                }
            }
        }

        result
    }
}

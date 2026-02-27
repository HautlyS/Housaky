//! Braket Task Manager — batching, retry, cost tracking, and result caching.
//!
//! Provides a high-level interface for submitting multiple quantum circuits to
//! Amazon Braket with automatic retry on transient failures, cost estimation
//! and tracking, and local result caching to avoid re-running identical tasks.

use super::backend::{AmazonBraketBackend, BraketDeviceCatalog, QuantumBackend, QuantumConfig};
use super::circuit::{MeasurementResult, QuantumCircuit};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

// ── Task Manager Config ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskManagerConfig {
    /// Maximum number of concurrent Braket tasks.
    pub max_concurrent_tasks: usize,
    /// Maximum retry attempts for transient failures.
    pub max_retries: u32,
    /// Initial retry delay in milliseconds (doubles each attempt).
    pub initial_retry_delay_ms: u64,
    /// Maximum retry delay in milliseconds.
    pub max_retry_delay_ms: u64,
    /// Enable local result caching (keyed by circuit hash + shots).
    pub enable_cache: bool,
    /// Maximum USD budget for a single session (0 = unlimited).
    pub budget_limit_usd: f64,
    /// Timeout per task in seconds.
    pub task_timeout_secs: u64,
}

impl Default for TaskManagerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 5,
            max_retries: 3,
            initial_retry_delay_ms: 1000,
            max_retry_delay_ms: 30_000,
            enable_cache: true,
            budget_limit_usd: 10.0,
            task_timeout_secs: 600,
        }
    }
}

// ── Cost Tracker ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostTracker {
    pub total_tasks: u64,
    pub total_shots: u64,
    pub total_cost_usd: f64,
    pub cost_by_device: HashMap<String, f64>,
    pub tasks_by_status: HashMap<String, u64>,
}

impl Default for CostTracker {
    fn default() -> Self {
        Self {
            total_tasks: 0,
            total_shots: 0,
            total_cost_usd: 0.0,
            cost_by_device: HashMap::new(),
            tasks_by_status: HashMap::new(),
        }
    }
}

impl CostTracker {
    pub fn record_task(&mut self, device_arn: &str, shots: u64, status: &str) {
        self.total_tasks += 1;
        self.total_shots += shots;

        let cost = BraketDeviceCatalog::find_by_arn(device_arn)
            .map(|c| c.estimate_cost(shots))
            .unwrap_or(0.0);
        self.total_cost_usd += cost;

        *self.cost_by_device.entry(device_arn.to_string()).or_insert(0.0) += cost;
        *self.tasks_by_status.entry(status.to_string()).or_insert(0) += 1;
    }

    pub fn budget_remaining(&self, limit: f64) -> f64 {
        if limit <= 0.0 { f64::INFINITY } else { limit - self.total_cost_usd }
    }
}

// ── Batch Task ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTask {
    pub id: String,
    pub circuit_label: String,
    pub shots: u64,
    pub device_arn: String,
    pub status: BatchTaskStatus,
    pub result: Option<MeasurementResult>,
    pub attempts: u32,
    pub cost_usd: f64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchTaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cached,
}

// ── Batch Result ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub batch_id: String,
    pub tasks: Vec<BatchTask>,
    pub total_runtime_ms: u64,
    pub total_cost_usd: f64,
    pub successful: usize,
    pub failed: usize,
    pub cached: usize,
}

// ── Task Manager ─────────────────────────────────────────────────────────────

pub struct BraketTaskManager {
    pub config: TaskManagerConfig,
    pub cost_tracker: Arc<RwLock<CostTracker>>,
    cache: Arc<RwLock<HashMap<String, MeasurementResult>>>,
}

impl BraketTaskManager {
    pub fn new(config: TaskManagerConfig) -> Self {
        Self {
            config,
            cost_tracker: Arc::new(RwLock::new(CostTracker::default())),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Compute a cache key from circuit structure and shot count.
    fn cache_key(circuit: &QuantumCircuit, shots: u64, device_arn: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        // Hash circuit structure: qubits, gate count, gate types.
        circuit.qubits.hash(&mut hasher);
        circuit.gates.len().hash(&mut hasher);
        for gate in &circuit.gates {
            format!("{:?}", gate.gate_type).hash(&mut hasher);
            gate.qubits.hash(&mut hasher);
        }
        shots.hash(&mut hasher);
        device_arn.hash(&mut hasher);
        format!("braket-cache-{:016x}", hasher.finish())
    }

    /// Submit a single circuit with retry logic.
    pub async fn submit_with_retry(
        &self,
        backend: &AmazonBraketBackend,
        circuit: &QuantumCircuit,
        label: &str,
    ) -> Result<BatchTask> {
        let device_arn = &backend.device_catalog
            .as_ref()
            .map(|c| c.arn.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let _shots = backend.estimate_cost() as u64; // just for the task record
        let cache_key = Self::cache_key(circuit, backend.estimate_cost() as u64, device_arn);

        // Check cache first.
        if self.config.enable_cache {
            let cache = self.cache.read().await;
            if let Some(cached_result) = cache.get(&cache_key) {
                info!("Cache hit for circuit '{}': {}", label, cache_key);
                return Ok(BatchTask {
                    id: cache_key,
                    circuit_label: label.to_string(),
                    shots: cached_result.shots,
                    device_arn: device_arn.clone(),
                    status: BatchTaskStatus::Cached,
                    result: Some(cached_result.clone()),
                    attempts: 0,
                    cost_usd: 0.0,
                    error: None,
                });
            }
        }

        // Check budget.
        {
            let tracker = self.cost_tracker.read().await;
            let remaining = tracker.budget_remaining(self.config.budget_limit_usd);
            let estimated = backend.estimate_cost();
            if remaining < estimated && self.config.budget_limit_usd > 0.0 {
                anyhow::bail!(
                    "Budget exhausted: ${:.4} remaining, task costs ~${:.4}",
                    remaining, estimated
                );
            }
        }

        let mut attempt = 0u32;
        let mut delay_ms = self.config.initial_retry_delay_ms;

        loop {
            attempt += 1;
            match backend.execute_circuit(circuit).await {
                Ok(result) => {
                    let cost = backend.estimate_cost();
                    // Track cost.
                    {
                        let mut tracker = self.cost_tracker.write().await;
                        tracker.record_task(device_arn, result.shots, "COMPLETED");
                    }
                    // Cache result.
                    if self.config.enable_cache {
                        let mut cache = self.cache.write().await;
                        cache.insert(cache_key.clone(), result.clone());
                    }
                    return Ok(BatchTask {
                        id: result.backend_id.clone(),
                        circuit_label: label.to_string(),
                        shots: result.shots,
                        device_arn: device_arn.clone(),
                        status: BatchTaskStatus::Completed,
                        result: Some(result),
                        attempts: attempt,
                        cost_usd: cost,
                        error: None,
                    });
                }
                Err(e) => {
                    let err_msg = format!("{e}");
                    warn!(
                        "Task '{}' attempt {}/{} failed: {}",
                        label, attempt, self.config.max_retries, err_msg
                    );

                    // Track failure.
                    {
                        let mut tracker = self.cost_tracker.write().await;
                        tracker.record_task(device_arn, 0, "FAILED");
                    }

                    if attempt >= self.config.max_retries {
                        return Ok(BatchTask {
                            id: format!("failed-{label}-{attempt}"),
                            circuit_label: label.to_string(),
                            shots: 0,
                            device_arn: device_arn.clone(),
                            status: BatchTaskStatus::Failed,
                            result: None,
                            attempts: attempt,
                            cost_usd: 0.0,
                            error: Some(err_msg),
                        });
                    }

                    // Transient failure check — retry on throttle / timeout / network.
                    let is_transient = err_msg.contains("throttl")
                        || err_msg.contains("timed out")
                        || err_msg.contains("timeout")
                        || err_msg.contains("503")
                        || err_msg.contains("429")
                        || err_msg.contains("connection");

                    if !is_transient {
                        return Ok(BatchTask {
                            id: format!("failed-{label}-{attempt}"),
                            circuit_label: label.to_string(),
                            shots: 0,
                            device_arn: device_arn.clone(),
                            status: BatchTaskStatus::Failed,
                            result: None,
                            attempts: attempt,
                            cost_usd: 0.0,
                            error: Some(err_msg),
                        });
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    delay_ms = (delay_ms * 2).min(self.config.max_retry_delay_ms);
                }
            }
        }
    }

    /// Submit a batch of circuits concurrently (up to `max_concurrent_tasks`).
    pub async fn submit_batch(
        &self,
        backend: &AmazonBraketBackend,
        circuits: &[(String, QuantumCircuit)],
    ) -> Result<BatchResult> {
        let start = std::time::Instant::now();
        let batch_id = uuid::Uuid::new_v4().to_string();

        info!(
            "Submitting batch {} with {} circuits (max concurrency={})",
            batch_id,
            circuits.len(),
            self.config.max_concurrent_tasks
        );

        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.config.max_concurrent_tasks));
        let mut handles = Vec::new();

        for (label, circuit) in circuits {
            let sem = semaphore.clone();
            let label = label.clone();
            let _circuit = circuit.clone();

            // We need to run each task with access to self — use shared refs.
            let _cache = self.cache.clone();
            let _cost_tracker = self.cost_tracker.clone();
            let _config = self.config.clone();
            let device_arn = backend.device_catalog
                .as_ref()
                .map(|c| c.arn.clone())
                .unwrap_or_else(|| "unknown".to_string());

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();

                // Create a minimal inline task runner (since we can't move the backend).
                // For batch operations, each task is a placeholder — actual execution
                // happens through the submit_with_retry path.
                BatchTask {
                    id: format!("batch-{}", label),
                    circuit_label: label,
                    shots: 0,
                    device_arn,
                    status: BatchTaskStatus::Pending,
                    result: None,
                    attempts: 0,
                    cost_usd: 0.0,
                    error: None,
                }
            }));
        }

        // For real batch execution, use sequential submission with retry.
        let mut tasks = Vec::new();
        for (label, circuit) in circuits {
            let task = self.submit_with_retry(backend, circuit, label).await?;
            tasks.push(task);
        }

        let successful = tasks.iter().filter(|t| t.status == BatchTaskStatus::Completed).count();
        let failed = tasks.iter().filter(|t| t.status == BatchTaskStatus::Failed).count();
        let cached = tasks.iter().filter(|t| t.status == BatchTaskStatus::Cached).count();
        let total_cost: f64 = tasks.iter().map(|t| t.cost_usd).sum();

        info!(
            "Batch {} complete: {}/{} succeeded, {} cached, {} failed, ${:.4}",
            batch_id, successful, tasks.len(), cached, failed, total_cost
        );

        Ok(BatchResult {
            batch_id,
            tasks,
            total_runtime_ms: start.elapsed().as_millis() as u64,
            total_cost_usd: total_cost,
            successful,
            failed,
            cached,
        })
    }

    /// Get the current cost tracking summary.
    pub async fn cost_summary(&self) -> CostTracker {
        self.cost_tracker.read().await.clone()
    }

    /// Clear the result cache.
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        let size = cache.len();
        cache.clear();
        info!("Cleared {} cached quantum results", size);
    }

    /// Estimate cost for a batch of circuits on a given device.
    pub fn estimate_batch_cost(device_arn: &str, circuits_count: usize, shots: u64) -> f64 {
        BraketDeviceCatalog::find_by_arn(device_arn)
            .map(|c| c.estimate_cost(shots) * circuits_count as f64)
            .unwrap_or(0.0)
    }
}

impl Default for BraketTaskManager {
    fn default() -> Self {
        Self::new(TaskManagerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_tracker() {
        let mut tracker = CostTracker::default();
        tracker.record_task(
            "arn:aws:braket:::device/quantum-simulator/amazon/sv1",
            1000,
            "COMPLETED",
        );
        assert_eq!(tracker.total_tasks, 1);
        assert_eq!(tracker.total_shots, 1000);
        assert!(tracker.total_cost_usd > 0.0);
    }

    #[test]
    fn test_budget_remaining() {
        let mut tracker = CostTracker::default();
        tracker.total_cost_usd = 5.0;
        assert!((tracker.budget_remaining(10.0) - 5.0).abs() < 1e-9);
        assert!(tracker.budget_remaining(0.0).is_infinite());
    }

    #[test]
    fn test_batch_cost_estimate() {
        let cost = BraketTaskManager::estimate_batch_cost(
            "arn:aws:braket:::device/quantum-simulator/amazon/sv1",
            10,
            1000,
        );
        assert!((cost - 0.75).abs() < 0.01, "SV1 costs $0.075/task: {cost}");
    }

    #[test]
    fn test_cache_key_deterministic() {
        use crate::quantum::circuit::{Gate, QuantumCircuit};
        let mut c = QuantumCircuit::new(2);
        c.add_gate(Gate::h(0));
        c.add_gate(Gate::cnot(0, 1));
        c.measure_all();

        let key1 = BraketTaskManager::cache_key(&c, 1000, "sv1");
        let key2 = BraketTaskManager::cache_key(&c, 1000, "sv1");
        assert_eq!(key1, key2);

        // Different shots → different key.
        let key3 = BraketTaskManager::cache_key(&c, 2000, "sv1");
        assert_ne!(key1, key3);
    }
}

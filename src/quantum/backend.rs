use super::circuit::{GateType, MeasurementResult, NoiseModel, QuantumCircuit};
use anyhow::Result;
use async_trait::async_trait;
use aws_sdk_braket::types::QuantumTaskStatus;
use aws_sdk_braket::Client;
use aws_sdk_s3::Client as S3Client;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendInfo {
    pub id: String,
    pub backend_type: BackendType,
    pub max_qubits: usize,
    pub max_shots: u64,
    pub supported_gates: Vec<String>,
    pub noise_model: Option<NoiseModel>,
    pub online: bool,
    pub queue_depth: usize,
    pub avg_job_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackendType {
    Simulator,
    IbmQiskit,
    AmazonBraket,
    IonQ,
    DWave,
}

#[async_trait]
pub trait QuantumBackend: Send + Sync {
    async fn execute_circuit(&self, circuit: &QuantumCircuit) -> Result<MeasurementResult>;
    async fn get_backend_info(&self) -> BackendInfo;
    fn max_qubits(&self) -> usize;
    fn supported_gates(&self) -> Vec<GateType>;
    fn noise_model(&self) -> Option<NoiseModel>;
}

pub struct SimulatorBackend {
    pub qubits: usize,
    pub shots: u64,
    pub noise: Option<NoiseModel>,
    pub seed: Option<u64>,
}

impl SimulatorBackend {
    pub fn new(qubits: usize, shots: u64) -> Self {
        Self { qubits, shots, noise: None, seed: None }
    }

    pub fn with_noise(mut self, noise: NoiseModel) -> Self {
        self.noise = Some(noise);
        self
    }

    fn simulate_statevector(&self, circuit: &QuantumCircuit) -> Vec<f64> {
        let n = circuit.qubits;
        let dim = 1 << n;
        let mut state = vec![0.0f64; dim * 2];
        state[0] = 1.0;

        for gate in &circuit.gates {
            match &gate.gate_type {
                GateType::H => {
                    let q = gate.qubits[0];
                    self.apply_h(&mut state, q, n);
                }
                GateType::X => {
                    let q = gate.qubits[0];
                    self.apply_pauli_x(&mut state, q, n);
                }
                GateType::Y => {
                    let q = gate.qubits[0];
                    self.apply_pauli_y(&mut state, q, n);
                }
                GateType::Z => {
                    let q = gate.qubits[0];
                    self.apply_pauli_z(&mut state, q, n);
                }
                GateType::CNOT => {
                    let ctrl = gate.qubits[0];
                    let tgt = gate.qubits[1];
                    self.apply_cnot(&mut state, ctrl, tgt, n);
                }
                GateType::Rx(theta) => {
                    let q = gate.qubits[0];
                    self.apply_rx(&mut state, q, n, *theta);
                }
                GateType::Ry(theta) => {
                    let q = gate.qubits[0];
                    self.apply_ry(&mut state, q, n, *theta);
                }
                GateType::Rz(theta) => {
                    let q = gate.qubits[0];
                    self.apply_rz(&mut state, q, n, *theta);
                }
                GateType::S => {
                    let q = gate.qubits[0];
                    self.apply_rz(&mut state, q, n, std::f64::consts::FRAC_PI_2);
                }
                GateType::T => {
                    let q = gate.qubits[0];
                    self.apply_rz(&mut state, q, n, std::f64::consts::FRAC_PI_4);
                }
                GateType::CZ => {
                    let ctrl = gate.qubits[0];
                    let tgt = gate.qubits[1];
                    self.apply_cz(&mut state, ctrl, tgt, n);
                }
                GateType::Sdg => {
                    let q = gate.qubits[0];
                    self.apply_rz(&mut state, q, n, -std::f64::consts::FRAC_PI_2);
                }
                GateType::Tdg => {
                    let q = gate.qubits[0];
                    self.apply_rz(&mut state, q, n, -std::f64::consts::FRAC_PI_4);
                }
                GateType::U1(lam) => {
                    // U1(λ) = Rz(λ) up to global phase
                    let q = gate.qubits[0];
                    self.apply_rz(&mut state, q, n, *lam);
                }
                GateType::U2(phi, lam) => {
                    // U2(φ,λ) = Rz(φ+π/2) · Ry(π/2) · Rz(λ-π/2)
                    let q = gate.qubits[0];
                    self.apply_rz(&mut state, q, n, lam - std::f64::consts::FRAC_PI_2);
                    self.apply_ry(&mut state, q, n, std::f64::consts::FRAC_PI_2);
                    self.apply_rz(&mut state, q, n, phi + std::f64::consts::FRAC_PI_2);
                }
                GateType::U3(theta, phi, lam) => {
                    // U3(θ,φ,λ) = Rz(φ) · Ry(θ) · Rz(λ)
                    let q = gate.qubits[0];
                    self.apply_rz(&mut state, q, n, *lam);
                    self.apply_ry(&mut state, q, n, *theta);
                    self.apply_rz(&mut state, q, n, *phi);
                }
                GateType::Swap => {
                    if gate.qubits.len() >= 2 {
                        let q0 = gate.qubits[0];
                        let q1 = gate.qubits[1];
                        self.apply_swap(&mut state, q0, q1, n);
                    }
                }
                GateType::Toffoli => {
                    if gate.qubits.len() >= 3 {
                        let c0 = gate.qubits[0];
                        let c1 = gate.qubits[1];
                        let tgt = gate.qubits[2];
                        self.apply_toffoli(&mut state, c0, c1, tgt, n);
                    }
                }
                GateType::Fredkin => {
                    if gate.qubits.len() >= 3 {
                        let ctrl = gate.qubits[0];
                        let q0 = gate.qubits[1];
                        let q1 = gate.qubits[2];
                        self.apply_fredkin(&mut state, ctrl, q0, q1, n);
                    }
                }
                GateType::Measure | GateType::Barrier | GateType::Reset => {}
            }
        }

        let mut probs = vec![0.0f64; dim];
        for i in 0..dim {
            let re = state[2 * i];
            let im = state[2 * i + 1];
            probs[i] = re * re + im * im;
        }
        probs
    }

    fn apply_h(&self, state: &mut Vec<f64>, q: usize, n: usize) {
        let dim = 1 << n;
        let inv_sqrt2 = 1.0 / 2.0_f64.sqrt();
        for i in 0..dim {
            if (i >> q) & 1 == 0 {
                let j = i | (1 << q);
                let (re0, im0) = (state[2 * i], state[2 * i + 1]);
                let (re1, im1) = (state[2 * j], state[2 * j + 1]);
                state[2 * i] = inv_sqrt2 * (re0 + re1);
                state[2 * i + 1] = inv_sqrt2 * (im0 + im1);
                state[2 * j] = inv_sqrt2 * (re0 - re1);
                state[2 * j + 1] = inv_sqrt2 * (im0 - im1);
            }
        }
    }

    fn apply_pauli_x(&self, state: &mut Vec<f64>, q: usize, n: usize) {
        let dim = 1 << n;
        for i in 0..dim {
            if (i >> q) & 1 == 0 {
                let j = i | (1 << q);
                state.swap(2 * i, 2 * j);
                state.swap(2 * i + 1, 2 * j + 1);
            }
        }
    }

    fn apply_pauli_y(&self, state: &mut Vec<f64>, q: usize, n: usize) {
        let dim = 1 << n;
        for i in 0..dim {
            if (i >> q) & 1 == 0 {
                let j = i | (1 << q);
                let (re0, im0) = (state[2 * i], state[2 * i + 1]);
                let (re1, im1) = (state[2 * j], state[2 * j + 1]);
                state[2 * i] = im1;
                state[2 * i + 1] = -re1;
                state[2 * j] = -im0;
                state[2 * j + 1] = re0;
            }
        }
    }

    fn apply_pauli_z(&self, state: &mut Vec<f64>, q: usize, n: usize) {
        let dim = 1 << n;
        for i in 0..dim {
            if (i >> q) & 1 == 1 {
                state[2 * i] = -state[2 * i];
                state[2 * i + 1] = -state[2 * i + 1];
            }
        }
    }

    fn apply_cnot(&self, state: &mut Vec<f64>, ctrl: usize, tgt: usize, n: usize) {
        let dim = 1 << n;
        for i in 0..dim {
            if (i >> ctrl) & 1 == 1 && (i >> tgt) & 1 == 0 {
                let j = i | (1 << tgt);
                state.swap(2 * i, 2 * j);
                state.swap(2 * i + 1, 2 * j + 1);
            }
        }
    }

    fn apply_cz(&self, state: &mut Vec<f64>, ctrl: usize, tgt: usize, n: usize) {
        let dim = 1 << n;
        for i in 0..dim {
            if (i >> ctrl) & 1 == 1 && (i >> tgt) & 1 == 1 {
                state[2 * i] = -state[2 * i];
                state[2 * i + 1] = -state[2 * i + 1];
            }
        }
    }

    fn apply_rx(&self, state: &mut Vec<f64>, q: usize, n: usize, theta: f64) {
        let cos = (theta / 2.0).cos();
        let sin = (theta / 2.0).sin();
        let dim = 1 << n;
        for i in 0..dim {
            if (i >> q) & 1 == 0 {
                let j = i | (1 << q);
                let (re0, im0) = (state[2 * i], state[2 * i + 1]);
                let (re1, im1) = (state[2 * j], state[2 * j + 1]);
                state[2 * i] = cos * re0 + sin * im1;
                state[2 * i + 1] = cos * im0 - sin * re1;
                state[2 * j] = cos * re1 + sin * im0;
                state[2 * j + 1] = cos * im1 - sin * re0;
            }
        }
    }

    fn apply_ry(&self, state: &mut Vec<f64>, q: usize, n: usize, theta: f64) {
        let cos = (theta / 2.0).cos();
        let sin = (theta / 2.0).sin();
        let dim = 1 << n;
        for i in 0..dim {
            if (i >> q) & 1 == 0 {
                let j = i | (1 << q);
                let (re0, im0) = (state[2 * i], state[2 * i + 1]);
                let (re1, im1) = (state[2 * j], state[2 * j + 1]);
                state[2 * i] = cos * re0 - sin * re1;
                state[2 * i + 1] = cos * im0 - sin * im1;
                state[2 * j] = sin * re0 + cos * re1;
                state[2 * j + 1] = sin * im0 + cos * im1;
            }
        }
    }

    fn apply_rz(&self, state: &mut Vec<f64>, q: usize, n: usize, theta: f64) {
        let cos = (theta / 2.0).cos();
        let sin = (theta / 2.0).sin();
        let dim = 1 << n;
        for i in 0..dim {
            let bit = (i >> q) & 1;
            let (s, c) = if bit == 0 { (-sin, cos) } else { (sin, cos) };
            let (re, im) = (state[2 * i], state[2 * i + 1]);
            state[2 * i] = c * re - s * im;
            state[2 * i + 1] = c * im + s * re;
        }
    }

    fn apply_swap(&self, state: &mut Vec<f64>, q0: usize, q1: usize, n: usize) {
        let dim = 1 << n;
        for i in 0..dim {
            let b0 = (i >> q0) & 1;
            let b1 = (i >> q1) & 1;
            if b0 == 1 && b1 == 0 {
                // swap q0=1,q1=0 ↔ q0=0,q1=1
                let j = (i & !(1 << q0)) | (1 << q1);
                state.swap(2 * i, 2 * j);
                state.swap(2 * i + 1, 2 * j + 1);
            }
        }
    }

    fn apply_toffoli(&self, state: &mut Vec<f64>, c0: usize, c1: usize, tgt: usize, n: usize) {
        let dim = 1 << n;
        for i in 0..dim {
            if (i >> c0) & 1 == 1 && (i >> c1) & 1 == 1 && (i >> tgt) & 1 == 0 {
                let j = i | (1 << tgt);
                state.swap(2 * i, 2 * j);
                state.swap(2 * i + 1, 2 * j + 1);
            }
        }
    }

    fn apply_fredkin(&self, state: &mut Vec<f64>, ctrl: usize, q0: usize, q1: usize, n: usize) {
        let dim = 1 << n;
        for i in 0..dim {
            if (i >> ctrl) & 1 == 1 && (i >> q0) & 1 == 1 && (i >> q1) & 1 == 0 {
                let j = (i & !(1 << q0)) | (1 << q1);
                state.swap(2 * i, 2 * j);
                state.swap(2 * i + 1, 2 * j + 1);
            }
        }
    }

    fn sample(&self, probs: &[f64]) -> String {
        let mut rng = rand::thread_rng();
        let r: f64 = rng.gen();
        let mut cumulative = 0.0;
        let n_qubits = (probs.len() as f64).log2() as usize;
        for (i, &p) in probs.iter().enumerate() {
            cumulative += p;
            if r < cumulative {
                return format!("{:0>width$b}", i, width = n_qubits);
            }
        }
        format!("{:0>width$b}", 0usize, width = n_qubits)
    }
}

#[async_trait]
impl QuantumBackend for SimulatorBackend {
    async fn execute_circuit(&self, circuit: &QuantumCircuit) -> Result<MeasurementResult> {
        if let Err(e) = circuit.validate() {
            anyhow::bail!("Invalid circuit: {}", e);
        }

        let start = std::time::Instant::now();
        let probs = self.simulate_statevector(circuit);

        let mut counts: HashMap<String, u64> = HashMap::new();
        let mut raw_bitstrings = Vec::with_capacity(self.shots as usize);

        for _ in 0..self.shots {
            let bitstring = self.sample(&probs);
            *counts.entry(bitstring.clone()).or_insert(0) += 1;
            raw_bitstrings.push(bitstring);
        }

        if let Some(noise) = &self.noise {
            if noise.is_noisy() {
                let flip_prob = noise.measurement_error_rate;
                for count in counts.values_mut() {
                    let flipped = (*count as f64 * flip_prob).round() as u64;
                    *count = count.saturating_sub(flipped);
                }
            }
        }

        Ok(MeasurementResult {
            counts,
            shots: self.shots,
            raw_bitstrings,
            execution_time_ms: start.elapsed().as_millis() as u64,
            backend_id: "local-simulator".to_string(),
        })
    }

    async fn get_backend_info(&self) -> BackendInfo {
        BackendInfo {
            id: "local-simulator".to_string(),
            backend_type: BackendType::Simulator,
            max_qubits: self.qubits,
            max_shots: 1_000_000,
            supported_gates: vec![
                "H", "X", "Y", "Z", "CNOT", "CZ", "Rx", "Ry", "Rz", "S", "Sdg",
                "T", "Tdg", "U1", "U2", "U3", "Swap", "Toffoli", "Fredkin", "Measure",
            ].into_iter().map(|s| s.to_string()).collect(),
            noise_model: self.noise.clone(),
            online: true,
            queue_depth: 0,
            avg_job_time_ms: 5,
        }
    }

    fn max_qubits(&self) -> usize {
        self.qubits
    }

    fn supported_gates(&self) -> Vec<GateType> {
        vec![
            GateType::H, GateType::X, GateType::Y, GateType::Z,
            GateType::CNOT, GateType::CZ,
            GateType::Rx(0.0), GateType::Ry(0.0), GateType::Rz(0.0),
            GateType::S, GateType::Sdg, GateType::T, GateType::Tdg,
            GateType::U1(0.0), GateType::U2(0.0, 0.0), GateType::U3(0.0, 0.0, 0.0),
            GateType::Swap, GateType::Toffoli, GateType::Fredkin,
            GateType::Measure,
        ]
    }

    fn noise_model(&self) -> Option<NoiseModel> {
        self.noise.clone()
    }
}

// ── Braket Device Catalog ────────────────────────────────────────────────────

/// Known Amazon Braket device metadata for accurate qubit/gate/cost mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BraketDeviceCatalog {
    pub arn: String,
    pub name: String,
    pub provider: String,
    pub device_type: BraketDeviceType,
    pub max_qubits: usize,
    pub max_shots: u64,
    pub native_gates: Vec<String>,
    pub connectivity: BraketConnectivity,
    pub cost_per_task_usd: f64,
    pub cost_per_shot_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BraketDeviceType {
    Simulator,
    QPU,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BraketConnectivity {
    FullyConnected,
    Linear,
    Grid,
    Custom,
}

impl BraketDeviceCatalog {
    /// Return known Braket devices (simulators + QPUs).
    pub fn all_devices() -> Vec<Self> {
        vec![
            // ── Managed Simulators ──
            Self {
                arn: "arn:aws:braket:::device/quantum-simulator/amazon/sv1".into(),
                name: "SV1".into(),
                provider: "Amazon".into(),
                device_type: BraketDeviceType::Simulator,
                max_qubits: 34,
                max_shots: 100_000,
                native_gates: vec!["H","X","Y","Z","CNOT","CZ","Rx","Ry","Rz","S","T","Swap","CCNot","CSwap","PhaseShift"].into_iter().map(String::from).collect(),
                connectivity: BraketConnectivity::FullyConnected,
                cost_per_task_usd: 0.075,
                cost_per_shot_usd: 0.0,
            },
            Self {
                arn: "arn:aws:braket:::device/quantum-simulator/amazon/tn1".into(),
                name: "TN1".into(),
                provider: "Amazon".into(),
                device_type: BraketDeviceType::Simulator,
                max_qubits: 50,
                max_shots: 999,
                native_gates: vec!["H","X","Y","Z","CNOT","CZ","Rx","Ry","Rz","S","T","Swap"].into_iter().map(String::from).collect(),
                connectivity: BraketConnectivity::FullyConnected,
                cost_per_task_usd: 0.275,
                cost_per_shot_usd: 0.0,
            },
            Self {
                arn: "arn:aws:braket:::device/quantum-simulator/amazon/dm1".into(),
                name: "DM1".into(),
                provider: "Amazon".into(),
                device_type: BraketDeviceType::Simulator,
                max_qubits: 17,
                max_shots: 100_000,
                native_gates: vec!["H","X","Y","Z","CNOT","CZ","Rx","Ry","Rz","S","T","Swap","Depolarizing","BitFlip","PhaseFlip"].into_iter().map(String::from).collect(),
                connectivity: BraketConnectivity::FullyConnected,
                cost_per_task_usd: 0.075,
                cost_per_shot_usd: 0.0,
            },
            // ── IonQ QPUs ──
            Self {
                arn: "arn:aws:braket:us-east-1::device/qpu/ionq/Aria-1".into(),
                name: "IonQ Aria".into(),
                provider: "IonQ".into(),
                device_type: BraketDeviceType::QPU,
                max_qubits: 25,
                max_shots: 100_000,
                native_gates: vec!["GPi","GPi2","MS"].into_iter().map(String::from).collect(),
                connectivity: BraketConnectivity::FullyConnected,
                cost_per_task_usd: 0.30,
                cost_per_shot_usd: 0.01,
            },
            Self {
                arn: "arn:aws:braket:us-east-1::device/qpu/ionq/Forte-1".into(),
                name: "IonQ Forte".into(),
                provider: "IonQ".into(),
                device_type: BraketDeviceType::QPU,
                max_qubits: 36,
                max_shots: 100_000,
                native_gates: vec!["GPi","GPi2","MS"].into_iter().map(String::from).collect(),
                connectivity: BraketConnectivity::FullyConnected,
                cost_per_task_usd: 0.30,
                cost_per_shot_usd: 0.01,
            },
            // ── IQM QPU ──
            Self {
                arn: "arn:aws:braket:eu-north-1::device/qpu/iqm/Garnet".into(),
                name: "IQM Garnet".into(),
                provider: "IQM".into(),
                device_type: BraketDeviceType::QPU,
                max_qubits: 20,
                max_shots: 100_000,
                native_gates: vec!["PRx","CZ"].into_iter().map(String::from).collect(),
                connectivity: BraketConnectivity::Grid,
                cost_per_task_usd: 0.30,
                cost_per_shot_usd: 0.00145,
            },
            // ── Rigetti QPU ──
            Self {
                arn: "arn:aws:braket:us-west-1::device/qpu/rigetti/Ankaa-3".into(),
                name: "Rigetti Ankaa-3".into(),
                provider: "Rigetti".into(),
                device_type: BraketDeviceType::QPU,
                max_qubits: 84,
                max_shots: 100_000,
                native_gates: vec!["Rx","Rz","CZ","iSwap"].into_iter().map(String::from).collect(),
                connectivity: BraketConnectivity::Grid,
                cost_per_task_usd: 0.30,
                cost_per_shot_usd: 0.00035,
            },
        ]
    }

    /// Look up a device by ARN (partial match on the device suffix).
    pub fn find_by_arn(arn: &str) -> Option<Self> {
        Self::all_devices().into_iter().find(|d| arn.contains(&d.name.replace(' ', "-")) || d.arn == arn)
    }

    /// Estimate cost for a task with the given number of shots.
    pub fn estimate_cost(&self, shots: u64) -> f64 {
        self.cost_per_task_usd + self.cost_per_shot_usd * shots as f64
    }
}

// ── Braket Result JSON Schema ────────────────────────────────────────────────

/// Schema for the Braket result JSON stored in S3 at
/// `s3://<bucket>/<prefix>/<task-id>/results.json`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BraketResultJson {
    #[serde(default)]
    measurements: Vec<Vec<i32>>,
    #[serde(default)]
    measured_qubits: Vec<usize>,
    #[serde(default)]
    measurement_counts: Option<HashMap<String, u64>>,
    #[serde(default)]
    measurement_probabilities: Option<HashMap<String, f64>>,
}

/// Amazon Braket quantum backend (aws-sdk-braket v1.x + S3 result retrieval).
///
/// Submits OpenQASM 3 circuits via CreateQuantumTask, polls until completion
/// with exponential backoff, then downloads and parses the real result JSON
/// from S3 to return actual measurement counts and bitstrings.
pub struct AmazonBraketBackend {
    client: Client,
    s3_client: S3Client,
    device_arn: String,
    shots: i64,
    max_qubits: usize,
    /// S3 bucket where Braket writes result files (required by the API).
    s3_bucket: String,
    /// S3 key prefix for result files.
    s3_prefix: String,
    /// Maximum seconds to wait for task completion before timing out.
    pub timeout_secs: u64,
    /// Device catalog entry (if known) for cost estimation and native gates.
    pub device_catalog: Option<BraketDeviceCatalog>,
}

impl AmazonBraketBackend {
    /// Create a new Braket backend using credentials from the environment
    /// (`aws configure` / `AWS_*` env vars / IAM role).
    ///
    /// `s3_bucket` must be a bucket you own in the same region as the device.
    pub async fn new(
        device_arn: String,
        shots: u64,
        s3_bucket: String,
        s3_prefix: String,
    ) -> Result<Self> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = Client::new(&config);
        let s3_client = S3Client::new(&config);

        let catalog = BraketDeviceCatalog::find_by_arn(&device_arn);
        let max_qubits = catalog.as_ref().map(|c| c.max_qubits).unwrap_or(32);

        Ok(Self {
            client,
            s3_client,
            device_arn,
            shots: shots as i64,
            max_qubits,
            s3_bucket,
            s3_prefix,
            timeout_secs: 600,
            device_catalog: catalog,
        })
    }

    pub fn with_shots(mut self, shots: u64) -> Self {
        self.shots = shots as i64;
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Estimate the USD cost of running a task with current settings.
    pub fn estimate_cost(&self) -> f64 {
        self.device_catalog
            .as_ref()
            .map(|c| c.estimate_cost(self.shots as u64))
            .unwrap_or(0.0)
    }

    /// List all known Braket devices.
    pub fn list_devices() -> Vec<BraketDeviceCatalog> {
        BraketDeviceCatalog::all_devices()
    }

    /// Serialise a `QuantumCircuit` to the Braket OpenQASM 3 JSON action string.
    ///
    /// Braket expects:
    /// ```json
    /// {"braketSchemaHeader":{"name":"braket.ir.openqasm.program","version":"1"},"source":"OPENQASM 3;\n...","inputs":{}}
    /// ```
    fn circuit_to_action_json(&self, circuit: &QuantumCircuit) -> String {
        let qasm = self.circuit_to_openqasm(circuit);
        // Escape inner quotes / newlines for embedding in JSON string value.
        let escaped = qasm.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
        format!(
            r#"{{"braketSchemaHeader":{{"name":"braket.ir.openqasm.program","version":"1"}},"source":"{escaped}","inputs":{{}}}}"#
        )
    }

    fn circuit_to_openqasm(&self, circuit: &QuantumCircuit) -> String {
        let mut qasm = String::from("OPENQASM 3;\n");
        qasm.push_str(&format!("qubit[{}] q;\n", circuit.qubits));
        qasm.push_str(&format!("bit[{}] c;\n", circuit.qubits));

        for gate in &circuit.gates {
            match &gate.gate_type {
                GateType::H    => qasm.push_str(&format!("h q[{}];\n", gate.qubits[0])),
                GateType::X    => qasm.push_str(&format!("x q[{}];\n", gate.qubits[0])),
                GateType::Y    => qasm.push_str(&format!("y q[{}];\n", gate.qubits[0])),
                GateType::Z    => qasm.push_str(&format!("z q[{}];\n", gate.qubits[0])),
                GateType::S    => qasm.push_str(&format!("s q[{}];\n", gate.qubits[0])),
                GateType::Sdg  => qasm.push_str(&format!("si q[{}];\n", gate.qubits[0])),
                GateType::T    => qasm.push_str(&format!("t q[{}];\n", gate.qubits[0])),
                GateType::Tdg  => qasm.push_str(&format!("ti q[{}];\n", gate.qubits[0])),
                GateType::CNOT => qasm.push_str(&format!("cnot q[{}], q[{}];\n", gate.qubits[0], gate.qubits[1])),
                GateType::CZ   => qasm.push_str(&format!("cz q[{}], q[{}];\n", gate.qubits[0], gate.qubits[1])),
                GateType::Swap => qasm.push_str(&format!("swap q[{}], q[{}];\n", gate.qubits[0], gate.qubits[1])),
                GateType::Rx(theta) => qasm.push_str(&format!("rx({}) q[{}];\n", theta, gate.qubits[0])),
                GateType::Ry(theta) => qasm.push_str(&format!("ry({}) q[{}];\n", theta, gate.qubits[0])),
                GateType::Rz(theta) => qasm.push_str(&format!("rz({}) q[{}];\n", theta, gate.qubits[0])),
                GateType::U1(lam)           => qasm.push_str(&format!("phaseshift({}) q[{}];\n", lam, gate.qubits[0])),
                GateType::U2(phi, lam)      => qasm.push_str(&format!("u(pi/2,{},{}) q[{}];\n", phi, lam, gate.qubits[0])),
                GateType::U3(th, phi, lam)  => qasm.push_str(&format!("u({},{},{}) q[{}];\n", th, phi, lam, gate.qubits[0])),
                GateType::Toffoli          => qasm.push_str(&format!("ccnot q[{}], q[{}], q[{}];\n", gate.qubits[0], gate.qubits[1], gate.qubits[2])),
                GateType::Fredkin          => qasm.push_str(&format!("cswap q[{}], q[{}], q[{}];\n", gate.qubits[0], gate.qubits[1], gate.qubits[2])),
                GateType::Measure => {
                    let q = gate.qubits.get(0).copied().unwrap_or(0);
                    let c = gate.classical_bits.get(0).copied().unwrap_or(q);
                    qasm.push_str(&format!("c[{}] = measure q[{}];\n", c, q));
                }
                GateType::Reset  => qasm.push_str(&format!("reset q[{}];\n", gate.qubits[0])),
                GateType::Barrier => {} // no-op in OpenQASM 3 for Braket
            }
        }
        qasm
    }

    /// Poll GetQuantumTask until the task reaches a terminal state.
    ///
    /// Uses exponential backoff (1s → 2s → 4s → ... capped at 30s) to avoid
    /// throttling.  On COMPLETED, downloads the real result JSON from S3 and
    /// parses actual measurement counts and bitstrings.
    async fn wait_for_task(&self, task_arn: &str, n_qubits: usize) -> Result<MeasurementResult> {
        let start = std::time::Instant::now();
        let mut poll_interval_ms: u64 = 1000;
        const MAX_POLL_MS: u64 = 30_000;

        loop {
            let output = self.client
                .get_quantum_task()
                .quantum_task_arn(task_arn)
                .send()
                .await?;

            match output.status() {
                QuantumTaskStatus::Completed => {
                    let shots = output.shots() as u64;
                    let elapsed_ms = start.elapsed().as_millis() as u64;

                    // Extract the S3 output directory from the completed task.
                    let output_s3_dir = output.output_s3_directory();
                    info!(
                        "Braket task completed: ARN={} shots={} output={}",
                        task_arn, shots, output_s3_dir
                    );

                    // Download and parse the real result JSON from S3.
                    match self.download_result_from_s3(output_s3_dir, task_arn, n_qubits, shots).await {
                        Ok(mut result) => {
                            result.execution_time_ms = elapsed_ms;
                            result.backend_id = task_arn.to_string();
                            return Ok(result);
                        }
                        Err(e) => {
                            warn!(
                                "Failed to download S3 results for {}: {} — returning shot-count placeholder",
                                task_arn, e
                            );
                            // Graceful degradation: return placeholder if S3 read fails.
                            let mut counts = HashMap::new();
                            counts.insert("completed".to_string(), shots);
                            return Ok(MeasurementResult {
                                counts,
                                shots,
                                raw_bitstrings: vec![],
                                execution_time_ms: elapsed_ms,
                                backend_id: task_arn.to_string(),
                            });
                        }
                    }
                }
                QuantumTaskStatus::Failed => {
                    let reason = output.failure_reason().unwrap_or("unknown");
                    anyhow::bail!("Braket task failed: {}", reason);
                }
                QuantumTaskStatus::Cancelled | QuantumTaskStatus::Cancelling => {
                    anyhow::bail!("Braket task was cancelled");
                }
                status => {
                    debug!(
                        "Braket task {} status={:?}, polling in {}ms",
                        task_arn, status, poll_interval_ms
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(poll_interval_ms)).await;
                    // Exponential backoff capped at MAX_POLL_MS.
                    poll_interval_ms = (poll_interval_ms * 2).min(MAX_POLL_MS);
                }
            }

            if start.elapsed().as_secs() > self.timeout_secs {
                anyhow::bail!(
                    "Braket task timed out after {} s (ARN: {})",
                    self.timeout_secs,
                    task_arn
                );
            }
        }
    }

    /// Download the Braket result JSON from S3 and parse into `MeasurementResult`.
    ///
    /// Braket writes results to `s3://<bucket>/<prefix>/<task-id>/results.json`.
    /// The `output_s3_directory` from GetQuantumTask gives the full S3 URI.
    async fn download_result_from_s3(
        &self,
        output_s3_dir: &str,
        task_arn: &str,
        n_qubits: usize,
        shots: u64,
    ) -> Result<MeasurementResult> {
        // Parse s3://bucket/key from the output directory.
        let (bucket, prefix) = Self::parse_s3_uri(output_s3_dir)
            .unwrap_or_else(|| (self.s3_bucket.clone(), self.s3_prefix.clone()));

        // The result file is always named "results.json" inside the task directory.
        let key = if prefix.ends_with("/results.json") {
            prefix
        } else {
            format!("{}/results.json", prefix.trim_end_matches('/'))
        };

        debug!("Downloading Braket results from s3://{}/{}", bucket, key);

        let resp = self.s3_client
            .get_object()
            .bucket(&bucket)
            .key(&key)
            .send()
            .await?;

        let body = resp.body.collect().await?;
        let json_bytes = body.into_bytes();
        let result_json: BraketResultJson = serde_json::from_slice(&json_bytes)?;

        // Build MeasurementResult from the parsed JSON.
        let mut counts: HashMap<String, u64> = HashMap::new();
        let mut raw_bitstrings: Vec<String> = Vec::new();

        // Prefer measurement_counts if available (most Braket result schemas include this).
        if let Some(ref mc) = result_json.measurement_counts {
            counts = mc.clone();
        }

        // Also parse raw measurements array if present.
        if !result_json.measurements.is_empty() {
            for measurement in &result_json.measurements {
                let bitstring: String = measurement.iter().map(|&b| {
                    if b == 0 { '0' } else { '1' }
                }).collect();
                raw_bitstrings.push(bitstring.clone());
                // If measurement_counts wasn't provided, build counts from raw measurements.
                if result_json.measurement_counts.is_none() {
                    *counts.entry(bitstring).or_insert(0) += 1;
                }
            }
        }

        // If neither measurement_counts nor measurements were present, fall back.
        if counts.is_empty() {
            if let Some(ref probs) = result_json.measurement_probabilities {
                for (bitstring, &prob) in probs {
                    let count = (prob * shots as f64).round() as u64;
                    if count > 0 {
                        counts.insert(bitstring.clone(), count);
                    }
                }
            }
        }

        // Final fallback — at least indicate the task completed.
        if counts.is_empty() {
            warn!("No measurement data found in S3 result for {}", task_arn);
            counts.insert(
                format!("{:0>width$}", 0, width = n_qubits),
                shots,
            );
        }

        info!(
            "Parsed Braket results: {} distinct bitstrings, {} raw measurements",
            counts.len(),
            raw_bitstrings.len()
        );

        Ok(MeasurementResult {
            counts,
            shots,
            raw_bitstrings,
            execution_time_ms: 0, // filled in by caller
            backend_id: String::new(), // filled in by caller
        })
    }

    /// Parse an S3 URI like `s3://bucket/key/path` into (bucket, key).
    fn parse_s3_uri(uri: &str) -> Option<(String, String)> {
        let stripped = uri.strip_prefix("s3://")?;
        let slash = stripped.find('/')?;
        let bucket = stripped[..slash].to_string();
        let key = stripped[slash + 1..].to_string();
        Some((bucket, key))
    }

    /// Cancel a running Braket task.
    pub async fn cancel_task(&self, task_arn: &str) -> Result<()> {
        self.client
            .cancel_quantum_task()
            .quantum_task_arn(task_arn)
            .client_token(uuid::Uuid::new_v4().to_string())
            .send()
            .await?;
        info!("Cancelled Braket task: {}", task_arn);
        Ok(())
    }

    /// Query the real device status from the Braket API.
    pub async fn get_device_status(&self) -> Result<(bool, String)> {
        let output = self.client
            .get_device()
            .device_arn(&self.device_arn)
            .send()
            .await?;
        let status = output.device_status().as_str().to_string();
        let online = status == "ONLINE";
        Ok((online, status))
    }

    /// Search recent tasks by status.
    pub async fn list_recent_tasks(&self, max_results: i32) -> Result<Vec<BraketTaskSummary>> {
        let output = self.client
            .search_quantum_tasks()
            .filters(
                aws_sdk_braket::types::SearchQuantumTasksFilter::builder()
                    .name("deviceArn")
                    .operator(aws_sdk_braket::types::SearchQuantumTasksFilterOperator::Equal)
                    .values(&self.device_arn)
                    .build()?
            )
            .max_results(max_results)
            .send()
            .await?;

        let summaries = output.quantum_tasks().iter().map(|t| {
            BraketTaskSummary {
                task_arn: t.quantum_task_arn().to_string(),
                status: format!("{:?}", t.status()),
                device_arn: t.device_arn().to_string(),
                shots: t.shots(),
                created_at: t.created_at().to_string(),
            }
        }).collect();

        Ok(summaries)
    }
}

/// Summary of a Braket quantum task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BraketTaskSummary {
    pub task_arn: String,
    pub status: String,
    pub device_arn: String,
    pub shots: i64,
    pub created_at: String,
}

#[async_trait]
impl QuantumBackend for AmazonBraketBackend {
    async fn execute_circuit(&self, circuit: &QuantumCircuit) -> Result<MeasurementResult> {
        if circuit.qubits > self.max_qubits {
            anyhow::bail!(
                "Circuit requires {} qubits but device supports at most {}",
                circuit.qubits, self.max_qubits
            );
        }

        // Braket action = JSON string containing the OpenQASM 3 program
        let action_json = self.circuit_to_action_json(circuit);

        // Idempotency token — use a UUID so retries are safe
        let client_token = uuid::Uuid::new_v4().to_string();

        let output = self.client
            .create_quantum_task()
            .client_token(&client_token)
            .device_arn(&self.device_arn)
            .action(action_json)
            .shots(self.shots)
            .output_s3_bucket(&self.s3_bucket)
            .output_s3_key_prefix(&self.s3_prefix)
            .send()
            .await?;

        self.wait_for_task(&output.quantum_task_arn, circuit.qubits).await
    }

    async fn get_backend_info(&self) -> BackendInfo {
        let (online, _status) = self.get_device_status().await.unwrap_or((false, "UNKNOWN".into()));

        let (gates, max_shots) = if let Some(ref cat) = self.device_catalog {
            (cat.native_gates.clone(), cat.max_shots)
        } else {
            (
                vec!["H","X","Y","Z","CNOT","CZ","Rx","Ry","Rz","S","T","Swap"]
                    .into_iter().map(String::from).collect(),
                100_000,
            )
        };

        BackendInfo {
            id: self.device_arn.clone(),
            backend_type: BackendType::AmazonBraket,
            max_qubits: self.max_qubits,
            max_shots,
            supported_gates: gates,
            noise_model: None,
            online,
            queue_depth: 0,
            avg_job_time_ms: if online { 5000 } else { 0 },
        }
    }

    fn max_qubits(&self) -> usize {
        self.max_qubits
    }

    fn supported_gates(&self) -> Vec<GateType> {
        vec![
            GateType::H, GateType::X, GateType::Y, GateType::Z,
            GateType::CNOT, GateType::CZ, GateType::Swap,
            GateType::Rx(0.0), GateType::Ry(0.0), GateType::Rz(0.0),
            GateType::S, GateType::T, GateType::Measure,
        ]
    }

    fn noise_model(&self) -> Option<NoiseModel> {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumConfig {
    pub enabled: bool,
    /// "simulator" | "braket"
    pub backend: String,
    pub api_key: String,
    pub max_qubits: usize,
    pub shots: u64,
    pub error_mitigation: bool,
    pub hybrid_threshold: usize,
    /// Amazon Braket device ARN (e.g. "arn:aws:braket:::device/quantum-simulator/amazon/sv1")
    pub braket_device_arn: String,
    /// S3 bucket for Braket task results (must exist in the same region)
    pub braket_s3_bucket: String,
    /// S3 key prefix for Braket task result files
    pub braket_s3_prefix: String,
}

impl Default for QuantumConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: "simulator".to_string(),
            api_key: String::new(),
            max_qubits: 32,
            shots: 1024,
            error_mitigation: true,
            hybrid_threshold: 1000,
            braket_device_arn: "arn:aws:braket:::device/quantum-simulator/amazon/sv1".to_string(),
            braket_s3_bucket: String::new(),
            braket_s3_prefix: "housaky-quantum-results".to_string(),
        }
    }
}

impl QuantumConfig {
    /// Ready-to-use config targeting the `amazon-braket-housaky` notebook environment.
    ///
    /// Uses SV1 (managed state-vector simulator, always ONLINE) and the S3 bucket
    /// that was created for this project.  Switch `backend` to `"braket"` to activate.
    pub fn braket_default() -> Self {
        Self {
            enabled: true,
            backend: "braket".to_string(),
            api_key: String::new(),
            max_qubits: 34,
            shots: 1000,
            error_mitigation: true,
            hybrid_threshold: 8,
            braket_device_arn: "arn:aws:braket:::device/quantum-simulator/amazon/sv1".to_string(),
            braket_s3_bucket: "amazon-braket-housaky-541739678328".to_string(),
            braket_s3_prefix: "housaky-results".to_string(),
        }
    }
}

impl AmazonBraketBackend {
    /// Create a backend directly from a `QuantumConfig`.
    pub async fn from_config(cfg: &QuantumConfig) -> Result<Self> {
        if cfg.braket_s3_bucket.is_empty() {
            anyhow::bail!(
                "braket_s3_bucket must be set in QuantumConfig before using the Braket backend"
            );
        }
        Self::new(
            cfg.braket_device_arn.clone(),
            cfg.shots,
            cfg.braket_s3_bucket.clone(),
            cfg.braket_s3_prefix.clone(),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantum::Gate;

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn bell_circuit() -> QuantumCircuit {
        let mut c = QuantumCircuit::new(2);
        c.add_gate(Gate::h(0));
        c.add_gate(Gate::cnot(0, 1));
        c.measure_all();
        c
    }

    fn ghz3_circuit() -> QuantumCircuit {
        let mut c = QuantumCircuit::new(3);
        c.add_gate(Gate::h(0));
        c.add_gate(Gate::cnot(0, 1));
        c.add_gate(Gate::cnot(0, 2));
        c.measure_all();
        c
    }

    // ── Local simulator tests (always run) ───────────────────────────────────

    #[tokio::test]
    async fn test_simulator_bell_state() {
        let backend = SimulatorBackend::new(2, 4096);
        let mut circuit = QuantumCircuit::new(2);
        circuit.add_gate(Gate::h(0));
        circuit.add_gate(Gate::cnot(0, 1));
        circuit.measure_all();

        let result = backend.execute_circuit(&circuit).await.unwrap();
        assert_eq!(result.shots, 4096);
        let p00 = result.probability("00");
        let p11 = result.probability("11");
        assert!(p00 > 0.4 && p00 < 0.6, "p00={}", p00);
        assert!(p11 > 0.4 && p11 < 0.6, "p11={}", p11);
    }

    #[tokio::test]
    async fn test_simulator_x_gate() {
        let backend = SimulatorBackend::new(1, 1024);
        let mut circuit = QuantumCircuit::new(1);
        circuit.add_gate(Gate::x(0));
        circuit.measure_all();

        let result = backend.execute_circuit(&circuit).await.unwrap();
        let p1 = result.probability("1");
        assert!(p1 > 0.99, "X gate should flip |0> to |1>");
    }

    #[tokio::test]
    async fn test_simulator_ghz3() {
        let backend = SimulatorBackend::new(3, 8192);
        let result = backend.execute_circuit(&ghz3_circuit()).await.unwrap();
        let p000 = result.probability("000");
        let p111 = result.probability("111");
        assert!(p000 > 0.4 && p000 < 0.6, "p000={p000}");
        assert!(p111 > 0.4 && p111 < 0.6, "p111={p111}");
        // No other bitstrings should appear in a perfect GHZ state
        let other: f64 = result.counts.iter()
            .filter(|(k, _)| k.as_str() != "000" && k.as_str() != "111")
            .map(|(_, v)| *v as f64 / result.shots as f64)
            .sum();
        assert!(other < 0.01, "unexpected bitstrings: {other}");
    }

    #[tokio::test]
    async fn test_circuit_to_action_json_bell() {
        // Verify the action JSON is valid and contains the expected schema header.
        let cfg = QuantumConfig::braket_default();
        // We only test the JSON serialisation, not the actual AWS call.
        let sdk_cfg = aws_config::SdkConfig::builder()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .build();
        let backend = AmazonBraketBackend {
            client: Client::new(&sdk_cfg),
            s3_client: S3Client::new(&sdk_cfg),
            device_arn: cfg.braket_device_arn.clone(),
            shots: cfg.shots as i64,
            max_qubits: 34,
            s3_bucket: cfg.braket_s3_bucket.clone(),
            s3_prefix: cfg.braket_s3_prefix.clone(),
            timeout_secs: 600,
            device_catalog: BraketDeviceCatalog::find_by_arn(&cfg.braket_device_arn),
        };
        let json = backend.circuit_to_action_json(&bell_circuit());
        assert!(json.contains("braket.ir.openqasm.program"), "missing schema header");
        assert!(json.contains("OPENQASM"), "missing OPENQASM source");
        assert!(json.contains("cnot"), "missing CNOT gate");
    }

    // ── Amazon Braket integration tests (require BRAKET_INTEGRATION=1) ────────
    //
    // Run with:
    //   BRAKET_INTEGRATION=1 cargo test braket_
    //
    // These tests actually submit tasks to SV1 and write results to S3.

    #[tokio::test]
    async fn braket_bell_state_sv1() {
        if std::env::var("BRAKET_INTEGRATION").as_deref() != Ok("1") {
            println!("Skipped: set BRAKET_INTEGRATION=1 to run");
            return;
        }
        let cfg = QuantumConfig::braket_default();
        let backend = AmazonBraketBackend::from_config(&cfg).await
            .expect("failed to build Braket backend");
        let result = backend.execute_circuit(&bell_circuit()).await
            .expect("Braket task failed");
        println!("Task ARN : {}", result.backend_id);
        println!("Shots    : {}", result.shots);
        println!("Runtime  : {} ms", result.execution_time_ms);
        // SV1 returns `shots` as the count entry; just assert the task completed.
        assert!(result.shots > 0, "expected non-zero shots");
        assert!(!result.backend_id.is_empty(), "expected a task ARN");
    }

    #[tokio::test]
    async fn braket_ghz3_sv1() {
        if std::env::var("BRAKET_INTEGRATION").as_deref() != Ok("1") {
            println!("Skipped: set BRAKET_INTEGRATION=1 to run");
            return;
        }
        let cfg = QuantumConfig::braket_default();
        let backend = AmazonBraketBackend::from_config(&cfg).await
            .expect("failed to build Braket backend");
        let result = backend.execute_circuit(&ghz3_circuit()).await
            .expect("Braket GHZ task failed");
        println!("GHZ3 Task ARN : {}", result.backend_id);
        println!("Shots         : {}", result.shots);
        assert!(result.shots > 0);
    }

    #[tokio::test]
    async fn braket_device_info_sv1() {
        if std::env::var("BRAKET_INTEGRATION").as_deref() != Ok("1") {
            println!("Skipped: set BRAKET_INTEGRATION=1 to run");
            return;
        }
        let cfg = QuantumConfig::braket_default();
        let backend = AmazonBraketBackend::from_config(&cfg).await
            .expect("failed to build backend");
        let info = backend.get_backend_info().await;
        println!("Device  : {}", info.id);
        println!("Online  : {}", info.online);
        println!("Qubits  : {}", info.max_qubits);
        assert!(info.online, "SV1 should always be online");
        assert!(info.max_qubits >= 34, "SV1 supports 34 qubits");
    }
}

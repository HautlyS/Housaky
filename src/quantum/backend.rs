use super::circuit::{GateType, MeasurementResult, NoiseModel, QuantumCircuit};
use anyhow::Result;
use async_trait::async_trait;
use aws_sdk_braket::types::QuantumTaskStatus;
use aws_sdk_braket::Client;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// Amazon Braket quantum backend (aws-sdk-braket v1.x).
///
/// Results are stored in S3 after task completion; this backend polls
/// GetQuantumTask until COMPLETED/FAILED, then returns a synthetic
/// MeasurementResult derived from the task metadata (shot count, etc.).
/// For full result retrieval, configure `s3_bucket` + `s3_prefix` and
/// read `output_s3_directory` from the completed task output.
pub struct AmazonBraketBackend {
    client: Client,
    device_arn: String,
    shots: i64,
    max_qubits: usize,
    /// S3 bucket where Braket writes result files (required by the API).
    s3_bucket: String,
    /// S3 key prefix for result files.
    s3_prefix: String,
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

        let max_qubits = match device_arn.as_str() {
            s if s.contains("Aspen-M-1") => 31,
            s if s.contains("Aspen-11") => 19,
            s if s.contains("IonQ") => 11,
            s if s.contains("SV1") => 34,
            s if s.contains("TN1") => 28,
            _ => 32,
        };

        Ok(Self {
            client,
            device_arn,
            shots: shots as i64,
            max_qubits,
            s3_bucket,
            s3_prefix,
        })
    }

    pub fn with_shots(mut self, shots: u64) -> Self {
        self.shots = shots as i64;
        self
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
    /// Braket stores results in S3; on COMPLETED we return the shot count as
    /// a synthetic result. The caller can read `output_s3_directory` from the
    /// task to fetch the full result JSON from S3 if needed.
    async fn wait_for_task(&self, task_arn: &str) -> Result<MeasurementResult> {
        let start = std::time::Instant::now();

        loop {
            let output = self.client
                .get_quantum_task()
                .quantum_task_arn(task_arn)
                .send()
                .await?;

            // status() returns &QuantumTaskStatus (not Option) in v1.x
            match output.status() {
                QuantumTaskStatus::Completed => {
                    let shots = output.shots() as u64;
                    // Build a placeholder result — full counts require S3 read.
                    // Returning a single "completed" entry lets callers detect success.
                    let mut counts = HashMap::new();
                    counts.insert("completed".to_string(), shots);
                    return Ok(MeasurementResult {
                        counts,
                        shots,
                        raw_bitstrings: vec![],
                        execution_time_ms: start.elapsed().as_millis() as u64,
                        backend_id: task_arn.to_string(),
                    });
                }
                QuantumTaskStatus::Failed => {
                    let reason = output.failure_reason().unwrap_or("unknown");
                    anyhow::bail!("Braket task failed: {}", reason);
                }
                QuantumTaskStatus::Cancelled | QuantumTaskStatus::Cancelling => {
                    anyhow::bail!("Braket task was cancelled");
                }
                _ => {
                    // QUEUED / CREATED / RUNNING — keep polling
                    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                }
            }

            if start.elapsed().as_secs() > 300 {
                anyhow::bail!("Braket task timed out after 300 s (ARN: {})", task_arn);
            }
        }
    }
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

        self.wait_for_task(&output.quantum_task_arn).await
    }

    async fn get_backend_info(&self) -> BackendInfo {
        let ok = self.client
            .get_device()
            .device_arn(&self.device_arn)
            .send()
            .await
            .is_ok();

        BackendInfo {
            id: self.device_arn.clone(),
            backend_type: BackendType::AmazonBraket,
            max_qubits: self.max_qubits,
            max_shots: 100_000,
            supported_gates: vec![
                "H", "X", "Y", "Z", "CNOT", "CZ", "Rx", "Ry", "Rz",
                "S", "T", "Swap", "iSwap", "PSwap",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
            noise_model: None,
            online: ok,
            queue_depth: 0,
            avg_job_time_ms: if ok { 5000 } else { 0 },
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
        let backend = AmazonBraketBackend {
            client: {
                let sdk_cfg = aws_config::SdkConfig::builder().build();
                Client::new(&sdk_cfg)
            },
            device_arn: cfg.braket_device_arn.clone(),
            shots: cfg.shots as i64,
            max_qubits: 34,
            s3_bucket: cfg.braket_s3_bucket.clone(),
            s3_prefix: cfg.braket_s3_prefix.clone(),
        };
        let json = backend.circuit_to_action_json(&bell_circuit());
        assert!(json.contains("braket.ir.openqasm.program"), "missing schema header");
        assert!(json.contains("OPENQASM"), "missing OPENQASM source");
        assert!(json.contains("cnot"), "missing CNOT gate");
    }

    // ── Amazon Braket integration tests (require BRAKET_INTEGRATION=1) ────────
    //
    // Run with:
    //   BRAKET_INTEGRATION=1 cargo test braket_ -- --nocapture --ignored
    //
    // These tests actually submit tasks to SV1 and write results to S3.

    #[tokio::test]
    #[ignore = "requires BRAKET_INTEGRATION=1 and live AWS credentials"]
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
    #[ignore = "requires BRAKET_INTEGRATION=1 and live AWS credentials"]
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
    #[ignore = "requires BRAKET_INTEGRATION=1 and live AWS credentials"]
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

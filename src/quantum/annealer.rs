use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsingModel {
    pub num_variables: usize,
    pub linear: HashMap<usize, f64>,
    pub quadratic: HashMap<(usize, usize), f64>,
    pub offset: f64,
}

impl IsingModel {
    pub fn new(num_variables: usize) -> Self {
        Self {
            num_variables,
            linear: HashMap::new(),
            quadratic: HashMap::new(),
            offset: 0.0,
        }
    }

    pub fn add_linear(&mut self, i: usize, h: f64) {
        *self.linear.entry(i).or_insert(0.0) += h;
    }

    pub fn add_quadratic(&mut self, i: usize, j: usize, j_ij: f64) {
        let key = if i < j { (i, j) } else { (j, i) };
        *self.quadratic.entry(key).or_insert(0.0) += j_ij;
    }

    pub fn energy(&self, spins: &[i8]) -> f64 {
        let mut e = self.offset;
        for (&i, &h) in &self.linear {
            if i < spins.len() {
                e += h * spins[i] as f64;
            }
        }
        for (&(i, j), &j_ij) in &self.quadratic {
            if i < spins.len() && j < spins.len() {
                e += j_ij * spins[i] as f64 * spins[j] as f64;
            }
        }
        e
    }

    pub fn from_qubo(qubo: &HashMap<(usize, usize), f64>, num_variables: usize) -> Self {
        let mut model = Self::new(num_variables);
        for (&(i, j), &q) in qubo {
            if i == j {
                model.add_linear(i, q / 2.0);
                model.offset += q / 2.0;
            } else {
                model.add_quadratic(i, j, q / 4.0);
                model.add_linear(i, q / 4.0);
                model.add_linear(j, q / 4.0);
                model.offset += q / 4.0;
            }
        }
        model
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnealingConfig {
    pub initial_temperature: f64,
    pub final_temperature: f64,
    pub steps: usize,
    pub num_reads: usize,
    pub annealing_schedule: AnnealingSchedule,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnnealingSchedule {
    Linear,
    Geometric,
    Custom(Vec<f64>),
}

impl Default for AnnealingConfig {
    fn default() -> Self {
        Self {
            initial_temperature: 10.0,
            final_temperature: 0.01,
            steps: 1000,
            num_reads: 100,
            annealing_schedule: AnnealingSchedule::Geometric,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnealingResult {
    pub best_spins: Vec<i8>,
    pub best_energy: f64,
    pub all_reads: Vec<AnnealingRead>,
    pub runtime_ms: u64,
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnealingRead {
    pub spins: Vec<i8>,
    pub energy: f64,
    pub occurrence: u32,
}

pub struct QuantumAnnealer {
    pub config: AnnealingConfig,
}

impl QuantumAnnealer {
    pub fn new(config: AnnealingConfig) -> Self {
        Self { config }
    }

    pub async fn anneal(&self, model: &IsingModel) -> Result<AnnealingResult> {
        let start = std::time::Instant::now();
        let mut all_reads: Vec<AnnealingRead> = Vec::new();
        let mut best_spins = vec![1i8; model.num_variables];
        let mut best_energy = model.energy(&best_spins);

        for _ in 0..self.config.num_reads {
            let spins = self.simulated_annealing(model).await;
            let energy = model.energy(&spins);

            if energy < best_energy {
                best_energy = energy;
                best_spins = spins.clone();
            }

            if let Some(existing) = all_reads.iter_mut().find(|r| r.spins == spins) {
                existing.occurrence += 1;
            } else {
                all_reads.push(AnnealingRead { spins, energy, occurrence: 1 });
            }
        }

        all_reads.sort_by(|a, b| a.energy.partial_cmp(&b.energy).unwrap_or(std::cmp::Ordering::Equal));

        Ok(AnnealingResult {
            best_spins,
            best_energy,
            all_reads,
            runtime_ms: start.elapsed().as_millis() as u64,
            method: "simulated-quantum-annealing".to_string(),
        })
    }

    async fn simulated_annealing(&self, model: &IsingModel) -> Vec<i8> {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let mut spins: Vec<i8> = (0..model.num_variables)
            .map(|_| if rng.gen_bool(0.5) { 1 } else { -1 })
            .collect();

        let mut current_energy = model.energy(&spins);
        let temperatures = self.temperature_schedule();

        for &temp in &temperatures {
            let flip_idx = rng.gen_range(0..model.num_variables.max(1));
            spins[flip_idx] = -spins[flip_idx];
            let new_energy = model.energy(&spins);
            let delta_e = new_energy - current_energy;

            if delta_e < 0.0 || rng.gen::<f64>() < (-delta_e / temp).exp() {
                current_energy = new_energy;
            } else {
                spins[flip_idx] = -spins[flip_idx];
            }
        }

        spins
    }

    fn temperature_schedule(&self) -> Vec<f64> {
        let t_i = self.config.initial_temperature;
        let t_f = self.config.final_temperature;
        let n = self.config.steps;

        match &self.config.annealing_schedule {
            AnnealingSchedule::Linear => {
                (0..n).map(|k| t_i + (t_f - t_i) * k as f64 / n as f64).collect()
            }
            AnnealingSchedule::Geometric => {
                let ratio = (t_f / t_i).powf(1.0 / n as f64);
                let mut temps = Vec::with_capacity(n);
                let mut t = t_i;
                for _ in 0..n {
                    temps.push(t);
                    t *= ratio;
                }
                temps
            }
            AnnealingSchedule::Custom(schedule) => schedule.clone(),
        }
    }

    pub async fn solve_knowledge_graph_inference(
        &self,
        entities: &[String],
        relations: &HashMap<(usize, usize), f64>,
    ) -> Result<AnnealingResult> {
        let n = entities.len();
        let mut model = IsingModel::new(n);

        for (&(i, j), &strength) in relations {
            model.add_quadratic(i, j, -strength);
        }

        for i in 0..n {
            model.add_linear(i, -0.1);
        }

        self.anneal(&model).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_annealing() {
        let mut model = IsingModel::new(4);
        model.add_linear(0, -1.0);
        model.add_linear(1, -1.0);
        model.add_quadratic(0, 1, -2.0);

        let annealer = QuantumAnnealer::new(AnnealingConfig {
            steps: 200,
            num_reads: 10,
            ..Default::default()
        });

        let result = annealer.anneal(&model).await.unwrap();
        assert_eq!(result.best_spins.len(), 4);
        assert!(result.best_energy <= 0.0);
    }

    #[test]
    fn test_ising_energy() {
        let mut model = IsingModel::new(2);
        model.add_linear(0, 1.0);
        model.add_linear(1, -1.0);
        model.add_quadratic(0, 1, -1.0);

        let spins = vec![1i8, 1i8];
        let e = model.energy(&spins);
        assert!((e - (1.0 - 1.0 - 1.0)).abs() < 1e-9);
    }
}

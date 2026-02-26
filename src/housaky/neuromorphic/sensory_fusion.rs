use super::reflex_arc::{SensorEvent, SensorType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub sensor_id: String,
    pub sensor_type: SensorType,
    pub value: f64,
    pub variance: f64,
    pub timestamp: DateTime<Utc>,
    pub weight: f64,
    pub reliability: f64,
}

impl SensorReading {
    pub fn new(sensor_id: &str, sensor_type: SensorType, value: f64, variance: f64) -> Self {
        Self {
            sensor_id: sensor_id.to_string(),
            sensor_type,
            value,
            variance,
            timestamp: Utc::now(),
            weight: 1.0,
            reliability: 1.0,
        }
    }

    pub fn from_event(event: &SensorEvent) -> Self {
        Self::new(&event.sensor_id, event.sensor_type.clone(), event.value, 0.1)
    }

    pub fn information_gain(&self) -> f64 {
        1.0 / (self.variance.max(1e-9))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusedPercept {
    pub percept_type: String,
    pub fused_value: f64,
    pub confidence: f64,
    pub contributing_sensors: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub fusion_method: FusionMethod,
    pub anomaly_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FusionMethod {
    WeightedAverage,
    KalmanFilter,
    BayesianFusion,
    MaximumLikelihood,
    AttentionWeighted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KalmanState {
    pub estimate: f64,
    pub error_covariance: f64,
    pub process_noise: f64,
    pub measurement_noise: f64,
}

impl KalmanState {
    pub fn new(initial: f64, process_noise: f64, measurement_noise: f64) -> Self {
        Self {
            estimate: initial,
            error_covariance: 1.0,
            process_noise,
            measurement_noise,
        }
    }

    pub fn update(&mut self, measurement: f64) -> f64 {
        self.error_covariance += self.process_noise;
        let kalman_gain = self.error_covariance / (self.error_covariance + self.measurement_noise);
        self.estimate += kalman_gain * (measurement - self.estimate);
        self.error_covariance *= 1.0 - kalman_gain;
        self.estimate
    }
}

pub struct SensoryFusionEngine {
    pub sensor_buffers: Arc<RwLock<HashMap<String, VecDeque<SensorReading>>>>,
    pub kalman_states: Arc<RwLock<HashMap<String, KalmanState>>>,
    pub attention_weights: Arc<RwLock<HashMap<String, f64>>>,
    pub fused_percepts: Arc<RwLock<VecDeque<FusedPercept>>>,
    pub sensor_groups: Arc<RwLock<HashMap<String, Vec<String>>>>,
    pub buffer_size: usize,
    pub max_percept_history: usize,
}

impl SensoryFusionEngine {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            sensor_buffers: Arc::new(RwLock::new(HashMap::new())),
            kalman_states: Arc::new(RwLock::new(HashMap::new())),
            attention_weights: Arc::new(RwLock::new(HashMap::new())),
            fused_percepts: Arc::new(RwLock::new(VecDeque::new())),
            sensor_groups: Arc::new(RwLock::new(HashMap::new())),
            buffer_size,
            max_percept_history: 1000,
        }
    }

    pub async fn register_sensor_group(&self, group_name: &str, sensor_ids: Vec<String>) {
        self.sensor_groups.write().await.insert(group_name.to_string(), sensor_ids);
    }

    pub async fn set_attention_weight(&self, sensor_id: &str, weight: f64) {
        self.attention_weights.write().await.insert(sensor_id.to_string(), weight.clamp(0.0, 10.0));
    }

    pub async fn ingest(&self, reading: SensorReading) {
        let mut buffers = self.sensor_buffers.write().await;
        let buf = buffers.entry(reading.sensor_id.clone()).or_default();
        if buf.len() >= self.buffer_size {
            buf.pop_front();
        }
        buf.push_back(reading);
    }

    pub async fn ingest_event(&self, event: &SensorEvent) {
        self.ingest(SensorReading::from_event(event)).await;
    }

    pub async fn fuse_group(&self, group_name: &str, method: FusionMethod) -> Option<FusedPercept> {
        let groups = self.sensor_groups.read().await;
        let sensor_ids = groups.get(group_name)?.clone();
        drop(groups);

        let buffers = self.sensor_buffers.read().await;
        let weights = self.attention_weights.read().await;

        let mut readings: Vec<(SensorReading, f64)> = Vec::new();
        for sid in &sensor_ids {
            if let Some(buf) = buffers.get(sid) {
                if let Some(latest) = buf.back() {
                    let w = weights.get(sid).copied().unwrap_or(1.0);
                    readings.push((latest.clone(), w));
                }
            }
        }
        drop(buffers);
        drop(weights);

        if readings.is_empty() {
            return None;
        }

        let percept = match method {
            FusionMethod::WeightedAverage => self.weighted_average(group_name, &readings),
            FusionMethod::KalmanFilter => self.kalman_fuse(group_name, &readings).await,
            FusionMethod::BayesianFusion => self.bayesian_fuse(group_name, &readings),
            FusionMethod::MaximumLikelihood => self.maximum_likelihood(group_name, &readings),
            FusionMethod::AttentionWeighted => self.attention_weighted(group_name, &readings),
        };

        let mut fused = self.fused_percepts.write().await;
        if fused.len() >= self.max_percept_history {
            fused.pop_front();
        }
        fused.push_back(percept.clone());
        debug!("SensoryFusion: fused '{}' from {} sensors, confidence={:.3}", group_name, readings.len(), percept.confidence);

        Some(percept)
    }

    fn weighted_average(&self, group: &str, readings: &[(SensorReading, f64)]) -> FusedPercept {
        let total_weight: f64 = readings.iter().map(|(_, w)| w).sum();
        let fused_value: f64 = readings.iter()
            .map(|(r, w)| r.value * w)
            .sum::<f64>()
            / total_weight.max(1e-9);

        let variance: f64 = readings.iter()
            .map(|(r, w)| w * (r.value - fused_value).powi(2))
            .sum::<f64>()
            / total_weight.max(1e-9);

        let confidence = 1.0 / (1.0 + variance.sqrt());
        let anomaly = self.compute_anomaly_score(readings, fused_value);

        FusedPercept {
            percept_type: group.to_string(),
            fused_value,
            confidence,
            contributing_sensors: readings.iter().map(|(r, _)| r.sensor_id.clone()).collect(),
            timestamp: Utc::now(),
            fusion_method: FusionMethod::WeightedAverage,
            anomaly_score: anomaly,
        }
    }

    async fn kalman_fuse(&self, group: &str, readings: &[(SensorReading, f64)]) -> FusedPercept {
        let avg_value: f64 = readings.iter().map(|(r, _)| r.value).sum::<f64>()
            / readings.len() as f64;
        let avg_variance: f64 = readings.iter().map(|(r, _)| r.variance).sum::<f64>()
            / readings.len() as f64;

        let mut kalman_states = self.kalman_states.write().await;
        let state = kalman_states
            .entry(group.to_string())
            .or_insert_with(|| KalmanState::new(avg_value, 0.01, avg_variance.max(0.01)));

        let fused_value = state.update(avg_value);
        let confidence = 1.0 - state.error_covariance.sqrt().min(1.0);
        let anomaly = self.compute_anomaly_score(readings, fused_value);

        FusedPercept {
            percept_type: group.to_string(),
            fused_value,
            confidence: confidence.max(0.0),
            contributing_sensors: readings.iter().map(|(r, _)| r.sensor_id.clone()).collect(),
            timestamp: Utc::now(),
            fusion_method: FusionMethod::KalmanFilter,
            anomaly_score: anomaly,
        }
    }

    fn bayesian_fuse(&self, group: &str, readings: &[(SensorReading, f64)]) -> FusedPercept {
        let mut posterior_mean = 0.0f64;
        let mut posterior_precision = 0.0f64;

        for (r, w) in readings {
            let precision = w / r.variance.max(1e-9);
            posterior_mean += precision * r.value;
            posterior_precision += precision;
        }

        let fused_value = if posterior_precision > 1e-9 {
            posterior_mean / posterior_precision
        } else {
            readings.first().map(|(r, _)| r.value).unwrap_or(0.0)
        };

        let confidence = (1.0 - 1.0 / (1.0 + posterior_precision)).min(1.0);
        let anomaly = self.compute_anomaly_score(readings, fused_value);

        FusedPercept {
            percept_type: group.to_string(),
            fused_value,
            confidence,
            contributing_sensors: readings.iter().map(|(r, _)| r.sensor_id.clone()).collect(),
            timestamp: Utc::now(),
            fusion_method: FusionMethod::BayesianFusion,
            anomaly_score: anomaly,
        }
    }

    fn maximum_likelihood(&self, group: &str, readings: &[(SensorReading, f64)]) -> FusedPercept {
        let best = readings.iter()
            .max_by(|(a, wa), (b, wb)| {
                let score_a = wa * a.reliability / a.variance.max(1e-9);
                let score_b = wb * b.reliability / b.variance.max(1e-9);
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            });

        let fused_value = best.map(|(r, _)| r.value).unwrap_or(0.0);
        let confidence = best.map(|(r, w)| (w * r.reliability).min(1.0)).unwrap_or(0.0);
        let anomaly = self.compute_anomaly_score(readings, fused_value);

        FusedPercept {
            percept_type: group.to_string(),
            fused_value,
            confidence,
            contributing_sensors: readings.iter().map(|(r, _)| r.sensor_id.clone()).collect(),
            timestamp: Utc::now(),
            fusion_method: FusionMethod::MaximumLikelihood,
            anomaly_score: anomaly,
        }
    }

    fn attention_weighted(&self, group: &str, readings: &[(SensorReading, f64)]) -> FusedPercept {
        let softmax_weights: Vec<f64> = {
            let exps: Vec<f64> = readings.iter().map(|(_, w)| w.exp()).collect();
            let sum: f64 = exps.iter().sum();
            exps.iter().map(|e| e / sum.max(1e-9)).collect()
        };

        let fused_value: f64 = readings.iter().zip(softmax_weights.iter())
            .map(|((r, _), sw)| r.value * sw)
            .sum();

        let confidence: f64 = readings.iter().zip(softmax_weights.iter())
            .map(|((r, _), sw)| r.reliability * sw)
            .sum();

        let anomaly = self.compute_anomaly_score(readings, fused_value);

        FusedPercept {
            percept_type: group.to_string(),
            fused_value,
            confidence: confidence.min(1.0),
            contributing_sensors: readings.iter().map(|(r, _)| r.sensor_id.clone()).collect(),
            timestamp: Utc::now(),
            fusion_method: FusionMethod::AttentionWeighted,
            anomaly_score: anomaly,
        }
    }

    fn compute_anomaly_score(&self, readings: &[(SensorReading, f64)], fused: f64) -> f64 {
        if readings.is_empty() {
            return 0.0;
        }
        let max_dev = readings.iter()
            .map(|(r, _)| (r.value - fused).abs() / r.variance.sqrt().max(1e-9))
            .fold(0.0f64, f64::max);
        (1.0 - (-max_dev / 3.0).exp()).min(1.0)
    }

    pub async fn latest_percept(&self, group_name: &str) -> Option<FusedPercept> {
        self.fused_percepts.read().await
            .iter()
            .rev()
            .find(|p| p.percept_type == group_name)
            .cloned()
    }

    pub async fn anomalies(&self, threshold: f64) -> Vec<FusedPercept> {
        self.fused_percepts.read().await
            .iter()
            .filter(|p| p.anomaly_score > threshold)
            .cloned()
            .collect()
    }

    pub async fn sensor_count(&self) -> usize {
        self.sensor_buffers.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_weighted_average_fusion() {
        let engine = SensoryFusionEngine::new(100);
        engine.register_sensor_group("temperature", vec!["t1".into(), "t2".into()]).await;

        engine.ingest(SensorReading::new("t1", SensorType::Temperature, 22.5, 0.1)).await;
        engine.ingest(SensorReading::new("t2", SensorType::Temperature, 23.5, 0.2)).await;

        let percept = engine.fuse_group("temperature", FusionMethod::WeightedAverage).await.unwrap();
        assert!(percept.fused_value > 22.0 && percept.fused_value < 24.0);
        assert!(percept.confidence > 0.0);
        assert_eq!(percept.contributing_sensors.len(), 2);
    }

    #[tokio::test]
    async fn test_kalman_filter_fusion() {
        let engine = SensoryFusionEngine::new(50);
        engine.register_sensor_group("motion", vec!["m1".into()]).await;

        for i in 0..5 {
            engine.ingest(SensorReading::new("m1", SensorType::Motion, 10.0 + i as f64 * 0.1, 0.5)).await;
            engine.fuse_group("motion", FusionMethod::KalmanFilter).await;
        }

        let percept = engine.latest_percept("motion").await.unwrap();
        assert!(percept.fused_value > 9.0 && percept.fused_value < 12.0);
    }

    #[tokio::test]
    async fn test_anomaly_detection() {
        let engine = SensoryFusionEngine::new(100);
        engine.register_sensor_group("pressure", vec!["p1".into(), "p2".into()]).await;

        engine.ingest(SensorReading::new("p1", SensorType::Pressure, 1.0, 0.01)).await;
        engine.ingest(SensorReading::new("p2", SensorType::Pressure, 100.0, 0.01)).await;

        let percept = engine.fuse_group("pressure", FusionMethod::BayesianFusion).await.unwrap();
        assert!(percept.anomaly_score > 0.0);
    }
}

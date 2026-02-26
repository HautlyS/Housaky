use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChemicalSensorType {
    MOX,
    Electrochemical,
    Photoionization,
    InfraredGas,
    AirQuality,
    VOCDetector,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemicalSensorConfig {
    pub id: String,
    pub sensor_type: ChemicalSensorType,
    pub target_gases: Vec<String>,
    pub ppm_range: (f64, f64),
    pub warm_up_time_s: u64,
    pub sample_rate_hz: f64,
    pub location: String,
}

impl Default for ChemicalSensorConfig {
    fn default() -> Self {
        Self {
            id: "aq_sensor_0".to_string(),
            sensor_type: ChemicalSensorType::AirQuality,
            target_gases: vec!["CO2".to_string(), "VOC".to_string(), "CO".to_string()],
            ppm_range: (0.0, 10_000.0),
            warm_up_time_s: 30,
            sample_rate_hz: 1.0,
            location: "environment".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasReading {
    pub sensor_id: String,
    pub gas_name: String,
    pub concentration_ppm: f64,
    pub normalized: f64,
    pub reliability: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirQualityIndex {
    pub overall: f64,
    pub category: AQICategory,
    pub dominant_pollutant: Option<String>,
    pub components: HashMap<String, f64>,
    pub timestamp: DateTime<Utc>,
}

impl AirQualityIndex {
    pub fn compute(components: HashMap<String, f64>) -> Self {
        let overall = components.values().cloned().fold(0.0_f64, f64::max);
        let category = AQICategory::from_normalized(overall);
        let dominant_pollutant = components
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, _)| k.clone());

        Self {
            overall,
            category,
            dominant_pollutant,
            components,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AQICategory {
    Good,
    Moderate,
    UnhealthyForSensitiveGroups,
    Unhealthy,
    VeryUnhealthy,
    Hazardous,
}

impl AQICategory {
    pub fn from_normalized(value: f64) -> Self {
        match value as u32 {
            0..=20 => AQICategory::Good,
            21..=40 => AQICategory::Moderate,
            41..=60 => AQICategory::UnhealthyForSensitiveGroups,
            61..=80 => AQICategory::Unhealthy,
            81..=90 => AQICategory::VeryUnhealthy,
            _ => AQICategory::Hazardous,
        }
    }

    pub fn is_safe(&self) -> bool {
        matches!(self, AQICategory::Good | AQICategory::Moderate)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OdorProfile {
    pub id: String,
    pub name: String,
    pub gas_signature: HashMap<String, f64>,
    pub confidence_threshold: f64,
    pub hazard_level: HazardLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HazardLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl HazardLevel {
    pub fn from_concentration_ratio(ratio: f64) -> Self {
        match (ratio * 100.0) as u32 {
            0..=10 => HazardLevel::None,
            11..=30 => HazardLevel::Low,
            31..=60 => HazardLevel::Medium,
            61..=85 => HazardLevel::High,
            _ => HazardLevel::Critical,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OdorClassification {
    pub id: String,
    pub matched_profile: Option<String>,
    pub confidence: f64,
    pub hazard_level: HazardLevel,
    pub raw_readings: Vec<GasReading>,
    pub aqi: AirQualityIndex,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasAlarm {
    pub id: String,
    pub sensor_id: String,
    pub gas_name: String,
    pub concentration_ppm: f64,
    pub threshold_ppm: f64,
    pub hazard_level: HazardLevel,
    pub timestamp: DateTime<Utc>,
    pub acknowledged: bool,
}

pub struct OlfactoryProcessor {
    pub sensors: Arc<RwLock<HashMap<String, ChemicalSensorConfig>>>,
    pub reading_history: Arc<RwLock<HashMap<String, VecDeque<GasReading>>>>,
    pub aqi_history: Arc<RwLock<VecDeque<AirQualityIndex>>>,
    pub odor_profiles: Arc<RwLock<Vec<OdorProfile>>>,
    pub active_alarms: Arc<RwLock<Vec<GasAlarm>>>,
    pub classification_history: Arc<RwLock<VecDeque<OdorClassification>>>,
    pub alarm_thresholds: Arc<RwLock<HashMap<String, f64>>>,
    pub max_history: usize,
    pub baseline: Arc<RwLock<HashMap<String, f64>>>,
}

impl OlfactoryProcessor {
    pub fn new() -> Self {
        let mut alarm_thresholds = HashMap::new();
        alarm_thresholds.insert("CO".to_string(), 50.0);
        alarm_thresholds.insert("CO2".to_string(), 5_000.0);
        alarm_thresholds.insert("CH4".to_string(), 1_000.0);
        alarm_thresholds.insert("H2S".to_string(), 10.0);
        alarm_thresholds.insert("VOC".to_string(), 500.0);
        alarm_thresholds.insert("NO2".to_string(), 3.0);
        alarm_thresholds.insert("O3".to_string(), 0.1);

        let mut processor = Self {
            sensors: Arc::new(RwLock::new(HashMap::new())),
            reading_history: Arc::new(RwLock::new(HashMap::new())),
            aqi_history: Arc::new(RwLock::new(VecDeque::new())),
            odor_profiles: Arc::new(RwLock::new(Vec::new())),
            active_alarms: Arc::new(RwLock::new(Vec::new())),
            classification_history: Arc::new(RwLock::new(VecDeque::new())),
            alarm_thresholds: Arc::new(RwLock::new(alarm_thresholds)),
            max_history: 500,
            baseline: Arc::new(RwLock::new(HashMap::new())),
        };

        processor.load_default_profiles();
        processor
    }

    fn load_default_profiles(&mut self) {
        // Profiles are added via register_odor_profile at runtime.
        // We store them as static knowledge here for reference.
    }

    pub async fn register_sensor(&self, config: ChemicalSensorConfig) {
        info!("Registering chemical sensor '{}' ({:?})", config.id, config.sensor_type);
        let id = config.id.clone();
        self.sensors.write().await.insert(id.clone(), config);
        self.reading_history
            .write()
            .await
            .entry(id)
            .or_insert_with(VecDeque::new);
    }

    pub async fn register_odor_profile(&self, profile: OdorProfile) {
        info!("Registered odor profile '{}'", profile.name);
        self.odor_profiles.write().await.push(profile);
    }

    pub async fn set_alarm_threshold(&self, gas: impl Into<String>, threshold_ppm: f64) {
        self.alarm_thresholds
            .write()
            .await
            .insert(gas.into(), threshold_ppm);
    }

    pub async fn process_reading(
        &self,
        sensor_id: &str,
        gas_name: &str,
        concentration_ppm: f64,
    ) -> Result<GasReading> {
        let sensors = self.sensors.read().await;
        let config = sensors
            .get(sensor_id)
            .ok_or_else(|| anyhow::anyhow!("Sensor '{}' not registered", sensor_id))?;

        let (min_ppm, max_ppm) = config.ppm_range;
        let normalized = ((concentration_ppm - min_ppm) / (max_ppm - min_ppm)).clamp(0.0, 1.0);
        drop(sensors);

        let reading = GasReading {
            sensor_id: sensor_id.to_string(),
            gas_name: gas_name.to_string(),
            concentration_ppm,
            normalized,
            reliability: 0.95,
            timestamp: Utc::now(),
        };

        // Check alarm threshold
        let threshold = self
            .alarm_thresholds
            .read()
            .await
            .get(gas_name)
            .copied()
            .unwrap_or(f64::MAX);

        if concentration_ppm > threshold {
            warn!(
                "Gas alarm: {} = {:.2} ppm (threshold {:.2} ppm) on sensor '{}'",
                gas_name, concentration_ppm, threshold, sensor_id
            );
            let hazard = HazardLevel::from_concentration_ratio(concentration_ppm / threshold);
            self.trigger_alarm(sensor_id, gas_name, concentration_ppm, threshold, hazard)
                .await;
        }

        // Store reading
        let mut history = self.reading_history.write().await;
        let sensor_hist = history
            .entry(sensor_id.to_string())
            .or_insert_with(VecDeque::new);
        if sensor_hist.len() >= self.max_history {
            sensor_hist.pop_front();
        }
        sensor_hist.push_back(reading.clone());

        // Update baseline (slow exponential moving average)
        let mut baseline = self.baseline.write().await;
        let current = baseline.entry(gas_name.to_string()).or_insert(concentration_ppm);
        *current = *current * 0.99 + concentration_ppm * 0.01;

        debug!(
            "Gas reading '{}' {}: {:.2} ppm (normalized={:.3})",
            sensor_id, gas_name, concentration_ppm, normalized
        );

        Ok(reading)
    }

    pub async fn process_multi_gas(
        &self,
        sensor_id: &str,
        readings: HashMap<String, f64>,
    ) -> Result<OdorClassification> {
        let mut gas_readings = Vec::new();
        let mut components = HashMap::new();

        for (gas, ppm) in &readings {
            let reading = self.process_reading(sensor_id, gas, *ppm).await?;
            components.insert(gas.clone(), reading.normalized);
            gas_readings.push(reading);
        }

        let aqi = AirQualityIndex::compute(components);
        let (matched_profile, confidence) = self.classify_odor(&gas_readings).await;

        let hazard_level = self
            .active_alarms
            .read()
            .await
            .iter()
            .map(|a| {
                match a.hazard_level {
                    HazardLevel::Critical => 4,
                    HazardLevel::High => 3,
                    HazardLevel::Medium => 2,
                    HazardLevel::Low => 1,
                    HazardLevel::None => 0,
                }
            })
            .max()
            .map(|v| match v {
                4 => HazardLevel::Critical,
                3 => HazardLevel::High,
                2 => HazardLevel::Medium,
                1 => HazardLevel::Low,
                _ => HazardLevel::None,
            })
            .unwrap_or(HazardLevel::None);

        let classification = OdorClassification {
            id: format!("odor_{}_{}", sensor_id, Utc::now().timestamp_millis()),
            matched_profile,
            confidence,
            hazard_level,
            raw_readings: gas_readings,
            aqi,
            timestamp: Utc::now(),
        };

        let mut hist = self.classification_history.write().await;
        if hist.len() >= self.max_history {
            hist.pop_front();
        }
        hist.push_back(classification.clone());

        Ok(classification)
    }

    async fn classify_odor(
        &self,
        readings: &[GasReading],
    ) -> (Option<String>, f64) {
        let profiles = self.odor_profiles.read().await;
        if profiles.is_empty() {
            return (None, 0.0);
        }

        let reading_map: HashMap<String, f64> = readings
            .iter()
            .map(|r| (r.gas_name.clone(), r.normalized))
            .collect();

        let mut best_profile: Option<String> = None;
        let mut best_score = 0.0_f64;

        for profile in profiles.iter() {
            let score = self.cosine_similarity(&reading_map, &profile.gas_signature);
            if score > best_score && score >= profile.confidence_threshold {
                best_score = score;
                best_profile = Some(profile.name.clone());
            }
        }

        (best_profile, best_score)
    }

    fn cosine_similarity(
        &self,
        a: &HashMap<String, f64>,
        b: &HashMap<String, f64>,
    ) -> f64 {
        let dot: f64 = a
            .iter()
            .filter_map(|(k, v)| b.get(k).map(|bv| v * bv))
            .sum();
        let mag_a = a.values().map(|v| v.powi(2)).sum::<f64>().sqrt();
        let mag_b = b.values().map(|v| v.powi(2)).sum::<f64>().sqrt();
        if mag_a < 1e-10 || mag_b < 1e-10 {
            0.0
        } else {
            dot / (mag_a * mag_b)
        }
    }

    async fn trigger_alarm(
        &self,
        sensor_id: &str,
        gas_name: &str,
        concentration_ppm: f64,
        threshold_ppm: f64,
        hazard_level: HazardLevel,
    ) {
        let alarm = GasAlarm {
            id: format!("alarm_{}_{}_{}", sensor_id, gas_name, Utc::now().timestamp_millis()),
            sensor_id: sensor_id.to_string(),
            gas_name: gas_name.to_string(),
            concentration_ppm,
            threshold_ppm,
            hazard_level,
            timestamp: Utc::now(),
            acknowledged: false,
        };
        self.active_alarms.write().await.push(alarm);
    }

    pub async fn acknowledge_alarm(&self, alarm_id: &str) -> bool {
        let mut alarms = self.active_alarms.write().await;
        if let Some(alarm) = alarms.iter_mut().find(|a| a.id == alarm_id) {
            alarm.acknowledged = true;
            return true;
        }
        false
    }

    pub async fn clear_acknowledged_alarms(&self) {
        self.active_alarms
            .write()
            .await
            .retain(|a| !a.acknowledged);
    }

    pub async fn get_active_alarms(&self) -> Vec<GasAlarm> {
        self.active_alarms
            .read()
            .await
            .iter()
            .filter(|a| !a.acknowledged)
            .cloned()
            .collect()
    }

    pub async fn get_latest_aqi(&self) -> Option<AirQualityIndex> {
        self.aqi_history.read().await.back().cloned()
    }

    pub async fn get_baseline(&self, gas: &str) -> Option<f64> {
        self.baseline.read().await.get(gas).copied()
    }

    pub async fn get_stats(&self) -> OlfactoryStats {
        let total_readings: usize = self
            .reading_history
            .read()
            .await
            .values()
            .map(|h| h.len())
            .sum();

        OlfactoryStats {
            sensors_registered: self.sensors.read().await.len(),
            odor_profiles: self.odor_profiles.read().await.len(),
            total_gas_readings: total_readings,
            active_alarms: self.active_alarms.read().await.len(),
            classifications: self.classification_history.read().await.len(),
        }
    }
}

impl Default for OlfactoryProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OlfactoryStats {
    pub sensors_registered: usize,
    pub odor_profiles: usize,
    pub total_gas_readings: usize,
    pub active_alarms: usize,
    pub classifications: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(id: &str) -> ChemicalSensorConfig {
        ChemicalSensorConfig {
            id: id.to_string(),
            sensor_type: ChemicalSensorType::AirQuality,
            target_gases: vec!["CO".to_string(), "CO2".to_string()],
            ppm_range: (0.0, 10_000.0),
            warm_up_time_s: 5,
            sample_rate_hz: 1.0,
            location: "lab".to_string(),
        }
    }

    #[test]
    fn test_aqi_category() {
        assert_eq!(AQICategory::from_normalized(0.1), AQICategory::Good);
        assert_eq!(AQICategory::from_normalized(0.5), AQICategory::Good);
        assert!(AQICategory::Good.is_safe());
        assert!(!AQICategory::Hazardous.is_safe());
    }

    #[test]
    fn test_hazard_from_ratio() {
        assert_eq!(HazardLevel::from_concentration_ratio(0.05), HazardLevel::None);
        assert_eq!(HazardLevel::from_concentration_ratio(1.5), HazardLevel::Critical);
    }

    #[tokio::test]
    async fn test_process_reading_normal() {
        let proc = OlfactoryProcessor::default();
        proc.register_sensor(make_config("s1")).await;
        let reading = proc.process_reading("s1", "CO2", 400.0).await.unwrap();
        assert_eq!(reading.gas_name, "CO2");
        assert!(reading.normalized >= 0.0 && reading.normalized <= 1.0);
    }

    #[tokio::test]
    async fn test_alarm_triggered_over_threshold() {
        let proc = OlfactoryProcessor::default();
        proc.register_sensor(make_config("s1")).await;
        proc.process_reading("s1", "CO", 100.0).await.unwrap(); // threshold = 50 ppm
        let alarms = proc.get_active_alarms().await;
        assert!(!alarms.is_empty());
        assert_eq!(alarms[0].gas_name, "CO");
    }

    #[tokio::test]
    async fn test_acknowledge_alarm() {
        let proc = OlfactoryProcessor::default();
        proc.register_sensor(make_config("s1")).await;
        proc.process_reading("s1", "CO", 100.0).await.unwrap();
        let alarms = proc.get_active_alarms().await;
        let id = alarms[0].id.clone();
        let acked = proc.acknowledge_alarm(&id).await;
        assert!(acked);
        proc.clear_acknowledged_alarms().await;
        assert!(proc.get_active_alarms().await.is_empty());
    }

    #[tokio::test]
    async fn test_multi_gas_classification_no_profile() {
        let proc = OlfactoryProcessor::default();
        proc.register_sensor(make_config("s1")).await;
        let mut readings = HashMap::new();
        readings.insert("CO".to_string(), 10.0);
        readings.insert("CO2".to_string(), 400.0);
        let classification = proc.process_multi_gas("s1", readings).await.unwrap();
        assert_eq!(classification.matched_profile, None);
    }

    #[tokio::test]
    async fn test_cosine_similarity_identical() {
        let proc = OlfactoryProcessor::default();
        let mut a = HashMap::new();
        a.insert("CO".to_string(), 0.5);
        a.insert("CO2".to_string(), 0.3);
        let score = proc.cosine_similarity(&a, &a);
        assert!((score - 1.0).abs() < 1e-9);
    }

    #[tokio::test]
    async fn test_baseline_update() {
        let proc = OlfactoryProcessor::default();
        proc.register_sensor(make_config("s1")).await;
        proc.process_reading("s1", "CO2", 400.0).await.unwrap();
        let baseline = proc.get_baseline("CO2").await;
        assert!(baseline.is_some());
    }
}

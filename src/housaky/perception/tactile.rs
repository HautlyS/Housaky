use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TactileSensorType {
    Pressure,
    Force,
    Vibration,
    Temperature,
    SlipDetector,
    ProximitySkin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TactileSensorConfig {
    pub id: String,
    pub sensor_type: TactileSensorType,
    pub location: String,
    pub resolution: (u32, u32),
    pub max_pressure_kpa: f64,
    pub sample_rate_hz: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressureMap {
    pub sensor_id: String,
    pub width: u32,
    pub height: u32,
    pub cells: Vec<f64>,
    pub max_value: f64,
    pub mean_value: f64,
    pub contact_area_ratio: f64,
    pub center_of_pressure: Option<(f64, f64)>,
    pub timestamp: DateTime<Utc>,
}

impl PressureMap {
    pub fn new(sensor_id: impl Into<String>, width: u32, height: u32) -> Self {
        let n = (width * height) as usize;
        Self {
            sensor_id: sensor_id.into(),
            width,
            height,
            cells: vec![0.0; n],
            max_value: 0.0,
            mean_value: 0.0,
            contact_area_ratio: 0.0,
            center_of_pressure: None,
            timestamp: Utc::now(),
        }
    }

    pub fn from_values(
        sensor_id: impl Into<String>,
        width: u32,
        height: u32,
        cells: Vec<f64>,
    ) -> Self {
        let n = cells.len() as f64;
        let max_value = cells.iter().cloned().fold(0.0_f64, f64::max);
        let mean_value = cells.iter().sum::<f64>() / n.max(1.0);
        let contact_count = cells.iter().filter(|&&v| v > 0.01).count();
        let contact_area_ratio = contact_count as f64 / n.max(1.0);

        let cop = if contact_count > 0 {
            let total: f64 = cells.iter().sum();
            if total > 1e-10 {
                let mut cx = 0.0_f64;
                let mut cy = 0.0_f64;
                for (i, &v) in cells.iter().enumerate() {
                    let x = (i % width as usize) as f64;
                    let y = (i / width as usize) as f64;
                    cx += x * v;
                    cy += y * v;
                }
                Some((cx / total, cy / total))
            } else {
                None
            }
        } else {
            None
        };

        Self {
            sensor_id: sensor_id.into(),
            width,
            height,
            cells,
            max_value,
            mean_value,
            contact_area_ratio,
            center_of_pressure: cop,
            timestamp: Utc::now(),
        }
    }

    pub fn is_in_contact(&self, threshold: f64) -> bool {
        self.max_value >= threshold
    }

    pub fn dominant_quadrant(&self) -> Option<&'static str> {
        let hw = self.width as usize / 2;
        let hh = self.height as usize / 2;
        let mut quadrant_sums = [0.0_f64; 4];

        for (i, &v) in self.cells.iter().enumerate() {
            let x = i % self.width as usize;
            let y = i / self.width as usize;
            let q = match (x >= hw, y >= hh) {
                (false, false) => 0,
                (true, false) => 1,
                (false, true) => 2,
                (true, true) => 3,
            };
            quadrant_sums[q] += v;
        }

        let max_q = quadrant_sums
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i);

        max_q.map(|q| match q {
            0 => "top-left",
            1 => "top-right",
            2 => "bottom-left",
            3 => "bottom-right",
            _ => "unknown",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VibrationReading {
    pub sensor_id: String,
    pub frequency_hz: f64,
    pub amplitude: f64,
    pub dominant_frequency_hz: f64,
    pub rms_velocity: f64,
    pub slip_detected: bool,
    pub timestamp: DateTime<Utc>,
}

impl VibrationReading {
    pub fn analyze(sensor_id: impl Into<String>, samples: &[f64], sample_rate_hz: f64) -> Self {
        if samples.is_empty() {
            return Self {
                sensor_id: sensor_id.into(),
                frequency_hz: 0.0,
                amplitude: 0.0,
                dominant_frequency_hz: 0.0,
                rms_velocity: 0.0,
                slip_detected: false,
                timestamp: Utc::now(),
            };
        }

        let amplitude = samples.iter().cloned().fold(0.0_f64, f64::max);
        let rms = (samples.iter().map(|&s| s.powi(2)).sum::<f64>() / samples.len() as f64).sqrt();

        // Estimate dominant frequency via zero-crossing rate
        let zcr = samples
            .windows(2)
            .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
            .count();
        let dominant_freq = (zcr as f64 / 2.0) * (sample_rate_hz / samples.len() as f64);

        // Slip detection: high-frequency vibration with low mean pressure implies slip
        let mean_abs = samples.iter().map(|&s| s.abs()).sum::<f64>() / samples.len() as f64;
        let slip_detected = dominant_freq > 100.0 && mean_abs < 0.1;

        Self {
            sensor_id: sensor_id.into(),
            frequency_hz: dominant_freq,
            amplitude,
            dominant_frequency_hz: dominant_freq,
            rms_velocity: rms,
            slip_detected,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalReading {
    pub sensor_id: String,
    pub temperature_c: f64,
    pub rate_of_change_c_per_s: f64,
    pub above_safe_threshold: bool,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TactileEvent {
    pub id: String,
    pub event_type: TactileEventType,
    pub sensor_id: String,
    pub timestamp: DateTime<Utc>,
    pub severity: f64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TactileEventType {
    Contact,
    Release,
    SlipDetected,
    OverloadForce,
    ThermalWarning,
    Vibration,
    Unknown,
}

pub struct TactileProcessor {
    pub sensors: Arc<RwLock<HashMap<String, TactileSensorConfig>>>,
    pub pressure_history: Arc<RwLock<HashMap<String, VecDeque<PressureMap>>>>,
    pub vibration_history: Arc<RwLock<VecDeque<VibrationReading>>>,
    pub thermal_history: Arc<RwLock<VecDeque<ThermalReading>>>,
    pub tactile_events: Arc<RwLock<VecDeque<TactileEvent>>>,
    pub contact_thresholds: Arc<RwLock<HashMap<String, f64>>>,
    pub force_overload_threshold_kpa: f64,
    pub thermal_safe_threshold_c: f64,
    pub max_history: usize,
}

impl TactileProcessor {
    pub fn new(force_overload_threshold_kpa: f64, thermal_safe_threshold_c: f64) -> Self {
        Self {
            sensors: Arc::new(RwLock::new(HashMap::new())),
            pressure_history: Arc::new(RwLock::new(HashMap::new())),
            vibration_history: Arc::new(RwLock::new(VecDeque::new())),
            thermal_history: Arc::new(RwLock::new(VecDeque::new())),
            tactile_events: Arc::new(RwLock::new(VecDeque::new())),
            contact_thresholds: Arc::new(RwLock::new(HashMap::new())),
            force_overload_threshold_kpa,
            thermal_safe_threshold_c,
            max_history: 200,
        }
    }

    pub async fn register_sensor(&self, config: TactileSensorConfig) {
        info!("Registering tactile sensor '{}' ({:?})", config.id, config.sensor_type);
        let id = config.id.clone();
        self.sensors.write().await.insert(id.clone(), config);
        self.pressure_history
            .write()
            .await
            .entry(id.clone())
            .or_insert_with(VecDeque::new);
        self.contact_thresholds.write().await.insert(id, 0.05);
    }

    pub async fn process_pressure(
        &self,
        sensor_id: &str,
        raw_cells: Vec<f64>,
    ) -> Result<PressureMap> {
        let sensors = self.sensors.read().await;
        let config = sensors
            .get(sensor_id)
            .ok_or_else(|| anyhow::anyhow!("Sensor '{}' not registered", sensor_id))?;
        let (w, h) = config.resolution;
        let max_kpa = config.max_pressure_kpa;
        drop(sensors);

        // Normalize cells to [0, 1]
        let normalized: Vec<f64> = raw_cells
            .iter()
            .map(|&v| (v / max_kpa).clamp(0.0, 1.0))
            .collect();

        let map = PressureMap::from_values(sensor_id, w, h, normalized);

        let threshold = *self
            .contact_thresholds
            .read()
            .await
            .get(sensor_id)
            .unwrap_or(&0.05);

        // Detect transitions
        let prev_contact = self
            .pressure_history
            .read()
            .await
            .get(sensor_id)
            .and_then(|h| h.back())
            .map(|m| m.is_in_contact(threshold))
            .unwrap_or(false);

        let curr_contact = map.is_in_contact(threshold);

        if curr_contact != prev_contact {
            let event_type = if curr_contact {
                TactileEventType::Contact
            } else {
                TactileEventType::Release
            };
            self.emit_event(
                sensor_id,
                event_type,
                if curr_contact { map.max_value } else { 0.0 },
            )
            .await;
        }

        if map.max_value > 0.9 {
            warn!("Force overload on sensor '{}': {:.2}", sensor_id, map.max_value);
            self.emit_event(sensor_id, TactileEventType::OverloadForce, map.max_value)
                .await;
        }

        let mut history = self.pressure_history.write().await;
        let sensor_history = history.entry(sensor_id.to_string()).or_insert_with(VecDeque::new);
        if sensor_history.len() >= self.max_history {
            sensor_history.pop_front();
        }
        sensor_history.push_back(map.clone());

        debug!(
            "Pressure map '{}': contact={}, max={:.3}, CoP={:?}",
            sensor_id, curr_contact, map.max_value, map.center_of_pressure
        );

        Ok(map)
    }

    pub async fn process_vibration(
        &self,
        sensor_id: &str,
        samples: Vec<f64>,
        sample_rate_hz: f64,
    ) -> Result<VibrationReading> {
        let reading = VibrationReading::analyze(sensor_id, &samples, sample_rate_hz);

        if reading.slip_detected {
            warn!("Slip detected on sensor '{}'", sensor_id);
            self.emit_event(sensor_id, TactileEventType::SlipDetected, reading.amplitude)
                .await;
        }

        let mut history = self.vibration_history.write().await;
        if history.len() >= self.max_history {
            history.pop_front();
        }
        history.push_back(reading.clone());

        Ok(reading)
    }

    pub async fn process_thermal(
        &self,
        sensor_id: &str,
        temperature_c: f64,
    ) -> Result<ThermalReading> {
        let rate = {
            let history = self.thermal_history.read().await;
            history.back().map(|prev| {
                let dt = (Utc::now() - prev.timestamp).num_milliseconds() as f64 / 1000.0;
                if dt > 0.0 {
                    (temperature_c - prev.temperature_c) / dt
                } else {
                    0.0
                }
            }).unwrap_or(0.0)
        };

        let above_safe = temperature_c > self.thermal_safe_threshold_c;

        if above_safe {
            warn!(
                "Thermal warning on '{}': {:.1}°C > {:.1}°C",
                sensor_id, temperature_c, self.thermal_safe_threshold_c
            );
            self.emit_event(sensor_id, TactileEventType::ThermalWarning, temperature_c)
                .await;
        }

        let reading = ThermalReading {
            sensor_id: sensor_id.to_string(),
            temperature_c,
            rate_of_change_c_per_s: rate,
            above_safe_threshold: above_safe,
            timestamp: Utc::now(),
        };

        let mut history = self.thermal_history.write().await;
        if history.len() >= self.max_history {
            history.pop_front();
        }
        history.push_back(reading.clone());

        Ok(reading)
    }

    async fn emit_event(
        &self,
        sensor_id: &str,
        event_type: TactileEventType,
        severity: f64,
    ) {
        let event = TactileEvent {
            id: format!("tact_{}_{}", sensor_id, Utc::now().timestamp_millis()),
            event_type,
            sensor_id: sensor_id.to_string(),
            timestamp: Utc::now(),
            severity,
            metadata: HashMap::new(),
        };
        let mut events = self.tactile_events.write().await;
        if events.len() >= self.max_history {
            events.pop_front();
        }
        events.push_back(event);
    }

    pub async fn get_latest_pressure(&self, sensor_id: &str) -> Option<PressureMap> {
        self.pressure_history
            .read()
            .await
            .get(sensor_id)
            .and_then(|h| h.back().cloned())
    }

    pub async fn is_in_contact(&self, sensor_id: &str) -> bool {
        let threshold = *self
            .contact_thresholds
            .read()
            .await
            .get(sensor_id)
            .unwrap_or(&0.05);
        self.get_latest_pressure(sensor_id)
            .await
            .map(|m| m.is_in_contact(threshold))
            .unwrap_or(false)
    }

    pub async fn get_recent_events(&self, limit: usize) -> Vec<TactileEvent> {
        self.tactile_events
            .read()
            .await
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub async fn get_stats(&self) -> TactileStats {
        TactileStats {
            sensors_registered: self.sensors.read().await.len(),
            total_pressure_readings: self
                .pressure_history
                .read()
                .await
                .values()
                .map(|v| v.len())
                .sum(),
            vibration_readings: self.vibration_history.read().await.len(),
            thermal_readings: self.thermal_history.read().await.len(),
            events_emitted: self.tactile_events.read().await.len(),
        }
    }
}

impl Default for TactileProcessor {
    fn default() -> Self {
        Self::new(500.0, 60.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TactileStats {
    pub sensors_registered: usize,
    pub total_pressure_readings: usize,
    pub vibration_readings: usize,
    pub thermal_readings: usize,
    pub events_emitted: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(id: &str) -> TactileSensorConfig {
        TactileSensorConfig {
            id: id.to_string(),
            sensor_type: TactileSensorType::Pressure,
            location: "right_fingertip".to_string(),
            resolution: (8, 8),
            max_pressure_kpa: 200.0,
            sample_rate_hz: 100,
        }
    }

    #[test]
    fn test_pressure_map_cop() {
        let cells = {
            let mut v = vec![0.0; 64];
            v[0] = 1.0; // top-left corner
            v
        };
        let map = PressureMap::from_values("s1", 8, 8, cells);
        assert!(map.center_of_pressure.is_some());
        let (cx, cy) = map.center_of_pressure.unwrap();
        assert!((cx - 0.0).abs() < 1e-9);
        assert!((cy - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_pressure_map_contact() {
        let cells = vec![0.5; 64];
        let map = PressureMap::from_values("s1", 8, 8, cells);
        assert!(map.is_in_contact(0.1));
        assert!(!map.is_in_contact(0.9));
    }

    #[test]
    fn test_vibration_slip_detection() {
        let samples: Vec<f64> = (0..1000)
            .map(|i| 0.01 * (i as f64 * 0.628).sin()) // 100Hz vibration, low amplitude
            .collect();
        let reading = VibrationReading::analyze("vib1", &samples, 16_000.0);
        // High ZCR, low amplitude → slip
        assert!(reading.dominant_frequency_hz >= 0.0);
    }

    #[tokio::test]
    async fn test_process_pressure_contact_event() {
        let proc = TactileProcessor::default();
        proc.register_sensor(make_config("s1")).await;

        let cells = vec![100.0; 64]; // 100 kPa across all cells
        let map = proc.process_pressure("s1", cells).await.unwrap();
        assert!(map.is_in_contact(0.05));

        let events = proc.get_recent_events(10).await;
        assert!(!events.is_empty());
        assert_eq!(events[0].event_type, TactileEventType::Contact);
    }

    #[tokio::test]
    async fn test_thermal_warning() {
        let proc = TactileProcessor::new(500.0, 40.0);
        let reading = proc.process_thermal("therm1", 55.0).await.unwrap();
        assert!(reading.above_safe_threshold);

        let events = proc.get_recent_events(10).await;
        assert!(events.iter().any(|e| e.event_type == TactileEventType::ThermalWarning));
    }
}

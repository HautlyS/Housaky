use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SensorType {
    IMU,
    Lidar,
    Camera,
    Ultrasonic,
    GPS,
    Encoder,
    ForceTorque,
    Barometer,
    Magnetometer,
    Thermometer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub sensor_id: String,
    pub sensor_type: SensorType,
    pub timestamp: DateTime<Utc>,
    pub values: Vec<f64>,
    pub labels: Vec<String>,
    pub confidence: f64,
    pub noise_estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IMUData {
    pub accel_x: f64,
    pub accel_y: f64,
    pub accel_z: f64,
    pub gyro_x: f64,
    pub gyro_y: f64,
    pub gyro_z: f64,
    pub mag_x: f64,
    pub mag_y: f64,
    pub mag_z: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LidarScan {
    pub points: Vec<LidarPoint>,
    pub range_min_m: f64,
    pub range_max_m: f64,
    pub angle_min_rad: f64,
    pub angle_max_rad: f64,
    pub scan_time_ms: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LidarPoint {
    pub angle_rad: f64,
    pub distance_m: f64,
    pub intensity: f64,
    pub valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusedState {
    pub position_x: f64,
    pub position_y: f64,
    pub position_z: f64,
    pub velocity_x: f64,
    pub velocity_y: f64,
    pub velocity_z: f64,
    pub orientation_roll: f64,
    pub orientation_pitch: f64,
    pub orientation_yaw: f64,
    pub angular_velocity_x: f64,
    pub angular_velocity_y: f64,
    pub angular_velocity_z: f64,
    pub covariance: Vec<f64>,
    pub timestamp: DateTime<Utc>,
    pub sensor_sources: Vec<String>,
}

impl FusedState {
    pub fn zero() -> Self {
        Self {
            position_x: 0.0,
            position_y: 0.0,
            position_z: 0.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            velocity_z: 0.0,
            orientation_roll: 0.0,
            orientation_pitch: 0.0,
            orientation_yaw: 0.0,
            angular_velocity_x: 0.0,
            angular_velocity_y: 0.0,
            angular_velocity_z: 0.0,
            covariance: vec![1.0; 144],
            timestamp: Utc::now(),
            sensor_sources: Vec::new(),
        }
    }
}

/// Extended Kalman Filter state for 12D state vector:
/// [px, py, pz, vx, vy, vz, roll, pitch, yaw, wx, wy, wz]
pub struct KalmanFilter {
    pub state: Vec<f64>,
    pub covariance: Vec<Vec<f64>>,
    pub process_noise: Vec<Vec<f64>>,
    pub dim: usize,
}

impl KalmanFilter {
    pub fn new(dim: usize) -> Self {
        let state = vec![0.0; dim];
        let covariance = vec![vec![1.0; dim]; dim];
        let process_noise = vec![vec![0.01; dim]; dim];

        Self {
            state,
            covariance,
            process_noise,
            dim,
        }
    }

    /// Predict step: propagate state forward by dt seconds.
    pub fn predict(&mut self, dt: f64) {
        // Constant velocity model: pos += vel * dt
        if self.dim >= 6 {
            self.state[0] += self.state[3] * dt;
            self.state[1] += self.state[4] * dt;
            self.state[2] += self.state[5] * dt;
        }
        if self.dim >= 12 {
            self.state[6] += self.state[9] * dt;
            self.state[7] += self.state[10] * dt;
            self.state[8] += self.state[11] * dt;
        }

        // Add process noise to diagonal covariance
        for i in 0..self.dim {
            self.covariance[i][i] += self.process_noise[i][i];
        }
    }

    /// Update step: incorporate a measurement z with observation matrix H and noise R.
    pub fn update(&mut self, z: &[f64], h_indices: &[usize], measurement_noise: f64) {
        for (&hi, &zi) in h_indices.iter().zip(z.iter()) {
            if hi >= self.dim {
                continue;
            }
            let predicted = self.state[hi];
            let innovation = zi - predicted;
            let s = self.covariance[hi][hi] + measurement_noise;
            let k = self.covariance[hi][hi] / s;

            // State update
            self.state[hi] += k * innovation;

            // Covariance update (Joseph form simplified)
            self.covariance[hi][hi] *= 1.0 - k;
        }
    }

    pub fn get_position(&self) -> (f64, f64, f64) {
        (self.state[0], self.state[1], self.state[2])
    }

    pub fn get_velocity(&self) -> (f64, f64, f64) {
        if self.dim >= 6 {
            (self.state[3], self.state[4], self.state[5])
        } else {
            (0.0, 0.0, 0.0)
        }
    }

    pub fn get_orientation(&self) -> (f64, f64, f64) {
        if self.dim >= 9 {
            (self.state[6], self.state[7], self.state[8])
        } else {
            (0.0, 0.0, 0.0)
        }
    }
}

pub struct EmbodimentSensorFusion {
    pub kalman: Arc<RwLock<KalmanFilter>>,
    pub reading_history: Arc<RwLock<VecDeque<SensorReading>>>,
    pub fused_state: Arc<RwLock<FusedState>>,
    pub lidar_scans: Arc<RwLock<VecDeque<LidarScan>>>,
    pub imu_history: Arc<RwLock<VecDeque<IMUData>>>,
    pub max_history: usize,
    pub last_update: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl EmbodimentSensorFusion {
    pub fn new() -> Self {
        Self {
            kalman: Arc::new(RwLock::new(KalmanFilter::new(12))),
            reading_history: Arc::new(RwLock::new(VecDeque::new())),
            fused_state: Arc::new(RwLock::new(FusedState::zero())),
            lidar_scans: Arc::new(RwLock::new(VecDeque::new())),
            imu_history: Arc::new(RwLock::new(VecDeque::new())),
            max_history: 1000,
            last_update: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn update_imu(&self, data: IMUData) -> Result<()> {
        debug!("Fusing IMU reading");

        let dt = {
            let last = self.last_update.read().await;
            last.map(|t| (Utc::now() - t).num_milliseconds() as f64 / 1000.0)
                .unwrap_or(0.01)
                .clamp(0.001, 0.1)
        };

        {
            let mut kf = self.kalman.write().await;
            kf.predict(dt);

            // Update orientation from gyro integration
            let z_orient = vec![data.gyro_x * dt, data.gyro_y * dt, data.gyro_z * dt];
            kf.update(&z_orient, &[6, 7, 8], 0.01);

            // Update angular velocity
            let z_angvel = vec![data.gyro_x, data.gyro_y, data.gyro_z];
            kf.update(&z_angvel, &[9, 10, 11], 0.005);
        }

        {
            let mut history = self.imu_history.write().await;
            if history.len() >= self.max_history {
                history.pop_front();
            }
            history.push_back(data.clone());
        }

        self.sync_fused_state(vec!["imu".to_string()]).await;
        *self.last_update.write().await = Some(Utc::now());

        Ok(())
    }

    pub async fn update_lidar(&self, scan: LidarScan) -> Result<()> {
        info!("Fusing lidar scan with {} points", scan.points.len());

        // Extract nearest obstacle position as a rough position anchor
        if let Some(nearest) = scan
            .points
            .iter()
            .filter(|p| p.valid)
            .min_by(|a, b| a.distance_m.partial_cmp(&b.distance_m).unwrap())
        {
            let obstacle_x = nearest.distance_m * nearest.angle_rad.cos();
            let obstacle_y = nearest.distance_m * nearest.angle_rad.sin();
            debug!(
                "Nearest obstacle at ({:.2}, {:.2}) m",
                obstacle_x, obstacle_y
            );
        }

        {
            let mut scans = self.lidar_scans.write().await;
            if scans.len() >= 50 {
                scans.pop_front();
            }
            scans.push_back(scan);
        }

        self.sync_fused_state(vec!["lidar".to_string()]).await;
        Ok(())
    }

    pub async fn update_gps(&self, lat: f64, lon: f64, alt: f64) -> Result<()> {
        debug!("Fusing GPS: lat={:.6}, lon={:.6}, alt={:.2}", lat, lon, alt);

        // Convert lat/lon to local Cartesian (simplified flat-earth)
        let x = (lon - 0.0) * 111_320.0 * lat.to_radians().cos();
        let y = (lat - 0.0) * 110_574.0;

        let mut kf = self.kalman.write().await;
        kf.update(&[x, y, alt], &[0, 1, 2], 5.0);
        drop(kf);

        self.sync_fused_state(vec!["gps".to_string()]).await;
        Ok(())
    }

    pub async fn update_encoder(
        &self,
        delta_x: f64,
        delta_y: f64,
        delta_theta: f64,
    ) -> Result<()> {
        let mut kf = self.kalman.write().await;
        let px = kf.state[0] + delta_x;
        let py = kf.state[1] + delta_y;
        let yaw = kf.state[8] + delta_theta;
        kf.update(&[px, py], &[0, 1], 0.1);
        kf.update(&[yaw], &[8], 0.05);
        drop(kf);

        self.sync_fused_state(vec!["encoder".to_string()]).await;
        Ok(())
    }

    async fn sync_fused_state(&self, sources: Vec<String>) {
        let kf = self.kalman.read().await;
        let (px, py, pz) = kf.get_position();
        let (vx, vy, vz) = kf.get_velocity();
        let (roll, pitch, yaw) = kf.get_orientation();
        let cov: Vec<f64> = kf
            .covariance
            .iter()
            .flat_map(|row| row.iter().copied())
            .collect();
        drop(kf);

        let mut state = self.fused_state.write().await;
        state.position_x = px;
        state.position_y = py;
        state.position_z = pz;
        state.velocity_x = vx;
        state.velocity_y = vy;
        state.velocity_z = vz;
        state.orientation_roll = roll;
        state.orientation_pitch = pitch;
        state.orientation_yaw = yaw;
        state.covariance = cov;
        state.timestamp = Utc::now();

        for s in sources {
            if !state.sensor_sources.contains(&s) {
                state.sensor_sources.push(s);
            }
        }
    }

    pub async fn get_fused_state(&self) -> FusedState {
        self.fused_state.read().await.clone()
    }

    pub async fn get_latest_lidar(&self) -> Option<LidarScan> {
        self.lidar_scans.read().await.back().cloned()
    }

    pub async fn get_latest_imu(&self) -> Option<IMUData> {
        self.imu_history.read().await.back().cloned()
    }

    pub async fn position_uncertainty_m(&self) -> f64 {
        let kf = self.kalman.read().await;
        (kf.covariance[0][0] + kf.covariance[1][1] + kf.covariance[2][2]).sqrt()
    }
}

impl Default for EmbodimentSensorFusion {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_predict() {
        let mut kf = KalmanFilter::new(12);
        kf.state[3] = 1.0; // velocity x = 1 m/s
        kf.predict(1.0);
        assert!((kf.state[0] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_kalman_update() {
        let mut kf = KalmanFilter::new(12);
        kf.update(&[5.0], &[0], 1.0);
        // State should move toward measurement
        assert!(kf.state[0] > 0.0);
    }

    #[tokio::test]
    async fn test_imu_fusion() {
        let fusion = EmbodimentSensorFusion::new();
        fusion
            .update_imu(IMUData {
                accel_x: 0.0,
                accel_y: 0.0,
                accel_z: 9.81,
                gyro_x: 0.1,
                gyro_y: 0.0,
                gyro_z: 0.0,
                mag_x: 0.0,
                mag_y: 0.0,
                mag_z: 0.0,
                timestamp: Utc::now(),
            })
            .await
            .unwrap();

        let state = fusion.get_fused_state().await;
        assert!(state.sensor_sources.contains(&"imu".to_string()));
    }
}

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PIDController {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
    pub setpoint: f64,
    pub integral: f64,
    pub previous_error: f64,
    pub output_min: f64,
    pub output_max: f64,
    pub last_update: Option<DateTime<Utc>>,
}

impl PIDController {
    pub fn new(kp: f64, ki: f64, kd: f64, output_min: f64, output_max: f64) -> Self {
        Self {
            kp,
            ki,
            kd,
            setpoint: 0.0,
            integral: 0.0,
            previous_error: 0.0,
            output_min,
            output_max,
            last_update: None,
        }
    }

    pub fn compute(&mut self, measured_value: f64, dt_secs: f64) -> f64 {
        let error = self.setpoint - measured_value;

        self.integral += error * dt_secs;
        let derivative = if dt_secs > 0.0 {
            (error - self.previous_error) / dt_secs
        } else {
            0.0
        };

        let output = self.kp * error + self.ki * self.integral + self.kd * derivative;
        self.previous_error = error;
        self.last_update = Some(Utc::now());

        output.clamp(self.output_min, self.output_max)
    }

    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.previous_error = 0.0;
    }

    pub fn set_setpoint(&mut self, setpoint: f64) {
        if (setpoint - self.setpoint).abs() > 0.001 {
            self.reset();
        }
        self.setpoint = setpoint;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MotorType {
    DC,
    Servo,
    Stepper,
    BLDC,
    Linear,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MotorState {
    Idle,
    Running,
    Stopped,
    Error(String),
    Calibrating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotorConfig {
    pub name: String,
    pub motor_type: MotorType,
    pub pin: u32,
    pub max_velocity: f64,
    pub max_acceleration: f64,
    pub gear_ratio: f64,
    pub encoder_ticks_per_rev: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotorStatus {
    pub name: String,
    pub state: MotorState,
    pub current_position: f64,
    pub current_velocity: f64,
    pub target_position: f64,
    pub target_velocity: f64,
    pub pwm_output: f64,
    pub temperature_c: Option<f64>,
    pub current_amps: Option<f64>,
    pub error_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JointAngle {
    pub joint_name: String,
    pub angle_rad: f64,
    pub velocity_rad_s: f64,
    pub torque_nm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pose3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub roll: f64,
    pub pitch: f64,
    pub yaw: f64,
}

impl Pose3D {
    pub fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryPoint {
    pub pose: Pose3D,
    pub velocity: f64,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trajectory {
    pub id: String,
    pub points: Vec<TrajectoryPoint>,
    pub total_duration_ms: u64,
    pub smooth: bool,
    pub created_at: DateTime<Utc>,
}

impl Trajectory {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            points: Vec::new(),
            total_duration_ms: 0,
            smooth: true,
            created_at: Utc::now(),
        }
    }

    pub fn add_point(&mut self, pose: Pose3D, velocity: f64) {
        let ts = self
            .points
            .last()
            .map(|p| p.timestamp_ms + 100)
            .unwrap_or(0);
        self.points.push(TrajectoryPoint {
            pose,
            velocity,
            timestamp_ms: ts,
        });
        self.total_duration_ms = ts + 100;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InverseKinematicsSolution {
    pub joint_angles: Vec<JointAngle>,
    pub reachable: bool,
    pub error_mm: f64,
    pub iterations: u32,
    pub computation_time_ms: u64,
}

pub struct MotorController {
    pub motors: Arc<RwLock<HashMap<String, MotorConfig>>>,
    pub statuses: Arc<RwLock<HashMap<String, MotorStatus>>>,
    pub pid_controllers: Arc<RwLock<HashMap<String, PIDController>>>,
    pub active_trajectory: Arc<RwLock<Option<Trajectory>>>,
    pub current_pose: Arc<RwLock<Pose3D>>,
    pub joint_angles: Arc<RwLock<Vec<JointAngle>>>,
    pub enabled: bool,
}

impl MotorController {
    pub fn new(enabled: bool) -> Self {
        Self {
            motors: Arc::new(RwLock::new(HashMap::new())),
            statuses: Arc::new(RwLock::new(HashMap::new())),
            pid_controllers: Arc::new(RwLock::new(HashMap::new())),
            active_trajectory: Arc::new(RwLock::new(None)),
            current_pose: Arc::new(RwLock::new(Pose3D::zero())),
            joint_angles: Arc::new(RwLock::new(Vec::new())),
            enabled,
        }
    }

    pub async fn register_motor(&self, config: MotorConfig) -> Result<()> {
        info!("Registering motor: {}", config.name);
        let name = config.name.clone();

        let pid = PIDController::new(1.0, 0.01, 0.1, -config.max_velocity, config.max_velocity);

        let status = MotorStatus {
            name: name.clone(),
            state: MotorState::Idle,
            current_position: 0.0,
            current_velocity: 0.0,
            target_position: 0.0,
            target_velocity: 0.0,
            pwm_output: 0.0,
            temperature_c: None,
            current_amps: None,
            error_count: 0,
        };

        self.motors.write().await.insert(name.clone(), config);
        self.statuses.write().await.insert(name.clone(), status);
        self.pid_controllers.write().await.insert(name, pid);

        Ok(())
    }

    pub async fn set_velocity(&self, motor_name: &str, velocity: f64) -> Result<()> {
        let motors = self.motors.read().await;
        let motor = motors
            .get(motor_name)
            .ok_or_else(|| anyhow::anyhow!("Motor '{}' not found", motor_name))?;

        let clamped = velocity.clamp(-motor.max_velocity, motor.max_velocity);
        drop(motors);

        let mut statuses = self.statuses.write().await;
        if let Some(status) = statuses.get_mut(motor_name) {
            status.target_velocity = clamped;
            status.state = if clamped.abs() < 1e-6 {
                MotorState::Idle
            } else {
                MotorState::Running
            };
            debug!("Motor '{}' velocity set to {:.3}", motor_name, clamped);
        }

        Ok(())
    }

    pub async fn move_to_position(&self, motor_name: &str, position: f64) -> Result<()> {
        {
            let mut pids = self.pid_controllers.write().await;
            if let Some(pid) = pids.get_mut(motor_name) {
                pid.set_setpoint(position);
            }
        }

        let mut statuses = self.statuses.write().await;
        if let Some(status) = statuses.get_mut(motor_name) {
            status.target_position = position;
            status.state = MotorState::Running;
        }

        Ok(())
    }

    pub async fn execute_trajectory(&self, trajectory: Trajectory) -> Result<()> {
        info!(
            "Executing trajectory '{}' with {} points",
            trajectory.id,
            trajectory.points.len()
        );
        *self.active_trajectory.write().await = Some(trajectory);
        Ok(())
    }

    pub async fn stop_all(&self) -> Result<()> {
        warn!("Emergency stop: halting all motors");
        let mut statuses = self.statuses.write().await;
        for status in statuses.values_mut() {
            status.target_velocity = 0.0;
            status.target_position = status.current_position;
            status.state = MotorState::Stopped;
        }
        *self.active_trajectory.write().await = None;
        Ok(())
    }

    /// Compute inverse kinematics for a 6-DOF robotic arm (analytical approximation).
    pub async fn solve_ik(
        &self,
        target_pose: &Pose3D,
        link_lengths: &[f64],
    ) -> InverseKinematicsSolution {
        let start = std::time::Instant::now();

        let reach = link_lengths.iter().sum::<f64>();
        let dist = (target_pose.x.powi(2) + target_pose.y.powi(2) + target_pose.z.powi(2)).sqrt();
        let reachable = dist <= reach;

        if !reachable {
            return InverseKinematicsSolution {
                joint_angles: Vec::new(),
                reachable: false,
                error_mm: (dist - reach) * 1000.0,
                iterations: 0,
                computation_time_ms: start.elapsed().as_millis() as u64,
            };
        }

        // Simplified IK via Jacobian pseudoinverse iteration
        let mut joints: Vec<f64> = vec![0.0; link_lengths.len()];
        let mut error = f64::MAX;
        let max_iter = 100u32;
        let mut iterations = 0u32;

        for _ in 0..max_iter {
            iterations += 1;
            // Forward kinematics estimate
            let mut ex = 0.0_f64;
            let mut ey = 0.0_f64;
            let mut cumulative_angle = 0.0_f64;
            for (i, &len) in link_lengths.iter().enumerate() {
                cumulative_angle += joints[i];
                ex += len * cumulative_angle.cos();
                ey += len * cumulative_angle.sin();
            }

            let dx = target_pose.x - ex;
            let dy = target_pose.y - ey;
            error = (dx * dx + dy * dy).sqrt();

            if error < 0.001 {
                break;
            }

            // Simple gradient descent step
            for i in 0..joints.len() {
                joints[i] += 0.01 * (dx * cumulative_angle.sin() - dy * cumulative_angle.cos());
            }
        }

        let joint_angles = link_lengths
            .iter()
            .enumerate()
            .map(|(i, _)| JointAngle {
                joint_name: format!("joint_{}", i + 1),
                angle_rad: joints[i],
                velocity_rad_s: 0.0,
                torque_nm: 0.0,
            })
            .collect();

        InverseKinematicsSolution {
            joint_angles,
            reachable: true,
            error_mm: error * 1000.0,
            iterations,
            computation_time_ms: start.elapsed().as_millis() as u64,
        }
    }

    pub async fn get_motor_status(&self, motor_name: &str) -> Option<MotorStatus> {
        self.statuses.read().await.get(motor_name).cloned()
    }

    pub async fn get_all_statuses(&self) -> Vec<MotorStatus> {
        self.statuses.read().await.values().cloned().collect()
    }

    pub async fn calibrate_motor(&self, motor_name: &str) -> Result<()> {
        info!("Calibrating motor: {}", motor_name);
        let mut statuses = self.statuses.write().await;
        if let Some(status) = statuses.get_mut(motor_name) {
            status.state = MotorState::Calibrating;
            status.current_position = 0.0;
            status.current_velocity = 0.0;
            status.state = MotorState::Idle;
        }
        let mut pids = self.pid_controllers.write().await;
        if let Some(pid) = pids.get_mut(motor_name) {
            pid.reset();
        }
        Ok(())
    }
}

impl Default for MotorController {
    fn default() -> Self {
        Self::new(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_compute() {
        let mut pid = PIDController::new(1.0, 0.0, 0.0, -100.0, 100.0);
        pid.set_setpoint(10.0);
        let output = pid.compute(5.0, 0.1);
        assert!((output - 5.0).abs() < 1e-9);
    }

    #[tokio::test]
    async fn test_motor_register_and_stop() {
        let ctrl = MotorController::new(false);
        ctrl.register_motor(MotorConfig {
            name: "left_wheel".to_string(),
            motor_type: MotorType::DC,
            pin: 18,
            max_velocity: 1.0,
            max_acceleration: 0.5,
            gear_ratio: 1.0,
            encoder_ticks_per_rev: 360,
        })
        .await
        .unwrap();

        ctrl.set_velocity("left_wheel", 0.5).await.unwrap();
        let status = ctrl.get_motor_status("left_wheel").await.unwrap();
        assert_eq!(status.state, MotorState::Running);

        ctrl.stop_all().await.unwrap();
        let status = ctrl.get_motor_status("left_wheel").await.unwrap();
        assert_eq!(status.state, MotorState::Stopped);
    }

    #[tokio::test]
    async fn test_ik_unreachable() {
        let ctrl = MotorController::new(false);
        let pose = Pose3D {
            x: 100.0,
            y: 100.0,
            z: 100.0,
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
        };
        let sol = ctrl.solve_ik(&pose, &[0.5, 0.5]).await;
        assert!(!sol.reachable);
    }
}

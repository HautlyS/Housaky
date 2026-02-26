use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::motor_control::Pose3D;
use super::sensor_fusion::{IMUData, LidarScan, LidarPoint};
use super::spatial_reasoning::Point2D;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ROS2QoSReliability {
    Reliable,
    BestEffort,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ROS2QoSDurability {
    TransientLocal,
    Volatile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ROS2QoSProfile {
    pub reliability: ROS2QoSReliability,
    pub durability: ROS2QoSDurability,
    pub depth: usize,
}

impl ROS2QoSProfile {
    pub fn sensor_data() -> Self {
        Self {
            reliability: ROS2QoSReliability::BestEffort,
            durability: ROS2QoSDurability::Volatile,
            depth: 10,
        }
    }

    pub fn reliable() -> Self {
        Self {
            reliability: ROS2QoSReliability::Reliable,
            durability: ROS2QoSDurability::TransientLocal,
            depth: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ROS2Header {
    pub stamp_sec: i64,
    pub stamp_nanosec: u32,
    pub frame_id: String,
}

impl ROS2Header {
    pub fn now(frame_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            stamp_sec: now.timestamp(),
            stamp_nanosec: now.timestamp_subsec_nanos(),
            frame_id: frame_id.into(),
        }
    }
}

/// ROS2 Twist message (velocity command)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Twist {
    pub linear_x: f64,
    pub linear_y: f64,
    pub linear_z: f64,
    pub angular_x: f64,
    pub angular_y: f64,
    pub angular_z: f64,
}

impl Twist {
    pub fn zero() -> Self {
        Self {
            linear_x: 0.0,
            linear_y: 0.0,
            linear_z: 0.0,
            angular_x: 0.0,
            angular_y: 0.0,
            angular_z: 0.0,
        }
    }

    pub fn from_velocity(linear: f64, angular: f64) -> Self {
        Self {
            linear_x: linear,
            angular_z: angular,
            ..Self::zero()
        }
    }
}

/// ROS2 Odometry message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Odometry {
    pub header: ROS2Header,
    pub child_frame_id: String,
    pub pose_x: f64,
    pub pose_y: f64,
    pub pose_z: f64,
    pub orientation_x: f64,
    pub orientation_y: f64,
    pub orientation_z: f64,
    pub orientation_w: f64,
    pub twist: Twist,
    pub pose_covariance: Vec<f64>,
    pub twist_covariance: Vec<f64>,
}

impl Odometry {
    pub fn from_pose_and_twist(pose: &Pose3D, twist: Twist) -> Self {
        // Euler to quaternion (yaw only for 2D)
        let half_yaw = pose.yaw / 2.0;
        Self {
            header: ROS2Header::now("odom"),
            child_frame_id: "base_link".to_string(),
            pose_x: pose.x,
            pose_y: pose.y,
            pose_z: pose.z,
            orientation_x: 0.0,
            orientation_y: 0.0,
            orientation_z: half_yaw.sin(),
            orientation_w: half_yaw.cos(),
            twist,
            pose_covariance: vec![0.01; 36],
            twist_covariance: vec![0.01; 36],
        }
    }

    pub fn to_pose2d(&self) -> Point2D {
        Point2D::new(self.pose_x, self.pose_y)
    }

    pub fn yaw_rad(&self) -> f64 {
        2.0 * self.orientation_z.atan2(self.orientation_w)
    }
}

/// ROS2 LaserScan message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ROS2LaserScan {
    pub header: ROS2Header,
    pub angle_min: f64,
    pub angle_max: f64,
    pub angle_increment: f64,
    pub time_increment: f64,
    pub scan_time: f64,
    pub range_min: f64,
    pub range_max: f64,
    pub ranges: Vec<f32>,
    pub intensities: Vec<f32>,
}

impl ROS2LaserScan {
    pub fn to_lidar_scan(&self) -> LidarScan {
        let points: Vec<LidarPoint> = self
            .ranges
            .iter()
            .enumerate()
            .map(|(i, &r)| {
                let angle = self.angle_min + i as f64 * self.angle_increment;
                LidarPoint {
                    angle_rad: angle,
                    distance_m: r as f64,
                    intensity: self.intensities.get(i).copied().unwrap_or(0.0) as f64,
                    valid: r >= self.range_min as f32 && r <= self.range_max as f32 && r.is_finite(),
                }
            })
            .collect();

        LidarScan {
            points,
            range_min_m: self.range_min,
            range_max_m: self.range_max,
            angle_min_rad: self.angle_min,
            angle_max_rad: self.angle_max,
            scan_time_ms: (self.scan_time * 1000.0) as u64,
            timestamp: Utc::now(),
        }
    }
}

/// ROS2 IMU message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ROS2IMU {
    pub header: ROS2Header,
    pub orientation_x: f64,
    pub orientation_y: f64,
    pub orientation_z: f64,
    pub orientation_w: f64,
    pub angular_velocity_x: f64,
    pub angular_velocity_y: f64,
    pub angular_velocity_z: f64,
    pub linear_acceleration_x: f64,
    pub linear_acceleration_y: f64,
    pub linear_acceleration_z: f64,
}

impl ROS2IMU {
    pub fn to_imu_data(&self) -> IMUData {
        IMUData {
            accel_x: self.linear_acceleration_x,
            accel_y: self.linear_acceleration_y,
            accel_z: self.linear_acceleration_z,
            gyro_x: self.angular_velocity_x,
            gyro_y: self.angular_velocity_y,
            gyro_z: self.angular_velocity_z,
            mag_x: 0.0,
            mag_y: 0.0,
            mag_z: 0.0,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ROS2Topic {
    pub name: String,
    pub message_type: String,
    pub qos: ROS2QoSProfile,
    pub message_count: u64,
    pub last_message_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ROS2NodeInfo {
    pub name: String,
    pub namespace: String,
    pub publishers: Vec<String>,
    pub subscribers: Vec<String>,
    pub services: Vec<String>,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ROSBridgeConfig {
    pub enabled: bool,
    pub domain_id: u32,
    pub namespace: String,
    pub cmd_vel_topic: String,
    pub odom_topic: String,
    pub scan_topic: String,
    pub imu_topic: String,
    pub tf_topic: String,
    pub heartbeat_interval_ms: u64,
}

impl Default for ROSBridgeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            domain_id: 0,
            namespace: "/housaky".to_string(),
            cmd_vel_topic: "/cmd_vel".to_string(),
            odom_topic: "/odom".to_string(),
            scan_topic: "/scan".to_string(),
            imu_topic: "/imu/data".to_string(),
            tf_topic: "/tf".to_string(),
            heartbeat_interval_ms: 1000,
        }
    }
}

pub type TopicCallback = Arc<dyn Fn(serde_json::Value) + Send + Sync>;

pub struct ROSBridge {
    pub config: ROSBridgeConfig,
    pub connected: Arc<RwLock<bool>>,
    pub topics: Arc<RwLock<HashMap<String, ROS2Topic>>>,
    pub node_info: Arc<RwLock<ROS2NodeInfo>>,
    pub message_queue: Arc<RwLock<Vec<(String, serde_json::Value)>>>,
    pub callbacks: Arc<RwLock<HashMap<String, TopicCallback>>>,
    pub published_count: Arc<RwLock<u64>>,
    pub received_count: Arc<RwLock<u64>>,
}

impl ROSBridge {
    pub fn new(config: ROSBridgeConfig) -> Self {
        let node_info = ROS2NodeInfo {
            name: "housaky_node".to_string(),
            namespace: config.namespace.clone(),
            publishers: vec![
                config.cmd_vel_topic.clone(),
            ],
            subscribers: vec![
                config.odom_topic.clone(),
                config.scan_topic.clone(),
                config.imu_topic.clone(),
            ],
            services: vec!["/housaky/get_status".to_string()],
            actions: vec!["/housaky/navigate_to".to_string()],
        };

        Self {
            config,
            connected: Arc::new(RwLock::new(false)),
            topics: Arc::new(RwLock::new(HashMap::new())),
            node_info: Arc::new(RwLock::new(node_info)),
            message_queue: Arc::new(RwLock::new(Vec::new())),
            callbacks: Arc::new(RwLock::new(HashMap::new())),
            published_count: Arc::new(RwLock::new(0)),
            received_count: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn connect(&self) -> Result<()> {
        if !self.config.enabled {
            info!("ROS bridge disabled by config, running in simulation mode");
            *self.connected.write().await = false;
            return Ok(());
        }

        info!(
            "Connecting to ROS2 (domain_id={}, namespace={})",
            self.config.domain_id, self.config.namespace
        );

        // In a full implementation this would establish DDS/rclrs connection.
        // For now we mark connected and register default topics.
        *self.connected.write().await = true;

        let mut topics = self.topics.write().await;
        for (name, msg_type, qos) in [
            (&self.config.cmd_vel_topic, "geometry_msgs/Twist", ROS2QoSProfile::reliable()),
            (&self.config.odom_topic, "nav_msgs/Odometry", ROS2QoSProfile::sensor_data()),
            (&self.config.scan_topic, "sensor_msgs/LaserScan", ROS2QoSProfile::sensor_data()),
            (&self.config.imu_topic, "sensor_msgs/Imu", ROS2QoSProfile::sensor_data()),
        ] {
            topics.insert(
                name.clone(),
                ROS2Topic {
                    name: name.clone(),
                    message_type: msg_type.to_string(),
                    qos,
                    message_count: 0,
                    last_message_at: None,
                },
            );
        }

        info!("ROS2 bridge connected, {} topics registered", topics.len());
        Ok(())
    }

    pub async fn disconnect(&self) {
        *self.connected.write().await = false;
        info!("ROS2 bridge disconnected");
    }

    pub async fn publish_cmd_vel(&self, linear: f64, angular: f64) -> Result<()> {
        let twist = Twist::from_velocity(linear, angular);
        let msg = serde_json::json!({
            "linear": {"x": twist.linear_x, "y": twist.linear_y, "z": twist.linear_z},
            "angular": {"x": twist.angular_x, "y": twist.angular_y, "z": twist.angular_z},
        });

        self.publish(&self.config.cmd_vel_topic.clone(), msg).await?;
        debug!("Published cmd_vel: linear={:.3}, angular={:.3}", linear, angular);
        Ok(())
    }

    pub async fn publish(&self, topic: &str, message: serde_json::Value) -> Result<()> {
        let connected = *self.connected.read().await;

        // Update topic stats
        let mut topics = self.topics.write().await;
        if let Some(t) = topics.get_mut(topic) {
            t.message_count += 1;
            t.last_message_at = Some(Utc::now());
        }
        drop(topics);

        if connected {
            // In production: serialize and send via DDS transport
            debug!("ROS2 publish on '{}': {} bytes", topic, message.to_string().len());
        } else {
            // Simulation: queue for inspection
            let mut queue = self.message_queue.write().await;
            if queue.len() >= 1000 {
                queue.drain(..100);
            }
            queue.push((topic.to_string(), message));
        }

        *self.published_count.write().await += 1;
        Ok(())
    }

    pub async fn subscribe(
        &self,
        topic: &str,
        callback: TopicCallback,
    ) -> Result<()> {
        self.callbacks
            .write()
            .await
            .insert(topic.to_string(), callback);
        info!("Subscribed to ROS2 topic '{}'", topic);
        Ok(())
    }

    /// Simulate receiving an odometry message (for testing without real ROS2).
    pub async fn inject_odometry(&self, odom: Odometry) -> Result<()> {
        let msg = serde_json::to_value(&odom)?;
        self.receive_message(&self.config.odom_topic.clone(), msg).await;
        Ok(())
    }

    /// Simulate receiving a laser scan.
    pub async fn inject_laser_scan(&self, scan: ROS2LaserScan) -> Result<()> {
        let msg = serde_json::to_value(&scan)?;
        self.receive_message(&self.config.scan_topic.clone(), msg).await;
        Ok(())
    }

    /// Simulate receiving an IMU message.
    pub async fn inject_imu(&self, imu: ROS2IMU) -> Result<()> {
        let msg = serde_json::to_value(&imu)?;
        self.receive_message(&self.config.imu_topic.clone(), msg).await;
        Ok(())
    }

    async fn receive_message(&self, topic: &str, message: serde_json::Value) {
        *self.received_count.write().await += 1;

        let callbacks = self.callbacks.read().await;
        if let Some(cb) = callbacks.get(topic) {
            cb(message);
        }
    }

    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    pub async fn get_topic_list(&self) -> Vec<ROS2Topic> {
        self.topics.read().await.values().cloned().collect()
    }

    pub async fn get_node_info(&self) -> ROS2NodeInfo {
        self.node_info.read().await.clone()
    }

    pub async fn get_stats(&self) -> ROSBridgeStats {
        ROSBridgeStats {
            connected: *self.connected.read().await,
            topic_count: self.topics.read().await.len(),
            published_messages: *self.published_count.read().await,
            received_messages: *self.received_count.read().await,
            queued_messages: self.message_queue.read().await.len(),
            domain_id: self.config.domain_id,
        }
    }

    pub async fn drain_queue(&self) -> Vec<(String, serde_json::Value)> {
        let mut queue = self.message_queue.write().await;
        std::mem::take(&mut *queue)
    }
}

impl Default for ROSBridge {
    fn default() -> Self {
        Self::new(ROSBridgeConfig::default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ROSBridgeStats {
    pub connected: bool,
    pub topic_count: usize,
    pub published_messages: u64,
    pub received_messages: u64,
    pub queued_messages: usize,
    pub domain_id: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bridge_disabled_no_connect() {
        let bridge = ROSBridge::default();
        bridge.connect().await.unwrap();
        assert!(!bridge.is_connected().await);
    }

    #[tokio::test]
    async fn test_publish_queues_when_disconnected() {
        let bridge = ROSBridge::default();
        bridge
            .publish("/test", serde_json::json!({"data": 42}))
            .await
            .unwrap();
        let queue = bridge.drain_queue().await;
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].0, "/test");
    }

    #[tokio::test]
    async fn test_publish_cmd_vel() {
        let bridge = ROSBridge::default();
        bridge.publish_cmd_vel(0.5, 0.1).await.unwrap();
        let queue = bridge.drain_queue().await;
        assert!(!queue.is_empty());
    }

    #[test]
    fn test_laser_scan_conversion() {
        let scan = ROS2LaserScan {
            header: ROS2Header::now("laser"),
            angle_min: -std::f64::consts::PI,
            angle_max: std::f64::consts::PI,
            angle_increment: 0.01,
            time_increment: 0.0,
            scan_time: 0.1,
            range_min: 0.1,
            range_max: 10.0,
            ranges: vec![1.0; 628],
            intensities: vec![100.0; 628],
        };
        let lidar = scan.to_lidar_scan();
        assert_eq!(lidar.points.len(), 628);
        assert!(lidar.points[0].valid);
    }

    #[test]
    fn test_odometry_yaw() {
        let odom = Odometry::from_pose_and_twist(
            &Pose3D {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                roll: 0.0,
                pitch: 0.0,
                yaw: std::f64::consts::FRAC_PI_2,
            },
            Twist::zero(),
        );
        let yaw = odom.yaw_rad();
        assert!((yaw - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    }

    #[tokio::test]
    async fn test_subscribe_and_receive() {
        let bridge = ROSBridge::default();
        let received = Arc::new(RwLock::new(false));
        let received_clone = received.clone();

        bridge
            .subscribe(
                "/test_topic",
                Arc::new(move |_msg| {
                    let r = received_clone.clone();
                    tokio::spawn(async move {
                        *r.write().await = true;
                    });
                }),
            )
            .await
            .unwrap();

        bridge
            .receive_message("/test_topic", serde_json::json!({}))
            .await;

        // Allow spawned task to run
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert!(*received.read().await);
    }
}

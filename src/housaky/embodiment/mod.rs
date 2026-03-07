pub mod manipulation;
pub mod motor_control;
pub mod navigation;
pub mod ros_bridge;
pub mod sensor_fusion;
pub mod spatial_reasoning;

use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub use manipulation::{
    DetectedObject, ForceFeedback, GraspCandidate, GraspExecution, GraspState, GraspType,
    ManipulationEngine, ManipulationResult, ManipulationTask, ManipulationTaskType,
};
pub use motor_control::{
    InverseKinematicsSolution, JointAngle, MotorConfig, MotorController, MotorState, MotorStatus,
    MotorType, PIDController, Pose3D, Trajectory, TrajectoryPoint,
};
pub use navigation::{
    NavigationGoal, NavigationResult, NavigationState, Navigator, PurePursuitController,
    RecoveryBehavior, SLAMMap, SLAMPose,
};
pub use ros_bridge::{
    Odometry, ROS2Header, ROS2LaserScan, ROS2QoSProfile, ROS2Topic, ROSBridge, ROSBridgeConfig,
    ROSBridgeStats, Twist, ROS2IMU,
};
pub use sensor_fusion::{
    EmbodimentSensorFusion, FusedState, IMUData, KalmanFilter, LidarPoint, LidarScan,
    SensorReading, SensorType,
};
pub use spatial_reasoning::{
    BoundingBox, Obstacle, OccupancyGrid, Path, Point2D, Point3D, SpatialReasoner,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmbodimentControllerConfig {
    pub enabled: bool,
    pub ros_enabled: bool,
    pub sensor_fusion: bool,
}

pub struct EmbodimentController {
    config: EmbodimentControllerConfig,
    ros_bridge: Option<Arc<ROSBridge>>,
    sensor_fusion: Option<Arc<EmbodimentSensorFusion>>,
    navigator: Option<Arc<Navigator>>,
    manipulator: Option<Arc<ManipulationEngine>>,
}

impl EmbodimentController {
    pub fn new(config: EmbodimentControllerConfig) -> Self {
        let ros_bridge = if config.ros_enabled {
            Some(Arc::new(ROSBridge::new(ROSBridgeConfig::default())))
        } else {
            None
        };

        let sensor_fusion = if config.sensor_fusion {
            Some(Arc::new(EmbodimentSensorFusion::new()))
        } else {
            None
        };

        Self {
            config,
            ros_bridge,
            sensor_fusion,
            navigator: None,
            manipulator: None,
        }
    }

    pub fn config(&self) -> &EmbodimentControllerConfig {
        &self.config
    }

    pub fn is_available(&self) -> bool {
        self.config.enabled && (self.ros_bridge.is_some() || self.sensor_fusion.is_some())
    }
}

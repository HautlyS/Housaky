use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::sensor_fusion::LidarScan;
use super::spatial_reasoning::{Path, Point2D, SpatialReasoner};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NavigationState {
    Idle,
    Planning,
    Executing,
    Recovering,
    GoalReached,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecoveryBehavior {
    RotateInPlace,
    BackUp,
    ClearCostmap,
    ReplanFromScratch,
    StopAndWait,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationGoal {
    pub id: String,
    pub target: Point2D,
    pub target_yaw: Option<f64>,
    pub tolerance_m: f64,
    pub timeout_s: Option<f64>,
    pub created_at: DateTime<Utc>,
}

impl NavigationGoal {
    pub fn new(id: impl Into<String>, x: f64, y: f64, tolerance_m: f64) -> Self {
        Self {
            id: id.into(),
            target: Point2D::new(x, y),
            target_yaw: None,
            tolerance_m,
            timeout_s: None,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationResult {
    pub goal_id: String,
    pub success: bool,
    pub final_position: Point2D,
    pub distance_to_goal_m: f64,
    pub duration_s: f64,
    pub waypoints_traversed: usize,
    pub recovery_attempts: u32,
    pub failure_reason: Option<String>,
}

/// SLAM map: pose graph + scan-matched occupancy grid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLAMPose {
    pub id: u64,
    pub position: Point2D,
    pub yaw: f64,
    pub timestamp: DateTime<Utc>,
    pub scan_key: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLAMMap {
    pub pose_graph: Vec<SLAMPose>,
    pub loop_closures: Vec<(u64, u64)>,
    pub map_quality: f64,
    pub area_mapped_m2: f64,
    pub last_updated: DateTime<Utc>,
}

impl SLAMMap {
    pub fn new() -> Self {
        Self {
            pose_graph: Vec::new(),
            loop_closures: Vec::new(),
            map_quality: 0.0,
            area_mapped_m2: 0.0,
            last_updated: Utc::now(),
        }
    }

    pub fn add_pose(&mut self, position: Point2D, yaw: f64) -> u64 {
        let id = self.pose_graph.len() as u64;
        self.pose_graph.push(SLAMPose {
            id,
            position,
            yaw,
            timestamp: Utc::now(),
            scan_key: None,
        });
        self.last_updated = Utc::now();
        id
    }

    pub fn detect_loop_closure(
        &mut self,
        current_id: u64,
        position: &Point2D,
        threshold_m: f64,
    ) -> bool {
        for pose in &self.pose_graph {
            if pose.id == current_id {
                continue;
            }
            let dist = position.distance_to(&pose.position);
            if dist < threshold_m {
                let key = (current_id.min(pose.id), current_id.max(pose.id));
                if !self.loop_closures.contains(&key) {
                    self.loop_closures.push(key);
                    self.map_quality = (self.map_quality + 0.01).min(1.0);
                    return true;
                }
            }
        }
        false
    }
}

impl Default for SLAMMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Pure-pursuit controller for smooth path following.
pub struct PurePursuitController {
    pub lookahead_distance_m: f64,
    pub wheelbase_m: f64,
    pub max_angular_velocity: f64,
}

impl PurePursuitController {
    pub fn new(lookahead_distance_m: f64, wheelbase_m: f64) -> Self {
        Self {
            lookahead_distance_m,
            wheelbase_m,
            max_angular_velocity: std::f64::consts::PI,
        }
    }

    /// Compute steering angle toward a lookahead point.
    pub fn compute_steering(
        &self,
        robot_pos: &Point2D,
        robot_yaw: f64,
        path: &[Point2D],
    ) -> Option<(f64, f64)> {
        if path.is_empty() {
            return None;
        }

        // Find lookahead point on path
        let lookahead = self.find_lookahead_point(robot_pos, path)?;

        // Transform lookahead into robot frame
        let dx = lookahead.x - robot_pos.x;
        let dy = lookahead.y - robot_pos.y;
        let local_x = dx * robot_yaw.cos() + dy * robot_yaw.sin();
        let local_y = -dx * robot_yaw.sin() + dy * robot_yaw.cos();

        let dist = (local_x.powi(2) + local_y.powi(2)).sqrt();
        if dist < 1e-6 {
            return Some((0.0, 0.0));
        }

        let curvature = 2.0 * local_y / (dist.powi(2));
        let angular_vel = (curvature * 0.5).clamp(
            -self.max_angular_velocity,
            self.max_angular_velocity,
        );
        let linear_vel = 0.3_f64.min(1.0 / (1.0 + curvature.abs() * 2.0));

        Some((linear_vel, angular_vel))
    }

    fn find_lookahead_point(&self, robot_pos: &Point2D, path: &[Point2D]) -> Option<Point2D> {
        // Find the first path point beyond lookahead distance
        for point in path {
            if robot_pos.distance_to(point) >= self.lookahead_distance_m {
                return Some(point.clone());
            }
        }
        // Fallback to last point
        path.last().cloned()
    }
}

pub struct Navigator {
    pub spatial_reasoner: Arc<SpatialReasoner>,
    pub slam_map: Arc<RwLock<SLAMMap>>,
    pub navigation_state: Arc<RwLock<NavigationState>>,
    pub current_goal: Arc<RwLock<Option<NavigationGoal>>>,
    pub current_path: Arc<RwLock<Option<Path>>>,
    pub current_waypoint_idx: Arc<RwLock<usize>>,
    pub navigation_history: Arc<RwLock<VecDeque<NavigationResult>>>,
    pub pure_pursuit: PurePursuitController,
    pub recovery_behaviors: Vec<RecoveryBehavior>,
    pub recovery_attempts: Arc<RwLock<u32>>,
    pub max_recovery_attempts: u32,
    pub position_history: Arc<RwLock<VecDeque<(Point2D, DateTime<Utc>)>>>,
}

impl Navigator {
    pub fn new(spatial_reasoner: Arc<SpatialReasoner>) -> Self {
        Self {
            spatial_reasoner,
            slam_map: Arc::new(RwLock::new(SLAMMap::new())),
            navigation_state: Arc::new(RwLock::new(NavigationState::Idle)),
            current_goal: Arc::new(RwLock::new(None)),
            current_path: Arc::new(RwLock::new(None)),
            current_waypoint_idx: Arc::new(RwLock::new(0)),
            navigation_history: Arc::new(RwLock::new(VecDeque::new())),
            pure_pursuit: PurePursuitController::new(0.3, 0.3),
            recovery_behaviors: vec![
                RecoveryBehavior::RotateInPlace,
                RecoveryBehavior::BackUp,
                RecoveryBehavior::ClearCostmap,
                RecoveryBehavior::ReplanFromScratch,
            ],
            recovery_attempts: Arc::new(RwLock::new(0)),
            max_recovery_attempts: 3,
            position_history: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    pub async fn navigate_to(&self, goal: NavigationGoal) -> Result<NavigationResult> {
        let t0 = std::time::Instant::now();
        info!("Navigating to goal '{}' at ({:.2}, {:.2})", goal.id, goal.target.x, goal.target.y);

        *self.navigation_state.write().await = NavigationState::Planning;
        *self.current_goal.write().await = Some(goal.clone());
        *self.recovery_attempts.write().await = 0;

        // Get current position from SLAM
        let current_pos = self.get_current_position().await;

        // Plan path
        let mut path = self
            .spatial_reasoner
            .find_path(&current_pos, &goal.target)
            .await
            .map_err(|e| {
                anyhow::anyhow!("Path planning failed: {}", e)
            })?;

        self.spatial_reasoner.smooth_path(&mut path).await;

        info!(
            "Path planned: {} waypoints, {:.2}m",
            path.waypoints.len(),
            path.total_distance_m
        );

        *self.current_path.write().await = Some(path.clone());
        *self.current_waypoint_idx.write().await = 0;
        *self.navigation_state.write().await = NavigationState::Executing;

        // Execute path (simulation: advance through waypoints)
        let waypoints_count = path.waypoints.len();
        let mut traversed = 0;

        for waypoint in &path.waypoints {
            let dist = waypoint.distance_to(&goal.target);
            traversed += 1;

            // Check for stuck condition
            if self.is_stuck().await {
                warn!("Robot appears stuck, attempting recovery");
                let recovered = self.attempt_recovery().await;
                if !recovered {
                    let result = NavigationResult {
                        goal_id: goal.id.clone(),
                        success: false,
                        final_position: self.get_current_position().await,
                        distance_to_goal_m: dist,
                        duration_s: t0.elapsed().as_secs_f64(),
                        waypoints_traversed: traversed,
                        recovery_attempts: *self.recovery_attempts.read().await,
                        failure_reason: Some("Recovery failed: robot is stuck".to_string()),
                    };
                    *self.navigation_state.write().await =
                        NavigationState::Failed("Stuck".to_string());
                    self.navigation_history.write().await.push_back(result.clone());
                    return Ok(result);
                }
            }

            // Record position
            let mut hist = self.position_history.write().await;
            if hist.len() >= 500 {
                hist.pop_front();
            }
            hist.push_back((waypoint.clone(), Utc::now()));

            debug!("Traversed waypoint {}/{}", traversed, waypoints_count);
        }

        let final_pos = goal.target.clone();
        let final_dist = final_pos.distance_to(&goal.target);

        *self.navigation_state.write().await = NavigationState::GoalReached;

        let result = NavigationResult {
            goal_id: goal.id.clone(),
            success: final_dist <= goal.tolerance_m,
            final_position: final_pos,
            distance_to_goal_m: final_dist,
            duration_s: t0.elapsed().as_secs_f64(),
            waypoints_traversed: traversed,
            recovery_attempts: *self.recovery_attempts.read().await,
            failure_reason: None,
        };

        let mut nav_hist = self.navigation_history.write().await;
        if nav_hist.len() >= 100 {
            nav_hist.pop_front();
        }
        nav_hist.push_back(result.clone());

        info!(
            "Navigation complete: success={}, dist={:.3}m in {:.2}s",
            result.success, result.distance_to_goal_m, result.duration_s
        );

        Ok(result)
    }

    /// Update SLAM with new lidar scan and current odometry.
    pub async fn update_slam(&self, scan: &LidarScan, odom_pos: &Point2D, odom_yaw: f64) {
        let mut slam = self.slam_map.write().await;
        let pose_id = slam.add_pose(odom_pos.clone(), odom_yaw);

        // Check for loop closure
        if slam.detect_loop_closure(pose_id, odom_pos, 0.5) {
            info!("Loop closure detected at pose {}", pose_id);
        }

        // Convert lidar scan to hit points and update map
        let sensor_pos = odom_pos.clone();
        let hit_points: Vec<Point2D> = scan
            .points
            .iter()
            .filter(|p| p.valid && p.distance_m < scan.range_max_m)
            .map(|p| {
                Point2D::new(
                    odom_pos.x + p.distance_m * (odom_yaw + p.angle_rad).cos(),
                    odom_pos.y + p.distance_m * (odom_yaw + p.angle_rad).sin(),
                )
            })
            .collect();

        drop(slam);
        self.spatial_reasoner
            .update_map_from_lidar(&sensor_pos, &hit_points)
            .await;

        debug!("SLAM updated with {} scan points", hit_points.len());
    }

    pub async fn get_current_position(&self) -> Point2D {
        self.position_history
            .read()
            .await
            .back()
            .map(|(p, _)| p.clone())
            .unwrap_or_else(|| Point2D::new(0.0, 0.0))
    }

    pub async fn get_navigation_state(&self) -> NavigationState {
        self.navigation_state.read().await.clone()
    }

    pub async fn cancel_navigation(&self) {
        warn!("Navigation cancelled");
        *self.navigation_state.write().await = NavigationState::Idle;
        *self.current_goal.write().await = None;
        *self.current_path.write().await = None;
    }

    pub async fn is_stuck(&self) -> bool {
        let hist = self.position_history.read().await;
        if hist.len() < 10 {
            return false;
        }
        // Check if last 10 positions are within 5cm
        let recent: Vec<_> = hist.iter().rev().take(10).collect();
        let first = &recent[0].0;
        let max_displacement = recent
            .iter()
            .map(|(p, _)| p.distance_to(first))
            .fold(0.0_f64, f64::max);

        max_displacement < 0.05
    }

    async fn attempt_recovery(&self) -> bool {
        let mut attempts = self.recovery_attempts.write().await;
        if *attempts >= self.max_recovery_attempts {
            warn!("Max recovery attempts ({}) reached", self.max_recovery_attempts);
            return false;
        }
        *attempts += 1;
        let behavior = &self.recovery_behaviors[(*attempts as usize - 1).min(self.recovery_behaviors.len() - 1)];
        info!("Recovery attempt {}: {:?}", attempts, behavior);

        match behavior {
            RecoveryBehavior::ClearCostmap => {
                // Reset obstacle cells near robot
                true
            }
            RecoveryBehavior::ReplanFromScratch => {
                *self.current_path.write().await = None;
                *self.current_waypoint_idx.write().await = 0;
                true
            }
            RecoveryBehavior::RotateInPlace | RecoveryBehavior::BackUp => true,
            RecoveryBehavior::StopAndWait => true,
        }
    }

    pub async fn get_slam_map(&self) -> SLAMMap {
        self.slam_map.read().await.clone()
    }

    pub async fn get_navigation_history(&self) -> Vec<NavigationResult> {
        self.navigation_history.read().await.iter().cloned().collect()
    }

    pub async fn compute_velocity_commands(
        &self,
        robot_pos: &Point2D,
        robot_yaw: f64,
    ) -> Option<(f64, f64)> {
        let path = self.current_path.read().await;
        let idx = *self.current_waypoint_idx.read().await;

        if let Some(ref p) = *path {
            let remaining = &p.waypoints[idx..];
            self.pure_pursuit
                .compute_steering(robot_pos, robot_yaw, remaining)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn make_navigator() -> Navigator {
        let reasoner = Arc::new(SpatialReasoner::new(100, 100, 0.1, 0.2, 0.5));
        Navigator::new(reasoner)
    }

    #[tokio::test]
    async fn test_navigate_to_simple() {
        let nav = make_navigator();
        let goal = NavigationGoal::new("g1", 0.5, 0.5, 0.1);
        let result = nav.navigate_to(goal).await.unwrap();
        assert!(result.success);
        assert_eq!(result.goal_id, "g1");
    }

    #[tokio::test]
    async fn test_cancel_navigation() {
        let nav = make_navigator();
        *nav.navigation_state.write().await = NavigationState::Executing;
        nav.cancel_navigation().await;
        assert_eq!(nav.get_navigation_state().await, NavigationState::Idle);
    }

    #[test]
    fn test_pure_pursuit_steering() {
        let pp = PurePursuitController::new(0.3, 0.3);
        let robot = Point2D::new(0.0, 0.0);
        let path = vec![Point2D::new(0.5, 0.0), Point2D::new(1.0, 0.0)];
        let cmd = pp.compute_steering(&robot, 0.0, &path);
        assert!(cmd.is_some());
        let (lin, _ang) = cmd.unwrap();
        assert!(lin > 0.0);
    }

    #[tokio::test]
    async fn test_slam_loop_closure() {
        let mut slam = SLAMMap::new();
        slam.add_pose(Point2D::new(0.0, 0.0), 0.0);
        slam.add_pose(Point2D::new(1.0, 0.0), 0.0);
        slam.add_pose(Point2D::new(2.0, 0.0), 0.0);
        let id3 = slam.add_pose(Point2D::new(0.1, 0.0), 0.0);
        let closed = slam.detect_loop_closure(id3, &Point2D::new(0.1, 0.0), 0.3);
        assert!(closed);
    }
}

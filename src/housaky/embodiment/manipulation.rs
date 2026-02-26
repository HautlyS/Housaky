use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::motor_control::{InverseKinematicsSolution, Pose3D};
use super::spatial_reasoning::{BoundingBox, Point3D};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GraspType {
    Pinch,
    Power,
    Lateral,
    Spherical,
    Hook,
    Precision,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GraspState {
    Open,
    Approaching,
    Grasping,
    Holding,
    Releasing,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraspCandidate {
    pub id: String,
    pub target_object_id: String,
    pub grasp_type: GraspType,
    pub approach_pose: Pose3D,
    pub grasp_pose: Pose3D,
    pub retreat_pose: Pose3D,
    pub quality_score: f64,
    pub force_limit_n: f64,
    pub estimated_success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraspExecution {
    pub candidate: GraspCandidate,
    pub state: GraspState,
    pub actual_force_n: f64,
    pub slip_detected: bool,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub success: bool,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedObject {
    pub id: String,
    pub label: String,
    pub bounds: BoundingBox,
    pub centroid: Point3D,
    pub mass_kg: Option<f64>,
    pub friction_coefficient: Option<f64>,
    pub fragile: bool,
    pub confidence: f64,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForceFeedback {
    pub fx: f64,
    pub fy: f64,
    pub fz: f64,
    pub tx: f64,
    pub ty: f64,
    pub tz: f64,
    pub timestamp: DateTime<Utc>,
}

impl ForceFeedback {
    pub fn zero() -> Self {
        Self {
            fx: 0.0,
            fy: 0.0,
            fz: 0.0,
            tx: 0.0,
            ty: 0.0,
            tz: 0.0,
            timestamp: Utc::now(),
        }
    }

    pub fn magnitude_n(&self) -> f64 {
        (self.fx.powi(2) + self.fy.powi(2) + self.fz.powi(2)).sqrt()
    }

    pub fn torque_magnitude_nm(&self) -> f64 {
        (self.tx.powi(2) + self.ty.powi(2) + self.tz.powi(2)).sqrt()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManipulationTask {
    pub id: String,
    pub task_type: ManipulationTaskType,
    pub target_object_id: Option<String>,
    pub target_pose: Option<Pose3D>,
    pub parameters: HashMap<String, f64>,
    pub priority: u8,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ManipulationTaskType {
    Pick,
    Place,
    PickAndPlace,
    Push,
    Pour,
    Insert,
    Unscrew,
    Draw,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManipulationResult {
    pub task_id: String,
    pub success: bool,
    pub ik_solution: Option<InverseKinematicsSolution>,
    pub grasp_execution: Option<GraspExecution>,
    pub final_pose: Option<Pose3D>,
    pub duration_ms: u64,
    pub failure_reason: Option<String>,
}

pub struct ManipulationEngine {
    pub detected_objects: Arc<RwLock<HashMap<String, DetectedObject>>>,
    pub grasp_history: Arc<RwLock<Vec<GraspExecution>>>,
    pub force_feedback: Arc<RwLock<ForceFeedback>>,
    pub active_task: Arc<RwLock<Option<ManipulationTask>>>,
    pub gripper_state: Arc<RwLock<GraspState>>,
    pub gripper_opening_m: Arc<RwLock<f64>>,
    pub link_lengths: Vec<f64>,
    pub force_limit_n: f64,
}

impl ManipulationEngine {
    pub fn new(link_lengths: Vec<f64>, force_limit_n: f64) -> Self {
        Self {
            detected_objects: Arc::new(RwLock::new(HashMap::new())),
            grasp_history: Arc::new(RwLock::new(Vec::new())),
            force_feedback: Arc::new(RwLock::new(ForceFeedback::zero())),
            active_task: Arc::new(RwLock::new(None)),
            gripper_state: Arc::new(RwLock::new(GraspState::Open)),
            gripper_opening_m: Arc::new(RwLock::new(0.08)),
            link_lengths,
            force_limit_n,
        }
    }

    pub async fn register_object(&self, object: DetectedObject) {
        info!("Registering object '{}' ({})", object.id, object.label);
        self.detected_objects
            .write()
            .await
            .insert(object.id.clone(), object);
    }

    pub async fn update_force_feedback(&self, feedback: ForceFeedback) {
        let force = feedback.magnitude_n();
        if force > self.force_limit_n * 0.9 {
            warn!("Force feedback approaching limit: {:.2}N / {:.2}N", force, self.force_limit_n);
        }
        *self.force_feedback.write().await = feedback;
    }

    /// Generate grasp candidates for a target object using heuristics.
    pub async fn plan_grasp(&self, object_id: &str) -> Result<Vec<GraspCandidate>> {
        let objects = self.detected_objects.read().await;
        let obj = objects
            .get(object_id)
            .ok_or_else(|| anyhow::anyhow!("Object '{}' not found", object_id))?;

        let center = obj.bounds.center();
        let size_x = obj.bounds.max.x - obj.bounds.min.x;
        let size_y = obj.bounds.max.y - obj.bounds.min.y;
        let size_z = obj.bounds.max.z - obj.bounds.min.z;

        let grasp_type = if size_x.min(size_y).min(size_z) < 0.03 {
            GraspType::Pinch
        } else if obj.fragile {
            GraspType::Precision
        } else {
            GraspType::Power
        };

        // Top-down grasp approach
        let approach_pose = Pose3D {
            x: center.x,
            y: center.y,
            z: center.z + 0.15,
            roll: 0.0,
            pitch: std::f64::consts::FRAC_PI_2,
            yaw: 0.0,
        };

        let grasp_pose = Pose3D {
            x: center.x,
            y: center.y,
            z: center.z + size_z / 2.0,
            roll: 0.0,
            pitch: std::f64::consts::FRAC_PI_2,
            yaw: 0.0,
        };

        let retreat_pose = Pose3D {
            x: center.x,
            y: center.y,
            z: center.z + 0.25,
            roll: 0.0,
            pitch: std::f64::consts::FRAC_PI_2,
            yaw: 0.0,
        };

        let force_limit = if obj.fragile {
            self.force_limit_n * 0.3
        } else {
            self.force_limit_n
        };

        let mut candidates = vec![GraspCandidate {
            id: format!("grasp_top_{}", object_id),
            target_object_id: object_id.to_string(),
            grasp_type,
            approach_pose,
            grasp_pose,
            retreat_pose,
            quality_score: 0.85,
            force_limit_n: force_limit,
            estimated_success_rate: 0.82,
        }];

        // Side grasp if object is tall
        if size_z > 0.1 {
            candidates.push(GraspCandidate {
                id: format!("grasp_side_{}", object_id),
                target_object_id: object_id.to_string(),
                grasp_type: GraspType::Power,
                approach_pose: Pose3D {
                    x: center.x + 0.15,
                    y: center.y,
                    z: center.z,
                    roll: 0.0,
                    pitch: 0.0,
                    yaw: 0.0,
                },
                grasp_pose: Pose3D {
                    x: center.x,
                    y: center.y,
                    z: center.z,
                    roll: 0.0,
                    pitch: 0.0,
                    yaw: 0.0,
                },
                retreat_pose: Pose3D {
                    x: center.x + 0.15,
                    y: center.y,
                    z: center.z + 0.1,
                    roll: 0.0,
                    pitch: 0.0,
                    yaw: 0.0,
                },
                quality_score: 0.75,
                force_limit_n: force_limit,
                estimated_success_rate: 0.72,
            });
        }

        candidates.sort_by(|a, b| {
            b.quality_score
                .partial_cmp(&a.quality_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        debug!("Generated {} grasp candidates for '{}'", candidates.len(), object_id);
        Ok(candidates)
    }

    /// Execute a grasp candidate (simulation / hardware abstraction).
    pub async fn execute_grasp(&self, candidate: GraspCandidate) -> Result<GraspExecution> {
        info!("Executing grasp '{}' on '{}'", candidate.id, candidate.target_object_id);

        let mut execution = GraspExecution {
            candidate: candidate.clone(),
            state: GraspState::Approaching,
            actual_force_n: 0.0,
            slip_detected: false,
            started_at: Utc::now(),
            completed_at: None,
            success: false,
            failure_reason: None,
        };

        *self.gripper_state.write().await = GraspState::Approaching;

        // Check force feedback during grasp
        let current_force = self.force_feedback.read().await.magnitude_n();
        if current_force > candidate.force_limit_n {
            execution.state = GraspState::Failed;
            execution.failure_reason = Some(format!(
                "Force limit exceeded: {:.2}N > {:.2}N",
                current_force, candidate.force_limit_n
            ));
            execution.completed_at = Some(Utc::now());
            self.grasp_history.write().await.push(execution.clone());
            return Ok(execution);
        }

        *self.gripper_state.write().await = GraspState::Grasping;
        execution.state = GraspState::Grasping;
        execution.actual_force_n = current_force;

        *self.gripper_state.write().await = GraspState::Holding;
        execution.state = GraspState::Holding;
        execution.success = true;
        execution.completed_at = Some(Utc::now());

        self.grasp_history.write().await.push(execution.clone());
        info!("Grasp successful: force={:.2}N", execution.actual_force_n);

        Ok(execution)
    }

    pub async fn release(&self) -> Result<()> {
        *self.gripper_state.write().await = GraspState::Releasing;
        *self.gripper_opening_m.write().await = 0.08;
        *self.gripper_state.write().await = GraspState::Open;
        info!("Gripper released");
        Ok(())
    }

    /// Full pick-and-place pipeline.
    pub async fn pick_and_place(
        &self,
        object_id: &str,
        place_pose: Pose3D,
    ) -> Result<ManipulationResult> {
        let t0 = std::time::Instant::now();

        let candidates = self.plan_grasp(object_id).await?;
        let best = candidates
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No grasp candidates found for '{}'", object_id))?;

        let grasp_result = self.execute_grasp(best).await?;

        if !grasp_result.success {
            return Ok(ManipulationResult {
                task_id: format!("pick_place_{}", object_id),
                success: false,
                ik_solution: None,
                grasp_execution: Some(grasp_result),
                final_pose: None,
                duration_ms: t0.elapsed().as_millis() as u64,
                failure_reason: Some("Grasp failed".to_string()),
            });
        }

        // Move to place pose (IK handled by MotorController externally)
        self.release().await?;

        Ok(ManipulationResult {
            task_id: format!("pick_place_{}", object_id),
            success: true,
            ik_solution: None,
            grasp_execution: Some(grasp_result),
            final_pose: Some(place_pose),
            duration_ms: t0.elapsed().as_millis() as u64,
            failure_reason: None,
        })
    }

    pub async fn get_gripper_state(&self) -> GraspState {
        self.gripper_state.read().await.clone()
    }

    pub async fn get_grasp_history(&self) -> Vec<GraspExecution> {
        self.grasp_history.read().await.clone()
    }

    pub async fn grasp_success_rate(&self) -> f64 {
        let history = self.grasp_history.read().await;
        if history.is_empty() {
            return 0.0;
        }
        let successes = history.iter().filter(|e| e.success).count();
        successes as f64 / history.len() as f64
    }
}

impl Default for ManipulationEngine {
    fn default() -> Self {
        Self::new(vec![0.3, 0.25, 0.2, 0.15], 50.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_object(id: &str) -> DetectedObject {
        DetectedObject {
            id: id.to_string(),
            label: "test_object".to_string(),
            bounds: BoundingBox::new(
                Point3D::new(0.0, 0.0, 0.0),
                Point3D::new(0.05, 0.05, 0.1),
            ),
            centroid: Point3D::new(0.025, 0.025, 0.05),
            mass_kg: Some(0.2),
            friction_coefficient: Some(0.5),
            fragile: false,
            confidence: 0.9,
            last_seen: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_plan_grasp() {
        let engine = ManipulationEngine::default();
        engine.register_object(make_object("obj1")).await;
        let candidates = engine.plan_grasp("obj1").await.unwrap();
        assert!(!candidates.is_empty());
        assert!(candidates[0].quality_score >= candidates.last().unwrap().quality_score);
    }

    #[tokio::test]
    async fn test_execute_grasp_success() {
        let engine = ManipulationEngine::default();
        engine.register_object(make_object("obj2")).await;
        let candidates = engine.plan_grasp("obj2").await.unwrap();
        let result = engine.execute_grasp(candidates[0].clone()).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_pick_and_place() {
        let engine = ManipulationEngine::default();
        engine.register_object(make_object("obj3")).await;
        let place_pose = Pose3D {
            x: 0.5,
            y: 0.0,
            z: 0.1,
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
        };
        let result = engine.pick_and_place("obj3", place_pose).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_grasp_success_rate_empty() {
        let engine = ManipulationEngine::default();
        assert_eq!(engine.grasp_success_rate().await, 0.0);
    }
}

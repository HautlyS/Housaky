use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl BoundingRect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }

    pub fn area(&self) -> f64 {
        self.width * self.height
    }

    pub fn center(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn iou(&self, other: &BoundingRect) -> f64 {
        let ix = (self.x + self.width).min(other.x + other.width) - self.x.max(other.x);
        let iy = (self.y + self.height).min(other.y + other.height) - self.y.max(other.y);
        if ix <= 0.0 || iy <= 0.0 {
            return 0.0;
        }
        let intersection = ix * iy;
        let union = self.area() + other.area() - intersection;
        if union <= 0.0 { 0.0 } else { intersection / union }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    pub id: String,
    pub label: String,
    pub class_id: u32,
    pub confidence: f64,
    pub bounding_box: BoundingRect,
    pub mask: Option<Vec<Vec<bool>>>,
    pub keypoints: Vec<(f64, f64, f64)>,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneNode {
    pub object_id: String,
    pub label: String,
    pub position_3d: Option<(f64, f64, f64)>,
    pub relations: Vec<SceneRelation>,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneRelation {
    pub relation_type: SceneRelationType,
    pub target_id: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SceneRelationType {
    On,
    Under,
    NextTo,
    Inside,
    InFrontOf,
    Behind,
    LeftOf,
    RightOf,
    Contains,
    IsA,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneGraph {
    pub nodes: Vec<SceneNode>,
    pub frame_id: u64,
    pub timestamp: DateTime<Utc>,
    pub scene_description: String,
    pub dominant_objects: Vec<String>,
}

impl SceneGraph {
    pub fn new(frame_id: u64) -> Self {
        Self {
            nodes: Vec::new(),
            frame_id,
            timestamp: Utc::now(),
            scene_description: String::new(),
            dominant_objects: Vec::new(),
        }
    }

    pub fn find_node(&self, label: &str) -> Option<&SceneNode> {
        self.nodes.iter().find(|n| n.label.eq_ignore_ascii_case(label))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFrame {
    pub frame_id: u64,
    pub timestamp: DateTime<Utc>,
    pub width: u32,
    pub height: u32,
    pub channel_count: u8,
    pub data_base64: Option<String>,
    pub source: String,
}

impl VideoFrame {
    pub fn new(frame_id: u64, width: u32, height: u32, source: impl Into<String>) -> Self {
        Self {
            frame_id,
            timestamp: Utc::now(),
            width,
            height,
            channel_count: 3,
            data_base64: None,
            source: source.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedFrame {
    pub frame: VideoFrame,
    pub detections: Vec<Detection>,
    pub scene_graph: SceneGraph,
    pub depth_map: Option<Vec<Vec<f32>>>,
    pub processing_time_ms: u64,
    pub model_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectTrack {
    pub track_id: String,
    pub label: String,
    pub history: VecDeque<(BoundingRect, DateTime<Utc>)>,
    pub velocity_px_s: (f64, f64),
    pub last_seen: DateTime<Utc>,
    pub active: bool,
    pub confidence: f64,
}

impl ObjectTrack {
    pub fn new(track_id: impl Into<String>, label: impl Into<String>, initial: BoundingRect) -> Self {
        let now = Utc::now();
        let mut history = VecDeque::new();
        history.push_back((initial, now));
        Self {
            track_id: track_id.into(),
            label: label.into(),
            history,
            velocity_px_s: (0.0, 0.0),
            last_seen: now,
            active: true,
            confidence: 1.0,
        }
    }

    pub fn update(&mut self, new_box: BoundingRect) {
        let now = Utc::now();

        if let Some((prev_box, prev_time)) = self.history.back() {
            let dt = (now - *prev_time).num_milliseconds() as f64 / 1000.0;
            if dt > 0.0 {
                self.velocity_px_s = (
                    (new_box.center().0 - prev_box.center().0) / dt,
                    (new_box.center().1 - prev_box.center().1) / dt,
                );
            }
        }

        if self.history.len() >= 50 {
            self.history.pop_front();
        }
        self.history.push_back((new_box, now));
        self.last_seen = now;
        self.active = true;
    }

    pub fn predict_next(&self, dt_s: f64) -> Option<BoundingRect> {
        self.history.back().map(|(bbox, _)| BoundingRect {
            x: bbox.x + self.velocity_px_s.0 * dt_s,
            y: bbox.y + self.velocity_px_s.1 * dt_s,
            width: bbox.width,
            height: bbox.height,
        })
    }
}

pub struct VisionPipeline {
    pub frame_history: Arc<RwLock<VecDeque<ProcessedFrame>>>,
    pub active_tracks: Arc<RwLock<HashMap<String, ObjectTrack>>>,
    pub frame_counter: Arc<RwLock<u64>>,
    pub max_history: usize,
    pub iou_threshold: f64,
    pub max_track_age_s: f64,
    pub model_name: String,
    pub confidence_threshold: f64,
}

impl VisionPipeline {
    pub fn new(model_name: impl Into<String>, confidence_threshold: f64) -> Self {
        Self {
            frame_history: Arc::new(RwLock::new(VecDeque::new())),
            active_tracks: Arc::new(RwLock::new(HashMap::new())),
            frame_counter: Arc::new(RwLock::new(0)),
            max_history: 300,
            iou_threshold: 0.3,
            max_track_age_s: 2.0,
            model_name: model_name.into(),
            confidence_threshold,
        }
    }

    /// Process a video frame: run detection + scene graph construction + tracking.
    pub async fn process_frame(&self, frame: VideoFrame, detections: Vec<Detection>) -> ProcessedFrame {
        let t0 = std::time::Instant::now();

        let filtered: Vec<Detection> = detections
            .into_iter()
            .filter(|d| d.confidence >= self.confidence_threshold)
            .collect();

        let scene_graph = self.build_scene_graph(frame.frame_id, &filtered);

        let mut tracks = self.active_tracks.write().await;
        self.update_tracks(&mut tracks, &filtered);
        self.age_out_tracks(&mut tracks);
        drop(tracks);

        let mut fc = self.frame_counter.write().await;
        *fc += 1;
        drop(fc);

        let processed = ProcessedFrame {
            frame,
            detections: filtered,
            scene_graph,
            depth_map: None,
            processing_time_ms: t0.elapsed().as_millis() as u64,
            model_name: self.model_name.clone(),
        };

        let mut history = self.frame_history.write().await;
        if history.len() >= self.max_history {
            history.pop_front();
        }
        history.push_back(processed.clone());

        debug!(
            "Processed frame {}: {} detections, {} tracks, {}ms",
            processed.frame.frame_id,
            processed.detections.len(),
            self.active_tracks.read().await.len(),
            processed.processing_time_ms
        );

        processed
    }

    fn build_scene_graph(&self, frame_id: u64, detections: &[Detection]) -> SceneGraph {
        let mut graph = SceneGraph::new(frame_id);

        for det in detections {
            let node = SceneNode {
                object_id: det.id.clone(),
                label: det.label.clone(),
                position_3d: None,
                relations: Vec::new(),
                attributes: det.attributes.clone(),
            };
            graph.nodes.push(node);
        }

        // Infer spatial relations between detections
        for i in 0..detections.len() {
            for j in 0..detections.len() {
                if i == j {
                    continue;
                }
                let a = &detections[i];
                let b = &detections[j];
                let (ax, ay) = a.bounding_box.center();
                let (bx, by) = b.bounding_box.center();

                if let Some(relation) = self.infer_relation(ax, ay, bx, by, &a.bounding_box, &b.bounding_box) {
                    if let Some(node) = graph.nodes.iter_mut().find(|n| n.object_id == a.id) {
                        node.relations.push(SceneRelation {
                            relation_type: relation,
                            target_id: b.id.clone(),
                            confidence: 0.7,
                        });
                    }
                }
            }
        }

        // Build description
        let labels: Vec<String> = detections.iter().map(|d| d.label.clone()).collect();
        graph.dominant_objects = {
            let mut counts: HashMap<String, usize> = HashMap::new();
            for l in &labels {
                *counts.entry(l.clone()).or_insert(0) += 1;
            }
            let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));
            sorted.into_iter().take(5).map(|(l, _)| l).collect()
        };

        graph.scene_description = if graph.dominant_objects.is_empty() {
            "Empty scene".to_string()
        } else {
            format!("Scene contains: {}", graph.dominant_objects.join(", "))
        };

        graph
    }

    fn infer_relation(
        &self,
        ax: f64,
        ay: f64,
        bx: f64,
        by: f64,
        a_box: &BoundingRect,
        b_box: &BoundingRect,
    ) -> Option<SceneRelationType> {
        let dy = ay - by;
        let dx = ax - bx;
        let threshold = (a_box.width.max(a_box.height) + b_box.width.max(b_box.height)) / 4.0;

        if dy < -threshold {
            Some(SceneRelationType::Under)
        } else if dy > threshold {
            Some(SceneRelationType::On)
        } else if dx < -threshold {
            Some(SceneRelationType::LeftOf)
        } else if dx > threshold {
            Some(SceneRelationType::RightOf)
        } else {
            None
        }
    }

    fn update_tracks(&self, tracks: &mut HashMap<String, ObjectTrack>, detections: &[Detection]) {
        let mut matched_det_ids: Vec<String> = Vec::new();

        // Match existing tracks to new detections via IoU
        for track in tracks.values_mut() {
            if !track.active {
                continue;
            }

            let predicted = match track.predict_next(0.033) {
                Some(p) => p,
                None => continue,
            };

            let best = detections
                .iter()
                .filter(|d| d.label == track.label && !matched_det_ids.contains(&d.id))
                .map(|d| (d, predicted.iou(&d.bounding_box)))
                .filter(|(_, iou)| *iou > self.iou_threshold)
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            if let Some((det, _)) = best {
                track.update(det.bounding_box.clone());
                track.confidence = det.confidence;
                matched_det_ids.push(det.id.clone());
            } else {
                track.active = false;
            }
        }

        // Create new tracks for unmatched detections
        for det in detections {
            if matched_det_ids.contains(&det.id) {
                continue;
            }
            let track_id = format!("track_{}_{}", det.label, det.id);
            tracks.insert(
                track_id.clone(),
                ObjectTrack::new(track_id, &det.label, det.bounding_box.clone()),
            );
        }
    }

    fn age_out_tracks(&self, tracks: &mut HashMap<String, ObjectTrack>) {
        let now = Utc::now();
        tracks.retain(|_, track| {
            let age_s = (now - track.last_seen).num_milliseconds() as f64 / 1000.0;
            age_s < self.max_track_age_s
        });
    }

    pub async fn get_active_tracks(&self) -> Vec<ObjectTrack> {
        self.active_tracks
            .read()
            .await
            .values()
            .filter(|t| t.active)
            .cloned()
            .collect()
    }

    pub async fn get_latest_frame(&self) -> Option<ProcessedFrame> {
        self.frame_history.read().await.back().cloned()
    }

    pub async fn get_frames_since(&self, frame_id: u64) -> Vec<ProcessedFrame> {
        self.frame_history
            .read()
            .await
            .iter()
            .filter(|f| f.frame.frame_id > frame_id)
            .cloned()
            .collect()
    }

    pub async fn find_object(&self, label: &str) -> Option<Detection> {
        self.frame_history
            .read()
            .await
            .back()
            .and_then(|f| f.detections.iter().find(|d| d.label.eq_ignore_ascii_case(label)).cloned())
    }

    pub async fn pipeline_stats(&self) -> VisionPipelineStats {
        let history = self.frame_history.read().await;
        let avg_ms = if history.is_empty() {
            0.0
        } else {
            history.iter().map(|f| f.processing_time_ms as f64).sum::<f64>() / history.len() as f64
        };

        VisionPipelineStats {
            frames_processed: *self.frame_counter.read().await,
            active_tracks: self.active_tracks.read().await.values().filter(|t| t.active).count(),
            history_size: history.len(),
            avg_processing_ms: avg_ms,
            model_name: self.model_name.clone(),
            confidence_threshold: self.confidence_threshold,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionPipelineStats {
    pub frames_processed: u64,
    pub active_tracks: usize,
    pub history_size: usize,
    pub avg_processing_ms: f64,
    pub model_name: String,
    pub confidence_threshold: f64,
}

impl Default for VisionPipeline {
    fn default() -> Self {
        Self::new("yolov8n", 0.5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_detection(id: &str, label: &str, x: f64, y: f64, conf: f64) -> Detection {
        Detection {
            id: id.to_string(),
            label: label.to_string(),
            class_id: 0,
            confidence: conf,
            bounding_box: BoundingRect::new(x, y, 50.0, 50.0),
            mask: None,
            keypoints: Vec::new(),
            attributes: HashMap::new(),
        }
    }

    #[test]
    fn test_bounding_rect_iou_perfect() {
        let a = BoundingRect::new(0.0, 0.0, 100.0, 100.0);
        let b = BoundingRect::new(0.0, 0.0, 100.0, 100.0);
        assert!((a.iou(&b) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_bounding_rect_iou_no_overlap() {
        let a = BoundingRect::new(0.0, 0.0, 50.0, 50.0);
        let b = BoundingRect::new(100.0, 100.0, 50.0, 50.0);
        assert_eq!(a.iou(&b), 0.0);
    }

    #[tokio::test]
    async fn test_process_frame_detections() {
        let pipeline = VisionPipeline::default();
        let frame = VideoFrame::new(1, 640, 480, "camera0");
        let dets = vec![
            make_detection("d1", "person", 100.0, 100.0, 0.9),
            make_detection("d2", "chair", 300.0, 200.0, 0.7),
        ];
        let processed = pipeline.process_frame(frame, dets).await;
        assert_eq!(processed.detections.len(), 2);
        assert!(!processed.scene_graph.nodes.is_empty());
    }

    #[tokio::test]
    async fn test_confidence_filter() {
        let pipeline = VisionPipeline::new("test", 0.8);
        let frame = VideoFrame::new(1, 640, 480, "camera0");
        let dets = vec![
            make_detection("d1", "person", 0.0, 0.0, 0.9),
            make_detection("d2", "unknown", 0.0, 0.0, 0.3), // below threshold
        ];
        let processed = pipeline.process_frame(frame, dets).await;
        assert_eq!(processed.detections.len(), 1);
    }

    #[tokio::test]
    async fn test_object_tracking() {
        let pipeline = VisionPipeline::default();

        let frame1 = VideoFrame::new(1, 640, 480, "cam");
        let dets1 = vec![make_detection("d1", "person", 100.0, 100.0, 0.9)];
        pipeline.process_frame(frame1, dets1).await;

        let frame2 = VideoFrame::new(2, 640, 480, "cam");
        let dets2 = vec![make_detection("d2", "person", 110.0, 105.0, 0.85)];
        pipeline.process_frame(frame2, dets2).await;

        let tracks = pipeline.get_active_tracks().await;
        assert!(!tracks.is_empty());
    }
}

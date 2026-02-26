use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

use super::audio_pipeline::{AudioClassification, AudioEventType};
use super::olfactory::{HazardLevel, OdorClassification};
use super::tactile::PressureMap;
use super::vision_pipeline::ProcessedFrame;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PerceptualModality {
    Vision,
    Audio,
    Tactile,
    Olfactory,
    Proprioception,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalityWeight {
    pub modality: PerceptualModality,
    pub weight: f64,
    pub reliability: f64,
    pub last_updated: DateTime<Utc>,
}

impl ModalityWeight {
    pub fn new(modality: PerceptualModality, weight: f64) -> Self {
        Self {
            modality,
            weight,
            reliability: 1.0,
            last_updated: Utc::now(),
        }
    }

    pub fn effective_weight(&self) -> f64 {
        self.weight * self.reliability
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusedPercept {
    pub id: String,
    pub timestamp: DateTime<Utc>,

    // Situational awareness
    pub scene_description: String,
    pub environment_summary: String,
    pub dominant_objects: Vec<String>,
    pub detected_events: Vec<PerceptualEvent>,

    // Safety state
    pub hazard_level: OverallHazardLevel,
    pub immediate_threats: Vec<String>,
    pub safety_summary: String,

    // Sensory state
    pub visual_confidence: f64,
    pub audio_confidence: f64,
    pub tactile_confidence: f64,
    pub olfactory_confidence: f64,

    // Attention direction
    pub attention_focus: Option<String>,
    pub novelty_score: f64,

    // Modalities contributing
    pub active_modalities: Vec<PerceptualModality>,

    pub processing_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OverallHazardLevel {
    Safe,
    Caution,
    Warning,
    Danger,
    Critical,
}

impl OverallHazardLevel {
    pub fn from_components(
        gas_hazard: &HazardLevel,
        force_overload: bool,
        thermal_warning: bool,
        audio_alarm: bool,
    ) -> Self {
        let mut score: u32 = 0;
        match gas_hazard {
            HazardLevel::Critical => score += 4,
            HazardLevel::High => score += 3,
            HazardLevel::Medium => score += 2,
            HazardLevel::Low => score += 1,
            HazardLevel::None => {}
        }
        if force_overload { score += 2; }
        if thermal_warning { score += 2; }
        if audio_alarm { score += 1; }

        match score {
            0 => OverallHazardLevel::Safe,
            1..=2 => OverallHazardLevel::Caution,
            3..=4 => OverallHazardLevel::Warning,
            5..=6 => OverallHazardLevel::Danger,
            _ => OverallHazardLevel::Critical,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptualEvent {
    pub event_type: String,
    pub description: String,
    pub confidence: f64,
    pub source_modality: PerceptualModality,
    pub timestamp: DateTime<Utc>,
    pub requires_attention: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptualConflict {
    pub modality_a: PerceptualModality,
    pub modality_b: PerceptualModality,
    pub description: String,
    pub resolution: String,
    pub confidence_a: f64,
    pub confidence_b: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionInput {
    pub vision: Option<ProcessedFrame>,
    pub audio_classification: Option<AudioClassification>,
    pub tactile_maps: Vec<PressureMap>,
    pub olfactory: Option<OdorClassification>,
    pub proprioceptive_state: Option<ProprioceptiveState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProprioceptiveState {
    pub position_x: f64,
    pub position_y: f64,
    pub position_z: f64,
    pub velocity_magnitude: f64,
    pub orientation_yaw: f64,
    pub joint_angles: Vec<f64>,
    pub is_moving: bool,
    pub is_stable: bool,
}

pub struct PerceptualFusion {
    pub modality_weights: Arc<RwLock<HashMap<String, ModalityWeight>>>,
    pub fused_history: Arc<RwLock<Vec<FusedPercept>>>,
    pub conflict_log: Arc<RwLock<Vec<PerceptualConflict>>>,
    pub attention_weights: Arc<RwLock<HashMap<String, f64>>>,
    pub max_history: usize,
    pub percept_counter: Arc<RwLock<u64>>,
}

impl PerceptualFusion {
    pub fn new() -> Self {
        let mut weights = HashMap::new();
        weights.insert(
            "vision".to_string(),
            ModalityWeight::new(PerceptualModality::Vision, 0.4),
        );
        weights.insert(
            "audio".to_string(),
            ModalityWeight::new(PerceptualModality::Audio, 0.25),
        );
        weights.insert(
            "tactile".to_string(),
            ModalityWeight::new(PerceptualModality::Tactile, 0.20),
        );
        weights.insert(
            "olfactory".to_string(),
            ModalityWeight::new(PerceptualModality::Olfactory, 0.10),
        );
        weights.insert(
            "proprioception".to_string(),
            ModalityWeight::new(PerceptualModality::Proprioception, 0.05),
        );

        Self {
            modality_weights: Arc::new(RwLock::new(weights)),
            fused_history: Arc::new(RwLock::new(Vec::new())),
            conflict_log: Arc::new(RwLock::new(Vec::new())),
            attention_weights: Arc::new(RwLock::new(HashMap::new())),
            max_history: 500,
            percept_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Fuse all available modality inputs into a single `FusedPercept`.
    pub async fn fuse(&self, input: FusionInput) -> Result<FusedPercept> {
        let t0 = std::time::Instant::now();

        let mut active_modalities = Vec::new();
        let mut events: Vec<PerceptualEvent> = Vec::new();
        let mut threats: Vec<String> = Vec::new();

        // ─── Vision ───────────────────────────────────────────────────────────
        let (scene_description, dominant_objects, visual_confidence) =
            if let Some(ref frame) = input.vision {
                active_modalities.push(PerceptualModality::Vision);
                let events_from_vision = self.extract_visual_events(frame);
                for e in events_from_vision {
                    events.push(e);
                }
                (
                    frame.scene_graph.scene_description.clone(),
                    frame.scene_graph.dominant_objects.clone(),
                    frame.detections.iter().map(|d| d.confidence).sum::<f64>()
                        / frame.detections.len().max(1) as f64,
                )
            } else {
                ("No visual input".to_string(), vec![], 0.0)
            };

        // ─── Audio ────────────────────────────────────────────────────────────
        let (audio_summary, audio_confidence) = if let Some(ref audio) = input.audio_classification
        {
            active_modalities.push(PerceptualModality::Audio);
            let audio_event = self.extract_audio_event(audio);
            let is_alarm = matches!(
                audio.primary_event,
                AudioEventType::Alarm | AudioEventType::Crash
            );
            if is_alarm {
                threats.push(format!("Audio alert: {:?}", audio.primary_event));
            }
            events.push(audio_event);
            let conf = audio
                .event_probabilities
                .first()
                .map(|(_, p)| *p)
                .unwrap_or(0.0);
            (format!("{:?}", audio.primary_event), conf)
        } else {
            ("No audio input".to_string(), 0.0)
        };

        // ─── Tactile ──────────────────────────────────────────────────────────
        let (tactile_summary, tactile_confidence, force_overload, thermal_warning) =
            if !input.tactile_maps.is_empty() {
                active_modalities.push(PerceptualModality::Tactile);
                let contacts: Vec<_> = input.tactile_maps.iter().filter(|m| m.is_in_contact(0.05)).collect();
                let max_force = input
                    .tactile_maps
                    .iter()
                    .map(|m| m.max_value)
                    .fold(0.0_f64, f64::max);
                let overload = max_force > 0.9;
                if overload {
                    threats.push(format!("Force overload: {:.2}", max_force));
                    events.push(PerceptualEvent {
                        event_type: "force_overload".to_string(),
                        description: format!("Excessive force detected: {:.2}", max_force),
                        confidence: 0.95,
                        source_modality: PerceptualModality::Tactile,
                        timestamp: Utc::now(),
                        requires_attention: true,
                    });
                }
                (
                    format!("{} contact zones, max force={:.2}", contacts.len(), max_force),
                    0.9,
                    overload,
                    false,
                )
            } else {
                ("No tactile input".to_string(), 0.0, false, false)
            };

        // ─── Olfactory ────────────────────────────────────────────────────────
        let (olfactory_summary, olfactory_confidence, gas_hazard) =
            if let Some(ref odor) = input.olfactory {
                active_modalities.push(PerceptualModality::Olfactory);
                if !matches!(odor.hazard_level, HazardLevel::None) {
                    threats.push(format!("Chemical hazard: {:?}", odor.hazard_level));
                    events.push(PerceptualEvent {
                        event_type: "chemical_hazard".to_string(),
                        description: format!("Hazardous gas detected: {:?}", odor.hazard_level),
                        confidence: 0.90,
                        source_modality: PerceptualModality::Olfactory,
                        timestamp: Utc::now(),
                        requires_attention: true,
                    });
                }
                let aqi_safe = odor.aqi.category.is_safe();
                (
                    format!(
                        "AQI: {:?}, hazard: {:?}",
                        odor.aqi.category, odor.hazard_level
                    ),
                    if aqi_safe { 0.9 } else { 0.7 },
                    odor.hazard_level.clone(),
                )
            } else {
                ("No olfactory input".to_string(), 0.0, HazardLevel::None)
            };

        // ─── Proprioception ───────────────────────────────────────────────────
        if let Some(ref proprio) = input.proprioceptive_state {
            active_modalities.push(PerceptualModality::Proprioception);
            if !proprio.is_stable {
                threats.push("Robot is unstable".to_string());
            }
        }

        // ─── Detect cross-modal conflicts ─────────────────────────────────────
        let conflicts = self.detect_conflicts(&input).await;
        for conflict in &conflicts {
            self.conflict_log.write().await.push(conflict.clone());
        }

        // ─── Compute attention focus ──────────────────────────────────────────
        let attention_focus = self.compute_attention_focus(&events, &dominant_objects).await;

        // ─── Novelty score ────────────────────────────────────────────────────
        let novelty_score = self.compute_novelty(&events).await;

        // ─── Overall hazard ───────────────────────────────────────────────────
        let audio_alarm = input
            .audio_classification
            .as_ref()
            .map(|a| matches!(a.primary_event, AudioEventType::Alarm | AudioEventType::Crash))
            .unwrap_or(false);

        let hazard_level = OverallHazardLevel::from_components(
            &gas_hazard,
            force_overload,
            thermal_warning,
            audio_alarm,
        );

        // ─── Build environment summary ────────────────────────────────────────
        let environment_summary = self.build_environment_summary(
            &scene_description,
            &audio_summary,
            &tactile_summary,
            &olfactory_summary,
            &hazard_level,
        );

        let safety_summary = if threats.is_empty() {
            "All systems nominal".to_string()
        } else {
            format!("THREATS: {}", threats.join("; "))
        };

        let mut counter = self.percept_counter.write().await;
        *counter += 1;
        let percept_id = format!("percept_{}", *counter);
        drop(counter);

        let percept = FusedPercept {
            id: percept_id,
            timestamp: Utc::now(),
            scene_description,
            environment_summary,
            dominant_objects,
            detected_events: events,
            hazard_level,
            immediate_threats: threats,
            safety_summary,
            visual_confidence,
            audio_confidence,
            tactile_confidence,
            olfactory_confidence,
            attention_focus,
            novelty_score,
            active_modalities,
            processing_time_ms: t0.elapsed().as_millis() as u64,
        };

        // Store in history
        let mut history = self.fused_history.write().await;
        if history.len() >= self.max_history {
            history.remove(0);
        }
        history.push(percept.clone());

        debug!(
            "Fused percept '{}': hazard={:?}, events={}, modalities={}, {}ms",
            percept.id,
            percept.hazard_level,
            percept.detected_events.len(),
            percept.active_modalities.len(),
            percept.processing_time_ms
        );

        Ok(percept)
    }

    fn extract_visual_events(&self, frame: &ProcessedFrame) -> Vec<PerceptualEvent> {
        let mut events = Vec::new();

        // Detect person in frame
        for det in &frame.detections {
            if det.label.eq_ignore_ascii_case("person") {
                events.push(PerceptualEvent {
                    event_type: "person_detected".to_string(),
                    description: format!("Person detected with confidence {:.2}", det.confidence),
                    confidence: det.confidence,
                    source_modality: PerceptualModality::Vision,
                    timestamp: frame.frame.timestamp,
                    requires_attention: false,
                });
            }

            // Detect potential hazard objects
            let hazard_labels = ["fire", "smoke", "weapon", "danger"];
            if hazard_labels.iter().any(|l| det.label.to_lowercase().contains(l)) {
                events.push(PerceptualEvent {
                    event_type: "visual_hazard".to_string(),
                    description: format!("Hazardous object detected: {} ({:.2})", det.label, det.confidence),
                    confidence: det.confidence,
                    source_modality: PerceptualModality::Vision,
                    timestamp: frame.frame.timestamp,
                    requires_attention: true,
                });
            }
        }

        events
    }

    fn extract_audio_event(&self, audio: &AudioClassification) -> PerceptualEvent {
        let requires_attention = matches!(
            audio.primary_event,
            AudioEventType::Alarm
                | AudioEventType::Crash
                | AudioEventType::DoorBell
                | AudioEventType::Knock
        );

        let confidence = audio
            .event_probabilities
            .first()
            .map(|(_, p)| *p)
            .unwrap_or(0.0);

        PerceptualEvent {
            event_type: format!("audio_{:?}", audio.primary_event).to_lowercase(),
            description: format!("Audio event: {:?} (conf={:.2})", audio.primary_event, confidence),
            confidence,
            source_modality: PerceptualModality::Audio,
            timestamp: audio.timestamp,
            requires_attention,
        }
    }

    async fn detect_conflicts(&self, input: &FusionInput) -> Vec<PerceptualConflict> {
        let mut conflicts = Vec::new();

        // Conflict: vision says person present but tactile shows no contact at expected position
        if let Some(ref frame) = input.vision {
            let person_detected = frame.detections.iter().any(|d| d.label.eq_ignore_ascii_case("person"));
            if person_detected && !input.tactile_maps.is_empty() {
                let any_contact = input.tactile_maps.iter().any(|m| m.is_in_contact(0.05));
                if !any_contact {
                    // Not a conflict per se — person can be in visual field without touching robot
                    // Only add if proximity sensor disagrees
                }
            }

            // Conflict: audio says silence but vision detects loud activity
            if let Some(ref audio) = input.audio_classification {
                let visually_busy = frame.detections.len() > 5;
                let audio_silent = matches!(audio.primary_event, AudioEventType::Silence);
                if visually_busy && audio_silent {
                    conflicts.push(PerceptualConflict {
                        modality_a: PerceptualModality::Vision,
                        modality_b: PerceptualModality::Audio,
                        description: "Vision detects busy scene but audio reports silence".to_string(),
                        resolution: "Trust vision; audio sensor may be muted or obstructed".to_string(),
                        confidence_a: 0.8,
                        confidence_b: 0.5,
                    });
                }
            }
        }

        // Conflict: olfactory hazard but no visual confirmation
        if let Some(ref odor) = input.olfactory {
            if !matches!(odor.hazard_level, HazardLevel::None) && input.vision.is_none() {
                conflicts.push(PerceptualConflict {
                    modality_a: PerceptualModality::Olfactory,
                    modality_b: PerceptualModality::Vision,
                    description: "Chemical hazard detected but no visual data to confirm source".to_string(),
                    resolution: "Trust olfactory; initiate visual scan".to_string(),
                    confidence_a: 0.85,
                    confidence_b: 0.0,
                });
            }
        }

        conflicts
    }

    async fn compute_attention_focus(
        &self,
        events: &[PerceptualEvent],
        dominant_objects: &[String],
    ) -> Option<String> {
        // Highest priority: events requiring attention
        if let Some(urgent) = events.iter().filter(|e| e.requires_attention).max_by(|a, b| {
            a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal)
        }) {
            return Some(urgent.description.clone());
        }

        // Next: most prominent visual object
        if let Some(obj) = dominant_objects.first() {
            return Some(format!("Dominant object: {}", obj));
        }

        None
    }

    async fn compute_novelty(&self, events: &[PerceptualEvent]) -> f64 {
        let history = self.fused_history.read().await;
        if history.is_empty() {
            return 1.0;
        }

        let last = &history[history.len() - 1];
        let new_event_types: usize = events
            .iter()
            .filter(|e| {
                !last
                    .detected_events
                    .iter()
                    .any(|le| le.event_type == e.event_type)
            })
            .count();

        (new_event_types as f64 / events.len().max(1) as f64).clamp(0.0, 1.0)
    }

    fn build_environment_summary(
        &self,
        scene: &str,
        audio: &str,
        tactile: &str,
        olfactory: &str,
        hazard: &OverallHazardLevel,
    ) -> String {
        format!(
            "Vision: {} | Audio: {} | Tactile: {} | Olfactory: {} | Hazard: {:?}",
            scene, audio, tactile, olfactory, hazard
        )
    }

    pub async fn update_modality_weight(&self, modality: &str, weight: f64, reliability: f64) {
        let mut weights = self.modality_weights.write().await;
        if let Some(w) = weights.get_mut(modality) {
            w.weight = weight;
            w.reliability = reliability;
            w.last_updated = Utc::now();
        }
    }

    pub async fn get_latest_percept(&self) -> Option<FusedPercept> {
        self.fused_history.read().await.last().cloned()
    }

    pub async fn get_percept_history(&self, limit: usize) -> Vec<FusedPercept> {
        let history = self.fused_history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }

    pub async fn get_conflict_log(&self, limit: usize) -> Vec<PerceptualConflict> {
        self.conflict_log
            .read()
            .await
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub async fn get_fusion_stats(&self) -> FusionStats {
        let weights = self.modality_weights.read().await;
        FusionStats {
            percepts_generated: *self.percept_counter.read().await,
            history_size: self.fused_history.read().await.len(),
            conflicts_detected: self.conflict_log.read().await.len(),
            modality_weights: weights
                .iter()
                .map(|(k, w)| (k.clone(), w.effective_weight()))
                .collect(),
        }
    }
}

impl Default for PerceptualFusion {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionStats {
    pub percepts_generated: u64,
    pub history_size: usize,
    pub conflicts_detected: usize,
    pub modality_weights: HashMap<String, f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::housaky::perception::audio_pipeline::{AudioClassification, AudioEventType};
    use crate::housaky::perception::vision_pipeline::{
        Detection, ProcessedFrame, SceneGraph, VideoFrame,
    };
    use std::collections::HashMap;

    fn make_frame() -> ProcessedFrame {
        ProcessedFrame {
            frame: VideoFrame::new(1, 640, 480, "camera"),
            detections: vec![Detection {
                id: "d1".to_string(),
                label: "person".to_string(),
                class_id: 0,
                confidence: 0.9,
                bounding_box: super::super::vision_pipeline::BoundingRect::new(
                    100.0, 100.0, 80.0, 180.0,
                ),
                mask: None,
                keypoints: vec![],
                attributes: HashMap::new(),
            }],
            scene_graph: SceneGraph::new(1),
            depth_map: None,
            processing_time_ms: 15,
            model_name: "yolov8n".to_string(),
        }
    }

    fn make_audio(event_type: AudioEventType) -> AudioClassification {
        AudioClassification {
            chunk_id: 0,
            primary_event: event_type.clone(),
            event_probabilities: vec![(format!("{:?}", event_type).to_lowercase(), 0.8)],
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_fuse_vision_only() {
        let fusion = PerceptualFusion::new();
        let input = FusionInput {
            vision: Some(make_frame()),
            audio_classification: None,
            tactile_maps: vec![],
            olfactory: None,
            proprioceptive_state: None,
        };
        let percept = fusion.fuse(input).await.unwrap();
        assert!(percept.active_modalities.contains(&PerceptualModality::Vision));
        assert_eq!(percept.hazard_level, OverallHazardLevel::Safe);
    }

    #[tokio::test]
    async fn test_fuse_audio_alarm() {
        let fusion = PerceptualFusion::new();
        let input = FusionInput {
            vision: None,
            audio_classification: Some(make_audio(AudioEventType::Alarm)),
            tactile_maps: vec![],
            olfactory: None,
            proprioceptive_state: None,
        };
        let percept = fusion.fuse(input).await.unwrap();
        assert!(!percept.immediate_threats.is_empty());
        assert!(percept.active_modalities.contains(&PerceptualModality::Audio));
    }

    #[tokio::test]
    async fn test_fuse_all_modalities_safe() {
        let fusion = PerceptualFusion::new();
        let input = FusionInput {
            vision: Some(make_frame()),
            audio_classification: Some(make_audio(AudioEventType::Speech)),
            tactile_maps: vec![],
            olfactory: None,
            proprioceptive_state: Some(ProprioceptiveState {
                position_x: 0.0,
                position_y: 0.0,
                position_z: 0.0,
                velocity_magnitude: 0.1,
                orientation_yaw: 0.0,
                joint_angles: vec![0.0; 6],
                is_moving: true,
                is_stable: true,
            }),
        };
        let percept = fusion.fuse(input).await.unwrap();
        assert_eq!(percept.hazard_level, OverallHazardLevel::Safe);
        assert!(percept.active_modalities.len() >= 3);
    }

    #[tokio::test]
    async fn test_novelty_decreases_on_repeat() {
        let fusion = PerceptualFusion::new();
        let make_input = || FusionInput {
            vision: Some(make_frame()),
            audio_classification: Some(make_audio(AudioEventType::Speech)),
            tactile_maps: vec![],
            olfactory: None,
            proprioceptive_state: None,
        };

        let p1 = fusion.fuse(make_input()).await.unwrap();
        let p2 = fusion.fuse(make_input()).await.unwrap();
        // Second percept should have lower or equal novelty
        assert!(p2.novelty_score <= p1.novelty_score + 0.01);
    }

    #[tokio::test]
    async fn test_hazard_from_audio_alarm() {
        let level = OverallHazardLevel::from_components(
            &HazardLevel::None,
            false,
            false,
            true,
        );
        assert_eq!(level, OverallHazardLevel::Caution);
    }

    #[tokio::test]
    async fn test_fusion_stats() {
        let fusion = PerceptualFusion::new();
        fusion
            .fuse(FusionInput {
                vision: None,
                audio_classification: None,
                tactile_maps: vec![],
                olfactory: None,
                proprioceptive_state: None,
            })
            .await
            .unwrap();
        let stats = fusion.get_fusion_stats().await;
        assert_eq!(stats.percepts_generated, 1);
        assert_eq!(stats.history_size, 1);
    }
}

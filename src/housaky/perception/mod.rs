pub mod audio_pipeline;
pub mod fusion;
pub mod olfactory;
pub mod tactile;
pub mod vision_pipeline;

use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub use audio_pipeline::{
    AudioChunk, AudioChunkResult, AudioClassification, AudioEvent, AudioEventType, AudioPipeline,
    AudioPipelineStats, SpectralAnalyzer, TranscriptionResult, TranscriptionSegment,
    VoiceActivityDetector, VoiceActivityResult,
};
pub use fusion::{
    FusedPercept, FusionInput, FusionStats, ModalityWeight, OverallHazardLevel, PerceptualConflict,
    PerceptualEvent, PerceptualFusion, PerceptualModality, ProprioceptiveState,
};
pub use olfactory::{
    AQICategory, AirQualityIndex, ChemicalSensorConfig, ChemicalSensorType, GasAlarm, GasReading,
    HazardLevel, OdorClassification, OdorProfile, OlfactoryProcessor, OlfactoryStats,
};
pub use tactile::{
    PressureMap, TactileEvent, TactileEventType, TactileProcessor, TactileSensorConfig,
    TactileSensorType, TactileStats, ThermalReading, VibrationReading,
};
pub use vision_pipeline::{
    BoundingRect, Detection, ObjectTrack, ProcessedFrame, SceneGraph, SceneNode, SceneRelation,
    SceneRelationType, VideoFrame, VisionPipeline, VisionPipelineStats,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerceptualSystemConfig {
    pub vision_enabled: bool,
    pub audio_enabled: bool,
    pub tactile_enabled: bool,
    pub olfactory_enabled: bool,
}

pub struct PerceptualSystem {
    config: PerceptualSystemConfig,
    vision: Option<Arc<VisionPipeline>>,
    audio: Option<Arc<AudioPipeline>>,
    fusion: Arc<PerceptualFusion>,
}

impl PerceptualSystem {
    pub fn new(config: PerceptualSystemConfig) -> Self {
        let vision = if config.vision_enabled {
            Some(Arc::new(VisionPipeline::new("yolov8".to_string(), 0.5)))
        } else {
            None
        };

        let audio = if config.audio_enabled {
            Some(Arc::new(AudioPipeline::new()))
        } else {
            None
        };

        Self {
            config,
            vision,
            audio,
            fusion: Arc::new(PerceptualFusion::new()),
        }
    }

    pub fn config(&self) -> &PerceptualSystemConfig {
        &self.config
    }
}

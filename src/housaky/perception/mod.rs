pub mod audio_pipeline;
pub mod fusion;
pub mod olfactory;
pub mod tactile;
pub mod vision_pipeline;

pub use audio_pipeline::{
    AudioChunk, AudioChunkResult, AudioClassification, AudioEvent, AudioEventType, AudioPipeline,
    AudioPipelineStats, SpectralAnalyzer, TranscriptionResult, TranscriptionSegment,
    VoiceActivityDetector, VoiceActivityResult,
};
pub use fusion::{
    FusedPercept, FusionInput, FusionStats, ModalityWeight, OverallHazardLevel,
    PerceptualConflict, PerceptualEvent, PerceptualFusion, PerceptualModality, ProprioceptiveState,
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

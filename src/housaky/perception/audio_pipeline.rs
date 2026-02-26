use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AudioEventType {
    Speech,
    Music,
    Noise,
    Silence,
    Alarm,
    Knock,
    Crash,
    Footstep,
    DoorBell,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioChunk {
    pub id: u64,
    pub samples: Vec<f32>,
    pub sample_rate_hz: u32,
    pub channels: u8,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
    pub source: String,
}

impl AudioChunk {
    pub fn new(
        id: u64,
        samples: Vec<f32>,
        sample_rate_hz: u32,
        channels: u8,
        source: impl Into<String>,
    ) -> Self {
        let duration_ms = if sample_rate_hz > 0 && channels > 0 {
            (samples.len() as u64 * 1000) / (sample_rate_hz as u64 * channels as u64)
        } else {
            0
        };
        Self {
            id,
            samples,
            sample_rate_hz,
            channels,
            duration_ms,
            timestamp: Utc::now(),
            source: source.into(),
        }
    }

    pub fn rms_energy(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f64 = self.samples.iter().map(|&s| (s as f64).powi(2)).sum();
        (sum_sq / self.samples.len() as f64).sqrt()
    }

    pub fn peak_amplitude(&self) -> f64 {
        self.samples.iter().map(|&s| s.abs() as f64).fold(0.0_f64, f64::max)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceActivityResult {
    pub chunk_id: u64,
    pub is_speech: bool,
    pub speech_probability: f64,
    pub energy_db: f64,
    pub timestamp: DateTime<Utc>,
}

/// Simple energy-based Voice Activity Detection.
pub struct VoiceActivityDetector {
    pub energy_threshold: f64,
    pub min_speech_duration_ms: u64,
    pub hangover_ms: u64,
    active_since: Option<DateTime<Utc>>,
    hangover_remaining_ms: u64,
}

impl VoiceActivityDetector {
    pub fn new(energy_threshold: f64) -> Self {
        Self {
            energy_threshold,
            min_speech_duration_ms: 100,
            hangover_ms: 300,
            active_since: None,
            hangover_remaining_ms: 0,
        }
    }

    pub fn process(&mut self, chunk: &AudioChunk) -> VoiceActivityResult {
        let energy = chunk.rms_energy();
        let energy_db = if energy > 1e-10 {
            20.0 * energy.log10()
        } else {
            -100.0
        };

        let above_threshold = energy > self.energy_threshold;

        let is_speech = if above_threshold {
            self.active_since.get_or_insert(Utc::now());
            self.hangover_remaining_ms = self.hangover_ms;
            let active_ms = (Utc::now() - self.active_since.unwrap())
                .num_milliseconds()
                .max(0) as u64;
            active_ms >= self.min_speech_duration_ms
        } else if self.hangover_remaining_ms > 0 {
            self.hangover_remaining_ms =
                self.hangover_remaining_ms.saturating_sub(chunk.duration_ms);
            true
        } else {
            self.active_since = None;
            false
        };

        let speech_probability = (energy / self.energy_threshold).clamp(0.0, 1.0);

        VoiceActivityResult {
            chunk_id: chunk.id,
            is_speech,
            speech_probability,
            energy_db,
            timestamp: chunk.timestamp,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEvent {
    pub id: String,
    pub event_type: AudioEventType,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub confidence: f64,
    pub source: String,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub confidence: f64,
    pub speaker_id: Option<String>,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub id: String,
    pub full_text: String,
    pub segments: Vec<TranscriptionSegment>,
    pub language: String,
    pub overall_confidence: f64,
    pub duration_ms: u64,
    pub processing_time_ms: u64,
    pub timestamp: DateTime<Utc>,
}

impl TranscriptionResult {
    pub fn empty(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            full_text: String::new(),
            segments: Vec::new(),
            language: "unknown".to_string(),
            overall_confidence: 0.0,
            duration_ms: 0,
            processing_time_ms: 0,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioClassification {
    pub chunk_id: u64,
    pub primary_event: AudioEventType,
    pub event_probabilities: Vec<(String, f64)>,
    pub timestamp: DateTime<Utc>,
}

/// Spectral feature extractor for audio classification.
pub struct SpectralAnalyzer {
    pub sample_rate: u32,
    pub fft_size: usize,
}

impl SpectralAnalyzer {
    pub fn new(sample_rate: u32, fft_size: usize) -> Self {
        Self { sample_rate, fft_size }
    }

    /// Compute zero-crossing rate — a simple discriminant between speech and noise.
    pub fn zero_crossing_rate(&self, samples: &[f32]) -> f64 {
        if samples.len() < 2 {
            return 0.0;
        }
        let crossings = samples
            .windows(2)
            .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
            .count();
        crossings as f64 / (samples.len() - 1) as f64
    }

    /// Compute spectral centroid from magnitude spectrum.
    pub fn spectral_centroid(&self, samples: &[f32]) -> f64 {
        if samples.is_empty() {
            return 0.0;
        }
        let magnitudes: Vec<f64> = samples
            .iter()
            .take(self.fft_size / 2)
            .enumerate()
            .map(|(_, &s)| (s as f64).abs())
            .collect();

        let total_mag: f64 = magnitudes.iter().sum();
        if total_mag < 1e-10 {
            return 0.0;
        }

        let weighted: f64 = magnitudes
            .iter()
            .enumerate()
            .map(|(i, &m)| i as f64 * m)
            .sum();

        let bin_hz = self.sample_rate as f64 / self.fft_size as f64;
        (weighted / total_mag) * bin_hz
    }

    /// Classify audio based on zero-crossing rate and energy.
    pub fn classify(&self, chunk: &AudioChunk) -> AudioClassification {
        let zcr = self.zero_crossing_rate(&chunk.samples);
        let energy = chunk.rms_energy();
        let centroid = self.spectral_centroid(&chunk.samples);

        // Heuristic classification
        let (primary, probs) = if energy < 1e-4 {
            (
                AudioEventType::Silence,
                vec![
                    ("silence".to_string(), 0.95),
                    ("noise".to_string(), 0.05),
                ],
            )
        } else if zcr > 0.1 && centroid > 2000.0 {
            (
                AudioEventType::Speech,
                vec![
                    ("speech".to_string(), 0.80),
                    ("music".to_string(), 0.12),
                    ("noise".to_string(), 0.08),
                ],
            )
        } else if zcr < 0.05 && centroid < 500.0 {
            (
                AudioEventType::Music,
                vec![
                    ("music".to_string(), 0.70),
                    ("speech".to_string(), 0.20),
                    ("noise".to_string(), 0.10),
                ],
            )
        } else if energy > 0.5 {
            (
                AudioEventType::Crash,
                vec![
                    ("crash".to_string(), 0.60),
                    ("alarm".to_string(), 0.25),
                    ("noise".to_string(), 0.15),
                ],
            )
        } else {
            (
                AudioEventType::Noise,
                vec![
                    ("noise".to_string(), 0.70),
                    ("silence".to_string(), 0.20),
                    ("unknown".to_string(), 0.10),
                ],
            )
        };

        AudioClassification {
            chunk_id: chunk.id,
            primary_event: primary,
            event_probabilities: probs,
            timestamp: chunk.timestamp,
        }
    }
}

pub struct AudioPipeline {
    pub vad: Arc<RwLock<VoiceActivityDetector>>,
    pub spectral_analyzer: Arc<SpectralAnalyzer>,
    pub chunk_history: Arc<RwLock<VecDeque<AudioChunk>>>,
    pub transcriptions: Arc<RwLock<VecDeque<TranscriptionResult>>>,
    pub detected_events: Arc<RwLock<VecDeque<AudioEvent>>>,
    pub classifications: Arc<RwLock<VecDeque<AudioClassification>>>,
    pub speech_buffer: Arc<RwLock<Vec<AudioChunk>>>,
    pub max_history: usize,
    pub chunk_counter: Arc<RwLock<u64>>,
}

impl AudioPipeline {
    pub fn new() -> Self {
        Self {
            vad: Arc::new(RwLock::new(VoiceActivityDetector::new(0.01))),
            spectral_analyzer: Arc::new(SpectralAnalyzer::new(16_000, 512)),
            chunk_history: Arc::new(RwLock::new(VecDeque::new())),
            transcriptions: Arc::new(RwLock::new(VecDeque::new())),
            detected_events: Arc::new(RwLock::new(VecDeque::new())),
            classifications: Arc::new(RwLock::new(VecDeque::new())),
            speech_buffer: Arc::new(RwLock::new(Vec::new())),
            max_history: 500,
            chunk_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Process an audio chunk through the full pipeline:
    /// VAD → classification → optional speech buffering.
    pub async fn process_chunk(&self, mut chunk: AudioChunk) -> Result<AudioChunkResult> {
        let t0 = std::time::Instant::now();

        // Assign sequential ID
        {
            let mut counter = self.chunk_counter.write().await;
            chunk.id = *counter;
            *counter += 1;
        }

        let classification = self.spectral_analyzer.classify(&chunk);
        let vad_result = self.vad.write().await.process(&chunk);

        // Buffer speech segments for eventual transcription
        if vad_result.is_speech {
            self.speech_buffer.write().await.push(chunk.clone());
        } else {
            // Speech segment ended — flush buffer
            let buf = self.speech_buffer.write().await;
            if !buf.is_empty() {
                let transcription = self.transcribe_buffer(&buf).await;
                drop(buf);

                let mut transcriptions = self.transcriptions.write().await;
                if transcriptions.len() >= self.max_history {
                    transcriptions.pop_front();
                }
                transcriptions.push_back(transcription.clone());
                drop(transcriptions);

                info!("Speech transcribed: \"{}\"", transcription.full_text);
            } else {
                drop(buf);
            }

            *self.speech_buffer.write().await = Vec::new();
        }

        // Detect notable audio events
        if let Some(event) = self.detect_event(&classification, &chunk) {
            let mut events = self.detected_events.write().await;
            if events.len() >= self.max_history {
                events.pop_front();
            }
            events.push_back(event);
        }

        // Store classification
        {
            let mut classes = self.classifications.write().await;
            if classes.len() >= self.max_history {
                classes.pop_front();
            }
            classes.push_back(classification.clone());
        }

        // Store chunk
        {
            let mut history = self.chunk_history.write().await;
            if history.len() >= self.max_history {
                history.pop_front();
            }
            history.push_back(chunk.clone());
        }

        let processing_ms = t0.elapsed().as_millis() as u64;
        debug!(
            "Chunk {}: {:?}, speech={}, energy={:.4}, {}ms",
            chunk.id,
            classification.primary_event,
            vad_result.is_speech,
            chunk.rms_energy(),
            processing_ms
        );

        Ok(AudioChunkResult {
            chunk_id: chunk.id,
            vad: vad_result,
            classification,
            processing_ms,
        })
    }

    /// Assemble buffered speech chunks and produce a transcription.
    async fn transcribe_buffer(&self, buffer: &[AudioChunk]) -> TranscriptionResult {
        if buffer.is_empty() {
            return TranscriptionResult::empty("empty");
        }

        let total_duration_ms: u64 = buffer.iter().map(|c| c.duration_ms).sum();
        let total_energy: f64 = buffer.iter().map(|c| c.rms_energy()).sum::<f64>() / buffer.len() as f64;

        // In production this calls Whisper/Vosk/etc. via ONNX Runtime or HTTP.
        // Here we produce a placeholder reflecting real signal characteristics.
        let placeholder_text = if total_energy > 0.02 {
            "[speech detected — transcription requires ONNX/Whisper runtime]"
        } else {
            "[low-energy speech — possibly whispered or distant]"
        };

        let id = format!("tr_{}", Utc::now().timestamp_millis());
        TranscriptionResult {
            id,
            full_text: placeholder_text.to_string(),
            segments: vec![TranscriptionSegment {
                text: placeholder_text.to_string(),
                start_ms: 0,
                end_ms: total_duration_ms,
                confidence: 0.5,
                speaker_id: None,
                language: "en".to_string(),
            }],
            language: "en".to_string(),
            overall_confidence: 0.5,
            duration_ms: total_duration_ms,
            processing_time_ms: 0,
            timestamp: Utc::now(),
        }
    }

    fn detect_event(
        &self,
        classification: &AudioClassification,
        chunk: &AudioChunk,
    ) -> Option<AudioEvent> {
        let notable = matches!(
            classification.primary_event,
            AudioEventType::Alarm
                | AudioEventType::Crash
                | AudioEventType::DoorBell
                | AudioEventType::Knock
        );

        if !notable {
            return None;
        }

        let top_confidence = classification
            .event_probabilities
            .first()
            .map(|(_, p)| *p)
            .unwrap_or(0.0);

        if top_confidence < 0.5 {
            return None;
        }

        Some(AudioEvent {
            id: format!("evt_{}", chunk.id),
            event_type: classification.primary_event.clone(),
            start_time: chunk.timestamp,
            end_time: None,
            confidence: top_confidence,
            source: chunk.source.clone(),
            metadata: std::collections::HashMap::new(),
        })
    }

    pub async fn get_latest_transcription(&self) -> Option<TranscriptionResult> {
        self.transcriptions.read().await.back().cloned()
    }

    pub async fn get_recent_events(&self, limit: usize) -> Vec<AudioEvent> {
        self.detected_events
            .read()
            .await
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub async fn get_latest_classification(&self) -> Option<AudioClassification> {
        self.classifications.read().await.back().cloned()
    }

    pub async fn get_pipeline_stats(&self) -> AudioPipelineStats {
        AudioPipelineStats {
            chunks_processed: *self.chunk_counter.read().await,
            transcriptions: self.transcriptions.read().await.len(),
            events_detected: self.detected_events.read().await.len(),
            speech_buffer_size: self.speech_buffer.read().await.len(),
            history_size: self.chunk_history.read().await.len(),
        }
    }
}

impl Default for AudioPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioChunkResult {
    pub chunk_id: u64,
    pub vad: VoiceActivityResult,
    pub classification: AudioClassification,
    pub processing_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioPipelineStats {
    pub chunks_processed: u64,
    pub transcriptions: usize,
    pub events_detected: usize,
    pub speech_buffer_size: usize,
    pub history_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chunk(id: u64, energy_scale: f32, n: usize) -> AudioChunk {
        let samples: Vec<f32> = (0..n)
            .map(|i| energy_scale * (i as f32 * 0.1).sin())
            .collect();
        AudioChunk::new(id, samples, 16_000, 1, "microphone")
    }

    #[test]
    fn test_rms_energy_silence() {
        let chunk = AudioChunk::new(0, vec![0.0; 1000], 16_000, 1, "mic");
        assert_eq!(chunk.rms_energy(), 0.0);
    }

    #[test]
    fn test_rms_energy_nonzero() {
        let chunk = make_chunk(0, 0.5, 1000);
        assert!(chunk.rms_energy() > 0.0);
    }

    #[test]
    fn test_vad_silence_no_speech() {
        let mut vad = VoiceActivityDetector::new(0.1);
        let chunk = AudioChunk::new(0, vec![0.0; 1600], 16_000, 1, "mic");
        let result = vad.process(&chunk);
        assert!(!result.is_speech);
    }

    #[test]
    fn test_vad_loud_is_speech() {
        let mut vad = VoiceActivityDetector::new(0.001);
        vad.min_speech_duration_ms = 0;
        let chunk = make_chunk(0, 0.5, 1600);
        let result = vad.process(&chunk);
        assert!(result.is_speech);
    }

    #[test]
    fn test_spectral_silence_classification() {
        let analyzer = SpectralAnalyzer::new(16_000, 512);
        let chunk = AudioChunk::new(0, vec![0.0; 1000], 16_000, 1, "mic");
        let class = analyzer.classify(&chunk);
        assert_eq!(class.primary_event, AudioEventType::Silence);
    }

    #[tokio::test]
    async fn test_process_chunk_speech() {
        let pipeline = AudioPipeline::new();
        // Use a very low VAD threshold so the test chunk registers as speech
        *pipeline.vad.write().await = VoiceActivityDetector::new(0.0001);
        pipeline.vad.write().await.min_speech_duration_ms = 0;

        let chunk = make_chunk(0, 0.3, 16_000);
        let result = pipeline.process_chunk(chunk).await.unwrap();
        assert!(result.vad.speech_probability > 0.0);
    }

    #[tokio::test]
    async fn test_pipeline_stats_increment() {
        let pipeline = AudioPipeline::new();
        for i in 0..5 {
            let chunk = make_chunk(i, 0.0, 100);
            pipeline.process_chunk(chunk).await.unwrap();
        }
        let stats = pipeline.get_pipeline_stats().await;
        assert_eq!(stats.chunks_processed, 5);
    }
}

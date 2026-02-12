//! Photon detector using camera sensors

use anyhow::Result;
use image::{ImageBuffer, Rgb};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Configuration for photon detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorConfig {
    /// Camera index to use
    pub camera_index: u32,
    /// Capture resolution width
    pub width: u32,
    /// Capture resolution height
    pub height: u32,
    /// Frames per second
    pub fps: u32,
    /// Exposure time in microseconds
    pub exposure_us: u32,
    /// Gain value
    pub gain: f32,
    /// Threshold for photon detection (0-255)
    pub threshold: u8,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            camera_index: 0,
            width: 640,
            height: 480,
            fps: 30,
            exposure_us: 10000,
            gain: 1.0,
            threshold: 128,
        }
    }
}

/// A detected photon event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotonEvent {
    /// X coordinate in image
    pub x: u32,
    /// Y coordinate in image
    pub y: u32,
    /// Intensity value (0-255)
    pub intensity: u8,
    /// Timestamp in microseconds
    pub timestamp_us: u64,
}

/// Photon detector using camera
pub struct PhotonDetector {
    config: DetectorConfig,
    frame_tx: Option<mpsc::Sender<ImageBuffer<Rgb<u8>, Vec<u8>>>>,
    event_tx: mpsc::Sender<PhotonEvent>,
    running: bool,
}

impl PhotonDetector {
    /// Create a new photon detector
    pub fn new(config: DetectorConfig) -> (Self, mpsc::Receiver<PhotonEvent>) {
        let (event_tx, event_rx) = mpsc::channel(1000);

        let detector = Self {
            config,
            frame_tx: None,
            event_tx,
            running: false,
        };

        (detector, event_rx)
    }

    /// Start detecting photons
    pub async fn start(&mut self) -> Result<()> {
        if self.running {
            return Err(anyhow::anyhow!("Detector already running"));
        }

        self.running = true;

        // In a real implementation, this would initialize the camera
        // For now, we simulate detection
        let event_tx = self.event_tx.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_millis(1000 / config.fps as u64));

            while let Ok(_) = event_tx.try_send(PhotonEvent {
                x: rand::random::<u32>() % config.width,
                y: rand::random::<u32>() % config.height,
                intensity: rand::random::<u8>(),
                timestamp_us: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_micros() as u64,
            }) {
                interval.tick().await;
            }
        });

        tracing::info!("Photon detector started");
        Ok(())
    }

    /// Stop the detector
    pub fn stop(&mut self) {
        self.running = false;
        tracing::info!("Photon detector stopped");
    }

    /// Get current configuration
    pub fn config(&self) -> &DetectorConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: DetectorConfig) {
        self.config = config;
        tracing::info!("Detector configuration updated");
    }
}

/// Multiple detector array for increased resolution
pub struct DetectorArray {
    detectors: Vec<PhotonDetector>,
}

impl DetectorArray {
    /// Create an array of detectors
    pub fn new(count: usize, base_config: DetectorConfig) -> Vec<mpsc::Receiver<PhotonEvent>> {
        let mut receivers = Vec::new();

        for i in 0..count {
            let mut config = base_config.clone();
            config.camera_index = i as u32;
            let (detector, rx) = PhotonDetector::new(config);
            receivers.push(rx);
        }

        receivers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_config() {
        let config = DetectorConfig::default();
        assert_eq!(config.width, 640);
        assert_eq!(config.height, 480);
    }

    #[test]
    fn test_photon_event() {
        let event = PhotonEvent {
            x: 100,
            y: 200,
            intensity: 255,
            timestamp_us: 1234567890,
        };

        assert_eq!(event.x, 100);
        assert_eq!(event.intensity, 255);
    }
}

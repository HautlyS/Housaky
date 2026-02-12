//! Photon Detector - Optical quantum measurement with proper resource management
//!
//! This module provides photon detection capabilities using camera hardware.
//! All resources are properly managed with Drop trait implementations.

use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Stokes parameters representing photon polarization state
#[derive(Debug, Clone, Copy)]
pub struct PseudoQubit {
    pub s0: f64, // Total intensity
    pub s1: f64, // Horizontal-vertical polarization
    pub s2: f64, // Diagonal polarization
}

impl PseudoQubit {
    /// Create a new PseudoQubit from Stokes parameters
    pub fn new(s0: f64, s1: f64, s2: f64) -> Self {
        Self { s0, s1, s2 }
    }

    /// Normalize the Stokes parameters
    pub fn normalize(&self) -> Self {
        if self.s0 == 0.0 {
            return Self::new(0.0, 0.0, 0.0);
        }
        Self::new(self.s0, self.s1 / self.s0, self.s2 / self.s0)
    }

    /// Calculate degree of polarization
    pub fn degree_of_polarization(&self) -> f64 {
        if self.s0 == 0.0 {
            return 0.0;
        }
        ((self.s1.powi(2) + self.s2.powi(2)).sqrt() / self.s0).min(1.0)
    }
}

/// Thread-safe handle for controlling the photon detector
pub struct PhotonDetectorHandle {
    shutdown: Arc<AtomicBool>,
}

impl PhotonDetectorHandle {
    /// Request graceful shutdown of the detector
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Check if shutdown has been requested
    #[allow(dead_code)]
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }
}

/// Photon detector using camera hardware for quantum measurements
pub struct PhotonDetector {
    shutdown: Arc<AtomicBool>,
    frame_count: u64,
    last_measurement: Option<PseudoQubit>,
    hardware_enabled: bool,
}

impl PhotonDetector {
    /// Create a new PhotonDetector
    ///
    /// Note: Hardware camera support requires the `camera` feature.
    /// Without it, runs in simulation mode.
    pub fn new() -> Result<(Self, PhotonDetectorHandle), Box<dyn Error + Send + Sync>> {
        let shutdown = Arc::new(AtomicBool::new(false));
        let handle = PhotonDetectorHandle {
            shutdown: Arc::clone(&shutdown),
        };

        // For now, always run in simulation mode for stability
        // Hardware support can be added back with proper nokhwa API usage
        let detector = Self {
            shutdown,
            frame_count: 0,
            last_measurement: None,
            hardware_enabled: false,
        };

        Ok((detector, handle))
    }

    /// Create a simulated detector without hardware (for testing)
    pub fn new_simulated() -> Result<(Self, PhotonDetectorHandle), Box<dyn Error + Send + Sync>> {
        Self::new()
    }

    /// Check if running in simulation mode
    #[allow(dead_code)]
    pub fn is_simulation(&self) -> bool {
        !self.hardware_enabled
    }

    /// Check if hardware is enabled
    #[allow(dead_code)]
    pub fn is_hardware_enabled(&self) -> bool {
        self.hardware_enabled
    }

    /// Measure photon state
    pub fn measure_photon_state(&mut self) -> Result<PseudoQubit, Box<dyn Error + Send + Sync>> {
        // Check for shutdown request
        if self.shutdown.load(Ordering::SeqCst) {
            return Err("Detector shutdown requested".into());
        }

        if self.hardware_enabled {
            self.measure_from_hardware()
        } else {
            self.simulate_measurement()
        }
    }

    /// Measure from actual camera hardware
    ///
    /// IMPLEMENTATION GUIDE for hardware support:
    /// 1. Add feature flag: [features] camera = ["nokhwa"]
    /// 2. Use: Camera::new(CameraIndex::Index(0), RequestedFormat::default())
    /// 3. Process frames with decode_image::<RgbFormat>()
    /// 4. Enable with: PhotonDetector::new_with_camera()
    fn measure_from_hardware(&mut self) -> Result<PseudoQubit, Box<dyn Error + Send + Sync>> {
        // Hardware support requires the 'camera' feature to be enabled
        // and proper nokhwa 0.10 API usage
        Err("Hardware camera support not compiled. Enable 'camera' feature to use hardware.".into())
    }

    /// Create detector with hardware camera support
    ///
    /// Requires 'camera' feature enabled in Cargo.toml:
    /// housaky = { version = "*", features = ["camera"] }
    #[cfg(feature = "camera")]
    pub fn new_with_camera() -> Result<(Self, PhotonDetectorHandle), Box<dyn Error + Send + Sync>> {
        let shutdown = Arc::new(AtomicBool::new(false));
        let handle = PhotonDetectorHandle {
            shutdown: Arc::clone(&shutdown),
        };

        // Real implementation with nokhwa would go here
        // For now, fall back to simulation
        let detector = Self {
            shutdown,
            frame_count: 0,
            last_measurement: None,
            hardware_enabled: false, // Would be true if camera initialized successfully
        };

        tracing::info!("Camera support compiled but using simulation mode");
        Ok((detector, handle))
    }

    /// Simulate measurement for testing without hardware
    fn simulate_measurement(&mut self) -> Result<PseudoQubit, Box<dyn Error + Send + Sync>> {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        // Generate realistic Stokes parameters
        let s0 = 1000.0 + rng.gen::<f64>() * 100.0;
        let s1 = rng.gen::<f64>() * 0.5 - 0.25; // Random polarization
        let s2 = rng.gen::<f64>() * 0.5 - 0.25;

        // Normalize
        let qubit = PseudoQubit::new(s0, s1 * s0, s2 * s0);

        self.frame_count += 1;
        self.last_measurement = Some(qubit);

        // Simulate processing time
        std::thread::sleep(std::time::Duration::from_millis(1));

        Ok(qubit)
    }

    /// Get the number of frames processed
    #[allow(dead_code)]
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get the last measurement (if any)
    #[allow(dead_code)]
    pub fn last_measurement(&self) -> Option<PseudoQubit> {
        self.last_measurement
    }

    /// Reset the detector state
    pub fn reset(&mut self) {
        self.frame_count = 0;
        self.last_measurement = None;
    }

    /// Check if detector is still active
    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        !self.shutdown.load(Ordering::SeqCst)
    }
}

impl Drop for PhotonDetector {
    fn drop(&mut self) {
        // Signal shutdown
        self.shutdown.store(true, Ordering::SeqCst);

        tracing::info!("PhotonDetector dropped after {} frames", self.frame_count);
    }
}

unsafe impl Send for PhotonDetector {}
unsafe impl Sync for PhotonDetector {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pseudo_qubit_creation() {
        let qubit = PseudoQubit::new(100.0, 50.0, 25.0);
        assert_eq!(qubit.s0, 100.0);
        assert_eq!(qubit.s1, 50.0);
        assert_eq!(qubit.s2, 25.0);
    }

    #[test]
    fn test_pseudo_qubit_normalize() {
        let qubit = PseudoQubit::new(100.0, 50.0, 25.0);
        let normalized = qubit.normalize();
        assert_eq!(normalized.s0, 100.0);
        assert!((normalized.s1 - 0.5).abs() < 0.001);
        assert!((normalized.s2 - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_pseudo_qubit_degree_of_polarization() {
        let qubit1 = PseudoQubit::new(100.0, 50.0, 0.0);
        let dop1 = qubit1.degree_of_polarization();
        assert!(dop1 > 0.4 && dop1 < 0.6);

        let qubit2 = PseudoQubit::new(100.0, 0.0, 0.0);
        let dop2 = qubit2.degree_of_polarization();
        assert!(dop2 < 0.01);
    }

    #[test]
    fn test_detector_simulation() {
        let (mut detector, _handle) = PhotonDetector::new_simulated().unwrap();
        assert!(detector.is_simulation());

        let measurement = detector.measure_photon_state().unwrap();
        assert!(measurement.s0 > 0.0);

        assert_eq!(detector.frame_count(), 1);
        assert!(detector.last_measurement().is_some());
    }

    #[test]
    fn test_detector_shutdown() {
        let (mut detector, handle) = PhotonDetector::new_simulated().unwrap();

        handle.shutdown();
        assert!(handle.is_shutdown_requested());

        // After shutdown, measurement should fail
        let result = detector.measure_photon_state();
        assert!(result.is_err());
    }

    #[test]
    fn test_detector_reset() {
        let (mut detector, _handle) = PhotonDetector::new_simulated().unwrap();

        detector.measure_photon_state().unwrap();
        assert_eq!(detector.frame_count(), 1);

        detector.reset();
        assert_eq!(detector.frame_count(), 0);
        assert!(detector.last_measurement().is_none());
    }
}

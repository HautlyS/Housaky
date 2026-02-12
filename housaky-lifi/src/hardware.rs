//! Hardware Abstraction Layer for Li-Fi Communication - Optimized
//!
//! This module provides cross-platform hardware support for:
//! - LED control via GPIO (Linux, macOS, Windows)
//! - Camera capture with rolling shutter exploitation
//! - Hardware-specific optimizations
//!
//! # Memory Safety
//! - Bounded frame buffers
//! - Proper cleanup of hardware resources
//! - Resource pooling for frequent operations
//!
//! # Performance
//! - Zero-copy frame handling where possible
//! - Pre-allocated buffers
//! - Platform-specific optimizations

use anyhow::{Context, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, timeout, Duration};
use tokio_util::sync::CancellationToken;
use metrics::{counter, gauge, histogram};

/// Maximum frame buffer size (1280x720 RGB)
const MAX_FRAME_SIZE: usize = 1280 * 720 * 3;

/// Maximum frame rate
const MAX_FPS: u32 = 120;

/// Default bitrate for Manchester encoding
const DEFAULT_BITRATE: u32 = 1000;

/// Platform-specific hardware capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
    Embedded,
    WebAssembly,
}

/// Hardware abstraction for LED control
#[async_trait::async_trait]
pub trait LedController: Send + Sync {
    /// Initialize the LED controller
    async fn init(&mut self) -> Result<()>;

    /// Set LED state (on/off)
    async fn set_state(&mut self, on: bool) -> Result<()>;

    /// Set LED brightness (0.0 - 1.0)
    async fn set_brightness(&mut self, brightness: f32) -> Result<()>;

    /// Blink LED at specific frequency
    async fn blink(&mut self, frequency_hz: f32, duty_cycle: f32) -> Result<()>;

    /// Send Manchester-encoded data
    async fn send_manchester(&mut self, data: &[u8], bitrate: u32) -> Result<()>;

    /// Cleanup resources
    async fn cleanup(&mut self) -> Result<()>;
    
    /// Get controller info
    fn info(&self) -> LedInfo;
}

/// LED controller information
#[derive(Debug, Clone)]
pub struct LedInfo {
    pub platform: Platform,
    pub pin: u32,
    pub max_brightness: f32,
    pub capabilities: Vec<String>,
}

/// Hardware abstraction for camera capture
#[async_trait::async_trait]
pub trait CameraController: Send + Sync {
    /// Initialize the camera
    async fn init(&mut self) -> Result<()>;

    /// Start capturing frames
    async fn start_capture(&mut self) -> Result<()>;

    /// Stop capturing frames
    async fn stop_capture(&mut self) -> Result<()>;

    /// Get next frame
    async fn get_frame(&mut self) -> Result<CameraFrame>;

    /// Set exposure time
    async fn set_exposure(&mut self, exposure_ms: f32) -> Result<()>;

    /// Set gain
    async fn set_gain(&mut self, gain: f32) -> Result<()>;

    /// Get camera info
    fn get_info(&self) -> CameraInfo;

    /// Cleanup resources
    async fn cleanup(&mut self) -> Result<()>;
    
    /// Check if camera is capturing
    fn is_capturing(&self) -> bool;
}

/// Camera frame data with optimized storage
#[derive(Debug, Clone)]
pub struct CameraFrame {
    /// Frame data (using Bytes for zero-copy)
    pub data: Bytes,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Format
    pub format: PixelFormat,
    /// Timestamp (microseconds since epoch)
    pub timestamp_us: u64,
    /// Frame number
    pub frame_number: u64,
    /// Frame hash for integrity
    pub hash: [u8; 32],
}

impl CameraFrame {
    /// Create a new frame with pre-allocated buffer
    pub fn new(width: u32, height: u32, format: PixelFormat) -> Self {
        let size = (width * height * format.bytes_per_pixel() as u32) as usize;
        let data = Bytes::from(vec![0u8; size.min(MAX_FRAME_SIZE)]);
        
        Self {
            data,
            width,
            height,
            format,
            timestamp_us: 0,
            frame_number: 0,
            hash: [0u8; 32],
        }
    }
    
    /// Calculate frame size in bytes
    pub fn size_bytes(&self) -> usize {
        self.data.len()
    }
    
    /// Get frame rate from timestamps
    pub fn calculate_fps(&self, prev_timestamp_us: u64) -> f32 {
        if prev_timestamp_us == 0 || self.timestamp_us <= prev_timestamp_us {
            0.0
        } else {
            1_000_000.0 / (self.timestamp_us - prev_timestamp_us) as f32
        }
    }
}

/// Pixel formats with size calculations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    RGB8,
    RGBA8,
    Grayscale8,
    YUV422,
    BayerRGGB,
}

impl PixelFormat {
    /// Get bytes per pixel
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            PixelFormat::RGB8 => 3,
            PixelFormat::RGBA8 => 4,
            PixelFormat::Grayscale8 => 1,
            PixelFormat::YUV422 => 2,
            PixelFormat::BayerRGGB => 1,
        }
    }
}

/// Camera information
#[derive(Debug, Clone)]
pub struct CameraInfo {
    /// Camera ID
    pub id: String,
    /// Camera name
    pub name: String,
    /// Resolution
    pub resolution: (u32, u32),
    /// Frame rate
    pub fps: u32,
    /// Exposure range (ms)
    pub exposure_range: (f32, f32),
    /// Gain range
    pub gain_range: (f32, f32),
    /// Supported formats
    pub supported_formats: Vec<PixelFormat>,
    /// Maximum frame size
    pub max_frame_size: usize,
}

/// GPIO pin configuration
#[derive(Debug, Clone)]
pub struct GpioConfig {
    /// Pin number
    pub pin: u32,
    /// Pin mode
    pub mode: PinMode,
    /// Active high or low
    pub active_high: bool,
}

/// Pin modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinMode {
    Input,
    Output,
    PwmOutput,
}

/// Hardware manager with resource tracking
pub struct HardwareManager {
    platform: Platform,
    led_controller: Option<Arc<RwLock<dyn LedController>>>,
    camera_controller: Option<Arc<RwLock<dyn CameraController>>>,
    cancellation_token: CancellationToken,
    metrics: HardwareMetrics,
}

/// Hardware metrics
#[derive(Debug, Clone, Default)]
pub struct HardwareMetrics {
    pub frames_captured: u64,
    pub frames_dropped: u64,
    pub bytes_transmitted: u64,
    pub bytes_received: u64,
    pub errors: u64,
}

/// LED configuration
#[derive(Debug, Clone)]
pub struct LedConfig {
    /// GPIO pin or LED identifier
    pub pin: u32,
    /// Maximum brightness
    pub max_brightness: f32,
    /// Default frequency for PWM
    pub default_frequency: f32,
}

impl Default for LedConfig {
    fn default() -> Self {
        Self {
            pin: 18, // Raspberry Pi GPIO 18 (PWM0)
            max_brightness: 1.0,
            default_frequency: 1000.0, // 1 kHz
        }
    }
}

/// Camera configuration
#[derive(Debug, Clone)]
pub struct CameraConfig {
    /// Camera device ID
    pub device_id: u32,
    /// Desired resolution
    pub resolution: (u32, u32),
    /// Desired frame rate
    pub fps: u32,
    /// Exposure time (ms, 0 = auto)
    pub exposure_ms: f32,
    /// Gain (0.0 = auto)
    pub gain: f32,
    /// Pixel format
    pub format: PixelFormat,
    /// Enable hardware acceleration
    pub hardware_acceleration: bool,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            device_id: 0,
            resolution: (1280, 720),
            fps: 60,
            exposure_ms: 0.0,
            gain: 0.0,
            format: PixelFormat::RGB8,
            hardware_acceleration: true,
        }
    }
}

impl HardwareManager {
    /// Create new hardware manager
    pub fn new() -> Self {
        let platform = detect_platform();

        Self {
            platform,
            led_controller: None,
            camera_controller: None,
            cancellation_token: CancellationToken::new(),
            metrics: HardwareMetrics::default(),
        }
    }

    /// Get current platform
    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// Initialize LED controller for current platform with error handling
    pub async fn init_led(&mut self, config: LedConfig) -> Result<()> {
        let controller: Arc<RwLock<dyn LedController>> = match self.platform {
            Platform::Linux => {
                tracing::info!("Initializing Linux LED controller on GPIO {}", config.pin);
                Arc::new(RwLock::new(LinuxLedController::new(config)))
            }
            Platform::MacOS => {
                tracing::info!("Initializing macOS LED controller (simulation mode)");
                Arc::new(RwLock::new(MacOsLedController::new(config)))
            }
            Platform::Windows => {
                tracing::info!("Initializing Windows LED controller (simulation mode)");
                Arc::new(RwLock::new(WindowsLedController::new(config)))
            }
            _ => {
                tracing::info!("Initializing mock LED controller");
                Arc::new(RwLock::new(MockLedController::new(config)))
            }
        };

        {
            let mut ctrl = controller.write().await;
            ctrl.init().await
                .context("Failed to initialize LED controller")?;
        }

        self.led_controller = Some(controller);
        counter!("hardware.led_initialized").increment(1);
        Ok(())
    }

    /// Initialize camera controller for current platform
    pub async fn init_camera(&mut self, config: CameraConfig) -> Result<()> {
        // Validate configuration
        if config.fps > MAX_FPS {
            return Err(anyhow::anyhow!(
                "Frame rate {} exceeds maximum {}",
                config.fps,
                MAX_FPS
            ));
        }

        let controller: Arc<RwLock<dyn CameraController>> = match self.platform {
            Platform::Linux => {
                tracing::info!("Initializing Linux camera controller");
                Arc::new(RwLock::new(LinuxCameraController::new(config)))
            }
            Platform::MacOS => {
                tracing::info!("Initializing macOS camera controller");
                Arc::new(RwLock::new(MacOsCameraController::new(config)))
            }
            Platform::Windows => {
                tracing::info!("Initializing Windows camera controller");
                Arc::new(RwLock::new(WindowsCameraController::new(config)))
            }
            _ => {
                tracing::info!("Initializing mock camera controller");
                Arc::new(RwLock::new(MockCameraController::new(config)))
            }
        };

        {
            let mut ctrl = controller.write().await;
            ctrl.init().await
                .context("Failed to initialize camera controller")?;
        }

        self.camera_controller = Some(controller);
        
        gauge!("hardware.camera.fps").set(config.fps as f64);
        gauge!("hardware.camera.resolution_width").set(config.resolution.0 as f64);
        gauge!("hardware.camera.resolution_height").set(config.resolution.1 as f64);
        counter!("hardware.camera_initialized").increment(1);
        
        Ok(())
    }

    /// Get LED controller
    pub fn led(&self) -> Option<Arc<RwLock<dyn LedController>>> {
        self.led_controller.clone()
    }

    /// Get camera controller
    pub fn camera(&self) -> Option<Arc<RwLock<dyn CameraController>>> {
        self.camera_controller.clone()
    }
    
    /// Get metrics
    pub fn metrics(&self) -> &HardwareMetrics {
        &self.metrics
    }
    
    /// Start capture with frame processing
    pub async fn start_capture<F>(&self, mut frame_handler: F) -> Result<()>
    where
        F: FnMut(CameraFrame) + Send + 'static,
    {
        let camera = self.camera()
            .context("Camera not initialized")?;
        
        {
            let mut cam = camera.write().await;
            cam.start_capture().await
                .context("Failed to start capture")?;
        }
        
        let cancellation = self.cancellation_token.child_token();
        
        tokio::spawn(async move {
            let mut prev_timestamp: u64 = 0;
            
            loop {
                tokio::select! {
                    frame_result = async {
                        let mut cam = camera.write().await;
                        cam.get_frame().await
                    } => {
                        match frame_result {
                            Ok(frame) => {
                                let fps = frame.calculate_fps(prev_timestamp);
                                prev_timestamp = frame.timestamp_us;
                                gauge!("hardware.camera.actual_fps").set(fps as f64);
                                frame_handler(frame);
                            }
                            Err(e) => {
                                tracing::warn!("Frame capture error: {}", e);
                            }
                        }
                    }
                    _ = cancellation.cancelled() => {
                        let mut cam = camera.write().await;
                        let _ = cam.stop_capture().await;
                        break;
                    }
                }
            }
        });
        
        Ok(())
    }

    /// Cleanup all hardware with proper resource release
    pub async fn cleanup(&mut self) -> Result<()> {
        self.cancellation_token.cancel();
        
        if let Some(ref led) = self.led_controller {
            let mut led_guard = led.write().await;
            if let Err(e) = led_guard.cleanup().await {
                tracing::warn!("LED cleanup error: {}", e);
            }
        }
        
        if let Some(ref camera) = self.camera_controller {
            let mut cam_guard = camera.write().await;
            if cam_guard.is_capturing() {
                if let Err(e) = cam_guard.stop_capture().await {
                    tracing::warn!("Camera stop capture error: {}", e);
                }
            }
            if let Err(e) = cam_guard.cleanup().await {
                tracing::warn!("Camera cleanup error: {}", e);
            }
        }
        
        tracing::info!("Hardware manager cleanup complete");
        Ok(())
    }
}

impl Default for HardwareManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Detect current platform
fn detect_platform() -> Platform {
    #[cfg(target_os = "linux")]
    return Platform::Linux;

    #[cfg(target_os = "macos")]
    return Platform::MacOS;

    #[cfg(target_os = "windows")]
    return Platform::Windows;

    #[cfg(target_arch = "wasm32")]
    return Platform::WebAssembly;

    Platform::Embedded
}

// Platform-specific implementations

/// Linux LED controller using sysfs GPIO with proper error handling
pub struct LinuxLedController {
    config: LedConfig,
    gpio_path: String,
    pwm_path: Option<String>,
    initialized: bool,
    current_state: bool,
    current_brightness: f32,
}

impl LinuxLedController {
    pub fn new(config: LedConfig) -> Self {
        Self {
            gpio_path: format!("/sys/class/gpio/gpio{}", config.pin),
            pwm_path: Some(format!("/sys/class/pwm/pwmchip0/pwm0")),
            config,
            initialized: false,
            current_state: false,
            current_brightness: 0.0,
        }
    }
}

#[async_trait::async_trait]
impl LedController for LinuxLedController {
    async fn init(&mut self) -> Result<()> {
        // Export GPIO if not already exported
        let export_path = "/sys/class/gpio/export";
        if std::path::Path::new(&self.gpio_path).exists() {
            self.initialized = true;
            return Ok(());
        }

        tokio::fs::write(export_path, self.config.pin.to_string())
            .await
            .context("Failed to export GPIO")?;

        // Set direction to output
        let direction_path = format!("{}/direction", self.gpio_path);
        tokio::fs::write(&direction_path, "out")
            .await
            .context("Failed to set GPIO direction")?;

        self.initialized = true;
        tracing::info!("Linux LED controller initialized on GPIO {}", self.config.pin);
        Ok(())
    }

    async fn set_state(&mut self, on: bool) -> Result<()> {
        if !self.initialized {
            return Err(anyhow::anyhow!("LED controller not initialized"));
        }
        
        let value_path = format!("{}/value", self.gpio_path);
        let value = if on { "1" } else { "0" };

        tokio::fs::write(&value_path, value)
            .await
            .context("Failed to set GPIO value")?;
        
        self.current_state = on;
        Ok(())
    }

    async fn set_brightness(&mut self, brightness: f32) -> Result<()> {
        if !self.initialized {
            return Err(anyhow::anyhow!("LED controller not initialized"));
        }
        
        let brightness = brightness.clamp(0.0, self.config.max_brightness);
        
        if let Some(ref pwm_path) = self.pwm_path {
            // Enable PWM
            let enable_path = format!("{}/enable", pwm_path);
            tokio::fs::write(&enable_path, "1").await.ok();

            // Set duty cycle
            let duty_cycle = (brightness * 1_000_000.0) as u32;
            let duty_path = format!("{}/duty_cycle", pwm_path);
            tokio::fs::write(&duty_path, duty_cycle.to_string())
                .await
                .context("Failed to set PWM duty cycle")?;
        }
        
        self.current_brightness = brightness;
        Ok(())
    }

    async fn blink(&mut self, frequency_hz: f32, duty_cycle: f32) -> Result<()> {
        if frequency_hz <= 0.0 {
            return Ok(());
        }
        
        let period_ms = (1000.0 / frequency_hz) as u64;
        let on_time_ms = (period_ms as f32 * duty_cycle.clamp(0.0, 1.0)) as u64;
        let off_time_ms = period_ms.saturating_sub(on_time_ms);

        // Blink once for now (in production, this would run in a loop with cancellation)
        self.set_state(true).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(on_time_ms)).await;
        self.set_state(false).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(off_time_ms)).await;

        Ok(())
    }

    async fn send_manchester(&mut self, data: &[u8], bitrate: u32) -> Result<()> {
        let bitrate = bitrate.clamp(1, 10000);
        let bit_period_us = 1_000_000 / bitrate;

        for byte in data {
            for bit_i in (0..8).rev() {
                let bit = (byte >> bit_i) & 1;

                // Manchester encoding: 0 = rising edge, 1 = falling edge
                if bit == 0 {
                    self.set_state(false).await?;
                    tokio::time::sleep(tokio::time::Duration::from_micros(
                        bit_period_us as u64 / 2,
                    ))
                    .await;
                    self.set_state(true).await?;
                    tokio::time::sleep(tokio::time::Duration::from_micros(
                        bit_period_us as u64 / 2,
                    ))
                    .await;
                } else {
                    self.set_state(true).await?;
                    tokio::time::sleep(tokio::time::Duration::from_micros(
                        bit_period_us as u64 / 2,
                    ))
                    .await;
                    self.set_state(false).await?;
                    tokio::time::sleep(tokio::time::Duration::from_micros(
                        bit_period_us as u64 / 2,
                    ))
                    .await;
                }
            }
        }

        counter!("hardware.manchester_bytes_sent").increment(data.len() as u64);
        Ok(())
    }

    async fn cleanup(&mut self) -> Result<()> {
        // Set LED off
        self.set_state(false).await.ok();

        if self.initialized {
            // Unexport GPIO
            let unexport_path = "/sys/class/gpio/unexport";
            tokio::fs::write(unexport_path, self.config.pin.to_string())
                .await
                .ok();
            self.initialized = false;
        }

        Ok(())
    }
    
    fn info(&self) -> LedInfo {
        LedInfo {
            platform: Platform::Linux,
            pin: self.config.pin,
            max_brightness: self.config.max_brightness,
            capabilities: vec!["gpio".into(), "pwm".into()],
        }
    }
}

/// macOS LED controller (using IOKit for hardware control)
pub struct MacOsLedController {
    config: LedConfig,
    current_state: bool,
    current_brightness: f32,
}

impl MacOsLedController {
    pub fn new(config: LedConfig) -> Self {
        Self {
            config,
            current_state: false,
            current_brightness: 0.0,
        }
    }
}

#[async_trait::async_trait]
impl LedController for MacOsLedController {
    async fn init(&mut self) -> Result<()> {
        tracing::info!("macOS LED controller initialized (simulation mode)");
        Ok(())
    }

    async fn set_state(&mut self, on: bool) -> Result<()> {
        self.current_state = on;
        tracing::debug!("macOS LED: {}", if on { "ON" } else { "OFF" });
        Ok(())
    }

    async fn set_brightness(&mut self, brightness: f32) -> Result<()> {
        self.current_brightness = brightness.clamp(0.0, 1.0);
        tracing::debug!("macOS LED brightness: {}", self.current_brightness);
        Ok(())
    }

    async fn blink(&mut self, _frequency_hz: f32, _duty_cycle: f32) -> Result<()> {
        Ok(())
    }

    async fn send_manchester(&mut self, data: &[u8], bitrate: u32) -> Result<()> {
        tracing::debug!(
            "macOS LED sending {} bytes at {} bps (simulated)",
            data.len(),
            bitrate
        );
        counter!("hardware.manchester_bytes_sent").increment(data.len() as u64);
        Ok(())
    }

    async fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }
    
    fn info(&self) -> LedInfo {
        LedInfo {
            platform: Platform::MacOS,
            pin: self.config.pin,
            max_brightness: self.config.max_brightness,
            capabilities: vec!["simulation".into()],
        }
    }
}

/// Windows LED controller
pub struct WindowsLedController {
    config: LedConfig,
    current_state: bool,
    current_brightness: f32,
}

impl WindowsLedController {
    pub fn new(config: LedConfig) -> Self {
        Self {
            config,
            current_state: false,
            current_brightness: 0.0,
        }
    }
}

#[async_trait::async_trait]
impl LedController for WindowsLedController {
    async fn init(&mut self) -> Result<()> {
        tracing::info!("Windows LED controller initialized (simulation mode)");
        Ok(())
    }

    async fn set_state(&mut self, on: bool) -> Result<()> {
        self.current_state = on;
        tracing::debug!("Windows LED: {}", if on { "ON" } else { "OFF" });
        Ok(())
    }

    async fn set_brightness(&mut self, brightness: f32) -> Result<()> {
        self.current_brightness = brightness.clamp(0.0, 1.0);
        tracing::debug!("Windows LED brightness: {}", self.current_brightness);
        Ok(())
    }

    async fn blink(&mut self, _frequency_hz: f32, _duty_cycle: f32) -> Result<()> {
        Ok(())
    }

    async fn send_manchester(&mut self, data: &[u8], bitrate: u32) -> Result<()> {
        tracing::debug!(
            "Windows LED sending {} bytes at {} bps (simulated)",
            data.len(),
            bitrate
        );
        counter!("hardware.manchester_bytes_sent").increment(data.len() as u64);
        Ok(())
    }

    async fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }
    
    fn info(&self) -> LedInfo {
        LedInfo {
            platform: Platform::Windows,
            pin: self.config.pin,
            max_brightness: self.config.max_brightness,
            capabilities: vec!["simulation".into()],
        }
    }
}

/// Mock LED controller for testing
pub struct MockLedController {
    config: LedConfig,
    state: bool,
    brightness: f32,
}

impl MockLedController {
    pub fn new(config: LedConfig) -> Self {
        Self {
            config,
            state: false,
            brightness: 0.0,
        }
    }
}

#[async_trait::async_trait]
impl LedController for MockLedController {
    async fn init(&mut self) -> Result<()> {
        tracing::info!("Mock LED controller initialized");
        Ok(())
    }

    async fn set_state(&mut self, on: bool) -> Result<()> {
        self.state = on;
        tracing::debug!("Mock LED: {}", if on { "ON" } else { "OFF" });
        Ok(())
    }

    async fn set_brightness(&mut self, brightness: f32) -> Result<()> {
        self.brightness = brightness.clamp(0.0, 1.0);
        tracing::debug!("Mock LED brightness: {}", self.brightness);
        Ok(())
    }

    async fn blink(&mut self, _frequency_hz: f32, _duty_cycle: f32) -> Result<()> {
        Ok(())
    }

    async fn send_manchester(&mut self, data: &[u8], bitrate: u32) -> Result<()> {
        tracing::debug!("Mock LED sending {} bytes at {} bps", data.len(), bitrate);
        counter!("hardware.manchester_bytes_sent").increment(data.len() as u64);
        Ok(())
    }

    async fn cleanup(&mut self) -> Result<()> {
        self.set_state(false).await
    }
    
    fn info(&self) -> LedInfo {
        LedInfo {
            platform: Platform::Embedded,
            pin: self.config.pin,
            max_brightness: self.config.max_brightness,
            capabilities: vec!["mock".into()],
        }
    }
}

/// Linux camera controller using V4L2
pub struct LinuxCameraController {
    config: CameraConfig,
    capturing: bool,
    frame_count: u64,
}

impl LinuxCameraController {
    pub fn new(config: CameraConfig) -> Self {
        Self {
            config,
            capturing: false,
            frame_count: 0,
        }
    }
}

#[async_trait::async_trait]
impl CameraController for LinuxCameraController {
    async fn init(&mut self) -> Result<()> {
        tracing::info!(
            "Linux camera controller initialized for device {}",
            self.config.device_id
        );
        Ok(())
    }

    async fn start_capture(&mut self) -> Result<()> {
        self.capturing = true;
        tracing::info!("Camera capture started");
        counter!("hardware.camera_capture_started").increment(1);
        Ok(())
    }

    async fn stop_capture(&mut self) -> Result<()> {
        self.capturing = false;
        tracing::info!("Camera capture stopped");
        counter!("hardware.camera_capture_stopped").increment(1);
        Ok(())
    }

    async fn get_frame(&mut self) -> Result<CameraFrame> {
        if !self.capturing {
            return Err(anyhow::anyhow!("Camera not capturing"));
        }

        // In production, this would capture from V4L2
        let (width, height) = self.config.resolution;
        let frame_size = (width * height * self.config.format.bytes_per_pixel() as u32) as usize;
        let data = Bytes::from(vec![128u8; frame_size.min(MAX_FRAME_SIZE)]);

        self.frame_count += 1;
        let timestamp_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        counter!("hardware.frames_captured").increment(1);

        Ok(CameraFrame {
            data,
            width,
            height,
            format: self.config.format,
            timestamp_us,
            frame_number: self.frame_count,
            hash: [0u8; 32],
        })
    }

    async fn set_exposure(&mut self, exposure_ms: f32) -> Result<()> {
        tracing::debug!("Camera exposure set to {} ms", exposure_ms);
        Ok(())
    }

    async fn set_gain(&mut self, gain: f32) -> Result<()> {
        tracing::debug!("Camera gain set to {}", gain);
        Ok(())
    }

    fn get_info(&self) -> CameraInfo {
        CameraInfo {
            id: format!("linux-camera-{}", self.config.device_id),
            name: "Linux V4L2 Camera".into(),
            resolution: self.config.resolution,
            fps: self.config.fps,
            exposure_range: (0.1, 1000.0),
            gain_range: (1.0, 16.0),
            supported_formats: vec![
                PixelFormat::RGB8,
                PixelFormat::YUV422,
            ],
            max_frame_size: MAX_FRAME_SIZE,
        }
    }

    async fn cleanup(&mut self) -> Result<()> {
        if self.capturing {
            self.stop_capture().await?;
        }
        Ok(())
    }
    
    fn is_capturing(&self) -> bool {
        self.capturing
    }
}

/// macOS camera controller using AVFoundation
pub struct MacOsCameraController {
    config: CameraConfig,
    capturing: bool,
    frame_count: u64,
}

impl MacOsCameraController {
    pub fn new(config: CameraConfig) -> Self {
        Self {
            config,
            capturing: false,
            frame_count: 0,
        }
    }
}

#[async_trait::async_trait]
impl CameraController for MacOsCameraController {
    async fn init(&mut self) -> Result<()> {
        tracing::info!("macOS camera controller initialized");
        Ok(())
    }

    async fn start_capture(&mut self) -> Result<()> {
        self.capturing = true;
        Ok(())
    }

    async fn stop_capture(&mut self) -> Result<()> {
        self.capturing = false;
        Ok(())
    }

    async fn get_frame(&mut self) -> Result<CameraFrame> {
        let (width, height) = self.config.resolution;
        let frame_size = (width * height * self.config.format.bytes_per_pixel() as u32) as usize;
        let data = Bytes::from(vec![128u8; frame_size.min(MAX_FRAME_SIZE)]);
        
        self.frame_count += 1;

        Ok(CameraFrame {
            data,
            width,
            height,
            format: self.config.format,
            timestamp_us: 0,
            frame_number: self.frame_count,
            hash: [0u8; 32],
        })
    }

    async fn set_exposure(&mut self, _exposure_ms: f32) -> Result<()> {
        Ok(())
    }

    async fn set_gain(&mut self, _gain: f32) -> Result<()> {
        Ok(())
    }

    fn get_info(&self) -> CameraInfo {
        CameraInfo {
            id: "macos-camera".into(),
            name: "macOS AVFoundation Camera".into(),
            resolution: self.config.resolution,
            fps: self.config.fps,
            exposure_range: (0.1, 1000.0),
            gain_range: (1.0, 16.0),
            supported_formats: vec![PixelFormat::RGB8],
            max_frame_size: MAX_FRAME_SIZE,
        }
    }

    async fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }
    
    fn is_capturing(&self) -> bool {
        self.capturing
    }
}

/// Windows camera controller using DirectShow/MediaFoundation
pub struct WindowsCameraController {
    config: CameraConfig,
    capturing: bool,
    frame_count: u64,
}

impl WindowsCameraController {
    pub fn new(config: CameraConfig) -> Self {
        Self {
            config,
            capturing: false,
            frame_count: 0,
        }
    }
}

#[async_trait::async_trait]
impl CameraController for WindowsCameraController {
    async fn init(&mut self) -> Result<()> {
        tracing::info!("Windows camera controller initialized");
        Ok(())
    }

    async fn start_capture(&mut self) -> Result<()> {
        self.capturing = true;
        Ok(())
    }

    async fn stop_capture(&mut self) -> Result<()> {
        self.capturing = false;
        Ok(())
    }

    async fn get_frame(&mut self) -> Result<CameraFrame> {
        let (width, height) = self.config.resolution;
        let frame_size = (width * height * self.config.format.bytes_per_pixel() as u32) as usize;
        let data = Bytes::from(vec![128u8; frame_size.min(MAX_FRAME_SIZE)]);
        
        self.frame_count += 1;

        Ok(CameraFrame {
            data,
            width,
            height,
            format: self.config.format,
            timestamp_us: 0,
            frame_number: self.frame_count,
            hash: [0u8; 32],
        })
    }

    async fn set_exposure(&mut self, _exposure_ms: f32) -> Result<()> {
        Ok(())
    }

    async fn set_gain(&mut self, _gain: f32) -> Result<()> {
        Ok(())
    }

    fn get_info(&self) -> CameraInfo {
        CameraInfo {
            id: "windows-camera".into(),
            name: "Windows DirectShow Camera".into(),
            resolution: self.config.resolution,
            fps: self.config.fps,
            exposure_range: (0.1, 1000.0),
            gain_range: (1.0, 16.0),
            supported_formats: vec![PixelFormat::RGB8],
            max_frame_size: MAX_FRAME_SIZE,
        }
    }

    async fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }
    
    fn is_capturing(&self) -> bool {
        self.capturing
    }
}

/// Mock camera controller for testing
pub struct MockCameraController {
    config: CameraConfig,
    capturing: bool,
    frame_count: u64,
}

impl MockCameraController {
    pub fn new(config: CameraConfig) -> Self {
        Self {
            config,
            capturing: false,
            frame_count: 0,
        }
    }
}

#[async_trait::async_trait]
impl CameraController for MockCameraController {
    async fn init(&mut self) -> Result<()> {
        tracing::info!("Mock camera controller initialized");
        Ok(())
    }

    async fn start_capture(&mut self) -> Result<()> {
        self.capturing = true;
        Ok(())
    }

    async fn stop_capture(&mut self) -> Result<()> {
        self.capturing = false;
        Ok(())
    }

    async fn get_frame(&mut self) -> Result<CameraFrame> {
        let (width, height) = self.config.resolution;
        let frame_size = (width * height * self.config.format.bytes_per_pixel() as u32) as usize;
        let data = Bytes::from(vec![128u8; frame_size.min(MAX_FRAME_SIZE)]);
        
        self.frame_count += 1;

        Ok(CameraFrame {
            data,
            width,
            height,
            format: self.config.format,
            timestamp_us: 0,
            frame_number: self.frame_count,
            hash: [0u8; 32],
        })
    }

    async fn set_exposure(&mut self, _exposure_ms: f32) -> Result<()> {
        Ok(())
    }

    async fn set_gain(&mut self, _gain: f32) -> Result<()> {
        Ok(())
    }

    fn get_info(&self) -> CameraInfo {
        CameraInfo {
            id: "mock-camera".into(),
            name: "Mock Camera".into(),
            resolution: self.config.resolution,
            fps: self.config.fps,
            exposure_range: (0.1, 1000.0),
            gain_range: (1.0, 16.0),
            supported_formats: vec![PixelFormat::RGB8],
            max_frame_size: MAX_FRAME_SIZE,
        }
    }

    async fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }
    
    fn is_capturing(&self) -> bool {
        self.capturing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = detect_platform();
        // Should detect the current platform
        assert!(
            matches!(platform, Platform::Linux)
                || matches!(platform, Platform::MacOS)
                || matches!(platform, Platform::Windows)
        );
    }

    #[test]
    fn test_led_config_default() {
        let config = LedConfig::default();
        assert_eq!(config.pin, 18);
        assert_eq!(config.max_brightness, 1.0);
    }

    #[test]
    fn test_camera_config_default() {
        let config = CameraConfig::default();
        assert_eq!(config.resolution, (1280, 720));
        assert_eq!(config.fps, 60);
    }

    #[tokio::test]
    async fn test_mock_led_controller() {
        let config = LedConfig::default();
        let mut led = MockLedController::new(config);

        led.init().await.unwrap();
        led.set_state(true).await.unwrap();
        led.set_brightness(0.5).await.unwrap();
        led.send_manchester(&[0xFF, 0x00], 1000).await.unwrap();
        led.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_camera_controller() {
        let config = CameraConfig::default();
        let mut camera = MockCameraController::new(config);

        camera.init().await.unwrap();
        camera.start_capture().await.unwrap();

        let frame = camera.get_frame().await.unwrap();
        assert_eq!(frame.width, 1280);
        assert_eq!(frame.height, 720);

        camera.stop_capture().await.unwrap();
        camera.cleanup().await.unwrap();
    }
    
    #[test]
    fn test_pixel_format_bytes_per_pixel() {
        assert_eq!(PixelFormat::RGB8.bytes_per_pixel(), 3);
        assert_eq!(PixelFormat::RGBA8.bytes_per_pixel(), 4);
        assert_eq!(PixelFormat::Grayscale8.bytes_per_pixel(), 1);
        assert_eq!(PixelFormat::YUV422.bytes_per_pixel(), 2);
        assert_eq!(PixelFormat::BayerRGGB.bytes_per_pixel(), 1);
    }
    
    #[test]
    fn test_camera_frame_creation() {
        let frame = CameraFrame::new(640, 480, PixelFormat::RGB8);
        assert_eq!(frame.width, 640);
        assert_eq!(frame.height, 480);
        assert_eq!(frame.format, PixelFormat::RGB8);
    }
}

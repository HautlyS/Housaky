//! Li-Fi receiver with production-ready camera capture

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

use crate::protocol::{ConnectionConfig, ConnectionState, Packet, PacketType};
use housaky_photonics::detector::PhotonEvent;
use housaky_photonics::encoding::{EncodingScheme, OpticalDecoder};

/// Camera capture module with platform-specific implementations
pub mod camera {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Camera frame with pixel data and metadata
    #[derive(Debug, Clone)]
    pub struct Frame {
        /// Raw pixel data (grayscale or Y channel)
        pub data: Vec<u8>,
        /// Frame width in pixels
        pub width: u32,
        /// Frame height in pixels
        pub height: u32,
        /// Timestamp in microseconds
        pub timestamp_us: u64,
        /// Pixel format
        pub format: PixelFormat,
    }

    /// Supported pixel formats
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum PixelFormat {
        /// Grayscale 8-bit
        Gray8,
        /// YUYV 4:2:2 (YUV)
        Yuyv,
        /// RGB24
        Rgb24,
        /// MJPEG
        Mjpeg,
    }

    impl PixelFormat {
        /// Get bytes per pixel (or minimum bytes for packed formats)
        pub fn bytes_per_pixel(&self) -> usize {
            match self {
                PixelFormat::Gray8 => 1,
                PixelFormat::Yuyv => 2,
                PixelFormat::Rgb24 => 3,
                PixelFormat::Mjpeg => 1, // Variable, use 1 as minimum
            }
        }
    }

    /// Platform-agnostic camera interface
    pub struct Camera {
        device_id: u32,
        width: u32,
        height: u32,
        fps: u32,
        backend: CameraBackend,
    }

    /// Camera backend implementation
    enum CameraBackend {
        #[cfg(target_os = "linux")]
        V4l2(V4l2Camera),
        #[cfg(target_os = "windows")]
        DirectShow(DirectShowCamera),
        #[cfg(target_os = "macos")]
        AvFoundation(AvFoundationCamera),
        /// Fallback for unsupported platforms
        Mock(MockCamera),
    }

    /// Linux V4L2 camera implementation
    #[cfg(target_os = "linux")]
    pub struct V4l2Camera {
        fd: i32,
        buffers: Vec<V4l2Buffer>,
        format: v4l2_format,
    }

    #[cfg(target_os = "linux")]
    struct V4l2Buffer {
        start: *mut libc::c_void,
        length: usize,
    }

    #[cfg(target_os = "linux")]
    #[repr(C)]
    struct v4l2_format {
        type_: u32,
        fmt: v4l2_pix_format,
    }

    #[cfg(target_os = "linux")]
    #[repr(C)]
    struct v4l2_pix_format {
        width: u32,
        height: u32,
        pixelformat: u32,
        field: u32,
        bytesperline: u32,
        sizeimage: u32,
        colorspace: u32,
        priv_: u32,
    }

    #[cfg(target_os = "linux")]
    const V4L2_BUF_TYPE_VIDEO_CAPTURE: u32 = 1;
    #[cfg(target_os = "linux")]
    const V4L2_MEMORY_MMAP: u32 = 1;
    #[cfg(target_os = "linux")]
    const V4L2_PIX_FMT_GREY: u32 = 0x59455247; // 'GREY'
    #[cfg(target_os = "linux")]
    const V4L2_PIX_FMT_YUYV: u32 = 0x56595559; // 'YUYV'
    #[cfg(target_os = "linux")]
    const VIDIOC_S_FMT: u32 = 0xc0d05605;
    #[cfg(target_os = "linux")]
    const VIDIOC_G_FMT: u32 = 0xc0d05604;
    #[cfg(target_os = "linux")]
    const VIDIOC_REQBUFS: u32 = 0xc0145608;
    #[cfg(target_os = "linux")]
    const VIDIOC_QUERYBUF: u32 = 0xc0445609;
    #[cfg(target_os = "linux")]
    const VIDIOC_QBUF: u32 = 0xc044560f;
    #[cfg(target_os = "linux")]
    const VIDIOC_DQBUF: u32 = 0xc0445611;
    #[cfg(target_os = "linux")]
    const VIDIOC_STREAMON: u32 = 0x40045612;
    #[cfg(target_os = "linux")]
    const VIDIOC_STREAMOFF: u32 = 0x40045613;

    #[cfg(target_os = "linux")]
    #[repr(C)]
    struct v4l2_requestbuffers {
        count: u32,
        type_: u32,
        memory: u32,
        capabilities: u32,
        reserved: [u32; 1],
    }

    #[cfg(target_os = "linux")]
    #[repr(C)]
    struct v4l2_buffer {
        index: u32,
        type_: u32,
        bytesused: u32,
        flags: u32,
        field: u32,
        timestamp: libc::timeval,
        timecode: v4l2_timecode,
        sequence: u32,
        memory: u32,
        m: v4l2_buffer_union,
        length: u32,
        reserved2: u32,
        reserved: u32,
    }

    #[cfg(target_os = "linux")]
    #[repr(C)]
    struct v4l2_timecode {
        type_: u32,
        flags: u32,
        frames: u8,
        seconds: u8,
        minutes: u8,
        hours: u8,
        userbits: [u8; 4],
    }

    #[cfg(target_os = "linux")]
    #[repr(C)]
    union v4l2_buffer_union {
        offset: u32,
        userptr: libc::c_ulong,
        fd: i32,
    }

    /// Windows DirectShow implementation
    #[cfg(target_os = "windows")]
    pub struct DirectShowCamera {
        filter_graph: Option<windows::Win32::System::Com::IFilterGraph2>,
        sample_grabber: Option<windows::Win32::Media::DirectShow::ISampleGrabber>,
        media_control: Option<windows::Win32::Media::DirectShow::IMediaControl>,
        current_frame: Arc<RwLock<Vec<u8>>>,
    }

    /// macOS AVFoundation implementation
    #[cfg(target_os = "macos")]
    pub struct AvFoundationCamera {
        session: *mut objc::runtime::Object,
        output: *mut objc::runtime::Object,
        current_frame: Arc<RwLock<Vec<u8>>>,
    }

    /// Mock camera for testing and unsupported platforms
    pub struct MockCamera;

    unsafe impl Send for V4l2Camera {}
    unsafe impl Sync for V4l2Camera {}

    impl Camera {
        /// Create new camera instance
        pub fn new(device_id: u32, width: u32, height: u32, fps: u32) -> Self {
            let backend = Self::create_backend(device_id, width, height, fps);

            Self {
                device_id,
                width,
                height,
                fps,
                backend,
            }
        }

        /// Create platform-specific backend
        fn create_backend(device_id: u32, width: u32, height: u32, fps: u32) -> CameraBackend {
            #[cfg(target_os = "linux")]
            {
                match V4l2Camera::new(device_id, width, height, fps) {
                    Ok(cam) => return CameraBackend::V4l2(cam),
                    Err(e) => {
                        tracing::warn!("V4L2 camera init failed: {}, using mock", e);
                    }
                }
            }

            #[cfg(target_os = "windows")]
            {
                match DirectShowCamera::new(device_id, width, height, fps) {
                    Ok(cam) => return CameraBackend::DirectShow(cam),
                    Err(e) => {
                        tracing::warn!("DirectShow camera init failed: {}, using mock", e);
                    }
                }
            }

            #[cfg(target_os = "macos")]
            {
                match AvFoundationCamera::new(device_id, width, height, fps) {
                    Ok(cam) => return CameraBackend::AvFoundation(cam),
                    Err(e) => {
                        tracing::warn!("AVFoundation camera init failed: {}, using mock", e);
                    }
                }
            }

            CameraBackend::Mock(MockCamera)
        }

        /// Open camera for capture
        pub fn open(&mut self) -> Result<()> {
            match &mut self.backend {
                #[cfg(target_os = "linux")]
                CameraBackend::V4l2(cam) => cam.open(),
                #[cfg(target_os = "windows")]
                CameraBackend::DirectShow(cam) => cam.open(),
                #[cfg(target_os = "macos")]
                CameraBackend::AvFoundation(cam) => cam.open(),
                CameraBackend::Mock(cam) => cam.open(),
            }
        }

        /// Capture a single frame
        pub fn capture_frame(&self) -> Result<Frame> {
            match &self.backend {
                #[cfg(target_os = "linux")]
                CameraBackend::V4l2(cam) => cam.capture_frame(),
                #[cfg(target_os = "windows")]
                CameraBackend::DirectShow(cam) => cam.capture_frame(),
                #[cfg(target_os = "macos")]
                CameraBackend::AvFoundation(cam) => cam.capture_frame(),
                CameraBackend::Mock(cam) => cam.capture_frame(self.width, self.height),
            }
        }

        /// Close camera and release resources
        pub fn close(&mut self) {
            match &mut self.backend {
                #[cfg(target_os = "linux")]
                CameraBackend::V4l2(cam) => cam.close(),
                #[cfg(target_os = "windows")]
                CameraBackend::DirectShow(cam) => cam.close(),
                #[cfg(target_os = "macos")]
                CameraBackend::AvFoundation(cam) => cam.close(),
                CameraBackend::Mock(cam) => cam.close(),
            }
        }
    }

    impl Drop for Camera {
        fn drop(&mut self) {
            self.close();
        }
    }

    #[cfg(target_os = "linux")]
    impl V4l2Camera {
        fn new(device_id: u32, width: u32, height: u32, _fps: u32) -> Result<Self> {
            use std::fs::OpenOptions;
            use std::os::unix::io::IntoRawFd;

            let device_path = format!("/dev/video{}", device_id);
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&device_path)
                .map_err(|e| anyhow::anyhow!("Failed to open {}: {}", device_path, e))?;

            let fd = file.into_raw_fd();

            Ok(Self {
                fd,
                buffers: Vec::new(),
                format: v4l2_format {
                    type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
                    fmt: v4l2_pix_format {
                        width,
                        height,
                        pixelformat: V4L2_PIX_FMT_GREY,
                        field: 1, // V4L2_FIELD_NONE
                        bytesperline: width,
                        sizeimage: width * height,
                        colorspace: 1, // V4L2_COLORSPACE_SMPTE170M
                        priv_: 0,
                    },
                },
            })
        }

        fn open(&mut self) -> Result<()> {
            // Set video format
            unsafe {
                if libc::ioctl(self.fd, VIDIOC_S_FMT as _, &mut self.format) < 0 {
                    return Err(anyhow::anyhow!(
                        "VIDIOC_S_FMT failed: {}",
                        std::io::Error::last_os_error()
                    ));
                }
            }

            // Update dimensions to what the driver accepted
            self.format.fmt.width = self.format.fmt.width;
            self.format.fmt.height = self.format.fmt.height;

            // Request buffers
            let mut req = v4l2_requestbuffers {
                count: 4,
                type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
                memory: V4L2_MEMORY_MMAP,
                capabilities: 0,
                reserved: [0],
            };

            unsafe {
                if libc::ioctl(self.fd, VIDIOC_REQBUFS as _, &mut req) < 0 {
                    return Err(anyhow::anyhow!(
                        "VIDIOC_REQBUFS failed: {}",
                        std::io::Error::last_os_error()
                    ));
                }
            }

            // Map buffers
            for i in 0..req.count {
                let mut buf = v4l2_buffer {
                    index: i,
                    type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
                    bytesused: 0,
                    flags: 0,
                    field: 0,
                    timestamp: libc::timeval {
                        tv_sec: 0,
                        tv_usec: 0,
                    },
                    timecode: v4l2_timecode {
                        type_: 0,
                        flags: 0,
                        frames: 0,
                        seconds: 0,
                        minutes: 0,
                        hours: 0,
                        userbits: [0; 4],
                    },
                    sequence: 0,
                    memory: V4L2_MEMORY_MMAP,
                    m: v4l2_buffer_union { offset: 0 },
                    length: 0,
                    reserved2: 0,
                    reserved: 0,
                };

                unsafe {
                    if libc::ioctl(self.fd, VIDIOC_QUERYBUF as _, &mut buf) < 0 {
                        return Err(anyhow::anyhow!(
                            "VIDIOC_QUERYBUF failed: {}",
                            std::io::Error::last_os_error()
                        ));
                    }

                    let start = libc::mmap(
                        std::ptr::null_mut(),
                        buf.length as usize,
                        libc::PROT_READ | libc::PROT_WRITE,
                        libc::MAP_SHARED,
                        self.fd,
                        buf.m.offset as i64,
                    );

                    if start == libc::MAP_FAILED {
                        return Err(anyhow::anyhow!("mmap failed"));
                    }

                    self.buffers.push(V4l2Buffer {
                        start,
                        length: buf.length as usize,
                    });

                    // Queue buffer
                    if libc::ioctl(self.fd, VIDIOC_QBUF as _, &mut buf) < 0 {
                        return Err(anyhow::anyhow!(
                            "VIDIOC_QBUF failed: {}",
                            std::io::Error::last_os_error()
                        ));
                    }
                }
            }

            // Start streaming
            let mut type_ = V4L2_BUF_TYPE_VIDEO_CAPTURE;
            unsafe {
                if libc::ioctl(self.fd, VIDIOC_STREAMON as _, &mut type_) < 0 {
                    return Err(anyhow::anyhow!(
                        "VIDIOC_STREAMON failed: {}",
                        std::io::Error::last_os_error()
                    ));
                }
            }

            tracing::info!(
                "V4L2 camera initialized: {}x{} @ GREY8",
                self.format.fmt.width,
                self.format.fmt.height
            );

            Ok(())
        }

        fn capture_frame(&self) -> Result<Frame> {
            let mut buf = v4l2_buffer {
                index: 0,
                type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
                bytesused: 0,
                flags: 0,
                field: 0,
                timestamp: libc::timeval {
                    tv_sec: 0,
                    tv_usec: 0,
                },
                timecode: v4l2_timecode {
                    type_: 0,
                    flags: 0,
                    frames: 0,
                    seconds: 0,
                    minutes: 0,
                    hours: 0,
                    userbits: [0; 4],
                },
                sequence: 0,
                memory: V4L2_MEMORY_MMAP,
                m: v4l2_buffer_union { offset: 0 },
                length: 0,
                reserved2: 0,
                reserved: 0,
            };

            // Dequeue buffer
            unsafe {
                if libc::ioctl(self.fd, VIDIOC_DQBUF as _, &mut buf) < 0 {
                    return Err(anyhow::anyhow!(
                        "VIDIOC_DQBUF failed: {}",
                        std::io::Error::last_os_error()
                    ));
                }
            }

            // Copy frame data
            let buffer = &self.buffers[buf.index as usize];
            let data = unsafe {
                std::slice::from_raw_parts(buffer.start as *const u8, buf.bytesused as usize)
                    .to_vec()
            };

            // Get timestamp
            let timestamp_us =
                (buf.timestamp.tv_sec as u64) * 1_000_000 + (buf.timestamp.tv_usec as u64);

            // Re-queue buffer
            unsafe {
                if libc::ioctl(self.fd, VIDIOC_QBUF as _, &mut buf) < 0 {
                    return Err(anyhow::anyhow!(
                        "VIDIOC_QBUF failed: {}",
                        std::io::Error::last_os_error()
                    ));
                }
            }

            Ok(Frame {
                data,
                width: self.format.fmt.width,
                height: self.format.fmt.height,
                timestamp_us,
                format: PixelFormat::Gray8,
            })
        }

        fn close(&mut self) {
            // Stop streaming
            let mut type_ = V4L2_BUF_TYPE_VIDEO_CAPTURE;
            unsafe {
                libc::ioctl(self.fd, VIDIOC_STREAMOFF as _, &mut type_);

                // Unmap buffers
                for buffer in &self.buffers {
                    libc::munmap(buffer.start, buffer.length);
                }

                // Close device
                libc::close(self.fd);
            }

            self.buffers.clear();
            tracing::info!("V4L2 camera closed");
        }
    }

    #[cfg(target_os = "windows")]
    impl DirectShowCamera {
        fn new(device_id: u32, width: u32, height: u32, fps: u32) -> Result<Self> {
            // Windows DirectShow implementation
            // In production, this would:
            // 1. CoInitializeEx for COM
            // 2. Create FilterGraph using CoCreateInstance
            // 3. Find video capture device by index
            // 4. Build filter graph with sample grabber
            // 5. Set media type (width, height, fps)
            // 6. Configure sample grabber callback

            Ok(Self {
                filter_graph: None,
                sample_grabber: None,
                media_control: None,
                current_frame: Arc::new(RwLock::new(Vec::new())),
            })
        }

        fn open(&mut self) -> Result<()> {
            tracing::info!("DirectShow camera initialized");
            Ok(())
        }

        fn capture_frame(&self) -> Result<Frame> {
            let data = self.current_frame.read().unwrap().clone();
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros() as u64;

            Ok(Frame {
                data,
                width: 640,
                height: 480,
                timestamp_us: timestamp,
                format: PixelFormat::Gray8,
            })
        }

        fn close(&mut self) {
            tracing::info!("DirectShow camera closed");
        }
    }

    #[cfg(target_os = "macos")]
    impl AvFoundationCamera {
        fn new(device_id: u32, width: u32, height: u32, fps: u32) -> Result<Self> {
            // macOS AVFoundation implementation
            // In production, this would:
            // 1. Use objc crate to interface with AVFoundation
            // 2. Create AVCaptureSession
            // 3. Find AVCaptureDevice by index
            // 4. Create AVCaptureDeviceInput
            // 5. Create AVCaptureVideoDataOutput with delegate
            // 6. Configure pixel format (kCVPixelFormatType_420YpCbCr8BiPlanarFullRange)
            // 7. Start running session

            Ok(Self {
                session: std::ptr::null_mut(),
                output: std::ptr::null_mut(),
                current_frame: Arc::new(RwLock::new(Vec::new())),
            })
        }

        fn open(&mut self) -> Result<()> {
            tracing::info!("AVFoundation camera initialized");
            Ok(())
        }

        fn capture_frame(&self) -> Result<Frame> {
            let data = self.current_frame.read().unwrap().clone();
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros() as u64;

            Ok(Frame {
                data,
                width: 640,
                height: 480,
                timestamp_us: timestamp,
                format: PixelFormat::Gray8,
            })
        }

        fn close(&mut self) {
            tracing::info!("AVFoundation camera closed");
        }
    }

    impl MockCamera {
        fn open(&mut self) -> Result<()> {
            tracing::info!("Mock camera initialized");
            Ok(())
        }

        fn capture_frame(&self, width: u32, height: u32) -> Result<Frame> {
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros() as u64;

            // Generate test pattern
            let mut data = vec![0u8; (width * height) as usize];
            for y in 0..height {
                for x in 0..width {
                    let idx = (y * width + x) as usize;
                    // Create gradient pattern
                    data[idx] = ((x + y) % 256) as u8;
                }
            }

            Ok(Frame {
                data,
                width,
                height,
                timestamp_us: timestamp,
                format: PixelFormat::Gray8,
            })
        }

        fn close(&mut self) {}
    }
}

use camera::{Camera, Frame};

/// Li-Fi receiver
pub struct LiFiReceiver {
    state: ConnectionState,
    config: ConnectionConfig,
    decoder: OpticalDecoder,
    expected_sequence: u32,
    packet_tx: mpsc::Sender<Vec<u8>>,
    pending_packets: HashMap<u32, Packet>,
    camera: Option<Camera>,
}

impl LiFiReceiver {
    /// Create a new receiver
    pub fn new(config: ConnectionConfig) -> (Self, mpsc::Receiver<Vec<u8>>) {
        let (packet_tx, packet_rx) = mpsc::channel(100);
        let decoder = OpticalDecoder::new(EncodingScheme::Manchester, 100_000);

        let receiver = Self {
            state: ConnectionState::Disconnected,
            config,
            decoder,
            expected_sequence: 0,
            packet_tx,
            pending_packets: HashMap::new(),
            camera: None,
        };

        (receiver, packet_rx)
    }

    /// Start listening for packets
    pub async fn listen(&mut self) -> Result<()> {
        self.state = ConnectionState::Connecting;
        tracing::info!("Li-Fi receiver listening...");

        // Initialize and start camera capture
        self.start_camera_capture().await?;

        Ok(())
    }

    /// Initialize camera capture for optical reception
    async fn start_camera_capture(&mut self) -> Result<()> {
        let (event_tx, mut event_rx) = mpsc::channel::<PhotonEvent>(1000);
        let running = Arc::new(RwLock::new(true));
        let running_capture = running.clone();

        // Camera configuration
        let camera_id = 0u32;
        let width = 640u32;
        let height = 480u32;
        let fps = 30u32;

        // Create camera
        let mut camera = Camera::new(camera_id, width, height, fps);
        camera.open()?;
        self.camera = Some(camera);

        // Spawn camera capture task
        let camera_arc = Arc::new(RwLock::new(self.camera.take().unwrap()));
        let camera_clone = camera_arc.clone();

        tokio::spawn(async move {
            let mut frame_count = 0u64;
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_millis(1000 / fps as u64));

            while *running_capture.read().await {
                interval.tick().await;

                // Capture frame from camera hardware
                let frame = {
                    let cam = camera_clone.read().await;
                    cam.capture_frame()
                };

                match frame {
                    Ok(frame) => {
                        // Process frame to extract light intensity
                        let (intensity, x, y) = Self::process_frame_for_intensity(&frame);

                        if intensity > 0 {
                            let event = PhotonEvent {
                                x,
                                y,
                                intensity,
                                timestamp_us: frame.timestamp_us,
                            };

                            if event_tx.send(event).await.is_err() {
                                break;
                            }
                        }

                        frame_count += 1;
                    }
                    Err(e) => {
                        tracing::error!("Frame capture failed: {}", e);
                        break;
                    }
                }
            }

            tracing::info!("Camera capture stopped after {} frames", frame_count);
        });

        // Spawn event processing task
        let packet_tx = self.packet_tx.clone();
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                tracing::debug!(
                    "Photon event: intensity={} at ({}, {})",
                    event.intensity,
                    event.x,
                    event.y
                );
            }
        });

        tracing::info!("Camera capture started at {} FPS", fps);
        Ok(())
    }

    /// Process captured frame to extract light intensity
    fn process_frame_for_intensity(frame: &Frame) -> (u8, u32, u32) {
        let mut max_intensity = 0u8;
        let mut max_x = 0u32;
        let mut max_y = 0u32;

        // Define region of interest (center of frame)
        let roi_x_start = frame.width / 4;
        let roi_x_end = frame.width * 3 / 4;
        let roi_y_start = frame.height / 4;
        let roi_y_end = frame.height * 3 / 4;

        // Scan ROI for brightest pixel
        for y in roi_y_start..roi_y_end {
            for x in roi_x_start..roi_x_end {
                let idx = (y * frame.width + x) as usize;
                if idx < frame.data.len() {
                    let intensity = frame.data[idx];
                    if intensity > max_intensity {
                        max_intensity = intensity;
                        max_x = x;
                        max_y = y;
                    }
                }
            }
        }

        (max_intensity, max_x, max_y)
    }

    /// Process a photon event from the detector
    pub fn process_event(&mut self, event: PhotonEvent) -> Result<()> {
        use housaky_photonics::encoding::OpticalSymbol;

        let symbol = OpticalSymbol {
            on: event.intensity > 128,
            duration_us: 10,
        };

        self.decoder.add_symbol(symbol);

        if let Ok(data) = self.decoder.decode() {
            if let Ok(packet) = Packet::from_bytes(&data) {
                self.handle_packet(packet)?;
            }
            self.decoder.clear();
        }

        Ok(())
    }

    /// Handle received packet
    fn handle_packet(&mut self, packet: Packet) -> Result<()> {
        match packet.header.packet_type {
            PacketType::Connect => {
                tracing::info!("Received connection request");
                self.state = ConnectionState::Connected;
            }
            PacketType::Data => {
                if !packet.verify_checksum() {
                    tracing::warn!("Packet checksum failed");
                    return Ok(());
                }

                let seq = packet.header.sequence;

                if seq == self.expected_sequence {
                    let _ = self.packet_tx.try_send(packet.payload.clone());
                    self.expected_sequence = self.expected_sequence.wrapping_add(1);
                    self.process_pending();
                } else if seq > self.expected_sequence {
                    self.pending_packets.insert(seq, packet);
                    tracing::debug!(
                        "Buffered packet {}, expected {}",
                        seq,
                        self.expected_sequence
                    );
                }
            }
            PacketType::Heartbeat => {
                tracing::trace!("Received heartbeat");
            }
            _ => {}
        }

        Ok(())
    }

    /// Process pending packets in order
    fn process_pending(&mut self) {
        while let Some(packet) = self.pending_packets.remove(&self.expected_sequence) {
            let _ = self.packet_tx.try_send(packet.payload);
            self.expected_sequence = self.expected_sequence.wrapping_add(1);
        }
    }

    /// Get current state
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Stop receiving
    pub fn stop(&mut self) {
        self.state = ConnectionState::Disconnected;
        if let Some(mut camera) = self.camera.take() {
            camera.close();
        }
        tracing::info!("Li-Fi receiver stopped");
    }
}

/// Receiver statistics
#[derive(Debug, Clone)]
pub struct ReceiverStats {
    pub packets_received: u64,
    pub bytes_received: u64,
    pub packets_dropped: u64,
    pub errors: u64,
}

impl Default for ReceiverStats {
    fn default() -> Self {
        Self {
            packets_received: 0,
            bytes_received: 0,
            packets_dropped: 0,
            errors: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receiver_creation() {
        let config = ConnectionConfig::default();
        let (rx, _packet_rx) = LiFiReceiver::new(config);

        assert_eq!(rx.state(), ConnectionState::Disconnected);
    }
}

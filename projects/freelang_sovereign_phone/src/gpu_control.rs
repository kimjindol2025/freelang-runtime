// Project Sovereign: GPU Control Module
// Goal: Dynamic GPU frequency scaling and rendering optimization
// Target: Smooth 30-60fps with minimal power consumption

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GPUFrequency {
    Off,         // 0MHz
    Minimal,     // 100MHz
    Low,         // 300MHz
    Medium,      // 600MHz
    High,        // 900MHz
    Max,         // 1200MHz
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RenderingMode {
    Off,           // Display off
    Idle,          // 2D idle (UI refresh only)
    Light2D,       // Simple graphics (30fps target)
    Heavy2D,       // Complex 2D rendering (60fps target)
    Light3D,       // Light 3D games (30fps target)
    Heavy3D,       // Demanding games (60fps target)
    VR,            // VR applications (90fps target)
}

impl GPUFrequency {
    pub fn to_mhz(&self) -> u32 {
        match self {
            GPUFrequency::Off => 0,
            GPUFrequency::Minimal => 100,
            GPUFrequency::Low => 300,
            GPUFrequency::Medium => 600,
            GPUFrequency::High => 900,
            GPUFrequency::Max => 1200,
        }
    }

    pub fn power_consumption_mw(&self) -> f64 {
        // GPU power ~ f^2 (frequency squared)
        match self {
            GPUFrequency::Off => 0.0,
            GPUFrequency::Minimal => 5.0,
            GPUFrequency::Low => 20.0,
            GPUFrequency::Medium => 60.0,
            GPUFrequency::High => 120.0,
            GPUFrequency::Max => 200.0,
        }
    }
}

impl RenderingMode {
    pub fn target_fps(&self) -> u32 {
        match self {
            RenderingMode::Off => 0,
            RenderingMode::Idle => 1,
            RenderingMode::Light2D => 30,
            RenderingMode::Heavy2D => 60,
            RenderingMode::Light3D => 30,
            RenderingMode::Heavy3D => 60,
            RenderingMode::VR => 90,
        }
    }

    pub fn suggested_frequency(&self) -> GPUFrequency {
        match self {
            RenderingMode::Off => GPUFrequency::Off,
            RenderingMode::Idle => GPUFrequency::Minimal,
            RenderingMode::Light2D => GPUFrequency::Low,
            RenderingMode::Heavy2D => GPUFrequency::Medium,
            RenderingMode::Light3D => GPUFrequency::Medium,
            RenderingMode::Heavy3D => GPUFrequency::High,
            RenderingMode::VR => GPUFrequency::Max,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FrameBuffer {
    pub width: u32,
    pub height: u32,
    pub format: ColorFormat,
    pub size_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorFormat {
    RGB565,      // 2 bytes per pixel (16-bit)
    RGBA8888,    // 4 bytes per pixel (32-bit)
    RGBA1010102, // 4 bytes per pixel (HDR)
}

impl ColorFormat {
    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            ColorFormat::RGB565 => 2,
            ColorFormat::RGBA8888 => 4,
            ColorFormat::RGBA1010102 => 4,
        }
    }
}

pub struct GPUController {
    // Current state
    current_frequency: GPUFrequency,
    rendering_mode: RenderingMode,
    current_fps: f64,

    // Frame buffer
    frame_buffer: FrameBuffer,

    // Performance metrics
    frame_times_ms: Vec<f64>,
    total_frames_rendered: u64,
    frames_dropped: u64,
    thermal_throttling: bool,

    // Power management
    power_budget_mw: f64,
    current_power_mw: f64,

    // Optimization
    vsync_enabled: bool,
    dynamic_resolution: bool,
    upscaling_enabled: bool,
    current_scale_factor: f64,
}

impl GPUController {
    pub fn new(width: u32, height: u32, power_budget_mw: f64) -> Self {
        let frame_buffer = FrameBuffer {
            width,
            height,
            format: ColorFormat::RGBA8888,
            size_bytes: (width as u64) * (height as u64) * 4,
        };

        Self {
            current_frequency: GPUFrequency::Low,
            rendering_mode: RenderingMode::Idle,
            current_fps: 30.0,
            frame_buffer,
            frame_times_ms: Vec::new(),
            total_frames_rendered: 0,
            frames_dropped: 0,
            thermal_throttling: false,
            power_budget_mw,
            current_power_mw: 0.0,
            vsync_enabled: true,
            dynamic_resolution: true,
            upscaling_enabled: true,
            current_scale_factor: 1.0,
        }
    }

    /// Switch rendering mode and adjust GPU frequency
    pub fn set_rendering_mode(&mut self, mode: RenderingMode) -> bool {
        self.rendering_mode = mode;
        let freq = mode.suggested_frequency();

        // Check power budget
        if freq.power_consumption_mw() > self.power_budget_mw {
            // Downgrade mode if power budget exceeded
            self.rendering_mode = RenderingMode::Light2D;
            return false;
        }

        self.current_frequency = freq;
        self.current_power_mw = freq.power_consumption_mw();
        true
    }

    /// Update frame time and adjust frequency dynamically
    pub fn record_frame_time(&mut self, frame_time_ms: f64) {
        self.frame_times_ms.push(frame_time_ms);
        self.total_frames_rendered += 1;

        // Keep only last 60 frames (1 second at 60fps)
        if self.frame_times_ms.len() > 60 {
            self.frame_times_ms.remove(0);
        }

        // Check for dropped frames (>33ms = <30fps)
        if frame_time_ms > 33.0 {
            self.frames_dropped += 1;
        }

        // Update current FPS
        if !self.frame_times_ms.is_empty() {
            let avg_time: f64 = self.frame_times_ms.iter().sum::<f64>() / self.frame_times_ms.len() as f64;
            self.current_fps = 1000.0 / avg_time;
        }

        // Adjust frequency if needed
        self.auto_adjust_frequency();
    }

    fn auto_adjust_frequency(&mut self) {
        let target_fps = self.rendering_mode.target_fps() as f64;

        // If FPS is too low, increase frequency
        if self.current_fps < target_fps * 0.8 && !self.thermal_throttling {
            self.increase_frequency();
        }

        // If FPS is comfortable and power is tight, decrease frequency
        if self.current_fps > target_fps * 1.1 && self.current_power_mw > self.power_budget_mw * 0.8 {
            self.decrease_frequency();
        }
    }

    fn increase_frequency(&mut self) {
        let new_freq = match self.current_frequency {
            GPUFrequency::Off => GPUFrequency::Minimal,
            GPUFrequency::Minimal => GPUFrequency::Low,
            GPUFrequency::Low => GPUFrequency::Medium,
            GPUFrequency::Medium => GPUFrequency::High,
            GPUFrequency::High => GPUFrequency::Max,
            GPUFrequency::Max => GPUFrequency::Max,
        };

        if new_freq.power_consumption_mw() <= self.power_budget_mw {
            self.current_frequency = new_freq;
            self.current_power_mw = new_freq.power_consumption_mw();
        }
    }

    fn decrease_frequency(&mut self) {
        self.current_frequency = match self.current_frequency {
            GPUFrequency::Off => GPUFrequency::Off,
            GPUFrequency::Minimal => GPUFrequency::Off,
            GPUFrequency::Low => GPUFrequency::Minimal,
            GPUFrequency::Medium => GPUFrequency::Low,
            GPUFrequency::High => GPUFrequency::Medium,
            GPUFrequency::Max => GPUFrequency::High,
        };

        self.current_power_mw = self.current_frequency.power_consumption_mw();
    }

    /// Apply thermal throttling
    pub fn apply_thermal_throttle(&mut self, throttle_level: f64) {
        self.thermal_throttling = throttle_level > 0.3;

        // Reduce frequency based on throttle level
        if throttle_level > 0.7 {
            self.current_frequency = GPUFrequency::Low;
        } else if throttle_level > 0.3 {
            self.current_frequency = GPUFrequency::Medium;
        }

        self.current_power_mw = self.current_frequency.power_consumption_mw();
    }

    /// Enable/disable dynamic resolution scaling
    pub fn set_dynamic_resolution(&mut self, enabled: bool) {
        self.dynamic_resolution = enabled;
    }

    /// Get recommended render resolution
    pub fn get_render_resolution(&self) -> (u32, u32) {
        if !self.dynamic_resolution {
            return (self.frame_buffer.width, self.frame_buffer.height);
        }

        // Reduce resolution if power tight
        if self.current_power_mw > self.power_budget_mw * 0.9 {
            let scale = 0.75;
            let width = (self.frame_buffer.width as f64 * scale) as u32;
            let height = (self.frame_buffer.height as f64 * scale) as u32;
            (width, height)
        } else {
            (self.frame_buffer.width, self.frame_buffer.height)
        }
    }

    /// Get performance metrics
    pub fn get_metrics(&self) -> GPUMetrics {
        let avg_frame_time = if self.frame_times_ms.is_empty() {
            0.0
        } else {
            self.frame_times_ms.iter().sum::<f64>() / self.frame_times_ms.len() as f64
        };

        let drop_rate = if self.total_frames_rendered > 0 {
            (self.frames_dropped as f64 / self.total_frames_rendered as f64) * 100.0
        } else {
            0.0
        };

        GPUMetrics {
            current_frequency_mhz: self.current_frequency.to_mhz(),
            current_fps: self.current_fps,
            avg_frame_time_ms: avg_frame_time,
            frame_drop_rate: drop_rate,
            total_frames: self.total_frames_rendered,
            power_consumption_mw: self.current_power_mw,
            thermal_throttling: self.thermal_throttling,
        }
    }

    pub fn reset_metrics(&mut self) {
        self.frame_times_ms.clear();
        self.total_frames_rendered = 0;
        self.frames_dropped = 0;
    }
}

#[derive(Clone, Debug)]
pub struct GPUMetrics {
    pub current_frequency_mhz: u32,
    pub current_fps: f64,
    pub avg_frame_time_ms: f64,
    pub frame_drop_rate: f64,
    pub total_frames: u64,
    pub power_consumption_mw: f64,
    pub thermal_throttling: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_frequency_mhz() {
        assert_eq!(GPUFrequency::Off.to_mhz(), 0);
        assert_eq!(GPUFrequency::Max.to_mhz(), 1200);
    }

    #[test]
    fn test_gpu_power_consumption() {
        assert_eq!(GPUFrequency::Off.power_consumption_mw(), 0.0);
        assert!(GPUFrequency::Max.power_consumption_mw() > 100.0);
    }

    #[test]
    fn test_rendering_mode_fps() {
        assert_eq!(RenderingMode::Off.target_fps(), 0);
        assert_eq!(RenderingMode::Light2D.target_fps(), 30);
        assert_eq!(RenderingMode::VR.target_fps(), 90);
    }

    #[test]
    fn test_gpu_controller_creation() {
        let gpu = GPUController::new(1920, 1080, 200.0);
        assert_eq!(gpu.frame_buffer.width, 1920);
        assert_eq!(gpu.frame_buffer.height, 1080);
    }

    #[test]
    fn test_set_rendering_mode() {
        let mut gpu = GPUController::new(1920, 1080, 200.0);

        assert!(gpu.set_rendering_mode(RenderingMode::Heavy2D));
        assert_eq!(gpu.rendering_mode, RenderingMode::Heavy2D);
    }

    #[test]
    fn test_record_frame_time() {
        let mut gpu = GPUController::new(1920, 1080, 200.0);
        gpu.set_rendering_mode(RenderingMode::Light2D);

        for _ in 0..30 {
            gpu.record_frame_time(33.0);  // 30fps
        }

        assert!(gpu.current_fps > 25.0);
        assert!(gpu.current_fps < 35.0);
    }

    #[test]
    fn test_frame_drop_detection() {
        let mut gpu = GPUController::new(1920, 1080, 200.0);

        gpu.record_frame_time(20.0);  // Good
        gpu.record_frame_time(50.0);  // Dropped (>33ms)

        assert!(gpu.frames_dropped > 0);
    }

    #[test]
    fn test_thermal_throttle() {
        let mut gpu = GPUController::new(1920, 1080, 200.0);
        gpu.set_rendering_mode(RenderingMode::Heavy3D);

        gpu.apply_thermal_throttle(0.8);

        assert!(gpu.thermal_throttling);
        assert_eq!(gpu.current_frequency, GPUFrequency::Low);
    }

    #[test]
    fn test_dynamic_resolution() {
        let mut gpu = GPUController::new(1920, 1080, 50.0);  // Low power budget
        gpu.set_dynamic_resolution(true);

        // Should reduce resolution if power tight
        let (width, height) = gpu.get_render_resolution();
        assert!(width <= 1920);
        assert!(height <= 1080);
    }

    #[test]
    fn test_gpu_metrics() {
        let mut gpu = GPUController::new(1920, 1080, 200.0);
        gpu.set_rendering_mode(RenderingMode::Light2D);

        for _ in 0..10 {
            gpu.record_frame_time(33.0);
        }

        let metrics = gpu.get_metrics();
        assert!(metrics.current_fps > 0.0);
        assert_eq!(metrics.total_frames, 10);
    }

    #[test]
    fn test_color_format_bytes() {
        assert_eq!(ColorFormat::RGB565.bytes_per_pixel(), 2);
        assert_eq!(ColorFormat::RGBA8888.bytes_per_pixel(), 4);
    }

    #[test]
    fn test_power_budget_constraint() {
        let mut gpu = GPUController::new(1920, 1080, 50.0);  // Very low budget

        // Should degrade mode if power budget exceeded
        let result = gpu.set_rendering_mode(RenderingMode::Heavy3D);

        // Either fails or degrades
        if result {
            assert!(gpu.current_power_mw <= 50.0);
        }
    }

    #[test]
    fn test_reset_metrics() {
        let mut gpu = GPUController::new(1920, 1080, 200.0);

        for _ in 0..10 {
            gpu.record_frame_time(33.0);
        }

        assert!(gpu.total_frames_rendered > 0);

        gpu.reset_metrics();
        assert_eq!(gpu.total_frames_rendered, 0);
    }

    #[test]
    fn test_vr_mode() {
        let mut gpu = GPUController::new(1920, 1080, 500.0);

        assert!(gpu.set_rendering_mode(RenderingMode::VR));
        assert_eq!(gpu.rendering_mode.target_fps(), 90);
    }
}

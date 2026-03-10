// Project Sovereign: Public API Interface Module
// Goal: External system control and telemetry interface
// Target: <5ms response time, 100% API availability

use std::sync::{Arc, Mutex};
use crate::system_adaptation::{BatteryMode, ThermalState, WorkloadClass};
use crate::cpu_frequency::CPUFrequency;
use crate::gpu_control::GPUFrequency;
use crate::optimization_engine::{OptimizationStrategy, OptimizationMetrics};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemMode {
    Idle,
    Interactive,
    Background,
    Performance,
    LowPower,
    Emergency,
}

#[derive(Clone, Debug)]
pub struct SystemStatus {
    pub current_mode: SystemMode,
    pub battery_percent: f64,
    pub battery_mode: String,
    pub cpu_frequency_mhz: u32,
    pub gpu_frequency_mhz: u32,
    pub temperature_celsius: f64,
    pub memory_pressure_percent: f64,
    pub uptime_seconds: u64,
    pub power_consumption_mw: f64,
}

#[derive(Clone, Debug)]
pub struct PerformanceMetrics {
    pub avg_frame_time_ms: f64,
    pub frame_drop_rate: f64,
    pub app_startup_time_ms: f64,
    pub memory_usage_mb: f64,
    pub cpu_load_percent: f64,
    pub battery_drain_mw: f64,
    pub thermal_efficiency: f64,  // 0.0-1.0
}

#[derive(Clone, Debug)]
pub struct TelemetryData {
    pub timestamp: u64,
    pub system_status: SystemStatus,
    pub performance_metrics: PerformanceMetrics,
    pub optimization_metrics: OptimizationMetrics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum APIError {
    InvalidMode,
    OutOfBounds,
    SystemBusy,
    NotInitialized,
    PermissionDenied,
    InternalError,
}

pub type APIResult<T> = Result<T, APIError>;

pub trait SystemController {
    /// Set system mode
    fn set_mode(&mut self, mode: SystemMode) -> APIResult<()>;

    /// Get current system status
    fn get_status(&self) -> APIResult<SystemStatus>;

    /// Get performance metrics
    fn get_metrics(&self) -> APIResult<PerformanceMetrics>;

    /// Get telemetry data
    fn get_telemetry(&self) -> APIResult<TelemetryData>;

    /// Request performance boost
    fn request_performance_boost(&mut self, duration_ms: u32) -> APIResult<()>;

    /// Request power saving mode
    fn request_power_saving(&mut self) -> APIResult<()>;

    /// Get optimization suggestions
    fn get_optimization_suggestions(&self) -> APIResult<Vec<String>>;

    /// Apply optimization
    fn apply_optimization(&mut self, strategy: OptimizationStrategy) -> APIResult<()>;
}

pub struct SovereignAPI {
    // System state (protected by mutex for thread-safety)
    system_status: Arc<Mutex<SystemStatus>>,
    performance_metrics: Arc<Mutex<PerformanceMetrics>>,
    optimization_metrics: Arc<Mutex<OptimizationMetrics>>,

    // Configuration
    api_version: String,
    max_requests_per_second: usize,
    request_timeout_ms: u32,

    // Statistics
    total_requests: usize,
    successful_requests: usize,
    failed_requests: usize,
}

impl SovereignAPI {
    pub fn new() -> Self {
        Self {
            system_status: Arc::new(Mutex::new(SystemStatus {
                current_mode: SystemMode::Idle,
                battery_percent: 100.0,
                battery_mode: "Excellent".to_string(),
                cpu_frequency_mhz: 1500,
                gpu_frequency_mhz: 300,
                temperature_celsius: 25.0,
                memory_pressure_percent: 30.0,
                uptime_seconds: 0,
                power_consumption_mw: 500.0,
            })),
            performance_metrics: Arc::new(Mutex::new(PerformanceMetrics {
                avg_frame_time_ms: 16.7,
                frame_drop_rate: 0.0,
                app_startup_time_ms: 800.0,
                memory_usage_mb: 2048.0,
                cpu_load_percent: 20.0,
                battery_drain_mw: 500.0,
                thermal_efficiency: 0.85,
            })),
            optimization_metrics: Arc::new(Mutex::new(OptimizationMetrics {
                total_optimizations: 0,
                successful_optimizations: 0,
                average_power_savings_mw: 0.0,
                average_latency_impact_ms: 0.0,
                false_positive_rate: 0.0,
                effectiveness_score: 0.0,
            })),

            api_version: "1.0.0".to_string(),
            max_requests_per_second: 1000,
            request_timeout_ms: 5,

            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
        }
    }

    /// Update system status (called by core system)
    pub fn update_system_status(&mut self, status: SystemStatus) -> APIResult<()> {
        let mut state = self.system_status.lock().map_err(|_| APIError::SystemBusy)?;
        *state = status;
        self.successful_requests += 1;
        Ok(())
    }

    /// Update performance metrics (called by core system)
    pub fn update_metrics(&mut self, metrics: PerformanceMetrics) -> APIResult<()> {
        let mut state = self.performance_metrics.lock().map_err(|_| APIError::SystemBusy)?;
        *state = metrics;
        self.successful_requests += 1;
        Ok(())
    }

    /// Update optimization metrics (called by core system)
    pub fn update_optimization_metrics(&mut self, metrics: OptimizationMetrics) -> APIResult<()> {
        let mut state = self.optimization_metrics.lock().map_err(|_| APIError::SystemBusy)?;
        *state = metrics;
        self.successful_requests += 1;
        Ok(())
    }

    /// Check API health
    pub fn health_check(&self) -> bool {
        // Check if all internal state can be locked (not deadlocked)
        self.system_status.try_lock().is_ok() &&
        self.performance_metrics.try_lock().is_ok() &&
        self.optimization_metrics.try_lock().is_ok()
    }

    /// Get API version
    pub fn get_version(&self) -> String {
        self.api_version.clone()
    }

    /// Get API statistics
    pub fn get_statistics(&self) -> (usize, usize, usize) {
        (self.total_requests, self.successful_requests, self.failed_requests)
    }

    /// Reset statistics
    pub fn reset_statistics(&mut self) {
        self.total_requests = 0;
        self.successful_requests = 0;
        self.failed_requests = 0;
    }
}

impl SystemController for SovereignAPI {
    fn set_mode(&mut self, mode: SystemMode) -> APIResult<()> {
        self.total_requests += 1;

        let mut status = self.system_status.lock().map_err(|_| APIError::SystemBusy)?;

        // Validate mode transition
        match mode {
            SystemMode::Emergency => {
                if status.battery_percent < 5.0 {
                    status.current_mode = mode;
                    self.successful_requests += 1;
                    Ok(())
                } else {
                    self.failed_requests += 1;
                    Err(APIError::InvalidMode)
                }
            }
            SystemMode::LowPower => {
                if status.battery_percent < 30.0 {
                    status.current_mode = mode;
                    self.successful_requests += 1;
                    Ok(())
                } else {
                    status.current_mode = mode;
                    self.successful_requests += 1;
                    Ok(())
                }
            }
            SystemMode::Performance => {
                if status.battery_percent < 10.0 {
                    self.failed_requests += 1;
                    return Err(APIError::InvalidMode);
                }
                status.current_mode = mode;
                self.successful_requests += 1;
                Ok(())
            }
            _ => {
                status.current_mode = mode;
                self.successful_requests += 1;
                Ok(())
            }
        }
    }

    fn get_status(&self) -> APIResult<SystemStatus> {
        self.total_requests += 1;
        let status = self.system_status.lock().map_err(|_| APIError::SystemBusy)?;
        self.successful_requests += 1;
        Ok(status.clone())
    }

    fn get_metrics(&self) -> APIResult<PerformanceMetrics> {
        self.total_requests += 1;
        let metrics = self.performance_metrics.lock().map_err(|_| APIError::SystemBusy)?;
        self.successful_requests += 1;
        Ok(metrics.clone())
    }

    fn get_telemetry(&self) -> APIResult<TelemetryData> {
        self.total_requests += 1;

        let status = self.system_status.lock().map_err(|_| APIError::SystemBusy)?;
        let metrics = self.performance_metrics.lock().map_err(|_| APIError::SystemBusy)?;
        let opt_metrics = self.optimization_metrics.lock().map_err(|_| APIError::SystemBusy)?;

        let telemetry = TelemetryData {
            timestamp: status.uptime_seconds,
            system_status: status.clone(),
            performance_metrics: metrics.clone(),
            optimization_metrics: opt_metrics.clone(),
        };

        self.successful_requests += 1;
        Ok(telemetry)
    }

    fn request_performance_boost(&mut self, duration_ms: u32) -> APIResult<()> {
        self.total_requests += 1;

        if duration_ms > 30000 {
            // Max 30 seconds boost
            self.failed_requests += 1;
            return Err(APIError::OutOfBounds);
        }

        let mut status = self.system_status.lock().map_err(|_| APIError::SystemBusy)?;

        if status.battery_percent < 20.0 {
            self.failed_requests += 1;
            return Err(APIError::InvalidMode);
        }

        status.current_mode = SystemMode::Performance;
        status.cpu_frequency_mhz = 3400;
        status.gpu_frequency_mhz = 1200;
        self.successful_requests += 1;
        Ok(())
    }

    fn request_power_saving(&mut self) -> APIResult<()> {
        self.total_requests += 1;

        let mut status = self.system_status.lock().map_err(|_| APIError::SystemBusy)?;
        status.current_mode = SystemMode::LowPower;
        status.cpu_frequency_mhz = 1200;
        status.gpu_frequency_mhz = 300;
        self.successful_requests += 1;
        Ok(())
    }

    fn get_optimization_suggestions(&self) -> APIResult<Vec<String>> {
        self.total_requests += 1;

        let status = self.system_status.lock().map_err(|_| APIError::SystemBusy)?;
        let metrics = self.performance_metrics.lock().map_err(|_| APIError::SystemBusy)?;

        let mut suggestions = Vec::new();

        // CPU optimization
        if metrics.cpu_load_percent > 80.0 {
            suggestions.push("Reduce CPU workload or reduce frame rate".to_string());
        }

        // Memory optimization
        if status.memory_pressure_percent > 85.0 {
            suggestions.push("Close background applications and clear cache".to_string());
        }

        // Thermal optimization
        if status.temperature_celsius > 45.0 {
            suggestions.push("Reduce load and enable active cooling".to_string());
        }

        // Battery optimization
        if status.battery_percent < 20.0 && status.power_consumption_mw > 700.0 {
            suggestions.push("Enable low power mode to extend battery life".to_string());
        }

        // Frame rate optimization
        if metrics.frame_drop_rate > 5.0 {
            suggestions.push("Reduce screen refresh rate or application complexity".to_string());
        }

        self.successful_requests += 1;
        Ok(suggestions)
    }

    fn apply_optimization(&mut self, strategy: OptimizationStrategy) -> APIResult<()> {
        self.total_requests += 1;

        let mut status = self.system_status.lock().map_err(|_| APIError::SystemBusy)?;

        match strategy {
            OptimizationStrategy::PowerSaving => {
                status.cpu_frequency_mhz = 1200;
                status.gpu_frequency_mhz = 300;
                status.current_mode = SystemMode::LowPower;
            }
            OptimizationStrategy::Balanced => {
                status.cpu_frequency_mhz = 2000;
                status.gpu_frequency_mhz = 600;
                status.current_mode = SystemMode::Interactive;
            }
            OptimizationStrategy::Performance => {
                status.cpu_frequency_mhz = 3400;
                status.gpu_frequency_mhz = 1200;
                status.current_mode = SystemMode::Performance;
            }
            OptimizationStrategy::ThermalControl => {
                status.cpu_frequency_mhz = 1500;
                status.gpu_frequency_mhz = 400;
                status.current_mode = SystemMode::Interactive;
            }
            OptimizationStrategy::MemoryOptimization => {
                status.memory_pressure_percent = status.memory_pressure_percent * 0.7;
            }
        }

        self.successful_requests += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_creation() {
        let api = SovereignAPI::new();
        assert_eq!(api.get_version(), "1.0.0");
    }

    #[test]
    fn test_health_check() {
        let api = SovereignAPI::new();
        assert!(api.health_check());
    }

    #[test]
    fn test_get_status() {
        let api = SovereignAPI::new();
        let status = api.get_status().unwrap();

        assert_eq!(status.current_mode, SystemMode::Idle);
        assert_eq!(status.battery_percent, 100.0);
    }

    #[test]
    fn test_get_metrics() {
        let api = SovereignAPI::new();
        let metrics = api.get_metrics().unwrap();

        assert_eq!(metrics.avg_frame_time_ms, 16.7);
    }

    #[test]
    fn test_set_mode_interactive() {
        let mut api = SovereignAPI::new();
        let result = api.set_mode(SystemMode::Interactive);

        assert!(result.is_ok());
        let status = api.get_status().unwrap();
        assert_eq!(status.current_mode, SystemMode::Interactive);
    }

    #[test]
    fn test_set_mode_performance_low_battery() {
        let mut api = SovereignAPI::new();

        // Set low battery
        let mut status = SystemStatus {
            current_mode: SystemMode::Idle,
            battery_percent: 8.0,
            battery_mode: "Critical".to_string(),
            cpu_frequency_mhz: 1500,
            gpu_frequency_mhz: 300,
            temperature_celsius: 25.0,
            memory_pressure_percent: 30.0,
            uptime_seconds: 0,
            power_consumption_mw: 500.0,
        };

        let _ = api.update_system_status(status);

        // Performance mode should fail
        let result = api.set_mode(SystemMode::Performance);
        assert!(result.is_err());
    }

    #[test]
    fn test_set_mode_emergency() {
        let mut api = SovereignAPI::new();

        // Set critical battery
        let status = SystemStatus {
            current_mode: SystemMode::Idle,
            battery_percent: 3.0,
            battery_mode: "Emergency".to_string(),
            cpu_frequency_mhz: 1500,
            gpu_frequency_mhz: 300,
            temperature_celsius: 25.0,
            memory_pressure_percent: 30.0,
            uptime_seconds: 0,
            power_consumption_mw: 500.0,
        };

        let _ = api.update_system_status(status);
        let result = api.set_mode(SystemMode::Emergency);
        assert!(result.is_ok());
    }

    #[test]
    fn test_request_performance_boost() {
        let mut api = SovereignAPI::new();
        let result = api.request_performance_boost(5000);

        assert!(result.is_ok());
        let status = api.get_status().unwrap();
        assert_eq!(status.current_mode, SystemMode::Performance);
        assert_eq!(status.cpu_frequency_mhz, 3400);
    }

    #[test]
    fn test_request_performance_boost_too_long() {
        let mut api = SovereignAPI::new();
        let result = api.request_performance_boost(50000);

        assert!(result.is_err());
    }

    #[test]
    fn test_request_performance_boost_low_battery() {
        let mut api = SovereignAPI::new();

        let status = SystemStatus {
            current_mode: SystemMode::Idle,
            battery_percent: 15.0,
            battery_mode: "Warning".to_string(),
            cpu_frequency_mhz: 1500,
            gpu_frequency_mhz: 300,
            temperature_celsius: 25.0,
            memory_pressure_percent: 30.0,
            uptime_seconds: 0,
            power_consumption_mw: 500.0,
        };

        let _ = api.update_system_status(status);
        let result = api.request_performance_boost(5000);
        assert!(result.is_err());
    }

    #[test]
    fn test_request_power_saving() {
        let mut api = SovereignAPI::new();
        let result = api.request_power_saving();

        assert!(result.is_ok());
        let status = api.get_status().unwrap();
        assert_eq!(status.current_mode, SystemMode::LowPower);
    }

    #[test]
    fn test_get_optimization_suggestions() {
        let api = SovereignAPI::new();
        let suggestions = api.get_optimization_suggestions().unwrap();

        assert!(suggestions.is_empty()); // All metrics are healthy
    }

    #[test]
    fn test_get_optimization_suggestions_high_cpu() {
        let mut api = SovereignAPI::new();

        let metrics = PerformanceMetrics {
            avg_frame_time_ms: 33.0,
            frame_drop_rate: 2.0,
            app_startup_time_ms: 1500.0,
            memory_usage_mb: 3000.0,
            cpu_load_percent: 85.0,
            battery_drain_mw: 800.0,
            thermal_efficiency: 0.70,
        };

        let _ = api.update_metrics(metrics);
        let suggestions = api.get_optimization_suggestions().unwrap();

        assert!(!suggestions.is_empty());
        assert!(suggestions[0].contains("CPU"));
    }

    #[test]
    fn test_get_telemetry() {
        let api = SovereignAPI::new();
        let telemetry = api.get_telemetry().unwrap();

        assert_eq!(telemetry.system_status.current_mode, SystemMode::Idle);
    }

    #[test]
    fn test_apply_optimization_power_saving() {
        let mut api = SovereignAPI::new();
        let result = api.apply_optimization(OptimizationStrategy::PowerSaving);

        assert!(result.is_ok());
        let status = api.get_status().unwrap();
        assert_eq!(status.current_mode, SystemMode::LowPower);
        assert_eq!(status.cpu_frequency_mhz, 1200);
    }

    #[test]
    fn test_apply_optimization_performance() {
        let mut api = SovereignAPI::new();
        let result = api.apply_optimization(OptimizationStrategy::Performance);

        assert!(result.is_ok());
        let status = api.get_status().unwrap();
        assert_eq!(status.current_mode, SystemMode::Performance);
        assert_eq!(status.cpu_frequency_mhz, 3400);
    }

    #[test]
    fn test_apply_optimization_thermal_control() {
        let mut api = SovereignAPI::new();
        let result = api.apply_optimization(OptimizationStrategy::ThermalControl);

        assert!(result.is_ok());
        let status = api.get_status().unwrap();
        assert!(status.cpu_frequency_mhz < 2000);
    }

    #[test]
    fn test_api_statistics() {
        let mut api = SovereignAPI::new();

        let _ = api.get_status();
        let _ = api.get_metrics();
        let _ = api.request_power_saving();

        let (total, success, failed) = api.get_statistics();
        assert_eq!(total, 3);
        assert!(success > 0);
    }

    #[test]
    fn test_reset_statistics() {
        let mut api = SovereignAPI::new();

        let _ = api.get_status();
        let _ = api.get_metrics();

        api.reset_statistics();
        let (total, _, _) = api.get_statistics();
        assert_eq!(total, 0);
    }

    #[test]
    fn test_update_system_status() {
        let mut api = SovereignAPI::new();

        let new_status = SystemStatus {
            current_mode: SystemMode::Performance,
            battery_percent: 50.0,
            battery_mode: "Good".to_string(),
            cpu_frequency_mhz: 3000,
            gpu_frequency_mhz: 1000,
            temperature_celsius: 40.0,
            memory_pressure_percent: 60.0,
            uptime_seconds: 1000,
            power_consumption_mw: 1000.0,
        };

        let result = api.update_system_status(new_status.clone());
        assert!(result.is_ok());

        let current = api.get_status().unwrap();
        assert_eq!(current.battery_percent, 50.0);
    }

    #[test]
    fn test_concurrent_access() {
        let api = SovereignAPI::new();

        // Simulate multiple concurrent reads
        let result1 = api.get_status();
        let result2 = api.get_metrics();
        let result3 = api.get_telemetry();

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert!(result3.is_ok());
    }
}

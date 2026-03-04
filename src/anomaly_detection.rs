// Project Sovereign: AnomalyDetection Module
// Goal: Detect system anomalies before they impact user experience
// Target: 85% detection rate for battery drain, temperature spikes, memory leaks, network issues

use std::collections::VecDeque;

pub type Timestamp = u64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnomalyType {
    BatteryDrain,      // Unexplained battery loss
    TemperatureSpike,  // Rapid temperature increase
    MemoryLeak,        // Memory not being released
    NetworkAnomaly,    // Unusual network pattern
    SecurityThreat,    // Suspicious process/network activity
    CPUStall,          // CPU hung or spinning
    None,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SeverityLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug)]
pub struct AnomalyEvent {
    pub anomaly_type: AnomalyType,
    pub severity: SeverityLevel,
    pub confidence: f64,
    pub timestamp: Timestamp,
    pub details: String,
}

#[derive(Clone, Debug)]
pub struct SystemMetrics {
    pub battery_level: f64,         // 0.0 - 1.0
    pub battery_drain_rate: f64,    // mW
    pub temperature: f64,           // °C
    pub memory_used_mb: f64,
    pub memory_available_mb: f64,
    pub network_bytes_in: u64,
    pub network_bytes_out: u64,
    pub cpu_usage: f64,             // 0.0 - 1.0
    pub active_processes: usize,
    pub timestamp: Timestamp,
}

struct DetectionBaseline {
    battery_drain: f64,              // Normal drain rate
    temperature: f64,                // Normal operating temp
    memory_usage: f64,               // Normal memory footprint
    network_activity: f64,           // Normal network bytes/sec
    cpu_utilization: f64,            // Normal CPU %
}

pub struct AnomalyDetector {
    // Historical data windows
    battery_history: VecDeque<(Timestamp, f64)>,
    temperature_history: VecDeque<(Timestamp, f64)>,
    memory_history: VecDeque<(Timestamp, f64)>,
    network_history: VecDeque<(Timestamp, f64)>,
    cpu_history: VecDeque<(Timestamp, f64)>,

    // Baselines (learned from normal operation)
    baseline: DetectionBaseline,

    // Statistics
    total_anomalies_detected: usize,
    battery_drain_flags: usize,
    thermal_flags: usize,
    memory_flags: usize,
    network_flags: usize,
    security_flags: usize,

    // Configuration
    window_size: usize,
    std_dev_threshold: f64,
    rate_of_change_threshold: f64,
}

impl AnomalyDetector {
    pub fn new() -> Self {
        Self {
            battery_history: VecDeque::with_capacity(120),
            temperature_history: VecDeque::with_capacity(120),
            memory_history: VecDeque::with_capacity(120),
            network_history: VecDeque::with_capacity(120),
            cpu_history: VecDeque::with_capacity(120),

            baseline: DetectionBaseline {
                battery_drain: 5.0,      // mW normal drain
                temperature: 35.0,       // °C normal
                memory_usage: 2000.0,    // MB typical
                network_activity: 1024.0, // bytes/sec
                cpu_utilization: 0.3,    // 30% typical
            },

            total_anomalies_detected: 0,
            battery_drain_flags: 0,
            thermal_flags: 0,
            memory_flags: 0,
            network_flags: 0,
            security_flags: 0,

            window_size: 120,  // 2 minutes at 1-second intervals
            std_dev_threshold: 2.0,  // 2 standard deviations
            rate_of_change_threshold: 1.5,  // 1.5x normal rate
        }
    }

    /// Update detector with new system metrics
    pub fn update_metrics(&mut self, metrics: SystemMetrics) -> Option<AnomalyEvent> {
        self.record_metric(&mut self.battery_history, metrics.timestamp, metrics.battery_drain_rate);
        self.record_metric(&mut self.temperature_history, metrics.timestamp, metrics.temperature);
        self.record_metric(&mut self.memory_history, metrics.timestamp, metrics.memory_used_mb);
        self.record_metric(&mut self.network_history, metrics.timestamp, metrics.network_bytes_in as f64);
        self.record_metric(&mut self.cpu_history, metrics.timestamp, metrics.cpu_usage);

        // Run detection algorithms
        if let Some(anomaly) = self.detect_battery_drain(&metrics) {
            self.total_anomalies_detected += 1;
            self.battery_drain_flags += 1;
            return Some(anomaly);
        }

        if let Some(anomaly) = self.detect_temperature_spike(&metrics) {
            self.total_anomalies_detected += 1;
            self.thermal_flags += 1;
            return Some(anomaly);
        }

        if let Some(anomaly) = self.detect_memory_leak(&metrics) {
            self.total_anomalies_detected += 1;
            self.memory_flags += 1;
            return Some(anomaly);
        }

        if let Some(anomaly) = self.detect_network_anomaly(&metrics) {
            self.total_anomalies_detected += 1;
            self.network_flags += 1;
            return Some(anomaly);
        }

        if let Some(anomaly) = self.detect_cpu_stall(&metrics) {
            self.total_anomalies_detected += 1;
            self.security_flags += 1;
            return Some(anomaly);
        }

        None
    }

    fn record_metric(
        &self,
        history: &mut VecDeque<(Timestamp, f64)>,
        timestamp: Timestamp,
        value: f64,
    ) {
        history.push_back((timestamp, value));
        if history.len() > self.window_size {
            history.pop_front();
        }
    }

    /// Detect battery drain beyond expected rate
    fn detect_battery_drain(&mut self, metrics: &SystemMetrics) -> Option<AnomalyEvent> {
        if metrics.battery_level < 0.05 {
            return None;  // Don't flag when battery already critical
        }

        let current_drain = metrics.battery_drain_rate;

        // Check if drain rate is abnormally high
        if let Some(mean) = self.calculate_mean(&self.battery_history) {
            let deviation = current_drain - mean;
            let threshold = self.baseline.battery_drain * self.rate_of_change_threshold;

            if deviation > threshold {
                let confidence = (deviation / threshold).min(1.0);

                return Some(AnomalyEvent {
                    anomaly_type: AnomalyType::BatteryDrain,
                    severity: if current_drain > 20.0 {
                        SeverityLevel::High
                    } else {
                        SeverityLevel::Medium
                    },
                    confidence,
                    timestamp: metrics.timestamp,
                    details: format!(
                        "Battery drain {}mW (baseline: {}mW)",
                        current_drain as u32, mean as u32
                    ),
                });
            }
        }

        None
    }

    /// Detect rapid temperature increase
    fn detect_temperature_spike(&mut self, metrics: &SystemMetrics) -> Option<AnomalyEvent> {
        let current_temp = metrics.temperature;

        if let Some((prev_timestamp, prev_temp)) = self.temperature_history.back() {
            let time_delta = metrics.timestamp - prev_timestamp;
            if time_delta == 0 {
                return None;
            }

            // Rate of change: °C per second
            let temp_rate = (current_temp - prev_temp) / (time_delta as f64 / 1000.0);

            // Normal rate: <0.5°C/sec
            if temp_rate > 1.0 {
                let confidence = (temp_rate / 2.0).min(1.0);
                let severity = if current_temp > 55.0 {
                    SeverityLevel::Critical
                } else if current_temp > 50.0 {
                    SeverityLevel::High
                } else {
                    SeverityLevel::Medium
                };

                return Some(AnomalyEvent {
                    anomaly_type: AnomalyType::TemperatureSpike,
                    severity,
                    confidence,
                    timestamp: metrics.timestamp,
                    details: format!(
                        "Temperature rising at {:.2}°C/s (current: {:.1}°C)",
                        temp_rate, current_temp
                    ),
                });
            }
        }

        None
    }

    /// Detect memory leak (memory not being released)
    fn detect_memory_leak(&mut self, metrics: &SystemMetrics) -> Option<AnomalyEvent> {
        let memory_available_ratio = metrics.memory_available_mb
            / (metrics.memory_used_mb + metrics.memory_available_mb);

        if memory_available_ratio < 0.1 {
            // Less than 10% available
            return Some(AnomalyEvent {
                anomaly_type: AnomalyType::MemoryLeak,
                severity: SeverityLevel::High,
                confidence: 0.85,
                timestamp: metrics.timestamp,
                details: format!(
                    "Memory pressure: {:.1}MB used / {:.1}MB total",
                    metrics.memory_used_mb,
                    metrics.memory_used_mb + metrics.memory_available_mb
                ),
            });
        }

        // Check for slow growth trend
        if let Some(mean) = self.calculate_mean(&self.memory_history) {
            let growth = metrics.memory_used_mb - mean;

            // If growing consistently and no major events
            if growth > 500.0 {
                return Some(AnomalyEvent {
                    anomaly_type: AnomalyType::MemoryLeak,
                    severity: SeverityLevel::Medium,
                    confidence: 0.70,
                    timestamp: metrics.timestamp,
                    details: format!(
                        "Memory growing: {}MB increase from baseline",
                        growth as u32
                    ),
                });
            }
        }

        None
    }

    /// Detect unusual network patterns
    fn detect_network_anomaly(&mut self, metrics: &SystemMetrics) -> Option<AnomalyEvent> {
        let total_network = (metrics.network_bytes_in + metrics.network_bytes_out) as f64;

        // Threshold: 10MB/sec is extremely high for mobile
        if total_network > 10_000_000.0 {
            let confidence = (total_network / 20_000_000.0).min(1.0);

            return Some(AnomalyEvent {
                anomaly_type: AnomalyType::NetworkAnomaly,
                severity: SeverityLevel::High,
                confidence,
                timestamp: metrics.timestamp,
                details: format!(
                    "Excessive network activity: {:.1}MB/s",
                    total_network / 1_000_000.0
                ),
            });
        }

        // Detect unusual patterns at 3am (suspicious)
        if total_network > 100_000.0 {
            // More than 100KB/sec at odd hours
            let confidence = 0.65;

            return Some(AnomalyEvent {
                anomaly_type: AnomalyType::NetworkAnomaly,
                severity: SeverityLevel::Medium,
                confidence,
                timestamp: metrics.timestamp,
                details: format!(
                    "Unusual network activity: {:.1}MB/s",
                    total_network / 1_000_000.0
                ),
            });
        }

        None
    }

    /// Detect CPU stalls or spinning
    fn detect_cpu_stall(&mut self, metrics: &SystemMetrics) -> Option<AnomalyEvent> {
        // High CPU usage with low progress = stall
        if metrics.cpu_usage > 0.85 {
            // Check if memory is also growing (may indicate infinite loop)
            if let Some(mem_mean) = self.calculate_mean(&self.memory_history) {
                if metrics.memory_used_mb > mem_mean + 200.0 {
                    return Some(AnomalyEvent {
                        anomaly_type: AnomalyType::CPUStall,
                        severity: SeverityLevel::High,
                        confidence: 0.80,
                        timestamp: metrics.timestamp,
                        details: format!(
                            "CPU high ({}%) with memory growth, possible stall",
                            (metrics.cpu_usage * 100.0) as u32
                        ),
                    });
                }
            }
        }

        None
    }

    /// Calculate mean of recent values
    fn calculate_mean(&self, history: &VecDeque<(Timestamp, f64)>) -> Option<f64> {
        if history.is_empty() {
            return None;
        }

        let sum: f64 = history.iter().map(|(_, v)| v).sum();
        Some(sum / history.len() as f64)
    }

    /// Calculate standard deviation
    fn calculate_std_dev(&self, history: &VecDeque<(Timestamp, f64)>) -> f64 {
        if history.len() < 2 {
            return 0.0;
        }

        if let Some(mean) = self.calculate_mean(history) {
            let variance: f64 = history
                .iter()
                .map(|(_, v)| (v - mean).powi(2))
                .sum::<f64>() / history.len() as f64;

            variance.sqrt()
        } else {
            0.0
        }
    }

    /// Train detector with normal operation data
    pub fn train_baseline(&mut self, metrics: &SystemMetrics) {
        self.baseline.battery_drain = (self.baseline.battery_drain * 0.9)
            + (metrics.battery_drain_rate * 0.1);
        self.baseline.temperature = (self.baseline.temperature * 0.95)
            + (metrics.temperature * 0.05);
        self.baseline.memory_usage = (self.baseline.memory_usage * 0.95)
            + (metrics.memory_used_mb * 0.05);
        self.baseline.cpu_utilization = (self.baseline.cpu_utilization * 0.9)
            + (metrics.cpu_usage * 0.1);
    }

    /// Get detection statistics
    pub fn get_detection_stats(&self) -> DetectionStats {
        let total = self.total_anomalies_detected;

        DetectionStats {
            total_anomalies: total,
            battery_drain_detections: self.battery_drain_flags,
            thermal_detections: self.thermal_flags,
            memory_detections: self.memory_flags,
            network_detections: self.network_flags,
            security_detections: self.security_flags,
            detection_rate: if total > 0 {
                (total as f64) / 1000.0  // Assuming ~1000 checks per session
            } else {
                0.0
            },
        }
    }

    /// Clear old history
    pub fn cleanup_old_data(&mut self, cutoff_timestamp: Timestamp) {
        self.battery_history.retain(|(t, _)| *t > cutoff_timestamp);
        self.temperature_history.retain(|(t, _)| *t > cutoff_timestamp);
        self.memory_history.retain(|(t, _)| *t > cutoff_timestamp);
        self.network_history.retain(|(t, _)| *t > cutoff_timestamp);
        self.cpu_history.retain(|(t, _)| *t > cutoff_timestamp);
    }
}

#[derive(Clone, Debug)]
pub struct DetectionStats {
    pub total_anomalies: usize,
    pub battery_drain_detections: usize,
    pub thermal_detections: usize,
    pub memory_detections: usize,
    pub network_detections: usize,
    pub security_detections: usize,
    pub detection_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_metrics(
        battery: f64,
        drain: f64,
        temp: f64,
        memory: f64,
        network: u64,
        cpu: f64,
    ) -> SystemMetrics {
        SystemMetrics {
            battery_level: battery,
            battery_drain_rate: drain,
            temperature: temp,
            memory_used_mb: memory,
            memory_available_mb: 1000.0,
            network_bytes_in: network,
            network_bytes_out: 0,
            cpu_usage: cpu,
            active_processes: 10,
            timestamp: 1000,
        }
    }

    #[test]
    fn test_detector_creation() {
        let detector = AnomalyDetector::new();
        assert_eq!(detector.total_anomalies_detected, 0);
        assert_eq!(detector.window_size, 120);
    }

    #[test]
    fn test_baseline_training() {
        let mut detector = AnomalyDetector::new();
        let metrics = create_test_metrics(1.0, 5.0, 35.0, 2000.0, 0, 0.3);

        detector.train_baseline(&metrics);
        assert!(detector.baseline.battery_drain > 0.0);
    }

    #[test]
    fn test_detect_battery_drain() {
        let mut detector = AnomalyDetector::new();

        // Normal operation
        for i in 0..10 {
            let metrics = create_test_metrics(1.0, 5.0, 35.0, 2000.0, 0, 0.3);
            detector.train_baseline(&metrics);
        }

        // Abnormal drain
        let mut metrics = create_test_metrics(0.8, 25.0, 35.0, 2000.0, 0, 0.3);
        metrics.timestamp = 5000;

        let anomaly = detector.update_metrics(metrics);
        assert!(anomaly.is_some());
        assert_eq!(anomaly.unwrap().anomaly_type, AnomalyType::BatteryDrain);
    }

    #[test]
    fn test_detect_temperature_spike() {
        let mut detector = AnomalyDetector::new();

        // Normal operation
        let mut metrics = create_test_metrics(1.0, 5.0, 35.0, 2000.0, 0, 0.3);
        detector.update_metrics(metrics);

        // Rapid temperature increase
        metrics.temperature = 45.0;
        metrics.timestamp = 5000;  // 5 seconds later

        let anomaly = detector.update_metrics(metrics);
        assert!(anomaly.is_some());
        assert_eq!(anomaly.unwrap().anomaly_type, AnomalyType::TemperatureSpike);
    }

    #[test]
    fn test_detect_memory_leak() {
        let mut detector = AnomalyDetector::new();

        // Pressure from low available memory
        let metrics = SystemMetrics {
            battery_level: 1.0,
            battery_drain_rate: 5.0,
            temperature: 35.0,
            memory_used_mb: 7000.0,
            memory_available_mb: 500.0,  // Only 500MB available
            network_bytes_in: 0,
            network_bytes_out: 0,
            cpu_usage: 0.3,
            active_processes: 10,
            timestamp: 1000,
        };

        let anomaly = detector.update_metrics(metrics);
        assert!(anomaly.is_some());
        assert_eq!(anomaly.unwrap().anomaly_type, AnomalyType::MemoryLeak);
    }

    #[test]
    fn test_detect_network_anomaly() {
        let mut detector = AnomalyDetector::new();

        let metrics = SystemMetrics {
            battery_level: 1.0,
            battery_drain_rate: 5.0,
            temperature: 35.0,
            memory_used_mb: 2000.0,
            memory_available_mb: 1000.0,
            network_bytes_in: 20_000_000,  // 20MB in one check = 20MB/s (anomalous)
            network_bytes_out: 0,
            cpu_usage: 0.3,
            active_processes: 10,
            timestamp: 1000,
        };

        let anomaly = detector.update_metrics(metrics);
        assert!(anomaly.is_some());
        assert_eq!(anomaly.unwrap().anomaly_type, AnomalyType::NetworkAnomaly);
    }

    #[test]
    fn test_cpu_stall_detection() {
        let mut detector = AnomalyDetector::new();

        // Baseline
        for _ in 0..10 {
            let metrics = create_test_metrics(1.0, 5.0, 35.0, 2000.0, 0, 0.2);
            detector.train_baseline(&metrics);
        }

        // High CPU with memory growth
        let metrics = create_test_metrics(1.0, 5.0, 35.0, 2500.0, 0, 0.9);
        let anomaly = detector.update_metrics(metrics);

        assert!(anomaly.is_some());
        assert_eq!(anomaly.unwrap().anomaly_type, AnomalyType::CPUStall);
    }

    #[test]
    fn test_detection_statistics() {
        let mut detector = AnomalyDetector::new();

        // Normal operation multiple times
        for i in 0..5 {
            let metrics = create_test_metrics(1.0, 5.0, 35.0, 2000.0, 0, 0.3);
            detector.train_baseline(&metrics);
        }

        // Trigger anomaly
        let metrics = create_test_metrics(0.8, 25.0, 35.0, 2000.0, 0, 0.3);
        detector.update_metrics(metrics);

        let stats = detector.get_detection_stats();
        assert!(stats.total_anomalies > 0);
    }

    #[test]
    fn test_no_anomaly_normal_operation() {
        let mut detector = AnomalyDetector::new();

        let metrics = create_test_metrics(1.0, 5.0, 35.0, 2000.0, 1000, 0.3);
        let anomaly = detector.update_metrics(metrics);

        // Normal operation should not trigger anomaly
        assert!(anomaly.is_none());
    }

    #[test]
    fn test_multiple_anomalies_batched() {
        let mut detector = AnomalyDetector::new();

        // Baseline
        for _ in 0..10 {
            let metrics = create_test_metrics(1.0, 5.0, 35.0, 2000.0, 1000, 0.3);
            detector.train_baseline(&metrics);
        }

        // Trigger multiple anomalies
        let mut anomaly_count = 0;

        for i in 0..5 {
            let metrics = create_test_metrics(
                0.8,
                25.0,  // High drain
                45.0 + (i as f64 * 2.0),  // Rising temp
                2500.0,
                0,
                0.3
            );

            if let Some(_) = detector.update_metrics(metrics) {
                anomaly_count += 1;
            }
        }

        assert!(anomaly_count > 0);
    }

    #[test]
    fn test_cleanup_old_data() {
        let mut detector = AnomalyDetector::new();

        let mut metrics = create_test_metrics(1.0, 5.0, 35.0, 2000.0, 0, 0.3);
        metrics.timestamp = 1000;
        detector.update_metrics(metrics);

        metrics.timestamp = 2000;
        detector.update_metrics(metrics);

        detector.cleanup_old_data(1500);
        // After cleanup, only data from timestamp > 1500 should remain
    }

    #[test]
    fn test_severity_levels() {
        let mut detector = AnomalyDetector::new();

        // Low temperature spike
        let mut metrics = create_test_metrics(1.0, 5.0, 40.0, 2000.0, 0, 0.3);
        detector.update_metrics(metrics);

        metrics.temperature = 48.0;  // Warm but not critical
        metrics.timestamp = 2000;
        let anomaly = detector.update_metrics(metrics);

        assert!(anomaly.is_some());
        assert_eq!(
            anomaly.unwrap().severity,
            SeverityLevel::Medium
        );
    }

    #[test]
    fn test_confidence_scoring() {
        let mut detector = AnomalyDetector::new();

        // Baseline
        for _ in 0..10 {
            let metrics = create_test_metrics(1.0, 5.0, 35.0, 2000.0, 0, 0.3);
            detector.train_baseline(&metrics);
        }

        // Severe battery drain
        let metrics = create_test_metrics(0.8, 50.0, 35.0, 2000.0, 0, 0.3);
        if let Some(anomaly) = detector.update_metrics(metrics) {
            assert!(anomaly.confidence > 0.5);  // High confidence
        }
    }
}

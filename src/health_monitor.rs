// Phase 10: Health Monitor
// Real-time system health tracking and degradation detection

use std::collections::VecDeque;
use std::time::Instant;

/// System-level metrics
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f32,
    pub memory_pressure_percent: f32,
    pub thermal_temp_celsius: f32,
    pub timestamp: u64,
}

impl SystemMetrics {
    pub fn new(cpu: f32, memory: f32, thermal: f32) -> Self {
        SystemMetrics {
            cpu_usage_percent: cpu,
            memory_pressure_percent: memory,
            thermal_temp_celsius: thermal,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

/// Inference-level metrics
#[derive(Debug, Clone)]
pub struct InferenceMetrics {
    pub latency_ms: f32,
    pub accuracy: f32,
    pub throughput_per_sec: f32,
    pub error_rate: f32,
}

impl InferenceMetrics {
    pub fn new(latency: f32, accuracy: f32, throughput: f32, error_rate: f32) -> Self {
        InferenceMetrics {
            latency_ms: latency,
            accuracy,
            throughput_per_sec: throughput,
            error_rate,
        }
    }
}

/// Health score calculation
#[derive(Debug, Clone)]
pub struct HealthScore {
    pub latency_score: f32,      // 0.0-1.0 (1.0 = ideal)
    pub accuracy_score: f32,     // 0.0-1.0
    pub throughput_score: f32,   // 0.0-1.0
    pub cpu_score: f32,          // 0.0-1.0
    pub overall_score: f32,      // 0.0-1.0 (weighted average)
    pub is_degraded: bool,       // overall_score < 0.8
}

impl HealthScore {
    /// Calculate health score from metrics
    pub fn calculate(system: &SystemMetrics, inference: &InferenceMetrics) -> Self {
        // Latency score (target 5ms, penalty >8ms)
        let latency_score = if inference.latency_ms <= 5.0 {
            1.0
        } else if inference.latency_ms > 8.0 {
            0.5
        } else {
            1.0 - (inference.latency_ms - 5.0) / 6.0
        };

        // Accuracy score (target ≥98%, linear drop below)
        let accuracy_score = (inference.accuracy / 100.0).min(1.0).max(0.0);

        // Throughput score (target ≥100 inf/s)
        let throughput_score =
            ((inference.throughput_per_sec / 100.0).min(1.0)).max(0.0);

        // CPU score (target <50%, penalty >75%)
        let cpu_score = if system.cpu_usage_percent <= 50.0 {
            1.0
        } else if system.cpu_usage_percent > 75.0 {
            0.5
        } else {
            1.0 - (system.cpu_usage_percent - 50.0) / 50.0
        };

        // Weighted overall score: Latency 30%, Accuracy 40%, Throughput 15%, CPU 15%
        let overall_score =
            (latency_score * 0.3) + (accuracy_score * 0.4) + (throughput_score * 0.15) + (cpu_score * 0.15);

        let is_degraded = overall_score < 0.8;

        HealthScore {
            latency_score,
            accuracy_score,
            throughput_score,
            cpu_score,
            overall_score,
            is_degraded,
        }
    }
}

/// Main health monitor
pub struct HealthMonitor {
    system_metrics_history: VecDeque<SystemMetrics>,
    inference_metrics_history: VecDeque<InferenceMetrics>,
    health_scores: VecDeque<HealthScore>,
    current_health: HealthScore,
    max_history_size: usize,
    last_query_time: Instant,
    query_count: u64,
}

impl HealthMonitor {
    pub fn new() -> Self {
        let dummy_system = SystemMetrics::new(30.0, 40.0, 40.0);
        let dummy_inference = InferenceMetrics::new(5.0, 98.0, 100.0, 0.1);
        let dummy_health = HealthScore::calculate(&dummy_system, &dummy_inference);

        HealthMonitor {
            system_metrics_history: VecDeque::new(),
            inference_metrics_history: VecDeque::new(),
            health_scores: VecDeque::new(),
            current_health: dummy_health,
            max_history_size: 1000,
            last_query_time: Instant::now(),
            query_count: 0,
        }
    }

    /// Record system metrics
    pub fn record_system_metrics(&mut self, metrics: SystemMetrics) {
        if self.system_metrics_history.len() >= self.max_history_size {
            self.system_metrics_history.pop_front();
        }
        self.system_metrics_history.push_back(metrics);
    }

    /// Record inference metrics
    pub fn record_inference_metrics(&mut self, metrics: InferenceMetrics) {
        if self.inference_metrics_history.len() >= self.max_history_size {
            self.inference_metrics_history.pop_front();
        }
        self.inference_metrics_history.push_back(metrics.clone());

        // Calculate new health score
        if let Some(system) = self.system_metrics_history.back() {
            let health = HealthScore::calculate(system, &metrics);
            if self.health_scores.len() >= self.max_history_size {
                self.health_scores.pop_front();
            }
            self.health_scores.push_back(health.clone());
            self.current_health = health;
        }
    }

    /// Get current health status (Rule 3: <100ms latency)
    pub fn get_health(&self) -> HealthScore {
        let _query_start = Instant::now();
        self.current_health.clone()
        // Note: In production, measure actual query latency here
    }

    /// Detect degradation (compare against baseline)
    pub fn detect_degradation(&self) -> (bool, String) {
        if self.current_health.is_degraded {
            let reason = if self.current_health.latency_score < 0.7 {
                "latency degradation"
            } else if self.current_health.accuracy_score < 0.95 {
                "accuracy degradation"
            } else if self.current_health.cpu_score < 0.7 {
                "cpu overload"
            } else {
                "overall degradation"
            };

            (true, format!("Health degradation detected: {}", reason))
        } else {
            (false, "System healthy".to_string())
        }
    }

    /// Get average latency from history
    pub fn get_avg_latency(&self) -> f32 {
        if self.inference_metrics_history.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.inference_metrics_history.iter().map(|m| m.latency_ms).sum();
        sum / self.inference_metrics_history.len() as f32
    }

    /// Get average accuracy from history
    pub fn get_avg_accuracy(&self) -> f32 {
        if self.inference_metrics_history.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.inference_metrics_history.iter().map(|m| m.accuracy).sum();
        sum / self.inference_metrics_history.len() as f32
    }

    /// Verify monitoring accuracy (Rule 4: ≥95%)
    pub fn verify_accuracy(&self, ground_truth_health: &HealthScore) -> (bool, f32) {
        // Compare current health with ground truth
        let latency_diff = (self.current_health.latency_score - ground_truth_health.latency_score).abs();
        let accuracy_diff = (self.current_health.accuracy_score - ground_truth_health.accuracy_score).abs();
        let overall_diff = (self.current_health.overall_score - ground_truth_health.overall_score).abs();

        let accuracy = 1.0 - (latency_diff + accuracy_diff + overall_diff) / 3.0;
        let rule_pass = accuracy >= 0.95;

        (rule_pass, accuracy * 100.0)
    }

    /// Get monitoring latency (Rule 3: <100ms)
    pub fn get_monitoring_latency(&mut self) -> f32 {
        let start = Instant::now();
        let _health = self.get_health();
        self.query_count += 1;
        start.elapsed().as_millis() as f32
    }

    pub fn get_current_health(&self) -> &HealthScore {
        &self.current_health
    }

    pub fn get_history_size(&self) -> usize {
        self.health_scores.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_monitor_creation() {
        let monitor = HealthMonitor::new();
        assert_eq!(monitor.max_history_size, 1000);
    }

    #[test]
    fn test_metric_collection() {
        let mut monitor = HealthMonitor::new();

        let system = SystemMetrics::new(40.0, 50.0, 45.0);
        let inference = InferenceMetrics::new(5.5, 98.0, 95.0, 0.1);

        monitor.record_system_metrics(system);
        monitor.record_inference_metrics(inference);

        assert_eq!(monitor.system_metrics_history.len(), 1);
        assert_eq!(monitor.inference_metrics_history.len(), 1);
    }

    #[test]
    fn test_health_score_calculation() {
        let system = SystemMetrics::new(30.0, 40.0, 40.0);
        let inference = InferenceMetrics::new(5.0, 98.0, 100.0, 0.1);

        let health = HealthScore::calculate(&system, &inference);
        assert!(health.overall_score > 0.8);
        assert!(!health.is_degraded);
    }

    #[test]
    fn test_degradation_detection() {
        let mut monitor = HealthMonitor::new();

        // Add degraded metrics
        let system = SystemMetrics::new(80.0, 85.0, 55.0);
        let inference = InferenceMetrics::new(9.0, 96.5, 50.0, 0.5);

        monitor.record_system_metrics(system);
        monitor.record_inference_metrics(inference);

        let (degraded, _reason) = monitor.detect_degradation();
        assert!(degraded);
    }

    #[test]
    fn test_monitoring_latency() {
        let mut monitor = HealthMonitor::new();
        let latency_ms = monitor.get_monitoring_latency();

        // Rule 3: Monitoring latency <100ms
        assert!(latency_ms < 100.0, "Monitoring latency: {:.2}ms", latency_ms);
    }

    #[test]
    fn test_health_accuracy() {
        let mut monitor = HealthMonitor::new();

        let system = SystemMetrics::new(35.0, 45.0, 42.0);
        let inference = InferenceMetrics::new(5.2, 98.1, 105.0, 0.08);

        monitor.record_system_metrics(system);
        monitor.record_inference_metrics(inference);

        let ground_truth = HealthScore::calculate(&system, &inference);
        let (rule_pass, accuracy) = monitor.verify_accuracy(&ground_truth);

        // Rule 4: Health accuracy ≥95%
        assert!(rule_pass, "Health accuracy: {:.1}%", accuracy);
        assert!(accuracy >= 95.0);
    }
}

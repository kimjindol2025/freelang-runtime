// Phase 9: Performance Profiler
// Measure and report hardware performance metrics

use std::time::Instant;
use std::collections::VecDeque;

/// Latency tracker
#[derive(Debug, Clone)]
pub struct LatencyTracker {
    latencies_ms: VecDeque<f32>,
    max_samples: usize,
}

impl LatencyTracker {
    pub fn new(max_samples: usize) -> Self {
        LatencyTracker {
            latencies_ms: VecDeque::new(),
            max_samples,
        }
    }

    pub fn record(&mut self, latency_ms: f32) {
        if self.latencies_ms.len() >= self.max_samples {
            self.latencies_ms.pop_front();
        }
        self.latencies_ms.push_back(latency_ms);
    }

    pub fn get_average(&self) -> f32 {
        if self.latencies_ms.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.latencies_ms.iter().sum();
        sum / self.latencies_ms.len() as f32
    }

    pub fn get_p95(&self) -> f32 {
        if self.latencies_ms.is_empty() {
            return 0.0;
        }
        let mut sorted: Vec<f32> = self.latencies_ms.iter().cloned().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = ((sorted.len() as f32 * 0.95) as usize).min(sorted.len() - 1);
        sorted[idx]
    }

    pub fn get_p99(&self) -> f32 {
        if self.latencies_ms.is_empty() {
            return 0.0;
        }
        let mut sorted: Vec<f32> = self.latencies_ms.iter().cloned().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = ((sorted.len() as f32 * 0.99) as usize).min(sorted.len() - 1);
        sorted[idx]
    }

    pub fn get_max(&self) -> f32 {
        self.latencies_ms
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max)
    }
}

/// Power monitor
#[derive(Debug, Clone)]
pub struct PowerMonitor {
    power_readings_mw: VecDeque<f32>,
    max_samples: usize,
}

impl PowerMonitor {
    pub fn new(max_samples: usize) -> Self {
        PowerMonitor {
            power_readings_mw: VecDeque::new(),
            max_samples,
        }
    }

    pub fn record(&mut self, power_mw: f32) {
        if self.power_readings_mw.len() >= self.max_samples {
            self.power_readings_mw.pop_front();
        }
        self.power_readings_mw.push_back(power_mw);
    }

    pub fn get_average_power(&self) -> f32 {
        if self.power_readings_mw.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.power_readings_mw.iter().sum();
        sum / self.power_readings_mw.len() as f32
    }

    pub fn get_max_power(&self) -> f32 {
        self.power_readings_mw
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max)
    }

    pub fn get_min_power(&self) -> f32 {
        self.power_readings_mw
            .iter()
            .cloned()
            .fold(f32::INFINITY, f32::min)
    }

    pub fn get_power_std_dev(&self) -> f32 {
        if self.power_readings_mw.is_empty() {
            return 0.0;
        }
        let mean = self.get_average_power();
        let variance: f32 = self
            .power_readings_mw
            .iter()
            .map(|p| (p - mean).powi(2))
            .sum::<f32>()
            / self.power_readings_mw.len() as f32;
        variance.sqrt()
    }
}

/// Cache monitor
#[derive(Debug, Clone)]
pub struct CacheMonitor {
    cache_hits: u64,
    cache_misses: u64,
}

impl CacheMonitor {
    pub fn new() -> Self {
        CacheMonitor {
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    pub fn record_hit(&mut self) {
        self.cache_hits += 1;
    }

    pub fn record_miss(&mut self) {
        self.cache_misses += 1;
    }

    pub fn get_hit_rate(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            return 0.0;
        }
        (self.cache_hits as f32 / total as f32) * 100.0
    }

    pub fn get_miss_rate(&self) -> f32 {
        100.0 - self.get_hit_rate()
    }

    pub fn reset(&mut self) {
        self.cache_hits = 0;
        self.cache_misses = 0;
    }
}

/// Accuracy tracker
#[derive(Debug, Clone)]
pub struct AccuracyTracker {
    correct_predictions: u32,
    total_predictions: u32,
}

impl AccuracyTracker {
    pub fn new() -> Self {
        AccuracyTracker {
            correct_predictions: 0,
            total_predictions: 0,
        }
    }

    pub fn record_prediction(&mut self, is_correct: bool) {
        self.total_predictions += 1;
        if is_correct {
            self.correct_predictions += 1;
        }
    }

    pub fn get_accuracy(&self) -> f32 {
        if self.total_predictions == 0 {
            return 0.0;
        }
        (self.correct_predictions as f32 / self.total_predictions as f32) * 100.0
    }

    pub fn reset(&mut self) {
        self.correct_predictions = 0;
        self.total_predictions = 0;
    }
}

/// Main performance profiler
pub struct PerformanceProfiler {
    latency_tracker: LatencyTracker,
    power_monitor: PowerMonitor,
    cache_monitor: CacheMonitor,
    accuracy_tracker: AccuracyTracker,
    start_time: Option<Instant>,
}

impl PerformanceProfiler {
    pub fn new() -> Self {
        PerformanceProfiler {
            latency_tracker: LatencyTracker::new(1000),
            power_monitor: PowerMonitor::new(1000),
            cache_monitor: CacheMonitor::new(),
            accuracy_tracker: AccuracyTracker::new(),
            start_time: None,
        }
    }

    /// Start timing a measurement
    pub fn start_timer(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// End timing and record latency
    pub fn end_timer(&mut self) {
        if let Some(start) = self.start_time {
            let elapsed_ms = start.elapsed().as_secs_f32() * 1000.0;
            self.latency_tracker.record(elapsed_ms);
            self.start_time = None;
        }
    }

    /// Record power measurement
    pub fn record_power(&mut self, power_mw: f32) {
        self.power_monitor.record(power_mw);
    }

    /// Record cache hit
    pub fn record_cache_hit(&mut self) {
        self.cache_monitor.record_hit();
    }

    /// Record cache miss
    pub fn record_cache_miss(&mut self) {
        self.cache_monitor.record_miss();
    }

    /// Record inference accuracy
    pub fn record_accuracy(&mut self, is_correct: bool) {
        self.accuracy_tracker.record_prediction(is_correct);
    }

    /// Get comprehensive profiling report
    pub fn get_report(&self) -> ProfilingReport {
        ProfilingReport {
            avg_latency_ms: self.latency_tracker.get_average(),
            p95_latency_ms: self.latency_tracker.get_p95(),
            p99_latency_ms: self.latency_tracker.get_p99(),
            max_latency_ms: self.latency_tracker.get_max(),
            avg_power_mw: self.power_monitor.get_average_power(),
            max_power_mw: self.power_monitor.get_max_power(),
            min_power_mw: self.power_monitor.get_min_power(),
            power_std_dev: self.power_monitor.get_power_std_dev(),
            cache_hit_rate: self.cache_monitor.get_hit_rate(),
            cache_miss_rate: self.cache_monitor.get_miss_rate(),
            accuracy_percent: self.accuracy_tracker.get_accuracy(),
        }
    }

    /// Verify performance targets
    pub fn verify_performance(&self) -> (bool, String) {
        let report = self.get_report();

        let latency_pass = report.avg_latency_ms < 6.0; // Target: 5-6ms
        let power_pass = report.avg_power_mw < 150.0;   // Conservative limit
        let accuracy_pass = report.accuracy_percent >= 98.0; // Maintain ≥98%

        let all_pass = latency_pass && power_pass && accuracy_pass;
        let msg = format!(
            "Latency:{:.2}ms Power:{:.1}mW Accuracy:{:.1}% [Pass:{}]",
            report.avg_latency_ms, report.avg_power_mw, report.accuracy_percent, all_pass
        );

        (all_pass, msg)
    }

    /// Reset all trackers
    pub fn reset(&mut self) {
        self.latency_tracker = LatencyTracker::new(1000);
        self.power_monitor = PowerMonitor::new(1000);
        self.cache_monitor.reset();
        self.accuracy_tracker.reset();
    }
}

#[derive(Debug, Clone)]
pub struct ProfilingReport {
    pub avg_latency_ms: f32,
    pub p95_latency_ms: f32,
    pub p99_latency_ms: f32,
    pub max_latency_ms: f32,
    pub avg_power_mw: f32,
    pub max_power_mw: f32,
    pub min_power_mw: f32,
    pub power_std_dev: f32,
    pub cache_hit_rate: f32,
    pub cache_miss_rate: f32,
    pub accuracy_percent: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_creation() {
        let profiler = PerformanceProfiler::new();
        let report = profiler.get_report();
        assert_eq!(report.avg_latency_ms, 0.0);
    }

    #[test]
    fn test_latency_measurement() {
        let mut profiler = PerformanceProfiler::new();

        for _ in 0..10 {
            profiler.start_timer();
            // Simulate some work
            let _sum: f32 = (0..1000).map(|i| i as f32).sum();
            profiler.end_timer();
        }

        let report = profiler.get_report();
        assert!(report.avg_latency_ms > 0.0);
        assert!(report.p95_latency_ms >= report.avg_latency_ms);
    }

    #[test]
    fn test_power_measurement() {
        let mut profiler = PerformanceProfiler::new();

        profiler.record_power(50.0);
        profiler.record_power(60.0);
        profiler.record_power(70.0);

        let report = profiler.get_report();
        assert!((report.avg_power_mw - 60.0).abs() < 0.1);
        assert!(report.max_power_mw >= 70.0);
    }

    #[test]
    fn test_accuracy_tracking() {
        let mut profiler = PerformanceProfiler::new();

        profiler.record_accuracy(true);
        profiler.record_accuracy(true);
        profiler.record_accuracy(false);
        profiler.record_accuracy(true);

        let report = profiler.get_report();
        assert!((report.accuracy_percent - 75.0).abs() < 0.1);
    }

    #[test]
    fn test_cache_monitoring() {
        let mut profiler = PerformanceProfiler::new();

        for _ in 0..70 {
            profiler.record_cache_hit();
        }
        for _ in 0..30 {
            profiler.record_cache_miss();
        }

        let report = profiler.get_report();
        assert!((report.cache_hit_rate - 70.0).abs() < 0.1);
    }

    #[test]
    fn test_comprehensive_report() {
        let mut profiler = PerformanceProfiler::new();

        for i in 0..100 {
            profiler.record_power(50.0 + (i % 20) as f32);
            profiler.record_accuracy(i % 3 != 0);
        }

        let report = profiler.get_report();
        assert!(report.avg_power_mw > 0.0);
        assert!(report.accuracy_percent > 0.0);
    }

    #[test]
    fn test_performance_verification() {
        let mut profiler = PerformanceProfiler::new();

        // Simulate good performance
        for _ in 0..50 {
            profiler.start_timer();
            let _sum: f32 = (0..100).map(|i| i as f32).sum();
            profiler.end_timer();
            profiler.record_power(80.0);
            profiler.record_accuracy(true);
        }

        let (pass, msg) = profiler.verify_performance();
        assert!(!msg.is_empty());
    }
}

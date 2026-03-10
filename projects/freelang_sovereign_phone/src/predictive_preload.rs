// Project Sovereign: PredictivePreload Module
// Goal: Reduce app startup time from 2s to 300ms (85% improvement)
// Strategy: Preload next predicted app in memory + prefetch WiFi connections

use std::collections::VecDeque;

pub type AppId = u32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PreloadState {
    Inactive,
    Scheduled,
    Loading,
    Preloaded,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PreloadPriority {
    Critical,  // >80% confidence
    High,      // 60-80% confidence
    Medium,    // 40-60% confidence
    Low,       // <40% confidence
}

#[derive(Clone, Debug)]
pub struct AppPreloadProfile {
    app_id: AppId,
    confidence: f64,
    estimated_size_mb: f64,
    startup_latency_ms: u32,
    memory_footprint_mb: f64,
    is_preloaded: bool,
    preload_start_time_ms: u64,
    current_state: PreloadState,
    priority: PreloadPriority,
}

#[derive(Clone, Debug)]
pub struct WiFiPrediction {
    pub will_connect: bool,
    pub confidence: f64,
    pub estimated_delay_ms: u32,
}

#[derive(Clone, Debug)]
pub struct NetworkOptimization {
    pub dns_prefetch: Vec<String>,
    pub http_warmup: bool,
    pub tls_session_resume: bool,
}

pub struct PredictivePreload {
    // Preload queue: up to 3 apps queued for preloading
    preload_queue: VecDeque<AppPreloadProfile>,

    // Memory management
    available_memory_mb: f64,
    preloaded_apps_memory_mb: f64,

    // Statistics
    total_preloads: usize,
    successful_hits: usize,
    false_positives: usize,
    current_timestamp_ms: u64,

    // WiFi state
    last_wifi_state: bool,
    wifi_stability_score: f64,

    // Timing windows
    preload_window_ms: u32,  // How far ahead to preload (2000ms = 2s)
}

impl PredictivePreload {
    pub fn new(available_memory_mb: f64) -> Self {
        Self {
            preload_queue: VecDeque::with_capacity(3),
            available_memory_mb,
            preloaded_apps_memory_mb: 0.0,
            total_preloads: 0,
            successful_hits: 0,
            false_positives: 0,
            current_timestamp_ms: 0,
            last_wifi_state: false,
            wifi_stability_score: 0.5,
            preload_window_ms: 2000,  // 2 second preload window
        }
    }

    /// Update current timestamp (simulating real-time)
    pub fn update_timestamp(&mut self, timestamp_ms: u64) {
        self.current_timestamp_ms = timestamp_ms;
    }

    /// Predict next app and WiFi state
    pub fn predict_next_app_with_wifi(
        &self,
        next_app_id: AppId,
        app_confidence: f64,
        app_startup_latency_ms: u32,
        app_size_mb: f64,
        will_use_network: bool,
    ) -> (AppPreloadProfile, WiFiPrediction, NetworkOptimization) {
        let priority = self.confidence_to_priority(app_confidence);

        let app_profile = AppPreloadProfile {
            app_id: next_app_id,
            confidence: app_confidence,
            estimated_size_mb: app_size_mb,
            startup_latency_ms: app_startup_latency_ms,
            memory_footprint_mb: app_size_mb * 1.2,  // 20% overhead
            is_preloaded: false,
            preload_start_time_ms: self.current_timestamp_ms,
            current_state: PreloadState::Scheduled,
            priority,
        };

        let wifi_prediction = self.predict_wifi_connection(
            will_use_network,
            self.wifi_stability_score,
        );

        let network_opt = self.create_network_optimization(will_use_network);

        (app_profile, wifi_prediction, network_opt)
    }

    fn confidence_to_priority(&self, confidence: f64) -> PreloadPriority {
        match confidence {
            c if c >= 0.8 => PreloadPriority::Critical,
            c if c >= 0.6 => PreloadPriority::High,
            c if c >= 0.4 => PreloadPriority::Medium,
            _ => PreloadPriority::Low,
        }
    }

    fn predict_wifi_connection(
        &self,
        will_use_network: bool,
        wifi_stability: f64,
    ) -> WiFiPrediction {
        if !will_use_network {
            return WiFiPrediction {
                will_connect: false,
                confidence: 1.0,
                estimated_delay_ms: 0,
            };
        }

        // Predict WiFi based on historical stability
        let will_connect = wifi_stability > 0.5;
        let confidence = (wifi_stability * 0.8 + 0.2).min(0.95);
        let estimated_delay_ms = if will_connect {
            ((500.0 * (1.0 - wifi_stability)) as u32).max(50).min(500)
        } else {
            0
        };

        WiFiPrediction {
            will_connect,
            confidence,
            estimated_delay_ms,
        }
    }

    fn create_network_optimization(&self, will_use_network: bool) -> NetworkOptimization {
        if !will_use_network {
            return NetworkOptimization {
                dns_prefetch: vec![],
                http_warmup: false,
                tls_session_resume: false,
            };
        }

        NetworkOptimization {
            dns_prefetch: vec![
                "api.example.com".to_string(),
                "cdn.example.com".to_string(),
            ],
            http_warmup: true,
            tls_session_resume: self.wifi_stability_score > 0.7,
        }
    }

    /// Schedule app for preloading
    pub fn schedule_preload(&mut self, app_profile: AppPreloadProfile) -> bool {
        let required_memory = app_profile.memory_footprint_mb;

        // Check if we can fit in available memory
        if self.preloaded_apps_memory_mb + required_memory > self.available_memory_mb {
            // Try to evict low-priority apps
            self.evict_low_priority_apps(required_memory);

            // Check again
            if self.preloaded_apps_memory_mb + required_memory > self.available_memory_mb {
                return false;
            }
        }

        // Add to queue (max 3 apps)
        if self.preload_queue.len() >= 3 {
            self.preload_queue.pop_front();
        }

        self.preload_queue.push_back(app_profile);
        self.total_preloads += 1;
        true
    }

    /// Execute preload - simulate loading app into memory
    pub fn execute_preload(&mut self) -> Option<(AppId, f64)> {
        while let Some(mut app) = self.preload_queue.pop_front() {
            let elapsed_ms = self.current_timestamp_ms - app.preload_start_time_ms;

            // Skip if this prediction is too stale (>5s old)
            if elapsed_ms > 5000 {
                self.false_positives += 1;
                continue;
            }

            // Execute preload
            app.current_state = PreloadState::Loading;

            // Simulate load time: based on app size and memory speed
            let load_time_ms = (app.estimated_size_mb * 2.0) as u32;  // ~2ms per MB

            // After load_time_ms, app would be preloaded
            let time_to_preload = self.current_timestamp_ms + load_time_ms as u64;

            // If within preload window, mark as preloaded
            if elapsed_ms + load_time_ms as u64 <= self.preload_window_ms as u64 {
                app.current_state = PreloadState::Preloaded;
                app.is_preloaded = true;
                self.preloaded_apps_memory_mb += app.memory_footprint_mb;

                return Some((app.app_id, app.confidence));
            } else {
                app.current_state = PreloadState::Inactive;
            }
        }

        None
    }

    /// Record successful hit (preloaded app was opened)
    pub fn record_hit(&mut self, app_id: AppId, actual_startup_ms: u32) -> f64 {
        self.successful_hits += 1;

        // Calculate improvement: (original - actual) / original
        let original_latency = 2000u32;  // 2 second baseline
        let improvement = if actual_startup_ms < original_latency {
            (original_latency - actual_startup_ms) as f64 / original_latency as f64
        } else {
            0.0
        };

        // Target: 85% improvement (2000ms → 300ms)
        improvement
    }

    /// Update WiFi stability score based on actual connection
    pub fn update_wifi_state(&mut self, connected: bool, latency_ms: u32) {
        self.last_wifi_state = connected;

        // Use exponential moving average for stability
        let new_score = if connected {
            let latency_factor = (1.0 - (latency_ms as f64 / 500.0)).max(0.1);
            0.7 * self.wifi_stability_score + 0.3 * latency_factor
        } else {
            0.7 * self.wifi_stability_score + 0.3 * 0.0
        };

        self.wifi_stability_score = new_score.min(1.0).max(0.0);
    }

    fn evict_low_priority_apps(&mut self, required_memory: f64) {
        let mut removed_memory = 0.0;

        // Remove Low priority apps first
        self.preload_queue.retain(|app| {
            if removed_memory >= required_memory {
                true
            } else if app.priority == PreloadPriority::Low {
                removed_memory += app.memory_footprint_mb;
                false
            } else {
                true
            }
        });

        // If still not enough, remove Medium priority
        if removed_memory < required_memory {
            self.preload_queue.retain(|app| {
                if removed_memory >= required_memory {
                    true
                } else if app.priority == PreloadPriority::Medium {
                    removed_memory += app.memory_footprint_mb;
                    false
                } else {
                    true
                }
            });
        }
    }

    /// Get preload queue status
    pub fn get_queue_status(&self) -> Vec<(AppId, PreloadPriority, PreloadState)> {
        self.preload_queue
            .iter()
            .map(|app| (app.app_id, app.priority, app.current_state))
            .collect()
    }

    /// Get preload success metrics
    pub fn get_preload_metrics(&self) -> PreloadMetrics {
        let hit_rate = if self.total_preloads > 0 {
            self.successful_hits as f64 / self.total_preloads as f64
        } else {
            0.0
        };

        let false_positive_rate = if self.total_preloads > 0 {
            self.false_positives as f64 / self.total_preloads as f64
        } else {
            0.0
        };

        PreloadMetrics {
            total_preloads: self.total_preloads,
            successful_hits: self.successful_hits,
            false_positives: self.false_positives,
            hit_rate,
            false_positive_rate,
            current_memory_usage_mb: self.preloaded_apps_memory_mb,
            available_memory_mb: self.available_memory_mb,
            wifi_stability: self.wifi_stability_score,
        }
    }

    /// Clear memory of preloaded app
    pub fn clear_preload(&mut self, app_id: AppId) -> f64 {
        let removed_memory: f64 = self.preload_queue
            .iter()
            .filter(|app| app.app_id == app_id)
            .map(|app| app.memory_footprint_mb)
            .sum();

        self.preload_queue.retain(|app| app.app_id != app_id);
        self.preloaded_apps_memory_mb = (self.preloaded_apps_memory_mb - removed_memory).max(0.0);

        removed_memory
    }
}

#[derive(Clone, Debug)]
pub struct PreloadMetrics {
    pub total_preloads: usize,
    pub successful_hits: usize,
    pub false_positives: usize,
    pub hit_rate: f64,
    pub false_positive_rate: f64,
    pub current_memory_usage_mb: f64,
    pub available_memory_mb: f64,
    pub wifi_stability: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preload_creation() {
        let preload = PredictivePreload::new(512.0);
        assert_eq!(preload.available_memory_mb, 512.0);
        assert_eq!(preload.total_preloads, 0);
    }

    #[test]
    fn test_confidence_to_priority() {
        let preload = PredictivePreload::new(512.0);
        assert_eq!(preload.confidence_to_priority(0.85), PreloadPriority::Critical);
        assert_eq!(preload.confidence_to_priority(0.7), PreloadPriority::High);
        assert_eq!(preload.confidence_to_priority(0.5), PreloadPriority::Medium);
        assert_eq!(preload.confidence_to_priority(0.3), PreloadPriority::Low);
    }

    #[test]
    fn test_predict_next_app() {
        let preload = PredictivePreload::new(512.0);
        let (app, wifi, net) = preload.predict_next_app_with_wifi(
            1,
            0.85,
            150,
            50.0,
            true,
        );

        assert_eq!(app.app_id, 1);
        assert_eq!(app.confidence, 0.85);
        assert_eq!(app.priority, PreloadPriority::Critical);
        assert!(wifi.will_connect);
    }

    #[test]
    fn test_schedule_and_execute_preload() {
        let mut preload = PredictivePreload::new(512.0);
        preload.update_timestamp(1000);

        let (app, _, _) = preload.predict_next_app_with_wifi(1, 0.9, 150, 50.0, false);
        assert!(preload.schedule_preload(app));

        // Should have 1 app in queue
        let status = preload.get_queue_status();
        assert_eq!(status.len(), 1);

        let result = preload.execute_preload();
        assert!(result.is_some());
    }

    #[test]
    fn test_memory_management() {
        let mut preload = PredictivePreload::new(100.0);
        preload.update_timestamp(1000);

        // Schedule 3 apps @ 30MB each = 90MB (fits)
        for i in 1..=3 {
            let (app, _, _) = preload.predict_next_app_with_wifi(
                i,
                0.8,
                100,
                25.0,
                false,
            );
            assert!(preload.schedule_preload(app));
        }

        assert_eq!(preload.get_queue_status().len(), 3);
    }

    #[test]
    fn test_memory_eviction() {
        let mut preload = PredictivePreload::new(100.0);
        preload.update_timestamp(1000);

        // Fill with low priority
        let (low_app, _, _) = preload.predict_next_app_with_wifi(1, 0.2, 100, 30.0, false);
        preload.schedule_preload(low_app);

        // Add high priority - should evict low
        let (high_app, _, _) = preload.predict_next_app_with_wifi(2, 0.9, 100, 70.0, false);
        assert!(preload.schedule_preload(high_app));

        let status = preload.get_queue_status();
        assert!(status.iter().any(|(id, _, _)| *id == 2));
    }

    #[test]
    fn test_record_hit() {
        let mut preload = PredictivePreload::new(512.0);

        // Preload from 2000ms to 300ms = 85% improvement
        let improvement = preload.record_hit(1, 300);
        assert!(improvement > 0.80);  // >80% improvement
        assert!(improvement <= 0.85);  // Within 85% goal
    }

    #[test]
    fn test_wifi_stability_good_connection() {
        let mut preload = PredictivePreload::new(512.0);

        // Good WiFi connection
        preload.update_wifi_state(true, 100);  // 100ms latency
        assert!(preload.wifi_stability_score > 0.5);

        preload.update_wifi_state(true, 120);
        assert!(preload.wifi_stability_score > 0.6);
    }

    #[test]
    fn test_wifi_stability_bad_connection() {
        let mut preload = PredictivePreload::new(512.0);

        // Bad WiFi
        preload.update_wifi_state(false, 0);
        assert!(preload.wifi_stability_score < 0.5);
    }

    #[test]
    fn test_preload_metrics() {
        let mut preload = PredictivePreload::new(512.0);
        preload.update_timestamp(1000);

        let (app, _, _) = preload.predict_next_app_with_wifi(1, 0.9, 100, 50.0, false);
        preload.schedule_preload(app);
        preload.record_hit(1, 300);

        let metrics = preload.get_preload_metrics();
        assert_eq!(metrics.total_preloads, 1);
        assert_eq!(metrics.successful_hits, 1);
        assert!(metrics.hit_rate > 0.0);
    }

    #[test]
    fn test_clear_preload() {
        let mut preload = PredictivePreload::new(512.0);
        preload.update_timestamp(1000);

        let (app, _, _) = preload.predict_next_app_with_wifi(1, 0.8, 100, 50.0, false);
        preload.schedule_preload(app);

        let cleared = preload.clear_preload(1);
        assert!(cleared > 0.0);
        assert_eq!(preload.get_queue_status().len(), 0);
    }

    #[test]
    fn test_maximum_queue_size() {
        let mut preload = PredictivePreload::new(1024.0);
        preload.update_timestamp(1000);

        // Try to add 5 apps
        for i in 1..=5 {
            let (app, _, _) = preload.predict_next_app_with_wifi(i, 0.8, 100, 50.0, false);
            preload.schedule_preload(app);
        }

        // Should only have 3 (max queue size)
        assert!(preload.get_queue_status().len() <= 3);
    }

    #[test]
    fn test_multi_app_priority_scheduling() {
        let mut preload = PredictivePreload::new(512.0);
        preload.update_timestamp(1000);

        // Add critical priority app
        let (critical_app, _, _) = preload.predict_next_app_with_wifi(1, 0.95, 100, 50.0, false);
        preload.schedule_preload(critical_app);

        // Add medium priority
        let (medium_app, _, _) = preload.predict_next_app_with_wifi(2, 0.5, 100, 50.0, false);
        preload.schedule_preload(medium_app);

        let status = preload.get_queue_status();
        assert_eq!(status.len(), 2);
        assert!(status.iter().any(|(_, p, _)| *p == PreloadPriority::Critical));
    }
}

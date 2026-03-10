// Phase 10: Alerting System
// Real-time alerts for anomalies and failures

use std::time::Instant;

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Alert message
#[derive(Debug, Clone)]
pub struct Alert {
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: u64,
    pub latency_ms: f32,
    pub accuracy: f32,
    pub error_rate: f32,
}

impl Alert {
    pub fn new(severity: AlertSeverity, message: &str) -> Self {
        Alert {
            severity,
            message: message.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            latency_ms: 0.0,
            accuracy: 0.0,
            error_rate: 0.0,
        }
    }

    pub fn with_metrics(mut self, latency: f32, accuracy: f32, error_rate: f32) -> Self {
        self.latency_ms = latency;
        self.accuracy = accuracy;
        self.error_rate = error_rate;
        self
    }
}

/// Main alerting system
pub struct AlertingSystem {
    alert_queue: Vec<Alert>,
    delivered_count: u64,
    last_delivery_time: Option<Instant>,
}

impl AlertingSystem {
    pub fn new() -> Self {
        AlertingSystem {
            alert_queue: Vec::new(),
            delivered_count: 0,
            last_delivery_time: None,
        }
    }

    /// Generate alert for health degradation
    pub fn generate_alert(
        &mut self,
        latency_ms: f32,
        accuracy: f32,
        error_rate: f32,
    ) {
        let (severity, message) = self.classify_alert(latency_ms, accuracy, error_rate);

        let alert = Alert::new(severity, &message).with_metrics(latency_ms, accuracy, error_rate);

        self.alert_queue.push(alert);
    }

    /// Classify alert severity
    fn classify_alert(&self, latency_ms: f32, accuracy: f32, error_rate: f32) -> (AlertSeverity, String) {
        if latency_ms > 8.0 || accuracy < 97.0 || error_rate > 0.1 {
            (
                AlertSeverity::Critical,
                format!(
                    "CRITICAL: Latency {:.1}ms Accuracy {:.1}% Error {:.2}%",
                    latency_ms, accuracy, error_rate
                ),
            )
        } else if latency_ms > 6.5 || accuracy < 98.0 {
            (
                AlertSeverity::Warning,
                format!(
                    "WARNING: Degradation detected - Latency {:.1}ms Accuracy {:.1}%",
                    latency_ms, accuracy
                ),
            )
        } else {
            (
                AlertSeverity::Info,
                "INFO: Normal operation".to_string(),
            )
        }
    }

    /// Deliver alerts (push to external systems)
    pub fn deliver_alerts(&mut self) -> u32 {
        let start = Instant::now();
        let count = self.alert_queue.len() as u32;

        // Simulate delivery
        for alert in &self.alert_queue {
            // In production: push to logging, monitoring, app notifications
            let _ = format!("[{:?}] {}", alert.severity, alert.message);
        }

        self.delivered_count += count as u64;
        self.last_delivery_time = Some(start);
        self.alert_queue.clear();

        count
    }

    /// Measure delivery latency
    pub fn get_delivery_latency(&self) -> f32 {
        if let Some(last_time) = self.last_delivery_time {
            last_time.elapsed().as_millis() as f32
        } else {
            0.0
        }
    }

    pub fn get_delivered_count(&self) -> u64 {
        self.delivered_count
    }

    pub fn get_pending_alerts(&self) -> usize {
        self.alert_queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_generation() {
        let mut system = AlertingSystem::new();
        system.generate_alert(9.0, 96.5, 0.2);

        assert_eq!(system.get_pending_alerts(), 1);
    }

    #[test]
    fn test_alert_delivery() {
        let mut system = AlertingSystem::new();
        system.generate_alert(7.5, 97.5, 0.05);

        let delivered = system.deliver_alerts();
        assert_eq!(delivered, 1);
        assert_eq!(system.get_delivered_count(), 1);
        assert_eq!(system.get_pending_alerts(), 0);
    }
}

// Phase 10: Rollback Manager
// Automatic rollback on detected failures

use std::time::Instant;

/// Rollback version
#[derive(Debug, Clone)]
pub struct RollbackVersion {
    pub version_id: String,
    pub model_hash: String,
    pub is_stable: bool,
}

impl RollbackVersion {
    pub fn new(version_id: &str, model_hash: &str) -> Self {
        RollbackVersion {
            version_id: version_id.to_string(),
            model_hash: model_hash.to_string(),
            is_stable: true,
        }
    }
}

/// Rollback reason
#[derive(Debug, Clone, Copy)]
pub enum RollbackReason {
    LatencyDegradation,
    AccuracyDegradation,
    ErrorRateElevated,
    ManualTrigger,
}

/// Main rollback manager
pub struct RollbackManager {
    current_version: RollbackVersion,
    previous_version: Option<RollbackVersion>,
    rollback_count: u32,
    last_rollback_time: Option<Instant>,
    alert_threshold: u32, // Trigger rollback after N critical alerts
    current_alert_count: u32,
}

impl RollbackManager {
    pub fn new(current_version: RollbackVersion) -> Self {
        RollbackManager {
            current_version,
            previous_version: None,
            rollback_count: 0,
            last_rollback_time: None,
            alert_threshold: 3, // 3 critical alerts in 30 seconds
            current_alert_count: 0,
        }
    }

    /// Record critical alert
    pub fn record_critical_alert(&mut self) {
        self.current_alert_count += 1;

        if self.current_alert_count >= self.alert_threshold {
            // Trigger automatic rollback
            let _ = self.execute_rollback(RollbackReason::ErrorRateElevated);
            self.current_alert_count = 0;
        }
    }

    /// Execute rollback to previous version
    pub fn execute_rollback(&mut self, reason: RollbackReason) -> bool {
        if let Some(prev_ver) = self.previous_version.clone() {
            // Pre-rollback checks
            if !prev_ver.is_stable {
                return false;
            }

            // Perform rollback
            self.current_version = prev_ver;
            self.rollback_count += 1;
            self.last_rollback_time = Some(Instant::now());

            true
        } else {
            false
        }
    }

    /// Verify rollback success
    pub fn verify_rollback(&self, latency_ms: f32, accuracy: f32) -> bool {
        // After rollback, verify health metrics recover
        latency_ms < 6.0 && accuracy >= 98.0
    }

    /// Set previous version for rollback capability
    pub fn set_previous_version(&mut self, prev_version: RollbackVersion) {
        self.previous_version = Some(prev_version);
    }

    /// Get rollback execution time
    pub fn get_rollback_time_ms(&self) -> f32 {
        if let Some(last_time) = self.last_rollback_time {
            last_time.elapsed().as_millis() as f32
        } else {
            0.0
        }
    }

    pub fn get_rollback_count(&self) -> u32 {
        self.rollback_count
    }

    pub fn get_alert_count(&self) -> u32 {
        self.current_alert_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rollback_execution() {
        let current = RollbackVersion::new("v2.0", "hash_v2");
        let mut manager = RollbackManager::new(current);

        let prev = RollbackVersion::new("v1.9", "hash_v1_9");
        manager.set_previous_version(prev);

        assert!(manager.execute_rollback(RollbackReason::LatencyDegradation));
        assert_eq!(manager.rollback_count, 1);
    }

    #[test]
    fn test_rollback_verification() {
        let current = RollbackVersion::new("v2.0", "hash_v2");
        let manager = RollbackManager::new(current);

        // Should pass if metrics are good
        assert!(manager.verify_rollback(5.5, 98.2));

        // Should fail if latency too high
        assert!(!manager.verify_rollback(9.0, 98.2));
    }
}

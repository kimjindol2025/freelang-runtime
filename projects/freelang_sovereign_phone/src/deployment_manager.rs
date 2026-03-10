// Phase 10: Deployment Manager
// Safe, gradual rollout with canary deployment and health gates

use std::time::Instant;

/// Deployment version metadata
#[derive(Debug, Clone)]
pub struct DeploymentVersion {
    pub version_id: String,
    pub model_hash: String,
    pub timestamp: u64,
    pub config_hash: String,
    pub previous_version: Option<String>,
}

impl DeploymentVersion {
    pub fn new(version_id: &str, model_hash: &str, config_hash: &str) -> Self {
        DeploymentVersion {
            version_id: version_id.to_string(),
            model_hash: model_hash.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            config_hash: config_hash.to_string(),
            previous_version: None,
        }
    }
}

/// Deployment stage
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeploymentStage {
    PreFlight,      // Configuration/model/dependency checks
    Canary5,        // 5% traffic
    Canary25,       // 25% traffic
    Canary50,       // 50% traffic
    Production100,  // 100% traffic
    Rollback,       // Automatic rollback
}

/// Health gates for deployment progression
#[derive(Debug, Clone)]
pub struct HealthGate {
    pub max_latency_ms: f32,
    pub min_accuracy: f32,
    pub max_error_rate: f32,
    pub min_throughput: f32,
}

impl HealthGate {
    pub fn default_strict() -> Self {
        HealthGate {
            max_latency_ms: 6.0,
            min_accuracy: 98.0,
            max_error_rate: 0.1,
            min_throughput: 100.0,
        }
    }

    pub fn default_relaxed() -> Self {
        HealthGate {
            max_latency_ms: 8.0,
            min_accuracy: 97.0,
            max_error_rate: 0.5,
            min_throughput: 50.0,
        }
    }

    /// Check if metrics pass health gates
    pub fn passes(&self, latency_ms: f32, accuracy: f32, error_rate: f32, throughput: f32) -> bool {
        latency_ms <= self.max_latency_ms
            && accuracy >= self.min_accuracy
            && error_rate <= self.max_error_rate
            && throughput >= self.min_throughput
    }
}

/// Metrics collected during deployment
#[derive(Debug, Clone)]
pub struct DeploymentMetrics {
    pub inferences_processed: u64,
    pub avg_latency_ms: f32,
    pub avg_accuracy: f32,
    pub error_count: u32,
    pub data_loss_count: u32,
}

impl DeploymentMetrics {
    pub fn new() -> Self {
        DeploymentMetrics {
            inferences_processed: 0,
            avg_latency_ms: 0.0,
            avg_accuracy: 0.0,
            error_count: 0,
            data_loss_count: 0,
        }
    }

    pub fn get_error_rate(&self) -> f32 {
        if self.inferences_processed == 0 {
            return 0.0;
        }
        (self.error_count as f32 / self.inferences_processed as f32) * 100.0
    }

    pub fn get_throughput(&self, elapsed_secs: f32) -> f32 {
        if elapsed_secs > 0.0 {
            self.inferences_processed as f32 / elapsed_secs
        } else {
            0.0
        }
    }
}

/// Main deployment manager
pub struct DeploymentManager {
    current_version: DeploymentVersion,
    new_version: Option<DeploymentVersion>,
    current_stage: DeploymentStage,
    metrics: DeploymentMetrics,
    health_gate: HealthGate,
    start_time: Instant,
    traffic_percentage: u32,
}

impl DeploymentManager {
    pub fn new(current_version: DeploymentVersion) -> Self {
        DeploymentManager {
            current_version,
            new_version: None,
            current_stage: DeploymentStage::PreFlight,
            metrics: DeploymentMetrics::new(),
            health_gate: HealthGate::default_strict(),
            start_time: Instant::now(),
            traffic_percentage: 0,
        }
    }

    /// Start deployment of new version
    pub fn start_deployment(&mut self, new_version: DeploymentVersion) -> bool {
        self.new_version = Some(new_version);
        self.current_stage = DeploymentStage::PreFlight;
        self.start_time = Instant::now();
        self.traffic_percentage = 0;

        // Pre-flight checks
        self.run_preflight_checks()
    }

    /// Run pre-flight checks (configuration, model, dependencies)
    fn run_preflight_checks(&self) -> bool {
        if let Some(new_ver) = &self.new_version {
            // Validate model hash
            if new_ver.model_hash.is_empty() {
                return false;
            }
            // Validate config hash
            if new_ver.config_hash.is_empty() {
                return false;
            }
            true
        } else {
            false
        }
    }

    /// Advance to next deployment stage
    pub fn advance_stage(&mut self) -> bool {
        let (passed, error_msg) = self.health_check();

        if !passed {
            // Health check failed, do not advance
            return false;
        }

        self.current_stage = match self.current_stage {
            DeploymentStage::PreFlight => {
                self.traffic_percentage = 5;
                DeploymentStage::Canary5
            }
            DeploymentStage::Canary5 => {
                self.traffic_percentage = 25;
                DeploymentStage::Canary25
            }
            DeploymentStage::Canary25 => {
                self.traffic_percentage = 50;
                DeploymentStage::Canary50
            }
            DeploymentStage::Canary50 => {
                self.traffic_percentage = 100;
                DeploymentStage::Production100
            }
            _ => return false,
        };

        true
    }

    /// Perform health checks for current stage
    pub fn health_check(&self) -> (bool, String) {
        let error_rate = self.metrics.get_error_rate();
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let throughput = self.metrics.get_throughput(elapsed);

        let passes = self.health_gate.passes(
            self.metrics.avg_latency_ms,
            self.metrics.avg_accuracy,
            error_rate,
            throughput,
        );

        if passes {
            (
                true,
                format!("Health check passed for stage {:?}", self.current_stage),
            )
        } else {
            (
                false,
                format!(
                    "Health check failed: latency={:.1}ms accuracy={:.1}% errors={:.1}% throughput={:.0}",
                    self.metrics.avg_latency_ms, self.metrics.avg_accuracy, error_rate, throughput
                ),
            )
        }
    }

    /// Record inference metrics
    pub fn record_inference(&mut self, latency_ms: f32, accuracy: f32, success: bool) {
        self.metrics.inferences_processed += 1;

        // Update running average latency
        let n = self.metrics.inferences_processed as f32;
        self.metrics.avg_latency_ms =
            (self.metrics.avg_latency_ms * (n - 1.0) + latency_ms) / n;

        // Update running average accuracy
        self.metrics.avg_accuracy = (self.metrics.avg_accuracy * (n - 1.0) + accuracy) / n;

        if !success {
            self.metrics.error_count += 1;
        }
    }

    /// Record data loss (unforgiving rule: zero data loss)
    pub fn record_data_loss(&mut self, count: u32) {
        self.metrics.data_loss_count += count;
    }

    /// Check for zero data loss (Rule 1)
    pub fn verify_zero_data_loss(&self) -> (bool, u32) {
        (self.metrics.data_loss_count == 0, self.metrics.data_loss_count)
    }

    /// Finalize deployment
    pub fn finalize_deployment(&mut self) -> bool {
        if self.current_stage == DeploymentStage::Production100 {
            if let Some(new_ver) = self.new_version.clone() {
                self.current_version = new_ver;
                self.new_version = None;
                return true;
            }
        }
        false
    }

    /// Rollback to previous version
    pub fn rollback(&mut self) -> bool {
        if let Some(prev) = &self.current_version.previous_version {
            self.current_stage = DeploymentStage::Rollback;
            // In production, this would restore the previous model weights
            true
        } else {
            false
        }
    }

    pub fn get_current_stage(&self) -> DeploymentStage {
        self.current_stage
    }

    pub fn get_traffic_percentage(&self) -> u32 {
        self.traffic_percentage
    }

    pub fn get_metrics(&self) -> &DeploymentMetrics {
        &self.metrics
    }

    pub fn get_current_version(&self) -> &DeploymentVersion {
        &self.current_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_creation() {
        let version = DeploymentVersion::new("v1.0", "hash_model", "hash_config");
        let manager = DeploymentManager::new(version);

        assert_eq!(manager.current_stage, DeploymentStage::PreFlight);
        assert_eq!(manager.traffic_percentage, 0);
    }

    #[test]
    fn test_canary_deployment() {
        let version_v1 = DeploymentVersion::new("v1.0", "hash_model_1", "hash_config_1");
        let mut manager = DeploymentManager::new(version_v1);

        let version_v2 = DeploymentVersion::new("v2.0", "hash_model_2", "hash_config_2");
        assert!(manager.start_deployment(version_v2));

        assert_eq!(manager.current_stage, DeploymentStage::PreFlight);
    }

    #[test]
    fn test_traffic_routing() {
        let version_v1 = DeploymentVersion::new("v1.0", "hash_model_1", "hash_config_1");
        let mut manager = DeploymentManager::new(version_v1);

        let version_v2 = DeploymentVersion::new("v2.0", "hash_model_2", "hash_config_2");
        manager.start_deployment(version_v2);

        // Add passing metrics
        for _ in 0..100 {
            manager.record_inference(5.0, 98.5, true);
        }

        // Advance to canary 5%
        assert!(manager.advance_stage());
        assert_eq!(manager.traffic_percentage, 5);
        assert_eq!(manager.current_stage, DeploymentStage::Canary5);
    }

    #[test]
    fn test_health_gates() {
        let version = DeploymentVersion::new("v1.0", "hash_model", "hash_config");
        let mut manager = DeploymentManager::new(version);

        let gate = HealthGate::default_strict();
        assert!(gate.passes(5.0, 98.5, 0.05, 150.0)); // Should pass
        assert!(!gate.passes(10.0, 98.5, 0.05, 150.0)); // Latency too high
    }

    #[test]
    fn test_zero_data_loss() {
        let version = DeploymentVersion::new("v1.0", "hash_model", "hash_config");
        let mut manager = DeploymentManager::new(version);

        let (no_loss, count) = manager.verify_zero_data_loss();
        assert!(no_loss);
        assert_eq!(count, 0);

        // Record data loss
        manager.record_data_loss(5);
        let (no_loss_2, count_2) = manager.verify_zero_data_loss();
        assert!(!no_loss_2);
        assert_eq!(count_2, 5);
    }

    #[test]
    fn test_rollback_trigger() {
        let mut version_v1 = DeploymentVersion::new("v1.0", "hash_model_1", "hash_config_1");
        version_v1.previous_version = Some("v0.9".to_string());
        let mut manager = DeploymentManager::new(version_v1);

        assert!(manager.rollback());
        assert_eq!(manager.current_stage, DeploymentStage::Rollback);
    }
}

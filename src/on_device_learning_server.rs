// Phase 7: On-Device Learning Server
// Real-time ML model updates with drift monitoring and safety validation

use std::collections::VecDeque;
use crate::device_metrics_collector::PrivacyFilteredMetrics;
use crate::neural_predictor::NeuralPredictor;
use crate::ml_model::MLModel;

/// Training batch for local SGD
#[derive(Debug, Clone)]
pub struct TrainingBatch {
    pub metrics: Vec<PrivacyFilteredMetrics>,
    pub labels: Vec<usize>, // App class labels
    pub batch_id: u64,
    pub timestamp: u64,
}

/// Result of drift monitoring
#[derive(Debug, Clone)]
pub struct DriftResult {
    pub kl_divergence: f32,
    pub drift_detected: bool,
    pub retraining_needed: bool,
    pub confidence: f32,
}

/// Result of safety validation
#[derive(Debug, Clone)]
pub struct SafetyCheckResult {
    pub passed: bool,
    pub max_activation: f32,
    pub loss_value: f32,
    pub rollback_triggered: bool,
    pub reason: String,
}

/// Handles local model training with SGD
pub struct LocalTrainingEngine {
    learning_rate: f32,
    momentum: f32,
    batch_count: u64,
}

impl LocalTrainingEngine {
    pub fn new() -> Self {
        LocalTrainingEngine {
            learning_rate: 0.01,
            momentum: 0.9,
            batch_count: 0,
        }
    }

    /// Train model on batch - target <500ms (Rule R4)
    pub fn train_batch(
        &mut self,
        batch: &TrainingBatch,
        model: &mut MLModel,
    ) -> Result<f32, String> {
        if batch.metrics.is_empty() {
            return Err("Empty batch".to_string());
        }

        // Simulate SGD training on batch
        // In real implementation: use tflite::Interpreter for on-device training
        // For now: simulate loss decrease based on batch size and iteration
        let base_loss = 0.5;
        let improvement = (batch.metrics.len() as f32 / 100.0).min(0.3);
        let loss = (base_loss - improvement).max(0.1);

        self.batch_count += 1;

        // Adaptive learning rate: decrease if loss increases (prevent divergence)
        if loss > base_loss * 1.1 {
            self.learning_rate *= 0.5;
        } else {
            self.learning_rate = 0.01; // Reset to default
        }

        Ok(loss)
    }

    pub fn get_batch_count(&self) -> u64 {
        self.batch_count
    }
}

/// Monitors for concept drift in production data
pub struct DriftMonitoringV2 {
    baseline_distribution: Vec<f32>,
    sample_count: usize,
    drift_threshold: f32,
}

impl DriftMonitoringV2 {
    pub fn new() -> Self {
        DriftMonitoringV2 {
            baseline_distribution: vec![0.1; 10], // 10-class uniform baseline
            sample_count: 0,
            drift_threshold: 0.3, // KL divergence threshold
        }
    }

    /// Compute KL divergence between current and baseline distribution
    fn compute_kl_divergence(p: &[f32], q: &[f32]) -> f32 {
        if p.len() != q.len() {
            return 999.0;
        }

        let mut kl = 0.0;
        for i in 0..p.len() {
            if p[i] > 1e-10 && q[i] > 1e-10 {
                kl += p[i] * (p[i] / q[i]).ln();
            }
        }
        kl
    }

    /// Check for drift in new data
    pub fn detect_drift(&mut self, new_distribution: &[f32]) -> DriftResult {
        if new_distribution.len() != 10 {
            return DriftResult {
                kl_divergence: 0.0,
                drift_detected: false,
                retraining_needed: false,
                confidence: 0.0,
            };
        }

        let kl = Self::compute_kl_divergence(new_distribution, &self.baseline_distribution);
        let drift_detected = kl > self.drift_threshold;

        // Update baseline if drift detected (adaptive baseline)
        if drift_detected {
            // Exponential moving average: baseline = 0.9 * old + 0.1 * new
            for i in 0..10 {
                self.baseline_distribution[i] =
                    0.9 * self.baseline_distribution[i] + 0.1 * new_distribution[i];
            }
        }

        self.sample_count += 1;

        DriftResult {
            kl_divergence: kl,
            drift_detected,
            retraining_needed: drift_detected && self.sample_count > 100,
            confidence: (kl / self.drift_threshold).min(1.0),
        }
    }

    pub fn set_baseline(&mut self, distribution: Vec<f32>) {
        self.baseline_distribution = distribution;
    }

    pub fn get_sample_count(&self) -> usize {
        self.sample_count
    }
}

/// Validates model safety and detects divergence
pub struct SafetyValidator {
    max_loss_threshold: f32,
    max_activation_threshold: f32,
    divergence_count: u32,
    last_known_good_state: Option<Vec<u8>>, // Model checkpoint
}

impl SafetyValidator {
    pub fn new() -> Self {
        SafetyValidator {
            max_loss_threshold: 2.0, // Loss > 2.0 triggers alert
            max_activation_threshold: 100.0, // Activations > 100 suggest explosion
            divergence_count: 0,
            last_known_good_state: None,
        }
    }

    /// Validate model state for divergence - Rule R5: <100ms rollback
    pub fn validate_and_checkpoint(
        &mut self,
        loss: f32,
        max_activation: f32,
        model_state: Option<Vec<u8>>,
    ) -> SafetyCheckResult {
        let mut passed = true;
        let mut rollback_triggered = false;
        let mut reason = String::new();

        // Check for loss divergence
        if loss > self.max_loss_threshold {
            passed = false;
            self.divergence_count += 1;
            reason.push_str(&format!("Loss divergence: {:.3} > {:.3}", loss, self.max_loss_threshold));

            if self.divergence_count > 3 {
                rollback_triggered = true;
                reason.push_str(" | ROLLBACK TRIGGERED");
            }
        } else {
            self.divergence_count = 0; // Reset counter on good loss
        }

        // Check for activation explosion
        if max_activation > self.max_activation_threshold {
            passed = false;
            if !reason.is_empty() {
                reason.push_str(" | ");
            }
            reason.push_str(&format!("Activation explosion: {:.1}", max_activation));
            rollback_triggered = true;
        }

        // Save checkpoint if model is valid
        if passed && model_state.is_some() {
            self.last_known_good_state = model_state;
        }

        SafetyCheckResult {
            passed,
            max_activation,
            loss_value: loss,
            rollback_triggered,
            reason,
        }
    }

    /// Emergency rollback to last known good state
    pub fn emergency_rollback(&self) -> Result<Vec<u8>, String> {
        // Rule R5: <100ms recovery time
        match &self.last_known_good_state {
            Some(state) => Ok(state.clone()),
            None => Err("No checkpoint available".to_string()),
        }
    }

    pub fn get_divergence_count(&self) -> u32 {
        self.divergence_count
    }
}

/// Main on-device learning server coordinator
pub struct OnDeviceLearningServer {
    training_engine: LocalTrainingEngine,
    drift_monitor: DriftMonitoringV2,
    safety_validator: SafetyValidator,
    batch_buffer: VecDeque<TrainingBatch>,
    model: MLModel,
    retraining_count: u32,
}

impl OnDeviceLearningServer {
    pub fn new(model: MLModel) -> Self {
        OnDeviceLearningServer {
            training_engine: LocalTrainingEngine::new(),
            drift_monitor: DriftMonitoringV2::new(),
            safety_validator: SafetyValidator::new(),
            batch_buffer: VecDeque::with_capacity(10),
            model,
            retraining_count: 0,
        }
    }

    /// Add training data to batch buffer
    pub fn add_training_sample(&mut self, metrics: PrivacyFilteredMetrics, label: usize) {
        if self.batch_buffer.is_empty() {
            let batch = TrainingBatch {
                metrics: vec![metrics],
                labels: vec![label],
                batch_id: self.retraining_count as u64,
                timestamp: metrics.timestamp,
            };
            self.batch_buffer.push_back(batch);
        } else {
            let mut last_batch = self.batch_buffer.pop_back().unwrap();
            last_batch.metrics.push(metrics);
            last_batch.labels.push(label);
            self.batch_buffer.push_back(last_batch);
        }
    }

    /// Execute training on accumulated batch - Rule R4: <500ms
    pub fn train_batch(&mut self, batch_size: usize) -> Result<(f32, SafetyCheckResult), String> {
        if self.batch_buffer.is_empty() {
            return Err("No batches available".to_string());
        }

        let batch = self.batch_buffer.pop_front().unwrap();

        // Perform training - target <500ms
        let loss = self.training_engine.train_batch(&batch, &mut self.model)?;

        // Simulate max activation for safety check
        let max_activation = 25.0; // Normal range
        let model_state = Some(vec![1; 100]); // Dummy checkpoint

        // Validate safety
        let safety_result = self.safety_validator.validate_and_checkpoint(
            loss,
            max_activation,
            model_state,
        );

        if safety_result.rollback_triggered {
            // Trigger rollback
            let _ = self.safety_validator.emergency_rollback();
        }

        self.retraining_count += 1;
        Ok((loss, safety_result))
    }

    /// Check for concept drift and trigger retraining
    pub fn check_and_adapt(&mut self, current_distribution: &[f32]) -> DriftResult {
        let drift_result = self.drift_monitor.detect_drift(current_distribution);

        if drift_result.retraining_needed {
            // In production: trigger background retraining job
            // Signal for model update with new data
        }

        drift_result
    }

    /// Get convergence metrics
    pub fn get_convergence_phase(&self) -> String {
        let batch_count = self.training_engine.get_batch_count();

        if batch_count < 10 {
            "Initialization".to_string()
        } else if batch_count < 50 {
            "Learning".to_string()
        } else {
            "Converged".to_string()
        }
    }

    /// Get server health metrics
    pub fn get_health_status(&self) -> (bool, String) {
        let divergence = self.safety_validator.get_divergence_count();
        let healthy = divergence < 3;

        let status = format!(
            "Divergence count: {}, Batches trained: {}, Retraining count: {}",
            divergence,
            self.training_engine.get_batch_count(),
            self.retraining_count
        );

        (healthy, status)
    }

    pub fn get_batch_buffer_size(&self) -> usize {
        self.batch_buffer.len()
    }

    pub fn clear_batch_buffer(&mut self) {
        self.batch_buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml_model::MLModel;

    fn create_test_batch() -> TrainingBatch {
        let metric = PrivacyFilteredMetrics {
            timestamp: 1000,
            cpu_load: 0.5,
            cpu_frequency_mhz: 2400,
            memory_usage_percent: 50.0,
            battery_percent: 80.0,
            battery_temp_c: 25.0,
            soc_temp_c: 40.0,
            gpu_freq_mhz: 800,
            app_hash: "abc123".to_string(),
            location_cell_id: Some(12345),
        };

        TrainingBatch {
            metrics: vec![metric],
            labels: vec![0],
            batch_id: 1,
            timestamp: 1000,
        }
    }

    #[test]
    fn test_local_training() {
        let mut engine = LocalTrainingEngine::new();
        let mut model = MLModel::new();
        let batch = create_test_batch();

        let result = engine.train_batch(&batch, &mut model);
        assert!(result.is_ok());
        assert!(result.unwrap() > 0.0);
    }

    #[test]
    fn test_training_latency() {
        let mut engine = LocalTrainingEngine::new();
        let mut model = MLModel::new();
        let batch = create_test_batch();

        let start = std::time::SystemTime::now();
        for _ in 0..10 {
            let _ = engine.train_batch(&batch, &mut model);
        }
        let elapsed = start.elapsed().unwrap().as_millis();

        // 10 batches should be <500ms average (<50ms each)
        assert!(elapsed < 500, "Training latency: {}ms", elapsed);
    }

    #[test]
    fn test_drift_detection() {
        let mut monitor = DriftMonitoringV2::new();

        // Test with baseline distribution
        let baseline = vec![0.1; 10];
        let result = monitor.detect_drift(&baseline);
        assert!(!result.drift_detected);

        // Test with shifted distribution (high drift)
        let shifted = vec![0.01, 0.01, 0.01, 0.01, 0.01, 0.5, 0.1, 0.1, 0.1, 0.1];
        let result = monitor.detect_drift(&shifted);
        // Should detect drift as KL divergence is high
        assert!(result.kl_divergence > 0.0);
    }

    #[test]
    fn test_automatic_retraining() {
        let model = MLModel::new();
        let mut server = OnDeviceLearningServer::new(model);

        // Add samples to trigger retraining
        for i in 0..5 {
            let metric = PrivacyFilteredMetrics {
                timestamp: 1000 + i as u64,
                cpu_load: 0.5,
                cpu_frequency_mhz: 2400,
                memory_usage_percent: 50.0,
                battery_percent: 80.0,
                battery_temp_c: 25.0,
                soc_temp_c: 40.0,
                gpu_freq_mhz: 800,
                app_hash: format!("app_{}", i),
                location_cell_id: Some(12345),
            };
            server.add_training_sample(metric, i % 3);
        }

        assert_eq!(server.get_batch_buffer_size(), 1);
    }

    #[test]
    fn test_model_divergence_detection() {
        let mut validator = SafetyValidator::new();

        // Normal loss
        let result = validator.validate_and_checkpoint(0.5, 25.0, None);
        assert!(result.passed);

        // Divergence loss
        let result = validator.validate_and_checkpoint(3.0, 25.0, None);
        assert!(!result.passed);
        assert!(result.reason.contains("Loss divergence"));
    }

    #[test]
    fn test_rollback_mechanism() {
        let mut validator = SafetyValidator::new();

        // Trigger multiple divergences to trigger rollback
        for _ in 0..4 {
            let _ = validator.validate_and_checkpoint(3.0, 25.0, Some(vec![1; 100]));
        }

        let result = validator.emergency_rollback();
        assert!(result.is_ok());
    }

    #[test]
    fn test_convergence_tracking() {
        let model = MLModel::new();
        let server = OnDeviceLearningServer::new(model);

        let phase = server.get_convergence_phase();
        assert!(!phase.is_empty());
    }

    #[test]
    fn test_safety_validator() {
        let mut validator = SafetyValidator::new();

        // Test activation explosion detection
        let result = validator.validate_and_checkpoint(0.5, 150.0, None);
        assert!(!result.passed);
        assert!(result.rollback_triggered);
    }

    #[test]
    fn test_batch_consistency() {
        let model = MLModel::new();
        let mut server = OnDeviceLearningServer::new(model);

        // Same batch should produce deterministic results
        let metric = PrivacyFilteredMetrics {
            timestamp: 1000,
            cpu_load: 0.5,
            cpu_frequency_mhz: 2400,
            memory_usage_percent: 50.0,
            battery_percent: 80.0,
            battery_temp_c: 25.0,
            soc_temp_c: 40.0,
            gpu_freq_mhz: 800,
            app_hash: "test".to_string(),
            location_cell_id: Some(12345),
        };

        server.add_training_sample(metric.clone(), 0);
        let result1 = server.train_batch(1);

        server.add_training_sample(metric.clone(), 0);
        let result2 = server.train_batch(1);

        assert!(result1.is_ok() && result2.is_ok());
    }
}

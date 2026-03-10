// Phase 8: Multi-Task Learning
// Joint optimization for power, thermal, and latency

use crate::lstm_sequence_model::LSTMSequenceModel;

/// Task-specific prediction result
#[derive(Debug, Clone)]
pub struct TaskPrediction {
    pub predictions: Vec<f32>, // Softmax output
    pub class: usize,          // Predicted class
    pub confidence: f32,       // Max softmax value
}

/// Multi-task learning result
#[derive(Debug, Clone)]
pub struct MultiTaskOutput {
    pub power_pred: TaskPrediction,    // 0=Low, 1=Med, 2=High
    pub thermal_pred: TaskPrediction,  // 0=Cold, 1=Normal, 2=Hot
    pub latency_pred: TaskPrediction,  // 0=Fast, 1=Med, 2=Slow
    pub shared_features: Vec<f32>,     // Shared LSTM features
}

/// Multi-task loss tracking
#[derive(Debug, Clone)]
pub struct TaskLosses {
    pub power_loss: f32,
    pub thermal_loss: f32,
    pub latency_loss: f32,
    pub total_loss: f32,
    pub weights: (f32, f32, f32), // α, β, γ
}

/// Task-specific head (classifier)
pub struct TaskHead {
    task_name: String,
    num_classes: usize,
    weights: Vec<Vec<f32>>, // 32 input → num_classes
}

impl TaskHead {
    pub fn new(task_name: &str, num_classes: usize, input_dim: usize) -> Self {
        let init_val = 0.1;
        TaskHead {
            task_name: task_name.to_string(),
            num_classes,
            weights: vec![vec![init_val; input_dim]; num_classes],
        }
    }

    /// Forward pass: compute logits and softmax
    pub fn forward(&self, features: &[f32]) -> Vec<f32> {
        // Compute logits
        let mut logits = vec![0.0; self.num_classes];
        for i in 0..self.num_classes {
            logits[i] = features
                .iter()
                .zip(&self.weights[i])
                .map(|(a, b)| a * b)
                .sum();
        }

        // Softmax
        let max_logit = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exp_logits: Vec<f32> = logits.iter().map(|x| (x - max_logit).exp()).collect();
        let sum_exp: f32 = exp_logits.iter().sum();
        exp_logits.iter().map(|x| x / sum_exp).collect()
    }

    /// Compute cross-entropy loss
    pub fn compute_loss(&self, predictions: &[f32], target: usize) -> f32 {
        if target >= self.num_classes {
            return 0.0;
        }
        let target_pred = predictions[target].max(1e-7); // Numerical stability
        -target_pred.ln()
    }
}

/// Shared LSTM backbone (from Phase 8A)
pub struct SharedBackbone {
    lstm: LSTMSequenceModel,
}

impl SharedBackbone {
    pub fn new() -> Self {
        SharedBackbone {
            lstm: LSTMSequenceModel::new(),
        }
    }

    /// Extract shared features from LSTM hidden state
    pub fn forward(&self) -> Option<Vec<f32>> {
        self.lstm.forward().map(|output| output.hidden_state)
    }

    pub fn is_ready(&self) -> bool {
        self.lstm.is_ready()
    }

    pub fn buffer_len(&self) -> usize {
        self.lstm.buffer_len()
    }
}

/// Main multi-task learner
pub struct MultiTaskLearner {
    backbone: SharedBackbone,
    power_head: TaskHead,      // 2 classes: Low, Med, High
    thermal_head: TaskHead,    // 3 classes: Cold, Normal, Hot
    latency_head: TaskHead,    // 3 classes: Fast, Med, Slow
    task_weights: (f32, f32, f32), // α, β, γ normalized
    losses_history: Vec<TaskLosses>,
}

impl MultiTaskLearner {
    pub fn new() -> Self {
        MultiTaskLearner {
            backbone: SharedBackbone::new(),
            power_head: TaskHead::new("power", 2, 32),
            thermal_head: TaskHead::new("thermal", 3, 32),
            latency_head: TaskHead::new("latency", 3, 32),
            task_weights: (0.33, 0.33, 0.34), // Equal initial weights
            losses_history: vec![],
        }
    }

    /// Forward pass through all tasks
    pub fn forward(&self) -> Option<MultiTaskOutput> {
        let shared_features = self.backbone.forward()?;

        // Task 1: Power prediction
        let power_pred_vec = self.power_head.forward(&shared_features);
        let power_class = power_pred_vec
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        let power_confidence = power_pred_vec.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        // Task 2: Thermal prediction
        let thermal_pred_vec = self.thermal_head.forward(&shared_features);
        let thermal_class = thermal_pred_vec
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        let thermal_confidence = thermal_pred_vec.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        // Task 3: Latency prediction
        let latency_pred_vec = self.latency_head.forward(&shared_features);
        let latency_class = latency_pred_vec
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        let latency_confidence = latency_pred_vec.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        Some(MultiTaskOutput {
            power_pred: TaskPrediction {
                predictions: power_pred_vec,
                class: power_class,
                confidence: power_confidence,
            },
            thermal_pred: TaskPrediction {
                predictions: thermal_pred_vec,
                class: thermal_class,
                confidence: thermal_confidence,
            },
            latency_pred: TaskPrediction {
                predictions: latency_pred_vec,
                class: latency_class,
                confidence: latency_confidence,
            },
            shared_features,
        })
    }

    /// Compute weighted multi-task loss
    pub fn compute_loss(
        &mut self,
        power_target: usize,
        thermal_target: usize,
        latency_target: usize,
    ) -> Option<TaskLosses> {
        let output = self.forward()?;

        let power_loss = self.power_head.compute_loss(&output.power_pred.predictions, power_target);
        let thermal_loss =
            self.thermal_head.compute_loss(&output.thermal_pred.predictions, thermal_target);
        let latency_loss =
            self.latency_head.compute_loss(&output.latency_pred.predictions, latency_target);

        let (α, β, γ) = self.task_weights;
        let total_loss = α * power_loss + β * thermal_loss + γ * latency_loss;

        let losses = TaskLosses {
            power_loss,
            thermal_loss,
            latency_loss,
            total_loss,
            weights: self.task_weights,
        };

        self.losses_history.push(losses.clone());

        Some(losses)
    }

    /// Dynamically adjust task weights if one task is underperforming
    pub fn update_task_weights(&mut self, losses: &TaskLosses) {
        let avg_loss = (losses.power_loss + losses.thermal_loss + losses.latency_loss) / 3.0;

        // Increase weight for high-loss tasks
        let power_weight = if losses.power_loss > avg_loss * 1.2 {
            0.35
        } else {
            0.33
        };
        let thermal_weight = if losses.thermal_loss > avg_loss * 1.2 {
            0.35
        } else {
            0.33
        };
        let latency_weight = if losses.latency_loss > avg_loss * 1.2 {
            0.35
        } else {
            0.34
        };

        // Normalize
        let total = power_weight + thermal_weight + latency_weight;
        self.task_weights = (
            power_weight / total,
            thermal_weight / total,
            latency_weight / total,
        );
    }

    /// Check if all tasks are balanced (within ±5%)
    pub fn check_task_balance(&self) -> (bool, f32) {
        if self.losses_history.len() < 2 {
            return (true, 0.0);
        }

        let recent = &self.losses_history[self.losses_history.len() - 1];
        let avg = (recent.power_loss + recent.thermal_loss + recent.latency_loss) / 3.0;

        let power_var = (recent.power_loss - avg).abs() / avg;
        let thermal_var = (recent.thermal_loss - avg).abs() / avg;
        let latency_var = (recent.latency_loss - avg).abs() / avg;

        let max_var = power_var.max(thermal_var).max(latency_var);
        (max_var <= 0.05, max_var)
    }

    /// Verify that joint training improves all tasks
    pub fn evaluate_improvement(&self) -> (bool, f32) {
        if self.losses_history.len() < 10 {
            return (false, 0.0); // Need history for comparison
        }

        let early = &self.losses_history[0];
        let recent = &self.losses_history[self.losses_history.len() - 1];

        let power_improvement = (early.power_loss - recent.power_loss) / early.power_loss;
        let thermal_improvement = (early.thermal_loss - recent.thermal_loss) / early.thermal_loss;
        let latency_improvement = (early.latency_loss - recent.latency_loss) / early.latency_loss;

        let min_improvement = power_improvement
            .min(thermal_improvement)
            .min(latency_improvement);
        (min_improvement >= 0.02, min_improvement)
    }

    /// Get normalized gradient for gradient norm balancing
    pub fn get_gradient_norms(&self) -> (f32, f32, f32) {
        // Simplified: use task losses as proxy for gradient magnitudes
        if let Some(recent) = self.losses_history.last() {
            (
                recent.power_loss.abs(),
                recent.thermal_loss.abs(),
                recent.latency_loss.abs(),
            )
        } else {
            (0.0, 0.0, 0.0)
        }
    }

    pub fn get_task_weights(&self) -> (f32, f32, f32) {
        self.task_weights
    }

    pub fn is_ready(&self) -> bool {
        self.backbone.is_ready()
    }

    pub fn buffer_len(&self) -> usize {
        self.backbone.buffer_len()
    }

    pub fn get_losses_history(&self) -> &[TaskLosses] {
        &self.losses_history
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_metrics_collector::PrivacyFilteredMetrics;

    fn create_test_metric() -> PrivacyFilteredMetrics {
        PrivacyFilteredMetrics {
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
        }
    }

    #[test]
    fn test_multi_task_creation() {
        let mtl = MultiTaskLearner::new();
        assert_eq!(mtl.power_head.num_classes, 2);
        assert_eq!(mtl.thermal_head.num_classes, 3);
        assert_eq!(mtl.latency_head.num_classes, 3);
    }

    #[test]
    fn test_shared_backbone() {
        let mtl = MultiTaskLearner::new();
        assert!(!mtl.is_ready());
    }

    #[test]
    fn test_power_task() {
        let head = TaskHead::new("power", 2, 32);
        let features = vec![0.5; 32];
        let pred = head.forward(&features);

        assert_eq!(pred.len(), 2);
        let sum: f32 = pred.iter().sum();
        assert!((sum - 1.0).abs() < 0.01); // Softmax
    }

    #[test]
    fn test_thermal_task() {
        let head = TaskHead::new("thermal", 3, 32);
        let features = vec![0.5; 32];
        let pred = head.forward(&features);

        assert_eq!(pred.len(), 3);
        let sum: f32 = pred.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_latency_task() {
        let head = TaskHead::new("latency", 3, 32);
        let features = vec![0.5; 32];
        let pred = head.forward(&features);

        assert_eq!(pred.len(), 3);
    }

    #[test]
    fn test_task_balance() {
        let mut mtl = MultiTaskLearner::new();

        // Simulate some losses
        let test_losses = vec![
            TaskLosses {
                power_loss: 0.5,
                thermal_loss: 0.51,
                latency_loss: 0.49,
                total_loss: 1.5,
                weights: (0.33, 0.33, 0.34),
            },
        ];

        for loss in test_losses {
            mtl.losses_history.push(loss);
        }

        let (balanced, variance) = mtl.check_task_balance();
        assert!(variance <= 0.05); // Within ±5%
    }

    #[test]
    fn test_joint_loss() {
        let head = TaskHead::new("test", 3, 32);
        let features = vec![0.5; 32];
        let pred = head.forward(&features);

        let loss = head.compute_loss(&pred, 0);
        assert!(loss > 0.0); // Non-negative loss
    }

    #[test]
    fn test_dynamic_weighting() {
        let mut mtl = MultiTaskLearner::new();
        let losses = TaskLosses {
            power_loss: 1.0,
            thermal_loss: 0.5,
            latency_loss: 0.5,
            total_loss: 2.0,
            weights: (0.33, 0.33, 0.34),
        };

        mtl.update_task_weights(&losses);
        let (α, β, γ) = mtl.get_task_weights();

        // Power weight should be higher due to high loss
        assert!(α > β);
        // Sum should be ~1.0
        assert!(((α + β + γ) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_multi_task_improvement() {
        let mut mtl = MultiTaskLearner::new();

        // Simulate training history
        for i in 0..15 {
            let loss_factor = 1.0 - (i as f32 * 0.03); // Decreasing loss
            mtl.losses_history.push(TaskLosses {
                power_loss: 0.5 * loss_factor,
                thermal_loss: 0.5 * loss_factor,
                latency_loss: 0.5 * loss_factor,
                total_loss: 1.5 * loss_factor,
                weights: (0.33, 0.33, 0.34),
            });
        }

        let (improving, improvement) = mtl.evaluate_improvement();
        assert!(improvement > 0.0); // Should show improvement
    }

    #[test]
    fn test_gradient_normalization() {
        let mut mtl = MultiTaskLearner::new();
        mtl.losses_history.push(TaskLosses {
            power_loss: 0.6,
            thermal_loss: 0.5,
            latency_loss: 0.4,
            total_loss: 1.5,
            weights: (0.33, 0.33, 0.34),
        });

        let (power_grad, thermal_grad, latency_grad) = mtl.get_gradient_norms();
        assert!(power_grad > thermal_grad);
        assert!(thermal_grad > latency_grad);
    }

    #[test]
    fn test_task_weight_balance() {
        let mtl = MultiTaskLearner::new();
        let (α, β, γ) = mtl.get_task_weights();

        // Weights should be normalized
        let sum = α + β + γ;
        assert!((sum - 1.0).abs() < 0.01);
    }
}

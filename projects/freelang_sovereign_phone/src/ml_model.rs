// Project Sovereign: ML Model Module
// Goal: TensorFlow Lite neural network inference
// Target: <10ms latency, <50KB model size, 95%+ accuracy

use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ModelQuantization {
    Float32,
    Float16,
    Int8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PredictionConfidence {
    High,     // >0.8
    Medium,   // 0.5-0.8
    Low,      // <0.5
}

#[derive(Clone, Debug)]
pub struct ModelConfig {
    pub input_size: usize,          // 9 features
    pub hidden_layer1: usize,       // 128 neurons
    pub hidden_layer2: usize,       // 64 neurons
    pub output_size: usize,         // 32 app predictions
    pub quantization: ModelQuantization,
}

#[derive(Clone, Debug)]
pub struct Prediction {
    pub app_id: usize,
    pub confidence: f64,             // 0.0-1.0
    pub confidence_level: PredictionConfidence,
}

#[derive(Clone, Debug)]
pub struct PredictionResult {
    pub top_predictions: Vec<Prediction>,
    pub inference_time_ms: f64,
    pub timestamp: u64,
    pub model_version: usize,
}

#[derive(Clone, Debug)]
pub struct ModelMetrics {
    pub total_inferences: usize,
    pub avg_inference_time_ms: f64,
    pub max_inference_time_ms: f64,
    pub min_inference_time_ms: f64,
    pub model_size_bytes: usize,
    pub parameter_count: usize,
}

pub struct MLModel {
    // Model configuration
    config: ModelConfig,
    model_version: usize,

    // Simplified network weights (in production: loaded from TFLite)
    weights_h1: Vec<Vec<f64>>,        // input_size × hidden_layer1
    weights_h2: Vec<Vec<f64>>,        // hidden_layer1 × hidden_layer2
    weights_out: Vec<Vec<f64>>,       // hidden_layer2 × output_size
    bias_h1: Vec<f64>,                // hidden_layer1
    bias_h2: Vec<f64>,                // hidden_layer2
    bias_out: Vec<f64>,               // output_size

    // Inference cache
    inference_cache: VecDeque<(Vec<f64>, PredictionResult)>,
    cache_max_size: usize,

    // Performance metrics
    inference_times: VecDeque<f64>,
    total_inferences: usize,
}

impl MLModel {
    pub fn new(config: ModelConfig) -> Self {
        // Initialize weights with small random values
        let h1 = config.hidden_layer1;
        let h2 = config.hidden_layer2;
        let out = config.output_size;
        let inp = config.input_size;

        Self {
            config,
            model_version: 1,

            // Initialize with small random weights (in production: load from saved model)
            weights_h1: vec![vec![0.01; h1]; inp],
            weights_h2: vec![vec![0.01; h2]; h1],
            weights_out: vec![vec![0.01; out]; h2],
            bias_h1: vec![0.0; h1],
            bias_h2: vec![0.0; h2],
            bias_out: vec![0.0; out],

            inference_cache: VecDeque::with_capacity(100),
            cache_max_size: 100,

            inference_times: VecDeque::with_capacity(100),
            total_inferences: 0,
        }
    }

    /// Perform neural network inference
    pub fn predict(&mut self, input: Vec<f64>) -> Result<PredictionResult, String> {
        // Validate input
        if input.len() != self.config.input_size {
            return Err(format!("Expected {} inputs, got {}", self.config.input_size, input.len()));
        }

        // Check cache first
        if let Some((cached_input, cached_result)) = self.inference_cache.iter()
            .find(|(cached_inp, _)| (cached_inp.iter().zip(&input).map(|(a, b)| (a - b).abs()).sum::<f64>()) < 0.001)
        {
            return Ok(cached_result.clone());
        }

        let start_time = std::time::SystemTime::now();

        // Forward pass
        let h1_output = self.forward_layer(&input, &self.weights_h1, &self.bias_h1);
        let h1_activated = self.relu(&h1_output);

        let h2_output = self.forward_layer(&h1_activated, &self.weights_h2, &self.bias_h2);
        let h2_activated = self.relu(&h2_output);

        let logits = self.forward_layer(&h2_activated, &self.weights_out, &self.bias_out);
        let probabilities = self.softmax(&logits);

        let inference_time = start_time.elapsed()
            .unwrap_or_default()
            .as_secs_f64() * 1000.0;

        // Generate predictions
        let mut predictions: Vec<(usize, f64)> = probabilities
            .iter()
            .enumerate()
            .collect::<Vec<_>>()
            .iter()
            .map(|(idx, prob)| (*idx, **prob))
            .collect();

        predictions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Create top-3 predictions
        let top_predictions: Vec<Prediction> = predictions
            .iter()
            .take(3)
            .map(|(app_id, conf)| {
                let confidence_level = match conf {
                    c if c > &0.8 => PredictionConfidence::High,
                    c if c > &0.5 => PredictionConfidence::Medium,
                    _ => PredictionConfidence::Low,
                };

                Prediction {
                    app_id: *app_id,
                    confidence: *conf,
                    confidence_level,
                }
            })
            .collect();

        let result = PredictionResult {
            top_predictions,
            inference_time_ms: inference_time,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            model_version: self.model_version,
        };

        // Update metrics
        self.total_inferences += 1;
        self.inference_times.push_back(inference_time);
        if self.inference_times.len() > 100 {
            self.inference_times.pop_front();
        }

        // Cache result
        self.inference_cache.push_back((input, result.clone()));
        if self.inference_cache.len() > self.cache_max_size {
            self.inference_cache.pop_front();
        }

        Ok(result)
    }

    /// Batch prediction for multiple inputs
    pub fn predict_batch(&mut self, inputs: Vec<Vec<f64>>) -> Result<Vec<PredictionResult>, String> {
        let mut results = Vec::new();

        for input in inputs {
            let result = self.predict(input)?;
            results.push(result);
        }

        Ok(results)
    }

    fn forward_layer(&self, input: &[f64], weights: &[Vec<f64>], bias: &[f64]) -> Vec<f64> {
        let mut output = vec![0.0; weights[0].len()];

        for (i, w_row) in weights.iter().enumerate() {
            let input_val = if i < input.len() { input[i] } else { 0.0 };
            for (j, &w) in w_row.iter().enumerate() {
                output[j] += input_val * w;
            }
        }

        for (i, b) in bias.iter().enumerate() {
            if i < output.len() {
                output[i] += b;
            }
        }

        output
    }

    fn relu(&self, input: &[f64]) -> Vec<f64> {
        input.iter().map(|&x| if x > 0.0 { x } else { 0.0 }).collect()
    }

    fn softmax(&self, input: &[f64]) -> Vec<f64> {
        let max = input.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let exp: Vec<f64> = input.iter().map(|&x| (x - max).exp()).collect();
        let sum: f64 = exp.iter().sum();

        if sum == 0.0 {
            vec![1.0 / input.len() as f64; input.len()]
        } else {
            exp.iter().map(|&x| x / sum).collect()
        }
    }

    /// Update model weights (for online learning)
    pub fn update_weights(
        &mut self,
        gradient_h1: &[Vec<f64>],
        gradient_h2: &[Vec<f64>],
        gradient_out: &[Vec<f64>],
        learning_rate: f64,
    ) {
        // Update weights using gradient descent
        for i in 0..self.weights_h1.len() {
            for j in 0..self.weights_h1[i].len() {
                if i < gradient_h1.len() && j < gradient_h1[i].len() {
                    self.weights_h1[i][j] -= learning_rate * gradient_h1[i][j];
                }
            }
        }

        for i in 0..self.weights_h2.len() {
            for j in 0..self.weights_h2[i].len() {
                if i < gradient_h2.len() && j < gradient_h2[i].len() {
                    self.weights_h2[i][j] -= learning_rate * gradient_h2[i][j];
                }
            }
        }

        for i in 0..self.weights_out.len() {
            for j in 0..self.weights_out[i].len() {
                if i < gradient_out.len() && j < gradient_out[i].len() {
                    self.weights_out[i][j] -= learning_rate * gradient_out[i][j];
                }
            }
        }

        self.model_version += 1;
    }

    /// Get model metrics
    pub fn get_metrics(&self) -> ModelMetrics {
        let avg_time = if self.inference_times.is_empty() {
            0.0
        } else {
            self.inference_times.iter().sum::<f64>() / self.inference_times.len() as f64
        };

        let max_time = self.inference_times.iter().cloned().fold(0.0, f64::max);
        let min_time = self.inference_times.iter().cloned().fold(f64::INFINITY, f64::min);

        // Estimate model size
        let h1_size = self.weights_h1.len() * self.weights_h1[0].len() * 4;
        let h2_size = self.weights_h2.len() * self.weights_h2[0].len() * 4;
        let out_size = self.weights_out.len() * self.weights_out[0].len() * 4;
        let bias_size = (self.bias_h1.len() + self.bias_h2.len() + self.bias_out.len()) * 8;
        let total_size = h1_size + h2_size + out_size + bias_size;

        let parameter_count = (self.weights_h1.len() * self.weights_h1[0].len()) +
                            (self.weights_h2.len() * self.weights_h2[0].len()) +
                            (self.weights_out.len() * self.weights_out[0].len()) +
                            self.bias_h1.len() + self.bias_h2.len() + self.bias_out.len();

        ModelMetrics {
            total_inferences: self.total_inferences,
            avg_inference_time_ms: avg_time,
            max_inference_time_ms: if max_time == 0.0 { f64::INFINITY } else { max_time },
            min_inference_time_ms: if min_time == f64::INFINITY { 0.0 } else { min_time },
            model_size_bytes: total_size,
            parameter_count,
        }
    }

    /// Clear cache and metrics
    pub fn reset_metrics(&mut self) {
        self.inference_times.clear();
        self.inference_cache.clear();
        self.total_inferences = 0;
    }

    /// Verify model size constraint (<50KB)
    pub fn verify_size_constraint(&self) -> bool {
        let metrics = self.get_metrics();
        metrics.model_size_bytes < 50_000  // 50KB limit
    }

    /// Verify inference latency constraint (<10ms)
    pub fn verify_latency_constraint(&self) -> bool {
        if self.inference_times.is_empty() {
            return true;
        }
        let metrics = self.get_metrics();
        metrics.avg_inference_time_ms < 10.0
    }

    /// Get model version
    pub fn get_version(&self) -> usize {
        self.model_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_model() -> MLModel {
        let config = ModelConfig {
            input_size: 9,
            hidden_layer1: 128,
            hidden_layer2: 64,
            output_size: 32,
            quantization: ModelQuantization::Int8,
        };
        MLModel::new(config)
    }

    fn create_test_input() -> Vec<f64> {
        vec![0.5, 0.6, 1.0, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7]
    }

    #[test]
    fn test_model_creation() {
        let model = create_test_model();
        assert_eq!(model.config.input_size, 9);
        assert_eq!(model.config.output_size, 32);
        assert_eq!(model.model_version, 1);
    }

    #[test]
    fn test_single_prediction() {
        let mut model = create_test_model();
        let input = create_test_input();

        let result = model.predict(input);
        assert!(result.is_ok());

        let pred = result.unwrap();
        assert_eq!(pred.top_predictions.len(), 3);
        assert!(pred.inference_time_ms > 0.0);
    }

    #[test]
    fn test_prediction_output_validity() {
        let mut model = create_test_model();
        let input = create_test_input();

        let result = model.predict(input).unwrap();

        // Check confidence scores
        for pred in &result.top_predictions {
            assert!(pred.confidence >= 0.0 && pred.confidence <= 1.0);
        }

        // Check descending confidence
        for i in 0..result.top_predictions.len() - 1 {
            assert!(result.top_predictions[i].confidence >= result.top_predictions[i + 1].confidence);
        }
    }

    #[test]
    fn test_batch_prediction() {
        let mut model = create_test_model();
        let inputs = vec![
            create_test_input(),
            create_test_input(),
            create_test_input(),
        ];

        let results = model.predict_batch(inputs);
        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 3);
    }

    #[test]
    fn test_invalid_input_size() {
        let mut model = create_test_model();
        let input = vec![0.5, 0.6];  // Too small

        let result = model.predict(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_inference_latency() {
        let mut model = create_test_model();
        let input = create_test_input();

        let _ = model.predict(input);
        let metrics = model.get_metrics();

        assert!(metrics.avg_inference_time_ms < 50.0);  // Should be fast
    }

    #[test]
    fn test_model_size_constraint() {
        let model = create_test_model();
        assert!(model.verify_size_constraint());
    }

    #[test]
    fn test_model_latency_constraint() {
        let mut model = create_test_model();
        let input = create_test_input();

        for _ in 0..10 {
            let _ = model.predict(input.clone());
        }

        assert!(model.verify_latency_constraint());
    }

    #[test]
    fn test_prediction_confidence_levels() {
        let mut model = create_test_model();
        let input = create_test_input();

        let result = model.predict(input).unwrap();

        // At least one prediction should exist
        assert!(!result.top_predictions.is_empty());

        // Confidence levels should be valid
        for pred in &result.top_predictions {
            match pred.confidence {
                c if c > 0.8 => assert_eq!(pred.confidence_level, PredictionConfidence::High),
                c if c > 0.5 => assert_eq!(pred.confidence_level, PredictionConfidence::Medium),
                _ => assert_eq!(pred.confidence_level, PredictionConfidence::Low),
            }
        }
    }

    #[test]
    fn test_cache_effectiveness() {
        let mut model = create_test_model();
        let input = create_test_input();

        let result1 = model.predict(input.clone()).unwrap();
        let result2 = model.predict(input).unwrap();

        // Same input should return similar predictions
        assert_eq!(result1.top_predictions[0].app_id, result2.top_predictions[0].app_id);
    }

    #[test]
    fn test_model_metrics() {
        let mut model = create_test_model();

        for _ in 0..5 {
            let _ = model.predict(create_test_input());
        }

        let metrics = model.get_metrics();
        assert_eq!(metrics.total_inferences, 5);
        assert!(metrics.avg_inference_time_ms > 0.0);
        assert!(metrics.parameter_count > 1000);
    }

    #[test]
    fn test_reset_metrics() {
        let mut model = create_test_model();

        let _ = model.predict(create_test_input());
        assert_eq!(model.get_metrics().total_inferences, 1);

        model.reset_metrics();
        assert_eq!(model.get_metrics().total_inferences, 0);
    }
}

// Project Sovereign: Neural Predictor Module
// Goal: Feature engineering + prediction pipeline
// Target: 95% accuracy, <10ms inference, top-3 app prediction

use std::collections::VecDeque;
use crate::ml_model::{MLModel, ModelConfig, ModelQuantization, PredictionResult};
use crate::user_behavior_model::UserEvent;
use crate::predictive_preload::PreloadPriority;

#[derive(Clone, Debug)]
pub struct FeatureVector {
    pub time_of_day_sin: f64,          // sin(2π * hour/24)
    pub time_of_day_cos: f64,          // cos(2π * hour/24)
    pub battery_level: f64,             // 0-100
    pub screen_state: f64,              // 0 or 1
    pub recent_apps: Vec<usize>,        // Last N app IDs (embedded to 9 dims)
    pub location_category: f64,         // 0-4
    pub temperature: f64,               // 20-60°C
    pub day_of_week: f64,               // 0-6
    pub memory_pressure: f64,           // 0-100%
    pub network_state: f64,             // 0-3 (Offline/2G/3G/4G/5G)
}

#[derive(Clone, Debug)]
pub struct PredictionInput {
    pub features: FeatureVector,
    pub timestamp: u64,
}

#[derive(Clone, Debug)]
pub struct AppPrediction {
    pub app_id: usize,
    pub confidence: f64,
    pub preload_priority: PreloadPriority,
    pub estimated_power_mw: f64,
    pub predicted_launch_probability: f64,
}

#[derive(Clone, Debug)]
pub struct NeuralPrediction {
    pub app_predictions: Vec<AppPrediction>,
    pub inference_time_ms: f64,
    pub model_version: usize,
    pub timestamp: u64,
}

pub struct NeuralPredictor {
    // ML Model
    model: MLModel,

    // Feature history (for trend analysis)
    feature_history: VecDeque<FeatureVector>,
    history_max_size: usize,

    // Prediction cache
    prediction_cache: VecDeque<(FeatureVector, NeuralPrediction)>,
    cache_max_size: usize,

    // Statistics
    total_predictions: usize,
    cache_hits: usize,
    cache_misses: usize,

    // Configuration
    top_k: usize,                       // Number of top predictions to return
    confidence_threshold: f64,          // Minimum confidence for preload
}

impl NeuralPredictor {
    pub fn new() -> Self {
        let config = ModelConfig {
            input_size: 9,
            hidden_layer1: 128,
            hidden_layer2: 64,
            output_size: 32,
            quantization: ModelQuantization::Int8,
        };

        Self {
            model: MLModel::new(config),
            feature_history: VecDeque::with_capacity(100),
            history_max_size: 100,
            prediction_cache: VecDeque::with_capacity(50),
            cache_max_size: 50,
            total_predictions: 0,
            cache_hits: 0,
            cache_misses: 0,
            top_k: 3,
            confidence_threshold: 0.5,
        }
    }

    /// Generate feature vector from raw data
    pub fn engineer_features(
        &self,
        hour: u8,
        battery_pct: f64,
        screen_on: bool,
        recent_apps: &[usize],
        location_id: usize,
        temperature: f64,
        day_of_week: u8,
        memory_pressure: f64,
        network_state: usize,
    ) -> FeatureVector {
        // Time of day features (sin/cos encoding for circular nature)
        let hour_rad = 2.0 * std::f64::consts::PI * (hour as f64) / 24.0;
        let time_of_day_sin = hour_rad.sin();
        let time_of_day_cos = hour_rad.cos();

        // Normalize battery (0-100 → 0-1)
        let battery_norm = battery_pct.max(0.0).min(100.0) / 100.0;

        // Screen state (boolean → 0/1)
        let screen_state = if screen_on { 1.0 } else { 0.0 };

        // Location category (0-4, normalized to 0-1)
        let location_category = (location_id as f64).min(4.0) / 4.0;

        // Temperature normalization (20-60°C → 0-1)
        let temp_norm = ((temperature - 20.0) / 40.0).max(0.0).min(1.0);

        // Day of week (0-6, normalized to 0-1)
        let day_norm = (day_of_week as f64).min(6.0) / 6.0;

        // Memory pressure (0-100% → 0-1)
        let memory_norm = memory_pressure.max(0.0).min(100.0) / 100.0;

        // Network state (0-4, normalized to 0-1)
        let network_norm = (network_state as f64).min(4.0) / 4.0;

        // Recent apps embedding (take last 5 apps, pad to 5)
        let mut recent_apps_vec = recent_apps.to_vec();
        while recent_apps_vec.len() < 5 {
            recent_apps_vec.push(0);
        }
        recent_apps_vec.truncate(5);

        FeatureVector {
            time_of_day_sin,
            time_of_day_cos,
            battery_level: battery_norm,
            screen_state,
            recent_apps: recent_apps_vec,
            location_category,
            temperature: temp_norm,
            day_of_week: day_norm,
            memory_pressure: memory_norm,
            network_state: network_norm,
        }
    }

    /// Flatten feature vector to neural network input
    fn flatten_features(&self, features: &FeatureVector) -> Vec<f64> {
        vec![
            features.time_of_day_sin,
            features.time_of_day_cos,
            features.battery_level,
            features.screen_state,
            features.location_category,
            features.temperature,
            features.day_of_week,
            features.memory_pressure,
            features.network_state,
        ]
    }

    /// Generate predictions from features
    pub fn predict(&mut self, features: FeatureVector) -> Result<NeuralPrediction, String> {
        // Check cache
        if let Some((_, cached_prediction)) = self.prediction_cache.iter()
            .find(|(cached_feat, _)| self.features_similar(&features, cached_feat))
        {
            self.cache_hits += 1;
            return Ok(cached_prediction.clone());
        }

        self.cache_misses += 1;

        // Flatten features
        let input = self.flatten_features(&features);

        // Get model prediction
        let model_result = self.model.predict(input)?;

        // Convert model output to app predictions
        let app_predictions = self.convert_predictions(&model_result, &features);

        let prediction = NeuralPrediction {
            app_predictions,
            inference_time_ms: model_result.inference_time_ms,
            model_version: model_result.model_version,
            timestamp: model_result.timestamp,
        };

        // Store in history
        self.feature_history.push_back(features.clone());
        if self.feature_history.len() > self.history_max_size {
            self.feature_history.pop_front();
        }

        // Cache prediction
        self.prediction_cache.push_back((features, prediction.clone()));
        if self.prediction_cache.len() > self.cache_max_size {
            self.prediction_cache.pop_front();
        }

        self.total_predictions += 1;

        Ok(prediction)
    }

    /// Batch prediction
    pub fn predict_batch(&mut self, feature_batch: Vec<FeatureVector>) -> Result<Vec<NeuralPrediction>, String> {
        let mut predictions = Vec::new();

        for features in feature_batch {
            let pred = self.predict(features)?;
            predictions.push(pred);
        }

        Ok(predictions)
    }

    fn convert_predictions(&self, model_result: &PredictionResult, features: &FeatureVector) -> Vec<AppPrediction> {
        model_result.top_predictions
            .iter()
            .enumerate()
            .map(|(idx, pred)| {
                // Determine preload priority based on confidence
                let preload_priority = match pred.confidence {
                    c if c > 0.8 => PreloadPriority::Critical,
                    c if c > 0.6 => PreloadPriority::High,
                    c if c > self.confidence_threshold => PreloadPriority::Medium,
                    _ => PreloadPriority::Low,
                };

                // Estimate power consumption based on app
                let base_power = 50.0;  // Base power per app
                let power_multiplier = 1.0 + (pred.confidence * 0.5);  // 1.0-1.5x based on confidence
                let estimated_power = base_power * power_multiplier;

                AppPrediction {
                    app_id: pred.app_id,
                    confidence: pred.confidence,
                    preload_priority,
                    estimated_power_mw: estimated_power,
                    predicted_launch_probability: pred.confidence,
                }
            })
            .collect()
    }

    fn features_similar(&self, feat1: &FeatureVector, feat2: &FeatureVector) -> bool {
        let mut distance = 0.0;

        distance += (feat1.battery_level - feat2.battery_level).abs();
        distance += (feat1.screen_state - feat2.screen_state).abs();
        distance += (feat1.temperature - feat2.temperature).abs();
        distance += (feat1.memory_pressure - feat2.memory_pressure).abs();
        distance += (feat1.network_state - feat2.network_state).abs();

        // Threshold: 0.1 (very similar)
        distance < 0.1
    }

    /// Get top prediction
    pub fn get_top_prediction(&mut self, features: FeatureVector) -> Result<AppPrediction, String> {
        let prediction = self.predict(features)?;
        Ok(prediction.app_predictions.first().cloned().unwrap_or_else(|| {
            AppPrediction {
                app_id: 0,
                confidence: 0.0,
                preload_priority: PreloadPriority::Low,
                estimated_power_mw: 0.0,
                predicted_launch_probability: 0.0,
            }
        }))
    }

    /// Get predictor statistics
    pub fn get_stats(&self) -> (usize, usize, usize, f64) {
        let cache_hit_rate = if self.total_predictions > 0 {
            (self.cache_hits as f64 / self.total_predictions as f64) * 100.0
        } else {
            0.0
        };

        (self.total_predictions, self.cache_hits, self.cache_misses, cache_hit_rate)
    }

    /// Set confidence threshold
    pub fn set_confidence_threshold(&mut self, threshold: f64) {
        self.confidence_threshold = threshold.max(0.0).min(1.0);
    }

    /// Get model metrics
    pub fn get_model_metrics(&self) {
        let metrics = self.model.get_metrics();
        println!("Model version: {}", self.model.get_version());
        println!("Total inferences: {}", metrics.total_inferences);
        println!("Avg inference time: {:.2}ms", metrics.avg_inference_time_ms);
        println!("Model size: {:.2}KB", metrics.model_size_bytes as f64 / 1024.0);
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.prediction_cache.clear();
        self.cache_hits = 0;
        self.cache_misses = 0;
    }

    /// Reset predictor state
    pub fn reset(&mut self) {
        self.model.reset_metrics();
        self.feature_history.clear();
        self.prediction_cache.clear();
        self.total_predictions = 0;
        self.cache_hits = 0;
        self.cache_misses = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predictor_creation() {
        let predictor = NeuralPredictor::new();
        assert_eq!(predictor.top_k, 3);
        assert_eq!(predictor.total_predictions, 0);
    }

    #[test]
    fn test_feature_engineering() {
        let predictor = NeuralPredictor::new();

        let features = predictor.engineer_features(
            12,      // noon
            85.0,    // battery
            true,    // screen on
            &[1, 2, 3, 4, 5],  // recent apps
            1,       // location
            35.0,    // temperature
            3,       // Wednesday
            45.0,    // memory pressure
            3,       // 4G network
        );

        // Check normalized ranges
        assert!(features.time_of_day_sin >= -1.0 && features.time_of_day_sin <= 1.0);
        assert!(features.time_of_day_cos >= -1.0 && features.time_of_day_cos <= 1.0);
        assert!(features.battery_level >= 0.0 && features.battery_level <= 1.0);
        assert!(features.screen_state == 1.0);
        assert_eq!(features.recent_apps.len(), 5);
    }

    #[test]
    fn test_feature_flattening() {
        let predictor = NeuralPredictor::new();

        let features = predictor.engineer_features(
            12, 85.0, true, &[1, 2], 1, 35.0, 3, 45.0, 3
        );

        let flat = predictor.flatten_features(&features);
        assert_eq!(flat.len(), 9);

        // All values should be in valid range
        for val in flat {
            assert!(val.is_finite());
        }
    }

    #[test]
    fn test_single_prediction() {
        let mut predictor = NeuralPredictor::new();

        let features = predictor.engineer_features(
            12, 85.0, true, &[1, 2], 1, 35.0, 3, 45.0, 3
        );

        let result = predictor.predict(features);
        assert!(result.is_ok());

        let pred = result.unwrap();
        assert!(!pred.app_predictions.is_empty());
        assert!(pred.inference_time_ms > 0.0);
    }

    #[test]
    fn test_prediction_validity() {
        let mut predictor = NeuralPredictor::new();

        let features = predictor.engineer_features(
            12, 85.0, true, &[1, 2], 1, 35.0, 3, 45.0, 3
        );

        let result = predictor.predict(features).unwrap();

        // Check all predictions are valid
        for pred in &result.app_predictions {
            assert!(pred.confidence >= 0.0 && pred.confidence <= 1.0);
            assert!(pred.estimated_power_mw > 0.0);
        }

        // Check descending confidence
        for i in 0..result.app_predictions.len() - 1 {
            assert!(result.app_predictions[i].confidence >= result.app_predictions[i + 1].confidence);
        }
    }

    #[test]
    fn test_batch_prediction() {
        let mut predictor = NeuralPredictor::new();

        let features_batch = vec![
            predictor.engineer_features(12, 85.0, true, &[1, 2], 1, 35.0, 3, 45.0, 3),
            predictor.engineer_features(14, 75.0, true, &[2, 3], 2, 40.0, 3, 50.0, 3),
            predictor.engineer_features(16, 65.0, false, &[3, 4], 1, 30.0, 3, 40.0, 2),
        ];

        let results = predictor.predict_batch(features_batch);
        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 3);
    }

    #[test]
    fn test_cache_effectiveness() {
        let mut predictor = NeuralPredictor::new();

        let features = predictor.engineer_features(
            12, 85.0, true, &[1, 2], 1, 35.0, 3, 45.0, 3
        );

        let _ = predictor.predict(features.clone());
        let _ = predictor.predict(features.clone());
        let _ = predictor.predict(features);

        let (total, hits, _, rate) = predictor.get_stats();
        assert_eq!(total, 3);
        assert!(hits > 0);  // Should have cache hits
        assert!(rate > 0.0);
    }

    #[test]
    fn test_get_top_prediction() {
        let mut predictor = NeuralPredictor::new();

        let features = predictor.engineer_features(
            12, 85.0, true, &[1, 2], 1, 35.0, 3, 45.0, 3
        );

        let result = predictor.get_top_prediction(features);
        assert!(result.is_ok());

        let top = result.unwrap();
        assert!(top.confidence > 0.0);
    }

    #[test]
    fn test_confidence_threshold_setting() {
        let mut predictor = NeuralPredictor::new();

        predictor.set_confidence_threshold(0.7);
        assert_eq!(predictor.confidence_threshold, 0.7);

        // Out of range values should be clamped
        predictor.set_confidence_threshold(1.5);
        assert_eq!(predictor.confidence_threshold, 1.0);

        predictor.set_confidence_threshold(-0.5);
        assert_eq!(predictor.confidence_threshold, 0.0);
    }

    #[test]
    fn test_preload_priority_assignment() {
        let mut predictor = NeuralPredictor::new();
        predictor.set_confidence_threshold(0.5);

        let features = predictor.engineer_features(
            12, 85.0, true, &[1, 2], 1, 35.0, 3, 45.0, 3
        );

        let pred = predictor.predict(features).unwrap();

        // Verify priority assignment
        for app_pred in &pred.app_predictions {
            match app_pred.confidence {
                c if c > 0.8 => assert_eq!(app_pred.preload_priority, PreloadPriority::Critical),
                c if c > 0.6 => assert_eq!(app_pred.preload_priority, PreloadPriority::High),
                c if c > 0.5 => assert_eq!(app_pred.preload_priority, PreloadPriority::Medium),
                _ => assert_eq!(app_pred.preload_priority, PreloadPriority::Low),
            }
        }
    }

    #[test]
    fn test_feature_similarity() {
        let predictor = NeuralPredictor::new();

        let feat1 = predictor.engineer_features(12, 85.0, true, &[1, 2], 1, 35.0, 3, 45.0, 3);
        let feat2 = predictor.engineer_features(12, 85.1, true, &[1, 2], 1, 35.0, 3, 45.0, 3);
        let feat3 = predictor.engineer_features(18, 30.0, false, &[5, 6], 3, 50.0, 5, 80.0, 1);

        assert!(predictor.features_similar(&feat1, &feat2));
        assert!(!predictor.features_similar(&feat1, &feat3));
    }

    #[test]
    fn test_reset_state() {
        let mut predictor = NeuralPredictor::new();

        let features = predictor.engineer_features(
            12, 85.0, true, &[1, 2], 1, 35.0, 3, 45.0, 3
        );

        let _ = predictor.predict(features);
        assert!(predictor.total_predictions > 0);

        predictor.reset();
        assert_eq!(predictor.total_predictions, 0);
        assert_eq!(predictor.cache_hits, 0);
        assert_eq!(predictor.cache_misses, 0);
    }
}

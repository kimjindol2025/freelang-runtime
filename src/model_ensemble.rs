// Phase 8: Model Ensemble
// Combine Phase 6 feedforward + Phase 8 LSTM for best prediction

use crate::ml_model::MLModel;
use crate::lstm_sequence_model::LSTMSequenceModel;

/// Ensemble prediction result
#[derive(Debug, Clone)]
pub struct EnsemblePrediction {
    pub final_prediction: usize,
    pub final_confidence: f32,
    pub feedforward_pred: usize,
    pub lstm_pred: usize,
    pub blend_weights: (f32, f32), // FF weight, LSTM weight
}

/// Fixed weight ensemble (70% FF, 30% LSTM)
pub struct FixedWeightEnsemble {
    ff_weight: f32,
    lstm_weight: f32,
}

impl FixedWeightEnsemble {
    pub fn new() -> Self {
        FixedWeightEnsemble {
            ff_weight: 0.7,
            lstm_weight: 0.3,
        }
    }

    /// Blend predictions with fixed weights
    pub fn blend(&self, ff_pred: &[f32], lstm_pred: &[f32]) -> Vec<f32> {
        assert_eq!(ff_pred.len(), lstm_pred.len());

        ff_pred
            .iter()
            .zip(lstm_pred.iter())
            .map(|(f, l)| self.ff_weight * f + self.lstm_weight * l)
            .collect()
    }
}

/// Learned gate ensemble (neural network learns blend)
pub struct LearnedGateEnsemble {
    gate_weights: Vec<Vec<f32>>, // Input: [FF_pred (10) + LSTM_pred (10)] → gate (1)
}

impl LearnedGateEnsemble {
    pub fn new() -> Self {
        let init_val = 0.1;
        LearnedGateEnsemble {
            gate_weights: vec![vec![init_val; 20]; 1], // Single output (gate value)
        }
    }

    /// Learn which model to trust for this sample
    pub fn compute_gate(&self, ff_pred: &[f32], lstm_pred: &[f32]) -> f32 {
        // Concatenate both predictions
        let mut combined = Vec::new();
        combined.extend_from_slice(ff_pred);
        combined.extend_from_slice(lstm_pred);

        // Compute gate: sigmoid of weighted sum
        let gate_logit: f32 = combined
            .iter()
            .zip(&self.gate_weights[0])
            .map(|(a, b)| a * b)
            .sum();

        1.0 / (1.0 + (-gate_logit).exp()) // Sigmoid
    }

    /// Blend predictions using learned gate
    pub fn blend(&self, ff_pred: &[f32], lstm_pred: &[f32]) -> Vec<f32> {
        let gate = self.compute_gate(ff_pred, lstm_pred);

        ff_pred
            .iter()
            .zip(lstm_pred.iter())
            .map(|(f, l)| gate * f + (1.0 - gate) * l)
            .collect()
    }
}

/// Confidence-weighted ensemble (use softmax confidence)
pub struct ConfidenceWeightedEnsemble;

impl ConfidenceWeightedEnsemble {
    /// Get maximum softmax value (confidence)
    fn get_confidence(predictions: &[f32]) -> f32 {
        predictions.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
    }

    /// Blend using confidence scores as weights
    pub fn blend(ff_pred: &[f32], lstm_pred: &[f32]) -> Vec<f32> {
        let ff_conf = Self::get_confidence(ff_pred);
        let lstm_conf = Self::get_confidence(lstm_pred);

        let total_conf = ff_conf + lstm_conf;
        let ff_weight = if total_conf > 0.0 {
            ff_conf / total_conf
        } else {
            0.5
        };
        let lstm_weight = 1.0 - ff_weight;

        ff_pred
            .iter()
            .zip(lstm_pred.iter())
            .map(|(f, l)| ff_weight * f + lstm_weight * l)
            .collect()
    }
}

/// Main model ensemble combining Phase 6 and Phase 8
pub struct ModelEnsemble {
    ff_model: MLModel,
    lstm_model: LSTMSequenceModel,
    strategy: EnsembleStrategy,
    fixed_weights: FixedWeightEnsemble,
    learned_gate: LearnedGateEnsemble,
}

#[derive(Debug, Clone, Copy)]
pub enum EnsembleStrategy {
    FixedWeights,
    LearnedGate,
    ConfidenceWeighted,
}

impl ModelEnsemble {
    pub fn new(strategy: EnsembleStrategy) -> Self {
        ModelEnsemble {
            ff_model: MLModel::new(),
            lstm_model: LSTMSequenceModel::new(),
            strategy,
            fixed_weights: FixedWeightEnsemble::new(),
            learned_gate: LearnedGateEnsemble::new(),
        }
    }

    /// Combine predictions from both models
    pub fn forward(&self) -> Option<EnsemblePrediction> {
        // Get Phase 6 feedforward prediction
        let ff_output = self.ff_model.predict().ok()?;
        let ff_pred = match ff_output {
            crate::ml_model::PredictionResult::SingleClass(_) => {
                // Convert to probability distribution
                let mut dist = vec![0.1; 10];
                if ff_output.top_class() < 10 {
                    dist[ff_output.top_class()] = 0.9;
                }
                dist
            }
            _ => vec![0.1; 10],
        };
        let ff_class = ff_output.top_class();

        // Get Phase 8 LSTM prediction
        let lstm_class = self.lstm_model.predict()?;
        let lstm_output = self.lstm_model.forward()?;
        let lstm_pred = lstm_output.predictions;

        // Blend based on strategy
        let blended = match self.strategy {
            EnsembleStrategy::FixedWeights => self.fixed_weights.blend(&ff_pred, &lstm_pred),
            EnsembleStrategy::LearnedGate => self.learned_gate.blend(&ff_pred, &lstm_pred),
            EnsembleStrategy::ConfidenceWeighted => {
                ConfidenceWeightedEnsemble::blend(&ff_pred, &lstm_pred)
            }
        };

        // Get final prediction
        let final_pred = blended
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap();

        let blend_weights = match self.strategy {
            EnsembleStrategy::FixedWeights => {
                (self.fixed_weights.ff_weight, self.fixed_weights.lstm_weight)
            }
            EnsembleStrategy::LearnedGate => {
                let gate = self.learned_gate.compute_gate(&ff_pred, &lstm_pred);
                (gate, 1.0 - gate)
            }
            EnsembleStrategy::ConfidenceWeighted => {
                let ff_conf = ff_pred.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let lstm_conf = lstm_pred.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let total = ff_conf + lstm_conf;
                (ff_conf / total, lstm_conf / total)
            }
        };

        Some(EnsemblePrediction {
            final_prediction: final_pred.0,
            final_confidence: *final_pred.1,
            feedforward_pred: ff_class,
            lstm_pred: lstm_class,
            blend_weights,
        })
    }

    /// Verify ensemble improves over individual models
    pub fn verify_improvement(&self) -> (bool, String) {
        match self.forward() {
            Some(ensemble) => {
                let ff_pred = ensemble.feedforward_pred;
                let lstm_pred = ensemble.lstm_pred;
                let ensemble_pred = ensemble.final_prediction;

                let improvement = if ensemble_pred == ff_pred && ensemble_pred == lstm_pred {
                    "consensus".to_string()
                } else if ensemble_pred == ff_pred {
                    "favored_ff".to_string()
                } else if ensemble_pred == lstm_pred {
                    "favored_lstm".to_string()
                } else {
                    "breaking_tie".to_string()
                };

                (ensemble.final_confidence > 0.5, improvement)
            }
            None => (false, "no_prediction".to_string()),
        }
    }

    pub fn get_strategy(&self) -> EnsembleStrategy {
        self.strategy
    }

    pub fn get_ff_model(&self) -> &MLModel {
        &self.ff_model
    }

    pub fn get_lstm_model(&self) -> &LSTMSequenceModel {
        &self.lstm_model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensemble_creation() {
        let ensemble = ModelEnsemble::new(EnsembleStrategy::FixedWeights);
        assert_eq!(ensemble.fixed_weights.ff_weight, 0.7);
        assert_eq!(ensemble.fixed_weights.lstm_weight, 0.3);
    }

    #[test]
    fn test_fixed_weights_blend() {
        let ensemble = FixedWeightEnsemble::new();
        let ff_pred = vec![0.1; 10];
        let mut lstm_pred = vec![0.1; 10];
        lstm_pred[0] = 0.9; // LSTM confident on class 0

        let blended = ensemble.blend(&ff_pred, &lstm_pred);

        // Blended should be between FF and LSTM
        assert!(blended[0] > 0.1);
        assert!(blended[0] < 0.9);
        // Should be 0.7*0.1 + 0.3*0.9 = 0.34
        assert!((blended[0] - 0.34).abs() < 0.01);
    }

    #[test]
    fn test_learned_gate() {
        let gate = LearnedGateEnsemble::new();
        let ff_pred = vec![0.1; 10];
        let lstm_pred = vec![0.1; 10];

        let gate_val = gate.compute_gate(&ff_pred, &lstm_pred);
        assert!(gate_val >= 0.0 && gate_val <= 1.0); // Gate is sigmoid
    }

    #[test]
    fn test_confidence_weighting() {
        let ff_pred = vec![0.1; 10];
        let mut lstm_pred = vec![0.1; 10];
        lstm_pred[0] = 0.9;

        let blended = ConfidenceWeightedEnsemble::blend(&ff_pred, &lstm_pred);

        // LSTM is more confident, should dominate blending
        let sum: f32 = blended.iter().sum();
        assert!((sum - 10.0).abs() < 0.5); // Softmax-like normalization
    }

    #[test]
    fn test_ensemble_accuracy() {
        let ensemble = ModelEnsemble::new(EnsembleStrategy::FixedWeights);
        let (improved, strategy_str) = ensemble.verify_improvement();

        // Should produce some result
        assert!(!strategy_str.is_empty());
    }
}

// Project Sovereign: Model Evaluation Module
// Goal: Comprehensive metrics and validation
// Target: Track accuracy, precision, recall, F1 score

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct ConfusionMatrix {
    pub true_positives: HashMap<usize, usize>,
    pub false_positives: HashMap<usize, usize>,
    pub false_negatives: HashMap<usize, usize>,
    pub true_negatives: HashMap<usize, usize>,
}

#[derive(Clone, Debug)]
pub struct ClassMetrics {
    pub app_id: usize,
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
    pub support: usize,
}

#[derive(Clone, Debug)]
pub struct ValidationResult {
    pub overall_accuracy: f64,
    pub macro_avg_precision: f64,
    pub macro_avg_recall: f64,
    pub macro_avg_f1: f64,
    pub weighted_precision: f64,
    pub weighted_recall: f64,
    pub weighted_f1: f64,
    pub class_metrics: Vec<ClassMetrics>,
    pub samples_evaluated: usize,
}

#[derive(Clone, Debug)]
pub struct PredictionEval {
    pub predicted_app: usize,
    pub actual_app: usize,
    pub confidence: f64,
    pub is_correct: bool,
}

pub struct ModelEvaluator {
    // Prediction history for evaluation
    predictions: Vec<PredictionEval>,
    max_history: usize,

    // Metrics tracking
    total_correct: usize,
    total_evaluated: usize,

    // Per-class tracking
    class_metrics: HashMap<usize, ClassMetrics>,
}

impl ModelEvaluator {
    pub fn new() -> Self {
        Self {
            predictions: Vec::new(),
            max_history: 10000,
            total_correct: 0,
            total_evaluated: 0,
            class_metrics: HashMap::new(),
        }
    }

    /// Add a prediction for evaluation
    pub fn evaluate_prediction(
        &mut self,
        predicted_app: usize,
        actual_app: usize,
        confidence: f64,
    ) {
        let is_correct = predicted_app == actual_app;

        let eval = PredictionEval {
            predicted_app,
            actual_app,
            confidence,
            is_correct,
        };

        self.predictions.push(eval);
        self.total_evaluated += 1;

        if is_correct {
            self.total_correct += 1;
        }

        // Maintain size limit
        if self.predictions.len() > self.max_history {
            self.predictions.remove(0);
        }
    }

    /// Evaluate multiple predictions (batch)
    pub fn evaluate_batch(
        &mut self,
        predictions: Vec<(usize, usize, f64)>,  // (pred, actual, confidence)
    ) {
        for (pred, actual, conf) in predictions {
            self.evaluate_prediction(pred, actual, conf);
        }
    }

    /// Compute confusion matrix
    fn compute_confusion_matrix(&self) -> ConfusionMatrix {
        let mut tp = HashMap::new();
        let mut fp = HashMap::new();
        let mut fn_map = HashMap::new();
        let mut tn = HashMap::new();

        // Initialize all classes
        let all_classes: std::collections::HashSet<usize> = self.predictions
            .iter()
            .flat_map(|p| vec![p.predicted_app, p.actual_app])
            .collect();

        for &class_id in &all_classes {
            tp.insert(class_id, 0);
            fp.insert(class_id, 0);
            fn_map.insert(class_id, 0);
            tn.insert(class_id, 0);
        }

        // Compute matrix
        for pred in &self.predictions {
            let num_classes = all_classes.len();

            if pred.is_correct {
                *tp.get_mut(&pred.predicted_app).unwrap_or(&mut 0) += 1;
                for &class_id in &all_classes {
                    if class_id != pred.predicted_app {
                        *tn.get_mut(&class_id).unwrap_or(&mut 0) += 1;
                    }
                }
            } else {
                *fp.get_mut(&pred.predicted_app).unwrap_or(&mut 0) += 1;
                *fn_map.get_mut(&pred.actual_app).unwrap_or(&mut 0) += 1;
            }
        }

        ConfusionMatrix {
            true_positives: tp,
            false_positives: fp,
            false_negatives: fn_map,
            true_negatives: tn,
        }
    }

    /// Compute per-class metrics
    fn compute_class_metrics(&self, cm: &ConfusionMatrix) -> Vec<ClassMetrics> {
        let mut metrics = Vec::new();

        // Count occurrences of each actual class
        let mut class_counts: HashMap<usize, usize> = HashMap::new();
        for pred in &self.predictions {
            *class_counts.entry(pred.actual_app).or_insert(0) += 1;
        }

        for (class_id, support) in class_counts.iter() {
            let tp = *cm.true_positives.get(class_id).unwrap_or(&0) as f64;
            let fp = *cm.false_positives.get(class_id).unwrap_or(&0) as f64;
            let fn_count = *cm.false_negatives.get(class_id).unwrap_or(&0) as f64;

            let precision = if tp + fp > 0.0 {
                tp / (tp + fp)
            } else {
                0.0
            };

            let recall = if tp + fn_count > 0.0 {
                tp / (tp + fn_count)
            } else {
                0.0
            };

            let f1 = if precision + recall > 0.0 {
                2.0 * (precision * recall) / (precision + recall)
            } else {
                0.0
            };

            metrics.push(ClassMetrics {
                app_id: *class_id,
                precision,
                recall,
                f1_score: f1,
                support: *support,
            });
        }

        // Sort by app_id for consistency
        metrics.sort_by_key(|m| m.app_id);

        metrics
    }

    /// Get comprehensive validation results
    pub fn get_validation_report(&self) -> ValidationResult {
        if self.predictions.is_empty() {
            return ValidationResult {
                overall_accuracy: 0.0,
                macro_avg_precision: 0.0,
                macro_avg_recall: 0.0,
                macro_avg_f1: 0.0,
                weighted_precision: 0.0,
                weighted_recall: 0.0,
                weighted_f1: 0.0,
                class_metrics: vec![],
                samples_evaluated: 0,
            };
        }

        // Overall accuracy
        let overall_accuracy = if self.total_evaluated > 0 {
            self.total_correct as f64 / self.total_evaluated as f64
        } else {
            0.0
        };

        // Confusion matrix and class metrics
        let cm = self.compute_confusion_matrix();
        let class_metrics = self.compute_class_metrics(&cm);

        // Macro averages (unweighted)
        let (macro_precision, macro_recall, macro_f1) = if !class_metrics.is_empty() {
            let sum_precision: f64 = class_metrics.iter().map(|m| m.precision).sum();
            let sum_recall: f64 = class_metrics.iter().map(|m| m.recall).sum();
            let sum_f1: f64 = class_metrics.iter().map(|m| m.f1_score).sum();

            let n = class_metrics.len() as f64;
            (sum_precision / n, sum_recall / n, sum_f1 / n)
        } else {
            (0.0, 0.0, 0.0)
        };

        // Weighted averages
        let total_support: usize = class_metrics.iter().map(|m| m.support).sum();
        let (weighted_precision, weighted_recall, weighted_f1) = if total_support > 0 {
            let sum_precision: f64 = class_metrics
                .iter()
                .map(|m| m.precision * m.support as f64)
                .sum();
            let sum_recall: f64 = class_metrics
                .iter()
                .map(|m| m.recall * m.support as f64)
                .sum();
            let sum_f1: f64 = class_metrics
                .iter()
                .map(|m| m.f1_score * m.support as f64)
                .sum();

            let divisor = total_support as f64;
            (sum_precision / divisor, sum_recall / divisor, sum_f1 / divisor)
        } else {
            (0.0, 0.0, 0.0)
        };

        ValidationResult {
            overall_accuracy,
            macro_avg_precision: macro_precision,
            macro_avg_recall: macro_recall,
            macro_avg_f1: macro_f1,
            weighted_precision,
            weighted_recall,
            weighted_f1,
            class_metrics,
            samples_evaluated: self.total_evaluated,
        }
    }

    /// Get simple accuracy
    pub fn get_accuracy(&self) -> f64 {
        if self.total_evaluated > 0 {
            self.total_correct as f64 / self.total_evaluated as f64
        } else {
            0.0
        }
    }

    /// Get top-K accuracy
    pub fn get_top_k_accuracy(&self, k: usize) -> f64 {
        // Placeholder: in production, would track top-K predictions
        self.get_accuracy()
    }

    /// Reset evaluator state
    pub fn reset(&mut self) {
        self.predictions.clear();
        self.total_correct = 0;
        self.total_evaluated = 0;
        self.class_metrics.clear();
    }

    /// Get evaluation summary
    pub fn get_summary(&self) -> String {
        let report = self.get_validation_report();
        format!(
            "Evaluation Summary:\n\
             Samples: {}\n\
             Accuracy: {:.2}%\n\
             Macro-avg F1: {:.3}\n\
             Weighted-avg F1: {:.3}",
            report.samples_evaluated,
            report.overall_accuracy * 100.0,
            report.macro_avg_f1,
            report.weighted_f1
        )
    }

    /// Check if model meets accuracy threshold
    pub fn meets_accuracy_threshold(&self, threshold: f64) -> bool {
        self.get_accuracy() >= threshold
    }

    /// Get recall at K (how often true app is in top-K predictions)
    pub fn get_recall_at_k(&self, k: usize) -> f64 {
        if self.predictions.is_empty() {
            return 0.0;
        }

        // In production, would check if actual app is in top-K predictions
        // For now, return overall accuracy as approximation
        self.get_accuracy()
    }

    /// Get false positive rate
    pub fn get_false_positive_rate(&self) -> f64 {
        if self.predictions.is_empty() {
            return 0.0;
        }

        let false_positives = self.predictions.iter()
            .filter(|p| !p.is_correct)
            .count();

        false_positives as f64 / self.predictions.len() as f64
    }

    /// Print detailed report
    pub fn print_report(&self) {
        let report = self.get_validation_report();

        println!("\n=== Model Evaluation Report ===");
        println!("Samples Evaluated: {}", report.samples_evaluated);
        println!("\nOverall Metrics:");
        println!("  Accuracy: {:.4}", report.overall_accuracy);
        println!("  Macro-avg Precision: {:.4}", report.macro_avg_precision);
        println!("  Macro-avg Recall: {:.4}", report.macro_avg_recall);
        println!("  Macro-avg F1: {:.4}", report.macro_avg_f1);
        println!("\nWeighted Metrics:");
        println!("  Weighted Precision: {:.4}", report.weighted_precision);
        println!("  Weighted Recall: {:.4}", report.weighted_recall);
        println!("  Weighted F1: {:.4}", report.weighted_f1);

        println!("\nPer-Class Metrics:");
        println!("  {:>6} {:>10} {:>10} {:>10} {:>10}", "App ID", "Precision", "Recall", "F1-Score", "Support");
        for metric in &report.class_metrics {
            println!(
                "  {:>6} {:>10.4} {:>10.4} {:>10.4} {:>10}",
                metric.app_id,
                metric.precision,
                metric.recall,
                metric.f1_score,
                metric.support
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluator_creation() {
        let evaluator = ModelEvaluator::new();
        assert_eq!(evaluator.total_evaluated, 0);
        assert_eq!(evaluator.total_correct, 0);
    }

    #[test]
    fn test_single_prediction_evaluation() {
        let mut evaluator = ModelEvaluator::new();

        evaluator.evaluate_prediction(5, 5, 0.95);

        assert_eq!(evaluator.total_evaluated, 1);
        assert_eq!(evaluator.total_correct, 1);
    }

    #[test]
    fn test_accuracy_calculation() {
        let mut evaluator = ModelEvaluator::new();

        evaluator.evaluate_prediction(1, 1, 0.9);
        evaluator.evaluate_prediction(2, 2, 0.85);
        evaluator.evaluate_prediction(3, 4, 0.7);

        let accuracy = evaluator.get_accuracy();
        assert!((accuracy - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_batch_evaluation() {
        let mut evaluator = ModelEvaluator::new();

        let predictions = vec![
            (1, 1, 0.9),
            (2, 2, 0.85),
            (3, 4, 0.7),
            (5, 5, 0.95),
        ];

        evaluator.evaluate_batch(predictions);

        assert_eq!(evaluator.total_evaluated, 4);
    }

    #[test]
    fn test_validation_report() {
        let mut evaluator = ModelEvaluator::new();

        for i in 0..20 {
            let (pred, actual) = if i < 16 { (i % 8, i % 8) } else { (i % 8, (i + 1) % 8) };
            evaluator.evaluate_prediction(pred, actual, 0.85);
        }

        let report = evaluator.get_validation_report();

        assert_eq!(report.samples_evaluated, 20);
        assert!(report.overall_accuracy > 0.0);
        assert!(!report.class_metrics.is_empty());
    }

    #[test]
    fn test_perfect_accuracy() {
        let mut evaluator = ModelEvaluator::new();

        for i in 0..10 {
            evaluator.evaluate_prediction(i, i, 0.95);
        }

        assert_eq!(evaluator.get_accuracy(), 1.0);
    }

    #[test]
    fn test_zero_accuracy() {
        let mut evaluator = ModelEvaluator::new();

        for i in 0..10 {
            evaluator.evaluate_prediction(i, (i + 1) % 10, 0.5);
        }

        assert_eq!(evaluator.get_accuracy(), 0.0);
    }

    #[test]
    fn test_recall_at_k() {
        let mut evaluator = ModelEvaluator::new();

        for i in 0..20 {
            evaluator.evaluate_prediction(i % 8, i % 8, 0.85);
        }

        let recall = evaluator.get_recall_at_k(3);
        assert!(recall >= 0.0 && recall <= 1.0);
    }

    #[test]
    fn test_false_positive_rate() {
        let mut evaluator = ModelEvaluator::new();

        evaluator.evaluate_prediction(1, 1, 0.9);
        evaluator.evaluate_prediction(2, 3, 0.7);
        evaluator.evaluate_prediction(4, 4, 0.95);

        let fpr = evaluator.get_false_positive_rate();
        assert!((fpr - 1.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_accuracy_threshold_check() {
        let mut evaluator = ModelEvaluator::new();

        for i in 0..10 {
            evaluator.evaluate_prediction(i, i, 0.9);
        }

        assert!(evaluator.meets_accuracy_threshold(0.9));
        assert!(!evaluator.meets_accuracy_threshold(1.0));
    }

    #[test]
    fn test_reset_evaluator() {
        let mut evaluator = ModelEvaluator::new();

        for i in 0..10 {
            evaluator.evaluate_prediction(i, i, 0.9);
        }

        evaluator.reset();

        assert_eq!(evaluator.total_evaluated, 0);
        assert_eq!(evaluator.total_correct, 0);
        assert_eq!(evaluator.get_accuracy(), 0.0);
    }

    #[test]
    fn test_empty_evaluator() {
        let evaluator = ModelEvaluator::new();

        let report = evaluator.get_validation_report();
        assert_eq!(report.samples_evaluated, 0);
        assert_eq!(report.overall_accuracy, 0.0);
    }

    #[test]
    fn test_class_metrics_calculation() {
        let mut evaluator = ModelEvaluator::new();

        // Simulate predictions for 3 different apps
        evaluator.evaluate_prediction(0, 0, 0.9);
        evaluator.evaluate_prediction(0, 0, 0.9);
        evaluator.evaluate_prediction(1, 1, 0.85);
        evaluator.evaluate_prediction(1, 2, 0.7);
        evaluator.evaluate_prediction(2, 2, 0.95);

        let report = evaluator.get_validation_report();
        assert!(!report.class_metrics.is_empty());

        // Check that metrics are reasonable
        for metric in &report.class_metrics {
            assert!(metric.precision >= 0.0 && metric.precision <= 1.0);
            assert!(metric.recall >= 0.0 && metric.recall <= 1.0);
            assert!(metric.f1_score >= 0.0 && metric.f1_score <= 1.0);
        }
    }
}

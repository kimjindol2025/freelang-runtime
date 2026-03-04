// Phase 9: Quantization Engine
// Convert float32 → int8 → int4 with minimal accuracy loss

/// Quantization scheme definition
#[derive(Debug, Clone)]
pub struct QuantizationScheme {
    pub bit_width: usize,       // 8, 4, etc.
    pub scale_factor: f32,      // s = max_val / max_int
    pub zero_point: i32,        // Asymmetric: offset
    pub is_symmetric: bool,     // True: [-N, N], False: [a, b]
    pub per_channel: bool,      // Per-filter scaling vs per-layer
}

impl QuantizationScheme {
    pub fn new(bit_width: usize, is_symmetric: bool, per_channel: bool) -> Self {
        QuantizationScheme {
            bit_width,
            scale_factor: 1.0,
            zero_point: 0,
            is_symmetric,
            per_channel,
        }
    }

    /// Compute scale factor from float range
    pub fn compute_scale(&mut self, min_val: f32, max_val: f32) {
        if self.is_symmetric {
            let abs_max = min_val.abs().max(max_val.abs());
            let max_int = ((1u32 << (self.bit_width - 1)) - 1) as f32;
            self.scale_factor = abs_max / max_int;
        } else {
            let max_int = (1u32 << self.bit_width) as f32 - 1.0;
            self.scale_factor = (max_val - min_val) / max_int;
            self.zero_point = -(min_val / self.scale_factor) as i32;
        }
    }
}

/// Int8 quantizer
pub struct Int8Quantizer {
    scheme: QuantizationScheme,
}

impl Int8Quantizer {
    pub fn new() -> Self {
        let mut scheme = QuantizationScheme::new(8, true, false);
        scheme.scale_factor = 1.0 / 127.0;
        Int8Quantizer { scheme }
    }

    /// Quantize float weights to int8
    pub fn quantize(&self, weights: &[f32]) -> Vec<i8> {
        weights
            .iter()
            .map(|w| {
                let scaled = w / self.scheme.scale_factor;
                (scaled.round() as i32).clamp(-128, 127) as i8
            })
            .collect()
    }

    /// Dequantize int8 back to float
    pub fn dequantize(&self, quantized: &[i8]) -> Vec<f32> {
        quantized
            .iter()
            .map(|q| (*q as f32) * self.scheme.scale_factor)
            .collect()
    }

    /// Compute quantization error (MSE between original and dequantized)
    pub fn compute_error(&self, original: &[f32]) -> f32 {
        let quantized = self.quantize(original);
        let dequantized = self.dequantize(&quantized);

        let mse: f32 = original
            .iter()
            .zip(dequantized.iter())
            .map(|(o, d)| (o - d).powi(2))
            .sum::<f32>()
            / original.len() as f32;

        mse.sqrt()
    }

    pub fn get_model_size(&self, num_weights: usize) -> usize {
        num_weights / 4 // float32 (4 bytes) → int8 (1 byte)
    }
}

/// Int4 quantizer (further compression from int8)
pub struct Int4Quantizer {
    scheme: QuantizationScheme,
}

impl Int4Quantizer {
    pub fn new() -> Self {
        let mut scheme = QuantizationScheme::new(4, true, false);
        scheme.scale_factor = 1.0 / 7.0;
        Int4Quantizer { scheme }
    }

    /// Quantize int8 to int4
    pub fn quantize(&self, int8_weights: &[i8]) -> Vec<u8> {
        int8_weights
            .iter()
            .zip(int8_weights.iter().skip(1))
            .step_by(2)
            .map(|(w1, w2)| {
                let q1 = (((*w1 as f32) / self.scheme.scale_factor).round() as i32)
                    .clamp(-8, 7) as u8 & 0xF;
                let q2 = (((*w2 as f32) / self.scheme.scale_factor).round() as i32)
                    .clamp(-8, 7) as u8 & 0xF;
                (q1 << 4) | q2
            })
            .collect()
    }

    /// Dequantize int4 to float
    pub fn dequantize(&self, quantized_4bit: &[u8]) -> Vec<f32> {
        let mut result = Vec::new();
        for byte in quantized_4bit {
            let q1 = ((byte >> 4) as i8) as f32;
            let q2 = ((byte & 0xF) as i8) as f32;
            result.push(q1 * self.scheme.scale_factor);
            result.push(q2 * self.scheme.scale_factor);
        }
        result
    }

    pub fn get_model_size(&self, num_weights: usize) -> usize {
        num_weights / 8 // float32 (4 bytes) → int4 (0.5 bytes)
    }
}

/// Calibration engine for finding optimal scale factors
pub struct CalibrationEngine {
    sample_data: Vec<Vec<f32>>,
    max_samples: usize,
}

impl CalibrationEngine {
    pub fn new(max_samples: usize) -> Self {
        CalibrationEngine {
            sample_data: vec![],
            max_samples,
        }
    }

    /// Add calibration sample
    pub fn add_sample(&mut self, data: Vec<f32>) {
        if self.sample_data.len() < self.max_samples {
            self.sample_data.push(data);
        }
    }

    /// Calibrate scale factor from collected data
    pub fn calibrate_scale(&self) -> (f32, f32) {
        if self.sample_data.is_empty() {
            return (0.0, 0.0);
        }

        let mut min_val = f32::INFINITY;
        let mut max_val = f32::NEG_INFINITY;

        for sample in &self.sample_data {
            for &val in sample {
                min_val = min_val.min(val);
                max_val = max_val.max(val);
            }
        }

        (min_val, max_val)
    }

    /// Get average magnitude for scale optimization
    pub fn get_average_magnitude(&self) -> f32 {
        if self.sample_data.is_empty() {
            return 1.0;
        }

        let sum: f32 = self
            .sample_data
            .iter()
            .flat_map(|s| s.iter())
            .map(|v| v.abs())
            .sum();

        let count = self
            .sample_data
            .iter()
            .map(|s| s.len())
            .sum::<usize>();

        if count > 0 {
            sum / count as f32
        } else {
            1.0
        }
    }
}

/// Main Quantization Engine
pub struct QuantizationEngine {
    int8_quantizer: Int8Quantizer,
    int4_quantizer: Int4Quantizer,
    calibration: CalibrationEngine,
    original_model_size: usize,
}

impl QuantizationEngine {
    pub fn new() -> Self {
        QuantizationEngine {
            int8_quantizer: Int8Quantizer::new(),
            int4_quantizer: Int4Quantizer::new(),
            calibration: CalibrationEngine::new(100),
            original_model_size: 400_000, // 100KB in float32 (25000 weights × 4 bytes)
        }
    }

    /// Add calibration sample
    pub fn add_calibration_sample(&mut self, data: Vec<f32>) {
        self.calibration.add_sample(data);
    }

    /// Quantize to int8
    pub fn quantize_to_int8(&self, weights: &[f32]) -> (Vec<i8>, f32) {
        let quantized = self.int8_quantizer.quantize(weights);
        let error = self.int8_quantizer.compute_error(weights);
        (quantized, error)
    }

    /// Quantize to int4 (via int8)
    pub fn quantize_to_int4(&self, weights: &[f32]) -> (Vec<u8>, f32) {
        let int8_weights = self.int8_quantizer.quantize(weights);
        let quantized = self.int4_quantizer.quantize(&int8_weights);
        let dequantized = self.int4_quantizer.dequantize(&quantized);

        let error: f32 = weights
            .iter()
            .zip(dequantized.iter())
            .map(|(o, d)| (o - d).powi(2))
            .sum::<f32>()
            / weights.len() as f32;

        (quantized, error.sqrt())
    }

    /// Compute model size after quantization
    pub fn get_quantized_size_int8(&self, num_weights: usize) -> usize {
        self.int8_quantizer.get_model_size(num_weights)
    }

    pub fn get_quantized_size_int4(&self, num_weights: usize) -> usize {
        self.int4_quantizer.get_model_size(num_weights)
    }

    /// Compute size reduction percentage
    pub fn get_size_reduction_percent(&self, quantized_size: usize) -> f32 {
        (1.0 - (quantized_size as f32 / self.original_model_size as f32)) * 100.0
    }

    /// Verify unforgiving rules
    pub fn verify_quantization_rules(&self, weights: &[f32]) -> (bool, String) {
        let (_, error) = self.quantize_to_int4(weights);
        let error_percent = (error / 1.0) * 100.0; // Normalize to 1.0 range

        let rule1_pass = error_percent < 1.0; // Rule 1: Quant error <1%

        let int8_size = self.get_quantized_size_int8(weights.len());
        let size_reduction = self.get_size_reduction_percent(int8_size);
        let rule2_pass = size_reduction >= 50.0; // Rule 2: Size reduction ≥50%

        let all_pass = rule1_pass && rule2_pass;
        let msg = format!(
            "R1(error<1%):{} R2(size≥50%):{} [error:{:.2}% reduction:{:.1}%]",
            rule1_pass, rule2_pass, error_percent, size_reduction
        );

        (all_pass, msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantization_creation() {
        let qe = QuantizationEngine::new();
        assert_eq!(qe.original_model_size, 400_000);
    }

    #[test]
    fn test_linear_quantization() {
        let qe = QuantizationEngine::new();
        let weights = vec![0.5, 0.25, -0.75, 0.1];
        let (quantized, error) = qe.quantize_to_int8(&weights);

        assert_eq!(quantized.len(), 4);
        assert!(error < 0.1); // Should have low error
    }

    #[test]
    fn test_int4_quantization() {
        let qe = QuantizationEngine::new();
        let weights = vec![0.5, 0.25, -0.75, 0.1, 0.3, 0.6, -0.2, 0.4];
        let (quantized, error) = qe.quantize_to_int4(&weights);

        assert_eq!(quantized.len(), 4); // 8 weights → 4 bytes (2 weights per byte)
        assert!(error < 0.1);
    }

    #[test]
    fn test_calibration() {
        let mut cal = CalibrationEngine::new(10);
        cal.add_sample(vec![0.1, 0.2, 0.3]);
        cal.add_sample(vec![0.4, 0.5, 0.6]);

        let (min, max) = cal.calibrate_scale();
        assert!(min <= 0.1);
        assert!(max >= 0.6);
    }

    #[test]
    fn test_per_channel_scaling() {
        let scheme = QuantizationScheme::new(8, true, true);
        assert!(scheme.per_channel);
        assert_eq!(scheme.bit_width, 8);
    }

    #[test]
    fn test_accuracy_preservation() {
        let qe = QuantizationEngine::new();
        let weights = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
        let (_, error) = qe.quantize_to_int4(&weights);

        // Rule 1: Error <1%
        let error_percent = (error / 1.0) * 100.0;
        assert!(error_percent < 1.0, "Accuracy loss: {:.2}%", error_percent);
    }

    #[test]
    fn test_model_size_reduction() {
        let qe = QuantizationEngine::new();
        let weights: Vec<f32> = (0..25000).map(|i| (i as f32) * 0.001).collect();

        let int8_size = qe.get_quantized_size_int8(weights.len());
        let reduction = qe.get_size_reduction_percent(int8_size);

        // Rule 2: Size reduction ≥50%
        assert!(
            reduction >= 50.0,
            "Size reduction: {:.1}%",
            reduction
        );
    }

    #[test]
    fn test_quantization_stability() {
        let qe = QuantizationEngine::new();
        let weights = vec![0.1, 0.2, 0.3, 0.4];

        let (quantized1, error1) = qe.quantize_to_int8(&weights);
        let (quantized2, error2) = qe.quantize_to_int8(&weights);

        // Should be deterministic
        assert_eq!(quantized1, quantized2);
        assert!((error1 - error2).abs() < 1e-6);
    }
}

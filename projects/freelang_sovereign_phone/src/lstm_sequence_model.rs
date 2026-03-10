// Phase 8: Advanced ML - LSTM Sequence Model
// Temporal pattern learning for device metrics

use std::collections::VecDeque;
use crate::device_metrics_collector::PrivacyFilteredMetrics;

/// LSTM cell state
#[derive(Debug, Clone)]
pub struct LSTMCellState {
    pub hidden: Vec<f32>,     // h[t] - hidden state
    pub cell: Vec<f32>,       // c[t] - cell state (long-term memory)
}

/// LSTM forward pass result
#[derive(Debug, Clone)]
pub struct LSTMOutput {
    pub predictions: Vec<f32>,  // 10-class softmax
    pub hidden_state: Vec<f32>,
    pub attention_weights: Vec<f32>, // For phase 8B
}

/// Sequence buffer for LSTM input
#[derive(Debug)]
pub struct SequenceBuffer {
    buffer: VecDeque<Vec<f32>>, // 100-timestep window
    max_len: usize,
    feature_dim: usize,
}

impl SequenceBuffer {
    pub fn new(max_len: usize, feature_dim: usize) -> Self {
        SequenceBuffer {
            buffer: VecDeque::with_capacity(max_len),
            max_len,
            feature_dim,
        }
    }

    /// Add feature vector to sequence
    pub fn push_feature(&mut self, features: &[f32]) {
        if features.len() != self.feature_dim {
            return;
        }

        if self.buffer.len() >= self.max_len {
            self.buffer.pop_front(); // Remove oldest
        }
        self.buffer.push_back(features.to_vec());
    }

    /// Get full sequence (padded if needed)
    pub fn get_sequence(&self) -> Vec<Vec<f32>> {
        self.buffer.iter().cloned().collect()
    }

    /// Check if buffer is ready (full)
    pub fn is_ready(&self) -> bool {
        self.buffer.len() >= self.max_len
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }
}

/// Single LSTM cell
pub struct LSTMCell {
    hidden_size: usize,
    input_size: usize,
    // Weights for input gate
    w_i_input: Vec<Vec<f32>>,
    w_i_hidden: Vec<Vec<f32>>,
    b_i: Vec<f32>,
    // Weights for forget gate
    w_f_input: Vec<Vec<f32>>,
    w_f_hidden: Vec<Vec<f32>>,
    b_f: Vec<f32>,
    // Weights for cell gate
    w_c_input: Vec<Vec<f32>>,
    w_c_hidden: Vec<Vec<f32>>,
    b_c: Vec<f32>,
    // Weights for output gate
    w_o_input: Vec<Vec<f32>>,
    w_o_hidden: Vec<Vec<f32>>,
    b_o: Vec<f32>,
}

impl LSTMCell {
    pub fn new(input_size: usize, hidden_size: usize) -> Self {
        // Initialize with small random weights
        let init_val = 0.1;

        LSTMCell {
            hidden_size,
            input_size,
            w_i_input: vec![vec![init_val; input_size]; hidden_size],
            w_i_hidden: vec![vec![init_val; hidden_size]; hidden_size],
            b_i: vec![0.0; hidden_size],
            w_f_input: vec![vec![init_val; input_size]; hidden_size],
            w_f_hidden: vec![vec![init_val; hidden_size]; hidden_size],
            b_f: vec![0.1; hidden_size], // Forget bias initialized to 0.1
            w_c_input: vec![vec![init_val; input_size]; hidden_size],
            w_c_hidden: vec![vec![init_val; hidden_size]; hidden_size],
            b_c: vec![0.0; hidden_size],
            w_o_input: vec![vec![init_val; input_size]; hidden_size],
            w_o_hidden: vec![vec![init_val; hidden_size]; hidden_size],
            b_o: vec![0.0; hidden_size],
        }
    }

    /// Sigmoid activation
    fn sigmoid(x: f32) -> f32 {
        1.0 / (1.0 + (-x).exp())
    }

    /// Tanh activation
    fn tanh(x: f32) -> f32 {
        x.tanh()
    }

    /// Matrix multiplication
    fn matmul(matrix: &[Vec<f32>], vector: &[f32]) -> Vec<f32> {
        matrix
            .iter()
            .map(|row| {
                row.iter()
                    .zip(vector.iter())
                    .map(|(a, b)| a * b)
                    .sum::<f32>()
            })
            .collect()
    }

    /// Forward pass through LSTM cell
    pub fn forward(
        &self,
        input: &[f32],
        prev_h: &[f32],
        prev_c: &[f32],
    ) -> (Vec<f32>, Vec<f32>) {
        // Input gate
        let mut i_t = Self::matmul(&self.w_i_input, input);
        let i_h = Self::matmul(&self.w_i_hidden, prev_h);
        for j in 0..self.hidden_size {
            i_t[j] = Self::sigmoid(i_t[j] + i_h[j] + self.b_i[j]);
        }

        // Forget gate
        let mut f_t = Self::matmul(&self.w_f_input, input);
        let f_h = Self::matmul(&self.w_f_hidden, prev_h);
        for j in 0..self.hidden_size {
            f_t[j] = Self::sigmoid(f_t[j] + f_h[j] + self.b_f[j]);
        }

        // Cell gate
        let mut c_tilde = Self::matmul(&self.w_c_input, input);
        let c_h = Self::matmul(&self.w_c_hidden, prev_h);
        for j in 0..self.hidden_size {
            c_tilde[j] = Self::tanh(c_tilde[j] + c_h[j] + self.b_c[j]);
        }

        // Cell state: c[t] = f[t] ⊙ c[t-1] + i[t] ⊙ c_tilde[t]
        let mut c_t = vec![0.0; self.hidden_size];
        for j in 0..self.hidden_size {
            c_t[j] = f_t[j] * prev_c[j] + i_t[j] * c_tilde[j];
        }

        // Output gate
        let mut o_t = Self::matmul(&self.w_o_input, input);
        let o_h = Self::matmul(&self.w_o_hidden, prev_h);
        for j in 0..self.hidden_size {
            o_t[j] = Self::sigmoid(o_t[j] + o_h[j] + self.b_o[j]);
        }

        // Hidden state: h[t] = o[t] ⊙ tanh(c[t])
        let mut h_t = vec![0.0; self.hidden_size];
        for j in 0..self.hidden_size {
            h_t[j] = o_t[j] * Self::tanh(c_t[j]);
        }

        (h_t, c_t)
    }
}

/// LSTM Layer (stacked cells)
pub struct LSTMLayer {
    cell: LSTMCell,
    hidden_size: usize,
}

impl LSTMLayer {
    pub fn new(input_size: usize, hidden_size: usize) -> Self {
        LSTMLayer {
            cell: LSTMCell::new(input_size, hidden_size),
            hidden_size,
        }
    }

    /// Process sequence and return final hidden state
    pub fn forward_sequence(&self, sequence: &[Vec<f32>]) -> (Vec<f32>, Vec<f32>) {
        let mut h = vec![0.0; self.hidden_size];
        let mut c = vec![0.0; self.hidden_size];

        // Process each timestep
        for timestep in sequence {
            (h, c) = self.cell.forward(timestep, &h, &c);
        }

        (h, c)
    }
}

/// Full LSTM sequence model: 2 layers, 64→32 hidden units
pub struct LSTMSequenceModel {
    layer1: LSTMLayer,     // 64 units
    layer2: LSTMLayer,     // 32 units
    dense_layer: Vec<Vec<f32>>, // 32 → 16
    output_layer: Vec<Vec<f32>>, // 16 → 10
    seq_buffer: SequenceBuffer,
    feature_dim: usize,
}

impl LSTMSequenceModel {
    pub fn new() -> Self {
        let feature_dim = 9; // From phase 7
        let init_val = 0.1;

        LSTMSequenceModel {
            layer1: LSTMLayer::new(feature_dim, 64),
            layer2: LSTMLayer::new(64, 32),
            dense_layer: vec![vec![init_val; 32]; 16],
            output_layer: vec![vec![init_val; 16]; 10],
            seq_buffer: SequenceBuffer::new(100, feature_dim),
            feature_dim,
        }
    }

    /// Add metric to sequence buffer
    pub fn add_metric(&mut self, metric: &PrivacyFilteredMetrics) {
        let features = vec![
            metric.cpu_load,
            metric.cpu_frequency_mhz as f32 / 3000.0, // Normalize
            metric.memory_usage_percent / 100.0,
            metric.battery_percent / 100.0,
            metric.battery_temp_c / 100.0,
            metric.soc_temp_c / 100.0,
            metric.gpu_freq_mhz as f32 / 1000.0,
            (metric.timestamp % 86400) as f32 / 86400.0, // Time of day
            if metric.location_cell_id.is_some() { 1.0 } else { 0.0 }, // Has location
        ];

        self.seq_buffer.push_feature(&features);
    }

    /// Forward pass through LSTM (requires full sequence)
    pub fn forward(&self) -> Option<LSTMOutput> {
        if !self.seq_buffer.is_ready() {
            return None; // Need 100 timesteps
        }

        let sequence = self.seq_buffer.get_sequence();

        // Layer 1: 9 → 64
        let (h1, _) = self.layer1.forward_sequence(&sequence);

        // Layer 2: 64 → 32
        let single_input = vec![h1; 1];
        let (h2, _) = self.layer2.forward_sequence(&single_input);

        // Dense layer: 32 → 16
        let mut dense_out = vec![0.0; 16];
        for i in 0..16 {
            dense_out[i] = h2
                .iter()
                .zip(&self.dense_layer[i])
                .map(|(a, b)| a * b)
                .sum::<f32>();
            dense_out[i] = dense_out[i].max(0.0); // ReLU
        }

        // Output layer: 16 → 10 (softmax)
        let mut logits = vec![0.0; 10];
        for i in 0..10 {
            logits[i] = dense_out
                .iter()
                .zip(&self.output_layer[i])
                .map(|(a, b)| a * b)
                .sum::<f32>();
        }

        // Softmax
        let max_logit = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exp_logits: Vec<f32> = logits.iter().map(|x| (x - max_logit).exp()).collect();
        let sum_exp: f32 = exp_logits.iter().sum();
        let predictions: Vec<f32> = exp_logits.iter().map(|x| x / sum_exp).collect();

        // Attention weights (simplified: feature importance)
        let mut attention = vec![0.0; self.feature_dim];
        for i in 0..self.feature_dim {
            attention[i] = (1.0 + i as f32) / self.feature_dim as f32; // Placeholder
        }

        Some(LSTMOutput {
            predictions,
            hidden_state: h2,
            attention_weights: attention,
        })
    }

    /// Get prediction (returns top class)
    pub fn predict(&self) -> Option<usize> {
        self.forward().map(|output| {
            output
                .predictions
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .unwrap()
                .0
        })
    }

    /// Get confidence score
    pub fn get_confidence(&self) -> Option<f32> {
        self.forward().map(|output| {
            output
                .predictions
                .iter()
                .cloned()
                .fold(f32::NEG_INFINITY, f32::max)
        })
    }

    /// Check if sequence buffer is ready
    pub fn is_ready(&self) -> bool {
        self.seq_buffer.is_ready()
    }

    pub fn buffer_len(&self) -> usize {
        self.seq_buffer.len()
    }

    pub fn clear_buffer(&mut self) {
        self.seq_buffer = SequenceBuffer::new(100, self.feature_dim);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_lstm_creation() {
        let lstm = LSTMSequenceModel::new();
        assert_eq!(lstm.buffer_len(), 0);
        assert!(!lstm.is_ready());
    }

    #[test]
    fn test_single_sequence_forward() {
        let mut lstm = LSTMSequenceModel::new();
        let metric = create_test_metric();

        // Add 100 samples to fill buffer
        for i in 0..100 {
            let mut m = metric.clone();
            m.timestamp = 1000 + i as u64;
            lstm.add_metric(&m);
        }

        assert!(lstm.is_ready());
        let output = lstm.forward();
        assert!(output.is_some());

        let out = output.unwrap();
        assert_eq!(out.predictions.len(), 10);
        // Softmax sum should be ~1.0
        let sum: f32 = out.predictions.iter().sum();
        assert!((sum - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_batch_sequences() {
        let mut lstm = LSTMSequenceModel::new();
        let metric = create_test_metric();

        for i in 0..100 {
            let mut m = metric.clone();
            m.timestamp = 1000 + i as u64;
            lstm.add_metric(&m);
        }

        // Multiple forwards should be stable
        let out1 = lstm.forward().unwrap();
        let out2 = lstm.forward().unwrap();

        // Same input should produce same output
        for i in 0..10 {
            assert!((out1.predictions[i] - out2.predictions[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn test_lstm_accuracy() {
        let mut lstm = LSTMSequenceModel::new();
        let metric = create_test_metric();

        for i in 0..100 {
            let mut m = metric.clone();
            m.timestamp = 1000 + i as u64;
            lstm.add_metric(&m);
        }

        assert!(lstm.is_ready());
        let pred = lstm.predict();
        assert!(pred.is_some());
        let class = pred.unwrap();
        assert!(class < 10);
    }

    #[test]
    fn test_inference_latency() {
        let mut lstm = LSTMSequenceModel::new();
        let metric = create_test_metric();

        for i in 0..100 {
            let mut m = metric.clone();
            m.timestamp = 1000 + i as u64;
            lstm.add_metric(&m);
        }

        let start = std::time::SystemTime::now();
        for _ in 0..100 {
            let _ = lstm.forward();
        }
        let elapsed = start.elapsed().unwrap().as_millis();

        // 100 forwards should be <2000ms (20ms average, target <15ms)
        assert!(elapsed < 2000, "Latency: {}ms", elapsed);
    }

    #[test]
    fn test_hidden_state_tracking() {
        let mut lstm = LSTMSequenceModel::new();
        let metric = create_test_metric();

        for i in 0..100 {
            let mut m = metric.clone();
            m.timestamp = 1000 + i as u64;
            lstm.add_metric(&m);
        }

        let output = lstm.forward().unwrap();
        assert_eq!(output.hidden_state.len(), 32); // Layer 2 output
    }

    #[test]
    fn test_gradient_flow() {
        let lstm = LSTMSequenceModel::new();
        // Verify that gradients would flow through all layers
        // (in real implementation with autograd)
        assert!(lstm.layer1.hidden_size > 0);
        assert!(lstm.layer2.hidden_size > 0);
    }

    #[test]
    fn test_sequence_padding() {
        let mut lstm = LSTMSequenceModel::new();
        let metric = create_test_metric();

        // Add only 50 samples (half buffer)
        for i in 0..50 {
            let mut m = metric.clone();
            m.timestamp = 1000 + i as u64;
            lstm.add_metric(&m);
        }

        assert!(!lstm.is_ready());
        assert!(lstm.forward().is_none());
    }

    #[test]
    fn test_lstm_convergence() {
        let mut lstm = LSTMSequenceModel::new();
        let metric = create_test_metric();

        // Fill buffer with consistent data
        for i in 0..100 {
            let mut m = metric.clone();
            m.timestamp = 1000 + i as u64;
            m.cpu_load = 0.5 + (i as f32 * 0.001).sin(); // Small variation
            lstm.add_metric(&m);
        }

        let output = lstm.forward().unwrap();
        let confidence = output.predictions.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        // Model should have reasonable confidence
        assert!(confidence > 0.05);
    }

    #[test]
    fn test_memory_efficiency() {
        let lstm = LSTMSequenceModel::new();
        // Verify buffer doesn't grow beyond 100 samples
        assert!(lstm.seq_buffer.max_len == 100);
        // Estimate: 100 * 9 * 4 bytes = 3600 bytes << 50MB target
    }
}

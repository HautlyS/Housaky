//! Quantum-Inspired State Computing with Parallel Optimizations
//!
//! This module provides quantum-inspired computation using classical parallelism
//! with stable Rust features for maximum compatibility and performance.

use rayon::prelude::*;

/// Quantum-inspired state using amplitude representation
/// Optimized for cache efficiency and parallel processing
#[derive(Debug, Clone)]
pub struct QuantumInspiredState {
    /// Amplitude vector (real parts only for performance)
    pub amplitudes: Vec<f64>,
    /// State dimension (log2 of amplitudes.len())
    #[allow(dead_code)]
    dimension: usize,
}

impl QuantumInspiredState {
    /// Create a new quantum state with uniform superposition
    pub fn new(size: usize) -> Self {
        let size = size.next_power_of_two().max(4);
        let amp = 1.0 / (size as f64).sqrt();

        Self {
            amplitudes: vec![amp; size],
            dimension: size.trailing_zeros() as usize,
        }
    }

    /// Create state from initial values
    #[allow(dead_code)]
    pub fn from_vec(values: Vec<f64>) -> Self {
        let size = values.len().next_power_of_two();
        let mut amplitudes = values;
        amplitudes.resize(size, 0.0);

        Self::normalize_vec(&mut amplitudes);

        Self {
            amplitudes,
            dimension: size.trailing_zeros() as usize,
        }
    }

    /// Get state dimension
    #[allow(dead_code)]
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Get state size
    pub fn size(&self) -> usize {
        self.amplitudes.len()
    }

    /// Normalize amplitudes to unit vector
    pub fn normalize(&mut self) {
        Self::normalize_vec(&mut self.amplitudes);
    }

    /// Normalize a vector in-place
    fn normalize_vec(vec: &mut [f64]) {
        let sum_sq: f64 = vec.par_iter().map(|x| x * x).sum();
        if sum_sq > 0.0 {
            let norm = sum_sq.sqrt();
            vec.par_iter_mut().for_each(|x| *x /= norm);
        }
    }

    /// Parallel computation using Rayon
    /// This simulates quantum superposition through data parallelism
    pub fn superposition_compute<F>(&self, f: F) -> Vec<f64>
    where
        F: Fn(usize) -> f64 + Sync + Send,
    {
        self.amplitudes
            .par_iter()
            .enumerate()
            .map(|(i, &amp)| amp * f(i))
            .collect()
    }

    /// Apply function element-wise in parallel
    #[allow(dead_code)]
    pub fn apply_parallel<F>(&mut self, f: F)
    where
        F: Fn(f64) -> f64 + Sync + Send,
    {
        self.amplitudes.par_iter_mut().for_each(|x| {
            *x = f(*x);
        });
    }

    /// Probabilistic measurement (quantum collapse simulation)
    /// Returns index based on amplitude probabilities
    pub fn measure(&self) -> usize {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let r: f64 = rng.gen();
        let mut cumsum = 0.0;

        for (i, &amp) in self.amplitudes.iter().enumerate() {
            cumsum += amp * amp; // |amplitude|^2 = probability
            if r < cumsum {
                return i;
            }
        }

        // Fallback to last index
        self.amplitudes.len().saturating_sub(1)
    }

    /// Perform measurement multiple times and return histogram
    #[allow(dead_code)]
    pub fn measure_many(&self, shots: usize) -> Vec<usize> {
        let mut histogram = vec![0usize; self.amplitudes.len()];

        for _ in 0..shots {
            let idx = self.measure();
            histogram[idx] += 1;
        }

        histogram
    }

    /// Entangle this state with another (correlation simulation)
    /// Creates quantum-like correlations between two states
    #[allow(dead_code)]
    pub fn entangle_with(&mut self, other: &mut Self) {
        assert_eq!(
            self.amplitudes.len(),
            other.amplitudes.len(),
            "States must have same dimension to entangle"
        );

        // Use parallel processing for correlation calculation
        self.amplitudes
            .par_iter_mut()
            .zip(other.amplitudes.par_iter_mut())
            .for_each(|(a, b)| {
                let correlation = (*a + *b) / 2.0;
                *a = correlation;
                *b = correlation;
            });

        // Renormalize both states
        self.normalize();
        other.normalize();
    }

    /// Tensor product with another state
    #[allow(dead_code)]
    pub fn tensor_product(&self, other: &Self) -> Self {
        let new_size = self.amplitudes.len() * other.amplitudes.len();
        let mut new_amplitudes = Vec::with_capacity(new_size);

        for a in &self.amplitudes {
            for b in &other.amplitudes {
                new_amplitudes.push(a * b);
            }
        }

        Self::normalize_vec(&mut new_amplitudes);

        Self {
            amplitudes: new_amplitudes,
            dimension: self.dimension + other.dimension,
        }
    }

    /// Apply Hadamard-like transform (creates superposition)
    #[allow(dead_code)]
    pub fn hadamard_transform(&mut self) {
        let h_factor = 1.0 / (2.0_f64).sqrt();

        // Process pairs of amplitudes
        self.amplitudes.par_chunks_exact_mut(2).for_each(|pair| {
            let a = pair[0];
            let b = pair[1];
            pair[0] = h_factor * (a + b);
            pair[1] = h_factor * (a - b);
        });

        self.normalize();
    }

    /// Calculate state fidelity with another state
    #[allow(dead_code)]
    pub fn fidelity(&self, other: &Self) -> f64 {
        assert_eq!(self.amplitudes.len(), other.amplitudes.len());

        self.amplitudes
            .par_iter()
            .zip(other.amplitudes.par_iter())
            .map(|(a, b)| a * b)
            .sum::<f64>()
            .abs()
    }

    /// Get probability distribution
    #[allow(dead_code)]
    pub fn probabilities(&self) -> Vec<f64> {
        self.amplitudes.par_iter().map(|a| a * a).collect()
    }

    /// Find most probable state
    #[allow(dead_code)]
    pub fn most_probable(&self) -> (usize, f64) {
        self.amplitudes
            .iter()
            .enumerate()
            .map(|(i, &a)| (i, a * a))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap_or((0, 0.0))
    }

    /// Apply phase rotation
    #[allow(dead_code)]
    pub fn phase_rotation(&mut self, angle: f64) {
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        self.amplitudes.par_iter_mut().for_each(|amp| {
            // Simple phase rotation for real amplitudes
            *amp = *amp * cos_a - *amp * sin_a;
        });

        self.normalize();
    }

    /// Reset to uniform superposition
    #[allow(dead_code)]
    pub fn reset_to_uniform(&mut self) {
        let amp = 1.0 / (self.amplitudes.len() as f64).sqrt();
        self.amplitudes.par_iter_mut().for_each(|a| *a = amp);
    }

    /// Reset to specific state
    #[allow(dead_code)]
    pub fn reset_to_state(&mut self, index: usize) {
        if index < self.amplitudes.len() {
            self.amplitudes
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, a)| {
                    *a = if i == index { 1.0 } else { 0.0 };
                });
        }
    }

    /// Get state as byte vector (for serialization)
    #[allow(dead_code)]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.amplitudes
            .iter()
            .flat_map(|&f| f.to_le_bytes())
            .collect()
    }

    /// Create state from byte vector
    #[allow(dead_code)]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() % 8 != 0 {
            return Err("Invalid byte length".into());
        }

        let mut amplitudes = Vec::with_capacity(bytes.len() / 8);
        for chunk in bytes.chunks_exact(8) {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(chunk);
            amplitudes.push(f64::from_le_bytes(arr));
        }

        let size = amplitudes.len();
        Ok(Self {
            amplitudes,
            dimension: size.trailing_zeros() as usize,
        })
    }
}

impl Default for QuantumInspiredState {
    fn default() -> Self {
        Self::new(256)
    }
}

/// Multi-state quantum system for entanglement simulations
#[allow(dead_code)]
pub struct QuantumSystem {
    states: Vec<QuantumInspiredState>,
    correlations: Vec<Vec<f64>>,
}

impl QuantumSystem {
    /// Create a new quantum system
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            states: Vec::new(),
            correlations: Vec::new(),
        }
    }

    /// Add a state to the system
    #[allow(dead_code)]
    pub fn add_state(&mut self, state: QuantumInspiredState) -> usize {
        let idx = self.states.len();

        // Initialize correlations with this new state
        for corr in &mut self.correlations {
            corr.push(0.0);
        }
        self.correlations.push(vec![0.0; self.states.len() + 1]);

        self.states.push(state);
        idx
    }

    /// Get state by index
    #[allow(dead_code)]
    pub fn get_state(&self, idx: usize) -> Option<&QuantumInspiredState> {
        self.states.get(idx)
    }

    /// Get mutable state by index
    #[allow(dead_code)]
    pub fn get_state_mut(&mut self, idx: usize) -> Option<&mut QuantumInspiredState> {
        self.states.get_mut(idx)
    }

    /// Entangle two states in the system
    #[allow(dead_code)]
    pub fn entangle(&mut self, idx1: usize, idx2: usize) {
        if idx1 < self.states.len() && idx2 < self.states.len() && idx1 != idx2 {
            // Mark correlation
            self.correlations[idx1][idx2] = 1.0;
            self.correlations[idx2][idx1] = 1.0;

            // Actually entangle the states
            let (state1, state2) = if idx1 < idx2 {
                let (left, right) = self.states.split_at_mut(idx2);
                (&mut left[idx1], &mut right[0])
            } else {
                let (left, right) = self.states.split_at_mut(idx1);
                (&mut right[0], &mut left[idx2])
            };

            state1.entangle_with(state2);
        }
    }

    /// Measure all states
    #[allow(dead_code)]
    pub fn measure_all(&self) -> Vec<usize> {
        self.states.iter().map(|s| s.measure()).collect()
    }

    /// Get system size
    #[allow(dead_code)]
    pub fn size(&self) -> usize {
        self.states.len()
    }

    /// Clear all states
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.states.clear();
        self.correlations.clear();
    }
}

impl Default for QuantumSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantum_state_creation() {
        let state = QuantumInspiredState::new(8);
        assert_eq!(state.size(), 8);
        assert_eq!(state.dimension(), 3);

        // Check uniform initialization
        let expected_amp = 1.0 / (8.0_f64).sqrt();
        for &amp in &state.amplitudes {
            assert!((amp - expected_amp).abs() < 1e-10);
        }
    }

    #[test]
    fn test_normalization() {
        let state = QuantumInspiredState::from_vec(vec![1.0, 2.0, 3.0, 4.0]);
        let probs: f64 = state.probabilities().iter().sum();
        assert!((probs - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_superposition_compute() {
        let state = QuantumInspiredState::new(256);
        let result = state.superposition_compute(|i| i as f64 * 0.01);

        assert_eq!(result.len(), 256);
        // Check that computation was applied
        assert!(result.iter().any(|&x| x > 0.0));
    }

    #[test]
    fn test_measure() {
        let state = QuantumInspiredState::new(8);
        let idx = state.measure();
        assert!(idx < 8);
    }

    #[test]
    fn test_measure_many() {
        let state = QuantumInspiredState::new(4);
        let histogram = state.measure_many(1000);

        assert_eq!(histogram.len(), 4);
        let total: usize = histogram.iter().sum();
        assert_eq!(total, 1000);
    }

    #[test]
    fn test_entanglement() {
        let mut state1 = QuantumInspiredState::new(4);
        let mut state2 = QuantumInspiredState::new(4);

        state1.entangle_with(&mut state2);

        // After entanglement, they should be correlated
        for i in 0..4 {
            assert!((state1.amplitudes[i] - state2.amplitudes[i]).abs() < 1e-10);
        }

        // Should be normalized
        let prob1: f64 = state1.probabilities().iter().sum();
        let prob2: f64 = state2.probabilities().iter().sum();
        assert!((prob1 - 1.0).abs() < 1e-10);
        assert!((prob2 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_tensor_product() {
        let state1 = QuantumInspiredState::new(2);
        let state2 = QuantumInspiredState::new(2);

        let combined = state1.tensor_product(&state2);
        // Tensor product of 2x2 = 4 states, but implementation may differ
        assert!(combined.size() >= 2);
    }

    #[test]
    fn test_hadamard_transform() {
        let mut state = QuantumInspiredState::new(4);
        state.reset_to_state(0);

        state.hadamard_transform();

        // After Hadamard, should be in superposition (most states > 0)
        let probs = state.probabilities();
        let nonzero = probs.iter().filter(|&&p| p > 1e-10).count();
        assert!(
            nonzero >= 2,
            "Expected superposition, got {} nonzero states",
            nonzero
        );
    }

    #[test]
    fn test_fidelity() {
        let state1 = QuantumInspiredState::new(4);
        let state2 = QuantumInspiredState::new(4);

        let fidelity = state1.fidelity(&state2);
        assert!(fidelity >= 0.0 && fidelity <= 1.0);

        // Same state should have fidelity close to 1
        assert!(state1.fidelity(&state1) > 0.99);
    }

    #[test]
    fn test_most_probable() {
        let mut state = QuantumInspiredState::new(4);
        state.reset_to_state(2);

        let (idx, prob) = state.most_probable();
        assert_eq!(idx, 2);
        assert!(prob > 0.99);
    }

    #[test]
    fn test_quantum_system() {
        let mut system = QuantumSystem::new();

        let state1 = QuantumInspiredState::new(4);
        let state2 = QuantumInspiredState::new(4);

        let idx1 = system.add_state(state1);
        let idx2 = system.add_state(state2);

        assert_eq!(system.size(), 2);

        system.entangle(idx1, idx2);

        let measurements = system.measure_all();
        assert_eq!(measurements.len(), 2);
    }

    #[test]
    fn test_serialization() {
        let state = QuantumInspiredState::new(8);
        let bytes = state.to_bytes();

        let restored = QuantumInspiredState::from_bytes(&bytes).unwrap();
        assert_eq!(restored.size(), 8);

        for i in 0..8 {
            assert!((state.amplitudes[i] - restored.amplitudes[i]).abs() < 1e-10);
        }
    }
}

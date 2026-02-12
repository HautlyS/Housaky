//! Spike-Timing-Dependent Plasticity (STDP)
//! Biologically-inspired learning rule

pub struct STDPLearning {
    pub learning_rate: f32,
    pub tau_plus: f32,  // Time constant for potentiation
    pub tau_minus: f32, // Time constant for depression
    pub a_plus: f32,    // Amplitude for potentiation
    pub a_minus: f32,   // Amplitude for depression
}

impl Default for STDPLearning {
    fn default() -> Self {
        Self {
            learning_rate: 0.01,
            tau_plus: 20.0,
            tau_minus: 20.0,
            a_plus: 0.005,
            a_minus: 0.00525,
        }
    }
}

impl STDPLearning {
    pub fn new(learning_rate: f32) -> Self {
        Self {
            learning_rate,
            ..Default::default()
        }
    }

    /// Calculate weight change based on spike timing
    /// delta_t = t_post - t_pre
    pub fn weight_update(&self, delta_t: f32, current_weight: f32) -> f32 {
        let delta_w = if delta_t > 0.0 {
            // Post-synaptic spike after pre-synaptic (potentiation)
            self.a_plus * (-delta_t / self.tau_plus).exp()
        } else {
            // Post-synaptic spike before pre-synaptic (depression)
            -self.a_minus * (delta_t / self.tau_minus).exp()
        };

        let new_weight = current_weight + self.learning_rate * delta_w;
        new_weight.clamp(0.0, 1.0) // Keep weights bounded
    }

    /// Batch update weights based on spike pairs
    pub fn update_weights(
        &self,
        weights: &mut [Vec<f32>],
        pre_spike_times: &[Option<u64>],
        post_spike_times: &[Option<u64>],
    ) {
        for (from_idx, pre_time) in pre_spike_times.iter().enumerate() {
            if let Some(t_pre) = pre_time {
                for (to_idx, post_time) in post_spike_times.iter().enumerate() {
                    if let Some(t_post) = post_time {
                        let delta_t = *t_post as f32 - *t_pre as f32;
                        
                        if delta_t.abs() < 100.0 { // Only update if spikes are close in time
                            let current_w = weights[from_idx][to_idx];
                            weights[from_idx][to_idx] = self.weight_update(delta_t, current_w);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdp_potentiation() {
        let stdp = STDPLearning::default();
        let w = stdp.weight_update(10.0, 0.5); // Post after pre
        assert!(w > 0.5); // Weight should increase
    }

    #[test]
    fn test_stdp_depression() {
        let stdp = STDPLearning::default();
        let w = stdp.weight_update(-10.0, 0.5); // Post before pre
        assert!(w < 0.5); // Weight should decrease
    }
}

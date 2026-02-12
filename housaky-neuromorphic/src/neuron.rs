//! Leaky Integrate-and-Fire (LIF) Neuron Model
//! Energy-efficient spiking neuron implementation

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuronState {
    pub membrane_potential: f32,
    pub threshold: f32,
    pub resting_potential: f32,
    pub leak_factor: f32,
    pub refractory_period: u32,
    pub refractory_counter: u32,
    pub last_spike_time: Option<u64>,
}

impl Default for NeuronState {
    fn default() -> Self {
        Self {
            membrane_potential: -70.0,
            threshold: -55.0,
            resting_potential: -70.0,
            leak_factor: 0.95,
            refractory_period: 5,
            refractory_counter: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LIFNeuron {
    pub state: NeuronState,
    pub spike_count: u64,
}

impl LIFNeuron {
    pub fn new(threshold: f32) -> Self {
        Self {
            state: NeuronState {
                threshold,
                ..Default::default()
            },
            spike_count: 0,
        }
    }

    /// Process input current and return true if neuron spikes
    pub fn step(&mut self, input_current: f32, time: u64) -> bool {
        // Refractory period check
        if self.state.refractory_counter > 0 {
            self.state.refractory_counter -= 1;
            self.state.membrane_potential = self.state.resting_potential;
            return false;
        }

        // Leaky integration
        self.state.membrane_potential = 
            self.state.leak_factor * self.state.membrane_potential + input_current;

        // Spike detection
        if self.state.membrane_potential >= self.state.threshold {
            self.spike_count += 1;
            self.state.last_spike_time = Some(time);
            self.state.membrane_potential = self.state.resting_potential;
            self.state.refractory_counter = self.state.refractory_period;
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.state.membrane_potential = self.state.resting_potential;
        self.state.refractory_counter = 0;
        self.spike_count = 0;
        self.state.last_spike_time = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lif_neuron_spike() {
        let mut neuron = LIFNeuron::new(-55.0);
        
        // Strong input should cause spike
        let spiked = neuron.step(20.0, 0);
        assert!(spiked);
        assert_eq!(neuron.spike_count, 1);
    }

    #[test]
    fn test_refractory_period() {
        let mut neuron = LIFNeuron::new(-55.0);
        
        neuron.step(20.0, 0); // First spike
        let spiked = neuron.step(20.0, 1); // Should not spike (refractory)
        assert!(!spiked);
    }
}

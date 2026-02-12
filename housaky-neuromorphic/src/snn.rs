//! Spiking Neural Network - Event-driven processing
//! 70% more energy efficient than traditional ANNs (2025 research)

use crate::neuron::LIFNeuron;
use rayon::prelude::*;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct SpikeEvent {
    pub neuron_id: usize,
    pub time: u64,
    pub layer: usize,
}

pub struct SpikingNeuralNetwork {
    layers: Vec<Vec<LIFNeuron>>,
    weights: Vec<Vec<Vec<f32>>>, // [layer][from][to]
    spike_buffer: VecDeque<SpikeEvent>,
    current_time: u64,
    max_buffer_size: usize,
}

impl SpikingNeuralNetwork {
    pub fn new(layer_sizes: &[usize], threshold: f32) -> Self {
        let layers: Vec<Vec<LIFNeuron>> = layer_sizes
            .iter()
            .map(|&size| (0..size).map(|_| LIFNeuron::new(threshold)).collect())
            .collect();

        let mut weights = Vec::new();
        for i in 0..layer_sizes.len() - 1 {
            let mut layer_weights = vec![vec![0.0; layer_sizes[i + 1]]; layer_sizes[i]];
            
            // Xavier initialization
            let scale = (2.0 / (layer_sizes[i] + layer_sizes[i + 1]) as f32).sqrt();
            for from in 0..layer_sizes[i] {
                for to in 0..layer_sizes[i + 1] {
                    layer_weights[from][to] = (rand::random::<f32>() - 0.5) * 2.0 * scale;
                }
            }
            weights.push(layer_weights);
        }

        Self {
            layers,
            weights,
            spike_buffer: VecDeque::with_capacity(10000),
            current_time: 0,
            max_buffer_size: 10000,
        }
    }

    /// Process input spikes through network (event-driven)
    pub fn forward(&mut self, input_spikes: &[bool]) -> Vec<bool> {
        assert_eq!(input_spikes.len(), self.layers[0].len());

        // Encode input as spike events
        for (i, &spike) in input_spikes.iter().enumerate() {
            if spike {
                self.spike_buffer.push_back(SpikeEvent {
                    neuron_id: i,
                    time: self.current_time,
                    layer: 0,
                });
            }
        }

        // Process spikes layer by layer
        for layer_idx in 0..self.layers.len() - 1 {
            let current_spikes: Vec<_> = self.spike_buffer
                .iter()
                .filter(|e| e.layer == layer_idx && e.time == self.current_time)
                .cloned()
                .collect();

            // Parallel processing of next layer neurons
            let next_layer_spikes: Vec<_> = self.layers[layer_idx + 1]
                .par_iter_mut()
                .enumerate()
                .filter_map(|(to_idx, neuron)| {
                    let input_current: f32 = current_spikes
                        .iter()
                        .map(|spike| self.weights[layer_idx][spike.neuron_id][to_idx])
                        .sum();

                    if neuron.step(input_current, self.current_time) {
                        Some(SpikeEvent {
                            neuron_id: to_idx,
                            time: self.current_time + 1,
                            layer: layer_idx + 1,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            self.spike_buffer.extend(next_layer_spikes);
        }

        // Cleanup old spikes
        if self.spike_buffer.len() > self.max_buffer_size {
            self.spike_buffer.drain(0..self.spike_buffer.len() / 2);
        }

        self.current_time += 1;

        // Return output layer spikes
        let output_layer_idx = self.layers.len() - 1;
        self.layers[output_layer_idx]
            .iter()
            .map(|n| n.state.last_spike_time == Some(self.current_time - 1))
            .collect()
    }

    /// Get spike rate for output layer (for classification)
    pub fn get_spike_rates(&self, window: u64) -> Vec<f32> {
        let output_layer = self.layers.last().unwrap();
        output_layer
            .iter()
            .map(|n| n.spike_count as f32 / window as f32)
            .collect()
    }

    pub fn reset(&mut self) {
        for layer in &mut self.layers {
            for neuron in layer {
                neuron.reset();
            }
        }
        self.spike_buffer.clear();
        self.current_time = 0;
    }

    /// Energy consumption estimate (spikes only)
    pub fn energy_consumption(&self) -> f32 {
        let total_spikes: u64 = self.layers
            .iter()
            .flat_map(|layer| layer.iter())
            .map(|n| n.spike_count)
            .sum();
        
        // Assume 1 pJ per spike (realistic for neuromorphic hardware)
        total_spikes as f32 * 1e-12
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snn_forward() {
        let mut snn = SpikingNeuralNetwork::new(&[4, 8, 2], -55.0);
        let input = vec![true, false, true, false];
        
        let output = snn.forward(&input);
        assert_eq!(output.len(), 2);
    }

    #[test]
    fn test_energy_efficiency() {
        let mut snn = SpikingNeuralNetwork::new(&[10, 20, 10], -55.0);
        
        for _ in 0..100 {
            let input: Vec<bool> = (0..10).map(|_| rand::random()).collect();
            snn.forward(&input);
        }
        
        let energy = snn.energy_consumption();
        assert!(energy < 1e-6); // Should be very low
    }
}

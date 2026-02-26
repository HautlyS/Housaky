use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NeuronType {
    Sensory,
    Inter,
    Motor,
    Modulatory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neuron {
    pub id: String,
    pub membrane_potential: f64,
    pub threshold: f64,
    pub leak_rate: f64,
    pub last_spike: Option<DateTime<Utc>>,
    pub neuron_type: NeuronType,
    pub refractory_period_ms: f64,
    pub spike_count: u64,
    pub bias: f64,
}

impl Neuron {
    pub fn new(id: &str, threshold: f64, leak_rate: f64, neuron_type: NeuronType) -> Self {
        Self {
            id: id.to_string(),
            membrane_potential: 0.0,
            threshold,
            leak_rate,
            last_spike: None,
            neuron_type,
            refractory_period_ms: 2.0,
            spike_count: 0,
            bias: 0.0,
        }
    }

    pub fn sensory(id: &str) -> Self {
        Self::new(id, 0.5, 0.1, NeuronType::Sensory)
    }

    pub fn inter(id: &str) -> Self {
        Self::new(id, 0.7, 0.15, NeuronType::Inter)
    }

    pub fn motor(id: &str) -> Self {
        Self::new(id, 0.6, 0.12, NeuronType::Motor)
    }

    pub fn modulatory(id: &str) -> Self {
        Self::new(id, 0.4, 0.08, NeuronType::Modulatory)
    }

    pub fn is_in_refractory(&self) -> bool {
        if let Some(last) = self.last_spike {
            let elapsed_ms = (Utc::now() - last).num_microseconds().unwrap_or(0) as f64 / 1000.0;
            elapsed_ms < self.refractory_period_ms
        } else {
            false
        }
    }

    pub fn integrate(&mut self, input: f64, dt_ms: f64) -> bool {
        if self.is_in_refractory() {
            return false;
        }
        self.membrane_potential += input + self.bias;
        self.membrane_potential -= self.leak_rate * self.membrane_potential * dt_ms;
        self.membrane_potential = self.membrane_potential.clamp(-1.0, 2.0);

        if self.membrane_potential >= self.threshold {
            self.fire();
            return true;
        }
        false
    }

    fn fire(&mut self) {
        self.membrane_potential = 0.0;
        self.last_spike = Some(Utc::now());
        self.spike_count += 1;
    }

    pub fn reset(&mut self) {
        self.membrane_potential = 0.0;
        self.last_spike = None;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Synapse {
    pub pre_neuron: String,
    pub post_neuron: String,
    pub weight: f64,
    pub delay_ms: f64,
    pub synapse_type: SynapseType,
    pub transmission_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SynapseType {
    Excitatory,
    Inhibitory,
    Modulatory,
}

impl Synapse {
    pub fn new(pre: &str, post: &str, weight: f64, synapse_type: SynapseType) -> Self {
        Self {
            pre_neuron: pre.to_string(),
            post_neuron: post.to_string(),
            weight,
            delay_ms: 0.5,
            synapse_type,
            transmission_count: 0,
        }
    }

    pub fn excitatory(pre: &str, post: &str, weight: f64) -> Self {
        Self::new(pre, post, weight.abs(), SynapseType::Excitatory)
    }

    pub fn inhibitory(pre: &str, post: &str, weight: f64) -> Self {
        Self::new(pre, post, -weight.abs(), SynapseType::Inhibitory)
    }

    pub fn effective_weight(&self) -> f64 {
        match self.synapse_type {
            SynapseType::Excitatory => self.weight.abs(),
            SynapseType::Inhibitory => -self.weight.abs(),
            SynapseType::Modulatory => self.weight * 0.5,
        }
    }

    pub fn stdp_update(&mut self, pre_spiked: bool, post_spiked: bool) {
        const A_PLUS: f64 = 0.01;
        const A_MINUS: f64 = 0.012;
        if pre_spiked && post_spiked {
            self.weight += A_PLUS * (1.0 - self.weight);
        } else if !pre_spiked && post_spiked {
            self.weight -= A_MINUS * self.weight;
        }
        self.weight = self.weight.clamp(0.0, 2.0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpikeEvent {
    pub neuron_id: String,
    pub timestamp: DateTime<Utc>,
    pub potential_at_spike: f64,
}

#[derive(Debug, Clone)]
pub struct SpikeNetwork {
    pub neurons: HashMap<String, Neuron>,
    pub synapses: Vec<Synapse>,
    pub clock_hz: f64,
    pub refractory_period_ms: f64,
    pub spike_history: Vec<SpikeEvent>,
    pub max_spike_history: usize,
    pub stdp_enabled: bool,
}

impl SpikeNetwork {
    pub fn new(clock_hz: f64) -> Self {
        Self {
            neurons: HashMap::new(),
            synapses: Vec::new(),
            clock_hz,
            refractory_period_ms: 2.0,
            spike_history: Vec::new(),
            max_spike_history: 10_000,
            stdp_enabled: false,
        }
    }

    pub fn with_stdp(mut self) -> Self {
        self.stdp_enabled = true;
        self
    }

    pub fn add_neuron(&mut self, neuron: Neuron) {
        self.neurons.insert(neuron.id.clone(), neuron);
    }

    pub fn add_synapse(&mut self, synapse: Synapse) {
        self.synapses.push(synapse);
    }

    pub fn connect(&mut self, pre: &str, post: &str, weight: f64, excitatory: bool) {
        let synapse = if excitatory {
            Synapse::excitatory(pre, post, weight)
        } else {
            Synapse::inhibitory(pre, post, weight)
        };
        self.synapses.push(synapse);
    }

    pub fn step(&mut self, inputs: &HashMap<String, f64>) -> Vec<String> {
        let dt_ms = 1000.0 / self.clock_hz;
        let mut fired: Vec<String> = Vec::new();

        for (neuron_id, &input) in inputs {
            if let Some(neuron) = self.neurons.get_mut(neuron_id) {
                if neuron.integrate(input, dt_ms) {
                    fired.push(neuron_id.clone());
                    if self.spike_history.len() >= self.max_spike_history {
                        self.spike_history.remove(0);
                    }
                    self.spike_history.push(SpikeEvent {
                        neuron_id: neuron_id.clone(),
                        timestamp: Utc::now(),
                        potential_at_spike: neuron.threshold,
                    });
                }
            }
        }

        let synaptic_inputs: HashMap<String, f64> = {
            let mut map: HashMap<String, f64> = HashMap::new();
            for synapse in &mut self.synapses {
                if fired.contains(&synapse.pre_neuron) {
                    *map.entry(synapse.post_neuron.clone()).or_insert(0.0) +=
                        synapse.effective_weight();
                    synapse.transmission_count += 1;
                }
            }
            map
        };

        for (neuron_id, input) in &synaptic_inputs {
            if let Some(neuron) = self.neurons.get_mut(neuron_id) {
                if neuron.integrate(*input, dt_ms) && !fired.contains(neuron_id) {
                    fired.push(neuron_id.clone());
                }
            }
        }

        if self.stdp_enabled && !fired.is_empty() {
            let fired_set: std::collections::HashSet<&str> =
                fired.iter().map(|s| s.as_str()).collect();
            for synapse in &mut self.synapses {
                let pre_fired = fired_set.contains(synapse.pre_neuron.as_str());
                let post_fired = fired_set.contains(synapse.post_neuron.as_str());
                synapse.stdp_update(pre_fired, post_fired);
            }
        }

        fired
    }

    pub fn firing_rate(&self, neuron_id: &str, window_ms: u64) -> f64 {
        let cutoff = Utc::now() - chrono::Duration::milliseconds(window_ms as i64);
        let spikes = self.spike_history.iter()
            .filter(|e| e.neuron_id == neuron_id && e.timestamp > cutoff)
            .count();
        spikes as f64 / (window_ms as f64 / 1000.0)
    }

    pub fn total_spike_count(&self) -> u64 {
        self.neurons.values().map(|n| n.spike_count).sum()
    }

    pub fn reset_all(&mut self) {
        for neuron in self.neurons.values_mut() {
            neuron.reset();
        }
        self.spike_history.clear();
    }

    pub fn stats(&self) -> SpikeNetworkStats {
        let total_neurons = self.neurons.len();
        let total_synapses = self.synapses.len();
        let total_spikes = self.total_spike_count();
        let avg_weight = if self.synapses.is_empty() {
            0.0
        } else {
            self.synapses.iter().map(|s| s.weight.abs()).sum::<f64>() / self.synapses.len() as f64
        };
        SpikeNetworkStats { total_neurons, total_synapses, total_spikes, avg_weight }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpikeNetworkStats {
    pub total_neurons: usize,
    pub total_synapses: usize,
    pub total_spikes: u64,
    pub avg_weight: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neuron_integration_and_fire() {
        let mut n = Neuron::sensory("s1");
        n.threshold = 1.0;
        let fired = n.integrate(1.5, 1.0);
        assert!(fired);
        assert_eq!(n.spike_count, 1);
        assert_eq!(n.membrane_potential, 0.0);
    }

    #[test]
    fn test_synapse_stdp() {
        let mut s = Synapse::excitatory("pre", "post", 0.5);
        s.stdp_update(true, true);
        assert!(s.weight > 0.5);
    }

    #[test]
    fn test_network_step() {
        let mut net = SpikeNetwork::new(1000.0);
        net.add_neuron(Neuron::sensory("s1"));
        net.add_neuron(Neuron::motor("m1"));
        net.connect("s1", "m1", 1.0, true);

        let mut inputs = HashMap::new();
        inputs.insert("s1".to_string(), 2.0);
        let fired = net.step(&inputs);
        assert!(fired.contains(&"s1".to_string()));
    }

    #[test]
    fn test_refractory_period() {
        let mut n = Neuron::sensory("s");
        n.threshold = 0.5;
        n.integrate(1.0, 1.0);
        assert_eq!(n.spike_count, 1);
        let fired_again = n.integrate(1.0, 1.0);
        assert!(!fired_again);
    }
}

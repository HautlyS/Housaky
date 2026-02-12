//! Neuromorphic Computing Module - Spiking Neural Networks
//! Based on 2025-2026 research on brain-inspired computing

pub mod snn;
pub mod stdp;
pub mod neuron;

pub use snn::SpikingNeuralNetwork;
pub use neuron::{LIFNeuron, NeuronState};
pub use stdp::STDPLearning;

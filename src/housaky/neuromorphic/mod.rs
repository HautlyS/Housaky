pub mod event_bus;
pub mod habituation;
pub mod reflex_arc;
pub mod sensory_fusion;
pub mod spike_network;

use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub use event_bus::{EventBus, EventBusMetrics, EventPriority, NeuromorphicEvent};
pub use habituation::{HabituationConfig, HabituationSystem};
pub use reflex_arc::{
    HardwareAction, HardwareActionType, ReflexArc, ReflexArcSystem, ReflexCondition, SensorEvent,
    SensorType,
};
pub use sensory_fusion::{
    FusedPercept, FusionMethod, KalmanState, SensorReading, SensoryFusionEngine,
};
pub use spike_network::{Neuron, NeuronType, SpikeNetwork, Synapse, SynapseType};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NeuromorphicConfig {
    pub enabled: bool,
    pub spike_threshold: f64,
    pub learning_rate: f64,
    pub network_size: usize,
}

pub struct NeuromorphicEngine {
    config: NeuromorphicConfig,
    event_bus: Arc<EventBus>,
    spike_network: Arc<SpikeNetwork>,
    habituation: Arc<HabituationSystem>,
}

impl NeuromorphicEngine {
    pub fn new(config: NeuromorphicConfig) -> Self {
        Self {
            config: config.clone(),
            event_bus: Arc::new(EventBus::new(1000)),
            spike_network: Arc::new(SpikeNetwork::new(1000.0)),
            habituation: Arc::new(HabituationSystem::new(HabituationConfig::default())),
        }
    }

    pub fn config(&self) -> &NeuromorphicConfig {
        &self.config
    }

    pub fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    pub fn spike_network(&self) -> &Arc<SpikeNetwork> {
        &self.spike_network
    }
}

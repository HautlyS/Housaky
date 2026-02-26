pub mod event_bus;
pub mod habituation;
pub mod reflex_arc;
pub mod sensory_fusion;
pub mod spike_network;

pub use event_bus::{EventBus, EventBusMetrics, EventPriority, NeuromorphicEvent};
pub use habituation::{HabituationConfig, HabituationSystem};
pub use reflex_arc::{
    HardwareAction, HardwareActionType, ReflexArc, ReflexArcSystem, ReflexCondition,
    SensorEvent, SensorType,
};
pub use sensory_fusion::{FusedPercept, FusionMethod, KalmanState, SensoryFusionEngine, SensorReading};
pub use spike_network::{Neuron, NeuronType, SpikeNetwork, Synapse, SynapseType};

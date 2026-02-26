pub mod collective_memory;
pub mod consensus;
pub mod emergence;
pub mod pheromone;
pub mod stigmergy;
pub mod swarm_controller;
pub mod task_market;

pub use collective_memory::{CollectiveMemory, ConflictResolution, SharedMemoryEntry};
pub use consensus::{ConsensusEngine, ConsensusProtocol, ConsensusResult, Proposal};
pub use emergence::{EmergenceDetector, EmergentBehavior, EmergenceType, SwarmObservation};
pub use pheromone::{PheromoneMap, PheromoneService, PheromoneTrail};
pub use stigmergy::{MarkType, SharedEnvironment, StigmergyLayer};
pub use swarm_controller::{SwarmAgent, SwarmConfig, SwarmController, SwarmStats};
pub use task_market::{Bid, MarketTask, TaskMarket, TaskStatus};

pub mod agent_registry;
pub mod coordinator;
pub mod message;
pub mod replication;
pub mod emergent_protocol;

pub use agent_registry::AgentRegistry;
pub use coordinator::MultiAgentCoordinator;
pub use message::{AgentMessage, MessageType};
pub use replication::{AgentReplicator, ChildAgent, ForkRequest, Specialization, TaskResult};

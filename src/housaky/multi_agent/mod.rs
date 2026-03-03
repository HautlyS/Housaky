pub mod agent_registry;
pub mod coordinator;
pub mod emergent_protocol;
pub mod message;
pub mod replication;

pub use agent_registry::AgentRegistry;
pub use coordinator::MultiAgentCoordinator;
pub use message::{AgentMessage, MessageType};
pub use replication::{AgentReplicator, ChildAgent, ForkRequest, Specialization, TaskResult};

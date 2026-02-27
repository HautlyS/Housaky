pub mod agent_memory;
pub mod belief_tracker;
pub mod consolidation;
pub mod hierarchical;
pub mod provenance;
pub mod local_embeddings;

// Phase 3 — Consciousness Substrate & Self-Awareness
pub mod emotional_tags;
pub mod episodic;
pub mod autobiographical;
pub mod reconsolidation;
pub mod schema;
pub mod forgetting;

pub use belief_tracker::{Belief, BeliefSource, BeliefTracker};
pub use agent_memory::{AgentMemoryRecord, AgentMemoryStore, MemoryKind};
pub use consolidation::{ExtractedPattern, MemoryConsolidator, PatternType};
pub use hierarchical::HierarchicalMemory;
pub use provenance::ProvenanceTracker;
pub use local_embeddings::{LocalEmbeddingManager, LocalEmbeddingProvider, LocalEmbeddingConfig, EmbeddingProvider, EmbeddingResult};

pub use emotional_tags::EmotionalTag;
pub use episodic::{Episode, EpisodicContext, EpisodicEvent, EpisodicEventType, EpisodicMemory, EpisodicStats};
pub use autobiographical::{AutobiographicalMemory, CapabilityRecord, LifeEvent, LifeEventType, RelationshipRecord, RelationshipType};
pub use reconsolidation::{MemoryReconsolidator, ReconsolidationRecord, ReconsolidationTrigger};
pub use schema::{ForgettingCurve, MemorySchema, SchemaLibrary, SchemaStats};
pub use forgetting::{AdaptiveForgetting, ForgettingConfig, ForgettingReport};


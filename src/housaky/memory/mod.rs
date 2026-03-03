pub mod agent_memory;
pub mod belief_tracker;
pub mod consolidation;
pub mod hierarchical;
pub mod local_embeddings;
pub mod provenance;

// Phase 3 — Consciousness Substrate & Self-Awareness
pub mod autobiographical;
pub mod emotional_tags;
pub mod episodic;
pub mod forgetting;
pub mod reconsolidation;
pub mod schema;

pub use agent_memory::{AgentMemoryRecord, AgentMemoryStore, MemoryKind};
pub use belief_tracker::{Belief, BeliefSource, BeliefTracker};
pub use consolidation::{ExtractedPattern, MemoryConsolidator, PatternType};
pub use hierarchical::HierarchicalMemory;
pub use local_embeddings::{
    EmbeddingProvider, EmbeddingResult, LocalEmbeddingConfig, LocalEmbeddingManager,
    LocalEmbeddingProvider,
};
pub use provenance::ProvenanceTracker;

pub use autobiographical::{
    AutobiographicalMemory, CapabilityRecord, LifeEvent, LifeEventType, RelationshipRecord,
    RelationshipType,
};
pub use emotional_tags::EmotionalTag;
pub use episodic::{
    Episode, EpisodicContext, EpisodicEvent, EpisodicEventType, EpisodicMemory, EpisodicStats,
};
pub use forgetting::{AdaptiveForgetting, ForgettingConfig, ForgettingReport};
pub use reconsolidation::{MemoryReconsolidator, ReconsolidationRecord, ReconsolidationTrigger};
pub use schema::{ForgettingCurve, MemorySchema, SchemaLibrary, SchemaStats};

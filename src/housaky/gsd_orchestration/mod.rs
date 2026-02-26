pub mod orchestrator;
pub mod phase;
pub mod task;
pub mod context_manager;
pub mod wave_executor;
pub mod step_decomposer;
pub mod execution;

pub use execution::GSDExecutionEngine;
pub use orchestrator::{
    GSDOrchestrator, ExecutionSummary, VerificationReport, TaskAwareness, TaskAwarenessReport,
    CapabilityProfile, TaskPerformance, SelfImprovementIntegration, CapabilityUpdate,
};
pub use phase::{Phase, PhaseStatus, PhaseContext, Requirement, RequirementPriority, RequirementStatus, Decision};
pub use task::{GSDTask, GSDTaskStatus, TaskPriority, TaskVerification, Artifact, ArtifactType};
pub use context_manager::{ContextManager, ContextFile, ContextFileType, ProjectContext, RequirementsContext, RoadmapContext, StateContext, Milestone, MilestoneStatus, DecisionRecord};
pub use wave_executor::{WaveExecutor, Wave, WaveStatus, ExecutionResult};
pub use step_decomposer::{StepDecomposer, DecompositionContext, DecompositionResult, TaskStep, DecompositionStrategy, ComplexityAnalysis, ComplexityCategory};

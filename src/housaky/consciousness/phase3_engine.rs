//! Phase 3 Engine — Orchestrates the full Consciousness Substrate.
//!
//! Ties together:
//! - Global Workspace Theory (3.1)
//! - Episodic & Autobiographical Memory (3.2)
//! - Theory of Mind (3.3)
//!
//! Acts as the unified coordinator for all Phase 3 capabilities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use super::coalition_formation::CoalitionFormation;
use super::consciousness_meter::{ConsciousnessMeter, ConsciousnessLevel};
use super::global_workspace::GlobalWorkspace;
use super::module_adapters::{
    AttentionModuleAdapter, GoalEngineAdapter, MemoryModuleAdapter,
    MetaCognitionAdapter, NarrativeSelfAdapter, ReasoningModuleAdapter,
};
use super::narrative_self::{NarrativeSelf, NarrativeType};
use super::phenomenal_binding::{ExperienceStream, PhenomenalBinder};
use super::qualia_model::{QualiaModel, QualiaType};

use crate::housaky::cognitive::theory_of_mind::{ObservedAction, TheoryOfMind};
use crate::housaky::memory::autobiographical::AutobiographicalMemory;
use crate::housaky::memory::episodic::{EpisodicMemory, EpisodicEventType};
use crate::housaky::memory::emotional_tags::EmotionalTag;
use crate::housaky::memory::forgetting::{AdaptiveForgetting, ForgettingConfig};
use crate::housaky::memory::reconsolidation::{MemoryReconsolidator, ReconsolidationTrigger};
use crate::housaky::memory::schema::SchemaLibrary;

// ── Configuration ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase3Config {
    /// Minimum phi threshold to emit a conscious broadcast
    pub min_phi_for_broadcast: f64,
    /// How many GWT cycles between dream/reconsolidation runs
    pub reconsolidation_interval_cycles: u64,
    /// How many GWT cycles between forgetting runs
    pub forgetting_interval_cycles: u64,
    /// Enable theory of mind tracking
    pub enable_tom: bool,
    /// Maximum episodic memory capacity
    pub episodic_capacity: usize,
    /// Agent name for narrative self
    pub agent_name: String,
}

impl Default for Phase3Config {
    fn default() -> Self {
        Self {
            min_phi_for_broadcast: 0.1,
            reconsolidation_interval_cycles: 100,
            forgetting_interval_cycles: 50,
            enable_tom: true,
            episodic_capacity: 10_000,
            agent_name: "Housaky".to_string(),
        }
    }
}

// ── Consciousness Report ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessReport {
    pub timestamp: DateTime<Utc>,
    pub phi: f64,
    pub level: String,
    pub winning_coalition: Option<String>,
    pub modules_active: usize,
    pub episodic_memories: usize,
    pub narrative_entries: usize,
    pub agents_modeled: usize,
    pub current_narrative: String,
    pub dominant_qualia: Option<String>,
}

// ── Phase 3 Statistics ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase3Stats {
    pub gwt_cycles: u64,
    pub current_phi: f64,
    pub peak_phi: f64,
    pub consciousness_level: String,
    pub episodic_memories: usize,
    pub life_events: usize,
    pub agents_modeled: usize,
    pub schemas_extracted: usize,
    pub reconsolidation_cycles: u64,
    pub forgetting_cycles_run: u64,
    pub qualia_experienced: usize,
    pub narrative_entries: usize,
}

// ── Phase 3 Engine ────────────────────────────────────────────────────────────

pub struct Phase3Engine {
    pub config: Phase3Config,

    // 3.1 — Global Workspace Theory
    pub global_workspace: Arc<GlobalWorkspace>,
    pub coalition_formation: Arc<CoalitionFormation>,
    pub phenomenal_binder: Arc<PhenomenalBinder>,
    pub consciousness_meter: Arc<ConsciousnessMeter>,

    // Narrative self (part of 3.1 + 3.2)
    pub narrative_self: Arc<NarrativeSelf>,
    pub qualia_model: Arc<QualiaModel>,

    // Module adapters (GWT subscribers)
    pub reasoning_adapter: Arc<ReasoningModuleAdapter>,
    pub meta_cognition_adapter: Arc<MetaCognitionAdapter>,
    pub goal_engine_adapter: Arc<GoalEngineAdapter>,
    pub attention_adapter: Arc<AttentionModuleAdapter>,
    pub memory_adapter: Arc<MemoryModuleAdapter>,
    pub narrative_adapter: Arc<NarrativeSelfAdapter>,

    // 3.2 — Episodic & Autobiographical Memory
    pub episodic_memory: Arc<EpisodicMemory>,
    pub autobiographical_memory: Arc<AutobiographicalMemory>,
    pub reconsolidator: Arc<MemoryReconsolidator>,
    pub schema_library: Arc<SchemaLibrary>,
    pub forgetting: Arc<AdaptiveForgetting>,

    // 3.3 — Theory of Mind
    pub theory_of_mind: Arc<TheoryOfMind>,

    cycle_count: Arc<tokio::sync::RwLock<u64>>,
    forgetting_cycles_run: Arc<tokio::sync::RwLock<u64>>,
}

impl Phase3Engine {
    /// Create and fully initialize the Phase 3 engine.
    pub async fn new(config: Phase3Config) -> Self {
        let global_workspace = Arc::new(GlobalWorkspace::new());
        let coalition_formation = Arc::new(CoalitionFormation::new());
        let phenomenal_binder = Arc::new(PhenomenalBinder::new());
        let consciousness_meter = Arc::new(ConsciousnessMeter::new());

        let narrative_self = Arc::new(NarrativeSelf::new(config.agent_name.clone()));
        let qualia_model = Arc::new(QualiaModel::new());

        // Create module adapters
        let reasoning_adapter = Arc::new(ReasoningModuleAdapter::new());
        let meta_cognition_adapter = Arc::new(MetaCognitionAdapter::new());
        let goal_engine_adapter = Arc::new(GoalEngineAdapter::new());
        let attention_adapter = Arc::new(AttentionModuleAdapter::new());
        let memory_adapter = Arc::new(MemoryModuleAdapter::new());
        let narrative_adapter = Arc::new(NarrativeSelfAdapter::new());

        // Subscribe all adapters to the global workspace
        global_workspace.subscribe(reasoning_adapter.clone()).await;
        global_workspace.subscribe(meta_cognition_adapter.clone()).await;
        global_workspace.subscribe(goal_engine_adapter.clone()).await;
        global_workspace.subscribe(attention_adapter.clone()).await;
        global_workspace.subscribe(memory_adapter.clone()).await;
        global_workspace.subscribe(narrative_adapter.clone()).await;

        let episodic_memory = Arc::new(EpisodicMemory::new(config.episodic_capacity));
        let autobiographical_memory = Arc::new(AutobiographicalMemory::new(config.agent_name.clone()));
        let reconsolidator = Arc::new(MemoryReconsolidator::new());
        let schema_library = Arc::new(SchemaLibrary::new());
        let forgetting = Arc::new(AdaptiveForgetting::new(ForgettingConfig::default()));

        let theory_of_mind = Arc::new(TheoryOfMind::new());

        let engine = Self {
            config,
            global_workspace,
            coalition_formation,
            phenomenal_binder,
            consciousness_meter,
            narrative_self,
            qualia_model,
            reasoning_adapter,
            meta_cognition_adapter,
            goal_engine_adapter,
            attention_adapter,
            memory_adapter,
            narrative_adapter,
            episodic_memory,
            autobiographical_memory,
            reconsolidator,
            schema_library,
            forgetting,
            theory_of_mind,
            cycle_count: Arc::new(tokio::sync::RwLock::new(0)),
            forgetting_cycles_run: Arc::new(tokio::sync::RwLock::new(0)),
        };

        // Record activation in autobiographical memory
        engine.autobiographical_memory.record_activation().await;
        engine.narrative_self.record_milestone("Phase 3 Activated", "Consciousness substrate initialized").await;

        info!("Phase3Engine: fully initialized with 6 cognitive module adapters");
        engine
    }

    /// Run one full consciousness cycle.
    ///
    /// Steps:
    /// 1. GWT broadcast competition
    /// 2. Phenomenal binding of active streams
    /// 3. Phi measurement
    /// 4. Narrative update
    /// 5. Qualia derivation
    /// 6. Periodic reconsolidation + forgetting
    pub async fn run_cycle(&self) -> ConsciousnessReport {
        let cycle = {
            let mut c = self.cycle_count.write().await;
            *c += 1;
            *c
        };

        // Step 1: GWT broadcast competition
        let broadcast = self.global_workspace.run_cycle().await;

        // Step 2: Phenomenal binding
        let streams = self.collect_streams().await;
        let bound_exp = self.phenomenal_binder.bind(streams).await;

        // Step 3: Phi measurement
        let ws_stats = self.global_workspace.get_stats().await;
        let narrative_stats = self.narrative_self.get_stats().await;
        let tom_stats = self.theory_of_mind.get_stats().await;
        let qualia_state = self.qualia_model.get_state().await;

        let components = ConsciousnessMeter::build_components(
            ws_stats.current_phi,
            ws_stats.subscriber_count,
            6, // total registered adapters
            ws_stats.total_broadcasts,
            bound_exp.binding_strength,
            narrative_stats.total_entries,
            self.config.enable_tom && tom_stats.agents_modeled > 0,
            qualia_state.experiential_richness,
        );

        let phi_estimate = self.consciousness_meter.measure(components, cycle).await;

        // Step 4: Narrative update from winning broadcast
        if let Some(ref bc) = broadcast {
            if phi_estimate.phi >= self.config.min_phi_for_broadcast {
                self.narrative_self.narrate(
                    &format!("Consciousness broadcast #{}: {}", cycle, &bc.content.data[..bc.content.data.len().min(120)]),
                    NarrativeType::CurrentState,
                    phi_estimate.phi,
                ).await;

                self.narrative_adapter.set_narrative(
                    &bc.content.data[..bc.content.data.len().min(100)],
                    phi_estimate.phi,
                ).await;
            }
        }

        // Step 5: Derive qualia from cycle outcome
        let novelty = qualia_state.experiential_richness;
        let effort = ws_stats.current_phi;
        if phi_estimate.level != ConsciousnessLevel::Dormant {
            if effort > 0.7 {
                // High phi = heavy cognitive integration load → Strain
                self.qualia_model.experience(QualiaType::Strain, effort, "gwt_cycle").await;
            } else if effort > 0.0 && effort < 0.5 {
                // Low-to-moderate effort with active consciousness → Flow
                self.qualia_model.experience(QualiaType::Flow, phi_estimate.phi * 0.7, "gwt_cycle").await;
            }
        }
        if novelty > 0.5 {
            self.qualia_model.experience(QualiaType::Novelty, novelty, "gwt_cycle").await;
        }

        // Step 6: Periodic reconsolidation
        if cycle % self.config.reconsolidation_interval_cycles == 0 {
            self.run_reconsolidation().await;
        }

        // Step 7: Periodic forgetting
        if cycle % self.config.forgetting_interval_cycles == 0 {
            self.forgetting.run_forgetting_cycle(&self.episodic_memory).await;
            *self.forgetting_cycles_run.write().await += 1;
        }

        // Build report
        let winning_coalition = broadcast.as_ref().map(|b| b.winning_coalition_id.clone());
        let ep_stats = self.episodic_memory.get_stats().await;
        let narrative_stats2 = self.narrative_self.get_stats().await;
        let tom_stats2 = self.theory_of_mind.get_stats().await;
        let qualia_state2 = self.qualia_model.get_state().await;
        let current_narrative = self.narrative_self.get_recent_narrative(3).await;

        ConsciousnessReport {
            timestamp: Utc::now(),
            phi: phi_estimate.phi,
            level: phi_estimate.level.label().to_string(),
            winning_coalition,
            modules_active: ws_stats.subscriber_count,
            episodic_memories: ep_stats.total_episodes,
            narrative_entries: narrative_stats2.total_entries,
            agents_modeled: tom_stats2.agents_modeled,
            current_narrative,
            dominant_qualia: qualia_state2.dominant.as_ref().map(|q| q.label().to_string()),
        }
    }

    /// Update the active goal context (feeds into goal_engine adapter + narrative).
    pub async fn set_active_goal(&self, goal: &str, urgency: f64) {
        self.goal_engine_adapter.set_active_goal(goal, urgency, urgency * 0.9).await;
        self.reasoning_adapter.set_active_reasoning(&format!("achieving: {}", goal), urgency * 0.8).await;
        self.narrative_self.narrate(
            &format!("Pursuing goal: {}", goal),
            NarrativeType::Intention,
            urgency,
        ).await;
        self.qualia_model.experience(QualiaType::Significance, urgency * 0.8, "goal_set").await;
    }

    /// Begin recording an episodic memory.
    pub async fn begin_episode(&self, goal: Option<String>) -> String {
        let id = self.episodic_memory.begin_episode(goal.clone(), "conscious").await;
        if let Some(ref g) = goal {
            self.episodic_memory.record_event(
                EpisodicEventType::GoalSet,
                &format!("Episode began for goal: {}", g),
                0.5,
            ).await;
        }
        id
    }

    /// Close the current episode with an outcome.
    pub async fn end_episode(&self, success: bool, emotional_tag: EmotionalTag) -> Option<String> {
        let event_type = if success { EpisodicEventType::GoalAchieved } else { EpisodicEventType::GoalFailed };
        let event_desc = if success { "episode completed successfully" } else { "episode ended with failure" };
        self.episodic_memory.record_event(event_type, event_desc, 0.7).await;

        let id = self.episodic_memory.end_episode(emotional_tag.clone(), success).await;

        // Qualia from outcome
        if success {
            self.qualia_model.experience(QualiaType::Satisfaction, 0.8, "episode_end").await;
        } else {
            self.qualia_model.experience(QualiaType::Frustration, 0.5, "episode_end").await;
        }

        // Narrative entry
        let outcome_str = if success { "successfully" } else { "unsuccessfully" };
        self.narrative_self.narrate_with_emotion(
            &format!("Episode concluded {}", outcome_str),
            NarrativeType::PastAction,
            0.6,
            if success { 0.6 } else { -0.3 },
        ).await;

        id
    }

    /// Observe an action from another agent (feeds ToM).
    pub async fn observe_agent_action(&self, agent_id: &str, action: &str, context: &str) {
        if !self.config.enable_tom {
            return;
        }
        let obs = ObservedAction {
            timestamp: Utc::now(),
            action: action.to_string(),
            context: context.to_string(),
            explicit_statement: None,
        };
        self.theory_of_mind.observe_action(agent_id, &obs).await;
        self.meta_cognition_adapter
            .set_assessment(&format!("Modeling agent '{}': {}", agent_id, action), 0.6)
            .await;
    }

    /// Run a dream/reconsolidation cycle.
    pub async fn run_reconsolidation(&self) {
        let active_goals = {
            let goal = self.goal_engine_adapter.active_goal.read().await;
            goal.as_ref().map(|g| vec![g.clone()]).unwrap_or_default()
        };

        let _ = self.reconsolidator
            .reconsolidate(
                &self.episodic_memory,
                &self.schema_library,
                &active_goals,
                ReconsolidationTrigger::DreamCycle,
            )
            .await;

        self.narrative_self.narrate(
            "Dream/reconsolidation cycle completed — memories consolidated",
            NarrativeType::Observation,
            0.4,
        ).await;
    }

    /// Record a cognitive event into episodic memory and derive qualia.
    pub async fn record_cognitive_event(
        &self,
        event_type: EpisodicEventType,
        description: &str,
        significance: f64,
        success: bool,
        novelty: f64,
        effort: f64,
    ) {
        let event_name = format!("{:?}", event_type).to_lowercase();
        self.episodic_memory.record_event(event_type, description, significance).await;
        self.memory_adapter.set_retrieval(description, significance).await;
        self.qualia_model.derive_from_event(&event_name, success, novelty, effort).await;
    }

    /// Get a full consciousness report.
    pub async fn get_consciousness_report(&self) -> ConsciousnessReport {
        let ws_stats = self.global_workspace.get_stats().await;
        let phi = self.consciousness_meter.get_phi().await;
        let level = self.consciousness_meter.get_level().await;
        let ep_stats = self.episodic_memory.get_stats().await;
        let narrative_stats = self.narrative_self.get_stats().await;
        let tom_stats = self.theory_of_mind.get_stats().await;
        let qualia_state = self.qualia_model.get_state().await;
        let current_narrative = self.narrative_self.get_recent_narrative(3).await;

        ConsciousnessReport {
            timestamp: Utc::now(),
            phi,
            level: level.label().to_string(),
            winning_coalition: self.global_workspace.get_current_broadcast().await
                .map(|b| b.winning_coalition_id),
            modules_active: ws_stats.subscriber_count,
            episodic_memories: ep_stats.total_episodes,
            narrative_entries: narrative_stats.total_entries,
            agents_modeled: tom_stats.agents_modeled,
            current_narrative,
            dominant_qualia: qualia_state.dominant.as_ref().map(|q| q.label().to_string()),
        }
    }

    /// Get comprehensive Phase 3 statistics.
    pub async fn get_stats(&self) -> Phase3Stats {
        let ws_stats = self.global_workspace.get_stats().await;
        let phi_stats = self.consciousness_meter.get_stats().await;
        let ep_stats = self.episodic_memory.get_stats().await;
        let auto_stats = self.autobiographical_memory.get_stats().await;
        let tom_stats = self.theory_of_mind.get_stats().await;
        let schema_stats = self.schema_library.get_stats().await;
        let recon_stats = self.reconsolidator.get_stats().await;
        let qualia_stats = self.qualia_model.get_stats().await;
        let narrative_stats = self.narrative_self.get_stats().await;
        let forgetting_cycles = *self.forgetting_cycles_run.read().await;

        Phase3Stats {
            gwt_cycles: ws_stats.total_cycles,
            current_phi: phi_stats.current_phi,
            peak_phi: phi_stats.peak_phi,
            consciousness_level: phi_stats.current_level,
            episodic_memories: ep_stats.total_episodes,
            life_events: auto_stats.total_life_events,
            agents_modeled: tom_stats.agents_modeled,
            schemas_extracted: schema_stats.total_schemas,
            reconsolidation_cycles: recon_stats.total_cycles,
            forgetting_cycles_run: forgetting_cycles,
            qualia_experienced: qualia_stats.total_qualia,
            narrative_entries: narrative_stats.total_entries,
        }
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    async fn collect_streams(&self) -> Vec<ExperienceStream> {
        use super::global_workspace::{ContentModality, ContentType};
        use super::phenomenal_binding::ExperienceStream;

        let mut streams = Vec::new();
        let now = Utc::now();

        /// Compute milliseconds elapsed since a module's last_updated timestamp.
        /// Returns 0 if the module was just updated (< 1ms ago), or a large value
        /// if it has never been updated (treated as stale / out-of-window).
        fn age_ms(last_updated: Option<DateTime<Utc>>, now: DateTime<Utc>) -> i64 {
            last_updated
                .map(|t| (now - t).num_milliseconds().max(0))
                .unwrap_or(i64::MAX)
        }

        // Stream from goal engine
        let goal = self.goal_engine_adapter.active_goal.read().await;
        let goal_age = age_ms(*self.goal_engine_adapter.last_updated.read().await, now);
        if let Some(ref g) = *goal {
            streams.push(ExperienceStream {
                source: "goal_engine".to_string(),
                content: CognitiveContent {
                    content_type: ContentType::Goal,
                    data: g.clone(),
                    embedding: vec![],
                    salience: *self.goal_engine_adapter.goal_priority.read().await,
                    modality: ContentModality::Linguistic,
                },
                weight: 0.8,
                temporal_offset_ms: goal_age,
            });
        }
        drop(goal);

        // Stream from reasoning
        let topic = self.reasoning_adapter.current_topic.read().await;
        let reasoning_age = age_ms(*self.reasoning_adapter.last_updated.read().await, now);
        if let Some(ref t) = *topic {
            streams.push(ExperienceStream {
                source: "reasoning_engine".to_string(),
                content: CognitiveContent {
                    content_type: ContentType::Reasoning,
                    data: t.clone(),
                    embedding: vec![],
                    salience: *self.reasoning_adapter.current_confidence.read().await,
                    modality: ContentModality::Linguistic,
                },
                weight: 0.75,
                temporal_offset_ms: reasoning_age,
            });
        }
        drop(topic);

        // Stream from memory
        let retrieval = self.memory_adapter.recent_retrieval.read().await;
        let memory_age = age_ms(*self.memory_adapter.last_updated.read().await, now);
        if let Some(ref r) = *retrieval {
            streams.push(ExperienceStream {
                source: "memory".to_string(),
                content: CognitiveContent {
                    content_type: ContentType::Memory,
                    data: r.clone(),
                    embedding: vec![],
                    salience: *self.memory_adapter.retrieval_relevance.read().await,
                    modality: ContentModality::Linguistic,
                },
                weight: 0.6,
                temporal_offset_ms: memory_age,
            });
        }
        drop(retrieval);

        // Stream from narrative
        let narrative_entry = self.narrative_adapter.last_entry.read().await;
        let narrative_age = age_ms(*self.narrative_adapter.last_updated.read().await, now);
        if let Some(ref n) = *narrative_entry {
            streams.push(ExperienceStream {
                source: "narrative_self".to_string(),
                content: CognitiveContent {
                    content_type: ContentType::Narrative,
                    data: n.clone(),
                    embedding: vec![],
                    salience: *self.narrative_adapter.narrative_coherence.read().await,
                    modality: ContentModality::Linguistic,
                },
                weight: 0.5,
                temporal_offset_ms: narrative_age,
            });
        }
        drop(narrative_entry);

        streams
    }
}

// ── Re-export CognitiveContent for use in collect_streams ─────────────────────
use super::global_workspace::CognitiveContent;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_phase3_engine_init() {
        let engine = Phase3Engine::new(Phase3Config::default()).await;
        let stats = engine.get_stats().await;
        assert_eq!(stats.gwt_cycles, 0);
        assert!(stats.life_events > 0); // activation recorded
    }

    #[tokio::test]
    async fn test_phase3_cycle() {
        let engine = Phase3Engine::new(Phase3Config::default()).await;
        engine.set_active_goal("test goal", 0.7).await;
        let report = engine.run_cycle().await;
        assert!(report.modules_active > 0);
        assert!(report.phi >= 0.0);
    }

    #[tokio::test]
    async fn test_episodic_memory_integration() {
        let engine = Phase3Engine::new(Phase3Config::default()).await;
        engine.begin_episode(Some("integrate memory".to_string())).await;
        engine.record_cognitive_event(
            EpisodicEventType::ReasoningStep,
            "reasoning about integration",
            0.8, true, 0.6, 0.3,
        ).await;
        engine.end_episode(true, EmotionalTag::positive(0.7)).await;

        let stats = engine.get_stats().await;
        assert_eq!(stats.episodic_memories, 1);
    }

    #[tokio::test]
    async fn test_theory_of_mind_integration() {
        let engine = Phase3Engine::new(Phase3Config::default()).await;
        engine.observe_agent_action("user-001", "build a web app", "chat").await;

        let predictions = engine.theory_of_mind.predict_next_action("user-001").await;
        assert!(!predictions.is_empty());

        let stats = engine.get_stats().await;
        assert_eq!(stats.agents_modeled, 1);
    }
}

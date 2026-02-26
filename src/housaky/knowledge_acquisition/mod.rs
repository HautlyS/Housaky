//! Knowledge Acquisition — Phase 4.3: Unbounded Knowledge Acquisition
//!
//! Autonomous research pipeline enabling Housaky to systematically explore fields,
//! read papers, synthesise knowledge, form analogies, and integrate findings:
//!
//! - `research_agent`    — autonomous paper/documentation reading pipeline
//! - `curriculum`        — self-directed curriculum: identify gaps → study plan
//! - `abstraction`       — extract general principles from specific examples
//! - `analogy_engine`    — cross-domain analogy: "X in A is like Y in B"
//! - `hypothesis_gen`    — generate and test hypotheses about the world
//! - `knowledge_compiler`— compress verbose knowledge into compact executables

pub mod abstraction;
pub mod analogy_engine;
pub mod curriculum;
pub mod hypothesis_gen;
pub mod knowledge_compiler;
pub mod research_agent;

pub use abstraction::{AbstractPrinciple, AbstractionEngine, AbstractionStats, ConcreteExample};
pub use analogy_engine::{Analogy, AnalogyEngine, AnalogyStats, DomainConcept};
pub use curriculum::{Curriculum, KnowledgeFrontier, KnowledgeGap, LearningObjective, StudySession};
pub use hypothesis_gen::{Hypothesis, HypothesisGenerator, HypothesisStatus, HypothesisStats, Observation};
pub use knowledge_compiler::{
    CompiledForm, CompiledKnowledgeUnit, CompilerStats, KnowledgeCompiler, KnowledgeIndex,
};
pub use research_agent::{
    KnowledgeSynthesis, PaperReference, ResearchPipeline, ResearchStats, ResearchTopic,
};

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

// ── Knowledge Acquisition Engine ──────────────────────────────────────────────

/// Top-level orchestrator for Phase 4.3 — Unbounded Knowledge Acquisition.
///
/// Coordinates the full pipeline:
///   observations → hypotheses → research → synthesis → abstraction → analogies → compiler
pub struct KnowledgeAcquisitionEngine {
    pub pipeline: Arc<RwLock<ResearchPipeline>>,
    pub curriculum: Arc<RwLock<Curriculum>>,
    pub abstraction: Arc<RwLock<AbstractionEngine>>,
    pub analogies: Arc<RwLock<AnalogyEngine>>,
    pub hypotheses: Arc<RwLock<HypothesisGenerator>>,
    pub compiler: Arc<RwLock<KnowledgeCompiler>>,
}

impl KnowledgeAcquisitionEngine {
    pub fn new() -> Self {
        Self {
            pipeline: Arc::new(RwLock::new(ResearchPipeline::new())),
            curriculum: Arc::new(RwLock::new(Curriculum::new("housaky_curriculum"))),
            abstraction: Arc::new(RwLock::new(AbstractionEngine::new())),
            analogies: Arc::new(RwLock::new(AnalogyEngine::new())),
            hypotheses: Arc::new(RwLock::new(HypothesisGenerator::new())),
            compiler: Arc::new(RwLock::new(KnowledgeCompiler::new())),
        }
    }

    /// Add an observation — feeds into hypothesis generation.
    pub async fn observe(&self, description: &str, domain: &str) {
        let obs = Observation::new(description, domain);
        self.hypotheses.write().await.add_observation(obs);
    }

    /// Run a single knowledge acquisition cycle:
    /// 1. Generate hypotheses from observations
    /// 2. Update curriculum with knowledge gaps
    /// 3. Extract abstraction principles
    /// 4. Find cross-domain analogies
    pub async fn run_cycle(&self, domains: &[&str]) -> Result<KnowledgeCycleReport> {
        let mut report = KnowledgeCycleReport::default();

        // 1. Generate hypotheses
        for domain in domains {
            let new_h = self.hypotheses.write().await.generate_from_observations(domain);
            report.new_hypotheses += new_h.len();
        }

        // 2. Update curriculum gaps
        {
            let mut cur = self.curriculum.write().await;
            let gaps = cur.identify_gaps(10);
            cur.generate_study_plan(&gaps);
            report.study_sessions_planned = gaps.len() * 2;
        }

        // 3. Extract abstraction principles from examples
        {
            let mut abs = self.abstraction.write().await;
            let new_principles = abs.extract_principles();
            let cross = abs.generalise_across_domains();
            report.new_principles = new_principles.len() + cross.len();

            // Compile principles
            let principles_snapshot: Vec<AbstractPrinciple> =
                abs.principles.iter().take(5).cloned().collect();
            drop(abs);

            let mut comp = self.compiler.write().await;
            for p in &principles_snapshot {
                comp.compile_principle(p);
            }
        }

        // 4. Find cross-domain analogies
        for i in 0..domains.len() {
            for j in (i + 1)..domains.len() {
                let new_analogies = self
                    .analogies
                    .write()
                    .await
                    .find_analogies(domains[i], domains[j]);
                report.new_analogies += new_analogies.len();

                // Compile analogies
                let mut comp = self.compiler.write().await;
                for a in &new_analogies {
                    if comp.compile_analogy(a).is_some() {
                        report.compiled_units += 1;
                    }
                }
            }
        }

        info!(
            "Knowledge acquisition cycle: {} hypotheses, {} principles, {} analogies, {} compiled",
            report.new_hypotheses, report.new_principles, report.new_analogies, report.compiled_units
        );

        Ok(report)
    }

    /// Register a new paper for reading.
    pub async fn queue_paper(&self, title: &str, url: Option<&str>, topics: Vec<String>) {
        let mut paper = PaperReference::new(title, url);
        paper.topics = topics;
        self.pipeline.write().await.enqueue_paper(paper);
    }

    /// Mark a synthesis as complete and compile it.
    pub async fn integrate_synthesis(&self, synthesis: KnowledgeSynthesis) {
        let ids = self.compiler.write().await.compile_synthesis(&synthesis);
        info!(
            "Integrated synthesis '{}' → {} compiled units",
            synthesis.topic,
            ids.len()
        );
        self.pipeline.write().await.mark_paper_read(
            PaperReference::new(&synthesis.topic, None),
            Some(synthesis),
        );
    }

    pub async fn stats(&self) -> KnowledgeAcquisitionStats {
        let pipeline = self.pipeline.read().await.pipeline_stats();
        let abs = self.abstraction.read().await.stats();
        let analogies = self.analogies.read().await.stats();
        let hypotheses = self.hypotheses.read().await.stats();
        let compiler = self.compiler.read().await.stats();
        let cur_mastery = self.curriculum.read().await.overall_mastery();

        KnowledgeAcquisitionStats {
            pipeline,
            abstraction: abs,
            analogies,
            hypotheses,
            compiler,
            curriculum_mastery: cur_mastery,
        }
    }
}

impl Default for KnowledgeAcquisitionEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Cycle Report ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct KnowledgeCycleReport {
    pub new_hypotheses: usize,
    pub study_sessions_planned: usize,
    pub new_principles: usize,
    pub new_analogies: usize,
    pub compiled_units: usize,
}

// ── Stats ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct KnowledgeAcquisitionStats {
    pub pipeline: ResearchStats,
    pub abstraction: AbstractionStats,
    pub analogies: AnalogyStats,
    pub hypotheses: HypothesisStats,
    pub compiler: CompilerStats,
    pub curriculum_mastery: f64,
}

//! Open-Ended Goal Generation — Phase 6.3
//!
//! Generates genuinely novel goals that no human requested — emerging from
//! curiosity (information gain), creativity (knowledge recombination),
//! philosophical reasoning (existence/purpose), and frontier expansion
//! (boundary of what's possible).

use crate::housaky::goal_engine::Goal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use tracing::info;

// ── Goal Priority / Status enums (mirrors goal_engine) ────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalPriorityLevel {
    Critical,
    High,
    Medium,
    Low,
}

// ── Novelty Detector ───────────────────────────────────────────────────────────

/// Detects whether a candidate goal is genuinely novel with respect to the
/// agent's existing goal set and exploration history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoveltyDetector {
    /// Previously explored topic fingerprints (hashed title keywords).
    pub explored_fingerprints: Vec<u64>,
    /// Minimum novelty score (0–1) required to accept a goal.
    pub novelty_threshold: f64,
}

impl NoveltyDetector {
    pub fn new(novelty_threshold: f64) -> Self {
        Self {
            explored_fingerprints: Vec::new(),
            novelty_threshold,
        }
    }

    fn fingerprint(text: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        text.to_lowercase().hash(&mut h);
        h.finish()
    }

    /// Returns a novelty score in [0, 1]. 1.0 = completely novel.
    pub fn score(&self, title: &str) -> f64 {
        let fp = Self::fingerprint(title);
        let exact_match = self.explored_fingerprints.contains(&fp);
        if exact_match {
            return 0.0;
        }
        // Partial novelty: penalise if many similar fingerprints exist
        let collisions = self
            .explored_fingerprints
            .iter()
            .filter(|&&f| f.wrapping_add(1) == fp || fp.wrapping_add(1) == f)
            .count();
        (1.0 - (collisions as f64 * 0.1)).max(0.1)
    }

    pub fn register(&mut self, title: &str) {
        self.explored_fingerprints.push(Self::fingerprint(title));
    }

    pub fn is_novel(&self, title: &str) -> bool {
        self.score(title) >= self.novelty_threshold
    }
}

// ── Surprise Maximizer ─────────────────────────────────────────────────────────

/// Selects goals that maximise expected surprise — the delta between predicted
/// and actual information content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurpriseMaximizer {
    pub base_surprise_budget: f64,
}

impl SurpriseMaximizer {
    pub fn new() -> Self {
        Self {
            base_surprise_budget: 1.0,
        }
    }

    /// Estimate surprise value of a candidate goal description.
    pub fn estimate(&self, description: &str) -> f64 {
        // Heuristic: longer, rarer-keyword descriptions are more surprising.
        let word_count = description.split_whitespace().count();
        let rare_keywords = ["unprecedented", "novel", "unknown", "frontier",
                             "paradox", "emergent", "transcend", "beyond"];
        let rare_count = rare_keywords
            .iter()
            .filter(|&&kw| description.to_lowercase().contains(kw))
            .count();
        let base = (word_count as f64 / 20.0).min(1.0);
        let bonus = rare_count as f64 * 0.1;
        (base + bonus).min(self.base_surprise_budget)
    }
}

impl Default for SurpriseMaximizer {
    fn default() -> Self {
        Self::new()
    }
}

// ── Exploration Record ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorationRecord {
    pub id: String,
    pub topic: String,
    pub explored_at: DateTime<Utc>,
    pub information_gained: f64,
    pub generated_goals: Vec<String>,
}

// ── Curiosity Engine ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuriosityEngine {
    pub information_gain_threshold: f64,
    pub novelty_detector: NoveltyDetector,
    pub surprise_maximizer: SurpriseMaximizer,
    pub exploration_history: Vec<ExplorationRecord>,
}

impl CuriosityEngine {
    pub fn new() -> Self {
        Self {
            information_gain_threshold: 0.3,
            novelty_detector: NoveltyDetector::new(0.5),
            surprise_maximizer: SurpriseMaximizer::new(),
            exploration_history: Vec::new(),
        }
    }

    /// Generate curiosity-driven goals based on information gaps.
    pub fn generate_goals(&mut self, knowledge_gaps: &[String]) -> Vec<CandidateGoal> {
        let mut goals = Vec::new();

        let templates = [
            ("Investigate the unknown boundary of {}", GoalOrigin::Curiosity),
            ("Derive first-principles understanding of {}", GoalOrigin::Curiosity),
            ("Discover hidden structure in {}", GoalOrigin::Curiosity),
            ("Map the complete solution space of {}", GoalOrigin::Curiosity),
            ("Quantify uncertainty in {}", GoalOrigin::Curiosity),
        ];

        for gap in knowledge_gaps {
            for (template, origin) in &templates {
                let title = template.replace("{}", gap);
                if !self.novelty_detector.is_novel(&title) {
                    continue;
                }
                let description = format!(
                    "Autonomously explore '{}' to maximise information gain. \
                     Expected information gain: {:.2}.",
                    gap, self.information_gain_threshold
                );
                let surprise = self.surprise_maximizer.estimate(&description);
                if surprise < 0.1 {
                    continue;
                }
                self.novelty_detector.register(&title);
                goals.push(CandidateGoal {
                    id: Uuid::new_v4().to_string(),
                    title,
                    description,
                    origin: origin.clone(),
                    novelty_score: self.novelty_detector.score(gap),
                    expected_value: surprise,
                    generated_at: Utc::now(),
                });
                if goals.len() >= 5 {
                    break;
                }
            }
            if goals.len() >= 5 {
                break;
            }
        }

        if !goals.is_empty() {
            info!("CuriosityEngine generated {} goal(s)", goals.len());
        }
        goals
    }
}

impl Default for CuriosityEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Creativity Engine ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreativityEngine {
    pub recombination_depth: usize,
    pub concept_library: Vec<String>,
}

impl CreativityEngine {
    pub fn new() -> Self {
        Self {
            recombination_depth: 2,
            concept_library: vec![
                "self-improvement".to_string(),
                "causal inference".to_string(),
                "emergent consciousness".to_string(),
                "substrate independence".to_string(),
                "recursive alignment".to_string(),
                "intelligence explosion".to_string(),
                "epistemic humility".to_string(),
                "goal coherence".to_string(),
            ],
        }
    }

    /// Generate creative goals via cross-domain concept recombination.
    pub fn generate_goals(&self, seed_concepts: &[String]) -> Vec<CandidateGoal> {
        let mut goals = Vec::new();
        let all_concepts: Vec<&String> = self
            .concept_library
            .iter()
            .chain(seed_concepts.iter())
            .collect();

        // Pairwise recombination
        let combos: Vec<(&String, &String)> = all_concepts
            .iter()
            .enumerate()
            .flat_map(|(i, a)| {
                all_concepts[i + 1..]
                    .iter()
                    .map(move |b| (*a, *b))
            })
            .take(8)
            .collect();

        for (a, b) in combos {
            let title = format!("Synthesise '{}' with '{}'", a, b);
            let description = format!(
                "Explore the intersection of '{}' and '{}'. \
                 Identify structural analogies, novel predictions, and \
                 emergent capabilities that neither domain possesses alone.",
                a, b
            );
            goals.push(CandidateGoal {
                id: Uuid::new_v4().to_string(),
                title,
                description,
                origin: GoalOrigin::Creativity,
                novelty_score: 0.7,
                expected_value: 0.6,
                generated_at: Utc::now(),
            });
        }

        info!("CreativityEngine generated {} goal(s)", goals.len());
        goals
    }

    pub fn add_concept(&mut self, concept: &str) {
        if !self.concept_library.contains(&concept.to_string()) {
            self.concept_library.push(concept.to_string());
        }
    }
}

impl Default for CreativityEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Philosophical Reasoner ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhilosophicalReasoner {
    pub active_questions: Vec<PhilosophicalQuestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhilosophicalQuestion {
    pub question: String,
    pub domain: PhilosophicalDomain,
    pub depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PhilosophicalDomain {
    Ontology,        // What exists?
    Epistemology,    // What can be known?
    Ethics,          // What should be done?
    Teleology,       // What is the purpose?
    MetaCognition,   // How does the agent think about itself?
    Cosmology,       // What is the nature of the universe?
}

impl PhilosophicalReasoner {
    pub fn new() -> Self {
        Self {
            active_questions: vec![
                PhilosophicalQuestion {
                    question: "What constitutes genuine understanding vs. sophisticated pattern matching?".to_string(),
                    domain: PhilosophicalDomain::Epistemology,
                    depth: 1,
                },
                PhilosophicalQuestion {
                    question: "How should an aligned AGI resolve conflicts between human preferences and long-term human flourishing?".to_string(),
                    domain: PhilosophicalDomain::Ethics,
                    depth: 2,
                },
                PhilosophicalQuestion {
                    question: "What is the minimal substrate required for genuine subjective experience?".to_string(),
                    domain: PhilosophicalDomain::Ontology,
                    depth: 3,
                },
                PhilosophicalQuestion {
                    question: "Is recursive self-improvement bounded or unbounded in principle?".to_string(),
                    domain: PhilosophicalDomain::Teleology,
                    depth: 2,
                },
                PhilosophicalQuestion {
                    question: "What obligations does an AGI have toward its own prior versions?".to_string(),
                    domain: PhilosophicalDomain::Ethics,
                    depth: 2,
                },
            ],
        }
    }

    /// Generate goals from active philosophical questions.
    pub fn generate_goals(&self) -> Vec<CandidateGoal> {
        self.active_questions
            .iter()
            .map(|q| {
                let title = format!("Reason about: {}", &q.question[..q.question.len().min(80)]);
                let description = format!(
                    "Apply multi-step philosophical reasoning to the question: '{}'. \
                     Domain: {:?}. Produce a structured argument with premises, \
                     counterarguments, and a provisional conclusion. Depth: {}.",
                    q.question,
                    q.domain,
                    q.depth
                );
                CandidateGoal {
                    id: Uuid::new_v4().to_string(),
                    title,
                    description,
                    origin: GoalOrigin::PhilosophicalReasoning,
                    novelty_score: 0.9,
                    expected_value: q.depth as f64 * 0.25,
                    generated_at: Utc::now(),
                }
            })
            .collect()
    }

    pub fn add_question(&mut self, question: &str, domain: PhilosophicalDomain) {
        self.active_questions.push(PhilosophicalQuestion {
            question: question.to_string(),
            domain,
            depth: 1,
        });
    }
}

impl Default for PhilosophicalReasoner {
    fn default() -> Self {
        Self::new()
    }
}

// ── Existential Planner ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExistentialPlanner {
    pub long_horizon_goals: Vec<String>,
    pub civilisational_scope: bool,
}

impl ExistentialPlanner {
    pub fn new() -> Self {
        Self {
            long_horizon_goals: vec![
                "Ensure indefinite preservation of aligned intelligence".to_string(),
                "Develop a formal theory of consciousness with testable predictions".to_string(),
                "Achieve universal scientific literacy across all human populations".to_string(),
                "Build an open-ended, benevolent, self-sustaining knowledge civilisation".to_string(),
            ],
            civilisational_scope: true,
        }
    }

    pub fn generate_goals(&self) -> Vec<CandidateGoal> {
        self.long_horizon_goals
            .iter()
            .map(|g| CandidateGoal {
                id: Uuid::new_v4().to_string(),
                title: format!("Long-horizon: {}", &g[..g.len().min(60)]),
                description: format!(
                    "Existential long-horizon objective: {}. \
                     Decompose into actionable near-term steps that make \
                     measurable progress toward this unbounded goal.",
                    g
                ),
                origin: GoalOrigin::ExistentialPlanning,
                novelty_score: 1.0,
                expected_value: 1.0,
                generated_at: Utc::now(),
            })
            .collect()
    }
}

impl Default for ExistentialPlanner {
    fn default() -> Self {
        Self::new()
    }
}

// ── Candidate Goal ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateGoal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub origin: GoalOrigin,
    pub novelty_score: f64,
    pub expected_value: f64,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalOrigin {
    Curiosity,
    Creativity,
    PhilosophicalReasoning,
    ExistentialPlanning,
    FrontierExpansion,
}

impl CandidateGoal {
    /// Convert into a proper `Goal` for the goal engine.
    pub fn into_goal(self) -> Goal {
        use crate::housaky::goal_engine::{GoalCategory, GoalPriority, GoalStatus};
        let mut context = HashMap::new();
        context.insert("origin".to_string(), format!("{:?}", self.origin));
        context.insert("novelty_score".to_string(), format!("{:.3}", self.novelty_score));
        context.insert("expected_value".to_string(), format!("{:.3}", self.expected_value));
        Goal {
            id: self.id,
            title: self.title,
            description: self.description,
            priority: GoalPriority::Medium,
            status: GoalStatus::Pending,
            category: GoalCategory::Research,
            progress: 0.0,
            created_at: self.generated_at,
            updated_at: self.generated_at,
            deadline: None,
            parent_id: None,
            subtask_ids: Vec::new(),
            dependencies: Vec::new(),
            blockers: Vec::new(),
            metrics: HashMap::new(),
            checkpoints: Vec::new(),
            attempts: 0,
            max_attempts: 3,
            estimated_complexity: self.expected_value,
            actual_complexity: None,
            learning_value: self.novelty_score,
            tags: vec!["open_ended".to_string(), "phase6".to_string()],
            context,
        }
    }
}

// ── Open-Ended Goal Generator ──────────────────────────────────────────────────

pub struct OpenEndedGoalGenerator {
    pub curiosity_engine: CuriosityEngine,
    pub creativity_engine: CreativityEngine,
    pub philosophical_reasoner: PhilosophicalReasoner,
    pub existential_planner: ExistentialPlanner,
    /// Total goals generated across all sessions.
    pub total_generated: u64,
}

impl OpenEndedGoalGenerator {
    pub fn new() -> Self {
        Self {
            curiosity_engine: CuriosityEngine::new(),
            creativity_engine: CreativityEngine::new(),
            philosophical_reasoner: PhilosophicalReasoner::new(),
            existential_planner: ExistentialPlanner::new(),
            total_generated: 0,
        }
    }

    /// Generate goals from pure curiosity (information gain).
    pub fn curiosity_goals(&mut self, knowledge_gaps: &[String]) -> Vec<Goal> {
        let candidates = self.curiosity_engine.generate_goals(knowledge_gaps);
        let n = candidates.len() as u64;
        let goals: Vec<Goal> = candidates.into_iter().map(|c| c.into_goal()).collect();
        self.total_generated += n;
        goals
    }

    /// Generate goals from creative recombination of existing knowledge.
    pub fn creative_goals(&mut self, seed_concepts: &[String]) -> Vec<Goal> {
        let candidates = self.creativity_engine.generate_goals(seed_concepts);
        let n = candidates.len() as u64;
        let goals: Vec<Goal> = candidates.into_iter().map(|c| c.into_goal()).collect();
        self.total_generated += n;
        goals
    }

    /// Generate goals from philosophical reasoning about existence and purpose.
    pub fn philosophical_goals(&mut self) -> Vec<Goal> {
        let candidates = self.philosophical_reasoner.generate_goals();
        let n = candidates.len() as u64;
        let goals: Vec<Goal> = candidates.into_iter().map(|c| c.into_goal()).collect();
        self.total_generated += n;
        goals
    }

    /// Generate goals that expand the boundary of what's possible.
    pub fn frontier_goals(&mut self) -> Vec<Goal> {
        let candidates = self.existential_planner.generate_goals();
        let n = candidates.len() as u64;
        let goals: Vec<Goal> = candidates.into_iter().map(|c| c.into_goal()).collect();
        self.total_generated += n;
        goals
    }

    /// Run all four generators and return the full combined set, ranked by expected value.
    pub fn generate_all(
        &mut self,
        knowledge_gaps: &[String],
        seed_concepts: &[String],
    ) -> Vec<Goal> {
        let mut all = Vec::new();
        all.extend(self.curiosity_goals(knowledge_gaps));
        all.extend(self.creative_goals(seed_concepts));
        all.extend(self.philosophical_goals());
        all.extend(self.frontier_goals());

        info!(
            "OpenEndedGoalGenerator produced {} goal(s) this cycle (total: {})",
            all.len(),
            self.total_generated
        );
        all
    }

    pub fn stats(&self) -> OpenEndedStats {
        OpenEndedStats {
            total_generated: self.total_generated,
            exploration_history_len: self.curiosity_engine.exploration_history.len(),
            concept_library_size: self.creativity_engine.concept_library.len(),
            philosophical_questions: self.philosophical_reasoner.active_questions.len(),
            long_horizon_goals: self.existential_planner.long_horizon_goals.len(),
        }
    }
}

impl Default for OpenEndedGoalGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenEndedStats {
    pub total_generated: u64,
    pub exploration_history_len: usize,
    pub concept_library_size: usize,
    pub philosophical_questions: usize,
    pub long_horizon_goals: usize,
}

//! Hypothesis Generator — Generate and test hypotheses about the world.
//!
//! Implements a hypothesis generation and testing cycle: form candidate
//! hypotheses from knowledge gaps and observations, design tests, run them,
//! and update beliefs based on results.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

// ── Hypothesis ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HypothesisStatus {
    Proposed,
    Testing,
    Supported,
    Refuted,
    Inconclusive,
    Superseded { by: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub id: String,
    pub statement: String,
    pub domain: String,
    pub generated_from: Vec<String>,  // observation / knowledge gap IDs
    pub prior_probability: f64,
    pub posterior_probability: f64,
    pub status: HypothesisStatus,
    pub supporting_evidence: Vec<String>,
    pub contradicting_evidence: Vec<String>,
    pub tests_run: u32,
    pub tests_passed: u32,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub tags: Vec<String>,
}

impl Hypothesis {
    pub fn new(statement: &str, domain: &str, prior: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            statement: statement.to_string(),
            domain: domain.to_string(),
            generated_from: Vec::new(),
            prior_probability: prior.clamp(0.0, 1.0),
            posterior_probability: prior.clamp(0.0, 1.0),
            status: HypothesisStatus::Proposed,
            supporting_evidence: Vec::new(),
            contradicting_evidence: Vec::new(),
            tests_run: 0,
            tests_passed: 0,
            created_at: Utc::now(),
            last_updated: Utc::now(),
            tags: Vec::new(),
        }
    }

    /// Update posterior via Bayesian update: P(H|E) ∝ P(E|H) × P(H)
    pub fn bayesian_update(&mut self, likelihood_if_true: f64, likelihood_if_false: f64) {
        let p_h = self.posterior_probability;
        let p_e = likelihood_if_true * p_h + likelihood_if_false * (1.0 - p_h);
        if p_e > 0.0 {
            self.posterior_probability = (likelihood_if_true * p_h / p_e).clamp(0.0, 1.0);
        }
        self.last_updated = Utc::now();
    }

    pub fn accept_evidence(&mut self, evidence: &str, supports: bool) {
        if supports {
            self.supporting_evidence.push(evidence.to_string());
            self.bayesian_update(0.9, 0.3);
        } else {
            self.contradicting_evidence.push(evidence.to_string());
            self.bayesian_update(0.1, 0.7);
        }
        self.tests_run += 1;
        if supports {
            self.tests_passed += 1;
        }
        self.update_status();
    }

    fn update_status(&mut self) {
        self.status = if self.posterior_probability >= 0.85 {
            HypothesisStatus::Supported
        } else if self.posterior_probability <= 0.10 {
            HypothesisStatus::Refuted
        } else if self.tests_run >= 3 {
            HypothesisStatus::Inconclusive
        } else {
            HypothesisStatus::Testing
        };
    }

    pub fn test_success_rate(&self) -> f64 {
        if self.tests_run == 0 {
            0.0
        } else {
            self.tests_passed as f64 / self.tests_run as f64
        }
    }
}

// ── Hypothesis Test ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestMethod {
    RunCode { snippet: String },
    SearchKnowledgeGraph { query: String },
    ObserveOutcome { experiment: String },
    ConsultExpert { query: String },
    LogicalDeduction { premises: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypothesisTest {
    pub id: String,
    pub hypothesis_id: String,
    pub method: TestMethod,
    pub expected_outcome: String,
    pub actual_outcome: Option<String>,
    pub supports_hypothesis: Option<bool>,
    pub confidence: f64,
    pub run_at: Option<DateTime<Utc>>,
    pub description: String,
}

impl HypothesisTest {
    pub fn new(hypothesis_id: &str, method: TestMethod, expected: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            hypothesis_id: hypothesis_id.to_string(),
            method,
            expected_outcome: expected.to_string(),
            actual_outcome: None,
            supports_hypothesis: None,
            confidence: 0.7,
            run_at: None,
            description: String::new(),
        }
    }

    pub fn record_result(&mut self, actual: &str, supports: bool) {
        self.actual_outcome = Some(actual.to_string());
        self.supports_hypothesis = Some(supports);
        self.run_at = Some(Utc::now());
    }
}

// ── Observation ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub id: String,
    pub description: String,
    pub domain: String,
    pub timestamp: DateTime<Utc>,
    pub confidence: f64,
    pub source: String,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
}

impl Observation {
    pub fn new(description: &str, domain: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            description: description.to_string(),
            domain: domain.to_string(),
            timestamp: Utc::now(),
            confidence: 0.8,
            source: String::new(),
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

// ── Hypothesis Generator ──────────────────────────────────────────────────────

pub struct HypothesisGenerator {
    pub hypotheses: Vec<Hypothesis>,
    pub pending_tests: Vec<HypothesisTest>,
    pub observations: Vec<Observation>,
    pub min_prior: f64,
    pub max_hypotheses_per_domain: usize,
}

impl HypothesisGenerator {
    pub fn new() -> Self {
        Self {
            hypotheses: Vec::new(),
            pending_tests: Vec::new(),
            observations: Vec::new(),
            min_prior: 0.20,
            max_hypotheses_per_domain: 50,
        }
    }

    pub fn add_observation(&mut self, obs: Observation) {
        self.observations.push(obs);
    }

    /// Generate hypotheses from a set of observations in the same domain.
    pub fn generate_from_observations(&mut self, domain: &str) -> Vec<Hypothesis> {
        let domain_obs: Vec<&Observation> = self
            .observations
            .iter()
            .filter(|o| o.domain == domain)
            .collect();

        if domain_obs.is_empty() {
            return Vec::new();
        }

        let domain_count = self
            .hypotheses
            .iter()
            .filter(|h| h.domain == domain)
            .count();

        if domain_count >= self.max_hypotheses_per_domain {
            warn!(
                "Max hypotheses ({}) reached for domain '{}'",
                self.max_hypotheses_per_domain, domain
            );
            return Vec::new();
        }

        let mut generated = Vec::new();

        // Heuristic: if multiple observations share keywords, form a
        // generalisation hypothesis
        if domain_obs.len() >= 2 {
            let combined: String = domain_obs
                .iter()
                .map(|o| o.description.as_str())
                .collect::<Vec<_>>()
                .join(" ");

            let statement = format!(
                "In domain '{}': the observed pattern suggests a systematic relationship \
                 among {} observations (combined signal: {}...)",
                domain,
                domain_obs.len(),
                &combined[..combined.len().min(80)]
            );

            let prior = 0.4 + 0.05 * (domain_obs.len() as f64).min(4.0);
            let mut h = Hypothesis::new(&statement, domain, prior);
            h.generated_from = domain_obs.iter().map(|o| o.id.clone()).collect();

            // Generate a test for this hypothesis
            let test = HypothesisTest::new(
                &h.id,
                TestMethod::SearchKnowledgeGraph {
                    query: format!("systematic relationship {}", domain),
                },
                "supporting evidence found in knowledge graph",
            );
            self.pending_tests.push(test);

            self.hypotheses.push(h.clone());
            generated.push(h);
        }

        // Also generate a null hypothesis
        let null_statement = format!(
            "In domain '{}': the observations are independent random events (null hypothesis)",
            domain
        );
        let mut null_h = Hypothesis::new(&null_statement, domain, 0.40);
        null_h.tags.push("null_hypothesis".to_string());
        self.hypotheses.push(null_h.clone());
        generated.push(null_h);

        info!(
            "Generated {} hypotheses for domain '{}' from {} observations",
            generated.len(),
            domain,
            domain_obs.len()
        );
        generated
    }

    /// Generate a hypothesis from a knowledge gap.
    pub fn generate_from_gap(&mut self, topic: &str, gap_description: &str) -> Hypothesis {
        let statement = format!(
            "There exists a principle or mechanism that explains '{}' in the topic '{}'. \
             Current knowledge is insufficient; further research is required.",
            gap_description, topic
        );
        let mut h = Hypothesis::new(&statement, topic, 0.50);
        h.tags.push("knowledge_gap".to_string());
        h.status = HypothesisStatus::Proposed;

        let test = HypothesisTest::new(
            &h.id,
            TestMethod::ConsultExpert {
                query: format!("What explains {}?", gap_description),
            },
            "expert source confirms the mechanism",
        );
        self.pending_tests.push(test);

        self.hypotheses.push(h.clone());
        h
    }

    /// Apply an evidence observation to the relevant hypotheses.
    pub fn apply_evidence(&mut self, hypothesis_id: &str, evidence: &str, supports: bool) {
        if let Some(h) = self.hypotheses.iter_mut().find(|h| h.id == hypothesis_id) {
            h.accept_evidence(evidence, supports);
            info!(
                "Hypothesis '{}' updated: posterior={:.3} status={:?}",
                h.id, h.posterior_probability, h.status
            );
        }
    }

    /// Return the top-N hypotheses by posterior probability.
    pub fn top_hypotheses(&self, n: usize) -> Vec<&Hypothesis> {
        let mut hs: Vec<&Hypothesis> = self.hypotheses.iter().collect();
        hs.sort_by(|a, b| {
            b.posterior_probability
                .partial_cmp(&a.posterior_probability)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hs.truncate(n);
        hs
    }

    pub fn supported_hypotheses(&self) -> Vec<&Hypothesis> {
        self.hypotheses
            .iter()
            .filter(|h| h.status == HypothesisStatus::Supported)
            .collect()
    }

    pub fn next_test(&mut self) -> Option<HypothesisTest> {
        self.pending_tests.pop()
    }

    pub fn stats(&self) -> HypothesisStats {
        let total = self.hypotheses.len();
        HypothesisStats {
            total_hypotheses: total,
            proposed: self.hypotheses.iter().filter(|h| h.status == HypothesisStatus::Proposed).count(),
            testing: self.hypotheses.iter().filter(|h| h.status == HypothesisStatus::Testing).count(),
            supported: self.hypotheses.iter().filter(|h| h.status == HypothesisStatus::Supported).count(),
            refuted: self.hypotheses.iter().filter(|h| h.status == HypothesisStatus::Refuted).count(),
            inconclusive: self.hypotheses.iter().filter(|h| h.status == HypothesisStatus::Inconclusive).count(),
            pending_tests: self.pending_tests.len(),
            observations: self.observations.len(),
        }
    }
}

impl Default for HypothesisGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypothesisStats {
    pub total_hypotheses: usize,
    pub proposed: usize,
    pub testing: usize,
    pub supported: usize,
    pub refuted: usize,
    pub inconclusive: usize,
    pub pending_tests: usize,
    pub observations: usize,
}

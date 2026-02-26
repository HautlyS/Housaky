//! Abstraction — Extract general principles from specific examples.
//!
//! Implements concept abstraction: given a set of concrete examples or episodes,
//! extract higher-level patterns and generalisations that apply across contexts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

// ── Concrete Example ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcreteExample {
    pub id: String,
    pub domain: String,
    pub description: String,
    pub features: HashMap<String, String>,
    pub outcome: String,
    pub success: bool,
    pub source: String,
    pub timestamp: DateTime<Utc>,
}

impl ConcreteExample {
    pub fn new(domain: &str, description: &str, outcome: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            domain: domain.to_string(),
            description: description.to_string(),
            features: HashMap::new(),
            outcome: outcome.to_string(),
            success: true,
            source: String::new(),
            timestamp: Utc::now(),
        }
    }
}

// ── Abstract Principle ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbstractPrinciple {
    pub id: String,
    pub name: String,
    pub statement: String,
    pub domain_of_origin: String,
    pub applicable_domains: Vec<String>,
    pub supporting_examples: Vec<String>,  // example IDs
    pub confidence: f64,
    pub generality: f64,                   // 0.0 (narrow) – 1.0 (universal)
    pub extracted_at: DateTime<Utc>,
    pub uses: u64,                         // how many times applied
}

impl AbstractPrinciple {
    pub fn new(name: &str, statement: &str, domain: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            statement: statement.to_string(),
            domain_of_origin: domain.to_string(),
            applicable_domains: vec![domain.to_string()],
            supporting_examples: Vec::new(),
            confidence: 0.5,
            generality: 0.3,
            extracted_at: Utc::now(),
            uses: 0,
        }
    }

    pub fn record_use(&mut self) {
        self.uses += 1;
    }

    pub fn reinforce(&mut self, example_id: &str) {
        if !self.supporting_examples.contains(&example_id.to_string()) {
            self.supporting_examples.push(example_id.to_string());
        }
        let n = self.supporting_examples.len() as f64;
        // Confidence increases logarithmically with supporting evidence
        self.confidence = (1.0 - (-n * 0.3).exp()).min(0.99);
    }
}

// ── Abstraction Config ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbstractionConfig {
    pub min_examples_for_principle: usize,
    pub min_confidence_threshold: f64,
    pub max_principles_per_domain: usize,
    pub cross_domain_threshold: f64,  // similarity score to apply a principle cross-domain
}

impl Default for AbstractionConfig {
    fn default() -> Self {
        Self {
            min_examples_for_principle: 3,
            min_confidence_threshold: 0.60,
            max_principles_per_domain: 50,
            cross_domain_threshold: 0.70,
        }
    }
}

// ── Abstraction Engine ────────────────────────────────────────────────────────

pub struct AbstractionEngine {
    pub config: AbstractionConfig,
    pub examples: Vec<ConcreteExample>,
    pub principles: Vec<AbstractPrinciple>,
    pub domain_index: HashMap<String, Vec<String>>,  // domain → principle IDs
}

impl AbstractionEngine {
    pub fn new() -> Self {
        Self {
            config: AbstractionConfig::default(),
            examples: Vec::new(),
            principles: Vec::new(),
            domain_index: HashMap::new(),
        }
    }

    pub fn with_config(config: AbstractionConfig) -> Self {
        Self {
            config,
            examples: Vec::new(),
            principles: Vec::new(),
            domain_index: HashMap::new(),
        }
    }

    pub fn add_example(&mut self, example: ConcreteExample) {
        self.examples.push(example);
    }

    /// Attempt to extract a new principle from a cluster of examples.
    ///
    /// Groups examples by domain, looks for repeated patterns in features/outcomes,
    /// and proposes a principle if enough evidence exists.
    pub fn extract_principles(&mut self) -> Vec<AbstractPrinciple> {
        let mut new_principles = Vec::new();

        // Group examples by domain
        let mut by_domain: HashMap<String, Vec<&ConcreteExample>> = HashMap::new();
        for ex in &self.examples {
            by_domain.entry(ex.domain.clone()).or_default().push(ex);
        }

        for (domain, examples) in &by_domain {
            if examples.len() < self.config.min_examples_for_principle {
                continue;
            }

            // Find common features across successful examples in this domain
            let successful: Vec<&&ConcreteExample> =
                examples.iter().filter(|e| e.success).collect();
            if successful.len() < self.config.min_examples_for_principle {
                continue;
            }

            let common_features = self.find_common_features(&successful);
            for (feature, value) in &common_features {
                let principle_name = format!("{}_{}_{}", domain, feature, value)
                    .replace(' ', "_")
                    .to_lowercase();

                // Skip if already extracted
                if self.principles.iter().any(|p| p.name == principle_name) {
                    continue;
                }

                // Check max principles per domain
                let domain_count = self
                    .domain_index
                    .get(domain.as_str())
                    .map(|v| v.len())
                    .unwrap_or(0);
                if domain_count >= self.config.max_principles_per_domain {
                    continue;
                }

                let statement = format!(
                    "In domain '{}': when '{}' = '{}', the outcome tends to be successful",
                    domain, feature, value
                );

                let mut principle = AbstractPrinciple::new(&principle_name, &statement, domain);
                for ex in &successful {
                    principle.reinforce(&ex.id);
                }
                principle.generality = 0.3;

                if principle.confidence >= self.config.min_confidence_threshold {
                    self.domain_index
                        .entry(domain.clone())
                        .or_default()
                        .push(principle.id.clone());
                    new_principles.push(principle.clone());
                    self.principles.push(principle);
                }
            }
        }

        if !new_principles.is_empty() {
            info!(
                "Abstraction engine: extracted {} new principles",
                new_principles.len()
            );
        }

        new_principles
    }

    /// Promote principles to cross-domain generalisations when they appear in
    /// multiple domains with consistent outcomes.
    pub fn generalise_across_domains(&mut self) -> Vec<AbstractPrinciple> {
        let mut generalisations = Vec::new();

        // For each pair of principles with similar statements in different domains,
        // attempt to abstract a more general statement.
        let principles_snapshot = self.principles.clone();
        for (i, p1) in principles_snapshot.iter().enumerate() {
            for p2 in principles_snapshot[i + 1..].iter() {
                if p1.domain_of_origin == p2.domain_of_origin {
                    continue;
                }
                let similarity = self.statement_similarity(&p1.statement, &p2.statement);
                if similarity < self.config.cross_domain_threshold {
                    continue;
                }

                let general_name =
                    format!("cross_domain_{}_{}", i, self.principles.len());
                let general_statement = format!(
                    "Cross-domain generalisation (from '{}' and '{}'): {}",
                    p1.domain_of_origin,
                    p2.domain_of_origin,
                    p1.statement
                );

                // Check not already present
                if self.principles.iter().any(|p| p.name == general_name) {
                    continue;
                }

                let mut gen = AbstractPrinciple::new(
                    &general_name,
                    &general_statement,
                    "cross_domain",
                );
                gen.applicable_domains =
                    vec![p1.domain_of_origin.clone(), p2.domain_of_origin.clone()];
                gen.generality = 0.70 + similarity * 0.20;
                gen.confidence = (p1.confidence + p2.confidence) / 2.0;
                gen.supporting_examples
                    .extend(p1.supporting_examples.clone());
                gen.supporting_examples
                    .extend(p2.supporting_examples.clone());

                generalisations.push(gen.clone());
                self.principles.push(gen);
            }
        }

        generalisations
    }

    /// Look up principles applicable to a target domain.
    pub fn applicable_principles(&self, domain: &str) -> Vec<&AbstractPrinciple> {
        self.principles
            .iter()
            .filter(|p| {
                p.domain_of_origin == domain
                    || p.applicable_domains.iter().any(|d| d == domain)
            })
            .filter(|p| p.confidence >= self.config.min_confidence_threshold)
            .collect()
    }

    /// Apply a principle (increment its use counter).
    pub fn apply_principle(&mut self, principle_id: &str) {
        if let Some(p) = self.principles.iter_mut().find(|p| p.id == principle_id) {
            p.record_use();
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn find_common_features(
        &self,
        examples: &[&&ConcreteExample],
    ) -> HashMap<String, String> {
        if examples.is_empty() {
            return HashMap::new();
        }

        let first = &examples[0].features;
        let mut common: HashMap<String, String> = first.clone();

        for ex in examples[1..].iter() {
            common.retain(|k, v| ex.features.get(k) == Some(v));
        }

        common
    }

    fn statement_similarity(&self, a: &str, b: &str) -> f64 {
        // Simple Jaccard similarity on whitespace tokens
        let tokens_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
        let tokens_b: std::collections::HashSet<&str> = b.split_whitespace().collect();
        let intersection = tokens_a.intersection(&tokens_b).count();
        let union = tokens_a.union(&tokens_b).count();
        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    pub fn stats(&self) -> AbstractionStats {
        AbstractionStats {
            total_examples: self.examples.len(),
            total_principles: self.principles.len(),
            high_confidence_principles: self
                .principles
                .iter()
                .filter(|p| p.confidence >= 0.80)
                .count(),
            cross_domain_principles: self
                .principles
                .iter()
                .filter(|p| p.domain_of_origin == "cross_domain")
                .count(),
            average_confidence: if self.principles.is_empty() {
                0.0
            } else {
                self.principles.iter().map(|p| p.confidence).sum::<f64>()
                    / self.principles.len() as f64
            },
        }
    }
}

impl Default for AbstractionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbstractionStats {
    pub total_examples: usize,
    pub total_principles: usize,
    pub high_confidence_principles: usize,
    pub cross_domain_principles: usize,
    pub average_confidence: f64,
}

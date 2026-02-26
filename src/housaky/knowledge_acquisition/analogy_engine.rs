//! Analogy Engine — Cross-domain analogy: "X in domain A is like Y in domain B".
//!
//! Implements structure-mapping theory for analogical reasoning. Finds structural
//! correspondences between domains, evaluates analogy quality, and generates novel
//! predictions in a target domain by transferring knowledge from a source domain.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

// ── Domain Concept ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainConcept {
    pub id: String,
    pub domain: String,
    pub name: String,
    pub description: String,
    pub relations: HashMap<String, String>,  // relation_type → target_concept_id
    pub attributes: HashMap<String, String>,
}

impl DomainConcept {
    pub fn new(domain: &str, name: &str, description: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            domain: domain.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            relations: HashMap::new(),
            attributes: HashMap::new(),
        }
    }
}

// ── Analogy ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analogy {
    pub id: String,
    pub source_domain: String,
    pub target_domain: String,
    /// concept name in source → concept name in target
    pub mapping: HashMap<String, String>,
    /// Relations that are structurally preserved by the mapping
    pub preserved_relations: Vec<String>,
    pub structural_similarity: f64,
    /// How well source-domain predictions transfer to target domain
    pub predictive_power: f64,
    /// Novel predictions about the target domain derived from the source
    pub novel_predictions: Vec<String>,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
    pub validated: bool,
}

impl Analogy {
    pub fn new(source: &str, target: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source_domain: source.to_string(),
            target_domain: target.to_string(),
            mapping: HashMap::new(),
            preserved_relations: Vec::new(),
            structural_similarity: 0.0,
            predictive_power: 0.0,
            novel_predictions: Vec::new(),
            confidence: 0.0,
            created_at: Utc::now(),
            validated: false,
        }
    }

    pub fn add_mapping(&mut self, source_concept: &str, target_concept: &str) {
        self.mapping
            .insert(source_concept.to_string(), target_concept.to_string());
    }

    pub fn add_prediction(&mut self, prediction: &str) {
        self.novel_predictions.push(prediction.to_string());
    }

    /// Overall quality score combining structural similarity and predictive power.
    pub fn quality_score(&self) -> f64 {
        0.6 * self.structural_similarity + 0.4 * self.predictive_power
    }

    /// Number of mapped concepts.
    pub fn mapping_size(&self) -> usize {
        self.mapping.len()
    }
}

// ── Analogy Engine ────────────────────────────────────────────────────────────

pub struct AnalogyEngine {
    pub analogies: Vec<Analogy>,
    pub concept_registry: HashMap<String, Vec<DomainConcept>>,  // domain → concepts
    pub min_mapping_size: usize,
    pub min_structural_similarity: f64,
}

impl AnalogyEngine {
    pub fn new() -> Self {
        Self {
            analogies: Vec::new(),
            concept_registry: HashMap::new(),
            min_mapping_size: 2,
            min_structural_similarity: 0.40,
        }
    }

    /// Register concepts for a domain.
    pub fn register_domain(&mut self, domain: &str, concepts: Vec<DomainConcept>) {
        self.concept_registry.insert(domain.to_string(), concepts);
    }

    /// Add a concept to an existing domain.
    pub fn add_concept(&mut self, concept: DomainConcept) {
        self.concept_registry
            .entry(concept.domain.clone())
            .or_default()
            .push(concept);
    }

    /// Attempt to find analogies between `source_domain` and `target_domain`
    /// using structure-mapping heuristics.
    pub fn find_analogies(
        &mut self,
        source_domain: &str,
        target_domain: &str,
    ) -> Vec<Analogy> {
        let source_concepts = match self.concept_registry.get(source_domain) {
            Some(c) => c.clone(),
            None => return Vec::new(),
        };
        let target_concepts = match self.concept_registry.get(target_domain) {
            Some(c) => c.clone(),
            None => return Vec::new(),
        };

        let mut analogy = Analogy::new(source_domain, target_domain);

        // Build concept-name index for target
        let target_index: HashMap<&str, &DomainConcept> = target_concepts
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();

        // Structure mapping: match source concepts to target concepts with
        // similar attribute profiles (shared attribute keys → higher score)
        for src in &source_concepts {
            let mut best_target: Option<&str> = None;
            let mut best_score = 0.0f64;

            for tgt in &target_concepts {
                // Already mapped
                if analogy.mapping.values().any(|v| v == &tgt.name) {
                    continue;
                }

                let score = Self::concept_similarity(src, tgt);
                if score > best_score {
                    best_score = score;
                    best_target = Some(tgt.name.as_str());
                }
            }

            if best_score >= 0.35 {
                if let Some(target_name) = best_target {
                    analogy.add_mapping(&src.name, target_name);

                    // Check which relations are preserved
                    for (rel, rel_target) in &src.relations {
                        if let Some(mapped_target) = analogy.mapping.get(rel_target.as_str()) {
                            if let Some(tgt_concept) = target_index.get(analogy.mapping.get(&src.name).unwrap().as_str()) {
                                if tgt_concept.relations.get(rel) == Some(mapped_target) {
                                    analogy.preserved_relations.push(rel.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        if analogy.mapping.len() < self.min_mapping_size {
            return Vec::new();
        }

        // Compute structural similarity
        let total_relations: usize = source_concepts.iter().map(|c| c.relations.len()).sum();
        analogy.structural_similarity = if total_relations == 0 {
            analogy.mapping.len() as f64 / source_concepts.len().max(1) as f64
        } else {
            analogy.preserved_relations.len() as f64 / total_relations as f64
        };

        if analogy.structural_similarity < self.min_structural_similarity {
            return Vec::new();
        }

        // Generate novel predictions: for unmapped source relations, hypothesise
        // that the mapped target concept has a similar relation.
        // Collect all predictions first to avoid simultaneous immutable + mutable borrow.
        let new_predictions: Vec<String> = source_concepts
            .iter()
            .filter_map(|src| analogy.mapping.get(&src.name).map(|tn| (src, tn.clone())))
            .flat_map(|(src, target_name)| {
                src.relations
                    .iter()
                    .filter_map(|(rel, rel_target)| {
                        analogy.mapping.get(rel_target.as_str()).map(|mapped| {
                            format!(
                                "In domain '{}': '{}' has relation '{}' to '{}'",
                                target_domain, target_name, rel, mapped
                            )
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        for p in new_predictions {
            analogy.add_prediction(&p);
        }

        analogy.predictive_power = (analogy.novel_predictions.len() as f64 * 0.1)
            .min(1.0)
            * analogy.structural_similarity;
        analogy.confidence = analogy.quality_score();

        info!(
            "Analogy found: '{}' → '{}' (structural={:.2}, {} predictions)",
            source_domain,
            target_domain,
            analogy.structural_similarity,
            analogy.novel_predictions.len()
        );

        let result = vec![analogy.clone()];
        self.analogies.push(analogy);
        result
    }

    /// Retrieve analogies applicable to a target domain.
    pub fn analogies_for_domain(&self, domain: &str) -> Vec<&Analogy> {
        self.analogies
            .iter()
            .filter(|a| a.target_domain == domain || a.source_domain == domain)
            .collect()
    }

    /// Get the best analogy for a pair of domains.
    pub fn best_analogy(&self, source: &str, target: &str) -> Option<&Analogy> {
        self.analogies
            .iter()
            .filter(|a| a.source_domain == source && a.target_domain == target)
            .max_by(|a, b| {
                a.quality_score()
                    .partial_cmp(&b.quality_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Validate an analogy by confirming one of its predictions.
    pub fn validate_prediction(&mut self, analogy_id: &str, prediction: &str) {
        if let Some(a) = self.analogies.iter_mut().find(|a| a.id == analogy_id) {
            a.validated = true;
            a.predictive_power = (a.predictive_power + 0.1).min(1.0);
            a.confidence = a.quality_score();
            info!("Analogy '{}' prediction validated: '{}'", analogy_id, prediction);
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn concept_similarity(a: &DomainConcept, b: &DomainConcept) -> f64 {
        // Jaccard on attribute keys
        let keys_a: std::collections::HashSet<&str> =
            a.attributes.keys().map(|k| k.as_str()).collect();
        let keys_b: std::collections::HashSet<&str> =
            b.attributes.keys().map(|k| k.as_str()).collect();
        let intersection = keys_a.intersection(&keys_b).count();
        let union = keys_a.union(&keys_b).count();
        let attr_sim = if union == 0 { 0.0 } else { intersection as f64 / union as f64 };

        // Jaccard on relation keys
        let rel_a: std::collections::HashSet<&str> =
            a.relations.keys().map(|k| k.as_str()).collect();
        let rel_b: std::collections::HashSet<&str> =
            b.relations.keys().map(|k| k.as_str()).collect();
        let rel_inter = rel_a.intersection(&rel_b).count();
        let rel_union = rel_a.union(&rel_b).count();
        let rel_sim = if rel_union == 0 { 0.0 } else { rel_inter as f64 / rel_union as f64 };

        0.5 * attr_sim + 0.5 * rel_sim
    }

    pub fn stats(&self) -> AnalogyStats {
        AnalogyStats {
            total_analogies: self.analogies.len(),
            validated: self.analogies.iter().filter(|a| a.validated).count(),
            total_predictions: self.analogies.iter().map(|a| a.novel_predictions.len()).sum(),
            average_quality: if self.analogies.is_empty() {
                0.0
            } else {
                self.analogies.iter().map(|a| a.quality_score()).sum::<f64>()
                    / self.analogies.len() as f64
            },
            domains_covered: self
                .concept_registry
                .keys()
                .cloned()
                .collect(),
        }
    }
}

impl Default for AnalogyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalogyStats {
    pub total_analogies: usize,
    pub validated: usize,
    pub total_predictions: usize,
    pub average_quality: f64,
    pub domains_covered: Vec<String>,
}

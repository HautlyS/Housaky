//! Knowledge Compiler — Compress verbose knowledge into compact executable representations.
//!
//! Takes raw knowledge (papers, syntheses, principles, hypotheses) and compresses it
//! into compact, executable forms: decision rules, lookup tables, parametric models,
//! and code snippets — enabling fast retrieval without re-reading source material.

use crate::housaky::knowledge_acquisition::abstraction::AbstractPrinciple;
use crate::housaky::knowledge_acquisition::analogy_engine::Analogy;
use crate::housaky::knowledge_acquisition::hypothesis_gen::Hypothesis;
use crate::housaky::knowledge_acquisition::research_agent::KnowledgeSynthesis;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

// ── Compiled Knowledge Unit ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompiledForm {
    /// A simple if-then decision rule.
    DecisionRule {
        condition: String,
        action: String,
        confidence: f64,
    },
    /// A lookup table: input pattern → output.
    LookupTable { entries: HashMap<String, String> },
    /// A parametric model expressed as a formula.
    ParametricModel {
        formula: String,
        parameters: HashMap<String, f64>,
        domain: String,
    },
    /// An executable code snippet.
    CodeSnippet {
        language: String,
        code: String,
        description: String,
    },
    /// A compact heuristic.
    Heuristic {
        name: String,
        description: String,
        applicability: String,
    },
    /// A summarised principle for fast retrieval.
    CompactPrinciple { principle: String, domain: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledKnowledgeUnit {
    pub id: String,
    pub topic: String,
    pub domain: String,
    pub form: CompiledForm,
    /// Source knowledge IDs (synthesis, principle, hypothesis)
    pub source_ids: Vec<String>,
    pub compression_ratio: f64,
    pub retrieval_count: u64,
    pub confidence: f64,
    pub compiled_at: DateTime<Utc>,
    pub last_retrieved: Option<DateTime<Utc>>,
}

impl CompiledKnowledgeUnit {
    pub fn new(topic: &str, domain: &str, form: CompiledForm, source_ids: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            topic: topic.to_string(),
            domain: domain.to_string(),
            form,
            source_ids,
            compression_ratio: 1.0,
            retrieval_count: 0,
            confidence: 0.7,
            compiled_at: Utc::now(),
            last_retrieved: None,
        }
    }

    pub fn retrieve(&mut self) -> &CompiledForm {
        self.retrieval_count += 1;
        self.last_retrieved = Some(Utc::now());
        &self.form
    }
}

// ── Knowledge Index ───────────────────────────────────────────────────────────

/// Fast-access index over compiled knowledge units.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct KnowledgeIndex {
    /// topic → list of unit IDs
    by_topic: HashMap<String, Vec<String>>,
    /// domain → list of unit IDs
    by_domain: HashMap<String, Vec<String>>,
    /// total units
    pub total: usize,
}

impl KnowledgeIndex {
    pub fn insert(&mut self, unit: &CompiledKnowledgeUnit) {
        self.by_topic
            .entry(unit.topic.clone())
            .or_default()
            .push(unit.id.clone());
        self.by_domain
            .entry(unit.domain.clone())
            .or_default()
            .push(unit.id.clone());
        self.total += 1;
    }

    pub fn ids_for_topic(&self, topic: &str) -> &[String] {
        self.by_topic
            .get(topic)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn ids_for_domain(&self, domain: &str) -> &[String] {
        self.by_domain
            .get(domain)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

// ── Knowledge Compiler ────────────────────────────────────────────────────────

pub struct KnowledgeCompiler {
    pub units: HashMap<String, CompiledKnowledgeUnit>,
    pub index: KnowledgeIndex,
    pub min_confidence: f64,
}

impl KnowledgeCompiler {
    pub fn new() -> Self {
        Self {
            units: HashMap::new(),
            index: KnowledgeIndex::default(),
            min_confidence: 0.55,
        }
    }

    // ── Compilation entry points ──────────────────────────────────────────────

    /// Compile a `KnowledgeSynthesis` into compact lookup entries.
    pub fn compile_synthesis(&mut self, synthesis: &KnowledgeSynthesis) -> Vec<String> {
        let mut unit_ids = Vec::new();

        // Compile each finding as a lookup table entry
        if !synthesis.key_findings.is_empty() {
            let mut table: HashMap<String, String> = HashMap::new();
            for (i, finding) in synthesis.key_findings.iter().enumerate() {
                table.insert(format!("finding_{}", i), finding.clone());
            }
            let unit = CompiledKnowledgeUnit::new(
                &synthesis.topic,
                &synthesis.topic,
                CompiledForm::LookupTable { entries: table },
                synthesis.source_paper_ids.clone(),
            );
            let id = unit.id.clone();
            self.add_unit(unit);
            unit_ids.push(id);
        }

        // Compile actionable insights as decision rules
        for insight in &synthesis.actionable_insights {
            let parts: Vec<&str> = insight.splitn(2, ':').collect();
            let (condition, action) = if parts.len() == 2 {
                (parts[0].trim().to_string(), parts[1].trim().to_string())
            } else {
                ("applies".to_string(), insight.clone())
            };

            if synthesis.confidence >= self.min_confidence {
                let unit = CompiledKnowledgeUnit::new(
                    &synthesis.topic,
                    &synthesis.topic,
                    CompiledForm::DecisionRule {
                        condition,
                        action,
                        confidence: synthesis.confidence,
                    },
                    synthesis.source_paper_ids.clone(),
                );
                let id = unit.id.clone();
                self.add_unit(unit);
                unit_ids.push(id);
            }
        }

        info!(
            "Compiled synthesis '{}' → {} units",
            synthesis.topic,
            unit_ids.len()
        );
        unit_ids
    }

    /// Compile an `AbstractPrinciple` into a compact principle entry.
    pub fn compile_principle(&mut self, principle: &AbstractPrinciple) -> String {
        let unit = CompiledKnowledgeUnit {
            id: Uuid::new_v4().to_string(),
            topic: principle.name.clone(),
            domain: principle.domain_of_origin.clone(),
            form: CompiledForm::CompactPrinciple {
                principle: principle.statement.clone(),
                domain: principle.domain_of_origin.clone(),
            },
            source_ids: principle.supporting_examples.clone(),
            compression_ratio: Self::estimate_compression(&principle.statement, 20),
            retrieval_count: 0,
            confidence: principle.confidence,
            compiled_at: Utc::now(),
            last_retrieved: None,
        };
        let id = unit.id.clone();
        self.add_unit(unit);
        id
    }

    /// Compile a supported `Hypothesis` into a parametric model.
    pub fn compile_hypothesis(&mut self, hypothesis: &Hypothesis) -> Option<String> {
        if hypothesis.posterior_probability < self.min_confidence {
            return None;
        }

        let unit = CompiledKnowledgeUnit::new(
            &hypothesis.domain,
            &hypothesis.domain,
            CompiledForm::ParametricModel {
                formula: hypothesis.statement.clone(),
                parameters: HashMap::from([
                    ("posterior".to_string(), hypothesis.posterior_probability),
                    ("prior".to_string(), hypothesis.prior_probability),
                    ("test_success_rate".to_string(), hypothesis.test_success_rate()),
                ]),
                domain: hypothesis.domain.clone(),
            },
            hypothesis.supporting_evidence.clone(),
        );
        let id = unit.id.clone();
        self.add_unit(unit);
        Some(id)
    }

    /// Compile an `Analogy` into a heuristic.
    pub fn compile_analogy(&mut self, analogy: &Analogy) -> Option<String> {
        if analogy.quality_score() < self.min_confidence {
            return None;
        }

        let mapping_str: String = analogy
            .mapping
            .iter()
            .take(5)
            .map(|(k, v)| format!("{}→{}", k, v))
            .collect::<Vec<_>>()
            .join(", ");

        let unit = CompiledKnowledgeUnit::new(
            &format!("{}_to_{}", analogy.source_domain, analogy.target_domain),
            &analogy.target_domain,
            CompiledForm::Heuristic {
                name: format!(
                    "{} ≅ {}",
                    analogy.source_domain, analogy.target_domain
                ),
                description: format!(
                    "Structural analogy (quality={:.2}): {}",
                    analogy.quality_score(),
                    mapping_str
                ),
                applicability: analogy.target_domain.clone(),
            },
            vec![analogy.id.clone()],
        );
        let id = unit.id.clone();
        self.add_unit(unit);
        Some(id)
    }

    // ── Retrieval ─────────────────────────────────────────────────────────────

    /// Retrieve all compiled units for a topic (sorted by confidence desc).
    pub fn retrieve_for_topic(&mut self, topic: &str) -> Vec<&CompiledForm> {
        let ids: Vec<String> = self.index.ids_for_topic(topic).to_vec();
        let mut results: Vec<(f64, &mut CompiledKnowledgeUnit)> = self
            .units
            .iter_mut()
            .filter(|(id, _)| ids.contains(id))
            .map(|(_, u)| (u.confidence, u))
            .collect();
        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        results
            .into_iter()
            .map(|(_, u)| u.retrieve())
            .collect()
    }

    /// Retrieve all compiled units for a domain.
    pub fn retrieve_for_domain(&mut self, domain: &str) -> Vec<String> {
        self.index.ids_for_domain(domain).to_vec()
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    fn add_unit(&mut self, unit: CompiledKnowledgeUnit) {
        self.index.insert(&unit);
        self.units.insert(unit.id.clone(), unit);
    }

    fn estimate_compression(source: &str, target_tokens: usize) -> f64 {
        let source_tokens = source.split_whitespace().count().max(1);
        source_tokens as f64 / target_tokens.max(1) as f64
    }

    pub fn stats(&self) -> CompilerStats {
        CompilerStats {
            total_units: self.units.len(),
            by_form_type: {
                let mut map: HashMap<String, usize> = HashMap::new();
                for u in self.units.values() {
                    let key = match &u.form {
                        CompiledForm::DecisionRule { .. } => "decision_rule",
                        CompiledForm::LookupTable { .. } => "lookup_table",
                        CompiledForm::ParametricModel { .. } => "parametric_model",
                        CompiledForm::CodeSnippet { .. } => "code_snippet",
                        CompiledForm::Heuristic { .. } => "heuristic",
                        CompiledForm::CompactPrinciple { .. } => "compact_principle",
                    };
                    *map.entry(key.to_string()).or_insert(0) += 1;
                }
                map
            },
            total_retrievals: self.units.values().map(|u| u.retrieval_count).sum(),
            average_confidence: if self.units.is_empty() {
                0.0
            } else {
                self.units.values().map(|u| u.confidence).sum::<f64>()
                    / self.units.len() as f64
            },
        }
    }
}

impl Default for KnowledgeCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerStats {
    pub total_units: usize,
    pub by_form_type: HashMap<String, usize>,
    pub total_retrievals: u64,
    pub average_confidence: f64,
}

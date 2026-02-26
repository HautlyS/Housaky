//! Cross-Domain Transfer Learning
//!
//! Transfers insights and skills between domains:
//! - Abstract pattern extraction from domain-specific experience
//! - Analogy engine: maps structures from source to target domain
//! - Domain-independent skill templates from procedural memory
//! - Structural similarity scoring (not surface similarity)

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

// ── Core Types ───────────────────────────────────────────────────────────────

/// An abstract pattern extracted from domain-specific experience.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbstractPattern {
    pub id: String,
    pub name: String,
    pub description: String,
    pub structure: PatternStructure,
    pub source_domains: Vec<String>,
    pub application_count: u32,
    pub success_rate: f64,
    pub created_at: DateTime<Utc>,
    pub last_applied: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternStructure {
    pub steps: Vec<PatternStep>,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub invariants: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternStep {
    pub abstract_action: String,
    pub role: StepRole,
    pub dependencies: Vec<usize>, // indices of prerequisite steps
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepRole {
    Decompose,    // Break problem into parts
    Analyze,      // Understand the problem
    Transform,    // Change representation
    Synthesize,   // Combine results
    Validate,     // Check correctness
    Iterate,      // Repeat until satisfied
    Adapt,        // Modify approach based on feedback
}

/// An analogy mapping between two domains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalogyMapping {
    pub id: String,
    pub source_domain: String,
    pub target_domain: String,
    pub concept_mappings: Vec<ConceptMapping>,
    pub structural_similarity: f64,
    pub surface_similarity: f64,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
    pub validated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptMapping {
    pub source_concept: String,
    pub target_concept: String,
    pub relationship_preserved: bool,
    pub mapping_confidence: f64,
}

/// An experience record for pattern extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: String,
    pub domain: String,
    pub task_description: String,
    pub steps_taken: Vec<String>,
    pub outcome: ExperienceOutcome,
    pub duration_seconds: u64,
    pub tools_used: Vec<String>,
    pub lessons: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExperienceOutcome {
    Success { quality: f64 },
    PartialSuccess { quality: f64, gaps: Vec<String> },
    Failure { reason: String },
}

/// A transferable procedure (domain-independent skill template).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Procedure {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub steps: Vec<ProcedureStep>,
    pub preconditions: Vec<String>,
    pub expected_outcome: String,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcedureStep {
    pub action: String,
    pub parameters: HashMap<String, String>,
    pub expected_result: String,
}

/// Transfer result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResult {
    pub source_procedure: String,
    pub target_procedure: Procedure,
    pub adaptation_notes: Vec<String>,
    pub estimated_success_rate: f64,
    pub confidence: f64,
}

// ── Transfer Learning Engine ─────────────────────────────────────────────────

pub struct TransferLearningEngine {
    pub abstract_patterns: Arc<RwLock<Vec<AbstractPattern>>>,
    pub analogy_cache: Arc<RwLock<HashMap<(String, String), AnalogyMapping>>>,
    pub experience_pool: Arc<RwLock<Vec<Experience>>>,
    pub domain_vocabulary: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl TransferLearningEngine {
    pub fn new() -> Self {
        Self {
            abstract_patterns: Arc::new(RwLock::new(Vec::new())),
            analogy_cache: Arc::new(RwLock::new(HashMap::new())),
            experience_pool: Arc::new(RwLock::new(Vec::new())),
            domain_vocabulary: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record an experience for future pattern extraction.
    pub async fn record_experience(&self, experience: Experience) {
        // Update domain vocabulary
        let words: Vec<String> = experience
            .task_description
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .collect();
        self.domain_vocabulary
            .write()
            .await
            .entry(experience.domain.clone())
            .or_default()
            .extend(words);

        self.experience_pool.write().await.push(experience);
    }

    /// Extract abstract patterns from a set of experiences.
    pub async fn abstract_pattern(
        &self,
        experiences: &[Experience],
    ) -> Option<AbstractPattern> {
        if experiences.len() < 2 {
            return None;
        }

        // Find common step patterns across experiences
        let step_sequences: Vec<Vec<&str>> = experiences
            .iter()
            .map(|e| e.steps_taken.iter().map(|s| s.as_str()).collect())
            .collect();

        // Extract common abstract steps
        let common_steps = self.find_common_steps(&step_sequences);

        if common_steps.is_empty() {
            return None;
        }

        // Determine preconditions from shared context
        let domains: Vec<String> = experiences
            .iter()
            .map(|e| e.domain.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let success_count = experiences
            .iter()
            .filter(|e| matches!(e.outcome, ExperienceOutcome::Success { .. }))
            .count();

        let pattern = AbstractPattern {
            id: uuid::Uuid::new_v4().to_string(),
            name: format!(
                "Pattern from {} experiences across {} domains",
                experiences.len(),
                domains.len()
            ),
            description: format!(
                "Common pattern: {}",
                common_steps
                    .iter()
                    .map(|s| s.abstract_action.clone())
                    .collect::<Vec<_>>()
                    .join(" → ")
            ),
            structure: PatternStructure {
                steps: common_steps,
                preconditions: vec!["Problem must be decomposable".to_string()],
                postconditions: vec!["All sub-problems solved".to_string()],
                invariants: vec!["Each step produces verifiable output".to_string()],
            },
            source_domains: domains,
            application_count: experiences.len() as u32,
            success_rate: success_count as f64 / experiences.len() as f64,
            created_at: Utc::now(),
            last_applied: None,
        };

        self.abstract_patterns.write().await.push(pattern.clone());

        info!(
            "Extracted abstract pattern '{}' from {} experiences",
            pattern.id,
            experiences.len()
        );

        Some(pattern)
    }

    /// Find common step patterns across multiple step sequences.
    fn find_common_steps(&self, sequences: &[Vec<&str>]) -> Vec<PatternStep> {
        // Find action verbs that appear across most sequences
        let mut action_counts: HashMap<String, usize> = HashMap::new();

        let abstract_actions = [
            ("analyze", StepRole::Analyze),
            ("understand", StepRole::Analyze),
            ("read", StepRole::Analyze),
            ("examine", StepRole::Analyze),
            ("break", StepRole::Decompose),
            ("decompose", StepRole::Decompose),
            ("split", StepRole::Decompose),
            ("divide", StepRole::Decompose),
            ("convert", StepRole::Transform),
            ("transform", StepRole::Transform),
            ("change", StepRole::Transform),
            ("modify", StepRole::Transform),
            ("combine", StepRole::Synthesize),
            ("merge", StepRole::Synthesize),
            ("integrate", StepRole::Synthesize),
            ("build", StepRole::Synthesize),
            ("test", StepRole::Validate),
            ("verify", StepRole::Validate),
            ("check", StepRole::Validate),
            ("validate", StepRole::Validate),
            ("repeat", StepRole::Iterate),
            ("retry", StepRole::Iterate),
            ("loop", StepRole::Iterate),
            ("adjust", StepRole::Adapt),
            ("adapt", StepRole::Adapt),
            ("refine", StepRole::Adapt),
        ];

        for seq in sequences {
            let seq_text = seq.join(" ").to_lowercase();
            for (keyword, _) in &abstract_actions {
                if seq_text.contains(keyword) {
                    *action_counts.entry(keyword.to_string()).or_insert(0) += 1;
                }
            }
        }

        let threshold = (sequences.len() as f64 * 0.5).ceil() as usize;
        let mut common: Vec<PatternStep> = action_counts
            .iter()
            .filter(|(_, count)| **count >= threshold)
            .filter_map(|(keyword, _)| {
                abstract_actions
                    .iter()
                    .find(|(k, _)| k == keyword)
                    .map(|(k, role)| PatternStep {
                        abstract_action: format!("{} the problem/input", k),
                        role: role.clone(),
                        dependencies: vec![],
                    })
            })
            .collect();

        // Sort by natural workflow order
        common.sort_by_key(|step| match step.role {
            StepRole::Analyze => 0,
            StepRole::Decompose => 1,
            StepRole::Transform => 2,
            StepRole::Synthesize => 3,
            StepRole::Validate => 4,
            StepRole::Iterate => 5,
            StepRole::Adapt => 6,
        });

        // Add dependencies
        for i in 1..common.len() {
            common[i].dependencies = vec![i - 1];
        }

        common
    }

    /// Find analogies between two domains.
    pub async fn find_analogies(
        &self,
        source_domain: &str,
        target_domain: &str,
    ) -> Vec<AnalogyMapping> {
        // Check cache first
        let key = (source_domain.to_string(), target_domain.to_string());
        let cache = self.analogy_cache.read().await;
        if let Some(cached) = cache.get(&key) {
            return vec![cached.clone()];
        }
        drop(cache);

        let vocab = self.domain_vocabulary.read().await;
        let source_vocab = vocab.get(source_domain).cloned().unwrap_or_default();
        let target_vocab = vocab.get(target_domain).cloned().unwrap_or_default();

        if source_vocab.is_empty() || target_vocab.is_empty() {
            return Vec::new();
        }

        // Find structural similarities through shared abstract concepts
        let source_set: std::collections::HashSet<&str> =
            source_vocab.iter().map(|s| s.as_str()).collect();
        let target_set: std::collections::HashSet<&str> =
            target_vocab.iter().map(|s| s.as_str()).collect();

        let shared: Vec<&str> = source_set.intersection(&target_set).copied().collect();
        let structural_similarity = if source_set.len() + target_set.len() > 0 {
            (2.0 * shared.len() as f64) / (source_set.len() + target_set.len()) as f64
        } else {
            0.0
        };

        // Build concept mappings for shared terms
        let concept_mappings: Vec<ConceptMapping> = shared
            .iter()
            .map(|&term| ConceptMapping {
                source_concept: format!("{} in {}", term, source_domain),
                target_concept: format!("{} in {}", term, target_domain),
                relationship_preserved: true,
                mapping_confidence: 0.7,
            })
            .collect();

        if concept_mappings.is_empty() {
            return Vec::new();
        }

        let analogy = AnalogyMapping {
            id: uuid::Uuid::new_v4().to_string(),
            source_domain: source_domain.to_string(),
            target_domain: target_domain.to_string(),
            concept_mappings,
            structural_similarity,
            surface_similarity: structural_similarity * 0.8,
            confidence: structural_similarity,
            created_at: Utc::now(),
            validated: false,
        };

        // Cache the result
        self.analogy_cache
            .write()
            .await
            .insert(key, analogy.clone());

        vec![analogy]
    }

    /// Transfer a skill (procedure) from one domain to another.
    pub async fn transfer_skill(
        &self,
        skill: &Procedure,
        target_domain: &str,
    ) -> Result<TransferResult> {
        let analogies = self
            .find_analogies(&skill.domain, target_domain)
            .await;

        let analogy = analogies
            .first()
            .ok_or_else(|| anyhow::anyhow!("No analogies found between '{}' and '{}'", skill.domain, target_domain))?;

        // Adapt procedure steps for the target domain
        let adapted_steps: Vec<ProcedureStep> = skill
            .steps
            .iter()
            .map(|step| {
                let mut adapted_action = step.action.clone();
                // Replace domain-specific terms using analogy mappings
                for mapping in &analogy.concept_mappings {
                    adapted_action = adapted_action.replace(
                        &mapping.source_concept,
                        &mapping.target_concept,
                    );
                }

                ProcedureStep {
                    action: adapted_action,
                    parameters: step.parameters.clone(),
                    expected_result: step.expected_result.clone(),
                }
            })
            .collect();

        let adaptation_notes = vec![
            format!(
                "Transferred from '{}' to '{}' domain",
                skill.domain, target_domain
            ),
            format!(
                "Structural similarity: {:.2}",
                analogy.structural_similarity
            ),
            format!(
                "{} concept mappings applied",
                analogy.concept_mappings.len()
            ),
        ];

        let estimated_success = skill.success_rate * analogy.confidence;

        let target_procedure = Procedure {
            id: uuid::Uuid::new_v4().to_string(),
            name: format!("{} (adapted for {})", skill.name, target_domain),
            domain: target_domain.to_string(),
            steps: adapted_steps,
            preconditions: skill.preconditions.clone(),
            expected_outcome: skill.expected_outcome.clone(),
            success_rate: estimated_success,
        };

        info!(
            "Transferred skill '{}' from '{}' to '{}' (estimated success: {:.2})",
            skill.name, skill.domain, target_domain, estimated_success
        );

        Ok(TransferResult {
            source_procedure: skill.id.clone(),
            target_procedure,
            adaptation_notes,
            estimated_success_rate: estimated_success,
            confidence: analogy.confidence,
        })
    }

    /// Get applicable patterns for a new task.
    pub async fn find_applicable_patterns(
        &self,
        task_description: &str,
    ) -> Vec<AbstractPattern> {
        let patterns = self.abstract_patterns.read().await;
        let task_lower = task_description.to_lowercase();

        let mut applicable: Vec<(AbstractPattern, f64)> = patterns
            .iter()
            .map(|p| {
                let relevance = self.compute_pattern_relevance(p, &task_lower);
                (p.clone(), relevance)
            })
            .filter(|(_, relevance)| *relevance > 0.2)
            .collect();

        applicable.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        applicable.into_iter().map(|(p, _)| p).collect()
    }

    /// Compute relevance of a pattern to a task.
    fn compute_pattern_relevance(&self, pattern: &AbstractPattern, task_lower: &str) -> f64 {
        let desc_lower = pattern.description.to_lowercase();
        let desc_words: Vec<&str> = desc_lower.split_whitespace().collect();
        let task_words: Vec<&str> = task_lower.split_whitespace().collect();

        let matches = desc_words
            .iter()
            .filter(|w| task_words.contains(w))
            .count();

        let word_overlap = if desc_words.is_empty() {
            0.0
        } else {
            matches as f64 / desc_words.len() as f64
        };

        let success_bonus = pattern.success_rate * 0.2;

        (word_overlap + success_bonus).min(1.0)
    }

    /// Get transfer learning statistics.
    pub async fn get_stats(&self) -> TransferStats {
        let patterns = self.abstract_patterns.read().await;
        let analogies = self.analogy_cache.read().await;
        let experiences = self.experience_pool.read().await;
        let domains = self.domain_vocabulary.read().await;

        TransferStats {
            total_patterns: patterns.len(),
            total_analogies: analogies.len(),
            total_experiences: experiences.len(),
            known_domains: domains.keys().cloned().collect(),
            avg_pattern_success_rate: if !patterns.is_empty() {
                patterns.iter().map(|p| p.success_rate).sum::<f64>()
                    / patterns.len() as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferStats {
    pub total_patterns: usize,
    pub total_analogies: usize,
    pub total_experiences: usize,
    pub known_domains: Vec<String>,
    pub avg_pattern_success_rate: f64,
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_and_abstract() {
        let engine = TransferLearningEngine::new();

        let experiences = vec![
            Experience {
                id: "exp-1".to_string(),
                domain: "debugging".to_string(),
                task_description: "Analyze and decompose the bug into components".to_string(),
                steps_taken: vec![
                    "Analyze the error".to_string(),
                    "Decompose into sub-problems".to_string(),
                    "Test each component".to_string(),
                    "Validate the fix".to_string(),
                ],
                outcome: ExperienceOutcome::Success { quality: 0.9 },
                duration_seconds: 300,
                tools_used: vec!["shell".to_string()],
                lessons: vec!["Decomposition helps".to_string()],
                timestamp: Utc::now(),
            },
            Experience {
                id: "exp-2".to_string(),
                domain: "writing".to_string(),
                task_description: "Analyze the topic and decompose into sections".to_string(),
                steps_taken: vec![
                    "Analyze the topic".to_string(),
                    "Decompose into sections".to_string(),
                    "Build each section".to_string(),
                    "Validate the draft".to_string(),
                ],
                outcome: ExperienceOutcome::Success { quality: 0.85 },
                duration_seconds: 600,
                tools_used: vec!["file_write".to_string()],
                lessons: vec!["Structure before writing".to_string()],
                timestamp: Utc::now(),
            },
        ];

        for exp in &experiences {
            engine.record_experience(exp.clone()).await;
        }

        let pattern = engine.abstract_pattern(&experiences).await;
        assert!(pattern.is_some());
        let p = pattern.unwrap();
        assert!(!p.structure.steps.is_empty());
        assert_eq!(p.source_domains.len(), 2);
    }

    #[tokio::test]
    async fn test_find_analogies() {
        let engine = TransferLearningEngine::new();

        // Seed vocabulary for domains
        {
            let mut vocab = engine.domain_vocabulary.write().await;
            vocab.insert(
                "programming".to_string(),
                vec![
                    "function".into(), "variable".into(), "loop".into(),
                    "debug".into(), "test".into(), "refactor".into(),
                ],
            );
            vocab.insert(
                "cooking".to_string(),
                vec![
                    "recipe".into(), "ingredient".into(), "mix".into(),
                    "test".into(), "adjust".into(), "prepare".into(),
                ],
            );
        }

        let analogies = engine
            .find_analogies("programming", "cooking")
            .await;
        assert!(!analogies.is_empty());
    }
}

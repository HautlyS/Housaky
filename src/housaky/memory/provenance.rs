//! Provenance Tracking (Knowledge Lineage)
//!
//! Tracks how each piece of knowledge was derived:
//! - Source type (observation, inference, external, user, self-generated)
//! - Reasoning chain ID that produced the knowledge
//! - Supporting evidence and confidence at creation
//! - "Why do I believe X?" trace queries
//! - Conflict resolution based on provenance quality

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

// ── Core Types ───────────────────────────────────────────────────────────────

/// Provenance metadata attached to any knowledge item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub id: String,
    pub knowledge_item_id: String,
    pub source_type: ProvenanceSource,
    pub reasoning_chain_id: Option<String>,
    pub evidence: Vec<EvidenceLink>,
    pub created_at: DateTime<Utc>,
    pub confidence_at_creation: f64,
    pub derived_from: Vec<String>, // IDs of parent knowledge items
    pub version: u32,
    pub last_verified: Option<DateTime<Utc>>,
    pub verification_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProvenanceSource {
    DirectObservation,
    LLMInference { provider: String, model: String },
    Deduction { from_beliefs: Vec<String> },
    ExternalSource { url: String },
    UserProvided,
    SelfGenerated,
    TransferLearning { source_domain: String },
    Consensus { agent_ids: Vec<String> },
}

impl ProvenanceSource {
    /// Quality score for this source type (0.0 to 1.0).
    pub fn quality_score(&self) -> f64 {
        match self {
            ProvenanceSource::DirectObservation => 0.95,
            ProvenanceSource::UserProvided => 0.90,
            ProvenanceSource::ExternalSource { .. } => 0.75,
            ProvenanceSource::Consensus { agent_ids } => {
                0.70 + 0.05 * (agent_ids.len() as f64).min(6.0)
            }
            ProvenanceSource::LLMInference { .. } => 0.65,
            ProvenanceSource::Deduction { from_beliefs } => {
                0.60 + 0.05 * (from_beliefs.len() as f64).min(4.0)
            }
            ProvenanceSource::TransferLearning { .. } => 0.50,
            ProvenanceSource::SelfGenerated => 0.40,
        }
    }
}

/// A link to supporting evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceLink {
    pub evidence_type: EvidenceType,
    pub description: String,
    pub reference: String,
    pub strength: f64, // 0.0 to 1.0
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvidenceType {
    Observation,
    ToolOutput,
    LLMResponse,
    UserStatement,
    WebSource,
    InternalReasoning,
    ExperimentResult,
    HistoricalPattern,
}

/// Result of a provenance trace query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceTrace {
    pub query: String,
    pub root_item_id: String,
    pub trace_chain: Vec<ProvenanceNode>,
    pub total_depth: usize,
    pub overall_confidence: f64,
    pub weakest_link: Option<ProvenanceNode>,
}

/// A node in a provenance trace chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceNode {
    pub knowledge_item_id: String,
    pub source_type: ProvenanceSource,
    pub confidence: f64,
    pub depth: usize,
    pub evidence_count: usize,
    pub description: String,
}

/// Result of provenance-based conflict resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub item_a_id: String,
    pub item_b_id: String,
    pub winner_id: String,
    pub winner_score: f64,
    pub loser_score: f64,
    pub resolution_basis: String,
}

// ── Provenance Tracker ───────────────────────────────────────────────────────

pub struct ProvenanceTracker {
    pub provenance_records: Arc<RwLock<HashMap<String, Provenance>>>,
    pub conflict_history: Arc<RwLock<Vec<ConflictResolution>>>,
}

impl ProvenanceTracker {
    pub fn new() -> Self {
        Self {
            provenance_records: Arc::new(RwLock::new(HashMap::new())),
            conflict_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Attach provenance to a knowledge item.
    pub async fn attach_provenance(
        &self,
        knowledge_item_id: &str,
        source: ProvenanceSource,
        evidence: Vec<EvidenceLink>,
        derived_from: Vec<String>,
        confidence: f64,
        reasoning_chain_id: Option<String>,
    ) -> String {
        let provenance_id = uuid::Uuid::new_v4().to_string();

        let provenance = Provenance {
            id: provenance_id.clone(),
            knowledge_item_id: knowledge_item_id.to_string(),
            source_type: source,
            reasoning_chain_id,
            evidence,
            created_at: Utc::now(),
            confidence_at_creation: confidence,
            derived_from,
            version: 1,
            last_verified: None,
            verification_count: 0,
        };

        self.provenance_records
            .write()
            .await
            .insert(knowledge_item_id.to_string(), provenance);

        info!(
            "Attached provenance '{}' to knowledge item '{}'",
            provenance_id, knowledge_item_id
        );

        provenance_id
    }

    /// Get provenance for a knowledge item.
    pub async fn get_provenance(&self, knowledge_item_id: &str) -> Option<Provenance> {
        self.provenance_records
            .read()
            .await
            .get(knowledge_item_id)
            .cloned()
    }

    /// Trace the full provenance chain: "Why do I believe X?"
    pub async fn trace_provenance(
        &self,
        knowledge_item_id: &str,
    ) -> ProvenanceTrace {
        let records = self.provenance_records.read().await;
        let mut chain = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        queue.push_back((knowledge_item_id.to_string(), 0usize));

        while let Some((item_id, depth)) = queue.pop_front() {
            if !visited.insert(item_id.clone()) {
                continue;
            }

            if let Some(prov) = records.get(&item_id) {
                let node = ProvenanceNode {
                    knowledge_item_id: item_id.clone(),
                    source_type: prov.source_type.clone(),
                    confidence: prov.confidence_at_creation,
                    depth,
                    evidence_count: prov.evidence.len(),
                    description: format!(
                        "{:?} (confidence: {:.2}, {} evidence items)",
                        prov.source_type,
                        prov.confidence_at_creation,
                        prov.evidence.len()
                    ),
                };
                chain.push(node);

                // Follow derivation chain
                for parent_id in &prov.derived_from {
                    if !visited.contains(parent_id) {
                        queue.push_back((parent_id.clone(), depth + 1));
                    }
                }
            }
        }

        let overall_confidence = if chain.is_empty() {
            0.0
        } else {
            // Confidence diminishes with chain length
            chain
                .iter()
                .map(|n| n.confidence)
                .fold(1.0, |acc, c| acc * c)
                .powf(1.0 / chain.len() as f64) // geometric mean
        };

        let weakest_link = chain
            .iter()
            .min_by(|a, b| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned();

        let total_depth = chain.iter().map(|n| n.depth).max().unwrap_or(0);

        ProvenanceTrace {
            query: format!("Why do I believe '{}'?", knowledge_item_id),
            root_item_id: knowledge_item_id.to_string(),
            trace_chain: chain,
            total_depth,
            overall_confidence,
            weakest_link,
        }
    }

    /// Resolve a conflict between two knowledge items using provenance quality.
    pub async fn resolve_conflict(
        &self,
        item_a_id: &str,
        item_b_id: &str,
    ) -> ConflictResolution {
        let records = self.provenance_records.read().await;

        let score_a = records
            .get(item_a_id)
            .map(|p| self.compute_quality_score(p))
            .unwrap_or(0.0);

        let score_b = records
            .get(item_b_id)
            .map(|p| self.compute_quality_score(p))
            .unwrap_or(0.0);

        let (winner_id, winner_score, loser_score) = if score_a >= score_b {
            (item_a_id.to_string(), score_a, score_b)
        } else {
            (item_b_id.to_string(), score_b, score_a)
        };

        let resolution = ConflictResolution {
            item_a_id: item_a_id.to_string(),
            item_b_id: item_b_id.to_string(),
            winner_id: winner_id.clone(),
            winner_score,
            loser_score,
            resolution_basis: format!(
                "Provenance quality comparison: winner ({:.3}) vs loser ({:.3})",
                winner_score, loser_score
            ),
        };

        self.conflict_history.write().await.push(resolution.clone());

        info!(
            "Conflict resolved: '{}' wins (score: {:.3} vs {:.3})",
            winner_id, winner_score, loser_score
        );

        resolution
    }

    /// Compute an overall quality score for a provenance record.
    fn compute_quality_score(&self, provenance: &Provenance) -> f64 {
        let source_quality = provenance.source_type.quality_score();

        // Evidence quality: more evidence = higher quality
        let evidence_factor = 1.0 - (1.0 / (1.0 + provenance.evidence.len() as f64));

        // Average evidence strength
        let avg_evidence_strength = if !provenance.evidence.is_empty() {
            provenance.evidence.iter().map(|e| e.strength).sum::<f64>()
                / provenance.evidence.len() as f64
        } else {
            0.5
        };

        // Verification bonus
        let verification_bonus = 0.05 * provenance.verification_count.min(5) as f64;

        // Age penalty (very old provenance is less reliable)
        let age_hours = (Utc::now() - provenance.created_at).num_hours() as f64;
        let age_penalty = if age_hours > 720.0 {
            // > 30 days
            0.1
        } else {
            0.0
        };

        let score = source_quality * 0.4
            + provenance.confidence_at_creation * 0.25
            + evidence_factor * 0.15
            + avg_evidence_strength * 0.10
            + verification_bonus * 0.10
            - age_penalty;

        score.clamp(0.0, 1.0)
    }

    /// Mark a provenance record as verified.
    pub async fn verify(&self, knowledge_item_id: &str) -> Result<()> {
        let mut records = self.provenance_records.write().await;
        let record = records
            .get_mut(knowledge_item_id)
            .ok_or_else(|| anyhow::anyhow!("No provenance for '{}'", knowledge_item_id))?;

        record.last_verified = Some(Utc::now());
        record.verification_count += 1;
        Ok(())
    }

    /// Get provenance statistics.
    pub async fn get_stats(&self) -> ProvenanceStats {
        let records = self.provenance_records.read().await;
        let conflicts = self.conflict_history.read().await;

        let mut by_source: HashMap<String, usize> = HashMap::new();
        let mut total_evidence = 0usize;
        let mut avg_confidence = 0.0f64;

        for prov in records.values() {
            *by_source
                .entry(format!("{:?}", prov.source_type))
                .or_insert(0) += 1;
            total_evidence += prov.evidence.len();
            avg_confidence += prov.confidence_at_creation;
        }

        if !records.is_empty() {
            avg_confidence /= records.len() as f64;
        }

        ProvenanceStats {
            total_records: records.len(),
            by_source_type: by_source,
            total_evidence_links: total_evidence,
            avg_confidence,
            total_conflicts_resolved: conflicts.len(),
            verified_count: records
                .values()
                .filter(|p| p.verification_count > 0)
                .count(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceStats {
    pub total_records: usize,
    pub by_source_type: HashMap<String, usize>,
    pub total_evidence_links: usize,
    pub avg_confidence: f64,
    pub total_conflicts_resolved: usize,
    pub verified_count: usize,
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_attach_and_get_provenance() {
        let tracker = ProvenanceTracker::new();

        tracker
            .attach_provenance(
                "fact-1",
                ProvenanceSource::DirectObservation,
                vec![EvidenceLink {
                    evidence_type: EvidenceType::ToolOutput,
                    description: "File listing showed the file exists".to_string(),
                    reference: "tool:ls".to_string(),
                    strength: 0.95,
                    timestamp: Utc::now(),
                }],
                vec![],
                0.95,
                None,
            )
            .await;

        let prov = tracker.get_provenance("fact-1").await;
        assert!(prov.is_some());
        assert_eq!(prov.unwrap().source_type, ProvenanceSource::DirectObservation);
    }

    #[tokio::test]
    async fn test_provenance_trace() {
        let tracker = ProvenanceTracker::new();

        // Create a chain: fact-1 → fact-2 → fact-3
        tracker
            .attach_provenance(
                "fact-1",
                ProvenanceSource::DirectObservation,
                vec![],
                vec![],
                0.95,
                None,
            )
            .await;

        tracker
            .attach_provenance(
                "fact-2",
                ProvenanceSource::Deduction {
                    from_beliefs: vec!["fact-1".to_string()],
                },
                vec![],
                vec!["fact-1".to_string()],
                0.85,
                Some("chain-1".to_string()),
            )
            .await;

        let trace = tracker.trace_provenance("fact-2").await;
        assert_eq!(trace.trace_chain.len(), 2);
        assert!(trace.overall_confidence > 0.0);
    }

    #[tokio::test]
    async fn test_conflict_resolution() {
        let tracker = ProvenanceTracker::new();

        tracker
            .attach_provenance(
                "belief-a",
                ProvenanceSource::DirectObservation,
                vec![EvidenceLink {
                    evidence_type: EvidenceType::Observation,
                    description: "Directly observed".to_string(),
                    reference: "test".to_string(),
                    strength: 0.9,
                    timestamp: Utc::now(),
                }],
                vec![],
                0.95,
                None,
            )
            .await;

        tracker
            .attach_provenance(
                "belief-b",
                ProvenanceSource::SelfGenerated,
                vec![],
                vec![],
                0.3,
                None,
            )
            .await;

        let resolution = tracker.resolve_conflict("belief-a", "belief-b").await;
        assert_eq!(resolution.winner_id, "belief-a");
    }
}

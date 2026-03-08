//! Karma System: Non-monetary reputation-based incentive mechanism
//!
//! Based on Purdue 2025 lightweight reputation research. Karma replaces
//! financial incentives with reputation tracking, tiering, and free-rider detection.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Karma tiers with escalating benefits
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum KarmaTier {
    /// 0-100 karma: basic access
    Seeker,
    /// 100-1,000 karma: early features, community badge
    Contributor,
    /// 1,000-10,000 karma: voting rights, credits listing
    Devotee,
    /// 10,000+ karma: core community, direct coordination
    Enlightened,
}

impl KarmaTier {
    pub fn from_points(points: f64) -> Self {
        match points {
            p if p >= 10_000.0 => Self::Enlightened,
            p if p >= 1_000.0 => Self::Devotee,
            p if p >= 100.0 => Self::Contributor,
            _ => Self::Seeker,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Seeker => "Seeker",
            Self::Contributor => "Contributor",
            Self::Devotee => "Devotee",
            Self::Enlightened => "Enlightened",
        }
    }
}

/// Types of contributions that earn karma
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ContributionType {
    Compute,
    Inference,
    Validation,
    Knowledge,
    BugReport,
    Code,
}

/// A single contribution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contribution {
    pub contribution_type: ContributionType,
    pub base_karma: f64,
    pub quality_multiplier: f64,
    pub final_karma: f64,
    pub description: String,
    pub timestamp: DateTime<Utc>,
}

/// Per-peer karma tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerKarma {
    pub peer_id: String,
    pub total_points: f64,
    pub tier: KarmaTier,
    pub contributions_count: u64,
    pub validations_count: u64,
    pub compute_contributions: u64,
    pub inference_contributions: u64,
    pub knowledge_contributions: u64,
    pub code_contributions: u64,
    pub benefits_consumed: f64,
    pub flagged: bool,
    pub last_contribution: DateTime<Utc>,
}

impl PeerKarma {
    pub fn new(peer_id: String) -> Self {
        Self {
            peer_id,
            total_points: 0.0,
            tier: KarmaTier::Seeker,
            contributions_count: 0,
            validations_count: 0,
            compute_contributions: 0,
            inference_contributions: 0,
            knowledge_contributions: 0,
            code_contributions: 0,
            benefits_consumed: 0.0,
            flagged: false,
            last_contribution: Utc::now(),
        }
    }

    /// Benefit-to-contribution ratio (for free-rider detection)
    pub fn benefit_ratio(&self) -> f64 {
        if self.total_points > 0.0 {
            self.benefits_consumed / self.total_points
        } else if self.benefits_consumed > 0.0 {
            f64::INFINITY
        } else {
            0.0
        }
    }
}

/// The Karma System manages non-monetary incentives for the Seed Mind Network
pub struct KarmaSystem {
    /// Per-peer karma records
    peers: HashMap<String, PeerKarma>,
    /// Local node's peer ID
    local_peer_id: String,
    /// Base karma values per contribution type
    base_karma: HashMap<ContributionType, f64>,
    /// Free-rider detection threshold (benefit/contribution ratio)
    free_rider_threshold: f64,
    /// Minimum contributions before flagging
    free_rider_min_contributions: u64,
    /// Contribution history
    history: Vec<Contribution>,
}

impl KarmaSystem {
    pub fn new() -> Self {
        let local_peer_id = uuid::Uuid::new_v4().to_string();
        let mut base_karma = HashMap::new();
        base_karma.insert(ContributionType::Compute, 10.0);
        base_karma.insert(ContributionType::Inference, 1.0);
        base_karma.insert(ContributionType::Validation, 5.0);
        base_karma.insert(ContributionType::Knowledge, 3.0);
        base_karma.insert(ContributionType::BugReport, 2.0);
        base_karma.insert(ContributionType::Code, 15.0);

        let mut peers = HashMap::new();
        peers.insert(
            local_peer_id.clone(),
            PeerKarma::new(local_peer_id.clone()),
        );

        Self {
            peers,
            local_peer_id,
            base_karma,
            free_rider_threshold: 3.0,
            free_rider_min_contributions: 10,
            history: Vec::new(),
        }
    }

    /// Record a contribution from a peer
    pub fn record_contribution(
        &mut self,
        peer_id: &str,
        contribution_type: ContributionType,
        quality_multiplier: f64,
        description: String,
    ) -> f64 {
        let quality = quality_multiplier.clamp(0.5, 2.0);

        // Discriminant key for base karma lookup
        let base = match contribution_type {
            ContributionType::Compute => 10.0,
            ContributionType::Inference => 1.0,
            ContributionType::Validation => 5.0,
            ContributionType::Knowledge => 3.0,
            ContributionType::BugReport => 2.0,
            ContributionType::Code => 15.0,
        };
        let final_karma = base * quality;

        let peer = self
            .peers
            .entry(peer_id.to_string())
            .or_insert_with(|| PeerKarma::new(peer_id.to_string()));
        peer.total_points += final_karma;
        peer.contributions_count += 1;
        peer.last_contribution = Utc::now();
        peer.tier = KarmaTier::from_points(peer.total_points);

        match contribution_type {
            ContributionType::Compute => peer.compute_contributions += 1,
            ContributionType::Inference => peer.inference_contributions += 1,
            ContributionType::Validation => {
                peer.validations_count += 1;
            }
            ContributionType::Knowledge => peer.knowledge_contributions += 1,
            ContributionType::Code => peer.code_contributions += 1,
            ContributionType::BugReport => {}
        }

        self.history.push(Contribution {
            contribution_type,
            base_karma: base,
            quality_multiplier: quality,
            final_karma,
            description,
            timestamp: Utc::now(),
        });

        // Keep history bounded
        if self.history.len() > 10_000 {
            self.history.drain(0..5_000);
        }

        final_karma
    }

    /// Record that a peer consumed a benefit
    pub fn record_benefit(&mut self, peer_id: &str, benefit_value: f64) {
        let peer = self
            .peers
            .entry(peer_id.to_string())
            .or_insert_with(|| PeerKarma::new(peer_id.to_string()));
        peer.benefits_consumed += benefit_value;
    }

    /// Detect free-riders: peers consuming much more than they contribute
    pub fn detect_free_riders(&mut self) -> Vec<String> {
        let mut free_riders = Vec::new();
        for (id, peer) in &mut self.peers {
            if peer.contributions_count >= self.free_rider_min_contributions
                && peer.benefit_ratio() > self.free_rider_threshold
            {
                peer.flagged = true;
                free_riders.push(id.clone());
            }
        }
        free_riders
    }

    /// Get local node's karma
    pub fn get_local_karma(&self) -> PeerKarma {
        self.peers
            .get(&self.local_peer_id)
            .cloned()
            .unwrap_or_else(|| PeerKarma::new(self.local_peer_id.clone()))
    }

    /// Get karma for a specific peer
    pub fn get_peer_karma(&self, peer_id: &str) -> Option<&PeerKarma> {
        self.peers.get(peer_id)
    }

    /// Get all peers sorted by karma (descending)
    pub fn leaderboard(&self) -> Vec<&PeerKarma> {
        let mut peers: Vec<&PeerKarma> = self.peers.values().collect();
        peers.sort_by(|a, b| b.total_points.partial_cmp(&a.total_points).unwrap());
        peers
    }

    /// Total karma in the network
    pub fn total_network_karma(&self) -> f64 {
        self.peers.values().map(|p| p.total_points).sum()
    }
}

impl Default for KarmaSystem {
    fn default() -> Self {
        Self::new()
    }
}

// Manual hash impl for ContributionType to use in HashMap
impl std::hash::Hash for ContributionType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}
impl Eq for ContributionType {}
impl PartialEq for ContributionType {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_karma_tiers() {
        assert_eq!(KarmaTier::from_points(0.0), KarmaTier::Seeker);
        assert_eq!(KarmaTier::from_points(50.0), KarmaTier::Seeker);
        assert_eq!(KarmaTier::from_points(100.0), KarmaTier::Contributor);
        assert_eq!(KarmaTier::from_points(500.0), KarmaTier::Contributor);
        assert_eq!(KarmaTier::from_points(1_000.0), KarmaTier::Devotee);
        assert_eq!(KarmaTier::from_points(5_000.0), KarmaTier::Devotee);
        assert_eq!(KarmaTier::from_points(10_000.0), KarmaTier::Enlightened);
        assert_eq!(KarmaTier::from_points(50_000.0), KarmaTier::Enlightened);
    }

    #[test]
    fn test_record_contribution() {
        let mut ks = KarmaSystem::new();
        let local_id = ks.local_peer_id.clone();

        let karma = ks.record_contribution(
            &local_id,
            ContributionType::Code,
            1.5,
            "Improved reasoning module".to_string(),
        );

        assert_eq!(karma, 15.0 * 1.5);
        let local = ks.get_local_karma();
        assert_eq!(local.contributions_count, 1);
        assert_eq!(local.code_contributions, 1);
    }

    #[test]
    fn test_quality_multiplier_clamping() {
        let mut ks = KarmaSystem::new();
        let peer = "peer-1";

        // Quality below 0.5 should be clamped to 0.5
        let karma = ks.record_contribution(peer, ContributionType::Compute, 0.1, "test".into());
        assert_eq!(karma, 10.0 * 0.5);

        // Quality above 2.0 should be clamped to 2.0
        let karma = ks.record_contribution(peer, ContributionType::Compute, 5.0, "test".into());
        assert_eq!(karma, 10.0 * 2.0);
    }

    #[test]
    fn test_free_rider_detection() {
        let mut ks = KarmaSystem::new();
        let peer = "free-rider-1";

        // Give minimal contributions
        for _ in 0..10 {
            ks.record_contribution(peer, ContributionType::Inference, 0.5, "tiny".into());
        }
        // Consume a lot of benefits
        ks.record_benefit(peer, 1000.0);

        let free_riders = ks.detect_free_riders();
        assert!(free_riders.contains(&peer.to_string()));

        let peer_karma = ks.get_peer_karma(peer).unwrap();
        assert!(peer_karma.flagged);
    }

    #[test]
    fn test_leaderboard() {
        let mut ks = KarmaSystem::new();
        ks.record_contribution("alice", ContributionType::Code, 2.0, "feature".into());
        ks.record_contribution("bob", ContributionType::Compute, 1.0, "training".into());

        let lb = ks.leaderboard();
        assert!(lb.len() >= 2);
        // Alice should rank higher (code = 15 * 2.0 = 30 vs compute = 10 * 1.0 = 10)
        let alice_rank = lb.iter().position(|p| p.peer_id == "alice").unwrap();
        let bob_rank = lb.iter().position(|p| p.peer_id == "bob").unwrap();
        assert!(alice_rank < bob_rank);
    }

    #[test]
    fn test_total_network_karma() {
        let mut ks = KarmaSystem::new();
        ks.record_contribution("alice", ContributionType::Code, 1.0, "code".into());
        ks.record_contribution("bob", ContributionType::Compute, 1.0, "compute".into());

        let total = ks.total_network_karma();
        assert_eq!(total, 25.0); // 15 + 10
    }

    #[test]
    fn test_benefit_ratio() {
        let mut pk = PeerKarma::new("test".to_string());
        assert_eq!(pk.benefit_ratio(), 0.0);

        pk.total_points = 100.0;
        pk.benefits_consumed = 200.0;
        assert_eq!(pk.benefit_ratio(), 2.0);
    }
}

//! Federation sync utilities
//!
//! CRDT-inspired merge operations for conflict-free replication.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Last-Writer-Wins Register for conflict-free merging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LWWRegister<T: Clone> {
    pub value: T,
    pub timestamp: DateTime<Utc>,
    pub source: String,
}

impl<T: Clone> LWWRegister<T> {
    pub fn new(value: T, source: &str) -> Self {
        Self {
            value,
            timestamp: Utc::now(),
            source: source.to_string(),
        }
    }

    pub fn merge(&mut self, other: &LWWRegister<T>) {
        if other.timestamp > self.timestamp {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
            self.source = other.source.clone();
        }
    }
}

/// Grow-only Set for items that should never be deleted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GSet<T: Clone + Eq + std::hash::Hash> {
    pub items: std::collections::HashSet<T>,
}

impl<T: Clone + Eq + std::hash::Hash> GSet<T> {
    pub fn new() -> Self {
        Self {
            items: std::collections::HashSet::new(),
        }
    }

    pub fn add(&mut self, item: T) {
        self.items.insert(item);
    }

    pub fn merge(&mut self, other: &GSet<T>) {
        for item in &other.items {
            self.items.insert(item.clone());
        }
    }

    pub fn contains(&self, item: &T) -> bool {
        self.items.contains(item)
    }
}

/// Vector clock for causality tracking across peers.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorClock {
    pub clocks: HashMap<String, u64>,
}

impl VectorClock {
    pub fn new() -> Self {
        Self { clocks: HashMap::new() }
    }

    pub fn tick(&mut self, peer_id: &str) {
        *self.clocks.entry(peer_id.to_string()).or_insert(0) += 1;
    }

    pub fn merge(&mut self, other: &VectorClock) {
        for (peer, &clock) in &other.clocks {
            let entry = self.clocks.entry(peer.clone()).or_insert(0);
            *entry = (*entry).max(clock);
        }
    }

    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let mut at_least_one_less = false;
        for (peer, &clock) in &self.clocks {
            let other_clock = other.clocks.get(peer).copied().unwrap_or(0);
            if clock > other_clock {
                return false;
            }
            if clock < other_clock {
                at_least_one_less = true;
            }
        }
        // Check for peers in other that we don't have
        for (peer, &clock) in &other.clocks {
            if !self.clocks.contains_key(peer) && clock > 0 {
                at_least_one_less = true;
            }
        }
        at_least_one_less
    }

    pub fn concurrent(&self, other: &VectorClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self) && self.clocks != other.clocks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lww_register() {
        let mut reg_a = LWWRegister::new("hello".to_string(), "peer-a");
        let reg_b = LWWRegister::new("world".to_string(), "peer-b");
        reg_a.merge(&reg_b);
        assert_eq!(reg_a.value, "world");
    }

    #[test]
    fn test_gset() {
        let mut set_a = GSet::new();
        set_a.add("fact-1".to_string());
        let mut set_b = GSet::new();
        set_b.add("fact-2".to_string());
        set_a.merge(&set_b);
        assert!(set_a.contains(&"fact-1".to_string()));
        assert!(set_a.contains(&"fact-2".to_string()));
    }

    #[test]
    fn test_vector_clock() {
        let mut vc_a = VectorClock::new();
        vc_a.tick("peer-a");
        vc_a.tick("peer-a");

        let mut vc_b = VectorClock::new();
        vc_b.tick("peer-b");

        assert!(vc_a.concurrent(&vc_b));

        vc_b.merge(&vc_a);
        assert!(vc_a.happens_before(&vc_b));
    }
}

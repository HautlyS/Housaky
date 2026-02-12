//! Gossip protocol implementation
use anyhow::Result;

/// Gossip handler
pub struct GossipHandler {
    topics: Vec<String>,
}

impl GossipHandler {
    pub fn new() -> Self {
        Self { topics: Vec::new() }
    }

    pub fn subscribe(&mut self, topic: String) {
        if !self.topics.contains(&topic) {
            self.topics.push(topic);
        }
    }

    pub fn topics(&self) -> &[String] {
        &self.topics
    }
}

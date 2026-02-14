//! Gossip protocol implementation
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

const GOSSIP_FANOUT: usize = 3;
const GOSSIP_INTERVAL_MS: u64 = 100;
const MESSAGE_TTL_SECS: u64 = 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMessage {
    pub id: String,
    pub topic: String,
    pub payload: Vec<u8>,
    pub timestamp: u64,
    pub ttl: u8,
    pub sender: String,
}

/// Gossip handler
pub struct GossipHandler {
    topics: Arc<RwLock<HashSet<String>>>,
    seen_messages: Arc<RwLock<HashMap<String, SystemTime>>>,
    subscribers: Arc<RwLock<HashMap<String, Vec<tokio::sync::mpsc::Sender<GossipMessage>>>>>,
}

impl GossipHandler {
    pub fn new() -> Self {
        Self {
            topics: Arc::new(RwLock::new(HashSet::new())),
            seen_messages: Arc::new(RwLock::new(HashMap::new())),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn subscribe(&self, topic: String) -> tokio::sync::mpsc::Receiver<GossipMessage> {
        let mut topics = self.topics.write().await;
        topics.insert(topic.clone());
        
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let mut subs = self.subscribers.write().await;
        subs.entry(topic).or_insert_with(Vec::new).push(tx);
        
        rx
    }

    pub async fn publish(&self, topic: String, payload: Vec<u8>, sender: String) -> Result<()> {
        let msg = GossipMessage {
            id: uuid::Uuid::new_v4().to_string(),
            topic: topic.clone(),
            payload,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs(),
            ttl: 10,
            sender,
        };

        self.handle_message(msg).await
    }

    pub async fn handle_message(&self, msg: GossipMessage) -> Result<()> {
        // Check if already seen
        let mut seen = self.seen_messages.write().await;
        if seen.contains_key(&msg.id) {
            return Ok(());
        }
        seen.insert(msg.id.clone(), SystemTime::now());

        // Cleanup old messages
        seen.retain(|_, time| {
            SystemTime::now()
                .duration_since(*time)
                .map(|d| d.as_secs() < MESSAGE_TTL_SECS)
                .unwrap_or(false)
        });

        // Forward to subscribers
        let subs = self.subscribers.read().await;
        if let Some(topic_subs) = subs.get(&msg.topic) {
            for tx in topic_subs {
                let _ = tx.try_send(msg.clone());
            }
        }

        Ok(())
    }

    pub async fn topics(&self) -> Vec<String> {
        self.topics.read().await.iter().cloned().collect()
    }

    pub fn gossip_fanout(&self) -> usize {
        GOSSIP_FANOUT
    }

    pub fn gossip_interval(&self) -> Duration {
        Duration::from_millis(GOSSIP_INTERVAL_MS)
    }
}

impl Default for GossipHandler {
    fn default() -> Self {
        Self::new()
    }
}

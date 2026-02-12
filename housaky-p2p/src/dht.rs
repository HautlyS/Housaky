//! Distributed Hash Table (Kademlia-based) - Production Implementation

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

const K_BUCKET_SIZE: usize = 20;
const ALPHA: usize = 3;
const KEY_SIZE: usize = 32;
const VALUE_TTL_SECS: u64 = 3600;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct NodeId(pub [u8; KEY_SIZE]);

impl NodeId {
    pub fn new(data: [u8; KEY_SIZE]) -> Self {
        Self(data)
    }

    pub fn random() -> Self {
        let mut data = [0u8; KEY_SIZE];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut data);
        Self(data)
    }

    pub fn distance(&self, other: &NodeId) -> u32 {
        self.0
            .iter()
            .zip(other.0.iter())
            .map(|(a, b)| (a ^ b).count_ones())
            .sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtValue {
    pub data: Vec<u8>,
    pub timestamp: SystemTime,
    pub publisher: NodeId,
}

#[derive(Debug, Clone)]
pub struct DhtNode {
    pub id: NodeId,
    pub address: String,
    pub last_seen: SystemTime,
}

/// Distributed Hash Table
pub struct DHT {
    node_id: NodeId,
    storage: Arc<RwLock<HashMap<NodeId, DhtValue>>>,
    routing_table: Arc<RwLock<Vec<Vec<DhtNode>>>>,
}

impl DHT {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            storage: Arc::new(RwLock::new(HashMap::new())),
            routing_table: Arc::new(RwLock::new(vec![Vec::new(); KEY_SIZE * 8])),
        }
    }

    pub fn new_random() -> Self {
        Self::new(NodeId::random())
    }

    pub async fn put(&self, key: NodeId, value: Vec<u8>) -> Result<()> {
        let mut storage = self.storage.write().await;
        storage.insert(
            key,
            DhtValue {
                data: value,
                timestamp: SystemTime::now(),
                publisher: self.node_id.clone(),
            },
        );
        Ok(())
    }

    pub async fn get(&self, key: &NodeId) -> Option<Vec<u8>> {
        let storage = self.storage.read().await;
        storage.get(key).map(|v| v.data.clone())
    }

    pub async fn find_node(&self, target: &NodeId) -> Vec<DhtNode> {
        let table = self.routing_table.read().await;
        let mut closest = Vec::new();

        for bucket in table.iter() {
            for node in bucket {
                closest.push(node.clone());
            }
        }

        closest.sort_by_key(|n| n.id.distance(target));
        closest.truncate(K_BUCKET_SIZE);
        closest
    }

    pub async fn add_node(&self, node: DhtNode) -> Result<()> {
        let distance = self.node_id.distance(&node.id);
        let bucket_idx = (distance.leading_zeros() as usize).min(KEY_SIZE * 8 - 1);

        let mut table = self.routing_table.write().await;
        let bucket = &mut table[bucket_idx];

        if let Some(pos) = bucket.iter().position(|n| n.id == node.id) {
            bucket[pos] = node;
        } else if bucket.len() < K_BUCKET_SIZE {
            bucket.push(node);
        } else {
            if let Some(oldest) = bucket.iter_mut().min_by_key(|n| n.last_seen) {
                *oldest = node;
            }
        }

        Ok(())
    }

    pub async fn cleanup_expired(&self) {
        let mut storage = self.storage.write().await;
        let now = SystemTime::now();

        storage.retain(|_, value| {
            now.duration_since(value.timestamp)
                .map(|d| d.as_secs() < VALUE_TTL_SECS)
                .unwrap_or(false)
        });
    }

    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    pub async fn size(&self) -> usize {
        self.storage.read().await.len()
    }

    pub fn alpha(&self) -> usize {
        ALPHA
    }

    pub fn k_bucket_size(&self) -> usize {
        K_BUCKET_SIZE
    }
}

// Keep legacy KademliaDHT for compatibility
pub struct KademliaDHT {
    routing_table: HashMap<String, Vec<String>>,
    k: usize,
    node_id: String,
}

impl KademliaDHT {
    pub fn new(node_id: String, k: usize) -> Self {
        Self {
            routing_table: HashMap::new(),
            k,
            node_id,
        }
    }

    pub fn find_node(&self, id: &str) -> Option<Vec<String>> {
        self.routing_table.get(id).cloned()
    }

    pub fn store(&mut self, key: String, value: Vec<String>) {
        self.routing_table.insert(key, value);
    }

    pub fn find_closest_nodes(&self, target: &str, count: usize) -> Vec<String> {
        let mut nodes: Vec<(String, usize)> = self.routing_table
            .keys()
            .map(|k| (k.clone(), self.xor_distance(target, k)))
            .collect();
        
        nodes.sort_by_key(|(_, dist)| *dist);
        nodes.truncate(count);
        nodes.into_iter().map(|(k, _)| k).collect()
    }

    fn xor_distance(&self, a: &str, b: &str) -> usize {
        a.bytes().zip(b.bytes()).map(|(x, y)| (x ^ y).count_ones() as usize).sum()
    }

    pub fn add_node(&mut self, node_id: String, peers: Vec<String>) {
        self.routing_table.insert(node_id, peers);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dht_creation() {
        let dht = KademliaDHT::new("node1".to_string(), 20);
        assert_eq!(dht.k, 20);
    }

    #[test]
    fn test_store_and_find() {
        let mut dht = KademliaDHT::new("node1".to_string(), 20);
        dht.store("key1".to_string(), vec!["value1".to_string()]);
        
        let result = dht.find_node("key1");
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_modern_dht() {
        let dht = DHT::new_random();
        let key = NodeId::random();
        let value = b"test data".to_vec();
        
        dht.put(key.clone(), value.clone()).await.unwrap();
        let retrieved = dht.get(&key).await;
        
        assert_eq!(retrieved, Some(value));
    }
}

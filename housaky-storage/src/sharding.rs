//! Consistent Hashing for Storage Sharding - Enhanced

use anyhow::Result;
use housaky_core::crypto::hash;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;

const DEFAULT_SHARD_COUNT: usize = 256;
const REPLICATION_FACTOR: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shard {
    pub id: usize,
    pub data: Vec<u8>,
    pub hash: [u8; 32],
    pub replicas: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMetadata {
    pub shard_id: usize,
    pub size: usize,
    pub hash: [u8; 32],
    pub nodes: Vec<String>,
}

pub struct ConsistentHashing {
    num_shards: usize,
    virtual_nodes: usize,
}

impl ConsistentHashing {
    pub fn new(num_shards: usize) -> Self {
        Self {
            num_shards,
            virtual_nodes: 150,
        }
    }

    pub fn get_shard<T: Hash>(&self, key: &T) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() % self.num_shards as u64) as usize
    }

    pub fn get_replica_shards<T: Hash>(&self, key: &T, replicas: usize) -> Vec<usize> {
        let primary = self.get_shard(key);
        let mut shards = vec![primary];
        
        for i in 1..replicas {
            let replica_shard = (primary + i) % self.num_shards;
            shards.push(replica_shard);
        }
        
        shards
    }

    pub fn rebalance(&self, old_shards: usize, new_shards: usize) -> Vec<(usize, usize)> {
        let mut migrations = Vec::new();
        
        for old_shard in 0..old_shards {
            let new_shard = (old_shard * new_shards) / old_shards;
            if old_shard != new_shard {
                migrations.push((old_shard, new_shard));
            }
        }
        
        migrations
    }
}

pub struct ShardManager {
    shard_count: usize,
    shards: Arc<RwLock<HashMap<usize, Shard>>>,
    node_assignments: Arc<RwLock<HashMap<String, Vec<usize>>>>,
    consistent_hash: ConsistentHashing,
}

impl ShardManager {
    pub fn new(shard_count: usize) -> Self {
        Self {
            shard_count,
            shards: Arc::new(RwLock::new(HashMap::new())),
            node_assignments: Arc::new(RwLock::new(HashMap::new())),
            consistent_hash: ConsistentHashing::new(shard_count),
        }
    }

    pub fn default() -> Self {
        Self::new(DEFAULT_SHARD_COUNT)
    }

    pub async fn shard_data(&self, data: &[u8]) -> Result<Vec<Shard>> {
        let chunk_size = (data.len() + self.shard_count - 1) / self.shard_count;
        let mut shards = Vec::new();

        for (id, chunk) in data.chunks(chunk_size).enumerate() {
            let chunk_vec = chunk.to_vec();
            let chunk_hash = hash(&chunk_vec);
            
            let shard = Shard {
                id,
                data: chunk_vec,
                hash: chunk_hash,
                replicas: Vec::new(),
            };
            
            shards.push(shard.clone());
            
            let mut shard_map = self.shards.write().await;
            shard_map.insert(id, shard);
        }

        Ok(shards)
    }

    pub async fn reassemble_shards(&self, shard_ids: &[usize]) -> Result<Vec<u8>> {
        let shards = self.shards.read().await;
        let mut data = Vec::new();

        for id in shard_ids {
            if let Some(shard) = shards.get(id) {
                let computed_hash = hash(&shard.data);
                if computed_hash != shard.hash {
                    return Err(anyhow::anyhow!("Shard {} hash mismatch", id));
                }
                data.extend_from_slice(&shard.data);
            } else {
                return Err(anyhow::anyhow!("Shard {} not found", id));
            }
        }

        Ok(data)
    }

    pub async fn assign_shard_to_node(&self, shard_id: usize, node_id: String) -> Result<()> {
        let mut assignments = self.node_assignments.write().await;
        assignments
            .entry(node_id.clone())
            .or_insert_with(Vec::new)
            .push(shard_id);

        let mut shards = self.shards.write().await;
        if let Some(shard) = shards.get_mut(&shard_id) {
            if !shard.replicas.contains(&node_id) {
                shard.replicas.push(node_id);
            }
        }

        Ok(())
    }

    pub async fn get_shard(&self, shard_id: usize) -> Option<Shard> {
        let shards = self.shards.read().await;
        shards.get(&shard_id).cloned()
    }

    pub async fn get_node_shards(&self, node_id: &str) -> Vec<usize> {
        let assignments = self.node_assignments.read().await;
        assignments.get(node_id).cloned().unwrap_or_default()
    }

    pub fn shard_count(&self) -> usize {
        self.shard_count
    }

    pub fn replication_factor(&self) -> usize {
        REPLICATION_FACTOR
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consistent_hashing() {
        let ch = ConsistentHashing::new(10);
        let shard = ch.get_shard(&"test_key");
        assert!(shard < 10);
    }

    #[test]
    fn test_replicas() {
        let ch = ConsistentHashing::new(10);
        let shards = ch.get_replica_shards(&"test_key", 3);
        assert_eq!(shards.len(), 3);
    }

    #[tokio::test]
    async fn test_shard_manager() {
        let manager = ShardManager::new(4);
        let data = b"Hello, World! This is test data.";
        
        let shards = manager.shard_data(data).await.unwrap();
        assert!(shards.len() <= 4);
        
        let shard_ids: Vec<usize> = shards.iter().map(|s| s.id).collect();
        let reassembled = manager.reassemble_shards(&shard_ids).await.unwrap();
        
        assert_eq!(data.as_slice(), reassembled.as_slice());
    }
}


//! Consistent Hashing for Storage Sharding

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

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
}

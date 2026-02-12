//! KV-Cache - Key-Value cache for fast inference

use std::collections::VecDeque;

pub struct KVCache {
    max_seq_len: usize,
    num_layers: usize,
    hidden_dim: usize,
    keys: Vec<VecDeque<Vec<f32>>>,
    values: Vec<VecDeque<Vec<f32>>>,
}

impl KVCache {
    pub fn new(max_seq_len: usize, num_layers: usize, hidden_dim: usize) -> Self {
        let mut keys = Vec::new();
        let mut values = Vec::new();
        
        for _ in 0..num_layers {
            keys.push(VecDeque::with_capacity(max_seq_len));
            values.push(VecDeque::with_capacity(max_seq_len));
        }
        
        Self {
            max_seq_len,
            num_layers,
            hidden_dim,
            keys,
            values,
        }
    }

    pub fn push(&mut self, layer: usize, key: Vec<f32>, value: Vec<f32>) {
        if layer >= self.num_layers {
            return;
        }
        
        if self.keys[layer].len() >= self.max_seq_len {
            self.keys[layer].pop_front();
            self.values[layer].pop_front();
        }
        
        self.keys[layer].push_back(key);
        self.values[layer].push_back(value);
    }

    pub fn get(&self, layer: usize) -> Option<(&VecDeque<Vec<f32>>, &VecDeque<Vec<f32>>)> {
        if layer >= self.num_layers {
            return None;
        }
        
        Some((&self.keys[layer], &self.values[layer]))
    }

    pub fn clear(&mut self) {
        for layer in 0..self.num_layers {
            self.keys[layer].clear();
            self.values[layer].clear();
        }
    }

    pub fn len(&self, layer: usize) -> usize {
        if layer >= self.num_layers {
            return 0;
        }
        self.keys[layer].len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kv_cache() {
        let mut cache = KVCache::new(100, 32, 128);
        
        let key = vec![1.0; 128];
        let value = vec![2.0; 128];
        
        cache.push(0, key, value);
        
        assert_eq!(cache.len(0), 1);
    }

    #[test]
    fn test_cache_overflow() {
        let mut cache = KVCache::new(2, 1, 4);
        
        cache.push(0, vec![1.0; 4], vec![1.0; 4]);
        cache.push(0, vec![2.0; 4], vec![2.0; 4]);
        cache.push(0, vec![3.0; 4], vec![3.0; 4]);
        
        assert_eq!(cache.len(0), 2);
    }
}

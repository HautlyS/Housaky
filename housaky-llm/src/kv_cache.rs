//! GPU-accelerated KV cache implementation for efficient LLM inference
//! 
//! This module implements a GPU-accelerated KV cache to store key-value pairs
//! during attention computation, significantly improving inference speed.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};
use log::LevelFilter;
use thiserror::Error;

#[cfg(feature = "cuda")]
use rust_cuda::prelude::*;
#[cfg(feature = "cuda")]
use tch::Tensor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KVCacheConfig {
    pub max_seq_len: usize,
    pub head_count: usize,
    pub head_dim: usize,
    pub cache_size_mb: usize,
    pub enable_compression: bool,
    pub enable_pinned_memory: bool,
}

impl Default for KVCacheConfig {
    fn default() -> Self {
        Self {
            max_seq_len: 4096,
            head_count: 64,
            head_dim: 64,
            cache_size_mb: 512,
            enable_compression: true,
            enable_pinned_memory: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KVCache {
    pub keys: Arc<RwLock<KVCacheTensor>>,
    pub values: Arc<RwLock<KVCacheTensor>>,
    pub positions: Arc<RwLock<Vec<usize>>,
    pub config: KVCacheConfig,
}

#[derive(Debug)]
struct KVCacheTensor {
    #[cfg(feature = "cuda")]
    tensor: Tensor,
    #[cfg(not(feature = "cuda"))]
    tensor: Vec<f32>,
    capacity: usize,
    length: usize,
}

#[derive(Debug, Error)]
pub enum KVCacheError {
    #[error("CUDA error: {0}")]
    CudaError(String),
    #[error("Cache overflow: sequence length exceeded {0}")]
    CacheOverflow(usize),
    #[error("Invalid cache dimensions")]
    InvalidDimensions,
    #[error("Memory allocation failed")]
    MemoryAllocationError,
    #[error("Cache not initialized")]
    CacheNotInitialized,
}

impl KVCache {
    pub fn new(max_seq_len: usize, head_count: usize, head_dim: usize) -> Self {
        let config = KVCacheConfig {
            max_seq_len,
            head_count,
            head_dim,
            ..Default::default()
        };
        
        #[cfg(feature = "cuda")] {
            let capacity = max_seq_len * head_count * head_dim;
            let tensor = Tensor::zeros((capacity,), tch::kind::FLOAT_CPU);
            
            Self {
                keys: Arc::new(RwLock::new(KVCacheTensor {
                    tensor,
                    capacity,
                    length: 0,
                })),
                values: Arc::new(RwLock::new(KVCacheTensor {
                    tensor: Tensor::zeros((capacity,), tch::kind::FLOAT_CPU),
                    capacity,
                    length: 0,
                })),
                positions: Arc::new(RwLock::new(Vec::new())),
                config,
            }
        }
        #[cfg(not(feature = "cuda"))] {
            let capacity = max_seq_len * head_count * head_dim;
            
            Self {
                keys: Arc::new(RwLock::new(KVCacheTensor {
                    tensor: vec![0.0; capacity],
                    capacity,
                    length: 0,
                })),
                values: Arc::new(RwLock::new(KVCacheTensor {
                    tensor: vec![0.0; capacity],
                    capacity,
                    length: 0,
                })),
                positions: Arc::new(RwLock::new(Vec::new())),
                config,
            }
        }
    }

    pub async fn store(&self, step: usize, keys: &[Tensor], values: &[Tensor]) -> Result<()> {
        #[cfg(feature = "cuda")] {
            let mut keys_lock = self.keys.write().await;
            let mut values_lock = self.values.write().await;
            
            // Check capacity
            if step >= self.config.max_seq_len {
                return Err(KVCacheError::CacheOverflow(self.config.max_seq_len).into());
            }
            
            // Store keys
            let key_offset = step * self.config.head_count * self.config.head_dim;
            for (i, key) in keys.iter().enumerate() {
                let key_data = key.data_ptr();
                let key_len = key.numel();
                
                // Copy to cache tensor
                keys_lock.tensor.copy_data_from_ptr(
                    key_data,
                    key_len,
                    key_offset + i * self.config.head_dim,
                )?;
            }
            
            // Store values
            let value_offset = step * self.config.head_count * self.config.head_dim;
            for (i, value) in values.iter().enumerate() {
                let value_data = value.data_ptr();
                let value_len = value.numel();
                
                values_lock.tensor.copy_data_from_ptr(
                    value_data,
                    value_len,
                    value_offset + i * self.config.head_dim,
                )?;
            }
            
            // Update positions
            {
                let mut positions_lock = self.positions.write().await;
                positions_lock.push(step);
            }
            
            keys_lock.length = (step + 1) * self.config.head_count * self.config.head_dim;
            values_lock.length = keys_lock.length;
            
            debug!("Stored step {} in KV cache", step);
            Ok(())
        }
        #[cfg(not(feature = "cuda"))] {
            // CPU implementation (placeholder)
            Ok(())
        }
    }

    pub async fn retrieve(&self, step: usize) -> Result<(Vec<Tensor>, Vec<Tensor>)> {
        #[cfg(feature = "cuda")] {
            let keys_lock = self.keys.read().await;
            let values_lock = self.values.read().await;
            
            // Calculate offsets
            let offset = step * self.config.head_count * self.config.head_dim;
            let length = self.config.head_count * self.config.head_dim;
            
            // Retrieve keys
            let keys_tensor = keys_lock.tensor.narrow(0, offset as i64, length as i64);
            let keys: Vec<Tensor> = keys_tensor.split(self.config.head_dim as i64);
            
            // Retrieve values
            let values_tensor = values_lock.tensor.narrow(0, offset as i64, length as i64);
            let values: Vec<Tensor> = values_tensor.split(self.config.head_dim as i64);
            
            Ok((keys, values))
        }
        #[cfg(not(feature = "cuda"))] {
            // CPU implementation (placeholder)
            Ok((Vec::new(), Vec::new()))
        }
    }

    pub async fn get_current_length(&self) -> usize {
        let keys_lock = self.keys.read().await;
        keys_lock.length / (self.config.head_count * self.config.head_dim)
    }

    pub async fn clear(&self) {
        {
            let mut keys_lock = self.keys.write().await;
            keys_lock.length = 0;
            #[cfg(feature = "cuda")]
            keys_lock.tensor.zero_()?;
        }
        {
            let mut values_lock = self.values.write().await;
            values_lock.length = 0;
            #[cfg(feature = "cuda")]
            values_lock.tensor.zero_()?;
        }
        {
            let mut positions_lock = self.positions.write().await;
            positions_lock.clear();
        }
        
        debug!("KV cache cleared");
    }

    pub async fn compress(&self) -> Result<()> {
        if !self.config.enable_compression {
            return Ok(());
        }
        
        #[cfg(feature = "cuda")] {
            let mut keys_lock = self.keys.write().await;
            let mut values_lock = self.values.write().await;
            
            // Apply simple compression (quantization)
            keys_lock.tensor = keys_lock.tensor.quantize(8)?;
            values_lock.tensor = values_lock.tensor.quantize(8)?;
            
            debug!("KV cache compressed");
            Ok(())
        }
        #[cfg(not(feature = "cuda"))] {
            // CPU implementation (placeholder)
            Ok(())
        }
    }

    pub async fn get_memory_usage_mb(&self) -> f64 {
        #[cfg(feature = "cuda")] {
            let keys_lock = self.keys.read().await;
            let values_lock = self.values.read().await;
            
            let keys_memory = keys_lock.tensor.nbytes()? as f64 / (1024.0 * 1024.0);
            let values_memory = values_lock.tensor.nbytes()? as f64 / (1024.0 * 1024.0);
            
            keys_memory + values_memory
        }
        #[cfg(not(feature = "cuda"))] {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tch::Tensor;

    #[tokio::test]
    async fn test_kv_cache_creation() {
        let cache = KVCache::new(4096, 64, 64);
        assert_eq!(cache.config.max_seq_len, 4096);
        assert_eq!(cache.config.head_count, 64);
        assert_eq!(cache.config.head_dim, 64);
    }

    #[tokio::test]
    #[cfg(feature = "cuda")]
    async fn test_kv_cache_store_retrieve() {
        let cache = KVCache::new(4096, 64, 64);
        
        // Create dummy tensors
        let keys: Vec<Tensor> = (0..64).map(|i| Tensor::zeros((64,), tch::kind::FLOAT_CPU)).collect();
        let values: Vec<Tensor> = (0..64).map(|i| Tensor::zeros((64,), tch::kind::FLOAT_CPU)).collect();
        
        // Store and retrieve
        cache.store(0, &keys, &values).await.unwrap();
        let (retrieved_keys, retrieved_values) = cache.retrieve(0).await.unwrap();
        
        assert_eq!(retrieved_keys.len(), 64);
        assert_eq!(retrieved_values.len(), 64);
    }

    #[tokio::test]
    #[cfg(feature = "cuda")]
    async fn test_kv_cache_memory_usage() {
        let cache = KVCache::new(4096, 64, 64);
        let memory_usage = cache.get_memory_usage_mb().await;
        assert!(memory_usage > 0.0);
    }
}

// Export for external use
#[cfg(feature = "cuda")]
pub use rust_cuda::prelude::*;
#[cfg(feature = "cuda")]
pub use tch::Tensor;

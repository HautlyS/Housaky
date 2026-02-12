//! Erasure coding for redundancy
use anyhow::Result;
use reed_solomon_erasure::galois_8::ReedSolomon;

/// Erasure coder
pub struct ErasureCoder {
    data_shards: usize,
    parity_shards: usize,
    encoder: ReedSolomon,
}

impl ErasureCoder {
    pub fn new(data_shards: usize, parity_shards: usize) -> Result<Self> {
        let encoder = ReedSolomon::new(data_shards, parity_shards)?;
        Ok(Self {
            data_shards,
            parity_shards,
            encoder,
        })
    }

    pub fn encode(&self, data: &[u8]) -> Result<Vec<Vec<u8>>> {
        let shard_size = (data.len() + self.data_shards - 1) / self.data_shards;
        let total_shards = self.data_shards + self.parity_shards;

        let mut shards: Vec<Vec<u8>> = (0..total_shards)
            .map(|i| {
                if i < self.data_shards {
                    let start = i * shard_size;
                    let end = ((i + 1) * shard_size).min(data.len());
                    data[start..end].to_vec()
                } else {
                    vec![0u8; shard_size]
                }
            })
            .collect();

        // Pad shards to equal size
        for shard in &mut shards {
            if shard.len() < shard_size {
                shard.resize(shard_size, 0);
            }
        }

        // Encode parity shards
        let mut shard_refs: Vec<_> = shards.iter_mut().map(|s| &mut s[..]).collect();
        self.encoder.encode(&mut shard_refs)?;

        Ok(shards)
    }

    pub fn decode(&self, shards: &mut [Option<Vec<u8>>]) -> Result<Vec<u8>> {
        let shard_size = shards
            .iter()
            .find(|s| s.is_some())
            .unwrap()
            .as_ref()
            .unwrap()
            .len();

        // Pad missing shards
        for shard in shards.iter_mut() {
            if shard.is_none() {
                *shard = Some(vec![0u8; shard_size]);
            }
        }

        // Reconstruct
        let mut shard_refs: Vec<_> = shards
            .iter_mut()
            .map(|s| s.as_mut().unwrap().as_mut_slice())
            .collect();
        self.encoder.reconstruct(&mut shard_refs)?;

        // Extract data
        let mut result = Vec::new();
        for i in 0..self.data_shards {
            if let Some(ref shard) = shards[i] {
                result.extend_from_slice(shard);
            }
        }

        Ok(result)
    }
}

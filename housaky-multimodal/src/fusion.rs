//! Multimodal Fusion
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Modality {
    Vision,
    Language,
    Audio,
}

pub struct ModalityEmbedding {
    pub modality: Modality,
    pub data: Vec<f64>,
}

pub struct MultimodalFusion {
    dim: usize,
}

impl MultimodalFusion {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }

    pub fn fuse(&self, embeddings: Vec<ModalityEmbedding>) -> Vec<f64> {
        if embeddings.is_empty() {
            return vec![0.0; self.dim];
        }
        
        let mut result = vec![0.0; self.dim];
        for emb in embeddings {
            for (i, val) in emb.data.iter().enumerate().take(self.dim) {
                result[i] += val / embeddings.len() as f64;
            }
        }
        result
    }
}

pub struct CrossModalRetrieval {
    items: Vec<(String, Vec<f64>)>,
}

impl CrossModalRetrieval {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add(&mut self, id: String, embedding: Vec<f64>) {
        self.items.push((id, embedding));
    }

    pub fn retrieve(&self, query: &[f64]) -> Option<String> {
        self.items.iter()
            .map(|(id, emb)| {
                let sim: f64 = query.iter().zip(emb).map(|(a, b)| a * b).sum();
                (id.clone(), sim)
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(id, _)| id)
    }
}

impl Default for CrossModalRetrieval {
    fn default() -> Self {
        Self::new()
    }
}

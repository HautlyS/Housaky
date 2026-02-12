//! Cross-Attention Transformer for multimodal fusion

use std::f32::consts::PI;

pub struct CrossAttentionTransformer {
    num_heads: usize,
    hidden_dim: usize,
    head_dim: usize,
    dropout: f32,
}

impl CrossAttentionTransformer {
    pub fn new(hidden_dim: usize, num_heads: usize) -> Self {
        assert!(hidden_dim % num_heads == 0, "hidden_dim must be divisible by num_heads");
        
        Self {
            num_heads,
            hidden_dim,
            head_dim: hidden_dim / num_heads,
            dropout: 0.1,
        }
    }

    pub fn forward(&self, query: &[f32], key: &[f32], value: &[f32]) -> Vec<f32> {
        let seq_len_q = query.len() / self.hidden_dim;
        let seq_len_kv = key.len() / self.hidden_dim;
        
        // Multi-head attention
        let mut output = vec![0.0; query.len()];
        
        for head in 0..self.num_heads {
            let head_output = self.attention_head(query, key, value, head, seq_len_q, seq_len_kv);
            
            // Accumulate head outputs
            for (i, &val) in head_output.iter().enumerate() {
                output[i] += val;
            }
        }
        
        // Average across heads
        for val in output.iter_mut() {
            *val /= self.num_heads as f32;
        }
        
        output
    }

    fn attention_head(&self, query: &[f32], key: &[f32], value: &[f32], head: usize, seq_len_q: usize, seq_len_kv: usize) -> Vec<f32> {
        let head_offset = head * self.head_dim;
        let mut output = vec![0.0; seq_len_q * self.hidden_dim];
        
        // Compute attention scores
        for i in 0..seq_len_q {
            let mut attention_weights = vec![0.0; seq_len_kv];
            let mut sum_exp = 0.0;
            
            // Q * K^T / sqrt(d_k)
            for j in 0..seq_len_kv {
                let mut score = 0.0;
                for d in 0..self.head_dim {
                    let q_idx = i * self.hidden_dim + head_offset + d;
                    let k_idx = j * self.hidden_dim + head_offset + d;
                    
                    if q_idx < query.len() && k_idx < key.len() {
                        score += query[q_idx] * key[k_idx];
                    }
                }
                
                score /= (self.head_dim as f32).sqrt();
                let exp_score = score.exp();
                attention_weights[j] = exp_score;
                sum_exp += exp_score;
            }
            
            // Softmax
            for weight in attention_weights.iter_mut() {
                *weight /= sum_exp + 1e-8;
            }
            
            // Weighted sum of values
            for d in 0..self.hidden_dim {
                let mut weighted_sum = 0.0;
                for j in 0..seq_len_kv {
                    let v_idx = j * self.hidden_dim + d;
                    if v_idx < value.len() {
                        weighted_sum += attention_weights[j] * value[v_idx];
                    }
                }
                output[i * self.hidden_dim + d] = weighted_sum;
            }
        }
        
        output
    }

    pub fn cross_modal_attention(&self, modality1: &[f32], modality2: &[f32]) -> Vec<f32> {
        // modality1 as query, modality2 as key and value
        self.forward(modality1, modality2, modality2)
    }

    pub fn bidirectional_attention(&self, modality1: &[f32], modality2: &[f32]) -> (Vec<f32>, Vec<f32>) {
        let m1_to_m2 = self.cross_modal_attention(modality1, modality2);
        let m2_to_m1 = self.cross_modal_attention(modality2, modality1);
        
        (m1_to_m2, m2_to_m1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_attention() {
        let transformer = CrossAttentionTransformer::new(128, 8);
        
        let query = vec![1.0; 128 * 10]; // 10 tokens
        let key = vec![0.5; 128 * 20];   // 20 tokens
        let value = vec![0.3; 128 * 20];
        
        let output = transformer.forward(&query, &key, &value);
        
        assert_eq!(output.len(), query.len());
    }

    #[test]
    fn test_bidirectional() {
        let transformer = CrossAttentionTransformer::new(64, 4);
        
        let vision = vec![1.0; 64 * 5];
        let language = vec![0.5; 64 * 10];
        
        let (v_to_l, l_to_v) = transformer.bidirectional_attention(&vision, &language);
        
        assert_eq!(v_to_l.len(), vision.len());
        assert_eq!(l_to_v.len(), language.len());
    }
}

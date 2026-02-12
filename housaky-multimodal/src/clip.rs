//! CLIP-style contrastive learning for vision-language alignment

pub struct CLIPAlignment {
    temperature: f32,
    embedding_dim: usize,
}

impl CLIPAlignment {
    pub fn new(embedding_dim: usize) -> Self {
        Self {
            temperature: 0.07,
            embedding_dim,
        }
    }

    pub fn contrastive_loss(&self, vision_embeds: &[Vec<f32>], text_embeds: &[Vec<f32>]) -> f32 {
        assert_eq!(vision_embeds.len(), text_embeds.len(), "Batch sizes must match");
        
        let batch_size = vision_embeds.len();
        let mut total_loss = 0.0;
        
        // Compute similarity matrix
        let mut similarities = vec![vec![0.0; batch_size]; batch_size];
        
        for i in 0..batch_size {
            for j in 0..batch_size {
                similarities[i][j] = self.cosine_similarity(&vision_embeds[i], &text_embeds[j]);
            }
        }
        
        // Contrastive loss (InfoNCE)
        for i in 0..batch_size {
            // Vision to text
            let mut exp_sum = 0.0;
            for j in 0..batch_size {
                exp_sum += (similarities[i][j] / self.temperature).exp();
            }
            let positive_sim = (similarities[i][i] / self.temperature).exp();
            total_loss -= (positive_sim / exp_sum).ln();
            
            // Text to vision (symmetric)
            let mut exp_sum = 0.0;
            for j in 0..batch_size {
                exp_sum += (similarities[j][i] / self.temperature).exp();
            }
            let positive_sim = (similarities[i][i] / self.temperature).exp();
            total_loss -= (positive_sim / exp_sum).ln();
        }
        
        total_loss / (2.0 * batch_size as f32)
    }

    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len());
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        dot_product / (norm_a * norm_b + 1e-8)
    }

    pub fn align(&self, vision: &[f32], text: &[f32]) -> f32 {
        self.cosine_similarity(vision, text)
    }

    pub fn retrieve_top_k(&self, query: &[f32], candidates: &[Vec<f32>], k: usize) -> Vec<(usize, f32)> {
        let mut scores: Vec<(usize, f32)> = candidates
            .iter()
            .enumerate()
            .map(|(i, candidate)| (i, self.cosine_similarity(query, candidate)))
            .collect();
        
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores.truncate(k);
        
        scores
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let clip = CLIPAlignment::new(128);
        
        let a = vec![1.0; 128];
        let b = vec![1.0; 128];
        
        let sim = clip.cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_contrastive_loss() {
        let clip = CLIPAlignment::new(64);
        
        let vision = vec![vec![1.0; 64]; 4];
        let text = vec![vec![0.5; 64]; 4];
        
        let loss = clip.contrastive_loss(&vision, &text);
        assert!(loss > 0.0);
    }

    #[test]
    fn test_retrieval() {
        let clip = CLIPAlignment::new(32);
        
        let query = vec![1.0; 32];
        let candidates = vec![
            vec![1.0; 32],
            vec![0.5; 32],
            vec![0.1; 32],
        ];
        
        let top_k = clip.retrieve_top_k(&query, &candidates, 2);
        
        assert_eq!(top_k.len(), 2);
        assert_eq!(top_k[0].0, 0); // Most similar
    }
}

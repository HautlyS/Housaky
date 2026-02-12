//! Temporal Fusion for video and audio sequences

pub struct TemporalFusion {
    window_size: usize,
    stride: usize,
    hidden_dim: usize,
}

impl TemporalFusion {
    pub fn new(window_size: usize, stride: usize, hidden_dim: usize) -> Self {
        Self {
            window_size,
            stride,
            hidden_dim,
        }
    }

    pub fn fuse_temporal(&self, frames: &[Vec<f32>]) -> Vec<f32> {
        if frames.is_empty() {
            return vec![0.0; self.hidden_dim];
        }
        
        let num_frames = frames.len();
        let mut fused = vec![0.0; self.hidden_dim];
        
        // Sliding window temporal aggregation
        for start in (0..num_frames).step_by(self.stride) {
            let end = (start + self.window_size).min(num_frames);
            let window = &frames[start..end];
            
            // Aggregate window
            let window_agg = self.aggregate_window(window);
            
            // Add to fused representation
            for (i, &val) in window_agg.iter().enumerate() {
                if i < fused.len() {
                    fused[i] += val;
                }
            }
        }
        
        // Normalize
        let num_windows = ((num_frames - 1) / self.stride) + 1;
        for val in fused.iter_mut() {
            *val /= num_windows as f32;
        }
        
        fused
    }

    fn aggregate_window(&self, window: &[Vec<f32>]) -> Vec<f32> {
        if window.is_empty() {
            return vec![0.0; self.hidden_dim];
        }
        
        let mut aggregated = vec![0.0; self.hidden_dim];
        
        for frame in window {
            for (i, &val) in frame.iter().enumerate() {
                if i < aggregated.len() {
                    aggregated[i] += val;
                }
            }
        }
        
        // Average
        for val in aggregated.iter_mut() {
            *val /= window.len() as f32;
        }
        
        aggregated
    }

    pub fn temporal_attention(&self, frames: &[Vec<f32>]) -> Vec<f32> {
        if frames.is_empty() {
            return vec![0.0; self.hidden_dim];
        }
        
        let num_frames = frames.len();
        
        // Compute attention weights based on temporal position
        let mut weights = Vec::new();
        let mut sum_weights = 0.0;
        
        for i in 0..num_frames {
            // Exponential decay: recent frames have higher weight
            let weight = (-(num_frames - i - 1) as f32 / 10.0).exp();
            weights.push(weight);
            sum_weights += weight;
        }
        
        // Normalize weights
        for weight in weights.iter_mut() {
            *weight /= sum_weights;
        }
        
        // Weighted sum
        let mut output = vec![0.0; self.hidden_dim];
        
        for (frame, &weight) in frames.iter().zip(weights.iter()) {
            for (i, &val) in frame.iter().enumerate() {
                if i < output.len() {
                    output[i] += weight * val;
                }
            }
        }
        
        output
    }

    pub fn optical_flow_fusion(&self, frames: &[Vec<f32>]) -> Vec<f32> {
        if frames.len() < 2 {
            return self.fuse_temporal(frames);
        }
        
        let mut flow_features = vec![0.0; self.hidden_dim];
        
        // Compute frame differences (simplified optical flow)
        for i in 0..frames.len() - 1 {
            for d in 0..self.hidden_dim.min(frames[i].len()).min(frames[i + 1].len()) {
                let diff = frames[i + 1][d] - frames[i][d];
                flow_features[d] += diff.abs();
            }
        }
        
        // Normalize
        let num_diffs = frames.len() - 1;
        for val in flow_features.iter_mut() {
            *val /= num_diffs as f32;
        }
        
        flow_features
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temporal_fusion() {
        let fusion = TemporalFusion::new(3, 1, 64);
        
        let frames = vec![
            vec![1.0; 64],
            vec![2.0; 64],
            vec![3.0; 64],
            vec![4.0; 64],
        ];
        
        let fused = fusion.fuse_temporal(&frames);
        
        assert_eq!(fused.len(), 64);
        assert!(fused[0] > 0.0);
    }

    #[test]
    fn test_temporal_attention() {
        let fusion = TemporalFusion::new(3, 1, 32);
        
        let frames = vec![
            vec![1.0; 32],
            vec![2.0; 32],
            vec![3.0; 32],
        ];
        
        let output = fusion.temporal_attention(&frames);
        
        assert_eq!(output.len(), 32);
        // Recent frames should have higher weight
        assert!(output[0] > 2.0);
    }

    #[test]
    fn test_optical_flow() {
        let fusion = TemporalFusion::new(2, 1, 16);
        
        let frames = vec![
            vec![1.0; 16],
            vec![2.0; 16],
            vec![3.0; 16],
        ];
        
        let flow = fusion.optical_flow_fusion(&frames);
        
        assert_eq!(flow.len(), 16);
        assert!(flow[0] > 0.0);
    }
}

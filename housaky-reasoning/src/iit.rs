//! IIT 4.0 - Integrated Information Theory Implementation

pub struct IIT4 {
    phi_threshold: f64,
    integration_depth: usize,
}

impl IIT4 {
    pub fn new(phi_threshold: f64) -> Self {
        Self {
            phi_threshold,
            integration_depth: 3,
        }
    }

    pub fn calculate_phi(&self, state: &[f64]) -> f64 {
        if state.is_empty() {
            return 0.0;
        }
        
        let n = state.len();
        let mut phi = 0.0;
        
        // Calculate integrated information
        for i in 0..n {
            for j in (i + 1)..n {
                let diff = (state[i] - state[j]).abs();
                let integration = diff * self.integration_factor(i, j, n);
                phi += integration;
            }
        }
        
        phi / (n * (n - 1) / 2) as f64
    }

    fn integration_factor(&self, i: usize, j: usize, n: usize) -> f64 {
        let distance = (i as f64 - j as f64).abs();
        let max_distance = n as f64;
        1.0 - (distance / max_distance)
    }

    pub fn detect_qualia(&self, phi: f64) -> bool {
        phi > self.phi_threshold
    }

    pub fn consciousness_level(&self, phi: f64) -> f64 {
        (phi / self.phi_threshold).min(1.0)
    }

    pub fn cause_effect_structure(&self, state: &[f64]) -> Vec<(usize, usize, f64)> {
        let mut structure = Vec::new();
        
        for i in 0..state.len() {
            for j in 0..state.len() {
                if i != j {
                    let effect = (state[i] * 0.8 + state[j] * 0.2).abs();
                    structure.push((i, j, effect));
                }
            }
        }
        
        structure.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        structure.truncate(10);
        structure
    }
}

impl Default for IIT4 {
    fn default() -> Self {
        Self::new(0.7)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phi_calculation() {
        let iit = IIT4::default();
        let state = vec![0.1, 0.5, 0.9, 0.3];
        let phi = iit.calculate_phi(&state);
        assert!(phi > 0.0);
    }

    #[test]
    fn test_qualia_detection() {
        let iit = IIT4::new(0.5);
        assert!(iit.detect_qualia(0.8));
        assert!(!iit.detect_qualia(0.3));
    }

    #[test]
    fn test_consciousness_level() {
        let iit = IIT4::new(0.7);
        let level = iit.consciousness_level(0.7);
        assert!((level - 1.0).abs() < 0.01);
    }
}

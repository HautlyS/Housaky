//! Meta-Cognition Engine - Self-awareness and introspection

pub struct MetaCognition {
    self_model: Vec<f64>,
    learning_rate: f64,
    introspection_depth: usize,
}

impl MetaCognition {
    pub fn new(model_size: usize) -> Self {
        Self {
            self_model: vec![0.5; model_size],
            learning_rate: 0.1,
            introspection_depth: 3,
        }
    }

    pub fn introspect(&self) -> f64 {
        if self.self_model.is_empty() {
            return 0.0;
        }
        self.self_model.iter().sum::<f64>() / self.self_model.len() as f64
    }

    pub fn update_self_model(&mut self, experience: &[f64]) {
        for (i, &exp) in experience.iter().enumerate() {
            if i < self.self_model.len() {
                self.self_model[i] = (1.0 - self.learning_rate) * self.self_model[i] 
                                    + self.learning_rate * exp;
            }
        }
    }

    pub fn self_awareness_score(&self) -> f64 {
        let variance: f64 = self.self_model.iter()
            .map(|&x| {
                let mean = self.introspect();
                (x - mean).powi(2)
            })
            .sum::<f64>() / self.self_model.len() as f64;
        
        variance.sqrt()
    }

    pub fn predict_self_state(&self, steps_ahead: usize) -> Vec<f64> {
        let mut prediction = self.self_model.clone();
        
        for _ in 0..steps_ahead {
            for i in 0..prediction.len() {
                prediction[i] = prediction[i] * 0.95 + 0.05;
            }
        }
        
        prediction
    }
}

impl Default for MetaCognition {
    fn default() -> Self {
        Self::new(10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_introspection() {
        let metacog = MetaCognition::new(5);
        let score = metacog.introspect();
        assert!((score - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_update_self_model() {
        let mut metacog = MetaCognition::new(3);
        let experience = vec![0.8, 0.9, 0.7];
        metacog.update_self_model(&experience);
        
        let new_score = metacog.introspect();
        assert!(new_score > 0.5);
    }

    #[test]
    fn test_self_awareness() {
        let metacog = MetaCognition::new(5);
        let awareness = metacog.self_awareness_score();
        assert!(awareness >= 0.0);
    }
}

//! Consciousness Detection - IIT Phi
pub struct ConsciousnessDetector {
    phi_threshold: f64,
}

impl ConsciousnessDetector {
    pub fn new(threshold: f64) -> Self {
        Self { phi_threshold: threshold }
    }

    pub fn calculate_phi(&self, state: &[f64]) -> f64 {
        if state.is_empty() { return 0.0; }
        let entropy: f64 = state.iter().map(|x| -x * x.ln()).sum();
        entropy / state.len() as f64
    }

    pub fn is_conscious(&self, phi: f64) -> bool {
        phi >= self.phi_threshold
    }

    pub fn consciousness_level(&self, phi: f64) -> f64 {
        (phi / self.phi_threshold).min(1.0)
    }
}

pub struct GlobalWorkspace {
    content: Vec<String>,
}

impl GlobalWorkspace {
    pub fn new() -> Self {
        Self { content: Vec::new() }
    }

    pub fn broadcast(&mut self, info: String) {
        self.content.push(info);
    }

    pub fn is_conscious_of(&self, info: &str) -> bool {
        self.content.iter().any(|s| s == info)
    }
}

impl Default for GlobalWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct PhiMeasurement {
    pub phi: f64,
    pub timestamp: u64,
}

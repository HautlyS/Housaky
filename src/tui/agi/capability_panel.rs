use crate::meta_cognition::CapabilityAssessment;

pub struct CapabilityPanel {
    capabilities: Option<CapabilityAssessment>,
}

impl CapabilityPanel {
    pub fn new() -> Self {
        Self { capabilities: None }
    }

    pub fn set_capabilities(&mut self, capabilities: CapabilityAssessment) {
        self.capabilities = Some(capabilities);
    }
}

impl Default for CapabilityPanel {
    fn default() -> Self {
        Self::new()
    }
}

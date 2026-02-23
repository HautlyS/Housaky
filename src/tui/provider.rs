pub struct ProviderState {
    pub providers: Vec<String>,
    pub selected: usize,
    pub testing: bool,
    pub result: Option<String>,
}

impl ProviderState {
    pub fn new() -> Self {
        Self {
            providers: Self::default_providers(),
            selected: 0,
            testing: false,
            result: None,
        }
    }

    fn default_providers() -> Vec<String> {
        vec![
            "openrouter".into(),
            "anthropic".into(),
            "openai".into(),
            "gemini".into(),
            "ollama".into(),
            "groq".into(),
            "mistral".into(),
            "deepseek".into(),
            "xai".into(),
            "perplexity".into(),
            "cohere".into(),
        ]
    }

    pub fn select_next(&mut self) {
        if self.selected < self.providers.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    pub fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn get_selected(&self) -> Option<&String> {
        self.providers.get(self.selected)
    }

    pub fn set_testing(&mut self, testing: bool) {
        self.testing = testing;
    }

    pub fn set_result(&mut self, result: Option<String>) {
        self.result = result;
    }

    pub fn add_custom_provider(&mut self, name: String) {
        if !self.providers.contains(&name) {
            self.providers.push(name);
        }
    }

    pub fn refresh(&mut self) {
        self.providers = Self::default_providers();
        self.selected = 0;
        self.result = None;
    }
}

impl Default for ProviderState {
    fn default() -> Self {
        Self::new()
    }
}

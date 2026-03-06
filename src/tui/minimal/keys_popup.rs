//! Keys Manager Popup (Ctrl+K)
//!
//! Quick access popup for switching providers, models, and managing API keys.

use ratatui::{
    layout::{Alignment, Margin, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::theme;

/// Provider entry
#[derive(Debug, Clone)]
pub struct ProviderEntry {
    pub name: String,
    pub key_suffix: String, // Last 4 chars of key
    pub enabled: bool,
    pub models: Vec<String>,
}

impl ProviderEntry {
    pub fn new(name: impl Into<String>, key_suffix: impl Into<String>, enabled: bool) -> Self {
        Self {
            name: name.into(),
            key_suffix: key_suffix.into(),
            enabled,
            models: Vec::new(),
        }
    }

    pub fn with_models(mut self, models: Vec<String>) -> Self {
        self.models = models;
        self
    }
}

/// Popup mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeysPopupMode {
    Providers,
    Models,
}

/// Keys popup state
pub struct KeysPopup {
    pub visible: bool,
    pub mode: KeysPopupMode,
    pub providers: Vec<ProviderEntry>,
    pub provider_index: usize,
    pub model_index: usize,
    pub provider_state: ListState,
    pub model_state: ListState,
    pub current_provider: String,
    pub current_model: String,
}

impl KeysPopup {
    pub fn new() -> Self {
        let mut provider_state = ListState::default();
        provider_state.select(Some(0));
        let mut model_state = ListState::default();
        model_state.select(Some(0));

        Self {
            visible: false,
            mode: KeysPopupMode::Providers,
            providers: Vec::new(),
            provider_index: 0,
            model_index: 0,
            provider_state,
            model_state,
            current_provider: "openrouter".to_string(),
            current_model: "auto".to_string(),
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.mode = KeysPopupMode::Providers;
        self.provider_state.select(Some(self.provider_index));
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }
    }

    pub fn set_providers(&mut self, providers: Vec<ProviderEntry>) {
        self.providers = providers;
        // Find current provider index
        if let Some(idx) = self.providers.iter().position(|p| p.name == self.current_provider) {
            self.provider_index = idx;
        }
        self.provider_state.select(Some(self.provider_index));
    }

    pub fn set_current(&mut self, provider: &str, model: &str) {
        self.current_provider = provider.to_string();
        self.current_model = model.to_string();
        if let Some(idx) = self.providers.iter().position(|p| p.name == self.current_provider) {
            self.provider_index = idx;
            self.provider_state.select(Some(idx));
        }
    }

    pub fn next(&mut self) {
        match self.mode {
            KeysPopupMode::Providers => {
                if self.providers.is_empty() {
                    return;
                }
                self.provider_index = (self.provider_index + 1) % self.providers.len();
                self.provider_state.select(Some(self.provider_index));
            }
            KeysPopupMode::Models => {
                if let Some(provider) = self.providers.get(self.provider_index) {
                    if provider.models.is_empty() {
                        return;
                    }
                    self.model_index = (self.model_index + 1) % provider.models.len();
                    self.model_state.select(Some(self.model_index));
                }
            }
        }
    }

    pub fn previous(&mut self) {
        match self.mode {
            KeysPopupMode::Providers => {
                if self.providers.is_empty() {
                    return;
                }
                self.provider_index = if self.provider_index == 0 {
                    self.providers.len() - 1
                } else {
                    self.provider_index - 1
                };
                self.provider_state.select(Some(self.provider_index));
            }
            KeysPopupMode::Models => {
                if let Some(provider) = self.providers.get(self.provider_index) {
                    if provider.models.is_empty() {
                        return;
                    }
                    self.model_index = if self.model_index == 0 {
                        provider.models.len() - 1
                    } else {
                        self.model_index - 1
                    };
                    self.model_state.select(Some(self.model_index));
                }
            }
        }
    }

    /// Enter: select current, switch to models or confirm
    pub fn enter(&mut self) -> Option<(String, String)> {
        match self.mode {
            KeysPopupMode::Providers => {
                if let Some(provider) = self.providers.get(self.provider_index) {
                    if !provider.models.is_empty() {
                        self.mode = KeysPopupMode::Models;
                        self.model_index = 0;
                        self.model_state.select(Some(0));
                        return None;
                    }
                    // No models, just select provider
                    let name = provider.name.clone();
                    self.current_provider = name.clone();
                    self.hide();
                    return Some((name, self.current_model.clone()));
                }
            }
            KeysPopupMode::Models => {
                if let Some(provider) = self.providers.get(self.provider_index) {
                    if let Some(model) = provider.models.get(self.model_index) {
                        let prov_name = provider.name.clone();
                        let model_name = model.clone();
                        self.current_provider = prov_name.clone();
                        self.current_model = model_name.clone();
                        self.hide();
                        return Some((prov_name, model_name));
                    }
                }
            }
        }
        None
    }

    pub fn back(&mut self) {
        match self.mode {
            KeysPopupMode::Providers => {
                self.hide();
            }
            KeysPopupMode::Models => {
                self.mode = KeysPopupMode::Providers;
            }
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Center popup
        let popup_width = 50.min(area.width.saturating_sub(4));
        let popup_height = 15.min(area.height.saturating_sub(4));

        let popup_area = Rect {
            x: (area.width - popup_width) / 2,
            y: (area.height - popup_height) / 2,
            width: popup_width,
            height: popup_height,
        };

        // Clear background
        frame.render_widget(Clear, popup_area);

        let title = match self.mode {
            KeysPopupMode::Providers => " PROVIDERS [Ctrl+K] ",
            KeysPopupMode::Models => " MODELS ",
        };

        let block = Block::default()
            .title(Span::styled(title, theme::style_title()))
            .borders(Borders::ALL)
            .border_style(theme::style_border_active())
            .style(theme::style_popup_bg());

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        match self.mode {
            KeysPopupMode::Providers => self.draw_providers(frame, inner),
            KeysPopupMode::Models => self.draw_models(frame, inner),
        }

        // Footer with instructions
        let footer_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + popup_area.height - 2,
            width: popup_area.width - 2,
            height: 1,
        };

        let footer = Paragraph::new(Line::from(vec![
            Span::styled("j/k", theme::style_hotkey()),
            Span::raw(" nav  "),
            Span::styled("Enter", theme::style_hotkey()),
            Span::raw(" select  "),
            Span::styled("Esc", theme::style_hotkey()),
            Span::raw(" close"),
        ]))
        .alignment(Alignment::Center)
        .style(theme::style_dim());

        frame.render_widget(footer, footer_area);
    }

    fn draw_providers(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .providers
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let is_current = p.name == self.current_provider;
                let is_selected = i == self.provider_index;

                let status = if p.enabled { "[+]" } else { "[-]" };
                let marker = if is_current { "*" } else { " " };

                let style = if is_selected {
                    theme::style_selected()
                } else if !p.enabled {
                    theme::style_dim()
                } else {
                    theme::style_base()
                };

                let content = Line::from(vec![
                    Span::styled(marker, if is_current { theme::style_success() } else { style }),
                    Span::styled(status, if p.enabled { theme::style_success() } else { theme::style_dim() }),
                    Span::raw(" "),
                    Span::styled(&p.name, style),
                    Span::styled(format!(" ...{}", p.key_suffix), theme::style_dim()),
                ]);

                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .highlight_style(theme::style_selected())
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, area.inner(Margin { horizontal: 1, vertical: 0 }), &mut self.provider_state);
    }

    fn draw_models(&mut self, frame: &mut Frame, area: Rect) {
        let models = self
            .providers
            .get(self.provider_index)
            .map(|p| &p.models)
            .cloned()
            .unwrap_or_default();

        if models.is_empty() {
            let msg = Paragraph::new("No models available")
                .style(theme::style_dim())
                .alignment(Alignment::Center);
            frame.render_widget(msg, area);
            return;
        }

        let items: Vec<ListItem> = models
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let is_current = *m == self.current_model;
                let is_selected = i == self.model_index;

                let marker = if is_current { "*" } else { " " };

                let style = if is_selected {
                    theme::style_selected()
                } else {
                    theme::style_base()
                };

                let content = Line::from(vec![
                    Span::styled(marker, if is_current { theme::style_success() } else { style }),
                    Span::raw(" "),
                    Span::styled(m.as_str(), style),
                ]);

                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .highlight_style(theme::style_selected())
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, area.inner(Margin { horizontal: 1, vertical: 0 }), &mut self.model_state);
    }
}

impl Default for KeysPopup {
    fn default() -> Self {
        Self::new()
    }
}

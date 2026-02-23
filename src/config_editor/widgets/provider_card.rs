use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct ProviderCard<'a> {
    pub name: &'a str,
    pub api_key_masked: &'a str,
    pub status: &'a str,
    pub is_active: bool,
}

impl<'a> ProviderCard<'a> {
    pub fn new(name: &'a str, api_key_masked: &'a str, status: &'a str) -> Self {
        Self {
            name,
            api_key_masked,
            status,
            is_active: false,
        }
    }

    pub fn active(mut self, is_active: bool) -> Self {
        self.is_active = is_active;
        self
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let border_color = if self.is_active {
            Color::Green
        } else {
            Color::Gray
        };

        let content = format!(
            "Provider: {}\nAPI Key:  {}\nStatus:   {}",
            self.name, self.api_key_masked, self.status
        );

        let paragraph = Paragraph::new(content).block(
            Block::default()
                .title(if self.is_active {
                    " ● Active "
                } else {
                    " Provider "
                })
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        );

        f.render_widget(paragraph, area);
    }
}

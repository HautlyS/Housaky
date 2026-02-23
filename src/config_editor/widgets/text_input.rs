use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct TextInput<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub focused: bool,
}

impl<'a> TextInput<'a> {
    pub fn new(label: &'a str, value: &'a str) -> Self {
        Self {
            label,
            value,
            focused: false,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let style = if self.focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let paragraph = Paragraph::new(self.value).block(
            Block::default()
                .title(self.label)
                .borders(Borders::ALL)
                .style(style),
        );

        f.render_widget(paragraph, area);
    }
}

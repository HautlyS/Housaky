use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct Toggle<'a> {
    pub label: &'a str,
    pub value: bool,
    pub focused: bool,
}

impl<'a> Toggle<'a> {
    pub fn new(label: &'a str, value: bool) -> Self {
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

        let text = if self.value { "[✓] Yes" } else { "[ ] No" };

        let paragraph = Paragraph::new(text).block(
            Block::default()
                .title(self.label)
                .borders(Borders::ALL)
                .style(style),
        );

        f.render_widget(paragraph, area);
    }
}

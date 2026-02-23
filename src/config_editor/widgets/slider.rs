use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct Slider<'a> {
    pub label: &'a str,
    pub value: u64,
    pub min: u64,
    pub max: u64,
    pub focused: bool,
}

impl<'a> Slider<'a> {
    pub fn new(label: &'a str, value: u64, min: u64, max: u64) -> Self {
        Self {
            label,
            value,
            min,
            max,
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

        let range = self.max.saturating_sub(self.min);
        let percent = if range > 0 {
            ((self.value.saturating_sub(self.min)) as f64 / range as f64) * 100.0
        } else {
            0.0
        };

        let bar_width = area.width.saturating_sub(2) as usize;
        let filled = (percent / 100.0 * bar_width as f64) as usize;
        let empty = bar_width.saturating_sub(filled);

        let bar = format!(
            "[{}{}] {}",
            "█".repeat(filled),
            "░".repeat(empty),
            self.value
        );

        let paragraph = Paragraph::new(bar).block(
            Block::default()
                .title(self.label)
                .borders(Borders::ALL)
                .style(style),
        );

        f.render_widget(paragraph, area);
    }
}

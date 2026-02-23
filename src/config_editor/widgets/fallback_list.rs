use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub struct FallbackList<'a> {
    pub items: &'a [FallbackItem],
    pub active_index: usize,
}

#[derive(Debug, Clone)]
pub struct FallbackItem {
    pub name: String,
    pub usage_percent: u8,
    pub is_active: bool,
}

impl<'a> FallbackList<'a> {
    pub fn new(items: &'a [FallbackItem]) -> Self {
        Self {
            items,
            active_index: 0,
        }
    }

    pub fn active(mut self, index: usize) -> Self {
        self.active_index = index;
        self
    }

    pub fn render(&self, f: &mut Frame, area: Rect, state: &mut ListState) {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| {
                let marker = if item.is_active { "●" } else { "○" };
                let style = if item.is_active {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let bar = self.usage_bar(item.usage_percent);
                let text = format!("{} {} {}", marker, item.name, bar);

                ListItem::new(text).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Fallback Providers ")
                    .borders(Borders::ALL),
            )
            .highlight_style(Style::default().bg(Color::DarkGray));

        state.select(Some(self.active_index));
        f.render_stateful_widget(list, area, state);
    }

    fn usage_bar(&self, percent: u8) -> String {
        let filled = (percent as usize / 10).min(10);
        let empty = 10 - filled;
        let color_marker = if percent >= 80 { "!" } else { " " };
        format!(
            "[{}{}] {}%{}",
            "█".repeat(filled),
            "░".repeat(empty),
            percent,
            color_marker
        )
    }
}

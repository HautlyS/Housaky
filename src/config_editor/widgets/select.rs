use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub struct Select<'a> {
    pub label: &'a str,
    pub options: &'a [String],
    pub selected: usize,
    pub focused: bool,
}

impl<'a> Select<'a> {
    pub fn new(label: &'a str, options: &'a [String]) -> Self {
        Self {
            label,
            options,
            selected: 0,
            focused: false,
        }
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected.min(self.options.len().saturating_sub(1));
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn render(&self, f: &mut Frame, area: Rect, state: &mut ListState) {
        let style = if self.focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let items: Vec<ListItem> = self
            .options
            .iter()
            .map(|opt| ListItem::new(opt.as_str()))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(self.label)
                    .borders(Borders::ALL)
                    .style(style),
            )
            .highlight_style(Style::default().bg(Color::DarkGray));

        state.select(Some(self.selected));
        f.render_stateful_widget(list, area, state);
    }
}

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

#[derive(Debug, Clone)]
pub struct SearchState {
    pub query: String,
    pub active: bool,
    pub results: Vec<usize>,
    pub current_index: usize,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            active: false,
            results: Vec::new(),
            current_index: 0,
        }
    }

    pub fn activate(&mut self) {
        self.active = true;
        self.query.clear();
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.query.clear();
        self.results.clear();
        self.current_index = 0;
    }

    pub fn toggle(&mut self) {
        if self.active {
            self.deactivate();
        } else {
            self.activate();
        }
    }

    pub fn push_char(&mut self, c: char) {
        self.query.push(c);
    }

    pub fn pop_char(&mut self) {
        self.query.pop();
    }

    pub fn clear(&mut self) {
        self.query.clear();
        self.results.clear();
        self.current_index = 0;
    }

    pub fn next_result(&mut self) -> Option<usize> {
        if self.results.is_empty() {
            return None;
        }
        self.current_index = (self.current_index + 1) % self.results.len();
        self.results.get(self.current_index).copied()
    }

    pub fn previous_result(&mut self) -> Option<usize> {
        if self.results.is_empty() {
            return None;
        }
        if self.current_index == 0 {
            self.current_index = self.results.len() - 1;
        } else {
            self.current_index -= 1;
        }
        self.results.get(self.current_index).copied()
    }

    pub fn update_results(&mut self, message_ids: Vec<usize>) {
        self.results = message_ids;
        self.current_index = 0;
    }

    pub fn get_current_result(&self) -> Option<usize> {
        self.results.get(self.current_index).copied()
    }

    pub fn has_results(&self) -> bool {
        !self.results.is_empty()
    }

    pub fn result_count(&self) -> usize {
        self.results.len()
    }

    pub fn is_empty(&self) -> bool {
        self.query.is_empty()
    }

    pub fn draw(&self, f: &mut Frame, area: Rect) {
        if !self.active {
            return;
        }

        let search_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3)])
            .split(area);

        let result_text = if self.query.is_empty() {
            "Type to search...".to_string()
        } else if self.results.is_empty() {
            format!("No results for '{}'", self.query)
        } else {
            format!(
                "Result {} of {} for '{}'",
                self.current_index + 1,
                self.results.len(),
                self.query
            )
        };

        let result_style = if self.results.is_empty() && !self.query.is_empty() {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::Green)
        };

        let search_widget = Paragraph::new(self.query.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Search ({})", result_text))
                    .title_style(result_style),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(Clear, search_layout[0]);
        f.render_widget(search_widget, search_layout[0]);
    }

    pub fn draw_indicator(&self, f: &mut Frame, area: Rect) {
        if !self.active && !self.query.is_empty() {
            let indicator = Paragraph::new(format!(
                "🔍 {} ({} results)",
                self.query,
                self.results.len()
            ))
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(ratatui::layout::Alignment::Right);
            f.render_widget(indicator, area);
        }
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}

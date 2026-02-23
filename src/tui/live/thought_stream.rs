use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct ThoughtStreamPanel {
    thoughts: Vec<ThoughtEntry>,
    max_thoughts: usize,
    scroll_offset: usize,
}

#[derive(Clone, Debug)]
pub struct ThoughtEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub content: String,
    pub thought_type: ThoughtType,
    pub confidence: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThoughtType {
    Perception,
    Reasoning,
    Decision,
    Action,
    Reflection,
    Learning,
    Meta,
}

impl ThoughtStreamPanel {
    pub fn new() -> Self {
        Self {
            thoughts: Vec::new(),
            max_thoughts: 1000,
            scroll_offset: 0,
        }
    }

    pub fn add_thought(&mut self, content: &str, thought_type: ThoughtType, confidence: f64) {
        let entry = ThoughtEntry {
            timestamp: chrono::Utc::now(),
            content: content.to_string(),
            thought_type,
            confidence,
        };

        self.thoughts.push(entry);

        if self.thoughts.len() > self.max_thoughts {
            self.thoughts.remove(0);
        }
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset += 1;
    }

    pub fn scroll_to_bottom(&mut self) {
        if !self.thoughts.is_empty() {
            self.scroll_offset = self.thoughts.len().saturating_sub(1);
        }
    }

    pub fn clear(&mut self) {
        self.thoughts.clear();
        self.scroll_offset = 0;
    }

    pub fn get_recent(&self, count: usize) -> &[ThoughtEntry] {
        let start = self.thoughts.len().saturating_sub(count);
        &self.thoughts[start..]
    }

    pub fn draw(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            "💭 Live Thought Stream",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        let visible_count = area.height.saturating_sub(4) as usize;
        let start_idx = self.scroll_offset;
        let _end_idx = (start_idx + visible_count).min(self.thoughts.len());

        for entry in self
            .thoughts
            .iter()
            .rev()
            .skip(start_idx)
            .take(visible_count)
        {
            let type_color = match entry.thought_type {
                ThoughtType::Perception => Color::Cyan,
                ThoughtType::Reasoning => Color::Yellow,
                ThoughtType::Decision => Color::Green,
                ThoughtType::Action => Color::Blue,
                ThoughtType::Reflection => Color::Magenta,
                ThoughtType::Learning => Color::LightMagenta,
                ThoughtType::Meta => Color::DarkGray,
            };

            let type_icon = match entry.thought_type {
                ThoughtType::Perception => "👁",
                ThoughtType::Reasoning => "🧠",
                ThoughtType::Decision => "⚡",
                ThoughtType::Action => "🎬",
                ThoughtType::Reflection => "🪞",
                ThoughtType::Learning => "📚",
                ThoughtType::Meta => "🔬",
            };

            let time_str = entry.timestamp.format("%H:%M:%S").to_string();
            let confidence_str = format!("{:.0}%", entry.confidence * 100.0);

            lines.push(Line::from(vec![
                Span::styled(
                    format!("{} ", time_str),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(type_icon, Style::default().fg(type_color)),
                Span::styled(
                    format!(" [{:.3}]", confidence_str),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(" "),
                Span::styled(
                    entry
                        .content
                        .chars()
                        .take(area.width as usize - 20)
                        .collect::<String>(),
                    Style::default().fg(Color::White),
                ),
            ]));
        }

        if self.thoughts.is_empty() {
            lines.push(Line::from(
                "No thoughts yet. Start a conversation to see the thought stream.",
            ));
        }

        let footer = Line::from(vec![
            Span::styled(
                format!(" {} thoughts | ", self.thoughts.len()),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled("↑↓ Scroll", Style::default().fg(Color::Gray)),
        ]);
        lines.push(footer);

        let panel =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Thoughts"));
        f.render_widget(panel, area);
    }
}

impl Default for ThoughtStreamPanel {
    fn default() -> Self {
        Self::new()
    }
}

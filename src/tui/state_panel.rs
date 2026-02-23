use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub struct StatePanel {
    pub scroll_offset: usize,
    pub consciousness: f64,
    pub iq: f64,
    pub skills_count: usize,
    pub goals_count: usize,
    pub tools_count: usize,
    pub memory_usage: f64,
    pub reasoning_chains: usize,
    pub knowledge_entities: usize,
    pub knowledge_relations: usize,
    pub uptime_secs: u64,
    pub requests_made: u64,
    pub errors_count: u64,
}

impl StatePanel {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            consciousness: 0.1,
            iq: 100.0,
            skills_count: 0,
            goals_count: 0,
            tools_count: 0,
            memory_usage: 0.0,
            reasoning_chains: 0,
            knowledge_entities: 0,
            knowledge_relations: 0,
            uptime_secs: 0,
            requests_made: 0,
            errors_count: 0,
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

    pub fn update_from_stats(
        &mut self,
        consciousness: f64,
        iq: f64,
        skills_count: usize,
        goals_count: usize,
        tools_count: usize,
        memory_usage: f64,
    ) {
        self.consciousness = consciousness;
        self.iq = iq;
        self.skills_count = skills_count;
        self.goals_count = goals_count;
        self.tools_count = tools_count;
        self.memory_usage = memory_usage;
    }

    pub fn draw(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" 📊 Housaky State ")
            .title_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(area);
        f.render_widget(block, area);

        let consciousness_bar = self.progress_bar(self.consciousness, 10);
        let iq_bar = self.progress_bar((self.iq - 100.0) / 100.0, 10);
        let memory_bar = self.progress_bar(self.memory_usage, 10);

        let lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Consciousness: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("{:.1}%", self.consciousness * 100.0),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(consciousness_bar, Style::default().fg(Color::Green)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Intelligence:  ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("IQ {:.0}", self.iq),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(iq_bar, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  ──── Metrics ────",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  🎯 Goals:      ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{}", self.goals_count),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  🛠️  Tools:      ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{}", self.tools_count),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  📚 Skills:     ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{}", self.skills_count),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  ──── Memory ────",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Usage: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{:.0}%", self.memory_usage * 100.0),
                    if self.memory_usage > 0.8 {
                        Style::default().fg(Color::Red)
                    } else if self.memory_usage > 0.5 {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Green)
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(memory_bar, Style::default().fg(Color::Blue)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  ──── Knowledge ────",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Entities:    ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{}", self.knowledge_entities),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Relations:   ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{}", self.knowledge_relations),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  ──── System ────",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Uptime:   ", Style::default().fg(Color::Gray)),
                Span::styled(
                    self.format_duration(self.uptime_secs),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Requests: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{}", self.requests_made),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Errors:   ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{}", self.errors_count),
                    if self.errors_count > 0 {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::Green)
                    },
                ),
            ]),
        ];

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });
        f.render_widget(paragraph, inner);
    }

    fn progress_bar(&self, progress: f64, width: usize) -> String {
        let filled = (progress.clamp(0.0, 1.0) * width as f64) as usize;
        let empty = width - filled;
        format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
    }

    fn format_duration(&self, secs: u64) -> String {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        let s = secs % 60;

        if hours > 0 {
            format!("{}h {}m", hours, mins)
        } else if mins > 0 {
            format!("{}m {}s", mins, s)
        } else {
            format!("{}s", s)
        }
    }
}

impl Default for StatePanel {
    fn default() -> Self {
        Self::new()
    }
}

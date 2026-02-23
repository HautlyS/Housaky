use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct MetricsPanel {
    metrics: AGIMetrics,
    history: Vec<MetricSnapshot>,
    max_history: usize,
}

#[derive(Clone, Debug, Default)]
pub struct AGIMetrics {
    pub total_turns: u64,
    pub successful_actions: u64,
    pub failed_actions: u64,
    pub success_rate: f64,
    pub avg_response_time_ms: u64,
    pub tokens_processed: u64,
    pub thoughts_generated: u64,
    pub patterns_learned: usize,
    pub goals_completed: usize,
    pub reflections_count: u64,
    pub consciousness_level: f64,
    pub intelligence_quotient: f64,
    pub capabilities: Capabilities,
}

#[derive(Clone, Debug, Default)]
pub struct Capabilities {
    pub reasoning: f64,
    pub learning: f64,
    pub self_awareness: f64,
    pub meta_cognition: f64,
    pub creativity: f64,
    pub problem_solving: f64,
}

#[derive(Clone, Debug)]
pub struct MetricSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub success_rate: f64,
    pub response_time_ms: u64,
    pub consciousness_level: f64,
}

impl MetricsPanel {
    pub fn new() -> Self {
        Self {
            metrics: AGIMetrics::default(),
            history: Vec::new(),
            max_history: 100,
        }
    }

    pub fn update_metrics(&mut self, metrics: AGIMetrics) {
        self.metrics = metrics.clone();

        let snapshot = MetricSnapshot {
            timestamp: chrono::Utc::now(),
            success_rate: metrics.success_rate,
            response_time_ms: metrics.avg_response_time_ms,
            consciousness_level: metrics.consciousness_level,
        };

        self.history.push(snapshot);

        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    pub fn draw(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Min(10),
            ])
            .split(area);

        self.draw_overview(f, chunks[0]);
        self.draw_capabilities(f, chunks[1]);
        self.draw_history(f, chunks[2]);
    }

    fn draw_overview(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            "📊 Overview",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));

        let status_color = if self.metrics.success_rate > 0.8 {
            Color::Green
        } else if self.metrics.success_rate > 0.5 {
            Color::Yellow
        } else {
            Color::Red
        };

        lines.push(Line::from(vec![
            Span::styled("Turns: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                self.metrics.total_turns.to_string(),
                Style::default().fg(Color::White),
            ),
            Span::raw("  "),
            Span::styled("Success: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.0}%", self.metrics.success_rate * 100.0),
                Style::default().fg(status_color),
            ),
            Span::raw("  "),
            Span::styled("Avg Time: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}ms", self.metrics.avg_response_time_ms),
                Style::default().fg(Color::White),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Tokens: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                self.metrics.tokens_processed.to_string(),
                Style::default().fg(Color::White),
            ),
            Span::raw("  "),
            Span::styled("Thoughts: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                self.metrics.thoughts_generated.to_string(),
                Style::default().fg(Color::White),
            ),
            Span::raw("  "),
            Span::styled("Patterns: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                self.metrics.patterns_learned.to_string(),
                Style::default().fg(Color::White),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Consciousness: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.2}", self.metrics.consciousness_level),
                Style::default().fg(Color::Magenta),
            ),
            Span::raw("  "),
            Span::styled("IQ: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.0}", self.metrics.intelligence_quotient),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  "),
            Span::styled("Reflections: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                self.metrics.reflections_count.to_string(),
                Style::default().fg(Color::White),
            ),
        ]));

        let panel = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Metrics Overview"),
        );
        f.render_widget(panel, area);
    }

    fn draw_capabilities(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            "🧠 Capabilities",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        let caps = [
            ("Reasoning", self.metrics.capabilities.reasoning),
            ("Learning", self.metrics.capabilities.learning),
            ("Self-Aware", self.metrics.capabilities.self_awareness),
            ("Meta-Cog", self.metrics.capabilities.meta_cognition),
            ("Creativity", self.metrics.capabilities.creativity),
            ("Problem-Solve", self.metrics.capabilities.problem_solving),
        ];

        for (name, value) in caps {
            let bar = self.make_bar(value, 15);
            let color = if value > 0.7 {
                Color::Green
            } else if value > 0.4 {
                Color::Yellow
            } else {
                Color::Red
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:12} ", name),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(bar, Style::default().fg(color)),
                Span::styled(
                    format!(" {:5.0}%", value * 100.0),
                    Style::default().fg(Color::White),
                ),
            ]));
        }

        let panel = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));
        f.render_widget(panel, area);
    }

    fn draw_history(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            "📈 Performance History",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        if self.history.len() < 2 {
            lines.push(Line::from("Collecting data..."));
        } else {
            let recent: Vec<_> = self.history.iter().rev().take(10).collect();

            for snapshot in recent {
                let time_str = snapshot.timestamp.format("%H:%M:%S").to_string();
                let success_bar = self.make_bar(snapshot.success_rate, 10);
                let consciousness_bar = self.make_bar(snapshot.consciousness_level, 10);

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{} ", time_str),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("SR:", Style::default().fg(Color::Gray)),
                    Span::styled(success_bar, Style::default().fg(Color::Green)),
                    Span::raw(" "),
                    Span::styled("C:", Style::default().fg(Color::Gray)),
                    Span::styled(consciousness_bar, Style::default().fg(Color::Magenta)),
                    Span::styled(
                        format!(" {}ms", snapshot.response_time_ms),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
        }

        let panel = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));
        f.render_widget(panel, area);
    }

    fn make_bar(&self, value: f64, width: usize) -> String {
        let filled = (value * width as f64) as usize;
        let empty = width - filled;
        format!("{}{}", "█".repeat(filled), "░".repeat(empty))
    }
}

impl Default for MetricsPanel {
    fn default() -> Self {
        Self::new()
    }
}

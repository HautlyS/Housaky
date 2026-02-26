use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use crate::tui::enhanced_app::theme::{
    Palette, render_gauge_bar, style_border, style_dim, style_error, style_muted,
    style_success, style_tag_goal, style_tag_skill, style_tag_thought, style_tag_tool,
    style_title, style_warning,
};
use crate::tui::enhanced_app::state::SessionMetrics;

// ── Goal entry ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SidebarGoal {
    pub title:    String,
    pub progress: f64,
    pub priority: GoalPriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalPriority {
    Critical,
    High,
    Medium,
    Low,
}

impl GoalPriority {
    pub fn color(&self) -> ratatui::style::Color {
        match self {
            GoalPriority::Critical => Palette::ERROR,
            GoalPriority::High     => Palette::WARNING,
            GoalPriority::Medium   => Palette::ASSISTANT,
            GoalPriority::Low      => Palette::TEXT_DIM,
        }
    }
    pub fn icon(&self) -> &'static str {
        match self {
            GoalPriority::Critical => "◈",
            GoalPriority::High     => "◆",
            GoalPriority::Medium   => "◇",
            GoalPriority::Low      => "○",
        }
    }
}

// ── Activity entry ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ActivityEntry {
    pub kind:    ActivityKind,
    pub message: String,
    pub time:    String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityKind {
    Tool,
    Skill,
    Thought,
    Goal,
    System,
}

impl ActivityKind {
    pub fn icon(&self) -> &'static str {
        match self {
            ActivityKind::Tool    => "⚙",
            ActivityKind::Skill   => "◈",
            ActivityKind::Thought => "💭",
            ActivityKind::Goal    => "◆",
            ActivityKind::System  => "●",
        }
    }
}

// ── Sidebar state ─────────────────────────────────────────────────────────────

pub struct Sidebar {
    pub goals:      Vec<SidebarGoal>,
    pub activity:   Vec<ActivityEntry>,
    pub thoughts:   Vec<String>,
    pub scroll:     usize,
    pub goal_scroll:usize,
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            goals:       Vec::new(),
            activity:    Vec::new(),
            thoughts:    Vec::new(),
            scroll:      0,
            goal_scroll: 0,
        }
    }

    pub fn push_activity(&mut self, kind: ActivityKind, message: impl Into<String>) {
        use chrono::Local;
        self.activity.push(ActivityEntry {
            kind,
            message: message.into(),
            time: Local::now().format("%H:%M:%S").to_string(),
        });
        if self.activity.len() > 200 {
            self.activity.remove(0);
        }
    }

    pub fn push_thought(&mut self, thought: impl Into<String>) {
        self.thoughts.push(thought.into());
        if self.thoughts.len() > 100 {
            self.thoughts.remove(0);
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll += 1;
    }

    pub fn draw(&self, f: &mut Frame, area: Rect, metrics: &SessionMetrics) {
        let zones = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10), // metrics
                Constraint::Min(5),     // goals
                Constraint::Length(7),  // live activity
            ])
            .split(area);

        self.draw_metrics(f, zones[0], metrics);
        self.draw_goals(f, zones[1]);
        self.draw_activity(f, zones[2]);
    }

    // ── Metrics block ─────────────────────────────────────────────────────────

    fn draw_metrics(&self, f: &mut Frame, area: Rect, m: &SessionMetrics) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border())
            .title(Span::styled(" 📊 Session ", style_title()));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let error_style = if m.total_errors > 0 { style_error() } else { style_success() };

        let lines = vec![
            Line::from(vec![
                Span::styled("  Uptime     ", style_muted()),
                Span::styled(m.format_uptime(), style_success()),
            ]),
            Line::from(vec![
                Span::styled("  Messages   ", style_muted()),
                Span::styled(format!("{}", m.total_messages), ratatui::style::Style::default().fg(Palette::TEXT)),
            ]),
            Line::from(vec![
                Span::styled("  Tokens In  ", style_muted()),
                Span::styled(format!("{}", m.total_tokens_in), style_dim()),
            ]),
            Line::from(vec![
                Span::styled("  Tokens Out ", style_muted()),
                Span::styled(format!("{}", m.total_tokens_out), style_dim()),
            ]),
            Line::from(vec![
                Span::styled("  Requests   ", style_muted()),
                Span::styled(format!("{}", m.total_requests), ratatui::style::Style::default().fg(Palette::TEXT)),
            ]),
            Line::from(vec![
                Span::styled("  Errors     ", style_muted()),
                Span::styled(format!("{}", m.total_errors), error_style),
            ]),
            Line::from(vec![
                Span::styled("  t/s avg    ", style_muted()),
                Span::styled(
                    format!("{:.1}", m.avg_tokens_per_sec),
                    ratatui::style::Style::default().fg(Palette::CYAN),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Skills     ", style_muted()),
                Span::styled(format!("{}", m.skills_enabled), style_tag_skill()),
            ]),
        ];

        f.render_widget(Paragraph::new(lines), inner);
    }

    // ── Goals block ───────────────────────────────────────────────────────────

    fn draw_goals(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border())
            .title(Span::styled(
                format!(" 🎯 Goals ({}) ", self.goals.len()),
                style_tag_goal(),
            ));

        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.goals.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled(
                    "  No active goals",
                    style_muted(),
                )),
                inner,
            );
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        for goal in self.goals.iter().skip(self.goal_scroll) {
            let bar = render_gauge_bar(goal.progress, 10);
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", goal.priority.icon()),
                    ratatui::style::Style::default().fg(goal.priority.color()),
                ),
                Span::styled(
                    truncate(&goal.title, (inner.width as usize).saturating_sub(4)),
                    ratatui::style::Style::default()
                        .fg(Palette::TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("   ", style_muted()),
                Span::styled(bar, ratatui::style::Style::default().fg(Palette::CYAN_DIM)),
                Span::styled(
                    format!(" {:.0}%", goal.progress * 100.0),
                    style_dim(),
                ),
            ]));
        }

        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
    }

    // ── Live activity log ─────────────────────────────────────────────────────

    fn draw_activity(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border())
            .title(Span::styled(" ⚡ Activity ", style_tag_thought()));

        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.activity.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled("  Awaiting events…", style_muted())),
                inner,
            );
            return;
        }

        let visible = inner.height as usize;
        let total = self.activity.len();
        let start = total.saturating_sub(visible + self.scroll);

        let mut lines: Vec<Line> = Vec::new();
        for entry in self.activity.iter().skip(start).take(visible) {
            let icon_style = match entry.kind {
                ActivityKind::Tool    => style_tag_tool(),
                ActivityKind::Skill   => style_tag_skill(),
                ActivityKind::Thought => style_tag_thought(),
                ActivityKind::Goal    => style_tag_goal(),
                ActivityKind::System  => style_dim(),
            };
            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", entry.kind.icon()), icon_style),
                Span::styled(
                    truncate(&entry.message, (inner.width as usize).saturating_sub(10)),
                    ratatui::style::Style::default().fg(Palette::TEXT_DIM),
                ),
                Span::styled(format!(" {}", entry.time), style_muted()),
            ]));
        }

        f.render_widget(Paragraph::new(lines), inner);
    }
}

impl Default for Sidebar {
    fn default() -> Self { Self::new() }
}

// ── Helper ────────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        let end = s.char_indices().nth(max.saturating_sub(1)).map(|(i, _)| i).unwrap_or(s.len());
        format!("{}…", &s[..end])
    }
}

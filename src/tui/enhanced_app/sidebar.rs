use crate::tui::enhanced_app::state::SessionMetrics;
use crate::tui::enhanced_app::theme::{
    render_gauge_bar, style_border, style_dim, style_error, style_muted, style_success,
    style_tag_goal, style_tag_skill, style_tag_thought, style_title_2077, truncate, Palette,
    ICON_ERROR, ICON_GOAL, ICON_SKILL, ICON_SYSTEM, ICON_THOUGHT, ICON_TOOL,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

// ── Goal entry ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SidebarGoal {
    pub title: String,
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
            GoalPriority::High => Palette::WARNING,
            GoalPriority::Medium => Palette::ASSISTANT,
            GoalPriority::Low => Palette::TEXT_DIM,
        }
    }
    pub fn icon(&self) -> &'static str {
        match self {
            GoalPriority::Critical => "◈",
            GoalPriority::High => "◆",
            GoalPriority::Medium => "◇",
            GoalPriority::Low => "○",
        }
    }
}

// ── Activity entry ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ActivityEntry {
    pub kind: ActivityKind,
    pub message: String,
    pub time: String,
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
            ActivityKind::Tool => ICON_TOOL,
            ActivityKind::Skill => ICON_SKILL,
            ActivityKind::Thought => ICON_THOUGHT,
            ActivityKind::Goal => ICON_GOAL,
            ActivityKind::System => ICON_SYSTEM,
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        match self {
            ActivityKind::Tool => Palette::TOOL,
            ActivityKind::Skill => Palette::SKILL,
            ActivityKind::Thought => Palette::THOUGHT,
            ActivityKind::Goal => Palette::GOAL,
            ActivityKind::System => Palette::TEXT_MUTED,
        }
    }
}

// ── Sidebar state ─────────────────────────────────────────────────────────────

pub struct Sidebar {
    pub goals: Vec<SidebarGoal>,
    pub activity: Vec<ActivityEntry>,
    pub thoughts: Vec<String>,
    pub scroll: usize,
    pub goal_scroll: usize,
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            goals: Vec::new(),
            activity: Vec::new(),
            thoughts: Vec::new(),
            scroll: 0,
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
        if self.activity.len() > 150 {
            self.activity.remove(0);
        }
    }

    pub fn push_thought(&mut self, thought: impl Into<String>) {
        self.thoughts.push(thought.into());
        if self.thoughts.len() > 80 {
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
                Constraint::Length(10),
                Constraint::Min(5),
                Constraint::Length(7),
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
            .title(Span::styled(" ◆ SESSION ", style_title_2077()));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let error_style = if m.total_errors > 0 {
            style_error()
        } else {
            style_success()
        };

        // Compact 2077-style metrics
        let lines = vec![
            Line::from(vec![
                Span::styled("⏱", ratatui::style::Style::default().fg(Palette::YELLOW)),
                Span::styled("  Uptime    ", style_muted()),
                Span::styled(m.format_uptime(), style_success()),
            ]),
            Line::from(vec![
                Span::styled("💬", ratatui::style::Style::default().fg(Palette::CYAN)),
                Span::styled("  Messages  ", style_muted()),
                Span::styled(
                    format!("{}", m.total_messages),
                    ratatui::style::Style::default().fg(Palette::TEXT),
                ),
            ]),
            Line::from(vec![
                Span::styled("📥", ratatui::style::Style::default().fg(Palette::SUCCESS)),
                Span::styled("  Tokens↗  ", style_muted()),
                Span::styled(format!("{}", m.total_tokens_in), style_dim()),
            ]),
            Line::from(vec![
                Span::styled("📤", ratatui::style::Style::default().fg(Palette::INFO)),
                Span::styled("  Tokens↘  ", style_muted()),
                Span::styled(format!("{}", m.total_tokens_out), style_dim()),
            ]),
            Line::from(vec![
                Span::styled("↻", ratatui::style::Style::default().fg(Palette::VIOLET)),
                Span::styled("  Requests  ", style_muted()),
                Span::styled(
                    format!("{}", m.total_requests),
                    ratatui::style::Style::default().fg(Palette::TEXT),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    ICON_ERROR,
                    ratatui::style::Style::default().fg(Palette::ERROR),
                ),
                Span::styled("  Errors    ", style_muted()),
                Span::styled(format!("{}", m.total_errors), error_style),
            ]),
            Line::from(vec![
                Span::styled("⚡", ratatui::style::Style::default().fg(Palette::PINK)),
                Span::styled("  Speed     ", style_muted()),
                Span::styled(
                    format!("{:.1} t/s", m.avg_tokens_per_sec),
                    ratatui::style::Style::default().fg(Palette::CYAN),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    ICON_SKILL,
                    ratatui::style::Style::default().fg(Palette::SKILL),
                ),
                Span::styled("  Skills    ", style_muted()),
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
                format!(" {} Goals ({}) ", ICON_GOAL, self.goals.len()),
                style_tag_goal(),
            ));

        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.goals.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled("  No active goals", style_muted())),
                inner,
            );
            return;
        }

        let visible = inner.height as usize;
        let total = self.goals.len();
        let start = total.saturating_sub(visible + self.goal_scroll);

        let mut lines = Vec::new();
        for goal in self.goals.iter().skip(start).take(visible) {
            let bar = render_gauge_bar(goal.progress, 20);
            let bar_color = goal.priority.color();
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", goal.priority.icon()),
                    ratatui::style::Style::default()
                        .fg(goal.priority.color())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    truncate(&goal.title, (inner.width as usize).saturating_sub(6)),
                    ratatui::style::Style::default()
                        .fg(Palette::TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("   ", style_muted()),
                Span::styled(bar, ratatui::style::Style::default().fg(bar_color)),
                Span::styled(format!(" {:.0}%", goal.progress * 100.0), style_dim()),
            ]));
        }

        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
    }

    // ── Live activity log ─────────────────────────────────────────────────────

    fn draw_activity(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border())
            .title(Span::styled(
                format!(" {} Activity ", ICON_THOUGHT),
                style_tag_thought(),
            ));

        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.activity.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled("  Waiting for signal...", style_muted())),
                inner,
            );
            return;
        }

        let visible = inner.height as usize;
        let total = self.activity.len();
        let start = total.saturating_sub(visible + self.scroll);

        let mut lines: Vec<Line> = Vec::new();
        for entry in self.activity.iter().skip(start).take(visible) {
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", entry.kind.icon()),
                    ratatui::style::Style::default()
                        .fg(entry.kind.color())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    truncate(&entry.message, (inner.width as usize).saturating_sub(15)),
                    ratatui::style::Style::default().fg(Palette::TEXT),
                ),
            ]));
        }

        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

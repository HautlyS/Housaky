use crate::config::Config;
use crate::core::{DashboardMetrics, HousakyCore};
use crate::goal_engine::Goal;
use crate::providers::ChatMessage;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
    Frame,
};
use std::sync::Arc;

pub struct AGIDashboard {
    config: Config,
    core: Option<Arc<HousakyCore>>,
    provider_name: String,
    model_name: String,
    input: String,
    messages: Vec<ChatMessage>,
    thoughts: Vec<String>,
    goals: Vec<Goal>,
    metrics: Option<DashboardMetrics>,
    selected_tab: usize,
    tabs: Vec<&'static str>,
    should_quit: bool,
    status_message: String,
    input_focused: bool,
    scroll_offset: usize,
    thought_scroll: usize,
    goal_scroll: usize,
}

impl AGIDashboard {
    pub fn new(config: Config, provider_name: String, model_name: String) -> Self {
        Self {
            config,
            core: None,
            provider_name,
            model_name,
            input: String::new(),
            messages: Vec::new(),
            thoughts: Vec::new(),
            goals: Vec::new(),
            metrics: None,
            selected_tab: 0,
            tabs: vec!["Chat", "Goals", "Thoughts", "Metrics", "Help"],
            should_quit: false,
            status_message: "Housaky AGI Ready".to_string(),
            input_focused: true,
            scroll_offset: 0,
            thought_scroll: 0,
            goal_scroll: 0,
        }
    }

    pub fn with_core(mut self, core: Arc<HousakyCore>) -> Self {
        self.core = Some(core);
        self
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Tab => {
                self.selected_tab = (self.selected_tab + 1) % self.tabs.len();
            }
            KeyCode::BackTab => {
                self.selected_tab = if self.selected_tab == 0 {
                    self.tabs.len() - 1
                } else {
                    self.selected_tab - 1
                };
            }
            KeyCode::Enter if self.input_focused && !self.input.is_empty() => {
                self.send_message()?;
            }
            KeyCode::Char(c) if self.input_focused => {
                self.input.push(c);
            }
            KeyCode::Backspace if self.input_focused && !self.input.is_empty() => {
                self.input.pop();
            }
            KeyCode::Up => {
                if self.selected_tab == 2 {
                    self.thought_scroll = self.thought_scroll.saturating_sub(1);
                } else if self.selected_tab == 1 {
                    self.goal_scroll = self.goal_scroll.saturating_sub(1);
                } else {
                    self.scroll_offset = self.scroll_offset.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if self.selected_tab == 2 {
                    self.thought_scroll += 1;
                } else if self.selected_tab == 1 {
                    self.goal_scroll += 1;
                } else {
                    self.scroll_offset += 1;
                }
            }
            KeyCode::Char('q') if !self.input_focused => {
                self.should_quit = true;
            }
            KeyCode::Esc => {
                self.input_focused = !self.input_focused;
            }
            _ => {}
        }
        Ok(())
    }

    fn send_message(&mut self) -> Result<()> {
        let msg = self.input.clone();
        self.input.clear();

        self.messages.push(ChatMessage::user(&msg));
        self.status_message = "Processing...".to_string();

        self.thoughts.push(format!("User: {}", msg));

        self.status_message = format!("Processed: {}", msg.chars().take(30).collect::<String>());

        Ok(())
    }

    pub fn update(&mut self) {
        if let Some(core) = &self.core {
            let rt = tokio::runtime::Handle::current();
            let metrics = rt.block_on(async { core.get_dashboard_metrics().await });
            self.metrics = Some(metrics);

            let goals = rt.block_on(async { core.goal_engine.get_active_goals().await });
            self.goals = goals;

            let thoughts = rt.block_on(async { core.inner_monologue.get_recent(20).await });
            self.thoughts = thoughts;
        }
    }

    pub fn draw(&self, f: &mut Frame) {
        let size = f.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(size);

        let tabs = Tabs::new(self.tabs.iter().map(|s| Span::raw(*s)).collect::<Vec<_>>())
            .block(Block::default().borders(Borders::ALL).title("Housaky AGI"))
            .select(self.selected_tab)
            .style(Style::default().fg(Color::Cyan))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_widget(tabs, chunks[0]);

        match self.selected_tab {
            0 => self.draw_chat_tab(f, chunks[1]),
            1 => self.draw_goals_tab(f, chunks[1]),
            2 => self.draw_thoughts_tab(f, chunks[1]),
            3 => self.draw_metrics_tab(f, chunks[1]),
            4 => self.draw_help_tab(f, chunks[1]),
            _ => {}
        }

        let input_style = if self.input_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };

        let input = Paragraph::new(self.input.as_str())
            .style(input_style)
            .block(Block::default().borders(Borders::ALL).title("Input"));
        f.render_widget(input, chunks[2]);

        let status = Paragraph::new(self.status_message.as_str())
            .style(Style::default().fg(Color::Green))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(status, chunks[3]);
    }

    fn draw_chat_tab(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(50), Constraint::Length(30)])
            .split(area);

        let mut lines: Vec<Line> = Vec::new();
        let visible_messages: Vec<_> = self
            .messages
            .iter()
            .skip(self.scroll_offset)
            .take(area.height as usize - 2)
            .collect();

        for msg in visible_messages {
            let (prefix, color) = if msg.role == "user" {
                ("You: ", Color::Green)
            } else {
                ("AGI: ", Color::Cyan)
            };
            lines.push(Line::from(vec![
                Span::styled(
                    prefix,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(&msg.content),
            ]));
        }

        let chat = Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Conversation"));
        f.render_widget(chat, chunks[0]);

        let mut side_lines: Vec<Line> = Vec::new();

        if let Some(ref metrics) = self.metrics {
            side_lines.push(Line::from(Span::styled(
                "📊 Quick Metrics",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            side_lines.push(Line::from(Span::raw("")));
            side_lines.push(Line::from(Span::styled(
                format!("Confidence: {:.0}%", metrics.confidence_level * 100.0),
                Style::default().fg(Color::Green),
            )));
            side_lines.push(Line::from(Span::styled(
                format!("Turns: {}", metrics.total_turns),
                Style::default().fg(Color::White),
            )));
            side_lines.push(Line::from(Span::styled(
                format!("Success: {:.0}%", metrics.success_rate * 100.0),
                Style::default().fg(Color::Green),
            )));

            side_lines.push(Line::from(Span::raw("")));
            side_lines.push(Line::from(Span::styled(
                "🧠 Capabilities",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            side_lines.push(Line::from(Span::raw("")));

            self.draw_capability_bar(&mut side_lines, "Reasoning", metrics.capabilities.reasoning);
            self.draw_capability_bar(&mut side_lines, "Learning", metrics.capabilities.learning);
            self.draw_capability_bar(
                &mut side_lines,
                "Self-Aware",
                metrics.capabilities.self_awareness,
            );
            self.draw_capability_bar(
                &mut side_lines,
                "Meta-Cog",
                metrics.capabilities.meta_cognition,
            );
        } else {
            side_lines.push(Line::from("AGI Core not initialized"));
            side_lines.push(Line::from("Type to start chatting"));
        }

        let side = Paragraph::new(side_lines)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(side, chunks[1]);
    }

    fn draw_capability_bar(&self, lines: &mut Vec<Line>, name: &str, value: f64) {
        let bar_width = 10;
        let filled = (value * bar_width as f64) as usize;
        let empty = bar_width - filled;

        let bar = format!(
            "[{}{}] {:.0}%",
            "█".repeat(filled),
            "░".repeat(empty),
            value * 100.0
        );

        lines.push(Line::from(Span::styled(
            format!("{}: {}", name, bar),
            Style::default().fg(Color::Cyan),
        )));
    }

    fn draw_goals_tab(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            "📋 Active Goals",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::raw("")));

        let visible_goals: Vec<_> = self
            .goals
            .iter()
            .skip(self.goal_scroll)
            .take(area.height as usize - 5)
            .collect();

        for goal in visible_goals {
            let progress_bar = self.make_progress_bar(goal.progress);
            let priority_color = match goal.priority {
                crate::goal_engine::GoalPriority::Critical => Color::Red,
                crate::goal_engine::GoalPriority::High => Color::Yellow,
                crate::goal_engine::GoalPriority::Medium => Color::Blue,
                crate::goal_engine::GoalPriority::Low => Color::Gray,
                crate::goal_engine::GoalPriority::Background => Color::DarkGray,
            };

            lines.push(Line::from(vec![
                Span::styled("● ", Style::default().fg(priority_color)),
                Span::styled(&goal.title, Style::default().add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(Span::styled(
                format!("  {} {}", progress_bar, (goal.progress * 100.0) as i32),
                Style::default().fg(Color::Green),
            )));
            lines.push(Line::from(Span::styled(
                format!(
                    "  {}",
                    goal.description.chars().take(60).collect::<String>()
                ),
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(Span::raw("")));
        }

        if self.goals.is_empty() {
            lines.push(Line::from(
                "No active goals. Goals will be created as you interact.",
            ));
        }

        let goals_widget =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Goals"));
        f.render_widget(goals_widget, area);
    }

    fn make_progress_bar(&self, progress: f64) -> String {
        let width = 20;
        let filled = (progress * width as f64) as usize;
        let empty = width - filled;
        format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
    }

    fn draw_thoughts_tab(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            "💭 Inner Monologue",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::raw("")));

        let visible_thoughts: Vec<_> = self
            .thoughts
            .iter()
            .skip(self.thought_scroll)
            .take(area.height as usize - 5)
            .collect();

        for (i, thought) in visible_thoughts.iter().enumerate() {
            let num = self.thought_scroll + i + 1;
            lines.push(Line::from(vec![
                Span::styled(format!("{:3}. ", num), Style::default().fg(Color::DarkGray)),
                Span::styled(
                    thought
                        .chars()
                        .take(area.width as usize - 10)
                        .collect::<String>(),
                    Style::default().fg(Color::White),
                ),
            ]));
        }

        if self.thoughts.is_empty() {
            lines.push(Line::from("No thoughts recorded yet."));
            lines.push(Line::from(
                "Thoughts appear as the AGI processes your messages.",
            ));
        }

        let thoughts_widget =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Thoughts"));
        f.render_widget(thoughts_widget, area);
    }

    fn draw_metrics_tab(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        if let Some(ref metrics) = self.metrics {
            lines.push(Line::from(Span::styled(
                "📊 AGI Metrics Dashboard",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::raw("")));

            lines.push(Line::from(Span::styled(
                "Activity",
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(Span::raw(format!(
                "  Total Turns:        {}",
                metrics.total_turns
            ))));
            lines.push(Line::from(Span::raw(format!(
                "  Successful Actions: {}",
                metrics.successful_actions
            ))));
            lines.push(Line::from(Span::raw(format!(
                "  Failed Actions:     {}",
                metrics.failed_actions
            ))));
            lines.push(Line::from(Span::raw(format!(
                "  Success Rate:       {:.1}%",
                metrics.success_rate * 100.0
            ))));

            lines.push(Line::from(Span::raw("")));
            lines.push(Line::from(Span::styled(
                "State",
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(Span::raw(format!(
                "  Confidence Level:   {:.1}%",
                metrics.confidence_level * 100.0
            ))));
            lines.push(Line::from(Span::raw(format!(
                "  Evolution Stage:    {}",
                metrics.evolution_stage
            ))));
            lines.push(Line::from(Span::raw(format!(
                "  Uptime:             {}s",
                metrics.uptime_seconds
            ))));

            lines.push(Line::from(Span::raw("")));
            lines.push(Line::from(Span::styled(
                "Memory",
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(Span::raw(format!(
                "  Items:              {}",
                metrics.memory_items
            ))));
            lines.push(Line::from(Span::raw(format!(
                "  Tokens:             {}",
                metrics.memory_tokens
            ))));

            lines.push(Line::from(Span::raw("")));
            lines.push(Line::from(Span::styled(
                "Knowledge",
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(Span::raw(format!(
                "  Entities:           {}",
                metrics.knowledge_entities
            ))));
            lines.push(Line::from(Span::raw(format!(
                "  Relations:          {}",
                metrics.knowledge_relations
            ))));
        } else {
            lines.push(Line::from("Metrics will appear after initialization."));
        }

        let metrics_widget =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Metrics"));
        f.render_widget(metrics_widget, area);
    }

    fn draw_help_tab(&self, f: &mut Frame, area: Rect) {
        let lines = vec![
            Line::from(Span::styled(
                "❓ Housaky AGI Help",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                "Navigation",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(Span::raw("  Tab        - Switch between tabs")),
            Line::from(Span::raw("  ↑/↓        - Scroll content")),
            Line::from(Span::raw("  Esc        - Toggle input focus")),
            Line::from(Span::raw("  Ctrl+C     - Exit")),
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                "Chat Commands",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(Span::raw("  /goals     - Show active goals")),
            Line::from(Span::raw("  /metrics   - Show AGI metrics")),
            Line::from(Span::raw("  /reflect   - Trigger reflection")),
            Line::from(Span::raw("  /thoughts  - Show recent thoughts")),
            Line::from(Span::raw("  /quit      - Exit")),
            Line::from(Span::raw("")),
            Line::from(Span::styled("Tabs", Style::default().fg(Color::Yellow))),
            Line::from(Span::raw("  Chat       - Interactive conversation")),
            Line::from(Span::raw("  Goals      - Active AGI goals")),
            Line::from(Span::raw("  Thoughts   - Inner monologue stream")),
            Line::from(Span::raw("  Metrics    - Performance statistics")),
            Line::from(Span::raw("  Help       - This help screen")),
        ];

        let help =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Help"));
        f.render_widget(help, area);
    }
}

use crate::config::Config;
use crate::housaky::cognitive::cognitive_loop::{CognitiveLoop, CognitiveResponse};
use crate::housaky::goal_engine::Goal;
use crate::housaky::streaming::{StreamChunk, StreamState, StreamingManager};
use crate::providers::{create_provider, ChatMessage};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
    Frame,
};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tracing::info;

pub struct LiveAGIApp {
    config: Config,
    cognitive_loop: Option<Arc<CognitiveLoop>>,
    streaming_manager: Arc<StreamingManager>,

    provider_name: String,
    model_name: String,
    provider: Option<Box<dyn crate::providers::Provider>>,

    input: String,
    input_history: Vec<String>,
    history_index: usize,

    messages: Vec<ChatMessage>,
    current_response: String,
    streaming_content: String,

    thoughts: Vec<String>,
    goals: Vec<Goal>,

    selected_tab: usize,
    tabs: Vec<&'static str>,

    should_quit: bool,
    status_message: String,
    input_focused: bool,

    scroll_offset: usize,
    thought_scroll: usize,
    goal_scroll: usize,

    stream_receiver: Option<broadcast::Receiver<StreamChunk>>,
    state_receiver: Option<broadcast::Receiver<StreamState>>,

    pending_request: Option<String>,
    response_tx: Option<mpsc::Sender<CognitiveResponse>>,
    response_rx: Option<mpsc::Receiver<CognitiveResponse>>,

    is_streaming: bool,
    tokens_received: usize,
    elapsed_ms: u64,
    tokens_per_second: f64,
}

impl LiveAGIApp {
    pub fn new(config: Config, provider_name: String, model_name: String) -> Self {
        let streaming_manager = Arc::new(StreamingManager::new());
        let (response_tx, response_rx) = mpsc::channel(16);
        let stream_receiver = Some(streaming_manager.subscribe_chunks());
        let state_receiver = Some(streaming_manager.subscribe_state());

        Self {
            config,
            cognitive_loop: None,
            streaming_manager,
            provider_name,
            model_name,
            provider: None,
            input: String::new(),
            input_history: Vec::new(),
            history_index: 0,
            messages: Vec::new(),
            current_response: String::new(),
            streaming_content: String::new(),
            thoughts: Vec::new(),
            goals: Vec::new(),
            selected_tab: 0,
            tabs: vec!["Chat", "Goals", "Thoughts", "Metrics", "Help"],
            should_quit: false,
            status_message: "Housaky AGI Ready - Type to start".to_string(),
            input_focused: true,
            scroll_offset: 0,
            thought_scroll: 0,
            goal_scroll: 0,
            stream_receiver,
            state_receiver,
            pending_request: None,
            response_tx: Some(response_tx),
            response_rx: Some(response_rx),
            is_streaming: false,
            tokens_received: 0,
            elapsed_ms: 0,
            tokens_per_second: 0.0,
        }
    }

    pub fn with_cognitive_loop(mut self, loop_: Arc<CognitiveLoop>) -> Self {
        self.cognitive_loop = Some(loop_);
        self
    }

    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing Live AGI App...");

        let api_key = self
            .config
            .fallback
            .providers
            .iter()
            .find(|p| p.name == self.provider_name)
            .and_then(|p| p.api_key_encrypted.clone());

        self.provider = Some(create_provider(&self.provider_name, api_key.as_deref())?);

        if let Some(loop_) = &self.cognitive_loop {
            loop_.initialize().await?;
        }

        self.thoughts.push("System initialized".to_string());
        self.status_message = format!("Connected to {} ({})", self.provider_name, self.model_name);

        info!("Live AGI App initialized successfully");
        Ok(())
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.selected_tab = if self.selected_tab == 0 {
                        self.tabs.len() - 1
                    } else {
                        self.selected_tab - 1
                    };
                } else {
                    self.selected_tab = (self.selected_tab + 1) % self.tabs.len();
                }
            }
            KeyCode::Enter if self.input_focused && !self.input.is_empty() => {
                self.send_message();
            }
            KeyCode::Char(c) if self.input_focused => {
                self.input.push(c);
            }
            KeyCode::Backspace if self.input_focused && !self.input.is_empty() => {
                self.input.pop();
            }
            KeyCode::Up => {
                if self.input_focused && !self.input_history.is_empty() {
                    if self.history_index < self.input_history.len() {
                        self.history_index += 1;
                        if let Some(prev) =
                            self.input_history.iter().rev().nth(self.history_index - 1)
                        {
                            self.input = prev.clone();
                        }
                    }
                } else {
                    self.handle_scroll(-1);
                }
            }
            KeyCode::Down => {
                if self.input_focused && self.history_index > 0 {
                    self.history_index -= 1;
                    if self.history_index == 0 {
                        self.input.clear();
                    } else if let Some(prev) =
                        self.input_history.iter().rev().nth(self.history_index - 1)
                    {
                        self.input = prev.clone();
                    }
                } else {
                    self.handle_scroll(1);
                }
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Char('q') if !self.input_focused => {
                self.should_quit = true;
            }
            KeyCode::Esc => {
                self.input_focused = !self.input_focused;
            }
            KeyCode::Char('/') if self.input_focused && self.input.is_empty() => {
                self.handle_command_start();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_scroll(&mut self, direction: i32) {
        match self.selected_tab {
            0 => self.scroll_offset = (self.scroll_offset as i32 + direction).max(0) as usize,
            1 => self.goal_scroll = (self.goal_scroll as i32 + direction).max(0) as usize,
            2 => self.thought_scroll = (self.thought_scroll as i32 + direction).max(0) as usize,
            _ => {}
        }
    }

    fn handle_command_start(&mut self) {
        self.status_message = "Command mode - type command name".to_string();
    }

    fn send_message(&mut self) {
        let msg = self.input.trim().to_string();
        if msg.is_empty() {
            return;
        }

        self.input_history.push(msg.clone());
        self.history_index = 0;

        if msg.starts_with('/') {
            self.handle_command(&msg[1..]);
        } else {
            self.messages.push(ChatMessage::user(&msg));
            self.status_message = "Processing...".to_string();
            self.thoughts.push(format!("User: {}", msg));
            self.pending_request = Some(msg);
            self.streaming_content.clear();
            self.is_streaming = true;
        }

        self.input.clear();
    }

    fn handle_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let command = parts.first().unwrap_or(&"");

        match *command {
            "clear" => {
                self.messages.clear();
                self.streaming_content.clear();
                self.status_message = "Conversation cleared".to_string();
            }
            "goals" => {
                self.selected_tab = 1;
                self.status_message = "Viewing goals".to_string();
            }
            "thoughts" => {
                self.selected_tab = 2;
                self.status_message = "Viewing thought stream".to_string();
            }
            "metrics" => {
                self.selected_tab = 3;
                self.status_message = "Viewing metrics".to_string();
            }
            "help" => {
                self.selected_tab = 4;
                self.status_message = "Viewing help".to_string();
            }
            "quit" | "exit" => {
                self.should_quit = true;
            }
            "model" => {
                if let Some(new_model) = parts.get(1) {
                    self.model_name = new_model.to_string();
                    self.status_message = format!("Model changed to: {}", new_model);
                }
            }
            "reflect" => {
                if let Some(_loop_) = &self.cognitive_loop {
                    self.thoughts.push("Initiating reflection...".to_string());
                }
            }
            _ => {
                self.status_message = format!("Unknown command: /{}", command);
            }
        }
    }

    pub fn update(&mut self) {
        while let Ok(chunk) = self.stream_receiver.as_mut().unwrap().try_recv() {
            let content = chunk.content.clone();
            self.streaming_content = chunk.content;
            self.tokens_received = chunk.token_count;
            self.elapsed_ms = chunk.elapsed_ms;
            self.tokens_per_second = chunk.tokens_per_second;

            if chunk.is_complete {
                self.messages.push(ChatMessage::assistant(&content));
                self.is_streaming = false;
                self.status_message = format!(
                    "Response complete ({} tokens, {:.1} t/s)",
                    chunk.token_count, chunk.tokens_per_second
                );
                self.thoughts.push(format!(
                    "Assistant: {}",
                    content.chars().take(100).collect::<String>()
                ));
            }
        }

        while let Ok(state) = self.state_receiver.as_mut().unwrap().try_recv() {
            match state {
                StreamState::Streaming => {
                    self.status_message = format!("Streaming... ({} tokens)", self.tokens_received);
                }
                StreamState::Error => {
                    self.status_message = "Error during streaming".to_string();
                    self.is_streaming = false;
                }
                _ => {}
            }
        }

        if let Some(loop_) = &self.cognitive_loop {
            let rt = tokio::runtime::Handle::current();

            let goals = rt.block_on(async { loop_.goal_engine.get_active_goals().await });
            self.goals = goals;

            let recent = rt.block_on(async { loop_.inner_monologue.get_recent(50).await });
            self.thoughts = recent;
        }
    }

    pub async fn process_pending_request(&mut self) -> Result<()> {
        if let Some(request) = self.pending_request.take() {
            if let (Some(provider), Some(loop_)) = (&self.provider, &self.cognitive_loop) {
                let response = loop_
                    .process(&request, provider.as_ref(), &self.model_name, &[])
                    .await?;

                self.streaming_manager
                    .simulate_stream(&response.content, 30)
                    .await;

                self.status_message = format!("Confidence: {:.0}%", response.confidence * 100.0);
            } else if let Some(provider) = &self.provider {
                let response = provider
                    .chat_with_history(&self.messages, &self.model_name, 0.7)
                    .await?;
                self.streaming_manager.simulate_stream(&response, 30).await;
            }
        }
        Ok(())
    }

    pub fn draw(&self, f: &mut Frame) {
        let size = f.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(5),
                Constraint::Length(3),
            ])
            .split(size);

        self.draw_header(f, chunks[0]);

        match self.selected_tab {
            0 => self.draw_chat_tab(f, chunks[1]),
            1 => self.draw_goals_tab(f, chunks[1]),
            2 => self.draw_thoughts_tab(f, chunks[1]),
            3 => self.draw_metrics_tab(f, chunks[1]),
            4 => self.draw_help_tab(f, chunks[1]),
            _ => {}
        }

        self.draw_input_area(f, chunks[2]);
        self.draw_status_bar(f, chunks[3]);
    }

    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let tabs = Tabs::new(self.tabs.iter().map(|s| Span::raw(*s)).collect::<Vec<_>>())
            .block(Block::default().borders(Borders::ALL).title("Housaky AGI"))
            .select(self.selected_tab)
            .style(Style::default().fg(Color::Cyan))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_widget(tabs, area);
    }

    fn draw_chat_tab(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(50), Constraint::Length(35)])
            .split(area);

        let mut lines: Vec<Line> = Vec::new();

        let visible_messages: Vec<_> = self
            .messages
            .iter()
            .skip(self.scroll_offset)
            .take(chunks[0].height as usize - 2)
            .collect();

        for msg in visible_messages {
            let (prefix, color) = if msg.role == "user" {
                ("You: ", Color::Green)
            } else {
                ("AGI: ", Color::Cyan)
            };

            let content = msg.content.clone();
            let msg_lines: Vec<Line> = content
                .lines()
                .enumerate()
                .map(|(i, line_text)| {
                    if i == 0 {
                        Line::from(vec![
                            Span::styled(
                                prefix,
                                Style::default().fg(color).add_modifier(Modifier::BOLD),
                            ),
                            Span::raw(line_text.to_string()),
                        ])
                    } else {
                        Line::from(Span::raw(line_text.to_string()))
                    }
                })
                .collect();

            for line in msg_lines {
                lines.push(line);
            }
            lines.push(Line::from(""));
        }

        if self.is_streaming && !self.streaming_content.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    "AGI: ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(&self.streaming_content),
                Span::styled(
                    "▌",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
            ]));
        }

        let chat = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title("Conversation"));
        f.render_widget(chat, chunks[0]);

        self.draw_sidebar(f, chunks[1]);
    }

    fn draw_sidebar(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            "📊 Live Metrics",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        if self.is_streaming {
            lines.push(Line::from(Span::styled(
                "⏳ Streaming...",
                Style::default().fg(Color::Yellow),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "✓ Ready",
                Style::default().fg(Color::Green),
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Tokens: {}", self.tokens_received),
            Style::default().fg(Color::Cyan),
        )));

        lines.push(Line::from(Span::styled(
            format!("Time: {:.1}s", self.elapsed_ms as f64 / 1000.0),
            Style::default().fg(Color::Cyan),
        )));

        lines.push(Line::from(Span::styled(
            format!("Speed: {:.1} t/s", self.tokens_per_second),
            Style::default().fg(Color::Cyan),
        )));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Provider: {}", self.provider_name),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(Span::styled(
            format!("Model: {}", self.model_name),
            Style::default().fg(Color::DarkGray),
        )));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "💡 Quick Commands",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from("  /clear - Clear chat"));
        lines.push(Line::from("  /goals - View goals"));
        lines.push(Line::from("  /reflect - Reflect"));

        let sidebar =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(sidebar, area);
    }

    fn draw_goals_tab(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            "📋 Active Goals",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        for goal in self
            .goals
            .iter()
            .skip(self.goal_scroll)
            .take(area.height as usize - 5)
        {
            let progress_bar = self.make_progress_bar(goal.progress);
            let priority_color = match goal.priority {
                crate::housaky::goal_engine::GoalPriority::Critical => Color::Red,
                crate::housaky::goal_engine::GoalPriority::High => Color::Yellow,
                crate::housaky::goal_engine::GoalPriority::Medium => Color::Blue,
                crate::housaky::goal_engine::GoalPriority::Low => Color::Gray,
                crate::housaky::goal_engine::GoalPriority::Background => Color::DarkGray,
            };

            lines.push(Line::from(vec![
                Span::styled("● ", Style::default().fg(priority_color)),
                Span::styled(&goal.title, Style::default().add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(Span::styled(
                format!("  {} {}", progress_bar, (goal.progress * 100.0) as i32),
                Style::default().fg(Color::Green),
            )));
            lines.push(Line::from(""));
        }

        if self.goals.is_empty() {
            lines.push(Line::from(
                "No active goals. Goals will be created during interactions.",
            ));
        }

        let goals_widget =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Goals"));
        f.render_widget(goals_widget, area);
    }

    fn draw_thoughts_tab(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            "💭 Inner Monologue",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        for (i, thought) in self
            .thoughts
            .iter()
            .skip(self.thought_scroll)
            .take(area.height as usize - 5)
            .enumerate()
        {
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
        }

        let thoughts_widget =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Thoughts"));
        f.render_widget(thoughts_widget, area);
    }

    fn draw_metrics_tab(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            "📊 AGI Metrics",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        if let Some(loop_) = &self.cognitive_loop {
            let rt = tokio::runtime::Handle::current();
            let metrics = rt.block_on(async { loop_.get_metrics().await });
            lines.push(Line::from(Span::styled(
                "Activity",
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(format!(
                "  Total Turns: {}",
                metrics.total_turns
            )));
            lines.push(Line::from(format!(
                "  Success Rate: {:.1}%",
                metrics.success_rate * 100.0
            )));
            lines.push(Line::from(""));

            lines.push(Line::from(Span::styled(
                "Learning",
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(format!(
                "  Patterns: {}",
                metrics.learning_stats.patterns_discovered
            )));
            lines.push(Line::from(format!(
                "  Lessons: {}",
                metrics.learning_stats.lessons_learned
            )));
            lines.push(Line::from(""));

            lines.push(Line::from(Span::styled(
                "Memory",
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(format!(
                "  Short-term: {}",
                metrics.memory_stats.short_term_count
            )));
            lines.push(Line::from(format!(
                "  Long-term: {}",
                metrics.memory_stats.long_term_count
            )));
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
            Line::from(""),
            Line::from(Span::styled(
                "Navigation",
                Style::default().fg(Color::Yellow),
            )),
            Line::from("  Tab        - Switch tabs"),
            Line::from("  Shift+Tab  - Previous tab"),
            Line::from("  ↑/↓        - Scroll / History"),
            Line::from("  Esc        - Toggle input focus"),
            Line::from("  Ctrl+C     - Exit"),
            Line::from(""),
            Line::from(Span::styled("Commands", Style::default().fg(Color::Yellow))),
            Line::from("  /clear     - Clear conversation"),
            Line::from("  /goals     - View goals"),
            Line::from("  /thoughts  - View thought stream"),
            Line::from("  /metrics   - View metrics"),
            Line::from("  /reflect   - Trigger reflection"),
            Line::from("  /model X   - Change model"),
            Line::from("  /quit      - Exit"),
            Line::from(""),
            Line::from(Span::styled("Features", Style::default().fg(Color::Yellow))),
            Line::from("  • Real-time streaming responses"),
            Line::from("  • Live thought stream"),
            Line::from("  • Automatic goal tracking"),
            Line::from("  • Experience-based learning"),
            Line::from("  • Uncertainty detection"),
        ];

        let help =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Help"));
        f.render_widget(help, area);
    }

    fn draw_input_area(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(2)])
            .split(area);

        let input_style = if self.input_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };

        let prompt = if self.input.starts_with('/') {
            "Command"
        } else {
            "Message"
        };
        let input = Paragraph::new(self.input.as_str())
            .style(input_style)
            .block(Block::default().borders(Borders::ALL).title(prompt));
        f.render_widget(input, chunks[0]);

        let hint = if self.input_focused {
            "Enter to send | ↑↓ for history | / for commands"
        } else {
            "Press Esc to focus input"
        };
        let hint = Paragraph::new(hint).style(Style::default().fg(Color::DarkGray));
        f.render_widget(hint, chunks[1]);
    }

    fn draw_status_bar(&self, f: &mut Frame, area: Rect) {
        let status_style = if self.is_streaming {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Green)
        };

        let status = Paragraph::new(self.status_message.as_str())
            .style(status_style)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(status, area);
    }

    fn make_progress_bar(&self, progress: f64) -> String {
        let width = 20;
        let filled = (progress * width as f64) as usize;
        let empty = width - filled;
        format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
    }
}

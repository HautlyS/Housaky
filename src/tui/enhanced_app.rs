use crate::config::Config;
use crate::goal_engine::GoalEngine;
use crate::meta_cognition::MetaCognitionEngine;
use crate::reasoning_engine::{ReasoningChain, ReasoningEngine};
use crate::providers::{create_provider, ChatMessage};
use crate::tui::chat::{format_message_content, Message};
use crate::tui::command_palette::{CommandAction, CommandPalette};
use crate::tui::state_panel::StatePanel;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, Wrap,
    },
    Frame,
};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
    Search,
    CommandPalette,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Chat,
    State,
    Reasoning,
    Goals,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Single,
    Split,
    Triple,
}

pub struct StreamingState {
    pub is_streaming: bool,
    pub current_response: String,
    pub tokens_received: usize,
    pub start_time: Option<std::time::Instant>,
}

impl StreamingState {
    pub fn new() -> Self {
        Self {
            is_streaming: false,
            current_response: String::new(),
            tokens_received: 0,
            start_time: None,
        }
    }

    pub fn start(&mut self) {
        self.is_streaming = true;
        self.current_response.clear();
        self.tokens_received = 0;
        self.start_time = Some(std::time::Instant::now());
    }

    pub fn append(&mut self, text: &str) {
        self.current_response.push_str(text);
        self.tokens_received += text.len() / 4;
    }

    pub fn finish(&mut self) -> String {
        self.is_streaming = false;
        std::mem::take(&mut self.current_response)
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start_time
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0)
    }

    pub fn tokens_per_second(&self) -> f64 {
        if self.tokens_received == 0 || self.elapsed_ms() == 0 {
            return 0.0;
        }
        (self.tokens_received as f64) / (self.elapsed_ms() as f64 / 1000.0)
    }
}

pub struct EnhancedChatState {
    pub messages: Vec<Message>,
    pub provider_name: String,
    pub model: String,
    pub loading: bool,
    pub scroll_offset: usize,
    pub auto_scroll: bool,
    pub next_id: usize,
    pub streaming: StreamingState,
    pub context_messages: usize,
    pub total_tokens: usize,
}

impl EnhancedChatState {
    pub fn new(provider_name: String, model: String) -> Self {
        Self {
            messages: Vec::new(),
            provider_name,
            model,
            loading: false,
            scroll_offset: 0,
            auto_scroll: true,
            next_id: 0,
            streaming: StreamingState::new(),
            context_messages: 0,
            total_tokens: 0,
        }
    }

    pub fn add_user_message(&mut self, content: String) -> usize {
        let id = self.next_id;
        self.total_tokens += content.len() / 4;
        self.messages.push(Message::user(content, id));
        self.next_id += 1;
        id
    }

    pub fn add_assistant_message(&mut self, content: String) -> usize {
        let id = self.next_id;
        self.messages.push(Message::assistant(content, id));
        self.next_id += 1;
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
        id
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
            self.auto_scroll = false;
        }
    }

    pub fn scroll_down(&mut self) {
        let max_scroll = self.messages.len().saturating_sub(1);
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
        if self.scroll_offset >= max_scroll {
            self.auto_scroll = true;
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.messages.len().saturating_sub(1);
        self.auto_scroll = true;
    }

    pub fn get_last_assistant_message(&self) -> Option<&Message> {
        self.messages.iter().rev().find(|m| m.role == "assistant")
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
        self.next_id = 0;
        self.total_tokens = 0;
    }
}

pub struct EnhancedApp {
    pub input_mode: InputMode,
    pub input: String,
    pub should_quit: bool,
    pub chat: EnhancedChatState,
    pub config: Config,
    pub command_palette: CommandPalette,
    pub active_panel: Panel,
    pub view_mode: ViewMode,
    pub state_panel: StatePanel,
    pub goal_engine: Arc<GoalEngine>,
    pub reasoning_engine: Arc<ReasoningEngine>,
    pub meta_cognition: Arc<MetaCognitionEngine>,
    pub current_reasoning: Option<ReasoningChain>,
    pub notification: Option<(String, std::time::Instant)>,
    pub help_visible: bool,
}

impl EnhancedApp {
    pub fn new(config: Config, provider_name: String, model: String) -> Self {
        let workspace_dir = config.workspace_dir.clone();

        Self {
            input_mode: InputMode::Normal,
            input: String::new(),
            should_quit: false,
            chat: EnhancedChatState::new(provider_name, model),
            config,
            command_palette: CommandPalette::new(),
            active_panel: Panel::Chat,
            view_mode: ViewMode::Split,
            state_panel: StatePanel::new(),
            goal_engine: Arc::new(GoalEngine::new(&workspace_dir)),
            reasoning_engine: Arc::new(ReasoningEngine::new()),
            meta_cognition: Arc::new(MetaCognitionEngine::new()),
            current_reasoning: None,
            notification: None,
            help_visible: false,
        }
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn update(&mut self) {
        if let Some((_, time)) = &self.notification {
            if time.elapsed().as_secs() > 3 {
                self.notification = None;
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if self.help_visible {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::F(1)) {
                self.help_visible = false;
            }
            return Ok(());
        }

        match self.input_mode {
            InputMode::Normal => self.handle_normal_key(key),
            InputMode::Editing => self.handle_editing_key(key),
            InputMode::Search => self.handle_search_key(key),
            InputMode::CommandPalette => self.handle_command_palette_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Char('i') | KeyCode::Enter) => {
                self.input_mode = InputMode::Editing;
            }
            (KeyModifiers::NONE, KeyCode::Char('q')) => {
                self.should_quit = true;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                self.input_mode = InputMode::CommandPalette;
                self.command_palette.activate();
            }
            (KeyModifiers::NONE, KeyCode::Char('?') | KeyCode::F(1)) => {
                self.help_visible = true;
            }
            (KeyModifiers::NONE, KeyCode::Tab) => {
                self.cycle_panel();
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                self.cycle_panel_reverse();
            }
            (KeyModifiers::NONE, KeyCode::Char('v')) => {
                self.cycle_view_mode();
            }
            (KeyModifiers::NONE, KeyCode::Up) => match self.active_panel {
                Panel::Chat => {
                    self.chat.scroll_up();
                }
                Panel::State => {
                    self.state_panel.scroll_up();
                }
                _ => {}
            },
            (KeyModifiers::NONE, KeyCode::Down) => match self.active_panel {
                Panel::Chat => {
                    self.chat.scroll_down();
                }
                Panel::State => {
                    self.state_panel.scroll_down();
                }
                _ => {}
            },
            (KeyModifiers::NONE, KeyCode::PageUp) => {
                for _ in 0..10 {
                    self.chat.scroll_up();
                }
            }
            (KeyModifiers::NONE, KeyCode::PageDown) => {
                for _ in 0..10 {
                    self.chat.scroll_down();
                }
            }
            (KeyModifiers::NONE, KeyCode::Home) => {
                self.chat.scroll_offset = 0;
                self.chat.auto_scroll = false;
            }
            (KeyModifiers::NONE, KeyCode::End) => {
                self.chat.scroll_to_bottom();
            }
            (KeyModifiers::NONE, KeyCode::Char('c')) => {
                self.copy_last_response();
            }
            (KeyModifiers::NONE, KeyCode::Char('s')) => {
                self.save_conversation()?;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                self.chat.clear_messages();
                self.show_notification("Chat cleared");
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.active_panel = Panel::Reasoning;
                self.show_notification("Switched to Reasoning panel");
            }
            (KeyModifiers::NONE, KeyCode::Char('g')) => {
                self.active_panel = Panel::Goals;
                self.show_notification("Switched to Goals panel");
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_editing_key(&mut self, key: KeyEvent) -> Result<()> {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Enter) => {
                if !self.input.is_empty() {
                    self.send_message()?;
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.input_mode = InputMode::Normal;
            }
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.input_mode = InputMode::Normal;
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) => {
                self.input.push(c);
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                self.input.pop();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
                while let Some(c) = self.input.pop() {
                    if c == ' ' {
                        break;
                    }
                }
            }
            (KeyModifiers::NONE, KeyCode::Delete) => {
                self.input.clear();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> Result<()> {
        if let (KeyModifiers::NONE, KeyCode::Esc) = (key.modifiers, key.code) {
            self.input_mode = InputMode::Normal;
        }
        Ok(())
    }

    fn handle_command_palette_key(&mut self, key: KeyEvent) -> Result<()> {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.input_mode = InputMode::Normal;
                self.command_palette.deactivate();
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                if let Some(action) = self.command_palette.execute() {
                    self.execute_command(action)?;
                }
                self.input_mode = InputMode::Normal;
            }
            (KeyModifiers::NONE, KeyCode::Up) => {
                self.command_palette.prev();
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                self.command_palette.next();
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) => {
                self.command_palette.input.push(c);
                self.command_palette.filter();
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                self.command_palette.input.pop();
                self.command_palette.filter();
            }
            _ => {}
        }
        Ok(())
    }

    fn execute_command(&mut self, action: CommandAction) -> Result<()> {
        match action {
            CommandAction::ClearChat => {
                self.chat.clear_messages();
                self.show_notification("Chat cleared");
            }
            CommandAction::ToggleView => {
                self.cycle_view_mode();
            }
            CommandAction::ShowGoals => {
                self.active_panel = Panel::Goals;
            }
            CommandAction::ShowReasoning => {
                self.active_panel = Panel::Reasoning;
            }
            CommandAction::ShowState => {
                self.active_panel = Panel::State;
            }
            CommandAction::Reflect => {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async {
                    let _ = self
                        .meta_cognition
                        .reflect("User requested reflection")
                        .await;
                });
                self.show_notification("Reflection complete");
            }
            CommandAction::AddGoal(goal_desc) => {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async {
                    let goal = crate::goal_engine::Goal {
                        title: goal_desc,
                        ..Default::default()
                    };
                    let _ = self.goal_engine.add_goal(goal).await;
                });
                self.show_notification("Goal added");
            }
            CommandAction::ExportChat => {
                self.save_conversation()?;
            }
            CommandAction::ToggleAutoScroll => {
                self.chat.auto_scroll = !self.chat.auto_scroll;
                self.show_notification(if self.chat.auto_scroll {
                    "Auto-scroll on"
                } else {
                    "Auto-scroll off"
                });
            }
        }
        Ok(())
    }

    fn cycle_panel(&mut self) {
        self.active_panel = match self.active_panel {
            Panel::Chat => Panel::State,
            Panel::State => Panel::Reasoning,
            Panel::Reasoning => Panel::Goals,
            Panel::Goals => Panel::Chat,
        };
    }

    fn cycle_panel_reverse(&mut self) {
        self.active_panel = match self.active_panel {
            Panel::Chat => Panel::Goals,
            Panel::Goals => Panel::Reasoning,
            Panel::Reasoning => Panel::State,
            Panel::State => Panel::Chat,
        };
    }

    fn cycle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Single => ViewMode::Split,
            ViewMode::Split => ViewMode::Triple,
            ViewMode::Triple => ViewMode::Single,
        };
        self.show_notification(match self.view_mode {
            ViewMode::Single => "Single panel view",
            ViewMode::Split => "Split panel view",
            ViewMode::Triple => "Triple panel view",
        });
    }

    fn show_notification(&mut self, msg: &str) {
        self.notification = Some((msg.to_string(), std::time::Instant::now()));
    }

    fn copy_last_response(&mut self) {
        if let Some(msg) = self.chat.get_last_assistant_message() {
            self.show_notification(&format!("Copied {} chars", msg.content.len()));
        }
    }

    fn save_conversation(&self) -> Result<()> {
        let filename = format!(
            "housaky_chat_{}.md",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        );

        let mut content = format!(
            "# Housaky Chat Export\n\n**Provider**: {}  \n**Model**: {}  \n**Date**: {}\n\n---\n\n",
            self.chat.provider_name,
            self.chat.model,
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        );

        for msg in &self.chat.messages {
            let role = match msg.role.as_str() {
                "user" => "👤 **User**",
                "assistant" => "🤖 **Housaky**",
                _ => "⚙️ **System**",
            };
            content.push_str(&format!("{}\n\n{}\n\n---\n\n", role, msg.content));
        }

        std::fs::write(&filename, content)?;
        Ok(())
    }

    fn send_message(&mut self) -> Result<()> {
        let message = std::mem::take(&mut self.input);
        self.chat.add_user_message(message.clone());
        self.chat.loading = true;
        self.chat.streaming.start();

        let provider_name = self.chat.provider_name.clone();
        let model = self.chat.model.clone();
        let api_key = self.config.api_key.clone();
        let messages: Vec<ChatMessage> = self
            .chat
            .messages
            .iter()
            .map(|m| ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let provider = create_provider(&provider_name, api_key.as_deref())?;

        let rt = tokio::runtime::Runtime::new()?;
        let response =
            rt.block_on(async { provider.chat_with_history(&messages, &model, 0.7).await });

        match response {
            Ok(text) => {
                self.chat.streaming.finish();
                self.chat.add_assistant_message(text);
            }
            Err(e) => {
                self.chat.streaming.finish();
                self.chat.add_assistant_message(format!("Error: {}", e));
                self.show_notification(&format!("Error: {}", e));
            }
        }

        self.chat.loading = false;
        self.input_mode = InputMode::Normal;

        Ok(())
    }

    pub fn draw(&mut self, f: &mut Frame) {
        match self.view_mode {
            ViewMode::Single => self.draw_single(f),
            ViewMode::Split => self.draw_split(f),
            ViewMode::Triple => self.draw_triple(f),
        }

        if self.input_mode == InputMode::CommandPalette {
            self.command_palette.draw(f);
        }

        if self.help_visible {
            self.draw_help(f);
        }

        if let Some((msg, _)) = &self.notification {
            self.draw_notification(f, msg);
        }
    }

    fn draw_single(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),
                Constraint::Length(3),
                Constraint::Length(2),
            ])
            .split(f.area());

        self.draw_chat(f, chunks[0]);
        self.draw_input(f, chunks[1]);
        self.draw_status_bar(f, chunks[2]);
    }

    fn draw_split(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(f.area());

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),
                Constraint::Length(3),
                Constraint::Length(2),
            ])
            .split(chunks[0]);

        self.draw_chat(f, left_chunks[0]);
        self.draw_input(f, left_chunks[1]);
        self.draw_status_bar(f, left_chunks[2]);

        self.draw_side_panel(f, chunks[1]);
    }

    fn draw_triple(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Percentage(30),
                Constraint::Percentage(30),
            ])
            .split(f.area());

        let chat_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(3)])
            .split(chunks[0]);

        self.draw_chat(f, chat_chunks[0]);
        self.draw_input(f, chat_chunks[1]);

        self.draw_reasoning_panel(f, chunks[1]);
        self.draw_goals_panel(f, chunks[2]);
    }

    fn draw_chat(&self, f: &mut Frame, area: Rect) {
        let border_style = if self.active_panel == Panel::Chat {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::Gray)
        };

        let title = if self.chat.streaming.is_streaming {
            format!(
                " {} - {} (Streaming {} t/s) ",
                self.chat.provider_name,
                self.chat.model,
                self.chat.streaming.tokens_per_second() as i32
            )
        } else if self.chat.loading {
            format!(
                " {} - {} (Loading...) ",
                self.chat.provider_name, self.chat.model
            )
        } else {
            format!(" {} - {} ", self.chat.provider_name, self.chat.model)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(border_style);

        let inner = block.inner(area);
        f.render_widget(block, area);

        let visible_messages: Vec<_> = self
            .chat
            .messages
            .iter()
            .skip(self.chat.scroll_offset)
            .collect();

        let mut lines: Vec<Line> = Vec::new();

        for msg in visible_messages {
            let (prefix, prefix_style, content_style) = match msg.role.as_str() {
                "user" => (
                    format!("[{}] You: ", msg.timestamp),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                    Style::default().fg(Color::White),
                ),
                "assistant" => (
                    format!("[{}] 🤖: ", msg.timestamp),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                    Style::default().fg(Color::White),
                ),
                _ => (
                    format!("[{}] ⚙️: ", msg.timestamp),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                    Style::default().fg(Color::Gray),
                ),
            };

            lines.push(Line::from(vec![Span::styled(prefix, prefix_style)]));

            let content_lines = format_message_content(&msg.content);
            for line in content_lines {
                lines.push(Line::from(
                    line.spans
                        .into_iter()
                        .map(|s| Span::styled(s.content, content_style))
                        .collect::<Vec<_>>(),
                ));
            }

            lines.push(Line::from(""));
        }

        if self.chat.streaming.is_streaming {
            lines.push(Line::from(vec![
                Span::styled("⏳ ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    self.chat.streaming.current_response.clone(),
                    Style::default().fg(Color::White),
                ),
            ]));
        }

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });
        f.render_widget(paragraph, inner);

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        let mut scrollbar_state = ratatui::widgets::ScrollbarState::new(self.chat.messages.len())
            .position(self.chat.scroll_offset);

        f.render_stateful_widget(scrollbar, inner, &mut scrollbar_state);
    }

    fn draw_input(&self, f: &mut Frame, area: Rect) {
        let (input_style, title) = match self.input_mode {
            InputMode::Normal => (
                Style::default().fg(Color::Gray),
                " Press 'i' to type, Ctrl+P for commands, '?' for help ",
            ),
            InputMode::Editing => (
                Style::default().fg(Color::Yellow),
                " Type message (Enter=send, Esc=cancel, Ctrl+W=del word) ",
            ),
            InputMode::Search => (Style::default().fg(Color::Blue), " Search: "),
            InputMode::CommandPalette => (Style::default().fg(Color::Magenta), " Command Palette "),
        };

        let input_widget = Paragraph::new(self.input.as_str())
            .style(input_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .title_style(Style::default().fg(Color::Blue)),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(input_widget, area);
    }

    fn draw_status_bar(&self, f: &mut Frame, area: Rect) {
        let status = Line::from(vec![
            Span::styled(
                " Housaky v4 ",
                Style::default().bg(Color::Blue).fg(Color::White),
            ),
            Span::styled(
                format!(" {} msgs ", self.chat.messages.len()),
                Style::default().fg(Color::Gray),
            ),
            Span::styled(
                format!(" ~{} tok ", self.chat.total_tokens),
                Style::default().fg(Color::Gray),
            ),
            Span::styled(
                format!(" {:?} ", self.view_mode),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!(" {} ", self.active_panel.name()),
                Style::default().fg(Color::Cyan),
            ),
            if self.chat.auto_scroll {
                Span::styled(" 📜 ", Style::default().fg(Color::Green))
            } else {
                Span::styled(" 🔒 ", Style::default().fg(Color::Yellow))
            },
        ]);

        let status_bar = Paragraph::new(status);
        f.render_widget(status_bar, area);
    }

    fn draw_side_panel(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        self.draw_reasoning_panel(f, chunks[0]);
        self.draw_goals_panel(f, chunks[1]);
    }

    fn draw_reasoning_panel(&self, f: &mut Frame, area: Rect) {
        let border_style = if self.active_panel == Panel::Reasoning {
            Style::default().fg(Color::Magenta)
        } else {
            Style::default().fg(Color::Gray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" 🧠 Reasoning ")
            .title_style(
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(border_style);

        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();

        if let Some(ref chain) = self.current_reasoning {
            lines.push(Line::from(Span::styled(
                format!("Query: {}", chain.query),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));

            for step in &chain.steps {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{}. ", step.step_number),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled(&step.thought, Style::default().fg(Color::White)),
                ]));

                if let Some(ref action) = step.action {
                    lines.push(Line::from(vec![
                        Span::styled("   Action: ", Style::default().fg(Color::Green)),
                        Span::styled(action, Style::default().fg(Color::Gray)),
                    ]));
                }
            }

            if let Some(ref conclusion) = chain.conclusion {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("✓ {}", conclusion),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )));
            }
        } else {
            lines.push(Line::from(Span::styled(
                "No active reasoning chain",
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Press 'r' to start reasoning",
                Style::default().fg(Color::Gray),
            )));
        }

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });
        f.render_widget(paragraph, inner);
    }

    fn draw_goals_panel(&self, f: &mut Frame, area: Rect) {
        let border_style = if self.active_panel == Panel::Goals {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Gray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" 🎯 Goals ")
            .title_style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(border_style);

        let inner = block.inner(area);
        f.render_widget(block, area);

        let items: Vec<ListItem> = vec![
            ListItem::new(Line::from(Span::styled(
                "Implement streaming responses",
                Style::default().fg(Color::White),
            ))),
            ListItem::new(Line::from(Span::styled(
                "✓ Add reasoning engine",
                Style::default().fg(Color::Green),
            ))),
            ListItem::new(Line::from(Span::styled(
                "✓ Create goal management",
                Style::default().fg(Color::Green),
            ))),
            ListItem::new(Line::from(Span::styled(
                "Build knowledge graph",
                Style::default().fg(Color::Yellow),
            ))),
            ListItem::new(Line::from(Span::styled(
                "Add tool creation pipeline",
                Style::default().fg(Color::Gray),
            ))),
        ];

        let list = List::new(items);
        f.render_widget(list, inner);
    }

    fn draw_notification(&self, f: &mut Frame, msg: &str) {
        let area = Rect::new(
            f.area().width.saturating_sub(msg.len() as u16 + 4),
            2,
            msg.len() as u16 + 4,
            3,
        );

        let notification = Paragraph::new(msg)
            .style(Style::default().fg(Color::Black).bg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            );

        f.render_widget(Clear, area);
        f.render_widget(notification, area);
    }

    fn draw_help(&self, f: &mut Frame) {
        let help_text = r#"
╔══════════════════════════════════════════════════════════════════╗
║                    🤖 HOUSAKY AGI - Help                         ║
╠══════════════════════════════════════════════════════════════════╣
║                                                                  ║
║  Navigation                                                      ║
║  ────────────────────────────────────────────────────────────    ║
║  i/Enter    Start typing message                                 ║
║  Tab        Next panel                                           ║
║  Shift+Tab Previous panel                                        ║
║  v          Cycle view mode (Single → Split → Triple)            ║
║  ↑/↓        Scroll current panel                                 ║
║  PgUp/PgDn  Fast scroll                                          ║
║  Home/End   Scroll to top/bottom                                 ║
║                                                                  ║
║  Actions                                                         ║
║  ────────────────────────────────────────────────────────────    ║
║  Ctrl+P     Open command palette                                 ║
║  c          Copy last response                                   ║
║  s          Save conversation to file                            ║
║  Ctrl+U     Clear chat                                           ║
║  r          Focus reasoning panel                                ║
║  g          Focus goals panel                                    ║
║  q          Quit                                                 ║
║                                                                  ║
║  Command Palette (Ctrl+P)                                        ║
║  ────────────────────────────────────────────────────────────    ║
║  clear      Clear chat                                           ║
║  goal       Add new goal                                         ║
║  reflect    Trigger self-reflection                              ║
║  export     Export conversation                                  ║
║  view       Toggle view mode                                     ║
║                                                                  ║
║  Press any key to close                                          ║
╚══════════════════════════════════════════════════════════════════╝
"#;

        let area = Rect::new(
            (f.area().width.saturating_sub(70)) / 2,
            (f.area().height.saturating_sub(25)) / 2,
            70,
            25,
        );

        let help = Paragraph::new(help_text).style(Style::default().fg(Color::White));

        f.render_widget(Clear, area);
        f.render_widget(help, area);
    }
}

impl Panel {
    fn name(&self) -> &'static str {
        match self {
            Panel::Chat => "Chat",
            Panel::State => "State",
            Panel::Reasoning => "Reasoning",
            Panel::Goals => "Goals",
        }
    }
}

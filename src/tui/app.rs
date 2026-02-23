use crate::config::Config;
use crate::providers::{create_provider, ChatMessage};
use crate::tui::chat::{format_message_content, ChatState};
use crate::tui::help::HelpPopup;
use crate::tui::provider::ProviderState;
use crate::tui::search::SearchState;
use crate::tui::status_bar::StatusBar;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Chat,
    ProviderTest,
}

pub struct App {
    pub mode: AppMode,
    pub input_mode: InputMode,
    pub input: String,
    pub should_quit: bool,
    pub chat: Option<ChatState>,
    pub provider: Option<ProviderState>,
    pub config: Config,
    pub help_popup: HelpPopup,
    pub search_state: SearchState,
    pub status_bar: Option<StatusBar>,
    pub clipboard_content: Option<String>,
}

impl App {
    pub fn new_chat(config: Config, provider_name: String, model: String) -> Self {
        let status_bar = StatusBar::new(provider_name.clone(), model.clone());

        Self {
            mode: AppMode::Chat,
            input_mode: InputMode::Normal,
            input: String::new(),
            should_quit: false,
            chat: Some(ChatState::new(provider_name, model)),
            provider: None,
            config,
            help_popup: HelpPopup::new(),
            search_state: SearchState::new(),
            status_bar: Some(status_bar),
            clipboard_content: None,
        }
    }

    pub fn new_provider_test(config: Config) -> Self {
        Self {
            mode: AppMode::ProviderTest,
            input_mode: InputMode::Normal,
            input: String::new(),
            should_quit: false,
            chat: None,
            provider: Some(ProviderState::new()),
            config,
            help_popup: HelpPopup::new(),
            search_state: SearchState::new(),
            status_bar: None,
            clipboard_content: None,
        }
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn should_exit_on_ctrl_c(&self) -> bool {
        self.input_mode == InputMode::Normal
            && !self.help_popup.visible
            && !self.search_state.active
    }

    pub fn should_exit_on_esc(&self) -> bool {
        if self.help_popup.visible {
            return false; // Let handle_key close the popup
        }
        if self.search_state.active {
            return false; // Let handle_key close search
        }
        self.input_mode == InputMode::Normal
    }

    pub fn update(&mut self) {
        if let Some(ref mut status_bar) = self.status_bar {
            status_bar.update();
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Handle help popup first
        if self.help_popup.visible {
            self.help_popup.hide();
            return Ok(());
        }

        match self.mode {
            AppMode::Chat => self.handle_chat_key(key),
            AppMode::ProviderTest => self.handle_provider_key(key),
        }
    }

    fn handle_chat_key(&mut self, key: KeyEvent) -> Result<()> {
        // Handle search mode
        if self.search_state.active {
            return self.handle_search_key(key);
        }

        match self.input_mode {
            InputMode::Normal => self.handle_normal_key(key),
            InputMode::Editing => self.handle_editing_key(key),
            InputMode::Search => self.handle_search_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        match (key.modifiers, key.code) {
            // Navigation
            (KeyModifiers::NONE, KeyCode::Char('i') | KeyCode::Enter) => {
                self.input_mode = InputMode::Editing;
            }
            (KeyModifiers::NONE, KeyCode::Char('q')) => {
                self.should_quit = true;
            }
            (KeyModifiers::NONE, KeyCode::Up) => {
                if let Some(ref mut chat) = self.chat {
                    chat.scroll_up();
                    self.update_scroll_status();
                }
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                if let Some(ref mut chat) = self.chat {
                    chat.scroll_down();
                    self.update_scroll_status();
                }
            }
            (KeyModifiers::NONE, KeyCode::PageUp) => {
                if let Some(ref mut chat) = self.chat {
                    let page_size = 10;
                    chat.scroll_page_up(page_size);
                    self.update_scroll_status();
                }
            }
            (KeyModifiers::NONE, KeyCode::PageDown) => {
                if let Some(ref mut chat) = self.chat {
                    let page_size = 10;
                    chat.scroll_page_down(page_size);
                    self.update_scroll_status();
                }
            }
            (KeyModifiers::NONE, KeyCode::Home) => {
                if let Some(ref mut chat) = self.chat {
                    chat.scroll_to_top();
                    self.update_scroll_status();
                }
            }
            (KeyModifiers::NONE, KeyCode::End) => {
                if let Some(ref mut chat) = self.chat {
                    chat.scroll_to_bottom();
                    self.update_scroll_status();
                }
            }

            // Features
            (KeyModifiers::NONE, KeyCode::Char('?') | KeyCode::F(1)) => {
                self.help_popup.show();
            }
            (KeyModifiers::NONE, KeyCode::Char('/')) => {
                self.search_state.activate();
                self.input_mode = InputMode::Search;
            }
            (KeyModifiers::NONE, KeyCode::Char('a')) => {
                if let Some(ref mut chat) = self.chat {
                    chat.toggle_auto_scroll();
                    self.update_scroll_status();
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('c')) => {
                self.copy_last_response()?;
            }
            (KeyModifiers::NONE, KeyCode::Char('s')) => {
                self.save_conversation()?;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                if let Some(ref mut chat) = self.chat {
                    chat.clear_messages();
                    self.update_status_bar();
                }
            }

            // Search navigation
            (KeyModifiers::NONE, KeyCode::Char('n')) => {
                self.goto_next_search_result();
            }
            (KeyModifiers::SHIFT, KeyCode::Char('N')) => {
                self.goto_previous_search_result();
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
            (KeyModifiers::NONE, KeyCode::Delete) => {
                self.input.clear();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> Result<()> {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.search_state.deactivate();
                self.input_mode = InputMode::Normal;
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                self.perform_search();
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                self.search_state.pop_char();
                self.perform_search();
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) => {
                self.search_state.push_char(c);
                self.perform_search();
            }
            _ => {}
        }
        Ok(())
    }

    fn perform_search(&mut self) {
        if let Some(ref chat) = self.chat {
            if !self.search_state.is_empty() {
                let results = chat.search_messages(&self.search_state.query);
                self.search_state.update_results(results);
            }
        }
    }

    fn goto_next_search_result(&mut self) {
        if let Some(message_id) = self.search_state.next_result() {
            self.scroll_to_message(message_id);
        }
    }

    fn goto_previous_search_result(&mut self) {
        if let Some(message_id) = self.search_state.previous_result() {
            self.scroll_to_message(message_id);
        }
    }

    fn scroll_to_message(&mut self, message_id: usize) {
        if let Some(ref mut chat) = self.chat {
            if let Some(index) = chat.messages.iter().position(|m| m.id == message_id) {
                chat.scroll_offset = index;
                chat.auto_scroll = false;
                self.update_scroll_status();
            }
        }
    }

    fn update_scroll_status(&mut self) {
        if let Some(ref mut status_bar) = self.status_bar {
            if let Some(ref chat) = self.chat {
                status_bar.set_auto_scroll(chat.auto_scroll);
            }
        }
    }

    fn update_status_bar(&mut self) {
        if let Some(ref mut status_bar) = self.status_bar {
            if let Some(ref chat) = self.chat {
                status_bar.set_message_count(chat.messages.len());
                status_bar.set_loading(chat.loading);
            }
        }
    }

    fn send_message(&mut self) -> Result<()> {
        let message = self.input.clone();
        self.input.clear();

        if let Some(ref mut chat) = self.chat {
            chat.add_user_message(message.clone());
            chat.set_loading(true);
        }

        // Update status bar outside the borrow
        self.update_status_bar();

        // Clone all needed data before the API call
        let (provider_name, model, api_key, messages) = if let Some(ref chat) = self.chat {
            (
                chat.provider_name.clone(),
                chat.model.clone(),
                self.config.api_key.clone(),
                chat.messages.clone(),
            )
        } else {
            return Ok(());
        };

        let provider = create_provider(&provider_name, api_key.as_deref())?;

        // Use the current tokio runtime if available, otherwise create a new one
        let response = match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                // We're already in a tokio runtime, use block_in_place to avoid nested block_on
                tokio::task::block_in_place(|| {
                    handle.block_on(async {
                        let chat_messages: Vec<ChatMessage> = messages
                            .iter()
                            .map(|m| ChatMessage {
                                role: m.role.clone(),
                                content: m.content.clone(),
                            })
                            .collect();
                        provider
                            .chat_with_history(&chat_messages, &model, 0.7)
                            .await
                    })
                })
            }
            Err(_) => {
                // No runtime available, create a new one
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async {
                    let chat_messages: Vec<ChatMessage> = messages
                        .iter()
                        .map(|m| ChatMessage {
                            role: m.role.clone(),
                            content: m.content.clone(),
                        })
                        .collect();
                    provider
                        .chat_with_history(&chat_messages, &model, 0.7)
                        .await
                })
            }
        };

        // Handle response
        if let Some(ref mut chat) = self.chat {
            match response {
                Ok(text) => {
                    chat.add_assistant_message(text);
                }
                Err(e) => {
                    let error_msg = format!("Error: {e}");
                    chat.add_system_message(error_msg.clone());
                    if let Some(ref mut status_bar) = self.status_bar {
                        status_bar.set_error(error_msg);
                    }
                }
            }
            chat.set_loading(false);
        }

        self.update_status_bar();

        Ok(())
    }

    fn copy_last_response(&mut self) -> Result<()> {
        if let Some(ref chat) = self.chat {
            if let Some(last_msg) = chat.get_last_assistant_message() {
                self.clipboard_content = Some(last_msg.content.clone());
                if let Some(ref mut status_bar) = self.status_bar {
                    // Note: Actual clipboard integration would require additional crates
                    status_bar
                        .set_error("Response copied (simulated - use 's' to save)".to_string());
                }
            }
        }
        Ok(())
    }

    fn save_conversation(&self) -> Result<String> {
        if let Some(ref chat) = self.chat {
            let export = chat.export_to_string();
            let filename = format!(
                "housaky_chat_{}.txt",
                chrono::Local::now().format("%Y%m%d_%H%M%S")
            );

            match std::fs::write(&filename, export) {
                Ok(()) => Ok(filename),
                Err(e) => Err(anyhow::anyhow!("Failed to save conversation: {}", e)),
            }
        } else {
            Err(anyhow::anyhow!("No chat to save"))
        }
    }

    fn handle_provider_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.input_mode {
            InputMode::Normal => self.handle_provider_normal_key(key),
            InputMode::Editing => self.handle_provider_editing_key(key),
            _ => Ok(()),
        }
    }

    fn handle_provider_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Char('q')) => {
                self.should_quit = true;
            }
            (KeyModifiers::NONE, KeyCode::Up) => {
                if let Some(ref mut provider) = self.provider {
                    provider.select_previous();
                }
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                if let Some(ref mut provider) = self.provider {
                    provider.select_next();
                }
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                let selected = self
                    .provider
                    .as_ref()
                    .and_then(|p| p.get_selected().cloned());
                if let Some(selected) = selected {
                    self.test_provider(&selected)?;
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('e')) => {
                self.input_mode = InputMode::Editing;
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.refresh_provider_list();
            }
            (KeyModifiers::NONE, KeyCode::Char('?') | KeyCode::F(1)) => {
                self.help_popup.show();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_provider_editing_key(&mut self, key: KeyEvent) -> Result<()> {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Enter) => {
                if !self.input.is_empty() {
                    if let Some(ref mut provider) = self.provider {
                        provider.add_custom_provider(self.input.clone());
                    }
                    self.input.clear();
                }
                self.input_mode = InputMode::Normal;
            }
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.input_mode = InputMode::Normal;
                self.input.clear();
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) => {
                self.input.push(c);
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                self.input.pop();
            }
            _ => {}
        }
        Ok(())
    }

    fn test_provider(&mut self, provider_name: &str) -> Result<()> {
        if let Some(ref mut provider_state) = self.provider {
            provider_state.set_testing(true);
            provider_state.set_result(None);

            let api_key = self.config.api_key.clone();
            let provider = create_provider(provider_name, api_key.as_deref())?;

            // Use the current tokio runtime if available, otherwise create a new one
            let result = match tokio::runtime::Handle::try_current() {
                Ok(handle) => {
                    // We're already in a tokio runtime, use block_in_place
                    tokio::task::block_in_place(|| {
                        handle.block_on(async {
                            let warmup_result = provider.warmup().await;
                            if warmup_result.is_err() {
                                return warmup_result.map(|()| "Warmup successful".to_string());
                            }
                            provider
                                .simple_chat("Say 'Hello' in one word", "auto", 0.1)
                                .await
                        })
                    })
                }
                Err(_) => {
                    // No runtime available, create a new one
                    let rt = tokio::runtime::Runtime::new()?;
                    rt.block_on(async {
                        let warmup_result = provider.warmup().await;
                        if warmup_result.is_err() {
                            return warmup_result.map(|()| "Warmup successful".to_string());
                        }
                        provider
                            .simple_chat("Say 'Hello' in one word", "auto", 0.1)
                            .await
                    })
                }
            };

            match result {
                Ok(response) => {
                    provider_state.set_result(Some(format!("Success: {response}")));
                }
                Err(e) => {
                    provider_state.set_result(Some(format!("Failed: {e}")));
                }
            }
            provider_state.set_testing(false);
        }

        Ok(())
    }

    fn refresh_provider_list(&mut self) {
        if let Some(ref mut provider) = self.provider {
            provider.refresh();
        }
    }

    pub fn draw(&mut self, f: &mut Frame) {
        match self.mode {
            AppMode::Chat => self.draw_chat(f),
            AppMode::ProviderTest => self.draw_provider(f),
        }

        // Draw help popup on top if visible
        self.help_popup.draw(f);
    }

    fn draw_chat(&mut self, f: &mut Frame) {
        let area = f.area();

        // Main layout with status bar at bottom
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),
                Constraint::Length(3), // Input area
                Constraint::Length(3), // Status bar
            ])
            .split(area);

        // Split main area into messages and input
        let content_area = main_chunks[0];
        let input_area = main_chunks[1];
        let status_area = main_chunks[2];

        // Draw messages area
        self.draw_messages(f, content_area);

        // Draw search overlay if active
        if self.search_state.active {
            let search_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(3)])
                .split(content_area);
            self.search_state.draw(f, search_area[1]);
        }

        // Draw input area
        self.draw_input(f, input_area);

        // Draw status bar
        if let Some(ref mut status_bar) = self.status_bar {
            if let Some(ref chat) = self.chat {
                status_bar.set_message_count(chat.messages.len());
                status_bar.set_loading(chat.loading);
            }
            status_bar.draw(f, status_area);
        }
    }

    fn draw_messages(&self, f: &mut Frame, area: Rect) {
        if let Some(ref chat) = self.chat {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(if chat.loading {
                    format!(" {} - {} (Loading...)", chat.provider_name, chat.model)
                } else {
                    format!(" {} - {} ", chat.provider_name, chat.model)
                })
                .title_style(Style::default().fg(Color::Yellow));

            let inner_area = block.inner(area);
            f.render_widget(block, area);

            // Calculate visible messages based on scroll offset
            let visible_messages: Vec<_> = chat.messages.iter().skip(chat.scroll_offset).collect();

            let mut text_lines: Vec<Line> = Vec::new();

            for msg in visible_messages {
                // Determine style based on role
                let (prefix, prefix_style, content_style) = match msg.role.as_str() {
                    "user" => (
                        format!("[{}] You: ", msg.timestamp),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                        Style::default().fg(Color::White),
                    ),
                    "assistant" => (
                        format!("[{}] AI: ", msg.timestamp),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                        Style::default().fg(Color::White),
                    ),
                    _ => (
                        format!("[{}] System: ", msg.timestamp),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                        Style::default().fg(Color::Gray),
                    ),
                };

                // Check if this message is a search result
                let is_search_result = self.search_state.has_results()
                    && self.search_state.get_current_result() == Some(msg.id);

                // Add message header
                let header_style = if is_search_result {
                    prefix_style.add_modifier(Modifier::REVERSED)
                } else {
                    prefix_style
                };

                text_lines.push(Line::from(vec![Span::styled(prefix, header_style)]));

                // Format message content with markdown support
                let content_lines = format_message_content(&msg.content);
                for line in content_lines {
                    let styled_line = Line::from(
                        line.spans
                            .into_iter()
                            .map(|span| {
                                let new_style = if is_search_result {
                                    span.style.add_modifier(Modifier::REVERSED)
                                } else {
                                    content_style
                                };
                                Span::styled(span.content.to_string(), new_style)
                            })
                            .collect::<Vec<_>>(),
                    );
                    text_lines.push(styled_line);
                }

                // Add empty line between messages
                text_lines.push(Line::from(""));
            }

            let messages_widget = Paragraph::new(text_lines).wrap(Wrap { trim: true });

            f.render_widget(messages_widget, inner_area);

            // Render scrollbar
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            let mut scrollbar_state = ratatui::widgets::ScrollbarState::new(chat.messages.len())
                .position(chat.scroll_offset);

            f.render_stateful_widget(scrollbar, inner_area, &mut scrollbar_state);
        }
    }

    fn draw_input(&self, f: &mut Frame, area: Rect) {
        let input_style = match self.input_mode {
            InputMode::Normal => Style::default().fg(Color::Gray),
            InputMode::Editing => Style::default().fg(Color::Yellow),
            InputMode::Search => Style::default().fg(Color::Gray),
        };

        let title = match self.input_mode {
            InputMode::Normal => " Press 'i' to type, 'q' to quit, '?' for help ",
            InputMode::Editing => " Type your message (Enter to send, Esc to cancel) ",
            InputMode::Search => " Searching... ",
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

    fn draw_provider(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(5),
            ])
            .split(f.area());

        let title = Paragraph::new("Provider Test Mode")
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Housaky TUI ")
                    .title_style(Style::default().fg(Color::Magenta)),
            );

        f.render_widget(title, chunks[0]);

        if let Some(ref provider_state) = self.provider {
            let providers: Vec<ListItem> = provider_state
                .providers
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let style = if i == provider_state.selected {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ListItem::new(Line::from(Span::styled(p, style)))
                })
                .collect();

            let list = List::new(providers)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Providers (Up/Down to select, Enter to test, 'e' to add custom, 'r' to refresh, '?' for help) ")
                        .title_style(Style::default().fg(Color::Blue)),
                )
                .style(Style::default().fg(Color::White));

            f.render_widget(list, chunks[1]);

            let result_text = if provider_state.testing {
                "Testing...".to_string()
            } else if let Some(ref result) = provider_state.result {
                result.clone()
            } else {
                "Press Enter to test selected provider".to_string()
            };

            let result_style = if provider_state
                .result
                .as_ref()
                .map_or(false, |r| r.starts_with("Success"))
            {
                Style::default().fg(Color::Green)
            } else if provider_state
                .result
                .as_ref()
                .map_or(false, |r| r.starts_with("Failed"))
            {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::White)
            };

            let result = Paragraph::new(result_text).style(result_style).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Result ")
                    .title_style(Style::default().fg(Color::Green)),
            );

            f.render_widget(result, chunks[2]);
        }

        if self.input_mode == InputMode::Editing {
            let input_widget = Paragraph::new(self.input.as_str())
                .style(Style::default().fg(Color::Yellow))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Enter provider name or custom:URL (Enter to add, Esc to cancel) ")
                        .title_style(Style::default().fg(Color::Blue)),
                );
            let area = Rect::new(
                chunks[1].width / 4,
                chunks[1].height / 2,
                chunks[1].width / 2,
                3,
            );
            f.render_widget(Clear, area);
            f.render_widget(input_widget, area);
        }
    }
}

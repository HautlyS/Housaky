//! Minimal Chat Panel
//!
//! Clean chat interface with message history and streaming support.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};
use std::time::Instant;

use super::agents::AgentType;
use super::theme::{self, Theme};

/// Message role
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
    Agent(AgentType),
}

impl Role {
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::User => "YOU",
            Self::Assistant => "HOUSAKY",
            Self::System => "SYSTEM",
            Self::Agent(t) => t.name(),
        }
    }

    pub fn style(&self) -> Style {
        match self {
            Self::User => theme::style_user_message(),
            Self::Assistant => theme::style_assistant_message(),
            Self::System => theme::style_system_message(),
            Self::Agent(t) => t.style(),
        }
    }
}

/// Single chat message
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub timestamp: Instant,
    pub streaming: bool,
}

impl Message {
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            timestamp: Instant::now(),
            streaming: false,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }

    pub fn agent(agent_type: AgentType, content: impl Into<String>) -> Self {
        Self::new(Role::Agent(agent_type), content)
    }

    pub fn streaming(role: Role) -> Self {
        Self {
            role,
            content: String::new(),
            timestamp: Instant::now(),
            streaming: true,
        }
    }

    pub fn append(&mut self, chunk: &str) {
        self.content.push_str(chunk);
    }

    pub fn finish(&mut self) {
        self.streaming = false;
    }
}

/// Chat panel state
pub struct ChatPanel {
    pub messages: Vec<Message>,
    pub scroll_offset: usize,
    pub auto_scroll: bool,
    pub streaming: bool,
}

impl ChatPanel {
    pub fn new() -> Self {
        Self {
            messages: vec![Message::system(
                "HOUSAKY v0.1.0 | Type /help for commands | Ctrl+K for keys",
            )],
            scroll_offset: 0,
            auto_scroll: true,
            streaming: false,
        }
    }

    pub fn push(&mut self, message: Message) {
        self.messages.push(message);
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn push_user(&mut self, content: &str) {
        self.push(Message::user(content));
    }

    pub fn push_assistant(&mut self, content: &str) {
        self.push(Message::assistant(content));
    }

    pub fn push_system(&mut self, content: &str) {
        self.push(Message::system(content));
    }

    pub fn push_agent(&mut self, agent_type: AgentType, content: &str) {
        self.push(Message::agent(agent_type, content));
    }

    pub fn start_streaming(&mut self, role: Role) {
        self.streaming = true;
        self.push(Message::streaming(role));
    }

    pub fn append_stream(&mut self, chunk: &str) {
        if let Some(msg) = self.messages.last_mut() {
            if msg.streaming {
                msg.append(chunk);
                if self.auto_scroll {
                    self.scroll_to_bottom();
                }
            }
        }
    }

    pub fn finish_streaming(&mut self) {
        self.streaming = false;
        if let Some(msg) = self.messages.last_mut() {
            msg.finish();
        }
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
        self.auto_scroll = false;
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
        if self.scroll_offset == 0 {
            self.auto_scroll = true;
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = true;
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = self.messages.len().saturating_sub(1);
        self.auto_scroll = false;
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
        self.auto_scroll = true;
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            theme::style_border_focus()
        } else {
            theme::style_border()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(theme::style_base());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.messages.is_empty() {
            let empty = Paragraph::new("No messages yet. Start chatting!")
                .style(theme::style_dim())
                .alignment(Alignment::Center);
            frame.render_widget(empty, inner);
            return;
        }

        // Build message lines
        let mut lines: Vec<Line> = Vec::new();
        for msg in &self.messages {
            // Role header
            let header_spans = vec![
                Span::styled(
                    format!("[{}]", msg.role.prefix()),
                    msg.role.style().add_modifier(Modifier::BOLD),
                ),
                if msg.streaming {
                    Span::styled(" ...", theme::style_warning())
                } else {
                    Span::raw("")
                },
            ];
            lines.push(Line::from(header_spans));

            // Content lines
            for line in msg.content.lines() {
                lines.push(Line::from(Span::styled(
                    format!("  {}", line),
                    theme::style_base(),
                )));
            }

            // Empty line between messages
            lines.push(Line::from(""));
        }

        // Calculate visible portion
        let visible_height = inner.height as usize;
        let total_lines = lines.len();
        let start = if self.auto_scroll {
            total_lines.saturating_sub(visible_height)
        } else {
            total_lines.saturating_sub(visible_height).saturating_sub(self.scroll_offset)
        };
        let end = (start + visible_height).min(total_lines);

        let visible_lines: Vec<Line> = lines[start..end].to_vec();
        let text = Text::from(visible_lines);

        let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });

        frame.render_widget(paragraph, inner);

        // Scrollbar
        if total_lines > visible_height {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .style(theme::style_border());

            let mut scrollbar_state = ScrollbarState::new(total_lines)
                .position(start)
                .viewport_content_length(visible_height);

            frame.render_stateful_widget(
                scrollbar,
                area.inner(ratatui::layout::Margin { horizontal: 0, vertical: 1 }),
                &mut scrollbar_state,
            );
        }
    }
}

impl Default for ChatPanel {
    fn default() -> Self {
        Self::new()
    }
}

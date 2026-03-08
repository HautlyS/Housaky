//! Input Bar Component
//!
//! Minimal text input with cursor support and command autocomplete.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::command_palette::CommandPalette;
use super::theme::{self, PsychedelicBg};

/// Input bar state
pub struct InputBar {
    pub content: String,
    pub cursor: usize,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
    pub placeholder: String,
    pub command_palette: CommandPalette,
    pub psych_bg: PsychedelicBg,
}

impl InputBar {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor: 0,
            history: Vec::new(),
            history_index: None,
            placeholder: "Type a message... (/ for commands, Ctrl+K for keys)".into(),
            command_palette: CommandPalette::new(),
            psych_bg: PsychedelicBg::new(),
        }
    }

    pub fn insert(&mut self, c: char) {
        self.content.insert(self.cursor, c);
        self.cursor += 1;
        self.history_index = None;

        if self.content.starts_with('/') {
            self.command_palette.show();
            self.command_palette.update_query(&self.content);
        }
    }

    pub fn insert_str(&mut self, s: &str) {
        for c in s.chars() {
            self.insert(c);
        }
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.content.remove(self.cursor);
        }

        if self.content.starts_with('/') {
            self.command_palette.update_query(&self.content);
        } else {
            self.command_palette.hide();
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.content.len() {
            self.content.remove(self.cursor);
        }
    }

    pub fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.content.len());
    }

    pub fn move_start(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.content.len();
    }

    pub fn move_word_left(&mut self) {
        while self.cursor > 0 {
            self.cursor -= 1;
            if self.cursor == 0
                || self
                    .content
                    .chars()
                    .nth(self.cursor - 1)
                    .map_or(false, |c| c.is_whitespace())
            {
                break;
            }
        }
    }

    pub fn move_word_right(&mut self) {
        let len = self.content.len();
        while self.cursor < len {
            self.cursor += 1;
            if self.cursor == len
                || self
                    .content
                    .chars()
                    .nth(self.cursor)
                    .map_or(false, |c| c.is_whitespace())
            {
                break;
            }
        }
    }

    pub fn delete_word(&mut self) {
        let end = self.cursor;
        self.move_word_left();
        let start = self.cursor;
        if start < end {
            self.content.drain(start..end);
        }
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
        self.history_index = None;
        self.command_palette.hide();
    }

    pub fn take(&mut self) -> String {
        let content = std::mem::take(&mut self.content);
        self.cursor = 0;
        self.history_index = None;
        self.command_palette.hide();
        if !content.trim().is_empty() {
            self.history.push(content.clone());
        }
        content
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }

        match self.history_index {
            None => {
                self.history_index = Some(self.history.len() - 1);
            }
            Some(i) if i > 0 => {
                self.history_index = Some(i - 1);
            }
            _ => {}
        }

        if let Some(i) = self.history_index {
            self.content = self.history[i].clone();
            self.cursor = self.content.len();
        }
    }

    pub fn history_down(&mut self) {
        match self.history_index {
            Some(i) if i + 1 < self.history.len() => {
                self.history_index = Some(i + 1);
                self.content = self.history[i + 1].clone();
                self.cursor = self.content.len();
            }
            Some(_) => {
                self.history_index = None;
                self.clear();
            }
            None => {}
        }
    }

    pub fn update(&mut self) {
        self.psych_bg.update();
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            theme::style_border_active()
        } else {
            theme::style_border()
        };

        let accent_color = self.psych_bg.get_accent_color();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(if focused {
                Style::default().fg(accent_color)
            } else {
                border_style
            })
            .style(theme::style_input());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.content.is_empty() && !focused {
            let placeholder = Paragraph::new(Span::styled(&self.placeholder, theme::style_dim()));
            frame.render_widget(placeholder, inner);
            return;
        }

        let visible_width = inner.width as usize;
        let content_len = self.content.len();

        let cursor_offset = self.cursor;
        let start = if cursor_offset >= visible_width {
            cursor_offset - visible_width + 1
        } else {
            0
        };
        let end = (start + visible_width).min(content_len);

        let mut spans = Vec::new();

        if start < self.cursor {
            let before = &self.content[start..self.cursor];
            if self.content.starts_with('/') {
                spans.push(Span::styled(
                    before,
                    Style::default().fg(theme::Theme::CYAN),
                ));
            } else {
                spans.push(Span::styled(before, theme::style_input()));
            }
        }

        if focused {
            let cursor_char = self.content.chars().nth(self.cursor).unwrap_or(' ');
            spans.push(Span::styled(
                cursor_char.to_string(),
                Style::default().fg(theme::Theme::BG).bg(accent_color),
            ));
        }

        let after_start = self.cursor + 1;
        if after_start <= end && after_start <= content_len {
            let after_end = end.min(content_len);
            if after_start < after_end {
                let after = &self.content[after_start..after_end];
                spans.push(Span::styled(after, theme::style_input()));
            }
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, inner);

        if self.command_palette.visible {
            self.command_palette.draw(frame, area);
        }
    }
}

impl Default for InputBar {
    fn default() -> Self {
        Self::new()
    }
}

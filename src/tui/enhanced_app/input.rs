use crate::tui::enhanced_app::state::InputMode;
use crate::tui::enhanced_app::theme::{
    style_border, style_border_focus, style_dim, style_input_focused, style_input_idle,
    style_muted, style_title, Palette,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

// ── Slash command hint ────────────────────────────────────────────────────────

static SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/clear", "Clear conversation"),
    ("/export", "Export chat to markdown"),
    ("/model", "Switch model  e.g. /model gpt-4o"),
    ("/goals", "Switch to Goals tab"),
    ("/skills", "Switch to Skills tab"),
    ("/tools", "Switch to Tools tab"),
    ("/metrics", "Switch to Metrics tab"),
    ("/reflect", "Trigger AGI self-reflection"),
    ("/help", "Show keyboard help"),
    ("/quit", "Exit Housaky TUI"),
];

// ── Input state ───────────────────────────────────────────────────────────────

pub struct InputBar {
    pub text: String,
    pub history: Vec<String>,
    pub history_idx: usize, // 0 = not browsing history
    pub cursor_pos: usize,  // byte offset in text
    pub mode: InputMode,
    pub multiline: bool, // Shift+Enter inserts newline
    pub char_count: usize,
    pub max_chars: usize,
}

impl InputBar {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            history: Vec::new(),
            history_idx: 0,
            cursor_pos: 0,
            mode: InputMode::Insert,
            multiline: false,
            char_count: 0,
            max_chars: 4000,
        }
    }

    // ── Text operations ───────────────────────────────────────────────────────

    pub fn push_char(&mut self, c: char) {
        if self.char_count >= self.max_chars {
            return;
        }
        self.text.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
        self.char_count = self.text.chars().count();
        self.history_idx = 0;
    }

    pub fn backspace(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let prev = self.text[..self.cursor_pos]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.text.remove(prev);
        self.cursor_pos = prev;
        self.char_count = self.text.chars().count();
    }

    pub fn delete_forward(&mut self) {
        if self.cursor_pos >= self.text.len() {
            return;
        }
        self.text.remove(self.cursor_pos);
        self.char_count = self.text.chars().count();
    }

    pub fn move_cursor_left(&mut self) {
        self.cursor_pos = self.text[..self.cursor_pos]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.text.len() {
            let c = self.text[self.cursor_pos..].chars().next().unwrap();
            self.cursor_pos += c.len_utf8();
        }
    }

    pub fn move_cursor_home(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.cursor_pos = self.text.len();
    }

    pub fn kill_line(&mut self) {
        self.text.truncate(self.cursor_pos);
        self.char_count = self.text.chars().count();
    }

    pub fn kill_word_back(&mut self) {
        let before = &self.text[..self.cursor_pos];
        let trimmed = before.trim_end();
        let new_pos = trimmed.rfind(' ').map(|i| i + 1).unwrap_or(0);
        self.text.replace_range(new_pos..self.cursor_pos, "");
        self.cursor_pos = new_pos;
        self.char_count = self.text.chars().count();
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor_pos = 0;
        self.char_count = 0;
        self.history_idx = 0;
    }

    pub fn take(&mut self) -> String {
        let msg = self.text.trim().to_string();
        if !msg.is_empty() {
            self.history.push(msg.clone());
        }
        self.clear();
        msg
    }

    // ── History ───────────────────────────────────────────────────────────────

    pub fn history_prev(&mut self) {
        let len = self.history.len();
        if len == 0 {
            return;
        }
        if self.history_idx < len {
            self.history_idx += 1;
        }
        if let Some(entry) = self.history.iter().rev().nth(self.history_idx - 1) {
            self.text = entry.clone();
            self.cursor_pos = self.text.len();
            self.char_count = self.text.chars().count();
        }
    }

    pub fn history_next(&mut self) {
        if self.history_idx == 0 {
            return;
        }
        self.history_idx -= 1;
        if self.history_idx == 0 {
            self.clear();
        } else if let Some(entry) = self.history.iter().rev().nth(self.history_idx - 1) {
            self.text = entry.clone();
            self.cursor_pos = self.text.len();
            self.char_count = self.text.chars().count();
        }
    }

    // ── Introspection ─────────────────────────────────────────────────────────

    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }

    pub fn is_command(&self) -> bool {
        self.text.starts_with('/')
    }

    pub fn command_completions(&self) -> Vec<(&'static str, &'static str)> {
        if !self.is_command() {
            return vec![];
        }
        let q = self.text.to_lowercase();
        SLASH_COMMANDS
            .iter()
            .filter(|(cmd, _)| cmd.starts_with(q.as_str()))
            .copied()
            .collect()
    }

    pub fn is_near_limit(&self) -> bool {
        self.char_count > (self.max_chars * 80 / 100)
    }

    fn text_lines(&self, width: u16) -> usize {
        if self.text.is_empty() {
            return 1;
        }
        let inner_width = width.saturating_sub(4) as usize;
        if inner_width == 0 {
            return 1;
        }
        let mut lines = 1;
        let mut current_line_width = 0;
        for c in self.text.chars() {
            let char_width = if c.is_ascii() { 1 } else { 2 };
            current_line_width += char_width;
            if current_line_width >= inner_width {
                lines += 1;
                current_line_width = 0;
            }
        }
        lines
    }

    fn cursor_line_and_column(&self) -> (usize, usize) {
        let width: u16 = 80;
        let inner_width = width.saturating_sub(4) as usize;
        if inner_width == 0 {
            return (0, 0);
        }
        let mut line = 0;
        let mut col = 0;
        let mut current_line_width = 0;
        for (i, c) in self.text.chars().enumerate() {
            if i >= self.cursor_pos {
                break;
            }
            let char_width = if c.is_ascii() { 1 } else { 2 };
            current_line_width += char_width;
            if current_line_width >= inner_width {
                line += 1;
                current_line_width = 0;
                col = 0;
            } else {
                col += char_width;
            }
        }
        (line, col)
    }

    // ── Draw ──────────────────────────────────────────────────────────────────

    pub fn draw(&self, f: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            style_border_focus()
        } else {
            style_border()
        };

        // Show slash-command autocomplete strip above input if relevant
        let completions = self.command_completions();
        let show_completions = focused && !completions.is_empty() && completions.len() <= 6;

        let (comp_area, input_area) = if show_completions {
            let vt = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(2)])
                .split(area);
            (Some(vt[0]), vt[1])
        } else {
            (None, area)
        };

        // Draw completions bar
        if let Some(ca) = comp_area {
            let mut spans: Vec<Span> = vec![Span::styled(" ↵ ", style_muted())];
            for (i, (cmd, desc)) in completions.iter().enumerate() {
                if i > 0 {
                    spans.push(Span::styled("  ", style_muted()));
                }
                spans.push(Span::styled(
                    *cmd,
                    style_input_focused().add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(format!(" {}", desc), style_muted()));
            }
            f.render_widget(Paragraph::new(Line::from(spans)), ca);
        }

        // Character count indicator
        let count_str = if self.is_near_limit() {
            format!(" {}/{} ", self.char_count, self.max_chars)
        } else {
            String::new()
        };

        let mode_str = match self.mode {
            InputMode::Insert => "INSERT",
            InputMode::Normal => "NORMAL",
            InputMode::Command => "CMD",
            InputMode::Search => "SEARCH",
        };

        let title_style = if focused { style_title() } else { style_dim() };
        let title = format!(" ◉ {} {} ", mode_str, count_str);

        let hint = if self.is_command() {
            "Enter=send  Esc=cancel  Tab=complete"
        } else if focused {
            "Enter=send  ↑↓=history  Ctrl+P=palette  Esc=blur"
        } else {
            "Press i to type  Ctrl+P=palette  ?=help"
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(title, title_style))
            .title_bottom(Span::styled(format!(" {} ", hint), style_muted()));

        let inner = block.inner(input_area);
        f.render_widget(block, input_area);

        // Calculate required height based on text content
        let text_lines = self.text_lines(inner.width);
        let min_height = 1;
        let max_height = inner.height.saturating_sub(1).max(min_height);
        let required_height = (text_lines as u16).min(max_height).max(min_height);

        // Render text with cursor
        let display_text = if self.text.is_empty() && !focused {
            Span::styled("  Type a message or /command…", style_muted())
        } else {
            let text_style = if focused {
                style_input_focused()
            } else {
                style_input_idle()
            };
            Span::styled(self.text.clone(), text_style)
        };

        let text_with_cursor = if focused {
            let before = &self.text[..self.cursor_pos];
            let cursor_char = self.text[self.cursor_pos..]
                .chars()
                .next()
                .map(|c| c.to_string())
                .unwrap_or_else(|| " ".to_string());
            let after = if self.cursor_pos < self.text.len() {
                let skip = cursor_char.len();
                self.text[self.cursor_pos + skip..].to_string()
            } else {
                String::new()
            };

            let command_hint_color = if self.is_command() {
                Palette::CYAN
            } else {
                Palette::TEXT_BRIGHT
            };

            Line::from(vec![
                Span::styled(
                    before.to_string(),
                    ratatui::style::Style::default().fg(command_hint_color),
                ),
                Span::styled(
                    cursor_char,
                    ratatui::style::Style::default()
                        .fg(Palette::BG)
                        .bg(Palette::CYAN)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(after, ratatui::style::Style::default().fg(Palette::TEXT)),
            ])
        } else {
            Line::from(display_text)
        };

        let inner = Rect::new(inner.x, inner.y, inner.width, required_height);

        // Calculate scroll position based on cursor
        let (cursor_line, _) = self.cursor_line_and_column();
        let scroll_line = if cursor_line >= required_height as usize {
            cursor_line.saturating_sub(required_height as usize - 1)
        } else {
            0
        };

        f.render_widget(
            Paragraph::new(text_with_cursor)
                .wrap(Wrap { trim: false })
                .scroll((scroll_line as u16, 0)),
            inner,
        );
    }
}

impl Default for InputBar {
    fn default() -> Self {
        Self::new()
    }
}

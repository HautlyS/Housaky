use crate::tui::enhanced_app::theme::{
    style_assistant_msg, style_border, style_border_focus, style_code_block, style_code_inline,
    style_dim, style_muted, style_streaming_cursor, style_system_msg, style_title, style_user_msg,
    Palette,
};
use chrono::Local;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

// ── Tool call/message types ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ToolCallMessage {
    pub id: usize,
    pub name: String,
    pub arguments: String,
    pub status: ToolCallStatus,
    pub result: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCallStatus {
    Pending,
    Running,
    Success,
    Error,
}

impl ToolCallMessage {
    pub fn new(id: usize, name: String, arguments: String) -> Self {
        Self {
            id,
            name,
            arguments,
            status: ToolCallStatus::Pending,
            result: None,
            timestamp: Local::now().format("%H:%M").to_string(),
        }
    }

    pub fn running(&mut self) {
        self.status = ToolCallStatus::Running;
    }

    pub fn success(&mut self, result: String) {
        self.status = ToolCallStatus::Success;
        self.result = Some(result);
    }

    pub fn error(&mut self, result: String) {
        self.status = ToolCallStatus::Error;
        self.result = Some(result);
    }
}

// ── Message types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
}

impl Role {
    pub fn label(&self) -> &'static str {
        match self {
            Role::User => "You",
            Role::Assistant => "Housaky",
            Role::System => "SYS",
        }
    }

    pub fn api_role(&self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: usize,
    pub role: Role,
    pub content: String,
    pub timestamp: String,
    pub tokens: Option<u32>,
}

impl Message {
    pub fn user(id: usize, content: String) -> Self {
        Self {
            id,
            role: Role::User,
            content,
            timestamp: Local::now().format("%H:%M").to_string(),
            tokens: None,
        }
    }

    pub fn assistant(id: usize, content: String, tokens: Option<u32>) -> Self {
        Self {
            id,
            role: Role::Assistant,
            content,
            timestamp: Local::now().format("%H:%M").to_string(),
            tokens,
        }
    }

    pub fn system(id: usize, content: String) -> Self {
        Self {
            id,
            role: Role::System,
            content,
            timestamp: Local::now().format("%H:%M").to_string(),
            tokens: None,
        }
    }
}

// ── Selection state ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct Selection {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub active: bool,
}

impl Selection {
    pub fn new(start_line: usize, start_col: usize) -> Self {
        Self {
            start_line,
            start_col,
            end_line: start_line,
            end_col: start_col,
            active: true,
        }
    }

    pub fn update_end(&mut self, line: usize, col: usize) {
        self.end_line = line;
        self.end_col = col;
    }

    pub fn clear(&mut self) {
        self.active = false;
        self.start_line = 0;
        self.start_col = 0;
        self.end_line = 0;
        self.end_col = 0;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn get_selected_text(&self, lines: &[String]) -> String {
        if !self.active || self.start_line >= lines.len() {
            return String::new();
        }

        let (start_line, start_col, end_line, end_col) = if self.start_line < self.end_line
            || (self.start_line == self.end_line && self.start_col <= self.end_col)
        {
            (
                self.start_line,
                self.start_col,
                self.end_line.min(lines.len() - 1),
                self.end_col,
            )
        } else {
            (
                self.end_line,
                self.end_col,
                self.start_line.min(lines.len() - 1),
                self.start_col,
            )
        };

        let mut result = String::new();
        for (i, line) in lines
            .iter()
            .enumerate()
            .skip(start_line)
            .take(end_line - start_line + 1)
        {
            if i == start_line && i == end_line {
                // Single line selection
                let start = start_col.min(line.len());
                let end = end_col.min(line.len());
                if start < end {
                    result.push_str(&line[start..end]);
                }
            } else if i == start_line {
                // First line of multi-line selection
                let start = start_col.min(line.len());
                result.push_str(&line[start..]);
                result.push('\n');
            } else if i == end_line {
                // Last line of multi-line selection
                let end = end_col.min(line.len());
                result.push_str(&line[..end]);
            } else {
                // Middle lines
                result.push_str(line);
                result.push('\n');
            }
        }
        result
    }
}

// ── Chat pane state ───────────────────────────────────────────────────────────

pub struct ChatPane {
    pub messages: Vec<Message>,
    pub tool_calls: Vec<ToolCallMessage>,
    pub streaming_buf: String,
    pub is_streaming: bool,
    pub scroll_offset: usize,
    pub auto_scroll: bool,
    pub search_query: String,
    pub search_results: Vec<usize>,
    pub search_cursor: usize,
    pub selection: Selection,
    pub rendered_lines: Vec<String>, // Cache of rendered lines for selection
    next_id: usize,
    next_tool_id: usize,
}

impl ChatPane {
    pub fn messages_len(&self) -> usize {
        self.messages.len()
    }

    pub fn push_tool_call(&mut self, name: String, arguments: String) -> usize {
        let id = self.next_tool_id;
        self.next_tool_id += 1;
        self.tool_calls
            .push(ToolCallMessage::new(id, name, arguments));
        id
    }

    pub fn update_tool_call(&mut self, id: usize, status: ToolCallStatus, result: Option<String>) {
        if let Some(tc) = self.tool_calls.iter_mut().find(|t| t.id == id) {
            match status {
                ToolCallStatus::Running => tc.running(),
                ToolCallStatus::Success => tc.success(result.unwrap_or_default()),
                ToolCallStatus::Error => tc.error(result.unwrap_or_default()),
                ToolCallStatus::Pending => {}
            }
        }
    }

    pub fn get_tool_call(&self, id: usize) -> Option<&ToolCallMessage> {
        self.tool_calls.iter().find(|t| t.id == id)
    }

    pub fn tool_calls_len(&self) -> usize {
        self.tool_calls.len()
    }

    pub fn is_streaming(&self) -> bool {
        self.is_streaming
    }

    pub fn scroll_to_index(&mut self, idx: usize) {
        let max = self.messages.len().saturating_sub(1);
        self.scroll_offset = idx.min(max);
        self.auto_scroll = self.scroll_offset >= max;
    }

    /// Best-effort mapping of visible message headers to viewport line offsets.
    /// Returns Vec of (message_index, header_line_offset) where header_line_offset is relative to the
    /// top of the viewport content (0-based).
    pub fn get_message(&self, idx: usize) -> Option<&Message> {
        self.messages.get(idx)
    }

    pub fn visible_header_line_offsets(&self, viewport_height: u16) -> Vec<(usize, u16)> {
        let mut out = Vec::new();
        let mut line: u16 = 0;
        let max_lines = viewport_height;

        for (mi, msg) in self.messages.iter().enumerate().skip(self.scroll_offset) {
            if line >= max_lines {
                break;
            }
            // header is current line
            out.push((mi, line));
            line = line.saturating_add(1);

            // content lines (approx: number of text lines + markdown expansions)
            let content_lines = msg.content.lines().count() as u16;
            line = line.saturating_add(content_lines);

            // spacing line
            line = line.saturating_add(1);
        }

        out
    }
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            tool_calls: Vec::new(),
            streaming_buf: String::new(),
            is_streaming: false,
            scroll_offset: 0,
            auto_scroll: true,
            search_query: String::new(),
            search_results: Vec::new(),
            search_cursor: 0,
            selection: Selection::default(),
            rendered_lines: Vec::new(),
            next_id: 0,
            next_tool_id: 0,
        }
    }

    // ── Message management ────────────────────────────────────────────────────

    pub fn push_user(&mut self, content: String) -> usize {
        let id = self.alloc_id();
        self.messages.push(Message::user(id, content));
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
        id
    }

    const MAX_MESSAGES: usize = 1000;

    pub fn push_assistant(&mut self, content: String, tokens: Option<u32>) -> usize {
        let id = self.alloc_id();
        self.messages.push(Message::assistant(id, content, tokens));
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
        id
    }

    pub fn push_system(&mut self, content: String) -> usize {
        let id = self.alloc_id();
        self.messages.push(Message::system(id, content));
        if self.messages.len() > Self::MAX_MESSAGES {
            self.messages
                .drain(..self.messages.len() - Self::MAX_MESSAGES);
        }
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
        id
    }

    pub fn start_streaming(&mut self) {
        self.streaming_buf.clear();
        self.is_streaming = true;
    }

    pub fn append_stream_chunk(&mut self, chunk: &str) {
        self.streaming_buf.push_str(chunk);
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn finish_streaming(&mut self, tokens: Option<u32>) -> usize {
        let content = std::mem::take(&mut self.streaming_buf);
        self.is_streaming = false;
        self.push_assistant(content, tokens)
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.streaming_buf.clear();
        self.is_streaming = false;
        self.scroll_offset = 0;
        self.next_id = 0;
    }

    pub fn last_assistant(&self) -> Option<&Message> {
        self.messages
            .iter()
            .rev()
            .find(|m| m.role == Role::Assistant)
    }

    pub fn export_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# Housaky Chat Export\n\n");
        for m in &self.messages {
            let role = m.role.label();
            out.push_str(&format!(
                "**[{}] {}**\n\n{}\n\n---\n\n",
                m.timestamp, role, m.content
            ));
        }
        out
    }

    // ── Scroll ────────────────────────────────────────────────────────────────

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
        self.auto_scroll = false;
    }

    pub fn scroll_down(&mut self, n: usize) {
        let max = self.messages.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + n).min(max);
        if self.scroll_offset >= max {
            self.auto_scroll = true;
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = false;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.messages.len().saturating_sub(1);
    }

    pub fn toggle_auto_scroll(&mut self) {
        self.auto_scroll = !self.auto_scroll;
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    // ── Search ────────────────────────────────────────────────────────────────

    pub fn set_search(&mut self, query: String) {
        let q = query.to_lowercase();
        self.search_results = self
            .messages
            .iter()
            .filter(|m| m.content.to_lowercase().contains(&q))
            .map(|m| m.id)
            .collect();
        self.search_cursor = 0;
        self.search_query = query;
    }

    pub fn search_next(&mut self) {
        if !self.search_results.is_empty() {
            self.search_cursor = (self.search_cursor + 1) % self.search_results.len();
            self.jump_to_search_result();
        }
    }

    pub fn search_prev(&mut self) {
        if !self.search_results.is_empty() {
            let len = self.search_results.len();
            self.search_cursor = (self.search_cursor + len - 1) % len;
            self.jump_to_search_result();
        }
    }

    fn jump_to_search_result(&mut self) {
        if let Some(&id) = self.search_results.get(self.search_cursor) {
            if let Some(idx) = self.messages.iter().position(|m| m.id == id) {
                self.scroll_offset = idx;
                self.auto_scroll = false;
            }
        }
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.search_results.clear();
    }

    // ── Text Selection ─────────────────────────────────────────────────────────

    /// Start text selection at the given line and column
    pub fn start_selection(&mut self, line: usize, col: usize) {
        self.selection = Selection::new(line, col);
    }

    /// Update the end of the current selection
    pub fn update_selection(&mut self, line: usize, col: usize) {
        if self.selection.is_active() {
            self.selection.update_end(line, col);
        }
    }

    /// Clear the current selection
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    /// Get the currently selected text
    pub fn get_selected_text(&self) -> String {
        self.selection.get_selected_text(&self.rendered_lines)
    }

    /// Check if there's an active selection
    pub fn has_selection(&self) -> bool {
        self.selection.is_active()
    }

    /// Copy selected text to clipboard (if available)
    pub fn copy_selection(&self) -> Option<String> {
        let text = self.get_selected_text();
        if text.is_empty() {
            None
        } else {
            // Try to copy to system clipboard
            #[cfg(feature = "clipboard")]
            if let Ok(mut ctx) = arboard::Clipboard::new() {
                let _ = ctx.set_text(&text);
            }
            Some(text)
        }
    }

    /// Select all text in the chat
    pub fn select_all(&mut self) {
        if !self.rendered_lines.is_empty() {
            self.selection = Selection {
                start_line: 0,
                start_col: 0,
                end_line: self.rendered_lines.len() - 1,
                end_col: self.rendered_lines.last().map(|l| l.len()).unwrap_or(0),
                active: true,
            };
        }
    }

    // ── Private ───────────────────────────────────────────────────────────────

    fn alloc_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    // ── Draw ──────────────────────────────────────────────────────────────────

    pub fn draw(&mut self, f: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            style_border_focus()
        } else {
            style_border()
        };
        let title_style = style_title();

        let title = if !self.search_query.is_empty() {
            format!(
                " 💬 Chat  /{} [{}/{}] ",
                self.search_query,
                self.search_cursor + 1,
                self.search_results.len()
            )
        } else {
            format!(" 💬 Chat  {} msgs ", self.messages.len())
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(title, title_style));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();
        let mut raw_lines: Vec<String> = Vec::new(); // For selection support

        let visible: Vec<_> = self.messages.iter().skip(self.scroll_offset).collect();

        for msg in &visible {
            let is_search_hit = self.search_results.contains(&msg.id);
            let hit_current =
                is_search_hit && self.search_results.get(self.search_cursor) == Some(&msg.id);

            let (role_style, bracket_color) = match msg.role {
                Role::User => (style_user_msg(), Palette::USER),
                Role::Assistant => (style_assistant_msg(), Palette::ASSISTANT),
                Role::System => (style_system_msg(), Palette::SYSTEM),
            };

            // Header line
            let header_text = format!(" {}  {} ", msg.role.label(), msg.timestamp);
            let mut header_spans = vec![
                Span::styled(
                    format!(" {} ", msg.role.label()),
                    role_style.add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" {} ", msg.timestamp), style_muted()),
            ];
            if let Some(t) = msg.tokens {
                header_spans.push(Span::styled(
                    format!(" {}t ", t),
                    Style::default()
                        .fg(bracket_color)
                        .add_modifier(Modifier::DIM),
                ));
            }
            if hit_current {
                header_spans.push(Span::styled(" ◀ ", Style::default().fg(Palette::WARNING)));
            }
            lines.push(Line::from(header_spans));
            raw_lines.push(header_text);

            // Content with markdown rendering
            let (content_lines, content_raw) =
                render_markdown_with_raw(&msg.content, is_search_hit, &self.search_query);
            lines.extend(content_lines);
            raw_lines.extend(content_raw);
            lines.push(Line::from(""));
            raw_lines.push(String::new());
        }

        // Streaming buffer
        if self.is_streaming && !self.streaming_buf.is_empty() {
            let header_text = " Housaky ".to_string();
            lines.push(Line::from(Span::styled(
                header_text.clone(),
                style_assistant_msg().add_modifier(Modifier::BOLD),
            )));
            raw_lines.push(header_text);
            let (content_lines, content_raw) =
                render_markdown_with_raw(&self.streaming_buf, false, "");
            lines.extend(content_lines);
            raw_lines.extend(content_raw);
            // blinking cursor
            lines.push(Line::from(Span::styled("▌", style_streaming_cursor())));
            raw_lines.push("▌".to_string());
            lines.push(Line::from(""));
            raw_lines.push(String::new());
        } else if self.is_streaming {
            let header_text = " Housaky ".to_string();
            lines.push(Line::from(vec![
                Span::styled(
                    header_text.clone(),
                    style_assistant_msg().add_modifier(Modifier::BOLD),
                ),
                Span::styled("▌", style_streaming_cursor()),
            ]));
            raw_lines.push(format!("{}▌", header_text));
        }

        // Store rendered lines for selection
        self.rendered_lines = raw_lines;

        // Apply selection highlighting
        if self.selection.is_active() {
            lines = apply_selection_highlighting(lines, &self.selection);
        }

        let para = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset as u16, 0));
        f.render_widget(para, inner);

        // Scrollbar
        if !self.messages.is_empty() {
            let total = self.messages.len() + if self.is_streaming { 1 } else { 0 };
            let mut sb_state = ScrollbarState::new(total).position(self.scroll_offset);
            let sb = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .thumb_symbol("█")
                .track_symbol(Some("│"));
            f.render_stateful_widget(sb, inner, &mut sb_state);
        }
    }
}

impl Default for ChatPane {
    fn default() -> Self {
        Self::new()
    }
}

// ── Markdown renderer ─────────────────────────────────────────────────────────

fn render_markdown<'a>(text: &'a str, highlight: bool, query: &str) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let mut in_code = false;
    let mut code_lang = String::new();
    let mut code_buf = String::new();

    for raw in text.lines() {
        if raw.starts_with("```") {
            if in_code {
                // flush code block
                for cl in code_buf.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", cl),
                        style_code_block(),
                    )));
                }
                code_buf.clear();
                code_lang.clear();
                in_code = false;
            } else {
                in_code = true;
                code_lang = raw.trim_start_matches('`').trim().to_owned();
                if !code_lang.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("  ▶ {}", code_lang),
                        Style::default()
                            .fg(Palette::VIOLET)
                            .add_modifier(Modifier::BOLD),
                    )));
                }
            }
            continue;
        }

        if in_code {
            code_buf.push_str(raw);
            code_buf.push('\n');
            continue;
        }

        // Headings
        if raw.starts_with("### ") {
            lines.push(Line::from(Span::styled(
                raw[4..].to_owned(),
                Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if raw.starts_with("## ") {
            lines.push(Line::from(Span::styled(
                raw[3..].to_owned(),
                Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            continue;
        }
        if raw.starts_with("# ") {
            lines.push(Line::from(Span::styled(
                raw[2..].to_owned(),
                Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            continue;
        }

        // List items
        if raw.starts_with("- ") || raw.starts_with("* ") {
            let mut spans = vec![Span::styled("  • ", style_dim())];
            spans.extend(inline_md(&raw[2..]));
            lines.push(Line::from(spans));
            continue;
        }
        if raw.len() > 3 {
            let maybe_ordered = &raw[..3];
            if maybe_ordered.ends_with(". ")
                && maybe_ordered
                    .chars()
                    .next()
                    .map_or(false, |c| c.is_ascii_digit())
            {
                let mut spans = vec![Span::styled(format!("  {} ", &raw[..2]), style_dim())];
                spans.extend(inline_md(&raw[3..]));
                lines.push(Line::from(spans));
                continue;
            }
        }

        // Blockquote
        if raw.starts_with("> ") {
            lines.push(Line::from(vec![
                Span::styled("  ┃ ", Style::default().fg(Palette::CYAN_DIM)),
                Span::styled(raw[2..].to_owned(), Style::default().fg(Palette::TEXT_DIM)),
            ]));
            continue;
        }

        // Horizontal rule
        if raw == "---" || raw == "***" {
            lines.push(Line::from(Span::styled(
                "  ─────────────────────────────────────── ",
                style_muted(),
            )));
            continue;
        }

        // Regular line with inline formatting + optional search highlight
        let mut spans = inline_md(raw);
        if highlight && !query.is_empty() {
            spans = highlight_search(spans, query);
        }
        // Indent slightly for readability
        let mut full = vec![Span::raw("  ")];
        full.extend(spans);
        lines.push(Line::from(full));
    }

    // Flush unclosed code block
    if in_code {
        for cl in code_buf.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {}", cl),
                style_code_block(),
            )));
        }
    }

    lines
}

fn inline_md(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut remaining: String = text.to_owned();

    loop {
        if remaining.is_empty() {
            break;
        }

        // Bold **text**
        if let Some(s) = remaining.find("**") {
            if let Some(e) = remaining[s + 2..].find("**") {
                if s > 0 {
                    spans.push(Span::raw(remaining[..s].to_owned()));
                }
                spans.push(Span::styled(
                    remaining[s + 2..s + 2 + e].to_owned(),
                    Style::default()
                        .fg(Palette::TEXT_BRIGHT)
                        .add_modifier(Modifier::BOLD),
                ));
                remaining = remaining[s + 2 + e + 2..].to_owned();
                continue;
            }
        }

        // Inline code `text`
        if let Some(s) = remaining.find('`') {
            if let Some(e) = remaining[s + 1..].find('`') {
                if s > 0 {
                    spans.push(Span::raw(remaining[..s].to_owned()));
                }
                spans.push(Span::styled(
                    remaining[s + 1..s + 1 + e].to_owned(),
                    style_code_inline(),
                ));
                remaining = remaining[s + 1 + e + 1..].to_owned();
                continue;
            }
        }

        // Italic *text*
        if let Some(s) = remaining.find('*') {
            if let Some(e) = remaining[s + 1..].find('*') {
                if s > 0 {
                    spans.push(Span::raw(remaining[..s].to_owned()));
                }
                spans.push(Span::styled(
                    remaining[s + 1..s + 1 + e].to_owned(),
                    Style::default()
                        .fg(Palette::TEXT)
                        .add_modifier(Modifier::ITALIC),
                ));
                remaining = remaining[s + 1 + e + 1..].to_owned();
                continue;
            }
        }

        spans.push(Span::raw(remaining.clone()));
        break;
    }

    if spans.is_empty() {
        spans.push(Span::raw(text.to_owned()));
    }
    spans
}

fn highlight_search(spans: Vec<Span<'static>>, query: &str) -> Vec<Span<'static>> {
    let q = query.to_lowercase();
    let mut out = Vec::new();
    for span in spans {
        let content = span.content.to_lowercase();
        if content.contains(&q) {
            out.push(Span::styled(
                span.content.into_owned(),
                span.style
                    .bg(Palette::BG_SELECTED)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            out.push(span);
        }
    }
    out
}

// ── Enhanced markdown renderer with raw text ─────────────────────────────────

/// Render markdown and return both styled lines and raw text for selection
fn render_markdown_with_raw<'a>(
    text: &'a str,
    highlight: bool,
    query: &str,
) -> (Vec<Line<'a>>, Vec<String>) {
    let mut lines = Vec::new();
    let mut raw_lines = Vec::new();
    let mut in_code = false;
    let mut code_lang = String::new();
    let mut code_buf = String::new();

    for raw in text.lines() {
        if raw.starts_with("```") {
            if in_code {
                // flush code block
                for cl in code_buf.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", cl),
                        style_code_block(),
                    )));
                    raw_lines.push(cl.to_string());
                }
                code_buf.clear();
                code_lang.clear();
                in_code = false;
            } else {
                in_code = true;
                code_lang = raw.trim_start_matches('`').trim().to_owned();
                if !code_lang.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("  ▶ {}", code_lang),
                        Style::default()
                            .fg(Palette::VIOLET)
                            .add_modifier(Modifier::BOLD),
                    )));
                    raw_lines.push(format!("▶ {}", code_lang));
                }
            }
            continue;
        }

        if in_code {
            code_buf.push_str(raw);
            code_buf.push('\n');
            continue;
        }

        // Headings
        if raw.starts_with("### ") {
            lines.push(Line::from(Span::styled(
                raw[4..].to_owned(),
                Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(Modifier::BOLD),
            )));
            raw_lines.push(raw[4..].to_string());
            continue;
        }
        if raw.starts_with("## ") {
            lines.push(Line::from(Span::styled(
                raw[3..].to_owned(),
                Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            raw_lines.push(raw[3..].to_string());
            continue;
        }
        if raw.starts_with("# ") {
            lines.push(Line::from(Span::styled(
                raw[2..].to_owned(),
                Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            raw_lines.push(raw[2..].to_string());
            continue;
        }

        // List items
        if raw.starts_with("- ") || raw.starts_with("* ") {
            let mut spans = vec![Span::styled("  • ", style_dim())];
            spans.extend(inline_md(&raw[2..]));
            lines.push(Line::from(spans));
            raw_lines.push(raw.to_string());
            continue;
        }
        // Check for ordered list (e.g., "1. item") — use char iteration to avoid byte boundary panics
        let mut chars = raw.chars();
        if let Some(first) = chars.next() {
            if first.is_ascii_digit() {
                if let Some(second) = chars.next() {
                    if second == '.' {
                        if let Some(third) = chars.next() {
                            if third == ' ' {
                                let prefix_len = first.len_utf8() + second.len_utf8();
                                let mut spans = vec![Span::styled(
                                    format!("  {} ", &raw[..prefix_len]),
                                    style_dim(),
                                )];
                                let rest_start = prefix_len + third.len_utf8();
                                if rest_start < raw.len() {
                                    spans.extend(inline_md(&raw[rest_start..]));
                                }
                                lines.push(Line::from(spans));
                                raw_lines.push(raw.to_string());
                                continue;
                            }
                        }
                    }
                }
            }
        }

        // Blockquote
        if raw.starts_with("> ") {
            lines.push(Line::from(vec![
                Span::styled("  ┃ ", Style::default().fg(Palette::CYAN_DIM)),
                Span::styled(raw[2..].to_owned(), Style::default().fg(Palette::TEXT_DIM)),
            ]));
            raw_lines.push(raw.to_string());
            continue;
        }

        // Horizontal rule
        if raw == "---" || raw == "***" {
            lines.push(Line::from(Span::styled(
                "  ─────────────────────────────────────── ",
                style_muted(),
            )));
            raw_lines.push(raw.to_string());
            continue;
        }

        // Links [text](url) - render text and url
        if raw.contains('[') && raw.contains("](") {
            let mut processed = String::new();
            let mut chars = raw.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '[' {
                    let mut link_text = String::new();
                    while let Some(c) = chars.next() {
                        if c == ']' {
                            break;
                        }
                        link_text.push(c);
                    }
                    // Skip (
                    if chars.next() == Some('(') {
                        let mut url = String::new();
                        while let Some(c) = chars.next() {
                            if c == ')' {
                                break;
                            }
                            url.push(c);
                        }
                        processed.push_str(&link_text);
                        processed.push_str(" (");
                        processed.push_str(&url);
                        processed.push(')');
                    } else {
                        processed.push('[');
                        processed.push_str(&link_text);
                        processed.push(']');
                    }
                } else {
                    processed.push(c);
                }
            }
            let spans = inline_md(&processed);
            let mut full = vec![Span::raw("  ")];
            full.extend(spans);
            lines.push(Line::from(full));
            raw_lines.push(raw.to_string());
            continue;
        }

        // Regular line with inline formatting + optional search highlight
        let mut spans = inline_md(raw);
        if highlight && !query.is_empty() {
            spans = highlight_search(spans, query);
        }
        // Indent slightly for readability
        let mut full = vec![Span::raw("  ")];
        full.extend(spans);
        lines.push(Line::from(full));
        raw_lines.push(raw.to_string());
    }

    // Flush unclosed code block
    if in_code {
        for cl in code_buf.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {}", cl),
                style_code_block(),
            )));
            raw_lines.push(cl.to_string());
        }
    }

    (lines, raw_lines)
}

/// Apply selection highlighting to rendered lines
fn apply_selection_highlighting<'a>(lines: Vec<Line<'a>>, selection: &Selection) -> Vec<Line<'a>> {
    let (sel_start_line, sel_start_col, sel_end_line, sel_end_col) = if selection.start_line
        < selection.end_line
        || (selection.start_line == selection.end_line && selection.start_col <= selection.end_col)
    {
        (
            selection.start_line,
            selection.start_col,
            selection.end_line,
            selection.end_col,
        )
    } else {
        (
            selection.end_line,
            selection.end_col,
            selection.start_line,
            selection.start_col,
        )
    };

    lines
        .into_iter()
        .enumerate()
        .map(|(line_idx, line)| {
            if line_idx < sel_start_line || line_idx > sel_end_line {
                return line;
            }

            let spans: Vec<Span> = line
                .spans
                .into_iter()
                .flat_map(|span| {
                    let content = span.content.to_string();
                    let mut result = Vec::new();
                    let mut current_col = 0;

                    for ch in content.chars() {
                        let in_selection = if line_idx == sel_start_line && line_idx == sel_end_line
                        {
                            // Single line selection
                            current_col >= sel_start_col && current_col < sel_end_col
                        } else if line_idx == sel_start_line {
                            // First line of multi-line
                            current_col >= sel_start_col
                        } else if line_idx == sel_end_line {
                            // Last line of multi-line
                            current_col < sel_end_col
                        } else {
                            // Middle lines - fully selected
                            true
                        };

                        if in_selection {
                            result.push(Span::styled(
                                ch.to_string(),
                                span.style.bg(Palette::BG_SELECTED).fg(Palette::TEXT_BRIGHT),
                            ));
                        } else {
                            result.push(Span::styled(ch.to_string(), span.style));
                        }
                        current_col += 1;
                    }
                    result
                })
                .collect();

            Line::from(spans)
        })
        .collect()
}

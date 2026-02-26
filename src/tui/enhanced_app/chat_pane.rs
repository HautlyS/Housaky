use chrono::Local;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};
use crate::tui::enhanced_app::theme::{
    Palette, style_assistant_msg, style_border, style_border_focus, style_code_block,
    style_code_inline, style_dim, style_muted, style_streaming_cursor, style_system_msg,
    style_title, style_user_msg,
};

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
            Role::User      => "You",
            Role::Assistant => "AGI",
            Role::System    => "SYS",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id:        usize,
    pub role:      Role,
    pub content:   String,
    pub timestamp: String,
    pub tokens:    Option<u32>,
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

// ── Chat pane state ───────────────────────────────────────────────────────────

pub struct ChatPane {
    pub messages:       Vec<Message>,
    pub streaming_buf:  String,
    pub is_streaming:   bool,
    pub scroll_offset:  usize,
    pub auto_scroll:    bool,
    pub search_query:   String,
    pub search_results: Vec<usize>,
    pub search_cursor:  usize,
    next_id:            usize,
}

impl ChatPane {
    pub fn new() -> Self {
        Self {
            messages:       Vec::new(),
            streaming_buf:  String::new(),
            is_streaming:   false,
            scroll_offset:  0,
            auto_scroll:    true,
            search_query:   String::new(),
            search_results: Vec::new(),
            search_cursor:  0,
            next_id:        0,
        }
    }

    // ── Message management ────────────────────────────────────────────────────

    pub fn push_user(&mut self, content: String) -> usize {
        let id = self.alloc_id();
        self.messages.push(Message::user(id, content));
        if self.auto_scroll { self.scroll_to_bottom(); }
        id
    }

    pub fn push_assistant(&mut self, content: String, tokens: Option<u32>) -> usize {
        let id = self.alloc_id();
        self.messages.push(Message::assistant(id, content, tokens));
        if self.auto_scroll { self.scroll_to_bottom(); }
        id
    }

    pub fn push_system(&mut self, content: String) -> usize {
        let id = self.alloc_id();
        self.messages.push(Message::system(id, content));
        if self.auto_scroll { self.scroll_to_bottom(); }
        id
    }

    pub fn start_streaming(&mut self) {
        self.streaming_buf.clear();
        self.is_streaming = true;
    }

    pub fn append_stream_chunk(&mut self, chunk: &str) {
        self.streaming_buf.push_str(chunk);
        if self.auto_scroll { self.scroll_to_bottom(); }
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
        self.messages.iter().rev().find(|m| m.role == Role::Assistant)
    }

    pub fn export_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# Housaky Chat Export\n\n");
        for m in &self.messages {
            let role = m.role.label();
            out.push_str(&format!("**[{}] {}**\n\n{}\n\n---\n\n", m.timestamp, role, m.content));
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
        if self.scroll_offset >= max { self.auto_scroll = true; }
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
        if self.auto_scroll { self.scroll_to_bottom(); }
    }

    // ── Search ────────────────────────────────────────────────────────────────

    pub fn set_search(&mut self, query: String) {
        let q = query.to_lowercase();
        self.search_results = self.messages.iter()
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

    // ── Private ───────────────────────────────────────────────────────────────

    fn alloc_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    // ── Draw ──────────────────────────────────────────────────────────────────

    pub fn draw(&self, f: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused { style_border_focus() } else { style_border() };
        let title_style = style_title();

        let title = if !self.search_query.is_empty() {
            format!(" 💬 Chat  /{} [{}/{}] ", self.search_query,
                self.search_cursor + 1, self.search_results.len())
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

        let visible: Vec<_> = self.messages.iter().skip(self.scroll_offset).collect();

        for msg in &visible {
            let is_search_hit = self.search_results.contains(&msg.id);
            let hit_current = is_search_hit
                && self.search_results.get(self.search_cursor) == Some(&msg.id);

            let (role_style, bracket_color) = match msg.role {
                Role::User      => (style_user_msg(),      Palette::USER),
                Role::Assistant => (style_assistant_msg(), Palette::ASSISTANT),
                Role::System    => (style_system_msg(),    Palette::SYSTEM),
            };

            // Header line
            let mut header_spans = vec![
                Span::styled(
                    format!(" {} ", msg.role.label()),
                    role_style.add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {} ", msg.timestamp),
                    style_muted(),
                ),
            ];
            if let Some(t) = msg.tokens {
                header_spans.push(Span::styled(
                    format!(" {}t ", t),
                    Style::default().fg(bracket_color).add_modifier(Modifier::DIM),
                ));
            }
            if hit_current {
                header_spans.push(Span::styled(" ◀ ", Style::default().fg(Palette::WARNING)));
            }
            lines.push(Line::from(header_spans));

            // Content with markdown rendering
            let content_lines = render_markdown(&msg.content, is_search_hit, &self.search_query);
            lines.extend(content_lines);
            lines.push(Line::from(""));
        }

        // Streaming buffer
        if self.is_streaming && !self.streaming_buf.is_empty() {
            lines.push(Line::from(Span::styled(
                " AGI ".to_string(),
                style_assistant_msg().add_modifier(Modifier::BOLD),
            )));
            let content_lines = render_markdown(&self.streaming_buf, false, "");
            lines.extend(content_lines);
            // blinking cursor
            lines.push(Line::from(Span::styled("▌", style_streaming_cursor())));
            lines.push(Line::from(""));
        } else if self.is_streaming {
            lines.push(Line::from(vec![
                Span::styled(" AGI ", style_assistant_msg().add_modifier(Modifier::BOLD)),
                Span::styled("▌", style_streaming_cursor()),
            ]));
        }

        let para = Paragraph::new(lines)
            .wrap(Wrap { trim: false });
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
    fn default() -> Self { Self::new() }
}

// ── Markdown renderer ─────────────────────────────────────────────────────────

fn render_markdown<'a>(text: &'a str, highlight: bool, query: &str) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let mut in_code = false;
    let mut code_lang = String::new();
    let mut code_buf  = String::new();

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
                Style::default().fg(Palette::CYAN).add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if raw.starts_with("## ") {
            lines.push(Line::from(Span::styled(
                raw[3..].to_owned(),
                Style::default().fg(Palette::CYAN).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            continue;
        }
        if raw.starts_with("# ") {
            lines.push(Line::from(Span::styled(
                raw[2..].to_owned(),
                Style::default().fg(Palette::CYAN).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
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
            if maybe_ordered.ends_with(". ") && maybe_ordered.chars().next().map_or(false, |c| c.is_ascii_digit()) {
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
        if remaining.is_empty() { break; }

        // Bold **text**
        if let Some(s) = remaining.find("**") {
            if let Some(e) = remaining[s + 2..].find("**") {
                if s > 0 { spans.push(Span::raw(remaining[..s].to_owned())); }
                spans.push(Span::styled(
                    remaining[s + 2..s + 2 + e].to_owned(),
                    Style::default().fg(Palette::TEXT_BRIGHT).add_modifier(Modifier::BOLD),
                ));
                remaining = remaining[s + 2 + e + 2..].to_owned();
                continue;
            }
        }

        // Inline code `text`
        if let Some(s) = remaining.find('`') {
            if let Some(e) = remaining[s + 1..].find('`') {
                if s > 0 { spans.push(Span::raw(remaining[..s].to_owned())); }
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
                if s > 0 { spans.push(Span::raw(remaining[..s].to_owned())); }
                spans.push(Span::styled(
                    remaining[s + 1..s + 1 + e].to_owned(),
                    Style::default().fg(Palette::TEXT).add_modifier(Modifier::ITALIC),
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
                span.style.bg(Palette::BG_SELECTED).add_modifier(Modifier::BOLD),
            ));
        } else {
            out.push(span);
        }
    }
    out
}

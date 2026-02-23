use chrono::Local;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: String,
    pub id: usize,
}

impl Message {
    pub fn user(content: String, id: usize) -> Self {
        Self {
            role: "user".into(),
            content,
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            id,
        }
    }

    pub fn assistant(content: String, id: usize) -> Self {
        Self {
            role: "assistant".into(),
            content,
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            id,
        }
    }

    pub fn system(content: String, id: usize) -> Self {
        Self {
            role: "system".into(),
            content,
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            id,
        }
    }
}

pub struct ChatState {
    pub messages: Vec<Message>,
    pub provider_name: String,
    pub model: String,
    pub loading: bool,
    pub scroll_offset: usize,
    pub auto_scroll: bool,
    pub next_id: usize,
}

impl ChatState {
    pub fn new(provider_name: String, model: String) -> Self {
        Self {
            messages: Vec::new(),
            provider_name,
            model,
            loading: false,
            scroll_offset: 0,
            auto_scroll: true,
            next_id: 0,
        }
    }

    pub fn add_user_message(&mut self, content: String) -> usize {
        let id = self.next_id;
        self.messages.push(Message::user(content, id));
        self.next_id += 1;
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
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

    pub fn add_system_message(&mut self, content: String) -> usize {
        let id = self.next_id;
        self.messages.push(Message::system(content, id));
        self.next_id += 1;
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
        id
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
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

    pub fn scroll_page_up(&mut self, page_size: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
        self.auto_scroll = false;
    }

    pub fn scroll_page_down(&mut self, page_size: usize) {
        let max_scroll = self.messages.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + page_size).min(max_scroll);
        if self.scroll_offset >= max_scroll {
            self.auto_scroll = true;
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = false;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.messages.len().saturating_sub(1);
        self.auto_scroll = true;
    }

    pub fn toggle_auto_scroll(&mut self) {
        self.auto_scroll = !self.auto_scroll;
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn get_message_by_id(&self, id: usize) -> Option<&Message> {
        self.messages.iter().find(|m| m.id == id)
    }

    pub fn search_messages(&self, query: &str) -> Vec<usize> {
        let query_lower = query.to_lowercase();
        self.messages
            .iter()
            .filter(|m| m.content.to_lowercase().contains(&query_lower))
            .map(|m| m.id)
            .collect()
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
        self.next_id = 0;
    }

    pub fn export_to_string(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!(
            "# Housaky Chat Log\nProvider: {}\nModel: {}\n\n",
            self.provider_name, self.model
        ));
        for msg in &self.messages {
            let role_str = match msg.role.as_str() {
                "user" => "User",
                "assistant" => "Assistant",
                _ => "System",
            };
            output.push_str(&format!(
                "[{}] {}: {}\n\n",
                msg.timestamp, role_str, msg.content
            ));
        }
        output
    }

    pub fn get_last_user_message(&self) -> Option<&Message> {
        self.messages.iter().rev().find(|m| m.role == "user")
    }

    pub fn get_last_assistant_message(&self) -> Option<&Message> {
        self.messages.iter().rev().find(|m| m.role == "assistant")
    }
}

pub fn format_message_content(content: &str) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;
    let mut code_block_content = String::new();
    let mut code_block_language = String::new();

    for line in content.lines() {
        if line.starts_with("```") {
            if in_code_block {
                // End of code block
                if !code_block_content.is_empty() {
                    for code_line in code_block_content.lines() {
                        lines.push(Line::from(vec![Span::styled(
                            code_line.to_string(),
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Rgb(240, 240, 240)),
                        )]));
                    }
                }
                code_block_content.clear();
                code_block_language.clear();
                in_code_block = false;
            } else {
                // Start of code block
                in_code_block = true;
                code_block_language = line.trim_start_matches("```").trim().to_string();
                if !code_block_language.is_empty() {
                    lines.push(Line::from(vec![Span::styled(
                        format!("▶ {}", code_block_language),
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::Rgb(80, 80, 80))
                            .add_modifier(Modifier::BOLD),
                    )]));
                }
            }
        } else if in_code_block {
            code_block_content.push_str(line);
            code_block_content.push('\n');
        } else {
            let formatted_line = format_inline_markdown(line);
            lines.push(Line::from(formatted_line));
        }
    }

    // Handle unclosed code block
    if in_code_block && !code_block_content.is_empty() {
        for code_line in code_block_content.lines() {
            lines.push(Line::from(vec![Span::styled(
                code_line.to_string(),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Rgb(240, 240, 240)),
            )]));
        }
    }

    lines
}

fn format_inline_markdown(line: &str) -> Vec<Span<'_>> {
    let mut spans = Vec::new();
    let mut remaining = line;

    while !remaining.is_empty() {
        // Check for bold (**text**)
        if let Some(start) = remaining.find("**") {
            if start > 0 {
                spans.push(Span::raw(&remaining[..start]));
            }
            if let Some(end) = remaining[start + 2..].find("**") {
                let bold_text = &remaining[start + 2..start + 2 + end];
                spans.push(Span::styled(
                    bold_text.to_string(),
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::White),
                ));
                remaining = &remaining[start + 2 + end + 2..];
                continue;
            }
        }

        // Check for italic (*text*)
        if let Some(start) = remaining.find('*') {
            if start > 0 {
                spans.push(Span::raw(&remaining[..start]));
            }
            if let Some(end) = remaining[start + 1..].find('*') {
                let italic_text = &remaining[start + 1..start + 1 + end];
                spans.push(Span::styled(
                    italic_text.to_string(),
                    Style::default().add_modifier(Modifier::ITALIC),
                ));
                remaining = &remaining[start + 1 + end + 1..];
                continue;
            }
        }

        // Check for inline code (`text`)
        if let Some(start) = remaining.find('`') {
            if start > 0 {
                spans.push(Span::raw(&remaining[..start]));
            }
            if let Some(end) = remaining[start + 1..].find('`') {
                let code_text = &remaining[start + 1..start + 1 + end];
                spans.push(Span::styled(
                    code_text.to_string(),
                    Style::default()
                        .fg(Color::Rgb(255, 100, 100))
                        .bg(Color::Rgb(40, 40, 40)),
                ));
                remaining = &remaining[start + 1 + end + 1..];
                continue;
            }
        }

        // No more formatting found
        spans.push(Span::raw(remaining.to_string()));
        break;
    }

    if spans.is_empty() {
        spans.push(Span::raw(line.to_string()));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello".to_string(), 0);
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
        assert_eq!(msg.id, 0);
    }

    #[test]
    fn test_chat_state_add_message() {
        let mut state = ChatState::new("test".to_string(), "model".to_string());
        let id = state.add_user_message("Test message".to_string());
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].id, id);
    }

    #[test]
    fn test_search_messages() {
        let mut state = ChatState::new("test".to_string(), "model".to_string());
        state.add_user_message("Hello world".to_string());
        state.add_assistant_message("Goodbye world".to_string());

        let results = state.search_messages("hello");
        assert_eq!(results.len(), 1);

        let results = state.search_messages("world");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_export_to_string() {
        let mut state = ChatState::new("test".to_string(), "model".to_string());
        state.add_user_message("Test".to_string());
        let export = state.export_to_string();
        assert!(export.contains("Housaky Chat Log"));
        assert!(export.contains("Test"));
    }
}

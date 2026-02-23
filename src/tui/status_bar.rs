use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::time::{Duration, Instant};

pub struct StatusBar {
    pub provider: String,
    pub model: String,
    pub connected: bool,
    pub message_count: usize,
    pub auto_scroll: bool,
    pub last_error: Option<(String, Instant)>,
    pub typing_indicator: TypingIndicator,
}

pub struct TypingIndicator {
    pub active: bool,
    pub frame: usize,
    pub last_update: Instant,
}

impl TypingIndicator {
    pub fn new() -> Self {
        Self {
            active: false,
            frame: 0,
            last_update: Instant::now(),
        }
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
        if active {
            self.frame = 0;
        }
    }

    pub fn update(&mut self) {
        if self.active && self.last_update.elapsed() >= Duration::from_millis(500) {
            self.frame = (self.frame + 1) % 4;
            self.last_update = Instant::now();
        }
    }

    pub fn render(&self) -> String {
        if !self.active {
            return String::new();
        }
        let dots = match self.frame {
            0 => "⏳ ",
            1 => "⏳ .",
            2 => "⏳ ..",
            3 => "⏳ ...",
            _ => "⏳ ",
        };
        dots.to_string()
    }
}

impl StatusBar {
    pub fn new(provider: String, model: String) -> Self {
        Self {
            provider,
            model,
            connected: true,
            message_count: 0,
            auto_scroll: true,
            last_error: None,
            typing_indicator: TypingIndicator::new(),
        }
    }

    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
    }

    pub fn set_message_count(&mut self, count: usize) {
        self.message_count = count;
    }

    pub fn set_auto_scroll(&mut self, auto_scroll: bool) {
        self.auto_scroll = auto_scroll;
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.typing_indicator.set_active(loading);
    }

    pub fn set_error(&mut self, error: String) {
        self.last_error = Some((error, Instant::now()));
    }

    pub fn clear_error(&mut self) {
        self.last_error = None;
    }

    pub fn update(&mut self) {
        self.typing_indicator.update();

        // Clear old errors after 5 seconds
        if let Some((_, time)) = &self.last_error {
            if time.elapsed() > Duration::from_secs(5) {
                self.last_error = None;
            }
        }
    }

    pub fn draw(&self, f: &mut Frame, area: Rect) {
        let connection_status = if self.connected {
            Span::styled("● Connected", Style::default().fg(Color::Green))
        } else {
            Span::styled("● Disconnected", Style::default().fg(Color::Red))
        };

        let scroll_status = if self.auto_scroll {
            Span::styled("[AUTO]", Style::default().fg(Color::Green))
        } else {
            Span::styled("[MANUAL]", Style::default().fg(Color::Yellow))
        };

        let typing = self.typing_indicator.render();
        let typing_span = if self.typing_indicator.active {
            Span::styled(typing, Style::default().fg(Color::Cyan))
        } else {
            Span::raw("")
        };

        let status_text = Line::from(vec![
            Span::styled(
                format!(" {} ", self.provider),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(" | "),
            Span::styled(
                format!(" {} ", self.model),
                Style::default().fg(Color::Magenta),
            ),
            Span::raw(" | "),
            connection_status,
            Span::raw(" | "),
            Span::styled(
                format!(" Messages: {} ", self.message_count),
                Style::default().fg(Color::White),
            ),
            Span::raw(" | "),
            scroll_status,
            Span::raw(" "),
            typing_span,
        ]);

        let status_bar = Paragraph::new(status_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );

        f.render_widget(status_bar, area);

        // Draw error popup if present
        if let Some((error, _)) = &self.last_error {
            let error_area = Rect::new(
                area.x + area.width / 4,
                area.y.saturating_sub(3),
                area.width / 2,
                3,
            );
            let error_widget = Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::Red))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Error ")
                        .title_style(Style::default().fg(Color::Red)),
                );
            f.render_widget(ratatui::widgets::Clear, error_area);
            f.render_widget(error_widget, error_area);
        }
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new("unknown".to_string(), "unknown".to_string())
    }
}

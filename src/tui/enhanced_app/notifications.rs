use std::time::{Duration, Instant};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use crate::tui::enhanced_app::theme::{
    style_toast_info, style_toast_success, style_toast_error, style_toast_warn,
};

// ── Toast kind ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Error,
    Warn,
}

impl ToastKind {
    fn icon(&self) -> &'static str {
        match self {
            ToastKind::Info    => "ℹ",
            ToastKind::Success => "✓",
            ToastKind::Error   => "✗",
            ToastKind::Warn    => "⚠",
        }
    }

    fn ttl(&self) -> Duration {
        match self {
            ToastKind::Error => Duration::from_secs(6),
            ToastKind::Warn  => Duration::from_secs(5),
            _                => Duration::from_secs(3),
        }
    }
}

// ── Toast entry ───────────────────────────────────────────────────────────────

struct Toast {
    kind:    ToastKind,
    message: String,
    born:    Instant,
}

impl Toast {
    fn is_expired(&self) -> bool {
        self.born.elapsed() >= self.kind.ttl()
    }

    fn age_fraction(&self) -> f64 {
        let elapsed = self.born.elapsed().as_secs_f64();
        let ttl = self.kind.ttl().as_secs_f64();
        (elapsed / ttl).min(1.0)
    }
}

// ── Notification stack ────────────────────────────────────────────────────────

pub struct Notifications {
    stack: Vec<Toast>,
}

impl Notifications {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn push(&mut self, kind: ToastKind, message: impl Into<String>) {
        self.stack.retain(|t| !t.is_expired());
        if self.stack.len() >= 4 {
            self.stack.remove(0);
        }
        self.stack.push(Toast {
            kind,
            message: message.into(),
            born: Instant::now(),
        });
    }

    pub fn info(&mut self, msg: impl Into<String>) {
        self.push(ToastKind::Info, msg);
    }

    pub fn success(&mut self, msg: impl Into<String>) {
        self.push(ToastKind::Success, msg);
    }

    pub fn error(&mut self, msg: impl Into<String>) {
        self.push(ToastKind::Error, msg);
    }

    pub fn warn(&mut self, msg: impl Into<String>) {
        self.push(ToastKind::Warn, msg);
    }

    pub fn tick(&mut self) {
        self.stack.retain(|t| !t.is_expired());
    }

    pub fn draw(&self, f: &mut Frame) {
        if self.stack.is_empty() {
            return;
        }
        let area = f.area();
        let mut y = area.y + 1;

        for toast in self.stack.iter().rev().take(4) {
            let msg = format!(" {} {} ", toast.kind.icon(), toast.message);
            let width = (msg.len() as u16 + 2).min(area.width.saturating_sub(4));
            let x = area.x + area.width.saturating_sub(width + 2);

            let toast_area = Rect::new(x, y, width, 3);
            if toast_area.bottom() > area.bottom() {
                break;
            }

            f.render_widget(Clear, toast_area);

            let style = match toast.kind {
                ToastKind::Info    => style_toast_info(),
                ToastKind::Success => style_toast_success(),
                ToastKind::Error   => style_toast_error(),
                ToastKind::Warn    => style_toast_warn(),
            };

            // Fade-out effect: dim text slightly near expiry
            let age = toast.age_fraction();
            let display_style = if age > 0.75 {
                style.add_modifier(ratatui::style::Modifier::DIM)
            } else {
                style
            };

            let text_line = Line::from(Span::styled(msg, display_style));
            let widget = Paragraph::new(text_line)
                .block(Block::default().borders(Borders::NONE))
                .style(display_style);

            f.render_widget(widget, toast_area);
            y += 3;
        }
    }
}

impl Default for Notifications {
    fn default() -> Self {
        Self::new()
    }
}

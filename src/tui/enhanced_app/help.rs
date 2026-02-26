use crate::tui::enhanced_app::layout::centered_rect;
use crate::tui::enhanced_app::theme::{style_border_focus, style_muted, style_title, Palette};
use ratatui::{
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

// ── Keybind section ───────────────────────────────────────────────────────────

struct Section {
    title: &'static str,
    binds: &'static [(&'static str, &'static str)],
}

const SECTIONS: &[Section] = &[
    Section {
        title: "Navigation",
        binds: &[
            ("Tab / Shift+Tab", "Cycle tabs forward / backward"),
            ("v", "Cycle view mode (Full / Split / Dashboard)"),
            ("b", "Toggle sidebar"),
            ("Ctrl+P", "Open command palette"),
            ("?  or  F1", "Toggle this help"),
            ("q  or  Ctrl+C", "Quit Housaky"),
        ],
    },
    Section {
        title: "Chat",
        binds: &[
            ("Enter", "Send message"),
            ("↑ / ↓", "Browse input history"),
            ("Esc", "Blur input / back to normal"),
            ("/ + Enter", "Execute slash-command"),
            ("Ctrl+K", "Kill line (clear input)"),
            ("Ctrl+W", "Delete last word"),
            ("Home / End", "Move cursor to start / end"),
            ("PageUp / PageDown", "Scroll chat messages"),
            ("a", "Toggle auto-scroll"),
            ("s", "Export chat to markdown file"),
        ],
    },
    Section {
        title: "Search",
        binds: &[
            ("Ctrl+F  or  /", "Open inline search"),
            ("n", "Next search result"),
            ("N  (Shift+n)", "Previous search result"),
            ("Esc", "Close search"),
        ],
    },
    Section {
        title: "AGI / Skills",
        binds: &[
            ("1", "Chat tab"),
            ("2", "Skills tab"),
            ("3", "Tools tab"),
            ("4", "Goals tab"),
            ("5", "Metrics tab"),
            ("Space / Enter", "Toggle skill enable/disable"),
            ("r", "Refresh skills list"),
        ],
    },
    Section {
        title: "Slash Commands",
        binds: &[
            ("/clear", "Clear conversation"),
            ("/export", "Export chat to .md file"),
            ("/model <name>", "Switch AI model"),
            ("/goals", "Jump to Goals tab"),
            ("/skills", "Jump to Skills tab"),
            ("/reflect", "Trigger AGI self-reflection"),
            ("/quit", "Exit Housaky"),
        ],
    },
];

// ── Help overlay ──────────────────────────────────────────────────────────────

pub struct HelpOverlay {
    pub visible: bool,
    scroll: usize,
}

impl HelpOverlay {
    pub fn new() -> Self {
        Self {
            visible: false,
            scroll: 0,
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.scroll = 0;
    }
    pub fn hide(&mut self) {
        self.visible = false;
    }
    pub fn toggle(&mut self) {
        if self.visible {
            self.hide()
        } else {
            self.show()
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }
    pub fn scroll_down(&mut self) {
        self.scroll += 1;
    }

    pub fn draw(&self, f: &mut Frame) {
        if !self.visible {
            return;
        }

        let area = f.area();
        let popup = centered_rect(72, 82, area);

        f.render_widget(Clear, popup);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border_focus())
            .title(Span::styled(
                " ◈ HOUSAKY — Keyboard Reference ",
                style_title(),
            ))
            .title_bottom(Span::styled(" ↑↓ scroll  any key to close ", style_muted()));

        let inner = block.inner(popup);
        f.render_widget(block, popup);

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(""));

        for section in SECTIONS {
            // Section header
            lines.push(Line::from(Span::styled(
                format!("  ── {} ", section.title),
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            lines.push(Line::from(""));

            for (key, desc) in section.binds {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("    {:<22}", key),
                        ratatui::style::Style::default()
                            .fg(Palette::VIOLET)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(*desc, ratatui::style::Style::default().fg(Palette::TEXT)),
                ]));
            }
            lines.push(Line::from(""));
        }

        let visible_lines: Vec<Line> = lines.into_iter().skip(self.scroll).collect();

        f.render_widget(
            Paragraph::new(visible_lines).wrap(Wrap { trim: false }),
            inner,
        );
    }
}

impl Default for HelpOverlay {
    fn default() -> Self {
        Self::new()
    }
}

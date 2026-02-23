use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

#[derive(Debug, Clone)]
pub struct HelpItem {
    pub key: String,
    pub description: String,
    pub mode: HelpMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpMode {
    Normal,
    Editing,
    Both,
}

pub struct HelpPopup {
    pub visible: bool,
}

impl HelpPopup {
    pub fn new() -> Self {
        Self { visible: false }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    fn get_help_items() -> Vec<HelpItem> {
        vec![
            HelpItem {
                key: "i / Enter".to_string(),
                description: "Start typing (Normal mode)".to_string(),
                mode: HelpMode::Normal,
            },
            HelpItem {
                key: "Esc / Ctrl+C".to_string(),
                description: "Exit input mode / Cancel".to_string(),
                mode: HelpMode::Editing,
            },
            HelpItem {
                key: "Enter".to_string(),
                description: "Send message (Editing mode)".to_string(),
                mode: HelpMode::Editing,
            },
            HelpItem {
                key: "q".to_string(),
                description: "Quit application".to_string(),
                mode: HelpMode::Normal,
            },
            HelpItem {
                key: "? / F1".to_string(),
                description: "Toggle this help".to_string(),
                mode: HelpMode::Both,
            },
            HelpItem {
                key: "Up / Down".to_string(),
                description: "Scroll messages".to_string(),
                mode: HelpMode::Normal,
            },
            HelpItem {
                key: "PgUp / PgDown".to_string(),
                description: "Scroll page up/down".to_string(),
                mode: HelpMode::Normal,
            },
            HelpItem {
                key: "Home".to_string(),
                description: "Scroll to top".to_string(),
                mode: HelpMode::Normal,
            },
            HelpItem {
                key: "End".to_string(),
                description: "Scroll to bottom".to_string(),
                mode: HelpMode::Normal,
            },
            HelpItem {
                key: "a".to_string(),
                description: "Toggle auto-scroll".to_string(),
                mode: HelpMode::Normal,
            },
            HelpItem {
                key: "/".to_string(),
                description: "Search messages".to_string(),
                mode: HelpMode::Normal,
            },
            HelpItem {
                key: "n / N".to_string(),
                description: "Next/previous search result".to_string(),
                mode: HelpMode::Normal,
            },
            HelpItem {
                key: "c".to_string(),
                description: "Copy last response to clipboard".to_string(),
                mode: HelpMode::Normal,
            },
            HelpItem {
                key: "s".to_string(),
                description: "Save conversation to file".to_string(),
                mode: HelpMode::Normal,
            },
            HelpItem {
                key: "Delete".to_string(),
                description: "Clear input".to_string(),
                mode: HelpMode::Editing,
            },
            HelpItem {
                key: "Ctrl+U".to_string(),
                description: "Clear entire conversation".to_string(),
                mode: HelpMode::Normal,
            },
        ]
    }

    pub fn draw(&self, f: &mut Frame) {
        if !self.visible {
            return;
        }

        let area = f.area();
        let popup_area = Self::centered_rect(60, 70, area);

        // Clear the background
        f.render_widget(Clear, popup_area);

        let help_items = Self::get_help_items();
        let mut text_lines: Vec<Line> = vec![
            Line::from(vec![Span::styled(
                "Housaky TUI - Keyboard Shortcuts",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Normal Mode",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )]),
        ];

        for item in &help_items {
            if item.mode == HelpMode::Normal || item.mode == HelpMode::Both {
                text_lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {:20}", item.key),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(item.description.clone()),
                ]));
            }
        }

        text_lines.push(Line::from(""));
        text_lines.push(Line::from(vec![Span::styled(
            "Editing Mode",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )]));

        for item in &help_items {
            if item.mode == HelpMode::Editing || item.mode == HelpMode::Both {
                text_lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {:20}", item.key),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(item.description.clone()),
                ]));
            }
        }

        text_lines.push(Line::from(""));
        text_lines.push(Line::from(vec![Span::styled(
            "Press any key to close",
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
        )]));

        let help_text = Paragraph::new(text_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" Help (? to toggle) ")
                    .title_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(help_text, popup_area);
    }

    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}

impl Default for HelpPopup {
    fn default() -> Self {
        Self::new()
    }
}

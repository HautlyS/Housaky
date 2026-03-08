//! Command Palette - Autocomplete popup for "/" commands
//!
//! Shows available commands when user types "/" in the input bar.
//! Filters adaptively as user types more characters.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::theme;

/// A command suggestion
#[derive(Debug, Clone)]
pub struct CommandSuggestion {
    pub command: &'static str,
    pub description: &'static str,
    pub shortcut: &'static str,
}

/// All available commands
const COMMANDS: &[CommandSuggestion] = &[
    CommandSuggestion {
        command: "/help",
        description: "Show all commands",
        shortcut: "?",
    },
    CommandSuggestion {
        command: "/clear",
        description: "Clear chat history",
        shortcut: "c",
    },
    CommandSuggestion {
        command: "/status",
        description: "Show system status",
        shortcut: "s",
    },
    CommandSuggestion {
        command: "/quit",
        description: "Exit Housaky",
        shortcut: "q",
    },
    CommandSuggestion {
        command: "/keys",
        description: "Manage API keys",
        shortcut: "k",
    },
    CommandSuggestion {
        command: "/agents",
        description: "Toggle agents panel",
        shortcut: "a",
    },
    CommandSuggestion {
        command: "/agent <name>",
        description: "Select agent",
        shortcut: "",
    },
    CommandSuggestion {
        command: "/provider <name>",
        description: "Switch provider",
        shortcut: "p",
    },
    CommandSuggestion {
        command: "/model <name>",
        description: "Switch model",
        shortcut: "m",
    },
    CommandSuggestion {
        command: "/export",
        description: "Export conversation",
        shortcut: "e",
    },
    CommandSuggestion {
        command: "/save",
        description: "Save conversation",
        shortcut: "",
    },
    CommandSuggestion {
        command: "/config",
        description: "Edit configuration",
        shortcut: "",
    },
    CommandSuggestion {
        command: "/usage",
        description: "Show token usage",
        shortcut: "u",
    },
    CommandSuggestion {
        command: "/doctor",
        description: "Run diagnostics",
        shortcut: "d",
    },
    CommandSuggestion {
        command: "/goals",
        description: "Manage goals",
        shortcut: "g",
    },
    CommandSuggestion {
        command: "/thoughts",
        description: "Show inner monologue",
        shortcut: "t",
    },
    CommandSuggestion {
        command: "/memory",
        description: "Memory operations",
        shortcut: "",
    },
    CommandSuggestion {
        command: "/skills",
        description: "Manage skills",
        shortcut: "",
    },
    CommandSuggestion {
        command: "/fallback",
        description: "Fallback controls",
        shortcut: "f",
    },
];

/// Command palette state
pub struct CommandPalette {
    pub visible: bool,
    pub query: String,
    pub filtered: Vec<&'static CommandSuggestion>,
    pub selected: usize,
    pub max_visible: usize,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            visible: false,
            query: String::new(),
            filtered: COMMANDS.iter().collect(),
            selected: 0,
            max_visible: 8,
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.query.clear();
        self.filtered = COMMANDS.iter().collect();
        self.selected = 0;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }
    }

    pub fn update_query(&mut self, query: &str) {
        self.query = query.to_string();

        if query.is_empty() || query == "/" {
            self.filtered = COMMANDS.iter().collect();
        } else {
            let search = query.trim_start_matches('/').to_lowercase();
            self.filtered = COMMANDS
                .iter()
                .filter(|cmd| {
                    let cmd_lower = cmd.command.to_lowercase();
                    let desc_lower = cmd.description.to_lowercase();
                    cmd_lower.contains(&search) || desc_lower.contains(&search)
                })
                .collect();
        }

        self.selected = 0;
    }

    pub fn next(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 1) % self.filtered.len();
        }
    }

    pub fn prev(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = if self.selected == 0 {
                self.filtered.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn selected_command(&self) -> Option<&CommandSuggestion> {
        self.filtered.get(self.selected).copied()
    }

    pub fn accept(&mut self) -> Option<String> {
        let cmd = self.selected_command()?.command.to_string();
        self.hide();
        Some(cmd)
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        if !self.visible || self.filtered.is_empty() {
            return;
        }

        let visible_count = self.filtered.len().min(self.max_visible);
        let popup_height = visible_count + 2;

        let popup_area = Rect {
            x: area.x,
            y: area.y.saturating_sub(popup_height as u16),
            width: area.width.min(60),
            height: popup_height as u16,
        };

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme::style_border_active())
            .style(theme::style_popup_bg());

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let items: Vec<Line> = self
            .filtered
            .iter()
            .take(visible_count)
            .enumerate()
            .map(|(i, cmd)| {
                let is_selected = i == self.selected;
                let style = if is_selected {
                    theme::style_selected()
                } else {
                    theme::style_base()
                };

                let indicator = if is_selected { "▶ " } else { "  " };

                let cmd_style = if is_selected {
                    Style::default()
                        .fg(theme::Theme::WHITE)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::Theme::CYAN)
                };

                let desc_style = if is_selected {
                    Style::default().fg(theme::Theme::WHITE_DIM)
                } else {
                    Style::default().fg(theme::Theme::WHITE_GHOST)
                };

                let shortcut_style = if is_selected {
                    Style::default().fg(theme::Theme::MAGENTA)
                } else {
                    Style::default().fg(theme::Theme::WHITE_SUBTLE)
                };

                let mut spans = vec![
                    Span::styled(indicator, style),
                    Span::styled(cmd.command, cmd_style),
                    Span::raw(" "),
                    Span::styled(cmd.description, desc_style),
                ];

                if !cmd.shortcut.is_empty() {
                    spans.push(Span::styled(format!(" [{}]", cmd.shortcut), shortcut_style));
                }

                Line::from(spans)
            })
            .collect();

        let paragraph = Paragraph::new(items);
        frame.render_widget(paragraph, inner);
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

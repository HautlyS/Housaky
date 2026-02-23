use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandAction {
    ClearChat,
    ToggleView,
    ShowGoals,
    ShowReasoning,
    ShowState,
    Reflect,
    AddGoal(String),
    ExportChat,
    ToggleAutoScroll,
}

#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub shortcut: Option<String>,
    pub description: String,
    pub action: CommandAction,
    pub category: String,
}

pub struct CommandPalette {
    pub active: bool,
    pub input: String,
    pub commands: Vec<Command>,
    pub filtered: Vec<usize>,
    pub selected: usize,
}

impl CommandPalette {
    pub fn new() -> Self {
        let commands = vec![
            Command {
                name: "Clear chat".to_string(),
                shortcut: Some("Ctrl+U".to_string()),
                description: "Clear all messages in the chat".to_string(),
                action: CommandAction::ClearChat,
                category: "Chat".to_string(),
            },
            Command {
                name: "Toggle view".to_string(),
                shortcut: Some("v".to_string()),
                description: "Cycle through view modes".to_string(),
                action: CommandAction::ToggleView,
                category: "View".to_string(),
            },
            Command {
                name: "Show goals".to_string(),
                shortcut: Some("g".to_string()),
                description: "Switch to goals panel".to_string(),
                action: CommandAction::ShowGoals,
                category: "View".to_string(),
            },
            Command {
                name: "Show reasoning".to_string(),
                shortcut: Some("r".to_string()),
                description: "Switch to reasoning panel".to_string(),
                action: CommandAction::ShowReasoning,
                category: "View".to_string(),
            },
            Command {
                name: "Show state".to_string(),
                shortcut: None,
                description: "Switch to state panel".to_string(),
                action: CommandAction::ShowState,
                category: "View".to_string(),
            },
            Command {
                name: "Reflect".to_string(),
                shortcut: None,
                description: "Trigger self-reflection cycle".to_string(),
                action: CommandAction::Reflect,
                category: "AGI".to_string(),
            },
            Command {
                name: "Export chat".to_string(),
                shortcut: Some("s".to_string()),
                description: "Save conversation to file".to_string(),
                action: CommandAction::ExportChat,
                category: "Chat".to_string(),
            },
            Command {
                name: "Toggle auto-scroll".to_string(),
                shortcut: None,
                description: "Enable/disable auto-scroll".to_string(),
                action: CommandAction::ToggleAutoScroll,
                category: "Chat".to_string(),
            },
            Command {
                name: "Add goal: improve reasoning".to_string(),
                shortcut: None,
                description: "Add a new goal to improve reasoning".to_string(),
                action: CommandAction::AddGoal("Improve reasoning capabilities".to_string()),
                category: "Goals".to_string(),
            },
            Command {
                name: "Add goal: learn new skill".to_string(),
                shortcut: None,
                description: "Add a new goal to learn a skill".to_string(),
                action: CommandAction::AddGoal("Learn a new skill".to_string()),
                category: "Goals".to_string(),
            },
            Command {
                name: "Add goal: create tool".to_string(),
                shortcut: None,
                description: "Add a new goal to create a tool".to_string(),
                action: CommandAction::AddGoal("Create a new tool".to_string()),
                category: "Goals".to_string(),
            },
        ];

        let filtered = (0..commands.len()).collect();

        Self {
            active: false,
            input: String::new(),
            commands,
            filtered,
            selected: 0,
        }
    }

    pub fn activate(&mut self) {
        self.active = true;
        self.input.clear();
        self.filter();
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.input.clear();
        self.selected = 0;
    }

    pub fn filter(&mut self) {
        let input_lower = self.input.to_lowercase();

        self.filtered = self
            .commands
            .iter()
            .enumerate()
            .filter(|(_, cmd)| {
                if input_lower.is_empty() {
                    return true;
                }

                cmd.name.to_lowercase().contains(&input_lower)
                    || cmd.description.to_lowercase().contains(&input_lower)
                    || cmd.category.to_lowercase().contains(&input_lower)
            })
            .map(|(i, _)| i)
            .collect();

        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
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

    pub fn execute(&mut self) -> Option<CommandAction> {
        if let Some(&idx) = self.filtered.get(self.selected) {
            self.deactivate();
            return Some(self.commands[idx].action.clone());
        }
        None
    }

    pub fn draw(&self, f: &mut Frame) {
        if !self.active {
            return;
        }

        let area = Rect::new(
            f.area().width.saturating_sub(60) / 2,
            3,
            60.min(f.area().width),
            15.min(f.area().height - 5),
        );

        f.render_widget(Clear, area);

        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(" Command Palette ")
            .title_style(
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(Style::default().fg(Color::Magenta));

        let input_area = Rect::new(area.x, area.y, area.width, 3);
        let input_inner = input_block.inner(input_area);
        f.render_widget(input_block, input_area);

        let input_prompt = if self.input.is_empty() {
            Paragraph::new("Type to search commands...").style(Style::default().fg(Color::DarkGray))
        } else {
            Paragraph::new(self.input.as_str()).style(Style::default().fg(Color::White))
        };
        f.render_widget(input_prompt, input_inner);

        let list_area = Rect::new(area.x, area.y + 3, area.width, area.height - 3);

        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .enumerate()
            .map(|(i, &idx)| {
                let cmd = &self.commands[idx];
                let is_selected = i == self.selected;

                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Magenta)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let mut spans = vec![Span::styled(&cmd.name, style)];

                if let Some(ref shortcut) = cmd.shortcut {
                    spans.push(Span::styled(
                        format!(" [{}]", shortcut),
                        if is_selected {
                            Style::default().fg(Color::Black)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        },
                    ));
                }

                spans.push(Span::styled(
                    format!(" - {}", cmd.description),
                    if is_selected {
                        Style::default().fg(Color::Black)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                ));

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                .border_style(Style::default().fg(Color::Magenta)),
        );

        f.render_widget(list, list_area);

        let help_text = "↑↓ Navigate | Enter Select | Esc Cancel";
        let help_area = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
        let help = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));
        f.render_widget(help, help_area);
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

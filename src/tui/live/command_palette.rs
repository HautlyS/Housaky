use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub struct CommandPalette {
    input: String,
    commands: Vec<Command>,
    filtered_commands: Vec<Command>,
    selected_index: usize,
    is_active: bool,
}

#[derive(Clone, Debug)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub category: CommandCategory,
    pub aliases: Vec<String>,
    pub requires_arg: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandCategory {
    Navigation,
    Action,
    Query,
    Configuration,
    System,
}

impl CommandPalette {
    pub fn new() -> Self {
        let commands = Self::build_commands();

        Self {
            input: String::new(),
            commands: commands.clone(),
            filtered_commands: commands,
            selected_index: 0,
            is_active: false,
        }
    }

    fn build_commands() -> Vec<Command> {
        vec![
            Command {
                name: "clear".to_string(),
                description: "Clear the conversation history".to_string(),
                category: CommandCategory::Action,
                aliases: vec!["cls".to_string(), "reset".to_string()],
                requires_arg: false,
            },
            Command {
                name: "goals".to_string(),
                description: "View and manage goals".to_string(),
                category: CommandCategory::Navigation,
                aliases: vec!["g".to_string()],
                requires_arg: false,
            },
            Command {
                name: "thoughts".to_string(),
                description: "View the thought stream".to_string(),
                category: CommandCategory::Navigation,
                aliases: vec!["think".to_string(), "th".to_string()],
                requires_arg: false,
            },
            Command {
                name: "metrics".to_string(),
                description: "View AGI performance metrics".to_string(),
                category: CommandCategory::Navigation,
                aliases: vec!["stats".to_string()],
                requires_arg: false,
            },
            Command {
                name: "reflect".to_string(),
                description: "Trigger a reflection cycle".to_string(),
                category: CommandCategory::Action,
                aliases: vec!["ref".to_string()],
                requires_arg: false,
            },
            Command {
                name: "model".to_string(),
                description: "Change the active model".to_string(),
                category: CommandCategory::Configuration,
                aliases: vec!["m".to_string()],
                requires_arg: true,
            },
            Command {
                name: "provider".to_string(),
                description: "Change the LLM provider".to_string(),
                category: CommandCategory::Configuration,
                aliases: vec!["p".to_string()],
                requires_arg: true,
            },
            Command {
                name: "temperature".to_string(),
                description: "Set the temperature".to_string(),
                category: CommandCategory::Configuration,
                aliases: vec!["temp".to_string()],
                requires_arg: true,
            },
            Command {
                name: "save".to_string(),
                description: "Save the current session".to_string(),
                category: CommandCategory::Action,
                aliases: vec!["s".to_string()],
                requires_arg: false,
            },
            Command {
                name: "load".to_string(),
                description: "Load a previous session".to_string(),
                category: CommandCategory::Action,
                aliases: vec!["l".to_string()],
                requires_arg: true,
            },
            Command {
                name: "export".to_string(),
                description: "Export conversation to file".to_string(),
                category: CommandCategory::Action,
                aliases: vec!["ex".to_string()],
                requires_arg: true,
            },
            Command {
                name: "learn".to_string(),
                description: "Learn about a topic".to_string(),
                category: CommandCategory::Query,
                aliases: vec!["study".to_string()],
                requires_arg: true,
            },
            Command {
                name: "search".to_string(),
                description: "Search memories and knowledge".to_string(),
                category: CommandCategory::Query,
                aliases: vec!["find".to_string()],
                requires_arg: true,
            },
            Command {
                name: "skill".to_string(),
                description: "Manage skills".to_string(),
                category: CommandCategory::Action,
                aliases: vec!["skills".to_string()],
                requires_arg: true,
            },
            Command {
                name: "help".to_string(),
                description: "Show help information".to_string(),
                category: CommandCategory::System,
                aliases: vec!["?".to_string(), "h".to_string()],
                requires_arg: false,
            },
            Command {
                name: "quit".to_string(),
                description: "Exit the application".to_string(),
                category: CommandCategory::System,
                aliases: vec!["exit".to_string(), "q".to_string()],
                requires_arg: false,
            },
            Command {
                name: "debug".to_string(),
                description: "Toggle debug mode".to_string(),
                category: CommandCategory::System,
                aliases: vec!["dbg".to_string()],
                requires_arg: false,
            },
            Command {
                name: "status".to_string(),
                description: "Show system status".to_string(),
                category: CommandCategory::Query,
                aliases: vec!["st".to_string()],
                requires_arg: false,
            },
        ]
    }

    pub fn activate(&mut self) {
        self.is_active = true;
        self.input.clear();
        self.filtered_commands = self.commands.clone();
        self.selected_index = 0;
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn handle_input(&mut self, c: char) {
        self.input.push(c);
        self.filter_commands();
    }

    pub fn handle_backspace(&mut self) {
        self.input.pop();
        self.filter_commands();
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected_index < self.filtered_commands.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    fn filter_commands(&mut self) {
        let query = self.input.to_lowercase();

        self.filtered_commands = self
            .commands
            .iter()
            .filter(|cmd| {
                cmd.name.to_lowercase().contains(&query)
                    || cmd.description.to_lowercase().contains(&query)
                    || cmd
                        .aliases
                        .iter()
                        .any(|a| a.to_lowercase().contains(&query))
            })
            .cloned()
            .collect();

        self.selected_index = 0;
    }

    pub fn get_selected_command(&self) -> Option<&Command> {
        self.filtered_commands.get(self.selected_index)
    }

    pub fn execute_selected(&mut self) -> Option<String> {
        let selected = self.get_selected_command()?;
        let command_str = if selected.requires_arg {
            format!("{} ", selected.name)
        } else {
            selected.name.clone()
        };
        self.deactivate();
        Some(command_str)
    }

    pub fn draw(&self, f: &mut Frame, area: Rect) {
        if !self.is_active {
            return;
        }

        let popup_area = Rect {
            x: area.width / 4,
            y: area.height / 3,
            width: area.width / 2,
            height: (self.filtered_commands.len() + 3).min(15) as u16,
        };

        f.render_widget(Clear, popup_area);

        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Yellow)),
            Span::styled(&self.input, Style::default().fg(Color::White)),
            Span::styled(
                "▌",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
        ]));
        lines.push(Line::from(""));

        let visible_commands: Vec<_> = self
            .filtered_commands
            .iter()
            .skip(self.selected_index.saturating_sub(5))
            .take(10)
            .collect();

        for (i, cmd) in visible_commands.iter().enumerate() {
            let actual_index = self.selected_index.saturating_sub(5) + i;
            let is_selected = actual_index == self.selected_index;

            let category_color = match cmd.category {
                CommandCategory::Navigation => Color::Cyan,
                CommandCategory::Action => Color::Green,
                CommandCategory::Query => Color::Yellow,
                CommandCategory::Configuration => Color::Magenta,
                CommandCategory::System => Color::Red,
            };

            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else {
                Style::default()
            };

            let alias_str = if cmd.aliases.is_empty() {
                String::new()
            } else {
                format!(" ({})", cmd.aliases.join(", "))
            };

            lines.push(Line::from(vec![
                Span::styled(if is_selected { "→ " } else { "  " }, style),
                Span::styled(&cmd.name, style.add_modifier(Modifier::BOLD)),
                Span::styled(alias_str, Style::default().fg(category_color)),
                Span::styled(" - ", style),
                Span::styled(&cmd.description, style),
            ]));
        }

        if self.filtered_commands.is_empty() {
            lines.push(Line::from(Span::styled(
                "No matching commands",
                Style::default().fg(Color::Red),
            )));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Command Palette")
            .style(Style::default().fg(Color::Cyan));

        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, popup_area);
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

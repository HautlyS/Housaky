use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandAction {
    // Chat
    ClearChat,
    ExportChat,
    ToggleAutoScroll,
    // View
    ToggleView,
    // AGI
    ShowGoals,
    ShowReasoning,
    ShowState,
    Reflect,
    AddGoal(String),
    // Keys
    ListKeys,
    AddKey(String),
    RotateKey,
    ShowKeyStats,
    // Skills (new)
    ShowSkills,
    EnableSkill(String),
    DisableSkill(String),
    // Tools (new)
    ShowTools,
    ClearToolLog,
    // Metrics
    ShowMetrics,
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
            // ── Chat ──────────────────────────────────────────────────────────
            Command {
                name: "Clear chat".to_string(),
                shortcut: Some("Ctrl+U".to_string()),
                description: "Clear all messages in the current conversation".to_string(),
                action: CommandAction::ClearChat,
                category: "Chat".to_string(),
            },
            Command {
                name: "Export chat".to_string(),
                shortcut: Some("s".to_string()),
                description: "Save conversation as Markdown file".to_string(),
                action: CommandAction::ExportChat,
                category: "Chat".to_string(),
            },
            Command {
                name: "Toggle auto-scroll".to_string(),
                shortcut: None,
                description: "Enable/disable automatic scrolling to latest message".to_string(),
                action: CommandAction::ToggleAutoScroll,
                category: "Chat".to_string(),
            },
            // ── View ──────────────────────────────────────────────────────────
            Command {
                name: "Cycle view".to_string(),
                shortcut: Some("v".to_string()),
                description: "Switch between Full, Split, and Dashboard view modes".to_string(),
                action: CommandAction::ToggleView,
                category: "View".to_string(),
            },
            // ── AGI ───────────────────────────────────────────────────────────
            Command {
                name: "Show goals".to_string(),
                shortcut: Some("g".to_string()),
                description: "Switch to Goals tab".to_string(),
                action: CommandAction::ShowGoals,
                category: "AGI".to_string(),
            },
            Command {
                name: "Show reasoning".to_string(),
                shortcut: Some("r".to_string()),
                description: "Switch to Reasoning chain tab".to_string(),
                action: CommandAction::ShowReasoning,
                category: "AGI".to_string(),
            },
            Command {
                name: "Reflect".to_string(),
                shortcut: None,
                description: "Trigger meta-cognition self-reflection cycle".to_string(),
                action: CommandAction::Reflect,
                category: "AGI".to_string(),
            },
            Command {
                name: "Add goal: improve reasoning".to_string(),
                shortcut: None,
                description: "Add goal to improve reasoning capabilities".to_string(),
                action: CommandAction::AddGoal("Improve reasoning capabilities".to_string()),
                category: "AGI".to_string(),
            },
            Command {
                name: "Add goal: learn new skill".to_string(),
                shortcut: None,
                description: "Add goal to learn and integrate a new skill".to_string(),
                action: CommandAction::AddGoal("Learn and integrate a new skill".to_string()),
                category: "AGI".to_string(),
            },
            Command {
                name: "Add goal: create tool".to_string(),
                shortcut: None,
                description: "Add goal to create a new autonomous tool".to_string(),
                action: CommandAction::AddGoal("Create a new autonomous tool".to_string()),
                category: "AGI".to_string(),
            },
            Command {
                name: "Add goal: reduce errors".to_string(),
                shortcut: None,
                description: "Add goal to reduce error rate below 1%".to_string(),
                action: CommandAction::AddGoal("Reduce error rate below 1%".to_string()),
                category: "AGI".to_string(),
            },
            // ── Keys ──────────────────────────────────────────────────────────
            Command {
                name: "List API keys".to_string(),
                shortcut: None,
                description: "Open Keys tab showing all configured providers".to_string(),
                action: CommandAction::ListKeys,
                category: "Keys".to_string(),
            },
            Command {
                name: "Rotate API key".to_string(),
                shortcut: None,
                description: "Rotate to next available API key for selected provider".to_string(),
                action: CommandAction::RotateKey,
                category: "Keys".to_string(),
            },
            Command {
                name: "Show key stats".to_string(),
                shortcut: None,
                description: "Open Keys tab showing usage statistics per provider".to_string(),
                action: CommandAction::ShowKeyStats,
                category: "Keys".to_string(),
            },
            Command {
                name: "Add API key".to_string(),
                shortcut: None,
                description: "Instructions to add an API key via CLI".to_string(),
                action: CommandAction::AddKey(String::new()),
                category: "Keys".to_string(),
            },
            // ── Skills ────────────────────────────────────────────────────────
            Command {
                name: "Show skills".to_string(),
                shortcut: Some("2".to_string()),
                description: "Open Skills tab to browse and enable/disable skills".to_string(),
                action: CommandAction::ShowSkills,
                category: "Skills".to_string(),
            },
            // ── Tools ─────────────────────────────────────────────────────────
            Command {
                name: "Show tool log".to_string(),
                shortcut: Some("6".to_string()),
                description: "Open Tools tab to view execution history".to_string(),
                action: CommandAction::ShowTools,
                category: "Tools".to_string(),
            },
            Command {
                name: "Clear tool log".to_string(),
                shortcut: None,
                description: "Clear the tool execution history".to_string(),
                action: CommandAction::ClearToolLog,
                category: "Tools".to_string(),
            },
            // ── Metrics ───────────────────────────────────────────────────────
            Command {
                name: "Show metrics".to_string(),
                shortcut: Some("7".to_string()),
                description: "Open Metrics tab with AGI performance statistics".to_string(),
                action: CommandAction::ShowMetrics,
                category: "Metrics".to_string(),
            },
            Command {
                name: "Show state".to_string(),
                shortcut: None,
                description: "Open Metrics/state dashboard".to_string(),
                action: CommandAction::ShowState,
                category: "Metrics".to_string(),
            },
        ];

        let filtered: Vec<usize> = (0..commands.len()).collect();
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
        self.filtered = (0..self.commands.len()).collect();
        self.selected = 0;
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.input.clear();
    }

    pub fn filter(&mut self) {
        let q = self.input.to_lowercase();
        self.filtered = (0..self.commands.len())
            .filter(|&i| {
                let cmd = &self.commands[i];
                let haystack = format!(
                    "{} {} {} {}",
                    cmd.name.to_lowercase(),
                    cmd.description.to_lowercase(),
                    cmd.category.to_lowercase(),
                    cmd.shortcut.as_deref().unwrap_or("")
                );
                // fuzzy: all chars of query appear in order in haystack
                if q.is_empty() { return true; }
                let mut chars = q.chars().peekable();
                for c in haystack.chars() {
                    if chars.peek() == Some(&c) {
                        chars.next();
                    }
                }
                chars.peek().is_none()
            })
            .collect();
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

    pub fn execute(&mut self) -> Option<CommandAction> {
        let idx = self.filtered.get(self.selected)?;
        let action = self.commands[*idx].action.clone();
        self.deactivate();
        Some(action)
    }

    pub fn draw(&self, f: &mut Frame) {
        if !self.active { return; }

        let area = f.area();
        let width = 64u16.min(area.width.saturating_sub(4));
        let max_items = 12usize;
        let visible = self.filtered.len().min(max_items);
        let height = (visible as u16 + 4).min(area.height.saturating_sub(4));

        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            3,
            width,
            height,
        );

        f.render_widget(Clear, popup);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(popup);

        // Input box
        let input_block = Paragraph::new(self.input.as_str())
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(Span::styled(
                        " ⌘  Command Palette — type to filter ",
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    )),
            );
        f.render_widget(input_block, layout[0]);

        // Results list
        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .enumerate()
            .take(max_items)
            .map(|(display_idx, &cmd_idx)| {
                let cmd = &self.commands[cmd_idx];
                let selected = display_idx == self.selected;

                let cat_color = match cmd.category.as_str() {
                    "Chat"    => Color::Cyan,
                    "AGI"     => Color::Magenta,
                    "Keys"    => Color::Yellow,
                    "Skills"  => Color::Green,
                    "Tools"   => Color::Blue,
                    "Metrics" => Color::White,
                    "View"    => Color::Gray,
                    _         => Color::DarkGray,
                };

                let row_style = if selected {
                    Style::default().bg(Color::Rgb(30, 30, 60)).fg(Color::White)
                } else {
                    Style::default().fg(Color::White)
                };

                let shortcut_span = cmd.shortcut.as_deref().map(|s| {
                    Span::styled(
                        format!(" [{}]", s),
                        Style::default().fg(Color::DarkGray),
                    )
                });

                let mut spans = vec![
                    Span::styled(
                        format!(" {:8} ", cmd.category),
                        Style::default().fg(cat_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:30}", cmd.name),
                        row_style.add_modifier(if selected { Modifier::BOLD } else { Modifier::empty() }),
                    ),
                    Span::styled(
                        format!("  {}", cmd.description.chars().take(20).collect::<String>()),
                        Style::default().fg(Color::DarkGray),
                    ),
                ];
                if let Some(s) = shortcut_span {
                    spans.push(s);
                }
                if selected {
                    spans.insert(0, Span::styled("▶ ", Style::default().fg(Color::Cyan)));
                } else {
                    spans.insert(0, Span::raw("  "));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(Span::styled(
                        format!(" {} results — ↑↓ navigate  Enter execute  Esc close ", self.filtered.len()),
                        Style::default().fg(Color::DarkGray),
                    )),
            )
            .highlight_style(Style::default().bg(Color::Rgb(30, 30, 60)));

        f.render_widget(list, layout[1]);
    }
}

impl Default for CommandPalette {
    fn default() -> Self { Self::new() }
}

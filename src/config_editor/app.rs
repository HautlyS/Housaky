use anyhow::Result;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::menu::{MenuCategory, MENU_CATEGORIES};
use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    Normal,
    Editing,
}

#[allow(clippy::struct_excessive_bools)]
pub struct ConfigEditorApp {
    pub config: Config,
    pub original_config: Config,
    pub menu_state: ListState,
    pub active_category: MenuCategory,
    pub mode: EditorMode,
    pub selected_field: usize,
    pub input_buffer: String,
    pub dirty: bool,
    pub should_quit: bool,
    pub show_help: bool,
    pub show_save_prompt: bool,
    pub confirmed_exit: bool,
    pub error_message: Option<String>,
    pub success_message: Option<String>,
}

impl ConfigEditorApp {
    pub fn new(config: Config, section: Option<String>) -> Self {
        let initial_category = section
            .and_then(|s| MENU_CATEGORIES.iter().find(|c| c.id() == s))
            .copied()
            .unwrap_or(MENU_CATEGORIES[0]);

        let mut menu_state = ListState::default();
        menu_state.select(Some(0));

        Self {
            original_config: config.clone(),
            config,
            menu_state,
            active_category: initial_category,
            mode: EditorMode::Normal,
            selected_field: 0,
            input_buffer: String::new(),
            dirty: false,
            should_quit: false,
            show_help: false,
            show_save_prompt: false,
            confirmed_exit: false,
            error_message: None,
            success_message: None,
        }
    }

    pub fn next_section(&mut self) {
        let current_idx = MENU_CATEGORIES
            .iter()
            .position(|c| *c == self.active_category)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % MENU_CATEGORIES.len();
        self.active_category = MENU_CATEGORIES[next_idx];
        self.menu_state.select(Some(next_idx));
        self.selected_field = 0;
    }

    pub fn prev_section(&mut self) {
        let current_idx = MENU_CATEGORIES
            .iter()
            .position(|c| *c == self.active_category)
            .unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            MENU_CATEGORIES.len() - 1
        } else {
            current_idx - 1
        };
        self.active_category = MENU_CATEGORIES[prev_idx];
        self.menu_state.select(Some(prev_idx));
        self.selected_field = 0;
    }

    pub fn move_up(&mut self) {
        if self.selected_field > 0 {
            self.selected_field -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let field_count = self.get_field_count();
        if self.selected_field < field_count.saturating_sub(1) {
            self.selected_field += 1;
        }
    }

    pub fn edit_field(&mut self) -> Result<()> {
        self.mode = EditorMode::Editing;
        self.input_buffer = self.get_current_field_value();
        Ok(())
    }

    pub fn handle_char(&mut self, c: char) -> Result<()> {
        if self.mode == EditorMode::Editing {
            self.input_buffer.push(c);
        }
        Ok(())
    }

    pub fn handle_backspace(&mut self) {
        if self.mode == EditorMode::Editing {
            self.input_buffer.pop();
        }
    }

    pub fn save(&mut self) -> Result<()> {
        self.config.save()?;
        self.dirty = false;
        self.original_config = self.config.clone();
        self.success_message = Some("Configuration saved successfully!".to_string());
        Ok(())
    }

    fn get_field_count(&self) -> usize {
        match self.active_category {
            MenuCategory::Agent => 9,
            MenuCategory::Tools => 7,
            MenuCategory::Channels => 6,
            MenuCategory::Gateway => 12,
            MenuCategory::Memory => 8,
            MenuCategory::Providers | MenuCategory::Fallback => 3,
            MenuCategory::Security | MenuCategory::Cost => 4,
        }
    }

    fn get_current_field_value(&self) -> String {
        match self.active_category {
            MenuCategory::Agent => match self.selected_field {
                0 => format!("{}", self.config.agent.max_tool_iterations),
                1 => format!("{}", self.config.agent.max_history_messages),
                2 => format!("{}", self.config.agent.parallel_tools),
                3 => format!("{}", self.config.agent.compaction_keep_recent_messages),
                4 => format!("{}", self.config.agent.compaction_max_source_chars),
                5 => format!("{}", self.config.agent.compaction_max_summary_chars),
                6 => format!("{}", self.config.agent.bootstrap_max_chars),
                7 => self.config.agent.tool_dispatcher.clone(),
                8 => format!("{}", self.config.agent.compact_context),
                _ => String::new(),
            },
            MenuCategory::Tools => match self.selected_field {
                0 => format!("{}", self.config.tools.shell_timeout_secs),
                1 => format!("{}", self.config.tools.shell_max_output_bytes),
                2 => format!("{}", self.config.tools.file_read_max_bytes),
                3 => format!("{}", self.config.tools.delegate_timeout_secs),
                4 => format!("{}", self.config.tools.screenshot_timeout_secs),
                5 => format!("{}", self.config.tools.screenshot_max_bytes),
                6 => format!("{}", self.config.tools.image_max_bytes),
                _ => String::new(),
            },
            MenuCategory::Channels => match self.selected_field {
                0 => format!("{}", self.config.channels_config.message_timeout_secs),
                1 => format!("{}", self.config.channels_config.parallelism_per_channel),
                2 => format!("{}", self.config.channels_config.min_in_flight_messages),
                3 => format!("{}", self.config.channels_config.max_in_flight_messages),
                4 => format!("{}", self.config.channels_config.history_turns),
                5 => format!("{}", self.config.channels_config.bootstrap_max_chars),
                _ => String::new(),
            },
            MenuCategory::Gateway => match self.selected_field {
                0 => format!("{}", self.config.gateway.port),
                1 => self.config.gateway.host.clone(),
                2 => format!("{}", self.config.gateway.require_pairing),
                3 => format!("{}", self.config.gateway.max_body_size),
                4 => format!("{}", self.config.gateway.request_timeout_secs),
                5 => format!("{}", self.config.gateway.rate_limit_window_secs),
                6 => format!("{}", self.config.gateway.pair_rate_limit_per_minute),
                7 => format!("{}", self.config.gateway.webhook_rate_limit_per_minute),
                8 => format!("{}", self.config.gateway.pairing_max_attempts),
                9 => format!("{}", self.config.gateway.pairing_lockout_secs),
                10 => format!("{}", self.config.gateway.idempotency_ttl_secs),
                11 => format!("{}", self.config.gateway.allow_public_bind),
                _ => String::new(),
            },
            MenuCategory::Memory => match self.selected_field {
                0 => self.config.memory.backend.clone(),
                1 => format!("{}", self.config.memory.auto_save),
                2 => format!("{}", self.config.memory.hygiene_enabled),
                3 => format!("{}", self.config.memory.archive_after_days),
                4 => format!("{}", self.config.memory.purge_after_days),
                5 => format!("{}", self.config.memory.conversation_retention_days),
                6 => self.config.memory.embedding_provider.clone(),
                7 => format!("{}", self.config.memory.embedding_cache_size),
                _ => String::new(),
            },
            MenuCategory::Providers => match self.selected_field {
                0 => self.config.default_provider.clone().unwrap_or_default(),
                1 => self.config.default_model.clone().unwrap_or_default(),
                2 => format!("{}", self.config.default_temperature),
                _ => String::new(),
            },
            MenuCategory::Fallback => match self.selected_field {
                0 => format!("{}", self.config.fallback.enabled),
                1 => format!("{}", self.config.fallback.rotate_at_percent),
                2 => format!("{}", self.config.fallback.rotate_on_rate_limit),
                _ => String::new(),
            },
            MenuCategory::Security => match self.selected_field {
                0 => format!("{}", self.config.secrets.encrypt),
                1 => format!("{}", self.config.autonomy.workspace_only),
                2 => format!("{}", self.config.autonomy.max_actions_per_hour),
                3 => format!("{}", self.config.autonomy.max_cost_per_day_cents),
                _ => String::new(),
            },
            MenuCategory::Cost => match self.selected_field {
                0 => format!("{}", self.config.cost.enabled),
                1 => format!("{}", self.config.cost.daily_limit_usd),
                2 => format!("{}", self.config.cost.monthly_limit_usd),
                3 => format!("{}", self.config.cost.warn_at_percent),
                _ => String::new(),
            },
        }
    }

    pub fn draw(&mut self, f: &mut Frame) {
        let size = f.area();

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(20), Constraint::Min(0)])
            .split(size);

        self.draw_menu(f, chunks[0]);
        self.draw_editor(f, chunks[1]);

        if self.show_help {
            self.draw_help_popup(f);
        }

        if self.show_save_prompt {
            self.draw_save_prompt(f);
        }
    }

    fn draw_menu(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = MENU_CATEGORIES
            .iter()
            .map(|c| {
                let style = if *c == self.active_category {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(format!("  {}", c.label())).style(style)
            })
            .collect();

        let menu = List::new(items)
            .block(
                Block::default()
                    .title(" Configuration ")
                    .borders(Borders::ALL),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_stateful_widget(menu, area, &mut self.menu_state);
    }

    fn draw_editor(&mut self, f: &mut Frame, area: Rect) {
        let title = format!(" {} Settings ", self.active_category.label());

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(3)])
            .split(area);

        let content = self.get_editor_content();
        let editor = Paragraph::new(content)
            .block(Block::default().title(title).borders(Borders::ALL))
            .wrap(ratatui::widgets::Wrap { trim: false });
        f.render_widget(editor, chunks[0]);

        let status_text = if self.mode == EditorMode::Editing {
            format!(
                "Editing: {} (Enter to save, Esc to cancel)",
                self.input_buffer
            )
        } else if let Some(ref msg) = self.success_message {
            msg.clone()
        } else if let Some(ref err) = self.error_message {
            format!("Error: {}", err)
        } else {
            "[q] Quit  [s] Save  [?] Help  [↑↓] Navigate  [Enter] Edit  [Tab] Next Section"
                .to_string()
        };

        let status = Paragraph::new(status_text)
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(status, chunks[1]);
    }

    fn get_editor_content(&self) -> String {
        let mut lines = Vec::new();

        match self.active_category {
            MenuCategory::Agent => {
                lines.push(self.format_field(
                    0,
                    "Max Tool Iterations",
                    &format!("{}", self.config.agent.max_tool_iterations),
                ));
                lines.push(self.format_field(
                    1,
                    "Max History Messages",
                    &format!("{}", self.config.agent.max_history_messages),
                ));
                lines.push(self.format_field(
                    2,
                    "Parallel Tools",
                    &format!("{}", self.config.agent.parallel_tools),
                ));
                lines.push(self.format_field(
                    3,
                    "Compaction Keep Recent",
                    &format!("{}", self.config.agent.compaction_keep_recent_messages),
                ));
                lines.push(self.format_field(
                    4,
                    "Compaction Max Source",
                    &format!("{}", self.config.agent.compaction_max_source_chars),
                ));
                lines.push(self.format_field(
                    5,
                    "Compaction Max Summary",
                    &format!("{}", self.config.agent.compaction_max_summary_chars),
                ));
                lines.push(self.format_field(
                    6,
                    "Bootstrap Max Chars",
                    &format!("{}", self.config.agent.bootstrap_max_chars),
                ));
                lines.push(self.format_field(
                    7,
                    "Tool Dispatcher",
                    &self.config.agent.tool_dispatcher,
                ));
                lines.push(self.format_field(
                    8,
                    "Compact Context",
                    &format!("{}", self.config.agent.compact_context),
                ));
            }
            MenuCategory::Tools => {
                lines.push(self.format_field(
                    0,
                    "Shell Timeout (secs)",
                    &format!("{}", self.config.tools.shell_timeout_secs),
                ));
                lines.push(self.format_field(
                    1,
                    "Shell Max Output (bytes)",
                    &format!("{}", self.config.tools.shell_max_output_bytes),
                ));
                lines.push(self.format_field(
                    2,
                    "File Read Max (bytes)",
                    &format!("{}", self.config.tools.file_read_max_bytes),
                ));
                lines.push(self.format_field(
                    3,
                    "Delegate Timeout (secs)",
                    &format!("{}", self.config.tools.delegate_timeout_secs),
                ));
                lines.push(self.format_field(
                    4,
                    "Screenshot Timeout (secs)",
                    &format!("{}", self.config.tools.screenshot_timeout_secs),
                ));
                lines.push(self.format_field(
                    5,
                    "Screenshot Max (bytes)",
                    &format!("{}", self.config.tools.screenshot_max_bytes),
                ));
                lines.push(self.format_field(
                    6,
                    "Image Max (bytes)",
                    &format!("{}", self.config.tools.image_max_bytes),
                ));
            }
            MenuCategory::Channels => {
                lines.push(self.format_field(
                    0,
                    "Message Timeout (secs)",
                    &format!("{}", self.config.channels_config.message_timeout_secs),
                ));
                lines.push(self.format_field(
                    1,
                    "Parallelism/Channel",
                    &format!("{}", self.config.channels_config.parallelism_per_channel),
                ));
                lines.push(self.format_field(
                    2,
                    "Min In-Flight",
                    &format!("{}", self.config.channels_config.min_in_flight_messages),
                ));
                lines.push(self.format_field(
                    3,
                    "Max In-Flight",
                    &format!("{}", self.config.channels_config.max_in_flight_messages),
                ));
                lines.push(self.format_field(
                    4,
                    "History Turns",
                    &format!("{}", self.config.channels_config.history_turns),
                ));
                lines.push(self.format_field(
                    5,
                    "Bootstrap Max Chars",
                    &format!("{}", self.config.channels_config.bootstrap_max_chars),
                ));
            }
            MenuCategory::Gateway => {
                lines.push(self.format_field(0, "Port", &format!("{}", self.config.gateway.port)));
                lines.push(self.format_field(1, "Host", &self.config.gateway.host));
                lines.push(self.format_field(
                    2,
                    "Require Pairing",
                    &format!("{}", self.config.gateway.require_pairing),
                ));
                lines.push(self.format_field(
                    3,
                    "Max Body Size",
                    &format!("{}", self.config.gateway.max_body_size),
                ));
                lines.push(self.format_field(
                    4,
                    "Request Timeout (secs)",
                    &format!("{}", self.config.gateway.request_timeout_secs),
                ));
                lines.push(self.format_field(
                    5,
                    "Rate Limit Window (secs)",
                    &format!("{}", self.config.gateway.rate_limit_window_secs),
                ));
                lines.push(self.format_field(
                    6,
                    "Pair Rate Limit/min",
                    &format!("{}", self.config.gateway.pair_rate_limit_per_minute),
                ));
                lines.push(self.format_field(
                    7,
                    "Webhook Rate Limit/min",
                    &format!("{}", self.config.gateway.webhook_rate_limit_per_minute),
                ));
                lines.push(self.format_field(
                    8,
                    "Pairing Max Attempts",
                    &format!("{}", self.config.gateway.pairing_max_attempts),
                ));
                lines.push(self.format_field(
                    9,
                    "Pairing Lockout (secs)",
                    &format!("{}", self.config.gateway.pairing_lockout_secs),
                ));
                lines.push(self.format_field(
                    10,
                    "Idempotency TTL (secs)",
                    &format!("{}", self.config.gateway.idempotency_ttl_secs),
                ));
                lines.push(self.format_field(
                    11,
                    "Allow Public Bind",
                    &format!("{}", self.config.gateway.allow_public_bind),
                ));
            }
            MenuCategory::Memory => {
                lines.push(self.format_field(0, "Backend", &self.config.memory.backend));
                lines.push(self.format_field(
                    1,
                    "Auto Save",
                    &format!("{}", self.config.memory.auto_save),
                ));
                lines.push(self.format_field(
                    2,
                    "Hygiene Enabled",
                    &format!("{}", self.config.memory.hygiene_enabled),
                ));
                lines.push(self.format_field(
                    3,
                    "Archive After (days)",
                    &format!("{}", self.config.memory.archive_after_days),
                ));
                lines.push(self.format_field(
                    4,
                    "Purge After (days)",
                    &format!("{}", self.config.memory.purge_after_days),
                ));
                lines.push(self.format_field(
                    5,
                    "Conversation Retention (days)",
                    &format!("{}", self.config.memory.conversation_retention_days),
                ));
                lines.push(self.format_field(
                    6,
                    "Embedding Provider",
                    &self.config.memory.embedding_provider,
                ));
                lines.push(self.format_field(
                    7,
                    "Embedding Cache Size",
                    &format!("{}", self.config.memory.embedding_cache_size),
                ));
            }
            MenuCategory::Providers => {
                lines.push(self.format_field(
                    0,
                    "Default Provider",
                    &self.config.default_provider.clone().unwrap_or_default(),
                ));
                lines.push(self.format_field(
                    1,
                    "Default Model",
                    &self.config.default_model.clone().unwrap_or_default(),
                ));
                lines.push(self.format_field(
                    2,
                    "Default Temperature",
                    &format!("{}", self.config.default_temperature),
                ));
            }
            MenuCategory::Fallback => {
                lines.push(self.format_field(
                    0,
                    "Fallback Enabled",
                    &format!("{}", self.config.fallback.enabled),
                ));
                lines.push(self.format_field(
                    1,
                    "Rotate At Percent",
                    &format!("{}", self.config.fallback.rotate_at_percent),
                ));
                lines.push(self.format_field(
                    2,
                    "Rotate On Rate Limit",
                    &format!("{}", self.config.fallback.rotate_on_rate_limit),
                ));
            }
            MenuCategory::Security => {
                lines.push(self.format_field(
                    0,
                    "Encrypt Secrets",
                    &format!("{}", self.config.secrets.encrypt),
                ));
                lines.push(self.format_field(
                    1,
                    "Workspace Only",
                    &format!("{}", self.config.autonomy.workspace_only),
                ));
                lines.push(self.format_field(
                    2,
                    "Max Actions/Hour",
                    &format!("{}", self.config.autonomy.max_actions_per_hour),
                ));
                lines.push(self.format_field(
                    3,
                    "Max Cost/Day (cents)",
                    &format!("{}", self.config.autonomy.max_cost_per_day_cents),
                ));
            }
            MenuCategory::Cost => {
                lines.push(self.format_field(
                    0,
                    "Cost Tracking Enabled",
                    &format!("{}", self.config.cost.enabled),
                ));
                lines.push(self.format_field(
                    1,
                    "Daily Limit (USD)",
                    &format!("{}", self.config.cost.daily_limit_usd),
                ));
                lines.push(self.format_field(
                    2,
                    "Monthly Limit (USD)",
                    &format!("{}", self.config.cost.monthly_limit_usd),
                ));
                lines.push(self.format_field(
                    3,
                    "Warn At Percent",
                    &format!("{}", self.config.cost.warn_at_percent),
                ));
            }
        }

        lines.join("\n")
    }

    fn format_field(&self, index: usize, label: &str, value: &str) -> String {
        let marker = if index == self.selected_field {
            "►"
        } else {
            " "
        };
        format!("{} {:30} {}", marker, label, value)
    }

    fn draw_help_popup(&self, f: &mut Frame) {
        let area = centered_rect(60, 50, f.area());
        f.render_widget(Clear, area);

        let help_text = vec![
            "Keyboard Shortcuts:",
            "",
            "  ↑/↓     Navigate fields",
            "  Enter   Edit selected field",
            "  Tab     Next section",
            "  Shift+Tab  Previous section",
            "  s       Save configuration",
            "  q       Quit (prompts to save if dirty)",
            "  ?       Toggle this help",
            "  Esc     Cancel editing / Close popup",
            "  Ctrl+C  Force quit",
            "",
            "When editing:",
            "  Type to modify value",
            "  Enter to confirm",
            "  Esc to cancel",
        ];

        let help: Vec<Line> = help_text.iter().map(|s| Line::from(*s)).collect();

        let paragraph = Paragraph::new(help)
            .block(Block::default().title(" Help ").borders(Borders::ALL))
            .wrap(ratatui::widgets::Wrap { trim: false });
        f.render_widget(paragraph, area);
    }

    fn draw_save_prompt(&self, f: &mut Frame) {
        let area = centered_rect(40, 20, f.area());
        f.render_widget(Clear, area);

        let text = vec![
            Line::from(""),
            Line::from("  You have unsaved changes. Save before exit?"),
            Line::from(""),
            Line::from("  [y] Yes   [n] No   [Esc] Cancel"),
        ];

        let paragraph =
            Paragraph::new(text).block(Block::default().title(" Save? ").borders(Borders::ALL));
        f.render_widget(paragraph, area);
    }
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

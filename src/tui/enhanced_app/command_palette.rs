use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use crate::tui::enhanced_app::theme::{Palette, style_border_focus, style_dim, style_muted, style_title};

// ── Command action ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteAction {
    // Chat
    ClearChat,
    ExportChat,
    CopyLastResponse,
    ToggleAutoScroll,
    // View
    CycleView,
    ToggleSidebar,
    // Tabs
    GotoChat,
    GotoSkills,
    GotoTools,
    GotoGoals,
    GotoMetrics,
    GotoConfig,
    // AGI
    Reflect,
    AddGoal(String),
    AddGoalStatic(&'static str),
    // Provider
    SwitchModel(String),
    // Settings
    OpenHelp,
    Quit,
}

// ── Command definition ────────────────────────────────────────────────────────

struct PaletteCommand {
    name:     &'static str,
    shortcut: Option<&'static str>,
    desc:     &'static str,
    category: &'static str,
    action:   PaletteAction,
}

static COMMANDS: &[PaletteCommand] = &[
    // ── Chat ─────────────────────────────────────────────────────────────────
    PaletteCommand { name: "Clear chat",           shortcut: Some("Ctrl+U"), desc: "Erase all messages",            category: "Chat",    action: PaletteAction::ClearChat },
    PaletteCommand { name: "Export chat",          shortcut: Some("s"),      desc: "Save as markdown file",         category: "Chat",    action: PaletteAction::ExportChat },
    PaletteCommand { name: "Copy last response",   shortcut: Some("c"),      desc: "Copy assistant reply",          category: "Chat",    action: PaletteAction::CopyLastResponse },
    PaletteCommand { name: "Toggle auto-scroll",   shortcut: Some("a"),      desc: "Lock scroll to latest",         category: "Chat",    action: PaletteAction::ToggleAutoScroll },
    // ── View ──────────────────────────────────────────────────────────────────
    PaletteCommand { name: "Cycle view mode",      shortcut: Some("v"),      desc: "Full / Split / Dashboard",      category: "View",    action: PaletteAction::CycleView },
    PaletteCommand { name: "Toggle sidebar",       shortcut: Some("b"),      desc: "Show/hide right panel",         category: "View",    action: PaletteAction::ToggleSidebar },
    // ── Tabs ──────────────────────────────────────────────────────────────────
    PaletteCommand { name: "Go to Chat",           shortcut: Some("1"),      desc: "Switch to Chat tab",            category: "Tab",     action: PaletteAction::GotoChat },
    PaletteCommand { name: "Go to Skills",         shortcut: Some("2"),      desc: "Browse & enable skills",        category: "Tab",     action: PaletteAction::GotoSkills },
    PaletteCommand { name: "Go to Tools",          shortcut: Some("3"),      desc: "Tool execution log",            category: "Tab",     action: PaletteAction::GotoTools },
    PaletteCommand { name: "Go to Goals",          shortcut: Some("4"),      desc: "Active AGI goals",              category: "Tab",     action: PaletteAction::GotoGoals },
    PaletteCommand { name: "Go to Metrics",        shortcut: Some("5"),      desc: "Session statistics",            category: "Tab",     action: PaletteAction::GotoMetrics },
    PaletteCommand { name: "Config editor",         shortcut: Some("6"),      desc: "Edit ~/.housaky/config.toml",    category: "Tab",     action: PaletteAction::GotoConfig },
    // ── AGI ───────────────────────────────────────────────────────────────────
    PaletteCommand { name: "Reflect",              shortcut: None,           desc: "Meta-cognition cycle",          category: "AGI",     action: PaletteAction::Reflect },
    PaletteCommand { name: "Add goal: productivity", shortcut: None,         desc: "Add productivity goal",         category: "AGI",     action: PaletteAction::AddGoalStatic("Improve productivity and execution speed") },
    PaletteCommand { name: "Add goal: learn skill",  shortcut: None,         desc: "Add skill-learning goal",       category: "AGI",     action: PaletteAction::AddGoalStatic("Learn and integrate a new skill") },
    // ── System ────────────────────────────────────────────────────────────────
    PaletteCommand { name: "Help",                 shortcut: Some("?"),      desc: "Keyboard shortcuts reference",  category: "System",  action: PaletteAction::OpenHelp },
    PaletteCommand { name: "Quit",                 shortcut: Some("q"),      desc: "Exit Housaky TUI",              category: "System",  action: PaletteAction::Quit },
];

fn cat_color(cat: &str) -> ratatui::style::Color {
    match cat {
        "Chat"   => Palette::CYAN,
        "View"   => Palette::TEXT_DIM,
        "Tab"    => Palette::ASSISTANT,
        "AGI"    => Palette::VIOLET,
        "System" => Palette::WARNING,
        _        => Palette::TEXT_DIM,
    }
}

// ── Palette state ─────────────────────────────────────────────────────────────

pub struct CommandPalette {
    pub active:   bool,
    pub input:    String,
    filtered:     Vec<usize>,
    selected:     usize,
}

impl CommandPalette {
    pub fn new() -> Self {
        let filtered: Vec<usize> = (0..COMMANDS.len()).collect();
        Self {
            active:   false,
            input:    String::new(),
            filtered,
            selected: 0,
        }
    }

    pub fn open(&mut self) {
        self.active = true;
        self.input.clear();
        self.rebuild_filter();
        self.selected = 0;
    }

    pub fn close(&mut self) {
        self.active = false;
        self.input.clear();
    }

    pub fn push_char(&mut self, c: char) {
        self.input.push(c);
        self.rebuild_filter();
    }

    pub fn backspace(&mut self) {
        self.input.pop();
        self.rebuild_filter();
    }

    pub fn next(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 1) % self.filtered.len();
        }
    }

    pub fn prev(&mut self) {
        if !self.filtered.is_empty() {
            let len = self.filtered.len();
            self.selected = (self.selected + len - 1) % len;
        }
    }

    pub fn execute(&mut self) -> Option<PaletteAction> {
        let &idx = self.filtered.get(self.selected)?;
        let action = COMMANDS[idx].action.clone();
        self.close();
        Some(action)
    }

    // ── Fuzzy filter ──────────────────────────────────────────────────────────

    fn rebuild_filter(&mut self) {
        let q = self.input.to_lowercase();
        self.filtered = (0..COMMANDS.len())
            .filter(|&i| {
                if q.is_empty() { return true; }
                let cmd = &COMMANDS[i];
                let hay = format!(
                    "{} {} {} {}",
                    cmd.name.to_lowercase(),
                    cmd.desc.to_lowercase(),
                    cmd.category.to_lowercase(),
                    cmd.shortcut.unwrap_or("")
                );
                fuzzy_match(&q, &hay)
            })
            .collect();
        self.selected = 0;
    }

    // ── Draw ──────────────────────────────────────────────────────────────────

    pub fn draw(&self, f: &mut Frame) {
        if !self.active { return; }

        let area = f.area();
        let max_items = 14usize;
        let visible = self.filtered.len().min(max_items);
        let popup_width = 70u16.min(area.width.saturating_sub(6));
        let popup_height = (visible as u16 + 5).min(area.height.saturating_sub(4));

        let popup = Rect::new(
            (area.width.saturating_sub(popup_width)) / 2,
            4,
            popup_width,
            popup_height,
        );

        f.render_widget(Clear, popup);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(popup);

        // Input box
        let input_block = Paragraph::new(self.input.as_str())
            .style(ratatui::style::Style::default().fg(Palette::TEXT_BRIGHT))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(style_border_focus())
                    .title(Span::styled(
                        " ⌘  Command Palette  —  type to filter ",
                        style_title(),
                    )),
            );
        f.render_widget(input_block, layout[0]);

        // Results
        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .enumerate()
            .take(max_items)
            .map(|(di, &ci)| {
                let cmd = &COMMANDS[ci];
                let is_sel = di == self.selected;
                let bg = if is_sel { Palette::BG_SELECTED } else { Palette::BG_PANEL };
                let row_style = ratatui::style::Style::default().bg(bg);

                let prefix = if is_sel {
                    Span::styled(" ▶ ", ratatui::style::Style::default().fg(Palette::CYAN).bg(bg))
                } else {
                    Span::styled("   ", row_style)
                };

                let cat_span = Span::styled(
                    format!("{:8}", cmd.category),
                    ratatui::style::Style::default()
                        .fg(cat_color(cmd.category))
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                );

                let name_span = Span::styled(
                    format!("{:28}", cmd.name),
                    ratatui::style::Style::default()
                        .fg(if is_sel { Palette::TEXT_BRIGHT } else { Palette::TEXT })
                        .bg(bg)
                        .add_modifier(if is_sel { Modifier::BOLD } else { Modifier::empty() }),
                );

                let desc_span = Span::styled(
                    format!("  {}", truncate(cmd.desc, 18)),
                    ratatui::style::Style::default().fg(Palette::TEXT_DIM).bg(bg),
                );

                let mut spans = vec![prefix, cat_span, Span::styled(" ", row_style), name_span, desc_span];

                if let Some(sc) = cmd.shortcut {
                    spans.push(Span::styled(
                        format!("  [{}]", sc),
                        ratatui::style::Style::default().fg(Palette::CYAN_DIM).bg(bg),
                    ));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(style_dim())
                    .title(Span::styled(
                        format!(
                            "  {} results  ↑↓ navigate  Enter execute  Esc close ",
                            self.filtered.len()
                        ),
                        style_muted(),
                    )),
            );

        f.render_widget(list, layout[1]);
    }
}

impl Default for CommandPalette {
    fn default() -> Self { Self::new() }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn fuzzy_match(query: &str, haystack: &str) -> bool {
    let mut chars = query.chars().peekable();
    for c in haystack.chars() {
        if chars.peek() == Some(&c) {
            chars.next();
        }
    }
    chars.peek().is_none()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        let end = s.char_indices().nth(max.saturating_sub(1)).map(|(i, _)| i).unwrap_or(s.len());
        format!("{}…", &s[..end])
    }
}

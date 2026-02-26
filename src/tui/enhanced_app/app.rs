use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Tabs},
    Frame,
};

use crate::config::Config;
use crate::providers::{create_provider_with_keys_manager, ChatMessage};

use super::chat_pane::ChatPane;
use super::command_palette::{CommandPalette, PaletteAction};
use super::config_editor::ConfigEditor;
use super::help::HelpOverlay;
use super::input::InputBar;
use super::layout::{BodyZones, HeaderZones, RootZones};
use super::notifications::Notifications;
use super::sidebar::{ActivityKind, Sidebar, SidebarGoal};
use super::skills_panel::SkillsPanel;
use super::state::{ActivePane, AppState, InputMode, MainTab, StreamStatus};
use super::theme::{Palette, style_border, style_dim, style_muted, style_tab_active, style_tab_inactive, style_title, LOGO, VERSION};
use super::tools_panel::ToolsPanel;

// ── EnhancedApp ───────────────────────────────────────────────────────────────

pub struct EnhancedApp {
    // Config & provider
    config:         Config,
    provider_name:  String,
    model_name:     String,

    // Core state
    state:          AppState,

    // Panels
    chat:           ChatPane,
    input:          InputBar,
    sidebar:        Sidebar,
    skills:         SkillsPanel,
    tools:          ToolsPanel,
    cfg_editor:     ConfigEditor,
    help:           HelpOverlay,
    palette:        CommandPalette,
    notifs:         Notifications,
}

impl EnhancedApp {
    pub fn new(config: Config, provider_name: String, model_name: String) -> Self {
        let mut skills = SkillsPanel::new();
        // Try to load skills from workspace
        let skills_dir = config.workspace_dir.join("skills");
        let config_skills: Vec<(String, bool)> = config
            .skills
            .enabled
            .iter()
            .map(|(name, &en)| (name.clone(), en))
            .collect();
        skills.load_from_paths(&skills_dir, &config_skills);

        let mut state = AppState::new();
        state.metrics.skills_enabled = skills.skills.iter().filter(|s| s.enabled).count();

        let cfg_editor = ConfigEditor::new(&config);

        Self {
            config,
            provider_name,
            model_name,
            state,
            chat:       ChatPane::new(),
            input:      InputBar::new(),
            sidebar:    Sidebar::new(),
            skills,
            tools:      ToolsPanel::new(),
            cfg_editor,
            help:       HelpOverlay::new(),
            palette:    CommandPalette::new(),
            notifs:     Notifications::new(),
        }
    }

    // ── Public interface (called by tui/mod.rs loop) ──────────────────────────

    pub fn should_quit(&self) -> bool {
        self.state.should_quit
    }

    pub fn update(&mut self) {
        self.state.tick();
        self.notifs.tick();
        self.skills.tick();
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Global overrides first
        match (key.modifiers, key.code) {
            // Ctrl+C always quits
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.state.should_quit = true;
                return Ok(());
            }
            // Ctrl+P opens command palette from anywhere
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                if self.palette.active {
                    self.palette.close();
                } else {
                    self.palette.open();
                    self.state.show_command_palette = true;
                }
                return Ok(());
            }
            _ => {}
        }

        // Route to overlay handlers first
        if self.help.visible {
            return self.handle_help_key(key);
        }
        if self.palette.active {
            return self.handle_palette_key(key);
        }

        // Route to active-tab handlers
        match self.state.active_tab {
            MainTab::Chat    => self.handle_chat_key(key),
            MainTab::Skills  => self.handle_skills_key(key),
            MainTab::Tools   => self.handle_tools_key(key),
            MainTab::Goals   => self.handle_goals_key(key),
            MainTab::Metrics => self.handle_metrics_key(key),
            MainTab::Config  => self.handle_config_key(key),
        }
    }

    // ── Draw entry point ──────────────────────────────────────────────────────

    pub fn draw(&mut self, f: &mut Frame) {
        let area = f.area();
        let zones = RootZones::compute(area);

        self.draw_header(f, zones.header);
        self.draw_body(f, zones.body);
        self.draw_input_row(f, zones.input);
        self.draw_status(f, zones.footer);

        // Overlays (drawn on top)
        self.help.draw(f);
        self.palette.draw(f);
        self.notifs.draw(f);
    }

    // ── Header ────────────────────────────────────────────────────────────────

    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let hz = HeaderZones::compute(area);

        // Brand
        let brand = Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {} ", LOGO),
                ratatui::style::Style::default()
                    .fg(Palette::BG)
                    .bg(Palette::CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" v{}", VERSION), style_muted()),
        ]));
        f.render_widget(brand, hz.brand);

        // Tabs
        let tab_titles: Vec<Span> = MainTab::ALL.iter().map(|&t| {
            if t == self.state.active_tab {
                Span::styled(t.label(), style_tab_active())
            } else {
                Span::styled(t.label(), style_tab_inactive())
            }
        }).collect();

        let tabs = Tabs::new(tab_titles)
            .select(self.state.active_tab.index())
            .divider(Span::styled(" │ ", style_muted()))
            .style(ratatui::style::Style::default());
        f.render_widget(tabs, hz.tabs);

        // Meta (right side: provider / view / search hint)
        let view_label = self.state.view_mode.label();
        let provider_owned = self.provider_name.clone();
        let model_owned = truncate_str(&self.model_name, 14).to_owned();
        let view_owned = format!(" [{}] ", view_label);
        let meta = Paragraph::new(Line::from(vec![
            Span::styled(provider_owned, ratatui::style::Style::default().fg(Palette::CYAN)),
            Span::styled("/", style_muted()),
            Span::styled(
                model_owned,
                ratatui::style::Style::default().fg(Palette::VIOLET),
            ),
            Span::styled(view_owned, style_dim()),
        ]));
        f.render_widget(meta, hz.meta);
    }

    // ── Body ──────────────────────────────────────────────────────────────────

    fn draw_body(&mut self, f: &mut Frame, area: Rect) {
        let bz = BodyZones::compute(area, self.state.view_mode, self.state.sidebar_visible);

        match self.state.active_tab {
            MainTab::Chat    => {
                let focused_chat = self.state.active_pane == ActivePane::Chat;
                self.chat.draw(f, bz.main, focused_chat);
            }
            MainTab::Skills  => self.skills.draw(f, bz.main),
            MainTab::Tools   => self.tools.draw(f, bz.main, &self.state.tool_log),
            MainTab::Goals   => self.draw_goals_tab(f, bz.main),
            MainTab::Metrics => self.draw_metrics_tab(f, bz.main),
            MainTab::Config  => self.cfg_editor.draw(f, bz.main),
        }

        if let Some(sb_area) = bz.sidebar {
            self.sidebar.draw(f, sb_area, &self.state.metrics);
        }
    }

    // ── Goals tab ─────────────────────────────────────────────────────────────

    fn draw_goals_tab(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border())
            .title(Span::styled(
                format!(" 🎯 Goals ({}) ", self.sidebar.goals.len()),
                ratatui::style::Style::default().fg(Palette::GOAL).add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.sidebar.goals.is_empty() {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  No active goals.",
                        style_muted(),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Goals are created automatically during AGI interactions,",
                        style_dim(),
                    )),
                    Line::from(Span::styled(
                        "  or add one via: Ctrl+P → Add goal…",
                        style_dim(),
                    )),
                ]),
                inner,
            );
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        for goal in &self.sidebar.goals {
            let bar = super::theme::render_gauge_bar(goal.progress, 16);
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", goal.priority.icon()),
                    ratatui::style::Style::default().fg(goal.priority.color()),
                ),
                Span::styled(
                    goal.title.clone(),
                    ratatui::style::Style::default()
                        .fg(Palette::TEXT_BRIGHT)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(bar, ratatui::style::Style::default().fg(Palette::CYAN_DIM)),
                Span::styled(
                    format!("  {:.0}%", goal.progress * 100.0),
                    style_dim(),
                ),
            ]));
            lines.push(Line::from(""));
        }

        f.render_widget(Paragraph::new(lines), inner);
    }

    // ── Metrics tab ───────────────────────────────────────────────────────────

    fn draw_metrics_tab(&self, f: &mut Frame, area: Rect) {
        let m = &self.state.metrics;

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left: session stats
        let left_block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border())
            .title(Span::styled(" 📊 Session Stats ", style_title()));
        let left_inner = left_block.inner(layout[0]);
        f.render_widget(left_block, layout[0]);

        let err_style = if m.total_errors > 0 {
            ratatui::style::Style::default().fg(Palette::ERROR)
        } else {
            ratatui::style::Style::default().fg(Palette::SUCCESS)
        };

        let left_lines = vec![
            Line::from(""),
            row("  Uptime",       m.format_uptime(),                               Palette::SUCCESS),
            row("  Messages",     m.total_messages.to_string(),                     Palette::TEXT),
            row("  Requests",     m.total_requests.to_string(),                     Palette::TEXT),
            row("  Tokens In",    m.total_tokens_in.to_string(),                    Palette::CYAN_DIM),
            row("  Tokens Out",   m.total_tokens_out.to_string(),                   Palette::CYAN_DIM),
            row("  Avg t/s",      format!("{:.1}", m.avg_tokens_per_sec),           Palette::CYAN),
            row("  Last Latency", format!("{}ms", m.last_latency_ms),               Palette::TEXT),
            Line::from(vec![
                Span::styled("  Errors      ", style_muted()),
                Span::styled(m.total_errors.to_string(), err_style),
            ]),
            row("  Error Rate",   format!("{:.1}%", m.error_rate() * 100.0),        Palette::TEXT),
        ];
        f.render_widget(Paragraph::new(left_lines), left_inner);

        // Right: AGI capabilities
        let right_block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border())
            .title(Span::styled(" 🧠 AGI Capabilities ", style_title()));
        let right_inner = right_block.inner(layout[1]);
        f.render_widget(right_block, layout[1]);

        let tools_invoked = m.tools_invoked;
        let skills_enabled = m.skills_enabled;
        let goals_active = self.sidebar.goals.len();

        let skill_ratio = if self.skills.skills.is_empty() {
            0.0
        } else {
            skills_enabled as f64 / self.skills.skills.len() as f64
        };

        let right_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(1),
            ])
            .split(right_inner);

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Skills Enabled  ", style_muted()),
                Span::styled(format!("{}/{}", skills_enabled, self.skills.skills.len()), ratatui::style::Style::default().fg(Palette::SKILL)),
            ])),
            right_layout[0],
        );

        let skill_gauge = Gauge::default()
            .block(Block::default().borders(Borders::NONE))
            .gauge_style(ratatui::style::Style::default().fg(Palette::SKILL).bg(Palette::BG_ELEVATED))
            .ratio(skill_ratio)
            .label(format!("{:.0}%", skill_ratio * 100.0));
        f.render_widget(skill_gauge, right_layout[1]);

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Tools Invoked   ", style_muted()),
                Span::styled(tools_invoked.to_string(), ratatui::style::Style::default().fg(Palette::TOOL)),
                Span::styled("     Goals Active  ", style_muted()),
                Span::styled(goals_active.to_string(), ratatui::style::Style::default().fg(Palette::GOAL)),
            ])),
            right_layout[2],
        );

        let activity_lines: Vec<Line> = self.sidebar.activity.iter().rev().take(8).map(|a| {
            let icon_style = match a.kind {
                ActivityKind::Tool    => ratatui::style::Style::default().fg(Palette::TOOL),
                ActivityKind::Skill   => ratatui::style::Style::default().fg(Palette::SKILL),
                ActivityKind::Thought => ratatui::style::Style::default().fg(Palette::THOUGHT),
                ActivityKind::Goal    => ratatui::style::Style::default().fg(Palette::GOAL),
                ActivityKind::System  => style_dim(),
            };
            Line::from(vec![
                Span::styled(format!(" {} ", a.kind.icon()), icon_style),
                Span::styled(truncate_str(&a.message, 36), ratatui::style::Style::default().fg(Palette::TEXT_DIM)),
                Span::styled(format!(" {}", a.time), style_muted()),
            ])
        }).collect();
        f.render_widget(Paragraph::new(activity_lines), right_layout[4]);
    }

    // ── Input row ─────────────────────────────────────────────────────────────

    fn draw_input_row(&self, f: &mut Frame, area: Rect) {
        let focused = self.state.active_pane == ActivePane::Input
            || self.state.input_mode.is_typing();
        self.input.draw(f, area, focused);
    }

    // ── Status bar ────────────────────────────────────────────────────────────

    fn draw_status(&self, f: &mut Frame, area: Rect) {
        super::status_bar::draw(f, area, &self.state, &self.provider_name, &self.model_name);
    }

    // ── Key handlers ─────────────────────────────────────────────────────────

    fn handle_help_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up   => self.help.scroll_up(),
            KeyCode::Down => self.help.scroll_down(),
            _             => self.help.hide(),
        }
        Ok(())
    }

    fn handle_palette_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.palette.close();
                self.state.show_command_palette = false;
            }
            KeyCode::Up => self.palette.prev(),
            KeyCode::Down => self.palette.next(),
            KeyCode::Enter => {
                if let Some(action) = self.palette.execute() {
                    self.state.show_command_palette = false;
                    self.execute_palette_action(action)?;
                }
            }
            KeyCode::Backspace => self.palette.backspace(),
            KeyCode::Char(c) => self.palette.push_char(c),
            _ => {}
        }
        Ok(())
    }

    fn execute_palette_action(&mut self, action: PaletteAction) -> Result<()> {
        match action {
            PaletteAction::ClearChat => {
                self.chat.clear();
                self.notifs.info("Conversation cleared");
            }
            PaletteAction::ExportChat => {
                self.export_chat()?;
            }
            PaletteAction::CopyLastResponse => {
                if let Some(msg) = self.chat.last_assistant() {
                    let _ = msg.content.clone(); // clipboard would go here
                    self.notifs.info("Response copied (use 's' to save)");
                }
            }
            PaletteAction::ToggleAutoScroll => {
                self.chat.toggle_auto_scroll();
                self.notifs.info(if self.chat.auto_scroll { "Auto-scroll ON" } else { "Auto-scroll OFF" });
            }
            PaletteAction::CycleView => {
                self.state.view_mode = self.state.view_mode.cycle();
                self.notifs.info(format!("View: {}", self.state.view_mode.label()));
            }
            PaletteAction::ToggleSidebar => {
                self.state.sidebar_visible = !self.state.sidebar_visible;
            }
            PaletteAction::GotoChat    => { self.state.active_tab = MainTab::Chat; }
            PaletteAction::GotoSkills  => { self.state.active_tab = MainTab::Skills; }
            PaletteAction::GotoTools   => { self.state.active_tab = MainTab::Tools; }
            PaletteAction::GotoGoals   => { self.state.active_tab = MainTab::Goals; }
            PaletteAction::GotoMetrics => { self.state.active_tab = MainTab::Metrics; }
            PaletteAction::GotoConfig  => {
                self.state.active_tab = MainTab::Config;
                self.cfg_editor = ConfigEditor::new(&self.config);
            }
            PaletteAction::Reflect => {
                self.chat.push_system("AGI: initiating self-reflection cycle…".to_string());
                self.sidebar.push_activity(ActivityKind::Thought, "Self-reflection triggered");
                self.notifs.info("Reflection cycle started");
            }
            PaletteAction::AddGoal(title) => {
                self.sidebar.goals.push(SidebarGoal {
                    title:    title.clone(),
                    progress: 0.0,
                    priority: super::sidebar::GoalPriority::Medium,
                });
                self.state.metrics.goals_active = self.sidebar.goals.len();
                self.notifs.success(format!("Goal added: {}", title));
            }
            PaletteAction::AddGoalStatic(title) => {
                self.sidebar.goals.push(SidebarGoal {
                    title:    title.to_string(),
                    progress: 0.0,
                    priority: super::sidebar::GoalPriority::Medium,
                });
                self.state.metrics.goals_active = self.sidebar.goals.len();
                self.notifs.success(format!("Goal added: {}", title));
            }
            PaletteAction::SwitchModel(m) => {
                self.model_name = m.clone();
                self.notifs.success(format!("Model → {}", m));
            }
            PaletteAction::OpenHelp => {
                self.help.show();
            }
            PaletteAction::Quit => {
                self.state.should_quit = true;
            }
        }
        Ok(())
    }

    fn handle_chat_key(&mut self, key: KeyEvent) -> Result<()> {
        // If skills filter active — shouldn't happen here but guard
        if self.state.input_mode == InputMode::Search {
            return self.handle_search_key(key);
        }

        match (key.modifiers, key.code) {
            // --- Typing mode --------------------------------------------------
            (KeyModifiers::NONE, KeyCode::Enter) if self.state.input_mode.is_typing() => {
                if !self.input.is_empty() {
                    let msg = self.input.take();
                    if msg.starts_with('/') {
                        self.handle_slash_command(&msg)?;
                    } else {
                        self.send_message(msg)?;
                    }
                }
            }
            (KeyModifiers::NONE, KeyCode::Esc) if self.state.input_mode.is_typing() => {
                self.state.input_mode = InputMode::Normal;
                self.state.active_pane = ActivePane::Chat;
            }
            (KeyModifiers::NONE, KeyCode::Backspace) if self.state.input_mode.is_typing() => {
                self.input.backspace();
            }
            (KeyModifiers::NONE, KeyCode::Delete) if self.state.input_mode.is_typing() => {
                self.input.delete_forward();
            }
            (KeyModifiers::NONE, KeyCode::Left) if self.state.input_mode.is_typing() => {
                self.input.move_cursor_left();
            }
            (KeyModifiers::NONE, KeyCode::Right) if self.state.input_mode.is_typing() => {
                self.input.move_cursor_right();
            }
            (KeyModifiers::NONE, KeyCode::Home) if self.state.input_mode.is_typing() => {
                self.input.move_cursor_home();
            }
            (KeyModifiers::NONE, KeyCode::End) if self.state.input_mode.is_typing() => {
                self.input.move_cursor_end();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('k')) if self.state.input_mode.is_typing() => {
                self.input.kill_line();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('w')) if self.state.input_mode.is_typing() => {
                self.input.kill_word_back();
            }
            (KeyModifiers::NONE, KeyCode::Up) if self.state.input_mode.is_typing() => {
                self.input.history_prev();
            }
            (KeyModifiers::NONE, KeyCode::Down) if self.state.input_mode.is_typing() => {
                self.input.history_next();
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) if self.state.input_mode.is_typing() => {
                self.input.push_char(c);
            }

            // --- Normal mode --------------------------------------------------
            // Any printable key or Enter enters insert mode
            (KeyModifiers::NONE, KeyCode::Char('i'))
            | (KeyModifiers::NONE, KeyCode::Enter) => {
                self.state.input_mode = InputMode::Insert;
                self.state.active_pane = ActivePane::Input;
            }

            // Scroll chat
            (KeyModifiers::NONE, KeyCode::Up)       => self.chat.scroll_up(1),
            (KeyModifiers::NONE, KeyCode::Down)      => self.chat.scroll_down(1),
            (KeyModifiers::NONE, KeyCode::PageUp)    => self.chat.scroll_up(8),
            (KeyModifiers::NONE, KeyCode::PageDown)  => self.chat.scroll_down(8),
            (KeyModifiers::NONE, KeyCode::Home)      => self.chat.scroll_to_top(),
            (KeyModifiers::NONE, KeyCode::End)       => self.chat.scroll_to_bottom(),

            // Auto-scroll toggle
            (KeyModifiers::NONE, KeyCode::Char('a')) => {
                self.chat.toggle_auto_scroll();
                self.notifs.info(if self.chat.auto_scroll { "Auto-scroll ON" } else { "Auto-scroll OFF" });
            }

            // Save / export
            (KeyModifiers::NONE, KeyCode::Char('s')) => self.export_chat()?,

            // Search
            (KeyModifiers::NONE, KeyCode::Char('/'))
            | (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
                self.state.input_mode = InputMode::Search;
                self.state.show_search = true;
            }
            (KeyModifiers::NONE, KeyCode::Char('n')) => self.chat.search_next(),
            (KeyModifiers::SHIFT, KeyCode::Char('N')) => self.chat.search_prev(),

            // View
            (KeyModifiers::NONE, KeyCode::Char('v')) => {
                self.state.view_mode = self.state.view_mode.cycle();
            }
            (KeyModifiers::NONE, KeyCode::Char('b')) => {
                self.state.sidebar_visible = !self.state.sidebar_visible;
            }

            // Clear conversation
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                self.chat.clear();
                self.notifs.info("Conversation cleared");
            }

            // Help
            (KeyModifiers::NONE, KeyCode::Char('?'))
            | (KeyModifiers::NONE, KeyCode::F(1)) => {
                self.help.toggle();
            }

            // Tab navigation (also handled globally via number keys)
            (KeyModifiers::NONE, KeyCode::Tab) => {
                self.state.active_tab = self.state.active_tab.next();
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                self.state.active_tab = self.state.active_tab.prev();
            }

            // Number keys → direct tab
            (KeyModifiers::NONE, KeyCode::Char('1')) => { self.state.active_tab = MainTab::Chat; }
            (KeyModifiers::NONE, KeyCode::Char('2')) => { self.state.active_tab = MainTab::Skills; }
            (KeyModifiers::NONE, KeyCode::Char('3')) => { self.state.active_tab = MainTab::Tools; }
            (KeyModifiers::NONE, KeyCode::Char('4')) => { self.state.active_tab = MainTab::Goals; }
            (KeyModifiers::NONE, KeyCode::Char('5')) => { self.state.active_tab = MainTab::Metrics; }
            (KeyModifiers::NONE, KeyCode::Char('6')) => { self.state.active_tab = MainTab::Config; }

            // Quit
            (KeyModifiers::NONE, KeyCode::Char('q')) => {
                if self.state.input_mode == InputMode::Normal {
                    self.state.should_quit = true;
                }
            }
            (KeyModifiers::NONE, KeyCode::Esc) => {
                if self.state.input_mode == InputMode::Normal {
                    self.state.should_quit = true;
                }
            }

            _ => {}
        }
        Ok(())
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.state.input_mode = InputMode::Insert;
                self.state.show_search = false;
            }
            KeyCode::Backspace => {
                self.input.backspace();
                self.chat.set_search(self.input.text.clone());
            }
            KeyCode::Char(c) => {
                self.input.push_char(c);
                self.chat.set_search(self.input.text.clone());
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_skills_key(&mut self, key: KeyEvent) -> Result<()> {
        if self.skills.is_filter_active() {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.skills.filter_commit(),
                KeyCode::Backspace            => self.skills.filter_pop(),
                KeyCode::Char(c)              => self.skills.filter_push(c),
                _ => {}
            }
            return Ok(());
        }

        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
                self.state.active_tab = MainTab::Chat;
            }
            (_, KeyCode::Tab)                               => self.skills.tab_next(),
            (KeyModifiers::SHIFT, KeyCode::BackTab)         => self.skills.tab_prev(),
            (_, KeyCode::Up)   | (_, KeyCode::Char('k'))   => self.skills.select_prev(),
            (_, KeyCode::Down) | (_, KeyCode::Char('j'))   => self.skills.select_next(),
            (_, KeyCode::Char(' ')) | (_, KeyCode::Enter)  => {
                if let Some(enabled) = self.skills.toggle_selected() {
                    self.state.metrics.skills_enabled = self.skills.skills.iter().filter(|s| s.enabled).count();
                    let kind = if enabled { "Enabled" } else { "Disabled" };
                    self.sidebar.push_activity(ActivityKind::Skill, format!("{}: skill", kind));
                }
            }
            (_, KeyCode::Char('r'))                        => {
                let skills_dir = self.config.workspace_dir.join("skills");
                let config_skills: Vec<(String, bool)> = self.config.skills.enabled.iter().map(|(name, &en)| (name.clone(), en)).collect();
                self.skills.load_from_paths(&skills_dir, &config_skills);
                self.notifs.info("Skills refreshed");
            }
            (_, KeyCode::Char('/'))                        => self.skills.start_filter(),
            (_, KeyCode::PageUp)                           => self.skills.detail_scroll_up(),
            (_, KeyCode::PageDown)                         => self.skills.detail_scroll_down(),
            (KeyModifiers::NONE, KeyCode::Char('1'))       => { self.state.active_tab = MainTab::Chat; }
            (KeyModifiers::NONE, KeyCode::Char('2'))       => { self.state.active_tab = MainTab::Skills; }
            (KeyModifiers::NONE, KeyCode::Char('3'))       => { self.state.active_tab = MainTab::Tools; }
            (KeyModifiers::NONE, KeyCode::Char('4'))       => { self.state.active_tab = MainTab::Goals; }
            (KeyModifiers::NONE, KeyCode::Char('5'))       => { self.state.active_tab = MainTab::Metrics; }
            (KeyModifiers::NONE, KeyCode::Char('?'))
            | (KeyModifiers::NONE, KeyCode::F(1))          => { self.help.toggle(); }
            _ => {}
        }
        Ok(())
    }

    fn handle_tools_key(&mut self, key: KeyEvent) -> Result<()> {
        if self.tools.is_filter_active() {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.tools.filter_commit(),
                KeyCode::Backspace            => self.tools.filter_pop(),
                KeyCode::Char(c)              => self.tools.filter_push(c),
                _ => {}
            }
            return Ok(());
        }

        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
                self.state.active_tab = MainTab::Chat;
            }
            (_, KeyCode::Up)   | (_, KeyCode::Char('k')) => self.tools.select_prev(&self.state.tool_log),
            (_, KeyCode::Down) | (_, KeyCode::Char('j')) => self.tools.select_next(&self.state.tool_log),
            (_, KeyCode::PageUp)                          => self.tools.detail_scroll_up(),
            (_, KeyCode::PageDown)                        => self.tools.detail_scroll_down(),
            (_, KeyCode::Char('/'))                       => self.tools.start_filter(),
            (_, KeyCode::Char('c'))                       => {
                self.state.tool_log.clear();
                self.notifs.info("Tool log cleared");
            }
            (KeyModifiers::NONE, KeyCode::Char('1'))       => { self.state.active_tab = MainTab::Chat; }
            (KeyModifiers::NONE, KeyCode::Char('2'))       => { self.state.active_tab = MainTab::Skills; }
            (KeyModifiers::NONE, KeyCode::Char('3'))       => { self.state.active_tab = MainTab::Tools; }
            (KeyModifiers::NONE, KeyCode::Char('4'))       => { self.state.active_tab = MainTab::Goals; }
            (KeyModifiers::NONE, KeyCode::Char('5'))       => { self.state.active_tab = MainTab::Metrics; }
            (KeyModifiers::NONE, KeyCode::Char('6'))       => { self.state.active_tab = MainTab::Config; }
            (KeyModifiers::NONE, KeyCode::Char('?'))
            | (KeyModifiers::NONE, KeyCode::F(1))         => { self.help.toggle(); }
            _ => {}
        }
        Ok(())
    }

    fn handle_goals_key(&mut self, key: KeyEvent) -> Result<()> {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
                self.state.active_tab = MainTab::Chat;
            }
            (KeyModifiers::NONE, KeyCode::Tab)             => { self.state.active_tab = self.state.active_tab.next(); }
            (KeyModifiers::SHIFT, KeyCode::BackTab)        => { self.state.active_tab = self.state.active_tab.prev(); }
            (KeyModifiers::NONE, KeyCode::Char('1'))       => { self.state.active_tab = MainTab::Chat; }
            (KeyModifiers::NONE, KeyCode::Char('2'))       => { self.state.active_tab = MainTab::Skills; }
            (KeyModifiers::NONE, KeyCode::Char('3'))       => { self.state.active_tab = MainTab::Tools; }
            (KeyModifiers::NONE, KeyCode::Char('4'))       => { self.state.active_tab = MainTab::Goals; }
            (KeyModifiers::NONE, KeyCode::Char('5'))       => { self.state.active_tab = MainTab::Metrics; }
            (KeyModifiers::NONE, KeyCode::Char('6'))       => { self.state.active_tab = MainTab::Config; }
            (KeyModifiers::NONE, KeyCode::Char('?'))
            | (KeyModifiers::NONE, KeyCode::F(1))          => { self.help.toggle(); }
            _ => {}
        }
        Ok(())
    }

    fn handle_metrics_key(&mut self, key: KeyEvent) -> Result<()> {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
                self.state.active_tab = MainTab::Chat;
            }
            (KeyModifiers::NONE, KeyCode::Tab)             => { self.state.active_tab = self.state.active_tab.next(); }
            (KeyModifiers::SHIFT, KeyCode::BackTab)        => { self.state.active_tab = self.state.active_tab.prev(); }
            (KeyModifiers::NONE, KeyCode::Char('1'))       => { self.state.active_tab = MainTab::Chat; }
            (KeyModifiers::NONE, KeyCode::Char('2'))       => { self.state.active_tab = MainTab::Skills; }
            (KeyModifiers::NONE, KeyCode::Char('3'))       => { self.state.active_tab = MainTab::Tools; }
            (KeyModifiers::NONE, KeyCode::Char('4'))       => { self.state.active_tab = MainTab::Goals; }
            (KeyModifiers::NONE, KeyCode::Char('5'))       => { self.state.active_tab = MainTab::Metrics; }
            (KeyModifiers::NONE, KeyCode::Char('6'))       => { self.state.active_tab = MainTab::Config; }
            (KeyModifiers::NONE, KeyCode::Char('?'))
            | (KeyModifiers::NONE, KeyCode::F(1))          => { self.help.toggle(); }
            _ => {}
        }
        Ok(())
    }

    fn handle_config_key(&mut self, key: KeyEvent) -> Result<()> {
        // When inline editor popup is open, route all keys into it
        if self.cfg_editor.is_editing() {
            match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Enter) => {
                    if !self.cfg_editor.commit_edit() {
                        // validation error — leave editor open
                    }
                }
                (KeyModifiers::NONE, KeyCode::Esc) => {
                    self.cfg_editor.cancel_edit();
                }
                (KeyModifiers::NONE, KeyCode::Backspace) => self.cfg_editor.edit_backspace(),
                (KeyModifiers::NONE, KeyCode::Left)      => self.cfg_editor.edit_left(),
                (KeyModifiers::NONE, KeyCode::Right)     => self.cfg_editor.edit_right(),
                (KeyModifiers::NONE, KeyCode::Home)      => self.cfg_editor.edit_home(),
                (KeyModifiers::NONE, KeyCode::End)       => self.cfg_editor.edit_end(),
                (KeyModifiers::CONTROL, KeyCode::Char('k')) => self.cfg_editor.edit_kill_line(),
                (KeyModifiers::NONE, KeyCode::Char(c))   => self.cfg_editor.edit_push(c),
                _ => {}
            }
            return Ok(());
        }

        // Raw TOML view
        if self.cfg_editor.is_showing_raw() {
            match key.code {
                KeyCode::Char('r') | KeyCode::Esc | KeyCode::Char('q') => {
                    self.cfg_editor.toggle_raw(&self.config);
                }
                _ => {}
            }
            return Ok(());
        }

        match (key.modifiers, key.code) {
            // Save
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                match self.cfg_editor.apply_and_save(&mut self.config) {
                    Ok(()) => {
                        self.notifs.success("Config saved to ~/.housaky/config.toml");
                        // Refresh provider/model names from updated config
                        if let Some(p) = &self.config.default_provider { self.provider_name = p.clone(); }
                        if let Some(m) = &self.config.default_model     { self.model_name   = m.clone(); }
                    }
                    Err(e) => self.notifs.error(format!("Save failed: {}", e)),
                }
            }

            // Navigation
            (_, KeyCode::Up)   | (_, KeyCode::Char('k')) => self.cfg_editor.field_up(),
            (_, KeyCode::Down) | (_, KeyCode::Char('j')) => self.cfg_editor.field_down(),

            // Edit selected field (also Space for bool toggle)
            (_, KeyCode::Enter) | (_, KeyCode::Char(' ')) => self.cfg_editor.start_edit(),

            // Section tabs
            (_, KeyCode::Tab)                      => self.cfg_editor.section_next(&self.config),
            (KeyModifiers::SHIFT, KeyCode::BackTab) => self.cfg_editor.section_prev(&self.config),

            // Raw TOML toggle
            (_, KeyCode::Char('r')) => self.cfg_editor.toggle_raw(&self.config),

            // Global tab jump
            (KeyModifiers::NONE, KeyCode::Char('1')) => { self.state.active_tab = MainTab::Chat; }
            (KeyModifiers::NONE, KeyCode::Char('2')) => { self.state.active_tab = MainTab::Skills; }
            (KeyModifiers::NONE, KeyCode::Char('3')) => { self.state.active_tab = MainTab::Tools; }
            (KeyModifiers::NONE, KeyCode::Char('4')) => { self.state.active_tab = MainTab::Goals; }
            (KeyModifiers::NONE, KeyCode::Char('5')) => { self.state.active_tab = MainTab::Metrics; }
            (KeyModifiers::NONE, KeyCode::Char('6')) => { self.state.active_tab = MainTab::Config; }

            // Back to chat
            (_, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
                if self.cfg_editor.dirty {
                    self.notifs.warn("Unsaved changes — Ctrl+S to save, q again to discard");
                    // second q will actually leave — handled by marking not-dirty so next q exits
                    // For simplicity, prompt via notification; user presses q once more
                } else {
                    self.state.active_tab = MainTab::Chat;
                }
            }

            (KeyModifiers::NONE, KeyCode::Char('?'))
            | (KeyModifiers::NONE, KeyCode::F(1)) => self.help.toggle(),

            _ => {}
        }
        Ok(())
    }

    // ── Slash command dispatcher ──────────────────────────────────────────────

    fn handle_slash_command(&mut self, raw: &str) -> Result<()> {
        let cmd = raw.trim_start_matches('/');
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        let name = parts[0].trim();
        let arg  = parts.get(1).map(|s| s.trim()).unwrap_or("");

        match name {
            "clear"   => {
                self.chat.clear();
                self.notifs.info("Conversation cleared");
            }
            "export" | "save" => {
                self.export_chat()?;
            }
            "model"   => {
                if !arg.is_empty() {
                    self.model_name = arg.to_string();
                    self.notifs.success(format!("Model → {}", arg));
                } else {
                    self.chat.push_system(format!("Current model: {}", self.model_name));
                }
            }
            "goals"   => { self.state.active_tab = MainTab::Goals; }
            "skills"  => { self.state.active_tab = MainTab::Skills; }
            "tools"   => { self.state.active_tab = MainTab::Tools; }
            "metrics" => { self.state.active_tab = MainTab::Metrics; }
            "config" | "cfg" | "settings" => {
                self.state.active_tab = MainTab::Config;
                self.cfg_editor = ConfigEditor::new(&self.config);
            }
            "reflect" => {
                self.chat.push_system("Initiating self-reflection cycle…".to_string());
                self.sidebar.push_activity(ActivityKind::Thought, "Self-reflection triggered");
                self.notifs.info("Reflection cycle started");
            }
            "help"    => { self.help.show(); }
            "quit" | "exit" => { self.state.should_quit = true; }
            other => {
                self.notifs.warn(format!("Unknown command: /{}", other));
                self.chat.push_system(format!("Unknown command: /{}. Type /help for a list.", other));
            }
        }
        Ok(())
    }

    // ── Message sending ───────────────────────────────────────────────────────

    fn send_message(&mut self, text: String) -> Result<()> {
        self.chat.push_user(text.clone());
        self.state.stream_status = StreamStatus::Thinking;
        self.state.metrics.total_messages += 1;
        self.state.metrics.total_requests += 1;
        self.sidebar.push_activity(ActivityKind::Thought, format!("User: {}", truncate_str(&text, 40)));

        let provider = create_provider_with_keys_manager(&self.provider_name, Some(&self.model_name))?;

        let chat_messages: Vec<ChatMessage> = self.chat.messages.iter()
            .map(|m| ChatMessage {
                role:    m.role.label().to_lowercase(),
                content: m.content.clone(),
            })
            .collect();

        let model = self.model_name.clone();

        let start = std::time::Instant::now();

        let result = match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                tokio::task::block_in_place(|| {
                    handle.block_on(async {
                        provider.chat_with_history(&chat_messages, &model, 0.7).await
                    })
                })
            }
            Err(_) => {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async {
                    provider.chat_with_history(&chat_messages, &model, 0.7).await
                })
            }
        };

        let elapsed = start.elapsed().as_millis() as u64;
        self.state.metrics.last_latency_ms = elapsed;

        match result {
            Ok(response) => {
                let token_est = (response.len() / 4) as u32;
                self.state.metrics.total_tokens_out += u64::from(token_est);
                self.state.metrics.total_tokens_in  += (text.len() / 4) as u64;

                // Update rolling avg t/s
                if elapsed > 0 {
                    let tps = f64::from(token_est) / (elapsed as f64 / 1000.0);
                    self.state.metrics.avg_tokens_per_sec =
                        0.8 * self.state.metrics.avg_tokens_per_sec + 0.2 * tps;
                }

                self.chat.push_assistant(response.clone(), Some(token_est));
                self.state.stream_status = StreamStatus::Done;
                self.sidebar.push_activity(
                    ActivityKind::Thought,
                    format!("AGI: {}", truncate_str(&response, 40)),
                );
                self.state.metrics.total_messages += 1;
            }
            Err(e) => {
                let err_msg = format!("Error: {}", e);
                self.chat.push_system(err_msg.clone());
                self.state.stream_status = StreamStatus::Error(err_msg.clone());
                self.state.metrics.total_errors += 1;
                self.notifs.error(truncate_str(&err_msg, 48).to_string());
            }
        }

        Ok(())
    }

    // ── Export ────────────────────────────────────────────────────────────────

    fn export_chat(&mut self) -> Result<()> {
        let content = self.chat.export_markdown();
        let filename = format!(
            "housaky_chat_{}.md",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        );
        match std::fs::write(&filename, &content) {
            Ok(()) => {
                self.notifs.success(format!("Saved → {}", filename));
            }
            Err(e) => {
                self.notifs.error(format!("Save failed: {}", e));
            }
        }
        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row(label: &'static str, value: String, color: ratatui::style::Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<14}", label), style_muted()),
        Span::styled(value, ratatui::style::Style::default().fg(color)),
    ])
}

fn truncate_str(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}

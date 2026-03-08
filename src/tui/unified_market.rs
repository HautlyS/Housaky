//! Unified Marketplace - Skills + MCPs
//!
//! A comprehensive marketplace TUI showing both Skills and MCP servers
//! from Claude official, OpenClaw, and local sources.

use crate::config::Config;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::path::PathBuf;

// ============================================================================
// Data Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarketTab {
    Skills,
    Mcps,
}

impl MarketTab {
    fn all() -> &'static [MarketTab] {
        &[MarketTab::Skills, MarketTab::Mcps]
    }

    fn label(&self) -> &'static str {
        match self {
            MarketTab::Skills => "🧩 Skills",
            MarketTab::Mcps => "🔌 MCPs",
        }
    }

    fn index(&self) -> usize {
        Self::all().iter().position(|t| t == self).unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemSource {
    Local,
    OpenClaw,
    ClaudeOfficial,
    Config,
    Community,
}

impl ItemSource {
    fn label(&self) -> &'static str {
        match self {
            ItemSource::Local => "local",
            ItemSource::OpenClaw => "openclaw",
            ItemSource::ClaudeOfficial => "claude",
            ItemSource::Config => "config",
            ItemSource::Community => "community",
        }
    }

    fn color(&self) -> Color {
        match self {
            ItemSource::Local => Color::Cyan,
            ItemSource::OpenClaw => Color::Green,
            ItemSource::ClaudeOfficial => Color::Yellow,
            ItemSource::Config => Color::Gray,
            ItemSource::Community => Color::Magenta,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MarketItem {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub installed: bool,
    pub source: ItemSource,
    pub version: String,
    pub tags: Vec<String>,
    pub item_type: MarketTab,
    pub command: Option<String>,
    pub readme: Option<String>,
}

impl MarketItem {
    fn status_color(&self) -> Color {
        if self.enabled && self.installed {
            Color::Green
        } else if self.installed {
            Color::Yellow
        } else {
            Color::DarkGray
        }
    }

    fn status_icon(&self) -> &'static str {
        if self.enabled && self.installed {
            "●"
        } else if self.installed {
            "○"
        } else {
            "◌"
        }
    }

    fn status_label(&self) -> &'static str {
        if self.enabled && self.installed {
            "ENABLED"
        } else if self.installed {
            "DISABLED"
        } else {
            "AVAILABLE"
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterTab {
    All,
    Installed,
    Available,
}

impl FilterTab {
    fn all() -> &'static [FilterTab] {
        &[FilterTab::All, FilterTab::Installed, FilterTab::Available]
    }

    fn label(&self) -> &'static str {
        match self {
            FilterTab::All => "All",
            FilterTab::Installed => "Installed",
            FilterTab::Available => "Available",
        }
    }
}

// ============================================================================
// App State
// ============================================================================

pub struct UnifiedMarketApp {
    pub config: Config,
    pub workspace_dir: PathBuf,
    pub repo_root: PathBuf,

    // Items
    pub skills: Vec<MarketItem>,
    pub mcps: Vec<MarketItem>,

    // UI State
    pub market_tab: MarketTab,
    pub filter_tab: FilterTab,
    pub selected: usize,
    pub list_state: ratatui::widgets::ListState,
    pub filter: String,
    pub filter_active: bool,
    pub status: String,
    pub quit: bool,
    pub notification: Option<(String, std::time::Instant)>,
    pub detail_scroll: usize,
    pub loading: bool,

    // Layout cache for mouse
    last_layout: Option<[Rect; 4]>,
    last_content_cols: Option<[Rect; 2]>,
}

impl UnifiedMarketApp {
    pub fn new(config: Config, workspace_dir: PathBuf, repo_root: PathBuf) -> Self {
        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(0));

        Self {
            config,
            workspace_dir,
            repo_root,
            skills: Vec::new(),
            mcps: Vec::new(),
            market_tab: MarketTab::Skills,
            filter_tab: FilterTab::All,
            selected: 0,
            list_state,
            filter: String::new(),
            filter_active: false,
            status: String::new(),
            quit: false,
            notification: None,
            detail_scroll: 0,
            loading: true,
            last_layout: None,
            last_content_cols: None,
        }
    }

    pub fn load_all(&mut self) {
        self.loading = true;
        self.skills.clear();
        self.mcps.clear();

        // Load skills
        self.load_skills();

        // Load MCPs
        self.load_mcps();

        self.loading = false;
        self.status = format!(
            "Loaded {} skills, {} MCPs",
            self.skills.len(),
            self.mcps.len()
        );

        if self.selected >= self.current_items().len() {
            self.selected = 0;
        }
        self.list_state.select(Some(self.selected));
    }

    fn load_skills(&mut self) {
        // Load local skills from workspace
        let skills_dir = self.workspace_dir.join("skills");
        if let Ok(entries) = std::fs::read_dir(&skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                let name = entry.file_name().to_string_lossy().to_string();
                let enabled = self
                    .config
                    .skills
                    .enabled
                    .get(&name)
                    .copied()
                    .unwrap_or(false);
                let readme_path = path.join("SKILL.md");
                let readme = std::fs::read_to_string(&readme_path).ok();

                let description = readme
                    .as_deref()
                    .and_then(|r| {
                        r.lines()
                            .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
                    })
                    .unwrap_or("Local skill")
                    .trim()
                    .chars()
                    .take(100)
                    .collect();

                self.skills.push(MarketItem {
                    name: name.clone(),
                    description,
                    enabled,
                    installed: true,
                    source: ItemSource::Local,
                    version: "local".to_string(),
                    tags: vec!["local".to_string()],
                    item_type: MarketTab::Skills,
                    command: None,
                    readme,
                });
            }
        }

        // Load Claude official plugins
        if let Ok(plugins) = crate::skills::marketplace::list_claude_official_plugins(
            &self.workspace_dir,
            &self.config,
        ) {
            for plugin in plugins {
                // Check if already added from local
                if self.skills.iter().any(|s| s.name == plugin.name) {
                    continue;
                }

                let enabled = self
                    .config
                    .skills
                    .enabled
                    .get(&plugin.name)
                    .copied()
                    .unwrap_or(false);

                self.skills.push(MarketItem {
                    name: plugin.name.clone(),
                    description: plugin.description.clone(),
                    enabled,
                    installed: false,
                    source: if plugin.source.is_claude() {
                        ItemSource::ClaudeOfficial
                    } else {
                        ItemSource::OpenClaw
                    },
                    version: "official".to_string(),
                    tags: vec!["claude".to_string()],
                    item_type: MarketTab::Skills,
                    command: None,
                    readme: None,
                });
            }
        }

        // Load OpenClaw vendored skills
        let openclaw_dir = self.repo_root.join("openclaw").join("skills");
        if let Ok(entries) = std::fs::read_dir(&openclaw_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                let name = entry.file_name().to_string_lossy().to_string();
                if self.skills.iter().any(|s| s.name == name) {
                    continue;
                }

                let readme_path = path.join("SKILL.md");
                let readme = std::fs::read_to_string(&readme_path).ok();
                let enabled = self
                    .config
                    .skills
                    .enabled
                    .get(&name)
                    .copied()
                    .unwrap_or(false);

                let description = readme
                    .as_deref()
                    .and_then(|r| {
                        r.lines()
                            .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
                    })
                    .unwrap_or("OpenClaw skill")
                    .trim()
                    .chars()
                    .take(100)
                    .collect();

                self.skills.push(MarketItem {
                    name: name.clone(),
                    description,
                    enabled,
                    installed: false,
                    source: ItemSource::OpenClaw,
                    version: "openclaw".to_string(),
                    tags: vec!["openclaw".to_string()],
                    item_type: MarketTab::Skills,
                    command: None,
                    readme,
                });
            }
        }

        self.skills.sort_by(|a, b| a.name.cmp(&b.name));
    }

    fn load_mcps(&mut self) {
        // Load installed MCPs
        if let Ok(installed) =
            crate::mcp::marketplace::McpMarketplace::new(&self.workspace_dir).list_installed()
        {
            for mcp in installed {
                let enabled = mcp.enabled;

                self.mcps.push(MarketItem {
                    name: mcp.name.clone(),
                    description: mcp.description.clone(),
                    enabled,
                    installed: true,
                    source: ItemSource::Local,
                    version: "installed".to_string(),
                    tags: vec!["mcp".to_string()],
                    item_type: MarketTab::Mcps,
                    command: mcp.command.clone(),
                    readme: None,
                });
            }
        }

        // Load available MCPs from Claude registry
        if let Ok(available) =
            crate::mcp::marketplace::McpMarketplace::new(&self.workspace_dir).list_available()
        {
            for mcp in available {
                if self.mcps.iter().any(|m| m.name == mcp.name) {
                    continue;
                }

                self.mcps.push(MarketItem {
                    name: mcp.name.clone(),
                    description: mcp.description.clone(),
                    enabled: false,
                    installed: false,
                    source: ItemSource::ClaudeOfficial,
                    version: "claude".to_string(),
                    tags: vec!["mcp".to_string(), "claude".to_string()],
                    item_type: MarketTab::Mcps,
                    command: mcp.command.clone(),
                    readme: None,
                });
            }
        }

        self.mcps.sort_by(|a, b| a.name.cmp(&b.name));
    }

    fn current_items(&self) -> &Vec<MarketItem> {
        match self.market_tab {
            MarketTab::Skills => &self.skills,
            MarketTab::Mcps => &self.mcps,
        }
    }

    fn current_items_mut(&mut self) -> &mut Vec<MarketItem> {
        match self.market_tab {
            MarketTab::Skills => &mut self.skills,
            MarketTab::Mcps => &mut self.mcps,
        }
    }

    fn filtered_indices(&self) -> Vec<usize> {
        let q = self.filter.to_lowercase();
        self.current_items()
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                let tab_ok = match self.filter_tab {
                    FilterTab::All => true,
                    FilterTab::Installed => item.installed,
                    FilterTab::Available => !item.installed,
                };

                let filter_ok = if q.is_empty() {
                    true
                } else {
                    item.name.to_lowercase().contains(&q)
                        || item.description.to_lowercase().contains(&q)
                        || item.tags.iter().any(|t| t.to_lowercase().contains(&q))
                };

                tab_ok && filter_ok
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn selected_real_idx(&self) -> Option<usize> {
        self.filtered_indices().get(self.selected).copied()
    }

    fn toggle_selected(&mut self) {
        if let Some(idx) = self.selected_real_idx() {
            let (installed, enabled, name) = {
                let items = self.current_items();
                let item = &items[idx];
                (item.installed, item.enabled, item.name.clone())
            };

            if installed {
                let new_enabled = !enabled;

                // Update config first (before borrowing items mutably)
                self.config.skills.enabled.insert(name.clone(), new_enabled);
                let _ = self.config.save();

                // Update item
                {
                    let items = self.current_items_mut();
                    items[idx].enabled = new_enabled;
                }

                let msg = if new_enabled {
                    format!("✓ Enabled: {}", name)
                } else {
                    format!("○ Disabled: {}", name)
                };
                self.status = msg.clone();
                self.notify(&msg);
            } else {
                // Install the item
                self.install_item(idx);
            }
        }
    }

    fn install_item(&mut self, idx: usize) {
        let items = self.current_items_mut();
        let item = &items[idx];
        let name = item.name.clone();
        let is_mcp = item.item_type == MarketTab::Mcps;

        let _ = items; // Release borrow

        self.status = format!("Installing {}...", name);

        let result = if is_mcp {
            crate::mcp::marketplace::McpMarketplace::new(&self.workspace_dir)
                .install(&name)
                .map(|_| name.clone())
        } else {
            crate::skills::marketplace::install_claude_plugin(&self.workspace_dir, &name)
                .map(|names| names.join(", "))
        };

        match result {
            Ok(installed_names) => {
                self.notify(&format!("✓ Installed: {}", installed_names));

                // Refresh the list
                self.load_all();

                // Enable in config
                for n in installed_names.split(", ") {
                    self.config.skills.enabled.insert(n.to_string(), true);
                }
                let _ = self.config.save();
            }
            Err(e) => {
                self.notify(&format!("✗ Install failed: {}", e));
                self.status = format!("Install failed: {}", e);
            }
        }
    }

    fn notify(&mut self, msg: &str) {
        self.notification = Some((msg.to_string(), std::time::Instant::now()));
    }

    // ========================================================================
    // Key Handling
    // ========================================================================

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if self.filter_active {
            return self.handle_filter_key(key);
        }

        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q')) | (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.quit = true;
            }

            // Tab navigation between Skills/MCPs
            (_, KeyCode::Tab) => {
                self.market_tab = match self.market_tab {
                    MarketTab::Skills => MarketTab::Mcps,
                    MarketTab::Mcps => MarketTab::Skills,
                };
                self.selected = 0;
                self.list_state.select(Some(0));
                self.detail_scroll = 0;
            }

            // Filter tabs
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                self.filter_tab = match self.filter_tab {
                    FilterTab::All => FilterTab::Available,
                    FilterTab::Installed => FilterTab::All,
                    FilterTab::Available => FilterTab::Installed,
                };
                self.selected = 0;
                self.list_state.select(Some(0));
            }

            // Navigation
            (_, KeyCode::Up | KeyCode::Char('k')) => {
                let max = self.filtered_indices().len();
                if max > 0 && self.selected > 0 {
                    self.selected -= 1;
                    self.list_state.select(Some(self.selected));
                    self.detail_scroll = 0;
                }
            }

            (_, KeyCode::Down | KeyCode::Char('j')) => {
                let max = self.filtered_indices().len();
                if max > 0 && self.selected + 1 < max {
                    self.selected += 1;
                    self.list_state.select(Some(self.selected));
                    self.detail_scroll = 0;
                }
            }

            // Actions
            (_, KeyCode::Char(' ') | KeyCode::Enter) => {
                self.toggle_selected();
            }

            (_, KeyCode::Char('r')) => {
                self.load_all();
                self.notify("Refreshed marketplace");
            }

            (_, KeyCode::Char('/')) => {
                self.filter_active = true;
                self.filter.clear();
            }

            (_, KeyCode::Char('i')) => {
                if let Some(idx) = self.selected_real_idx() {
                    self.install_item(idx);
                }
            }

            (_, KeyCode::Esc) => {
                self.filter.clear();
                self.selected = 0;
                self.list_state.select(Some(0));
            }

            (_, KeyCode::PageUp) => {
                if self.detail_scroll > 0 {
                    self.detail_scroll -= 1;
                }
            }

            (_, KeyCode::PageDown) => {
                self.detail_scroll += 1;
            }

            // Number keys for filter tabs
            (_, KeyCode::Char('1')) => {
                self.filter_tab = FilterTab::All;
                self.selected = 0;
                self.list_state.select(Some(0));
            }
            (_, KeyCode::Char('2')) => {
                self.filter_tab = FilterTab::Installed;
                self.selected = 0;
                self.list_state.select(Some(0));
            }
            (_, KeyCode::Char('3')) => {
                self.filter_tab = FilterTab::Available;
                self.selected = 0;
                self.list_state.select(Some(0));
            }

            _ => {}
        }

        Ok(())
    }

    fn handle_filter_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.filter_active = false;
                self.selected = 0;
                self.list_state.select(Some(0));
            }
            KeyCode::Char(c) => {
                self.filter.push(c);
            }
            KeyCode::Backspace => {
                self.filter.pop();
            }
            _ => {}
        }
        Ok(())
    }

    // ========================================================================
    // Mouse Handling
    // ========================================================================

    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        let Some(_layout) = self.last_layout else {
            return Ok(());
        };
        let Some(cols) = self.last_content_cols else {
            return Ok(());
        };

        let pos = ratatui::layout::Position::new(mouse.column, mouse.row);

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                if cols[0].contains(pos) {
                    let max = self.filtered_indices().len();
                    if max > 0 && self.selected > 0 {
                        self.selected -= 1;
                        self.list_state.select(Some(self.selected));
                        self.detail_scroll = 0;
                    }
                } else if cols[1].contains(pos) {
                    if self.detail_scroll > 0 {
                        self.detail_scroll -= 1;
                    }
                }
            }

            MouseEventKind::ScrollDown => {
                if cols[0].contains(pos) {
                    let max = self.filtered_indices().len();
                    if max > 0 && self.selected + 1 < max {
                        self.selected += 1;
                        self.list_state.select(Some(self.selected));
                        self.detail_scroll = 0;
                    }
                } else if cols[1].contains(pos) {
                    self.detail_scroll += 1;
                }
            }

            MouseEventKind::Down(MouseButton::Left) => {
                // Click in list selects
                if cols[0].contains(pos) {
                    let rel_y = mouse.row.saturating_sub(cols[0].y);
                    let idx = rel_y.saturating_sub(1) as usize;
                    let max = self.filtered_indices().len();
                    if idx < max {
                        self.selected = idx;
                        self.list_state.select(Some(idx));
                        self.detail_scroll = 0;
                    }
                }

                // Double-click in detail toggles
                if cols[1].contains(pos) {
                    self.toggle_selected();
                }
            }

            _ => {}
        }

        Ok(())
    }

    // ========================================================================
    // Drawing
    // ========================================================================

    pub fn draw(&mut self, f: &mut Frame) {
        let area = f.area();

        // Tick notification
        if let Some((_, t)) = &self.notification {
            if t.elapsed().as_secs() > 4 {
                self.notification = None;
            }
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // header
                Constraint::Length(2), // tabs
                Constraint::Min(5),    // content
                Constraint::Length(1), // footer
            ])
            .split(area);

        self.last_layout = Some([layout[0], layout[1], layout[2], layout[3]]);

        self.draw_header(f, layout[0]);
        self.draw_tabs(f, layout[1]);
        self.draw_content(f, layout[2]);
        self.draw_footer(f, layout[3]);

        if let Some((ref msg, _)) = self.notification.clone() {
            self.draw_toast(f, msg);
        }

        if self.loading {
            self.draw_loading(f, area);
        }
    }

    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let skills_enabled = self.skills.iter().filter(|s| s.enabled).count();
        let mcps_enabled = self.mcps.iter().filter(|m| m.enabled).count();
        let skills_total = self.skills.len();
        let mcps_total = self.mcps.len();

        let left = Line::from(vec![
            Span::styled(
                " 🏪 HOUSAKY ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Marketplace ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        let right = if area.width > 60 {
            Line::from(vec![
                Span::styled(
                    format!(" 🧩 {}/{} ", skills_enabled, skills_total),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!(" 🔌 {}/{} ", mcps_enabled, mcps_total),
                    Style::default().fg(Color::Yellow),
                ),
                if !self.filter.is_empty() {
                    Span::styled(
                        format!(" filter: \"{}\" ", self.filter),
                        Style::default().fg(Color::Magenta),
                    )
                } else {
                    Span::raw("")
                },
            ])
        } else {
            Line::from(vec![
                Span::styled(
                    format!(" 🧩{} ", skills_enabled),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!(" 🔌{} ", mcps_enabled),
                    Style::default().fg(Color::Yellow),
                ),
            ])
        };

        // Responsive split for header
        let right_len = if area.width > 80 {
            40
        } else if area.width > 60 {
            30
        } else {
            20
        };
        let splits = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(10), Constraint::Length(right_len)])
            .split(area);

        f.render_widget(Paragraph::new(left), splits[0]);
        f.render_widget(Paragraph::new(right).alignment(Alignment::Right), splits[1]);
    }

    fn draw_tabs(&self, f: &mut Frame, area: Rect) {
        // Main tabs (Skills/MCPs)
        let main_tabs: Vec<Span> = MarketTab::all()
            .iter()
            .map(|t| {
                let count = match t {
                    MarketTab::Skills => self.skills.len(),
                    MarketTab::Mcps => self.mcps.len(),
                };
                let label = format!(" {} ({}) ", t.label(), count);
                if *t == self.market_tab {
                    Span::styled(
                        label,
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled(label, Style::default().fg(Color::DarkGray))
                }
            })
            .collect();

        // Filter tabs
        let filter_tabs: Vec<Span> = FilterTab::all()
            .iter()
            .map(|t| {
                let count = match t {
                    FilterTab::All => self.current_items().len(),
                    FilterTab::Installed => {
                        self.current_items().iter().filter(|i| i.installed).count()
                    }
                    FilterTab::Available => {
                        self.current_items().iter().filter(|i| !i.installed).count()
                    }
                };
                let label = format!(" {} ({}) ", t.label(), count);
                if *t == self.filter_tab {
                    Span::styled(
                        label,
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled(label, Style::default().fg(Color::DarkGray))
                }
            })
            .collect();

        let mut spans = vec![Span::raw(" ")];
        spans.extend(main_tabs);
        spans.push(Span::raw("  │  "));
        spans.extend(filter_tabs);

        if self.filter_active {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                format!("[filter: {}]", self.filter),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn draw_content(&mut self, f: &mut Frame, area: Rect) {
        // Responsive split: use 40/60 on narrow terminals, 45/55 on wider
        let left_pct = if area.width < 80 { 40 } else { 45 };
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(left_pct),
                Constraint::Percentage(100 - left_pct),
            ])
            .split(area);

        self.last_content_cols = Some([cols[0], cols[1]]);

        self.draw_item_list(f, cols[0]);
        self.draw_item_detail(f, cols[1]);
    }

    fn draw_item_list(&mut self, f: &mut Frame, area: Rect) {
        let filtered = self.filtered_indices();
        let type_label = match self.market_tab {
            MarketTab::Skills => "Skills",
            MarketTab::Mcps => "MCPs",
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(Span::styled(
                format!(" {} ({}) ", type_label, filtered.len()),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        f.render_widget(block, area);

        if filtered.is_empty() {
            let msg = if self.filter.is_empty() {
                "No items found. Press 'r' to refresh."
            } else {
                "No items match the filter."
            };
            f.render_widget(
                Paragraph::new(Span::styled(
                    format!("  {}", msg),
                    Style::default().fg(Color::DarkGray),
                )),
                inner,
            );
            return;
        }

        // Responsive name width
        let name_width = (area.width.saturating_sub(15)).max(10) as usize;
        let items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(display_idx, &real_idx)| {
                let item = &self.current_items()[real_idx];
                let selected = display_idx == self.selected;
                let row_bg = if selected {
                    Color::Rgb(20, 40, 50)
                } else {
                    Color::Reset
                };
                let row_mod = if selected {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                };

                ListItem::new(vec![Line::from(vec![
                    Span::styled(
                        format!("{} ", item.status_icon()),
                        Style::default().fg(item.status_color()),
                    ),
                    Span::styled(
                        format!("{:width$}", item.name, width = name_width),
                        Style::default()
                            .fg(Color::White)
                            .bg(row_bg)
                            .add_modifier(row_mod),
                    ),
                    if area.width > 30 {
                        Span::styled(
                            format!(" [{}]", item.source.label()),
                            Style::default().fg(item.source.color()).bg(row_bg),
                        )
                    } else {
                        Span::raw("")
                    },
                    if item.installed && area.width > 40 {
                        Span::styled(" ✓", Style::default().fg(Color::Green))
                    } else {
                        Span::raw("")
                    },
                ])])
            })
            .collect();

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(self.selected));

        f.render_stateful_widget(
            List::new(items).highlight_style(Style::default().bg(Color::Rgb(20, 40, 50))),
            inner,
            &mut list_state,
        );
    }

    fn draw_item_detail(&self, f: &mut Frame, area: Rect) {
        let filtered = self.filtered_indices();
        let real_idx = filtered.get(self.selected).copied();

        let title = real_idx
            .and_then(|i| self.current_items().get(i))
            .map(|s| format!(" {} ", s.name))
            .unwrap_or_else(|| " Details ".to_string());

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let item = match real_idx.and_then(|i| self.current_items().get(i)) {
            Some(s) => s,
            None => {
                f.render_widget(
                    Paragraph::new(Span::styled(
                        "Select an item",
                        Style::default().fg(Color::DarkGray),
                    )),
                    inner,
                );
                return;
            }
        };

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(if area.width < 40 { 7 } else { 10 }), // info - shorter on narrow
                Constraint::Min(0),                                       // description
            ])
            .split(inner);

        // Info section
        let (status_label, status_color) = if item.enabled {
            ("● ENABLED", Color::Green)
        } else if item.installed {
            ("○ DISABLED", Color::Yellow)
        } else {
            ("◌ AVAILABLE", Color::DarkGray)
        };

        let info_lines = vec![
            Line::from(Span::styled(
                item.name.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Status:   ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    status_label,
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Type:     ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    match item.item_type {
                        MarketTab::Skills => "Skill",
                        MarketTab::Mcps => "MCP Server",
                    },
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(vec![
                Span::styled("Source:   ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    item.source.label(),
                    Style::default().fg(item.source.color()),
                ),
            ]),
            Line::from(vec![
                Span::styled("Version:  ", Style::default().fg(Color::DarkGray)),
                Span::styled(item.version.clone(), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Tags:     ", Style::default().fg(Color::DarkGray)),
                Span::styled(item.tags.join(", "), Style::default().fg(Color::Cyan)),
            ]),
            if let Some(ref cmd) = item.command {
                Line::from(vec![
                    Span::styled("Command:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(cmd, Style::default().fg(Color::Yellow)),
                ])
            } else {
                Line::from("")
            },
        ];
        f.render_widget(Paragraph::new(info_lines), layout[0]);

        // Description
        let content = item
            .readme
            .clone()
            .unwrap_or_else(|| item.description.clone());
        let lines: Vec<Line> = content
            .lines()
            .skip(self.detail_scroll)
            .map(|l| {
                if l.starts_with("# ") {
                    Line::from(Span::styled(
                        l,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else if l.starts_with("## ") {
                    Line::from(Span::styled(
                        l,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else if l.starts_with("- ") || l.starts_with("* ") {
                    Line::from(vec![
                        Span::styled("  • ", Style::default().fg(Color::Green)),
                        Span::raw(l[2..].to_string()),
                    ])
                } else {
                    Line::from(Span::raw(l.to_string()))
                }
            })
            .collect();

        let desc_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(Span::styled(
                " Description ",
                Style::default().fg(Color::DarkGray),
            ));

        let desc_inner = desc_block.inner(layout[1]);
        f.render_widget(desc_block, layout[1]);
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), desc_inner);
    }

    fn draw_footer(&self, f: &mut Frame, area: Rect) {
        let hint = if self.filter_active {
            " Type to filter — Enter/Esc to confirm "
        } else if area.width < 80 {
            " ↑↓/jk=nav  Space=toggle  i=install  Tab=tab  /=search  r=refresh  q=quit "
        } else {
            " ↑↓/jk=navigate  Space/Enter=toggle  i=install  Tab=Skills/MCPs  1/2/3=filter  /=search  r=refresh  q=quit "
        };
        f.render_widget(
            Paragraph::new(Span::styled(hint, Style::default().fg(Color::DarkGray))),
            area,
        );
    }

    fn draw_toast(&self, f: &mut Frame, msg: &str) {
        let width = (msg.len() + 6).min(f.area().width as usize) as u16;
        let area = Rect::new(f.area().width.saturating_sub(width + 1), 1, width, 3);

        let toast = Paragraph::new(msg)
            .style(Style::default().fg(Color::Black).bg(Color::Cyan))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            );

        f.render_widget(Clear, area);
        f.render_widget(toast, area);
    }

    fn draw_loading(&self, f: &mut Frame, area: Rect) {
        let loading = Paragraph::new(" Loading marketplace...")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);

        let center = Rect::new(area.width / 4, area.height / 2 - 1, area.width / 2, 3);
        f.render_widget(Clear, center);
        f.render_widget(
            loading.block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            ),
            center,
        );
    }
}

impl Default for UnifiedMarketApp {
    fn default() -> Self {
        let config = Config::load_or_init().unwrap_or_default();
        let workspace_dir = config.workspace_dir.clone();
        let repo_root = std::env::current_dir().unwrap_or_else(|_| workspace_dir.clone());
        Self::new(config, workspace_dir, repo_root)
    }
}

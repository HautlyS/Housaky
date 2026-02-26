use crate::config::Config;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap,
    },
    Frame,
};
use std::path::PathBuf;

// ─── Data ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSource {
    Local,
    OpenClaw,
    ClaudeOfficial,
    Config,
}

impl SkillSource {
    fn label(&self) -> &'static str {
        match self {
            SkillSource::Local          => "local",
            SkillSource::OpenClaw       => "openclaw",
            SkillSource::ClaudeOfficial => "claude",
            SkillSource::Config         => "config",
        }
    }
    fn color(&self) -> Color {
        match self {
            SkillSource::Local          => Color::Cyan,
            SkillSource::OpenClaw       => Color::Green,
            SkillSource::ClaudeOfficial => Color::Yellow,
            SkillSource::Config         => Color::Gray,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MarketSkill {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub source: SkillSource,
    pub version: String,
    pub tags: Vec<String>,
    pub path: Option<PathBuf>,
    pub readme: Option<String>,
}

impl MarketSkill {
    fn status_color(&self) -> Color {
        if self.enabled { Color::Green } else { Color::DarkGray }
    }
    fn status_icon(&self) -> &'static str {
        if self.enabled { "●" } else { "○" }
    }
}

// ─── App State ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkillTab {
    All,
    Enabled,
    Available,
}

impl SkillTab {
    fn all() -> &'static [SkillTab] { &[SkillTab::All, SkillTab::Enabled, SkillTab::Available] }
    fn label(&self) -> &'static str {
        match self {
            SkillTab::All       => "All",
            SkillTab::Enabled   => "Enabled",
            SkillTab::Available => "Available",
        }
    }
    fn index(&self) -> usize { Self::all().iter().position(|t| t == self).unwrap_or(0) }
    fn next(&self) -> SkillTab { let a = Self::all(); a[(self.index() + 1) % a.len()] }
    fn prev(&self) -> SkillTab { let a = Self::all(); a[if self.index() == 0 { a.len()-1 } else { self.index()-1 }] }
}

pub struct SkillsMarketApp {
    pub config: Config,
    pub repo_root: PathBuf,
    pub items: Vec<MarketSkill>,
    pub selected: usize,
    pub list_state: ratatui::widgets::ListState,
    pub status: String,
    pub quit: bool,

    tab: SkillTab,
    filter: String,
    filter_active: bool,
    notification: Option<(String, std::time::Instant)>,
    detail_scroll: usize,
}

impl SkillsMarketApp {
    pub fn new(config: Config, repo_root: PathBuf) -> Self {
        let mut ls = ratatui::widgets::ListState::default();
        ls.select(Some(0));
        Self {
            config,
            repo_root,
            items: Vec::new(),
            selected: 0,
            list_state: ls,
            status: String::new(),
            quit: false,
            tab: SkillTab::All,
            filter: String::new(),
            filter_active: false,
            notification: None,
            detail_scroll: 0,
        }
    }

    pub fn load_skills(&mut self) {
        self.items.clear();

        // Local skills from workspace/skills
        let skills_dir = self.repo_root.join("skills");
        if let Ok(rd) = std::fs::read_dir(&skills_dir) {
            for entry in rd.flatten() {
                let path = entry.path();
                if !path.is_dir() { continue; }
                let name = entry.file_name().to_string_lossy().to_string();
                let enabled = path.join("enabled").exists() || path.join("SKILL.toml").exists();
                let readme_path = path.join("SKILL.md");
                let readme = std::fs::read_to_string(&readme_path).ok();
                let description = readme.as_deref()
                    .and_then(|r| r.lines().find(|l| !l.trim().is_empty() && !l.starts_with('#')))
                    .unwrap_or("No description available")
                    .trim()
                    .chars()
                    .take(100)
                    .collect::<String>();

                self.items.push(MarketSkill {
                    name: name.clone(),
                    description,
                    enabled,
                    source: SkillSource::Local,
                    version: "local".to_string(),
                    tags: vec!["local".to_string()],
                    path: Some(path),
                    readme,
                });
            }
        }

        // Skills from config
        for (skill_name, skill_enabled) in &self.config.skills.enabled {
            if !self.items.iter().any(|s| &s.name == skill_name) {
                self.items.push(MarketSkill {
                    name: skill_name.clone(),
                    description: "Configured skill".to_string(),
                    enabled: *skill_enabled,
                    source: SkillSource::Config,
                    version: "config".to_string(),
                    tags: vec!["config".to_string()],
                    path: None,
                    readme: None,
                });
            }
        }

        // Fallback demo items if nothing found
        if self.items.is_empty() {
            self.items.extend(vec![
                MarketSkill {
                    name: "get-shit-done".to_string(),
                    description: "Productivity & execution focused AI skill pack. Task decomposition, action planning, execution tracking.".to_string(),
                    enabled: true,
                    source: SkillSource::Local,
                    version: "1.0.0".to_string(),
                    tags: vec!["productivity".to_string(), "planning".to_string()],
                    path: Some(self.repo_root.join("skills/get-shit-done")),
                    readme: None,
                },
                MarketSkill {
                    name: "ui-ux-pro-max".to_string(),
                    description: "Advanced UI/UX design, implementation patterns, accessibility, and design systems.".to_string(),
                    enabled: false,
                    source: SkillSource::Local,
                    version: "1.0.0".to_string(),
                    tags: vec!["design".to_string(), "frontend".to_string()],
                    path: Some(self.repo_root.join("skills/ui-ux-pro-max")),
                    readme: None,
                },
                MarketSkill {
                    name: "code-review".to_string(),
                    description: "Deep code review: security, performance, maintainability, best practices.".to_string(),
                    enabled: false,
                    source: SkillSource::OpenClaw,
                    version: "0.8.2".to_string(),
                    tags: vec!["review".to_string(), "quality".to_string()],
                    path: None,
                    readme: None,
                },
                MarketSkill {
                    name: "data-analysis".to_string(),
                    description: "Statistical analysis, data visualization, CSV/JSON data processing.".to_string(),
                    enabled: false,
                    source: SkillSource::OpenClaw,
                    version: "1.2.1".to_string(),
                    tags: vec!["data".to_string(), "analysis".to_string()],
                    path: None,
                    readme: None,
                },
                MarketSkill {
                    name: "computer-use".to_string(),
                    description: "Claude's official computer use capability — desktop automation.".to_string(),
                    enabled: false,
                    source: SkillSource::ClaudeOfficial,
                    version: "official".to_string(),
                    tags: vec!["automation".to_string(), "computer".to_string()],
                    path: None,
                    readme: None,
                },
                MarketSkill {
                    name: "web-search".to_string(),
                    description: "Real-time web search and content extraction capability.".to_string(),
                    enabled: false,
                    source: SkillSource::ClaudeOfficial,
                    version: "official".to_string(),
                    tags: vec!["search".to_string(), "web".to_string()],
                    path: None,
                    readme: None,
                },
            ]);
        }

        self.status = format!("{} skills loaded", self.items.len());
        if self.selected >= self.items.len() { self.selected = 0; }
        self.list_state.select(Some(self.selected));
    }

    fn filtered_indices(&self) -> Vec<usize> {
        let q = self.filter.to_lowercase();
        self.items.iter().enumerate()
            .filter(|(_, s)| {
                let tab_ok = match self.tab {
                    SkillTab::All       => true,
                    SkillTab::Enabled   => s.enabled,
                    SkillTab::Available => !s.enabled,
                };
                let filter_ok = if q.is_empty() { true } else {
                    s.name.to_lowercase().contains(&q) ||
                    s.description.to_lowercase().contains(&q) ||
                    s.tags.iter().any(|t| t.to_lowercase().contains(&q))
                };
                tab_ok && filter_ok
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn selected_real_idx(&self) -> Option<usize> {
        let filtered = self.filtered_indices();
        filtered.get(self.selected).copied()
    }

    fn toggle_selected(&mut self) {
        if let Some(idx) = self.selected_real_idx() {
            let skill = &mut self.items[idx];
            skill.enabled = !skill.enabled;
            let msg = if skill.enabled {
                format!("Enabled: {}", skill.name)
            } else {
                format!("Disabled: {}", skill.name)
            };
            self.status = msg.clone();
            self.notify(&msg);
        }
    }

    fn notify(&mut self, msg: &str) {
        self.notification = Some((msg.to_string(), std::time::Instant::now()));
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if self.filter_active {
            return self.handle_filter_key(key);
        }
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q')) | (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.quit = true;
            }
            (_, KeyCode::Tab) => {
                self.tab = self.tab.next();
                self.selected = 0;
                self.list_state.select(Some(0));
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                self.tab = self.tab.prev();
                self.selected = 0;
                self.list_state.select(Some(0));
            }
            (_, KeyCode::Up) | (_, KeyCode::Char('k')) => {
                let max = self.filtered_indices().len();
                if max > 0 && self.selected > 0 {
                    self.selected -= 1;
                    self.list_state.select(Some(self.selected));
                    self.detail_scroll = 0;
                }
            }
            (_, KeyCode::Down) | (_, KeyCode::Char('j')) => {
                let max = self.filtered_indices().len();
                if max > 0 && self.selected + 1 < max {
                    self.selected += 1;
                    self.list_state.select(Some(self.selected));
                    self.detail_scroll = 0;
                }
            }
            (_, KeyCode::Char(' ')) | (_, KeyCode::Enter) => {
                self.toggle_selected();
            }
            (_, KeyCode::Char('r')) => {
                self.load_skills();
                self.notify("Skills refreshed");
            }
            (_, KeyCode::Char('/')) => {
                self.filter_active = true;
                self.filter.clear();
            }
            (_, KeyCode::Esc) => {
                self.filter.clear();
                self.selected = 0;
                self.list_state.select(Some(0));
            }
            (_, KeyCode::PageUp) => {
                if self.detail_scroll > 0 { self.detail_scroll -= 1; }
            }
            (_, KeyCode::PageDown) => {
                self.detail_scroll += 1;
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
            KeyCode::Char(c) => { self.filter.push(c); }
            KeyCode::Backspace => { self.filter.pop(); }
            _ => {}
        }
        Ok(())
    }

    pub fn draw(&mut self, f: &mut Frame) {
        let area = f.area();

        // Tick notification expiry
        if let Some((_, t)) = &self.notification {
            if t.elapsed().as_secs() > 4 { self.notification = None; }
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),  // header
                Constraint::Length(2),  // tabs
                Constraint::Min(5),     // content
                Constraint::Length(1),  // footer
            ])
            .split(area);

        self.draw_header(f, layout[0]);
        self.draw_tabs(f, layout[1]);
        self.draw_content(f, layout[2]);
        self.draw_footer(f, layout[3]);

        if let Some((ref msg, _)) = self.notification.clone() {
            self.draw_toast(f, msg);
        }
    }

    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let enabled_count = self.items.iter().filter(|s| s.enabled).count();
        let total = self.items.len();

        let left = Line::from(vec![
            Span::styled(" 🧩 HOUSAKY ", Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(" Skills Marketplace ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]);
        let right = Line::from(vec![
            Span::styled(format!(" {}/{} enabled ", enabled_count, total), Style::default().fg(Color::Green)),
            if !self.filter.is_empty() {
                Span::styled(format!("  filter: \"{}\" ", self.filter), Style::default().fg(Color::Yellow))
            } else {
                Span::raw("")
            },
        ]);
        let splits = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(10), Constraint::Length(30)])
            .split(area);
        f.render_widget(Paragraph::new(left), splits[0]);
        f.render_widget(Paragraph::new(right).alignment(Alignment::Right), splits[1]);
    }

    fn draw_tabs(&self, f: &mut Frame, area: Rect) {
        let tab_titles: Vec<Span> = SkillTab::all().iter().map(|t| {
            let count = match t {
                SkillTab::All       => self.items.len(),
                SkillTab::Enabled   => self.items.iter().filter(|s| s.enabled).count(),
                SkillTab::Available => self.items.iter().filter(|s| !s.enabled).count(),
            };
            let label = format!(" {} ({}) ", t.label(), count);
            if *t == self.tab {
                Span::styled(label, Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD))
            } else {
                Span::styled(label, Style::default().fg(Color::DarkGray))
            }
        }).collect();

        let filter_hint = if self.filter_active {
            Span::styled(
                format!(" /{}| ", self.filter),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )
        } else if !self.filter.is_empty() {
            Span::styled(
                format!(" filter: {} ", self.filter),
                Style::default().fg(Color::Yellow),
            )
        } else {
            Span::styled(" / to filter ", Style::default().fg(Color::DarkGray))
        };

        let mut spans = vec![Span::raw(" ")];
        spans.extend(tab_titles);
        spans.push(Span::raw("   "));
        spans.push(filter_hint);

        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn draw_content(&mut self, f: &mut Frame, area: Rect) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
            .split(area);

        self.draw_skill_list(f, cols[0]);
        self.draw_skill_detail(f, cols[1]);
    }

    fn draw_skill_list(&mut self, f: &mut Frame, area: Rect) {
        let filtered = self.filtered_indices();
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(Span::styled(
                format!(" Skills ({}) ", filtered.len()),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        f.render_widget(block, area);

        if filtered.is_empty() {
            let msg = if self.filter.is_empty() {
                "No skills found. Press r to refresh."
            } else {
                "No skills match the filter."
            };
            f.render_widget(
                Paragraph::new(Span::styled(format!("  {}", msg), Style::default().fg(Color::DarkGray))),
                inner,
            );
            return;
        }

        let items: Vec<ListItem> = filtered.iter().enumerate().map(|(display_idx, &real_idx)| {
            let skill = &self.items[real_idx];
            let selected = display_idx == self.selected;
            let row_bg = if selected { Color::Rgb(20, 40, 50) } else { Color::Reset };
            let row_mod = if selected { Modifier::BOLD } else { Modifier::empty() };

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("{} ", skill.status_icon()),
                        Style::default().fg(skill.status_color()),
                    ),
                    Span::styled(
                        format!("{:22}", skill.name),
                        Style::default().fg(Color::White).bg(row_bg).add_modifier(row_mod),
                    ),
                    Span::styled(
                        format!(" [{}]", skill.source.label()),
                        Style::default().fg(skill.source.color()).bg(row_bg).add_modifier(Modifier::DIM),
                    ),
                ]),
            ])
        }).collect();

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(self.selected));
        f.render_stateful_widget(
            List::new(items).highlight_style(Style::default().bg(Color::Rgb(20, 40, 50))),
            inner,
            &mut list_state,
        );
    }

    fn draw_skill_detail(&self, f: &mut Frame, area: Rect) {
        let filtered = self.filtered_indices();
        let real_idx = filtered.get(self.selected).copied();

        let title = real_idx
            .and_then(|i| self.items.get(i))
            .map(|s| format!(" {} ", s.name))
            .unwrap_or_else(|| " Details ".to_string());

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta))
            .title(Span::styled(title, Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let skill = match real_idx.and_then(|i| self.items.get(i)) {
            Some(s) => s,
            None => {
                f.render_widget(
                    Paragraph::new(Span::styled("Select a skill", Style::default().fg(Color::DarkGray))),
                    inner,
                );
                return;
            }
        };

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7),  // info
                Constraint::Length(3),  // gauge
                Constraint::Min(0),     // readme / description
            ])
            .split(inner);

        // Info section
        let (status_label, status_color) = if skill.enabled {
            ("● ENABLED", Color::Green)
        } else {
            ("○ DISABLED", Color::DarkGray)
        };

        let info_lines = vec![
            Line::from(Span::styled(
                skill.name.clone(),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Status:   ", Style::default().fg(Color::DarkGray)),
                Span::styled(status_label, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Source:   ", Style::default().fg(Color::DarkGray)),
                Span::styled(skill.source.label(), Style::default().fg(skill.source.color())),
            ]),
            Line::from(vec![
                Span::styled("Version:  ", Style::default().fg(Color::DarkGray)),
                Span::styled(skill.version.clone(), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Tags:     ", Style::default().fg(Color::DarkGray)),
                Span::styled(skill.tags.join(", "), Style::default().fg(Color::Cyan)),
            ]),
        ];
        f.render_widget(Paragraph::new(info_lines), layout[0]);

        // Enabled ratio gauge
        let enabled_total = self.items.iter().filter(|s| s.enabled).count();
        let ratio = if self.items.is_empty() { 0.0 } else {
            enabled_total as f64 / self.items.len() as f64
        };
        let gauge = Gauge::default()
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(Span::styled(" Marketplace Activation ", Style::default().fg(Color::DarkGray))))
            .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray))
            .ratio(ratio)
            .label(format!("{}/{} enabled", enabled_total, self.items.len()));
        f.render_widget(gauge, layout[2].clone());
        // We borrow layout[2] below so use layout[1] for gauge and layout[2] for readme
        let gauge2 = Gauge::default()
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(Span::styled(" Activation ", Style::default().fg(Color::DarkGray))))
            .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray))
            .ratio(ratio)
            .label(format!("{}/{}", enabled_total, self.items.len()));
        f.render_widget(gauge2, layout[1]);

        // Readme / description
        let content = skill.readme.clone().unwrap_or_else(|| skill.description.clone());
        let lines: Vec<Line> = content.lines().skip(self.detail_scroll).map(|l| {
            if l.starts_with("# ") {
                Line::from(Span::styled(l.to_string(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
            } else if l.starts_with("## ") {
                Line::from(Span::styled(l.to_string(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
            } else if l.starts_with("- ") || l.starts_with("* ") {
                Line::from(vec![
                    Span::styled("  • ", Style::default().fg(Color::Green)),
                    Span::raw(l[2..].to_string()),
                ])
            } else if l.starts_with("```") {
                Line::from(Span::styled(l.to_string(), Style::default().fg(Color::DarkGray).bg(Color::Rgb(30, 30, 30))))
            } else {
                Line::from(Span::raw(l.to_string()))
            }
        }).collect();

        let readme_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(Span::styled(" Description / README ", Style::default().fg(Color::DarkGray)));
        let readme_inner = readme_block.inner(layout[2]);
        f.render_widget(readme_block, layout[2]);
        f.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }),
            readme_inner,
        );
    }

    fn draw_footer(&self, f: &mut Frame, area: Rect) {
        let hint = if self.filter_active {
            " Type to filter — Enter/Esc to confirm "
        } else {
            " ↑↓/jk=navigate  Space/Enter=toggle  Tab=tab  /=filter  r=refresh  PgUp/PgDn=scroll  q=quit "
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
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
        f.render_widget(Clear, area);
        f.render_widget(toast, area);
    }
}

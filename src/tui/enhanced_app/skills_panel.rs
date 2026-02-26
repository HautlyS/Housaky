use std::path::PathBuf;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};
use crate::tui::enhanced_app::theme::{
    Palette, render_gauge_bar, style_border, style_border_active, style_border_focus,
    style_dim, style_muted, style_tag_skill, style_title, style_error, style_success,
    style_warning,
};

// ── Skill source ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSource {
    Local,
    OpenClaw,
    ClaudeOfficial,
    Config,
    Remote,
}

impl SkillSource {
    pub fn label(&self) -> &'static str {
        match self {
            SkillSource::Local          => "local",
            SkillSource::OpenClaw       => "openclaw",
            SkillSource::ClaudeOfficial => "claude",
            SkillSource::Config         => "config",
            SkillSource::Remote         => "remote",
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        match self {
            SkillSource::Local          => Palette::CYAN,
            SkillSource::OpenClaw       => Palette::SUCCESS,
            SkillSource::ClaudeOfficial => Palette::WARNING,
            SkillSource::Config         => Palette::TEXT_DIM,
            SkillSource::Remote         => Palette::VIOLET,
        }
    }

    pub fn badge(&self) -> &'static str {
        match self {
            SkillSource::Local          => "local",
            SkillSource::OpenClaw       => "openclaw",
            SkillSource::ClaudeOfficial => "claude",
            SkillSource::Config         => "cfg",
            SkillSource::Remote         => "remote",
        }
    }
}

// ── Skill entry ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Skill {
    pub name:        String,
    pub description: String,
    pub enabled:     bool,
    pub source:      SkillSource,
    pub version:     String,
    pub tags:        Vec<String>,
    pub path:        Option<PathBuf>,
    pub readme:      Option<String>,
}

impl Skill {
    pub fn status_icon(&self) -> &'static str {
        if self.enabled { "●" } else { "○" }
    }

    pub fn status_color(&self) -> ratatui::style::Color {
        if self.enabled { Palette::SUCCESS } else { Palette::TEXT_MUTED }
    }
}

// ── Tab ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkillTab { All, Enabled, Available }

impl SkillTab {
    const ALL: &'static [SkillTab] = &[SkillTab::All, SkillTab::Enabled, SkillTab::Available];
    fn label(self) -> &'static str {
        match self { SkillTab::All => " All ", SkillTab::Enabled => " Enabled ", SkillTab::Available => " Available " }
    }
    fn index(self) -> usize { Self::ALL.iter().position(|t| *t == self).unwrap_or(0) }
    fn next(self) -> SkillTab { Self::ALL[(self.index() + 1) % Self::ALL.len()] }
    fn prev(self) -> SkillTab {
        let len = Self::ALL.len();
        Self::ALL[(self.index() + len - 1) % len]
    }
}

// ── Skills panel state ────────────────────────────────────────────────────────

pub struct SkillsPanel {
    pub skills:        Vec<Skill>,
    pub selected:      usize,
    pub status:        String,
    tab:               SkillTab,
    filter:            String,
    filter_active:     bool,
    detail_scroll:     usize,
    notification:      Option<(String, std::time::Instant)>,
}

impl SkillsPanel {
    pub fn new() -> Self {
        Self {
            skills:        Vec::new(),
            selected:      0,
            status:        "No skills loaded".to_string(),
            tab:           SkillTab::All,
            filter:        String::new(),
            filter_active: false,
            detail_scroll: 0,
            notification:  None,
        }
    }

    // ── Data loading ──────────────────────────────────────────────────────────

    pub fn load_from_paths(&mut self, skills_dir: &PathBuf, config_skills: &[(String, bool)]) {
        self.skills.clear();

        if let Ok(rd) = std::fs::read_dir(skills_dir) {
            for entry in rd.flatten() {
                let path = entry.path();
                if !path.is_dir() { continue; }
                let name = entry.file_name().to_string_lossy().to_string();
                let enabled = path.join("enabled").exists() || path.join("SKILL.toml").exists();
                let readme = std::fs::read_to_string(path.join("SKILL.md")).ok();
                let description = readme.as_deref()
                    .and_then(|r| r.lines().find(|l| !l.trim().is_empty() && !l.starts_with('#')))
                    .unwrap_or("No description available")
                    .trim().chars().take(120).collect();
                self.skills.push(Skill {
                    name,
                    description,
                    enabled,
                    source: SkillSource::Local,
                    version: "local".into(),
                    tags: vec!["local".into()],
                    path: Some(path),
                    readme,
                });
            }
        }

        for (name, en) in config_skills {
            if !self.skills.iter().any(|s| &s.name == name) {
                self.skills.push(Skill {
                    name: name.clone(),
                    description: "Configured skill".into(),
                    enabled: *en,
                    source: SkillSource::Config,
                    version: "config".into(),
                    tags: vec!["config".into()],
                    path: None,
                    readme: None,
                });
            }
        }

        if self.skills.is_empty() {
            self.inject_demo_skills();
        }

        self.status = format!("{} skills", self.skills.len());
        if self.selected >= self.skills.len() { self.selected = 0; }
    }

    fn inject_demo_skills(&mut self) {
        let demos: &[(&str, &str, bool, SkillSource, &str)] = &[
            ("get-shit-done",   "Productivity & execution: task decomposition, planning, tracking.", true,  SkillSource::Local,          "1.0.0"),
            ("ui-ux-pro-max",   "Advanced UI/UX design, a11y, design systems, Figma patterns.",     false, SkillSource::Local,          "1.0.0"),
            ("code-review",     "Deep code review: security, perf, maintainability.",                false, SkillSource::OpenClaw,       "0.9.1"),
            ("data-analysis",   "Stats, visualization, CSV/JSON data processing.",                   false, SkillSource::OpenClaw,       "1.2.1"),
            ("computer-use",    "Claude official: desktop automation.",                              false, SkillSource::ClaudeOfficial, "official"),
            ("web-search",      "Real-time web search + content extraction.",                        false, SkillSource::ClaudeOfficial, "official"),
            ("memory-graph",    "Long-term knowledge graph management.",                             true,  SkillSource::Local,          "0.4.2"),
            ("rust-expert",     "Advanced Rust: unsafe, async, macro development.",                  false, SkillSource::OpenClaw,       "1.0.0"),
        ];
        for (name, desc, enabled, source, version) in demos {
            self.skills.push(Skill {
                name:        name.to_string(),
                description: desc.to_string(),
                enabled:     *enabled,
                source:      source.clone(),
                version:     version.to_string(),
                tags:        vec![],
                path:        None,
                readme:      None,
            });
        }
    }

    // ── Filtering ─────────────────────────────────────────────────────────────

    fn filtered_indices(&self) -> Vec<usize> {
        let q = self.filter.to_lowercase();
        self.skills.iter().enumerate()
            .filter(|(_, s)| {
                let tab_ok = match self.tab {
                    SkillTab::All       => true,
                    SkillTab::Enabled   => s.enabled,
                    SkillTab::Available => !s.enabled,
                };
                let filter_ok = q.is_empty()
                    || s.name.to_lowercase().contains(&q)
                    || s.description.to_lowercase().contains(&q)
                    || s.tags.iter().any(|t| t.to_lowercase().contains(&q));
                tab_ok && filter_ok
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn selected_real_idx(&self) -> Option<usize> {
        self.filtered_indices().get(self.selected).copied()
    }

    // ── Interaction ───────────────────────────────────────────────────────────

    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.detail_scroll = 0;
        }
    }

    pub fn select_next(&mut self) {
        let max = self.filtered_indices().len();
        if max > 0 && self.selected + 1 < max {
            self.selected += 1;
            self.detail_scroll = 0;
        }
    }

    pub fn toggle_selected(&mut self) -> Option<bool> {
        let idx = self.selected_real_idx()?;
        let skill = &mut self.skills[idx];
        skill.enabled = !skill.enabled;
        let msg = if skill.enabled {
            format!("Enabled: {}", skill.name)
        } else {
            format!("Disabled: {}", skill.name)
        };
        self.status = msg.clone();
        self.notification = Some((msg, std::time::Instant::now()));
        Some(self.skills[idx].enabled)
    }

    pub fn tab_next(&mut self) { self.tab = self.tab.next(); self.selected = 0; }
    pub fn tab_prev(&mut self) { self.tab = self.tab.prev(); self.selected = 0; }

    pub fn start_filter(&mut self) { self.filter_active = true; self.filter.clear(); }
    pub fn filter_push(&mut self, c: char) { self.filter.push(c); self.selected = 0; }
    pub fn filter_pop(&mut self) { self.filter.pop(); self.selected = 0; }
    pub fn filter_commit(&mut self) { self.filter_active = false; self.selected = 0; }

    pub fn detail_scroll_up(&mut self) { self.detail_scroll = self.detail_scroll.saturating_sub(1); }
    pub fn detail_scroll_down(&mut self) { self.detail_scroll += 1; }

    pub fn is_filter_active(&self) -> bool { self.filter_active }

    pub fn tick(&mut self) {
        if let Some((_, t)) = &self.notification {
            if t.elapsed().as_secs() > 3 { self.notification = None; }
        }
    }

    // ── Draw ──────────────────────────────────────────────────────────────────

    pub fn draw(&self, f: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // tab bar
                Constraint::Min(5),    // content
                Constraint::Length(1), // footer hint
            ])
            .split(area);

        self.draw_tab_bar(f, layout[0]);
        self.draw_content(f, layout[1]);
        self.draw_footer(f, layout[2]);

        if let Some((ref msg, _)) = self.notification {
            self.draw_toast(f, area, msg);
        }
    }

    fn draw_tab_bar(&self, f: &mut Frame, area: Rect) {
        let titles: Vec<Span> = SkillTab::ALL.iter().map(|&t| {
            let count = match t {
                SkillTab::All       => self.skills.len(),
                SkillTab::Enabled   => self.skills.iter().filter(|s| s.enabled).count(),
                SkillTab::Available => self.skills.iter().filter(|s| !s.enabled).count(),
            };
            let label = format!("{} ({}) ", t.label(), count);
            if t == self.tab {
                Span::styled(label, ratatui::style::Style::default().fg(Palette::BG).bg(Palette::CYAN).add_modifier(Modifier::BOLD))
            } else {
                Span::styled(label, style_muted())
            }
        }).collect();

        let mut spans = vec![Span::raw(" ")];
        spans.extend(titles);
        if !self.filter.is_empty() || self.filter_active {
            spans.push(Span::styled(
                format!("   /{}{}  ", self.filter, if self.filter_active { "|" } else { "" }),
                ratatui::style::Style::default().fg(Palette::WARNING).add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled("   / filter ", style_muted()));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn draw_content(&self, f: &mut Frame, area: Rect) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);
        self.draw_list(f, cols[0]);
        self.draw_detail(f, cols[1]);
    }

    fn draw_list(&self, f: &mut Frame, area: Rect) {
        let filtered = self.filtered_indices();
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border_focus())
            .title(Span::styled(
                format!(" 🧩 Skills ({}) ", filtered.len()),
                style_tag_skill(),
            ));
        let inner = block.inner(area);
        f.render_widget(block, area);

        if filtered.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled("  No skills found", style_muted())),
                inner,
            );
            return;
        }

        let items: Vec<ListItem> = filtered.iter().enumerate().map(|(di, &ri)| {
            let s = &self.skills[ri];
            let sel = di == self.selected;
            let bg = if sel { Palette::BG_SELECTED } else { Palette::BG_PANEL };
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!(" {} ", s.status_icon()),
                        ratatui::style::Style::default().fg(s.status_color()).bg(bg),
                    ),
                    Span::styled(
                        format!("{:<20}", truncate(&s.name, 20)),
                        ratatui::style::Style::default()
                            .fg(if sel { Palette::TEXT_BRIGHT } else { Palette::TEXT })
                            .bg(bg)
                            .add_modifier(if sel { Modifier::BOLD } else { Modifier::empty() }),
                    ),
                    Span::styled(
                        format!(" {:7}", s.source.badge()),
                        ratatui::style::Style::default().fg(s.source.color()).bg(bg).add_modifier(Modifier::DIM),
                    ),
                ]),
            ])
        }).collect();

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(self.selected));
        f.render_stateful_widget(
            List::new(items).highlight_style(ratatui::style::Style::default().bg(Palette::BG_SELECTED)),
            inner,
            &mut list_state,
        );
    }

    fn draw_detail(&self, f: &mut Frame, area: Rect) {
        let filtered = self.filtered_indices();
        let real_idx = filtered.get(self.selected).copied();

        let title = real_idx
            .and_then(|i| self.skills.get(i))
            .map(|s| format!(" {} ", s.name))
            .unwrap_or_else(|| " Detail ".to_string());

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border_active())
            .title(Span::styled(title, style_tag_skill()));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let skill = match real_idx.and_then(|i| self.skills.get(i)) {
            Some(s) => s,
            None => {
                f.render_widget(Paragraph::new(Span::styled("  Select a skill to view details", style_muted())), inner);
                return;
            }
        };

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Length(3), Constraint::Min(4)])
            .split(inner);

        // Info block
        let (status_label, status_style) = if skill.enabled {
            ("● ENABLED", style_success())
        } else {
            ("○ DISABLED", style_muted())
        };
        let info = vec![
            Line::from(Span::styled(&skill.name, ratatui::style::Style::default().fg(Palette::TEXT_BRIGHT).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(vec![Span::styled("  Status   ", style_muted()), Span::styled(status_label, status_style.add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::styled("  Source   ", style_muted()), Span::styled(skill.source.label(), ratatui::style::Style::default().fg(skill.source.color()))]),
            Line::from(vec![Span::styled("  Version  ", style_muted()), Span::styled(&skill.version, ratatui::style::Style::default().fg(Palette::TEXT))]),
            Line::from(vec![Span::styled("  Tags     ", style_muted()), Span::styled(skill.tags.join(", "), style_dim())]),
        ];
        f.render_widget(Paragraph::new(info), layout[0]);

        // Activation gauge
        let enabled_n = self.skills.iter().filter(|s| s.enabled).count();
        let total = self.skills.len();
        let ratio = if total == 0 { 0.0 } else { enabled_n as f64 / total as f64 };
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).border_style(style_border()).title(Span::styled(" Marketplace Activation ", style_muted())))
            .gauge_style(ratatui::style::Style::default().fg(Palette::CYAN).bg(Palette::BG_ELEVATED))
            .ratio(ratio)
            .label(format!("{}/{} enabled", enabled_n, total));
        f.render_widget(gauge, layout[1]);

        // README / description
        let content = skill.readme.clone().unwrap_or_else(|| skill.description.clone());
        let lines: Vec<Line> = content.lines().skip(self.detail_scroll).map(|l| {
            if l.starts_with("# ") {
                Line::from(Span::styled(l.to_owned(), ratatui::style::Style::default().fg(Palette::CYAN).add_modifier(Modifier::BOLD)))
            } else if l.starts_with("## ") {
                Line::from(Span::styled(l.to_owned(), ratatui::style::Style::default().fg(Palette::CYAN).add_modifier(Modifier::UNDERLINED)))
            } else if l.starts_with("- ") || l.starts_with("* ") {
                Line::from(vec![Span::styled("  • ", style_dim()), Span::raw(l[2..].to_owned())])
            } else {
                Line::from(Span::raw(l.to_owned()))
            }
        }).collect();
        let readme_block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border())
            .title(Span::styled(" README ", style_muted()));
        let readme_inner = readme_block.inner(layout[2]);
        f.render_widget(readme_block, layout[2]);
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), readme_inner);
    }

    fn draw_footer(&self, f: &mut Frame, area: Rect) {
        let hint = if self.filter_active {
            " Type to filter  Enter/Esc=confirm "
        } else {
            " ↑↓/jk=navigate  Space/Enter=toggle  Tab=tab  /=filter  r=refresh  PgUp/Dn=scroll "
        };
        f.render_widget(
            Paragraph::new(Span::styled(hint, style_muted())),
            area,
        );
    }

    fn draw_toast(&self, f: &mut Frame, area: Rect, msg: &str) {
        let width = (msg.len() as u16 + 6).min(area.width.saturating_sub(2));
        let toast_area = Rect::new(area.x + area.width.saturating_sub(width + 1), area.y + 1, width, 3);
        let toast = Paragraph::new(msg)
            .style(ratatui::style::Style::default().fg(Palette::BG).bg(Palette::CYAN))
            .block(Block::default().borders(Borders::ALL).border_style(style_border_focus()));
        f.render_widget(Clear, toast_area);
        f.render_widget(toast, toast_area);
    }
}

impl Default for SkillsPanel {
    fn default() -> Self { Self::new() }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max { s.to_owned() }
    else {
        let end = s.char_indices().nth(max.saturating_sub(1)).map(|(i, _)| i).unwrap_or(s.len());
        format!("{}…", &s[..end])
    }
}

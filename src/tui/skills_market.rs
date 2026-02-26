use crate::config::Config;
use crate::skills::marketplace::{
    list_claude_official_plugins, list_openclaw_vendored_skills, MarketSkill, MarketSource,
};
use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::path::{Path, PathBuf};

pub struct SkillsMarketApp {
    pub config: Config,
    pub repo_root: PathBuf,
    pub items: Vec<MarketSkill>,
    pub selected: usize,
    pub list_state: ratatui::widgets::ListState,
    pub status: String,
    pub quit: bool,
}

impl SkillsMarketApp {
    pub fn new(config: Config, repo_root: PathBuf) -> Result<Self> {
        let items = refresh_items(&repo_root, &config)?;
        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(0));
        Ok(Self {
            config,
            repo_root,
            items,
            selected: 0,
            list_state,
            status: "↑/↓ move  Enter toggle-enable  r refresh  q quit".into(),
            quit: false,
        })
    }

    pub fn refresh(&mut self) -> Result<()> {
        self.items = refresh_items(&self.repo_root, &self.config)?;
        if self.selected >= self.items.len() {
            self.selected = self.items.len().saturating_sub(1);
        }
        self.list_state.select(Some(self.selected));
        Ok(())
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.quit = true;
            }
            KeyCode::Down => {
                if !self.items.is_empty() {
                    self.selected = (self.selected + 1).min(self.items.len() - 1);
                    self.list_state.select(Some(self.selected));
                }
            }
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                self.list_state.select(Some(self.selected));
            }
            KeyCode::Enter => {
                self.toggle_selected()?;
            }
            KeyCode::Char('r') => {
                self.status = "Refreshing markets...".into();
                self.refresh()?;
                self.status = "Refreshed.".into();
            }
            _ => {}
        }
        Ok(())
    }

    fn toggle_selected(&mut self) -> Result<()> {
        if self.items.is_empty() {
            return Ok(());
        }

        let name = self.items[self.selected].name.clone();
        let current = self
            .config
            .skills
            .enabled
            .get(&name)
            .copied()
            .unwrap_or(false);
        let next = !current;
        self.config.skills.enabled.insert(name.clone(), next);

        // If enabling, perform installation depending on source.
        if next {
            if let Some(item) = self.items.get(self.selected) {
                match &item.source {
                    MarketSource::OpenClawVendored { skill_dir, .. } => {
                        ensure_skill_copied_into_workspace(
                            skill_dir,
                            &self.config.workspace_dir,
                            &name,
                        )?;
                        self.status = format!("Installed (vendored) and enabled: {}", name);
                    }
                    MarketSource::ClaudeOfficialPlugin { .. } => {
                        match crate::skills::marketplace::install_claude_plugin(
                            &self.config.workspace_dir,
                            &name,
                        ) {
                            Ok(installed) => {
                                for s in installed {
                                    self.config.skills.enabled.insert(s.clone(), true);
                                }
                                self.status = format!("Installed (Claude) and enabled: {}", name);
                            }
                            Err(e) => {
                                // Rollback enable on failure
                                self.config.skills.enabled.insert(name.clone(), false);
                                self.status = format!("Claude install failed for {}: {}", name, e);
                            }
                        }
                    }
                }
            }
        }

        save_config(&self.config)?;
        self.refresh()?;
        self.status = format!("{} {}", if next { "Enabled" } else { "Disabled" }, name);
        Ok(())
    }

    pub fn draw(&mut self, f: &mut Frame<'_>) {
        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(5),
            ])
            .split(f.area());

        let title = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Skills Market", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("  (Claude official + OpenClaw vendored)"),
            ]),
            Line::from(Span::styled(
                format!("Workspace: {}", self.config.workspace_dir.display()),
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, areas[0]);

        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|s| {
                let enabled = if s.enabled { "✓" } else { " " };
                let src = match s.source {
                    MarketSource::OpenClawVendored { .. } => "openclaw",
                    MarketSource::ClaudeOfficialPlugin { .. } => "claude",
                };
                ListItem::new(Line::from(vec![
                    Span::raw(format!("[{enabled}] ")),
                    Span::styled(
                        format!("{}", s.name),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("  ({src}) "), Style::default().fg(Color::DarkGray)),
                    Span::raw(truncate_one_line(&s.description, 60)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Skills"))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("▶ ");
        f.render_stateful_widget(list, areas[1], &mut self.list_state);

        let detail = if let Some(item) = self.items.get(self.selected) {
            let src = match &item.source {
                MarketSource::OpenClawVendored { skill_md, .. } => {
                    format!("OpenClaw (vendored) — {}", skill_md.display())
                }
                MarketSource::ClaudeOfficialPlugin { plugin_path, .. } => {
                    format!("Claude official plugin — {}", plugin_path)
                }
            };
            format!(
                "Name: {}\nEnabled: {}\nSource: {}\n\n{}",
                item.name,
                item.enabled,
                src,
                item.description
            )
        } else {
            "No skill selected".into()
        };

        let bottom = Paragraph::new(detail)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title(self.status.clone()));
        f.render_widget(bottom, areas[2]);
    }
}

fn truncate_one_line(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut out = s[..max].to_string();
    out.push_str("…");
    out
}

fn refresh_items(repo_root: &Path, config: &Config) -> Result<Vec<MarketSkill>> {
    let mut all = Vec::new();
    // OpenClaw vendored skills from repo root.
    all.extend(list_openclaw_vendored_skills(repo_root, config)?);
    // Claude official marketplace plugins (as selectable entries; download/install is phase-2).
    all.extend(list_claude_official_plugins(&config.workspace_dir, config)?);

    // Dedup by name, preferring OpenClaw vendored entries.
    all.sort_by(|a, b| a.name.cmp(&b.name));
    all.dedup_by(|a, b| a.name == b.name);

    Ok(all)
}

fn ensure_skill_copied_into_workspace(src_dir: &Path, workspace_dir: &Path, name: &str) -> Result<()> {
    let dest_dir = workspace_dir.join("skills").join(name);
    if dest_dir.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dest_dir.parent().unwrap_or(workspace_dir))?;

    // Copy only SKILL.md for now.
    let src_md = src_dir.join("SKILL.md");
    let dest_md = dest_dir.join("SKILL.md");
    std::fs::create_dir_all(&dest_dir)?;
    std::fs::copy(&src_md, &dest_md).with_context(|| {
        format!(
            "Failed to copy {} to {}",
            src_md.display(),
            dest_md.display()
        )
    })?;

    Ok(())
}

fn save_config(config: &Config) -> Result<()> {
    config.save()
}

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use crate::tui::enhanced_app::theme::{
    Palette, style_border, style_border_focus, style_dim, style_error, style_muted,
    style_success, style_tag_tool, style_title, style_warning,
};
use crate::tui::enhanced_app::state::{ToolEntry, ToolStatus};

// ── Tools panel state ─────────────────────────────────────────────────────────

pub struct ToolsPanel {
    pub selected:      usize,
    pub detail_scroll: usize,
    pub filter:        String,
    pub filter_active: bool,
}

impl ToolsPanel {
    pub fn new() -> Self {
        Self {
            selected:      0,
            detail_scroll: 0,
            filter:        String::new(),
            filter_active: false,
        }
    }

    pub fn select_prev(&mut self, tools: &[ToolEntry]) {
        if !tools.is_empty() && self.selected > 0 {
            self.selected -= 1;
            self.detail_scroll = 0;
        }
    }

    pub fn select_next(&mut self, tools: &[ToolEntry]) {
        if !tools.is_empty() && self.selected + 1 < tools.len() {
            self.selected += 1;
            self.detail_scroll = 0;
        }
    }

    pub fn detail_scroll_up(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_sub(1);
    }

    pub fn detail_scroll_down(&mut self) {
        self.detail_scroll += 1;
    }

    pub fn start_filter(&mut self) {
        self.filter_active = true;
        self.filter.clear();
    }

    pub fn filter_push(&mut self, c: char) {
        self.filter.push(c);
        self.selected = 0;
    }

    pub fn filter_pop(&mut self) {
        self.filter.pop();
        self.selected = 0;
    }

    pub fn filter_commit(&mut self) {
        self.filter_active = false;
    }

    pub fn is_filter_active(&self) -> bool {
        self.filter_active
    }

    fn filtered_indices<'a>(&self, tools: &'a [ToolEntry]) -> Vec<usize> {
        let q = self.filter.to_lowercase();
        tools.iter().enumerate()
            .filter(|(_, t)| {
                q.is_empty()
                    || t.name.to_lowercase().contains(&q)
                    || t.input_summary.to_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect()
    }

    // ── Draw ──────────────────────────────────────────────────────────────────

    pub fn draw(&self, f: &mut Frame, area: Rect, tools: &[ToolEntry]) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // header bar
                Constraint::Min(5),    // content
                Constraint::Length(1), // footer hint
            ])
            .split(area);

        self.draw_header(f, layout[0], tools);
        self.draw_content(f, layout[1], tools);
        self.draw_footer(f, layout[2]);
    }

    fn draw_header(&self, f: &mut Frame, area: Rect, tools: &[ToolEntry]) {
        let running = tools.iter().filter(|t| t.status == ToolStatus::Running).count();
        let success = tools.iter().filter(|t| t.status == ToolStatus::Success).count();
        let failed  = tools.iter().filter(|t| t.status == ToolStatus::Failed).count();

        let mut spans = vec![
            Span::styled(" ⚙ ", style_tag_tool()),
            Span::styled("Tools  ", style_title()),
            Span::styled(format!("total:{} ", tools.len()), style_dim()),
        ];
        if running > 0 {
            spans.push(Span::styled(format!("running:{} ", running), style_warning()));
        }
        if success > 0 {
            spans.push(Span::styled(format!("ok:{} ", success), style_success()));
        }
        if failed > 0 {
            spans.push(Span::styled(format!("err:{} ", failed), style_error()));
        }
        if !self.filter.is_empty() || self.filter_active {
            spans.push(Span::styled(
                format!("  filter: {}{}  ", self.filter, if self.filter_active { "|" } else { "" }),
                ratatui::style::Style::default().fg(Palette::WARNING),
            ));
        }

        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn draw_content(&self, f: &mut Frame, area: Rect, tools: &[ToolEntry]) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
            .split(area);

        self.draw_list(f, cols[0], tools);
        self.draw_detail(f, cols[1], tools);
    }

    fn draw_list(&self, f: &mut Frame, area: Rect, tools: &[ToolEntry]) {
        let filtered = self.filtered_indices(tools);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border_focus())
            .title(Span::styled(
                format!(" Tool Log ({}) ", filtered.len()),
                style_tag_tool(),
            ));
        let inner = block.inner(area);
        f.render_widget(block, area);

        if filtered.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled(
                    if tools.is_empty() {
                        "  No tools invoked yet"
                    } else {
                        "  No tools match filter"
                    },
                    style_muted(),
                )),
                inner,
            );
            return;
        }

        let items: Vec<ListItem> = filtered.iter().enumerate().map(|(di, &ri)| {
            let t = &tools[ri];
            let sel = di == self.selected;
            let bg = if sel { Palette::BG_SELECTED } else { Palette::BG_PANEL };

            let (icon, icon_style) = match t.status {
                ToolStatus::Running   => ("⟳", ratatui::style::Style::default().fg(Palette::WARNING).bg(bg)),
                ToolStatus::Success   => ("✓", ratatui::style::Style::default().fg(Palette::SUCCESS).bg(bg)),
                ToolStatus::Failed    => ("✗", ratatui::style::Style::default().fg(Palette::ERROR).bg(bg)),
                ToolStatus::Cancelled => ("⊘", ratatui::style::Style::default().fg(Palette::TEXT_DIM).bg(bg)),
            };

            let dur_str = t.duration_ms
                .map(|ms| format!(" {}ms", ms))
                .unwrap_or_default();

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!(" {} ", icon), icon_style),
                    Span::styled(
                        format!("{:<22}", truncate(&t.name, 22)),
                        ratatui::style::Style::default()
                            .fg(if sel { Palette::TEXT_BRIGHT } else { Palette::TEXT })
                            .bg(bg)
                            .add_modifier(if sel { Modifier::BOLD } else { Modifier::empty() }),
                    ),
                    Span::styled(
                        format!("{:>6}", dur_str),
                        ratatui::style::Style::default().fg(Palette::TEXT_DIM).bg(bg),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(
                        truncate(&t.input_summary, (inner.width as usize).saturating_sub(5)),
                        ratatui::style::Style::default().fg(Palette::TEXT_MUTED).bg(bg),
                    ),
                    Span::styled(
                        format!(" {}", t.timestamp),
                        ratatui::style::Style::default().fg(Palette::TEXT_MUTED).bg(bg).add_modifier(Modifier::DIM),
                    ),
                ]),
            ])
        }).collect();

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(self.selected));
        f.render_stateful_widget(
            List::new(items).highlight_style(
                ratatui::style::Style::default().bg(Palette::BG_SELECTED)
            ),
            inner,
            &mut list_state,
        );
    }

    fn draw_detail(&self, f: &mut Frame, area: Rect, tools: &[ToolEntry]) {
        let filtered = self.filtered_indices(tools);
        let real_idx = filtered.get(self.selected).copied();

        let title = real_idx
            .and_then(|i| tools.get(i))
            .map(|t| format!(" {} ", t.name))
            .unwrap_or_else(|| " Detail ".to_string());

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border())
            .title(Span::styled(title, style_tag_tool()));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let tool = match real_idx.and_then(|i| tools.get(i)) {
            Some(t) => t,
            None => {
                f.render_widget(
                    Paragraph::new(Span::styled("  Select a tool invocation to inspect", style_muted())),
                    inner,
                );
                return;
            }
        };

        let (status_label, status_style) = match tool.status {
            ToolStatus::Running   => ("⟳ RUNNING",   style_warning()),
            ToolStatus::Success   => ("✓ SUCCESS",   style_success()),
            ToolStatus::Failed    => ("✗ FAILED",    style_error()),
            ToolStatus::Cancelled => ("⊘ CANCELLED", style_muted()),
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled("  Tool       ", style_muted()),
                Span::styled(&tool.name, ratatui::style::Style::default().fg(Palette::TOOL).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  Status     ", style_muted()),
                Span::styled(status_label, status_style.add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  Time       ", style_muted()),
                Span::styled(&tool.timestamp, style_dim()),
            ]),
        ];

        if let Some(ms) = tool.duration_ms {
            lines.push(Line::from(vec![
                Span::styled("  Duration   ", style_muted()),
                Span::styled(format!("{}ms", ms), style_dim()),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("  ─── Input ", style_muted())));
        lines.push(Line::from(""));
        for l in tool.input_summary.lines().skip(self.detail_scroll) {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(l.to_owned(), ratatui::style::Style::default().fg(Palette::TEXT)),
            ]));
        }

        if let Some(ref out) = tool.output_summary {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("  ─── Output ", style_muted())));
            lines.push(Line::from(""));
            for l in out.lines() {
                let style = if tool.status == ToolStatus::Failed {
                    style_error()
                } else {
                    ratatui::style::Style::default().fg(Palette::TEXT)
                };
                lines.push(Line::from(vec![Span::raw("    "), Span::styled(l.to_owned(), style)]));
            }
        }

        f.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }),
            inner,
        );
    }

    fn draw_footer(&self, f: &mut Frame, area: Rect) {
        let hint = if self.filter_active {
            " Type to filter  Enter/Esc=confirm "
        } else {
            " ↑↓=navigate  /=filter  PgUp/Dn=scroll detail  c=clear log "
        };
        f.render_widget(
            Paragraph::new(Span::styled(hint, style_muted())),
            area,
        );
    }
}

impl Default for ToolsPanel {
    fn default() -> Self { Self::new() }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        let end = s.char_indices().nth(max.saturating_sub(1)).map(|(i, _)| i).unwrap_or(s.len());
        format!("{}…", &s[..end])
    }
}

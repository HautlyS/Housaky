use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::doctor::{CheckCategory, CheckResult, DoctorReport, Severity};
use crate::tui::enhanced_app::theme::{
    style_border, style_border_focus, style_dim, style_error, style_muted, style_success,
    style_title, style_warning, truncate, Palette,
};

// ── DoctorPanel ──────────────────────────────────────────────────────────────

pub struct DoctorPanel {
    pub report: Option<DoctorReport>,
    pub selected: usize,
    pub list_state: ListState,
    pub detail_scroll: usize,
    pub filter: CategoryFilter,
    pub last_refresh: Option<std::time::Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryFilter {
    All,
    Problems,
    Daemon,
    Channel,
    Security,
    Config,
    Keys,
}

impl CategoryFilter {
    pub const ALL_FILTERS: &'static [CategoryFilter] = &[
        CategoryFilter::All,
        CategoryFilter::Problems,
        CategoryFilter::Daemon,
        CategoryFilter::Channel,
        CategoryFilter::Security,
        CategoryFilter::Config,
        CategoryFilter::Keys,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CategoryFilter::All => "All",
            CategoryFilter::Problems => "Problems",
            CategoryFilter::Daemon => "Daemon",
            CategoryFilter::Channel => "Channel",
            CategoryFilter::Security => "Security",
            CategoryFilter::Config => "Config",
            CategoryFilter::Keys => "Keys",
        }
    }

    pub fn next(self) -> CategoryFilter {
        let idx = Self::ALL_FILTERS
            .iter()
            .position(|&f| f == self)
            .unwrap_or(0);
        Self::ALL_FILTERS[(idx + 1) % Self::ALL_FILTERS.len()]
    }

    pub fn prev(self) -> CategoryFilter {
        let idx = Self::ALL_FILTERS
            .iter()
            .position(|&f| f == self)
            .unwrap_or(0);
        let len = Self::ALL_FILTERS.len();
        Self::ALL_FILTERS[(idx + len - 1) % len]
    }
}

impl DoctorPanel {
    pub fn new() -> Self {
        Self {
            report: None,
            selected: 0,
            list_state: ListState::default(),
            detail_scroll: 0,
            filter: CategoryFilter::All,
            last_refresh: None,
        }
    }

    pub fn load(&mut self, report: DoctorReport) {
        self.report = Some(report);
        self.selected = 0;
        self.detail_scroll = 0;
        self.list_state.select(Some(0));
        self.last_refresh = Some(std::time::Instant::now());
    }

    pub fn visible_checks(&self) -> Vec<&CheckResult> {
        let Some(ref report) = self.report else {
            return vec![];
        };
        report
            .checks
            .iter()
            .filter(|c| match self.filter {
                CategoryFilter::All => true,
                CategoryFilter::Problems => c.severity.is_problem(),
                CategoryFilter::Daemon => {
                    matches!(c.category, CheckCategory::Daemon | CheckCategory::Scheduler)
                }
                CategoryFilter::Channel => c.category == CheckCategory::Channel,
                CategoryFilter::Security => {
                    matches!(c.category, CheckCategory::Security | CheckCategory::Keys)
                }
                CategoryFilter::Config => c.category == CheckCategory::Config,
                CategoryFilter::Keys => c.category == CheckCategory::Keys,
            })
            .collect()
    }

    pub fn set_selected(&mut self, idx: usize) {
        let len = self.visible_checks().len();
        if len == 0 {
            return;
        }
        self.selected = idx.min(len - 1);
        self.list_state.select(Some(self.selected));
        self.detail_scroll = 0;
    }

    pub fn select_prev(&mut self) {
        let len = self.visible_checks().len();
        if len == 0 {
            return;
        }
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = len - 1;
        }
        self.list_state.select(Some(self.selected));
        self.detail_scroll = 0;
    }

    pub fn select_next(&mut self) {
        let len = self.visible_checks().len();
        if len == 0 {
            return;
        }
        self.selected = (self.selected + 1) % len;
        self.list_state.select(Some(self.selected));
        self.detail_scroll = 0;
    }

    pub fn detail_scroll_up(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_sub(1);
    }

    pub fn detail_scroll_down(&mut self) {
        self.detail_scroll += 1;
    }

    pub fn cycle_filter_next(&mut self) {
        self.filter = self.filter.next();
        self.selected = 0;
        self.list_state.select(Some(0));
        self.detail_scroll = 0;
    }

    pub fn cycle_filter_prev(&mut self) {
        self.filter = self.filter.prev();
        self.selected = 0;
        self.list_state.select(Some(0));
        self.detail_scroll = 0;
    }

    // ── Draw ──────────────────────────────────────────────────────────────────

    pub fn draw(&mut self, f: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // header / summary bar
                Constraint::Min(0),    // main split
            ])
            .split(area);

        self.draw_header(f, layout[0]);

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(layout[1]);

        self.draw_list(f, body[0]);
        self.draw_detail(f, body[1]);
    }

    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let (ok, warn, err, auto_fix, total) = if let Some(ref r) = self.report {
            (
                r.total_ok,
                r.total_warnings,
                r.total_errors,
                r.auto_fixable,
                r.checks.len(),
            )
        } else {
            (0, 0, 0, 0, 0)
        };

        let age = self
            .last_refresh
            .map(|t| format!("{}s ago", t.elapsed().as_secs()))
            .unwrap_or_else(|| "never".to_string());

        let health_icon = if err > 0 {
            "❌"
        } else if warn > 0 {
            "⚠️ "
        } else {
            "✅"
        };

        let mut spans = vec![
            Span::styled(format!(" {} Doctor  ", health_icon), style_title()),
            Span::styled(format!("  ✅ {ok}  "), style_success()),
            Span::styled(format!("⚠  {warn}  "), style_warning()),
            Span::styled(format!("❌ {err}  "), style_error()),
            Span::styled(format!("total {total}  "), style_dim()),
        ];

        if auto_fix > 0 {
            spans.push(Span::styled(
                format!("🔧 {auto_fix} auto-fixable  "),
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        // filter tabs
        spans.push(Span::styled("  │ filter: ", style_muted()));
        for &flt in CategoryFilter::ALL_FILTERS {
            if flt == self.filter {
                spans.push(Span::styled(
                    format!("[{}] ", flt.label()),
                    ratatui::style::Style::default()
                        .fg(Palette::BG)
                        .bg(Palette::CYAN)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::styled(format!("{} ", flt.label()), style_dim()));
            }
        }

        spans.push(Span::styled(format!("  refreshed {age}"), style_muted()));

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border())
            .title(Span::styled(" 🩺 System Health ", style_title()));

        let inner = block.inner(area);
        f.render_widget(block, area);
        f.render_widget(Paragraph::new(Line::from(spans)), inner);
    }

    fn draw_list(&mut self, f: &mut Frame, area: Rect) {
        let checks = self.visible_checks();
        let focused = true;

        let items: Vec<ListItem> = checks
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let sev_style = severity_style(c.severity);
                let cat_label = format!("{:<10}", c.category.label());
                let name_short = truncate(&c.name, 22);
                let line = Line::from(vec![
                    Span::styled(format!(" {} ", c.severity.icon()), sev_style),
                    Span::styled(
                        cat_label,
                        ratatui::style::Style::default().fg(category_color(c.category)),
                    ),
                    Span::styled(
                        name_short,
                        if i == self.selected {
                            ratatui::style::Style::default()
                                .fg(Palette::TEXT_BRIGHT)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            ratatui::style::Style::default().fg(Palette::TEXT_DIM)
                        },
                    ),
                ]);
                ListItem::new(line)
            })
            .collect();

        let title = format!(" Checks ({}) ", checks.len());
        let border_style = if focused {
            style_border_focus()
        } else {
            style_border()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title(Span::styled(title, style_title())),
            )
            .highlight_style(
                ratatui::style::Style::default()
                    .bg(Palette::BG_SELECTED)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn draw_detail(&self, f: &mut Frame, area: Rect) {
        let checks = self.visible_checks();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border())
            .title(Span::styled(" Detail ", style_title()));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let Some(check) = checks.get(self.selected) else {
            f.render_widget(
                Paragraph::new(Span::styled(
                    "  Select a check to view details",
                    style_muted(),
                )),
                inner,
            );
            return;
        };

        let sev_style = severity_style(check.severity);
        let mut lines: Vec<Line> = vec![
            Line::from(vec![
                Span::styled(format!(" {} ", check.severity.icon()), sev_style),
                Span::styled(
                    check.name.clone(),
                    ratatui::style::Style::default()
                        .fg(Palette::TEXT_BRIGHT)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Category  ", style_muted()),
                Span::styled(
                    check.category.label(),
                    ratatui::style::Style::default().fg(category_color(check.category)),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled("  Message   ", style_muted())]),
            Line::from(vec![
                Span::styled("  ", style_muted()),
                Span::styled(
                    check.message.clone(),
                    ratatui::style::Style::default().fg(Palette::TEXT),
                ),
            ]),
        ];

        if let Some(ref hint) = check.fix_hint {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "  💡 Hint   ",
                ratatui::style::Style::default()
                    .fg(Palette::INFO)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(vec![
                Span::styled("  ", style_muted()),
                Span::styled(
                    hint.clone(),
                    ratatui::style::Style::default().fg(Palette::TEXT_DIM),
                ),
            ]));
        }

        if let Some(ref cmd) = check.fix_command {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "  🔧 Fix    ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(vec![
                Span::styled("  ", style_muted()),
                Span::styled(
                    cmd.clone(),
                    ratatui::style::Style::default()
                        .fg(Palette::CODE_FG)
                        .bg(Palette::CODE_BG),
                ),
            ]));
        }

        if check.auto_fixable {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "  ⚡ Auto-fixable — run `housaky doctor fix`",
                ratatui::style::Style::default()
                    .fg(Palette::SKILL)
                    .add_modifier(Modifier::BOLD),
            )]));
        }

        // key hints at bottom
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  ↑↓ navigate  ", style_muted()),
            Span::styled("Tab/S-Tab filter  ", style_muted()),
            Span::styled("r refresh  ", style_muted()),
            Span::styled("PageUp/Dn scroll", style_muted()),
        ]));

        let total = lines.len();
        let visible = inner.height as usize;
        let scroll = self.detail_scroll.min(total.saturating_sub(visible));

        let visible_lines: Vec<Line> = lines.into_iter().skip(scroll).collect();
        f.render_widget(
            Paragraph::new(visible_lines).wrap(Wrap { trim: false }),
            inner,
        );
    }
}

impl Default for DoctorPanel {
    fn default() -> Self {
        Self::new()
    }
}

// ── Style helpers ─────────────────────────────────────────────────────────────

fn severity_style(sev: Severity) -> ratatui::style::Style {
    match sev {
        Severity::Ok => ratatui::style::Style::default().fg(Palette::SUCCESS),
        Severity::Warning => ratatui::style::Style::default().fg(Palette::WARNING),
        Severity::Error => ratatui::style::Style::default()
            .fg(Palette::ERROR)
            .add_modifier(Modifier::BOLD),
        Severity::Info => ratatui::style::Style::default().fg(Palette::INFO),
    }
}

fn category_color(cat: CheckCategory) -> ratatui::style::Color {
    match cat {
        CheckCategory::Daemon => Palette::CYAN,
        CheckCategory::Scheduler => Palette::CYAN_DIM,
        CheckCategory::Channel => Palette::SKILL,
        CheckCategory::Config => Palette::VIOLET,
        CheckCategory::Security => Palette::ERROR,
        CheckCategory::FileSystem => Palette::WARNING,
        CheckCategory::Keys => Palette::GOAL,
    }
}


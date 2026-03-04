use crate::tui::enhanced_app::state::{ToolEntry, ToolStatus};
use crate::tui::enhanced_app::theme::{
    style_border, style_border_focus, style_dim, style_error, style_muted, style_success,
    style_tag_tool, style_title, style_warning, truncate, Palette,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::collections::HashMap;

// Known available tools in Housaky
const AVAILABLE_TOOLS: &[&str] = &[
    "shell",
    "file_read",
    "file_write",
    "file_list",
    "file_search",
    "file_move",
    "file_delete",
    "file_info",
    "memory_store",
    "memory_recall",
    "memory_forget",
    "schedule",
    "git_operations",
    "browser_open",
    "browser",
    "http_request",
    "clawd_cursor",
    "delegate",
    "web_search",
    "web_fetch",
    "code_search",
    "skill_http",
    "skill_script",
    "skill_tool",
    "arduino_upload",
];

// ── Tool usage tracking for TUI ─────────────────────────────────────────────────

#[derive(Clone)]
pub struct TrackedTool {
    pub name: String,
    pub description: String,
    pub execution_count: u32,
    pub last_used: Option<String>,
    pub status: ToolStatus,
}

// ── Tools panel state ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToolsViewMode {
    Available,
    History,
}

pub struct ToolsPanel {
    pub selected: usize,
    pub detail_scroll: usize,
    pub filter: String,
    pub filter_active: bool,
    pub view_mode: ToolsViewMode,
    pub tool_usage: HashMap<String, TrackedTool>,
}

impl ToolsPanel {
    pub fn new() -> Self {
        Self {
            selected: 0,
            detail_scroll: 0,
            filter: String::new(),
            filter_active: false,
            view_mode: ToolsViewMode::History,
            tool_usage: HashMap::new(),
        }
    }

    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ToolsViewMode::Available => ToolsViewMode::History,
            ToolsViewMode::History => ToolsViewMode::Available,
        };
        self.selected = 0;
        self.detail_scroll = 0;
    }

    pub fn available_tools_count(&self) -> usize {
        if self.filter.is_empty() {
            AVAILABLE_TOOLS.len()
        } else {
            let q = self.filter.to_lowercase();
            AVAILABLE_TOOLS
                .iter()
                .filter(|t| t.to_lowercase().contains(&q))
                .count()
        }
    }

    pub fn select_prev(&mut self, tools: &[ToolEntry]) {
        let max = match self.view_mode {
            ToolsViewMode::Available => self.available_tools_count(),
            ToolsViewMode::History => self.filtered_indices(tools).len(),
        };
        if max > 0 && self.selected > 0 {
            self.selected -= 1;
            self.detail_scroll = 0;
        }
    }

    pub fn select_next(&mut self, tools: &[ToolEntry]) {
        let max = match self.view_mode {
            ToolsViewMode::Available => self.available_tools_count(),
            ToolsViewMode::History => self.filtered_indices(tools).len(),
        };
        if max > 0 && self.selected + 1 < max {
            self.selected += 1;
            self.detail_scroll = 0;
        }
    }

    pub fn filtered_count(&self, tools: &[ToolEntry]) -> usize {
        match self.view_mode {
            ToolsViewMode::Available => self.available_tools_count(),
            ToolsViewMode::History => self.filtered_indices(tools).len(),
        }
    }

    pub fn set_selected(&mut self, display_idx: usize, tools: &[ToolEntry]) {
        let max = match self.view_mode {
            ToolsViewMode::Available => self.available_tools_count(),
            ToolsViewMode::History => self.filtered_indices(tools).len(),
        };
        if max == 0 {
            self.selected = 0;
        } else {
            self.selected = display_idx.min(max - 1);
        }
        self.detail_scroll = 0;
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
        tools
            .iter()
            .enumerate()
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
        let running = tools
            .iter()
            .filter(|t| t.status == ToolStatus::Running)
            .count();
        let success = tools
            .iter()
            .filter(|t| t.status == ToolStatus::Success)
            .count();
        let failed = tools
            .iter()
            .filter(|t| t.status == ToolStatus::Failed)
            .count();

        let view_indicator = match self.view_mode {
            ToolsViewMode::Available => "[Available] ",
            ToolsViewMode::History => "[History] ",
        };

        let mut spans = vec![
            Span::styled(" ⚙ ", style_tag_tool()),
            Span::styled("Tools  ", style_title()),
            Span::styled(
                format!(
                    "{}{} ",
                    view_indicator,
                    match self.view_mode {
                        ToolsViewMode::Available => format!("total:{}", AVAILABLE_TOOLS.len()),
                        ToolsViewMode::History => format!("total:{}", tools.len()),
                    }
                ),
                style_dim(),
            ),
        ];
        if self.view_mode == ToolsViewMode::History {
            if running > 0 {
                spans.push(Span::styled(
                    format!("running:{} ", running),
                    style_warning(),
                ));
            }
            if success > 0 {
                spans.push(Span::styled(format!("ok:{} ", success), style_success()));
            }
            if failed > 0 {
                spans.push(Span::styled(format!("err:{} ", failed), style_error()));
            }
        }
        if !self.filter.is_empty() || self.filter_active {
            spans.push(Span::styled(
                format!(
                    "  filter: {}{}  ",
                    self.filter,
                    if self.filter_active { "|" } else { "" }
                ),
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
        let inner_area = Rect::new(
            area.x + 1,
            area.y + 1,
            area.width.saturating_sub(2),
            area.height.saturating_sub(2),
        );

        match self.view_mode {
            ToolsViewMode::Available => self.draw_available_tools(f, area, inner_area),
            ToolsViewMode::History => self.draw_history_list(f, area, inner_area, tools),
        }
    }

    fn draw_available_tools(&self, f: &mut Frame, area: Rect, inner: Rect) {
        let filtered: Vec<&str> = if self.filter.is_empty() {
            AVAILABLE_TOOLS.iter().copied().collect()
        } else {
            let q = self.filter.to_lowercase();
            AVAILABLE_TOOLS
                .iter()
                .filter(|t| t.to_lowercase().contains(&q))
                .copied()
                .collect()
        };

        let count = filtered.len();
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border_focus())
            .title(Span::styled(
                format!(" Available Tools ({}) ", count),
                style_tag_tool(),
            ));
        f.render_widget(block, area);

        if filtered.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled("  No tools match filter", style_muted())),
                inner,
            );
            return;
        }

        let items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(di, &tool_name)| {
                let sel = di == self.selected;
                let bg = if sel {
                    Palette::BG_SELECTED
                } else {
                    Palette::BG_PANEL
                };

                ListItem::new(vec![Line::from(vec![
                    Span::styled(
                        " ◆",
                        ratatui::style::Style::default().fg(Palette::CYAN).bg(bg),
                    ),
                    Span::styled(
                        format!(" {:width$}", tool_name, width = 20),
                        ratatui::style::Style::default()
                            .fg(if sel {
                                Palette::TEXT_BRIGHT
                            } else {
                                Palette::TEXT
                            })
                            .bg(bg)
                            .add_modifier(if sel {
                                Modifier::BOLD
                            } else {
                                Modifier::empty()
                            }),
                    ),
                ])])
            })
            .collect();

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(self.selected));
        f.render_stateful_widget(
            List::new(items)
                .highlight_style(ratatui::style::Style::default().bg(Palette::BG_SELECTED)),
            inner,
            &mut list_state,
        );
    }

    fn draw_history_list(&self, f: &mut Frame, area: Rect, inner: Rect, tools: &[ToolEntry]) {
        let filtered = self.filtered_indices(tools);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border_focus())
            .title(Span::styled(
                format!(" Tool Log ({}) ", filtered.len()),
                style_tag_tool(),
            ));
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

        let items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(di, &ri)| {
                let t = &tools[ri];
                let sel = di == self.selected;
                let bg = if sel {
                    Palette::BG_SELECTED
                } else {
                    Palette::BG_PANEL
                };

                let (icon, icon_style) = match t.status {
                    ToolStatus::Running => (
                        "⟳",
                        ratatui::style::Style::default().fg(Palette::WARNING).bg(bg),
                    ),
                    ToolStatus::Success => (
                        "✓",
                        ratatui::style::Style::default().fg(Palette::SUCCESS).bg(bg),
                    ),
                    ToolStatus::Failed => (
                        "✗",
                        ratatui::style::Style::default().fg(Palette::ERROR).bg(bg),
                    ),
                    ToolStatus::Cancelled => (
                        "⊘",
                        ratatui::style::Style::default()
                            .fg(Palette::TEXT_DIM)
                            .bg(bg),
                    ),
                };

                let dur_str = t
                    .duration_ms
                    .map(|ms| format!(" {}ms", ms))
                    .unwrap_or_default();

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(format!(" {} ", icon), icon_style),
                        Span::styled(
                            format!("{:<22}", truncate(&t.name, 22)),
                            ratatui::style::Style::default()
                                .fg(if sel {
                                    Palette::TEXT_BRIGHT
                                } else {
                                    Palette::TEXT
                                })
                                .bg(bg)
                                .add_modifier(if sel {
                                    Modifier::BOLD
                                } else {
                                    Modifier::empty()
                                }),
                        ),
                        Span::styled(
                            format!("{:>6}", dur_str),
                            ratatui::style::Style::default()
                                .fg(Palette::TEXT_DIM)
                                .bg(bg),
                        ),
                    ]),
                    Line::from(vec![
                        Span::raw("   "),
                        Span::styled(
                            truncate(&t.input_summary, (inner.width as usize).saturating_sub(5)),
                            ratatui::style::Style::default()
                                .fg(Palette::TEXT_MUTED)
                                .bg(bg),
                        ),
                        Span::styled(
                            format!(" {}", t.timestamp),
                            ratatui::style::Style::default()
                                .fg(Palette::TEXT_MUTED)
                                .bg(bg)
                                .add_modifier(Modifier::DIM),
                        ),
                    ]),
                ])
            })
            .collect();

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(self.selected));
        f.render_stateful_widget(
            List::new(items)
                .highlight_style(ratatui::style::Style::default().bg(Palette::BG_SELECTED)),
            inner,
            &mut list_state,
        );
    }

    fn draw_detail(&self, f: &mut Frame, area: Rect, tools: &[ToolEntry]) {
        match self.view_mode {
            ToolsViewMode::Available => self.draw_available_detail(f, area),
            ToolsViewMode::History => self.draw_history_detail(f, area, tools),
        }
    }

    fn draw_available_detail(&self, f: &mut Frame, area: Rect) {
        let filtered: Vec<&str> = if self.filter.is_empty() {
            AVAILABLE_TOOLS.iter().copied().collect()
        } else {
            let q = self.filter.to_lowercase();
            AVAILABLE_TOOLS
                .iter()
                .filter(|t| t.to_lowercase().contains(&q))
                .copied()
                .collect()
        };

        let tool_name = filtered.get(self.selected).copied();

        let title = tool_name
            .map(|t| format!(" {} ", t))
            .unwrap_or_else(|| " Detail ".to_string());

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border())
            .title(Span::styled(title, style_tag_tool()));
        let inner = block.inner(area);
        f.render_widget(block, area);

        match tool_name {
            Some(name) => {
                let desc = Self::get_tool_description(name);
                let lines = vec![
                    Line::from(vec![
                        Span::styled("  Tool       ", style_muted()),
                        Span::styled(
                            name,
                            ratatui::style::Style::default()
                                .fg(Palette::TOOL)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(vec![Span::styled("  Description", style_muted())]),
                    Line::from(""),
                    Line::from(vec![
                        Span::raw("    "),
                        Span::styled(desc, ratatui::style::Style::default().fg(Palette::TEXT)),
                    ]),
                ];
                f.render_widget(Paragraph::new(lines), inner);
            }
            None => {
                f.render_widget(
                    Paragraph::new(Span::styled(
                        "  Select a tool to view details",
                        style_muted(),
                    )),
                    inner,
                );
            }
        }
    }

    fn get_tool_description(name: &str) -> &str {
        match name {
            "shell" => "Execute shell commands in the terminal",
            "file_read" => "Read contents of files from the filesystem",
            "file_write" => "Write content to files",
            "file_list" => "List files in a directory",
            "file_search" => "Search for files by pattern",
            "file_move" => "Move or rename files",
            "file_delete" => "Delete files or directories",
            "file_info" => "Get file/directory information",
            "memory_store" => "Store information in persistent memory",
            "memory_recall" => "Recall information from memory",
            "memory_forget" => "Remove information from memory",
            "schedule" => "Schedule tasks to run later",
            "git_operations" => "Perform git operations",
            "browser_open" => "Open URLs in browser",
            "browser" => "Full browser automation",
            "http_request" => "Make HTTP requests",
            "clawd_cursor" => "Cursor.ai integration tool",
            "delegate" => "Delegate tasks to sub-agents",
            "web_search" => "Search the web",
            "web_fetch" => "Fetch web page content",
            "code_search" => "Search code using Exa API",
            "skill_http" => "Execute HTTP-based skills",
            "skill_script" => "Execute script-based skills",
            "skill_tool" => "Execute tool-based skills",
            "arduino_upload" => "Upload Arduino sketches",
            _ => "Available tool in Housaky",
        }
    }

    fn draw_history_detail(&self, f: &mut Frame, area: Rect, tools: &[ToolEntry]) {
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
                    Paragraph::new(Span::styled(
                        "  Select a tool invocation to inspect",
                        style_muted(),
                    )),
                    inner,
                );
                return;
            }
        };

        let (status_label, status_style) = match tool.status {
            ToolStatus::Running => ("⟳ RUNNING", style_warning()),
            ToolStatus::Success => ("✓ SUCCESS", style_success()),
            ToolStatus::Failed => ("✗ FAILED", style_error()),
            ToolStatus::Cancelled => ("⊘ CANCELLED", style_muted()),
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled("  Tool       ", style_muted()),
                Span::styled(
                    &tool.name,
                    ratatui::style::Style::default()
                        .fg(Palette::TOOL)
                        .add_modifier(Modifier::BOLD),
                ),
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
                Span::styled(
                    l.to_owned(),
                    ratatui::style::Style::default().fg(Palette::TEXT),
                ),
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
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(l.to_owned(), style),
                ]));
            }
        }

        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }

    fn draw_footer(&self, f: &mut Frame, area: Rect) {
        let view_toggle = match self.view_mode {
            ToolsViewMode::Available => "tab=history",
            ToolsViewMode::History => "tab=available",
        };
        let hint = if self.filter_active {
            " Type to filter  Enter/Esc=confirm "
        } else {
            " ↑↓=navigate  /=filter  PgUp/Dn=scroll  tab=toggle view  c=clear "
        };
        let full_hint = format!("{}  [{}]", hint, view_toggle);
        f.render_widget(Paragraph::new(Span::styled(full_hint, style_muted())), area);
    }
}

impl Default for ToolsPanel {
    fn default() -> Self {
        Self::new()
    }
}


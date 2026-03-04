use crate::tui::enhanced_app::state::{AppState, StreamStatus};
use crate::tui::enhanced_app::theme::{
    style_dim, style_muted, style_status_err, style_status_loading, style_status_ok, truncate_str,
    Palette, LOGO,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn draw(f: &mut Frame, area: Rect, state: &AppState, provider: &str, model: &str) {
    let zones = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(30),    // left: brand + status
            Constraint::Length(42), // center: provider / model / view
            Constraint::Length(28), // right: key hints
        ])
        .split(area);

    draw_left(f, zones[0], state);
    draw_center(f, zones[1], provider, model, state);
    draw_right(f, zones[2], state);
}

fn draw_left(f: &mut Frame, area: Rect, state: &AppState) {
    let (status_span, indicator) = match &state.stream_status {
        StreamStatus::Idle => (Span::styled(" ● READY ", style_status_ok()), Span::raw("")),
        StreamStatus::Thinking => (
            Span::styled(
                format!(" {} THINKING ", state.spinner()),
                style_status_loading(),
            ),
            Span::raw(""),
        ),
        StreamStatus::Streaming => (
            Span::styled(
                format!(" {} STREAMING ", state.spinner()),
                style_status_loading(),
            ),
            Span::styled(
                format!(" {}t/s ", state.metrics.avg_tokens_per_sec as u64),
                style_dim(),
            ),
        ),
        StreamStatus::Done => (
            Span::styled(" ✓ DONE ", style_status_ok()),
            Span::styled(
                format!(" {}ms ", state.metrics.last_latency_ms),
                style_dim(),
            ),
        ),
        StreamStatus::Error(e) => (
            Span::styled(
                format!(" ✗ ERROR: {} ", truncate_str(e, 24)),
                style_status_err(),
            ),
            Span::raw(""),
        ),
    };

    let line = Line::from(vec![
        Span::styled(
            format!(" {} ", LOGO),
            ratatui::style::Style::default()
                .fg(Palette::BG)
                .bg(Palette::CYAN)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::raw(" "),
        status_span,
        indicator,
    ]);

    f.render_widget(Paragraph::new(line), area);
}

fn draw_center(f: &mut Frame, area: Rect, provider: &str, model: &str, state: &AppState) {
    let view_label = state.view_mode.label();
    let tab_label = state.active_tab.label().trim();

    let line = Line::from(vec![
        Span::styled(" ◉ ", style_muted()),
        Span::styled(provider, ratatui::style::Style::default().fg(Palette::CYAN)),
        Span::styled(" / ", style_muted()),
        Span::styled(model, ratatui::style::Style::default().fg(Palette::VIOLET)),
        Span::styled("  ╱  ", style_muted()),
        Span::styled(
            tab_label,
            ratatui::style::Style::default().fg(Palette::TEXT),
        ),
        Span::styled("  ╱  ", style_muted()),
        Span::styled(view_label, style_dim()),
        Span::styled("  ╱  ", style_muted()),
        Span::styled(state.metrics.format_uptime(), style_dim()),
    ]);

    f.render_widget(Paragraph::new(line), area);
}

fn draw_right(f: &mut Frame, area: Rect, state: &AppState) {
    use crate::tui::enhanced_app::state::MainTab;

    let hints: Vec<Span> = match state.active_tab {
        MainTab::Chat => vec![
            Span::styled(
                " ↵ ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("send ", style_muted()),
            Span::styled(
                " / ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("cmd ", style_muted()),
            Span::styled(
                " ↑↓ ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("hist ", style_muted()),
            Span::styled(
                " Ctrl+P ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("menu ", style_muted()),
        ],
        MainTab::Skills => vec![
            Span::styled(
                " ↵ ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("toggle ", style_muted()),
            Span::styled(
                " r ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("refresh ", style_muted()),
            Span::styled(
                " / ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("filter ", style_muted()),
        ],
        MainTab::Tools => vec![
            Span::styled(
                " ↑↓ ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("navigate ", style_muted()),
            Span::styled(
                " / ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("filter ", style_muted()),
            Span::styled(
                " c ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("clear ", style_muted()),
        ],
        MainTab::Goals => vec![
            Span::styled(
                " ↵ ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("add goal ", style_muted()),
            Span::styled(
                " Ctrl+P ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("menu ", style_muted()),
        ],
        MainTab::Metrics => vec![
            Span::styled(
                " ↑↓ ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("scroll ", style_muted()),
        ],
        MainTab::Logs => vec![
            Span::styled(
                " ↑↓ ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("scroll ", style_muted()),
            Span::styled(
                " c ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("clear ", style_muted()),
        ],
        MainTab::Config => vec![
            Span::styled(
                " ↵ ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("edit ", style_muted()),
            Span::styled(
                " Ctrl+S ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("save ", style_muted()),
            Span::styled(
                " r ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("raw  ", style_muted()),
        ],
        MainTab::Doctor => vec![
            Span::styled(
                " ↵ ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("run diagnostics ", style_muted()),
            Span::styled(
                " r ",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("refresh ", style_muted()),
        ],
    };

    f.render_widget(Paragraph::new(Line::from(hints)), area);
}


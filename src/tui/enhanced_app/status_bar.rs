use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use crate::tui::enhanced_app::theme::{
    Palette, style_dim, style_error, style_muted, style_status_err, style_status_loading,
    style_status_ok, style_warning, LOGO,
};
use crate::tui::enhanced_app::state::{AppState, StreamStatus, ViewMode};

pub fn draw(f: &mut Frame, area: Rect, state: &AppState, provider: &str, model: &str) {
    let zones = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(30),      // left: brand + status
            Constraint::Length(42),   // center: provider / model / view
            Constraint::Length(28),   // right: key hints
        ])
        .split(area);

    draw_left(f, zones[0], state);
    draw_center(f, zones[1], provider, model, state);
    draw_right(f, zones[2], state);
}

fn draw_left(f: &mut Frame, area: Rect, state: &AppState) {
    let (status_span, indicator) = match &state.stream_status {
        StreamStatus::Idle => (
            Span::styled(" ● READY ", style_status_ok()),
            Span::raw(""),
        ),
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
        Span::styled(tab_label, ratatui::style::Style::default().fg(Palette::TEXT)),
        Span::styled("  ╱  ", style_muted()),
        Span::styled(view_label, style_dim()),
        Span::styled("  ╱  ", style_muted()),
        Span::styled(state.metrics.format_uptime(), style_dim()),
    ]);

    f.render_widget(Paragraph::new(line), area);
}

fn draw_right(f: &mut Frame, area: Rect, state: &AppState) {
    // Contextual hints based on mode
    let hints: Vec<Span> = vec![
        Span::styled(" Ctrl+P ", ratatui::style::Style::default().fg(Palette::CYAN).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled("cmd ", style_muted()),
        Span::styled(" ? ", ratatui::style::Style::default().fg(Palette::CYAN).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled("help ", style_muted()),
        Span::styled(" Tab ", ratatui::style::Style::default().fg(Palette::CYAN).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled("tab  ", style_muted()),
    ];

    f.render_widget(Paragraph::new(Line::from(hints)), area);
}

fn truncate_str(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}

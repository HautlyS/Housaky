use crate::tui::enhanced_app::state::{AppState, StreamStatus};
use crate::tui::enhanced_app::theme::{
    style_muted, truncate_str, Palette, ICON_ERROR, ICON_PLAY, ICON_SUCCESS, LOGO_2077,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, area: Rect, state: &AppState, provider: &str, model: &str) {
    let zones = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(28),
            Constraint::Min(35),
            Constraint::Length(28),
        ])
        .split(area);

    draw_left(f, zones[0], state);
    draw_center(f, zones[1], provider, model, state);
    draw_right(f, zones[2], state);
}

fn draw_left(f: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .borders(Borders::NONE)
        .style(ratatui::style::Style::default().bg(Palette::BG_PANEL));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Status with 2077 indicators
    let (status_text, status_color, status_icon) = match &state.stream_status {
        StreamStatus::Idle => ("READY", Palette::SUCCESS, ICON_SUCCESS),
        StreamStatus::Thinking => ("PROCESSING", Palette::PINK, "◐"),
        StreamStatus::Streaming => ("STREAMING", Palette::CYAN, ICON_PLAY),
        StreamStatus::Done => ("COMPLETE", Palette::SUCCESS, ICON_SUCCESS),
        StreamStatus::Error(e) => (truncate_str(e, 18), Palette::ERROR, ICON_ERROR),
    };

    // Animated status indicator
    let line = Line::from(vec![
        Span::styled(
            format!(" {} ", LOGO_2077),
            ratatui::style::Style::default()
                .fg(Palette::BG)
                .bg(Palette::CYAN)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            status_icon,
            ratatui::style::Style::default()
                .fg(status_color)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} ", status_text),
            ratatui::style::Style::default()
                .fg(status_color)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
    ]);

    f.render_widget(Paragraph::new(line).alignment(Alignment::Left), inner);
}

fn draw_center(f: &mut Frame, area: Rect, provider: &str, model: &str, state: &AppState) {
    let block = Block::default()
        .borders(Borders::NONE)
        .style(ratatui::style::Style::default().bg(Palette::BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let view_label = state.view_mode.label();
    let tab_label = state.active_tab.label().trim();
    let uptime = state.metrics.format_uptime();

    // Cyberpunk styled badges - compact
    let provider_display = if provider.is_empty() {
        "NO_LINK"
    } else {
        provider
    };
    let model_display = if model.is_empty() { "DEFAULT" } else { model };

    let line = Line::from(vec![
        // Provider
        Span::styled(
            "⟪",
            ratatui::style::Style::default().fg(Palette::TEXT_MUTED),
        ),
        Span::styled(
            provider_display,
            ratatui::style::Style::default()
                .fg(Palette::CYAN)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::styled(
            "⟫",
            ratatui::style::Style::default().fg(Palette::TEXT_MUTED),
        ),
        // Separator
        Span::styled(" ║ ", ratatui::style::Style::default().fg(Palette::BORDER)),
        // Model
        Span::styled(
            "[",
            ratatui::style::Style::default().fg(Palette::TEXT_MUTED),
        ),
        Span::styled(
            model_display,
            ratatui::style::Style::default()
                .fg(Palette::PINK)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::styled(
            "]",
            ratatui::style::Style::default().fg(Palette::TEXT_MUTED),
        ),
        // Separator
        Span::styled(" ║ ", ratatui::style::Style::default().fg(Palette::BORDER)),
        // Tab
        Span::styled(
            tab_label,
            ratatui::style::Style::default()
                .fg(Palette::VIOLET)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        // Separator
        Span::styled(" ║ ", ratatui::style::Style::default().fg(Palette::BORDER)),
        // View
        Span::styled(
            view_label,
            ratatui::style::Style::default().fg(Palette::TEXT_DIM),
        ),
        // Separator
        Span::styled(" ║ ", ratatui::style::Style::default().fg(Palette::BORDER)),
        // Uptime
        Span::styled("⏱", ratatui::style::Style::default().fg(Palette::YELLOW)),
        Span::styled(
            format!(" {}", uptime),
            ratatui::style::Style::default().fg(Palette::TEXT_DIM),
        ),
    ]);

    f.render_widget(Paragraph::new(line).alignment(Alignment::Center), inner);
}

fn draw_right(f: &mut Frame, area: Rect, state: &AppState) {
    use crate::tui::enhanced_app::state::MainTab;

    let hints: Vec<Span> = match state.active_tab {
        MainTab::Chat => vec![
            Span::styled(
                "↵",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("send ", style_muted()),
            Span::styled(
                "/",
                ratatui::style::Style::default()
                    .fg(Palette::PINK)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("cmd ", style_muted()),
            Span::styled(
                "↑↓",
                ratatui::style::Style::default()
                    .fg(Palette::VIOLET)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("hist ", style_muted()),
            Span::styled(
                "⌘P",
                ratatui::style::Style::default()
                    .fg(Palette::YELLOW)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("menu", style_muted()),
        ],
        MainTab::Skills => vec![
            Span::styled(
                "↵",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("toggle ", style_muted()),
            Span::styled(
                "r",
                ratatui::style::Style::default()
                    .fg(Palette::PINK)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("refresh ", style_muted()),
            Span::styled(
                "/",
                ratatui::style::Style::default()
                    .fg(Palette::VIOLET)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("filter ", style_muted()),
        ],
        MainTab::Tools => vec![
            Span::styled(
                "↑↓",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("nav ", style_muted()),
            Span::styled(
                "/",
                ratatui::style::Style::default()
                    .fg(Palette::PINK)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("filter ", style_muted()),
            Span::styled(
                "c",
                ratatui::style::Style::default()
                    .fg(Palette::VIOLET)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("clear ", style_muted()),
        ],
        MainTab::Goals => vec![
            Span::styled(
                "↵",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("add goal ", style_muted()),
            Span::styled(
                "⌘P",
                ratatui::style::Style::default()
                    .fg(Palette::PINK)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("menu ", style_muted()),
        ],
        MainTab::Metrics => vec![
            Span::styled(
                "↑↓",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("scroll ", style_muted()),
        ],
        MainTab::Logs => vec![
            Span::styled(
                "↑↓",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("scroll ", style_muted()),
            Span::styled(
                "c",
                ratatui::style::Style::default()
                    .fg(Palette::PINK)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("clear ", style_muted()),
        ],
        MainTab::Config => vec![
            Span::styled(
                "↵",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("edit ", style_muted()),
            Span::styled(
                "⌘S",
                ratatui::style::Style::default()
                    .fg(Palette::PINK)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("save ", style_muted()),
            Span::styled(
                "R",
                ratatui::style::Style::default()
                    .fg(Palette::VIOLET)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("raw  ", style_muted()),
        ],
        MainTab::Doctor => vec![
            Span::styled(
                "↵",
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("diagnose ", style_muted()),
            Span::styled(
                "r",
                ratatui::style::Style::default()
                    .fg(Palette::PINK)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled("refresh ", style_muted()),
        ],
    };

    let block = Block::default()
        .borders(Borders::NONE)
        .style(ratatui::style::Style::default().bg(Palette::BG_PANEL));

    let inner = block.inner(area);
    f.render_widget(block, area);

    f.render_widget(
        Paragraph::new(Line::from(hints)).alignment(Alignment::Right),
        inner,
    );
}

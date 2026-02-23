use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::{CommandState, COMMAND_HELP};

pub fn draw_command_bar(f: &mut Frame, state: &CommandState, area: Rect) {
    if !state.active {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    if !state.suggestions.is_empty() {
        draw_suggestions(f, state, chunks[0]);
    }

    draw_input(f, state, chunks[1]);
}

fn draw_input(f: &mut Frame, state: &CommandState, area: Rect) {
    let input_text = format!("/{}", state.input);
    let paragraph = Paragraph::new(input_text).block(
        Block::default()
            .title(" Command (Esc to cancel, Tab for suggestions) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    f.render_widget(paragraph, area);
}

fn draw_suggestions(f: &mut Frame, state: &CommandState, area: Rect) {
    let max_height = 8.min(state.suggestions.len() + 2) as u16;
    let popup_area = Rect {
        x: area.x,
        y: area.y.saturating_sub(max_height),
        width: area.width.min(60),
        height: max_height,
    };

    f.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = state
        .suggestions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let style = if i == state.selected_suggestion {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let text = if s.args_hint.is_empty() {
                format!("{} - {}", s.command, s.description)
            } else {
                format!("{} {} - {}", s.command, s.args_hint, s.description)
            };

            ListItem::new(text).style(style)
        })
        .take(max_height.saturating_sub(2) as usize)
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Suggestions ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_suggestion));

    f.render_stateful_widget(list, popup_area, &mut list_state);
}

pub fn draw_command_help(f: &mut Frame, area: Rect) {
    let help_lines: Vec<Line> = COMMAND_HELP
        .iter()
        .map(|(cmd, desc, hint)| {
            if hint.is_empty() {
                Line::from(vec![
                    Span::styled(
                        *cmd,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(*desc, Style::default().fg(Color::Gray)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(
                        *cmd,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(*hint, Style::default().fg(Color::DarkGray)),
                    Span::raw("  "),
                    Span::styled(*desc, Style::default().fg(Color::Gray)),
                ])
            }
        })
        .collect();

    let paragraph = Paragraph::new(help_lines).block(
        Block::default()
            .title(" Commands (press / to enter command mode) ")
            .borders(Borders::ALL),
    );

    f.render_widget(paragraph, area);
}

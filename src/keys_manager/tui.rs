use crate::keys_manager::manager::KeysManager;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, Paragraph,
    },
    Frame,
};
use std::io;

// ─── State ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeysView {
    List,
    AddProvider,
    AddKey,
    Detail,
}

#[derive(Debug, Clone)]
struct ProviderRow {
    name: String,
    key_count: usize,
    enabled_count: usize,
    priority: u32,
    default_model: Option<String>,
    total_requests: u64,
    failed_requests: u64,
    rate_limited: u64,
}

struct KeysTui {
    providers: Vec<ProviderRow>,
    selected: usize,
    scroll: usize,
    view: KeysView,
    notification: Option<(String, std::time::Instant)>,
    should_quit: bool,

    // Add provider form
    form_name: String,
    form_template: String,
    form_key: String,
    form_priority: String,
    form_field: usize, // 0=name,1=template,2=key,3=priority

    // Add key form
    addkey_provider: String,
    addkey_value: String,
    addkey_field: usize, // 0=provider,1=key

    status: String,
}

impl KeysTui {
    fn new() -> Self {
        Self {
            providers: Vec::new(),
            selected: 0,
            scroll: 0,
            view: KeysView::List,
            notification: None,
            should_quit: false,

            form_name: String::new(),
            form_template: "openai".to_string(),
            form_key: String::new(),
            form_priority: "50".to_string(),
            form_field: 0,

            addkey_provider: String::new(),
            addkey_value: String::new(),
            addkey_field: 0,

            status: "Keys Manager — Housaky 2026".to_string(),
        }
    }

    async fn reload(&mut self, manager: &KeysManager) {
        let _ = manager.load().await;
        let providers = manager.get_providers().await;
        self.providers = providers.iter().map(|p| {
            let enabled = p.keys.iter().filter(|k| k.enabled).count();
            let total: u64 = p.keys.iter().map(|k| k.usage.total_requests).sum();
            let failed: u64 = p.keys.iter().map(|k| k.usage.failed_requests).sum();
            let rl: u64 = p.keys.iter().map(|k| k.usage.rate_limited_count).sum();
            ProviderRow {
                name: p.name.clone(),
                key_count: p.keys.len(),
                enabled_count: enabled,
                priority: p.priority as u32,
                default_model: p.default_model.clone(),
                total_requests: total,
                failed_requests: failed,
                rate_limited: rl,
            }
        }).collect();
    }

    fn notify(&mut self, msg: &str) {
        self.notification = Some((msg.to_string(), std::time::Instant::now()));
    }

    fn tick(&mut self) {
        if let Some((_, t)) = &self.notification {
            if t.elapsed().as_secs() > 4 { self.notification = None; }
        }
    }

    fn draw(&self, f: &mut Frame) {
        let area = f.area();
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(5),
                Constraint::Length(1),
            ])
            .split(area);

        self.draw_header(f, layout[0]);

        match self.view {
            KeysView::List      => self.draw_list_view(f, layout[1]),
            KeysView::Detail    => self.draw_detail_view(f, layout[1]),
            KeysView::AddProvider => self.draw_add_provider_form(f, layout[1]),
            KeysView::AddKey    => self.draw_add_key_form(f, layout[1]),
        }

        self.draw_footer(f, layout[2]);

        if let Some((ref msg, _)) = self.notification {
            self.draw_toast(f, msg);
        }
    }

    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let left = Line::from(vec![
            Span::styled(" 🔑 HOUSAKY ", Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" Keys Manager ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]);
        let right = Line::from(vec![
            Span::styled(format!(" {} providers ", self.providers.len()), Style::default().fg(Color::DarkGray)),
        ]);
        let splits = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(10), Constraint::Length(20)])
            .split(area);
        f.render_widget(Paragraph::new(left), splits[0]);
        f.render_widget(Paragraph::new(right).alignment(Alignment::Right), splits[1]);
    }

    fn draw_footer(&self, f: &mut Frame, area: Rect) {
        let hint = match self.view {
            KeysView::List => " ↑↓/jk=navigate  Enter/d=detail  a=add provider  k=add key  R=rotate  D=delete  s=save  q=quit ",
            KeysView::Detail => " q/Esc=back  R=rotate key  s=save ",
            KeysView::AddProvider => " Tab=next field  Enter=submit  Esc=cancel ",
            KeysView::AddKey      => " Tab=next field  Enter=submit  Esc=cancel ",
        };
        f.render_widget(
            Paragraph::new(Span::styled(hint, Style::default().fg(Color::DarkGray))),
            area,
        );
    }

    fn draw_list_view(&self, f: &mut Frame, area: Rect) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
            .split(area);

        // Provider list
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(Span::styled(" Providers ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
        let inner = block.inner(cols[0]);
        f.render_widget(block, cols[0]);

        if self.providers.is_empty() {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled("  No providers configured.", Style::default().fg(Color::DarkGray))),
                Line::from(""),
                Line::from(Span::styled("  Press a to add a provider.", Style::default().fg(Color::Gray))),
                Line::from(""),
                Line::from(Span::styled("  Or from CLI:", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled("    housaky keys manager add-provider", Style::default().fg(Color::Cyan))),
            ];
            f.render_widget(Paragraph::new(lines), inner);
        } else {
            let items: Vec<ListItem> = self.providers.iter().enumerate().map(|(i, p)| {
                let health_color = if p.enabled_count > 0 { Color::Green } else { Color::Red };
                let selected = i == self.selected;
                let row_bg = if selected { Color::Rgb(30, 30, 50) } else { Color::Reset };
                let row_mod = if selected { Modifier::BOLD } else { Modifier::empty() };

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled("● ", Style::default().fg(health_color)),
                        Span::styled(
                            format!("{:22}", p.name),
                            Style::default().fg(Color::White).bg(row_bg).add_modifier(row_mod),
                        ),
                        Span::styled(
                            format!(" {}/{}", p.enabled_count, p.key_count),
                            Style::default().fg(Color::Gray).bg(row_bg),
                        ),
                    ]),
                ])
            }).collect();

            let mut state = ratatui::widgets::ListState::default();
            state.select(Some(self.selected));
            f.render_stateful_widget(
                List::new(items).highlight_style(Style::default().bg(Color::Rgb(30, 30, 50))),
                inner,
                &mut state,
            );
        }

        // Detail panel
        self.draw_selected_detail(f, cols[1]);
    }

    fn draw_selected_detail(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(Span::styled(" Provider Details ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
        let inner = block.inner(area);
        f.render_widget(block, area);

        if let Some(p) = self.providers.get(self.selected) {
            let success_rate = if p.total_requests > 0 {
                (p.total_requests - p.failed_requests) as f64 / p.total_requests as f64
            } else { 1.0 };

            let sections = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(8), Constraint::Length(3), Constraint::Min(0)])
                .split(inner);

            let info_lines = vec![
                Line::from(Span::styled(p.name.clone(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Keys:         ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{} total", p.key_count), Style::default().fg(Color::White)),
                    Span::styled(format!("  ({} enabled)", p.enabled_count), Style::default().fg(Color::Green)),
                ]),
                Line::from(vec![
                    Span::styled("  Priority:     ", Style::default().fg(Color::DarkGray)),
                    Span::styled(p.priority.to_string(), Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("  Default model:", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        p.default_model.clone().unwrap_or_else(|| " —".into()),
                        Style::default().fg(Color::White),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  Requests:     ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{} total", p.total_requests), Style::default().fg(Color::White)),
                    Span::styled(format!("  {} failed", p.failed_requests), Style::default().fg(if p.failed_requests > 0 { Color::Red } else { Color::Green })),
                ]),
                Line::from(vec![
                    Span::styled("  Rate limited: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(p.rate_limited.to_string(), Style::default().fg(if p.rate_limited > 0 { Color::Yellow } else { Color::Green })),
                ]),
            ];
            f.render_widget(Paragraph::new(info_lines), sections[0]);

            let gauge = Gauge::default()
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(Span::styled(" Success Rate ", Style::default().fg(Color::Green))))
                .gauge_style(Style::default().fg(Color::Green).bg(Color::DarkGray))
                .ratio(success_rate.clamp(0.0, 1.0))
                .label(format!("{:.1}%", success_rate * 100.0));
            f.render_widget(gauge, sections[1]);
        } else {
            f.render_widget(
                Paragraph::new(Span::styled("  Select a provider from the list.", Style::default().fg(Color::DarkGray))),
                inner,
            );
        }
    }

    fn draw_detail_view(&self, f: &mut Frame, area: Rect) {
        // Full detail view for selected provider
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(Span::styled(
                format!(" {} — Full Details ", self.providers.get(self.selected).map(|p| p.name.as_str()).unwrap_or("?")),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        f.render_widget(block, area);
        self.draw_selected_detail(f, inner);
    }

    fn draw_add_provider_form(&self, f: &mut Frame, area: Rect) {
        let width = 60u16.min(area.width.saturating_sub(4));
        let height = 18u16;
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            (area.height.saturating_sub(height)) / 2,
            width,
            height,
        );
        f.render_widget(Clear, popup);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(Span::styled(" Add Provider ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
        let inner = block.inner(popup);
        f.render_widget(block, popup);

        let fields = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(3), // name
                Constraint::Length(3), // template
                Constraint::Length(3), // key
                Constraint::Length(3), // priority
                Constraint::Length(1), // hint
            ])
            .split(inner);

        f.render_widget(Paragraph::new(Span::styled("Fill in the provider details:", Style::default().fg(Color::Gray))), fields[0]);

        let field_data = [
            ("Provider name", &self.form_name, 0),
            ("Template (openai/anthropic/google/custom)", &self.form_template, 1),
            ("API key", &self.form_key, 2),
            ("Priority (0-100, higher=preferred)", &self.form_priority, 3),
        ];

        for (idx, (label, value, field_idx)) in field_data.iter().enumerate() {
            let is_active = self.form_field == *field_idx;
            let border_color = if is_active { Color::Yellow } else { Color::DarkGray };
            let display = if is_active {
                format!("{}|", value)
            } else {
                (*value).clone()
            };
            let widget = Paragraph::new(display.as_str())
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(Span::styled(*label, Style::default().fg(border_color))));
            f.render_widget(widget, fields[idx + 1]);
        }

        f.render_widget(
            Paragraph::new(Span::styled("Tab=next field  Enter=submit  Esc=cancel", Style::default().fg(Color::DarkGray))),
            fields[5],
        );
    }

    fn draw_add_key_form(&self, f: &mut Frame, area: Rect) {
        let width = 58u16.min(area.width.saturating_sub(4));
        let height = 12u16;
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            (area.height.saturating_sub(height)) / 2,
            width,
            height,
        );
        f.render_widget(Clear, popup);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green))
            .title(Span::styled(" Add API Key to Provider ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)));
        let inner = block.inner(popup);
        f.render_widget(block, popup);

        let fields = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(inner);

        f.render_widget(Paragraph::new(Span::styled("Enter provider name and API key:", Style::default().fg(Color::Gray))), fields[0]);

        let field_data = [
            ("Provider name", &self.addkey_provider, 0),
            ("API key value", &self.addkey_value, 1),
        ];
        for (idx, (label, value, field_idx)) in field_data.iter().enumerate() {
            let is_active = self.addkey_field == *field_idx;
            let border_color = if is_active { Color::Yellow } else { Color::DarkGray };
            let display = if is_active { format!("{}|", value) } else { (*value).clone() };
            let widget = Paragraph::new(display.as_str())
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(Span::styled(*label, Style::default().fg(border_color))));
            f.render_widget(widget, fields[idx + 1]);
        }
        f.render_widget(
            Paragraph::new(Span::styled("Tab=next field  Enter=submit  Esc=cancel", Style::default().fg(Color::DarkGray))),
            fields[3],
        );
    }

    fn draw_toast(&self, f: &mut Frame, msg: &str) {
        let width = (msg.len() + 6).min(f.area().width as usize) as u16;
        let area = Rect::new(f.area().width.saturating_sub(width + 1), 1, width, 3);
        let toast = Paragraph::new(msg)
            .style(Style::default().fg(Color::Black).bg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)));
        f.render_widget(Clear, area);
        f.render_widget(toast, area);
    }
}

// ─── Entry point ──────────────────────────────────────────────────────────────

pub async fn run_keys_tui(manager: &KeysManager) -> Result<()> {
    use crossterm::{
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        event::EnableMouseCapture,
        event::DisableMouseCapture,
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut app = KeysTui::new();
    app.reload(manager).await;

    let res = run_keys_loop(&mut terminal, &mut app, manager).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

async fn run_keys_loop(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    app: &mut KeysTui,
    manager: &KeysManager,
) -> Result<()> {
    loop {
        terminal.draw(|f| app.draw(f))?;
        app.tick();

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.view {
                    KeysView::List => {
                        handle_list_key(app, key, manager).await?;
                    }
                    KeysView::Detail => {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('q') => { app.view = KeysView::List; }
                            KeyCode::Char('R') => { rotate_selected(app, manager).await; }
                            KeyCode::Char('s') => { manager.save().await?; app.notify("Saved."); }
                            _ => {}
                        }
                    }
                    KeysView::AddProvider => {
                        handle_add_provider_key(app, key, manager).await?;
                    }
                    KeysView::AddKey => {
                        handle_add_key_key(app, key, manager).await?;
                    }
                }
            }
        }

        if app.should_quit { break; }
    }
    Ok(())
}

async fn handle_list_key(
    app: &mut KeysTui,
    key: crossterm::event::KeyEvent,
    manager: &KeysManager,
) -> Result<()> {
    match (key.modifiers, key.code) {
        (_, KeyCode::Char('q')) | (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            app.should_quit = true;
        }
        (_, KeyCode::Up) => {
            if app.selected > 0 { app.selected -= 1; }
        }
        (_, KeyCode::Char('k')) if key.modifiers == KeyModifiers::NONE => {
            if app.selected > 0 { app.selected -= 1; }
        }
        (_, KeyCode::Down) => {
            if app.selected + 1 < app.providers.len() { app.selected += 1; }
        }
        (_, KeyCode::Char('j')) => {
            if app.selected + 1 < app.providers.len() { app.selected += 1; }
        }
        (_, KeyCode::Enter) | (_, KeyCode::Char('d')) => {
            if !app.providers.is_empty() { app.view = KeysView::Detail; }
        }
        (_, KeyCode::Char('a')) => {
            app.form_name.clear();
            app.form_template = "openai".to_string();
            app.form_key.clear();
            app.form_priority = "50".to_string();
            app.form_field = 0;
            app.view = KeysView::AddProvider;
        }
        (_, KeyCode::Char('K')) => {
            app.addkey_provider = app.providers.get(app.selected).map(|p| p.name.clone()).unwrap_or_default();
            app.addkey_value.clear();
            app.addkey_field = 0;
            app.view = KeysView::AddKey;
        }
        (_, KeyCode::Char('R')) => {
            rotate_selected(app, manager).await;
        }
        (_, KeyCode::Char('r')) => {
            app.reload(manager).await;
            app.notify("Refreshed.");
        }
        (_, KeyCode::Char('s')) => {
            manager.save().await?;
            app.notify("Saved.");
        }
        (_, KeyCode::Char('D')) => {
            // delete selected provider (future: confirm dialog)
            app.notify("Use CLI: housaky keys manager — delete not yet implemented in TUI.");
        }
        _ => {}
    }
    Ok(())
}

async fn handle_add_provider_key(
    app: &mut KeysTui,
    key: crossterm::event::KeyEvent,
    manager: &KeysManager,
) -> Result<()> {
    match key.code {
        KeyCode::Esc => { app.view = KeysView::List; }
        KeyCode::Tab => { app.form_field = (app.form_field + 1) % 4; }
        KeyCode::BackTab => { app.form_field = if app.form_field == 0 { 3 } else { app.form_field - 1 }; }
        KeyCode::Enter => {
            if app.form_field < 3 {
                app.form_field += 1;
            } else {
                // Submit
                let name = app.form_name.trim().to_string();
                let template = app.form_template.trim().to_string();
                let key_val = app.form_key.trim().to_string();
                let priority_val: u32 = app.form_priority.trim().parse().unwrap_or(50);

                if name.is_empty() {
                    app.notify("Provider name is required.");
                    return Ok(());
                }
                use crate::keys_manager::manager::ProviderPriority;
                let priority = ProviderPriority::from_str(&priority_val.to_string()).unwrap_or_default();
                let keys = if key_val.is_empty() { vec![] } else { vec![key_val] };
                match manager.add_provider_with_template(&name, &template, keys, priority).await {
                    Ok(_) => {
                        manager.save().await?;
                        app.reload(manager).await;
                        app.notify(&format!("Added provider: {}", name));
                        app.view = KeysView::List;
                    }
                    Err(e) => { app.notify(&format!("Error: {}", e)); }
                }
            }
        }
        KeyCode::Backspace => {
            let field = match app.form_field {
                0 => &mut app.form_name,
                1 => &mut app.form_template,
                2 => &mut app.form_key,
                3 => &mut app.form_priority,
                _ => return Ok(()),
            };
            field.pop();
        }
        KeyCode::Char(c) => {
            let field = match app.form_field {
                0 => &mut app.form_name,
                1 => &mut app.form_template,
                2 => &mut app.form_key,
                3 => &mut app.form_priority,
                _ => return Ok(()),
            };
            field.push(c);
        }
        _ => {}
    }
    Ok(())
}

async fn handle_add_key_key(
    app: &mut KeysTui,
    key: crossterm::event::KeyEvent,
    manager: &KeysManager,
) -> Result<()> {
    match key.code {
        KeyCode::Esc => { app.view = KeysView::List; }
        KeyCode::Tab => { app.addkey_field = (app.addkey_field + 1) % 2; }
        KeyCode::BackTab => { app.addkey_field = if app.addkey_field == 0 { 1 } else { 0 }; }
        KeyCode::Enter => {
            if app.addkey_field == 0 {
                app.addkey_field = 1;
            } else {
                let provider = app.addkey_provider.trim().to_string();
                let key_val = app.addkey_value.trim().to_string();
                if provider.is_empty() || key_val.is_empty() {
                    app.notify("Both fields are required.");
                    return Ok(());
                }
                match manager.add_key(&provider, key_val, None).await {
                    Ok(_) => {
                        manager.save().await?;
                        app.reload(manager).await;
                        app.notify(&format!("Added key to {}", provider));
                        app.view = KeysView::List;
                    }
                    Err(e) => { app.notify(&format!("Error: {}", e)); }
                }
            }
        }
        KeyCode::Backspace => {
            if app.addkey_field == 0 { app.addkey_provider.pop(); }
            else { app.addkey_value.pop(); }
        }
        KeyCode::Char(c) => {
            if app.addkey_field == 0 { app.addkey_provider.push(c); }
            else { app.addkey_value.push(c); }
        }
        _ => {}
    }
    Ok(())
}

async fn rotate_selected(app: &mut KeysTui, manager: &KeysManager) {
    if let Some(provider) = app.providers.get(app.selected) {
        let name = provider.name.clone();
        if let Ok(Some(key)) = manager.rotate_key(&name).await {
            let tail = &key.key[key.key.len().saturating_sub(6)..];
            app.notify(&format!("Rotated {} → …{}", name, tail));
        } else {
            app.notify(&format!("No next key available for {}", name));
        }
    }
}

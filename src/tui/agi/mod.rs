pub mod agi_dashboard;
pub mod capability_panel;
pub mod goal_panel;
pub mod thought_panel;

pub use agi_dashboard::AGIDashboard;

use crate::config::Config;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use std::io;
use std::time::Duration;

pub fn run_agi_tui(config: Config, provider: Option<String>, model: Option<String>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let provider_name = provider
        .or_else(|| config.default_provider.clone())
        .unwrap_or_else(|| "openrouter".to_string());
    let model_name = model
        .or_else(|| config.default_model.clone())
        .unwrap_or_else(|| "auto".to_string());

    let mut dashboard = AGIDashboard::new(config, provider_name, model_name);
    let res = run_dashboard(&mut terminal, &mut dashboard);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {err:?}");
    }

    Ok(())
}

fn run_dashboard(
    terminal: &mut ratatui::Terminal<CrosstermBackend<io::Stdout>>,
    dashboard: &mut AGIDashboard,
) -> Result<()> {
    let mut last_update = std::time::Instant::now();

    loop {
        if last_update.elapsed() >= Duration::from_millis(100) {
            terminal.draw(|f| dashboard.draw(f))?;
            last_update = std::time::Instant::now();
        }

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                match (key.modifiers, key.code) {
                    (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                        return Ok(());
                    }
                    (_, KeyCode::Esc) => {
                        if dashboard.should_quit() {
                            return Ok(());
                        }
                    }
                    _ => {}
                }
                dashboard.handle_key(key)?;
            }
        }

        dashboard.update();

        if dashboard.should_quit() {
            return Ok(());
        }
    }
}

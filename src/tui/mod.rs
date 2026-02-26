#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::format_push_string, clippy::match_same_arms, clippy::match_wildcard_for_single_variants, clippy::trivially_copy_pass_by_ref)]

pub mod agi;
pub mod app;
pub mod chat;
pub mod command;
pub mod command_palette;
pub mod enhanced_app;  // now a folder: src/tui/enhanced_app/
pub mod help;
pub mod live;
pub mod provider;
pub mod search;
pub mod skills_market;
pub mod state_panel;
pub mod status_bar;

pub use agi::AGIDashboard;
pub use live::LiveAGIApp;

pub use app::App;
#[allow(unused_imports)]
pub use command::{Command, CommandState, CommandSuggestion};
pub use enhanced_app::EnhancedApp;

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
    agi::run_agi_tui(config, provider, model)
}

pub fn run_chat_tui(config: Config, provider: Option<String>, model: Option<String>) -> Result<()> {
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

    let mut app = EnhancedApp::new(config, provider_name, model_name);
    let res = run_enhanced_app(&mut terminal, &mut app);

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

pub fn run_skills_market_tui(config: Config, repo_root: std::path::PathBuf) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut app = skills_market::SkillsMarketApp::new(config, repo_root);
    app.load_skills();

    let res = loop {
        terminal.draw(|f| app.draw(f))?;
        if event::poll(Duration::from_millis(33))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key)?;
                if app.quit {
                    break Ok(());
                }
            }
        }
    };

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

pub fn run_provider_tui(config: Config) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut app = App::new_provider_test(config);
    let res = run_app(&mut terminal, &mut app);

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

fn run_enhanced_app(
    terminal: &mut ratatui::Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut EnhancedApp,
) -> Result<()> {
    let tick = Duration::from_millis(33); // ~30 fps
    let mut last_draw = std::time::Instant::now();

    loop {
        // Tick state (spinner, notifications, etc.)
        app.update();

        // Draw at ~30 fps
        if last_draw.elapsed() >= tick {
            terminal.draw(|f| app.draw(f))?;
            last_draw = std::time::Instant::now();
        }

        // Poll for events — short timeout keeps animations smooth
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key)?;
            }
        }

        if app.should_quit() {
            return Ok(());
        }
    }
}

fn run_app(
    terminal: &mut ratatui::Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let mut last_update = std::time::Instant::now();

    loop {
        // Update UI at 30 FPS
        if last_update.elapsed() >= Duration::from_millis(33) {
            terminal.draw(|f| app.draw(f))?;
            last_update = std::time::Instant::now();
        }

        // Handle events with timeout for responsive UI
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                match (key.modifiers, key.code) {
                    (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                        if app.should_exit_on_ctrl_c() {
                            return Ok(());
                        }
                    }
                    (_, KeyCode::Esc) => {
                        if app.should_exit_on_esc() {
                            return Ok(());
                        }
                    }
                    _ => {}
                }
                app.handle_key(key)?;
            }
        }

        // Update status bar animations
        app.update();

        if app.should_quit() {
            return Ok(());
        }
    }
}

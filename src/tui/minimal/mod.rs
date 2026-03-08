//! Minimal TUI Module
//!
//! A clean, focused terminal user interface for Housaky with Kowalski integration.
//!
//! Features:
//! - Minimal chat-focused interface
//! - Kowalski agent integration (code, web, academic, data)
//! - Ctrl+K hotkey for quick provider/model switching
//! - A2A WebSocket panel for inter-agent communication
//! - AMOLED black & white aesthetic with psychedelic accents
//! - "/" command autocomplete with adaptive filtering

pub mod a2a_panel;
pub mod agents;
pub mod app;
pub mod chat;
pub mod command_palette;
pub mod input;
pub mod keys_popup;
pub mod theme;

pub use app::MinimalApp;
pub use command_palette::CommandPalette;
pub use theme::Theme;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use std::io;
use std::time::Duration;

use crate::config::Config;

/// Run the minimal TUI
pub fn run_minimal_tui(
    config: Config,
    provider: Option<String>,
    model: Option<String>,
) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    // Resolve provider/model
    let provider_name = provider
        .or_else(|| config.default_provider.clone())
        .unwrap_or_else(|| "openrouter".to_string());
    let model_name = model
        .or_else(|| config.default_model.clone())
        .unwrap_or_else(|| "auto".to_string());

    // Create app
    let mut app = MinimalApp::new(config, provider_name, model_name);

    // Main loop
    let res = run_app_loop(&mut terminal, &mut app);

    // Cleanup
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

/// Main application loop
fn run_app_loop(
    terminal: &mut ratatui::Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut MinimalApp,
) -> Result<()> {
    let tick = Duration::from_millis(33); // ~30 fps
    let mut last_draw = std::time::Instant::now();

    loop {
        // Update state (animations, streaming, etc.)
        app.update();

        // Draw at ~30 fps
        if last_draw.elapsed() >= tick {
            terminal.draw(|f| app.draw(f))?;
            last_draw = std::time::Instant::now();
        }

        // Poll for events
        if event::poll(Duration::from_millis(10))? {
            match event::read()? {
                Event::Key(key) => {
                    if let Err(e) = app.handle_key(key) {
                        app.chat.push_system(&format!("Error: {}", e));
                    }
                }
                Event::Mouse(mouse) => {
                    let _ = app.handle_mouse(mouse);
                }
                Event::Resize(_, _) => {
                    terminal.autoresize()?;
                    terminal.draw(|f| app.draw(f))?;
                    last_draw = std::time::Instant::now();
                }
                _ => {}
            }
        }

        if app.should_quit() {
            return Ok(());
        }
    }
}

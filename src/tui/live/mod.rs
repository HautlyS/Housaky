pub mod command_palette;
pub mod live_app;
pub mod metrics_panel;
pub mod suggestions;
pub mod thought_stream;

pub use command_palette::CommandPalette;
pub use live_app::LiveAGIApp;
pub use metrics_panel::MetricsPanel;
pub use suggestions::SuggestionEngine;
pub use thought_stream::ThoughtStreamPanel;

use crate::config::Config;
use anyhow::Result;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

pub fn run_live_agi_tui(
    config: Config,
    provider: Option<String>,
    model: Option<String>,
) -> Result<()> {
    let provider_name = provider
        .or_else(|| config.default_provider.clone())
        .unwrap_or_else(|| "openrouter".to_string());
    let model_name = model
        .or_else(|| config.default_model.clone())
        .unwrap_or_else(|| "auto".to_string());

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = LiveAGIApp::new(config, provider_name, model_name);

    loop {
        terminal.draw(|f| app.draw(f))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key)?;
                if app.should_quit() {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
    )?;
    terminal.show_cursor()?;

    Ok(())
}

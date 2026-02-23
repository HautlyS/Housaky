pub mod app;
pub mod editor;
pub mod menu;
pub mod sections;
pub mod widgets;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::config::Config;
use app::ConfigEditorApp;

pub fn run_config_tui(config: Config, section: Option<String>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = ConfigEditorApp::new(config, section);

    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = res {
        eprintln!("Error: {e}");
    }

    if app.dirty {
        println!(
            "Configuration saved to {}",
            app.config.config_path.display()
        );
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut ConfigEditorApp,
) -> Result<()> {
    loop {
        terminal.draw(|f| app.draw(f))?;

        if let Event::Key(key) = event::read()? {
            match (key.modifiers, key.code) {
                (event::KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                    break;
                }
                (event::KeyModifiers::NONE, KeyCode::Char('q')) => {
                    if app.dirty && !app.confirmed_exit {
                        app.show_save_prompt = true;
                    } else {
                        break;
                    }
                }
                (event::KeyModifiers::NONE, KeyCode::Char('s')) => {
                    app.save()?;
                }
                (event::KeyModifiers::NONE, KeyCode::Char('?')) => {
                    app.show_help = !app.show_help;
                }
                (event::KeyModifiers::NONE, KeyCode::Esc) => {
                    if app.show_help || app.show_save_prompt {
                        app.show_help = false;
                        app.show_save_prompt = false;
                    }
                }
                (event::KeyModifiers::NONE, KeyCode::Tab) => {
                    app.next_section();
                }
                (event::KeyModifiers::SHIFT, KeyCode::BackTab) => {
                    app.prev_section();
                }
                (event::KeyModifiers::NONE, KeyCode::Up) => {
                    app.move_up();
                }
                (event::KeyModifiers::NONE, KeyCode::Down) => {
                    app.move_down();
                }
                (event::KeyModifiers::NONE, KeyCode::Enter) => {
                    app.edit_field()?;
                }
                (event::KeyModifiers::NONE, KeyCode::Char(c)) => {
                    if app.show_save_prompt {
                        match c {
                            'y' | 'Y' => {
                                app.save()?;
                                break;
                            }
                            'n' | 'N' => {
                                break;
                            }
                            _ => {}
                        }
                    } else {
                        app.handle_char(c)?;
                    }
                }
                (event::KeyModifiers::NONE, KeyCode::Backspace) => {
                    app.handle_backspace();
                }
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

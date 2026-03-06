//! Minimal TUI Application
//!
//! A clean, focused chat interface with Kowalski agent integration.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::config::Config;
use crate::housaky::kowalski_integration::KowalskiBridge;
use crate::housaky::housaky_agent::KowalskiIntegrationConfig;
use crate::keys_manager::manager::get_global_keys_manager;
use crate::providers::{create_provider_with_keys_manager, ChatMessage, Provider};

use super::agents::{AgentStatus, AgentType, AgentsPanel};
use super::chat::{ChatPanel, Message, Role};
use super::input::InputBar;
use super::keys_popup::{KeysPopup, ProviderEntry};
use super::theme::{self, LOGO_MINI};

/// Focus state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Chat,
    Input,
    Agents,
}

/// App state
pub struct MinimalApp {
    // Config
    config: Config,
    provider_name: String,
    model_name: String,

    // Components
    pub chat: ChatPanel,
    pub input: InputBar,
    pub agents: AgentsPanel,
    pub keys_popup: KeysPopup,

    // State
    focus: Focus,
    quit: bool,

    // Animation
    anim_frame: usize,
    anim_tick: usize,

    // Kowalski bridge
    kowalski: Option<Arc<KowalskiBridge>>,

    // Streaming
    streaming: Arc<AtomicBool>,
    stream_result: Arc<std::sync::Mutex<Option<Result<String, String>>>>,
    stream_chunks: Arc<std::sync::Mutex<Vec<String>>>,

    // Provider
    provider: Option<Arc<dyn Provider>>,
    resolved_key: Option<String>,

    // Log receiver
    log_rx: Option<std::sync::mpsc::Receiver<String>>,
}

impl MinimalApp {
    pub fn new(config: Config, provider_name: String, model_name: String) -> Self {
        // Resolve API key
        let resolved_key = Self::resolve_api_key(&provider_name, &config);

        // Create provider
        let provider = create_provider_with_keys_manager(&provider_name, resolved_key.as_deref())
            .ok()
            .map(Arc::from);

        // Create Kowalski bridge - use vendor path
        let kowalski_path = config.workspace_dir.parent()
            .map(|p| p.join("vendor/kowalski"))
            .unwrap_or_else(|| std::path::PathBuf::from("vendor/kowalski"));
        
        let kowalski = if kowalski_path.exists() {
            Some(Arc::new(KowalskiBridge::new(&KowalskiIntegrationConfig {
                enabled: true,
                kowalski_path,
                enable_federation: true,
                enable_code_agent: true,
                enable_web_agent: true,
                enable_academic_agent: true,
                enable_data_agent: true,
            })))
        } else {
            None
        };

        let mut app = Self {
            config,
            provider_name: provider_name.clone(),
            model_name: model_name.clone(),
            chat: ChatPanel::new(),
            input: InputBar::new(),
            agents: AgentsPanel::new(),
            keys_popup: KeysPopup::new(),
            focus: Focus::Input,
            quit: false,
            anim_frame: 0,
            anim_tick: 0,
            kowalski,
            streaming: Arc::new(AtomicBool::new(false)),
            stream_result: Arc::new(std::sync::Mutex::new(None)),
            stream_chunks: Arc::new(std::sync::Mutex::new(Vec::new())),
            provider,
            resolved_key,
            log_rx: None,
        };

        // Initialize keys popup with providers
        app.refresh_providers();

        // Set current provider/model in popup
        app.keys_popup.set_current(&provider_name, &model_name);

        app
    }

    pub fn set_log_receiver(&mut self, rx: std::sync::mpsc::Receiver<String>) {
        self.log_rx = Some(rx);
    }

    fn resolve_api_key(provider: &str, config: &Config) -> Option<String> {
        // Try keys manager first (blocking call)
        let keys_manager = get_global_keys_manager();
        let result = std::thread::spawn({
            let provider = provider.to_string();
            move || {
                let rt = tokio::runtime::Runtime::new().ok()?;
                rt.block_on(async {
                    keys_manager.get_next_key(&provider).await.map(|k| k.key)
                })
            }
        }).join().unwrap_or(None);
        
        if let Some(key) = result {
            return Some(key);
        }

        // Try config
        if let Some(key) = &config.api_key {
            if !key.is_empty() {
                return Some(key.clone());
            }
        }

        // Try env vars
        for var in &["OPENROUTER_API_KEY", "ANTHROPIC_API_KEY", "OPENAI_API_KEY"] {
            if let Ok(key) = std::env::var(var) {
                if !key.is_empty() {
                    return Some(key);
                }
            }
        }

        None
    }

    fn refresh_providers(&mut self) {
        let keys_manager = get_global_keys_manager();

        // Get providers from keys manager
        let providers = std::thread::spawn({
            move || {
                let rt = tokio::runtime::Runtime::new().ok()?;
                rt.block_on(async {
                    let providers = keys_manager.get_providers().await;
                    Some(providers)
                })
            }
        })
        .join()
        .unwrap_or(None)
        .unwrap_or_default();

        let entries: Vec<ProviderEntry> = providers
            .iter()
            .map(|p| {
                let key_suffix = p.keys.first()
                    .map(|k| {
                        if k.key.len() > 4 {
                            k.key[k.key.len() - 4..].to_string()
                        } else {
                            "****".to_string()
                        }
                    })
                    .unwrap_or_else(|| "none".to_string());

                let enabled = p.keys.iter().any(|k| k.enabled);

                // Common models per provider
                let models = match p.name.as_str() {
                    "openrouter" => vec![
                        "auto".to_string(),
                        "anthropic/claude-3.5-sonnet".to_string(),
                        "openai/gpt-4-turbo".to_string(),
                        "google/gemini-pro".to_string(),
                    ],
                    "anthropic" => vec![
                        "claude-3-5-sonnet-20241022".to_string(),
                        "claude-3-opus-20240229".to_string(),
                        "claude-3-haiku-20240307".to_string(),
                    ],
                    "openai" => vec![
                        "gpt-4-turbo".to_string(),
                        "gpt-4".to_string(),
                        "gpt-3.5-turbo".to_string(),
                    ],
                    _ => vec!["default".to_string()],
                };

                ProviderEntry::new(&p.name, key_suffix, enabled).with_models(models)
            })
            .collect();

        // If no providers, add defaults
        let entries = if entries.is_empty() {
            vec![
                ProviderEntry::new("openrouter", "none", false)
                    .with_models(vec!["auto".to_string()]),
                ProviderEntry::new("anthropic", "none", false)
                    .with_models(vec!["claude-3-5-sonnet-20241022".to_string()]),
                ProviderEntry::new("openai", "none", false)
                    .with_models(vec!["gpt-4-turbo".to_string()]),
            ]
        } else {
            entries
        };

        self.keys_popup.set_providers(entries);
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    pub fn update(&mut self) {
        // Animation tick
        self.anim_tick += 1;
        if self.anim_tick >= 10 {
            self.anim_tick = 0;
            self.anim_frame = (self.anim_frame + 1) % 4;
        }

        // Drain log receiver
        if let Some(rx) = &self.log_rx {
            while let Ok(msg) = rx.try_recv() {
                self.chat.push_system(&msg);
            }
        }

        // Check streaming result
        if self.streaming.load(Ordering::Relaxed) {
            // Drain chunks
            if let Ok(mut chunks) = self.stream_chunks.try_lock() {
                for chunk in chunks.drain(..) {
                    self.chat.append_stream(&chunk);
                }
            }
        }

        // Check if streaming finished
        if let Ok(mut result) = self.stream_result.try_lock() {
            if let Some(res) = result.take() {
                self.streaming.store(false, Ordering::Relaxed);
                self.chat.finish_streaming();
                if let Err(e) = res {
                    self.chat.push_system(&format!("Error: {}", e));
                }
            }
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Layout: header, body (chat + agents sidebar), input
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(10),    // Body
                Constraint::Length(3),  // Input
            ])
            .split(area);

        self.draw_header(frame, main_layout[0]);
        self.draw_body(frame, main_layout[1]);
        self.draw_input(frame, main_layout[2]);

        // Keys popup overlay
        if self.keys_popup.visible {
            self.keys_popup.draw(frame, area);
        }
    }

    fn draw_header(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(theme::style_border())
            .style(theme::style_base());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Header content
        let spinner = theme::SPINNER[self.anim_frame];
        let status = if self.streaming.load(Ordering::Relaxed) {
            format!("{} streaming...", spinner)
        } else {
            format!("{} ready", spinner)
        };

        let header_line = Line::from(vec![
            Span::styled(LOGO_MINI, theme::style_title()),
            Span::raw("  "),
            Span::styled(&self.provider_name, theme::style_subtitle()),
            Span::raw("/"),
            Span::styled(&self.model_name, theme::style_dim()),
            Span::raw("  "),
            Span::styled(status, if self.streaming.load(Ordering::Relaxed) {
                theme::style_warning()
            } else {
                theme::style_success()
            }),
        ]);

        let header = Paragraph::new(header_line);
        frame.render_widget(header, inner);
    }

    fn draw_body(&mut self, frame: &mut Frame, area: Rect) {
        // Split: chat (main) + agents sidebar (if visible)
        if self.agents.visible {
            let body_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(40),       // Chat
                    Constraint::Length(22),    // Agents sidebar
                ])
                .split(area);

            self.chat.draw(frame, body_layout[0], self.focus == Focus::Chat);
            self.agents.draw(frame, body_layout[1], self.focus == Focus::Agents);
        } else {
            self.chat.draw(frame, area, self.focus == Focus::Chat);
        }
    }

    fn draw_input(&self, frame: &mut Frame, area: Rect) {
        self.input.draw(frame, area, self.focus == Focus::Input);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Global hotkeys
        match (key.modifiers, key.code) {
            // Ctrl+C - quit
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.quit = true;
                return Ok(());
            }
            // Ctrl+K - keys popup
            (KeyModifiers::CONTROL, KeyCode::Char('k')) => {
                self.keys_popup.toggle();
                return Ok(());
            }
            // Ctrl+L - clear chat
            (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
                self.chat.clear();
                self.chat.push_system("Chat cleared");
                return Ok(());
            }
            // Ctrl+A - toggle agents panel
            (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
                self.agents.toggle_visibility();
                return Ok(());
            }
            _ => {}
        }

        // Keys popup handling
        if self.keys_popup.visible {
            match key.code {
                KeyCode::Esc => self.keys_popup.back(),
                KeyCode::Enter => {
                    if let Some((provider, model)) = self.keys_popup.enter() {
                        self.switch_provider(&provider, &model);
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => self.keys_popup.previous(),
                KeyCode::Down | KeyCode::Char('j') => self.keys_popup.next(),
                _ => {}
            }
            return Ok(());
        }

        // Focus-specific handling
        match self.focus {
            Focus::Input => self.handle_input_key(key)?,
            Focus::Chat => self.handle_chat_key(key)?,
            Focus::Agents => self.handle_agents_key(key)?,
        }

        Ok(())
    }

    fn handle_input_key(&mut self, key: KeyEvent) -> Result<()> {
        match (key.modifiers, key.code) {
            // Navigation
            (_, KeyCode::Tab) => {
                self.focus = if self.agents.visible {
                    Focus::Agents
                } else {
                    Focus::Chat
                };
            }
            _ => {}
        }

        match (key.modifiers, key.code) {
            // Submit
            (_, KeyCode::Enter) => {
                let content = self.input.take();
                if !content.trim().is_empty() {
                    self.handle_submit(&content)?;
                }
            }

            // Escape
            (_, KeyCode::Esc) => {
                if !self.input.is_empty() {
                    self.input.clear();
                } else {
                    self.focus = Focus::Chat;
                }
            }

            // Text editing
            (_, KeyCode::Backspace) => self.input.backspace(),
            (_, KeyCode::Delete) => self.input.delete(),
            (KeyModifiers::CONTROL | KeyModifiers::ALT, KeyCode::Left) => self.input.move_word_left(),
            (KeyModifiers::CONTROL | KeyModifiers::ALT, KeyCode::Right) => self.input.move_word_right(),
            (_, KeyCode::Left) => self.input.move_left(),
            (_, KeyCode::Right) => self.input.move_right(),
            (_, KeyCode::Home) | (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
                self.input.move_start()
            }
            (_, KeyCode::End) | (KeyModifiers::CONTROL, KeyCode::Char('e')) => {
                self.input.move_end()
            }
            (KeyModifiers::CONTROL, KeyCode::Char('w')) => self.input.delete_word(),
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => self.input.clear(),

            // History
            (_, KeyCode::Up) => self.input.history_up(),
            (_, KeyCode::Down) => self.input.history_down(),

            // Character input
            (_, KeyCode::Char(c)) => self.input.insert(c),

            _ => {}
        }
        Ok(())
    }

    fn handle_chat_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Tab => self.focus = Focus::Input,
            KeyCode::Up | KeyCode::Char('k') => self.chat.scroll_up(1),
            KeyCode::Down | KeyCode::Char('j') => self.chat.scroll_down(1),
            KeyCode::PageUp => self.chat.scroll_up(10),
            KeyCode::PageDown => self.chat.scroll_down(10),
            KeyCode::Home | KeyCode::Char('g') => self.chat.scroll_to_top(),
            KeyCode::End | KeyCode::Char('G') => self.chat.scroll_to_bottom(),
            KeyCode::Char('i') => self.focus = Focus::Input,
            _ => {}
        }
        Ok(())
    }

    fn handle_agents_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Tab => self.focus = Focus::Input,
            KeyCode::Up | KeyCode::Char('k') => self.agents.previous(),
            KeyCode::Down | KeyCode::Char('j') => self.agents.next(),
            KeyCode::Enter => {
                // Select agent for next message
                if let Some(agent) = self.agents.selected_agent() {
                    self.chat.push_system(&format!(
                        "Selected agent: {} - {}",
                        agent.agent_type.display(),
                        agent.agent_type.description()
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                if self.focus == Focus::Chat {
                    self.chat.scroll_up(3);
                }
            }
            MouseEventKind::ScrollDown => {
                if self.focus == Focus::Chat {
                    self.chat.scroll_down(3);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_submit(&mut self, content: &str) -> Result<()> {
        let content = content.trim();

        // Handle commands
        if content.starts_with('/') {
            return self.handle_command(content);
        }

        // Add user message
        self.chat.push_user(content);

        // Check if targeting a specific agent
        let selected_agent = self.agents.selected_type();

        // Send to LLM or Kowalski agent
        if let Some(agent_type) = selected_agent {
            if self.kowalski.is_some() {
                self.send_to_kowalski(agent_type, content)?;
            } else {
                self.chat.push_system("Kowalski not available. Using default LLM.");
                self.send_to_llm(content)?;
            }
        } else {
            self.send_to_llm(content)?;
        }

        Ok(())
    }

    fn handle_command(&mut self, cmd: &str) -> Result<()> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let command = parts.first().copied().unwrap_or("");

        match command {
            "/help" | "/h" | "/?" => {
                self.chat.push_system(
                    "Commands:\n\
                     /help        - Show this help\n\
                     /clear       - Clear chat\n\
                     /keys        - Open keys popup (or Ctrl+K)\n\
                     /agents      - Toggle agents panel (or Ctrl+A)\n\
                     /agent <n>   - Select agent (code, web, academic, data)\n\
                     /provider <n>- Switch provider\n\
                     /model <n>   - Switch model\n\
                     /status      - Show status\n\
                     /quit        - Quit (or Ctrl+C)",
                );
            }
            "/clear" | "/c" => {
                self.chat.clear();
                self.chat.push_system("Chat cleared");
            }
            "/keys" | "/k" => {
                self.keys_popup.show();
            }
            "/agents" => {
                self.agents.toggle_visibility();
                let status = if self.agents.visible { "visible" } else { "hidden" };
                self.chat.push_system(&format!("Agents panel {}", status));
            }
            "/agent" => {
                if let Some(name) = parts.get(1) {
                    match *name {
                        "code" | "c" => {
                            self.agents.selected = 0;
                            self.agents.list_state.select(Some(0));
                            self.chat.push_system("Selected: Code Agent");
                        }
                        "web" | "w" => {
                            self.agents.selected = 1;
                            self.agents.list_state.select(Some(1));
                            self.chat.push_system("Selected: Web Agent");
                        }
                        "academic" | "a" => {
                            self.agents.selected = 2;
                            self.agents.list_state.select(Some(2));
                            self.chat.push_system("Selected: Academic Agent");
                        }
                        "data" | "d" => {
                            self.agents.selected = 3;
                            self.agents.list_state.select(Some(3));
                            self.chat.push_system("Selected: Data Agent");
                        }
                        _ => {
                            self.chat.push_system("Unknown agent. Use: code, web, academic, data");
                        }
                    }
                } else {
                    self.chat.push_system("Usage: /agent <code|web|academic|data>");
                }
            }
            "/provider" => {
                if let Some(name) = parts.get(1) {
                    self.switch_provider(name, &self.model_name.clone());
                } else {
                    self.chat.push_system(&format!("Current provider: {}", self.provider_name));
                }
            }
            "/model" => {
                if let Some(name) = parts.get(1) {
                    self.switch_provider(&self.provider_name.clone(), name);
                } else {
                    self.chat.push_system(&format!("Current model: {}", self.model_name));
                }
            }
            "/status" | "/s" => {
                let kowalski_status = if self.kowalski.is_some() { "enabled" } else { "disabled" };
                self.chat.push_system(&format!(
                    "Provider: {}\nModel: {}\nKowalski: {}\nAgents: {}",
                    self.provider_name,
                    self.model_name,
                    kowalski_status,
                    self.agents.agents.len()
                ));
            }
            "/quit" | "/q" | "/exit" => {
                self.quit = true;
            }
            _ => {
                self.chat.push_system(&format!("Unknown command: {}. Type /help for commands.", command));
            }
        }
        Ok(())
    }

    fn switch_provider(&mut self, provider: &str, model: &str) {
        self.provider_name = provider.to_string();
        self.model_name = model.to_string();

        // Resolve new key
        self.resolved_key = Self::resolve_api_key(provider, &self.config);

        // Create new provider
        self.provider = create_provider_with_keys_manager(provider, self.resolved_key.as_deref())
            .ok()
            .map(Arc::from);

        // Update popup
        self.keys_popup.set_current(provider, model);

        self.chat.push_system(&format!("Switched to {}/{}", provider, model));
    }

    fn send_to_llm(&mut self, content: &str) -> Result<()> {
        let Some(provider) = self.provider.clone() else {
            self.chat.push_system("No provider configured. Use /keys or Ctrl+K to configure.");
            return Ok(());
        };

        // Start streaming
        self.streaming.store(true, Ordering::Relaxed);
        self.chat.start_streaming(Role::Assistant);

        // Clear previous chunks
        if let Ok(mut chunks) = self.stream_chunks.lock() {
            chunks.clear();
        }

        let model = self.model_name.clone();
        let content = content.to_string();
        let result = self.stream_result.clone();
        let chunks = self.stream_chunks.clone();

        // Spawn task
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    if let Ok(mut r) = result.lock() {
                        *r = Some(Err(format!("Runtime error: {}", e)));
                    }
                    return;
                }
            };

            rt.block_on(async {
                match provider.chat_with_system(None, &content, &model, 0.7).await {
                    Ok(response) => {
                        // Add to chunks for display
                        if let Ok(mut c) = chunks.lock() {
                            c.push(response.clone());
                        }
                        if let Ok(mut r) = result.lock() {
                            *r = Some(Ok(response));
                        }
                    }
                    Err(e) => {
                        if let Ok(mut r) = result.lock() {
                            *r = Some(Err(format!("{}", e)));
                        }
                    }
                }
            });
        });

        Ok(())
    }

    fn send_to_kowalski(&mut self, agent_type: AgentType, content: &str) -> Result<()> {
        let Some(kowalski) = self.kowalski.clone() else {
            self.chat.push_system("Kowalski not available");
            return Ok(());
        };

        // Update agent status
        self.agents.set_agent_status(agent_type, AgentStatus::Busy);

        let agent_name = format!("kowalski-{}", agent_type.name());
        let content = content.to_string();

        self.chat.push_system(&format!("Sending to {}...", agent_type.display()));

        // Start streaming
        self.streaming.store(true, Ordering::Relaxed);
        self.chat.start_streaming(Role::Agent(agent_type));

        let result = self.stream_result.clone();
        let chunks = self.stream_chunks.clone();

        // Clear previous chunks
        if let Ok(mut c) = chunks.lock() {
            c.clear();
        }

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    if let Ok(mut r) = result.lock() {
                        *r = Some(Err(format!("Runtime error: {}", e)));
                    }
                    return;
                }
            };

            rt.block_on(async {
                match kowalski.send_task(&agent_name, &content).await {
                    Ok(task_result) => {
                        if task_result.success {
                            if let Ok(mut c) = chunks.lock() {
                                c.push(task_result.output.clone());
                            }
                            if let Ok(mut r) = result.lock() {
                                *r = Some(Ok(task_result.output));
                            }
                        } else {
                            let error = task_result.error.unwrap_or_else(|| "Unknown error".to_string());
                            if let Ok(mut r) = result.lock() {
                                *r = Some(Err(error));
                            }
                        }
                    }
                    Err(e) => {
                        if let Ok(mut r) = result.lock() {
                            *r = Some(Err(format!("{}", e)));
                        }
                    }
                }
            });
        });

        Ok(())
    }
}

impl Default for MinimalApp {
    fn default() -> Self {
        let config = Config::load_or_init().unwrap_or_default();
        let provider = config.default_provider.clone().unwrap_or_else(|| "openrouter".to_string());
        let model = config.default_model.clone().unwrap_or_else(|| "auto".to_string());
        Self::new(config, provider, model)
    }
}

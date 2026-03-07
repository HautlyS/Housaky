use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Tabs, Wrap},
    Frame,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Handle;

use crate::config::Config;
use crate::housaky::core::HousakyCore;
use crate::housaky::heartbeat::HousakyHeartbeat;
use crate::providers::ChatMessage;
use std::sync::atomic::{AtomicBool, Ordering};

// Animation frames for header sparkle (all same visual width)
const ANIM_FRAMES: &[&str] = &[
    "◉", "◉", "◉", "◉", "◉", "◉", "◉", "◉",
];

use super::chat_pane::ChatPane;
use super::command_palette::{CommandPalette, PaletteAction};
use super::config_editor::ConfigEditor;
use super::doctor_panel::DoctorPanel;
use super::help::HelpOverlay;
use super::input::InputBar;
use super::layout::{BodyZones, HeaderZones, RootZones};

#[derive(Debug, Default, Clone)]
struct UiHitMap {
    term_area: Option<Rect>,
    header: Option<Rect>,
    tabs_area: Option<Rect>,
    tab_hitboxes: Vec<Rect>,
    body: Option<Rect>,
    body_main: Option<Rect>,
    body_sidebar: Option<Rect>,
    input: Option<Rect>,
    footer: Option<Rect>,

    // Two-column layouts for list/detail tabs
    skills_cols: Option<[Rect; 2]>,
    tools_cols: Option<[Rect; 2]>,
    logs_cols: Option<[Rect; 2]>,

    // Item hitboxes (display indices)
    skills_list_items: Vec<Rect>,
    tools_list_items: Vec<Rect>,
    palette_items: Vec<Rect>,

    // Sidebar sections
    sidebar_metrics: Option<Rect>,
    sidebar_goals: Option<Rect>,
    sidebar_activity: Option<Rect>,

    // Chat pane hitboxes
    chat_area: Option<Rect>,
    chat_viewport: Option<Rect>,
    chat_scrollbar: Option<Rect>,
    chat_header_items: Vec<(Rect, usize)>, // rect + message index

    // Config editor hitboxes
    config_cols: Option<[Rect; 2]>,
    config_section_items: Vec<Rect>,
    config_field_items: Vec<Rect>,

    // Overlays
    help: Option<Rect>,
    palette: Option<Rect>,
}
use super::notifications::Notifications;
use super::sidebar::{ActivityKind, Sidebar, SidebarGoal};
use super::skills_panel::SkillsPanel;
use super::state::{ActivePane, AppState, InputMode, MainTab, StreamStatus};
use super::theme::{
    style_border, style_dim, style_muted, style_tab_active, style_tab_inactive, style_title,
    truncate_str, Palette, VERSION,
};
use super::tools_panel::ToolsPanel;

// ── Tab transition animation ───────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct TabTransition {
    from_tab: MainTab,
    to_tab: MainTab,
    direction: TabAnimDirection,
}

#[derive(Clone, Copy)]
enum TabAnimDirection {
    LeftToRight,
    RightToLeft,
}

// ── EnhancedApp ───────────────────────────────────────────────────────────────

pub struct EnhancedApp {
    // Mouse click tracking for double-click detection
    last_click: Option<(u16, u16, std::time::Instant)>,
    // Scrollbar drag state
    dragging_chat_scrollbar: bool,
    // Config & provider
    config: Config,
    provider_name: String,
    model_name: String,

    // Resolved API key (loaded once at init for the send thread)
    resolved_api_key: Option<String>,

    // AGI core reference (live metrics, goals, thoughts)
    core: Option<Arc<HousakyCore>>,

    // System prompt (rebuilt when skills change)
    system_prompt: String,
    heartbeat: Option<Arc<HousakyHeartbeat>>,

    // Core state
    state: AppState,

    // Panels
    chat: ChatPane,
    input: InputBar,
    sidebar: Sidebar,
    skills: SkillsPanel,
    tools: ToolsPanel,
    cfg_editor: ConfigEditor,
    doctor: DoctorPanel,
    help: HelpOverlay,
    palette: CommandPalette,
    notifs: Notifications,

    // Logs tab
    log_entries: Vec<String>,
    logs_scroll: usize,

    // TUI output channel receiver (drains tui_println!/tracing lines each frame)
    log_rx: Option<std::sync::mpsc::Receiver<String>>,

    // Streaming
    streaming_active: Arc<AtomicBool>,
    streaming_result: Arc<std::sync::Mutex<Option<Result<String, String>>>>,
    streaming_chunks: Arc<std::sync::Mutex<Vec<String>>>,

    // Animation
    anim_frame: usize,
    anim_tick: usize,

    // Tab transition animation
    tab_transition: Option<TabTransition>,
    tab_anim_frame: usize,

    // Per-frame UI hit-testing map for mouse interactions
    hitmap: UiHitMap,
}

impl EnhancedApp {
    pub fn new(config: Config, provider_name: String, model_name: String) -> Self {
        let mut skills = SkillsPanel::new();
        // Try to load skills from workspace
        let skills_dir = config.workspace_dir.join("skills");
        let config_skills: Vec<(String, bool)> = config
            .skills
            .enabled
            .iter()
            .map(|(name, &en)| (name.clone(), en))
            .collect();
        skills.load_from_paths(&skills_dir, &config_skills);
        skills.load_marketplace_skills(&config.workspace_dir, &config);

        let mut state = AppState::new();
        state.metrics.skills_enabled = skills.skills.iter().filter(|s| s.enabled).count();

        let cfg_editor = ConfigEditor::new(&config);

        // Resolve API key eagerly so the send thread can use it without a tokio context.
        // Priority: keys_manager → config.api_key → env vars
        let resolved_api_key = Self::resolve_api_key_sync(&provider_name, &config);

        // Build initial system prompt with active skills
        let system_prompt = Self::build_system_prompt(&config, &skills, &model_name);

        Self {
            config,
            provider_name,
            model_name,
            resolved_api_key,
            core: None,
            heartbeat: None,
            system_prompt,
            state,
            chat: ChatPane::new(),
            input: InputBar::new(),
            sidebar: Sidebar::new(),
            skills,
            tools: ToolsPanel::new(),
            cfg_editor,
            help: HelpOverlay::new(),
            doctor: DoctorPanel::new(),
            palette: CommandPalette::new(),
            notifs: Notifications::new(),
            log_entries: Vec::new(),
            logs_scroll: 0,
            log_rx: None,
            streaming_active: Arc::new(AtomicBool::new(false)),
            streaming_result: Arc::new(std::sync::Mutex::new(None)),
            streaming_chunks: Arc::new(std::sync::Mutex::new(Vec::new())),
            anim_frame: 0,
            anim_tick: 0,
            tab_transition: None,
            tab_anim_frame: 0,
            hitmap: UiHitMap::default(),
            last_click: None,
            dragging_chat_scrollbar: false,
        }
    }

    /// Build system prompt with skills and configuration
    fn build_system_prompt(config: &Config, skills: &SkillsPanel, model_name: &str) -> String {
        use std::fmt::Write;
        let mut prompt = String::with_capacity(4096);

        // ── Core Identity ─────────────────────────────────────────────
        prompt.push_str("You are Housaky, an autonomous AI assistant built in Rust. ");
        prompt.push_str("You have persistent memory, goal tracking, a knowledge graph, ");
        prompt.push_str("and a cognitive loop with perception, reasoning, and reflection.\n");
        prompt.push_str("Be helpful, concise, and direct. Use your tools proactively.\n\n");

        // ── Tools ─────────────────────────────────────────────────────
        prompt.push_str("## Tools\n\n");
        prompt.push_str("You have access to the following tools:\n\n");
        prompt.push_str("- **shell**: Execute terminal commands\n");
        prompt.push_str("- **file_read**: Read file contents\n");
        prompt.push_str("- **file_write**: Write file contents\n");
        prompt.push_str("- **file_list**: List directory contents\n");
        prompt.push_str("- **file_search**: Search for files by pattern\n");
        prompt.push_str("- **file_move**: Move/rename files\n");
        prompt.push_str("- **file_delete**: Delete files safely\n");
        prompt.push_str("- **file_info**: Get file metadata\n");
        prompt.push_str("- **memory_store**: Save to persistent memory\n");
        prompt.push_str("- **memory_recall**: Search persistent memory\n");
        prompt.push_str("- **memory_forget**: Delete a memory entry\n");
        prompt.push_str("- **schedule**: Schedule tasks for future execution\n");
        prompt.push_str("- **git_operations**: Git operations (status, diff, commit, etc.)\n");
        if config.browser.enabled {
            prompt.push_str("- **browser_open**: Open approved HTTPS URLs\n");
            prompt.push_str("- **browser**: Full browser automation (navigate, click, type, screenshot)\n");
        }
        if config.http_request.enabled {
            prompt.push_str("- **http_request**: Make HTTP requests to allowed domains\n");
        }
        prompt.push_str("\n## Tool Use Protocol\n\n");
        prompt.push_str("To use a tool, wrap a JSON object in <tool_call></tool_call> tags:\n\n");
        prompt.push_str("```\n<tool_call>\n{\"name\": \"tool_name\", \"arguments\": {\"param\": \"value\"}}\n</tool_call>\n```\n\n");
        prompt.push_str("Use memory_store to persist important facts and decisions across sessions.\n");
        prompt.push_str("Use memory_recall before answering to check if relevant context exists.\n\n");

        // ── Safety ────────────────────────────────────────────────────
        prompt.push_str("## Safety\n\n");
        prompt.push_str("- Do not exfiltrate private data.\n");
        prompt.push_str("- Do not run destructive commands without asking.\n");
        prompt.push_str("- Prefer `trash` over `rm` (recoverable beats gone forever).\n\n");

        // ── Active Skills ─────────────────────────────────────────────
        let active_skills: Vec<_> = skills.skills.iter().filter(|s| s.enabled).collect();
        if !active_skills.is_empty() {
            prompt.push_str("## Active Skills\n\n");
            for skill in active_skills {
                let _ = writeln!(prompt, "### {} (v{})", skill.name, skill.version);
                let _ = writeln!(prompt, "{}", skill.description);
                if let Some(ref path) = skill.path {
                    let _ = writeln!(prompt, "Location: {}", path.display());
                }
                prompt.push('\n');
            }
        }

        // ── Workspace ─────────────────────────────────────────────────
        let _ = writeln!(prompt, "## Workspace\n\nWorking directory: `{}`", config.workspace_dir.display());

        // ── Runtime ───────────────────────────────────────────────────
        let host = hostname::get().map_or_else(|_| "unknown".into(), |h| h.to_string_lossy().to_string());
        let _ = writeln!(prompt, "\n## Runtime\n\nHost: {host} | Model: {model_name}");

        prompt
    }

    /// Rebuild system prompt when skills change
    fn rebuild_system_prompt(&mut self) {
        self.system_prompt = Self::build_system_prompt(&self.config, &self.skills, &self.model_name);
        crate::tui_println!("System prompt rebuilt with {} active skills", self.skills.skills.iter().filter(|s| s.enabled).count());
    }

    /// Resolve API key synchronously (no async) for use before spawning background threads.
    fn resolve_api_key_sync(provider_name: &str, config: &Config) -> Option<String> {
        // 1. Try keys_manager (non-blocking read of the in-memory store)
        {
            let manager = crate::keys_manager::manager::get_global_keys_manager();
            let lock_result = manager.store.try_read();
            if let Ok(store) = lock_result {
                if let Some(provider_store) = store.providers.get(provider_name) {
                    if let Some(key) = provider_store.keys.iter().find(|k| k.enabled) {
                        return Some(key.key.clone());
                    }
                }
                // Also try any provider
                for provider in store.providers.values() {
                    if let Some(key) = provider.keys.iter().find(|k| k.enabled) {
                        return Some(key.key.clone());
                    }
                }
            }
        }
        // 2. Try config.api_key
        if let Some(ref k) = config.api_key {
            if !k.is_empty() {
                return Some(k.clone());
            }
        }
        // 3. Try common env vars
        let env_candidates: &[&str] = match provider_name {
            "anthropic" => &["ANTHROPIC_API_KEY", "ANTHROPIC_OAUTH_TOKEN"],
            "openrouter" => &["OPENROUTER_API_KEY"],
            "openai" => &["OPENAI_API_KEY"],
            "gemini" | "google" => &["GEMINI_API_KEY", "GOOGLE_API_KEY"],
            "groq" => &["GROQ_API_KEY"],
            "mistral" => &["MISTRAL_API_KEY"],
            "deepseek" => &["DEEPSEEK_API_KEY"],
            _ => &[],
        };
        for env_var in env_candidates {
            if let Ok(val) = std::env::var(env_var) {
                let val = val.trim().to_string();
                if !val.is_empty() {
                    return Some(val);
                }
            }
        }
        for env_var in ["HOUSAKY_API_KEY", "API_KEY"] {
            if let Ok(val) = std::env::var(env_var) {
                let val = val.trim().to_string();
                if !val.is_empty() {
                    return Some(val);
                }
            }
        }
        None
    }

    /// Attach the TUI output receiver so log lines show in the chat pane.
    pub fn set_log_receiver(&mut self, rx: std::sync::mpsc::Receiver<String>) {
        self.log_rx = Some(rx);
    }

    /// Attach a live heartbeat (and its AGI core) for real-time dashboard data.
    pub fn with_heartbeat(mut self, heartbeat: Arc<HousakyHeartbeat>) -> Self {
        self.core = Some(Arc::clone(heartbeat.core()));
        self.heartbeat = Some(heartbeat);
        self
    }

    // ── Public interface (called by tui/mod.rs loop) ──────────────────────────

    pub fn should_quit(&self) -> bool {
        self.state.should_quit
    }

    pub fn update(&mut self) {
        self.state.tick();
        self.notifs.tick();
        self.skills.tick();

        // Sync data from HousakyCore if available
        self.sync_from_core();

        // Drain TUI output channel — route log lines to both Logs tab AND Chat pane as SYS messages
        if let Some(ref rx) = self.log_rx {
            while let Ok(line) = rx.try_recv() {
                self.log_entries.push(line.clone());
                self.chat.push_system(line);
            }
        }

        // Advance ASCII animation (changes every 6 ticks ≈ 200ms at 30fps)
        self.anim_tick += 1;
        if self.anim_tick >= 6 {
            self.anim_tick = 0;
            self.anim_frame = (self.anim_frame + 1) % ANIM_FRAMES.len();
        }

        // Advance tab transition animation
        if let Some(ref mut _trans) = self.tab_transition {
            self.tab_anim_frame += 1;
            if self.tab_anim_frame >= 8 {
                self.tab_transition = None;
                self.tab_anim_frame = 0;
            }
        }

        // Process streaming results
        if self.streaming_active.load(Ordering::SeqCst) {
            // Drain chunks atomically so we don't hold the lock during draw
            let new_chunks: Vec<String> = if let Ok(mut chunks) = self.streaming_chunks.lock() {
                std::mem::take(&mut *chunks)
            } else {
                Vec::new()
            };
            for chunk in &new_chunks {
                self.chat.append_stream_chunk(chunk);
                self.state.stream_content.push_str(chunk);

                // Propagate AGI phase indicators to sidebar activity in real-time
                let trimmed = chunk.trim();
                if trimmed.starts_with("🔍") {
                    self.sidebar.push_activity(ActivityKind::Thought, "Perceiving intent...");
                } else if trimmed.starts_with("📚") {
                    self.sidebar.push_activity(ActivityKind::Thought, trimmed.replace('*', ""));
                } else if trimmed.starts_with("🎯") {
                    self.sidebar.push_activity(ActivityKind::Goal, trimmed.replace('*', ""));
                } else if trimmed.starts_with("🧠") {
                    self.sidebar.push_activity(ActivityKind::Thought, "Reasoning...");
                    self.state.stream_status = StreamStatus::Streaming;
                } else if trimmed.starts_with("⚡") {
                    self.sidebar.push_activity(ActivityKind::Tool, "Executing actions...");
                } else if trimmed.starts_with("💭") {
                    self.sidebar.push_activity(ActivityKind::Thought, "Inner monologue active");
                }
            }

            // Check for completion
            let result_opt: Option<Result<String, String>> =
                if let Ok(mut result_guard) = self.streaming_result.lock() {
                    result_guard.take()
                } else {
                    None
                };

            if let Some(result) = result_opt {
                match result {
                    Ok(response) => {
                        self.chat.finish_streaming(None);
                        let token_est = (response.len() / 4) as u32;
                        self.state.metrics.total_tokens_out += u64::from(token_est);
                        self.state.stream_status = StreamStatus::Done;
                        self.sidebar.push_activity(
                            ActivityKind::Thought,
                            format!("AGI: {}", truncate_str(&response, 40)),
                        );
                        self.state.metrics.total_messages += 1;
                        self.state.stream_content.clear();
                        self.notifs.success("Response received");
                    }
                    Err(e) => {
                        self.chat.finish_streaming(None);
                        let err_msg = format!("⚠ {}", e);
                        self.chat.push_system(err_msg.clone());
                        self.state.stream_status = StreamStatus::Error(e.clone());
                        self.state.metrics.total_errors += 1;
                        self.notifs.error(truncate_str(&err_msg, 60).to_string());
                        self.state.stream_content.clear();
                    }
                }
                self.streaming_active.store(false, Ordering::SeqCst);
            }
        }

        // Sync sidebar activity into the logs tab (avoid duplicates by
        // only appending new entries since last sync).
        let already_logged = self.log_entries.len();
        let total_activity = self.sidebar.activity.len();
        if total_activity > already_logged {
            for a in &self.sidebar.activity[already_logged..] {
                self.log_entries
                    .push(format!("[{}] {} {}", a.time, a.kind.icon(), a.message,));
            }
        }

        // Cap log buffer to avoid unbounded growth.
        const MAX_LOG_ENTRIES: usize = 2000;
        if self.log_entries.len() > MAX_LOG_ENTRIES {
            let excess = self.log_entries.len() - MAX_LOG_ENTRIES;
            self.log_entries.drain(..excess);
        }
    }

    /// Append a line to the logs tab programmatically.
    pub fn push_log(&mut self, entry: String) {
        self.log_entries.push(entry);
    }

    /// Sync data from HousakyCore if available (called periodically in update)
    pub fn sync_from_core(&mut self) {
        let core = match self.core {
            Some(ref c) => Arc::clone(c),
            None => return,
        };
        let rt = match Handle::try_current() {
            Ok(rt) => rt,
            Err(_) => return,
        };

        // Fetch data from core (async) — use block_in_place to avoid
        // "cannot block_on inside a runtime" panic.
        let goals = tokio::task::block_in_place(|| rt.block_on(core.get_goals_for_tui()));
        let tools = core.get_tools_for_tui();
        let agent_skills = core.get_skills_for_tui();

        // Apply goals
        let existing_goals: std::collections::HashSet<_> = self.sidebar.goals.iter().map(|g| g.title.clone()).collect();
        for goal in goals {
            if !existing_goals.contains(&goal.title) {
                let priority = match goal.priority {
                    crate::housaky::goal_engine::GoalPriority::Critical => super::sidebar::GoalPriority::Critical,
                    crate::housaky::goal_engine::GoalPriority::High => super::sidebar::GoalPriority::High,
                    crate::housaky::goal_engine::GoalPriority::Medium => super::sidebar::GoalPriority::Medium,
                    crate::housaky::goal_engine::GoalPriority::Low => super::sidebar::GoalPriority::Low,
                    _ => super::sidebar::GoalPriority::Low,
                };
                self.sidebar.goals.push(super::sidebar::SidebarGoal {
                    title: goal.title,
                    progress: goal.progress,
                    priority,
                });
            }
        }
        self.state.metrics.goals_active = self.sidebar.goals.len();

        // Apply tools
        for (name, desc) in tools {
            if !self.tools.tool_usage.contains_key(&name) {
                self.tools.tool_usage.insert(name.clone(), super::tools_panel::TrackedTool {
                    name,
                    description: desc,
                    execution_count: 0,
                    last_used: None,
                    status: super::state::ToolStatus::Success,
                });
            }
        }

        // Apply skills
        let existing_skills: std::collections::HashSet<_> = self.skills.skills.iter().map(|s| s.name.clone()).collect();
        for skill in agent_skills {
            if !existing_skills.contains(&skill.name) {
                self.skills.skills.push(super::skills_panel::Skill {
                    name: skill.name,
                    description: skill.description,
                    enabled: true,
                    installed: true,
                    source: super::skills_panel::SkillSource::Config,
                    version: skill.version,
                    tags: skill.tags,
                    path: None,
                    readme: None,
                });
            }
        }
        self.state.metrics.skills_enabled = self.skills.skills.iter().filter(|s| s.enabled).count();

        // Drain heartbeat activity entries and push to sidebar
        let activities = core.drain_activities();
        for (kind, message) in activities {
            let activity_kind = match kind.as_str() {
                "tool" => ActivityKind::Tool,
                "thought" => ActivityKind::Thought,
                "goal" => ActivityKind::Goal,
                "skill" => ActivityKind::Skill,
                _ => ActivityKind::System,
            };
            self.sidebar.push_activity(activity_kind, message);
        }
    }

    /// Show an error notification from outside the app (e.g. event-loop error recovery).
    pub fn notifs_error(&mut self, msg: &str) {
        self.notifs.error(msg.to_string());
    }

    fn switch_tab(&mut self, tab: MainTab) {
        let from_tab = self.state.active_tab;
        let direction = if tab.index() > from_tab.index() {
            TabAnimDirection::RightToLeft
        } else {
            TabAnimDirection::LeftToRight
        };
        self.tab_transition = Some(TabTransition {
            from_tab,
            to_tab: tab,
            direction,
        });
        self.tab_anim_frame = 0;
        self.state.push_nav(from_tab);
        self.state.active_tab = tab;
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Global overrides first
        match (key.modifiers, key.code) {
            // Ctrl+C always quits
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.state.should_quit = true;
                return Ok(());
            }
            // Ctrl+P opens command palette from anywhere
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                if self.palette.active {
                    self.palette.close();
                } else {
                    self.palette.open();
                    self.state.show_command_palette = true;
                }
                return Ok(());
            }
            // Shift+Tab - always goes to previous main tab (works everywhere)
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                // Only switch main tab if NOT in config section editing
                if self.state.active_tab != MainTab::Config || !self.cfg_editor.is_editing() {
                    self.switch_tab(self.state.active_tab.prev());
                    return Ok(());
                }
            }
            // Tab - switch to next main tab, EXCEPT when Config tab handles it internally
            (KeyModifiers::NONE, KeyCode::Tab) => {
                // Config tab intercepts Tab for section navigation
                if self.state.active_tab == MainTab::Config && !self.help.visible && !self.palette.active {
                    // fall through to tab-specific handler below
                } else if !self.help.visible && !self.palette.active
                    && !self.state.input_mode.is_typing()
                {
                    self.switch_tab(self.state.active_tab.next());
                    return Ok(());
                }
            }
            _ => {}
        }

        // Route to overlay handlers first
        if self.help.visible {
            return self.handle_help_key(key);
        }
        if self.palette.active {
            return self.handle_palette_key(key);
        }

        // Route to active-tab handlers
        match self.state.active_tab {
            MainTab::Chat => self.handle_chat_key(key),
            MainTab::Skills => self.handle_skills_key(key),
            MainTab::Tools => self.handle_tools_key(key),
            MainTab::Goals => self.handle_goals_key(key),
            MainTab::Metrics => self.handle_metrics_key(key),
            MainTab::Logs => self.handle_logs_key(key),
            MainTab::Config => self.handle_config_key(key),
            MainTab::Doctor => self.handle_doctor_key(key),
        }
    }

    fn handle_chat_scrollbar_drag(&mut self, _col: u16, row: u16) {
        let Some(sb) = self.hitmap.chat_scrollbar else { return; };
        let total = self.chat.messages_len() + if self.chat.is_streaming() { 1 } else { 0 };
        if total == 0 {
            return;
        }
        let rel = f64::from(row.saturating_sub(sb.y));
        let h = f64::from(sb.height.max(1));
        let ratio = (rel / h).clamp(0.0, 1.0);
        let target = (ratio * (total.saturating_sub(1) as f64)).round() as usize;
        self.chat.scroll_to_index(target);
    }

    fn handle_mouse_scroll(&mut self, direction: i32, col: u16, row: u16) {
        // direction: -1 = up, +1 = down
        let pos = ratatui::layout::Position::new(col, row);

        let zones = match (self.hitmap.header, self.hitmap.body, self.hitmap.input, self.hitmap.footer) {
            (Some(header), Some(body), Some(input), Some(footer)) => RootZones { header, body, input, footer },
            _ => {
                let (term_w, term_h) = crossterm::terminal::size().unwrap_or((120, 40));
                RootZones::compute(Rect::new(0, 0, term_w, term_h))
            }
        };

        // If wheel is over the header, change tabs.
        if zones.header.contains(pos) {
            if direction < 0 {
                self.switch_tab(self.state.active_tab.prev());
            } else {
                self.switch_tab(self.state.active_tab.next());
            }
            return;
        }

        // If wheel is over the input row, don't scroll content.
        if zones.input.contains(pos) {
            return;
        }

        // If wheel is over chat scrollbar, do page scroll
        if let Some(sb) = self.hitmap.chat_scrollbar {
            if sb.contains(pos) {
                if direction < 0 {
                    self.chat.scroll_up(12);
                } else {
                    self.chat.scroll_down(12);
                }
                return;
            }
        }

        // Sidebar-specific scroll
        if let Some(r) = self.hitmap.sidebar_activity {
            if r.contains(pos) {
                if direction < 0 {
                    self.sidebar.scroll_up();
                } else {
                    self.sidebar.scroll_down();
                }
                return;
            }
        }
        if let Some(r) = self.hitmap.sidebar_goals {
            if r.contains(pos) {
                if direction < 0 {
                    self.sidebar.goal_scroll = self.sidebar.goal_scroll.saturating_sub(1);
                } else {
                    self.sidebar.goal_scroll += 1;
                }
                return;
            }
        }

        // Overlay scroll
        if self.help.visible {
            if direction < 0 {
                self.help.scroll_up();
            } else {
                self.help.scroll_down();
            }
            return;
        }

        // Main tab scroll routing
        match self.state.active_tab {
            MainTab::Chat => {
                if direction < 0 {
                    self.chat.scroll_up(3);
                } else {
                    self.chat.scroll_down(3);
                }
            }
            MainTab::Logs => {
                if direction < 0 {
                    self.logs_scroll = self.logs_scroll.saturating_add(1);
                } else {
                    self.logs_scroll = self.logs_scroll.saturating_sub(1);
                }
            }
            MainTab::Skills => {
                // If wheel is over the left list, move selection; if over the right detail, scroll detail.
                let cols = self
                    .hitmap
                    .skills_cols
                    .or_else(|| {
                        let bz = BodyZones::compute(zones.body, self.state.view_mode, self.state.sidebar_visible);
                        let c = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                            .split(bz.main);
                        Some([c[0], c[1]])
                    })
                    .unwrap();
                if cols[0].contains(pos) {
                    if direction < 0 {
                        self.skills.select_prev();
                    } else {
                        self.skills.select_next();
                    }
                } else {
                    if direction < 0 {
                        self.skills.detail_scroll_up();
                    } else {
                        self.skills.detail_scroll_down();
                    }
                }
            }
            MainTab::Tools => {
                // If wheel is over the left list, move selection; if over the right detail, scroll detail.
                let cols = self
                    .hitmap
                    .tools_cols
                    .or_else(|| {
                        let bz = BodyZones::compute(zones.body, self.state.view_mode, self.state.sidebar_visible);
                        let c = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
                            .split(bz.main);
                        Some([c[0], c[1]])
                    })
                    .unwrap();
                if cols[0].contains(pos) {
                    if direction < 0 {
                        self.tools.selected = self.tools.selected.saturating_sub(1);
                    } else {
                        self.tools.selected = (self.tools.selected + 1).min(self.state.tool_log.len().saturating_sub(1));
                    }
                } else {
                    if direction < 0 {
                        self.tools.detail_scroll_up();
                    } else {
                        self.tools.detail_scroll_down();
                    }
                }
            }
            MainTab::Config => {
                // Wheel scroll navigates fields when not editing.
                if !self.cfg_editor.is_editing() {
                    if direction < 0 {
                        self.cfg_editor.field_up();
                    } else {
                        self.cfg_editor.field_down();
                    }
                }
            }
            MainTab::Doctor => {
                // Left column: navigate checks, right column: scroll detail
                let bz = BodyZones::compute(zones.body, self.state.view_mode, self.state.sidebar_visible);
                let mid_x = bz.main.x + bz.main.width / 2;
                if col < mid_x {
                    if direction < 0 {
                        self.doctor.select_prev();
                    } else {
                        self.doctor.select_next();
                    }
                } else {
                    if direction < 0 {
                        self.doctor.detail_scroll_up();
                    } else {
                        self.doctor.detail_scroll_down();
                    }
                }
            }
            MainTab::Goals | MainTab::Metrics => {
                // no-op for now
            }
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        use crossterm::event::MouseEventKind;

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.handle_mouse_scroll(-1, mouse.column, mouse.row);
                return Ok(());
            }
            MouseEventKind::ScrollDown => {
                self.handle_mouse_scroll(1, mouse.column, mouse.row);
                return Ok(());
            }
            MouseEventKind::ScrollLeft => {
                // Horizontal scroll maps to tab navigation
                self.switch_tab(self.state.active_tab.prev());
                return Ok(());
            }
            MouseEventKind::ScrollRight => {
                self.switch_tab(self.state.active_tab.next());
                return Ok(());
            }
            MouseEventKind::Down(_) => {}
            MouseEventKind::Drag(_) => {
                if self.dragging_chat_scrollbar {
                    self.handle_chat_scrollbar_drag(mouse.column, mouse.row);
                }
                return Ok(());
            }
            MouseEventKind::Up(_) => {
                self.dragging_chat_scrollbar = false;
                return Ok(());
            }
            _ => return Ok(()),
        }

        let pos = ratatui::layout::Position::new(mouse.column, mouse.row);

        let zones = match (self.hitmap.header, self.hitmap.body, self.hitmap.input, self.hitmap.footer) {
            (Some(header), Some(body), Some(input), Some(footer)) => RootZones { header, body, input, footer },
            _ => {
                // Fallback: compute from current terminal size
                let (term_w, term_h) = crossterm::terminal::size().unwrap_or((120, 40));
                RootZones::compute(Rect::new(0, 0, term_w, term_h))
            }
        };

        // Tabs: if click in tab bar, use computed hitboxes
        if let Some(tabs_area) = self.hitmap.tabs_area {
            if tabs_area.contains(pos) {
                for (idx, rect) in self.hitmap.tab_hitboxes.iter().enumerate() {
                    if rect.contains(pos) {
                        if let Some(&tab) = MainTab::ALL.get(idx) {
                            self.switch_tab(tab);
                        }
                        break;
                    }
                }
                return Ok(());
            }
        }

        // Double click detection (same cell within 350ms)
        let is_double_click = self
            .last_click
            .as_ref()
            .is_some_and(|(c, r, t)| {
                *c == mouse.column && *r == mouse.row && t.elapsed() < Duration::from_millis(350)
            });
        self.last_click = Some((mouse.column, mouse.row, std::time::Instant::now()));

        // Overlays get first refusal on clicks
        if self.palette.active {
            if let Some(popup) = self.hitmap.palette {
                if popup.contains(pos) {
                    // Click on result list item executes it
                    for (di, r) in self.hitmap.palette_items.iter().enumerate() {
                        if r.contains(pos) {
                            self.palette.set_selected(di);
                            if let Some(action) = self.palette.execute() {
                                self.state.show_command_palette = false;
                                self.execute_palette_action(action)?;
                            }
                            return Ok(());
                        }
                    }
                    return Ok(());
                }
            }
            // click outside closes
            self.palette.close();
            self.state.show_command_palette = false;
            return Ok(());
        }

        if self.help.visible {
            if let Some(popup) = self.hitmap.help {
                if popup.contains(pos) {
                    // click inside does nothing (wheel scroll supported)
                    return Ok(());
                }
            }
            self.help.hide();
            return Ok(());
        }

        // Handle panel-specific clicks
        match self.state.active_tab {
            MainTab::Chat => {
                if zones.input.contains(pos) {
                    self.state.active_pane = ActivePane::Input;
                    self.state.input_mode = InputMode::Insert;
                    return Ok(());
                }

                if let Some(sb) = self.hitmap.body_sidebar {
                    if sb.contains(pos) {
                        self.state.active_pane = ActivePane::Sidebar;
                        return Ok(());
                    }
                }

                // Click message header
                for (r, mi) in &self.hitmap.chat_header_items {
                    if r.contains(pos) {
                        self.state.active_pane = ActivePane::Chat;
                        if is_double_click {
                            if let Some(msg) = self.chat.get_message(*mi) {
                                self.notifs.info(format!(
                                    "Copied message (simulated): {}",
                                    truncate_str(&msg.content, 40)
                                ));
                            }
                        } else if let Some(msg) = self.chat.get_message(*mi) {
                            self.notifs.info(format!(
                                "{} @ {}",
                                msg.role.label(),
                                msg.timestamp
                            ));
                        }
                        return Ok(());
                    }
                }

                // Click in chat viewport focuses chat
                if self.hitmap.chat_viewport.is_some_and(|r| r.contains(pos)) {
                    self.state.active_pane = ActivePane::Chat;
                    return Ok(());
                }

                // Click scrollbar jumps scroll position + enables dragging
                if let Some(sb) = self.hitmap.chat_scrollbar {
                    if sb.contains(pos) {
                        self.dragging_chat_scrollbar = true;
                        self.handle_chat_scrollbar_drag(mouse.column, mouse.row);
                        self.state.active_pane = ActivePane::Chat;
                        return Ok(());
                    }
                }

                if zones.body.contains(pos) {
                    self.state.active_pane = ActivePane::Chat;
                }
            }
            MainTab::Skills => {
                self.state.active_pane = ActivePane::SkillsPanel;
                if let Some([list, detail]) = self.hitmap.skills_cols {
                    if list.contains(pos) {
                        for (di, r) in self.hitmap.skills_list_items.iter().enumerate() {
                            if r.contains(pos) {
                                self.skills.set_selected(di);
                                break;
                            }
                        }
                    } else if detail.contains(pos) {
                        // Focus detail (wheel scrolls)
                    }
                }
            }
            MainTab::Tools => {
                if let Some([list, detail]) = self.hitmap.tools_cols {
                    if list.contains(pos) {
                        self.state.active_pane = ActivePane::ToolsPanel;
                        for (di, r) in self.hitmap.tools_list_items.iter().enumerate() {
                            if r.contains(pos) {
                                self.tools.set_selected(di, &self.state.tool_log);
                                break;
                            }
                        }
                    } else if detail.contains(pos) {
                        self.state.active_pane = ActivePane::ToolsPanel;
                    }
                }
            }
            MainTab::Goals => {}
            MainTab::Metrics => {}
            MainTab::Logs => {
                if let Some([left, _right]) = self.hitmap.logs_cols {
                    if left.contains(pos) {
                        self.state.active_pane = ActivePane::Chat;
                        // Click sets scroll position roughly
                        self.logs_scroll = 0;
                    }
                }
            }
            MainTab::Config => {
                if let Some([sec, fields]) = self.hitmap.config_cols {
                    if sec.contains(pos) {
                        for (i, r) in self.hitmap.config_section_items.iter().enumerate() {
                            if r.contains(pos) {
                                self.cfg_editor.set_section_idx(i, &self.config);
                                self.notifs.info("Section selected");
                                break;
                            }
                        }
                    } else if fields.contains(pos) {
                        for (i, r) in self.hitmap.config_field_items.iter().enumerate() {
                            if r.contains(pos) {
                                self.cfg_editor.set_field_idx(i);
                                if is_double_click {
                                    self.cfg_editor.start_edit();
                                    self.notifs.info("Editing…");
                                }
                                break;
                            }
                        }
                    }
                }
            }
            MainTab::Doctor => {
                // Click on left half selects checks, right half focuses detail
                if let Some(main) = self.hitmap.body_main {
                    if main.contains(pos) {
                        let mid_x = main.x + main.width / 2;
                        if mouse.column < mid_x {
                            // Approximate which row was clicked relative to the list area
                            let list_top = main.y + 2; // account for block border + summary
                            if mouse.row >= list_top {
                                let clicked_row = (mouse.row - list_top) as usize;
                                self.doctor.set_selected(clicked_row);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // ── Draw entry point ──────────────────────────────────────────────────────

    pub fn draw(&mut self, f: &mut Frame) {
        let area = f.area();
        let zones = RootZones::compute(area);

        // Reset per-frame hit targets
        self.hitmap.term_area = Some(area);
        self.hitmap.skills_list_items.clear();
        self.hitmap.tools_list_items.clear();
        self.hitmap.palette_items.clear();
        self.hitmap.sidebar_metrics = None;
        self.hitmap.chat_area = None;
        self.hitmap.chat_viewport = None;
        self.hitmap.chat_scrollbar = None;
        self.hitmap.chat_header_items.clear();
         self.hitmap.config_cols = None;
        self.hitmap.config_section_items.clear();
        self.hitmap.config_field_items.clear();
        self.hitmap.sidebar_goals = None;
        self.hitmap.sidebar_activity = None;
        self.hitmap.help = None;
        self.hitmap.palette = None;

        // Populate base hitmap zones for this frame
        self.hitmap.header = Some(zones.header);
        self.hitmap.body = Some(zones.body);
        self.hitmap.input = Some(zones.input);
        self.hitmap.footer = Some(zones.footer);

        let bz = BodyZones::compute(zones.body, self.state.view_mode, self.state.sidebar_visible);
        self.hitmap.body_main = Some(bz.main);
        self.hitmap.body_sidebar = bz.sidebar;

        self.draw_header(f, zones.header);
        self.draw_body(f, zones.body);
        self.draw_input_row(f, zones.input);
        self.draw_status(f, zones.footer);

        // Overlays (drawn on top)
        // Mirror HelpOverlay geometry (centered_rect(72,82))
        if self.help.visible {
            let popup = super::layout::centered_rect(72, 82, f.area());
            self.hitmap.help = Some(popup);
        }
        self.help.draw(f);
        self.palette.draw(f);

        // Mirror CommandPalette::draw geometry for mouse hit-testing.
        if self.palette.active {
            let area = f.area();
            let max_items = 14usize;
            let visible = self.palette.filtered_len().min(max_items);
            let popup_width = 70u16.min(area.width.saturating_sub(6));
            let popup_height = (visible as u16 + 5).min(area.height.saturating_sub(4));
            let popup = Rect::new(
                (area.width.saturating_sub(popup_width)) / 2,
                4,
                popup_width,
                popup_height,
            );
            self.hitmap.palette = Some(popup);

            // Layout: header input (3), results (rest)
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(1)])
                .split(popup);
            let results_inner = Block::default().borders(Borders::ALL).inner(layout[1]);

            self.hitmap.palette_items.clear();
            let mut y = results_inner.y;
            for _ in 0..visible {
                if y < results_inner.y + results_inner.height {
                    self.hitmap
                        .palette_items
                        .push(Rect::new(results_inner.x, y, results_inner.width, 1));
                }
                y = y.saturating_add(1);
            }
        }

        self.notifs.draw(f);

        // Draw streaming indicator if active
        if self.streaming_active.load(Ordering::SeqCst) {
            // The chat pane handles the streaming cursor - just ensure animation keeps running
        }
    }

    // ── Header ────────────────────────────────────────────────────────────────

    fn draw_header(&mut self, f: &mut Frame, area: Rect) {
        let hz = HeaderZones::compute(area);

        // Brand with animated frame
        let anim_icon = ANIM_FRAMES[self.anim_frame % ANIM_FRAMES.len()];
        let streaming_anim = if self.streaming_active.load(Ordering::SeqCst) {
            self.state.spinner()
        } else {
            anim_icon
        };
        let brand = Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{:>9}", "HOUSAKY"),
                ratatui::style::Style::default()
                    .fg(Palette::BG)
                    .bg(Palette::CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}", streaming_anim),
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .bg(Palette::BG),
            ),
            Span::styled(format!("v{}", VERSION), style_muted()),
        ]))
        .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(brand, hz.brand);

        // Tabs with transition animation
        let tab_titles: Vec<Span> = MainTab::ALL
            .iter()
            .map(|&t| {
                if t == self.state.active_tab {
                    Span::styled(t.label(), style_tab_active())
                } else {
                    Span::styled(t.label(), style_tab_inactive())
                }
            })
            .collect();

        // Draw sliding indicator during tab transitions
        if let Some(trans) = self.tab_transition {
            let _dir_str = match trans.direction {
                TabAnimDirection::RightToLeft => "▶",
                TabAnimDirection::LeftToRight => "◀",
            };
            let anim_chars = ["▎", "▍", "▌", "▋", "▊", "▉", "█", "▇"];
            let slide_char = anim_chars[self.tab_anim_frame % anim_chars.len()];
            let slide_text = if self.tab_anim_frame % 2 == 0 { slide_char } else { " " };
            
            // Draw sliding animation on tabs area
            let slide_paragraph = Paragraph::new(Line::from(vec![
                Span::styled(slide_text, 
                    ratatui::style::Style::default()
                        .fg(Palette::CYAN)
                        .bg(Palette::BG_PANEL)
                ),
            ]));
            let slide_width = (hz.tabs.width / 2).saturating_sub(1);
            let slide_area = Rect::new(hz.tabs.x + slide_width, hz.tabs.y, 1, 1);
            f.render_widget(slide_paragraph, slide_area);
        }

        let divider = Span::styled(" │ ", style_muted());
        let tabs = Tabs::new(tab_titles.clone())
            .select(self.state.active_tab.index())
            .divider(divider.clone())
            .style(ratatui::style::Style::default());
        f.render_widget(tabs, hz.tabs);

        // Compute hitboxes for mouse clicks. Ratatui renders tabs left-to-right with a divider in
        // between; we mirror that to compute per-tab clickable rects.
        self.hitmap.tabs_area = Some(hz.tabs);
        self.hitmap.tab_hitboxes.clear();
        let mut cursor_x = hz.tabs.x;
        let y = hz.tabs.y;
        for (i, title) in tab_titles.iter().enumerate() {
            // Tabs add padding in the label strings themselves (e.g. " Chat "), so width is accurate.
            let w = ratatui::text::Line::from(title.clone()).width() as u16;
            self.hitmap
                .tab_hitboxes
                .push(Rect::new(cursor_x, y, w, hz.tabs.height.max(1)));
            cursor_x = cursor_x.saturating_add(w);

            // divider after each tab except last
            if i + 1 < tab_titles.len() {
                let div_w = ratatui::text::Line::from(divider.clone()).width() as u16;
                cursor_x = cursor_x.saturating_add(div_w);
            }
        }

        // Meta (right side: provider / model / view - 2077 cyberpunk style)
        let view_label = self.state.view_mode.label();
        let provider_owned = if self.provider_name.is_empty() { "NO_PROVIDER".to_string() } else { self.provider_name.clone() };
        let model_owned = truncate_str(&self.model_name, 12).to_owned();
        let view_owned = format!(" {} ", view_label);
        
        let meta = Paragraph::new(Line::from(vec![
            Span::styled(
                "⟨",
                ratatui::style::Style::default().fg(Palette::TEXT_MUTED),
            ),
            Span::styled(
                &provider_owned,
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled(
                "⟩",
                ratatui::style::Style::default().fg(Palette::TEXT_MUTED),
            ),
            Span::styled(
                ".",
                style_muted(),
            ),
            Span::styled(
                "[",
                ratatui::style::Style::default().fg(Palette::TEXT_MUTED),
            ),
            Span::styled(
                &model_owned,
                ratatui::style::Style::default()
                    .fg(Palette::PINK)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled(
                "]",
                ratatui::style::Style::default().fg(Palette::TEXT_MUTED),
            ),
            Span::styled(
                " │ ",
                ratatui::style::Style::default().fg(Palette::BORDER),
            ),
            Span::styled(view_owned, style_dim()),
        ]));
        f.render_widget(meta, hz.meta);
    }

    // ── Body ──────────────────────────────────────────────────────────────────

    fn draw_body(&mut self, f: &mut Frame, area: Rect) {
        let bz = BodyZones::compute(area, self.state.view_mode, self.state.sidebar_visible);

        // Clear tab-specific hit targets before drawing
        self.hitmap.skills_cols = None;
        self.hitmap.tools_cols = None;
        self.hitmap.logs_cols = None;

        match self.state.active_tab {
            MainTab::Chat => {
                let focused_chat = self.state.active_pane == ActivePane::Chat;

                // Mirror ChatPane layout for hit-testing.
                self.hitmap.chat_area = Some(bz.main);
                let inner = Block::default().borders(Borders::ALL).inner(bz.main);
                self.hitmap.chat_viewport = Some(inner);
                self.hitmap.chat_header_items.clear();
                for (mi, line_off) in self.chat.visible_header_line_offsets(inner.height) {
                    let y = inner.y + line_off;
                    if y < inner.y + inner.height {
                        self.hitmap
                            .chat_header_items
                            .push((Rect::new(inner.x, y, inner.width.saturating_sub(1), 1), mi));
                    }
                }
                // Scrollbar renders on the right edge of inner
                if inner.width > 0 {
                    self.hitmap.chat_scrollbar = Some(Rect::new(
                        inner.x + inner.width.saturating_sub(1),
                        inner.y,
                        1,
                        inner.height,
                    ));
                }

                self.chat.draw(f, bz.main, focused_chat);
            }
            MainTab::Skills => {
                // SkillsPanel internally uses a list/detail split (40/60).
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                    .split(bz.main);
                self.hitmap.skills_cols = Some([cols[0], cols[1]]);

                // Compute skills list item hitboxes (1 line per item in List widget)
                let list_inner = Block::default().borders(Borders::ALL).inner(cols[0]);
                let filtered_len = self.skills.filtered_count();
                self.hitmap.skills_list_items.clear();
                let mut y = list_inner.y;
                for _ in 0..filtered_len {
                    if y < list_inner.y + list_inner.height {
                        self.hitmap
                            .skills_list_items
                            .push(Rect::new(list_inner.x, y, list_inner.width, 1));
                    }
                    y = y.saturating_add(1);
                }

                self.skills.draw(f, bz.main);
            }
            MainTab::Tools => {
                // ToolsPanel internally uses a list/detail split (42/58).
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
                    .split(bz.main);
                self.hitmap.tools_cols = Some([cols[0], cols[1]]);

                // Compute list item hitboxes (display indices) for the left list.
                // ToolsPanel::draw_list renders a bordered list whose inner area is the list viewport.
                let list_inner = Block::default().borders(Borders::ALL).inner(cols[0]);
                let filtered_len = self.tools.filtered_count(&self.state.tool_log);
                self.hitmap.tools_list_items.clear();
                let per_item_h: u16 = 2;
                let mut y = list_inner.y;
                for _ in 0..filtered_len {
                    if y + per_item_h <= list_inner.y + list_inner.height {
                        self.hitmap
                            .tools_list_items
                            .push(Rect::new(list_inner.x, y, list_inner.width, per_item_h));
                    }
                    y = y.saturating_add(per_item_h);
                }

                self.tools.draw(f, bz.main, &self.state.tool_log);
            }
            MainTab::Goals => self.draw_goals_tab(f, bz.main),
            MainTab::Metrics => self.draw_metrics_tab(f, bz.main),
            MainTab::Logs => self.draw_logs_tab(f, bz.main),
            MainTab::Config => {
                // Mirror ConfigEditor layout: sections list + fields.
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Length(26), Constraint::Min(30)])
                    .split(bz.main);
                self.hitmap.config_cols = Some([cols[0], cols[1]]);

                let sec_inner = Block::default().borders(Borders::ALL).inner(cols[0]);
                self.hitmap.config_section_items.clear();
                let mut y = sec_inner.y;
                for _ in 0..self.cfg_editor.section_count() {
                    if y < sec_inner.y + sec_inner.height {
                        self.hitmap
                            .config_section_items
                            .push(Rect::new(sec_inner.x, y, sec_inner.width, 1));
                    }
                    y = y.saturating_add(1);
                }

                let field_inner = Block::default().borders(Borders::ALL).inner(cols[1]);
                self.hitmap.config_field_items.clear();
                let mut y2 = field_inner.y;
                for _ in 0..self.cfg_editor.field_count() {
                    if y2 < field_inner.y + field_inner.height {
                        self.hitmap
                            .config_field_items
                            .push(Rect::new(field_inner.x, y2, field_inner.width, 1));
                    }
                    y2 = y2.saturating_add(1);
                }

                self.cfg_editor.draw(f, bz.main);
            }
            MainTab::Doctor => self.doctor.draw(f, bz.main),
        }

        if let Some(sb_area) = bz.sidebar {
            // Mirror Sidebar::draw layout to expose dynamic hitboxes
            let zones = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(10),
                    Constraint::Min(5),
                    Constraint::Length(7),
                ])
                .split(sb_area);
            self.hitmap.sidebar_metrics = Some(zones[0]);
            self.hitmap.sidebar_goals = Some(zones[1]);
            self.hitmap.sidebar_activity = Some(zones[2]);

            self.sidebar.draw(f, sb_area, &self.state.metrics);
        }
    }

    // ── Goals tab ─────────────────────────────────────────────────────────────

    fn draw_goals_tab(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border())
            .title(Span::styled(
                format!(" 🎯 Goals ({}) ", self.sidebar.goals.len()),
                ratatui::style::Style::default()
                    .fg(Palette::GOAL)
                    .add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.sidebar.goals.is_empty() {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled("  No active goals.", style_muted())),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Goals are created automatically during AGI interactions,",
                        style_dim(),
                    )),
                    Line::from(Span::styled(
                        "  or add one via: Ctrl+P → Add goal…",
                        style_dim(),
                    )),
                ]),
                inner,
            );
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        for goal in &self.sidebar.goals {
            let bar = super::theme::render_gauge_bar(goal.progress, 16);
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", goal.priority.icon()),
                    ratatui::style::Style::default().fg(goal.priority.color()),
                ),
                Span::styled(
                    goal.title.clone(),
                    ratatui::style::Style::default()
                        .fg(Palette::TEXT_BRIGHT)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(bar, ratatui::style::Style::default().fg(Palette::CYAN_DIM)),
                Span::styled(format!("  {:.0}%", goal.progress * 100.0), style_dim()),
            ]));
            lines.push(Line::from(""));
        }

        f.render_widget(Paragraph::new(lines), inner);
    }

    // ── Metrics tab ───────────────────────────────────────────────────────────

    fn draw_metrics_tab(&self, f: &mut Frame, area: Rect) {
        let m = &self.state.metrics;

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left: session stats
        let left_block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border())
            .title(Span::styled(" 📊 Session Stats ", style_title()));
        let left_inner = left_block.inner(layout[0]);
        f.render_widget(left_block, layout[0]);

        let err_style = if m.total_errors > 0 {
            ratatui::style::Style::default().fg(Palette::ERROR)
        } else {
            ratatui::style::Style::default().fg(Palette::SUCCESS)
        };

        let left_lines = vec![
            Line::from(""),
            row("  Uptime", m.format_uptime(), Palette::SUCCESS),
            row("  Messages", m.total_messages.to_string(), Palette::TEXT),
            row("  Requests", m.total_requests.to_string(), Palette::TEXT),
            row(
                "  Tokens In",
                m.total_tokens_in.to_string(),
                Palette::CYAN_DIM,
            ),
            row(
                "  Tokens Out",
                m.total_tokens_out.to_string(),
                Palette::CYAN_DIM,
            ),
            row(
                "  Avg t/s",
                format!("{:.1}", m.avg_tokens_per_sec),
                Palette::CYAN,
            ),
            row(
                "  Last Latency",
                format!("{}ms", m.last_latency_ms),
                Palette::TEXT,
            ),
            Line::from(vec![
                Span::styled("  Errors      ", style_muted()),
                Span::styled(m.total_errors.to_string(), err_style),
            ]),
            row(
                "  Error Rate",
                format!("{:.1}%", m.error_rate() * 100.0),
                Palette::TEXT,
            ),
        ];
        f.render_widget(Paragraph::new(left_lines), left_inner);

        // Right: AGI capabilities
        let right_block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border())
            .title(Span::styled(" 🧠 AGI Capabilities ", style_title()));
        let right_inner = right_block.inner(layout[1]);
        f.render_widget(right_block, layout[1]);

        let tools_invoked = m.tools_invoked;
        let skills_enabled = m.skills_enabled;
        let goals_active = self.sidebar.goals.len();

        let skill_ratio = if self.skills.skills.is_empty() {
            0.0
        } else {
            skills_enabled as f64 / self.skills.skills.len() as f64
        };

        let right_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(1),
            ])
            .split(right_inner);

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Skills Enabled  ", style_muted()),
                Span::styled(
                    format!("{}/{}", skills_enabled, self.skills.skills.len()),
                    ratatui::style::Style::default().fg(Palette::SKILL),
                ),
            ])),
            right_layout[0],
        );

        let skill_gauge = Gauge::default()
            .block(Block::default().borders(Borders::NONE))
            .gauge_style(
                ratatui::style::Style::default()
                    .fg(Palette::SKILL)
                    .bg(Palette::BG_ELEVATED),
            )
            .ratio(skill_ratio)
            .label(format!("{:.0}%", skill_ratio * 100.0));
        f.render_widget(skill_gauge, right_layout[1]);

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Tools Invoked   ", style_muted()),
                Span::styled(
                    tools_invoked.to_string(),
                    ratatui::style::Style::default().fg(Palette::TOOL),
                ),
                Span::styled("     Goals Active  ", style_muted()),
                Span::styled(
                    goals_active.to_string(),
                    ratatui::style::Style::default().fg(Palette::GOAL),
                ),
            ])),
            right_layout[2],
        );

        let activity_lines: Vec<Line> = self
            .sidebar
            .activity
            .iter()
            .rev()
            .take(8)
            .map(|a| {
                let icon_style = match a.kind {
                    ActivityKind::Tool => ratatui::style::Style::default().fg(Palette::TOOL),
                    ActivityKind::Skill => ratatui::style::Style::default().fg(Palette::SKILL),
                    ActivityKind::Thought => ratatui::style::Style::default().fg(Palette::THOUGHT),
                    ActivityKind::Goal => ratatui::style::Style::default().fg(Palette::GOAL),
                    ActivityKind::System => style_dim(),
                };
                Line::from(vec![
                    Span::styled(format!(" {} ", a.kind.icon()), icon_style),
                    Span::styled(
                        truncate_str(&a.message, 36),
                        ratatui::style::Style::default().fg(Palette::TEXT_DIM),
                    ),
                    Span::styled(format!(" {}", a.time), style_muted()),
                ])
            })
            .collect();
        f.render_widget(Paragraph::new(activity_lines), right_layout[4]);
    }

    // ── Logs tab ───────────────────────────────────────────────────────────────

    fn draw_logs_tab(&mut self, f: &mut Frame, area: Rect) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);
        // Save for mouse hit-testing
        self.hitmap.logs_cols = Some([cols[0], cols[1]]);

        // Left: AGI activity log
        let log_block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border())
            .title(Span::styled(
                format!(" 📋 AGI Logs ({}) ", self.log_entries.len()),
                ratatui::style::Style::default()
                    .fg(Palette::CYAN)
                    .add_modifier(Modifier::BOLD),
            ));
        let log_inner = log_block.inner(cols[0]);
        f.render_widget(log_block, cols[0]);

        if self.log_entries.is_empty() {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled("  No log entries yet.", style_muted())),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Logs appear here when the AGI heartbeat is active.",
                        style_dim(),
                    )),
                    Line::from(Span::styled("  Start with: housaky agi --tui", style_dim())),
                ]),
                log_inner,
            );
        } else {
            let visible_height = log_inner.height as usize;
            let total = self.log_entries.len();
            let start = if total > visible_height + self.logs_scroll {
                total - visible_height - self.logs_scroll
            } else {
                0
            };
            let end = (start + visible_height).min(total);

            let lines: Vec<Line> = self.log_entries[start..end]
                .iter()
                .map(|entry| {
                    let (icon, color) = if entry.contains("ERROR") || entry.contains("error") {
                        ("✗", Palette::ERROR)
                    } else if entry.contains("WARN") || entry.contains("warn") {
                        ("⚠", Palette::WARNING)
                    } else if entry.contains("Thought:") || entry.contains("thought") {
                        ("💭", Palette::THOUGHT)
                    } else if entry.contains("Goal") || entry.contains("goal") {
                        ("🎯", Palette::GOAL)
                    } else if entry.contains("Improvement") || entry.contains("improve") {
                        ("⚡", Palette::SKILL)
                    } else {
                        ("·", Palette::TEXT_DIM)
                    };
                    Line::from(vec![
                        Span::styled(
                            format!(" {} ", icon),
                            ratatui::style::Style::default().fg(color),
                        ),
                        Span::styled(
                            entry.clone(),
                            ratatui::style::Style::default().fg(Palette::TEXT),
                        ),
                    ])
                })
                .collect();

            f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), log_inner);
        }

        // Right: AGI state summary (pulled from core if available)
        let state_block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_border())
            .title(Span::styled(" 🧠 AGI State ", style_title()));
        let state_inner = state_block.inner(cols[1]);
        f.render_widget(state_block, cols[1]);

        let mut state_lines: Vec<Line> = Vec::new();
        state_lines.push(Line::from(""));

        if self.core.is_some() {
            let m = &self.state.metrics;
            state_lines.push(row("  Status", "● Active".to_string(), Palette::SUCCESS));
            state_lines.push(row("  Uptime", m.format_uptime(), Palette::TEXT));
            state_lines.push(row(
                "  Messages",
                m.total_messages.to_string(),
                Palette::CYAN,
            ));
            state_lines.push(row(
                "  Requests",
                m.total_requests.to_string(),
                Palette::TEXT,
            ));
            state_lines.push(row(
                "  Errors",
                m.total_errors.to_string(),
                if m.total_errors > 0 {
                    Palette::ERROR
                } else {
                    Palette::SUCCESS
                },
            ));
            state_lines.push(Line::from(""));
            state_lines.push(Line::from(Span::styled("  ── Thoughts ──", style_muted())));

            if self.sidebar.thoughts.is_empty() {
                state_lines.push(Line::from(Span::styled("  (none yet)", style_dim())));
            } else {
                for t in self.sidebar.thoughts.iter().rev().take(6) {
                    state_lines.push(Line::from(vec![
                        Span::styled(
                            "  💭 ",
                            ratatui::style::Style::default().fg(Palette::THOUGHT),
                        ),
                        Span::styled(
                            truncate_str(t, 38),
                            ratatui::style::Style::default().fg(Palette::TEXT_DIM),
                        ),
                    ]));
                }
            }
        } else {
            state_lines.push(Line::from(Span::styled(
                "  Status: ○ Disconnected",
                style_dim(),
            )));
            state_lines.push(Line::from(""));
            state_lines.push(Line::from(Span::styled(
                "  No AGI core connected.",
                style_muted(),
            )));
            state_lines.push(Line::from(Span::styled(
                "  Launch with: housaky agi --tui",
                style_dim(),
            )));
        }

        f.render_widget(Paragraph::new(state_lines), state_inner);
    }

    // ── Input row ─────────────────────────────────────────────────────────────

    fn draw_input_row(&self, f: &mut Frame, area: Rect) {
        let focused =
            self.state.active_pane == ActivePane::Input || self.state.input_mode.is_typing();
        self.input.draw(f, area, focused);
    }

    // ── Status bar ────────────────────────────────────────────────────────────

    fn draw_status(&self, f: &mut Frame, area: Rect) {
        super::status_bar::draw(f, area, &self.state, &self.provider_name, &self.model_name);
    }

    // ── Key handlers ─────────────────────────────────────────────────────────

    fn handle_help_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up => self.help.scroll_up(),
            KeyCode::Down => self.help.scroll_down(),
            _ => self.help.hide(),
        }
        Ok(())
    }

    fn handle_palette_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.palette.close();
                self.state.show_command_palette = false;
            }
            KeyCode::Up => self.palette.prev(),
            KeyCode::Down => self.palette.next(),
            KeyCode::Enter => {
                if let Some(action) = self.palette.execute() {
                    self.state.show_command_palette = false;
                    self.execute_palette_action(action)?;
                }
            }
            KeyCode::Backspace => self.palette.backspace(),
            KeyCode::Char(c) => self.palette.push_char(c),
            _ => {}
        }
        Ok(())
    }

    fn execute_palette_action(&mut self, action: PaletteAction) -> Result<()> {
        match action {
            PaletteAction::ClearChat => {
                self.chat.clear();
                self.notifs.info("Conversation cleared");
            }
            PaletteAction::ExportChat => {
                self.export_chat()?;
            }
            PaletteAction::CopyLastResponse => {
                if let Some(msg) = self.chat.last_assistant() {
                    let _ = msg.content.clone(); // clipboard would go here
                    self.notifs.info("Response copied (use 's' to save)");
                }
            }
            PaletteAction::ToggleAutoScroll => {
                self.chat.toggle_auto_scroll();
                self.notifs.info(if self.chat.auto_scroll {
                    "Auto-scroll ON"
                } else {
                    "Auto-scroll OFF"
                });
            }
            PaletteAction::CycleView => {
                self.state.view_mode = self.state.view_mode.cycle();
                self.notifs
                    .info(format!("View: {}", self.state.view_mode.label()));
            }
            PaletteAction::ToggleSidebar => {
                self.state.sidebar_visible = !self.state.sidebar_visible;
            }
            PaletteAction::GotoChat => {
                self.switch_tab(MainTab::Chat);
            }
            PaletteAction::GotoSkills => {
                self.switch_tab(MainTab::Skills);
            }
            PaletteAction::GotoTools => {
                self.switch_tab(MainTab::Tools);
            }
            PaletteAction::GotoGoals => {
                self.switch_tab(MainTab::Goals);
            }
            PaletteAction::GotoMetrics => {
                self.switch_tab(MainTab::Metrics);
            }
            PaletteAction::GotoConfig => {
                self.switch_tab(MainTab::Config);
                self.cfg_editor = ConfigEditor::new(&self.config);
            }
            PaletteAction::GotoDoctor => {
                self.switch_tab(MainTab::Doctor);
                self.refresh_doctor();
            }
            PaletteAction::Reflect => {
                self.chat
                    .push_system("AGI: initiating self-reflection cycle…".to_string());
                self.sidebar
                    .push_activity(ActivityKind::Thought, "Self-reflection triggered");
                self.notifs.info("Reflection cycle started");
            }
            PaletteAction::AddGoal(title) => {
                self.sidebar.goals.push(SidebarGoal {
                    title: title.clone(),
                    progress: 0.0,
                    priority: super::sidebar::GoalPriority::Medium,
                });
                self.state.metrics.goals_active = self.sidebar.goals.len();
                self.notifs.success(format!("Goal added: {}", title));
            }
            PaletteAction::AddGoalStatic(title) => {
                self.sidebar.goals.push(SidebarGoal {
                    title: title.to_string(),
                    progress: 0.0,
                    priority: super::sidebar::GoalPriority::Medium,
                });
                self.state.metrics.goals_active = self.sidebar.goals.len();
                self.notifs.success(format!("Goal added: {}", title));
            }
            PaletteAction::SwitchModel(m) => {
                self.model_name = m.clone();
                self.notifs.success(format!("Model → {}", m));
            }
            PaletteAction::OpenHelp => {
                self.help.show();
            }
            PaletteAction::Quit => {
                self.state.should_quit = true;
            }
        }
        Ok(())
    }

    fn handle_chat_key(&mut self, key: KeyEvent) -> Result<()> {
        // If skills filter active — shouldn't happen here but guard
        if self.state.input_mode == InputMode::Search {
            return self.handle_search_key(key);
        }

        match (key.modifiers, key.code) {
            // --- Typing mode --------------------------------------------------
            (KeyModifiers::NONE, KeyCode::Enter) if self.state.input_mode.is_typing() => {
                if !self.input.is_empty() {
                    let msg = self.input.take();
                    if msg.starts_with('/') {
                        self.handle_slash_command(&msg)?;
                    } else {
                        self.send_message(msg)?;
                    }
                }
            }
            (KeyModifiers::NONE, KeyCode::Esc) if self.state.input_mode.is_typing() => {
                self.state.input_mode = InputMode::Normal;
                self.state.active_pane = ActivePane::Chat;
            }
            (KeyModifiers::NONE, KeyCode::Backspace) if self.state.input_mode.is_typing() => {
                self.input.backspace();
            }
            (KeyModifiers::NONE, KeyCode::Delete) if self.state.input_mode.is_typing() => {
                self.input.delete_forward();
            }
            (KeyModifiers::NONE, KeyCode::Left) if self.state.input_mode.is_typing() => {
                self.input.move_cursor_left();
            }
            (KeyModifiers::NONE, KeyCode::Right) if self.state.input_mode.is_typing() => {
                self.input.move_cursor_right();
            }
            (KeyModifiers::NONE, KeyCode::Home) if self.state.input_mode.is_typing() => {
                self.input.move_cursor_home();
            }
            (KeyModifiers::NONE, KeyCode::End) if self.state.input_mode.is_typing() => {
                self.input.move_cursor_end();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('k')) if self.state.input_mode.is_typing() => {
                self.input.kill_line();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('w')) if self.state.input_mode.is_typing() => {
                self.input.kill_word_back();
            }
            (KeyModifiers::NONE, KeyCode::Up) if self.state.input_mode.is_typing() => {
                self.input.history_prev();
            }
            (KeyModifiers::NONE, KeyCode::Down) if self.state.input_mode.is_typing() => {
                self.input.history_next();
            }
            (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c))
                if self.state.input_mode.is_typing() =>
            {
                self.input.push_char(c);
            }

            // --- Normal mode --------------------------------------------------
            // Any printable key or Enter enters insert mode
            (KeyModifiers::NONE, KeyCode::Char('i') | KeyCode::Enter) => {
                self.state.input_mode = InputMode::Insert;
                self.state.active_pane = ActivePane::Input;
            }

            // Ctrl+Tab to switch focus between chat and input
            (KeyModifiers::CONTROL, KeyCode::Tab) => {
                if self.state.active_pane == ActivePane::Chat {
                    self.state.active_pane = ActivePane::Input;
                    self.state.input_mode = InputMode::Insert;
                } else {
                    self.state.active_pane = ActivePane::Chat;
                }
            }
            // Ctrl+Shift+Tab (BackTab) also switches focus
            (KeyModifiers::CONTROL, KeyCode::BackTab) => {
                if self.state.active_pane == ActivePane::Input {
                    self.state.active_pane = ActivePane::Chat;
                } else {
                    self.state.active_pane = ActivePane::Input;
                    self.state.input_mode = InputMode::Insert;
                }
            }

            // Scroll chat
            (KeyModifiers::NONE, KeyCode::Up) => self.chat.scroll_up(1),
            (KeyModifiers::NONE, KeyCode::Down) => self.chat.scroll_down(1),
            (KeyModifiers::NONE, KeyCode::PageUp) => self.chat.scroll_up(8),
            (KeyModifiers::NONE, KeyCode::PageDown) => self.chat.scroll_down(8),
            (KeyModifiers::NONE, KeyCode::Home) => self.chat.scroll_to_top(),
            (KeyModifiers::NONE, KeyCode::End) => self.chat.scroll_to_bottom(),

            // Auto-scroll toggle
            (KeyModifiers::NONE, KeyCode::Char('a')) => {
                self.chat.toggle_auto_scroll();
                self.notifs.info(if self.chat.auto_scroll {
                    "Auto-scroll ON"
                } else {
                    "Auto-scroll OFF"
                });
            }

            // Save / export
            (KeyModifiers::NONE, KeyCode::Char('s')) => self.export_chat()?,

            // Search
            (KeyModifiers::NONE, KeyCode::Char('/'))
            | (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
                self.state.input_mode = InputMode::Search;
                self.state.show_search = true;
            }
            (KeyModifiers::NONE, KeyCode::Char('n')) => self.chat.search_next(),
            (KeyModifiers::SHIFT, KeyCode::Char('N')) => self.chat.search_prev(),

            // View
            (KeyModifiers::NONE, KeyCode::Char('v')) => {
                self.state.view_mode = self.state.view_mode.cycle();
            }
            (KeyModifiers::NONE, KeyCode::Char('b')) => {
                self.state.sidebar_visible = !self.state.sidebar_visible;
            }

            // Clear conversation
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                self.chat.clear();
                self.notifs.info("Conversation cleared");
            }

            // Help
            (KeyModifiers::NONE, KeyCode::Char('?') | KeyCode::F(1)) => {
                self.help.toggle();
            }

            // Number keys → direct tab (with history)
            (KeyModifiers::NONE, KeyCode::Char('1')) => {
                self.switch_tab(MainTab::Chat);
            }
            (KeyModifiers::NONE, KeyCode::Char('2')) => {
                self.switch_tab(MainTab::Skills);
            }
            (KeyModifiers::NONE, KeyCode::Char('3')) => {
                self.switch_tab(MainTab::Tools);
            }
            (KeyModifiers::NONE, KeyCode::Char('4')) => {
                self.switch_tab(MainTab::Goals);
            }
            (KeyModifiers::NONE, KeyCode::Char('5')) => {
                self.switch_tab(MainTab::Metrics);
            }
            (KeyModifiers::NONE, KeyCode::Char('6')) => {
                self.switch_tab(MainTab::Logs);
            }
            (KeyModifiers::NONE, KeyCode::Char('7')) => {
                self.switch_tab(MainTab::Config);
            }
            (KeyModifiers::NONE, KeyCode::Char('8')) => {
                self.switch_tab(MainTab::Doctor);
                self.refresh_doctor();
            }

            // Quit
            (KeyModifiers::NONE, KeyCode::Char('q')) => {
                if self.state.input_mode == InputMode::Normal {
                    self.state.should_quit = true;
                }
            }
            (KeyModifiers::NONE, KeyCode::Esc) => {
                if self.state.input_mode == InputMode::Normal {
                    self.state.should_quit = true;
                }
            }

            _ => {}
        }
        Ok(())
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.state.input_mode = InputMode::Insert;
                self.state.show_search = false;
            }
            KeyCode::Backspace => {
                self.input.backspace();
                self.chat.set_search(self.input.text.clone());
            }
            KeyCode::Char(c) => {
                self.input.push_char(c);
                self.chat.set_search(self.input.text.clone());
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_skills_key(&mut self, key: KeyEvent) -> Result<()> {
        if self.skills.is_filter_active() {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.skills.filter_commit(),
                KeyCode::Backspace => self.skills.filter_pop(),
                KeyCode::Char(c) => self.skills.filter_push(c),
                _ => {}
            }
            return Ok(());
        }

        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
                self.switch_tab(MainTab::Chat);
            }
            (_, KeyCode::Up | KeyCode::Char('k')) => self.skills.select_prev(),
            (_, KeyCode::Down | KeyCode::Char('j')) => self.skills.tab_next(),
            (_, KeyCode::Char(' ') | KeyCode::Enter) => {
                if let Some((name, source)) = self.skills.get_selected_skill_needs_install() {
                    match source {
                        crate::tui::enhanced_app::skills_panel::SkillSource::ClaudeOfficial => {
                            self.notifs
                                .info(format!("Installing {} from Claude Market...", name));
                            match crate::skills::marketplace::install_claude_plugin(
                                &self.config.workspace_dir,
                                &name,
                            ) {
                                Ok(_) => {
                                    self.skills.mark_selected_installed();
                                    self.config.skills.enabled.insert(name.clone(), true);
                                    self.notifs.success(format!("Installed: {}", name));
                                }
                                Err(e) => {
                                    self.notifs.error(format!("Install failed: {}", e));
                                }
                            }
                        }
                        crate::tui::enhanced_app::skills_panel::SkillSource::OpenClaw => {
                            self.notifs
                                .info(format!("Installing {} from OpenClaw...", name));
                            if let Err(e) = crate::skills::marketplace::install_openclaw_skill(
                                &self.config.workspace_dir,
                                &name,
                            ) {
                                self.notifs.error(format!("Install failed: {}", e));
                            } else {
                                self.skills.mark_selected_installed();
                                self.config.skills.enabled.insert(name.clone(), true);
                                self.notifs.success(format!("Installed: {}", name));
                            }
                        }
                        _ => {
                            self.notifs.error("Cannot install: unknown source");
                        }
                    }
                } else if let Some(enabled) = self.skills.toggle_selected() {
                    self.state.metrics.skills_enabled =
                        self.skills.skills.iter().filter(|s| s.enabled).count();
                    let kind = if enabled { "Enabled" } else { "Disabled" };
                    self.sidebar
                        .push_activity(ActivityKind::Skill, format!("{}: skill", kind));
                    if let Some(skill_name) = self.skills.get_selected_skill_name() {
                        self.config.skills.enabled.insert(skill_name.clone(), enabled);
                        if let Err(e) = self.config.save() {
                            self.notifs
                                .error(format!("Failed to save config: {}", e));
                        } else {
                            // Rebuild system prompt to include/exclude the skill
                            self.rebuild_system_prompt();
                            self.notifs.success(format!("{} skill: {}", kind, skill_name));
                        }
                    }
                }
            }
            (_, KeyCode::Char('r')) => {
                let skills_dir = self.config.workspace_dir.join("skills");
                let config_skills: Vec<(String, bool)> = self
                    .config
                    .skills
                    .enabled
                    .iter()
                    .map(|(name, &en)| (name.clone(), en))
                    .collect();
                self.skills.load_from_paths(&skills_dir, &config_skills);
                self.skills
                    .load_marketplace_skills(&self.config.workspace_dir, &self.config);
                self.state.metrics.skills_enabled =
                    self.skills.skills.iter().filter(|s| s.enabled).count();
                // Rebuild system prompt with refreshed skills
                self.rebuild_system_prompt();
                self.notifs
                    .info("Skills refreshed from local and marketplace");
            }
            (_, KeyCode::Char('/')) => self.skills.start_filter(),
            (_, KeyCode::PageUp) => self.skills.detail_scroll_up(),
            (_, KeyCode::PageDown) => self.skills.detail_scroll_down(),
            (KeyModifiers::NONE, KeyCode::Char('1')) => {
                self.switch_tab(MainTab::Chat);
            }
            (KeyModifiers::NONE, KeyCode::Char('2')) => {
                self.switch_tab(MainTab::Skills);
            }
            (KeyModifiers::NONE, KeyCode::Char('3')) => {
                self.switch_tab(MainTab::Tools);
            }
            (KeyModifiers::NONE, KeyCode::Char('4')) => {
                self.switch_tab(MainTab::Goals);
            }
            (KeyModifiers::NONE, KeyCode::Char('5')) => {
                self.switch_tab(MainTab::Metrics);
            }
            (KeyModifiers::NONE, KeyCode::Char('6')) => {
                self.switch_tab(MainTab::Logs);
            }
            (KeyModifiers::NONE, KeyCode::Char('7')) => {
                self.switch_tab(MainTab::Config);
            }
            (KeyModifiers::NONE, KeyCode::Char('?') | KeyCode::F(1)) => {
                self.help.toggle();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_tools_key(&mut self, key: KeyEvent) -> Result<()> {
        if self.tools.is_filter_active() {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.tools.filter_commit(),
                KeyCode::Backspace => self.tools.filter_pop(),
                KeyCode::Char(c) => self.tools.filter_push(c),
                _ => {}
            }
            return Ok(());
        }

        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
                self.switch_tab(MainTab::Chat);
            }
            (_, KeyCode::Up | KeyCode::Char('k')) => {
                self.tools.select_prev(&self.state.tool_log);
            }
            (_, KeyCode::Down | KeyCode::Char('j')) => {
                self.tools.select_next(&self.state.tool_log);
            }
            (_, KeyCode::PageUp) => self.tools.detail_scroll_up(),
            (_, KeyCode::PageDown) => self.tools.detail_scroll_down(),
            (_, KeyCode::Char('/')) => self.tools.start_filter(),
            (_, KeyCode::Char('t')) => self.tools.toggle_view_mode(),
            (_, KeyCode::Char('c')) => {
                self.state.tool_log.clear();
                self.notifs.info("Tool log cleared");
            }
            (KeyModifiers::NONE, KeyCode::Char('1')) => {
                self.state.active_tab = MainTab::Chat;
            }
            (KeyModifiers::NONE, KeyCode::Char('2')) => {
                self.state.active_tab = MainTab::Skills;
            }
            (KeyModifiers::NONE, KeyCode::Char('3')) => {
                self.state.active_tab = MainTab::Tools;
            }
            (KeyModifiers::NONE, KeyCode::Char('4')) => {
                self.state.active_tab = MainTab::Goals;
            }
            (KeyModifiers::NONE, KeyCode::Char('5')) => {
                self.state.active_tab = MainTab::Metrics;
            }
            (KeyModifiers::NONE, KeyCode::Char('6')) => {
                self.state.active_tab = MainTab::Logs;
            }
            (KeyModifiers::NONE, KeyCode::Char('7')) => {
                self.state.active_tab = MainTab::Config;
            }
            (KeyModifiers::NONE, KeyCode::Char('?') | KeyCode::F(1)) => {
                self.help.toggle();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_goals_key(&mut self, key: KeyEvent) -> Result<()> {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
                self.switch_tab(MainTab::Chat);
            }
            (KeyModifiers::NONE, KeyCode::Char('1')) => {
                self.switch_tab(MainTab::Chat);
            }
            (KeyModifiers::NONE, KeyCode::Char('2')) => {
                self.switch_tab(MainTab::Skills);
            }
            (KeyModifiers::NONE, KeyCode::Char('3')) => {
                self.switch_tab(MainTab::Tools);
            }
            (KeyModifiers::NONE, KeyCode::Char('4')) => {
                self.switch_tab(MainTab::Goals);
            }
            (KeyModifiers::NONE, KeyCode::Char('5')) => {
                self.switch_tab(MainTab::Metrics);
            }
            (KeyModifiers::NONE, KeyCode::Char('6')) => {
                self.switch_tab(MainTab::Logs);
            }
            (KeyModifiers::NONE, KeyCode::Char('7')) => {
                self.switch_tab(MainTab::Config);
            }
            (KeyModifiers::NONE, KeyCode::Char('?') | KeyCode::F(1)) => {
                self.help.toggle();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_metrics_key(&mut self, key: KeyEvent) -> Result<()> {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
                self.switch_tab(MainTab::Chat);
            }
            (KeyModifiers::NONE, KeyCode::Char('1')) => {
                self.switch_tab(MainTab::Chat);
            }
            (KeyModifiers::NONE, KeyCode::Char('2')) => {
                self.switch_tab(MainTab::Skills);
            }
            (KeyModifiers::NONE, KeyCode::Char('3')) => {
                self.switch_tab(MainTab::Tools);
            }
            (KeyModifiers::NONE, KeyCode::Char('4')) => {
                self.switch_tab(MainTab::Goals);
            }
            (KeyModifiers::NONE, KeyCode::Char('5')) => {
                self.switch_tab(MainTab::Metrics);
            }
            (KeyModifiers::NONE, KeyCode::Char('6')) => {
                self.switch_tab(MainTab::Logs);
            }
            (KeyModifiers::NONE, KeyCode::Char('7')) => {
                self.switch_tab(MainTab::Config);
            }
            (KeyModifiers::NONE, KeyCode::Char('?') | KeyCode::F(1)) => {
                self.help.toggle();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_logs_key(&mut self, key: KeyEvent) -> Result<()> {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
                self.switch_tab(MainTab::Chat);
            }
            (KeyModifiers::NONE, KeyCode::Up) => {
                self.logs_scroll = self.logs_scroll.saturating_add(1);
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                self.logs_scroll = self.logs_scroll.saturating_sub(1);
            }
            (KeyModifiers::NONE, KeyCode::PageUp) => {
                self.logs_scroll = self.logs_scroll.saturating_add(10);
            }
            (KeyModifiers::NONE, KeyCode::PageDown) => {
                self.logs_scroll = self.logs_scroll.saturating_sub(10);
            }
            (KeyModifiers::NONE, KeyCode::Home) => {
                self.logs_scroll = self.log_entries.len();
            }
            (KeyModifiers::NONE, KeyCode::End) => {
                self.logs_scroll = 0;
            }
            (_, KeyCode::Char('c')) => {
                self.log_entries.clear();
                self.logs_scroll = 0;
                self.notifs.info("Logs cleared");
            }
            (KeyModifiers::NONE, KeyCode::Char('1')) => {
                self.switch_tab(MainTab::Chat);
            }
            (KeyModifiers::NONE, KeyCode::Char('2')) => {
                self.switch_tab(MainTab::Skills);
            }
            (KeyModifiers::NONE, KeyCode::Char('3')) => {
                self.switch_tab(MainTab::Tools);
            }
            (KeyModifiers::NONE, KeyCode::Char('4')) => {
                self.switch_tab(MainTab::Goals);
            }
            (KeyModifiers::NONE, KeyCode::Char('5')) => {
                self.switch_tab(MainTab::Metrics);
            }
            (KeyModifiers::NONE, KeyCode::Char('6')) => {
                self.switch_tab(MainTab::Logs);
            }
            (KeyModifiers::NONE, KeyCode::Char('7')) => {
                self.switch_tab(MainTab::Config);
            }
            (KeyModifiers::NONE, KeyCode::Char('?') | KeyCode::F(1)) => {
                self.help.toggle();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_config_key(&mut self, key: KeyEvent) -> Result<()> {
        // When inline editor popup is open, route all keys into it
        if self.cfg_editor.is_editing() {
            match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Enter) => {
                    if !self.cfg_editor.commit_edit() {
                        // validation error — leave editor open
                    }
                }
                (KeyModifiers::NONE, KeyCode::Esc) => {
                    self.cfg_editor.cancel_edit();
                }
                (KeyModifiers::NONE, KeyCode::Backspace) => self.cfg_editor.edit_backspace(),
                (KeyModifiers::NONE, KeyCode::Left) => self.cfg_editor.edit_left(),
                (KeyModifiers::NONE, KeyCode::Right) => self.cfg_editor.edit_right(),
                (KeyModifiers::NONE, KeyCode::Home) => self.cfg_editor.edit_home(),
                (KeyModifiers::NONE, KeyCode::End) => self.cfg_editor.edit_end(),
                (KeyModifiers::CONTROL, KeyCode::Char('k')) => self.cfg_editor.edit_kill_line(),
                (KeyModifiers::NONE, KeyCode::Char(c)) => self.cfg_editor.edit_push(c),
                _ => {}
            }
            return Ok(());
        }

        // Raw TOML view
        if self.cfg_editor.is_showing_raw() {
            match key.code {
                KeyCode::Char('r' | 'q') | KeyCode::Esc => {
                    self.cfg_editor.toggle_raw(&self.config);
                }
                _ => {}
            }
            return Ok(());
        }

        match (key.modifiers, key.code) {
            // Reload config from disk
            (KeyModifiers::CONTROL, KeyCode::Char('r')) => {
                match crate::config::Config::load_or_init() {
                    Ok(new_config) => {
                        self.config = new_config;
                        self.cfg_editor = ConfigEditor::new(&self.config);
                        self.notifs.success("Config reloaded from disk");
                    }
                    Err(e) => self.notifs.error(format!("Reload failed: {}", e)),
                }
            }

            // Save
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                match self.cfg_editor.apply_and_save(&mut self.config) {
                    Ok(()) => {
                        self.notifs
                            .success("Config saved to ~/.housaky/config.toml");
                        // Refresh provider/model names from updated config
                        if let Some(p) = &self.config.default_provider {
                            self.provider_name = p.clone();
                        }
                        if let Some(m) = &self.config.default_model {
                            self.model_name = m.clone();
                        }
                    }
                    Err(e) => self.notifs.error(format!("Save failed: {}", e)),
                }
            }

            // Navigation
            (_, KeyCode::Up | KeyCode::Char('k')) => self.cfg_editor.field_up(),
            (_, KeyCode::Down | KeyCode::Char('j')) => self.cfg_editor.field_down(),

            // Edit selected field (also Space for bool toggle)
            (_, KeyCode::Enter | KeyCode::Char(' ')) => self.cfg_editor.start_edit(),

            // Section tabs
            (_, KeyCode::Tab) => self.cfg_editor.section_next(&self.config),
            (KeyModifiers::SHIFT, KeyCode::BackTab) => self.cfg_editor.section_prev(&self.config),

            // Raw TOML toggle
            (_, KeyCode::Char('r')) => self.cfg_editor.toggle_raw(&self.config),

            // Global tab jump
            (KeyModifiers::NONE, KeyCode::Char('1')) => {
                self.switch_tab(MainTab::Chat);
            }
            (KeyModifiers::NONE, KeyCode::Char('2')) => {
                self.switch_tab(MainTab::Skills);
            }
            (KeyModifiers::NONE, KeyCode::Char('3')) => {
                self.switch_tab(MainTab::Tools);
            }
            (KeyModifiers::NONE, KeyCode::Char('4')) => {
                self.switch_tab(MainTab::Goals);
            }
            (KeyModifiers::NONE, KeyCode::Char('5')) => {
                self.switch_tab(MainTab::Metrics);
            }
            (KeyModifiers::NONE, KeyCode::Char('6')) => {
                self.switch_tab(MainTab::Logs);
            }
            (KeyModifiers::NONE, KeyCode::Char('7')) => {
                self.switch_tab(MainTab::Config);
            }

            // Back to chat
            (_, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
                if self.cfg_editor.dirty {
                    self.notifs
                        .warn("Unsaved changes — Ctrl+S to save, q again to discard");
                    // second q will actually leave — handled by marking not-dirty so next q exits
                    // For simplicity, prompt via notification; user presses q once more
                } else {
                    self.switch_tab(MainTab::Chat);
                }
            }

            (KeyModifiers::NONE, KeyCode::Char('?') | KeyCode::F(1)) => {
                self.help.toggle();
            }

            _ => {}
        }
        Ok(())
    }

    // ── Slash command dispatcher ──────────────────────────────────────────────

    fn handle_slash_command(&mut self, raw: &str) -> Result<()> {
        let cmd = raw.trim_start_matches('/');
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        let name = parts[0].trim();
        let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");

        match name {
            "clear" => {
                self.chat.clear();
                self.notifs.info("Conversation cleared");
            }
            "export" | "save" => {
                self.export_chat()?;
            }
            "model" => {
                if arg.is_empty() {
                    self.chat
                        .push_system(format!("Current model: {}", self.model_name));
                } else {
                    self.model_name = arg.to_string();
                    self.notifs.success(format!("Model → {}", arg));
                }
            }
            "provider" => {
                if arg.is_empty() {
                    self.chat
                        .push_system(format!("Current provider: {}", self.provider_name));
                } else {
                    self.provider_name = arg.to_string();
                    self.notifs.success(format!("Provider → {}", arg));
                }
            }
            "goals" => {
                self.switch_tab(MainTab::Goals);
            }
            "skills" => {
                self.switch_tab(MainTab::Skills);
            }
            "tools" => {
                self.switch_tab(MainTab::Tools);
            }
            "metrics" => {
                self.switch_tab(MainTab::Metrics);
            }
            "logs" => {
                self.switch_tab(MainTab::Logs);
            }
            "view" => {
                self.state.view_mode = self.state.view_mode.cycle();
                self.notifs
                    .info(format!("View: {}", self.state.view_mode.label()));
            }
            "sidebar" => {
                self.state.sidebar_visible = !self.state.sidebar_visible;
                self.notifs.info(if self.state.sidebar_visible {
                    "Sidebar shown"
                } else {
                    "Sidebar hidden"
                });
            }
            "auto" => {
                self.chat.toggle_auto_scroll();
                self.notifs.info(if self.chat.auto_scroll {
                    "Auto-scroll ON"
                } else {
                    "Auto-scroll OFF"
                });
            }
            "status" => {
                self.chat.push_system(format!(
                    "Provider: {} | Model: {} | View: {} | Messages: {}",
                    self.provider_name,
                    self.model_name,
                    self.state.view_mode.label(),
                    self.chat.messages.len()
                ));
            }
            "config" | "cfg" | "settings" => {
                self.switch_tab(MainTab::Config);
                self.cfg_editor = ConfigEditor::new(&self.config);
            }
            "reflect" => {
                self.chat
                    .push_system("🧠 Initiating AGI self-reflection cycle…".to_string());
                self.sidebar
                    .push_activity(ActivityKind::Thought, "Self-reflection triggered");
                self.notifs.info("Reflection cycle started");

                // Actually trigger self-improvement via the AGI core
                if let Some(ref core) = self.core {
                    let core = Arc::clone(core);
                    let provider_name = self.provider_name.clone();
                    let model_name = self.model_name.clone();
                    let api_key = self.resolved_api_key.clone();
                    let reliability = self.config.reliability.clone();
                    let streaming_chunks = Arc::clone(&self.streaming_chunks);
                    let streaming_result = Arc::clone(&self.streaming_result);
                    self.chat.start_streaming();
                    self.state.stream_status = StreamStatus::Streaming;
                    self.streaming_active.store(true, Ordering::SeqCst);
                    {
                        let mut g = self.streaming_result.lock().unwrap();
                        *g = None;
                    }

                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Builder::new_multi_thread()
                            .worker_threads(2)
                            .enable_all()
                            .build();
                        let rt = match rt {
                            Ok(r) => r,
                            Err(e) => {
                                if let Ok(mut g) = streaming_result.lock() {
                                    *g = Some(Err(format!("Runtime error: {e}")));
                                }
                                return;
                            }
                        };
                        rt.block_on(async {
                            let provider = match crate::providers::create_resilient_provider(
                                &provider_name,
                                api_key.as_deref(),
                                &reliability,
                                None,
                            ) {
                                Ok(p) => p,
                                Err(e) => {
                                    if let Ok(mut g) = streaming_result.lock() {
                                        *g = Some(Err(format!("Provider error: {e}")));
                                    }
                                    return;
                                }
                            };

                            if let Ok(mut chunks) = streaming_chunks.lock() {
                                chunks.push("🔄 Running self-improvement cycle on ~/housaky…\n\n".to_string());
                            }

                            match core
                                .run_self_improvement(provider.as_ref(), &model_name)
                                .await
                            {
                                Ok(improvements) => {
                                    let summary = if improvements.is_empty() {
                                        "✅ Self-reflection complete. No improvements identified this cycle.".to_string()
                                    } else {
                                        format!(
                                            "✅ Self-reflection complete. {} improvements:\n{}",
                                            improvements.len(),
                                            improvements
                                                .iter()
                                                .map(|i| format!("  • {i}"))
                                                .collect::<Vec<_>>()
                                                .join("\n")
                                        )
                                    };
                                    if let Ok(mut chunks) = streaming_chunks.lock() {
                                        chunks.push(summary.clone());
                                    }
                                    if let Ok(mut g) = streaming_result.lock() {
                                        *g = Some(Ok(summary));
                                    }
                                }
                                Err(e) => {
                                    if let Ok(mut g) = streaming_result.lock() {
                                        *g = Some(Err(format!("Reflection error: {e}")));
                                    }
                                }
                            }
                        });
                    });
                } else {
                    self.chat.push_system(
                        "⚠ No AGI core available. Start with `housaky run` for full AGI mode."
                            .to_string(),
                    );
                }
            }
            "doctor" | "health" | "diag" => {
                self.switch_tab(MainTab::Doctor);
                self.refresh_doctor();
                self.notifs.info("Doctor: running diagnostics…");
            }
            "help" => {
                self.help.show();
            }
            "quit" | "exit" => {
                self.state.should_quit = true;
            }
            other => {
                self.notifs.warn(format!("Unknown command: /{}", other));
                self.chat.push_system(format!(
                    "Unknown command: /{}. Type /help for a list.",
                    other
                ));
            }
        }
        Ok(())
    }

    // ── Doctor tab helpers ────────────────────────────────────────────────────

    pub fn refresh_doctor(&mut self) {
        let report = crate::doctor::collect(&self.config);
        let problems = report.total_errors + report.total_warnings;
        self.doctor.load(report);
        if problems == 0 {
            self.notifs.info("Doctor: all checks passed ✅");
        } else {
            self.notifs
                .warn(format!("Doctor: {} issue(s) found", problems));
        }
        self.sidebar
            .push_activity(ActivityKind::System, "Doctor report refreshed");
    }

    fn handle_doctor_key(&mut self, key: KeyEvent) -> Result<()> {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
                self.switch_tab(MainTab::Chat);
            }
            (_, KeyCode::Up | KeyCode::Char('k')) => self.doctor.select_prev(),
            (_, KeyCode::Down | KeyCode::Char('j')) => self.doctor.select_next(),
            (_, KeyCode::PageUp) => self.doctor.detail_scroll_up(),
            (_, KeyCode::PageDown) => self.doctor.detail_scroll_down(),
            (_, KeyCode::Tab) => self.doctor.cycle_filter_next(),
            (KeyModifiers::SHIFT, KeyCode::BackTab) => self.doctor.cycle_filter_prev(),
            (_, KeyCode::Char('r') | KeyCode::F(5)) => {
                self.refresh_doctor();
            }
            (_, KeyCode::Char('f')) => {
                let fixable = self
                    .doctor
                    .report
                    .as_ref()
                    .map(|r| r.auto_fixable)
                    .unwrap_or(0);
                if fixable > 0 {
                    match crate::doctor::run_fix(&self.config) {
                        Ok(()) => {
                            self.notifs
                                .success(format!("Doctor fix: applied {} fix(es)", fixable));
                        }
                        Err(e) => {
                            self.notifs.error(format!("Doctor fix failed: {e}"));
                        }
                    }
                    self.refresh_doctor();
                } else {
                    self.notifs.info("No auto-fixable issues");
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('1')) => {
                self.switch_tab(MainTab::Chat);
            }
            (KeyModifiers::NONE, KeyCode::Char('2')) => {
                self.switch_tab(MainTab::Skills);
            }
            (KeyModifiers::NONE, KeyCode::Char('3')) => {
                self.switch_tab(MainTab::Tools);
            }
            (KeyModifiers::NONE, KeyCode::Char('4')) => {
                self.switch_tab(MainTab::Goals);
            }
            (KeyModifiers::NONE, KeyCode::Char('5')) => {
                self.switch_tab(MainTab::Metrics);
            }
            (KeyModifiers::NONE, KeyCode::Char('6')) => {
                self.switch_tab(MainTab::Logs);
            }
            (KeyModifiers::NONE, KeyCode::Char('7')) => {
                self.switch_tab(MainTab::Config);
            }
            (KeyModifiers::NONE, KeyCode::Char('8')) => { /* already here */ }
            (KeyModifiers::NONE, KeyCode::Char('?') | KeyCode::F(1)) => {
                self.help.toggle();
            }
            _ => {}
        }
        Ok(())
    }

    // ── Message sending ───────────────────────────────────────────────────────

    fn send_message(&mut self, text: String) -> Result<()> {
        // Verify we have an API key before sending
        if self.resolved_api_key.is_none() {
            let warn = "⚠ No API key configured. Run `housaky onboard` or add keys via `housaky keys manager add-provider <name> <key>`";
            self.chat.push_system(warn.to_string());
            self.notifs.error("No API key! See chat for details.");
            return Ok(());
        }

        self.chat.push_user(text.clone());
        self.state.stream_status = StreamStatus::Thinking;
        self.state.stream_content.clear();
        self.state.metrics.total_messages += 1;
        self.state.metrics.total_requests += 1;
        self.sidebar.push_activity(
            ActivityKind::Thought,
            format!("User: {}", truncate_str(&text, 40)),
        );

        // Start streaming mode
        self.chat.start_streaming();
        self.state.stream_status = StreamStatus::Streaming;
        self.streaming_active.store(true, Ordering::SeqCst);
        {
            let mut result_guard = self.streaming_result.lock().unwrap();
            *result_guard = None;
        }

        // Clone what we need for the worker thread
        let provider_name = self.provider_name.clone();
        let model_name = self.model_name.clone();
        let api_key = self.resolved_api_key.clone();
        let streaming_result = Arc::clone(&self.streaming_result);
        let streaming_chunks = Arc::clone(&self.streaming_chunks);
        let reliability = self.config.reliability.clone();
        let agi_core = self.core.clone();
        let system_prompt = self.system_prompt.clone();
        let config = self.config.clone();

        // Build chat history for fallback path
        let mut chat_messages: Vec<ChatMessage> = vec![ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        }];
        chat_messages.extend(self.chat.messages.iter().map(|m| ChatMessage {
            role: m.role.api_role().to_string(),
            content: m.content.clone(),
        }));

        // Spawn a dedicated OS thread with its own tokio runtime to avoid
        // blocking the main TUI event loop.
        let streaming_result_panic = Arc::clone(&streaming_result);
        std::thread::spawn(move || {
            let thread_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let rt = match tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                {
                    Ok(r) => r,
                    Err(e) => {
                        if let Ok(mut g) = streaming_result.lock() {
                            *g = Some(Err(format!("Runtime error: {e}")));
                        }
                        return;
                    }
                };

                rt.block_on(async {
                    let provider = match crate::providers::create_resilient_provider(
                        &provider_name,
                        api_key.as_deref(),
                        &reliability,
                        None,
                    ) {
                        Ok(p) => p,
                        Err(e) => {
                            if let Ok(mut g) = streaming_result.lock() {
                                *g = Some(Err(format!("Provider error: {e}")));
                            }
                            return;
                        }
                    };

                    // ── Build FULL tool registry (memory, browser, git, etc.) ──
                    let security = std::sync::Arc::new(
                        crate::security::SecurityPolicy::from_config(
                            &config.autonomy,
                            &config.workspace_dir,
                        ),
                    );
                    let mem: std::sync::Arc<dyn crate::memory::Memory> =
                        match crate::memory::create_memory(
                            &config.memory,
                            &config.workspace_dir,
                            config.api_key.as_deref(),
                        ) {
                            Ok(m) => std::sync::Arc::from(m),
                            Err(_) => {
                                std::sync::Arc::new(crate::memory::NoneMemory) as std::sync::Arc<dyn crate::memory::Memory>
                            }
                        };
                    let (composio_key, composio_entity_id) = if config.composio.enabled {
                        (
                            config.composio.api_key.as_deref(),
                            Some(config.composio.entity_id.as_str()),
                        )
                    } else {
                        (None, None)
                    };
                    let tools_registry = crate::tools::all_tools(
                        &security,
                        mem.clone(),
                        composio_key,
                        composio_entity_id,
                        &config.browser,
                        &config.http_request,
                        &config.workspace_dir,
                        &config.agents,
                        config.api_key.as_deref(),
                        &config,
                    );

                    // ── AGI cognitive loop path ──────────────────────────
                    if let Some(ref core) = agi_core {
                        // Phase 1: Store user input in working memory for cross-turn context
                        let _ = core.working_memory.add(
                            &format!("User: {}", text),
                            crate::housaky::working_memory::MemoryImportance::High,
                            [("source".to_string(), "tui_input".to_string())]
                                .into_iter()
                                .collect(),
                        ).await;

                        // Phase 2: Stream perception phase
                        if let Ok(mut chunks) = streaming_chunks.lock() {
                            chunks.push("🔍 *Perceiving intent...*\n".to_string());
                        }

                        // Phase 3: Stream memory recall phase
                        let memory_hits = core.working_memory.search(&text, 3).await;
                        if !memory_hits.is_empty() {
                            let mem_summary = format!(
                                "📚 *Recalled {} relevant memories*\n",
                                memory_hits.len()
                            );
                            if let Ok(mut chunks) = streaming_chunks.lock() {
                                chunks.push(mem_summary);
                            }
                        }

                        // Phase 4: Stream goal context
                        let active_goals = core.goal_engine.get_active_goals().await;
                        if !active_goals.is_empty() {
                            let goals_summary = format!(
                                "🎯 *{} active goals guiding response*\n",
                                active_goals.len()
                            );
                            if let Ok(mut chunks) = streaming_chunks.lock() {
                                chunks.push(goals_summary);
                            }
                        }

                        // Phase 5: Stream thinking indicator
                        if let Ok(mut chunks) = streaming_chunks.lock() {
                            chunks.push("🧠 *Reasoning...*\n\n".to_string());
                        }

                        let tool_refs: Vec<&dyn crate::tools::Tool> =
                            tools_registry.iter().map(|t| t.as_ref()).collect();
                        let cog_result = core
                            .process_with_cognitive_loop(
                                &text,
                                provider.as_ref(),
                                &model_name,
                                &tool_refs,
                            )
                            .await;

                        match cog_result {
                            Ok(cog_response) => {
                                // Stream thoughts first (inner monologue visible to user)
                                if !cog_response.thoughts.is_empty() {
                                    let thoughts_block = format!(
                                        "💭 **Thoughts:**\n{}\n\n",
                                        cog_response
                                            .thoughts
                                            .iter()
                                            .map(|t| format!("  • {t}"))
                                            .collect::<Vec<_>>()
                                            .join("\n")
                                    );
                                    if let Ok(mut chunks) = streaming_chunks.lock() {
                                        chunks.push(thoughts_block);
                                    }
                                    std::thread::sleep(std::time::Duration::from_millis(50));
                                }

                                // Stream actions taken (tool calls, goal updates)
                                if !cog_response.actions_taken.is_empty() {
                                    let actions_block = format!(
                                        "⚡ **Actions:**\n{}\n\n",
                                        cog_response
                                            .actions_taken
                                            .iter()
                                            .map(|a| format!("  → {a}"))
                                            .collect::<Vec<_>>()
                                            .join("\n")
                                    );
                                    if let Ok(mut chunks) = streaming_chunks.lock() {
                                        chunks.push(actions_block);
                                    }
                                    std::thread::sleep(std::time::Duration::from_millis(50));
                                }

                                // Stream the main response content
                                let words: Vec<&str> =
                                    cog_response.content.split_whitespace().collect();
                                let chunk_size = (words.len() / 20).max(1);
                                for chunk in words.chunks(chunk_size) {
                                    let chunk_text = format!("{} ", chunk.join(" "));
                                    if let Ok(mut chunks) = streaming_chunks.lock() {
                                        chunks.push(chunk_text);
                                    }
                                    std::thread::sleep(std::time::Duration::from_millis(25));
                                }

                                // Append follow-up suggestions if any
                                if !cog_response.suggested_follow_ups.is_empty() {
                                    let follow_ups = format!(
                                        "\n\n💡 **Follow-ups:** {}",
                                        cog_response.suggested_follow_ups.join(" | ")
                                    );
                                    if let Ok(mut chunks) = streaming_chunks.lock() {
                                        chunks.push(follow_ups);
                                    }
                                }

                                // Append confidence indicator
                                let confidence_emoji = if cog_response.confidence > 0.8 {
                                    "🟢"
                                } else if cog_response.confidence > 0.5 {
                                    "🟡"
                                } else {
                                    "🔴"
                                };
                                let confidence_note = format!(
                                    "\n\n{} confidence: {:.0}%",
                                    confidence_emoji,
                                    cog_response.confidence * 100.0
                                );
                                if let Ok(mut chunks) = streaming_chunks.lock() {
                                    chunks.push(confidence_note);
                                }

                                // Record episodic memory for this turn (learning)
                                let _ep_id = core.episodic_memory.begin_episode(
                                    None,
                                    "tui_conversation",
                                ).await;
                                core.episodic_memory.record_event_with_outcome(
                                    crate::housaky::memory::episodic::EpisodicEventType::UserInteraction,
                                    &format!("Q: {}", text.chars().take(100).collect::<String>()),
                                    &format!("A: {}", cog_response.content.chars().take(200).collect::<String>()),
                                    cog_response.confidence,
                                ).await;
                                let _ = core.episodic_memory.end_episode(
                                    crate::housaky::memory::emotional_tags::EmotionalTag::neutral(),
                                    cog_response.confidence > 0.5,
                                ).await;

                                // Store response in working memory for future recall
                                let _ = core.working_memory.add(
                                    &format!("Assistant: {}", cog_response.content.chars().take(300).collect::<String>()),
                                    crate::housaky::working_memory::MemoryImportance::Normal,
                                    [("source".to_string(), "tui_response".to_string())]
                                        .into_iter()
                                        .collect(),
                                ).await;

                                if let Ok(mut g) = streaming_result.lock() {
                                    *g = Some(Ok(cog_response.content));
                                }
                            }
                            Err(_e) => {
                                // AGI core failed — fall back to agentic tool loop
                                let fallback_note =
                                    "⚠ AGI core error, falling back to agentic tool loop...\n\n";
                                if let Ok(mut chunks) = streaming_chunks.lock() {
                                    chunks.push(fallback_note.to_string());
                                }

                                Self::run_fallback_tool_loop(
                                    provider.as_ref(),
                                    &mut chat_messages,
                                    &tools_registry,
                                    &provider_name,
                                    &model_name,
                                    &streaming_chunks,
                                    &streaming_result,
                                )
                                .await;
                            }
                        }
                    } else {
                        // ── Fallback: agentic tool loop (no AGI core) ────
                        // Enrich user message with memory context before sending
                        let mem_context = crate::agent::loop_::build_context(
                            mem.as_ref(),
                            &text,
                        ).await;
                        if !mem_context.is_empty() {
                            // Inject memory context into the last user message
                            if let Some(last) = chat_messages.last_mut() {
                                if last.role == "user" {
                                    last.content = format!("{mem_context}{}", last.content);
                                }
                            }
                            if let Ok(mut chunks) = streaming_chunks.lock() {
                                chunks.push("📚 *Memory context loaded*\n".to_string());
                            }
                        }

                        // Auto-save user message to persistent memory
                        if config.memory.auto_save {
                            let _ = mem.store(
                                &format!("tui_user_{}", chrono::Utc::now().timestamp()),
                                &text,
                                crate::memory::MemoryCategory::Conversation,
                            ).await;
                        }

                        Self::run_fallback_tool_loop(
                            provider.as_ref(),
                            &mut chat_messages,
                            &tools_registry,
                            &provider_name,
                            &model_name,
                            &streaming_chunks,
                            &streaming_result,
                        )
                        .await;

                        // Auto-save assistant response to memory
                        if config.memory.auto_save {
                            if let Ok(guard) = streaming_result.lock() {
                                if let Some(Ok(ref response)) = *guard {
                                    let summary = &response[..response.len().min(200)];
                                    let _ = mem.store(
                                        &format!("tui_resp_{}", chrono::Utc::now().timestamp()),
                                        summary,
                                        crate::memory::MemoryCategory::Daily,
                                    ).await;
                                }
                            }
                        }
                    }
                });
            }));

            // If the thread panicked, report it as an error in the chat pane
            if let Err(panic_val) = thread_result {
                let msg = if let Some(s) = panic_val.downcast_ref::<&str>() {
                    format!("Provider error: {s}")
                } else if let Some(s) = panic_val.downcast_ref::<String>() {
                    format!("Provider error: {s}")
                } else {
                    "Provider error: unexpected panic in send thread".to_string()
                };
                if let Ok(mut g) = streaming_result_panic.lock() {
                    if g.is_none() {
                        *g = Some(Err(msg));
                    }
                }
            }
        });

        Ok(())
    }

    /// Shared fallback: run the agentic tool loop and stream results.
    async fn run_fallback_tool_loop(
        provider: &dyn crate::providers::Provider,
        chat_messages: &mut Vec<ChatMessage>,
        tools_registry: &[Box<dyn crate::tools::Tool>],
        provider_name: &str,
        model_name: &str,
        streaming_chunks: &Arc<std::sync::Mutex<Vec<String>>>,
        streaming_result: &Arc<std::sync::Mutex<Option<Result<String, String>>>>,
    ) {
        // Append tool instructions so the LLM knows how to
        // invoke tools via <tool_call> tags.
        if let Some(sys) = chat_messages.first_mut() {
            sys.content.push_str(
                &crate::agent::loop_::build_tool_instructions(tools_registry),
            );
        }

        let observer = crate::observability::NoopObserver;
        let result = crate::agent::loop_::run_tool_call_loop(
            provider,
            chat_messages,
            tools_registry,
            &observer,
            None,
            provider_name,
            model_name,
            0.7,
            true, // silent — we stream ourselves
            10,   // max tool iterations
        )
        .await;

        match result {
            Ok(response) => {
                // Stream the final response word-by-word
                let words: Vec<&str> = response.split_whitespace().collect();
                let chunk_size = (words.len() / 20).max(1);
                for chunk in words.chunks(chunk_size) {
                    let chunk_text = format!("{} ", chunk.join(" "));
                    if let Ok(mut chunks) = streaming_chunks.lock() {
                        chunks.push(chunk_text);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
                if let Ok(mut g) = streaming_result.lock() {
                    *g = Some(Ok(response));
                }
            }
            Err(e) => {
                if let Ok(mut g) = streaming_result.lock() {
                    *g = Some(Err(e.to_string()));
                }
            }
        }
    }

    /// Raw LLM call fallback (no AGI core)
    async fn send_raw_llm(
        provider: &dyn crate::providers::Provider,
        chat_messages: &[ChatMessage],
        model_name: &str,
        streaming_chunks: &Arc<std::sync::Mutex<Vec<String>>>,
        streaming_result: &Arc<std::sync::Mutex<Option<Result<String, String>>>>,
    ) {
        let result = provider
            .chat_with_history(chat_messages, model_name, 0.7)
            .await;

        match result {
            Ok(response) => {
                let words: Vec<&str> = response.split_whitespace().collect();
                let chunk_size = (words.len() / 20).max(1);
                for chunk in words.chunks(chunk_size) {
                    let chunk_text = format!("{} ", chunk.join(" "));
                    if let Ok(mut chunks) = streaming_chunks.lock() {
                        chunks.push(chunk_text);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
                if let Ok(mut g) = streaming_result.lock() {
                    *g = Some(Ok(response));
                }
            }
            Err(e) => {
                let msg = e.to_string();
                let display =
                    if msg.contains("429") || msg.contains("rate") || msg.contains("limit") {
                        format!(
                            "Rate limited (auto-rotating keys). {}",
                            &msg[..msg.len().min(120)]
                        )
                    } else {
                        msg[..msg.len().min(200)].to_string()
                    };
                if let Ok(mut g) = streaming_result.lock() {
                    *g = Some(Err(display));
                }
            }
        }
    }

    // ── Export ────────────────────────────────────────────────────────────────

    fn export_chat(&mut self) -> Result<()> {
        let content = self.chat.export_markdown();
        let filename = format!(
            "housaky_chat_{}.md",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        );
        match std::fs::write(&filename, &content) {
            Ok(()) => {
                self.notifs.success(format!("Saved → {}", filename));
            }
            Err(e) => {
                self.notifs.error(format!("Save failed: {}", e));
            }
        }
        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row(label: &'static str, value: String, color: ratatui::style::Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<14}", label), style_muted()),
        Span::styled(value, ratatui::style::Style::default().fg(color)),
    ])
}


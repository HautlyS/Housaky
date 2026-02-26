use std::time::Instant;

// ── Input mode ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Insert,
    Command,
    Search,
}

impl InputMode {
    pub fn is_typing(&self) -> bool {
        matches!(self, InputMode::Insert | InputMode::Command | InputMode::Search)
    }
}

// ── Active pane (keyboard focus) ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    Chat,
    Sidebar,
    Input,
    CommandPalette,
    SkillsPanel,
    ToolsPanel,
    Help,
}

// ── Main tab ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainTab {
    Chat = 0,
    Skills = 1,
    Tools = 2,
    Goals = 3,
    Metrics = 4,
    Config = 5,
}

impl MainTab {
    pub const ALL: &'static [MainTab] = &[
        MainTab::Chat,
        MainTab::Skills,
        MainTab::Tools,
        MainTab::Goals,
        MainTab::Metrics,
        MainTab::Config,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            MainTab::Chat    => " Chat ",
            MainTab::Skills  => " Skills ",
            MainTab::Tools   => " Tools ",
            MainTab::Goals   => " Goals ",
            MainTab::Metrics => " Metrics ",
            MainTab::Config  => " Config ",
        }
    }

    pub fn index(&self) -> usize {
        *self as usize
    }

    pub fn from_index(i: usize) -> MainTab {
        match i {
            0 => MainTab::Chat,
            1 => MainTab::Skills,
            2 => MainTab::Tools,
            3 => MainTab::Goals,
            4 => MainTab::Metrics,
            5 => MainTab::Config,
            _ => MainTab::Chat,
        }
    }

    pub fn next(&self) -> MainTab {
        MainTab::from_index((self.index() + 1) % MainTab::ALL.len())
    }

    pub fn prev(&self) -> MainTab {
        let len = MainTab::ALL.len();
        MainTab::from_index((self.index() + len - 1) % len)
    }
}

// ── View layout ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Full,          // Full-width chat, no sidebar
    Split,         // Chat + sidebar (default)
    Dashboard,     // Wide sidebar / metrics focus
}

impl ViewMode {
    pub fn cycle(&self) -> ViewMode {
        match self {
            ViewMode::Full      => ViewMode::Split,
            ViewMode::Split     => ViewMode::Dashboard,
            ViewMode::Dashboard => ViewMode::Full,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ViewMode::Full      => "FULL",
            ViewMode::Split     => "SPLIT",
            ViewMode::Dashboard => "DASH",
        }
    }
}

// ── Streaming state ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamStatus {
    Idle,
    Thinking,
    Streaming,
    Done,
    Error(String),
}

impl StreamStatus {
    pub fn is_busy(&self) -> bool {
        matches!(self, StreamStatus::Thinking | StreamStatus::Streaming)
    }
}

// ── Per-session AGI metrics ───────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct SessionMetrics {
    pub total_messages: usize,
    pub total_tokens_in: u64,
    pub total_tokens_out: u64,
    pub total_requests: u64,
    pub total_errors: u64,
    pub session_start: Option<Instant>,
    pub last_latency_ms: u64,
    pub avg_tokens_per_sec: f64,
    pub goals_active: usize,
    pub skills_enabled: usize,
    pub tools_invoked: u64,
}

impl SessionMetrics {
    pub fn new() -> Self {
        Self {
            session_start: Some(Instant::now()),
            ..Default::default()
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        self.session_start
            .map(|s| s.elapsed().as_secs())
            .unwrap_or(0)
    }

    pub fn format_uptime(&self) -> String {
        let s = self.uptime_secs();
        let h = s / 3600;
        let m = (s % 3600) / 60;
        let sec = s % 60;
        if h > 0 {
            format!("{h}h {m:02}m")
        } else if m > 0 {
            format!("{m}m {sec:02}s")
        } else {
            format!("{sec}s")
        }
    }

    pub fn error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.total_errors as f64 / self.total_requests as f64
        }
    }
}

// ── Tool execution entry ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ToolEntry {
    pub id: usize,
    pub name: String,
    pub status: ToolStatus,
    pub input_summary: String,
    pub output_summary: Option<String>,
    pub duration_ms: Option<u64>,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolStatus {
    Running,
    Success,
    Failed,
    Cancelled,
}

impl ToolStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            ToolStatus::Running   => "⟳",
            ToolStatus::Success   => "✓",
            ToolStatus::Failed    => "✗",
            ToolStatus::Cancelled => "⊘",
        }
    }
}

// ── Global app state ──────────────────────────────────────────────────────────

pub struct AppState {
    pub input_mode:        InputMode,
    pub active_pane:       ActivePane,
    pub active_tab:        MainTab,
    pub view_mode:         ViewMode,
    pub stream_status:     StreamStatus,
    pub metrics:           SessionMetrics,
    pub tool_log:          Vec<ToolEntry>,
    pub next_tool_id:      usize,
    pub spinner_frame:     usize,
    pub last_tick:         Instant,
    pub should_quit:       bool,
    pub show_help:         bool,
    pub show_command_palette: bool,
    pub show_search:       bool,
    pub sidebar_visible:   bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            input_mode:           InputMode::Insert,
            active_pane:          ActivePane::Input,
            active_tab:           MainTab::Chat,
            view_mode:            ViewMode::Split,
            stream_status:        StreamStatus::Idle,
            metrics:              SessionMetrics::new(),
            tool_log:             Vec::new(),
            next_tool_id:         0,
            spinner_frame:        0,
            last_tick:            Instant::now(),
            should_quit:          false,
            show_help:            false,
            show_command_palette: false,
            show_search:          false,
            sidebar_visible:      true,
        }
    }

    pub fn tick(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % crate::tui::enhanced_app::theme::SPINNER_FRAMES.len();
        self.last_tick = Instant::now();
    }

    pub fn spinner(&self) -> &'static str {
        crate::tui::enhanced_app::theme::SPINNER_FRAMES[self.spinner_frame]
    }

    pub fn push_tool(&mut self, name: String, input_summary: String) -> usize {
        use chrono::Local;
        let id = self.next_tool_id;
        self.next_tool_id += 1;
        self.tool_log.push(ToolEntry {
            id,
            name,
            status: ToolStatus::Running,
            input_summary,
            output_summary: None,
            duration_ms: None,
            timestamp: Local::now().format("%H:%M:%S").to_string(),
        });
        id
    }

    pub fn finish_tool(&mut self, id: usize, success: bool, output: Option<String>, duration_ms: u64) {
        if let Some(entry) = self.tool_log.iter_mut().find(|e| e.id == id) {
            entry.status = if success { ToolStatus::Success } else { ToolStatus::Failed };
            entry.output_summary = output;
            entry.duration_ms = Some(duration_ms);
            self.metrics.tools_invoked += 1;
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

use ratatui::style::{Color, Modifier, Style};

// ═══════════════════════════════════════════════════════════════════════════════
// 2077 NIGHT CITY CYBERPUNK THEME
// Aggressive neons, deep blacks, corporate tech aesthetic
// ═══════════════════════════════════════════════════════════════════════════════

pub struct Palette;

impl Palette {
    // ─── Backgrounds ────────────────────────────────────────────────────────────
    // Deep void blacks with subtle purple/cyan undertones
    pub const BG: Color = Color::Rgb(2, 3, 8);
    pub const BG_PANEL: Color = Color::Rgb(8, 10, 18);
    pub const BG_ELEVATED: Color = Color::Rgb(15, 18, 32);
    pub const BG_SELECTED: Color = Color::Rgb(25, 20, 45);
    pub const BG_HOVER: Color = Color::Rgb(18, 22, 38);
    pub const BG_INPUT: Color = Color::Rgb(5, 7, 15);

    // ─── Corporate Brand Colors ────────────────────────────────────────────────
    // Arasaka-inspired: cyan, red accents
    pub const CYAN: Color = Color::Rgb(0, 220, 235);
    pub const CYAN_DIM: Color = Color::Rgb(0, 160, 170);
    pub const CYAN_DARK: Color = Color::Rgb(0, 100, 110);

    pub const PINK: Color = Color::Rgb(255, 0, 110);
    pub const PINK_DIM: Color = Color::Rgb(200, 0, 85);
    pub const RED: Color = Color::Rgb(255, 50, 70);
    pub const RED_BRIGHT: Color = Color::Rgb(255, 80, 90);

    // ─── Accent Colors ────────────────────────────────────────────────────────
    pub const VIOLET: Color = Color::Rgb(145, 45, 255);
    pub const VIOLET_DIM: Color = Color::Rgb(100, 30, 180);
    pub const YELLOW: Color = Color::Rgb(255, 220, 0);
    pub const YELLOW_DIM: Color = Color::Rgb(200, 170, 0);
    pub const ORANGE: Color = Color::Rgb(255, 130, 0);
    pub const MAGENTA: Color = Color::Rgb(255, 0, 220);

    // ─── Semantic Colors ──────────────────────────────────────────────────────
    pub const SUCCESS: Color = Color::Rgb(0, 255, 130);
    pub const SUCCESS_DIM: Color = Color::Rgb(0, 200, 100);
    pub const WARNING: Color = Color::Rgb(255, 200, 20);
    pub const ERROR: Color = Color::Rgb(255, 60, 80);
    pub const ERROR_BRIGHT: Color = Color::Rgb(255, 100, 120);
    pub const INFO: Color = Color::Rgb(0, 180, 255);

    // ─── Text Colors ───────────────────────────────────────────────────────────
    pub const TEXT: Color = Color::Rgb(235, 240, 248);
    pub const TEXT_DIM: Color = Color::Rgb(130, 140, 165);
    pub const TEXT_MUTED: Color = Color::Rgb(80, 90, 115);
    pub const TEXT_BRIGHT: Color = Color::Rgb(255, 255, 255);

    // ─── Role Colors ─────────────────────────────────────────────────────────
    pub const USER: Color = Color::Rgb(0, 255, 195);
    pub const ASSISTANT: Color = Color::Rgb(170, 90, 255);
    pub const SYSTEM: Color = Color::Rgb(255, 175, 50);
    pub const TOOL: Color = Color::Rgb(255, 95, 255);

    // ─── AGI/Tech Tags ──────────────────────────────────────────────────────
    pub const GOAL: Color = Color::Rgb(255, 110, 60);
    pub const SKILL: Color = Color::Rgb(50, 255, 175);
    pub const THOUGHT: Color = Color::Rgb(175, 145, 255);
    pub const MEMORY: Color = Color::Rgb(95, 175, 255);

    // ─── Borders ─────────────────────────────────────────────────────────────
    pub const BORDER: Color = Color::Rgb(25, 30, 55);
    pub const BORDER_FOCUS: Color = Color::Rgb(0, 220, 235);
    pub const BORDER_ACTIVE: Color = Color::Rgb(255, 0, 110);
    pub const BORDER_DIM: Color = Color::Rgb(20, 25, 45);

    // ─── Code Blocks ───────────────────────────────────────────────────────
    pub const CODE_BG: Color = Color::Rgb(12, 15, 28);
    pub const CODE_FG: Color = Color::Rgb(0, 230, 140);
    pub const CODE_HEADER: Color = Color::Rgb(30, 35, 60);
}

// ═══════════════════════════════════════════════════════════════════════════════
// STYLE HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

pub fn style_base() -> Style {
    Style::default().fg(Palette::TEXT).bg(Palette::BG)
}

pub fn style_panel() -> Style {
    Style::default().fg(Palette::TEXT).bg(Palette::BG_PANEL)
}

pub fn style_border() -> Style {
    Style::default().fg(Palette::BORDER)
}

pub fn style_border_focus() -> Style {
    Style::default().fg(Palette::BORDER_FOCUS)
}

pub fn style_border_active() -> Style {
    Style::default().fg(Palette::BORDER_ACTIVE)
}

// ═══════════════════════════════════════════════════════════════════════════════
// TITLE STYLES
// ═══════════════════════════════════════════════════════════════════════════════

pub fn style_title() -> Style {
    Style::default()
        .fg(Palette::CYAN)
        .add_modifier(Modifier::BOLD)
}

pub fn style_title_2077() -> Style {
    Style::default()
        .fg(Palette::PINK)
        .add_modifier(Modifier::BOLD)
}

pub fn style_title_dim() -> Style {
    Style::default().fg(Palette::CYAN_DIM)
}

pub fn style_title_subtle() -> Style {
    Style::default()
        .fg(Palette::TEXT_MUTED)
        .add_modifier(Modifier::DIM)
}

// ═══════════════════════════════════════════════════════════════════════════════
// SELECTION & INTERACTION
// ═══════════════════════════════════════════════════════════════════════════════

pub fn style_selected() -> Style {
    Style::default()
        .fg(Palette::TEXT_BRIGHT)
        .bg(Palette::BG_SELECTED)
        .add_modifier(Modifier::BOLD)
}

pub fn style_hover() -> Style {
    Style::default().fg(Palette::TEXT).bg(Palette::BG_HOVER)
}

pub fn style_active() -> Style {
    Style::default()
        .fg(Palette::BG)
        .bg(Palette::CYAN)
        .add_modifier(Modifier::BOLD)
}

// ═══════════════════════════════════════════════════════════════════════════════
// MESSAGE ROLES
// ═══════════════════════════════════════════════════════════════════════════════

pub fn style_user_msg() -> Style {
    Style::default().fg(Palette::USER)
}

pub fn style_user_badge() -> Style {
    Style::default()
        .fg(Palette::BG)
        .bg(Palette::USER)
        .add_modifier(Modifier::BOLD)
}

pub fn style_assistant_msg() -> Style {
    Style::default().fg(Palette::ASSISTANT)
}

pub fn style_assistant_badge() -> Style {
    Style::default()
        .fg(Palette::BG)
        .bg(Palette::ASSISTANT)
        .add_modifier(Modifier::BOLD)
}

pub fn style_system_msg() -> Style {
    Style::default().fg(Palette::SYSTEM)
}

pub fn style_system_badge() -> Style {
    Style::default()
        .fg(Palette::BG)
        .bg(Palette::SYSTEM)
        .add_modifier(Modifier::BOLD)
}

// ═══════════════════════════════════════════════════════════════════════════════
// SEMANTIC STYLES
// ═══════════════════════════════════════════════════════════════════════════════

pub fn style_success() -> Style {
    Style::default().fg(Palette::SUCCESS)
}

pub fn style_success_bold() -> Style {
    Style::default()
        .fg(Palette::SUCCESS)
        .add_modifier(Modifier::BOLD)
}

pub fn style_warning() -> Style {
    Style::default().fg(Palette::WARNING)
}

pub fn style_warning_bold() -> Style {
    Style::default()
        .fg(Palette::WARNING)
        .add_modifier(Modifier::BOLD)
}

pub fn style_error() -> Style {
    Style::default().fg(Palette::ERROR)
}

pub fn style_error_bold() -> Style {
    Style::default()
        .fg(Palette::ERROR)
        .add_modifier(Modifier::BOLD)
}

pub fn style_info() -> Style {
    Style::default().fg(Palette::INFO)
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEXT UTILITY
// ═══════════════════════════════════════════════════════════════════════════════

pub fn style_dim() -> Style {
    Style::default().fg(Palette::TEXT_DIM)
}

pub fn style_muted() -> Style {
    Style::default().fg(Palette::TEXT_MUTED)
}

pub fn style_bright() -> Style {
    Style::default().fg(Palette::TEXT_BRIGHT)
}

// ═══════════════════════════════════════════════════════════════════════════════
// CODE STYLES
// ═══════════════════════════════════════════════════════════════════════════════

pub fn style_code_inline() -> Style {
    Style::default().fg(Palette::CODE_FG).bg(Palette::CODE_BG)
}

pub fn style_code_block() -> Style {
    Style::default().fg(Palette::TEXT).bg(Palette::CODE_BG)
}

pub fn style_code_header() -> Style {
    Style::default()
        .fg(Palette::CYAN)
        .bg(Palette::CODE_HEADER)
        .add_modifier(Modifier::BOLD)
}

// ═══════════════════════════════════════════════════════════════════════════════
// TABS
// ═══════════════════════════════════════════════════════════════════════════════

pub fn style_tab_active() -> Style {
    Style::default()
        .fg(Palette::BG)
        .bg(Palette::CYAN)
        .add_modifier(Modifier::BOLD)
}

pub fn style_tab_inactive() -> Style {
    Style::default().fg(Palette::TEXT_DIM)
}

pub fn style_tab_hover() -> Style {
    Style::default().fg(Palette::TEXT).bg(Palette::BG_HOVER)
}

// ═══════════════════════════════════════════════════════════════════════════════
// TAGS (Goal, Skill, Tool, Thought)
// ═══════════════════════════════════════════════════════════════════════════════

pub fn style_tag_goal() -> Style {
    Style::default()
        .fg(Palette::GOAL)
        .add_modifier(Modifier::BOLD)
}

pub fn style_tag_skill() -> Style {
    Style::default().fg(Palette::SKILL)
}

pub fn style_tag_tool() -> Style {
    Style::default().fg(Palette::TOOL)
}

pub fn style_tag_thought() -> Style {
    Style::default()
        .fg(Palette::THOUGHT)
        .add_modifier(Modifier::DIM)
}

pub fn style_tag_memory() -> Style {
    Style::default().fg(Palette::MEMORY)
}

// ═══════════════════════════════════════════════════════════════════════════════
// INPUT
// ═══════════════════════════════════════════════════════════════════════════════

pub fn style_input_focused() -> Style {
    Style::default().fg(Palette::TEXT_BRIGHT)
}

pub fn style_input_idle() -> Style {
    Style::default().fg(Palette::TEXT_DIM)
}

pub fn style_input_cursor() -> Style {
    Style::default()
        .fg(Palette::CYAN)
        .add_modifier(Modifier::RAPID_BLINK)
}

pub fn style_streaming_cursor() -> Style {
    Style::default()
        .fg(Palette::CYAN)
        .add_modifier(Modifier::SLOW_BLINK)
}

// ═══════════════════════════════════════════════════════════════════════════════
// STATUS INDICATORS
// ═══════════════════════════════════════════════════════════════════════════════

pub fn style_status_ok() -> Style {
    Style::default()
        .fg(Palette::SUCCESS)
        .add_modifier(Modifier::BOLD)
}

pub fn style_status_loading() -> Style {
    Style::default()
        .fg(Palette::PINK)
        .add_modifier(Modifier::BOLD)
}

pub fn style_status_err() -> Style {
    Style::default()
        .fg(Palette::ERROR)
        .add_modifier(Modifier::BOLD)
}

pub fn style_status_idle() -> Style {
    Style::default()
        .fg(Palette::TEXT_MUTED)
        .add_modifier(Modifier::DIM)
}

// ═══════════════════════════════════════════════════════════════════════════════
// KEYBINDS
// ═══════════════════════════════════════════════════════════════════════════════

pub fn style_keybind() -> Style {
    Style::default()
        .fg(Palette::CYAN)
        .add_modifier(Modifier::BOLD)
}

pub fn style_keybind_dark() -> Style {
    Style::default()
        .fg(Palette::CYAN_DIM)
        .add_modifier(Modifier::BOLD)
}

pub fn style_keybind_label() -> Style {
    Style::default().fg(Palette::TEXT_DIM)
}

// ═══════════════════════════════════════════════════════════════════════════════
// TOAST NOTIFICATIONS
// ═══════════════════════════════════════════════════════════════════════════════

pub fn style_toast_info() -> Style {
    Style::default()
        .fg(Palette::BG)
        .bg(Palette::CYAN)
        .add_modifier(Modifier::BOLD)
}

pub fn style_toast_success() -> Style {
    Style::default()
        .fg(Palette::BG)
        .bg(Palette::SUCCESS)
        .add_modifier(Modifier::BOLD)
}

pub fn style_toast_error() -> Style {
    Style::default()
        .fg(Palette::TEXT_BRIGHT)
        .bg(Palette::ERROR)
        .add_modifier(Modifier::BOLD)
}

pub fn style_toast_warn() -> Style {
    Style::default()
        .fg(Palette::BG)
        .bg(Palette::WARNING)
        .add_modifier(Modifier::BOLD)
}

// ═══════════════════════════════════════════════════════════════════════════════
// PROGRESS BARS
// ═══════════════════════════════════════════════════════════════════════════════

pub fn style_gauge_filled() -> Style {
    Style::default().fg(Palette::CYAN)
}

pub fn style_gauge_empty() -> Style {
    Style::default().fg(Palette::BORDER)
}

pub fn style_gauge_warning() -> Style {
    Style::default().fg(Palette::WARNING)
}

pub fn style_gauge_error() -> Style {
    Style::default().fg(Palette::ERROR)
}

// ═══════════════════════════════════════════════════════════════════════════════
// BADGES
// ═══════════════════════════════════════════════════════════════════════════════

pub fn style_badge_success() -> Style {
    Style::default()
        .fg(Palette::SUCCESS)
        .add_modifier(Modifier::BOLD)
}

pub fn style_badge_warning() -> Style {
    Style::default()
        .fg(Palette::WARNING)
        .add_modifier(Modifier::BOLD)
}

pub fn style_badge_error() -> Style {
    Style::default()
        .fg(Palette::ERROR)
        .add_modifier(Modifier::BOLD)
}

pub fn style_badge_info() -> Style {
    Style::default()
        .fg(Palette::INFO)
        .add_modifier(Modifier::BOLD)
}

// ═══════════════════════════════════════════════════════════════════════════════
// BRANDING & ANIMATION
// ═══════════════════════════════════════════════════════════════════════════════

pub const LOGO: &str = "◈ HOUSAKY";
pub const LOGO_2077: &str = "◆ HOUSAKY";
pub const LOGO_COMPACT: &str = "◆";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Scanning line animation for headers
pub const SCANLINE_FRAMES: &[&str] = &[
    "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃", "▂",
];

// Processing spinner - tech feel
pub const SPINNER_FRAMES: &[&str] = &["◐", "◑", "◒", "◓", "◉", "◈"];

// Glitch effect frames
pub const GLITCH_FRAMES: &[&str] = &["", "̴", "̵", "̶", "̷"];

// Connection dots
pub const CONN_FRAMES: &[&str] = &["○", "◔", "◑", "◕", "◉"];

// ═══════════════════════════════════════════════════════════════════════════════
// PROGRESS BARS
// ═══════════════════════════════════════════════════════════════════════════════

pub fn render_bar(value: f64, width: usize, filled_char: &str, empty_char: &str) -> String {
    let clamped = value.clamp(0.0, 1.0);
    let filled_count = (clamped * width as f64).round() as usize;
    let empty_count = width.saturating_sub(filled_count);
    format!(
        "{}{}",
        filled_char.repeat(filled_count),
        empty_char.repeat(empty_count)
    )
}

pub fn render_gauge_bar(value: f64, width: usize) -> String {
    render_bar(value, width, "█", "░")
}

pub fn render_cyber_bar(value: f64, width: usize) -> String {
    render_bar(value, width, "▓", "▒")
}

pub fn render_dotted_bar(value: f64, width: usize) -> String {
    render_bar(value, width, "●", "○")
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEXT HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        let end = s
            .char_indices()
            .nth(max.saturating_sub(1))
            .map(|(i, _)| i)
            .unwrap_or(s.len());
        format!("{}…", &s[..end])
    }
}

pub fn truncate_str(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    match s.char_indices().nth(max) {
        Some((byte_idx, _)) => &s[..byte_idx],
        None => s,
    }
}

// Pad string to fixed width
pub fn pad(s: &str, width: usize) -> String {
    let len = s.len();
    if len >= width {
        truncate(s, width)
    } else {
        format!("{}{}", s, " ".repeat(width - len))
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ICONS (Cyberpunk-themed)
// ═══════════════════════════════════════════════════════════════════════════════

pub const ICON_USER: &str = "⊕";
pub const ICON_ASSISTANT: &str = "⊖";
pub const ICON_SYSTEM: &str = "⊛";
pub const ICON_TOOL: &str = "⚡";
pub const ICON_SKILL: &str = "◈";
pub const ICON_GOAL: &str = "◆";
pub const ICON_THOUGHT: &str = "◇";
pub const ICON_MEMORY: &str = "◉";
pub const ICON_SUCCESS: &str = "✓";
pub const ICON_ERROR: &str = "✗";
pub const ICON_WARNING: &str = "⚠";
pub const ICON_INFO: &str = "ℹ";
pub const ICON_SEARCH: &str = "⌕";
pub const ICON_CONFIG: &str = "⚙";
pub const ICON_SAVE: &str = "▼";
pub const ICON_LOAD: &str = "▲";
pub const ICON_PLAY: &str = "▶";
pub const ICON_STOP: &str = "■";
pub const ICON_PAUSE: &str = "⏸";

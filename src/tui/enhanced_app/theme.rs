use ratatui::style::{Color, Modifier, Style};

// ── 2026 Housaky Palette ────────────────────────────────────────────────────
// Deep-space base with neon accent layers, inspired by claude-code / openclaw
// aesthetic: midnight bg, electric cyan primary, violet secondary, warm amber.

pub struct Palette;

impl Palette {
    // Backgrounds
    pub const BG: Color = Color::Rgb(8, 10, 18);
    pub const BG_PANEL: Color = Color::Rgb(13, 16, 28);
    pub const BG_ELEVATED: Color = Color::Rgb(20, 24, 40);
    pub const BG_SELECTED: Color = Color::Rgb(25, 35, 65);
    pub const BG_HOVER: Color = Color::Rgb(18, 28, 50);

    // Brand / Primary
    pub const CYAN: Color = Color::Rgb(0, 210, 240);
    pub const CYAN_DIM: Color = Color::Rgb(0, 140, 160);
    pub const VIOLET: Color = Color::Rgb(140, 80, 255);
    pub const VIOLET_DIM: Color = Color::Rgb(90, 50, 170);

    // Semantic
    pub const SUCCESS: Color = Color::Rgb(50, 220, 130);
    pub const WARNING: Color = Color::Rgb(255, 185, 40);
    pub const ERROR: Color = Color::Rgb(255, 70, 80);
    pub const INFO: Color = Color::Rgb(90, 160, 255);

    // Text - higher contrast white
    pub const TEXT: Color = Color::Rgb(235, 240, 250);
    pub const TEXT_DIM: Color = Color::Rgb(160, 170, 190);
    pub const TEXT_MUTED: Color = Color::Rgb(110, 120, 140);
    pub const TEXT_BRIGHT: Color = Color::Rgb(255, 255, 255);

    // Role colours
    pub const USER: Color = Color::Rgb(80, 200, 120);
    pub const ASSISTANT: Color = Color::Rgb(80, 170, 255);
    pub const SYSTEM: Color = Color::Rgb(200, 160, 80);

    // AGI tags
    pub const GOAL: Color = Color::Rgb(255, 160, 60);
    pub const SKILL: Color = Color::Rgb(60, 220, 180);
    pub const TOOL: Color = Color::Rgb(180, 100, 255);
    pub const THOUGHT: Color = Color::Rgb(180, 140, 255);

    // Borders
    pub const BORDER: Color = Color::Rgb(35, 45, 75);
    pub const BORDER_FOCUS: Color = Color::Rgb(0, 210, 240);
    pub const BORDER_ACTIVE: Color = Color::Rgb(140, 80, 255);

    // Inline code
    pub const CODE_BG: Color = Color::Rgb(22, 28, 45);
    pub const CODE_FG: Color = Color::Rgb(255, 130, 100);
}

// ── Composite style helpers ──────────────────────────────────────────────────

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

pub fn style_title() -> Style {
    Style::default()
        .fg(Palette::CYAN)
        .add_modifier(Modifier::BOLD)
}

pub fn style_title_dim() -> Style {
    Style::default().fg(Palette::CYAN_DIM)
}

pub fn style_selected() -> Style {
    Style::default()
        .fg(Palette::TEXT_BRIGHT)
        .bg(Palette::BG_SELECTED)
        .add_modifier(Modifier::BOLD)
}

pub fn style_user_msg() -> Style {
    Style::default().fg(Palette::USER)
}

pub fn style_assistant_msg() -> Style {
    Style::default().fg(Palette::ASSISTANT)
}

pub fn style_system_msg() -> Style {
    Style::default().fg(Palette::SYSTEM)
}

pub fn style_success() -> Style {
    Style::default().fg(Palette::SUCCESS)
}

pub fn style_warning() -> Style {
    Style::default().fg(Palette::WARNING)
}

pub fn style_error() -> Style {
    Style::default().fg(Palette::ERROR)
}

pub fn style_info() -> Style {
    Style::default().fg(Palette::INFO)
}

pub fn style_dim() -> Style {
    Style::default().fg(Palette::TEXT_DIM)
}

pub fn style_muted() -> Style {
    Style::default().fg(Palette::TEXT_MUTED)
}

pub fn style_code_inline() -> Style {
    Style::default().fg(Palette::CODE_FG).bg(Palette::CODE_BG)
}

pub fn style_code_block() -> Style {
    Style::default().fg(Palette::TEXT).bg(Palette::CODE_BG)
}

pub fn style_tab_active() -> Style {
    Style::default()
        .fg(Palette::BG)
        .bg(Palette::CYAN)
        .add_modifier(Modifier::BOLD)
}

pub fn style_tab_inactive() -> Style {
    Style::default().fg(Palette::TEXT_DIM)
}

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

pub fn style_input_focused() -> Style {
    Style::default().fg(Palette::TEXT_BRIGHT)
}

pub fn style_input_idle() -> Style {
    Style::default().fg(Palette::TEXT_DIM)
}

pub fn style_streaming_cursor() -> Style {
    Style::default()
        .fg(Palette::CYAN)
        .add_modifier(Modifier::SLOW_BLINK)
}

pub fn style_status_ok() -> Style {
    Style::default().fg(Palette::SUCCESS)
}

pub fn style_status_loading() -> Style {
    Style::default()
        .fg(Palette::WARNING)
        .add_modifier(Modifier::BOLD)
}

pub fn style_status_err() -> Style {
    Style::default()
        .fg(Palette::ERROR)
        .add_modifier(Modifier::BOLD)
}

pub fn style_keybind() -> Style {
    Style::default()
        .fg(Palette::CYAN)
        .add_modifier(Modifier::BOLD)
}

pub fn style_keybind_label() -> Style {
    Style::default().fg(Palette::TEXT_DIM)
}

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

// ── Branding ─────────────────────────────────────────────────────────────────

pub const LOGO: &str = "◈ HOUSAKY";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Spinner frames — Braille dot sweep (looks slick at 30fps)
pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

// ── Progress bar ─────────────────────────────────────────────────────────────

pub fn render_bar(value: f64, width: usize, filled_char: &str, empty_char: &str) -> String {
    let clamped = value.clamp(0.0, 1.0);
    let filled = (clamped * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", filled_char.repeat(filled), empty_char.repeat(empty))
}

pub fn render_gauge_bar(value: f64, width: usize) -> String {
    render_bar(value, width, "█", "░")
}

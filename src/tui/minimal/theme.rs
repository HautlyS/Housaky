//! AMOLED Black & White Psychedelic Futuristic Theme
//!
//! Pure void black with stark white accents, creating maximum contrast
//! and a striking futuristic aesthetic perfect for AMOLED displays.

use ratatui::style::{Color, Modifier, Style};

// ============================================================================
// HOUSAKY VOID THEME - Pure AMOLED Black + White Psychedelic
// ============================================================================

pub struct Theme;

impl Theme {
    // --- Backgrounds (Pure Black for AMOLED) ---
    pub const BG: Color = Color::Rgb(0, 0, 0);           // True black
    pub const BG_PANEL: Color = Color::Rgb(8, 8, 8);     // Barely visible
    pub const BG_ELEVATED: Color = Color::Rgb(15, 15, 15);
    pub const BG_SELECTED: Color = Color::Rgb(25, 25, 25);
    pub const BG_INPUT: Color = Color::Rgb(5, 5, 5);

    // --- Primary White Spectrum ---
    pub const WHITE: Color = Color::Rgb(255, 255, 255);
    pub const WHITE_DIM: Color = Color::Rgb(180, 180, 180);
    pub const WHITE_MUTED: Color = Color::Rgb(100, 100, 100);
    pub const WHITE_GHOST: Color = Color::Rgb(60, 60, 60);
    pub const WHITE_SUBTLE: Color = Color::Rgb(40, 40, 40);

    // --- Accent Colors (Minimal, High Impact) ---
    pub const ACCENT: Color = Color::Rgb(255, 255, 255);       // Pure white accent
    pub const ACCENT_DIM: Color = Color::Rgb(150, 150, 150);
    pub const CYAN: Color = Color::Rgb(0, 255, 255);           // Neon cyan for highlights
    pub const MAGENTA: Color = Color::Rgb(255, 0, 255);        // Psychedelic magenta

    // --- Agent Colors (Subtle differentiation) ---
    pub const AGENT_CODE: Color = Color::Rgb(200, 255, 200);   // Soft green
    pub const AGENT_WEB: Color = Color::Rgb(200, 200, 255);    // Soft blue
    pub const AGENT_ACADEMIC: Color = Color::Rgb(255, 200, 200); // Soft red
    pub const AGENT_DATA: Color = Color::Rgb(255, 255, 200);   // Soft yellow

    // --- Semantic ---
    pub const SUCCESS: Color = Color::Rgb(100, 255, 100);
    pub const ERROR: Color = Color::Rgb(255, 100, 100);
    pub const WARNING: Color = Color::Rgb(255, 200, 100);
    pub const INFO: Color = Color::Rgb(100, 200, 255);

    // --- Borders ---
    pub const BORDER: Color = Color::Rgb(40, 40, 40);
    pub const BORDER_FOCUS: Color = Color::Rgb(100, 100, 100);
    pub const BORDER_ACTIVE: Color = Color::Rgb(255, 255, 255);
}

// ============================================================================
// ASCII Art Constants
// ============================================================================

pub const LOGO_SMALL: &str = r#"
 _  _  ___  _   _ ___  _   _  _ __   __
| || |/ _ \| | | / __|| | / \| |\ \ / /
| __ | (_) | |_| \__ \| |/ _ \ | \ V /
|_||_|\___/ \___/|___/|_/_/ \_|  |_|
"#;

pub const LOGO_MINI: &str = "HOUSAKY";

pub const SEPARATOR: &str = "--------------------------------------------------------------------------------";
pub const SEPARATOR_THIN: &str = "........................................................................";
pub const SEPARATOR_DOUBLE: &str = "================================================================================";

pub const BOX_TL: char = '+';
pub const BOX_TR: char = '+';
pub const BOX_BL: char = '+';
pub const BOX_BR: char = '+';
pub const BOX_H: char = '-';
pub const BOX_V: char = '|';

// Psychedelic animation frames
pub const ANIM_FRAMES: &[&str] = &["*", "+", "x", "+"];
pub const SPINNER: &[&str] = &["|", "/", "-", "\\"];
pub const PULSE: &[&str] = &[".", "o", "O", "o"];

// Agent status indicators
pub const STATUS_ACTIVE: &str = "[+]";
pub const STATUS_IDLE: &str = "[-]";
pub const STATUS_ERROR: &str = "[!]";
pub const STATUS_LOADING: &str = "[~]";

// ============================================================================
// Style Functions
// ============================================================================

pub fn style_base() -> Style {
    Style::default().fg(Theme::WHITE_DIM).bg(Theme::BG)
}

pub fn style_panel() -> Style {
    Style::default().fg(Theme::WHITE_DIM).bg(Theme::BG_PANEL)
}

pub fn style_border() -> Style {
    Style::default().fg(Theme::BORDER)
}

pub fn style_border_focus() -> Style {
    Style::default().fg(Theme::BORDER_FOCUS)
}

pub fn style_border_active() -> Style {
    Style::default().fg(Theme::BORDER_ACTIVE)
}

pub fn style_title() -> Style {
    Style::default()
        .fg(Theme::WHITE)
        .add_modifier(Modifier::BOLD)
}

pub fn style_subtitle() -> Style {
    Style::default().fg(Theme::WHITE_MUTED)
}

pub fn style_dim() -> Style {
    Style::default().fg(Theme::WHITE_GHOST)
}

pub fn style_muted() -> Style {
    Style::default().fg(Theme::WHITE_SUBTLE)
}

pub fn style_selected() -> Style {
    Style::default()
        .fg(Theme::WHITE)
        .bg(Theme::BG_SELECTED)
        .add_modifier(Modifier::BOLD)
}

pub fn style_input() -> Style {
    Style::default().fg(Theme::WHITE).bg(Theme::BG_INPUT)
}

pub fn style_input_cursor() -> Style {
    Style::default()
        .fg(Theme::BG)
        .bg(Theme::WHITE)
}

pub fn style_user_message() -> Style {
    Style::default().fg(Theme::WHITE)
}

pub fn style_assistant_message() -> Style {
    Style::default().fg(Theme::WHITE_DIM)
}

pub fn style_system_message() -> Style {
    Style::default().fg(Theme::WHITE_MUTED)
}

pub fn style_agent_code() -> Style {
    Style::default().fg(Theme::AGENT_CODE)
}

pub fn style_agent_web() -> Style {
    Style::default().fg(Theme::AGENT_WEB)
}

pub fn style_agent_academic() -> Style {
    Style::default().fg(Theme::AGENT_ACADEMIC)
}

pub fn style_agent_data() -> Style {
    Style::default().fg(Theme::AGENT_DATA)
}

pub fn style_success() -> Style {
    Style::default().fg(Theme::SUCCESS)
}

pub fn style_error() -> Style {
    Style::default().fg(Theme::ERROR)
}

pub fn style_warning() -> Style {
    Style::default().fg(Theme::WARNING)
}

pub fn style_info() -> Style {
    Style::default().fg(Theme::INFO)
}

pub fn style_hotkey() -> Style {
    Style::default()
        .fg(Theme::WHITE)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
}

pub fn style_popup_bg() -> Style {
    Style::default().fg(Theme::WHITE).bg(Theme::BG_ELEVATED)
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Truncate string with ellipsis
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        ".".repeat(max_len)
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Center text within width
pub fn center(s: &str, width: usize) -> String {
    if s.len() >= width {
        s.to_string()
    } else {
        let padding = (width - s.len()) / 2;
        format!("{:>width$}", s, width = s.len() + padding)
    }
}

/// Create a horizontal line
pub fn hline(width: usize) -> String {
    BOX_H.to_string().repeat(width)
}

/// Format agent status with color indicator
pub fn format_agent_status(active: bool, loading: bool) -> &'static str {
    if loading {
        STATUS_LOADING
    } else if active {
        STATUS_ACTIVE
    } else {
        STATUS_IDLE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
        assert_eq!(truncate("hi", 2), "hi");
    }

    #[test]
    fn test_center() {
        assert_eq!(center("hi", 6), "  hi");
    }
}

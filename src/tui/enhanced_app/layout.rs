use ratatui::layout::{Constraint, Direction, Layout, Rect};
use crate::tui::enhanced_app::state::ViewMode;

// ── Terminal size breakpoints ─────────────────────────────────────────────────

pub struct TermSize {
    pub width:  u16,
    pub height: u16,
}

impl TermSize {
    pub fn from(area: Rect) -> Self {
        Self { width: area.width, height: area.height }
    }

    pub fn is_narrow(&self) -> bool { self.width < 100 }
    pub fn is_compact(&self) -> bool { self.height < 30 }
    pub fn sidebar_width(&self) -> u16 {
        if self.is_narrow() { 0 } else { 32 }
    }
}

// ── Top-level zones ───────────────────────────────────────────────────────────

pub struct RootZones {
    pub header:  Rect,   // title bar + tab bar
    pub body:    Rect,   // main content (chat + optional sidebar)
    pub input:   Rect,   // message input box
    pub footer:  Rect,   // status bar
}

impl RootZones {
    pub fn compute(area: Rect) -> Self {
        let sz = TermSize::from(area);
        let input_height = if sz.is_compact() { 3 } else { 4 };

        let vert = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),           // header
                Constraint::Min(8),              // body
                Constraint::Length(input_height),// input
                Constraint::Length(1),           // footer
            ])
            .split(area);

        RootZones {
            header: vert[0],
            body:   vert[1],
            input:  vert[2],
            footer: vert[3],
        }
    }
}

// ── Body zones (depends on ViewMode + sidebar toggle) ────────────────────────

pub struct BodyZones {
    pub main:    Rect,
    pub sidebar: Option<Rect>,
}

impl BodyZones {
    pub fn compute(body: Rect, view: ViewMode, sidebar_visible: bool) -> Self {
        let sz = TermSize::from(body);

        match view {
            ViewMode::Full => BodyZones { main: body, sidebar: None },

            ViewMode::Split => {
                if !sidebar_visible || sz.is_narrow() {
                    BodyZones { main: body, sidebar: None }
                } else {
                    let sidebar_w = sz.sidebar_width();
                    let horiz = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Min(40),
                            Constraint::Length(sidebar_w),
                        ])
                        .split(body);
                    BodyZones {
                        main:    horiz[0],
                        sidebar: Some(horiz[1]),
                    }
                }
            }

            ViewMode::Dashboard => {
                if sz.is_narrow() {
                    BodyZones { main: body, sidebar: None }
                } else {
                    let sidebar_w = (body.width / 2).max(36);
                    let horiz = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Min(30),
                            Constraint::Length(sidebar_w),
                        ])
                        .split(body);
                    BodyZones {
                        main:    horiz[0],
                        sidebar: Some(horiz[1]),
                    }
                }
            }
        }
    }
}

// ── Sidebar internal zones ────────────────────────────────────────────────────

pub struct SidebarZones {
    pub metrics:  Rect,
    pub goals:    Rect,
    pub activity: Rect,
}

impl SidebarZones {
    pub fn compute(sidebar: Rect) -> Self {
        let vert = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(9),
                Constraint::Min(5),
                Constraint::Length(6),
            ])
            .split(sidebar);
        SidebarZones {
            metrics:  vert[0],
            goals:    vert[1],
            activity: vert[2],
        }
    }
}

// ── Header internal zones ─────────────────────────────────────────────────────

pub struct HeaderZones {
    pub brand: Rect,
    pub tabs:  Rect,
    pub meta:  Rect,
}

impl HeaderZones {
    pub fn compute(header: Rect) -> Self {
        let horiz = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(16),
                Constraint::Min(20),
                Constraint::Length(24),
            ])
            .split(header);
        HeaderZones {
            brand: horiz[0],
            tabs:  horiz[1],
            meta:  horiz[2],
        }
    }
}

// ── Centered popup helper ─────────────────────────────────────────────────────

pub fn centered_rect(pct_x: u16, pct_y: u16, area: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - pct_y) / 2),
            Constraint::Percentage(pct_y),
            Constraint::Percentage((100 - pct_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - pct_x) / 2),
            Constraint::Percentage(pct_x),
            Constraint::Percentage((100 - pct_x) / 2),
        ])
        .split(vert[1])[1]
}

pub fn fixed_popup(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

//! A2A Panel - Inter-Agent Communication UI
//!
//! Real-time WebSocket connection status, peer list, message history,
//! and task delegation interface for agent-to-agent communication.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use std::time::{Instant, SystemTime, UNIX_EPOCH};

// ============================================================================
// Peer Status
// ============================================================================

/// Status of a connected peer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerConnectionStatus {
    Connected,
    Connecting,
    Disconnected,
    Error,
}

impl PeerConnectionStatus {
    pub fn indicator(&self) -> &'static str {
        match self {
            Self::Connected => "●",
            Self::Connecting => "◐",
            Self::Disconnected => "○",
            Self::Error => "✗",
        }
    }

    pub fn style(&self) -> Style {
        match self {
            Self::Connected => Style::default().fg(Color::Green),
            Self::Connecting => Style::default().fg(Color::Yellow),
            Self::Disconnected => Style::default().fg(Color::DarkGray),
            Self::Error => Style::default().fg(Color::Red),
        }
    }
}

// ============================================================================
// Peer Info
// ============================================================================

/// Information about a connected peer
#[derive(Debug, Clone)]
pub struct Peer {
    pub id: String,
    pub name: String,
    pub status: PeerConnectionStatus,
    pub address: String,
    pub latency_ms: Option<u64>,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub last_seen: Instant,
    pub tasks_active: u32,
}

impl Peer {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            status: PeerConnectionStatus::Disconnected,
            address: String::new(),
            latency_ms: None,
            messages_sent: 0,
            messages_received: 0,
            last_seen: Instant::now(),
            tasks_active: 0,
        }
    }

    pub fn latency_indicator(&self) -> String {
        match self.latency_ms {
            Some(ms) if ms < 50 => format!("{}ms", ms),
            Some(ms) if ms < 200 => format!("{}ms", ms),
            Some(ms) => format!("{}ms!", ms),
            None => "—".to_string(),
        }
    }
}

// ============================================================================
// Message Types
// ============================================================================

/// Type of A2A message
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A2AMsgType {
    Task,
    Result,
    Learning,
    Sync,
    Ping,
    Error,
    System,
}

impl A2AMsgType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Task => "TASK",
            Self::Result => "RES",
            Self::Learning => "LRN",
            Self::Sync => "SYNC",
            Self::Ping => "PING",
            Self::Error => "ERR",
            Self::System => "SYS",
        }
    }

    pub fn style(&self) -> Style {
        match self {
            Self::Task => Style::default().fg(Color::Cyan),
            Self::Result => Style::default().fg(Color::Green),
            Self::Learning => Style::default().fg(Color::Magenta),
            Self::Sync => Style::default().fg(Color::Blue),
            Self::Ping => Style::default().fg(Color::DarkGray),
            Self::Error => Style::default().fg(Color::Red),
            Self::System => Style::default().fg(Color::Yellow),
        }
    }
}

/// A message in the A2A communication
#[derive(Debug, Clone)]
pub struct A2AMsg {
    pub id: u64,
    pub msg_type: A2AMsgType,
    pub from: String,
    pub to: String,
    pub content: String,
    pub timestamp: u64,
    pub encrypted: bool,
}

impl A2AMsg {
    pub fn new(msg_type: A2AMsgType, from: &str, to: &str, content: &str) -> Self {
        Self {
            id: rand::random(),
            msg_type,
            from: from.to_string(),
            to: to.to_string(),
            content: content.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            encrypted: false,
        }
    }

    pub fn system(content: &str) -> Self {
        Self::new(A2AMsgType::System, "system", "all", content)
    }

    pub fn format_time(&self) -> String {
        let secs = self.timestamp % 86400;
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        let secs = secs % 60;
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    }
}

// ============================================================================
// Metrics Display
// ============================================================================

/// Real-time metrics for display
#[derive(Debug, Clone, Default)]
pub struct Metrics {
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub messages_per_sec: f32,
    pub active_connections: u32,
    pub total_messages: u64,
    pub uptime_secs: u64,
}

impl Metrics {
    pub fn cpu_bar(&self) -> String {
        let filled = (self.cpu_percent / 10.0) as usize;
        let empty = 10 - filled.min(10);
        format!(
            "[{}{}]",
            "█".repeat(filled),
            "░".repeat(empty)
        )
    }

    pub fn memory_bar(&self) -> String {
        let filled = (self.memory_percent / 10.0) as usize;
        let empty = 10 - filled.min(10);
        format!(
            "[{}{}]",
            "█".repeat(filled),
            "░".repeat(empty)
        )
    }

    pub fn uptime_formatted(&self) -> String {
        let days = self.uptime_secs / 86400;
        let hours = (self.uptime_secs % 86400) / 3600;
        let mins = (self.uptime_secs % 3600) / 60;
        if days > 0 {
            format!("{}d {}h", days, hours)
        } else if hours > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}m", mins)
        }
    }
}

// ============================================================================
// A2A Panel State
// ============================================================================

/// State for the A2A communication panel
pub struct A2APanel {
    /// Connected peers
    pub peers: Vec<Peer>,
    /// Message history
    pub messages: Vec<A2AMsg>,
    /// Current metrics
    pub metrics: Metrics,
    /// Selected peer index
    pub selected_peer: usize,
    /// Selected message index
    pub selected_message: usize,
    /// List state for peers
    pub peer_list_state: ListState,
    /// List state for messages
    pub msg_list_state: ListState,
    /// Input buffer for composing messages
    pub input: String,
    /// Panel focus: 0 = peers, 1 = messages, 2 = input
    pub focus: u8,
    /// WebSocket connection status
    pub ws_status: PeerConnectionStatus,
    /// Secure channel status
    pub secure_established: bool,
    /// Panel visible
    pub visible: bool,
    /// Last heartbeat
    pub last_heartbeat: Instant,
}

impl A2APanel {
    pub fn new() -> Self {
        let mut peer_list_state = ListState::default();
        peer_list_state.select(Some(0));

        let mut msg_list_state = ListState::default();
        msg_list_state.select(Some(0));

        Self {
            peers: Vec::new(),
            messages: vec![A2AMsg::system("A2A Panel initialized")],
            metrics: Metrics::default(),
            selected_peer: 0,
            selected_message: 0,
            peer_list_state,
            msg_list_state,
            input: String::new(),
            focus: 0,
            ws_status: PeerConnectionStatus::Disconnected,
            secure_established: false,
            visible: true,
            last_heartbeat: Instant::now(),
        }
    }

    /// Toggle panel visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Cycle focus between panels
    pub fn cycle_focus(&mut self) {
        self.focus = (self.focus + 1) % 3;
    }

    /// Select next peer
    pub fn next_peer(&mut self) {
        if self.peers.is_empty() {
            return;
        }
        self.selected_peer = (self.selected_peer + 1) % self.peers.len();
        self.peer_list_state.select(Some(self.selected_peer));
    }

    /// Select previous peer
    pub fn prev_peer(&mut self) {
        if self.peers.is_empty() {
            return;
        }
        self.selected_peer = if self.selected_peer == 0 {
            self.peers.len() - 1
        } else {
            self.selected_peer - 1
        };
        self.peer_list_state.select(Some(self.selected_peer));
    }

    /// Select next message
    pub fn next_message(&mut self) {
        if self.messages.is_empty() {
            return;
        }
        self.selected_message = (self.selected_message + 1) % self.messages.len();
        self.msg_list_state.select(Some(self.selected_message));
    }

    /// Select previous message
    pub fn prev_message(&mut self) {
        if self.messages.is_empty() {
            return;
        }
        self.selected_message = if self.selected_message == 0 {
            self.messages.len() - 1
        } else {
            self.selected_message - 1
        };
        self.msg_list_state.select(Some(self.selected_message));
    }

    /// Add a peer
    pub fn add_peer(&mut self, peer: Peer) {
        if !self.peers.iter().any(|p| p.id == peer.id) {
            self.peers.push(peer);
        }
    }

    /// Remove a peer
    pub fn remove_peer(&mut self, peer_id: &str) {
        self.peers.retain(|p| p.id != peer_id);
    }

    /// Update peer status
    pub fn update_peer_status(&mut self, peer_id: &str, status: PeerConnectionStatus) {
        if let Some(peer) = self.peers.iter_mut().find(|p| p.id == peer_id) {
            peer.status = status;
            peer.last_seen = Instant::now();
        }
    }

    /// Add a message
    pub fn add_message(&mut self, msg: A2AMsg) {
        self.messages.push(msg);
        // Keep only last 500 messages
        if self.messages.len() > 500 {
            self.messages.remove(0);
        }
    }

    /// Clear message history
    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.add_message(A2AMsg::system("Message history cleared"));
    }

    /// Update metrics
    pub fn update_metrics(&mut self, metrics: Metrics) {
        self.metrics = metrics;
    }

    /// Handle input character
    pub fn handle_input(&mut self, c: char) {
        self.input.push(c);
    }

    /// Handle backspace
    pub fn handle_backspace(&mut self) {
        self.input.pop();
    }

    /// Send current input
    pub fn send_input(&mut self) -> Option<String> {
        if self.input.is_empty() {
            return None;
        }
        let msg = self.input.clone();
        self.input.clear();
        Some(msg)
    }

    /// Get selected peer
    pub fn selected_peer(&self) -> Option<&Peer> {
        self.peers.get(self.selected_peer)
    }

    /// Draw the panel
    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Split into left (peers) and right (messages/input)
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);

        self.draw_peers(frame, chunks[0]);
        self.draw_messages_and_input(frame, chunks[1]);
    }

    fn draw_peers(&mut self, frame: &mut Frame, area: Rect) {
        let focused = self.focus == 0;
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Header with connection status
        let title = Span::styled(
            format!(" PEERS [{}] ", self.ws_status.indicator()),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let items: Vec<ListItem> = self
            .peers
            .iter()
            .enumerate()
            .map(|(i, peer)| {
                let is_selected = i == self.selected_peer;
                let status_style = peer.status.style();

                let content = Line::from(vec![
                    Span::styled(peer.status.indicator(), status_style),
                    Span::raw(" "),
                    Span::styled(
                        &peer.name,
                        if is_selected {
                            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::Gray)
                        },
                    ),
                    Span::raw(" "),
                    Span::styled(
                        peer.latency_indicator(),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]);

                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().fg(Color::Yellow))
            .highlight_symbol("► ");

        frame.render_stateful_widget(list, area, &mut self.peer_list_state);

        // Draw metrics footer
        let footer_area = Rect::new(area.x, area.y + area.height - 3, area.width, 3);
        let metrics_text = vec![
            Line::from(vec![
                Span::styled("CPU ", Style::default().fg(Color::DarkGray)),
                Span::styled(self.metrics.cpu_bar(), Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::styled("MEM ", Style::default().fg(Color::DarkGray)),
                Span::styled(self.metrics.memory_bar(), Style::default().fg(Color::Blue)),
            ]),
        ];

        let metrics_para = Paragraph::new(metrics_text).wrap(Wrap { trim: false });
        frame.render_widget(metrics_para, footer_area);
    }

    fn draw_messages_and_input(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(3)])
            .split(area);

        self.draw_messages(frame, chunks[0]);
        self.draw_input(frame, chunks[1]);
    }

    fn draw_messages(&mut self, frame: &mut Frame, area: Rect) {
        let focused = self.focus == 1;
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let secure_indicator = if self.secure_established { "🔒" } else { "🔓" };
        let title = Span::styled(
            format!(" A2A MESSAGES {} ", secure_indicator),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let items: Vec<ListItem> = self
            .messages
            .iter()
            .enumerate()
            .map(|(i, msg)| {
                let is_selected = i == self.selected_message;
                let type_style = msg.msg_type.style();

                let content = Line::from(vec![
                    Span::styled(
                        format!("[{}]", msg.format_time()),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw(" "),
                    Span::styled(format!("[{}]", msg.msg_type.label()), type_style),
                    Span::raw(" "),
                    Span::styled(
                        if msg.encrypted { "E" } else { " " },
                        Style::default().fg(Color::Magenta),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!("{}:", msg.from),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        if msg.content.len() > 60 {
                            format!("{}...", &msg.content[..57])
                        } else {
                            msg.content.clone()
                        },
                        if is_selected {
                            Style::default().fg(Color::White)
                        } else {
                            Style::default().fg(Color::Gray)
                        },
                    ),
                ]);

                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().fg(Color::Yellow))
            .highlight_symbol("► ");

        frame.render_stateful_widget(list, area, &mut self.msg_list_state);
    }

    fn draw_input(&mut self, frame: &mut Frame, area: Rect) {
        let focused = self.focus == 2;
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let title = Span::styled(
            " SEND MESSAGE ",
            Style::default().fg(Color::White),
        );

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let input_text = if self.input.is_empty() {
            Line::from(Span::styled(
                "Type message to selected peer... (Enter to send)",
                Style::default().fg(Color::DarkGray),
            ))
        } else {
            Line::from(Span::raw(&self.input))
        };

        let para = Paragraph::new(input_text).block(block);
        frame.render_widget(para, area);
    }
}

impl Default for A2APanel {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Quick Commands
// ============================================================================

/// Quick command for A2A panel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickCommand {
    Ping,
    Sync,
    Status,
    Delegate,
    ShareLearning,
    RequestReview,
}

impl QuickCommand {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Ping => "PING",
            Self::Sync => "SYNC",
            Self::Status => "STATUS",
            Self::Delegate => "DELEGATE",
            Self::ShareLearning => "LEARN",
            Self::RequestReview => "REVIEW",
        }
    }

    pub fn all() -> &'static [QuickCommand] {
        &[
            Self::Ping,
            Self::Sync,
            Self::Status,
            Self::Delegate,
            Self::ShareLearning,
            Self::RequestReview,
        ]
    }
}

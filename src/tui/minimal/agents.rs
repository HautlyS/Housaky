//! Kowalski Agents Panel
//!
//! Displays available agents and their status in a minimal sidebar.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use super::theme::{self, Theme};

/// Kowalski agent types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentType {
    Code,
    Web,
    Academic,
    Data,
}

impl AgentType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Web => "web",
            Self::Academic => "academic",
            Self::Data => "data",
        }
    }

    pub fn display(&self) -> &'static str {
        match self {
            Self::Code => "Code Agent",
            Self::Web => "Web Agent",
            Self::Academic => "Academic Agent",
            Self::Data => "Data Agent",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Code => "Code analysis & refactoring",
            Self::Web => "Web research & scraping",
            Self::Academic => "Paper analysis & citations",
            Self::Data => "Data processing & analysis",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Code => "</>",
            Self::Web => "WWW",
            Self::Academic => "DOC",
            Self::Data => "CSV",
        }
    }

    pub fn style(&self) -> Style {
        match self {
            Self::Code => theme::style_agent_code(),
            Self::Web => theme::style_agent_web(),
            Self::Academic => theme::style_agent_academic(),
            Self::Data => theme::style_agent_data(),
        }
    }

    pub fn all() -> &'static [AgentType] {
        &[Self::Code, Self::Web, Self::Academic, Self::Data]
    }
}

/// Agent status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Available,
    Busy,
    Offline,
    Error,
}

impl AgentStatus {
    pub fn indicator(&self) -> &'static str {
        match self {
            Self::Available => "[+]",
            Self::Busy => "[~]",
            Self::Offline => "[-]",
            Self::Error => "[!]",
        }
    }

    pub fn style(&self) -> Style {
        match self {
            Self::Available => theme::style_success(),
            Self::Busy => theme::style_warning(),
            Self::Offline => theme::style_dim(),
            Self::Error => theme::style_error(),
        }
    }
}

/// Single agent entry
#[derive(Debug, Clone)]
pub struct Agent {
    pub agent_type: AgentType,
    pub status: AgentStatus,
    pub task_count: u64,
    pub last_response: Option<String>,
}

impl Agent {
    pub fn new(agent_type: AgentType) -> Self {
        Self {
            agent_type,
            status: AgentStatus::Offline,
            task_count: 0,
            last_response: None,
        }
    }

    pub fn set_status(&mut self, status: AgentStatus) {
        self.status = status;
    }

    pub fn increment_tasks(&mut self) {
        self.task_count += 1;
    }
}

/// Agents panel state
pub struct AgentsPanel {
    pub agents: Vec<Agent>,
    pub selected: usize,
    pub list_state: ListState,
    pub visible: bool,
}

impl AgentsPanel {
    pub fn new() -> Self {
        let agents = AgentType::all()
            .iter()
            .map(|&t| Agent::new(t))
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            agents,
            selected: 0,
            list_state,
            visible: true,
        }
    }

    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    pub fn next(&mut self) {
        if self.agents.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.agents.len();
        self.list_state.select(Some(self.selected));
    }

    pub fn previous(&mut self) {
        if self.agents.is_empty() {
            return;
        }
        self.selected = if self.selected == 0 {
            self.agents.len() - 1
        } else {
            self.selected - 1
        };
        self.list_state.select(Some(self.selected));
    }

    pub fn selected_agent(&self) -> Option<&Agent> {
        self.agents.get(self.selected)
    }

    pub fn selected_type(&self) -> Option<AgentType> {
        self.selected_agent().map(|a| a.agent_type)
    }

    pub fn set_agent_status(&mut self, agent_type: AgentType, status: AgentStatus) {
        if let Some(agent) = self.agents.iter_mut().find(|a| a.agent_type == agent_type) {
            agent.set_status(status);
        }
    }

    pub fn set_all_available(&mut self) {
        for agent in &mut self.agents {
            agent.set_status(AgentStatus::Available);
        }
    }

    pub fn set_all_offline(&mut self) {
        for agent in &mut self.agents {
            agent.set_status(AgentStatus::Offline);
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect, focused: bool) {
        if !self.visible {
            return;
        }

        let border_style = if focused {
            theme::style_border_focus()
        } else {
            theme::style_border()
        };

        let block = Block::default()
            .title(Span::styled(" AGENTS ", theme::style_title()))
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(theme::style_panel());

        let items: Vec<ListItem> = self
            .agents
            .iter()
            .enumerate()
            .map(|(i, agent)| {
                let is_selected = i == self.selected;
                let status_style = agent.status.style();

                let content = Line::from(vec![
                    Span::styled(agent.status.indicator(), status_style),
                    Span::raw(" "),
                    Span::styled(agent.agent_type.icon(), agent.agent_type.style()),
                    Span::raw(" "),
                    Span::styled(
                        agent.agent_type.name(),
                        if is_selected {
                            theme::style_selected()
                        } else {
                            Style::default().fg(Theme::WHITE_DIM)
                        },
                    ),
                ]);

                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(theme::style_selected())
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    /// Compact single-line status for header
    pub fn status_line(&self) -> Line<'static> {
        let spans: Vec<Span> = self
            .agents
            .iter()
            .flat_map(|a| {
                vec![
                    Span::styled(a.status.indicator(), a.status.style()),
                    Span::styled(a.agent_type.icon(), a.agent_type.style()),
                    Span::raw(" "),
                ]
            })
            .collect();
        Line::from(spans)
    }
}

impl Default for AgentsPanel {
    fn default() -> Self {
        Self::new()
    }
}

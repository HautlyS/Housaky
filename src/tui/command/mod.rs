pub mod autocomplete;
pub mod commands;
pub mod parser;
pub mod ui;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Config { section: Option<String> },
    Model { name: Option<String> },
    Provider { name: Option<String> },
    Fallback { action: FallbackAction },
    Clear,
    Save { path: Option<String> },
    Export { format: String },
    Usage,
    Keys,
    Status,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FallbackAction {
    List,
    Rotate,
    Add { name: String },
    Status,
}

#[derive(Debug, Clone)]
pub struct CommandState {
    pub active: bool,
    pub input: String,
    pub cursor: usize,
    pub suggestions: Vec<CommandSuggestion>,
    pub selected_suggestion: usize,
    pub history: Vec<String>,
    pub history_index: usize,
}

#[derive(Debug, Clone)]
pub struct CommandSuggestion {
    pub command: String,
    pub description: String,
    pub args_hint: String,
}

impl Default for CommandState {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandState {
    pub fn new() -> Self {
        Self {
            active: false,
            input: String::new(),
            cursor: 0,
            suggestions: Vec::new(),
            selected_suggestion: 0,
            history: Vec::new(),
            history_index: 0,
        }
    }

    pub fn activate(&mut self) {
        self.active = true;
        self.input.clear();
        self.cursor = 0;
        self.suggestions.clear();
        self.selected_suggestion = 0;
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.input.clear();
        self.cursor = 0;
        self.suggestions.clear();
    }

    pub fn push_char(&mut self, c: char) {
        self.input.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.update_suggestions();
    }

    pub fn pop_char(&mut self) {
        if self.cursor > 0 {
            let idx = self.cursor - 1;
            if idx < self.input.len() {
                self.input.remove(idx);
                self.cursor = idx;
            }
        }
        self.update_suggestions();
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.cursor.saturating_sub(1);
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor < self.input.len() {
            self.cursor += 1;
        }
    }

    pub fn select_next_suggestion(&mut self) {
        if !self.suggestions.is_empty() {
            self.selected_suggestion = (self.selected_suggestion + 1) % self.suggestions.len();
        }
    }

    pub fn select_prev_suggestion(&mut self) {
        if !self.suggestions.is_empty() {
            self.selected_suggestion = if self.selected_suggestion == 0 {
                self.suggestions.len() - 1
            } else {
                self.selected_suggestion - 1
            };
        }
    }

    pub fn accept_suggestion(&mut self) {
        if let Some(suggestion) = self.suggestions.get(self.selected_suggestion) {
            self.input = suggestion.command.clone();
            self.cursor = self.input.len();
        }
    }

    pub fn add_to_history(&mut self, command: &str) {
        if !command.is_empty() && self.history.last().map_or(true, |h| h != command) {
            self.history.push(command.to_string());
            if self.history.len() > 100 {
                self.history.remove(0);
            }
        }
        self.history_index = self.history.len();
    }

    pub fn history_prev(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            if let Some(cmd) = self.history.get(self.history_index) {
                self.input = cmd.clone();
                self.cursor = self.input.len();
            }
        }
    }

    pub fn history_next(&mut self) {
        if self.history_index < self.history.len().saturating_sub(1) {
            self.history_index += 1;
            if let Some(cmd) = self.history.get(self.history_index) {
                self.input = cmd.clone();
                self.cursor = self.input.len();
            }
        } else {
            self.history_index = self.history.len();
            self.input.clear();
            self.cursor = 0;
        }
    }

    fn update_suggestions(&mut self) {
        self.suggestions = autocomplete::get_suggestions(&self.input);
        self.selected_suggestion = 0;
    }

    pub fn parse(&self) -> Option<Command> {
        parser::parse(&self.input)
    }
}

pub static COMMAND_HELP: &[(&str, &str, &str)] = &[
    ("/help", "Show this help", ""),
    (
        "/config [section]",
        "Open config editor",
        "agent|tools|gateway|...",
    ),
    (
        "/model [name]",
        "Change or show model",
        "claude-sonnet-4|gpt-4o|...",
    ),
    (
        "/provider [name]",
        "Change or show provider",
        "openrouter|anthropic|...",
    ),
    ("/fallback list", "List fallback providers", ""),
    ("/fallback rotate", "Rotate to next provider", ""),
    ("/fallback status", "Show fallback status", ""),
    ("/clear", "Clear conversation", ""),
    ("/save [path]", "Save conversation", "chat.md|..."),
    (
        "/export [format]",
        "Export conversation",
        "json|markdown|txt",
    ),
    ("/usage", "Show usage statistics", ""),
    ("/keys", "Manage API keys", ""),
    ("/status", "Show system status", ""),
    ("/quit", "Exit the TUI", ""),
];

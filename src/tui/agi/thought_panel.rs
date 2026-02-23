pub struct ThoughtPanel {
    thoughts: Vec<String>,
    scroll_offset: usize,
}

impl ThoughtPanel {
    pub fn new() -> Self {
        Self {
            thoughts: Vec::new(),
            scroll_offset: 0,
        }
    }

    pub fn set_thoughts(&mut self, thoughts: Vec<String>) {
        self.thoughts = thoughts;
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset += 1;
    }
}

impl Default for ThoughtPanel {
    fn default() -> Self {
        Self::new()
    }
}

use crate::housaky::goal_engine::Goal;

pub struct GoalPanel {
    goals: Vec<Goal>,
    scroll_offset: usize,
}

impl GoalPanel {
    pub fn new() -> Self {
        Self {
            goals: Vec::new(),
            scroll_offset: 0,
        }
    }

    pub fn set_goals(&mut self, goals: Vec<Goal>) {
        self.goals = goals;
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset += 1;
    }
}

impl Default for GoalPanel {
    fn default() -> Self {
        Self::new()
    }
}

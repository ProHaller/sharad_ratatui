// ui/utils.rs

use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

pub struct Spinner {
    pub current_frame: usize,
    pub is_spinning: bool,
}
impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Spinner {
    pub fn new() -> Self {
        Spinner {
            current_frame: 0,
            is_spinning: false,
        }
    }

    pub fn start(&mut self) {
        self.is_spinning = true;
    }

    pub fn stop(&mut self) {
        self.is_spinning = false;
    }

    pub fn tick(&mut self) {
        if self.is_spinning {
            self.current_frame = (self.current_frame + 1) % SPINNER_CHARS.len();
        }
    }

    pub fn get_frame(&self) -> char {
        SPINNER_CHARS[self.current_frame]
    }
}

pub fn spinner_frame(spinner: &Spinner) -> String {
    format!("AI is thinking {}", spinner.get_frame())
}

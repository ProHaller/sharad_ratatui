// ui/utils.rs

use crossterm::execute;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    style::Print,
    terminal::{Clear, ClearType},
};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::io::{self, Write};
use std::thread::sleep;
use std::time::Duration;

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

use std::time::Instant;

const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const SPINNER_INTERVAL: Duration = Duration::from_millis(100);

pub struct Spinner {
    last_update: Instant,
    current_frame: usize,
}

impl Spinner {
    pub fn new() -> Self {
        Spinner {
            last_update: Instant::now(),
            current_frame: 0,
        }
    }

    pub fn get_frame(&mut self) -> char {
        let now = Instant::now();
        if now.duration_since(self.last_update) >= SPINNER_INTERVAL {
            self.current_frame = (self.current_frame + 1) % SPINNER_CHARS.len();
            self.last_update = now;
        }
        SPINNER_CHARS[self.current_frame]
    }
}

pub fn spinner_frame(spinner: &mut Spinner) -> String {
    format!("AI is thinking {}", spinner.get_frame())
}

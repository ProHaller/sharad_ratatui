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

use std::sync::{Arc, Mutex};
use std::thread;

const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const SPINNER_INTERVAL: Duration = Duration::from_millis(100);

pub struct Spinner {
    current_frame: Arc<Mutex<usize>>,
    is_spinning: Arc<Mutex<bool>>,
}

impl Spinner {
    pub fn new() -> Self {
        Spinner {
            current_frame: Arc::new(Mutex::new(0)),
            is_spinning: Arc::new(Mutex::new(false)),
        }
    }

    pub fn start(&self) {
        let current_frame = Arc::clone(&self.current_frame);
        let is_spinning = Arc::clone(&self.is_spinning);

        *is_spinning.lock().unwrap() = true;

        thread::spawn(move || {
            while *is_spinning.lock().unwrap() {
                let mut frame = current_frame.lock().unwrap();
                *frame = (*frame + 1) % SPINNER_CHARS.len();
                drop(frame); // Explicitly drop the lock
                thread::sleep(SPINNER_INTERVAL);
            }
        });
    }

    pub fn stop(&self) {
        *self.is_spinning.lock().unwrap() = false;
    }

    pub fn get_frame(&self) -> char {
        let frame = *self.current_frame.lock().unwrap();
        SPINNER_CHARS[frame]
    }
}

pub fn spinner_frame(spinner: &Spinner) -> String {
    format!("AI is thinking {}", spinner.get_frame())
}

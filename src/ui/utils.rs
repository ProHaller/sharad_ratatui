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

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

pub struct Spinner {
    current_frame: Arc<AtomicUsize>,
}

impl Spinner {
    pub fn new() -> Self {
        Spinner {
            current_frame: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn next_frame(&self) {
        self.current_frame.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_frame(&self) -> char {
        let frame = self.current_frame.load(Ordering::Relaxed) % SPINNER_CHARS.len();
        SPINNER_CHARS[frame]
    }
}

pub fn spinner_frame(spinner: &Spinner) -> String {
    format!(" Game Master is thinking {} ", spinner.get_frame())
}

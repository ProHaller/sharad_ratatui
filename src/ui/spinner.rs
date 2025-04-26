use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

// HACK: Search for a spinner crate
const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
// const SPINNER_DICE: &[char] = &['⚀', '⚁', '⚂', '⚃', '⚄', '⚅',];

#[derive(Clone)]
pub struct Spinner {
    current_frame: Arc<AtomicUsize>,
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
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

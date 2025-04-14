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
// TODO: implement the spinner logic here
//
// TODO: Spinner logic
//
// app.update_spinner();
// if app.spinner_active {
//     let spinner_area = Rect::new(
//         left_chunk[0].x,
//         left_chunk[0].bottom() - 1,
//         left_chunk[0].width,
//         1,
//     );
//
//     let spinner_text = spinner_frame(&app.spinner);
//     let spinner_widget = Paragraph::new(spinner_text)
//         .style(Style::default().fg(Color::Green))
//         .alignment(Alignment::Center);
//
//     f.render_widget(spinner_widget, spinner_area);
// }

// pub fn start_spinner(&mut self) {
//     self.spinner_active = true;
//     self.last_spinner_update = Instant::now();
// }
//
// pub fn stop_spinner(&mut self) {
//     self.spinner_active = false;
// }
//
// pub fn update_spinner(&mut self) {
//     if self.spinner_active && self.last_spinner_update.elapsed() >= Duration::from_millis(100) {
//         self.spinner.next_frame();
//         self.last_spinner_update = Instant::now();
//     }
// }

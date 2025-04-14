use copypasta::ClipboardProvider;
use tui_input::Input;

use crate::context::Context;

pub trait Pastable {
    fn paste(&mut self, context: Context);
}

impl Pastable for Input {
    fn paste(&mut self, context: Context) {
        let mut clipboard = context.clipboard;
        if let Ok(pasted_text) = clipboard.get_contents() {
            let mut value = self.value().to_string();
            value.push_str(&pasted_text);
            Input::with_value(self.to_owned(), value);
        }
    }
}

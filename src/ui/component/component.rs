use crossterm::event::KeyEvent;
use ratatui::{buffer::Buffer, layout::Rect};
use std::fmt::Debug;

use crate::{app::Action, context::Context};

pub trait Component: Debug {
    fn on_key(&mut self, key: KeyEvent, context: Context) -> Option<Action>;
    // TODO: Implement KeyHints
    // fn key_hints(&mut self, key: KeyEvent, ) -> KeyHints
    fn render(&self, area: Rect, buffer: &mut Buffer, context: &Context);
}

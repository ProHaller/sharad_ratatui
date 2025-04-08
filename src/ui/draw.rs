// ui/draw.rs

use ratatui::layout::{Constraint, Flex, Layout, Rect};

// Constants for minimum terminal size.
pub const MIN_WIDTH: u16 = 40;
pub const MIN_HEIGHT: u16 = 20;

pub fn center_rect(original_area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [horizontal_area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(original_area);
    let [vertical_area] = Layout::vertical([vertical])
        .flex(Flex::Center)
        .areas(horizontal_area);
    vertical_area
}

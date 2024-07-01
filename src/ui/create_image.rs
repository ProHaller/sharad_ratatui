// ui/create_image.rs

use crate::app::App;
use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    widgets::*,
    Frame,
};

pub fn draw_create_image(f: &mut Frame, app: &App) {
    let chunk = f.size();

    let create_image_ui = Paragraph::new("Image creation functionality coming soon...")
        .style(Style::default().fg(Color::Magenta))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Create Image"));

    f.render_widget(create_image_ui, chunk);
}

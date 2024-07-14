// ui/save_name_input.rs

use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::Alignment,
    style::{Color, Style},
    widgets::*,
    Frame,
};

pub fn draw_save_name_input(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(f.size().height / 3)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(1),
            ]
            .as_ref(),
        )
        .split(f.size());

    let title = Paragraph::new("Enter Save Name")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    let input = Paragraph::new(app.save_name_input.value())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Save Name"));
    f.render_widget(input, chunks[1]);

    let instructions = Paragraph::new("Press Enter to confirm, Esc to cancel")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(instructions, chunks[2]);

    // Set cursor
    f.set_cursor(
        chunks[1].x + app.api_key_input.cursor() as u16 + 1,
        chunks[1].y + 1,
    );
}

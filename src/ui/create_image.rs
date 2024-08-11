use crate::app::{App, InputMode};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::Alignment,
    style::{Color, Style},
    widgets::*,
    Frame,
};

// Function to draw the image creation interface in the application.
pub fn draw_create_image(f: &mut Frame, app: &App) {
    let size = f.size();

    if size.width < 100 || size.height < 50 {
        let warning = Paragraph::new("Terminal too small. Please resize.")
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        f.render_widget(warning, size);
        return;
    }
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

    let title = Paragraph::new(" Enter an image prompt ")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    let input = Paragraph::new(app.image_prompt.value())
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(match app.input_mode {
                    InputMode::Normal => " Press 'e' to edit or 'r' to record",
                    InputMode::Editing => " Editing ",
                    InputMode::Recording => " Recording… Press 'Esc' to stop",
                })
                .border_style(Style::default().fg(match app.input_mode {
                    InputMode::Normal => Color::DarkGray,
                    InputMode::Editing => Color::Yellow,
                    InputMode::Recording => Color::Red,
                })),
        );
    f.render_widget(input, chunks[1]);

    let mode_indicator = match app.input_mode {
        InputMode::Normal => "NORMAL",
        InputMode::Editing => "EDITING",
        InputMode::Recording => "RECORDING",
    };
    let instructions = Paragraph::new(format!("{} | Enter: confirm | Esc: cancel", mode_indicator))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(instructions, chunks[2]);

    // Only show the cursor when in Editing mode
    if let InputMode::Editing = app.input_mode {
        f.set_cursor(
            chunks[1].x + app.image_prompt.visual_cursor() as u16 + 1,
            chunks[1].y + 1,
        );
    }
}

use crate::app::{App, InputMode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position},
    prelude::Alignment,
    style::{Color, Style},
    widgets::*,
};

pub fn draw_save_name_input(f: &mut Frame, app: &App) {
    let size = f.area();

    if size.width < 20 || size.height < 10 {
        let warning = Paragraph::new("Terminal too small. Please resize.")
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        f.render_widget(warning, size);
        return;
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(f.area().height / 3)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(1),
            ]
            .as_ref(),
        )
        .split(f.area());

    let title = Paragraph::new(" Enter Save Name ")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    let input = Paragraph::new(app.save_name_input.value())
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL)
                .title(match app.input_mode {
                    // TODO: Make the key description dynamic based on a Config File.
                    InputMode::Normal => " Press 'e' to edit or 'r' to record ",
                    InputMode::Editing => " Editing ",
                    InputMode::Recording => " Recording… Press 'Esc' to stop ",
                })
                .border_style(Style::default().fg(match app.input_mode {
                    InputMode::Normal => Color::DarkGray,
                    InputMode::Editing => Color::Yellow,
                    InputMode::Recording => Color::Red,
                })),
        );
    f.render_widget(input, chunks[1]);

    let mode_indicator = match app.input_mode {
        InputMode::Normal => " NORMAL ",
        InputMode::Editing => " EDITING ",
        InputMode::Recording => " RECORDING ",
    };
    let instructions = Paragraph::new(format!(
        "Mode:{} | Enter: confirm | Esc: cancel",
        mode_indicator
    ))
    .style(Style::default().fg(Color::Gray))
    .alignment(Alignment::Center);
    f.render_widget(instructions, chunks[2]);

    // Only show the cursor when in Editing mode
    if let InputMode::Editing = app.input_mode {
        f.set_cursor_position(Position::new(
            chunks[1].x + app.save_name_input.visual_cursor() as u16 + 1,
            chunks[1].y + 1,
        ));
    }
}

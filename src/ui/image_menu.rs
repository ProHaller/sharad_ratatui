use crate::{
    app::{Action, App, InputMode},
    context::Context,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position},
    prelude::{Alignment, Buffer, Rect},
    style::{Color, Style},
    widgets::*,
};
use tui_input::Input;

use super::Component;

#[derive(Default, Debug)]
pub struct ImageMenu {
    input: Input,
}

impl Component for ImageMenu {
    fn on_key(&mut self, key: crossterm::event::KeyEvent, context: Context) -> Option<Action> {
        todo!()

        // TODO: implement on_key with this of move it to input

        // fn handle_create_image_input(&mut self, key: KeyEvent) {
        //         match self.input_mode {
        //             InputMode::Normal => match key.code {
        //                 KeyCode::Char('e') => {
        //                     self.input_mode = InputMode::Editing;
        //                 }
        //                 KeyCode::Char('r') => {
        //                     self.start_recording();
        //                 }
        //                 KeyCode::Esc => self.state = AppState::MainMenu,
        //                 KeyCode::Enter => {
        //                     let prompt = self.image_prompt.value().to_owned();
        //
        //                     // let image_sender = self.image_sender.clone();
        //                     tokio::spawn(async move {
        //                         let _path = imager::generate_and_save_image(&prompt).await;
        //                         // let _ = image_sender.send(path.unwrap());
        //                     });
        //                     self.add_message(Message::new(
        //                         MessageType::System,
        //                         "Generating image...".to_string(),
        //                     ));
        //                     self.image_prompt.reset();
        //                     self.state = AppState::MainMenu;
        //                 }
        //                 _ => {}
        //             },
        //             InputMode::Editing => match key.code {
        //                 KeyCode::Esc => {
        //                     self.input_mode = InputMode::Normal;
        //                 }
        //                 KeyCode::Char('v') => {
        //                     if key.modifiers.contains(KeyModifiers::CONTROL) {
        //                         if let Err(e) = self.handle_paste() {
        //                             self.add_debug_message(format!("Failed to paste: {:#?}", e));
        //                         }
        //                     } else {
        //                         self.image_prompt.handle_event(&Event::Key(key));
        //                     }
        //                 }
        //                 _ => {
        //                     self.image_prompt.handle_event(&Event::Key(key));
        //                 }
        //             },
        //             InputMode::Recording if key.code == KeyCode::Esc => {
        //                 self.stop_recording();
        //             }
        //             _ => {}
        //         }
        //     }
        // fn handle_create_image_editing(&mut self, key: KeyEvent) {
        //     match key.code {
        //         KeyCode::Enter => {
        //             // Handle save name submission
        //             self.input_mode = InputMode::Normal;
        //         }
        //         KeyCode::Esc => {
        //             self.input_mode = InputMode::Normal;
        //         }
        //         KeyCode::Char('v') => {
        //             if key.modifiers.contains(KeyModifiers::CONTROL) {
        //                 if let Err(e) = self.handle_paste() {
        //                     self.add_debug_message(format!("Failed to paste: {:#?}", e));
        //                 }
        //             } else {
        //                 self.image_prompt.handle_event(&Event::Key(key));
        //             }
        //         }
        //         _ => {
        //             self.image_prompt.handle_event(&Event::Key(key));
        //         }
        //     }
        // }
    }

    fn render(&self, area: Rect, buffer: &mut Buffer, context: &Context) {
        todo!()
    }
}

// Function to draw the image creation interface in the application.
pub fn draw_create_image(f: &mut Frame, app: &App) {
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

    let title = Paragraph::new(" Enter an image prompt ")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    let input = Paragraph::new(app.image_prompt.value())
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .border_type(BorderType::Rounded)
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
        InputMode::Normal => " NORMAL ",
        InputMode::Editing => " EDITING ",
        InputMode::Recording => " RECORDING ",
    };
    let instructions = Paragraph::new(format!("{} | Enter: confirm | Esc: cancel", mode_indicator))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(instructions, chunks[2]);

    // Only show the cursor when in Editing mode
    if let InputMode::Editing = app.input_mode {
        f.set_cursor_position(Position::new(
            chunks[1].x + app.image_prompt.visual_cursor() as u16 + 1,
            chunks[1].y + 1,
        ));
    }
}

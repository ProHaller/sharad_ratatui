use std::path::PathBuf;

use crate::{
    app::{Action, App, InputMode},
    context::Context,
    imager,
};
use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Flex::Center, Layout, Position},
    prelude::{Alignment, Buffer, Rect},
    style::{Color, Style},
    widgets::*,
};
use tui_input::{Input, backend::crossterm::EventHandler};

use super::{Component, MainMenu};

#[derive(Default, Debug)]
pub struct ImageMenu {
    input: Input,
    path: Vec<PathBuf>,
}

impl Component for ImageMenu {
    fn on_key(&mut self, key: KeyEvent, context: Context) -> Option<Action> {
        match context.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('e') => Some(Action::SwitchInputMode(InputMode::Editing)),

                KeyCode::Char('r') => Some(Action::SwitchInputMode(InputMode::Recording)),
                KeyCode::Esc => Some(Action::SwitchComponent(Box::new(MainMenu::default()))),
                KeyCode::Enter => {
                    // let prompt = self.input.value().clone().to_string();
                    // let image_sender = self.image_sender.clone();
                    // tokio::spawn(async move {
                    //     self.path.push(
                    //         imager::generate_and_save_image(&prompt)
                    //             .await
                    //             .expect("Expected a valid image path"),
                    //     );
                    //     self.input.reset();
                    // });
                    Some(Action::SwitchComponent(Box::new(MainMenu::default())))
                }
                _ => None,
            },
            InputMode::Editing => match key.code {
                KeyCode::Esc => Some(Action::SwitchInputMode(InputMode::Normal)),
                _ => {
                    self.input.handle_event(&Event::Key(key));
                    None
                }
            },
            InputMode::Recording => Some(Action::SwitchInputMode(InputMode::Normal)),
        }
    }

    fn render(&self, area: Rect, buffer: &mut Buffer, context: &Context) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .flex(Center)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(1),
                ]
                .as_ref(),
            )
            .split(area);

        let title = Paragraph::new(" Enter an image prompt ")
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center);
        title.render(chunks[0], buffer);

        let input = Paragraph::new(self.input.value())
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .border_type(BorderType::Rounded)
                    .borders(Borders::ALL)
                    .title(match context.input_mode {
                        InputMode::Normal => " Press 'e' to edit or 'r' to record",
                        InputMode::Editing => " Editing ",
                        InputMode::Recording => " Recording… Press 'Esc' to stop",
                    })
                    .border_style(Style::default().fg(match context.input_mode {
                        InputMode::Normal => Color::DarkGray,
                        InputMode::Editing => Color::Yellow,
                        InputMode::Recording => Color::Red,
                    })),
            );
        input.render(chunks[1], buffer);

        let mode_indicator = match context.input_mode {
            InputMode::Normal => " NORMAL ",
            InputMode::Editing => " EDITING ",
            InputMode::Recording => " RECORDING ",
        };
        let instructions =
            Paragraph::new(format!("{} | Enter: confirm | Esc: cancel", mode_indicator))
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
        instructions.render(chunks[2], buffer);
    }
}

// Function to draw the image creation interface in the application.

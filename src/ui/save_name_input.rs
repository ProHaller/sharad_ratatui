// /ui/save_name_input.rs
use crate::{
    app::{Action, InputMode},
    audio::Transcription,
    context::Context,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Alignment,
    style::{Color, Style},
    widgets::*,
};
use tokio::sync::mpsc::UnboundedReceiver;
use tui_input::{Input, backend::crossterm::EventHandler};

use super::{Component, ComponentEnum, main_menu::MainMenu};

#[derive(Default, Debug)]
pub struct SaveName {
    input: Input,
    receiver: Option<UnboundedReceiver<String>>,
}

impl Component for SaveName {
    fn on_key(&mut self, key: KeyEvent, context: Context) -> Option<Action> {
        match context.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('e') => Some(Action::SwitchInputMode(InputMode::Editing)),
                KeyCode::Char('r') => {
                    if let Ok((receiver, transcription)) =
                        Transcription::new(None, context.ai_client?.clone())
                    {
                        self.receiver = Some(receiver);
                        Some(Action::SwitchInputMode(InputMode::Recording(transcription)))
                    } else {
                        Some(Action::SwitchInputMode(InputMode::Editing))
                    }
                }
                KeyCode::Esc => Some(Action::SwitchComponent(ComponentEnum::from(
                    MainMenu::default(),
                ))),
                KeyCode::Enter => {
                    if !self.input.value().is_empty() {
                        Some(Action::CreateNewGame(self.input.value().into()))
                    } else {
                        Some(Action::SwitchInputMode(InputMode::Editing))
                    }
                }
                _ => None,
            },
            InputMode::Editing => match key.code {
                KeyCode::Esc => Some(Action::SwitchInputMode(InputMode::Normal)),
                KeyCode::Char('v') => {
                    todo!("Centralize the text input handling for paste.")
                }
                KeyCode::Enter => Some(Action::SwitchInputMode(InputMode::Normal)),
                _ => {
                    self.input.handle_event(&crossterm::event::Event::Key(key));
                    None
                }
            },
            InputMode::Recording(_) if key.code == KeyCode::Esc => {
                // TODO: Stop recording if not in InputMode::Recording
                todo!("Need to implement the voice recording");
            }
            _ => None,
        }
    }
    fn render(&mut self, area: Rect, buffer: &mut Buffer, context: &Context) {
        self.check_transcription();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .flex(ratatui::layout::Flex::Center)
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

        let title = Paragraph::new(" Enter Save Name ")
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
                        // TODO: Make the key description dynamic based on a Config File.
                        InputMode::Normal => " Press 'e' to edit or 'r' to record ",
                        InputMode::Editing => " Editing ",
                        InputMode::Recording(_) => " Recording… Press 'Esc' to stop ",
                    })
                    .border_style(Style::default().fg(match context.input_mode {
                        InputMode::Normal => Color::DarkGray,
                        InputMode::Editing => Color::Yellow,
                        InputMode::Recording(_) => Color::Red,
                    })),
            );
        input.render(chunks[1], buffer);

        let mode_indicator = match context.input_mode {
            InputMode::Normal => " NORMAL ",
            InputMode::Editing => " EDITING ",
            InputMode::Recording(_) => " RECORDING ",
        };
        let instructions = Paragraph::new(format!(
            "Mode:{} | Enter: confirm | Esc: cancel",
            mode_indicator
        ))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
        instructions.render(chunks[2], buffer);
    }
}

impl SaveName {
    fn check_transcription(&mut self) {
        if let Some(receiver) = &mut self.receiver {
            if let Ok(transcription) = receiver.try_recv() {
                let input_value = format!("{} {}", self.input.value(), transcription);
                self.input = Input::with_value(self.input.clone(), input_value);
                self.receiver = None;
            }
        }
    }
}
